//! The workload shared by every runtime, the host state and the checksum.

use std::cell::RefCell;
use std::rc::Rc;

pub const CALLS: usize = 10_000;

/// The entity of call `i`: deterministic, no randomness.
pub fn entity_of(i: usize) -> i32 {
    ((i * 7 + 3) % 1024) as i32
}

/// The damage of call `i`: 0.5 to 99.5.
pub fn amount_of(i: usize) -> f64 {
    ((i * 37) % 100) as f64 + 0.5
}

/// What the host answers to `get_armor(entity)`: 0 to 0.4375, a pure function of the entity.
pub fn armor_of(entity: i32) -> f64 {
    ((entity & 7) as f64) * 0.0625
}

/// Host state shared with the script through callbacks.
#[derive(Default, Debug)]
pub struct Host {
    pub armor_calls: u64,
    pub spawns: u64,
    pub spawn_hash: u64,
}

impl Host {
    pub fn spawn(&mut self, kind: i32, slot: i32) {
        self.spawns += 1;
        self.spawn_hash = fnv(self.spawn_hash, (kind as u64) << 32 | slot as u32 as u64);
    }
    pub fn armor(&mut self, entity: i32) -> f64 {
        self.armor_calls += 1;
        armor_of(entity)
    }
}

pub type SharedHost = Rc<RefCell<Host>>;

pub fn fnv(h: u64, v: u64) -> u64 {
    let mut h = if h == 0 { 0xcbf29ce484222325 } else { h };
    for b in v.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// A runtime under test. Every method is one call across the script boundary.
pub trait Runtime {
    fn on_damage(&mut self, entity: i32, amount: f64) -> Result<f64, String>;
    fn busy(&mut self, n: i32) -> Result<f64, String>;
    /// Zero the script's own state (not the host's).
    fn reset(&mut self) -> Result<(), String>;
    /// Reload the code and replace the running instance, as a hot reload would. Returns what it cost to compile.
    fn reload(&mut self) -> Result<(), String>;
    fn host(&self) -> (u64, u64, u64);
    fn clear_host(&mut self);
    /// A sandbox probe: `spin`, `memory`, `recurse`, `fs` or `eval`.
    fn probe(&mut self, _which: &str) -> Result<String, String> {
        Err("no probes for this runtime".into())
    }
    /// Bytes the runtime itself says it holds, when it can tell.
    fn own_memory(&self) -> Option<usize> {
        None
    }
}

/// One full pass of the workload: returns (checksum over every result and every spawn, sum).
pub fn run_pass(rt: &mut dyn Runtime, calls: usize) -> Result<(u64, f64), String> {
    rt.reset()?;
    rt.clear_host();
    let mut h = 0u64;
    let mut sum = 0.0f64;
    for i in 0..calls {
        let r = rt.on_damage(entity_of(i), amount_of(i))?;
        h = fnv(h, r.to_bits());
        sum += r;
    }
    let (_, spawns, spawn_hash) = rt.host();
    h = fnv(h, spawns);
    h = fnv(h, spawn_hash);
    Ok((h, sum))
}

/// A hand-assembled WebAssembly module that imports `wasi_snapshot_preview1.path_open` and has no code: a guest that wants a file.
pub fn wasi_import_module() -> Vec<u8> {
    let mut m = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    m.extend([0x01, 0x04, 0x01, 0x60, 0x00, 0x00]); // type section: () -> ()
    let (module, name) = (b"wasi_snapshot_preview1", b"path_open");
    let body_len = 1 + 1 + module.len() + 1 + name.len() + 2;
    m.extend([0x02, body_len as u8, 0x01, module.len() as u8]);
    m.extend(module);
    m.push(name.len() as u8);
    m.extend(name);
    m.extend([0x00, 0x00]); // kind: function, type 0
    m
}
