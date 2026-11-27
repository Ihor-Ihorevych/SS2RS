//! The mod hook as a WebAssembly guest. Same arithmetic as `guest-dylib` and `src/native.rs`.
#![no_std]
#![allow(unsafe_code, static_mut_refs)]

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn get_armor(entity: i32) -> f64;
    fn spawn(kind: i32, slot: i32);
}

static mut TOTAL: [f64; 256] = [0.0; 256];

#[unsafe(no_mangle)]
pub extern "C" fn on_damage(entity: i32, amount: f64) -> f64 {
    let slot = (entity & 255) as usize;
    let armor = unsafe { get_armor(entity) };
    let dmg = amount * (1.0 - armor);
    unsafe {
        TOTAL[slot] += dmg;
        if TOTAL[slot] > 500.0 {
            TOTAL[slot] = 0.0;
            spawn(1, slot as i32);
        }
    }
    dmg
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
        x = core::hint::black_box(x.wrapping_add(1));
    }
}

/// Sandbox probe: asks for 64 KiB pages until the host refuses; returns the pages obtained.
#[unsafe(no_mangle)]
pub extern "C" fn grow_until_refused() -> i32 {
    let mut got = 0;
    loop {
        if core::arch::wasm32::memory_grow(0, 16) == usize::MAX {
            return got;
        }
        got += 16;
    }
}

/// Sandbox probe: unbounded recursion.
#[unsafe(no_mangle)]
pub extern "C" fn recurse(n: i32) -> i32 {
    let a = [n; 16];
    if n < 0 { return 0; }
    core::hint::black_box(&a);
    recurse(n + 1).wrapping_add(a[3])
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
