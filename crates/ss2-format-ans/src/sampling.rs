//! Pose evaluation, looping, event iteration and root-motion deltas.
//!
//! **Decisions this module makes because `docs/re/ans.md` leaves them open** (T01-058):
//!
//! - *Interpolation* between two keys: translation and scalars linearly, rotation by spherical
//!   linear interpolation along the shortest arc (falling back to a normalised lerp for nearly
//!   equal keys). The spec calls this "the likely answer".
//! - *Before the first and after the last key* of a track the value is held (the data always
//!   has keys on the animation's first and last frame, so this only matters for sparse input).
//! - *Time*: frame = `first_frame + time * fps`. The span from the first to the last key is
//!   `last - first` frames ([`Animation::span_seconds`]); a looping animation repeats every
//!   [`Animation::loop_period_frames`] frames.
//! - *Events* fire when their frame is passed: a query covers the half-open time range
//!   `[from, to)`, so consecutive ticks never fire an event twice or miss one.

use crate::{AnimEvent, Animation, Key};

/// What happens when time runs past the end of an animation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoopMode {
    /// Play once and hold the last frame.
    #[default]
    Once,
    /// Repeat from the start.
    Loop,
}

/// The values of every track of an animation at one moment, parallel to the animation's track
/// lists: `translations[i]` belongs to `Animation::translations[i]`, and so on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pose {
    /// Parent-relative local translations.
    pub translations: Vec<[f32; 3]>,
    /// Parent-relative local rotations, unit quaternions `[x, y, z, w]`.
    pub rotations: Vec<[f32; 4]>,
    /// Float channels.
    pub scalars: Vec<f32>,
}

impl Pose {
    /// Blend `self` towards `other` by `t` (0 gives `self`, 1 gives `other`, clamped): linear for
    /// translations and scalars, spherical for rotations. Both poses must come from the same
    /// animation (track counts are checked: extra tracks of the longer pose are dropped).
    pub fn blend(&self, other: &Self, t: f32) -> Self {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        Self {
            translations: self
                .translations
                .iter()
                .zip(&other.translations)
                .map(|(a, b)| lerp3(*a, *b, t))
                .collect(),
            rotations: self
                .rotations
                .iter()
                .zip(&other.rotations)
                .map(|(a, b)| slerp(*a, *b, t))
                .collect(),
            scalars: self
                .scalars
                .iter()
                .zip(&other.scalars)
                .map(|(a, b)| a + (b - a) * t)
                .collect(),
        }
    }
}

/// An event reported by [`Animation::events_between`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiredEvent<'a> {
    /// The event.
    pub event: &'a AnimEvent,
    /// Seconds since the start of the animation at which it fires (loops counted).
    pub time: f64,
    /// Which pass of the animation it belongs to (0 is the first).
    pub loop_index: u64,
}

/// Most loop passes one event query looks at (a guard against absurd time ranges).
pub const MAX_LOOPS_PER_QUERY: u64 = 1024;

pub(crate) fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Spherical interpolation along the shortest arc between unit quaternions.
pub(crate) fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let mut dot: f64 = a
        .iter()
        .zip(&b)
        .map(|(x, y)| f64::from(*x) * f64::from(*y))
        .sum();
    let sign = if dot < 0.0 { -1.0 } else { 1.0 };
    dot *= sign;
    let (wa, wb) = if dot > 0.9995 {
        (1.0 - f64::from(t), f64::from(t))
    } else {
        let theta = dot.min(1.0).acos();
        let s = theta.sin();
        (
            ((1.0 - f64::from(t)) * theta).sin() / s,
            (f64::from(t) * theta).sin() / s,
        )
    };
    let mut q = [0.0f64; 4];
    for i in 0..4 {
        q[i] = wa * f64::from(a[i]) + wb * sign * f64::from(b[i]);
    }
    let n = q.iter().map(|c| c * c).sum::<f64>().sqrt();
    if n < 1e-12 || !n.is_finite() {
        return a;
    }
    q.map(|c| (c / n) as f32)
}

/// Value of a key list at `frame`: held before the first and after the last key.
fn sample_keys<T: Copy>(keys: &[Key<T>], frame: f32, lerp: impl Fn(T, T, f32) -> T) -> T {
    debug_assert!(!keys.is_empty(), "the loader rejects empty tracks");
    let first = &keys[0];
    let last = &keys[keys.len() - 1];
    // A NaN frame holds the first key.
    if frame.is_nan() || frame <= first.frame as f32 {
        return first.value;
    }
    if frame >= last.frame as f32 {
        return last.value;
    }
    let next = keys.partition_point(|k| (k.frame as f32) <= frame);
    let (a, b) = (&keys[next - 1], &keys[next]);
    let u = (frame - a.frame as f32) / (b.frame - a.frame) as f32;
    lerp(a.value, b.value, u)
}

impl Animation {
    /// Number of frames from the first to the last frame inclusive: `last - first + 1`.
    pub fn frame_count(&self) -> u32 {
        (i64::from(self.last_frame) - i64::from(self.first_frame) + 1) as u32
    }

    /// Time from the first to the last key frame, in seconds: `(last - first) / fps`.
    pub fn span_seconds(&self) -> f64 {
        f64::from(self.last_frame - self.first_frame) / f64::from(self.fps)
    }

    /// Length of one pass of a looping animation, in frames: `last - first` (at least 1), so the
    /// last frame is the first frame of the next pass.
    ///
    /// This is a choice, not a fact of the format: in the retail `.ans` files 479 of the 851
    /// animations with more than one frame have identical poses on their first and last frame
    /// (within 1e-3), which is what a loop with a duplicated end frame looks like.
    pub fn loop_period_frames(&self) -> f64 {
        f64::from((self.last_frame - self.first_frame).max(1))
    }

    /// The (fractional) frame number shown `time` seconds after the start. Negative and
    /// non-finite times count as 0. With [`LoopMode::Once`] the result stops at `last_frame`;
    /// with [`LoopMode::Loop`] it wraps into `first_frame..last_frame`.
    pub fn frame_at(&self, time: f64, mode: LoopMode) -> f64 {
        let elapsed = clean_time(time) * f64::from(self.fps);
        let first = f64::from(self.first_frame);
        match mode {
            LoopMode::Once => first + elapsed.min(f64::from(self.last_frame - self.first_frame)),
            LoopMode::Loop => first + elapsed.rem_euclid(self.loop_period_frames()),
        }
    }

    /// The pose `time` seconds after the start.
    pub fn sample(&self, time: f64, mode: LoopMode) -> Pose {
        let mut pose = Pose::default();
        self.sample_into(time, mode, &mut pose);
        pose
    }

    /// Like [`Self::sample`], reusing the vectors of `pose` (no allocation once they are sized).
    pub fn sample_into(&self, time: f64, mode: LoopMode, pose: &mut Pose) {
        self.sample_frame_into(self.frame_at(time, mode) as f32, pose);
    }

    /// The pose at a frame number (held outside the keys, no wrapping).
    pub fn sample_frame(&self, frame: f32) -> Pose {
        let mut pose = Pose::default();
        self.sample_frame_into(frame, &mut pose);
        pose
    }

    /// Like [`Self::sample_frame`], reusing the vectors of `pose`.
    pub fn sample_frame_into(&self, frame: f32, pose: &mut Pose) {
        pose.translations.clear();
        pose.translations.extend(
            self.translations
                .iter()
                .map(|t| sample_keys(&t.keys, frame, lerp3)),
        );
        pose.rotations.clear();
        pose.rotations.extend(
            self.rotations
                .iter()
                .map(|t| sample_keys(&t.keys, frame, slerp)),
        );
        pose.scalars.clear();
        pose.scalars.extend(
            self.scalars
                .iter()
                .map(|t| sample_keys(&t.keys, frame, |a, b, u| a + (b - a) * u)),
        );
    }

    /// The events that fire in the time range `[from, to)` (seconds since the start; loops
    /// counted), in the order they fire.
    ///
    /// - [`LoopMode::Once`]: events whose frame lies in the range; time stops mattering after
    ///   the animation's end (events past the last frame fire once if the range covers them).
    /// - [`LoopMode::Loop`]: every pass of [`Self::loop_period_frames`] frames fires its events
    ///   again, except events marked `only_in_first_loop`, which fire in pass 0 only. Events
    ///   outside the pass (before the first frame or at or after the end of the period) never fire
    ///   in a loop. At most [`MAX_LOOPS_PER_QUERY`] passes (the latest ones) are examined.
    ///
    /// Empty if `to <= from`; negative and non-finite times count as 0.
    pub fn events_between(&self, from: f64, to: f64, mode: LoopMode) -> Vec<FiredEvent<'_>> {
        let (from, to) = (clean_time(from), clean_time(to));
        let mut out = Vec::new();
        if to <= from || self.events.is_empty() {
            return out;
        }
        let fps = f64::from(self.fps);
        let (from_f, to_f) = (from * fps, to * fps);
        match mode {
            LoopMode::Once => self.collect(0, 0.0, from_f, to_f, false, &mut out),
            LoopMode::Loop => {
                let period = self.loop_period_frames();
                let last_pass = (to_f / period).floor() as u64;
                let first_pass = ((from_f / period).floor() as u64)
                    .max(last_pass.saturating_sub(MAX_LOOPS_PER_QUERY - 1));
                for pass in first_pass..=last_pass {
                    let start = pass as f64 * period;
                    let lo = (from_f - start).max(0.0);
                    let hi = (to_f - start).min(period);
                    if hi > lo {
                        self.collect(pass, start, lo, hi, true, &mut out);
                    }
                }
            }
        }
        for e in &mut out {
            e.time /= fps;
        }
        out
    }

    /// Push the events with pass-relative frame offset in `[lo, hi)` (frames since the start of
    /// the pass); `time` is temporarily in frames.
    fn collect<'a>(
        &'a self,
        pass: u64,
        start: f64,
        lo: f64,
        hi: f64,
        looping: bool,
        out: &mut Vec<FiredEvent<'a>>,
    ) {
        let first = f64::from(self.first_frame);
        let period = self.loop_period_frames();
        let mut found: Vec<(f64, &AnimEvent)> = self
            .events
            .iter()
            .filter(|e| !(pass > 0 && e.only_in_first_loop))
            .map(|e| (f64::from(e.frame) - first, e))
            .filter(|(rel, _)| *rel >= lo && *rel < hi)
            .filter(|(rel, _)| !looping || (*rel >= 0.0 && *rel < period))
            .collect();
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        out.extend(found.into_iter().map(|(rel, event)| FiredEvent {
            event,
            time: start + rel,
            loop_index: pass,
        }));
    }

    /// How far the translation track `track` moves between two times: the **root-motion delta**
    /// of that track (which track is the root is the caller's business; the spec does not say).
    ///
    /// In [`LoopMode::Loop`] a pass boundary is crossed without counting the jump from the last
    /// key back to the first: the delta of a pass is `value(last frame) - value(first frame)`,
    /// so an entity driven by it moves smoothly across loops. Times beyond the end of a
    /// [`LoopMode::Once`] animation hold the last value. `None` if `track` is out of range.
    /// Reversed times (`to < from`) give the negated delta.
    pub fn translation_delta(
        &self,
        track: usize,
        from: f64,
        to: f64,
        mode: LoopMode,
    ) -> Option<[f32; 3]> {
        let t = self.translations.get(track)?;
        let at = |frame: f64| sample_keys(&t.keys, frame as f32, lerp3);
        let (first, last) = (f64::from(self.first_frame), f64::from(self.last_frame));
        let (from, to) = (clean_time(from), clean_time(to));
        let fps = f64::from(self.fps);
        let (pa, pb) = (from * fps, to * fps);
        let value = |p: f64| -> (u64, [f32; 3]) {
            match mode {
                LoopMode::Once => (0, at(first + p.min(last - first))),
                LoopMode::Loop => {
                    let period = self.loop_period_frames();
                    let pass = (p / period).floor();
                    let local = p - pass * period;
                    (pass as u64, at((first + local).min(last)))
                }
            }
        };
        let (pass_a, va) = value(pa);
        let (pass_b, vb) = value(pb);
        let per_pass = {
            let (s, e) = (at(first), at(last));
            [e[0] - s[0], e[1] - s[1], e[2] - s[2]]
        };
        let passes = pass_b as f32 - pass_a as f32;
        Some([
            vb[0] - va[0] + passes * per_pass[0],
            vb[1] - va[1] + passes * per_pass[1],
            vb[2] - va[2] + passes * per_pass[2],
        ])
    }
}

fn clean_time(t: f64) -> f64 {
    if t.is_finite() && t > 0.0 { t } else { 0.0 }
}
