//! The mod hook as a native plugin behind a C ABI. Same arithmetic as `guest-wasm` and `src/native.rs`.
#![allow(unsafe_code, static_mut_refs)]

type GetArmor = extern "C" fn(i32) -> f64;
type Spawn = extern "C" fn(i32, i32);

static mut GET_ARMOR: Option<GetArmor> = None;
static mut SPAWN: Option<Spawn> = None;
static mut TOTAL: [f64; 256] = [0.0; 256];

/// # Safety
/// Called once by the host before any other export.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plugin_init(g: GetArmor, s: Spawn) {
    unsafe {
        GET_ARMOR = Some(g);
        SPAWN = Some(s);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn on_damage(entity: i32, amount: f64) -> f64 {
    let slot = (entity & 255) as usize;
    unsafe {
        let armor = (GET_ARMOR.unwrap())(entity);
        let dmg = amount * (1.0 - armor);
        TOTAL[slot] += dmg;
        if TOTAL[slot] > 500.0 {
            TOTAL[slot] = 0.0;
            (SPAWN.unwrap())(1, slot as i32);
        }
        dmg
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn busy(n: i32) -> f64 {
    let mut acc = 0.0f64;
    let mut i = 0;
    while i < n {
        acc = acc * 1.0000001 + (i as f64);
        i += 1;
    }
    acc
}

#[unsafe(no_mangle)]
pub extern "C" fn reset() {
    unsafe { TOTAL = [0.0; 256] };
}

/// Sandbox probe: never returns.
#[unsafe(no_mangle)]
pub extern "C" fn spin() -> i32 {
    let mut x = 0i32;
    loop {
        x = std::hint::black_box(x.wrapping_add(1));
    }
}
