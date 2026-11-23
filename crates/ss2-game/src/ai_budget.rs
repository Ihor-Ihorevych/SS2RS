//! **AI performance** (roadmap T09-017, `docs/AI_BUDGET.md`): making a horde of hundreds cheap enough to simulate, without any enemy knowing about it.
//!
//! Three small pieces, all pure logic (time is the tick number, no randomness, no `HashMap`):
//!
//! * [`RatePolicy`]: how often an agent should "think" given how far it is from the nearest player and whether anyone can see it. Near and visible agents think every tick; far and
//!   hidden ones every 16 or 30 ticks.
//! * [`AiScheduler`]: turns those wishes into a list of who thinks **this tick**. Agents are **staggered** (a hash of the agent's key spreads their phases, so a hundred agents with the
//!   same rate do not all think on the same tick), the total per tick is **capped**, and when the cap bites the **most overdue go first**, so nobody starves. Each think carries the
//!   number of ticks that passed since the agent last thought, so it can advance its timers by that much.
//! * [`PathBudget`]: a queue of path requests with a per-tick limit on how many are answered and how much they cost. Requests **age** (a long-waiting low-priority request overtakes a
//!   fresh high-priority one), a new request from an agent **replaces** its old one but **keeps its place in line**, and the first request of a tick is always answered, so an expensive one
//!   cannot block the queue.

use std::collections::{BTreeMap, BTreeSet};

/// A well-mixed 64-bit hash of an agent key (splitmix64's finalizer). Sequential keys give unrelated phases.
pub fn mix(key: u64) -> u64 {
    let mut z = key.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Agents within `within` units think every `interval` ticks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    /// The distance this band reaches, in world units.
    pub within: f32,
    /// Ticks between thoughts.
    pub interval: u32,
}

/// Why a policy was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolicyError(pub &'static str);

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for PolicyError {}

/// How often agents think.
#[derive(Clone, Debug, PartialEq)]
pub struct RatePolicy {
    /// Bands from nearest to farthest: distances strictly increasing, intervals not decreasing.
    pub bands: Vec<Band>,
    /// Ticks between thoughts beyond the last band.
    pub far_interval: u32,
    /// Agents nobody can see think this many times less often.
    pub hidden_factor: u32,
    /// No agent waits longer than this many ticks between thoughts (the final cap).
    pub max_interval: u32,
}

impl Default for RatePolicy {
    /// Every tick within 15 units, every 2nd within 40, every 4th within 80, every 8th within 150, every 16th beyond; hidden agents half as often; never more than 30 ticks apart.
    fn default() -> Self {
        Self {
            bands: vec![
                Band {
                    within: 15.0,
                    interval: 1,
                },
                Band {
                    within: 40.0,
                    interval: 2,
                },
                Band {
                    within: 80.0,
                    interval: 4,
                },
                Band {
                    within: 150.0,
                    interval: 8,
                },
            ],
            far_interval: 16,
            hidden_factor: 2,
            max_interval: 30,
        }
    }
}

impl RatePolicy {
    /// Check the values make sense.
    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.far_interval == 0 || self.hidden_factor == 0 || self.max_interval == 0 {
            return Err(PolicyError(
                "intervals and the hidden factor must be at least 1",
            ));
        }
        let mut last: Option<Band> = None;
        for b in &self.bands {
            if !(b.within.is_finite() && b.within > 0.0) || b.interval == 0 {
                return Err(PolicyError(
                    "a band needs a positive finite distance and an interval of at least 1",
                ));
            }
            if last.is_some_and(|p| b.within <= p.within || b.interval < p.interval) {
                return Err(PolicyError(
                    "bands must go outward: distances increasing, intervals not decreasing",
                ));
            }
            last = Some(*b);
        }
        if last.is_some_and(|b| b.interval > self.far_interval) {
            return Err(PolicyError(
                "the far interval must not be shorter than the last band's",
            ));
        }
        Ok(())
    }

    /// Ticks between thoughts for an agent `distance` from the nearest player. A distance that is NaN counts as far; a negative one as zero. A hidden agent thinks less often. The result is
    /// at least 1 and at most `max_interval`.
    pub fn interval(&self, distance: f32, visible: bool) -> u32 {
        // A NaN distance fails every comparison, so no band matches and the far interval is used.
        let base = self
            .bands
            .iter()
            .find(|b| distance <= b.within)
            .map_or(self.far_interval, |b| b.interval);
        let scaled = if visible {
            base
        } else {
            base.saturating_mul(self.hidden_factor)
        };
        scaled.clamp(1, self.max_interval.max(1))
    }
}

/// One agent's turn to think.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Think {
    /// The agent.
    pub key: u64,
    /// Ticks since it last thought (for a first thought: its interval), so timers advance by this much.
    pub dt_ticks: u32,
}

/// Decides who thinks on each tick.
///
/// ```
/// use ss2_game::ai_budget::AiScheduler;
///
/// let mut ai = AiScheduler::new(100);                          // at most 100 thoughts per tick
/// let agents: Vec<(u64, u32)> = (0..400).map(|k| (k, 8)).collect(); // 400 agents, each wanting to think every 8 ticks
/// let thoughts: usize = (0..8).map(|tick| ai.plan(tick, &agents).len()).sum();
/// assert_eq!(thoughts, 400);                                   // each thought exactly once in 8 ticks
/// ```
#[derive(Clone, Debug)]
pub struct AiScheduler {
    max_per_tick: usize,
    /// The tick of each agent's last thought (a virtual one, possibly before tick 0, for an agent not thought for yet), so signed.
    last: BTreeMap<u64, i64>,
}

impl AiScheduler {
    /// A scheduler that lets at most `max_per_tick` agents think on one tick (at least 1).
    pub fn new(max_per_tick: usize) -> Self {
        Self {
            max_per_tick: max_per_tick.max(1),
            last: BTreeMap::new(),
        }
    }

    /// How many agents are remembered.
    pub fn tracked(&self) -> usize {
        self.last.len()
    }

    /// Who thinks on `tick`, given every living agent as `(key, wanted interval in ticks)` (an interval of 0 counts as 1). Agents that are no longer listed are forgotten. The result is
    /// ordered most overdue first, then by key, and has at most `max_per_tick` entries; agents that are due but cut by the cap are first in line next tick.
    ///
    /// A new agent is given a **phase** from the hash of its key, so its first thought comes between this tick and `interval - 1` ticks later, and agents with the same interval do not all
    /// think together. An agent whose wanted interval changes is due as soon as the new interval has passed since its last thought.
    pub fn plan(&mut self, tick: u64, agents: &[(u64, u32)]) -> Vec<Think> {
        let present: BTreeSet<u64> = agents.iter().map(|(k, _)| *k).collect();
        self.last.retain(|k, _| present.contains(k));
        let now = i64::try_from(tick).unwrap_or(i64::MAX);
        // (overdue by, key, interval, is_new, the last thought: virtual for a new agent)
        let mut due: Vec<(i64, u64, u32, bool, i64)> = Vec::new();
        for &(key, interval) in agents {
            let interval = interval.max(1);
            let (last, is_new) = match self.last.get(&key) {
                Some(l) => (*l, false),
                None => {
                    // A virtual last thought so that the first one falls `phase` ticks from now.
                    let phase = i64::try_from(mix(key) % u64::from(interval)).unwrap_or(0);
                    (now + phase - i64::from(interval), true)
                }
            };
            let due_at = last + i64::from(interval);
            if due_at <= now {
                due.push((now - due_at, key, interval, is_new, last));
            } else if is_new {
                self.last.insert(key, last);
            }
        }
        due.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut thoughts = Vec::new();
        for (i, (_, key, interval, is_new, last)) in due.iter().enumerate() {
            if i >= self.max_per_tick {
                // Cut by the cap: a new agent keeps its virtual last thought, so it stays overdue instead of being given a fresh phase.
                if *is_new {
                    self.last.insert(*key, *last);
                }
                continue;
            }
            let dt = if *is_new {
                i64::from(*interval)
            } else {
                now - *last
            };
            self.last.insert(*key, now);
            thoughts.push(Think {
                key: *key,
                dt_ticks: u32::try_from(dt).unwrap_or(u32::MAX),
            });
        }
        thoughts
    }
}

/// A request for a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathRequest {
    /// The agent asking (one request per agent at a time).
    pub key: u64,
    /// Higher is more urgent.
    pub priority: u8,
    /// An estimate of the work it will take (nodes to search, say).
    pub cost: u32,
}

/// The limits on path finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathConfig {
    /// At most this many requests are answered per tick.
    pub max_per_tick: usize,
    /// The total cost answered per tick is kept to this (the first request of a tick is answered whatever it costs).
    pub max_cost_per_tick: u32,
    /// Every this many ticks of waiting count as one more level of priority (0 turns ageing off).
    pub aging_ticks: u32,
    /// The most requests waiting.
    pub max_queue: usize,
}

impl Default for PathConfig {
    /// 8 requests and 4,000 cost per tick, a priority level for every 15 ticks waited, 1,024 waiting at most.
    fn default() -> Self {
        Self {
            max_per_tick: 8,
            max_cost_per_tick: 4_000,
            aging_ticks: 15,
            max_queue: 1_024,
        }
    }
}

/// Why a request was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueFull;

impl std::fmt::Display for QueueFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the path request queue is full")
    }
}

impl std::error::Error for QueueFull {}

/// A queue of path requests with a per-tick budget.
///
/// ```
/// use ss2_game::ai_budget::{PathBudget, PathConfig, PathRequest};
///
/// let mut paths = PathBudget::new(PathConfig { max_per_tick: 2, ..PathConfig::default() });
/// for key in 0..5 {
///     paths.submit(PathRequest { key, priority: 1, cost: 100 }, 0).unwrap();
/// }
/// assert_eq!(paths.take(0).len(), 2);   // two answered now, three still waiting
/// assert_eq!(paths.pending(), 3);
/// ```
#[derive(Clone, Debug)]
pub struct PathBudget {
    cfg: PathConfig,
    /// Waiting requests by agent, with the tick each first waited from.
    queue: BTreeMap<u64, (PathRequest, u64)>,
}

impl PathBudget {
    /// An empty queue.
    pub fn new(cfg: PathConfig) -> Self {
        Self {
            cfg,
            queue: BTreeMap::new(),
        }
    }

    /// How many requests are waiting.
    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    /// Whether an agent has a request waiting.
    pub fn is_pending(&self, key: u64) -> bool {
        self.queue.contains_key(&key)
    }

    /// Ask for a path at `tick`. An agent that already has a request waiting has it **replaced** (the new destination, priority and cost) but **keeps its place in line** (the tick it first
    /// waited from), so asking again and again cannot keep an agent at the back.
    ///
    /// # Errors
    /// [`QueueFull`] if the queue is full and the agent had no request waiting.
    pub fn submit(&mut self, req: PathRequest, tick: u64) -> Result<(), QueueFull> {
        if let Some(entry) = self.queue.get_mut(&req.key) {
            entry.0 = req;
            return Ok(());
        }
        if self.queue.len() >= self.cfg.max_queue {
            return Err(QueueFull);
        }
        self.queue.insert(req.key, (req, tick));
        Ok(())
    }

    /// Withdraw an agent's request (it died, or no longer needs the path). `true` if one was waiting.
    pub fn cancel(&mut self, key: u64) -> bool {
        self.queue.remove(&key).is_some()
    }

    fn effective_priority(&self, req: &PathRequest, since: u64, tick: u64) -> u64 {
        let waited = tick.saturating_sub(since);
        let aged = if self.cfg.aging_ticks == 0 {
            0
        } else {
            waited / u64::from(self.cfg.aging_ticks)
        };
        u64::from(req.priority) + aged
    }

    /// The requests to answer on `tick`, most urgent first: effective priority (priority plus ageing) highest, then the longest waiting, then the lowest key. At most `max_per_tick` of them and
    /// within `max_cost_per_tick` in total; the first is taken whatever it costs. A request whose cost does not fit is **skipped** (a cheaper one behind it may still be taken), and stays waiting.
    pub fn take(&mut self, tick: u64) -> Vec<PathRequest> {
        let mut order: Vec<(u64, u64, u64)> = self
            .queue
            .iter()
            .map(|(k, (r, since))| (self.effective_priority(r, *since, tick), *since, *k))
            .collect();
        // The queue iterates in key order and the sort is stable, so equal priority and wait go to the lowest key.
        order.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut taken = Vec::new();
        let mut spent: u64 = 0;
        for (_, _, key) in order {
            if taken.len() >= self.cfg.max_per_tick {
                break;
            }
            let req = self.queue[&key].0;
            if !taken.is_empty()
                && spent + u64::from(req.cost) > u64::from(self.cfg.max_cost_per_tick)
            {
                continue;
            }
            spent += u64::from(req.cost);
            self.queue.remove(&key);
            taken.push(req);
        }
        taken
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- rates ------------------------------------------------------------------------------------------

    #[test]
    fn the_default_policy_is_valid_and_thins_out_with_distance() {
        let p = RatePolicy::default();
        p.validate().unwrap();
        let at = |d: f32| p.interval(d, true);
        assert_eq!(
            [
                at(0.0),
                at(15.0),
                at(15.01),
                at(40.0),
                at(79.9),
                at(150.0),
                at(150.1),
                at(10_000.0)
            ],
            [1, 1, 2, 2, 4, 8, 16, 16]
        );
        assert!(
            (0..300)
                .map(|d| at(d as f32))
                .collect::<Vec<_>>()
                .windows(2)
                .all(|w| w[0] <= w[1]),
            "never more often with distance"
        );
    }

    #[test]
    fn hidden_agents_think_less_often_and_nobody_waits_longer_than_the_cap() {
        let p = RatePolicy::default();
        assert_eq!(
            (
                p.interval(5.0, false),
                p.interval(100.0, false),
                p.interval(1_000.0, false)
            ),
            (2, 16, 30),
            "16 x 2 = 32 is capped at 30"
        );
        let tight = RatePolicy {
            max_interval: 4,
            ..RatePolicy::default()
        };
        assert_eq!(tight.interval(1_000.0, true), 4);
        assert_eq!(
            RatePolicy {
                max_interval: 0,
                ..RatePolicy::default()
            }
            .interval(1.0, true),
            1,
            "a zero cap still gives at least 1"
        );
    }

    #[test]
    fn an_unvalidated_policy_with_zeros_still_gives_a_usable_interval() {
        let sloppy = RatePolicy {
            bands: vec![Band {
                within: 10.0,
                interval: 0,
            }],
            far_interval: 0,
            hidden_factor: 0,
            max_interval: 0,
        };
        for d in [0.0, 5.0, 50.0, f32::NAN] {
            for visible in [true, false] {
                assert_eq!(
                    sloppy.interval(d, visible),
                    1,
                    "distance {d}, visible {visible}"
                );
            }
        }
    }

    #[test]
    fn odd_distances_are_handled() {
        let p = RatePolicy::default();
        assert_eq!(p.interval(f32::NAN, true), 16, "unknown is far");
        assert_eq!(p.interval(f32::INFINITY, true), 16);
        assert_eq!(
            p.interval(-5.0, true),
            1,
            "negative counts as at the player"
        );
        assert_eq!(p.interval(f32::NEG_INFINITY, true), 1);
    }

    #[test]
    fn bad_policies_are_refused() {
        let ok = RatePolicy::default();
        for bad in [
            RatePolicy {
                far_interval: 0,
                ..ok.clone()
            },
            RatePolicy {
                hidden_factor: 0,
                ..ok.clone()
            },
            RatePolicy {
                max_interval: 0,
                ..ok.clone()
            },
            RatePolicy {
                bands: vec![Band {
                    within: 0.0,
                    interval: 1,
                }],
                ..ok.clone()
            },
            RatePolicy {
                bands: vec![Band {
                    within: f32::NAN,
                    interval: 1,
                }],
                ..ok.clone()
            },
            RatePolicy {
                bands: vec![Band {
                    within: 10.0,
                    interval: 0,
                }],
                ..ok.clone()
            },
            RatePolicy {
                bands: vec![
                    Band {
                        within: 10.0,
                        interval: 2,
                    },
                    Band {
                        within: 10.0,
                        interval: 3,
                    },
                ],
                ..ok.clone()
            },
            RatePolicy {
                bands: vec![
                    Band {
                        within: 10.0,
                        interval: 4,
                    },
                    Band {
                        within: 20.0,
                        interval: 2,
                    },
                ],
                ..ok.clone()
            },
            RatePolicy {
                bands: vec![Band {
                    within: 10.0,
                    interval: 32,
                }],
                ..ok.clone()
            },
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
        }
        assert!(
            RatePolicy {
                bands: vec![],
                ..ok.clone()
            }
            .validate()
            .is_ok(),
            "no bands: everyone is far"
        );
        assert!(
            RatePolicy {
                bands: vec![Band {
                    within: 10.0,
                    interval: 16
                }],
                ..ok
            }
            .validate()
            .is_ok(),
            "equal to the far interval is fine"
        );
        assert!(PolicyError("x").to_string() == "x");
    }

    // ---- the scheduler -------------------------------------------------------------------------------------

    #[test]
    fn the_hash_spreads_keys_that_step_by_powers_of_two_too() {
        // Entity ids often carry a generation in the high bits or step by a stride: the low bits of the hash must not follow the low bits of the key.
        for stride in [1u64, 2, 8, 64, 1 << 20] {
            let mut buckets = [0u32; 8];
            for k in 0..8_000u64 {
                buckets[(mix(k * stride) % 8) as usize] += 1;
            }
            assert!(
                buckets.iter().all(|b| (850..1150).contains(b)),
                "stride {stride}: {buckets:?}"
            );
        }
        // And a one-bit change in the key changes about half the bits of the hash.
        let flipped: u32 = (0..64)
            .map(|bit| (mix(12_345) ^ mix(12_345 ^ (1 << bit))).count_ones())
            .sum();
        assert!(
            (64 * 24..64 * 40).contains(&flipped),
            "average {} bits flip",
            f64::from(flipped) / 64.0
        );
    }

    #[test]
    fn the_hash_is_splitmix64_so_its_published_reference_value_holds() {
        // The first output of splitmix64 seeded with 0 (the reference implementation's well-known value).
        assert_eq!(mix(0), 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn the_hash_spreads_sequential_keys() {
        let mut buckets = [0u32; 8];
        for k in 0..8_000u64 {
            buckets[(mix(k) % 8) as usize] += 1;
        }
        assert!(
            buckets.iter().all(|b| (900..1100).contains(b)),
            "{buckets:?}"
        );
        assert_ne!(mix(1), mix(2));
        assert_eq!(mix(7), mix(7));
    }

    #[test]
    fn agents_with_one_rate_are_staggered_and_each_thinks_once_per_interval() {
        let mut ai = AiScheduler::new(10_000);
        let agents: Vec<(u64, u32)> = (0..800).map(|k| (k, 8)).collect();
        let mut per_tick = Vec::new();
        let mut count = BTreeMap::new();
        for tick in 0..16u64 {
            let thoughts = ai.plan(tick, &agents);
            per_tick.push(thoughts.len());
            for t in thoughts {
                *count.entry(t.key).or_insert(0u32) += 1;
            }
        }
        assert_eq!(count.len(), 800);
        assert!(count.values().all(|c| *c == 2), "twice in sixteen ticks");
        let (min, max) = (
            *per_tick.iter().min().unwrap(),
            *per_tick.iter().max().unwrap(),
        );
        assert!(
            max <= 130 && min >= 70,
            "800 agents every 8 ticks is 100 a tick: {per_tick:?}"
        );
    }

    #[test]
    fn a_thought_reports_the_ticks_since_the_last_one() {
        let mut ai = AiScheduler::new(100);
        let mut last: BTreeMap<u64, u64> = BTreeMap::new();
        let agents: Vec<(u64, u32)> = vec![(1, 4), (2, 1), (3, 7)];
        for tick in 0..40u64 {
            for t in ai.plan(tick, &agents) {
                if let Some(prev) = last.insert(t.key, tick) {
                    assert_eq!(
                        u64::from(t.dt_ticks),
                        tick - prev,
                        "agent {} at {tick}",
                        t.key
                    );
                } else {
                    assert_eq!(
                        t.dt_ticks,
                        agents.iter().find(|a| a.0 == t.key).unwrap().1,
                        "a first thought reports its interval"
                    );
                }
            }
        }
        assert!(last.len() == 3);
    }

    #[test]
    fn a_cap_defers_the_least_overdue_and_nobody_waits_more_than_a_bounded_time() {
        // 1,000 agents all wanting every tick, 100 thoughts a tick: each should think about every 10 ticks.
        let mut ai = AiScheduler::new(100);
        let agents: Vec<(u64, u32)> = (0..1_000).map(|k| (k, 1)).collect();
        let mut last: BTreeMap<u64, u64> = BTreeMap::new();
        let mut worst_gap = 0;
        for tick in 0..300u64 {
            let thoughts = ai.plan(tick, &agents);
            assert!(thoughts.len() <= 100);
            for t in thoughts {
                if let Some(prev) = last.insert(t.key, tick) {
                    worst_gap = worst_gap.max(tick - prev);
                    // The wanted interval is 1 but the cap made the agent wait: it is told how long it really was.
                    assert_eq!(u64::from(t.dt_ticks), tick - prev, "agent {}", t.key);
                }
            }
        }
        assert_eq!(last.len(), 1_000, "every agent got to think");
        assert!(
            (10..=11).contains(&worst_gap),
            "the most overdue go first, so the longest wait stays at 10 (+1): {worst_gap}"
        );
    }

    #[test]
    fn the_most_overdue_go_first_then_the_lowest_key() {
        let mut ai = AiScheduler::new(2);
        // All three are new with interval 1 (phase 0): all due at tick 0; only two fit. Keys 5 and 9 first (lowest keys), 12 waits.
        let agents: Vec<(u64, u32)> = vec![(12, 1), (9, 1), (5, 1)];
        let first: Vec<u64> = ai.plan(0, &agents).iter().map(|t| t.key).collect();
        assert_eq!(first, [5, 9]);
        let second: Vec<u64> = ai.plan(1, &agents).iter().map(|t| t.key).collect();
        assert_eq!(
            second,
            [12, 5],
            "12 is overdue by one tick; then the lowest key among the rest"
        );
    }

    #[test]
    fn a_changed_interval_takes_effect_from_the_last_thought_and_a_new_agent_starts_within_its_interval()
     {
        let mut ai = AiScheduler::new(100);
        let mut tick = 0;
        // Start slow: interval 10.
        let mut thinks = Vec::new();
        while thinks.len() < 2 {
            if !ai.plan(tick, &[(77, 10)]).is_empty() {
                thinks.push(tick);
            }
            tick += 1;
        }
        assert_eq!(thinks[1] - thinks[0], 10);
        assert!(
            thinks[0] < 10,
            "the first thought comes within one interval: {}",
            thinks[0]
        );
        // The agent comes close: interval 1. It thinks again on the very next tick that is at least 1 after the last.
        let at = thinks[1];
        assert!(
            ai.plan(at + 1, &[(77, 1)])
                .iter()
                .any(|t| t.key == 77 && u64::from(t.dt_ticks) == 1)
        );
        // Going slow again waits the full new interval from the last thought.
        assert!(ai.plan(at + 2, &[(77, 5)]).is_empty());
        assert!(ai.plan(at + 5, &[(77, 5)]).is_empty());
        assert_eq!(ai.plan(at + 6, &[(77, 5)]).len(), 1);
    }

    #[test]
    fn despawned_agents_are_forgotten_and_zero_means_one() {
        let mut ai = AiScheduler::new(100);
        let agents: Vec<(u64, u32)> = (0..50).map(|k| (k, 0)).collect();
        assert_eq!(
            ai.plan(0, &agents).len(),
            50,
            "an interval of 0 counts as 1: everyone thinks at once"
        );
        assert_eq!(ai.tracked(), 50);
        ai.plan(1, &agents[..10]);
        assert_eq!(ai.tracked(), 10);
        assert!(ai.plan(2, &[]).is_empty());
        assert_eq!(ai.tracked(), 0);
        assert_eq!(
            AiScheduler::new(0).plan(0, &[(1, 1), (2, 1)]).len(),
            1,
            "a cap of 0 still lets one think"
        );
    }

    #[test]
    fn a_new_agent_cut_by_the_cap_stays_overdue_and_is_not_given_a_fresh_phase() {
        let mut ai = AiScheduler::new(1);
        let agents: Vec<(u64, u32)> = (0..4).map(|k| (k, 1)).collect();
        let mut seen = BTreeSet::new();
        for tick in 0..4 {
            for t in ai.plan(tick, &agents) {
                seen.insert(t.key);
            }
        }
        assert_eq!(
            seen.len(),
            4,
            "one a tick for four ticks: all four got a turn"
        );
    }

    // ---- path requests -------------------------------------------------------------------------------------------

    fn req(key: u64, priority: u8, cost: u32) -> PathRequest {
        PathRequest {
            key,
            priority,
            cost,
        }
    }

    #[test]
    fn requests_are_answered_by_priority_then_by_how_long_they_waited_then_by_key() {
        let mut q = PathBudget::new(PathConfig {
            max_per_tick: 10,
            aging_ticks: 0,
            ..PathConfig::default()
        });
        q.submit(req(5, 1, 10), 3).unwrap();
        q.submit(req(2, 1, 10), 3).unwrap();
        q.submit(req(9, 9, 10), 4).unwrap();
        q.submit(req(7, 1, 10), 1).unwrap();
        let order: Vec<u64> = q.take(5).iter().map(|r| r.key).collect();
        assert_eq!(
            order,
            [9, 7, 2, 5],
            "priority 9 first; then tick 1 before tick 3; then key 2 before 5"
        );
        assert_eq!(q.pending(), 0);
    }

    #[test]
    fn ageing_lets_a_patient_low_priority_request_overtake_a_fresh_urgent_one() {
        let cfg = PathConfig {
            max_per_tick: 1,
            aging_ticks: 10,
            ..PathConfig::default()
        };
        let mut q = PathBudget::new(cfg);
        q.submit(req(1, 1, 10), 0).unwrap(); // priority 1, waiting
        q.submit(req(2, 5, 10), 95).unwrap(); // priority 5, fresh
        assert_eq!(
            q.take(95)[0].key,
            1,
            "after 95 ticks the old one counts as 1 + 9 = 10 against the new one's 5"
        );
        // But not yet after 30 ticks: 1 + 3 = 4 is below 5.
        let mut early = PathBudget::new(cfg);
        early.submit(req(1, 1, 10), 0).unwrap();
        early.submit(req(2, 5, 10), 30).unwrap();
        assert_eq!(early.take(30)[0].key, 2);
        // At 40 ticks 1 + 4 = 5 ties with the new one, and the older request wins the tie.
        let mut edge = PathBudget::new(cfg);
        edge.submit(req(1, 1, 10), 0).unwrap();
        edge.submit(req(2, 5, 10), 40).unwrap();
        assert_eq!(edge.take(40)[0].key, 1);
    }

    #[test]
    fn the_per_tick_count_and_cost_limits_hold_and_the_first_request_is_always_taken() {
        let cfg = PathConfig {
            max_per_tick: 3,
            max_cost_per_tick: 100,
            aging_ticks: 0,
            max_queue: 100,
        };
        let mut q = PathBudget::new(cfg);
        for (k, cost) in [(1, 60), (2, 60), (3, 30), (4, 5), (5, 5)] {
            q.submit(req(k, 1, cost), 0).unwrap();
        }
        let first: Vec<u64> = q.take(0).iter().map(|r| r.key).collect();
        assert_eq!(
            first,
            [1, 3, 4],
            "60 + 30 + 5 = 95; request 2 (60) did not fit and was skipped, a cheaper one behind it was taken, three is the count limit"
        );
        let second: Vec<u64> = q.take(1).iter().map(|r| r.key).collect();
        assert_eq!(second, [2, 5]);
        // An expensive request alone is still answered.
        q.submit(req(8, 1, 1_000_000), 2).unwrap();
        assert_eq!(q.take(2)[0].key, 8);
        // And one that is first in line is not blocked by its own cost even with others behind it.
        q.submit(req(8, 9, 500), 3).unwrap();
        q.submit(req(9, 1, 10), 3).unwrap();
        assert_eq!(
            q.take(3).iter().map(|r| r.key).collect::<Vec<_>>(),
            [8],
            "500 goes first although over the limit, and then 10 more does not fit"
        );
        assert_eq!(
            q.take(4).iter().map(|r| r.key).collect::<Vec<_>>(),
            [9],
            "the one that waited is answered next tick"
        );
    }

    #[test]
    fn asking_again_replaces_the_request_but_keeps_its_place_in_line() {
        let mut q = PathBudget::new(PathConfig {
            max_per_tick: 1,
            aging_ticks: 0,
            ..PathConfig::default()
        });
        q.submit(req(1, 1, 10), 0).unwrap();
        q.submit(req(2, 1, 10), 5).unwrap();
        q.submit(req(2, 1, 20), 9).unwrap(); // agent 2 asks again with a new destination
        q.submit(req(1, 1, 99), 9).unwrap();
        assert_eq!(q.pending(), 2, "one request per agent");
        let t = q.take(10);
        assert_eq!(
            (t[0].key, t[0].cost),
            (1, 99),
            "agent 1 asked first and still goes first, with its newest request"
        );
        assert_eq!(q.take(11)[0].cost, 20);
    }

    #[test]
    fn the_queue_is_bounded_cancel_works_and_a_replacement_needs_no_room() {
        let mut q = PathBudget::new(PathConfig {
            max_queue: 2,
            ..PathConfig::default()
        });
        q.submit(req(1, 1, 1), 0).unwrap();
        q.submit(req(2, 1, 1), 0).unwrap();
        assert_eq!(q.submit(req(3, 1, 1), 0), Err(QueueFull));
        assert!(q.submit(req(2, 5, 1), 1).is_ok(), "replacing is not adding");
        assert!(q.is_pending(1) && !q.is_pending(3));
        assert!(q.cancel(1) && !q.cancel(1));
        assert!(q.submit(req(3, 1, 1), 2).is_ok(), "room again");
        assert!(QueueFull.to_string().contains("full"));
        assert!(PathBudget::new(PathConfig::default()).take(0).is_empty());
    }

    // ---- the horde -----------------------------------------------------------------------------------------------

    #[test]
    fn a_horde_of_eight_hundred_stays_within_budget_and_no_agent_is_neglected() {
        let policy = RatePolicy::default();
        let mut ai = AiScheduler::new(150);
        let mut paths = PathBudget::new(PathConfig::default());
        // 800 agents spread from 5 to 400 units away, a third of them hidden; they drift closer and further as the player moves (distances change every 50 ticks).
        let n = 800u64;
        let mut thoughts_per_tick = Vec::new();
        let mut last_think: BTreeMap<u64, u64> = BTreeMap::new();
        let mut worst_gap: BTreeMap<u64, u64> = BTreeMap::new();
        let mut path_wait: BTreeMap<u64, u64> = BTreeMap::new();
        let mut worst_path_wait = 0;
        for tick in 0..600u64 {
            let phase = (tick / 50) % 4;
            let agents: Vec<(u64, u32)> = (0..n)
                .map(|k| {
                    let distance = 5.0 + ((k * 37 + phase * 91) % 396) as f32;
                    (k, policy.interval(distance, k % 3 != 0))
                })
                .collect();
            let thoughts = ai.plan(tick, &agents);
            thoughts_per_tick.push(thoughts.len());
            for t in &thoughts {
                if let Some(prev) = last_think.insert(t.key, tick) {
                    let gap = worst_gap.entry(t.key).or_insert(0);
                    *gap = (*gap).max(tick - prev);
                }
                // Every thought that is a multiple of 4 asks for a path.
                if (t.key + tick) % 4 == 0 {
                    let _ = paths.submit(
                        req(t.key, (t.key % 5) as u8, 50 + (t.key % 7) as u32 * 100),
                        tick,
                    );
                    path_wait.entry(t.key).or_insert(tick);
                }
            }
            for answered in paths.take(tick) {
                if let Some(since) = path_wait.remove(&answered.key) {
                    worst_path_wait = worst_path_wait.max(tick - since);
                }
            }
        }
        let max = *thoughts_per_tick.iter().max().unwrap();
        let mean = thoughts_per_tick.iter().sum::<usize>() as f64 / thoughts_per_tick.len() as f64;
        assert!(max <= 150, "the cap holds: {max}");
        assert!(
            mean < 150.0 && mean > 40.0,
            "far fewer than 800 a tick, and not nothing: {mean}"
        );
        assert_eq!(
            last_think.len(),
            n as usize,
            "every agent thought at least once"
        );
        let slowest = policy.max_interval as u64;
        let worst = *worst_gap.values().max().unwrap();
        assert!(
            worst <= slowest + 20,
            "no agent waited much longer than the slowest rate allows: {worst}"
        );
        assert!(
            worst_path_wait < 400,
            "no path request waited forever: {worst_path_wait}"
        );
    }
}
