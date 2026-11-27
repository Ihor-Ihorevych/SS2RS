//! Spike for RFC 0012 (T13-002): the same gameplay-shaped hook in several ways of running mod code.
//!
//! One process measures one runtime (so memory and start-up are not polluted by another):
//!
//! ```text
//! script-spike --runtime native|rhai|lua|wasmi|wasmtime-jit|wasmtime-pulley|dylib|none
//!              [--wasm FILE] [--dylib FILE] [--no-limits] [--batches N] [--probes]
//! ```
//!
//! Output is `key=value` lines starting with `R `, easy to grep.
#![allow(unsafe_code)]

mod common;
mod mem;
mod native;
#[cfg(feature = "dylib")]
mod rt_dylib;
#[cfg(feature = "lua")]
mod rt_lua;
#[cfg(feature = "rhai")]
mod rt_rhai;
#[cfg(feature = "wasmi")]
mod rt_wasmi;
#[cfg(feature = "wasmtime")]
mod rt_wasmtime;

use common::{CALLS, Runtime, amount_of, entity_of, run_pass};
use std::hint::black_box;
use std::time::Instant;

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

#[allow(unused_variables)]
fn make(which: &str, limits: bool, wasm: Option<&str>, dylib: Option<&str>) -> Result<Box<dyn Runtime>, String> {
    #[allow(unused_variables)]
    let read_wasm = || -> Result<Vec<u8>, String> { std::fs::read(wasm.ok_or("--wasm FILE is needed")?).map_err(|e| e.to_string()) };
    Ok(match which {
        "native" => Box::new(native::Native::new()),
        #[cfg(feature = "rhai")]
        "rhai" => Box::new(rt_rhai::Rhai::new(limits)?),
        #[cfg(feature = "lua")]
        "lua" => Box::new(rt_lua::LuaRt::new(limits)?),
        #[cfg(feature = "wasmi")]
        "wasmi" => Box::new(rt_wasmi::Wasmi::new(&read_wasm()?, limits)?),
        #[cfg(feature = "wasmtime")]
        "wasmtime-jit" | "wasmtime-pulley" => Box::new(rt_wasmtime::Wt::new(&read_wasm()?, which == "wasmtime-pulley", limits)?),
        #[cfg(feature = "dylib")]
        "dylib" => Box::new(rt_dylib::Dylib::new(dylib.ok_or("--dylib FILE is needed")?)?),
        other => return Err(format!("runtime {other} is not built in")),
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let which = arg(&args, "--runtime").unwrap_or_else(|| "native".into());
    let limits = !args.iter().any(|a| a == "--no-limits");
    let batches: usize = arg(&args, "--batches").and_then(|s| s.parse().ok()).unwrap_or(30);
    let want_probes = args.iter().any(|a| a == "--probes");
    let wasm = arg(&args, "--wasm");
    let dylib = arg(&args, "--dylib");

    let m0 = mem::now();
    if which == "none" {
        println!("R runtime=none working_set_kb={} private_kb={}", m0.0 / 1024, m0.2 / 1024);
        return;
    }
    let t = Instant::now();
    let mut rt = match make(&which, limits, wasm.as_deref(), dylib.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            println!("R runtime={which} error={e}");
            std::process::exit(1);
        }
    };
    let init = t.elapsed();
    let m1 = mem::now();
    println!("R runtime={which} limits={limits} init_ms={:.2}", init.as_secs_f64() * 1e3);

    // Determinism: two full passes in one process, and the float loop.
    let a = run_pass(rt.as_mut(), CALLS).expect("pass 1");
    let b = run_pass(rt.as_mut(), CALLS).expect("pass 2");
    let (_, spawns, _) = rt.host();
    let busy_bits = rt.busy(1000).expect("busy").to_bits();
    println!(
        "R pass1_checksum={:016x} pass2_checksum={:016x} same={} sum_bits={:016x} spawns={} busy1000_bits={:016x}",
        a.0,
        b.0,
        a.0 == b.0,
        a.1.to_bits(),
        spawns,
        busy_bits
    );

    // Timing: `batches` batches of 10,000 hook calls; the median batch gives the per-call cost.
    for _ in 0..3 {
        run_pass(rt.as_mut(), CALLS).unwrap();
    }
    let mut per_call = Vec::new();
    for _ in 0..batches {
        rt.reset().unwrap();
        let t = Instant::now();
        let mut s = 0.0;
        for i in 0..CALLS {
            s += rt.on_damage(black_box(entity_of(i)), black_box(amount_of(i))).unwrap();
        }
        black_box(s);
        per_call.push(t.elapsed().as_nanos() as f64 / CALLS as f64);
    }
    per_call.sort_by(f64::total_cmp);
    println!(
        "R hook_ns_median={:.0} hook_ns_min={:.0} hook_ns_max={:.0} batches={}",
        per_call[per_call.len() / 2],
        per_call[0],
        per_call[per_call.len() - 1],
        batches
    );

    // The inner loop: busy(1000) is 1000 iterations of a multiply-add inside the script.
    let mut loops = Vec::new();
    for _ in 0..20 {
        let t = Instant::now();
        for _ in 0..10 {
            black_box(rt.busy(black_box(1000)).unwrap());
        }
        loops.push(t.elapsed().as_nanos() as f64 / 10_000.0);
    }
    loops.sort_by(f64::total_cmp);
    println!("R loop_iter_ns_median={:.1} loop_iter_ns_min={:.1}", loops[loops.len() / 2], loops[0]);

    // Hot reload: rebuild the engine/instance from the same code and swap it in.
    let mut reloads = Vec::new();
    for _ in 0..5 {
        let t = Instant::now();
        if let Err(e) = rt.reload() {
            println!("R reload_error={e}");
            break;
        }
        reloads.push(t.elapsed().as_secs_f64() * 1e3);
    }
    reloads.sort_by(f64::total_cmp);
    if !reloads.is_empty() {
        println!("R reload_ms_median={:.2} reload_ms_max={:.2}", reloads[reloads.len() / 2], reloads[reloads.len() - 1]);
    }
    let again = run_pass(rt.as_mut(), CALLS).unwrap();
    println!("R after_reload_checksum_same={}", again.0 == a.0);

    let m2 = mem::now();
    println!(
        "R mem_before_kb={} mem_after_init_kb={} mem_end_kb={} peak_kb={} private_end_kb={} runtime_own_kb={}",
        m0.0 / 1024,
        m1.0 / 1024,
        m2.0 / 1024,
        m2.1 / 1024,
        m2.2 / 1024,
        rt.own_memory().map(|b| (b / 1024).to_string()).unwrap_or_else(|| "n/a".into())
    );

    if want_probes {
        for p in ["spin", "memory", "recurse", "fs", "eval"] {
            let t = Instant::now();
            let out = rt.probe(p);
            let ms = t.elapsed().as_secs_f64() * 1e3;
            let healthy = if which == "dylib" && p == "spin" { "n/a".to_string() } else { rt.on_damage(1, 10.0).is_ok().to_string() };
            let (ok, text): (&str, String) = match out {
                Ok(s) => ("returned", s),
                Err(e) => ("stopped", e.lines().next().unwrap_or("").chars().take(120).collect()),
            };
            let m = mem::now();
            println!("R probe={p} outcome={ok} ms={ms:.1} usable_after={healthy} peak_kb={} detail={text}", m.1 / 1024);
        }
        if which == "dylib" {
            // The spinning plugin thread never ends: leave the process.
            std::process::exit(0);
        }
    }
}
