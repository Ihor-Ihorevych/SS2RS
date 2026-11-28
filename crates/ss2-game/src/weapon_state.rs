//! The weapon state machine: states, animation-event-driven timing, catch-up firing and ammo
//! (roadmap T09-033, split (a) of the original `XL` T09-005). Pure logic: a tick only ever
//! advances by the `dt` it is given (never a wall clock), there is no randomness and no
//! `HashMap`, so the same sequence of ticks always gives the same state and the same events.
//!
//! **No rendering, no weapon data and no projectiles here.** Durations come in as plain numbers
//! (animation lengths, recoil min/max) through [`WeaponTimingParams`] — this module needs no
//! weapon model, `.ep` loading is T09-034. Ammunition is a caller-supplied [`AmmoSource`]
//! (`can_fire`/`consume_shot`), not a weapon's own kind/cost/cap — those are also T09-034's data,
//! and actually firing a bullet or a projectile into the world is T09-035. "Fire modes" here
//! means the four branches `CallFireStart` takes (plain, charging, spin-up) and their own
//! transitions and cadence, not what a shot *does*.
//!
//! Source: `docs/re/weapon-timing.md` (T01-117/T01-435, the state table of section 1, the firing
//! chain of section 2, `SetState`/`BringUp`/`PutDown` of sections 3-4, the catch-up arithmetic of
//! section 5, the minigun's spin ramp of section 6, the boring threshold of section 8) and
//! `docs/re/weapon-runtime-behaviour.md` section 2.1 (the Colt's magazine, restated). Both pages'
//! own confidence notes apply (mostly "high"/"medium"; see each page). `docs/re/weapon-view.md`'s
//! charge/recoil displacement and the minigun's spin-velocity ramp are view/effects concerns
//! already split off by `player_view.rs`'s own module docs, not reproduced here either — see
//! "Split off, by design" below.
//!
//! ## The ten states ([`WeaponState`])
//!
//! Every weapon starts [`WeaponState::Idle`]. Pressing (holding) fire from `Idle`/[`Boring`]
//! branches three ways by the weapon's own flags: no flag fires at once and enters `Recoil`; the
//! charging flag enters [`Charging`] (fires once on release, at the built-up charge, or once
//! after `charging_time + charging_auto_fire_time` at full power); the spin-up flag enters
//! [`SpinFiring`] (fires at the recoil cadence once `spin_up_time` has passed, releasing returns
//! straight to `Idle` with no recoil visit). `Recoil` ends (`OnRecoilEnd`) in [`CantFire`] (no
//! ammunition), [`Reload`] (the magazine is spent) or back to `Idle` — and, while the trigger is
//! still held on a non-charging weapon, the very next [`WeaponMachine::step`] call re-fires and
//! re-enters `Recoil` without the caller ever observing `Idle` in between (`docs/re/weapon-timing.md`
//! section 1's own words: "without a visit to `WS_IDLE`"). `Idle` becomes [`Boring`] once the
//! weapon has fired at least once and more than 5.0 s (strictly) have passed since; `CantFire`
//! leaves once [`AmmoSource::can_fire`] is true again. [`WeaponMachine::begin_bring_up`] and
//! [`WeaponMachine::begin_put_down`] drive [`BringingUp`]/[`PuttingDown`]; put-down always ends by
//! marking the machine deleted ([`WeaponMachine::is_deleted`]), matching the original always
//! destroying the weapon entity on switch. [`WaitFuseToBurn`] (the Serious Bomb) is reachable only
//! through [`WeaponMachine::begin_fuse`] here — nothing in this slice fires one (T09-035) — and is
//! the only state [`WeaponMachine::can_be_changed`] refuses.
//!
//! [`Boring`]: WeaponState::Boring
//! [`Charging`]: WeaponState::Charging
//! [`SpinFiring`]: WeaponState::SpinFiring
//! [`CantFire`]: WeaponState::CantFire
//! [`Reload`]: WeaponState::Reload
//! [`BringingUp`]: WeaponState::BringingUp
//! [`PuttingDown`]: WeaponState::PuttingDown
//! [`WaitFuseToBurn`]: WeaponState::WaitFuseToBurn
//!
//! ## Split off, by design
//!
//! - The minigun/circular-saw spin-velocity ramp itself (`MoveWeaponParts`'s acceleration curve,
//!   the vibration at full spin, section 6) is a view/effects concern, like `player_view.rs`'s own
//!   split-off charge/recoil shake. [`SpinFiring`] only tracks whether `spin_up_time` has elapsed
//!   since entry (enough to gate the first shot's timing correctly) — no velocity value is
//!   exposed.
//! - Which shot a weapon fires (hitscan or projectile, damage, scatter) is T09-035; this module
//!   only counts "fired N shots at fire power P" ([`WeaponEvent::Fired`]).
//! - Ammunition *kind*, *cost* and *cap* are T09-034's data; this module calls a caller-supplied
//!   [`AmmoSource`] once per shot and once per idle/boring tick, exactly where
//!   `CanBeUsedForFiring`/`OnFire`'s payment are read in the spec.
//! - The owner-level "time since any weapon last fired" that really drives the boring threshold
//!   (the threshold is the *owner's*, not this weapon entity's, per `docs/re/weapon-timing.md`
//!   section 8) is approximated here by this weapon's own last-fire time, since a single
//!   [`WeaponMachine`] models one weapon in isolation; true cross-weapon tracking is T09-036
//!   (switching/inventory).

/// The weapon's current state (`docs/re/weapon-timing.md` section 1's ten-state enumeration,
/// numbered there `WS_IDLE` 1 .. `WS_WAIT_FUSE_TO_BURN` 10; the numbers are not reproduced here,
/// only the names and transitions).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WeaponState {
    /// At rest; the idle animation loops. The only "fresh" state (`WeaponMachine::new` starts
    /// here).
    #[default]
    Idle,
    /// Switching away: the activate animation plays backward, then the machine is marked deleted.
    PuttingDown,
    /// Switching to: the activate animation plays forward, then `Idle`.
    BringingUp,
    /// Holding fire on a spin-up weapon (minigun, circular saw): fires at the recoil cadence once
    /// fully spun up; releasing returns straight to `Idle`.
    SpinFiring,
    /// Holding fire on a charging weapon (grenade launcher, cannon, zap gun): releasing fires at
    /// the built-up charge, or it auto-fires at full power once the charge time plus auto-fire
    /// time has passed.
    Charging,
    /// Out of ammunition; left once [`AmmoSource::can_fire`] is true again.
    CantFire,
    /// Idle for more than 5.0 s (strictly) since the last shot; a quick idle-pose interrupt, then
    /// behaves exactly like `Idle` for firing.
    Boring,
    /// The pause after every shot, whose length is [`recoil_duration`].
    Recoil,
    /// The magazine is spent (only the Colt, in the retail data); waits for the reload animation.
    Reload,
    /// The Serious Bomb's fuse is burning; the only state [`WeaponMachine::can_be_changed`]
    /// refuses. Reachable in this slice only through [`WeaponMachine::begin_fuse`].
    WaitFuseToBurn,
}

/// The 5.0 s, strictly-greater boring threshold (`docs/re/weapon-timing.md` section 8), the same
/// literal constant for every weapon and every game mode.
pub const BORING_THRESHOLD_S: f32 = 5.0;

/// A weapon's timing numbers, as plain data (no model; T09-034 loads these from the `.ep` files).
/// Every duration here is seconds; every flag is `docs/re/weapon-classes.md`'s own reading of the
/// weapon's default-constructor flags. `Default` is a weapon with all-zero timings and no flags
/// (a degenerate but harmless configuration: the zero guard in [`WeaponMachine::step`] simply
/// means it never fires on its own via the catch-up path, though the constructor's/release's
/// forced single shot still goes through).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaponTimingParams {
    /// The activate animation's own length, used both for bring-up (played forward) and put-down
    /// (played backward) — the original has only one such animation (`weapon-classes.md`'s
    /// `GetActivateAnim`).
    pub activate_duration: f32,
    /// `bwp_fMinRecoilDuration`.
    pub min_recoil_duration: f32,
    /// `bwp_fMaxRecoilDuration`.
    pub max_recoil_duration: f32,
    /// The recoil animation's own length, used instead of the lerp when `min_recoil_duration` is
    /// at most 0 on a weapon that is not both `charging_flag` and `scalable_recoil`.
    pub recoil_anim_duration: f32,
    /// The charging flag (`WCF_CHARGE`: grenade launcher, cannon, zap gun).
    pub charging_flag: bool,
    /// Whether a charging weapon's recoil still scales by fire power even when
    /// `min_recoil_duration` is at most 0 (`docs/re/weapon-timing.md` invariant 3; no retail
    /// weapon actually needs this branch, but the rule is general).
    pub scalable_recoil: bool,
    /// `bwp_fChargingTime`.
    pub charging_time: f32,
    /// `bwp_fChargingAutoFireTime`.
    pub charging_auto_fire_time: f32,
    /// The spin-up flag (minigun, circular saw).
    pub spin_up_flag: bool,
    /// `bwp_fSpinUpTime`: how long after entering [`WeaponState::SpinFiring`] the first shot can
    /// fire.
    pub spin_up_time: f32,
    /// The boring animation's own length, or `None` when the weapon has none (the boring state is
    /// then entered and stays until the next explicit transition, `docs/re/weapon-timing.md`
    /// section 3's `SetState` quirk — not externally different here, since nothing reads an
    /// animation name, only the state).
    pub boring_anim_duration: Option<f32>,
    /// `bwp_ctFiresToReload`; 0 means the weapon never reloads (every retail weapon but the Colt).
    pub fires_to_reload: u32,
    /// The reload animation's own length.
    pub reload_anim_duration: f32,
    /// `bwp_fReloadDuration`; a positive value rescales the reload wait to this instead of
    /// `reload_anim_duration` (never positive in the retail data).
    pub reload_duration_override: f32,
    /// The discrete-power rule (zap gun only): a fire power below 1.0 becomes exactly 0.0.
    pub discrete_power: bool,
}

/// `GetRecoilDuration` (`docs/re/weapon-timing.md` invariant 3): the recoil animation's own length
/// whenever `min_recoil_duration` is at most 0 on a weapon that is not both charging and
/// scalable; otherwise `lerp(min_recoil_duration, max_recoil_duration, fire_power)`, `fire_power`
/// clamped to `0.0..=1.0`.
pub fn recoil_duration(params: &WeaponTimingParams, fire_power: f32) -> f32 {
    if params.min_recoil_duration <= 0.0 && !(params.charging_flag && params.scalable_recoil) {
        params.recoil_anim_duration
    } else {
        let t = fire_power.clamp(0.0, 1.0);
        params.min_recoil_duration + (params.max_recoil_duration - params.min_recoil_duration) * t
    }
}

/// `DoFiringsFromLastFireTime`'s catch-up arithmetic (`docs/re/weapon-timing.md` section 5):
/// `trunc(elapsed / recoil_duration_s)`, truncated toward zero with no `+1` and no rounding to
/// nearest (Rust's `f32 as u32` cast already truncates toward zero for finite, non-negative
/// values, and saturates to 0 for NaN or a negative result). Callers are responsible for the
/// zero/invalid guard ("both `d` and `elapsed` are guarded against being exactly 0.0";
/// [`WeaponMachine::step`] applies it before calling this); this function assumes valid,
/// non-degenerate inputs and is exposed standalone so the arithmetic itself — including the
/// worked examples of `docs/re/weapon-timing.md` section 5's table — can be tested directly.
pub fn catch_up_shots(elapsed_s: f32, recoil_duration_s: f32) -> u32 {
    let q = elapsed_s / recoil_duration_s;
    if q > 0.0 { q as u32 } else { 0 }
}

/// What a tick of [`WeaponMachine::step`] (or [`WeaponMachine::begin_bring_up`]/
/// [`begin_put_down`](WeaponMachine::begin_put_down)/[`begin_fuse`](WeaponMachine::begin_fuse))
/// reports happened. A caller (T09-035 for `Fired`, T09-036 for `BroughtUp`/`PutDown`) drains
/// these; this module does not keep a history.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WeaponEvent {
    /// `DoFiringsFromLastFireTime` fired `count` shots in one call, each at `fire_power` (the
    /// power passed to the call, after the discrete-power rule; recoil keeps scaling by the
    /// *last* shot's power, so a long catch-up burst's own events are reported one call at a
    /// time, matching the original stamping every shot of a burst with the same `simNow()`).
    Fired {
        /// How many shots this call fired (at least 1, unless ammunition ran out mid-burst, in
        /// which case the burst up to and including the failed shot is not counted here — only
        /// successful shots are `Fired`).
        count: u32,
        /// The fire power applied to every shot of this burst.
        fire_power: f32,
    },
    /// The state changed (every transition, including the transient `Recoil` → `Idle` → `Recoil`
    /// collapse of a continuously held non-charging weapon, which a caller never observes as a
    /// pause but does see as two events in the same [`WeaponMachine::step`] call).
    StateChanged {
        /// The state left.
        from: WeaponState,
        /// The state entered.
        to: WeaponState,
    },
    /// The bring-up animation finished (`OnWeaponBroughtUp`).
    BroughtUp,
    /// The put-down animation finished; the machine is now [`WeaponMachine::is_deleted`]
    /// (`OnWeaponPutDown`, which always deletes the weapon entity).
    PutDown,
    /// The reload animation finished (`OnWeaponReloaded`).
    Reloaded,
    /// The Serious Bomb's fuse ran out (`Explode`, minus the actual explosion — T09-035).
    FuseBurned,
}

/// What a weapon asks of its ammunition, once per idle/boring tick and once per shot attempt.
/// T09-034 supplies the real kind/cost/cap; this trait only needs the two questions the state
/// machine itself asks (`CanBeUsedForFiring` and `OnFire`'s payment).
pub trait AmmoSource {
    /// Whether the weapon could fire right now, without spending anything.
    fn can_fire(&self) -> bool;
    /// Spend one shot's ammunition. Must return `false` whenever [`can_fire`](Self::can_fire) was
    /// `false` at the same point (a caller that is internally consistent never sees a mismatch);
    /// returning `false` here enters [`WeaponState::CantFire`] and stops the current burst
    /// without running `DoRecoil`.
    fn consume_shot(&mut self) -> bool;
}

/// Ammunition that never runs out and costs nothing to use — the Colt's own rule (it needs no
/// ammunition kind at all) and a convenient default for tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnlimitedAmmo;

impl AmmoSource for UnlimitedAmmo {
    fn can_fire(&self) -> bool {
        true
    }
    fn consume_shot(&mut self) -> bool {
        true
    }
}

/// The weapon state machine itself. `Default`/[`WeaponMachine::new`] is a freshly brought-up
/// weapon: [`WeaponState::Idle`], never fired, the "fire exactly one next" flag set (matching the
/// original's constructor, `docs/re/weapon-timing.md` section 5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponMachine {
    state: WeaponState,
    /// Accumulated sim seconds; advances only by [`step`](Self::step)'s own `dt`, never a wall
    /// clock.
    now: f64,
    /// The one pending think this entity may have (bring-up/put-down/boring/reload/recoil/fuse
    /// end), as an absolute `now`. Scheduling a new one (every state above does, on entry)
    /// replaces whatever was pending, matching the original's single-pending-think mechanism.
    scheduled_end: Option<f64>,
    trigger_was_held: bool,
    /// `en_tmLastFire`.
    last_fire_time: Option<f64>,
    /// The "fire exactly one next" flag.
    fire_one_next: bool,
    /// Shots fired since the last reload (the Colt's magazine; irrelevant, and always 0, for a
    /// weapon whose `fires_to_reload` is 0).
    shots_since_reload: u32,
    /// `en_fLastFirePower`.
    last_fire_power: f32,
    /// Set while [`WeaponState::Charging`], the `now` the charge began.
    charge_start: Option<f64>,
    /// Seconds since entering [`WeaponState::SpinFiring`] (reset at entry).
    spin_elapsed: f32,
    deleted: bool,
}

impl Default for WeaponMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl WeaponMachine {
    /// A freshly brought-up weapon: `Idle`, never fired, ready to fire exactly one shot on the
    /// very first press regardless of anything else.
    pub fn new() -> Self {
        Self {
            state: WeaponState::Idle,
            now: 0.0,
            scheduled_end: None,
            trigger_was_held: false,
            last_fire_time: None,
            fire_one_next: true,
            shots_since_reload: 0,
            last_fire_power: 0.0,
            charge_start: None,
            spin_elapsed: 0.0,
            deleted: false,
        }
    }

    /// The current state.
    pub fn state(&self) -> WeaponState {
        self.state
    }

    /// Whether `OnWeaponPutDown` has run: the caller should remove this weapon (and this
    /// machine); nothing further happens from [`step`](Self::step) once this is true.
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    /// `CanBeChanged`: every state refuses switching except [`WeaponState::WaitFuseToBurn`].
    pub fn can_be_changed(&self) -> bool {
        self.state != WeaponState::WaitFuseToBurn
    }

    /// Shots fired since the last reload (or since creation, for a weapon that never reloads).
    pub fn shots_since_reload(&self) -> u32 {
        self.shots_since_reload
    }

    /// The fire power of the last shot actually fired (0.0 before any shot).
    pub fn last_fire_power(&self) -> f32 {
        self.last_fire_power
    }

    /// Advance by `dt` seconds with the fire button's current state. A non-finite, zero or
    /// negative `dt`, or a call after [`is_deleted`](Self::is_deleted), does nothing and returns
    /// no events — this mirrors `Walker::step`'s own guard elsewhere in this crate.
    pub fn step<A: AmmoSource>(
        &mut self,
        dt: f32,
        trigger_held: bool,
        params: &WeaponTimingParams,
        ammo: &mut A,
    ) -> Vec<WeaponEvent> {
        let mut events = Vec::new();
        if self.deleted || !dt.is_finite() || dt <= 0.0 {
            return events;
        }
        self.now += dt as f64;

        // The base `OnFireReleased`, on every release: forces the next `DoFiringsFromLastFireTime`
        // call to exactly one shot, and (`FireReleasedInternal`) fires a charging weapon at its
        // built-up charge or leaves spin-firing straight to idle.
        if self.trigger_was_held && !trigger_held {
            self.fire_one_next = true;
            match self.state {
                WeaponState::Charging => {
                    let charge = if params.charging_time > 0.0 {
                        let start = self.charge_start.unwrap_or(self.now);
                        ((self.now - start) as f32 / params.charging_time).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    self.charge_start = None;
                    self.scheduled_end = None;
                    self.do_firings_from_last_fire_time(charge, params, ammo, &mut events);
                }
                WeaponState::SpinFiring => {
                    self.transition(WeaponState::Idle, &mut events);
                }
                _ => {}
            }
        }
        self.trigger_was_held = trigger_held;

        // The one pending think: bring-up, put-down, boring, reload, recoil or fuse end.
        if let Some(end) = self.scheduled_end
            && self.now >= end
        {
            self.scheduled_end = None;
            self.on_scheduled_end(params, ammo, &mut events);
        }

        // `CanBeUsedForFiring()` becoming true again leaves `WS_CANT_FIRE`.
        if self.state == WeaponState::CantFire && ammo.can_fire() {
            self.transition(WeaponState::Idle, &mut events);
        }

        // `OnStep`'s idle/boring branch (`docs/re/weapon-timing.md` section 8): the ammo check
        // first, then `CallFireStart` if the trigger is held, then (only if still idle) the
        // boring test.
        if matches!(self.state, WeaponState::Idle | WeaponState::Boring) {
            if !ammo.can_fire() {
                self.transition(WeaponState::CantFire, &mut events);
            } else if trigger_held {
                self.call_fire_start(params, ammo, &mut events);
            } else if self.state == WeaponState::Idle
                && let Some(t) = self.last_fire_time
                && (self.now - t) as f32 > BORING_THRESHOLD_S
            {
                self.enter_boring(params, &mut events);
            }
        }

        // Spin-firing never visits `WS_RECOIL` (`docs/re/weapon-timing.md` section 1: "staying in
        // WS_SPIN_FIRING"), so its own cadence needs a per-tick check here; every other state's
        // continuation is handled above, through the scheduled recoil end.
        if self.state == WeaponState::SpinFiring {
            self.spin_elapsed += dt;
            if self.spin_elapsed >= params.spin_up_time {
                let d = recoil_duration(params, self.last_fire_power);
                let should_fire = match self.last_fire_time {
                    None => true,
                    Some(t) => d > 0.0 && (self.now - t) as f32 > d,
                };
                if should_fire {
                    self.do_firings_from_last_fire_time(0.0, params, ammo, &mut events);
                }
            }
        }

        events
    }

    /// `BringUp`. `immediate` (`BringUp(nonzero)`) skips straight to [`WeaponState::Idle`] (the
    /// original's own "near-zero scheduled callback" collapsed into the same call, since nothing
    /// observable happens in between); otherwise (`BringUp(0)`, the normal case) enters
    /// [`WeaponState::BringingUp`] for `activate_duration`, ending in `Idle`
    /// ([`WeaponEvent::BroughtUp`]).
    pub fn begin_bring_up(
        &mut self,
        params: &WeaponTimingParams,
        immediate: bool,
    ) -> Vec<WeaponEvent> {
        let mut events = Vec::new();
        if self.deleted {
            return events;
        }
        if immediate {
            self.transition(WeaponState::Idle, &mut events);
            self.scheduled_end = None;
        } else {
            self.transition(WeaponState::BringingUp, &mut events);
            self.scheduled_end = Some(self.now + params.activate_duration.max(0.0) as f64);
        }
        events
    }

    /// `PutDown(forced)`: refused (no events) when not `forced` and
    /// [`can_be_changed`](Self::can_be_changed) is false; otherwise enters
    /// [`WeaponState::PuttingDown`] for `activate_duration`, ending with
    /// [`is_deleted`](Self::is_deleted) ([`WeaponEvent::PutDown`]).
    pub fn begin_put_down(
        &mut self,
        params: &WeaponTimingParams,
        forced: bool,
    ) -> Vec<WeaponEvent> {
        let mut events = Vec::new();
        if self.deleted || (!forced && !self.can_be_changed()) {
            return events;
        }
        self.transition(WeaponState::PuttingDown, &mut events);
        self.scheduled_end = Some(self.now + params.activate_duration.max(0.0) as f64);
        events
    }

    /// Enters [`WeaponState::WaitFuseToBurn`] for `fuse_duration` seconds, ending back at `Idle`
    /// ([`WeaponEvent::FuseBurned`] — the explosion itself is T09-035). Nothing in this slice
    /// calls this on its own; a Serious-Bomb-specific caller would, in place of the ordinary
    /// firing chain.
    pub fn begin_fuse(&mut self, fuse_duration: f32) -> Vec<WeaponEvent> {
        let mut events = Vec::new();
        if self.deleted {
            return events;
        }
        self.transition(WeaponState::WaitFuseToBurn, &mut events);
        self.scheduled_end = Some(self.now + fuse_duration.max(0.0) as f64);
        events
    }

    fn transition(&mut self, to: WeaponState, events: &mut Vec<WeaponEvent>) {
        if self.state != to {
            events.push(WeaponEvent::StateChanged {
                from: self.state,
                to,
            });
            self.state = to;
        }
    }

    fn enter_boring(&mut self, params: &WeaponTimingParams, events: &mut Vec<WeaponEvent>) {
        self.transition(WeaponState::Boring, events);
        self.scheduled_end = params
            .boring_anim_duration
            .map(|d| self.now + d.max(0.0) as f64);
    }

    /// `CallFireStart`: from `Idle`/`Boring` only (callers already guard this), branches by the
    /// weapon's own flags.
    fn call_fire_start<A: AmmoSource>(
        &mut self,
        params: &WeaponTimingParams,
        ammo: &mut A,
        events: &mut Vec<WeaponEvent>,
    ) {
        if params.charging_flag {
            self.transition(WeaponState::Charging, events);
            self.charge_start = Some(self.now);
            let dur = (params.charging_time + params.charging_auto_fire_time).max(0.0);
            self.scheduled_end = Some(self.now + dur as f64);
        } else if params.spin_up_flag {
            self.transition(WeaponState::SpinFiring, events);
            self.spin_elapsed = 0.0;
        } else {
            self.do_firings_from_last_fire_time(0.0, params, ammo, events);
        }
    }

    /// What happens when the one pending think fires, dispatched by the current state (only one
    /// state is ever scheduled at a time).
    fn on_scheduled_end<A: AmmoSource>(
        &mut self,
        params: &WeaponTimingParams,
        ammo: &mut A,
        events: &mut Vec<WeaponEvent>,
    ) {
        match self.state {
            WeaponState::BringingUp => {
                self.transition(WeaponState::Idle, events);
                events.push(WeaponEvent::BroughtUp);
            }
            WeaponState::PuttingDown => {
                self.deleted = true;
                events.push(WeaponEvent::PutDown);
            }
            WeaponState::Boring => {
                self.transition(WeaponState::Idle, events);
            }
            WeaponState::Reload => {
                self.transition(WeaponState::Idle, events);
                events.push(WeaponEvent::Reloaded);
            }
            WeaponState::WaitFuseToBurn => {
                self.transition(WeaponState::Idle, events);
                events.push(WeaponEvent::FuseBurned);
            }
            WeaponState::Recoil => {
                // `OnRecoilEnd`.
                if !ammo.can_fire() {
                    self.transition(WeaponState::CantFire, events);
                } else if params.fires_to_reload > 0
                    && self.shots_since_reload >= params.fires_to_reload
                {
                    self.shots_since_reload = 0;
                    self.fire_one_next = true;
                    self.transition(WeaponState::Reload, events);
                    let dur = if params.reload_duration_override > 0.0 {
                        params.reload_duration_override
                    } else {
                        params.reload_anim_duration
                    };
                    self.scheduled_end = Some(self.now + dur.max(0.0) as f64);
                } else {
                    self.transition(WeaponState::Idle, events);
                }
            }
            WeaponState::Idle
            | WeaponState::SpinFiring
            | WeaponState::Charging
            | WeaponState::CantFire => {}
        }
    }

    /// `DoFiringsFromLastFireTime(power)`: the single point that both fires "now" (a forced
    /// single shot) and catches up (a held trigger's elapsed time covering whole recoil periods).
    fn do_firings_from_last_fire_time<A: AmmoSource>(
        &mut self,
        power: f32,
        params: &WeaponTimingParams,
        ammo: &mut A,
        events: &mut Vec<WeaponEvent>,
    ) {
        let shots: u32 = if self.fire_one_next || params.charging_flag {
            self.fire_one_next = false;
            1
        } else {
            let d = recoil_duration(params, self.last_fire_power);
            let elapsed = match self.last_fire_time {
                Some(t) => (self.now - t) as f32,
                None => return,
            };
            if d == 0.0 || elapsed == 0.0 || !d.is_finite() || !elapsed.is_finite() {
                // The zero/invalid guard: fires nothing, and `DoRecoil` does not run either.
                return;
            }
            catch_up_shots(elapsed, d)
        };

        let mut fired = 0u32;
        let mut skip_recoil = false;
        for _ in 0..shots {
            if !ammo.can_fire() || !ammo.consume_shot() {
                self.transition(WeaponState::CantFire, events);
                skip_recoil = true;
                break;
            }
            let applied_power = if params.discrete_power && power < 1.0 {
                0.0
            } else {
                power
            };
            self.last_fire_power = applied_power;
            self.last_fire_time = Some(self.now);
            fired += 1;
            self.shots_since_reload = self.shots_since_reload.saturating_add(1);
            if params.fires_to_reload > 0 && self.shots_since_reload >= params.fires_to_reload {
                // `ShouldReload()` now holds: leave the loop early, but `DoRecoil` still runs.
                break;
            }
        }

        if fired > 0 {
            events.push(WeaponEvent::Fired {
                count: fired,
                fire_power: self.last_fire_power,
            });
        }
        if skip_recoil {
            return;
        }

        // `DoRecoil()`, exactly once: a spin-firing weapon never visits `WS_RECOIL` (it stays in
        // `SpinFiring`, its own per-tick check re-reads `last_fire_time`/`last_fire_power`
        // directly); every other state enters `Recoil` for `recoil_duration(last_fire_power)`,
        // including when `shots` was 0 (a recoil period starts with no shot fired).
        if self.state != WeaponState::SpinFiring {
            self.transition(WeaponState::Recoil, events);
            let d = recoil_duration(params, self.last_fire_power).max(0.0);
            self.scheduled_end = Some(self.now + d as f64);
        }
    }
}

#[cfg(test)]
#[path = "weapon_state_tests.rs"]
mod tests;
