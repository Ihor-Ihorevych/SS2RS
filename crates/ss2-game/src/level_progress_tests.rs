use super::*;

fn setup() -> LevelSetup {
    LevelSetup {
        players: 2,
        secrets: 3,
        checkpoints: 3,
        par_ms: Some(60_000),
        ..LevelSetup::default()
    }
}

fn level() -> LevelProgress {
    LevelProgress::new(setup()).unwrap()
}

fn payload(version: u32, n: usize) -> CheckpointPayload {
    CheckpointPayload {
        version,
        bytes: (0..n).map(|i| (i % 251) as u8).collect(),
    }
}

/// Recompute the checksum at the end, so damage is seen by the checks behind it.
fn reseal(bytes: &mut [u8]) {
    let n = bytes.len() - 4;
    let crc = crc32(&bytes[..n]);
    bytes[n..].copy_from_slice(&crc.to_le_bytes());
}

fn reads(l: &LevelProgress) -> Result<LevelProgress, ProgressError> {
    let mut b = l.to_bytes();
    reseal(&mut b);
    LevelProgress::from_bytes(&b)
}

// ---- setup ----------------------------------------------------------------------------------------------

#[test]
fn the_default_setup_is_valid_and_the_limits_are_exact() {
    LevelSetup::default().validate().unwrap();
    let ok = |f: &dyn Fn(&mut LevelSetup)| {
        let mut s = LevelSetup::default();
        f(&mut s);
        s.validate().is_ok()
    };
    assert!(ok(&|s| s.players = 1));
    assert!(ok(&|s| s.players = MAX_PLAYERS));
    assert!(!ok(&|s| s.players = 0));
    assert!(!ok(&|s| s.players = MAX_PLAYERS + 1));
    assert!(ok(&|s| s.secrets = MAX_SECRETS));
    assert!(!ok(&|s| s.secrets = MAX_SECRETS + 1));
    assert!(ok(&|s| s.checkpoints = MAX_CHECKPOINTS));
    assert!(!ok(&|s| s.checkpoints = MAX_CHECKPOINTS + 1));
    assert!(ok(&|s| s.max_payload = 1));
    assert!(ok(&|s| s.max_payload = MAX_PAYLOAD_LIMIT));
    assert!(!ok(&|s| s.max_payload = 0));
    assert!(!ok(&|s| s.max_payload = MAX_PAYLOAD_LIMIT + 1));
    assert!(
        LevelProgress::new(LevelSetup {
            players: 0,
            ..LevelSetup::default()
        })
        .is_err()
    );
}

#[test]
fn a_new_level_is_empty() {
    let l = level();
    assert_eq!(
        (
            l.kills(),
            l.kills_total(),
            l.secrets_found(),
            l.elapsed_ticks()
        ),
        (0, 0, 0, 0)
    );
    assert_eq!((l.checkpoints_reached(), l.restores()), (0, 0));
    assert!(!l.is_finished());
    assert_eq!(l.setup(), &setup());
}

// ---- kills ----------------------------------------------------------------------------------------------

#[test]
fn kills_are_credited_to_a_player_or_to_nobody_and_the_team_count_is_the_sum() {
    let mut l = level();
    l.register_enemies(10).unwrap();
    l.record_kill(Some(0), 0).unwrap();
    l.record_kill(Some(0), 0).unwrap();
    l.record_kill(Some(1), 0).unwrap();
    l.record_kill(None, 0).unwrap();
    assert_eq!(l.kills(), 4);
    assert_eq!(l.kills_total(), 10);
    let s = l.summary(30).unwrap();
    assert_eq!(
        s.players.iter().map(|p| p.kills).collect::<Vec<_>>(),
        [2, 1]
    );
    let credited: u32 = s.players.iter().map(|p| p.kills).sum();
    assert_eq!(
        credited + 1,
        s.kills,
        "player credit plus the uncredited kill is the team count"
    );
}

#[test]
fn the_total_is_never_below_the_kills() {
    let mut l = level();
    l.register_enemies(2).unwrap();
    for _ in 0..3 {
        l.record_kill(Some(0), 0).unwrap();
    }
    assert_eq!((l.kills(), l.kills_total()), (3, 3));
    assert_eq!(l.summary(30).unwrap().kill_percent(), Some(100));
    // Registering after the fact lifts it again.
    l.register_enemies(5).unwrap();
    assert_eq!(l.kills_total(), 7);
}

#[test]
fn a_kill_for_a_player_who_does_not_exist_is_refused_and_changes_nothing() {
    let mut l = level();
    let before = l.clone();
    assert_eq!(
        l.record_kill(Some(2), 100),
        Err(ProgressError::NoSuchPlayer { slot: 2 })
    );
    assert_eq!(
        l.add_score(Some(255), 1),
        Err(ProgressError::NoSuchPlayer { slot: 255 })
    );
    assert_eq!(l, before);
}

#[test]
fn counters_saturate_instead_of_wrapping() {
    let mut l = level();
    l.register_enemies(u32::MAX).unwrap();
    l.register_enemies(5).unwrap();
    assert_eq!(l.kills_total(), u32::MAX);
    l.live.players[0].kills = u32::MAX;
    l.live.players[0].score = u32::MAX;
    l.record_kill(Some(0), 10).unwrap();
    assert_eq!(l.live.players[0].kills, u32::MAX);
    assert_eq!(l.live.players[0].score, u32::MAX);
    l.live.kills_uncredited = u32::MAX;
    l.live.score_uncredited = u32::MAX;
    l.record_kill(None, 10).unwrap();
    assert_eq!(l.live.kills_uncredited, u32::MAX);
    assert_eq!(l.live.score_uncredited, u32::MAX);
    assert_eq!(l.kills(), u32::MAX, "the team sum saturates too");
    assert_eq!(l.summary(30).unwrap().score, Some(u32::MAX));
    l.elapsed = u64::MAX - 1;
    l.advance(10);
    assert_eq!(l.elapsed_ticks(), u64::MAX);
}

// ---- score ----------------------------------------------------------------------------------------------

#[test]
fn score_goes_to_the_killer_or_is_uncredited_and_the_team_score_is_the_sum() {
    let mut l = level();
    l.record_kill(Some(0), 100).unwrap();
    l.record_kill(Some(1), 50).unwrap();
    l.record_kill(None, 7).unwrap();
    l.add_score(Some(1), 1_000).unwrap();
    l.add_score(None, 3).unwrap();
    let s = l.summary(30).unwrap();
    assert_eq!(s.players[0].score, Some(100));
    assert_eq!(s.players[1].score, Some(1_050));
    assert_eq!(s.score, Some(100 + 1_050 + 7 + 3));
}

#[test]
fn a_mode_without_score_ignores_points_and_reports_none() {
    let mut l = LevelProgress::new(LevelSetup {
        scoring: false,
        ..setup()
    })
    .unwrap();
    l.record_kill(Some(0), 500).unwrap();
    l.add_score(None, 500).unwrap();
    l.add_score(Some(1), 500).unwrap();
    let s = l.summary(30).unwrap();
    assert_eq!(s.kills, 1, "the kill still counts");
    assert_eq!(s.score, None);
    assert!(s.players.iter().all(|p| p.score.is_none()));
    assert_eq!(l.live.score_uncredited, 0);
    assert_eq!(l.live.players[0].score, 0);
    assert_eq!(l.live.players[1].score, 0);
}

// ---- secrets --------------------------------------------------------------------------------------------

#[test]
fn a_secret_counts_once_and_the_first_finder_keeps_the_credit() {
    let mut l = level();
    assert_eq!(l.find_secret(1, Some(0)).unwrap(), SecretOutcome::New);
    assert_eq!(
        l.find_secret(1, Some(1)).unwrap(),
        SecretOutcome::AlreadyFound
    );
    assert_eq!(l.find_secret(1, None).unwrap(), SecretOutcome::AlreadyFound);
    assert_eq!(l.secrets_found(), 1);
    assert_eq!(l.find_secret(0, None).unwrap(), SecretOutcome::New);
    assert_eq!(l.find_secret(2, Some(1)).unwrap(), SecretOutcome::New);
    let s = l.summary(30).unwrap();
    assert_eq!((s.secrets, s.secrets_total), (3, 3));
    assert_eq!(
        s.players.iter().map(|p| p.secrets).collect::<Vec<_>>(),
        [1, 1]
    );
    assert_eq!(s.secret_percent(), Some(100));
}

#[test]
fn an_unknown_secret_or_finder_is_refused() {
    let mut l = level();
    assert_eq!(
        l.find_secret(3, None),
        Err(ProgressError::NoSuchSecret { id: 3 })
    );
    assert_eq!(
        l.find_secret(u32::MAX, None),
        Err(ProgressError::NoSuchSecret { id: u32::MAX })
    );
    assert_eq!(
        l.find_secret(0, Some(2)),
        Err(ProgressError::NoSuchPlayer { slot: 2 })
    );
    assert_eq!(l.secrets_found(), 0, "a refused call changes nothing");
    let none = LevelProgress::new(LevelSetup::default()).unwrap();
    assert_eq!(
        none.clone().find_secret(0, None),
        Err(ProgressError::NoSuchSecret { id: 0 })
    );
}

// ---- time and the summary -------------------------------------------------------------------------------

#[test]
fn time_is_ticks_and_converts_to_milliseconds_rounding_down() {
    let mut l = level();
    l.advance(30);
    l.advance(15);
    assert_eq!(l.elapsed_ticks(), 45);
    let s = l.summary(30).unwrap();
    assert_eq!((s.elapsed_ticks, s.elapsed_ms), (45, 1_500));
    // 1 tick at 60 Hz is 16.666 ms: rounded down.
    let mut one = level();
    one.advance(1);
    assert_eq!(one.summary(60).unwrap().elapsed_ms, 16);
    assert_eq!(one.summary(1).unwrap().elapsed_ms, 1_000);
    assert_eq!(one.summary(1000).unwrap().elapsed_ms, 1);
    assert_eq!(l.summary(0), Err(ProgressError::BadTickRate));
}

#[test]
fn a_huge_clock_does_not_overflow_the_conversion() {
    let mut l = level();
    l.advance(u64::MAX);
    assert_eq!(l.summary(1).unwrap().elapsed_ms, u64::MAX, "saturates");
    assert_eq!(l.summary(1000).unwrap().elapsed_ms, u64::MAX);
    assert_eq!(l.summary(2000).unwrap().elapsed_ms, u64::MAX / 2);
}

#[test]
fn par_time_is_within_when_at_or_under() {
    let mut l = level(); // par 60 s
    l.advance(60 * 30 - 1);
    assert_eq!(l.summary(30).unwrap().within_par, Some(true));
    l.advance(1);
    assert_eq!(
        l.summary(30).unwrap().within_par,
        Some(true),
        "exactly the par time is within"
    );
    l.advance(1);
    let s = l.summary(30).unwrap();
    assert_eq!(s.within_par, Some(false));
    assert_eq!(s.par_ms, Some(60_000));
    let np = LevelProgress::new(LevelSetup::default()).unwrap();
    assert_eq!(np.summary(30).unwrap().par_ms, None);
    assert_eq!(np.summary(30).unwrap().within_par, None);
}

#[test]
fn percentages_with_zero_totals_are_none_and_rounding_is_down() {
    let l = LevelProgress::new(LevelSetup::default()).unwrap();
    let s = l.summary(30).unwrap();
    assert_eq!((s.kill_percent(), s.secret_percent()), (None, None));
    assert_eq!(
        (s.kills, s.kills_total, s.secrets, s.secrets_total),
        (0, 0, 0, 0)
    );
    assert_eq!(percent(1, 3), Some(33));
    assert_eq!(percent(2, 3), Some(66));
    assert_eq!(percent(3, 3), Some(100));
    assert_eq!(percent(0, 3), Some(0));
    assert_eq!(percent(9, 3), Some(100), "a part over the total is capped");
    assert_eq!(percent(u32::MAX, u32::MAX), Some(100));
    assert_eq!(percent(1, 0), None);
}

#[test]
fn the_summary_reports_everything() {
    let mut l = level();
    l.register_enemies(4).unwrap();
    l.record_kill(Some(1), 10).unwrap();
    l.find_secret(0, Some(1)).unwrap();
    l.reach_checkpoint(0, payload(1, 3)).unwrap();
    l.restore().unwrap();
    l.advance(60);
    let s = l.summary(30).unwrap();
    assert_eq!((s.kills, s.kills_total), (1, 4));
    assert_eq!((s.secrets, s.secrets_total), (1, 3));
    assert_eq!((s.checkpoints, s.checkpoints_total), (1, 3));
    assert_eq!(s.restores, 1);
    assert_eq!(s.kill_percent(), Some(25));
    assert_eq!(s.secret_percent(), Some(33));
    assert_eq!(s.players.len(), 2);
    assert_eq!((s.players[0].slot, s.players[1].slot), (0, 1));
}

// ---- checkpoints ----------------------------------------------------------------------------------------

#[test]
fn checkpoints_are_reached_in_order_once_each() {
    let mut l = level();
    assert_eq!(
        l.reach_checkpoint(0, payload(1, 4)).unwrap(),
        CheckpointOutcome::New
    );
    assert_eq!(l.checkpoints_reached(), 1);
    assert_eq!(
        l.reach_checkpoint(0, payload(1, 4)).unwrap(),
        CheckpointOutcome::AlreadyReached
    );
    assert_eq!(l.checkpoints_reached(), 1);
    assert_eq!(
        l.reach_checkpoint(1, payload(1, 4)).unwrap(),
        CheckpointOutcome::New
    );
    assert_eq!(
        l.reach_checkpoint(0, payload(1, 4)).unwrap(),
        CheckpointOutcome::AlreadyReached,
        "going back over an old one"
    );
    assert_eq!(l.checkpoints_reached(), 2);
}

#[test]
fn a_checkpoint_out_of_order_or_beyond_the_level_is_refused_and_changes_nothing() {
    let mut l = level();
    assert_eq!(
        l.reach_checkpoint(1, payload(1, 1)),
        Err(ProgressError::OutOfOrder {
            expected: 0,
            got: 1
        })
    );
    assert_eq!(
        l.reach_checkpoint(2, payload(1, 1)),
        Err(ProgressError::OutOfOrder {
            expected: 0,
            got: 2
        })
    );
    assert_eq!(
        l.reach_checkpoint(3, payload(1, 1)),
        Err(ProgressError::NoSuchCheckpoint { index: 3 })
    );
    assert_eq!(
        l.reach_checkpoint(u32::MAX, payload(1, 1)),
        Err(ProgressError::NoSuchCheckpoint { index: u32::MAX })
    );
    assert_eq!(l.checkpoints_reached(), 0);
    assert!(l.last.is_none());
    l.reach_checkpoint(0, payload(1, 1)).unwrap();
    l.reach_checkpoint(1, payload(1, 1)).unwrap();
    l.reach_checkpoint(2, payload(1, 1)).unwrap();
    assert_eq!(
        l.reach_checkpoint(3, payload(1, 1)),
        Err(ProgressError::NoSuchCheckpoint { index: 3 })
    );
    let none = LevelProgress::new(LevelSetup::default()).unwrap();
    assert_eq!(
        none.clone().reach_checkpoint(0, payload(1, 1)),
        Err(ProgressError::NoSuchCheckpoint { index: 0 })
    );
}

#[test]
fn the_payload_limit_is_exact_and_a_refused_checkpoint_is_not_reached() {
    let mut l = LevelProgress::new(LevelSetup {
        max_payload: 8,
        ..setup()
    })
    .unwrap();
    assert_eq!(
        l.reach_checkpoint(0, payload(1, 9)),
        Err(ProgressError::PayloadTooLarge { len: 9, max: 8 })
    );
    assert_eq!(l.checkpoints_reached(), 0);
    assert!(l.last.is_none());
    assert_eq!(
        l.reach_checkpoint(0, payload(1, 8)).unwrap(),
        CheckpointOutcome::New
    );
    // An empty payload is allowed.
    assert_eq!(
        l.reach_checkpoint(1, CheckpointPayload::default()).unwrap(),
        CheckpointOutcome::New
    );
    // The payload of a checkpoint already reached is not looked at.
    assert_eq!(
        l.reach_checkpoint(0, payload(1, 9)).unwrap(),
        CheckpointOutcome::AlreadyReached
    );
}

#[test]
fn an_already_reached_checkpoint_keeps_its_first_payload() {
    let mut l = level();
    l.reach_checkpoint(0, payload(7, 3)).unwrap();
    l.reach_checkpoint(0, payload(8, 5)).unwrap();
    assert_eq!(l.last.as_ref().unwrap().payload, payload(7, 3));
}

// ---- restore --------------------------------------------------------------------------------------------

#[test]
fn restore_with_no_checkpoint_goes_back_to_the_start() {
    let mut l = level();
    l.register_enemies(5).unwrap();
    l.record_kill(Some(0), 10).unwrap();
    l.find_secret(2, Some(1)).unwrap();
    l.advance(100);
    assert_eq!(l.restore().unwrap(), Restored::LevelStart);
    assert_eq!((l.kills(), l.kills_total(), l.secrets_found()), (0, 0, 0));
    assert_eq!(l.summary(30).unwrap().score, Some(0));
    assert_eq!(l.restores(), 1);
    assert_eq!(l.elapsed_ticks(), 100, "the clock keeps running by default");
}

#[test]
fn restore_puts_the_counters_back_and_returns_the_payload() {
    let mut l = level();
    l.register_enemies(8).unwrap();
    l.record_kill(Some(0), 100).unwrap();
    l.find_secret(0, Some(0)).unwrap();
    l.reach_checkpoint(0, payload(3, 5)).unwrap();
    // After the checkpoint.
    l.register_enemies(4).unwrap();
    l.record_kill(Some(1), 50).unwrap();
    l.record_kill(None, 5).unwrap();
    l.find_secret(1, Some(1)).unwrap();
    l.add_score(Some(0), 999).unwrap();
    match l.restore().unwrap() {
        Restored::Checkpoint { index, payload: p } => {
            assert_eq!(index, 0);
            assert_eq!(p, &payload(3, 5));
        }
        Restored::LevelStart => panic!("a checkpoint was reached"),
    }
    assert_eq!(l.kills(), 1);
    assert_eq!(l.kills_total(), 8);
    assert_eq!(
        l.secrets_found(),
        1,
        "a secret found after the checkpoint can be found again"
    );
    assert_eq!(l.find_secret(1, Some(1)).unwrap(), SecretOutcome::New);
    assert_eq!(
        l.find_secret(0, Some(1)).unwrap(),
        SecretOutcome::AlreadyFound,
        "one found before it stays found"
    );
    let s = l.summary(30).unwrap();
    assert_eq!((s.players[0].kills, s.players[1].kills), (1, 0));
    assert_eq!(
        (s.players[0].score, s.players[1].score, s.score),
        (Some(100), Some(0), Some(100))
    );
    assert_eq!(s.players[0].secrets, 1);
    assert_eq!(s.restores, 1);
}

#[test]
fn restore_goes_to_the_last_checkpoint_and_reached_checkpoints_stay_reached() {
    let mut l = level();
    l.reach_checkpoint(0, payload(1, 1)).unwrap();
    l.record_kill(None, 0).unwrap();
    l.reach_checkpoint(1, payload(2, 2)).unwrap();
    l.record_kill(None, 0).unwrap();
    let r = l.restore().unwrap();
    assert_eq!(
        r,
        Restored::Checkpoint {
            index: 1,
            payload: &payload(2, 2)
        }
    );
    assert_eq!(l.kills(), 1);
    assert_eq!(l.checkpoints_reached(), 2);
    // The next one in order is still 2.
    assert_eq!(
        l.reach_checkpoint(2, payload(3, 0)).unwrap(),
        CheckpointOutcome::New
    );
}

#[test]
fn restoring_twice_gives_the_same_state_and_counts_two_retries() {
    let mut l = level();
    l.record_kill(Some(0), 3).unwrap();
    l.reach_checkpoint(0, payload(1, 2)).unwrap();
    l.record_kill(Some(0), 3).unwrap();
    l.restore().unwrap();
    let once = l.clone();
    l.restore().unwrap();
    assert_eq!(l.live, once.live);
    assert_eq!(l.restores(), 2);
}

#[test]
fn restoring_to_the_start_rewinds_the_clock_only_when_asked() {
    for (rule, expect) in [(RestoreTime::Continue, 10), (RestoreTime::Rewind, 0)] {
        let mut l = LevelProgress::new(LevelSetup {
            restore_time: rule,
            ..setup()
        })
        .unwrap();
        l.advance(10);
        assert_eq!(l.restore().unwrap(), Restored::LevelStart);
        assert_eq!(l.elapsed_ticks(), expect);
    }
}

#[test]
fn rewind_goes_back_to_the_checkpoint_tick_and_continue_does_not() {
    let mut r = LevelProgress::new(LevelSetup {
        restore_time: RestoreTime::Rewind,
        ..setup()
    })
    .unwrap();
    r.advance(40);
    r.reach_checkpoint(0, payload(1, 1)).unwrap();
    r.advance(100);
    r.restore().unwrap();
    assert_eq!(r.elapsed_ticks(), 40);
    let mut c = level();
    c.advance(40);
    c.reach_checkpoint(0, payload(1, 1)).unwrap();
    c.advance(100);
    c.restore().unwrap();
    assert_eq!(c.elapsed_ticks(), 140);
    assert_eq!(c.summary(10).unwrap().elapsed_ms, 14_000);
}

// ---- finish ---------------------------------------------------------------------------------------------

#[test]
fn a_finished_level_stops_everything() {
    let mut l = level();
    l.register_enemies(1).unwrap();
    l.record_kill(Some(0), 5).unwrap();
    l.advance(30);
    l.finish();
    l.finish();
    assert!(l.is_finished());
    let frozen = l.summary(30).unwrap();
    l.advance(1_000);
    assert_eq!(l.register_enemies(1), Err(ProgressError::Finished));
    assert_eq!(l.record_kill(None, 1), Err(ProgressError::Finished));
    assert_eq!(l.add_score(None, 1), Err(ProgressError::Finished));
    assert_eq!(l.find_secret(0, None), Err(ProgressError::Finished));
    assert_eq!(
        l.reach_checkpoint(0, payload(1, 1)),
        Err(ProgressError::Finished)
    );
    assert_eq!(l.restore(), Err(ProgressError::Finished));
    assert_eq!(l.summary(30).unwrap(), frozen);
    assert_eq!(l.restores(), 0, "a refused restore is not counted");
}

// ---- determinism ----------------------------------------------------------------------------------------

fn script(l: &mut LevelProgress) {
    l.register_enemies(12).unwrap();
    l.advance(5);
    l.record_kill(Some(1), 100).unwrap();
    l.find_secret(2, Some(1)).unwrap();
    l.advance(7);
    l.reach_checkpoint(0, payload(2, 40)).unwrap();
    l.record_kill(None, 20).unwrap();
    l.find_secret(0, None).unwrap();
    l.restore().unwrap();
    l.advance(9);
    l.record_kill(Some(0), 100).unwrap();
    l.reach_checkpoint(1, payload(2, 7)).unwrap();
    l.finish();
}

#[test]
fn the_same_events_give_byte_identical_state() {
    let (mut a, mut b) = (level(), level());
    script(&mut a);
    script(&mut b);
    assert_eq!(a, b);
    assert_eq!(a.to_bytes(), b.to_bytes());
    let mut c = level();
    script(&mut c);
    c.finished = false;
    c.advance(1);
    assert_ne!(a.to_bytes(), c.to_bytes());
}

#[test]
fn the_order_secrets_are_found_in_does_not_change_the_bytes() {
    let (mut a, mut b) = (level(), level());
    for id in [0, 1, 2] {
        a.find_secret(id, Some(0)).unwrap();
    }
    for id in [2, 0, 1] {
        b.find_secret(id, Some(0)).unwrap();
    }
    assert_eq!(a.to_bytes(), b.to_bytes());
}

// ---- bytes ----------------------------------------------------------------------------------------------

#[test]
fn crc32_matches_the_standard_check_value() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(crc32(b""), 0);
}

#[test]
fn a_played_level_survives_a_round_trip() {
    let mut l = level();
    script(&mut l);
    let back = LevelProgress::from_bytes(&l.to_bytes()).unwrap();
    assert_eq!(back, l);
    assert_eq!(back.summary(30).unwrap(), l.summary(30).unwrap());
    assert_eq!(back.to_bytes(), l.to_bytes());
}

#[test]
fn every_kind_of_state_round_trips() {
    let setups = [
        LevelSetup::default(),
        LevelSetup {
            players: MAX_PLAYERS,
            secrets: MAX_SECRETS,
            checkpoints: MAX_CHECKPOINTS,
            ..LevelSetup::default()
        },
        LevelSetup {
            scoring: false,
            par_ms: None,
            restore_time: RestoreTime::Rewind,
            max_payload: 3,
            ..setup()
        },
        LevelSetup {
            par_ms: Some(u64::MAX),
            ..setup()
        },
    ];
    for s in setups {
        let fresh = LevelProgress::new(s.clone()).unwrap();
        assert_eq!(LevelProgress::from_bytes(&fresh.to_bytes()).unwrap(), fresh);
        let mut busy = fresh;
        busy.advance(77);
        busy.register_enemies(3).unwrap();
        if busy.setup.secrets > 0 {
            busy.find_secret(0, None).unwrap();
            busy.find_secret(busy.setup.secrets - 1, Some(busy.setup.players - 1))
                .unwrap();
        }
        busy.record_kill(Some(busy.setup.players - 1), 4).unwrap();
        busy.record_kill(None, 4).unwrap();
        if busy.setup.checkpoints > 0 {
            busy.reach_checkpoint(0, payload(9, busy.setup.max_payload.min(3)))
                .unwrap();
        }
        let back = LevelProgress::from_bytes(&busy.to_bytes()).unwrap();
        assert_eq!(back, busy);
    }
}

#[test]
fn a_restored_game_behaves_like_the_original() {
    let mut l = level();
    l.advance(3);
    l.reach_checkpoint(0, payload(1, 2)).unwrap();
    let mut copy = LevelProgress::from_bytes(&l.to_bytes()).unwrap();
    for g in [&mut l, &mut copy] {
        g.record_kill(Some(0), 1).unwrap();
        g.restore().unwrap();
        g.reach_checkpoint(1, payload(1, 1)).unwrap();
    }
    assert_eq!(l, copy);
}

#[test]
fn every_prefix_and_every_flipped_byte_is_refused() {
    let mut l = level();
    script(&mut l);
    let bytes = l.to_bytes();
    for n in 0..bytes.len() {
        assert!(
            LevelProgress::from_bytes(&bytes[..n]).is_err(),
            "prefix of {n}"
        );
    }
    for i in 0..bytes.len() {
        let mut b = bytes.clone();
        b[i] ^= 0x5A;
        assert!(LevelProgress::from_bytes(&b).is_err(), "byte {i} flipped");
    }
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(LevelProgress::from_bytes(&longer).is_err());
    assert!(LevelProgress::from_bytes(&[]).is_err());
}

#[test]
fn damage_with_a_correct_checksum_is_still_refused() {
    let mut l = level();
    script(&mut l);
    let good = l.to_bytes();
    let expect_err = |edit: &dyn Fn(&mut Vec<u8>), why: &str| {
        let mut b = good.clone();
        edit(&mut b);
        reseal(&mut b);
        assert!(LevelProgress::from_bytes(&b).is_err(), "{why}");
    };
    expect_err(&|b| b[0] = b'X', "magic");
    expect_err(&|b| b[4] = 2, "version");
    expect_err(&|b| b[6] = 0, "zero players");
    expect_err(&|b| b[6] = MAX_PLAYERS + 1, "too many players");
    // The setup has a par time: flag at 15, the time at 16..24, then scoring, the payload limit, the restore rule.
    expect_err(&|b| b[15] = 2, "par flag");
    expect_err(&|b| b[24] = 2, "scoring flag");
    expect_err(&|b| b[29] = 2, "restore rule");
    expect_err(&|b| b[38] = 2, "finished flag");
    expect_err(
        &|b| {
            let n = b.len() - 4;
            b.splice(n..n, [0u8]);
        },
        "trailing byte",
    );
    // Truncation behind a good checksum.
    let mut short = good.clone();
    short.truncate(short.len() - 6);
    reseal(&mut short);
    assert!(LevelProgress::from_bytes(&short).is_err());
    assert!(LevelProgress::from_bytes(&[0; 9]).is_err());
}

#[test]
fn inconsistent_states_are_refused_when_read() {
    let mut base = level();
    base.find_secret(1, Some(0)).unwrap();
    base.reach_checkpoint(0, payload(1, 4)).unwrap();
    assert!(reads(&base).is_ok());

    let broken = |f: &dyn Fn(&mut LevelProgress), why: &str| {
        let mut l = base.clone();
        f(&mut l);
        assert!(reads(&l).is_err(), "{why}");
    };
    broken(&|l| l.reached = 4, "more checkpoints reached than exist");
    broken(&|l| l.reached = 0, "a snapshot but none reached");
    broken(&|l| l.last = None, "reached but no snapshot");
    broken(
        &|l| l.last.as_mut().unwrap().tick = l.elapsed + 1,
        "a checkpoint from the future",
    );
    broken(
        &|l| l.last.as_mut().unwrap().payload.bytes = vec![0; 1 << 21],
        "payload over the limit",
    );
    broken(
        &|l| {
            l.live.secrets.insert(3, None);
        },
        "a secret that does not exist",
    );
    broken(
        &|l| {
            l.live.secrets.insert(0, Some(2));
        },
        "a finder that is not a player",
    );
    broken(
        &|l| {
            l.last.as_mut().unwrap().counters.secrets.insert(9, None);
        },
        "a bad secret in the snapshot",
    );
    broken(
        &|l| {
            l.setup.scoring = false;
            l.live.players[0].score = 1;
        },
        "score without scoring (player)",
    );
    broken(
        &|l| {
            l.setup.scoring = false;
            l.live.score_uncredited = 1;
        },
        "score without scoring (uncredited)",
    );
    broken(
        &|l| {
            l.setup.scoring = false;
            l.last.as_mut().unwrap().counters.players[1].score = 1;
        },
        "score in a snapshot",
    );
    broken(&|l| l.setup.max_payload = 0, "setup out of range");
}

#[test]
fn secrets_out_of_order_or_repeated_are_refused() {
    let mut l = level();
    l.find_secret(0, None).unwrap();
    l.find_secret(1, None).unwrap();
    let good = l.to_bytes();
    // The secret table starts after the live counters: find the two entries (id, finder) at the end of the live block.
    let pos = good
        .windows(10)
        .position(|w| w == [0, 0, 0, 0, 0xFF, 1, 0, 0, 0, 0xFF])
        .expect("two entries side by side");
    let mut dup = good.clone();
    dup[pos + 5] = 0; // second id 0: repeated
    reseal(&mut dup);
    assert!(LevelProgress::from_bytes(&dup).is_err());
    let mut swapped = good.clone();
    swapped[pos] = 1;
    swapped[pos + 5] = 0; // 1 then 0: not ascending
    reseal(&mut swapped);
    assert!(LevelProgress::from_bytes(&swapped).is_err());
    let mut ok = good;
    reseal(&mut ok);
    assert!(LevelProgress::from_bytes(&ok).is_ok());
}

#[test]
fn a_hostile_length_does_not_allocate_or_panic() {
    let mut l = LevelProgress::new(LevelSetup {
        checkpoints: 1,
        ..setup()
    })
    .unwrap();
    l.reach_checkpoint(0, payload(1, 3)).unwrap();
    let good = l.to_bytes();
    // The payload length is the u32 before the 3 payload bytes and the checksum.
    let at = good.len() - 4 - 3 - 4;
    let mut b = good.clone();
    b[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    reseal(&mut b);
    assert!(LevelProgress::from_bytes(&b).is_err());
    let mut b = good.clone();
    b[at..at + 4].copy_from_slice(&4u32.to_le_bytes());
    reseal(&mut b);
    assert!(
        LevelProgress::from_bytes(&b).is_err(),
        "one byte more than is there"
    );
    let mut b = good;
    b[at..at + 4].copy_from_slice(&2u32.to_le_bytes());
    reseal(&mut b);
    assert!(
        LevelProgress::from_bytes(&b).is_err(),
        "one byte less leaves a byte over"
    );
}

#[test]
fn pseudo_random_garbage_never_panics() {
    let mut seed = 0x1234_5678_9ABC_DEF0u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut l = level();
    script(&mut l);
    let good = l.to_bytes();
    for _ in 0..2_000 {
        let mut b = good.clone();
        for _ in 0..(next() % 4 + 1) {
            let i = (next() as usize) % b.len();
            b[i] = next() as u8;
        }
        reseal(&mut b);
        let _ = LevelProgress::from_bytes(&b);
        let len = (next() as usize) % (good.len() + 8);
        let junk: Vec<u8> = (0..len).map(|_| next() as u8).collect();
        let _ = LevelProgress::from_bytes(&junk);
    }
}

#[test]
fn errors_have_readable_text() {
    let all = [
        ProgressError::BadSetup("x"),
        ProgressError::Finished,
        ProgressError::NoSuchPlayer { slot: 9 },
        ProgressError::NoSuchSecret { id: 8 },
        ProgressError::NoSuchCheckpoint { index: 7 },
        ProgressError::OutOfOrder {
            expected: 1,
            got: 3,
        },
        ProgressError::PayloadTooLarge { len: 5, max: 4 },
        ProgressError::BadTickRate,
        ProgressError::BadBytes("y"),
    ];
    for e in all {
        assert!(!e.to_string().is_empty());
    }
    assert!(
        ProgressError::OutOfOrder {
            expected: 1,
            got: 3
        }
        .to_string()
        .contains('3')
    );
}

#[test]
fn a_bad_flag_is_reported_as_that_flag() {
    let message = |b: &[u8]| match LevelProgress::from_bytes(b) {
        Err(ProgressError::BadBytes(m)) => m,
        other => panic!("expected a refusal, got {other:?}"),
    };
    // Par flag (offset 15, after the magic, version, players, secrets and checkpoints).
    let mut b = level().to_bytes();
    b[15] = 2;
    reseal(&mut b);
    assert_eq!(message(&b), "par flag is not 0 or 1");
    // Restore rule: with no par time the flag bytes shift left by 8.
    let mut b = LevelProgress::new(LevelSetup {
        par_ms: None,
        ..setup()
    })
    .unwrap()
    .to_bytes();
    b[21] = 2;
    reseal(&mut b);
    assert_eq!(message(&b), "unknown restore-time rule");
    // The checkpoint flag is the last byte before the checksum when no checkpoint was reached.
    let mut b = level().to_bytes();
    let at = b.len() - 5;
    assert_eq!(b[at], 0);
    b[at] = 2;
    reseal(&mut b);
    assert_eq!(message(&b), "checkpoint flag is not 0 or 1");
}
