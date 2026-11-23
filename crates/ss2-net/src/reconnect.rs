//! **Reconnecting** and **dropping in and out** (roadmap T08-014, `docs/RECONNECT.md`): who keeps a seat when a connection drops, and how a client tries again.
//!
//! Pure logic like the rest of `ss2-net`: time is a number of microseconds passed in, there is no randomness (tokens are supplied by the caller) and no `HashMap`.
//!
//! * [`SeatKeeper`] is the server's side. When a player's connection **times out** their seat is **held** for a grace period under a secret **token** they were given at
//!   join. They can come back with that token and get the same slot (and so the same player), and until the hold ends nobody else is given the seat. A player who
//!   **quit, was kicked or banned**, or whose server stopped, loses the seat at once.
//! * [`Backoff`] is the client's side: when to try again after a drop or a refusal, with exponential delays, a limit on attempts and on total time, and refusals split into
//!   **permanent** (stop) and **transient** (go on, honouring "retry after").
//!
//! The session messages themselves are not changed by this module: carrying the token in `Welcome` and `Hello` is a protocol change that follow-up T08-014b makes.

use std::collections::BTreeMap;

use crate::session::{LeaveReason, RefuseReason, constant_time_eq};

/// A secret that proves a returning player is the one who left.
pub type Token = [u8; 16];

/// What the server keeps for a seat that is being held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reservation {
    /// The player's slot.
    pub slot: u8,
    /// Their name when they left (a returning player must use the same one).
    pub name: String,
    /// When the hold ends (microseconds).
    pub until_us: u64,
    /// The last tick the server knew the player had received (so it can decide between catching up and a full snapshot).
    pub last_acked_tick: u64,
    /// How many wrong tokens have been tried against this seat.
    pub wrong_tries: u32,
}

/// What came of a player asking to come back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Claim {
    /// Welcome back: this slot is yours again.
    Resume {
        /// The slot.
        slot: u8,
        /// The last tick the server knew the player had; anything newer must be re-sent.
        last_acked_tick: u64,
    },
    /// There is no seat held for that token (never issued, expired, or already taken back): treat the player as new.
    Unknown,
    /// The token is right but the name is not the one the seat was held for.
    WrongName,
    /// Too many wrong tokens were tried against held seats lately: wait.
    LockedOut,
}

/// What became of a seat when its player's connection ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The seat is held for the player to come back.
    Held,
    /// The seat is free again (the player left for good, or there was no room to hold it).
    Released,
    /// Nobody was seated there.
    NotSeated,
}

/// How the server holds seats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeatPolicy {
    /// How long a timed-out player's seat is held, in microseconds.
    pub grace_us: u64,
    /// How many seats may be held at once (a flood of dropped connections must not hold the whole server).
    pub max_held: usize,
    /// Wrong tokens tolerated in total before claims are refused (`LockedOut`) until the next hold ends or `reset_lockout`.
    pub max_wrong_tries: u32,
}

impl Default for SeatPolicy {
    /// Hold a seat 60 s, at most 8 seats at once, lock out after 10 wrong tokens.
    fn default() -> Self {
        Self {
            grace_us: 60_000_000,
            max_held: 8,
            max_wrong_tries: 10,
        }
    }
}

/// Keeps the seats of players whose connection dropped.
///
/// ```
/// use ss2_net::reconnect::{Claim, SeatKeeper, SeatPolicy};
/// use ss2_net::session::LeaveReason;
///
/// let mut seats = SeatKeeper::new(SeatPolicy::default());
/// let token = [7u8; 16];
/// seats.player_joined(3, "Sam", token);
/// // The connection times out: the seat is held.
/// seats.player_left(3, LeaveReason::TimedOut, 1_000_000, 480);
/// assert!(seats.is_held(3));
/// // Ten seconds later Sam is back with the token.
/// assert_eq!(seats.claim(&token, "Sam", 11_000_000), Claim::Resume { slot: 3, last_acked_tick: 480 });
/// assert!(!seats.is_held(3));
/// ```
#[derive(Clone, Debug)]
pub struct SeatKeeper {
    policy: SeatPolicy,
    /// Tokens of players who are connected now, by slot.
    active: BTreeMap<u8, (String, Token)>,
    /// Held seats by token.
    held: BTreeMap<Token, Reservation>,
    wrong_tries: u32,
}

impl SeatKeeper {
    /// A keeper that holds nothing.
    pub fn new(policy: SeatPolicy) -> Self {
        Self {
            policy,
            active: BTreeMap::new(),
            held: BTreeMap::new(),
            wrong_tries: 0,
        }
    }

    /// A player was admitted into `slot` with this `token` (the keeper does not make tokens: the caller supplies unguessable ones). A token already in use for
    /// another seat is **replaced** for the old seat: the old holder loses it, which keeps tokens unique.
    pub fn player_joined(&mut self, slot: u8, name: &str, token: Token) {
        self.held.remove(&token);
        self.active.retain(|s, (_, t)| *s == slot || *t != token);
        self.active.insert(slot, (name.to_owned(), token));
    }

    /// A player's connection ended. A [`TimedOut`](LeaveReason::TimedOut) player's seat is held for the grace period (if there is room among the held seats); any other
    /// reason releases the seat. `last_acked_tick` is the last tick the player is known to have received. A slot nobody was seated in is [`Outcome::NotSeated`].
    pub fn player_left(
        &mut self,
        slot: u8,
        reason: LeaveReason,
        now_us: u64,
        last_acked_tick: u64,
    ) -> Outcome {
        let Some((name, token)) = self.active.remove(&slot) else {
            return Outcome::NotSeated;
        };
        if reason != LeaveReason::TimedOut || self.held.len() >= self.policy.max_held {
            return Outcome::Released;
        }
        self.held.insert(
            token,
            Reservation {
                slot,
                name,
                until_us: now_us.saturating_add(self.policy.grace_us),
                last_acked_tick,
                wrong_tries: 0,
            },
        );
        Outcome::Held
    }

    /// Whether a seat is currently held for a returning player.
    pub fn is_held(&self, slot: u8) -> bool {
        self.held.values().any(|r| r.slot == slot)
    }

    /// The slots held, in order. A new player must not be given any of them.
    pub fn held_slots(&self) -> Vec<u8> {
        let mut slots: Vec<u8> = self.held.values().map(|r| r.slot).collect();
        slots.sort_unstable();
        slots
    }

    /// The reservation for a slot, if held.
    pub fn reservation(&self, slot: u8) -> Option<&Reservation> {
        self.held.values().find(|r| r.slot == slot)
    }

    /// How many seats are held.
    pub fn held_count(&self) -> usize {
        self.held.len()
    }

    /// The lowest slot below `max_players` that is neither in use nor held (the slot a **new** player may take), or `None`.
    pub fn free_slot(&self, max_players: u8) -> Option<u8> {
        (0..max_players).find(|s| !self.active.contains_key(s) && !self.is_held(*s))
    }

    /// End the holds that have run out and return their slots, in order. Call every tick.
    pub fn expire(&mut self, now_us: u64) -> Vec<u8> {
        let mut gone: Vec<(u8, Token)> = self
            .held
            .iter()
            .filter(|(_, r)| r.until_us <= now_us)
            .map(|(t, r)| (r.slot, *t))
            .collect();
        gone.sort_unstable();
        for (_, token) in &gone {
            self.held.remove(token);
        }
        if !gone.is_empty() && self.held.is_empty() {
            self.wrong_tries = 0;
        }
        gone.into_iter().map(|(slot, _)| slot).collect()
    }

    /// A returning player presents their `token` and `name`. The token is compared in constant time against every held seat, so the time does not say how close a guess was.
    ///
    /// A wrong token counts toward a lockout; a right token with the wrong name does not resume (and counts too, since someone holds a token that is not theirs). A hold
    /// that ran out is not honoured even if [`expire`](Self::expire) has not run yet.
    pub fn claim(&mut self, token: &Token, name: &str, now_us: u64) -> Claim {
        if self.wrong_tries >= self.policy.max_wrong_tries {
            return Claim::LockedOut;
        }
        let mut found: Option<Token> = None;
        for t in self.held.keys() {
            // No early exit: every held seat is compared.
            if constant_time_eq(t, token) {
                found = Some(*t);
            }
        }
        let Some(t) = found else {
            if !self.held.is_empty() {
                self.wrong_tries += 1;
            }
            return Claim::Unknown;
        };
        let r = &self.held[&t];
        if r.until_us <= now_us {
            self.held.remove(&t);
            return Claim::Unknown;
        }
        if !r.name.eq_ignore_ascii_case(name) {
            self.wrong_tries += 1;
            if let Some(r) = self.held.get_mut(&t) {
                r.wrong_tries += 1;
            }
            return Claim::WrongName;
        }
        let r = self.held.remove(&t).expect("found above");
        self.active.insert(r.slot, (r.name, t));
        Claim::Resume {
            slot: r.slot,
            last_acked_tick: r.last_acked_tick,
        }
    }

    /// Forget the wrong tries (the host decided to).
    pub fn reset_lockout(&mut self) {
        self.wrong_tries = 0;
    }

    /// Total wrong tries counted toward the lockout.
    pub fn wrong_tries(&self) -> u32 {
        self.wrong_tries
    }
}

/// Why a refusal ends the reconnecting.
fn is_permanent(reason: &RefuseReason) -> bool {
    !matches!(
        reason,
        RefuseReason::ServerFull | RefuseReason::RateLimited { .. } | RefuseReason::NameTaken
    )
}

/// How a client retries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackoffPolicy {
    /// The first wait, in microseconds.
    pub first_us: u64,
    /// The longest wait between attempts.
    pub max_wait_us: u64,
    /// How many attempts at most (the first counts).
    pub max_attempts: u32,
    /// Give up this long after the connection dropped.
    pub give_up_after_us: u64,
}

impl Default for BackoffPolicy {
    /// 0.5 s first, doubling to at most 8 s, 12 attempts, and give up after 90 s (a little longer than the 60 s seat hold).
    fn default() -> Self {
        Self {
            first_us: 500_000,
            max_wait_us: 8_000_000,
            max_attempts: 12,
            give_up_after_us: 90_000_000,
        }
    }
}

/// What the client should do now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retry {
    /// Try again now.
    Now,
    /// Wait; ask again at this time.
    WaitUntil(u64),
    /// Stop: reconnecting is over (the reason is in [`Backoff::stopped`]).
    GiveUp,
}

/// Why reconnecting ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stopped {
    /// The server refused for a reason that will not change.
    Refused(RefuseReason),
    /// The attempt limit was reached.
    TooManyAttempts,
    /// The time limit was reached.
    TooLong,
}

/// The client's reconnect schedule.
///
/// ```
/// use ss2_net::reconnect::{Backoff, BackoffPolicy, Retry};
///
/// let mut b = Backoff::new(BackoffPolicy::default(), 0); // the connection dropped at time 0
/// assert_eq!(b.poll(0), Retry::Now);
/// b.attempt_failed(0);
/// assert_eq!(b.poll(100_000), Retry::WaitUntil(500_000));  // 0.5 s after the first attempt
/// assert_eq!(b.poll(500_000), Retry::Now);
/// ```
#[derive(Clone, Debug)]
pub struct Backoff {
    policy: BackoffPolicy,
    dropped_at: u64,
    attempts: u32,
    next_at: u64,
    stopped: Option<Stopped>,
}

impl Backoff {
    /// A schedule for a connection that dropped at `dropped_at_us`; the first attempt is due at once.
    pub fn new(policy: BackoffPolicy, dropped_at_us: u64) -> Self {
        Self {
            policy,
            dropped_at: dropped_at_us,
            attempts: 0,
            next_at: dropped_at_us,
            stopped: None,
        }
    }

    /// How many attempts have failed.
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Why it stopped, once it has.
    pub fn stopped(&self) -> Option<&Stopped> {
        self.stopped.as_ref()
    }

    /// The wait after the `n`th failed attempt (n from 1): `first`, `2 x first`, ... capped at `max_wait`.
    fn wait_after(&self, n: u32) -> u64 {
        let shift = (n - 1).min(40);
        self.policy
            .first_us
            .saturating_mul(1u64 << shift)
            .min(self.policy.max_wait_us)
    }

    /// What to do at time `now_us`.
    pub fn poll(&mut self, now_us: u64) -> Retry {
        if self.stopped.is_some() {
            return Retry::GiveUp;
        }
        if now_us.saturating_sub(self.dropped_at) >= self.policy.give_up_after_us {
            self.stopped = Some(Stopped::TooLong);
            return Retry::GiveUp;
        }
        if self.attempts >= self.policy.max_attempts {
            self.stopped = Some(Stopped::TooManyAttempts);
            return Retry::GiveUp;
        }
        if now_us >= self.next_at {
            Retry::Now
        } else {
            Retry::WaitUntil(self.next_at)
        }
    }

    /// An attempt at `now_us` got no answer (a timeout): schedule the next.
    pub fn attempt_failed(&mut self, now_us: u64) {
        self.attempts += 1;
        self.next_at = now_us.saturating_add(self.wait_after(self.attempts));
    }

    /// An attempt at `now_us` was refused. A permanent refusal (a ban, a version or mod mismatch, a wrong password, a failed auth, an invalid name) ends it; a
    /// transient one (the server is full, the name is still in use by the old connection, a rate limit) schedules the next attempt, no sooner than the server's `retry_after`.
    pub fn refused(&mut self, now_us: u64, reason: &RefuseReason) {
        if is_permanent(reason) {
            self.stopped = Some(Stopped::Refused(reason.clone()));
            return;
        }
        self.attempts += 1;
        let mut at = now_us.saturating_add(self.wait_after(self.attempts));
        if let RefuseReason::RateLimited { retry_after_ms } = reason {
            at = at.max(now_us.saturating_add(u64::from(*retry_after_ms) * 1000));
        }
        self.next_at = at;
    }

    /// The connection is back: forget the failures and start over (for the next drop).
    pub fn connected(&mut self, now_us: u64) {
        *self = Self::new(self.policy, now_us);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(n: u8) -> Token {
        [n; 16]
    }

    fn keeper() -> SeatKeeper {
        SeatKeeper::new(SeatPolicy::default())
    }

    // ---- holding seats --------------------------------------------------------------------------------

    #[test]
    fn a_timed_out_player_keeps_the_seat_and_gets_it_back_with_the_token() {
        let mut k = keeper();
        k.player_joined(2, "Sam", token(1));
        assert_eq!(
            k.player_left(2, LeaveReason::TimedOut, 1_000, 480),
            Outcome::Held,
            "held, not released"
        );
        assert!(k.is_held(2) && k.held_count() == 1);
        assert_eq!(k.held_slots(), [2]);
        let r = k.reservation(2).unwrap();
        assert_eq!(
            (r.slot, r.name.as_str(), r.until_us, r.last_acked_tick),
            (2, "Sam", 1_000 + 60_000_000, 480)
        );
        assert_eq!(
            k.claim(&token(1), "sam", 5_000_000),
            Claim::Resume {
                slot: 2,
                last_acked_tick: 480
            },
            "the name matches without case"
        );
        assert!(!k.is_held(2));
        // Having resumed, the player can drop again and be held again.
        assert_eq!(
            k.player_left(2, LeaveReason::TimedOut, 9_000_000, 600),
            Outcome::Held
        );
        assert_eq!(
            k.claim(&token(1), "Sam", 9_500_000),
            Claim::Resume {
                slot: 2,
                last_acked_tick: 600
            }
        );
    }

    #[test]
    fn leaving_for_good_releases_the_seat_at_once() {
        for reason in [
            LeaveReason::Quit,
            LeaveReason::Kicked,
            LeaveReason::Banned,
            LeaveReason::ServerShutdown,
        ] {
            let mut k = keeper();
            k.player_joined(0, "A", token(9));
            assert_eq!(
                k.player_left(0, reason, 0, 1),
                Outcome::Released,
                "{reason:?}"
            );
            assert!(!k.is_held(0));
            assert_eq!(
                k.claim(&token(9), "A", 1),
                Claim::Unknown,
                "{reason:?}: the token is worthless now"
            );
        }
        // Nobody seated there: nothing to hold.
        assert_eq!(
            keeper().player_left(5, LeaveReason::TimedOut, 0, 0),
            Outcome::NotSeated
        );
        assert!(!keeper().is_held(5));
    }

    #[test]
    fn a_held_seat_is_not_given_to_a_new_player_until_the_hold_ends() {
        let mut k = keeper();
        k.player_joined(0, "A", token(1));
        k.player_joined(1, "B", token(2));
        k.player_left(0, LeaveReason::TimedOut, 0, 0);
        assert_eq!(k.free_slot(4), Some(2), "0 is held, 1 is in use");
        k.player_left(1, LeaveReason::Quit, 0, 0);
        assert_eq!(k.free_slot(4), Some(1), "1 was released, 0 is still held");
        assert_eq!(
            k.free_slot(1),
            None,
            "a one-seat server has only the held seat"
        );
        assert_eq!(k.expire(59_999_999), Vec::<u8>::new());
        assert_eq!(
            k.expire(60_000_000),
            [0],
            "the hold ends exactly at its time"
        );
        assert_eq!(k.free_slot(4), Some(0));
    }

    #[test]
    fn an_expired_hold_is_not_honoured_even_before_expire_runs() {
        let mut k = keeper();
        k.player_joined(3, "Sam", token(1));
        k.player_left(3, LeaveReason::TimedOut, 0, 5);
        assert_eq!(
            k.claim(&token(1), "Sam", 60_000_000),
            Claim::Unknown,
            "exactly at the end it is over"
        );
        assert!(!k.is_held(3), "and the stale reservation was dropped");
        k.player_joined(3, "Sam", token(1));
        k.player_left(3, LeaveReason::TimedOut, 100_000_000, 6);
        assert_eq!(
            k.claim(&token(1), "Sam", 159_999_999),
            Claim::Resume {
                slot: 3,
                last_acked_tick: 6
            },
            "one microsecond before the end is fine"
        );
    }

    #[test]
    fn expiry_reports_each_slot_once_in_order_and_leaves_the_others() {
        let mut k = SeatKeeper::new(SeatPolicy {
            grace_us: 10,
            ..SeatPolicy::default()
        });
        for (slot, now) in [(4u8, 0u64), (1, 5), (2, 0)] {
            k.player_joined(slot, "x", token(slot));
            k.player_left(slot, LeaveReason::TimedOut, now, 0);
        }
        assert_eq!(
            k.expire(10),
            [2, 4],
            "slots 2 and 4 were held until 10; slot 1 until 15"
        );
        assert_eq!(k.held_slots(), [1]);
        assert_eq!(k.expire(10), Vec::<u8>::new());
        assert_eq!(k.expire(15), [1]);
        assert_eq!(k.held_count(), 0);
    }

    #[test]
    fn the_number_of_held_seats_is_limited() {
        let mut k = SeatKeeper::new(SeatPolicy {
            max_held: 2,
            ..SeatPolicy::default()
        });
        for slot in 0..4u8 {
            k.player_joined(slot, "p", token(slot + 1));
        }
        assert_eq!(k.player_left(0, LeaveReason::TimedOut, 0, 0), Outcome::Held);
        assert_eq!(k.player_left(1, LeaveReason::TimedOut, 0, 0), Outcome::Held);
        assert_eq!(
            k.player_left(2, LeaveReason::TimedOut, 0, 0),
            Outcome::Released,
            "no room to hold a third"
        );
        assert_eq!(k.held_slots(), [0, 1]);
        k.expire(60_000_000);
        assert_eq!(
            k.player_left(3, LeaveReason::TimedOut, 70_000_000, 0),
            Outcome::Held,
            "room again"
        );
    }

    // ---- claiming -------------------------------------------------------------------------------------------

    #[test]
    fn a_wrong_name_or_a_wrong_token_does_not_resume() {
        let mut k = keeper();
        k.player_joined(1, "Sam", token(1));
        k.player_left(1, LeaveReason::TimedOut, 0, 0);
        assert_eq!(k.claim(&token(1), "Mallory", 1), Claim::WrongName);
        assert_eq!(k.claim(&token(2), "Sam", 1), Claim::Unknown);
        assert!(k.is_held(1), "the seat is still held for its owner");
        assert_eq!(k.wrong_tries(), 2);
        assert_eq!(
            k.reservation(1).unwrap().wrong_tries,
            1,
            "the wrong name is counted against the seat"
        );
        assert!(matches!(
            k.claim(&token(1), "Sam", 2),
            Claim::Resume { slot: 1, .. }
        ));
    }

    #[test]
    fn a_player_with_nothing_held_is_just_unknown_and_does_not_count_as_guessing() {
        let mut k = keeper();
        for _ in 0..50 {
            assert_eq!(k.claim(&token(3), "New", 0), Claim::Unknown);
        }
        assert_eq!(
            k.wrong_tries(),
            0,
            "nothing was held, so nothing was guessed at"
        );
    }

    #[test]
    fn too_many_wrong_tokens_lock_claims_out_until_reset_or_the_holds_end() {
        let mut k = SeatKeeper::new(SeatPolicy {
            max_wrong_tries: 3,
            ..SeatPolicy::default()
        });
        k.player_joined(0, "Sam", token(1));
        k.player_left(0, LeaveReason::TimedOut, 0, 0);
        for i in 0..3 {
            assert_eq!(k.claim(&token(10 + i), "x", 1), Claim::Unknown);
        }
        assert_eq!(
            k.claim(&token(1), "Sam", 2),
            Claim::LockedOut,
            "even the right token is refused now"
        );
        k.reset_lockout();
        assert!(matches!(k.claim(&token(1), "Sam", 3), Claim::Resume { .. }));
        // The lockout also clears when the last hold ends.
        let mut j = SeatKeeper::new(SeatPolicy {
            max_wrong_tries: 1,
            grace_us: 10,
            ..SeatPolicy::default()
        });
        j.player_joined(0, "A", token(1));
        j.player_left(0, LeaveReason::TimedOut, 0, 0);
        j.claim(&token(9), "A", 1);
        assert_eq!(j.claim(&token(1), "A", 2), Claim::LockedOut);
        j.expire(10);
        assert_eq!(j.wrong_tries(), 0);
        j.player_joined(0, "A", token(1));
        j.player_left(0, LeaveReason::TimedOut, 20, 0);
        assert!(matches!(j.claim(&token(1), "A", 21), Claim::Resume { .. }));
    }

    #[test]
    fn a_token_that_differs_in_one_byte_never_matches_and_every_seat_is_compared() {
        let mut k = keeper();
        for slot in 0..4u8 {
            k.player_joined(slot, "p", token(slot + 1));
            k.player_left(slot, LeaveReason::TimedOut, 0, u64::from(slot));
        }
        let mut near = token(3);
        near[15] ^= 1;
        assert_eq!(k.claim(&near, "p", 1), Claim::Unknown);
        assert_eq!(
            k.claim(&token(3), "p", 1),
            Claim::Resume {
                slot: 2,
                last_acked_tick: 2
            },
            "the third seat, found among four"
        );
    }

    #[test]
    fn a_repeated_token_replaces_the_old_holder() {
        let mut k = keeper();
        k.player_joined(0, "A", token(1));
        k.player_joined(1, "B", token(1)); // the same token handed out twice: a bug of the caller, handled safely
        k.player_left(0, LeaveReason::TimedOut, 0, 0);
        k.player_left(1, LeaveReason::TimedOut, 0, 7);
        assert_eq!(k.held_count(), 1, "one token, one seat");
        assert_eq!(
            k.claim(&token(1), "B", 1),
            Claim::Resume {
                slot: 1,
                last_acked_tick: 7
            }
        );
    }

    #[test]
    fn a_new_player_with_a_token_that_is_being_held_replaces_that_reservation() {
        let mut k = keeper();
        k.player_joined(0, "A", token(1));
        k.player_left(0, LeaveReason::TimedOut, 0, 0);
        assert!(k.is_held(0));
        k.player_joined(1, "B", token(1)); // a token handed out twice: the new owner wins, the stale reservation goes
        assert!(
            !k.is_held(0),
            "A's reservation was dropped, so the seat can be given away"
        );
        assert_eq!(k.claim(&token(1), "A", 1), Claim::Unknown);
    }

    #[test]
    fn a_connected_player_whose_token_is_reused_by_another_seat_loses_the_seat() {
        let mut k = keeper();
        k.player_joined(0, "A", token(1));
        k.player_joined(1, "B", token(1)); // A is still connected, but B now owns the token
        assert_eq!(
            k.player_left(0, LeaveReason::TimedOut, 0, 0),
            Outcome::NotSeated,
            "A's seat entry was replaced"
        );
        assert_eq!(k.player_left(1, LeaveReason::TimedOut, 0, 3), Outcome::Held);
        assert_eq!(
            k.claim(&token(1), "B", 1),
            Claim::Resume {
                slot: 1,
                last_acked_tick: 3
            }
        );
    }

    #[test]
    fn the_lockout_survives_while_any_seat_is_still_held_and_while_nothing_has_expired() {
        let p = SeatPolicy {
            max_wrong_tries: 2,
            grace_us: 100,
            ..SeatPolicy::default()
        };
        let mut k = SeatKeeper::new(p);
        k.player_joined(0, "A", token(1));
        k.player_joined(1, "B", token(2));
        k.player_left(0, LeaveReason::TimedOut, 0, 0); // held until 100
        k.player_left(1, LeaveReason::TimedOut, 50, 0); // held until 150
        k.claim(&token(9), "x", 1);
        k.claim(&token(9), "x", 1);
        assert_eq!(k.wrong_tries(), 2);
        assert_eq!(k.expire(100), [0]);
        assert_eq!(k.wrong_tries(), 2, "B is still held: the lockout stays");
        // Nothing expiring does not clear it either, even when no seat is held any more (B came back).
        k.reset_lockout();
        k.claim(&token(9), "x", 101);
        assert!(matches!(k.claim(&token(2), "B", 102), Claim::Resume { .. }));
        assert_eq!(k.held_count(), 0);
        assert_eq!(k.wrong_tries(), 1);
        assert_eq!(k.expire(1_000), Vec::<u8>::new());
        assert_eq!(
            k.wrong_tries(),
            1,
            "an expire call that expired nothing clears nothing"
        );
    }

    // ---- backoff -------------------------------------------------------------------------------------------------

    #[test]
    fn the_waits_double_up_to_the_cap() {
        let mut b = Backoff::new(BackoffPolicy::default(), 0);
        let mut now = 0;
        let mut waits = Vec::new();
        for _ in 0..7 {
            assert_eq!(b.poll(now), Retry::Now);
            b.attempt_failed(now);
            let Retry::WaitUntil(at) = b.poll(now) else {
                panic!()
            };
            waits.push(at - now);
            now = at;
        }
        assert_eq!(
            waits,
            [
                500_000, 1_000_000, 2_000_000, 4_000_000, 8_000_000, 8_000_000, 8_000_000
            ]
        );
        assert_eq!(b.attempts(), 7);
    }

    #[test]
    fn it_gives_up_after_the_attempt_limit_or_the_time_limit() {
        let p = BackoffPolicy {
            max_attempts: 3,
            give_up_after_us: 1_000_000_000,
            ..BackoffPolicy::default()
        };
        let mut b = Backoff::new(p, 0);
        let mut now = 0;
        for _ in 0..3 {
            assert_eq!(b.poll(now), Retry::Now);
            b.attempt_failed(now);
            now += 100_000_000;
        }
        assert_eq!(b.poll(now), Retry::GiveUp);
        assert_eq!(b.stopped(), Some(&Stopped::TooManyAttempts));
        assert_eq!(b.poll(0), Retry::GiveUp, "and it stays over");
        // The time limit: the 90 s default.
        let mut t = Backoff::new(BackoffPolicy::default(), 1_000);
        assert!(matches!(t.poll(89_999_999 + 1_000), Retry::Now));
        assert_eq!(t.poll(90_000_000 + 1_000), Retry::GiveUp);
        assert_eq!(t.stopped(), Some(&Stopped::TooLong));
    }

    #[test]
    fn permanent_refusals_stop_it_and_transient_ones_do_not() {
        for reason in [
            RefuseReason::Banned,
            RefuseReason::BadPassword,
            RefuseReason::SchemaMismatch,
            RefuseReason::ModMismatch,
            RefuseReason::AuthFailed,
            RefuseReason::InvalidName,
            RefuseReason::VersionMismatch {
                server: crate::protocol::ProtocolVersion::new(9, 9),
            },
        ] {
            let mut b = Backoff::new(BackoffPolicy::default(), 0);
            b.refused(0, &reason);
            assert_eq!(b.poll(0), Retry::GiveUp, "{reason:?}");
            assert_eq!(b.stopped(), Some(&Stopped::Refused(reason.clone())));
            assert_eq!(
                b.attempts(),
                0,
                "a permanent refusal is not an attempt to count"
            );
        }
        for reason in [RefuseReason::ServerFull, RefuseReason::NameTaken] {
            let mut b = Backoff::new(BackoffPolicy::default(), 0);
            b.refused(0, &reason);
            assert_eq!(b.poll(0), Retry::WaitUntil(500_000), "{reason:?}");
            assert_eq!(b.attempts(), 1);
            assert!(b.stopped().is_none());
        }
    }

    #[test]
    fn a_rate_limit_is_waited_out_if_it_asks_for_longer_than_the_schedule() {
        let mut b = Backoff::new(BackoffPolicy::default(), 0);
        b.refused(
            0,
            &RefuseReason::RateLimited {
                retry_after_ms: 30_000,
            },
        );
        assert_eq!(
            b.poll(1),
            Retry::WaitUntil(30_000_000),
            "30 s beats the 0.5 s schedule"
        );
        let mut c = Backoff::new(BackoffPolicy::default(), 0);
        c.refused(
            0,
            &RefuseReason::RateLimited {
                retry_after_ms: 100,
            },
        );
        assert_eq!(
            c.poll(1),
            Retry::WaitUntil(500_000),
            "the schedule beats a short ask"
        );
    }

    #[test]
    fn an_attempt_is_due_exactly_at_the_scheduled_time() {
        let mut b = Backoff::new(BackoffPolicy::default(), 0);
        b.attempt_failed(0);
        assert_eq!(b.poll(499_999), Retry::WaitUntil(500_000));
        assert_eq!(b.poll(500_000), Retry::Now);
    }

    #[test]
    fn a_reconnect_resets_the_give_up_clock_too_not_only_the_attempt_count() {
        let mut b = Backoff::new(BackoffPolicy::default(), 0);
        b.attempt_failed(0);
        b.connected(50_000_000);
        // 100 s after the very first drop but only 50 s after the connection: still within the 90 s of the new schedule.
        assert_eq!(b.poll(100_000_000), Retry::Now);
        assert_eq!(b.poll(140_000_000), Retry::GiveUp, "90 s after the reset");
        // And a schedule that had stopped is revived.
        let mut c = Backoff::new(BackoffPolicy::default(), 0);
        c.refused(0, &RefuseReason::Banned);
        assert_eq!(c.poll(1), Retry::GiveUp);
        c.connected(2);
        assert_eq!(c.poll(2), Retry::Now);
        assert_eq!(c.stopped(), None);
    }

    #[test]
    fn a_successful_reconnect_starts_the_schedule_over() {
        let mut b = Backoff::new(BackoffPolicy::default(), 0);
        for i in 0..5 {
            b.attempt_failed(i);
        }
        b.connected(50_000_000);
        assert_eq!((b.attempts(), b.stopped()), (0, None));
        assert_eq!(b.poll(50_000_000), Retry::Now);
        b.attempt_failed(50_000_000);
        assert_eq!(
            b.poll(50_000_001),
            Retry::WaitUntil(50_500_000),
            "the first wait again, counted from the new drop"
        );
    }

    #[test]
    fn huge_times_and_many_failures_do_not_overflow() {
        let mut b = Backoff::new(
            BackoffPolicy {
                max_attempts: u32::MAX,
                give_up_after_us: u64::MAX,
                ..BackoffPolicy::default()
            },
            0,
        );
        for _ in 0..200 {
            b.attempt_failed(u64::MAX - 5);
        }
        assert!(matches!(
            b.poll(u64::MAX - 5),
            Retry::WaitUntil(_) | Retry::Now
        ));
        b.refused(
            u64::MAX,
            &RefuseReason::RateLimited {
                retry_after_ms: u32::MAX,
            },
        );
        assert!(matches!(
            b.poll(0),
            Retry::WaitUntil(u64::MAX) | Retry::Now | Retry::GiveUp
        ));
    }

    // ---- both together --------------------------------------------------------------------------------------------------

    #[test]
    fn a_drop_and_a_return_through_both_sides() {
        let mut seats = keeper();
        seats.player_joined(2, "Sam", token(5));
        // The connection dies at t = 10 s; the server notices and holds the seat.
        seats.player_left(2, LeaveReason::TimedOut, 10_000_000, 900);
        let mut client = Backoff::new(BackoffPolicy::default(), 10_000_000);
        let mut now = 10_000_000;
        let mut tries = 0;
        // The server is unreachable for the first 3 attempts, then answers.
        let resumed = loop {
            match client.poll(now) {
                Retry::Now => {
                    tries += 1;
                    if tries <= 3 {
                        client.attempt_failed(now);
                    } else {
                        break seats.claim(&token(5), "Sam", now);
                    }
                }
                Retry::WaitUntil(at) => now = at,
                Retry::GiveUp => panic!("gave up too early"),
            }
        };
        assert_eq!(
            resumed,
            Claim::Resume {
                slot: 2,
                last_acked_tick: 900
            }
        );
        assert_eq!(
            now - 10_000_000,
            500_000 + 1_000_000 + 2_000_000,
            "after waits of 0.5, 1 and 2 s"
        );
        assert_eq!(
            seats.free_slot(4),
            Some(0),
            "the returned player's seat is theirs, not free"
        );
    }
}
