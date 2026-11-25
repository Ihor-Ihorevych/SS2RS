# Mod gameplay strategy spike (T13-002)

Throwaway benchmark behind `docs/rfcs/0012-mod-gameplay-strategy.md`. The same gameplay-shaped hook,
`on_damage(entity, amount) -> amount` (one call back into the host for the armour, one `spawn` call now and then, per-entity state
inside the script), is run 10,000 times in each way of running mod code: compiled in (the baseline), a native plugin (`cdylib`,
`libloading`), Rhai, Lua 5.4 (`mlua`, vendored), WebAssembly in `wasmi`, and WebAssembly in `wasmtime` (Cranelift native code and
the Pulley interpreter). It also times a script-side loop, a reload, checks that results are identical bit for bit across
runtimes, and runs sandbox probes (an endless loop, a memory bomb, unbounded recursion, a file open, `eval`).

```
# the guests (Rust compiled to wasm and to a DLL; `rustup target add wasm32-unknown-unknown` first)
cd spikes/script/guest-wasm  && cargo build --release --target wasm32-unknown-unknown
cd spikes/script/guest-dylib && cargo build --release

# the host, one feature per runtime so that binary size and build time can be told apart
cd spikes/script
cargo build --release --features rhai,lua,wasmi,wasmtime,dylib      # or one feature at a time
target/release/ss2-spike-script --runtime native
target/release/ss2-spike-script --runtime rhai --probes
target/release/ss2-spike-script --runtime lua --probes
target/release/ss2-spike-script --runtime wasmi --wasm <guest .wasm> --probes
target/release/ss2-spike-script --runtime wasmtime-jit --wasm <guest .wasm> --probes     # or wasmtime-pulley
target/release/ss2-spike-script --runtime dylib --dylib <guest .dll> --probes
target/release/ss2-spike-script --runtime none                    # the memory of the bare process
# options: --no-limits (turn fuel, instruction hook and operation limits off), --batches N
```

`survey/` holds the scripts that measured what the two shipped mods change (the numbers of the section "What the shipped mods actually change" of the RFC). They need your own
install (`SS2_DATA_DIR`) and a built `meta-dump` (`META_DUMP`); they read the mod archives and never write game content anywhere in
the repository. `schemas.py` writes `schemas.json` (the type tables of every data file, base and mods) to the current directory, and
`final.py`, `classify2.py`, `changed.py` read it; `macros.py` reads the archives directly.

Everything here has its own `[workspace]` and `Cargo.lock`: it is **not** part of the main workspace, CI or the repository's
`cargo deny`, nothing in it ships, and no shipped crate depends on it. It uses `unsafe` freely (`libloading`, the memory counter), which
the workspace forbids; that is acceptable for a throwaway and is itself a data point in the RFC. Delete it once T13-004 exists.
