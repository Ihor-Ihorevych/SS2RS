# Main loop and time: step, timers, pause, time scale

- **Roadmap task:** T01-022
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** that the original has **no fixed tick rate** (one variable-length simulation step per rendered frame, 1 ms to 100 ms), for the time unit (seconds × 2³² in a signed 64-bit integer), for the order of the phases of one frame and of one step, for the pause flags and their rules, and for the time-scale variable; **medium** for the think queue (timers), for the physics sub-step count and for the dedicated-server loop details; **low / open** for everything that needs the running game (achieved rates, FPU precision effects), for network time and for the macro `Wait` re-test period.

Prerequisites: `console-config.md` and `console-variables.md` (the console variables named here), `wld-root.md` (`Time` in worlds, the state manager), `physics.md` (what the physics middleware is), `classes.md` (no RTTI: names below are exported function names or strings).

## Summary

**There is no tick rate.** The original does not run its world at a fixed frequency. Every pass of the application loop calls the game's frame function once; that function advances the world by **at most one simulation step**, whose length is the **real time since the previous frame** (times a time-scale factor), clamped to **0.1 s at most**; a frame that would give less than **0.001 s** does not step at all and its time is carried to the next frame. So the step is **variable, between 1 ms and 100 ms**; below 10 frames per second the game runs in slow motion (time is dropped, never caught up); above 1000 frames per second several frames share one step. Rendering draws the state of the last step (no interpolation between steps). There is no difference in kind between single player, a hosted game and a dedicated server: each machine steps once per loop pass; the dedicated server sleeps to hold its loop at **60 passes per second** (`sam_iDedicatedFPS`), a client or single-player game is limited only by the graphics frame limiter (**10,000** fps while the window is active, **20** while inactive) and the display's vertical sync setting.

Time is a `Time` value: a **signed 64-bit integer count of 2⁻³² s** (seconds × 4,294,967,296). Game code passes durations as `float` seconds and converts. A **think queue** (the state manager) schedules per-entity callbacks at absolute `Time` values; they run in the first step whose time is **at or after** the due time. **Pause** is three flags on the simulation (global pause, requested pause, local pause); a paused simulation's time stops advancing, and no catch-up happens on resume. The menu-type **local pause only stops time in a single-player (local) session**; in any network session (including a hosted or co-op game) the menus do not pause the world. **Slow motion** is one float, `sim_fRealTimeFactor` (default 1.0), that multiplies the real time that feeds the step.

This page answers roadmap T01-022 from the binaries `Core.dll`, `Engine.dll`, `Sam2Game.dll`, `Sam2.exe`, `DedicatedServer.exe` and `GfxD3D.dll` (static reading only) and from the data. **Measured frame timing needs the running game and is parked in T01-084.**

## Evidence

- **Binaries read.** Loose files of the retail install's `Bin` folder (sizes in bytes): `Core.dll` 794,624; `Engine.dll` 3,493,888; `Sam2Game.dll` 5,550,080; `Sam2.exe` 28,160; `DedicatedServer.exe` 18,432; `GfxD3D.dll` 81,920. Their PE time stamps are not a reliable build date (they read 2026 and 2025), so the build is identified by these sizes only. Read statically with Ghidra 12.2 (headless, read-only); the notes stayed outside the repository. Not read: `GfxOGL.dll`, the software renderer, `EngineGui.dll` (editor and tool shell), `InSamnity2Game.dll` (a mod's game DLL), the Xbox pieces.
- **How the pieces were found.** The DLLs export their C++ functions with decorated names, so the simulation, pause, time and network functions are named in the export tables (`CSimulation::Step`, `AdvanceCursorTime`, `simPause`, `timUptimeNow`, …); the console-variable names come from the module declaration strings. Callers were followed through cross references to the imported Windows APIs (`QueryPerformanceCounter`, `QueryPerformanceFrequency`, `Sleep`, `PeekMessage`, `GetMessage`, `SetTimer`, `WaitForSingleObject`, `GetSystemTimeAsFileTime`) and to the exported functions. Numbers are the initial values in the data sections or constants in the code; no number is a guess.
- **Cross-checks (two independent readings).** (1) Time unit: the constant 4,294,967,296 is used by the float-to-`Time` constructor, by `timTicksToFloat`, by `timGetPhase` and by the start-up code in `Core.dll`, and by the time-to-float helper that `Engine.dll` uses for the step, and the `Time` values stored in the 59 worlds are whole multiples of 2³² in 159,012 of 164,223 cases (`wld-root.md`, script `tools/re/wld_root_verify.py`). (2) Variable step: `Step` reads the real clock through `AdvanceCursorTime` and clamps with `sim_fStepMin` and `sim_fStepMax`; the physics asks for `simGetStep` to size its sub-steps; the player input code multiplies its look and movement deltas by `simGetStep`. Three independent consumers see a step that is not constant. (3) Defaults of the variables: initial value in the data section **and** the clamps inside `Step` (0.001 and 0.1) agree. (4) Pause rule: the single-player condition appears in `AdvanceCursorTime` and, separately, in `LocalIsPaused`; the interface classes' `IsLocal` answers were read for the three network interface kinds.
- **Data.** `python -I tools/re/mainloop_verify.py <install>` reads the 4 configuration scripts of the install (`autoexec.cfg`, `autoexec_xbox.cfg`, `SafeMode.cfg`, `user.cfg`; there is no `boot.cfg` in the install) and finds **0 lines** that set `sim_*`, `phy_*`, `net_bClientsMayPause` or `sam_iDedicatedFPS`, and only the `SafeMode.cfg` lines `gfx_iFinish = 0`, `gfx_iWaitVSyncs = 0`, `gfx_iMaxFPSActive = 10000`, `gfx_iMaxFPSInactive = 20` (equal to the code defaults for the two FPS caps). So the defaults below are what an unmodified install runs with, apart from a player's own `Sam2.ini` (not in the install).
- **Limits of static analysis.** Nothing here was measured at run time: achieved frame rates, the real distribution of step lengths, the effect of the floating-point precision mode set by the graphics driver, thread timing and network round trips are Open questions (T01-084, T01-085). The decompiler's reading of a few functions is partial (virtual calls through tables of unnamed members); where a rule rests on one function only it is marked `medium`.

## Layout / rules

### 1. Time representation (timers as game code sees them)

| Item | Value | Confidence |
|---|---|---|
| `Time` | signed 64-bit integer, **unit 2⁻³² s** (≈ 0.2328 ns). Range ±2³¹ s (about ±68 years). Stored as `SQUAD` in worlds | high |
| float → `Time` | `round(seconds × 4,294,967,296)` with round-to-nearest(-even) on the `float` value | high |
| `Time` → float | `(float)(ticks / 4,294,967,296)` computed in wider precision, then stored as `float` | high |
| `tim_tmQuantum` | the smallest `Time` step: **1 tick** | high |
| `timGetPhase(t, period)` | `(t mod period) / period` in 0..1, with `period` converted to ticks first; `t` is a `Time` | high |
| "uptime" (`timUptimeNow`) | the Windows performance counter divided by its frequency, plus a start offset, in ticks; used for everything that is **real time** (frame limiter, network time-outs, GUI animation, the step's real-time reference) | high |
| engine uptime (`timGetEngineUptime`) | the system file time minus the time at start-up, in 100 ns units converted to ticks; for log time stamps only | medium |
| wrap or overflow | none in practice: the 64-bit count lasts 68 years. Network packets carry a **32-bit** time stamp in units of **2⁻¹⁰ s (0.9765625 ms)**, wrapping after 4,194,304 s (48.5 days); the receiver accumulates the **difference** of consecutive stamps, so a wrap is harmless | high (unit and size), medium (use) |
| simulation clock | `Now()` of the simulation: `Time`, 0 at the start of a fresh world, restored from the saved game when one is loaded; **never reset during play** | high |
| last step length | `GetStep()` / `simGetStep()`: `float` seconds of the most recent step; 0 after a world start, **0.001** after a saved game was loaded, until the first step runs | high |

Game durations are **seconds as `float`** everywhere the data shows units: `3 sec` in macro scripts, freeze time 60 s, power-up durations; animations are stored as `frames per second` (`ans.md`, 30.0 in 709 of 852 animation sets), and animations are sampled at the **simulation clock** (see 5).

### 2. The main loop: phases of one frame

The executable `Sam2.exe` is a thin shell. After start-up (configuration scripts `boot.cfg`, `autoexec.cfg`, `user.cfg` are read, then the saved variables file, then the command line; window, graphics, sound, network start-up; a font; the project instance is created) it runs this loop until the game's frame function returns 0:

1. **System update** (`sysUpdate`): the Windows message pump (`PeekMessage`; a blocking `GetMessage` is used only in the wait-for-message variant) and the window callbacks. Confidence high that this is first; medium for the exact content.
2. **Per-frame callbacks** (`pfuCallPerFrameCallbacks`): functions modules registered (the time module registers one; the rest are not enumerated).
3. **The game's frame function** (a virtual function of the project instance; returns 0 to quit). Inside, in this order (high):
   1. frame-limiter set-up: `gfxLimitFrameRate(0)` (see 3);
   2. a virtual "enter" hook that makes this project instance's simulation and network interface the current ones and sets the animation clock to its `Now` (called here and again just before the simulation step);
   3. world loading bookkeeping (waits for the resource loader when a local game is starting; shows a failure message when the world did not load);
   4. **input**: exclusive input mode is started or stopped depending on window focus and an overlay flag; device values are polled; the key bindings are polled and the player's and the menu's binding sets are each given their values;
   5. the **pause decision** (see 6): the local pause flag is set or cleared;
   6. **network receive phase**: `netStep` with sending disallowed (`ser_bAllowSend = 0`), executed when the session is not remote or is connected;
   7. the **simulation step** (`CSimulation::Step`, see 4), only when no project-instance "blocked" flag is set (meaning not resolved), the simulation is **started**, and the session is not remote or is connected;
   8. **network send phase**: `netStep` again with sending allowed (`ser_bAllowSend = 1`);
   9. housekeeping: UI flags, the dedicated server's player-count check (see 6 and 8), a final hook.
4. **Render** (second virtual function): view rendering, HUD, the sound/movie/vibration synchronisation lists are updated with the current time **only when the simulation is neither paused nor locally paused**, the text `PAUSED` is drawn when the **global** pause flag is set (not for the local pause; a user setting switches it), a ping read-out for clients, debug overlays.
5. **Screenshot hook, then `gfxSwapBuffers`** (present; the frame limiter and the vertical-sync wait happen inside the back end, see 3), then `bmkTick` (benchmark tick).

So the per-frame order is **input → network receive → simulation step → network send → render → present/limit**. Sound mixing and music run in their own module (not part of this loop); only the positional "sync lists" are driven from the render phase. Confidence **high** for the order, **medium** for what housekeeping does.

### 3. Real-time clock, frame limiter, vertical sync

- The real clock is the performance counter (`QueryPerformanceCounter`/`Frequency`); a CPU-cycle counter is calibrated against it only for the profiler. Resolution is that of the counter (sub-microsecond on a modern PC); not measured.
- `gfxLimitFrameRate(n)` sets the minimum frame period to `1 / fps`, with `fps` = the active-window cap `gfx_iMaxFPSActive` (default **10000**, minimum 5) or the inactive-window cap `gfx_iMaxFPSInactive` (default **20**, minimum 5); a non-zero argument `n` lowers the cap to at most `n`. The frame function uses 0 (the variables only); the **menu frame uses 33**; a loading/UI loop uses 20 (medium: callers identified, purposes inferred). The period is stored and cleared by `gfxSwapBuffers` after every present.
- The Direct3D back end applies the period **before presenting**: it reads the performance counter, and if less than the period has passed since the previous present it `Sleep`s for the remainder minus a small safety margin (starts at 1 ms, grows if a sleep overshoots) and then **spins on the counter** until the deadline. With vertical sync on, the target is rounded to a multiple of the display refresh period. `gfx_iWaitVSyncs` is clamped to **0..4** (default 0; `SafeMode.cfg` also 0); values 17 to 20 are accepted by the back end as 1 to 4 (meaning not established). `gfx_iFinish` is clamped to 0..3 (default 0; where a GPU "finish" is inserted). The OpenGL back end uses the same variable and the same sleep function (not read).
- The `Sleep` granularity of Windows (about 1 ms to 15.6 ms) makes the real period of a limited loop depend on the system timer; measuring it is T01-084.

### 4. The simulation step (`CSimulation::Step`)

State kept by a simulation (names are ours where the binary has none): the **clock** `Now` (a `Time`), a **cursor** (a `Time`, the real-time-driven target the clock chases), a **real-time reference** (the uptime at the previous call), the **last step** (a `float`), and the **global-pause**, **pause-requested** and **local-pause** flags and a **client-unpause permission** flag (set elsewhere; probably fed by `net_bClientsMayPause`, not resolved).

Per call, in this order:

1. The world is made current and `HandleWorldChanges` runs (it stops the old world, starts loading the next one, starts the new world when loading finished). If it reports that no world is ready, the call **suspends the sound/movie/vibration sync lists and returns**: no time passes.
2. The console variables are **clamped and written back**: `sim_fStepMin` into 0.001..0.1; `sim_fStepMax` into `sim_fStepMin`..0.1; `sim_fSyncRate` to ≥ 0; `sim_fRealTimeFactor` to ≥ 0 (a negative value becomes 0). So **0.1 s is a hard ceiling for the step and 0.001 s a hard floor**; the defaults are **`sim_fStepMin` = 0.001, `sim_fStepMax` = 0.1, `sim_fSyncRate` = 0, `sim_fRealTimeFactor` = 1.0**.
3. **`AdvanceCursorTime`** computes the amount `delta` and adds it to the cursor:
   - if the simulation is **globally paused**, `delta` = 0;
   - else if the **local pause** is set **and** (there is no network interface, or the interface is the **local** kind) `delta` = 0;
   - else if `sim_fSyncRate` > 0, `delta` = `sim_fRealTimeFactor` × (1 / `sim_fSyncRate`) (a **fixed-length step per call**, independent of the real time; default off);
   - else `delta` = `sim_fRealTimeFactor` × (uptime now − real-time reference), in seconds;
   - `delta` is clamped to 0..10,000 s, converted to a `Time` and added to the cursor; **the real-time reference is set to the uptime now in every case** (also when paused), so a pause leaves no real time to be caught up on resume.
4. `step` = (cursor − `Now`) as float seconds; `step` = min(`step`, `sim_fStepMax`); `step` = max(`step`, 0).
5. If **`step` ≥ `sim_fStepMin`**: the last step is set to `step`; `Now` = `Now` + `Time(step)`; the **cursor is set to the new `Now`** (any real time above `sim_fStepMax` is discarded); the animation clock is set to `Now`; if a demo is being recorded the new time is written to it; then the world is advanced (see 5). If `step` < `sim_fStepMin` **nothing happens**: the cursor keeps its fraction and the next frame adds to it.
6. A "stepping" flag is set during 5 and cleared afterwards.

So per call there are **0 or 1 steps, never more**; no catch-up exists. Consequences: at 60 fps the step is the frame time (about 16.7 ms); at 8 fps each step is 100 ms and the game runs at 0.8 of real time (125 ms of real time give 100 ms of game time); at 2,000 fps pairs of frames share a step of about 1 ms.

`JogForward(seconds)` (the debug single-step) adds `Time(max(seconds, 0))` to the cursor; while paused, the next step is that long (the game's debug key uses **0.05 s**). `SetNow(t)` (loading a saved game, demo playback) sets the clock, the cursor and the real-time reference together and sets the last step to **0.001**; `ResetTime` (a fresh world) sets the clock to 0 and the last step to 0.

### 5. What one step does (order inside the world)

After `Now` has advanced (this order is from the code and from the profiler section names the code uses: **medium**, one reading each):

1. **Step functions.** Entities that registered for per-step calls are sorted every step (valid entries first, then ascending by a small integer priority) and their per-step virtual function is called in that order (an entity's `OnStep`, for example `CWorldInfoEntity::OnStep`). Entries whose entity no longer exists are dropped. Profiler label "step functions" inside "AI".
2. **Think functions** (the state manager, see 7): the state manager's clock is set to `Now` and every entry due at or before `Now` is run.
3. **Physics** (profiler label "Physics"): body motion handlers, collision detection, contact joints, the solver in **sub-steps**, the integrator, sleeping decisions, post-step hooks (see below).

Physics sub-steps: the number is **`ceil(step / phy_fSubstepSize)`** with `phy_fSubstepSize` = **0.01 s** (default), each sub-step lasting `step / n`; `phy_iSolverSubsteps` (initial value 10) is **overwritten every step** with that number. There is one more case in which it is exactly 1 (a caller passes a flag; not resolved). So a 16.7 ms step uses 2 sub-steps of 8.3 ms, and the longest step uses 10 or 11 (see Open questions). `phy_fSleepThresholdBaseFPS` (default 90) scales the sleep thresholds by `step`: the factor is `clamp(90 × step, 0.1, 1.0)`.

Animation time is the **simulation clock**: the engine's animation clock (`aniSetTime`) is set to `Now` at every step and by the project instance's GUI hook; the GUI frame (menus) sets it to the **uptime** for its own drawing and restores it. Hence world animations, effects driven by animation time and anything using `simNow()` stop with the pause; GUI animation does not.

### 6. Pause semantics

Flags and functions (all exported):

| Function | Effect | Confidence |
|---|---|---|
| `simPause()` / `CSimulation::Pause` | global pause := 1; the sound/movie/vibration sync lists are suspended | high |
| `simUnPause()` / `UnPause` | global pause := 0, **but on a client only when the client-unpause permission flag is set**; on a server or a local session always | high |
| `simRequestPause()` / `RequestPause` | pause-requested := 1; **on a non-client** also global pause := 1 and sync lists suspended; on a client only the request is recorded | high |
| `simRequestUnPause()` / `RequestUnPause` | pause-requested := 0; on a non-client global pause := 0; on a client nothing else | high |
| `LocalPause()` | local pause := 1; sync lists suspended **only if** there is no network interface or it is the local kind | high |
| `LocalUnPause()` | local pause := 0 | high |
| `LocalIsPaused()` | local pause **and** (no network interface or local interface) | high |
| `IsPaused()`, `IsPauseRequested()` | the two flags | high |

**Who sets them (high):**

- **The frame function, every frame** (local pause): it calls `LocalPause` when a menu or overlay layer owns the input (the GUI layer reports that a menu keeps the input), or when an overlay flag of the project instance is set (probably the console; not resolved), or when the **player's Netricsa screen or weapons menu** is active; otherwise `LocalUnPause`. So opening a menu pauses and closing it resumes with no other code.
- **The debug key handler** (reached only in a debug configuration; its gating is unresolved): toggles with `simRequestPause`/`simRequestUnPause`, and a repeat-capable key calls `simJogForward(0.05)`.
- **The dedicated server** (see 8): `simPause` when no player is connected, `simUnPause` when the first one joins (log lines "Pausing server - no players", "Unpausing server - player joining", "Resetting server - all players left").
- **Scripts**: the world-info entity exports `PauseSimulation` (calls `simPause`). **None of the 59 shipped worlds uses an action of that name** (the vocabulary lists in `level-logic.md` have none).
- **Clients in a network game**: the server's handler of a session-parameter message pauses or resumes its simulation on a client's request **only if `net_bClientsMayPause`** (default **0**) is set. The shape of the message and the reply path are not read (T01-085).

**What stops (from `AdvanceCursorTime` and the render phase):** the simulation clock (so all entities, scripts, the think queue, physics, world animation, anything on `Now`), the positional sound/movie/vibration sync lists (suspended, not updated by the render phase). **What keeps running:** the application loop, input polling, the GUI and menus (real-time clock), rendering (with the world frozen), the network (`netStep` runs twice every frame regardless; time-outs use the real clock), the music crossfade timers that use the uptime (not read; medium). Particles and effects: unknown (T01-074); a sound's own playback (the sound module's `sfxPauseSound`): who calls it was not found in the code read (T01-086).

**Single player versus co-op (high):** the **local pause** (menus, Netricsa, weapon wheel) freezes time only in a **local** session (no interface, or the interface kind whose `IsLocal` answer is true; the single-player interface is the only kind that answers true). A hosted game, a dedicated server and a client use other interface kinds, whose `IsLocal` answer is false, so **the menus never pause a network game**, including a co-op campaign. The **global pause** works in every kind of session, subject to the client rule above.

**Resume (high):** no catch-up (the real-time reference moves every frame); the first step after the pause is the normal length of that frame (clamped to 0.1 s). The sync lists resume updating at the next render.

### 7. Timers: the think queue and the other time facilities

- **State manager (think queue).** One per world. Each entry is `(entity handle, due Time, state/function ids, a few flags)`. API: `SetNextThink(handle, seconds, state id)` (the entity method `CEntity::SetNextThink(float, IDENT)` forwards to it), `SetThinkAfter(handle, struct, seconds, …)`, `SetThinkAt(handle, struct, absolute Time, …)`, `ClearNextThink(handle)`, `Sleep_t` (a script's `Sleep`; it re-arms the entity's think with a delay and keeps its state). **At most one entry per entity** (the lookup is by the entity's handle): setting a new one removes the entity's old entry first.
- **Due time** = the **state manager's clock** + `Time(seconds)`. The clock is set to `Now` just before the think functions run in a step; **a call made from a step function (which runs before that) therefore measures from the previous step's `Now`**, one step stale; a call from inside a think measures from this step's `Now` (high on the order, medium on the consequence).
- **Run rule:** while the earliest entry's due time is **less than or equal to** the clock, it is removed and run (a script state function in the virtual machine, or a generic function call); then the queue is examined again, so a think that re-arms itself with a delay of 0 (or less than the clock) runs again **in the same step**. No per-step limit was found. Timers are therefore **never early** and are late by up to one step (1 to 100 ms, typically one frame). A think that is due while the game is paused simply waits (the clock does not move).
- **Representation.** Delays are `float` seconds, due times and the clock `Time`. The queue is saved in worlds (an entry list of under 50 entries in each of the 59 worlds, `wld-root.md`), so due times are in the simulation's own clock.
- **Macro scripts** (`.mac`, `Wait`, `Wait 3 sec`): the macro interpreter's `OnStep` runs all its threads (who calls it each step was not resolved; probably the macro entity's per-step function); **it does not run on a remote client** (`netIsRemote` → nothing), so scripts run on the server or locally. Each time it runs, every thread executes until it blocks again; a `Wait (condition)` thread can therefore be resumed at the earliest on the next step. A recursion limit of 100 exists. The metadata names a macro member `me_fWaitPeriod` ("Wait period") and a thread flag `mt_bBlockedByWait`; whether a wait condition is re-tested at every step or only once per wait period is **not established** (T01-069).
- **World-info timer** (`ActivateTimer`, `StopTimer`, event `TimerFinished`): stored as a start `Time`, a duration and a passed time in `float` seconds (`wld-entity-schema.md`); the same clock (`en_tmTimerStarted` is a `Time`). Resolution is one step.
- **Mechanism motion blending:** `CMechanismMotionHandler::GetLerpRatio` = `clamp((animation clock − the handler's start time) × 5, 0, 1)`: a **0.2 s** linear blend, in animation (simulation) time.
- **View-origin smoothing:** `CPlayerPuppetEntity::UpdateViewOriginInterpolator`: when the view height changes, the offset decays linearly to 0 in **0.1 s** of simulation time (`offset = (old − new) × (1 − 10 × elapsed)` while elapsed < 0.1); details with the camera in T01-073.
- **Windows timers** (`sysCreateTimer`, `SetTimer`) exist for the window layer only; game logic does not use them.

### 8. The dedicated-server loop

`DedicatedServer.exe` has the same loop shape as `Sam2.exe` without rendering: system update → per-frame callbacks → the game's frame function (the same one: step, network receive/send) → a **pacing sleep**. Pacing: it measures the time since the end of the previous pass with the performance counter and, if shorter than `1 / sam_iDedicatedFPS`, calls `timSleep` for the rest. **`sam_iDedicatedFPS` defaults to 60**; while a world is **loading** the rate used is **33** (**high** for the numbers 60 and 33 in the pacing code; **medium** for their binding to the variable `sam_iDedicatedFPS`, which rests on the declaration order and the integer type). `timSleep` takes seconds and calls `Sleep`, so the real period is quantised by the system timer. Because the step is the real pass time, a server holding 60 passes per second steps about 16.7 ms (about 15.6 ms or 31.2 ms when the system timer is coarse; not measured). The dedicated server **pauses the simulation when no player is connected** and resets the world when all players leave (see 6). It does not call the graphics frame limiter (it has no window canvas).

### 9. Time scale and slow motion

- **One variable:** `sim_fRealTimeFactor` (float, default **1.0**, ≥ 0 enforced every step). It multiplies the real time that feeds `AdvanceCursorTime`; so it scales **everything on the simulation clock**: AI, physics, world animation, think timers, scripts; not the GUI, the frame limiter, network time-outs, nor the real-time reference. The `sim_fStepMax` clamp still applies after scaling: with factor 2 at 20 fps the step is capped at 0.1 s (a real 0.05 s gives 0.1 s; a real 0.1 s gives 0.1 s, not 0.2 s).
- **Scripted change:** `CWorldInfoEntity::SetRealTimeFactor(factor, duration)`: `factor` must be > 0, otherwise (and when not given) it is **1.0**; `duration` defaults to 0. The world-info entity's per-step function moves the variable **linearly** from its value at the call to `factor` over `duration` seconds of **simulation** time (the measure of elapsed time is itself scaled); when the elapsed fraction exceeds **0.99** (or `duration` is 0) it sets `factor` exactly and stops. `OnWorldInfoPreShutDown` sets the variable back to 1.0 **only if a change was still pending**. **None of the 59 worlds calls this action** (no use in the vocabulary lists), so the retail game never slows time from scripts.
- **Cheats (single player only, a cheat level of exactly 2, probably `sam_iEnableCheats`; the binding is not established):** one key **toggles the factor between 1.0 and 0.125** (printing "Real time factor %1"); in a debug key handler one key **halves** the factor and another **doubles** it.
- **No other writer** of the variable was found in `Sam2Game.dll` (the game DLL), so there is no gameplay slow-motion power-up or death-cam time change in the retail game. `InSamnity2Game.dll` was not read.
- Whether sound pitch follows the factor is not known (T01-067).

### 10. Interpolation and extrapolation

- **No interpolation between simulation steps for rendering was found** (confidence **medium**: absence in the function names of three DLLs, plus the step-equals-frame-time rule). The engine exports no render-state interpolation and the render phase draws entity state as left by the last step; since the step equals the frame time, there is no leftover fraction to blend with. The only blending functions in the three game DLLs are the ones named in 7 (mechanism motion, view origin) and a handful of per-entity "lerp to desired velocity and look" movement helpers (movement, T01-024).
- **Network clients:** the client interface keeps the server's time (from the packet stamps, see 1) and a queue of **delayed messages** (messages that cannot yet be applied are re-tried; an age limit of 2.0 s was read for the delayed queue, the variable bound to it is not established). No extrapolation or prediction function was found by name. How a client's `Now` is related to the server's is **not established** (the client's simulation advances with its own real-time clock through the same step rule; only demo playback calls `SetNow`). T01-085.
- Consequence for a reimplementation: our renderer's alpha between ticks (RFC 0002) is an addition of ours, not a feature of the original.

### 11. Network cadence (what is known)

`netStep` runs **twice per frame** (receive phase with sending off, send phase with sending on). There is no separate network tick: the server flushes each client's outgoing data once per frame, limited by the bandwidth variables (`cli_iMaxBPS` default **100,000**, `cli_iMaxOutBPS` **7,500**, `cli_iLowQualityBPS` 12,000, `cli_iHighQualityBPS` 100,000, `cli_iBPSQualityTreshold` 110,000). Connection and request time-outs of 2 s and 15 s constants exist; their variable bindings are not established. Everything else about the network time base is T01-085.

## Invariants

- One call of the game's frame function runs **0 or 1** simulation steps (checked in the only call site of `CSimulation::Step` in `Sam2Game.dll`; no loop around it).
- `0.001 ≤ sim_fStepMin ≤ sim_fStepMax ≤ 0.1` after every call of `Step` (the code writes the clamped values back); every executed step lasts `sim_fStepMin ≤ step ≤ sim_fStepMax` seconds before the `Time` rounding.
- `Now` never decreases except through `SetNow`/`ResetTime`; the cursor equals `Now` after every executed step.
- While `IsPaused`, or `LocalIsPaused` in a local session, `Now` does not change (the cursor gains 0; a jog adds to the cursor).
- Delays are `float` seconds; `Time` is seconds × 2³²; a due time is compared with `<=`.
- Macros do not execute on a remote client.

## Open questions

Ranked by what blocks an implementer.

1. **Measured behaviour (needs the running game):** the real distribution of step lengths at typical frame rates, whether the frame limiter or the display sync decides the pace, the `Sleep` quantisation of a limited loop, how the physics sub-step count behaves at **exact multiples** of 0.01 s (the division `step / 0.01` is not exact in `float`: for 0.05 the quotient is 5.00000019 and for 0.1 it is 10.0000004; the result is 6 and 11 if it is evaluated in wide precision, 5 and 10 if in single precision; the precision mode is set by the graphics driver and was not determined) and when the physics uses exactly 1 sub-step. **T01-084** (with T01-038/039 and T11-008).
2. **Network time:** the relation of a client's clock to the server's, update cadence per entity, the delayed-message limits, the session-parameter pause message end to end, whether `sim_fRealTimeFactor` is replicated, client smoothing. **T01-085**.
3. **Pause gating details:** the project instance's overlay flag, the GUI layer's "menu is up" rule, the unnamed "blocked" flag, the debug key's gating and names, the cheat level binding and the key names of the time-scale cheats, what happens to particles, effects, music and positional sounds during a pause. **T01-086**.
4. **Think queue and step functions in detail:** the priority numbers of step functions, the entry layout stored in worlds, the order of entries with equal due times, whether a re-arming think has a per-step bound. **T01-087**.
5. **Macro `Wait` re-evaluation** and the `me_fWaitPeriod` member: **T01-069** (existing; this page adds the interpreter's per-step execution as evidence).
6. **Effect and particle lifetime clocks**: T01-074 (existing). **Sound pitch under time scale**: T01-067 (existing).
7. **World loading flow and `.preload`:** `misc-types.md` points `.preload` at "T01-022 loading"; this spec only records the start/stop of a world in `HandleWorldChanges` (stop the old world, load through the resource loader, start, set the clock). The `.preload` numbers and the load order: **T01-088**.
8. **Not read:** the OpenGL and software graphics back ends' limiter, the editor shell, `InSamnity2Game.dll`, console builds. Covered by **T01-084**.

### Conflicts with the current design and other pages (reported, nothing changed here)

- [`SIMLOOP.md`](../SIMLOOP.md), `ss2-sim` `TickConfig` and [`DEDICATED_SERVER.md`](../DEDICATED_SERVER.md) use a **fixed** tick (default 30 Hz, "a placeholder until T01-022"). The original has **no fixed tick**: the placeholder is not "wrong", but it is not the original's. Differences that an implementer should know: the original's step is the frame time clamped to 1..100 ms; physics integrates in sub-steps of at most 10 ms; think timers have frame granularity; time above 100 ms per frame is **dropped** (slow motion) while our design's `max_frame_micros` default of 250 ms and `max_ticks_per_frame` of 8 are different numbers with the same purpose (the nearest equivalent is `max_frame_micros` = 100,000 and **no** catch-up). The **tick rate is a design decision of the owner**, not recoverable from the original: a fixed rate of 60 Hz or more keeps timer granularity close to a 60 fps original frame; 30 Hz doubles it. Follow-up for the owner: T01-089.
- [`SIMLOOP.md`](../SIMLOOP.md): "pause stops the ticks and does not save up the time" **agrees** with the original; "time scale ... limited to 8000 permille" is our limit (the original accepts any value ≥ 0 and limits only the step); "in a network game only the server may change it" matches the fact that the scripted change is a server-side action but the replication of the variable is unknown (open question 2).
- `console-variables.md` lists `sim_fRealTimeFactor`, `sim_fStepMax`, `sim_fStepMin`, `sim_fSyncRate` with blank defaults; the defaults are 1.0, 0.1, 0.001, 0 (this page). Not edited there.
- `console-config.md` says `sim_` is "the simulation time factor and similar": this page names all four timing variables (the other three of the seven `sim_` variables are switches: world-version check, memory-usage report, opened-files report).

## Test vectors

All with the defaults of the Summary, the simulation running (not paused), `Now` = cursor at the start of each frame, factor 1.0. `d` is the real time since the previous frame; ticks are 2⁻³² s.

| # | Input | Expected |
|---|---|---|
| 1 | `Time(1.0)` | 4,294,967,296 ticks; `Time(0.0625)` = 268,435,456; the `float` 0.1 (= 0.100000001490116) → **429,496,736** ticks; `Time(0)` = 0 |
| 2 | `d` = 0.0625 | one step of 0.0625 s: `Now` += 268,435,456 ticks; last step 0.0625 |
| 3 | `d` = 0.016667 | one step of 0.016667 s (float 0.016666999) |
| 4 | `d` = 0.1 | one step of 0.1 s |
| 5 | `d` = 0.25 | one step of **0.1** s; the other 0.15 s is lost; the next frame is normal |
| 6 | `d` = 10 | one step of 0.1 s |
| 7 | `d` = 0.0004 then 0.0007 | frame 1: **no step** (cursor − `Now` = 0.0004 < 0.001), `Now` unchanged, last step unchanged; frame 2: cursor − `Now` = 0.0011 ≥ 0.001 → one step of about 0.0011 s |
| 8 | `d` = 0 | no step |
| 9 | factor 0.5, `d` = 0.04 | step 0.02; factor 2, `d` = 0.08 → step **0.1** (0.16 clamped; the rest lost); factor 0 → no step; factor −1 → written back as 0, no step |
| 10 | paused for 5 s of real time, then `d` = 0.02 on the first frame after `UnPause` | the first step is 0.02 s (no burst: the real-time reference moved every frame) |
| 11 | paused, one `JogForward(0.05)`, then one frame | one step of 0.05 s (`Time(0.05f)` = 214,748,368 ticks), then paused again (cursor = `Now`) |
| 12 | `sim_fSyncRate` = 30, any `d` (also 0.5) | every call steps `1/30` = 0.0333333 s (a 0.5 s frame still gives 0.0333333 s); `sim_fSyncRate` = 5 → delta 0.2 → step **0.1** |
| 13 | `SetNow(t)` | `Now` = `t`, cursor = `t`, last step = 0.001; `ResetTime` → `Now` = 0, last step 0 |
| 14 | physics sub-steps for steps of 0.005, 0.02, 0.0333, 1/60, 0.0625 s | 1, 2, 4, 2, 7 sub-steps (lengths step/n). **Do not use** 0.05 and 0.1 as vectors (see Open question 1) |
| 15 | sleep factor | `phy_fSleepThresholdBaseFPS` × step with step 1/60: 1.5 → clamped to 1.0; with step 0.005: 0.45 |
| 16 | think queue: an entity calls `SetNextThink(0.5, S)` from inside a think at `Now` = 10.0 | due time = 10.5 (as ticks `Now + Time(0.5)`); it runs in the first step with `Now` ≥ 10.5, for example at `Now` = 10.51 (a 10 ms late start), never at 10.49 |
| 17 | the same call from a step function at `Now` = 10.0 when the previous step ended at 9.98 | due time = 9.98 + 0.5 = 10.48 (stale clock) |
| 18 | `SetRealTimeFactor(0.25, 2.0)` at factor 1.0 | each step the variable is `1 + (0.25 − 1) × (elapsed / 2.0)` (elapsed in simulation seconds) while `elapsed / 2.0` ≤ 0.99, then exactly 0.25; `SetRealTimeFactor(−3)` or no argument → target 1.0 |
| 19 | pause decision in a local session, menu open | `LocalPause` → `delta` = 0 → no step; the same menu in a hosted game → `LocalPause` is set but `delta` is **not** zeroed → steps continue |
| 20 | dedicated server, `sam_iDedicatedFPS` = 60, a pass took 4 ms | sleeps 1/60 − 0.004 = 12.67 ms (rounded by the system timer); a pass that took 20 ms sleeps 0; while loading the period is 1/33 |
| 21 | packet stamp wrap | stamps 4,294,967,280 then 16 (units of 1/1024 s): the accumulated server time grows by 32 units = 31.25 ms |
