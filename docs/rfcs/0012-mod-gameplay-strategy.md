# RFC 0012: How mods add gameplay: scripting runtime, plugin crates or hand ports

- **Number:** 0012 (`docs/rfcs/0012-mod-gameplay-strategy.md`)
- **Authors:** @Ihor-Ihorevych/agent
- **Status:** in review (proposed by the author; **approval is for the maintainers**, nothing here is decided)
- **Approvers:** owner of `ss2-script` **plus one** other maintainer
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T13-002 (this spike); unblocks T13-004 (runtime integration), T13-005 (Mod API v0), T13-006 (mod-aware netcode), T13-008 (port `InSamnity2`); informs T13-001, T13-003 and `docs/MODDING_POLICY.md` section 4 ("whether a mod may ship its own native module")

## Summary

**Recommendation:** treat the three kinds of mod change differently, and add exactly one new runtime.

1. **Data mods need no runtime.** `Sam2Renovation`-style mods are overlays and parameter files read by classes the engine already has (T13-001, T13-003).
2. **Level scripting is the game's own macro language** (`.mac` programs). The engine needs that interpreter anyway (T01-030, T09); the second shipped mod's 204 macro programs call it with 99.2 % of their actions already in the base vocabulary.
3. **New native behaviour (new entity classes, AI behaviour nodes, HUD elements, new parameters) runs as WebAssembly modules inside `wasmi`**, a pure-Rust interpreter, with fuel (an instruction budget), a memory limit and **no capability the host does not hand over**. The Mod API (T13-005) is a small, versioned set of host imports. Per call it costs about **80 ns** in this spike, against 5 ns for compiled-in code, 225 to 500 ns for Lua and 1.1 to 1.3 microseconds for Rhai, and it adds about **2.0 MB** to the binary and 16 crates.
4. **Not recommended:** native plugin crates for third-party mods (measured: nothing stops an endless loop, no sandbox, unstable ABI), Rhai (15 to 100 times slower than wasmi depending on the work, and its dependency `smartstring` currently fails `cargo deny` for an unmaintained-crate advisory), `wasmtime` as the default (11.4 MB, 111 crates, a JIT that iOS does not allow; its 70 ns buys nothing at this call size). Lua is the credible alternative if mod authors need a text language; it is cheaper to start with and weaker as a sandbox (section "Why wasmi and not Lua").
5. **First-party gameplay (the weapons, enemies and levels of T09) is written in Rust inside the engine**, never as scripts: that is the "hand port" option and it is not a competitor of 3, it is the other half. Whether the port of `InSamnity2` (T13-008) goes into the core or is written as a wasm mod against the API is an **open question for the maintainers**.

This is the output of a time-boxed spike. The measurements come from one Windows machine under the load of other work, from one CPU architecture, with a workload written for this RFC; the recommendation says where that is thin (section "What was not measured").

## Motivation

The roadmap says that a Rust engine **cannot load** `InSamnity2Game.dll` (a 6.1 MB native C++ module) and lists the options: "(a) reproducing its gameplay in Rust or (b) a new mod API" (`docs/roadmap/T13-mods.md`). `docs/MODDING_POLICY.md` leaves open "whether a mod may ship its own native module" and names this task. Four tasks cannot start before the choice (T13-004, T13-005, T13-006, T13-008), and T09 (gameplay) needs to know whether to leave a seam for mods in its entity API.

Two constraints from the rest of the project decide a lot:

- **The netcode is server-authoritative with snapshot replication and client prediction; lockstep was rejected** (`docs/superpowers/specs/2026-11-21-ss2rs-design.md`, alternatives table, RFC 0006, `docs/NETCODE.md`). So mod code does **not** have to give identical results on every machine for the game to stay in sync. It has to run on the **server**, and the client only needs mod code for presentation (HUD, effects) and for whatever it predicts. What it must never do is **harm the machine of a player who joins somebody else's modded session**.
- **The sim-safe crates** (`ss2-sim`, `ss2-physics`, `ss2-net`: no `HashMap`/`HashSet` iteration, no wall clock; enforced by their `clippy.toml`) are where the entity store and the replication live, and a mod runtime will be called from them. A runtime that reads a clock, or whose behaviour depends on a hasher seed, cannot be called from there.

## What the shipped mods actually change (measured)

Source: the retail install's mod archives, read-only. `docs/re/mods.md` already counted entries; this adds **what kind** of change each one is. Method (scripts in `spikes/script/survey/`): every data entry that is a CTSEMETA file (`.ep`, `.rsc`, `.ctl`, `.cb`, `.wld`, ...) was decoded with `tools/meta-dump` (the type table of the file: which classes it uses and which members each class has), and the type tables were compared with those of **all the base archives and patches** (993 distinct struct types). A file is "base schemas only" if every class in it has a member list that some base file also has. A class name that no base file has is "new"; a name that exists with a member list no base file has is "changed". The mod's `.dll` **was not opened** (clean room): everything below about what the DLL does is **inferred from the data it needs**.

| | `Sam2Renovation` (3,194 entries) | `InSamnity2` (3,170 entries, with its update packs) |
|---|---|---|
| Media and presentation (models, textures, particle effects, sounds, materials, names, text) | 2,475 | 2,426 |
| Data read by classes the base data already has (parameters, behaviour tables, menus, worlds: "base schemas only") | **660** | 262 |
| Data using a **changed** class (members added to an existing class) | 7 (menu and HUD layouts) | **235** (167 `.ep`, 49 `.rsc`, 13 `.ctl`, 5 `.sfc`, 1 `.mas`) |
| Data using a **new** class | 46 (40 `.dtn`, 6 `.mch`) | **43** (17 `.ep`, 12 `.cb`, 5 `.wld`, 4 `.rsc`, 3 `.bwh`, 1 `.mah`) |
| Macro programs (`.mac`) | 6 | 204 |
| Game module named in the `.mod` file | stock `Sam2Game` | its own `InSamnity2Game` |

**`Sam2Renovation` is data only.** Its `.mod` names the stock game module. Of its new class names, **6 of 7 are in the base game's class inventory** (`docs/re/`: the destruction effects `CDFX*`, `CDummyHullTemplate`, `CHovercraftJointTemplate`; the base data has no `.dtn` file at all, the mod simply is the first data to use them); one, `CRotateLetterTextEffect` (a text effect in one `.tfx`), is in no spec. The 7 changed layouts are menu and HUD classes the stock module already reads. Nothing in Renovation needs mod code, which is what T13-003 assumes.

**`InSamnity2` is not.** 278 of its data files (8.8 %) need a class or a member that no base data has, so its game module must add native code for them:

- **65 class names** the base data never uses, 60 of which no spec in `docs/re/` names. By role (a reading of the names, not of the DLL): 8 **world entity classes** (each with a `...Properties` twin, 16 types): a multi-path marker, a seated static model (seats), a red-plasma item, a posable model, a target model, a "requiem" text-effect entity, a teleport and a teleport pad; 7 skeleton and posing helper types; 15 **AI behaviour** types (behaviour-tree nodes such as "find a seat and sit", "force an evil puppet's foe", bot movement nodes; and bot skill and weapon parameters, so player-controlled-like **bots**); 11 global parameter types (barriers, per-level macro parameters, static-model tweaks); 5 template-provider types; 4 HUD and menu types (a radar, two kill counters, an opaque in-game menu background); 3 seat and tool-creator types; 2 variant types; 2 types that are in the base inventory (`CHovercraftJoint`, `CTextRenderingEffect3D`).
- **47 existing classes with members added** (none removed except in a few world entity classes), mostly one to six members: `CMiscPupeptParams` (3 members, in 153 files), `SLeggedPuppetParams` (`lpp_bCanMeander` and a character mover handle), `SBaseWeaponParams` (6, alternate fire: `bwp_pgppAltFireProjectileParams`), `SBaseScatterWeaponParams` (13, alternate-fire bullets), `CMCMarkersMovingOrdered` (4, marker progress), `SPuppetEntity` (6, including a handle to the seat it sits on). That is **new parameters read by new native code**: the data cannot say what the code does with them.
- **Macro programs** (`wld-macro.md`): 207 entries (204 distinct paths; three are replaced in update packs) use 105 distinct actions in 3,338 uses. **11 names with 28 uses (0.8 %) are outside the base vocabulary** (`wld-macro-vocab.md`): `StretchModel` 8, `IgnoreAmmoCheck` 7, `RechargeMoveResize` 5, and eight names used once (`StartCeremony`, `DisablePhysics`, `StartCountDown`, `DecreaseCounter`, `SetPatrolingRotateSpeedMultiplier`, and three `mac...` names that are probably macro functions defined in the mod rather than native actions). The other 99.2 % need only the interpreter the base game also needs.

So the amount of **new native behaviour** in the hardest mod is on the order of **a few dozen classes and behaviours plus a few hundred parameter members**, concentrated in entities, AI behaviours and HUD. It is not an open-ended "game logic" surface, which matters for the choice below. (Limit: type tables show what data can express, not what the DLL does beyond it; the DLL may also change existing behaviour with no trace in the data. Only a spec of the DLL, which is a T01 task, could say.)

## The options

| | A. Embedded scripting | B. Native plugin crates | C. Hand port |
|---|---|---|---|
| What a mod is | text or bytecode run by an interpreter inside the engine | a compiled library loaded at run time (`dylib`), or crates compiled into a build | Rust added to the engine or to a fork |
| Candidates measured | **Rhai** 1.27.0, **Lua 5.4.9 via `mlua` 0.12.3**, **WebAssembly in `wasmi` 2.0.3**, **WebAssembly in `wasmtime` 51.0.0** (Cranelift native code, and its Pulley interpreter) | `cdylib` with a C ABI loaded by `libloading` 0.8.9; and the baseline "compiled in" | the same hook as ordinary Rust (the baseline) |
| Not measured | `piccolo` (pure-Rust Lua 0.3.3), `rune`, `boa` (JavaScript), `wren`, `mun`: **not evaluated**, only listed so that nobody thinks they were rejected on evidence | Rust `dylib` with the Rust ABI (not stable across compiler versions; not tried), `abi_stable`, the `stabby` family | |

## The spike

`spikes/script/` (outside the workspace, own `Cargo.lock`, see its README). **One workload in every runtime:** `on_damage(entity: i32, amount: f64) -> f64` is called 10,000 times. Each call asks the host for the entity's armour (a host call back out), multiplies, adds to a **per-entity total kept inside the script** (256 slots), and when a total passes 500 resets it and calls the host's `spawn(kind, slot)` (620 spawns per 10,000 calls). A second function, `busy(n)`, does `n` iterations of `acc = acc * 1.0000001 + i` inside the script, to separate the cost of crossing the boundary from the cost of executing the script. The hook is the same arithmetic in every runtime: compiled Rust (baseline), a Rust `cdylib`, Rust compiled to `wasm32-unknown-unknown` (an **830-byte** module), and a hand-written Rhai and Lua script. All use `f64`.

Machine: Windows 11, Rust 1.99.1, 12th Gen Intel i5-12450HX, release build (`opt-level 3`, thin LTO, `codegen-units 1`, stripped), single thread. **Load caveat:** other agents were building and running on this machine during the whole spike (the CPU load I sampled was about 25 %, and it was higher at times); I could not get a quiet machine. Each runtime was run **three times in three separate processes** and the table gives the **median of 30 batches of 10,000 calls, the range over the three processes** in brackets. Treat differences under about 30 % as noise; the ratios between runtimes (5 to 300 times) are far outside it.

### Speed (limits on: fuel 2,000,000 per call for wasm, a hook every 1,000 VM instructions with a budget of 2,000,000 for Lua, `max_operations` 2,000,000 for Rhai)

| Runtime | per `on_damage` call (ns) | per iteration of the script loop (ns) | start-up: create the runtime and load the code (ms) | reload (ms, median; worst of 5) |
|---|---|---|---|---|
| compiled in (baseline) | 3 to 5 | 1.6 | 0 | 0 |
| native plugin (`cdylib`) | 5 to 6 | 1.6 | 0.3 to 2.2 | 6.3 to 7.0 (worst 12) |
| wasmtime, Cranelift native code | 70 to 81 | 2.0 to 2.2 | 3.7 to 6.0 | 2.4 to 2.9 (worst 3.7) |
| **wasmi** | **80 to 95** | **3.0 to 3.3** | 0.3 to 1.1 | **0.03 to 0.05** (worst 0.15) |
| wasmtime, Pulley interpreter | 241 to 342 | 18.7 to 26.9 | 3.9 to 6.2 | 2.4 to 3.4 (worst 3.9) |
| Lua 5.4 (`mlua`) | 474 to 497 | 18.6 to 31.5 | 0.2 to 0.3 | 0.10 to 0.16 (worst 0.24) |
| Rhai | 1,110 to 1,337 | 289 to 347 | 0.6 to 0.9 | 0.31 to 0.37 (worst 0.55) |

With the limits **off** (`--no-limits`, three runs each): Lua 225 to 276 ns per call and 9.2 to 15.9 ns per loop iteration (the instruction hook costs Lua about half of its speed); wasmi 79 to 106 and 3.0 to 3.3; wasmtime 67 to 69 and 1.7 to 1.8; Pulley 226 to 238 and 14.9 to 15.7; Rhai 1,075 to 1,160 and 278 to 288 (Rhai's operation counter is cheap). **Fuel metering costs wasmi and wasmtime nothing visible.**

How to read it: a call includes **one call from the script to the host** (and, 6 % of the time, a second). Wasm at 70 to 95 ns is dominated by the two boundary crossings, not by the arithmetic: the JIT (70 ns) and the interpreter (80 ns) are within noise of each other at this size, while in a pure loop the JIT is about 1.5 times faster (2.0 against 3.0 ns). The scripting languages pay for interpreting every operation: Rhai is 290 ns per loop iteration, **about 100 times wasmi's 3 ns**.

What that means for the game: a horde of 1,000 entities each calling a hook once per 30 Hz tick is 1,000 calls per tick. At 80 ns that is **0.08 ms per tick, 0.24 % of a 33 ms tick**; Lua 0.5 ms (1.4 %); Rhai 1.2 to 1.3 ms (**3.8 %**). A hook that does 1,000 loop iterations of real work costs 3 microseconds in wasmi and 0.29 ms in Rhai: ten such hooks per tick would take 3 % of the tick in Rhai. The workload is small and invented, **a real AI think is not this**; the point is the ratio, and that no runtime but Rhai is a performance problem at the scale that matters. Phones are slower (not measured).

### Memory, binary size, build cost, dependencies

| Runtime | binary (bytes; delta over the baseline of 192,000) | working set added by creating the runtime (KB) | private bytes at the end, over the bare process (KB) | cold build of its dependencies | crates in its tree (the spike excluded) |
|---|---|---|---|---|---|
| native plugin | 240,640 (+0.05 MB), plus a 102,912-byte plugin DLL | 75 to 85 | +250 to +280 | 3 s | 2 |
| Lua 5.4.9 (`mlua`, vendored C, built by the `cc` crate) | 616,448 (**+0.42 MB**) | 385 to 395 | +130 to +200 (Lua itself reports 27 KB in use) | 22 s | 25, plus a **C compiler** |
| wasmi | 2,158,592 (+1.97 MB) | 570 to 580 | +1,170 to +1,220 | 87 s | **16** |
| Rhai | 3,036,160 (+2.84 MB) | 1,270 to 1,280 | +400 to +430 | 98 to 143 s | 27 |
| wasmtime (Cranelift and Pulley) | 11,591,168 (**+11.4 MB**) | about 4,400 (JIT), about 4,050 (Pulley) | +1,900 to +2,300 (JIT), +2,200 to +2,700 (Pulley) | **2,727 s** | **111** |

The bare process (`--runtime none`) has a working set of 3,824 KB and 628 KB of private bytes. The working set also counts mapped code pages and is noisy (about 50 KB); read it as an order of magnitude. **Build times are single cold builds under load**: the 2,727 s of wasmtime (45 minutes) is an upper bound inflated by the other builds on the machine and by the spike's `codegen-units = 1` and thin LTO; it still says that wasmtime with Cranelift is by far the most expensive thing to compile, which CI feels on every cold cache. Binary sizes are of the spike's executable with the guest code included, stripped.

### Determinism

**Result:** the checksum over all 10,000 results (their `f64` bit patterns) and all 620 spawn calls is the same, `d8cb87acf2549dee`, and the script loop `busy(1000)` returns the same bits (`411e7cf277ecc835`), in **all seven configurations** (compiled in, native plugin, Rhai, Lua, wasmi, wasmtime with Cranelift, wasmtime with Pulley), in the two passes inside one process, after a reload, and in each of three separate processes. So **IEEE-754 addition, multiplication and comparison give the same bits in every runtime on this machine**; the interpreters did not do anything surprising with `f64`.

What that does **not** show: other CPU architectures (aarch64 for Android, Apple silicon); fused multiply-add (Rust does not contract, Cranelift does not by default; LuaJIT-style engines may; unchecked); **transcendental functions** (`sin`, `pow`: Lua's `math` library calls the C runtime of each platform and can differ between platforms; WebAssembly has no such instruction, the guest brings its own implementation, which is why wasm is deterministic by specification apart from the bit pattern of a NaN and the relaxed-SIMD and threads proposals); iteration order of maps inside a runtime (not checked; `wasmi` offers a `deterministic` feature for the Wasm 3.0 deterministic profile and a `prefer-btree-collections` feature, both **not tried**). Because the netcode is server-authoritative this matters for **replays, regression tests and prediction of anything a mod changes in the player's movement**, not for keeping clients in sync.

### Sandbox and limits (each probe was run, the runtime was then asked for a normal call again; every runtime but the native plugin stayed usable)

| Probe | Rhai | Lua | wasmi | wasmtime | native plugin |
|---|---|---|---|---|---|
| **Endless loop** | stopped in 0.9 ms (`Too many operations`) | stopped in 10.6 ms (budget error from the instruction hook) | stopped in 1.4 ms (`all fuel consumed by WebAssembly`) | stopped in 0.1 ms (JIT) and 0.9 ms (Pulley) by fuel (the message I printed was only the first line, `error while executing at wasm backtrace`; I did not print the trap reason) | **not stopped**: still running after 500 ms, and a thread cannot be stopped from outside; the only way out is to end the process |
| **Memory bomb** | stopped by the 100,000-element array limit, but only **after 2,675 ms** (the operation limit did not fire first; I did not investigate why it is slow) | stopped in 42 ms by the 16 MiB limit (`not enough memory`), peak working set 25 MB | `memory.grow` is **refused** at 14 MiB (limit 16 MiB); the guest sees `-1` and returns normally (a trap needs `trap_on_grow_failure`), peak 22 MB | same: refused at 14 MiB, peak 9 MB | n/a: no limit exists |
| **Unbounded recursion** | stopped in 0.1 ms (`Stack overflow`, 64 call levels) | stopped in 40 ms, **by the memory limit** (`not enough memory`) before Lua's own stack limit | stopped in 1 ms (`call stack exhausted`) | trapped in 0.4 to 0.5 ms; the process lives | would overflow the stack of the host: **not run** |
| **Open a file** | no such function (nothing is registered unless the host registers it) | `io` is `nil` because only the `math`, `table`, `string` and `utf8` libraries were loaded; **but `load` (compile a string) is still there** and returned a value | a module that imports `wasi_snapshot_preview1.path_open` **fails to instantiate** (`cannot find definition for import`): the host defined no such import | same (`unknown import ... has not been defined`) | **no restriction**: a native plugin has the rights of the process (not run) |
| **`eval`** | disabled at compile time (`reserved keyword 'eval' is disabled`) | n/a | n/a | n/a | n/a |

The structural difference is **allow-list against block-list**. A WebAssembly guest can call only the functions the host puts in its `Linker`; with none defined it cannot reach a file, a socket or a clock. Lua and Rhai start from a language that has capabilities (`io`, `os`, `load`, `require`, `debug`, `string.rep` memory) and **the host removes them**: one forgotten library or one registered convenience function and the sandbox has a hole. This is the point that matters most for the multiplayer risk below. Not tested: wasm host calls that themselves misbehave, a guest that allocates through the host API, the cost of the epoch or timer interruption (`wasmtime`), the interplay of fuel with the host's own time budget, Lua's `string.rep` memory attack (the memory limit would catch it), and any attempt to **escape** a runtime (these are third-party code bases; `wasmi` has been audited twice according to its README, which I have not verified).

## Comparison

### Decision matrix

`++` strong, `+` good, `~` adequate with work, `-` weak, `--` disqualifying for third-party mods. The notes carry the numbers; a matrix hides how close two cells are.

| Criterion | Lua (`mlua`) | Rhai | **wasmi** | wasmtime | native plugin | compiled in / hand port |
|---|---|---|---|---|---|---|
| Speed at hook size (measured) | `+` 225 to 500 ns | `-` 1.1 to 1.3 µs | `++` 80 ns | `++` 70 ns | `++` 5 ns | `++` 3 to 5 ns |
| Speed of script-side work (ns per loop iteration) | `~` 9 to 30 | `--` ~290 | `+` 3 | `++` 2 (Pulley: `~` 15 to 27) | `++` 1.6 | `++` 1.6 |
| Sandbox model | `-` block-list; instruction hook halves speed; `load` stays | `~` safe by default, block-list for what the host adds; slow memory check | `++` allow-list; fuel and memory limits, no cost seen | `++` same, more knobs (epochs) | `--` none; endless loop cannot be stopped | n/a (it is the engine) |
| Resource limits (CPU, memory, stack) | `~` hook budget, memory limit, no stack limit of its own seen | `~` operations, array/string/map size, call depth | `++` fuel, memory, table, instance, recursion depth | `++` the same | `--` none | n/a |
| Determinism for replays and prediction | `~` doubles are IEEE; libm functions differ per platform | `~` unchecked beyond the loop | `++` specified (NaN bits aside); `deterministic` feature exists, **not tried** | `++` same | `~` same code, but any host-ABI difference leaks | `++` |
| Security and supply chain | `-` C code (Lua 5.4.9) compiled on every platform; `unsafe` throughout | `-` `cargo deny` fails today: `smartstring` is unmaintained (RUSTSEC-2026-0249); would need an `ignore` the owner approves | `+` 16 crates, pure Rust, advisories clean today | `~` 111 crates, large `unsafe` JIT, advisories clean today | `--` full trust; arbitrary code from strangers | n/a |
| Licences against `deny.toml` | `+` MIT; `mlua`, `mlua-sys`, `lua-src` | `+` MIT or Apache-2.0 | `+` MIT or Apache-2.0 | `+` Apache-2.0 with LLVM exception (allowed) | `+` ISC (`libloading`) | n/a |
| Android and iOS | `~` C toolchain for each target; interpreter, no JIT: iOS ok | `+` pure Rust, no JIT | `+` pure Rust, interpreter: iOS ok, `no_std` capable | `-` JIT is not allowed on iOS; Pulley works but is 3 to 4 times slower per call and 9 times in a loop; builds for Android **not tried** | `--` `dlopen` of downloaded code: not allowed on iOS; per-ABI builds on Android | `+` |
| Binary size | `++` +0.42 MB | `~` +2.8 MB | `~` +2.0 MB | `-` +11.4 MB | `++` +0.05 MB | `++` |
| Build and CI cost | `+` 22 s, needs a C compiler on every runner | `~` 98 to 143 s | `~` 87 s | `--` 45 minutes under load | `+` plugin crate per mod, per platform | `++` |
| Mod-author ergonomics | `++` text, hot edit, huge community | `+` Rust-like text, small community | `-` a toolchain (Rust and `wasm32-unknown-unknown`, or C, Zig, AssemblyScript) and a build step; no REPL | `-` same | `-` Rust toolchain, ABI discipline, per-platform builds | `--` needs a fork and a rebuild |
| API stability (what a mod author can rely on) | `+` Lua 5.4 is stable; our host API is what changes | `~` a young language, minor versions have changed syntax | `+` a **binary interface we version** (host imports); the engine can change underneath | `+` same | `--` the Rust ABI is unstable; a C ABI works but every struct is hand-managed | `+` |
| Hot reload (measured) | `++` 0.1 ms | `++` 0.3 ms | `++` 0.03 to 0.15 ms | `+` 2.4 to 3.7 ms | `-` 6 to 12 ms **and the old library stays mapped** (a loaded DLL cannot be overwritten on Windows; the spike loads a copy under a new name) | `--` rebuild |
| Multiplayer: what a hostile server can do to a joining client | `-` depends on every removed library | `~` | `++` nothing it was not given | `++` | `--` runs as the player | n/a (no third-party code) |

### The three options against what the mods need

- **A hand port** is the right answer for everything that is "the game": T09's weapons, enemies, items, and the engine's own classes. It has no sandbox because there is no third party, no runtime cost, no ABI, and it is where the replication schema (RFC 0009) and prediction live. Its limit is **who can write it**: a mod author cannot, and `InSamnity2`'s author is not going to port to our engine. It is also the only way to carry the DLL's behaviour that **only a spec of the DLL can describe**: whichever option is chosen, T13-008 needs T01 specs of what `InSamnity2Game.dll` does, because the data above shows the shape of the change, not its rules.
- **Native plugin crates** are fast and simple for the author who is also an engine developer, and nothing else. The measured facts decide it: no way to stop a loop (500 ms later the thread was still spinning), no memory limit, the full rights of the process, an unstable Rust ABI (so a C ABI with hand-managed structs), a platform build per mod, no `dlopen` of downloaded code on iOS, and a **reload that cannot unload**. A mod that other players download and run on their machine must not be able to do that. Compiled-in plugin crates are the hand port again (first-party code in the build).
- **An embedded runtime** is the only option where a stranger's code can run safely and be updated without rebuilding the game. Among the runtimes measured, **WebAssembly in `wasmi`** is the best fit **for the constraints of this project** (server-authoritative; strangers' code on a client; iOS later; a project that already forbids `unsafe` in its own crates and wants few dependencies), and **Lua** is the best fit for **author friendliness**. They are close enough that the maintainers' weighting decides, and I say so.

### Why wasmi and not Lua, in one paragraph

On the numbers that were measured the two are not far apart for hooks (80 ns against 225 to 500 ns, both below 1.5 % of a tick for a thousand calls), and Lua is smaller (+0.42 MB against +1.97 MB). The reasons for wasmi are not in the speed table: **(1)** the sandbox is an allow-list by construction (a module that asks for a file does not instantiate), whereas Lua's is a list of things the host remembered to remove (`load` was still reachable in the spike's restricted state) and its instruction budget **halved its speed**, which fuel in wasmi did not; **(2)** it is pure Rust, so no C compiler on every CI runner and every Android NDK target, and no C `unsafe` to review; **(3)** the guest ABI is a binary interface we version, so the engine can change its internals without breaking mods, and the **same module can be loaded by wasmi or, on a desktop that wants it, by wasmtime** (the spike ran the same 830-byte module in both) without a mod author noticing; **(4)** determinism is a property of the specification, not of a platform's C library. The costs are real: authors need a toolchain and a build step, there is no text script to edit and reload (though reload itself is 0.05 ms), and **the host API has to be designed as an ABI** (integers, floats and pointers into the guest's memory), which is work T13-005 must do carefully. If the maintainers decide that authors matter more than the sandbox and the toolchain, **Lua with the libraries removed, a memory limit and an instruction budget is a defensible choice**, and everything else in this RFC (macros for levels, hand port for first-party, the API as a versioned host interface, mod hash in the handshake) would stay.

## Multiplayer

All of it follows from "server-authoritative, snapshot replication, no lockstep":

1. **Mod code that changes the simulation runs on the server.** Clients see its effects through snapshots like any other entity (`docs/NETCODE.md`). A hook runs in the server's tick, in a defined phase (`docs/SIMLOOP.md`), and may only change the world through the `World` trait of RFC 0009 (get and set properties by name or slot, spawn, despawn), which records changes for replication. **A mod never touches a socket and never sees a clock**; its time is the tick counter, so the sim-safe rules hold for the *interface* even though the runtime lives outside the sim-safe crates.
2. **New classes are replicated like old ones.** A mod registers its entity classes and properties with replication attributes (RFC 0009: mode, quantization, priority) at load time, **in a deterministic order**, and the schema hash of the class registry already covers them, so two builds that disagree about a mod class refuse each other. T13-006 must put the mod list and its hash in the handshake (it already carries `ModMismatch`, `docs/SESSION.md`), with a deterministic load order.
3. **Presentation hooks run on clients, and that is where the danger is.** A player who joins a modded session must run the mod's client part. The rule that makes this safe: **a server never sends code to a client.** The player must already have the mod, matched by the hash of its descriptor and its `.wasm` bytes. With a native plugin the client would be running a stranger's code as itself; with wasm it runs a stranger's code that can reach only what the client's host API gives it. A malicious host is already an attacker in `docs/THREAT_MODEL_NET.md`; mod code adds a new input to the threat model, and **that document needs a section for it** (follow-up).
4. **Prediction.** If a mod changes what the local player's movement or weapon fire does, the client has to run the same code to predict it, and then the two must agree bit for bit or the player rubber-bands (misprediction metrics, T08-009). This RFC proposes that **v0 of the Mod API does not allow mods to change the player controller or client-predicted weapon firing**; those stay in the core (T07-005, T09-005) and mods change them only through data (parameters). A later version can allow it once the determinism of a pinned runtime version is established across architectures (not measured here).
5. **Budgets are deterministic.** Fuel counts instructions, so a hook that exhausts its budget fails the same way on every run, which a replay needs; a wall-clock timeout would not. Fuel cost tables can change between `wasmi` versions, so **the runtime version is part of the mod API version** and of the replay header. What a mod does when a hook traps (drop that call and log; disable the hook for the entity; disable the mod for the session and tell the players) is a policy decision, not a technical one (open question).
6. **Mixed modded and unmodded peers** are refused by the mod hash, so there is no mixed mode to design.

## Detailed design (what T13-004 and T13-005 would build)

Only the shape, so that the tasks can be sized. Nothing here is implemented, and this RFC adds no dependency to any shipped crate.

- **Crate boundary.** `ss2-script` (it exists and is empty) owns the runtime. It depends on `ss2-sim` for the `World` trait, not the reverse; `ss2-sim` stays free of `wasmi`. A mod runs behind a `ModHost` trait with a mock for tests (the same pattern as RFC 0009's `SimpleWorld`), so gameplay code can call hooks before the real runtime exists. `wasmi` goes into `[workspace.dependencies]` with default features off and the features we use chosen on purpose (`std`; `wat` off; `validate` on; the `deterministic` and `prefer-btree-collections` features to be **measured** in T13-004); `cargo deny check` and `docs/DEPENDENCIES.md` apply as for any dependency.
- **Mod layout.** A mod lists its modules in the existing `.mod` file (`MODULES = ...`), where the retail format names DLLs: `Name.wasm` replaces `Name.dll` and **a `.dll` in a mod stays refused** (`MOD046` already warns; `docs/MODDING_POLICY.md` section 3 forbids it). The descriptor gains an API version (T13-001 decides the key; `REQUIRES_PATCH` is the precedent for our own keys).
- **The host interface** is a handful of imports in a versioned namespace (`ss2_v0`): `world_get_f32(entity, slot) -> f32`, `world_set_f32(...)`, the same for `i32`, `vec3`, text and entity references; `spawn(class, ...)`, `despawn`; `subscribe(event_id)`; `log`; `rand_u32()` drawn from the sim's own deterministic generator (never a clock). The guest exports `mod_init(api_version)`, one function per hook it registered, and `alloc` for host-to-guest buffers. Every function takes and returns `i32`, `i64`, `f32`, `f64` and offsets into the guest's memory; no struct crosses the boundary, so there is no layout to get wrong. The hook table is the part T13-005 designs with T09's entity API, **after** T09-001 exists.
- **Limits per mod, in configuration with safe defaults:** fuel per hook call, fuel per tick, linear memory (the spike used 16 MiB), table size, call depth, instances per mod. **Fuel is the unit of CPU**, charged to the mod; exhausting it traps the call.
- **Reload:** instantiate the new module next to the old, call `mod_init`, swap at a tick boundary; state that must survive lives in host properties (the world), not in the guest's memory. (The spike measured only the cost of re-creating the engine and instance, 0.03 to 0.15 ms for a tiny module; **the cost for a real module, and the state migration, are not measured**.)
- **Level scripts** are not wasm: the macro interpreter in the core (T01-030, T09). The Mod API adds a way to register **new macro actions** implemented in a mod (the 11 names above), so the macro language is the extension point for levels and the wasm modules for classes.

## Risks

| Risk | Likelihood and effect | Mitigation |
|---|---|---|
| The host API design (an ABI) is harder than the runtime choice and gets it wrong | likely; a bad v0 is paid for by every mod | build it with T09's real entity API (T13-005 depends on it); keep v0 small; version the namespace; port one real class (a teleport) as the dogfood before freezing |
| Authors do not come because they need a Rust toolchain | medium; the mod scene may stay at data mods | ship a template crate and a build script; allow AssemblyScript or C; keep the Lua alternative open (section "Why wasmi and not Lua") |
| `wasmi` is pre-3.0 and its API or fuel costs change | medium | pin the version in the mod API version; the `ModHost` trait is the seam; `wasmtime` loads the same modules if `wasmi` is abandoned |
| `InSamnity2`'s behaviour cannot be recovered without reading its DLL | **certain** for the rules behind the 47 changed classes and 65 new ones | T01 specs of the DLL (not read by anyone yet: `docs/re/*` say "InSamnity2Game.dll was not read" in several places); T13-008 stays `XL` and must be split; **the author of this RFC must not implement what it specs** (clean room) |
| Our measurement is too small to carry the claim "wasmi is fast enough" | low for hooks, **unknown for AI** | repeat with a real behaviour node (T09-010) before T13-004 commits to budgets |
| A trap or a slow hook in a mod stalls the server | medium | fuel per tick; a policy for repeat offenders; mods only run in defined phases |
| `smartstring`/Rhai, if the maintainers pick Rhai after all | known today | an `ignore` with a revisit condition (`docs/DEPENDENCIES.md`) needs the owner's approval; `compact_str` or `smol_str` are the advisory's suggestions |

## Compatibility and migration

- **No shipped crate changes in this task.** The spike is `spikes/script/` with its own `[workspace]`, `Cargo.lock` and three guest crates; it is not in CI or `cargo deny`. Its dependencies were checked by hand: `cargo deny --all-features check` on the spike's own manifest with the repository's `deny.toml` gives **licenses ok, sources ok, bans ok** and **one advisory failure, `smartstring` 1.0.1 (RUSTSEC-2026-0249: unmaintained, repository archived 2026-05-03), reached only through Rhai**. `wasmi`, `wasmtime`, `mlua` and `libloading` trees raised none today. This says nothing about tomorrow: the weekly advisory job exists for that.
- **T13-001** (loading semantics) needs nothing from this RFC except that `.mod` may list `*.wasm` modules and that the descriptor carries an API version.
- **T13-003** (Renovation) is unaffected: data only.
- **T13-004** (runtime integration): `wasmi` in `ss2-script`, the `ModHost` trait and mock, limits, reload, error isolation, as in "Detailed design"; its first deliverable is a real measurement with a real hook.
- **T13-005** (Mod API v0, `XL` and marked "split"): proposed split: (a) the ABI and the import set; (b) class registration and replication attributes; (c) events and macro-action registration; (d) UI hooks; each with a mock. It depends on T09's stable entity API.
- **T13-006** (mod-aware netcode): the mod hash covers descriptor and `.wasm` bytes; deterministic load order; a reason string for `ModMismatch`.
- **T13-008** (`InSamnity2`): see the open questions; a precondition is T01 specs of its DLL.
- `docs/MODDING_POLICY.md` section 4 can then answer its open sentence: **a mod may ship its own WebAssembly module; it may never ship a native module.** That text is the maintainers' to change; I have not touched it.
- **If this RFC is rejected:** nothing to undo; delete `spikes/script/`.

## Alternatives considered

- **Lua through `mlua`** (above): the strongest alternative; chosen against on sandbox shape, C toolchain and instruction-hook cost, not on speed. The vendored C build worked first time on Windows (`cc` with MSVC); Android and iOS builds were not tried.
- **Rhai:** pure Rust and safe by default, but 2.5 times slower than Lua per hook call and 15 times slower in the script loop (about 15 and 100 times slower than wasmi), **a first-class dependency with an open unmaintained-crate advisory**, and a language nobody knows. Its memory limit took 2.7 s to trip in the probe.
- **wasmtime as the engine:** the fastest wasm runtime in the spike in loops (2.0 against 3.0 ns), identical results, and a richer feature set (epochs, component model). Against: 11.4 MB, 111 crates, a 45-minute cold build under load, a JIT that cannot run on iOS, and no visible gain at hook size. It **remains the escape hatch** because it loads the same modules; a desktop-only "fast path" is a later, measured decision.
- **Native plugin crates (`dylib`):** section "The three options against what the mods need". `abi_stable`/`stabby`-style stable Rust ABIs were **not evaluated**.
- **Hand port of everything, no mod API:** viable for `InSamnity2` alone, and for T09 it is the plan. It leaves the next mod author with nothing, and the project's goal G6 is "support the shipped mods **eventually**", with the roadmap already listing a Mod API (T13-005). A hand port also needs the DLL spec first.
- **Only the macro language, no native extension:** covers 99.2 % of `InSamnity2`'s macro actions and none of its new classes. It might be enough for a different, smaller set of mods; the recommendation keeps it as the level layer.
- **Piccolo (pure-Rust Lua), `rune`, JavaScript engines, `mun`:** not evaluated.
- **Out-of-process mods** (a child process over a pipe; stronger isolation, a context switch per call, a nightmare on iOS): not measured. Mentioned because it is the other way to sandbox native code.

## What was not measured

- Any machine other than one Windows x86-64 laptop **under shared load**; no aarch64, no Android or iOS build or run (no NDK here; the claim about iOS and JIT is from the platform's public rules, not from a test), no Linux or macOS.
- A real mod: the workload is invented, tiny and arithmetic; no AI, no physics, no strings, no entity lists, no allocation inside the guest. The cost of **moving data** between host and guest (strings, vectors) is not in the numbers.
- Memory growth over time, GC pauses (Lua, Rhai) and their effect on tick time; peaks above 16 MiB; many mods at once; the cost of instantiating a larger module (the wasm module is 830 bytes; **wasmtime's 2.4 to 3.7 ms reload is the Cranelift compile of that small module and will grow with the module**, while `wasmi` compiles lazily).
- `wasmi`'s `deterministic` and `prefer-btree-collections` features, its `no_std` build, the Wasm component model, WASI.
- Whether any runtime's internals use hash iteration that could affect results (only the one workload's results were compared).
- Attempts to escape any sandbox; the quality of each project's maintenance (I looked at licences and the RustSec database, not at release history or bus factor); the `unsafe` line counts of the dependencies.
- What the DLL of `InSamnity2` does: **not read** (clean room). The section "What the shipped mods actually change" counts what its data needs.
- The classification of the shipped mods is by type tables compared with the base archives: a class that the base DLL has but that no base data uses looks "new" (`CDFX*` is the example found), so the InSamnity2 numbers may **over**-count the code the DLL adds, and a change of behaviour without any data trace is invisible.

## Open questions

1. **Which runtime: `wasmi` (my recommendation) or Lua?** It trades sandbox shape, no C toolchain and a versioned binary interface against author friendliness and size. *Decides:* the owner of `ss2-script` with the approving maintainer, ideally after asking the authors of Renovation and InSamnity! 2 (T13-010b already plans a review by them) which they could write.
2. **Does the `InSamnity2` port go into the core or into a wasm mod?** In the core it is simple and fast but it makes the engine carry one mod's gameplay (seats, bots, teleports); as a mod it is the best test of the API but depends on T13-005 and T09. *Decides:* the maintainers; it changes the order of T13-005 and T13-008.
3. **May v0 mods change client-predicted behaviour?** I propose no (section "Multiplayer", point 4). *Decides:* the owner of T08 prediction.
4. **What happens when a mod's hook traps or runs out of fuel in a live session?** *Decides:* the maintainers (it is a game-design and moderation policy).
5. **May a server push a mod to a client?** I propose never. *Decides:* the maintainers; the threat model gets a section either way.
6. **Where does the mod hash come from, and does it cover the runtime version?** *Decides:* T08-006 and T13-006 owners.
7. **Should `docs/MODDING_POLICY.md` say "no native modules, wasm allowed"?** *Decides:* the maintainers who own the policy.
8. **Follow-up measurements before T13-004 commits to numbers** (proposed as T13-002b): a Rust-compiled hook with real work, string and vector passing, aarch64 and Android builds, `wasmi`'s deterministic profile, a Lua run with the C `math` library compared across platforms, a quiet machine.
