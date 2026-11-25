//! The baseline: the hook compiled into the program, behind the same trait. Nothing crosses a boundary except a call.

use crate::common::{Host, Runtime};

pub struct Native {
    total: [f64; 256],
    host: Host,
}

impl Native {
    pub fn new() -> Self {
        Self { total: [0.0; 256], host: Host::default() }
    }
}

impl Runtime for Native {
    fn on_damage(&mut self, entity: i32, amount: f64) -> Result<f64, String> {
        let slot = (entity & 255) as usize;
        let armor = self.host.armor(entity);
        let dmg = amount * (1.0 - armor);
        self.total[slot] += dmg;
        if self.total[slot] > 500.0 {
            self.total[slot] = 0.0;
            self.host.spawn(1, slot as i32);
        }
        Ok(dmg)
    }
    fn busy(&mut self, n: i32) -> Result<f64, String> {
        let mut acc = 0.0f64;
        let mut i = 0;
        while i < n {
            acc = acc * 1.0000001 + (i as f64);
            i += 1;
        }
        Ok(acc)
    }
    fn reset(&mut self) -> Result<(), String> {
        self.total = [0.0; 256];
        Ok(())
    }
    fn reload(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn host(&self) -> (u64, u64, u64) {
        (self.host.armor_calls, self.host.spawns, self.host.spawn_hash)
    }
    fn clear_host(&mut self) {
        self.host = Host::default();
    }
}
