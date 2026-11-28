use super::*;

fn flat(d: f32) -> WeaponTimingParams {
    WeaponTimingParams {
        min_recoil_duration: d,
        max_recoil_duration: d,
        ..Default::default()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Limited {
    remaining: u32,
}

impl AmmoSource for Limited {
    fn can_fire(&self) -> bool {
        self.remaining > 0
    }
    fn consume_shot(&mut self) -> bool {
        if self.remaining > 0 {
            self.remaining -= 1;
            true
        } else {
            false
        }
    }
}

// --- `GetRecoilDuration` (invariant 3) ---------------------------------------------------------

#[test]
fn recoil_duration_lerps_by_fire_power() {
    let p = WeaponTimingParams {
        min_recoil_duration: 0.2,
        max_recoil_duration: 0.5,
        ..Default::default()
    };
    assert_eq!(recoil_duration(&p, 0.0), 0.2);
    assert_eq!(recoil_duration(&p, 1.0), 0.5);
    assert!((recoil_duration(&p, 0.5) - 0.35).abs() < 1e-6);
}

#[test]
fn recoil_duration_falls_back_to_the_animation_when_min_is_at_most_zero() {
    // The sniper: min <= 0, not charging -> the recoil animation's own length, regardless of power.
    let p = WeaponTimingParams {
        min_recoil_duration: 0.0,
        max_recoil_duration: 0.0,
        recoil_anim_duration: 1.0,
        ..Default::default()
    };
    assert_eq!(recoil_duration(&p, 0.0), 1.0);
    assert_eq!(recoil_duration(&p, 1.0), 1.0);
}

#[test]
fn recoil_duration_scalable_charging_weapon_lerps_even_with_a_non_positive_minimum() {
    let scalable = WeaponTimingParams {
        min_recoil_duration: 0.0,
        max_recoil_duration: 1.0,
        recoil_anim_duration: 9.0,
        charging_flag: true,
        scalable_recoil: true,
        ..Default::default()
    };
    assert_eq!(recoil_duration(&scalable, 0.5), 0.5); // lerp(0, 1, 0.5), not the animation's 9.0

    let not_scalable = WeaponTimingParams {
        scalable_recoil: false,
        ..scalable
    };
    assert_eq!(recoil_duration(&not_scalable, 0.5), 9.0); // falls back since it is not "scalable"
}

#[test]
fn t01_433_measured_fire_periods() {
    // `docs/re/weapon-timing.md`, "Live-check, T01-433": every non-charging weapon passes fire
    // power 0, so the fire period is simply `min_recoil_duration`.
    assert_eq!(recoil_duration(&flat(1.0), 0.0), 1.0); // auto shotgun
    assert_eq!(recoil_duration(&flat(1.3), 0.0), 1.3); // double shotgun
    assert_eq!(recoil_duration(&flat(0.6), 0.0), 0.6); // rocket launcher
}

// --- `DoFiringsFromLastFireTime`'s catch-up arithmetic (section 5) ------------------------------

#[test]
fn catch_up_shots_colt_worked_table() {
    let d = 0.5;
    assert_eq!(catch_up_shots(0.4999, d), 0);
    assert_eq!(catch_up_shots(0.5, d), 1);
    assert_eq!(catch_up_shots(0.5001, d), 1);
    assert_eq!(catch_up_shots(0.9999, d), 1);
    assert_eq!(catch_up_shots(1.0, d), 2);
    assert_eq!(catch_up_shots(1.2499, d), 2);
    assert_eq!(catch_up_shots(1.25, d), 2); // an exact half truncates, no tie rule
    assert_eq!(catch_up_shots(1.2501, d), 2);
    assert_eq!(catch_up_shots(1.4999, d), 2);
    assert_eq!(catch_up_shots(1.5, d), 3);
}

#[test]
fn catch_up_shots_minigun_worked_table() {
    let d = 0.035;
    assert_eq!(catch_up_shots(0.0349, d), 0);
    assert_eq!(catch_up_shots(0.035, d), 1);
    assert_eq!(catch_up_shots(0.0699, d), 1);
    assert_eq!(catch_up_shots(0.07, d), 2);
    assert_eq!(catch_up_shots(0.0701, d), 2);
    assert_eq!(catch_up_shots(0.22, d), 6); // test vector 4
}

// --- the state machine itself -------------------------------------------------------------------

fn fire_count(events: &[WeaponEvent]) -> u32 {
    events
        .iter()
        .filter_map(|e| match e {
            WeaponEvent::Fired { count, .. } => Some(*count),
            _ => None,
        })
        .sum()
}

#[test]
fn fresh_weapon_fires_exactly_one_shot_on_the_first_press() {
    let p = flat(0.5);
    let mut w = WeaponMachine::new();
    let mut ammo = UnlimitedAmmo;
    let events = w.step(0.001, true, &p, &mut ammo);
    assert_eq!(fire_count(&events), 1);
    assert_eq!(w.last_fire_power(), 0.0); // an uncharged shot's fire power is always 0.0
    assert_eq!(w.state(), WeaponState::Recoil);
}

#[test]
fn held_trigger_refires_at_the_recoil_cadence_without_a_visible_idle() {
    let p = flat(0.5);
    let mut w = WeaponMachine::new();
    let mut ammo = UnlimitedAmmo;
    w.step(0.001, true, &p, &mut ammo); // shot 1
    for _ in 0..5 {
        let events = w.step(0.5, true, &p, &mut ammo);
        assert_eq!(fire_count(&events), 1);
        assert_eq!(w.state(), WeaponState::Recoil);
    }
}

#[test]
fn boring_threshold_is_strictly_greater_than_five_seconds() {
    // `docs/re/weapon-timing.md` section 8's own worked example, scaled to this machine's clock:
    // fire once, let recoil end, then approach the 5.0 s threshold from the weapon's own last fire.
    let p = flat(0.5);
    let mut w = WeaponMachine::new();
    let mut ammo = UnlimitedAmmo;
    w.step(0.001, true, &p, &mut ammo); // fire at t = 0.001
    w.step(0.5, false, &p, &mut ammo); // recoil ends -> Idle at t = 0.501
    assert_eq!(w.state(), WeaponState::Idle);

    w.step(4.4999, false, &p, &mut ammo); // t = 5.0009, idle time 4.9999
    assert_eq!(w.state(), WeaponState::Idle);

    w.step(0.0001, false, &p, &mut ammo); // t = 5.0010, idle time exactly 5.0
    assert_eq!(
        w.state(),
        WeaponState::Idle,
        "exactly 5.0 s does not qualify"
    );

    w.step(0.0001, false, &p, &mut ammo); // t = 5.0011, idle time 5.0001
    assert_eq!(w.state(), WeaponState::Boring);
}

#[test]
fn fire_one_next_overrides_a_long_catch_up_after_a_release() {
    let p = flat(0.5);
    let mut w = WeaponMachine::new();
    let mut ammo = UnlimitedAmmo;
    w.step(0.001, true, &p, &mut ammo); // shot 1, consumes the constructor's forced flag
    w.step(0.001, false, &p, &mut ammo); // release: sets the flag again
    w.step(20.0, false, &p, &mut ammo); // recoil ends, then 20 s idle -> Boring

    // What the catch-up formula alone would have given, had the trigger never been released:
    assert_eq!(catch_up_shots(20.0, 0.5), 40);

    // A fresh press after the release forces exactly one shot instead.
    let events = w.step(0.001, true, &p, &mut ammo);
    assert_eq!(fire_count(&events), 1);
}

#[test]
fn a_long_frame_catches_up_by_truncating_elapsed_over_recoil_duration() {
    // Test vector 4: a continuously held trigger through a long frame fires several shots in one
    // call, all at once, truncated toward zero with no `+1`.
    let p = flat(0.035);
    let mut w = WeaponMachine::new();
    let mut ammo = UnlimitedAmmo;
    w.step(0.001, true, &p, &mut ammo); // shot 1 at t = 0.001
    let events = w.step(0.22, true, &p, &mut ammo); // t = 0.221, elapsed since last shot = 0.22
    assert_eq!(fire_count(&events), 6);
}

#[test]
fn ammunition_runs_out_mid_burst_and_skips_recoil() {
    let p = flat(0.035);
    let mut w = WeaponMachine::new();
    let mut ammo = Limited { remaining: 2 };
    w.step(0.001, true, &p, &mut ammo); // shot 1, remaining 2 -> 1
    assert_eq!(ammo.remaining, 1);

    let events = w.step(0.11, true, &p, &mut ammo); // wants 3 shots, only 1 left
    assert_eq!(fire_count(&events), 1);
    assert_eq!(ammo.remaining, 0);
    assert_eq!(w.state(), WeaponState::CantFire);
    assert!(events.iter().any(|e| matches!(
        e,
        WeaponEvent::StateChanged {
            to: WeaponState::CantFire,
            ..
        }
    )));

    ammo.remaining = 1; // ammunition restored
    let events = w.step(0.001, false, &p, &mut ammo);
    assert!(events.iter().any(|e| matches!(
        e,
        WeaponEvent::StateChanged {
            from: WeaponState::CantFire,
            to: WeaponState::Idle
        }
    )));
}

#[test]
fn discrete_power_rule_zeroes_any_power_below_one() {
    let p = WeaponTimingParams {
        min_recoil_duration: 0.25,
        max_recoil_duration: 0.5,
        charging_flag: true,
        charging_time: 0.5,
        discrete_power: true,
        ..Default::default()
    };
    let mut ammo = UnlimitedAmmo;

    let mut partial = WeaponMachine::new();
    partial.step(0.001, true, &p, &mut ammo); // enter charging
    partial.step(0.25, false, &p, &mut ammo); // release at half charge
    assert_eq!(partial.last_fire_power(), 0.0);

    let mut full = WeaponMachine::new();
    full.step(0.001, true, &p, &mut ammo);
    full.step(0.5, false, &p, &mut ammo); // release at full charge
    assert_eq!(full.last_fire_power(), 1.0);
}

#[test]
fn grenade_launcher_test_vector_tapped_at_quarter_second() {
    // weapon-timing.md test vector 2: charging time 0.5 s, tapped (released) at 0.25 s gives
    // charge 0.5, one shot at fire power 0.5, and recoil lerp(0.2, 0.5, 0.5) = 0.35 s.
    let p = WeaponTimingParams {
        min_recoil_duration: 0.2,
        max_recoil_duration: 0.5,
        charging_flag: true,
        scalable_recoil: true,
        charging_time: 0.5,
        charging_auto_fire_time: 0.0,
        ..Default::default()
    };
    let mut w = WeaponMachine::new();
    let mut ammo = UnlimitedAmmo;

    let events = w.step(0.001, true, &p, &mut ammo);
    assert_eq!(w.state(), WeaponState::Charging);
    assert_eq!(fire_count(&events), 0); // no shot yet, only the state change into Charging

    let events = w.step(0.25, false, &p, &mut ammo); // now = 0.251, charge = 0.25 / 0.5 = 0.5
    assert!(events
        .iter()
        .any(|e| matches!(e, WeaponEvent::Fired { count: 1, fire_power } if (*fire_power - 0.5).abs() < 1e-6)));
    assert_eq!(w.state(), WeaponState::Recoil);

    // The recoil period is lerp(0.2, 0.5, 0.5) = 0.35 s: not over just before, over just after.
    let events = w.step(0.349, false, &p, &mut ammo);
    assert_eq!(
        w.state(),
        WeaponState::Recoil,
        "events in between: {events:?}"
    );
    let events = w.step(0.002, false, &p, &mut ammo);
    assert_eq!(
        w.state(),
        WeaponState::Idle,
        "events in between: {events:?}"
    );
}

#[test]
fn colt_reloads_after_six_shots() {
    // weapon-runtime-behaviour.md section 2.1 / weapon-timing.md test vector 3.
    let p = WeaponTimingParams {
        min_recoil_duration: 0.5,
        max_recoil_duration: 0.5,
        fires_to_reload: 6,
        reload_anim_duration: 0.4,
        ..Default::default()
    };
    let mut w = WeaponMachine::new();
    let mut ammo = UnlimitedAmmo;

    w.step(0.001, true, &p, &mut ammo); // shot 1
    for n in 2..=6 {
        w.step(0.5, true, &p, &mut ammo);
        assert_eq!(w.shots_since_reload(), n.min(6));
    }
    // After the 6th shot's recoil ends, the magazine is spent: Reload, not Idle, and the counter
    // is reset at once.
    let events = w.step(0.5, true, &p, &mut ammo);
    assert_eq!(w.state(), WeaponState::Reload, "events: {events:?}");
    assert_eq!(w.shots_since_reload(), 0);

    let events = w.step(0.4, false, &p, &mut ammo); // the reload animation's own length
    assert_eq!(w.state(), WeaponState::Idle);
    assert!(events.contains(&WeaponEvent::Reloaded));
}

#[test]
fn spin_firing_waits_for_full_spin_then_fires_without_visiting_recoil() {
    let p = WeaponTimingParams {
        min_recoil_duration: 0.035,
        max_recoil_duration: 0.035,
        spin_up_flag: true,
        spin_up_time: 0.5,
        ..Default::default()
    };
    let mut w = WeaponMachine::new();
    let mut ammo = UnlimitedAmmo;

    let events = w.step(0.001, true, &p, &mut ammo); // CallFireStart -> SpinFiring, no shot yet
    assert_eq!(w.state(), WeaponState::SpinFiring);
    assert_eq!(fire_count(&events), 0);

    let events = w.step(0.5, true, &p, &mut ammo); // spin_elapsed reaches 0.501 >= 0.5
    assert_eq!(fire_count(&events), 1);
    assert_eq!(
        w.state(),
        WeaponState::SpinFiring,
        "spin-firing never visits WS_RECOIL"
    );

    let events = w.step(0.036, true, &p, &mut ammo); // strictly past the recoil cadence
    assert_eq!(fire_count(&events), 1);
    assert_eq!(w.state(), WeaponState::SpinFiring);

    let events = w.step(0.001, false, &p, &mut ammo); // release returns straight to idle
    assert_eq!(w.state(), WeaponState::Idle);
    assert!(events.iter().any(|e| matches!(
        e,
        WeaponEvent::StateChanged {
            to: WeaponState::Idle,
            ..
        }
    )));
}

#[test]
fn switching_brings_up_then_puts_down_and_deletes() {
    // Test vector 5 (switching), structurally: bring-up ends in Idle, put-down ends deleted.
    let p = WeaponTimingParams {
        activate_duration: 0.2,
        ..Default::default()
    };
    let mut w = WeaponMachine::new();
    let mut ammo = UnlimitedAmmo;

    let events = w.begin_bring_up(&p, false);
    assert_eq!(w.state(), WeaponState::BringingUp);
    assert!(events.contains(&WeaponEvent::StateChanged {
        from: WeaponState::Idle,
        to: WeaponState::BringingUp,
    }));
    let events = w.step(0.2, false, &p, &mut ammo);
    assert_eq!(w.state(), WeaponState::Idle);
    assert!(events.contains(&WeaponEvent::BroughtUp));

    let events = w.begin_put_down(&p, false);
    assert_eq!(w.state(), WeaponState::PuttingDown);
    assert!(!events.is_empty());
    assert!(!w.is_deleted());
    let events = w.step(0.2, false, &p, &mut ammo);
    assert!(w.is_deleted());
    assert!(events.contains(&WeaponEvent::PutDown));

    // Nothing further happens once deleted.
    let events = w.step(1.0, true, &p, &mut ammo);
    assert!(events.is_empty());
}

#[test]
fn immediate_bring_up_skips_straight_to_idle() {
    let p = WeaponTimingParams {
        activate_duration: 5.0,
        ..Default::default()
    };
    let mut w = WeaponMachine::new();
    w.begin_bring_up(&p, true);
    assert_eq!(w.state(), WeaponState::Idle);
}

#[test]
fn the_serious_bombs_fuse_refuses_an_unforced_put_down_and_burns_out_to_idle() {
    let p = WeaponTimingParams::default();
    let mut w = WeaponMachine::new();
    let mut ammo = UnlimitedAmmo;

    w.begin_fuse(1.0);
    assert_eq!(w.state(), WeaponState::WaitFuseToBurn);
    assert!(!w.can_be_changed());

    let refused = w.begin_put_down(&p, false);
    assert!(refused.is_empty());
    assert_eq!(w.state(), WeaponState::WaitFuseToBurn);

    let events = w.step(1.0, false, &p, &mut ammo);
    assert_eq!(w.state(), WeaponState::Idle);
    assert!(events.contains(&WeaponEvent::FuseBurned));
}

#[test]
fn scheduling_a_new_pending_think_replaces_the_old_one() {
    // Invariant 1: a forced put-down during the fuse replaces its pending end.
    let p = WeaponTimingParams {
        activate_duration: 0.1,
        ..Default::default()
    };
    let mut w = WeaponMachine::new();
    let mut ammo = UnlimitedAmmo;

    w.begin_fuse(100.0); // would otherwise still be burning for a long time
    w.begin_put_down(&p, true); // forced: refused nowhere, replaces the fuse's own pending end
    assert_eq!(w.state(), WeaponState::PuttingDown);

    let events = w.step(0.1, false, &p, &mut ammo); // the put-down's own, much shorter, duration
    assert!(w.is_deleted());
    assert!(events.contains(&WeaponEvent::PutDown));
    assert!(!events.contains(&WeaponEvent::FuseBurned));
}

#[test]
fn cant_fire_blocks_idle_firing_until_ammunition_returns() {
    let p = flat(0.5);
    let mut w = WeaponMachine::new();
    let mut ammo = Limited { remaining: 0 };

    let events = w.step(0.001, true, &p, &mut ammo);
    assert_eq!(w.state(), WeaponState::CantFire);
    assert_eq!(fire_count(&events), 0);

    ammo.remaining = 1;
    let events = w.step(0.001, false, &p, &mut ammo);
    assert_eq!(w.state(), WeaponState::Idle);
    assert!(events.contains(&WeaponEvent::StateChanged {
        from: WeaponState::CantFire,
        to: WeaponState::Idle,
    }));
}

#[test]
fn determinism_same_inputs_give_the_same_result() {
    let p = WeaponTimingParams {
        min_recoil_duration: 0.1,
        max_recoil_duration: 0.1,
        fires_to_reload: 4,
        reload_anim_duration: 0.3,
        ..Default::default()
    };
    let inputs: Vec<(f32, bool)> = vec![
        (0.01, true),
        (0.1, true),
        (0.1, true),
        (0.1, false),
        (0.05, true),
        (0.3, true),
        (0.3, false),
        (5.2, false),
        (0.02, true),
    ];

    let run = || {
        let mut w = WeaponMachine::new();
        let mut ammo = UnlimitedAmmo;
        let mut total_fired = 0u32;
        for (dt, held) in &inputs {
            let events = w.step(*dt, *held, &p, &mut ammo);
            total_fired += fire_count(&events);
        }
        (w, total_fired)
    };

    let (a, fired_a) = run();
    let (b, fired_b) = run();
    assert_eq!(a, b);
    assert_eq!(fired_a, fired_b);
}

#[test]
fn determinism_is_not_affected_by_how_a_held_period_is_split_into_steps() {
    let p = flat(0.5);

    let mut one_step = WeaponMachine::new();
    let mut ammo1 = UnlimitedAmmo;
    one_step.step(0.001, true, &p, &mut ammo1);
    one_step.step(6.0, false, &p, &mut ammo1); // well past the boring threshold in one go

    let mut many_steps = WeaponMachine::new();
    let mut ammo2 = UnlimitedAmmo;
    many_steps.step(0.001, true, &p, &mut ammo2);
    for _ in 0..60 {
        many_steps.step(0.1, false, &p, &mut ammo2);
    }

    assert_eq!(one_step.state(), WeaponState::Boring);
    assert_eq!(many_steps.state(), WeaponState::Boring);
}
