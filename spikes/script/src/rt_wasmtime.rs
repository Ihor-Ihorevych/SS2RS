//! WebAssembly through wasmtime 51: Cranelift native code ("jit") or the Pulley bytecode interpreter ("pulley").

use crate::common::{Host, Runtime};
use wasmtime::{Caller, Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc};

pub const FUEL: u64 = 2_000_000;
const MEMORY_LIMIT: usize = 16 * 1024 * 1024;

struct Data {
    host: Host,
    limits: StoreLimits,
}

pub struct Wt {
    engine: Engine,
    wasm: Vec<u8>,
    store: Store<Data>,
    on_damage: TypedFunc<(i32, f64), f64>,
    busy: TypedFunc<i32, f64>,
    reset: TypedFunc<(), ()>,
    spin: TypedFunc<(), i32>,
    grow: TypedFunc<(), i32>,
    recurse: TypedFunc<i32, i32>,
    limits: bool,
}

fn engine(pulley: bool, limits: bool) -> Result<Engine, String> {
    let mut c = Config::new();
    if limits {
        c.consume_fuel(true);
    }
    c.max_wasm_stack(256 * 1024);
    if pulley {
        c.target("pulley64").map_err(|e| e.to_string())?;
    }
    Engine::new(&c).map_err(|e| e.to_string())
}

fn instantiate(engine: &Engine, wasm: &[u8], limits: bool) -> Result<Wt, String> {
    let g = |e: wasmtime::Error| e.to_string();
    let module = Module::new(engine, wasm).map_err(g)?;
    let data = Data {
        host: Host::default(),
        limits: StoreLimitsBuilder::new().memory_size(if limits { MEMORY_LIMIT } else { usize::MAX >> 4 }).instances(1).build(),
    };
    let mut store = Store::new(engine, data);
    store.limiter(|d| &mut d.limits);
    let mut linker = <Linker<Data>>::new(engine);
    linker.func_wrap("env", "get_armor", |mut c: Caller<'_, Data>, e: i32| -> f64 { c.data_mut().host.armor(e) }).map_err(g)?;
    linker.func_wrap("env", "spawn", |mut c: Caller<'_, Data>, k: i32, s: i32| c.data_mut().host.spawn(k, s)).map_err(g)?;
    if limits {
        store.set_fuel(FUEL).map_err(g)?;
    }
    let inst = linker.instantiate(&mut store, &module).map_err(g)?;
    Ok(Wt {
        engine: engine.clone(),
        wasm: wasm.to_vec(),
        on_damage: inst.get_typed_func(&mut store, "on_damage").map_err(g)?,
        busy: inst.get_typed_func(&mut store, "busy").map_err(g)?,
        reset: inst.get_typed_func(&mut store, "reset").map_err(g)?,
        spin: inst.get_typed_func(&mut store, "spin").map_err(g)?,
        grow: inst.get_typed_func(&mut store, "grow_until_refused").map_err(g)?,
        recurse: inst.get_typed_func(&mut store, "recurse").map_err(g)?,
        store,
        limits,
    })
}

impl Wt {
    pub fn new(wasm: &[u8], pulley: bool, limits: bool) -> Result<Self, String> {
        instantiate(&engine(pulley, limits)?, wasm, limits)
    }
    fn refuel(&mut self) {
        if self.limits {
            let _ = self.store.set_fuel(FUEL);
        }
    }
    pub fn probe(&mut self, which: &str) -> Result<String, String> {
        self.refuel();
        let g = |e: wasmtime::Error| format!("{e}").lines().next().unwrap_or("").to_string();
        match which {
            "spin" => self.spin.clone().call(&mut self.store, ()).map(|v| format!("returned {v}")).map_err(g),
            "memory" => {
                let r = self.grow.clone().call(&mut self.store, ()).map_err(g)?;
                Ok(format!("memory.grow refused after {} pages ({} KiB), the call returned normally", r, r * 64))
            }
            "recurse" => self.recurse.clone().call(&mut self.store, 0).map(|v| format!("returned {v}")).map_err(g),
            "fs" => {
                let m = Module::new(&self.engine, crate::common::wasi_import_module()).map_err(g)?;
                let linker = <Linker<Data>>::new(&self.engine);
                linker.instantiate(&mut self.store, &m).map(|_| "instantiated".to_string()).map_err(g)
            }
            _ => Err("unknown probe".into()),
        }
    }
}

impl Runtime for Wt {
    fn probe(&mut self, which: &str) -> Result<String, String> {
        Wt::probe(self, which)
    }
    fn on_damage(&mut self, entity: i32, amount: f64) -> Result<f64, String> {
        self.refuel();
        self.on_damage.clone().call(&mut self.store, (entity, amount)).map_err(|e| e.to_string())
    }
    fn busy(&mut self, n: i32) -> Result<f64, String> {
        self.refuel();
        self.busy.clone().call(&mut self.store, n).map_err(|e| e.to_string())
    }
    fn reset(&mut self) -> Result<(), String> {
        self.refuel();
        self.reset.clone().call(&mut self.store, ()).map_err(|e| e.to_string())
    }
    fn reload(&mut self) -> Result<(), String> {
        let w = std::mem::take(&mut self.wasm);
        let r = instantiate(&self.engine.clone(), &w, self.limits)?;
        *self = r;
        Ok(())
    }
    fn host(&self) -> (u64, u64, u64) {
        let h = &self.store.data().host;
        (h.armor_calls, h.spawns, h.spawn_hash)
    }
    fn clear_host(&mut self) {
        self.store.data_mut().host = Host::default();
    }
}
