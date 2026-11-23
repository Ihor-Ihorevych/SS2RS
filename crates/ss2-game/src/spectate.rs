//! The **spectator** for a dead player in co-op (roadmap T08-024, `docs/SPECTATOR.md`): whom to watch, and the follow camera that watches them.
//!
//! Pure logic, no window and no clock (time is the `dt` argument), so it is tested without a GPU or a network.
//!
//! * [`Spectator`] chooses what the dead player looks at: **one of the living teammates** (never themselves), cycling in a fixed order, moving on by itself when the
//!   watched player dies or leaves, and falling back to the free-fly camera ([`crate::freefly`]) when nobody is left alive. The player can also choose the free-fly camera.
//! * [`FollowCamera`] is the third-person camera that trails the watched player: behind and above, smoothed so it does not jitter with every step, looking at them.
//!
//! Conventions as in [`crate::freefly`]: right-handed, Y up, -Z forward; a yaw of 0 looks down -Z and a positive yaw turns left.

use ss2_core::math::{Quat, Vec3};
use ss2_render_api::{Camera, Projection};

/// A player who could be watched.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    /// The player's id (any stable number; the order of ids is the cycling order).
    pub id: u64,
    /// Whether they are alive. Only living players are watched.
    pub alive: bool,
}

/// What the spectator is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpectateMode {
    /// Not spectating (the player is alive).
    Off,
    /// Watching this player.
    Follow(u64),
    /// Flying freely. `by_choice` is true when the player picked it, false when there was nobody to watch.
    FreeFly {
        /// Whether the player asked for it.
        by_choice: bool,
    },
}

/// Chooses whom a dead player watches.
///
/// ```
/// use ss2_game::spectate::{Candidate, SpectateMode, Spectator};
///
/// let team = |alive: &[u64]| -> Vec<Candidate> { (1..=3).map(|id| Candidate { id, alive: alive.contains(&id) }).collect() };
/// let mut s = Spectator::new(1);                       // I am player 1
/// s.begin(&team(&[2, 3]));                              // I died: watch the first living teammate
/// assert_eq!(s.mode(), SpectateMode::Follow(2));
/// s.update(&team(&[3]));                                // player 2 died too: move on to 3
/// assert_eq!(s.mode(), SpectateMode::Follow(3));
/// s.update(&team(&[]));                                 // nobody is left
/// assert_eq!(s.mode(), SpectateMode::FreeFly { by_choice: false });
/// ```
#[derive(Clone, Debug)]
pub struct Spectator {
    own: u64,
    mode: SpectateMode,
    /// The last player watched (to come back to them after a respawn or a pause in free-fly).
    last: Option<u64>,
}

impl Spectator {
    /// A spectator for the local player `own_id`, not spectating.
    pub fn new(own_id: u64) -> Self {
        Self {
            own: own_id,
            mode: SpectateMode::Off,
            last: None,
        }
    }

    /// What it is doing.
    pub fn mode(&self) -> SpectateMode {
        self.mode
    }

    /// The player being watched, if any.
    pub fn target(&self) -> Option<u64> {
        match self.mode {
            SpectateMode::Follow(id) => Some(id),
            _ => None,
        }
    }

    /// The living players other than the local one, in id order (a player listed twice appears twice, which changes nothing about who comes next).
    fn living(&self, candidates: &[Candidate]) -> Vec<u64> {
        let mut ids: Vec<u64> = candidates
            .iter()
            .filter(|c| c.alive && c.id != self.own)
            .map(|c| c.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    /// The local player died: start spectating. It picks the player watched last if they are still alive, otherwise the first living teammate by id; with nobody alive it
    /// flies freely.
    pub fn begin(&mut self, candidates: &[Candidate]) {
        let living = self.living(candidates);
        self.mode = match self
            .last
            .filter(|id| living.contains(id))
            .or_else(|| living.first().copied())
        {
            Some(id) => SpectateMode::Follow(id),
            None => SpectateMode::FreeFly { by_choice: false },
        };
        if let SpectateMode::Follow(id) = self.mode {
            self.last = Some(id);
        }
    }

    /// The local player is alive again (respawned): stop.
    pub fn end(&mut self) {
        self.mode = SpectateMode::Off;
    }

    /// Call every frame (or whenever the roster changes) while spectating, with the current roster. If the watched player died or left, move to the **next living player
    /// after them in id order** (wrapping); if nobody is alive, fly freely; if the camera is flying only because nobody was alive and someone is now, follow them.
    /// A player who chose free-fly stays free. Does nothing when not spectating.
    pub fn update(&mut self, candidates: &[Candidate]) {
        let living = self.living(candidates);
        match self.mode {
            SpectateMode::Off | SpectateMode::FreeFly { by_choice: true } => {}
            SpectateMode::Follow(id) => {
                if !living.contains(&id) {
                    self.mode = match next_after(&living, id) {
                        Some(next) => SpectateMode::Follow(next),
                        None => SpectateMode::FreeFly { by_choice: false },
                    };
                }
            }
            SpectateMode::FreeFly { by_choice: false } => {
                if let Some(first) = self
                    .last
                    .filter(|id| living.contains(id))
                    .or_else(|| living.first().copied())
                {
                    self.mode = SpectateMode::Follow(first);
                }
            }
        }
        if let SpectateMode::Follow(id) = self.mode {
            self.last = Some(id);
        }
    }

    /// Watch the next living player (the "next target" key). From free-fly it starts at the player watched last. With nobody alive it flies freely.
    pub fn next(&mut self, candidates: &[Candidate]) {
        self.step(candidates, true);
    }

    /// Watch the previous living player.
    pub fn previous(&mut self, candidates: &[Candidate]) {
        self.step(candidates, false);
    }

    fn step(&mut self, candidates: &[Candidate], forward: bool) {
        if self.mode == SpectateMode::Off {
            return;
        }
        let living = self.living(candidates);
        if living.is_empty() {
            self.mode = SpectateMode::FreeFly { by_choice: false };
            return;
        }
        let from = match self.mode {
            SpectateMode::Follow(id) => Some(id),
            _ => self.last,
        };
        let pick = match (from, forward) {
            (Some(id), true) => next_after(&living, id),
            (Some(id), false) => prev_before(&living, id),
            (None, true) => living.first().copied(),
            (None, false) => living.last().copied(),
        };
        // From free-fly with a remembered player who is still alive, "next" goes to the one after them; with none remembered it starts at the end the key points to.
        self.mode = SpectateMode::Follow(pick.unwrap_or(living[0]));
        self.last = self.target();
    }

    /// Toggle between watching a player and flying freely (the player's choice). When going back to watching it returns to the last player if alive, else the first living one;
    /// with nobody alive it stays in free-fly.
    pub fn toggle_free_fly(&mut self, candidates: &[Candidate]) {
        match self.mode {
            SpectateMode::Off => {}
            SpectateMode::Follow(_) => self.mode = SpectateMode::FreeFly { by_choice: true },
            SpectateMode::FreeFly { .. } => {
                let living = self.living(candidates);
                self.mode = match self
                    .last
                    .filter(|id| living.contains(id))
                    .or_else(|| living.first().copied())
                {
                    Some(id) => SpectateMode::Follow(id),
                    None => SpectateMode::FreeFly { by_choice: false },
                };
                self.last = self.target().or(self.last);
            }
        }
    }
}

/// The first id after `id` in a sorted list, wrapping to the start; `None` for an empty list. (`id` need not be in the list.)
fn next_after(sorted: &[u64], id: u64) -> Option<u64> {
    sorted
        .iter()
        .copied()
        .find(|x| *x > id)
        .or_else(|| sorted.first().copied())
}

/// The last id before `id` in a sorted list, wrapping to the end; `None` for an empty list.
fn prev_before(sorted: &[u64], id: u64) -> Option<u64> {
    sorted
        .iter()
        .copied()
        .rev()
        .find(|x| *x < id)
        .or_else(|| sorted.last().copied())
}

/// How the follow camera sits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FollowConfig {
    /// How far behind the watched player, in world units.
    pub distance: f32,
    /// How far above their position the camera sits.
    pub height: f32,
    /// How far above their position the camera looks (about head height).
    pub look_height: f32,
    /// Seconds in which the camera closes about two thirds of the gap to where it wants to be; 0 snaps.
    pub smoothing: f32,
}

impl FollowConfig {
    /// The longest step simulated at once, so a stall does not fling the camera.
    pub const MAX_STEP: f32 = 0.25;
}

impl Default for FollowConfig {
    fn default() -> Self {
        Self {
            distance: 3.0,
            height: 1.8,
            look_height: 1.5,
            smoothing: 0.12,
        }
    }
}

/// A third-person camera that trails a player.
///
/// ```
/// use ss2_core::math::Vec3;
/// use ss2_game::spectate::{FollowCamera, FollowConfig};
///
/// let mut cam = FollowCamera::new(FollowConfig { smoothing: 0.0, ..FollowConfig::default() });
/// // A player at the origin facing -Z (yaw 0): the camera ends up 3 units behind, on the +Z side, and 1.8 up.
/// cam.update(0.016, Vec3::ZERO, 0.0);
/// let p = cam.camera().position;
/// assert!((p.z - 3.0).abs() < 1e-5 && (p.y - 1.8).abs() < 1e-5);
/// ```
#[derive(Clone, Debug)]
pub struct FollowCamera {
    config: FollowConfig,
    position: Vec3,
    target: Vec3,
    primed: bool,
    projection: Projection,
}

impl FollowCamera {
    /// A camera with this configuration. Its first update places it at once (no flight from the origin).
    pub fn new(config: FollowConfig) -> Self {
        Self {
            config,
            position: Vec3::ZERO,
            target: Vec3::ZERO,
            primed: false,
            projection: Camera::default().projection,
        }
    }

    /// Use another projection (the field of view of the settings).
    pub fn set_projection(&mut self, projection: Projection) {
        self.projection = projection;
    }

    /// Forget the current position: the next update places the camera at once (the watched player changed, or teleported).
    pub fn snap(&mut self) {
        self.primed = false;
    }

    /// Advance by `dt` seconds with the watched player at `target_position` facing `target_yaw` (radians). A `dt` that is negative or not a number, or a target that is not
    /// finite, changes nothing; a `dt` over [`FollowConfig::MAX_STEP`] counts as that long.
    pub fn update(&mut self, dt: f32, target_position: Vec3, target_yaw: f32) {
        if !(dt.is_finite() && dt >= 0.0) || !target_position.is_finite() || !target_yaw.is_finite()
        {
            return;
        }
        // The direction the player faces: yaw 0 is -Z, positive turns left.
        let facing = Vec3::new(-target_yaw.sin(), 0.0, -target_yaw.cos());
        let desired =
            target_position + Vec3::Y * self.config.height - facing * self.config.distance;
        self.target = target_position + Vec3::Y * self.config.look_height;
        if !self.primed || self.config.smoothing <= 0.0 {
            self.position = desired;
            self.primed = true;
        } else {
            let k = 1.0 - (-dt.min(FollowConfig::MAX_STEP) / self.config.smoothing).exp();
            self.position += (desired - self.position) * k;
        }
    }

    /// The camera to render from: at the smoothed position, looking at the watched player's head.
    pub fn camera(&self) -> Camera {
        let d = self.target - self.position;
        let length = d.length();
        let (yaw, pitch) = if length < 1e-6 || !d.is_finite() {
            (0.0, 0.0)
        } else {
            let yaw = if d.x * d.x + d.z * d.z > 1e-12 {
                (-d.x).atan2(-d.z)
            } else {
                0.0
            };
            (yaw, (d.y / length).clamp(-1.0, 1.0).asin())
        };
        Camera {
            position: self.position,
            rotation: Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch),
            projection: self.projection,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn team(alive: &[u64]) -> Vec<Candidate> {
        (1..=5)
            .map(|id| Candidate {
                id,
                alive: alive.contains(&id),
            })
            .collect()
    }

    fn started(alive: &[u64]) -> Spectator {
        let mut s = Spectator::new(1);
        s.begin(&team(alive));
        s
    }

    // ---- choosing a target ------------------------------------------------------------------------

    #[test]
    fn spectating_starts_on_the_first_living_teammate_and_never_on_yourself() {
        let s = started(&[1, 3, 4]);
        assert_eq!(
            s.mode(),
            SpectateMode::Follow(3),
            "player 1 is me, even if marked alive"
        );
        assert_eq!(s.target(), Some(3));
        let none = started(&[1]);
        assert_eq!(none.mode(), SpectateMode::FreeFly { by_choice: false });
        assert_eq!(none.target(), None);
        assert_eq!(Spectator::new(1).mode(), SpectateMode::Off);
    }

    #[test]
    fn when_the_watched_player_dies_it_moves_to_the_next_living_one_after_them() {
        let mut s = started(&[2, 3, 4, 5]);
        assert_eq!(s.mode(), SpectateMode::Follow(2));
        s.update(&team(&[3, 4, 5]));
        assert_eq!(
            s.mode(),
            SpectateMode::Follow(3),
            "2 died: the next after 2"
        );
        s.update(&team(&[3, 4, 5])); // nothing changed: it stays
        assert_eq!(s.mode(), SpectateMode::Follow(3));
        s.update(&team(&[4, 5]));
        assert_eq!(s.mode(), SpectateMode::Follow(4));
        // 5 is watched and dies: wrap around to the first living one.
        s.next(&team(&[4, 5]));
        assert_eq!(s.mode(), SpectateMode::Follow(5));
        s.update(&team(&[4]));
        assert_eq!(s.mode(), SpectateMode::Follow(4), "wrapped");
    }

    #[test]
    fn a_watched_player_who_leaves_the_roster_counts_as_dead() {
        let mut s = started(&[2, 3]);
        let roster: Vec<Candidate> = vec![
            Candidate { id: 3, alive: true },
            Candidate {
                id: 1,
                alive: false,
            },
        ];
        s.update(&roster);
        assert_eq!(s.mode(), SpectateMode::Follow(3));
    }

    #[test]
    fn with_nobody_left_it_flies_freely_and_follows_again_when_someone_respawns() {
        let mut s = started(&[2]);
        s.update(&team(&[]));
        assert_eq!(s.mode(), SpectateMode::FreeFly { by_choice: false });
        s.update(&team(&[]));
        assert_eq!(s.mode(), SpectateMode::FreeFly { by_choice: false });
        s.update(&team(&[2, 4]));
        assert_eq!(
            s.mode(),
            SpectateMode::Follow(2),
            "the one watched last, back alive"
        );
        s.update(&team(&[]));
        s.update(&team(&[4, 5]));
        assert_eq!(
            s.mode(),
            SpectateMode::Follow(4),
            "the last one is not alive: the first living"
        );
    }

    #[test]
    fn next_and_previous_cycle_through_the_living_in_id_order_and_wrap() {
        let mut s = started(&[2, 3, 5]);
        let all = team(&[2, 3, 5]);
        let mut seen = vec![s.target().unwrap()];
        for _ in 0..3 {
            s.next(&all);
            seen.push(s.target().unwrap());
        }
        assert_eq!(seen, [2, 3, 5, 2]);
        s.previous(&all);
        assert_eq!(s.target(), Some(5), "previous from 2 wraps to 5");
        s.previous(&all);
        assert_eq!(s.target(), Some(3));
        // Dead players are skipped, and so is the local player.
        let some = team(&[1, 3, 4]);
        s.next(&some);
        assert_eq!(s.target(), Some(4));
        s.next(&some);
        assert_eq!(s.target(), Some(3), "skips 1 (me): 4 wraps to 3");
    }

    #[test]
    fn stepping_with_nobody_alive_flies_freely_and_stepping_when_off_does_nothing() {
        let mut s = started(&[2]);
        s.next(&team(&[]));
        assert_eq!(s.mode(), SpectateMode::FreeFly { by_choice: false });
        let mut off = Spectator::new(1);
        off.next(&team(&[2]));
        off.previous(&team(&[2]));
        off.toggle_free_fly(&team(&[2]));
        off.update(&team(&[2]));
        assert_eq!(off.mode(), SpectateMode::Off);
    }

    #[test]
    fn free_fly_by_choice_stays_until_toggled_back_and_returns_to_the_last_player() {
        let mut s = started(&[2, 3, 4]);
        s.next(&team(&[2, 3, 4]));
        assert_eq!(s.target(), Some(3));
        s.toggle_free_fly(&team(&[2, 3, 4]));
        assert_eq!(s.mode(), SpectateMode::FreeFly { by_choice: true });
        s.update(&team(&[2, 3, 4]));
        s.update(&team(&[]));
        assert_eq!(
            s.mode(),
            SpectateMode::FreeFly { by_choice: true },
            "a choice is not overridden by who is alive"
        );
        s.toggle_free_fly(&team(&[2, 4]));
        assert_eq!(
            s.mode(),
            SpectateMode::Follow(2),
            "3 is gone: the first living"
        );
        s.toggle_free_fly(&team(&[2, 4]));
        s.toggle_free_fly(&team(&[2, 4]));
        assert_eq!(
            s.mode(),
            SpectateMode::Follow(2),
            "toggling back returns to the last watched"
        );
        s.toggle_free_fly(&team(&[]));
        s.toggle_free_fly(&team(&[]));
        assert_eq!(
            s.mode(),
            SpectateMode::FreeFly { by_choice: false },
            "nobody to go back to"
        );
    }

    #[test]
    fn going_back_to_watching_remembers_the_player_it_chose_not_the_one_before() {
        let mut s = started(&[2, 3, 4]);
        s.next(&team(&[2, 3, 4])); // watching 3
        s.toggle_free_fly(&team(&[2, 3, 4]));
        s.toggle_free_fly(&team(&[2, 4])); // 3 is gone: back to watching 2
        assert_eq!(s.target(), Some(2));
        s.toggle_free_fly(&team(&[2, 3, 4]));
        s.toggle_free_fly(&team(&[2, 3, 4]));
        assert_eq!(
            s.target(),
            Some(2),
            "the last watched is 2 now, not the 3 from before"
        );
    }

    #[test]
    fn next_from_free_fly_continues_after_the_last_watched_player() {
        let mut s = started(&[2, 3, 4]);
        s.toggle_free_fly(&team(&[2, 3, 4]));
        s.next(&team(&[2, 3, 4]));
        assert_eq!(s.target(), Some(3), "after 2, the last watched");
        let mut fresh = Spectator::new(1);
        fresh.begin(&team(&[]));
        fresh.next(&team(&[2, 3]));
        assert_eq!(
            fresh.target(),
            Some(2),
            "nothing watched yet: start at the first"
        );
        let mut back = Spectator::new(1);
        back.begin(&team(&[]));
        back.previous(&team(&[2, 3]));
        assert_eq!(
            back.target(),
            Some(3),
            "previous with no history starts at the last"
        );
    }

    #[test]
    fn begin_prefers_the_player_watched_last_time_and_end_stops() {
        let mut s = started(&[2, 3, 4]);
        s.next(&team(&[2, 3, 4]));
        s.end();
        assert_eq!(s.mode(), SpectateMode::Off);
        s.begin(&team(&[2, 3, 4]));
        assert_eq!(s.target(), Some(3), "the same player as before");
        s.end();
        s.begin(&team(&[2, 4]));
        assert_eq!(s.target(), Some(2), "3 is not alive: the first living");
    }

    #[test]
    fn duplicate_and_unsorted_rosters_do_not_confuse_the_order() {
        let mut s = Spectator::new(9);
        let roster = vec![
            Candidate { id: 7, alive: true },
            Candidate { id: 3, alive: true },
            Candidate { id: 7, alive: true },
            Candidate {
                id: 5,
                alive: false,
            },
            Candidate { id: 3, alive: true },
        ];
        s.begin(&roster);
        assert_eq!(s.target(), Some(3));
        s.next(&roster);
        assert_eq!(s.target(), Some(7));
        s.next(&roster);
        assert_eq!(s.target(), Some(3));
    }

    #[test]
    fn neighbour_helpers_wrap_and_accept_ids_not_in_the_list() {
        assert_eq!(next_after(&[2, 4, 6], 4), Some(6));
        assert_eq!(next_after(&[2, 4, 6], 6), Some(2));
        assert_eq!(next_after(&[2, 4, 6], 5), Some(6), "5 is not in the list");
        assert_eq!(next_after(&[2, 4, 6], 99), Some(2));
        assert_eq!(next_after(&[], 1), None);
        assert_eq!(prev_before(&[2, 4, 6], 4), Some(2));
        assert_eq!(prev_before(&[2, 4, 6], 2), Some(6));
        assert_eq!(prev_before(&[2, 4, 6], 5), Some(4));
        assert_eq!(prev_before(&[2, 4, 6], 0), Some(6));
        assert_eq!(prev_before(&[], 1), None);
    }

    // ---- the follow camera ----------------------------------------------------------------------------------

    fn snap_camera(position: Vec3, yaw: f32) -> Camera {
        let mut cam = FollowCamera::new(FollowConfig {
            smoothing: 0.0,
            ..FollowConfig::default()
        });
        cam.update(0.016, position, yaw);
        cam.camera()
    }

    #[test]
    fn the_camera_sits_behind_and_above_the_player_whichever_way_they_face() {
        for (yaw, behind) in [
            (0.0f32, Vec3::Z),
            (std::f32::consts::FRAC_PI_2, Vec3::X),
            (std::f32::consts::PI, Vec3::NEG_Z),
            (-std::f32::consts::FRAC_PI_2, Vec3::NEG_X),
        ] {
            // yaw 0 faces -Z (camera on +Z); a quarter turn left faces -X (camera on +X); and so on.
            let cam = snap_camera(Vec3::new(10.0, 2.0, -5.0), yaw);
            let offset = cam.position - Vec3::new(10.0, 2.0, -5.0);
            assert!(
                (offset - (behind * 3.0 + Vec3::Y * 1.8)).length() < 1e-4,
                "yaw {yaw}: {offset:?}"
            );
        }
    }

    #[test]
    fn the_camera_looks_at_the_players_head() {
        let player = Vec3::new(4.0, 0.0, 9.0);
        let cam = snap_camera(player, 0.7);
        let head = player + Vec3::Y * 1.5;
        let forward = cam.rotation * Vec3::NEG_Z;
        let to_head = (head - cam.position).normalize();
        assert!(
            forward.dot(to_head) > 0.99999,
            "{forward:?} against {to_head:?}"
        );
        assert!(cam.is_valid());
    }

    #[test]
    fn smoothing_closes_the_gap_over_time_without_overshooting_and_a_step_is_capped() {
        let mut cam = FollowCamera::new(FollowConfig {
            smoothing: 0.2,
            ..FollowConfig::default()
        });
        cam.update(0.016, Vec3::ZERO, 0.0); // primed in place
        let start = cam.camera().position;
        // The player teleports 10 units away: the camera follows, never passing the goal.
        let goal = Vec3::new(10.0, 1.8, 3.0);
        let mut last = (goal - start).length();
        for _ in 0..200 {
            cam.update(0.016, Vec3::new(10.0, 0.0, 0.0), 0.0);
            let gap = (goal - cam.camera().position).length();
            assert!(gap <= last + 1e-5, "it never moves away from the goal");
            last = gap;
        }
        assert!(last < 0.01, "converged: {last}");
        // One huge step is limited: with smoothing 0.2 and a 0.25 s cap it closes at most 1 - e^(-1.25) = 71 % of the gap.
        let mut cam2 = FollowCamera::new(FollowConfig {
            smoothing: 0.2,
            ..FollowConfig::default()
        });
        cam2.update(0.0, Vec3::ZERO, 0.0);
        cam2.update(100.0, Vec3::new(10.0, 0.0, 0.0), 0.0);
        let moved = cam2.camera().position.x;
        assert!(moved > 6.9 && moved < 7.2, "{moved}");
    }

    #[test]
    fn with_no_smoothing_a_zero_length_step_still_places_the_camera_and_never_gives_nan() {
        let mut cam = FollowCamera::new(FollowConfig {
            smoothing: 0.0,
            ..FollowConfig::default()
        });
        cam.update(0.016, Vec3::ZERO, 0.0);
        cam.update(0.0, Vec3::new(5.0, 0.0, 0.0), 0.0); // 0 / 0 in the smoothing formula would be NaN
        let p = cam.camera().position;
        assert!(
            p.is_finite() && (p - Vec3::new(5.0, 1.8, 3.0)).length() < 1e-4,
            "{p:?}"
        );
        assert!(cam.camera().is_valid());
    }

    #[test]
    fn the_first_update_and_a_snap_place_the_camera_at_once() {
        let mut cam = FollowCamera::new(FollowConfig::default()); // smoothing on
        cam.update(0.016, Vec3::new(50.0, 0.0, 50.0), 0.0);
        assert!(
            (cam.camera().position - Vec3::new(50.0, 1.8, 53.0)).length() < 1e-4,
            "no flight from the origin"
        );
        cam.update(0.016, Vec3::new(0.0, 0.0, 0.0), 0.0);
        assert!(cam.camera().position.x > 40.0, "smoothed after that");
        cam.snap();
        cam.update(0.016, Vec3::new(0.0, 0.0, 0.0), 0.0);
        assert!(
            (cam.camera().position - Vec3::new(0.0, 1.8, 3.0)).length() < 1e-4,
            "snapped to the new player"
        );
    }

    #[test]
    fn bad_input_changes_nothing_and_a_camera_on_its_target_is_still_valid() {
        let mut cam = FollowCamera::new(FollowConfig::default());
        cam.update(0.016, Vec3::new(1.0, 2.0, 3.0), 0.5);
        let before = cam.camera();
        for (dt, pos, yaw) in [
            (f32::NAN, Vec3::ZERO, 0.0),
            (-1.0, Vec3::ZERO, 0.0),
            (f32::INFINITY, Vec3::ZERO, 0.0),
            (0.016, Vec3::new(f32::NAN, 0.0, 0.0), 0.0),
            (0.016, Vec3::new(f32::INFINITY, 0.0, 0.0), 0.0),
            (0.016, Vec3::ZERO, f32::NAN),
        ] {
            cam.update(dt, pos, yaw);
            assert_eq!(cam.camera(), before, "{dt} {pos:?} {yaw}");
        }
        // Zero distance and height: the camera is at the player's feet looking at their head: still a valid camera.
        let mut odd = FollowCamera::new(FollowConfig {
            distance: 0.0,
            height: 0.0,
            look_height: 0.0,
            smoothing: 0.0,
        });
        odd.update(0.016, Vec3::new(1.0, 1.0, 1.0), 0.0);
        assert!(odd.camera().is_valid(), "looking at its own position");
        let mut straight_down = FollowCamera::new(FollowConfig {
            distance: 0.0,
            height: 5.0,
            look_height: 0.0,
            smoothing: 0.0,
        });
        straight_down.update(0.016, Vec3::ZERO, 0.0);
        let c = straight_down.camera();
        assert!(
            c.is_valid() && (c.rotation * Vec3::NEG_Z).y < -0.999,
            "straight down"
        );
    }

    #[test]
    fn a_projection_can_be_set() {
        let mut cam = FollowCamera::new(FollowConfig::default());
        let p = Projection::Perspective {
            fov_y: 1.2,
            near: 0.3,
        };
        cam.set_projection(p);
        cam.update(0.016, Vec3::ZERO, 0.0);
        assert_eq!(cam.camera().projection, p);
    }
}
