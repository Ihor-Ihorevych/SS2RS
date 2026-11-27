//! Rhai 1.x: a pure-Rust scripting language, evaluated by walking the syntax tree.

use crate::common::{Host, Runtime, SharedHost};
use rhai::{AST, CallFnOptions, Dynamic, Engine, Scope};
use std::cell::RefCell;
use std::rc::Rc;

pub const SCRIPT: &str = r#"
fn on_damage(entity, amount) {
    let slot = entity & 255;
    let armor = get_armor(entity);
    let dmg = amount * (1.0 - armor);
    this[slot] += dmg;
    if this[slot] > 500.0 {
        this[slot] = 0.0;
        do_spawn(1, slot);
    }
    dmg
}
fn busy(n) {
    let acc = 0.0;
    let i = 0;
    while i < n {
        acc = acc * 1.0000001 + to_float(i);
        i += 1;
    }
    acc
}
fn spin() { loop { } }
fn memory() { let a = []; loop { a.push(1); } }
fn recurse(n) { recurse(n + 1) }
fn fs() { open_file("secret.txt") }
"#;

pub struct Rhai {
    engine: Engine,
    ast: AST,
    state: Dynamic,
    host: SharedHost,
    limits: bool,
}

fn new_state() -> Dynamic {
    let v: Vec<Dynamic> = (0..256).map(|_| Dynamic::from_float(0.0)).collect();
    Dynamic::from_array(v)
}

fn build(host: &SharedHost, limits: bool) -> Engine {
    let mut e = Engine::new();
    let h = host.clone();
    e.register_fn("get_armor", move |entity: i64| -> f64 { h.borrow_mut().armor(entity as i32) });
    let h = host.clone();
    e.register_fn("do_spawn", move |kind: i64, slot: i64| h.borrow_mut().spawn(kind as i32, slot as i32));
    if limits {
        e.set_max_operations(2_000_000);
        e.set_max_call_levels(64);
        e.set_max_expr_depths(64, 64);
        e.set_max_array_size(100_000);
        e.set_max_string_size(100_000);
        e.set_max_map_size(10_000);
    }
    e.disable_symbol("eval");
    e
}

impl Rhai {
    pub fn new(limits: bool) -> Result<Self, String> {
        let host: SharedHost = Rc::new(RefCell::new(Host::default()));
        let engine = build(&host, limits);
        let ast = engine.compile(SCRIPT).map_err(|e| e.to_string())?;
        Ok(Self { engine, ast, state: new_state(), host, limits })
    }
    fn call(&mut self, name: &str, args: impl rhai::FuncArgs) -> Result<Dynamic, String> {
        let mut scope = Scope::new();
        let opts = CallFnOptions::new().eval_ast(false).rewind_scope(true).bind_this_ptr(&mut self.state);
        self.engine.call_fn_with_options::<Dynamic>(opts, &mut scope, &self.ast, name, args).map_err(|e| e.to_string())
    }
    pub fn probe(&mut self, which: &str) -> Result<String, String> {
        let r = match which {
            "spin" => self.call("spin", ()),
            "memory" => self.call("memory", ()),
            "recurse" => self.call("recurse", (0i64,)),
            "fs" => self.call("fs", ()),
            "eval" => return self.engine.compile("fn e() { eval(\"1 + 1\") }").map(|_| "compiled".to_string()).map_err(|e| e.to_string()),
            _ => return Err("unknown probe".into()),
        };
        r.map(|v| format!("returned {v}"))
    }
}

impl Runtime for Rhai {
    fn probe(&mut self, which: &str) -> Result<String, String> {
        Rhai::probe(self, which)
    }
    fn on_damage(&mut self, entity: i32, amount: f64) -> Result<f64, String> {
        self.call("on_damage", (entity as i64, amount))?.as_float().map_err(|t| t.to_string())
    }
    fn busy(&mut self, n: i32) -> Result<f64, String> {
        self.call("busy", (n as i64,))?.as_float().map_err(|t| t.to_string())
    }
    fn reset(&mut self) -> Result<(), String> {
        self.state = new_state();
        Ok(())
    }
    fn reload(&mut self) -> Result<(), String> {
        self.engine = build(&self.host, self.limits);
        self.ast = self.engine.compile(SCRIPT).map_err(|e| e.to_string())?;
        Ok(())
    }
    fn host(&self) -> (u64, u64, u64) {
        let h = self.host.borrow();
        (h.armor_calls, h.spawns, h.spawn_hash)
    }
    fn clear_host(&mut self) {
        *self.host.borrow_mut() = Host::default();
    }
}
