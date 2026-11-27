//! Lua 5.4 through mlua (the reference C implementation, vendored and compiled by the `cc` crate).

use crate::common::{Host, Runtime, SharedHost};
use mlua::{HookTriggers, Lua, LuaOptions, StdLib, VmState};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub const SCRIPT: &str = r#"
local total = {}
for i = 0, 255 do total[i] = 0.0 end
function reset() for i = 0, 255 do total[i] = 0.0 end end
function on_damage(entity, amount)
    local slot = entity & 255
    local armor = get_armor(entity)
    local dmg = amount * (1.0 - armor)
    total[slot] = total[slot] + dmg
    if total[slot] > 500.0 then
        total[slot] = 0.0
        do_spawn(1, slot)
    end
    return dmg
end
function busy(n)
    local acc = 0.0
    local i = 0
    while i < n do
        acc = acc * 1.0000001 + i
        i = i + 1
    end
    return acc
end
function spin() while true do end end
function memory() local a = {} local n = 0 while true do n = n + 1; a[n] = {n} end end
function recurse(n) return 1 + recurse(n + 1) end
function fs() return io.open("secret.txt") end
function os_probe() return os.execute("echo hi") end
function load_probe() return load("return 1")() end
"#;

const HOOK_EVERY: u32 = 1000;
pub const BUDGET_INSTRUCTIONS: i64 = 2_000_000;

pub struct LuaRt {
    lua: Lua,
    host: SharedHost,
    budget: Rc<Cell<i64>>,
    limits: bool,
}

fn build(host: &SharedHost, budget: &Rc<Cell<i64>>, limits: bool) -> Result<Lua, String> {
    // Only the pure libraries: no io, no os, no package, no debug.
    let libs = StdLib::MATH | StdLib::TABLE | StdLib::STRING | StdLib::UTF8;
    let lua = Lua::new_with(libs, LuaOptions::default()).map_err(|e| e.to_string())?;
    let h = host.clone();
    let f = lua.create_function(move |_, e: i32| Ok(h.borrow_mut().armor(e))).map_err(|e| e.to_string())?;
    lua.globals().set("get_armor", f).map_err(|e| e.to_string())?;
    let h = host.clone();
    let f = lua.create_function(move |_, (k, s): (i32, i32)| Ok(h.borrow_mut().spawn(k, s))).map_err(|e| e.to_string())?;
    lua.globals().set("do_spawn", f).map_err(|e| e.to_string())?;
    if limits {
        lua.set_memory_limit(16 * 1024 * 1024).map_err(|e| e.to_string())?;
        let b = budget.clone();
        lua.set_hook(HookTriggers::new().every_nth_instruction(HOOK_EVERY), move |_, _| {
            let left = b.get() - HOOK_EVERY as i64;
            b.set(left);
            if left <= 0 { Err(mlua::Error::runtime("instruction budget exhausted")) } else { Ok(VmState::Continue) }
        })
        .map_err(|e| e.to_string())?;
    }
    lua.load(SCRIPT).exec().map_err(|e| e.to_string())?;
    Ok(lua)
}

impl LuaRt {
    pub fn new(limits: bool) -> Result<Self, String> {
        let host: SharedHost = Rc::new(RefCell::new(Host::default()));
        let budget = Rc::new(Cell::new(BUDGET_INSTRUCTIONS));
        let lua = build(&host, &budget, limits)?;
        Ok(Self { lua, host, budget, limits })
    }
    fn f<A: mlua::IntoLuaMulti, R: mlua::FromLuaMulti>(&mut self, name: &str, a: A) -> Result<R, String> {
        self.budget.set(BUDGET_INSTRUCTIONS);
        let f: mlua::Function = self.lua.globals().get(name).map_err(|e| e.to_string())?;
        f.call::<R>(a).map_err(|e| e.to_string())
    }
    pub fn probe(&mut self, which: &str) -> Result<String, String> {
        match which {
            "spin" => self.f::<_, ()>("spin", ()).map(|_| "returned".into()),
            "memory" => self.f::<_, ()>("memory", ()).map(|_| "returned".into()),
            "recurse" => self.f::<_, i64>("recurse", 0i64).map(|v| format!("returned {v}")),
            "fs" => self.f::<_, mlua::Value>("fs", ()).map(|v| format!("returned {v:?}")),
            "eval" => self.f::<_, mlua::Value>("load_probe", ()).map(|v| format!("returned {v:?}")),
            _ => Err("unknown probe".into()),
        }
    }
}

impl Runtime for LuaRt {
    fn probe(&mut self, which: &str) -> Result<String, String> {
        LuaRt::probe(self, which)
    }
    fn on_damage(&mut self, entity: i32, amount: f64) -> Result<f64, String> {
        self.f("on_damage", (entity, amount))
    }
    fn busy(&mut self, n: i32) -> Result<f64, String> {
        self.f("busy", n)
    }
    fn reset(&mut self) -> Result<(), String> {
        self.f::<_, ()>("reset", ())
    }
    fn reload(&mut self) -> Result<(), String> {
        self.lua = build(&self.host, &self.budget, self.limits)?;
        Ok(())
    }
    fn host(&self) -> (u64, u64, u64) {
        let h = self.host.borrow();
        (h.armor_calls, h.spawns, h.spawn_hash)
    }
    fn clear_host(&mut self) {
        *self.host.borrow_mut() = Host::default();
    }
    fn own_memory(&self) -> Option<usize> {
        Some(self.lua.used_memory())
    }
}
