//! Frame timing: the **numbers and text** behind a frame-time overlay and GPU pass timings (roadmap T04-019, `docs/FRAME_TIMING.md`).
//!
//! Device-free. The CPU side of profiling (scopes, a viewer) is `puffin` in `ss2-app` (T11-011, `docs/PROFILING.md`); this module is what a renderer and an overlay
//! need on top of it and can be tested without a GPU:
//!
//! * [`FrameTimeHistory`]: a ring of the last N frame times and the [`FrameTimeStats`] of it (average, worst, percentiles, the "1 % low" frame rate, hitches);
//! * [`resolve_pass_timings`]: turns the raw 64-bit **GPU timestamp ticks** of a frame's passes into milliseconds, with the tick period the graphics API
//!   reports, tolerating counter wrap-around and refusing the garbage a disjoint GPU timer gives;
//! * [`overlay_lines`]: the text of the overlay, one line per row, and [`classify`] to colour a time against a target.
//!
//! Nothing here reads a clock or touches the GPU: the caller supplies frame times and tick values.

/// Longest GPU time one pass is believed to take, in milliseconds. A "duration" beyond it comes from a disjoint timer (a power-state change, a device reset) and is
/// reported as unknown rather than as a nonsense number.
pub const MAX_PLAUSIBLE_PASS_MS: f64 = 1000.0;

/// How a time compares with its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// At or under the target.
    Good,
    /// Over the target by up to half of it.
    Warn,
    /// Over by more than half.
    Bad,
}

/// Compare a time with a target (both in milliseconds). Non-finite or non-positive input is [`Severity::Bad`]: if the number cannot be trusted the overlay should not
/// show green.
pub fn classify(ms: f32, target_ms: f32) -> Severity {
    if !(ms.is_finite() && target_ms.is_finite() && target_ms > 0.0 && ms >= 0.0) {
        Severity::Bad
    } else if ms <= target_ms {
        Severity::Good
    } else if ms <= target_ms * 1.5 {
        Severity::Warn
    } else {
        Severity::Bad
    }
}

/// What a history of frame times says.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameTimeStats {
    /// Frames in the history.
    pub count: usize,
    /// Mean frame time in ms.
    pub avg_ms: f32,
    /// Shortest frame in ms.
    pub min_ms: f32,
    /// Longest frame in ms.
    pub max_ms: f32,
    /// Median frame time (nearest rank) in ms.
    pub p50_ms: f32,
    /// 95th percentile (nearest rank) in ms.
    pub p95_ms: f32,
    /// 99th percentile (nearest rank) in ms.
    pub p99_ms: f32,
    /// `1000 / avg_ms`.
    pub fps_avg: f32,
    /// `1000 / p99_ms`: the frame rate the slowest 1 % of frames run at; what a player feels as stutter.
    pub fps_1pct_low: f32,
    /// Frames longer than twice the median (visible hitches).
    pub hitches: usize,
}

/// The last `capacity` frame times.
///
/// ```
/// use ss2_render_api::frame_timing::FrameTimeHistory;
///
/// let mut h = FrameTimeHistory::new(240).unwrap();
/// for _ in 0..239 {
///     h.push(16.0);
/// }
/// h.push(80.0); // one hitch
/// let s = h.stats().unwrap();
/// assert_eq!(s.hitches, 1);
/// assert_eq!(s.max_ms, 80.0);
/// assert!(s.fps_avg > 60.0 && s.fps_avg < 62.0);
/// ```
#[derive(Clone, Debug)]
pub struct FrameTimeHistory {
    times: Vec<f32>,
    next: usize,
    capacity: usize,
}

impl FrameTimeHistory {
    /// A history of up to `capacity` frames (1 to 100,000).
    pub fn new(capacity: usize) -> Option<Self> {
        (1..=100_000).contains(&capacity).then(|| Self {
            times: Vec::with_capacity(capacity),
            next: 0,
            capacity,
        })
    }

    /// Record a frame time in milliseconds. A time that is not a finite, non-negative number is ignored (a stall on a window event must not poison the statistics).
    pub fn push(&mut self, ms: f32) {
        if !(ms.is_finite() && ms >= 0.0) {
            return;
        }
        if self.times.len() < self.capacity {
            self.times.push(ms);
        } else {
            self.times[self.next] = ms;
        }
        self.next = (self.next + 1) % self.capacity;
    }

    /// How many frames are recorded.
    pub fn len(&self) -> usize {
        self.times.len()
    }

    /// Whether nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    /// The recorded times, **oldest first** (for drawing a graph).
    pub fn ordered(&self) -> Vec<f32> {
        if self.times.len() < self.capacity {
            self.times.clone()
        } else {
            self.times[self.next..]
                .iter()
                .chain(&self.times[..self.next])
                .copied()
                .collect()
        }
    }

    /// Forget everything (a level change: loading frames would skew the numbers).
    pub fn clear(&mut self) {
        self.times.clear();
        self.next = 0;
    }

    /// The statistics, or `None` for an empty history.
    pub fn stats(&self) -> Option<FrameTimeStats> {
        if self.times.is_empty() {
            return None;
        }
        let mut sorted = self.times.clone();
        sorted.sort_by(f32::total_cmp);
        let n = sorted.len();
        let rank = |p: usize| sorted[(n * p).div_ceil(100).clamp(1, n) - 1];
        let avg = (self.times.iter().map(|t| f64::from(*t)).sum::<f64>() / n as f64) as f32;
        let p50 = rank(50);
        let p99 = rank(99);
        let per_second = |ms: f32| if ms > 0.0 { 1000.0 / ms } else { f32::INFINITY };
        Some(FrameTimeStats {
            count: n,
            avg_ms: avg,
            min_ms: sorted[0],
            max_ms: sorted[n - 1],
            p50_ms: p50,
            p95_ms: rank(95),
            p99_ms: p99,
            fps_avg: per_second(avg),
            fps_1pct_low: per_second(p99),
            hitches: self.times.iter().filter(|t| **t > p50 * 2.0).count(),
        })
    }
}

/// The time between two GPU timestamps in milliseconds, or `None` if it is not believable.
///
/// `period_ns` is the number of nanoseconds per tick that the graphics API reports for its timestamp queries. The counter is 64 bits and may wrap, so the
/// difference is taken with wrap-around (`end` below `begin` is a wrap, not a negative time). A zero tick is "not written" and gives `None`, as does a period that is
/// not positive and finite, and any duration over [`MAX_PLAUSIBLE_PASS_MS`].
pub fn ticks_to_ms(begin: u64, end: u64, period_ns: f32) -> Option<f64> {
    if begin == 0 || end == 0 || !(period_ns.is_finite() && period_ns > 0.0) {
        return None;
    }
    let ticks = end.wrapping_sub(begin);
    let ms = ticks as f64 * f64::from(period_ns) / 1.0e6;
    (ms <= MAX_PLAUSIBLE_PASS_MS).then_some(ms)
}

/// One pass's GPU time.
#[derive(Clone, Debug, PartialEq)]
pub struct PassTiming {
    /// The pass name.
    pub name: String,
    /// Its GPU time in ms, or `None` if the timestamps could not be trusted.
    pub ms: Option<f64>,
}

/// The GPU times of a frame's passes. `ticks` holds one timestamp **between** consecutive passes: `names.len() + 1` values, the first before the first pass and the
/// last after the last. Returns `None` if the counts do not match (a bug or a lost query).
pub fn resolve_pass_timings(
    names: &[&str],
    ticks: &[u64],
    period_ns: f32,
) -> Option<Vec<PassTiming>> {
    if ticks.len() != names.len() + 1 {
        return None;
    }
    Some(
        names
            .iter()
            .zip(ticks.windows(2))
            .map(|(name, pair)| PassTiming {
                name: (*name).to_owned(),
                ms: ticks_to_ms(pair[0], pair[1], period_ns),
            })
            .collect(),
    )
}

/// The rows of the frame-time overlay. `target_ms` is the frame budget (16.7 for 60 fps) and `gpu` the pass timings of the last frame, if the device has timestamp
/// queries. The caller draws them and may colour a row by [`classify`].
///
/// ```
/// use ss2_render_api::frame_timing::{overlay_lines, FrameTimeHistory};
///
/// let mut h = FrameTimeHistory::new(60).unwrap();
/// for _ in 0..60 {
///     h.push(10.0);
/// }
/// let lines = overlay_lines(h.stats().as_ref(), None, 16.7);
/// assert_eq!(lines[0], "frame 10.0 ms avg  10.0 max  (100 fps, 1% low 100)");
/// ```
pub fn overlay_lines(
    stats: Option<&FrameTimeStats>,
    gpu: Option<&[PassTiming]>,
    target_ms: f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    match stats {
        None => lines.push("frame: no data".to_owned()),
        Some(s) => {
            lines.push(format!(
                "frame {:.1} ms avg  {:.1} max  ({:.0} fps, 1% low {:.0})",
                s.avg_ms, s.max_ms, s.fps_avg, s.fps_1pct_low
            ));
            lines.push(format!(
                "p50 {:.1}  p95 {:.1}  p99 {:.1} ms   hitches {}",
                s.p50_ms, s.p95_ms, s.p99_ms, s.hitches
            ));
            if classify(s.avg_ms, target_ms) != Severity::Good {
                lines.push(format!("over the {target_ms:.1} ms budget"));
            }
        }
    }
    match gpu {
        None => lines.push("gpu: no timestamp queries".to_owned()),
        Some(passes) => {
            let known: Vec<f64> = passes.iter().filter_map(|p| p.ms).collect();
            if known.len() == passes.len() && !passes.is_empty() {
                lines.push(format!("gpu {:.2} ms", known.iter().sum::<f64>()));
            } else {
                lines.push("gpu ? ms (some timings unavailable)".to_owned());
            }
            for p in passes {
                match p.ms {
                    Some(ms) => lines.push(format!("  {:<12} {:.2}", p.name, ms)),
                    None => lines.push(format!("  {:<12} ?", p.name)),
                }
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history(times: &[f32]) -> FrameTimeHistory {
        let mut h = FrameTimeHistory::new(times.len().max(1)).unwrap();
        for t in times {
            h.push(*t);
        }
        h
    }

    // ---- severity -------------------------------------------------------------------------------

    #[test]
    fn severity_compares_with_the_target_and_distrusts_bad_numbers() {
        assert_eq!(classify(16.0, 16.7), Severity::Good);
        assert_eq!(
            classify(16.7, 16.7),
            Severity::Good,
            "exactly on target is good"
        );
        assert_eq!(classify(16.8, 16.7), Severity::Warn);
        assert_eq!(classify(25.0, 16.7), Severity::Warn, "25.05 is the limit");
        assert_eq!(classify(25.1, 16.7), Severity::Bad);
        assert_eq!(classify(0.0, 16.7), Severity::Good);
        for bad in [
            (f32::NAN, 16.7),
            (f32::INFINITY, 16.7),
            (-1.0, 16.7),
            (10.0, 0.0),
            (10.0, -5.0),
            (10.0, f32::NAN),
        ] {
            assert_eq!(classify(bad.0, bad.1), Severity::Bad, "{bad:?}");
        }
    }

    // ---- history --------------------------------------------------------------------------------

    #[test]
    fn percentiles_are_nearest_rank_on_a_known_series() {
        let times: Vec<f32> = (1..=100).map(|i| i as f32).collect();
        let s = history(&times).stats().unwrap();
        assert_eq!((s.count, s.min_ms, s.max_ms), (100, 1.0, 100.0));
        assert_eq!((s.p50_ms, s.p95_ms, s.p99_ms), (50.0, 95.0, 99.0));
        assert!((s.avg_ms - 50.5).abs() < 1e-4);
        assert!((s.fps_avg - 1000.0 / 50.5).abs() < 1e-3);
        assert!((s.fps_1pct_low - 1000.0 / 99.0).abs() < 1e-3);
    }

    #[test]
    fn a_single_frame_and_unsorted_input_work() {
        let s = history(&[7.0]).stats().unwrap();
        assert_eq!(
            (s.min_ms, s.max_ms, s.p50_ms, s.p99_ms, s.hitches),
            (7.0, 7.0, 7.0, 7.0, 0)
        );
        let s = history(&[30.0, 10.0, 20.0, 40.0]).stats().unwrap();
        assert_eq!((s.min_ms, s.max_ms), (10.0, 40.0));
        assert_eq!(
            s.p50_ms, 20.0,
            "nearest rank of 4 values at 50 %: the 2nd smallest"
        );
        assert!(FrameTimeHistory::new(4).unwrap().stats().is_none());
    }

    #[test]
    fn hitches_are_frames_over_twice_the_median() {
        let mut times = vec![16.0f32; 97];
        times.extend([33.0, 40.0, 90.0]); // 33 is just over 2 x 16
        let s = history(&times).stats().unwrap();
        assert_eq!(s.hitches, 3);
        let mut times = vec![16.0f32; 99];
        times.push(32.0); // exactly twice is not a hitch
        assert_eq!(history(&times).stats().unwrap().hitches, 0);
    }

    #[test]
    fn the_ring_keeps_the_newest_frames_and_orders_them_oldest_first() {
        let mut h = FrameTimeHistory::new(4).unwrap();
        for t in 1..=6 {
            h.push(t as f32);
        }
        assert_eq!(h.len(), 4);
        assert_eq!(h.ordered(), [3.0, 4.0, 5.0, 6.0]);
        let s = h.stats().unwrap();
        assert_eq!(
            (s.min_ms, s.max_ms),
            (3.0, 6.0),
            "the evicted frames no longer count"
        );
        // Before it is full the order is the arrival order.
        let mut h = FrameTimeHistory::new(4).unwrap();
        h.push(9.0);
        h.push(8.0);
        assert_eq!(h.ordered(), [9.0, 8.0]);
        h.clear();
        assert!(h.is_empty() && h.stats().is_none() && h.ordered().is_empty());
        h.push(5.0);
        assert_eq!(
            h.ordered(),
            [5.0],
            "a cleared history starts at the beginning again"
        );
    }

    #[test]
    fn bad_frame_times_are_ignored_and_a_bad_capacity_is_refused() {
        let mut h = FrameTimeHistory::new(10).unwrap();
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.5] {
            h.push(bad);
        }
        assert!(h.is_empty());
        h.push(0.0);
        assert_eq!(h.len(), 1, "zero is a time");
        assert!(FrameTimeHistory::new(0).is_none() && FrameTimeHistory::new(100_001).is_none());
        assert!(FrameTimeHistory::new(1).is_some() && FrameTimeHistory::new(100_000).is_some());
        // A zero average does not divide by zero.
        let s = history(&[0.0, 0.0]).stats().unwrap();
        assert_eq!(s.fps_avg, f32::INFINITY);
        assert_eq!(s.fps_1pct_low, f32::INFINITY);
    }

    #[test]
    fn percentiles_round_up_when_the_count_is_not_a_multiple_of_a_hundred() {
        // 7 frames: the 99th percentile is the 7th (ceil(6.93)), the 95th the 7th (ceil(6.65)), the median the 4th (ceil(3.5)).
        let s = history(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0])
            .stats()
            .unwrap();
        assert_eq!((s.p50_ms, s.p95_ms, s.p99_ms), (4.0, 7.0, 7.0));
        // 250 frames of 1..=250: the 95th percentile is the 238th (ceil(237.5)).
        let times: Vec<f32> = (1..=250).map(|i| i as f32).collect();
        assert_eq!(history(&times).stats().unwrap().p95_ms, 238.0);
    }

    #[test]
    fn a_cleared_ring_that_refills_is_in_order_again() {
        let mut h = FrameTimeHistory::new(4).unwrap();
        for t in 1..=6 {
            h.push(t as f32);
        }
        h.clear();
        for t in 10..=13 {
            h.push(t as f32);
        }
        assert_eq!(h.ordered(), [10.0, 11.0, 12.0, 13.0]);
        h.push(14.0);
        assert_eq!(h.ordered(), [11.0, 12.0, 13.0, 14.0]);
    }

    #[test]
    fn a_negative_zero_frame_time_gives_an_infinite_rate_not_a_negative_one() {
        let s = history(&[-0.0]).stats().unwrap();
        assert_eq!(s.fps_avg, f32::INFINITY);
        assert_eq!(s.fps_1pct_low, f32::INFINITY);
    }

    #[test]
    fn exactly_one_and_a_half_times_the_target_is_still_a_warning() {
        assert_eq!(classify(15.0, 10.0), Severity::Warn);
        assert_eq!(classify(15.001, 10.0), Severity::Bad);
    }

    // ---- GPU timestamps ---------------------------------------------------------------------------

    #[test]
    fn ticks_become_milliseconds_through_the_period() {
        // 1,000,000 ticks at 1 ns each is 1 ms; at 52.08 ns (a typical 19.2 MHz counter) it is 52.08 ms.
        assert!((ticks_to_ms(100, 1_000_100, 1.0).unwrap() - 1.0).abs() < 1e-9);
        assert!((ticks_to_ms(5_000, 1_005_000, 52.08).unwrap() - 52.08).abs() < 1e-4);
        assert_eq!(
            ticks_to_ms(10, 10, 1.0),
            Some(0.0),
            "an instant pass is 0, not unknown"
        );
    }

    #[test]
    fn the_counter_wrapping_is_a_short_time_not_a_huge_one() {
        let begin = u64::MAX - 499_999;
        let end = 500_000; // 1,000,000 ticks later across the wrap
        assert!((ticks_to_ms(begin, end, 1.0).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn unwritten_or_implausible_timestamps_are_unknown() {
        assert_eq!(ticks_to_ms(0, 1000, 1.0), None, "begin not written");
        assert_eq!(ticks_to_ms(1000, 0, 1.0), None, "end not written");
        // Even with a period so small that the wrapped difference looks plausible, an unwritten end is unknown.
        assert_eq!(ticks_to_ms(1000, 0, 1.0e-12), None);
        for period in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(ticks_to_ms(1, 1000, period), None, "{period}");
        }
        // Just under and just over the plausibility limit (1000 ms = 1e9 ticks at 1 ns).
        assert!(ticks_to_ms(1, 1_000_000_001, 1.0).is_some());
        assert_eq!(ticks_to_ms(1, 1_000_000_002, 1.0), None);
        // A disjoint timer typically gives end far before begin: through the wrap that is a gigantic time.
        assert_eq!(ticks_to_ms(5_000_000_000, 4_000_000_000, 1.0), None);
    }

    #[test]
    fn pass_timings_pair_each_name_with_the_ticks_around_it() {
        let names = ["depth", "opaque", "post"];
        let ticks = [1_000u64, 1_001_000, 4_001_000, 5_001_000];
        let t = resolve_pass_timings(&names, &ticks, 1.0).unwrap();
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].name, "depth");
        assert!((t[0].ms.unwrap() - 1.0).abs() < 1e-9);
        assert!((t[1].ms.unwrap() - 3.0).abs() < 1e-9);
        assert_eq!(t[2].name, "post");
        assert!((t[2].ms.unwrap() - 1.0).abs() < 1e-9);
        // Counts that do not match are refused; no passes needs exactly one tick.
        assert!(resolve_pass_timings(&names, &ticks[..3], 1.0).is_none());
        assert!(resolve_pass_timings(&names, &[1, 2, 3, 4, 5], 1.0).is_none());
        assert_eq!(resolve_pass_timings(&[], &[5], 1.0), Some(vec![]));
        assert!(resolve_pass_timings(&[], &[], 1.0).is_none());
        // One bad timestamp spoils only the passes next to it.
        let t = resolve_pass_timings(&names, &[1_000, 0, 4_001_000, 5_001_000], 1.0).unwrap();
        assert_eq!((t[0].ms, t[1].ms), (None, None));
        assert!(t[2].ms.is_some());
    }

    // ---- overlay ----------------------------------------------------------------------------------

    #[test]
    fn the_overlay_shows_frame_numbers_and_flags_a_missed_budget() {
        let good = history(&[10.0; 30]).stats().unwrap();
        let lines = overlay_lines(Some(&good), None, 16.7);
        assert_eq!(
            lines[0],
            "frame 10.0 ms avg  10.0 max  (100 fps, 1% low 100)"
        );
        assert_eq!(lines[1], "p50 10.0  p95 10.0  p99 10.0 ms   hitches 0");
        assert_eq!(lines[2], "gpu: no timestamp queries");
        assert_eq!(lines.len(), 3);
        let slow = history(&[25.0; 30]).stats().unwrap();
        let lines = overlay_lines(Some(&slow), None, 16.7);
        assert_eq!(lines[2], "over the 16.7 ms budget");
        assert_eq!(
            overlay_lines(None, None, 16.7),
            ["frame: no data", "gpu: no timestamp queries"]
        );
    }

    #[test]
    fn the_overlay_lists_gpu_passes_and_marks_unknown_ones() {
        let stats = history(&[10.0; 5]).stats().unwrap();
        let all = vec![
            PassTiming {
                name: "depth".into(),
                ms: Some(1.25),
            },
            PassTiming {
                name: "opaque".into(),
                ms: Some(3.5),
            },
        ];
        let lines = overlay_lines(Some(&stats), Some(&all), 16.7);
        assert_eq!(lines[2], "gpu 4.75 ms");
        assert_eq!(lines[3], "  depth        1.25");
        assert_eq!(lines[4], "  opaque       3.50");
        let some_unknown = vec![
            PassTiming {
                name: "depth".into(),
                ms: Some(1.0),
            },
            PassTiming {
                name: "post".into(),
                ms: None,
            },
        ];
        let lines = overlay_lines(Some(&stats), Some(&some_unknown), 16.7);
        assert_eq!(lines[2], "gpu ? ms (some timings unavailable)");
        assert_eq!(lines[4], "  post         ?");
        let none: Vec<PassTiming> = Vec::new();
        assert_eq!(
            overlay_lines(Some(&stats), Some(&none), 16.7)[2],
            "gpu ? ms (some timings unavailable)",
            "no passes is not a time of zero"
        );
    }
}
