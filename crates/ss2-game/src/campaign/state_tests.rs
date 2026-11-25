use super::*;
use crate::campaign::carry::{CarryError, PlayerCarry};
use crate::campaign::list::Next;
use crate::campaign::unlocks::{MAX_UNLOCKS, Trigger, UnlockKind, UnlockRule};
use crate::difficulty::{ChangePolicy, ChangeRefusal};
use std::collections::BTreeMap;

fn e(id: &str, ep: &str) -> LevelEntry {
    LevelEntry::new(id, ep, "N")
}

/// a -> b -> c -> end, with a secret level s that b leads to by its exit 1 and that rejoins at c.
/// Movies: a has an episode movie, an intro and an outro; b an intro only; s none; c an episode movie and an outro.
fn list() -> LevelList {
    LevelList::new(vec![
        e("a", "ep1")
            .with_episode_movie("EP1")
            .with_intro("a_in")
            .with_outro("a_out"),
        e("b", "ep1").with_intro("b_in").with_alt_exit("s"),
        e("c", "ep2")
            .with_episode_movie("EP2")
            .with_outro("c_out")
            .with_next(Next::End),
        e("s", "ep1")
            .with_kind(LevelKind::Secret)
            .with_next(Next::Level("c".into())),
    ])
    .unwrap()
}

/// a -> b, no movies at all, b ends the campaign.
fn plain() -> LevelList {
    LevelList::new(vec![e("a", "e"), e("b", "e").with_next(Next::End)]).unwrap()
}

fn diff() -> CampaignDifficulty {
    CampaignDifficulty::new(Difficulty::Hard, ChangePolicy::BetweenLevels)
}

fn carry_of(health: u32, weapon: &str) -> PlayerCarry {
    PlayerCarry {
        health,
        armor: 5,
        preferred_weapon: Some(weapon.into()),
        weapons: BTreeMap::from([(weapon.to_string(), 1)]),
        ammo: BTreeMap::from([("bullets".to_string(), 30)]),
        items: BTreeMap::new(),
    }
}

fn start() -> PlayerCarry {
    PlayerCarry {
        health: 100,
        armor: 0,
        preferred_weapon: Some("colt".into()),
        weapons: BTreeMap::from([("colt".to_string(), 1)]),
        ammo: BTreeMap::from([("bullets".to_string(), 50)]),
        items: BTreeMap::new(),
    }
}

fn result(level: &str, exit: u8) -> LevelResult {
    LevelResult {
        level: level.into(),
        exit,
        secrets_found: 0,
        secrets_total: 0,
    }
}

fn solo_gate() -> ExitGate {
    let mut g = ExitGate::new(1, 0).unwrap();
    g.set_status(0, PlayerStatus::AtExit).unwrap();
    g
}

fn solo_carry() -> CarryOver {
    let mut c = CarryOver::new();
    c.set(0, Slot::Carried(carry_of(60, "shotgun"))).unwrap();
    c
}

fn leave_solo(c: &mut Campaign, level: &str, exit: u8) -> Result<Departure, TransitionError> {
    let gate = solo_gate();
    let rules = UnlockRules::none();
    c.leave(LeaveRequest {
        gate: &gate,
        carry: solo_carry(),
        result: result(level, exit),
        rules: &rules,
    })
}

fn event_for(stage: Stage) -> Progress {
    match stage {
        Stage::Load => Progress::Loaded,
        Stage::Spawn => Progress::Spawned,
        _ => Progress::MovieDone,
    }
}

/// Answer every stage until a level is being played (or the campaign is over); returns the movies that played.
fn enter(c: &mut Campaign) -> Vec<String> {
    let mut movies = Vec::new();
    loop {
        if let Some(m) = c.current_movie() {
            movies.push(m.to_string());
        }
        match c.phase() {
            Phase::Playing { .. } | Phase::Finished { .. } => return movies,
            Phase::Transition { stage, .. } => c.progress(event_for(stage)).unwrap(),
            Phase::Ending { .. } => c.progress(Progress::MovieDone).unwrap(),
        };
    }
}

fn playing(list: LevelList) -> Campaign {
    let mut c = Campaign::new(list, diff());
    enter(&mut c);
    c
}

// ---- a new game and the stages ------------------------------------------------------------------------------

#[test]
fn a_new_game_starts_at_the_first_level_in_the_load_stage() {
    let c = Campaign::new(list(), diff());
    assert_eq!(
        c.phase(),
        Phase::Transition {
            from: None,
            to: 0,
            stage: Stage::Load,
            movies: true
        }
    );
    assert_eq!(c.stage(), Some(Stage::Load));
    assert_eq!(c.current_level().id, "a");
    assert_eq!(c.current_movie(), None);
    assert!(c.unlocks().level_unlocked("a"));
    assert_eq!(c.unlocks().len(), 1);
    assert_eq!(c.elapsed_ticks(), 0);
    assert_eq!(c.restarts(), 0);
    assert!(c.entry_carry().is_empty() && c.pending_carry().is_empty());
    assert!(!c.difficulty().in_level());
    assert_eq!(c.difficulty().level(), Difficulty::Hard);
    assert_eq!(c.list().len(), 4);
}

#[test]
fn a_new_game_forgets_that_the_difficulty_was_in_a_level() {
    let mut d = diff();
    d.enter_level();
    let c = Campaign::new(list(), d);
    assert!(!c.difficulty().in_level());
}

#[test]
fn the_first_level_plays_its_episode_movie_then_its_intro() {
    let mut c = Campaign::new(list(), diff());
    let mut seen = Vec::new();
    let mut stages = Vec::new();
    while !matches!(c.phase(), Phase::Playing { .. }) {
        let stage = c.stage().unwrap();
        stages.push(stage);
        if let Some(m) = c.current_movie() {
            seen.push(m.to_string());
        }
        c.progress(event_for(stage)).unwrap();
    }
    assert_eq!(seen, ["EP1", "a_in"]);
    assert_eq!(
        stages,
        [Stage::Load, Stage::EpisodeMovie, Stage::Intro, Stage::Spawn]
    );
    assert_eq!(c.phase(), Phase::Playing { level: 0 });
    assert!(c.difficulty().in_level());
    assert_eq!(c.stage(), None);
    assert_eq!(c.current_movie(), None);
}

#[test]
fn a_stage_with_no_movie_is_skipped() {
    let mut c = Campaign::new(list(), diff());
    enter(&mut c); // a
    leave_solo(&mut c, "a", 0).unwrap();
    enter(&mut c); // b
    leave_solo(&mut c, "b", 1).unwrap(); // b has no outro: straight to load
    assert_eq!(c.stage(), Some(Stage::Load));
    // Level s has no movie at all: load goes straight to spawn.
    assert_eq!(
        c.progress(Progress::Loaded).unwrap(),
        Phase::Transition {
            from: Some(1),
            to: 3,
            stage: Stage::Spawn,
            movies: true,
        }
    );
    c.progress(Progress::Spawned).unwrap();
    leave_solo(&mut c, "s", 0).unwrap();
    assert_eq!(c.stage(), Some(Stage::Load));
    // Level c has an episode movie but no intro.
    assert_eq!(
        c.progress(Progress::Loaded).unwrap(),
        Phase::Transition {
            from: Some(3),
            to: 2,
            stage: Stage::EpisodeMovie,
            movies: true,
        }
    );
    assert_eq!(c.current_movie(), Some("EP2"));
    assert_eq!(
        c.progress(Progress::MovieDone).unwrap(),
        Phase::Transition {
            from: Some(3),
            to: 2,
            stage: Stage::Spawn,
            movies: true,
        }
    );
}

#[test]
fn an_event_that_does_not_end_the_stage_is_refused_and_changes_nothing() {
    let all = [Progress::MovieDone, Progress::Loaded, Progress::Spawned];
    let mut c = Campaign::new(list(), diff());
    let mut checked = Vec::new();
    while !matches!(c.phase(), Phase::Playing { .. }) {
        let stage = c.stage().unwrap();
        let before = c.clone();
        for ev in all.into_iter().filter(|ev| *ev != event_for(stage)) {
            assert_eq!(
                c.progress(ev).unwrap_err(),
                TransitionError::WrongStage { stage, got: ev }
            );
            assert_eq!(c, before);
        }
        checked.push(stage);
        c.progress(event_for(stage)).unwrap();
    }
    assert_eq!(
        checked,
        [Stage::Load, Stage::EpisodeMovie, Stage::Intro, Stage::Spawn]
    );
    // While playing there is nothing to progress.
    let before = c.clone();
    for ev in all {
        assert_eq!(
            c.progress(ev).unwrap_err(),
            TransitionError::NotInTransition
        );
    }
    assert_eq!(c, before);
}

// ---- leaving a level -----------------------------------------------------------------------------------------

#[test]
fn leaving_plays_the_outro_then_loads_then_plays_the_intro_then_spawns() {
    let mut c = playing(list());
    let d = leave_solo(&mut c, "a", 0).unwrap();
    assert_eq!(d.to.as_deref(), Some("b"));
    assert!(d.ghosted.is_empty() && d.dead.is_empty());
    assert!(!d.campaign_complete);
    assert_eq!(d.newly_unlocked, [Unlock::level("b")]);
    assert_eq!(
        c.phase(),
        Phase::Transition {
            from: Some(0),
            to: 1,
            stage: Stage::Outro,
            movies: true
        }
    );
    assert!(!c.difficulty().in_level());
    assert_eq!(c.current_level().id, "b");
    let movies = enter(&mut c);
    assert_eq!(movies, ["a_out", "b_in"]);
    assert_eq!(c.phase(), Phase::Playing { level: 1 });
    assert!(c.unlocks().level_unlocked("b"));
}

#[test]
fn a_level_without_an_outro_goes_straight_to_loading() {
    let mut c = playing(list());
    leave_solo(&mut c, "a", 0).unwrap();
    enter(&mut c);
    leave_solo(&mut c, "b", 0).unwrap();
    assert_eq!(c.stage(), Some(Stage::Load));
    assert_eq!(c.current_movie(), None);
}

#[test]
fn the_secret_exit_leads_to_the_secret_level_which_rejoins_the_main_path() {
    let mut c = playing(list());
    leave_solo(&mut c, "a", 0).unwrap();
    enter(&mut c);
    let d = leave_solo(&mut c, "b", 1).unwrap();
    assert_eq!(d.to.as_deref(), Some("s"));
    assert_eq!(d.newly_unlocked, [Unlock::level("s")]);
    let movies = enter(&mut c);
    assert!(movies.is_empty());
    assert_eq!(c.phase(), Phase::Playing { level: 3 });
    let d = leave_solo(&mut c, "s", 0).unwrap();
    assert_eq!(d.to.as_deref(), Some("c"));
    assert!(!d.campaign_complete);
    assert_eq!(enter(&mut c), ["EP2"]);
    assert_eq!(c.current_level().id, "c");
}

#[test]
fn the_last_level_with_an_outro_ends_in_two_steps() {
    let mut c = playing(list());
    leave_solo(&mut c, "a", 0).unwrap();
    enter(&mut c);
    leave_solo(&mut c, "b", 0).unwrap();
    enter(&mut c);
    let d = leave_solo(&mut c, "c", 0).unwrap();
    assert_eq!(d.to, None);
    assert!(d.campaign_complete);
    assert!(d.newly_unlocked.is_empty());
    assert_eq!(c.phase(), Phase::Ending { last: 2 });
    assert_eq!(c.stage(), Some(Stage::Outro));
    assert_eq!(c.current_movie(), Some("c_out"));
    assert_eq!(c.current_level().id, "c");
    let before = c.clone();
    for ev in [Progress::Loaded, Progress::Spawned] {
        assert_eq!(
            c.progress(ev).unwrap_err(),
            TransitionError::WrongStage {
                stage: Stage::Outro,
                got: ev
            }
        );
    }
    assert_eq!(c, before);
    assert_eq!(
        c.progress(Progress::MovieDone).unwrap(),
        Phase::Finished { last: 2 }
    );
    assert_eq!(c.stage(), None);
    assert_eq!(c.current_movie(), None);
    assert_eq!(c.current_level().id, "c");
}

#[test]
fn the_last_level_without_an_outro_finishes_at_once() {
    let mut c = playing(plain());
    leave_solo(&mut c, "a", 0).unwrap();
    enter(&mut c);
    let d = leave_solo(&mut c, "b", 0).unwrap();
    assert!(d.campaign_complete);
    assert_eq!(c.phase(), Phase::Finished { last: 1 });
}

#[test]
fn a_finished_campaign_accepts_nothing() {
    let mut c = playing(plain());
    leave_solo(&mut c, "a", 0).unwrap();
    enter(&mut c);
    leave_solo(&mut c, "b", 0).unwrap();
    let before = c.clone();
    assert_eq!(
        leave_solo(&mut c, "b", 0).unwrap_err(),
        TransitionError::NotPlaying
    );
    assert_eq!(
        c.progress(Progress::Loaded).unwrap_err(),
        TransitionError::NotInTransition
    );
    assert_eq!(c.restart().unwrap_err(), TransitionError::NotPlaying);
    c.advance(100);
    assert_eq!(c, before);
}

#[test]
fn a_one_level_campaign_plays_and_finishes() {
    let one = LevelList::new(vec![e("only", "e").with_next(Next::End)]).unwrap();
    let mut c = playing(one);
    let d = leave_solo(&mut c, "only", 0).unwrap();
    assert!(d.campaign_complete);
    assert_eq!(c.phase(), Phase::Finished { last: 0 });
}

#[test]
fn leaving_is_refused_when_the_rules_say_so_and_nothing_changes() {
    let rules = UnlockRules::none();
    let go = |c: &mut Campaign, gate: &ExitGate, carry: CarryOver, r: LevelResult| {
        c.leave(LeaveRequest {
            gate,
            carry,
            result: r,
            rules: &rules,
        })
    };
    let mut c = playing(list());
    let before = c.clone();

    // The wrong level.
    assert_eq!(
        go(&mut c, &solo_gate(), solo_carry(), result("b", 0)).unwrap_err(),
        TransitionError::WrongLevel {
            expected: "a".into(),
            got: "b".into()
        }
    );
    // An exit the level does not have.
    assert_eq!(
        go(&mut c, &solo_gate(), solo_carry(), result("a", 1)).unwrap_err(),
        TransitionError::NoSuchExit {
            id: "a".into(),
            exit: 1
        }
    );
    // The gate says wait (nobody at the exit), then nobody alive.
    let idle = ExitGate::new(1, 0).unwrap();
    assert_eq!(
        go(&mut c, &idle, solo_carry(), result("a", 0)).unwrap_err(),
        TransitionError::Waiting { remaining: None }
    );
    let mut dead = ExitGate::new(1, 0).unwrap();
    dead.set_status(0, PlayerStatus::Dead).unwrap();
    assert_eq!(
        go(&mut c, &dead, solo_carry(), result("a", 0)).unwrap_err(),
        TransitionError::NobodyAlive
    );
    // Waiting for the others, with the time left.
    let mut two = ExitGate::new(2, 100).unwrap();
    two.set_status(0, PlayerStatus::AtExit).unwrap();
    two.advance(30);
    assert_eq!(
        go(&mut c, &two, solo_carry(), result("a", 0)).unwrap_err(),
        TransitionError::Waiting {
            remaining: Some(70)
        }
    );
    // An impossible result.
    let mut bad = result("a", 0);
    bad.secrets_found = 2;
    bad.secrets_total = 1;
    assert_eq!(
        go(&mut c, &solo_gate(), solo_carry(), bad).unwrap_err(),
        TransitionError::BadResult
    );
    // A living player with no carried state, and a dead entry for a living player.
    assert_eq!(
        go(&mut c, &solo_gate(), CarryOver::new(), result("a", 0)).unwrap_err(),
        TransitionError::MissingCarry { slot: 0 }
    );
    let mut dead_entry = CarryOver::new();
    dead_entry.set(0, Slot::Dead).unwrap();
    assert_eq!(
        go(&mut c, &solo_gate(), dead_entry, result("a", 0)).unwrap_err(),
        TransitionError::MissingCarry { slot: 0 }
    );
    // A slot that the gate does not have.
    let mut extra = solo_carry();
    extra.set(1, Slot::Dead).unwrap();
    assert_eq!(
        go(&mut c, &solo_gate(), extra, result("a", 0)).unwrap_err(),
        TransitionError::UnknownSlot { slot: 1 }
    );
    assert_eq!(c, before, "a refused leave must change nothing");

    // Not while a transition is in progress.
    leave_solo(&mut c, "a", 0).unwrap();
    let mid = c.clone();
    assert_eq!(
        go(&mut c, &solo_gate(), solo_carry(), result("a", 0)).unwrap_err(),
        TransitionError::NotPlaying
    );
    assert_eq!(c, mid);
}

#[test]
fn the_unlock_set_being_full_refuses_the_leave_and_changes_nothing() {
    let mut note = playing(list()).note().unwrap();
    note.unlocks
        .grant((0..MAX_UNLOCKS - 1).map(|i| Unlock::cheat(&format!("c{i}"))))
        .unwrap();
    let mut c = Campaign::restore(list(), &note).unwrap();
    enter(&mut c);
    assert_eq!(c.unlocks().len(), MAX_UNLOCKS);
    let before = c.clone();
    // Leaving would unlock b, which is one thing too many.
    assert_eq!(
        leave_solo(&mut c, "a", 0).unwrap_err(),
        TransitionError::Unlock(UnlockError::TooManyUnlocks)
    );
    assert_eq!(c, before);
}

#[test]
fn unlock_rules_are_applied_on_leaving() {
    let l = list();
    let rules = UnlockRules::new(
        vec![UnlockRule {
            when: Trigger::SecretsFound {
                level: "a".into(),
                at_least: 2,
            },
            grants: vec![Unlock::cheat("god")],
        }],
        &l,
    )
    .unwrap();
    let mut c = playing(l);
    let gate = solo_gate();
    let mut r = result("a", 0);
    r.secrets_found = 2;
    r.secrets_total = 3;
    let d = c
        .leave(LeaveRequest {
            gate: &gate,
            carry: solo_carry(),
            result: r,
            rules: &rules,
        })
        .unwrap();
    assert_eq!(d.newly_unlocked, [Unlock::level("b"), Unlock::cheat("god")]);
    assert!(c.unlocks().contains(&Unlock::cheat("god")));
    assert!(c.unlocks().level_unlocked("a") && c.unlocks().level_unlocked("b"));
    assert_eq!(c.unlocks().len(), 3);
}

// ---- carry-over ----------------------------------------------------------------------------------------------

#[test]
fn what_the_player_leaves_with_is_what_the_next_level_is_entered_with() {
    let mut c = playing(list());
    assert!(c.entry_carry().is_empty());
    leave_solo(&mut c, "a", 0).unwrap();
    // During the transition the pending carry is what counts.
    assert_eq!(
        c.pending_carry().get(0),
        Some(&Slot::Carried(carry_of(60, "shotgun")))
    );
    let during = c.start_inventory(0, &start(), CarryPolicy::Keep);
    assert_eq!(during.health, 100); // raised to the chapter start
    assert_eq!(during.preferred_weapon.as_deref(), Some("shotgun"));
    assert_eq!(during.ammo["bullets"], 50);
    assert!(during.weapons.contains_key("shotgun") && during.weapons.contains_key("colt"));
    enter(&mut c);
    assert_eq!(
        c.entry_carry().get(0),
        Some(&Slot::Carried(carry_of(60, "shotgun")))
    );
    assert_eq!(
        c.start_inventory(0, &start(), CarryPolicy::Keep),
        during,
        "the same answer while playing"
    );
    assert_eq!(c.start_inventory(0, &start(), CarryPolicy::Reset), start());
}

#[test]
fn dead_players_start_over_late_joiners_too_and_disconnected_players_are_dropped() {
    let mut c = playing(list());
    let mut gate = ExitGate::new(4, 0).unwrap();
    gate.set_status(0, PlayerStatus::AtExit).unwrap();
    gate.set_status(1, PlayerStatus::Dead).unwrap();
    gate.set_status(2, PlayerStatus::Gone).unwrap();
    gate.set_status(3, PlayerStatus::AtExit).unwrap();
    let mut carry = CarryOver::new();
    carry
        .set(0, Slot::Carried(carry_of(80, "shotgun")))
        .unwrap();
    carry.set(1, Slot::Carried(carry_of(0, "rocket"))).unwrap(); // the gate wins: dead
    carry.set(2, Slot::Carried(carry_of(70, "laser"))).unwrap(); // gone: dropped
    carry.set(3, Slot::Carried(carry_of(90, "sniper"))).unwrap();
    let rules = UnlockRules::none();
    let d = c
        .leave(LeaveRequest {
            gate: &gate,
            carry,
            result: result("a", 0),
            rules: &rules,
        })
        .unwrap();
    assert_eq!(d.dead, [1]);
    assert!(d.ghosted.is_empty());
    let p = c.pending_carry();
    assert_eq!(p.len(), 3);
    assert_eq!(p.get(1), Some(&Slot::Dead));
    assert_eq!(p.get(2), None);
    assert_eq!(c.start_inventory(1, &start(), CarryPolicy::Keep), start());
    assert_eq!(c.start_inventory(2, &start(), CarryPolicy::Keep), start());
    // A player who joins later (slot 9) starts from the chapter too.
    assert_eq!(c.start_inventory(9, &start(), CarryPolicy::Keep), start());
    assert_eq!(
        c.start_inventory(3, &start(), CarryPolicy::Keep)
            .preferred_weapon
            .as_deref(),
        Some("sniper")
    );
}

#[test]
fn a_ghosted_player_is_carried_along_with_the_state_they_had() {
    let mut c = playing(list());
    let mut gate = ExitGate::new(3, 50).unwrap();
    gate.set_status(0, PlayerStatus::AtExit).unwrap();
    let mut carry = CarryOver::new();
    for (s, w) in [(0u8, "colt"), (1, "shotgun"), (2, "laser")] {
        carry
            .set(s, Slot::Carried(carry_of(40 + u32::from(s), w)))
            .unwrap();
    }
    let rules = UnlockRules::none();
    // Before the timeout the gate waits.
    gate.advance(49);
    assert_eq!(
        c.leave(LeaveRequest {
            gate: &gate,
            carry: carry.clone(),
            result: result("a", 0),
            rules: &rules
        })
        .unwrap_err(),
        TransitionError::Waiting { remaining: Some(1) }
    );
    gate.advance(1);
    let d = c
        .leave(LeaveRequest {
            gate: &gate,
            carry: carry.clone(),
            result: result("a", 0),
            rules: &rules,
        })
        .unwrap();
    assert_eq!(d.ghosted, [1, 2]);
    assert_eq!(c.pending_carry(), &carry);
}

#[test]
fn a_restart_returns_to_what_the_level_was_entered_with() {
    let mut c = playing(list());
    leave_solo(&mut c, "a", 0).unwrap();
    enter(&mut c);
    let entered = c.entry_carry().clone();
    assert!(!entered.is_empty());
    c.advance(500);
    c.restart().unwrap();
    assert_eq!(c.restarts(), 1);
    assert_eq!(
        c.phase(),
        Phase::Transition {
            from: None,
            to: 1,
            stage: Stage::Load,
            movies: false
        }
    );
    assert_eq!(c.pending_carry(), &entered);
    assert!(!c.difficulty().in_level());
    // No movie plays: load goes straight to spawn although b has an intro.
    assert_eq!(c.current_movie(), None);
    assert_eq!(
        c.progress(Progress::Loaded).unwrap(),
        Phase::Transition {
            from: None,
            to: 1,
            stage: Stage::Spawn,
            movies: false
        }
    );
    assert_eq!(c.current_movie(), None);
    c.progress(Progress::Spawned).unwrap();
    assert_eq!(c.phase(), Phase::Playing { level: 1 });
    assert_eq!(c.entry_carry(), &entered);
    assert!(c.difficulty().in_level());
    // The time of the failed attempt still counts.
    assert_eq!(c.elapsed_ticks(), 500);
    // Restarting twice counts twice.
    c.restart().unwrap();
    enter(&mut c);
    assert_eq!(c.restarts(), 2);
}

#[test]
fn a_restart_is_refused_unless_a_level_is_being_played() {
    let mut c = Campaign::new(list(), diff());
    let before = c.clone();
    assert_eq!(c.restart().unwrap_err(), TransitionError::NotPlaying);
    assert_eq!(c, before);
}

#[test]
fn restarting_the_first_level_returns_to_nothing_carried() {
    let mut c = playing(list());
    c.restart().unwrap();
    enter(&mut c);
    assert!(c.entry_carry().is_empty());
    assert_eq!(c.start_inventory(0, &start(), CarryPolicy::Keep), start());
}

// ---- difficulty ----------------------------------------------------------------------------------------------

#[test]
fn the_difficulty_carries_through_every_transition_and_may_change_only_between_levels() {
    let mut c = Campaign::new(list(), diff());
    // Before the first level: between levels, a change is accepted.
    assert_eq!(c.request_difficulty(Difficulty::Easy), Ok(true));
    assert_eq!(c.request_difficulty(Difficulty::Easy), Ok(false));
    enter(&mut c);
    assert_eq!(
        c.request_difficulty(Difficulty::Serious),
        Err(DifficultyError::ChangeRefused(
            ChangeRefusal::LevelInProgress
        ))
    );
    assert_eq!(c.difficulty().level(), Difficulty::Easy);
    leave_solo(&mut c, "a", 0).unwrap();
    assert_eq!(c.request_difficulty(Difficulty::Normal), Ok(true));
    enter(&mut c);
    assert_eq!(c.difficulty().level(), Difficulty::Normal);
    assert!(c.difficulty().in_level());
    // During a restart too.
    c.restart().unwrap();
    assert_eq!(c.request_difficulty(Difficulty::Hard), Ok(true));
    enter(&mut c);
    assert_eq!(c.difficulty().level(), Difficulty::Hard);
}

#[test]
fn the_change_policy_is_honoured() {
    let mut locked = Campaign::new(
        list(),
        CampaignDifficulty::new(Difficulty::Normal, ChangePolicy::Locked),
    );
    assert_eq!(
        locked.request_difficulty(Difficulty::Easy),
        Err(DifficultyError::ChangeRefused(ChangeRefusal::Locked))
    );
    enter(&mut locked);
    leave_solo(&mut locked, "a", 0).unwrap();
    assert!(locked.request_difficulty(Difficulty::Easy).is_err());
    assert_eq!(locked.difficulty().level(), Difficulty::Normal);

    let mut any = Campaign::new(
        list(),
        CampaignDifficulty::new(Difficulty::Normal, ChangePolicy::Anytime),
    );
    enter(&mut any);
    assert_eq!(any.request_difficulty(Difficulty::Serious), Ok(true));
    leave_solo(&mut any, "a", 0).unwrap();
    enter(&mut any);
    assert_eq!(any.difficulty().level(), Difficulty::Serious);
}

// ---- time ----------------------------------------------------------------------------------------------------

#[test]
fn time_only_counts_while_a_level_is_being_played() {
    let mut c = Campaign::new(list(), diff());
    c.advance(10);
    assert_eq!(c.elapsed_ticks(), 0);
    enter(&mut c);
    c.advance(10);
    c.advance(5);
    assert_eq!(c.elapsed_ticks(), 15);
    leave_solo(&mut c, "a", 0).unwrap();
    c.advance(1_000);
    assert_eq!(c.elapsed_ticks(), 15);
    enter(&mut c);
    c.advance(1);
    assert_eq!(c.elapsed_ticks(), 16);
    c.advance(u64::MAX);
    assert_eq!(c.elapsed_ticks(), u64::MAX);
    c.advance(1);
    assert_eq!(c.elapsed_ticks(), u64::MAX);
}

// ---- the whole campaign --------------------------------------------------------------------------------------

#[test]
fn the_movies_of_a_whole_campaign_play_in_order() {
    let mut c = Campaign::new(list(), diff());
    let mut all = enter(&mut c);
    leave_solo(&mut c, "a", 0).unwrap();
    all.extend(enter(&mut c));
    leave_solo(&mut c, "b", 1).unwrap();
    all.extend(enter(&mut c));
    leave_solo(&mut c, "s", 0).unwrap();
    all.extend(enter(&mut c));
    leave_solo(&mut c, "c", 0).unwrap();
    all.extend(enter(&mut c));
    assert_eq!(all, ["EP1", "a_in", "a_out", "b_in", "EP2", "c_out"]);
    assert_eq!(c.phase(), Phase::Finished { last: 2 });
    for id in ["a", "b", "c", "s"] {
        assert!(c.unlocks().level_unlocked(id), "{id}");
    }
}

fn script() -> Campaign {
    let mut c = Campaign::new(list(), diff());
    enter(&mut c);
    c.advance(120);
    c.restart().unwrap();
    enter(&mut c);
    c.advance(60);
    leave_solo(&mut c, "a", 0).unwrap();
    enter(&mut c);
    c.advance(33);
    c
}

#[test]
fn the_same_events_give_the_same_campaign_and_the_same_bytes() {
    let (x, y) = (script(), script());
    assert_eq!(x, y);
    assert_eq!(x.note().unwrap().to_bytes(), y.note().unwrap().to_bytes());
    assert_eq!(x.elapsed_ticks(), 213);
    assert_eq!(x.restarts(), 1);
}

// ---- the save note -------------------------------------------------------------------------------------------

#[test]
fn a_note_is_only_written_while_playing_or_when_finished() {
    let mut c = Campaign::new(list(), diff());
    assert_eq!(c.note().unwrap_err(), TransitionError::NotSaveable);
    enter(&mut c);
    assert!(c.note().is_ok());
    leave_solo(&mut c, "a", 0).unwrap();
    assert_eq!(c.note().unwrap_err(), TransitionError::NotSaveable);
    enter(&mut c);
    leave_solo(&mut c, "b", 0).unwrap();
    enter(&mut c);
    leave_solo(&mut c, "c", 0).unwrap();
    assert_eq!(c.phase(), Phase::Ending { last: 2 });
    assert_eq!(c.note().unwrap_err(), TransitionError::NotSaveable);
    c.progress(Progress::MovieDone).unwrap();
    let n = c.note().unwrap();
    assert!(n.finished);
    assert_eq!(n.level, "c");
}

#[test]
fn a_note_has_everything_the_campaign_remembers() {
    let c = script();
    let n = c.note().unwrap();
    assert_eq!(n.fingerprint, list().fingerprint());
    assert_eq!(n.level, "b");
    assert_eq!(n.difficulty, *c.difficulty());
    assert!(n.difficulty.in_level());
    assert_eq!(&n.carry, c.entry_carry());
    assert_eq!(&n.unlocks, c.unlocks());
    assert_eq!(n.elapsed_ticks, 213);
    assert_eq!(n.restarts, 1);
    assert!(!n.finished);
}

#[test]
fn a_campaign_comes_back_from_its_note_and_re_enters_the_level_without_movies() {
    let c = script();
    let note = c.note().unwrap();
    let bytes = note.to_bytes();
    let mut back = Campaign::restore_bytes(list(), &bytes).unwrap();
    assert_eq!(
        back.phase(),
        Phase::Transition {
            from: None,
            to: 1,
            stage: Stage::Load,
            movies: false
        }
    );
    assert!(!back.difficulty().in_level());
    assert_eq!(back.elapsed_ticks(), 213);
    assert_eq!(back.restarts(), 1);
    assert_eq!(back.unlocks(), c.unlocks());
    assert_eq!(back.pending_carry(), c.entry_carry());
    let movies = enter(&mut back);
    assert!(movies.is_empty());
    assert_eq!(back.phase(), Phase::Playing { level: 1 });
    // After re-entering, the note is the same as the one it came from.
    assert_eq!(back.note().unwrap(), note);
    assert_eq!(back.note().unwrap().to_bytes(), bytes);
    // And the restored campaign carries on like the original.
    let mut orig = c.clone();
    let d1 = leave_solo(&mut orig, "b", 1).unwrap();
    let d2 = leave_solo(&mut back, "b", 1).unwrap();
    assert_eq!(d1, d2);
    assert_eq!(enter(&mut orig), enter(&mut back));
    assert_eq!(orig.note().unwrap(), back.note().unwrap());
}

#[test]
fn a_finished_campaign_comes_back_finished() {
    let mut c = playing(plain());
    leave_solo(&mut c, "a", 0).unwrap();
    enter(&mut c);
    leave_solo(&mut c, "b", 0).unwrap();
    let back = Campaign::restore_bytes(plain(), &c.note().unwrap().to_bytes()).unwrap();
    assert_eq!(back.phase(), Phase::Finished { last: 1 });
    assert_eq!(back.note().unwrap(), c.note().unwrap());
}

#[test]
fn a_note_is_refused_with_another_list_an_unknown_level_or_an_unknown_unlock() {
    let c = script();
    let note = c.note().unwrap();
    // Another list.
    let other = plain();
    assert_eq!(
        Campaign::restore(other.clone(), &note).unwrap_err(),
        RestoreError::WrongList {
            saved: list().fingerprint(),
            current: other.fingerprint()
        }
    );
    // A level that is not in the list.
    let mut bad = note.clone();
    bad.level = "zz".into();
    assert_eq!(
        Campaign::restore(list(), &bad).unwrap_err(),
        RestoreError::UnknownLevel { id: "zz".into() }
    );
    // An unlocked level that is not in the list, but a cheat or a mode with that name is fine.
    let mut bad = note.clone();
    bad.unlocks.grant([Unlock::level("zz")]).unwrap();
    assert_eq!(
        Campaign::restore(list(), &bad).unwrap_err(),
        RestoreError::UnknownUnlock { id: "zz".into() }
    );
    let mut ok = note;
    ok.unlocks
        .grant([Unlock::cheat("zz"), Unlock::mode("zz")])
        .unwrap();
    assert!(Campaign::restore(list(), &ok).is_ok());
}

#[test]
fn every_prefix_and_every_flipped_byte_of_a_note_is_refused() {
    let b = script().note().unwrap().to_bytes();
    for n in 0..b.len() {
        assert!(
            Campaign::restore_bytes(list(), &b[..n]).is_err(),
            "prefix {n}"
        );
        assert!(CampaignNote::from_bytes(&b[..n]).is_err(), "prefix {n}");
    }
    for i in 0..b.len() {
        for bit in [0x01u8, 0x20, 0x80] {
            let mut d = b.clone();
            d[i] ^= bit;
            assert!(CampaignNote::from_bytes(&d).is_err(), "byte {i} ^ {bit:#x}");
            assert!(
                Campaign::restore_bytes(list(), &d).is_err(),
                "byte {i} ^ {bit:#x}"
            );
        }
    }
}

fn reseal(bytes: &mut [u8]) {
    let n = bytes.len() - 4;
    let crc = crate::level_progress::crc32(&bytes[..n]);
    bytes[n..].copy_from_slice(&crc.to_le_bytes());
}

#[test]
fn damage_under_a_valid_checksum_is_refused_with_the_right_error() {
    let note = CampaignNote {
        fingerprint: 7,
        level: "ab".into(),
        difficulty: diff(),
        carry: CarryOver::new(),
        unlocks: UnlockSet::new(),
        elapsed_ticks: 1,
        restarts: 2,
        finished: false,
    };
    let good = note.to_bytes();
    assert_eq!(CampaignNote::from_bytes(&good).unwrap(), note);
    let fix = |f: &dyn Fn(&mut Vec<u8>)| {
        let mut d = good.clone();
        f(&mut d);
        reseal(&mut d);
        CampaignNote::from_bytes(&d)
    };
    assert_eq!(fix(&|d| d[3] = b'X').unwrap_err(), BytesError::WrongMagic);
    assert_eq!(
        fix(&|d| d[4] = 3).unwrap_err(),
        BytesError::UnknownVersion { found: 3 }
    );
    // Layout: 6 header, fingerprint u32 (6..10), level: u16 length (10..12) + "ab" (12..14), difficulty 4 (14..18).
    assert_eq!(
        fix(&|d| d[10..12].copy_from_slice(&65u16.to_le_bytes())).unwrap_err(),
        BytesError::Invalid("a text is too long")
    );
    assert_eq!(
        fix(&|d| d[15] = 9).unwrap_err(),
        BytesError::Invalid("the difficulty is not valid")
    );
    assert_eq!(
        fix(&|d| d[14] = 2).unwrap_err(),
        BytesError::Invalid("the difficulty is not valid")
    );
    // The finished flag is the last byte before the checksum.
    assert_eq!(
        fix(&|d| {
            let n = d.len() - 5;
            d[n] = 2;
        })
        .unwrap_err(),
        BytesError::Invalid("a flag is not 0 or 1")
    );
    assert_eq!(
        fix(&|d| {
            let n = d.len() - 4;
            d.insert(n, 0);
        })
        .unwrap_err(),
        BytesError::Trailing
    );
    // A level text that is not UTF-8.
    assert_eq!(
        fix(&|d| d[12] = 0xFF).unwrap_err(),
        BytesError::Invalid("a text is not UTF-8")
    );
}

#[test]
fn pseudo_random_garbage_never_panics_when_reading_a_note() {
    let good = script().note().unwrap().to_bytes();
    let mut x: u64 = 0x1234_5678_9ABC_DEF1;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for _ in 0..2_000 {
        let mut d = good.clone();
        for _ in 0..(next() % 4 + 1) {
            let i = (next() % d.len() as u64) as usize;
            d[i] = next() as u8;
        }
        if next() % 2 == 0 {
            reseal(&mut d);
        }
        let _ = Campaign::restore_bytes(list(), &d);
    }
}

// ---- the texts -----------------------------------------------------------------------------------------------

#[test]
fn errors_say_what_is_wrong() {
    let t = [
        (TransitionError::NotPlaying, "no level"),
        (TransitionError::NotInTransition, "no transition"),
        (
            TransitionError::WrongStage {
                stage: Stage::Load,
                got: Progress::Spawned,
            },
            "Load",
        ),
        (
            TransitionError::WrongLevel {
                expected: "a".into(),
                got: "b".into(),
            },
            "`b`",
        ),
        (
            TransitionError::NoSuchExit {
                id: "a".into(),
                exit: 4,
            },
            "exit 4",
        ),
        (
            TransitionError::Waiting {
                remaining: Some(12),
            },
            "12 ticks",
        ),
        (
            TransitionError::Waiting { remaining: None },
            "nobody is at the exit",
        ),
        (TransitionError::NobodyAlive, "nobody is alive"),
        (TransitionError::MissingCarry { slot: 3 }, "player 3"),
        (TransitionError::UnknownSlot { slot: 5 }, "slot 5"),
        (TransitionError::BadResult, "secrets"),
        (
            TransitionError::Unlock(UnlockError::NoGrants),
            "grants nothing",
        ),
        (TransitionError::NotSaveable, "not written"),
    ];
    for (e, needle) in t {
        assert!(e.to_string().contains(needle), "{e}");
    }
    let r = [
        (
            RestoreError::WrongList {
                saved: 0xAB,
                current: 0xCD,
            },
            "000000ab",
        ),
        (RestoreError::UnknownLevel { id: "q".into() }, "`q`"),
        (RestoreError::UnknownUnlock { id: "w".into() }, "`w`"),
        (RestoreError::Bytes(BytesError::Trailing), "left over"),
    ];
    for (e, needle) in r {
        assert!(e.to_string().contains(needle), "{e}");
    }
    assert!(std::error::Error::source(&TransitionError::NotPlaying).is_none());
    // Every error type of the module is nameable from here.
    let _ = CarryError::PreferredNotOwned;
    let _ = UnlockKind::Cheat;
}
