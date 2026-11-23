//! Comparing rendered audio with a reference (roadmap T06-013, golden-audio tests): the audio counterpart of the golden-image tolerance.
//!
//! Two limits, like `ss2_render::golden`: the **largest single difference** (catches a click or a dropout that an average hides) and the
//! **RMS difference** (catches a level or a filter change spread over everything). Different machines may differ in the last bits of a float, so
//! a golden test compares within a tolerance, never for equality.

/// How two renderings differ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioDiff {
    /// The largest absolute difference between two samples.
    pub max_abs: f32,
    /// The root mean square of the differences.
    pub rms: f32,
    /// How many samples were compared.
    pub samples: usize,
}

/// The limits a rendering must stay within.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioTolerance {
    /// The largest allowed single difference.
    pub max_abs: f32,
    /// The largest allowed RMS difference.
    pub rms: f32,
}

impl Default for AudioTolerance {
    /// 1e-4 for a single sample (about 0.001 dB at full scale) and 2e-5 RMS: a different machine's rounding passes, a changed gain or an off-by-one sample does not.
    fn default() -> Self {
        Self {
            max_abs: 1e-4,
            rms: 2e-5,
        }
    }
}

impl AudioTolerance {
    /// Whether `diff` is within the limits.
    pub fn accepts(&self, diff: &AudioDiff) -> bool {
        diff.max_abs <= self.max_abs && diff.rms <= self.rms
    }
}

/// Compare two sample sequences; `None` if their lengths differ or a sample is not finite (a NaN in a rendering is a failure, not a difference).
pub fn compare(a: &[f32], b: &[f32]) -> Option<AudioDiff> {
    if a.len() != b.len() || a.iter().chain(b).any(|s| !s.is_finite()) {
        return None;
    }
    let (mut max_abs, mut sum) = (0.0f32, 0.0f64);
    for (x, y) in a.iter().zip(b) {
        let d = (x - y).abs();
        max_abs = max_abs.max(d);
        sum += f64::from(d) * f64::from(d);
    }
    let rms = if a.is_empty() {
        0.0
    } else {
        (sum / a.len() as f64).sqrt() as f32
    };
    Some(AudioDiff {
        max_abs,
        rms,
        samples: a.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_renderings_have_no_difference_and_pass() {
        let a: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.01).sin()).collect();
        let d = compare(&a, &a).unwrap();
        assert_eq!((d.max_abs, d.rms, d.samples), (0.0, 0.0, 1000));
        assert!(AudioTolerance::default().accepts(&d));
        assert_eq!(compare(&[], &[]).unwrap().rms, 0.0);
    }

    #[test]
    fn a_single_click_fails_on_the_peak_though_the_rms_is_tiny() {
        let a = vec![0.0f32; 48_000];
        let mut b = a.clone();
        b[20_000] = 0.5; // one sample, a click
        let d = compare(&a, &b).unwrap();
        assert!(d.rms < 0.003, "the average hides it: {}", d.rms);
        assert!(!AudioTolerance::default().accepts(&d));
        assert_eq!(d.max_abs, 0.5);
    }

    #[test]
    fn a_level_change_everywhere_fails_on_the_rms_and_rounding_noise_passes() {
        let a: Vec<f32> = (0..4800).map(|i| 0.5 * (i as f32 * 0.05).sin()).collect();
        let quieter: Vec<f32> = a.iter().map(|s| s * 0.99).collect(); // 0.09 dB
        assert!(!AudioTolerance::default().accepts(&compare(&a, &quieter).unwrap()));
        let noisy: Vec<f32> = a
            .iter()
            .enumerate()
            .map(|(i, s)| s + if i % 2 == 0 { 1e-6 } else { -1e-6 })
            .collect();
        assert!(AudioTolerance::default().accepts(&compare(&a, &noisy).unwrap()));
    }

    #[test]
    fn different_lengths_and_non_finite_samples_are_not_comparable() {
        assert!(compare(&[0.0; 3], &[0.0; 4]).is_none());
        assert!(compare(&[0.0, f32::NAN], &[0.0, 0.0]).is_none());
        assert!(compare(&[0.0, 0.0], &[f32::INFINITY, 0.0]).is_none());
    }
}
