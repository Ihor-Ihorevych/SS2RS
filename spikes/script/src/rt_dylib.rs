//! A native plugin: a `cdylib` with a C ABI, loaded at run time with `libloading`. No sandbox of any kind.

use crate::common::{Host, Runtime};
use libloading::{Library, Symbol};
use std::cell::RefCell;

thread_local! {
    static HOST: RefCell<Host> = RefCell::new(Host::default());
}

extern "C" fn get_armor(e: i32) -> f64 {
    HOST.with(|h| h.borrow_mut().armor(e))
}
extern "C" fn spawn(k: i32, s: i32) {
    HOST.with(|h| h.borrow_mut().spawn(k, s))
}

pub struct Dylib {
    path: std::path::PathBuf,
    _lib: Library,
    on_damage: unsafe extern "C" fn(i32, f64) -> f64,
    busy: unsafe extern "C" fn(i32) -> f64,
    reset: unsafe extern "C" fn(),
    spin: unsafe extern "C" fn() -> i32,
    generation: u32,
}

fn load(path: &std::path::Path) -> Result<Dylib, String> {
    unsafe {
        let lib = Library::new(path).map_err(|e| e.to_string())?;
        let init: Symbol<unsafe extern "C" fn(extern "C" fn(i32) -> f64, extern "C" fn(i32, i32))> =
            lib.get(b"plugin_init").map_err(|e| e.to_string())?;
        init(get_armor, spawn);
        let on_damage = *lib.get::<unsafe extern "C" fn(i32, f64) -> f64>(b"on_damage").map_err(|e| e.to_string())?;
        let busy = *lib.get::<unsafe extern "C" fn(i32) -> f64>(b"busy").map_err(|e| e.to_string())?;
        let reset = *lib.get::<unsafe extern "C" fn()>(b"reset").map_err(|e| e.to_string())?;
        let spin = *lib.get::<unsafe extern "C" fn() -> i32>(b"spin").map_err(|e| e.to_string())?;
        Ok(Dylib { path: path.to_path_buf(), _lib: lib, on_damage, busy, reset, spin, generation: 0 })
    }
}

impl Dylib {
    pub fn new(path: &str) -> Result<Self, String> {
        load(std::path::Path::new(path))
    }
    pub fn probe(&mut self, which: &str) -> Result<String, String> {
        match which {
            "spin" => {
                let spin = self.spin;
                let t = std::thread::spawn(move || unsafe { spin() });
                std::thread::sleep(std::time::Duration::from_millis(500));
                Ok(format!("still running after 500 ms, finished = {}: nothing in the process can stop it", t.is_finished()))
            }
            other => Err(format!("probe {other}: not applicable, a native plugin has the full rights of the process")),
        }
    }
}

impl Runtime for Dylib {
    fn probe(&mut self, which: &str) -> Result<String, String> {
        Dylib::probe(self, which)
    }
    fn on_damage(&mut self, entity: i32, amount: f64) -> Result<f64, String> {
        Ok(unsafe { (self.on_damage)(entity, amount) })
    }
    fn busy(&mut self, n: i32) -> Result<f64, String> {
        Ok(unsafe { (self.busy)(n) })
    }
    fn reset(&mut self) -> Result<(), String> {
        unsafe { (self.reset)() };
        Ok(())
    }
    fn reload(&mut self) -> Result<(), String> {
        // A loaded DLL cannot be overwritten on Windows: load a copy under a new name (the old one stays mapped).
        let n = self.generation + 1;
        let copy = self.path.with_file_name(format!("reload{n}_{}", self.path.file_name().unwrap().to_string_lossy()));
        std::fs::copy(&self.path, &copy).map_err(|e| e.to_string())?;
        let mut next = load(&copy)?;
        next.generation = n;
        next.path = self.path.clone();
        *self = next;
        Ok(())
    }
    fn host(&self) -> (u64, u64, u64) {
        HOST.with(|h| {
            let h = h.borrow();
            (h.armor_calls, h.spawns, h.spawn_hash)
        })
    }
    fn clear_host(&mut self) {
        HOST.with(|h| *h.borrow_mut() = Host::default());
    }
}
