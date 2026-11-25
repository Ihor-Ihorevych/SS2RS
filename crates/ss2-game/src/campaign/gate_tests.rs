use super::PlayerStatus::{AtExit, Away, Dead, Gone};
use super::*;

fn gate(n: u8, timeout: u64) -> ExitGate {
    ExitGate::new(n, timeout).unwrap()
}

fn wait(remaining: Option<u64>) -> GateDecision {
    GateDecision::Wait { remaining }
}

fn go(ghosted: &[u8], dead: &[u8]) -> GateDecision {
    GateDecision::Go {
        ghosted: ghosted.to_vec(),
        dead: dead.to_vec(),
    }
}

#[test]
fn the_player_count_is_one_to_sixty_four() {
    assert!(ExitGate::new(1, 0).is_ok());
    assert!(ExitGate::new(MAX_PLAYERS, 0).is_ok());
    assert_eq!(ExitGate::new(0, 0).unwrap_err(), GateError::BadPlayerCount);
    assert_eq!(
        ExitGate::new(MAX_PLAYERS + 1, 0).unwrap_err(),
        GateError::BadPlayerCount
    );
    let g = gate(3, 77);
    assert_eq!(g.players(), 3);
    assert_eq!(g.timeout_ticks(), 77);
    assert_eq!(g.status(0), Some(Away));
    assert_eq!(g.status(2), Some(Away));
    assert_eq!(g.status(3), None);
}

#[test]
fn a_single_player_goes_as_soon_as_they_are_at_the_exit() {
    let mut g = gate(1, 600);
    assert_eq!(g.decision(), wait(None));
    g.set_status(0, AtExit).unwrap();
    assert_eq!(g.decision(), go(&[], &[]));
}

#[test]
fn everyone_alive_must_be_at_the_exit_before_the_timeout() {
    let mut g = gate(3, 100);
    g.set_status(0, AtExit).unwrap();
    assert_eq!(g.decision(), wait(Some(100)));
    g.set_status(1, AtExit).unwrap();
    assert_eq!(g.decision(), wait(Some(100)));
    g.advance(40);
    assert_eq!(g.decision(), wait(Some(60)));
    g.set_status(2, AtExit).unwrap();
    assert_eq!(g.decision(), go(&[], &[]));
}

#[test]
fn nothing_runs_until_somebody_is_at_the_exit() {
    let mut g = gate(2, 10);
    g.advance(1_000);
    assert_eq!(g.decision(), wait(None));
    g.set_status(0, AtExit).unwrap();
    // The clock did not run while nobody was there.
    assert_eq!(g.decision(), wait(Some(10)));
}

#[test]
fn the_timeout_ghosts_the_players_who_are_still_away() {
    let mut g = gate(4, 100);
    g.set_status(0, AtExit).unwrap();
    g.set_status(3, Dead).unwrap();
    g.advance(99);
    assert_eq!(g.decision(), wait(Some(1)));
    g.advance(1);
    assert_eq!(g.decision(), go(&[1, 2], &[3]));
    // Past the timeout it stays go.
    g.advance(1_000);
    assert_eq!(g.decision(), go(&[1, 2], &[3]));
}

#[test]
fn a_timeout_of_zero_does_not_wait() {
    let mut g = gate(3, 0);
    assert_eq!(g.decision(), wait(None));
    g.set_status(1, AtExit).unwrap();
    assert_eq!(g.decision(), go(&[0, 2], &[]));
}

#[test]
fn the_timer_restarts_when_nobody_is_at_the_exit() {
    let mut g = gate(2, 100);
    g.set_status(0, AtExit).unwrap();
    g.advance(90);
    g.set_status(0, Away).unwrap();
    g.set_status(0, AtExit).unwrap();
    assert_eq!(g.decision(), wait(Some(100)));
    // Another player arriving does not restart it.
    g.advance(30);
    g.set_status(1, AtExit).unwrap();
    g.set_status(1, Away).unwrap();
    assert_eq!(g.decision(), wait(Some(70)));
}

#[test]
fn leaving_the_exit_while_another_stays_keeps_the_timer() {
    let mut g = gate(2, 100);
    g.set_status(0, AtExit).unwrap();
    g.set_status(1, AtExit).unwrap();
    g.advance(60);
    g.set_status(1, Away).unwrap();
    assert_eq!(g.decision(), wait(Some(40)));
}

#[test]
fn dead_players_do_not_block_and_disconnected_players_do_not_count() {
    let mut g = gate(4, 1_000);
    g.set_status(0, AtExit).unwrap();
    g.set_status(1, Dead).unwrap();
    g.set_status(2, Gone).unwrap();
    g.set_status(3, AtExit).unwrap();
    assert_eq!(g.decision(), go(&[], &[1]));
}

#[test]
fn nobody_alive_never_opens_the_gate() {
    let mut g = gate(2, 0);
    g.set_status(0, Dead).unwrap();
    g.set_status(1, Gone).unwrap();
    assert_eq!(g.decision(), GateDecision::NobodyAlive);
    g.advance(10_000);
    assert_eq!(g.decision(), GateDecision::NobodyAlive);
    // A dead player at the exit does not count as arrived.
    let mut h = gate(2, 0);
    h.set_status(0, Dead).unwrap();
    assert_eq!(h.decision(), wait(None));
}

#[test]
fn a_player_who_dies_at_the_exit_stops_the_timer_if_they_were_the_only_one() {
    let mut g = gate(2, 100);
    g.set_status(0, AtExit).unwrap();
    g.advance(50);
    g.set_status(0, Dead).unwrap();
    g.set_status(0, AtExit).unwrap();
    assert_eq!(g.decision(), wait(Some(100)));
}

#[test]
fn an_unknown_slot_is_refused_and_nothing_changes() {
    let mut g = gate(2, 5);
    let before = g.clone();
    assert_eq!(
        g.set_status(2, AtExit).unwrap_err(),
        GateError::NoSuchPlayer { slot: 2 }
    );
    assert_eq!(g, before);
}

#[test]
fn the_timer_saturates() {
    let mut g = gate(2, u64::MAX);
    g.set_status(0, AtExit).unwrap();
    g.advance(u64::MAX);
    g.advance(u64::MAX);
    assert_eq!(g.decision(), go(&[1], &[]));
}

#[test]
fn the_same_events_give_the_same_gate() {
    let run = || {
        let mut g = gate(5, 50);
        g.set_status(2, AtExit).unwrap();
        g.advance(20);
        g.set_status(4, Dead).unwrap();
        g.advance(30);
        g
    };
    assert_eq!(run(), run());
    assert_eq!(run().decision(), go(&[0, 1, 3], &[4]));
}

#[test]
fn errors_say_what_is_wrong() {
    assert!(GateError::BadPlayerCount.to_string().contains("1 to 64"));
    assert!(
        GateError::NoSuchPlayer { slot: 9 }
            .to_string()
            .contains('9')
    );
}
