//! The golden-image harness (roadmap T04-021): compare a rendered image with a stored one, with a
//! tolerance that cannot be fooled by a small broken region, and leave the evidence when it fails.
//!
//! Why not just the mean difference (`mean_abs_diff`): a missing button is 2 % of a 720p image;
//! its difference averaged over all pixels is under the noise of a different GPU's anti-aliasing,
//! so a mean alone passes a screen with a hole in it. [`Tolerance`] has **two** limits: the mean
//! difference, and the **share of pixels that differ by more than a channel threshold**. A rasteriser
//! that rounds an edge differently changes many pixels by a few levels (under the threshold, so
//! harmless); a missing or moved element changes a block by a lot (over it, so caught).
//!
//! When a golden test fails, [`check_golden`] writes the image that was rendered and a **difference
//! image** (what changed, amplified) next to each other in a folder the caller names (the tests use
//! `target/golden-failures/`), and the error says where they are. Regenerating goldens after an
//! intended change is `cargo xtask golden update` (`docs/GOLDEN_IMAGES.md`).

use std::path::{Path, PathBuf};

use crate::ImageRgba8;

/// The environment variable that forces the software adapter (any of `1`, `true`).
pub const FORCE_SOFTWARE_ENV: &str = "SS2_FORCE_SOFTWARE_ADAPTER";
/// The environment variable that regenerates goldens instead of comparing.
pub const UPDATE_ENV: &str = "UPDATE_GOLDEN";

/// What to compare and how much of a difference is harmless.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    /// Limit of the mean absolute difference over all channels, in 0..=255 units.
    pub mean_abs: f32,
    /// A pixel counts as differing when any channel differs by more than this.
    pub channel_threshold: u8,
    /// The most pixels that may differ, as a share of all pixels (0.001 is 0.1 %).
    pub max_differing_fraction: f32,
}

impl Default for Tolerance {
    fn default() -> Self {
        Self {
            mean_abs: 4.0,
            channel_threshold: 16,
            max_differing_fraction: 0.001,
        }
    }
}

impl Tolerance {
    /// Whether `stats` (made with this tolerance's `channel_threshold`) is within the limits.
    pub fn accepts(&self, stats: &Stats) -> bool {
        stats.mean_abs <= self.mean_abs && stats.fraction() <= self.max_differing_fraction
    }
}

/// How two images of the same size differ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    /// The largest single-channel difference.
    pub max_channel: u8,
    /// Pixels with at least one channel differing by more than the threshold.
    pub differing_pixels: usize,
    /// All pixels.
    pub total_pixels: usize,
    /// Mean absolute difference over all channels.
    pub mean_abs: f32,
}

impl Stats {
    /// `differing_pixels` as a share of `total_pixels` (0 for an empty image).
    pub fn fraction(&self) -> f32 {
        if self.total_pixels == 0 {
            0.0
        } else {
            self.differing_pixels as f32 / self.total_pixels as f32
        }
    }
}

fn comparable(a: &ImageRgba8, b: &ImageRgba8) -> bool {
    a.width == b.width
        && a.height == b.height
        && a.pixels.len() == b.pixels.len()
        && a.pixels.len() == a.width as usize * a.height as usize * 4
}

/// Compare two images; `None` when their sizes differ (or a pixel buffer has the wrong length).
pub fn compare(a: &ImageRgba8, b: &ImageRgba8, channel_threshold: u8) -> Option<Stats> {
    if !comparable(a, b) {
        return None;
    }
    let (mut max_channel, mut differing, mut total) = (0u8, 0usize, 0u64);
    for (pa, pb) in a.pixels.chunks_exact(4).zip(b.pixels.chunks_exact(4)) {
        let mut pixel_max = 0u8;
        for (&x, &y) in pa.iter().zip(pb) {
            let d = x.abs_diff(y);
            total += u64::from(d);
            pixel_max = pixel_max.max(d);
        }
        max_channel = max_channel.max(pixel_max);
        if pixel_max > channel_threshold {
            differing += 1;
        }
    }
    let channels = a.pixels.len();
    Some(Stats {
        max_channel,
        differing_pixels: differing,
        total_pixels: channels / 4,
        mean_abs: if channels == 0 {
            0.0
        } else {
            total as f32 / channels as f32
        },
    })
}

/// A picture of what changed: black where the images agree, red where they differ (the largest
/// channel difference times eight, so a change of 3 levels is visible and 32 saturates). Opaque.
pub fn diff_image(a: &ImageRgba8, b: &ImageRgba8) -> Option<ImageRgba8> {
    if !comparable(a, b) {
        return None;
    }
    let mut pixels = Vec::with_capacity(a.pixels.len());
    for (pa, pb) in a.pixels.chunks_exact(4).zip(b.pixels.chunks_exact(4)) {
        let d = pa
            .iter()
            .zip(pb)
            .map(|(&x, &y)| x.abs_diff(y))
            .max()
            .unwrap_or(0);
        pixels.extend_from_slice(&[d.saturating_mul(8), 0, 0, 255]);
    }
    Some(ImageRgba8 {
        width: a.width,
        height: a.height,
        pixels,
    })
}

/// Encode an image as an 8-bit RGBA PNG.
pub fn encode_png(img: &ImageRgba8) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .map_err(|e| e.to_string())?
        .write_image_data(&img.pixels)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

/// Decode an 8-bit RGBA PNG; anything else (or a damaged file) is an error.
pub fn decode_png(bytes: &[u8]) -> Result<ImageRgba8, String> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "image too large".to_string())?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(format!(
            "must be 8-bit RGBA, is {:?} {:?}",
            info.color_type, info.bit_depth
        ));
    }
    buf.truncate(info.buffer_size());
    Ok(ImageRgba8 {
        width: info.width,
        height: info.height,
        pixels: buf,
    })
}

/// How a check behaves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mode {
    /// Write the rendered image as the new golden instead of comparing.
    pub update: bool,
    /// Where a failing check leaves the rendered and the difference images (`None`: nowhere).
    pub failures_dir: Option<PathBuf>,
}

impl Mode {
    /// Update when [`UPDATE_ENV`] is set; leave failures in `failures_dir`.
    pub fn from_env(failures_dir: PathBuf) -> Self {
        Self {
            update: std::env::var_os(UPDATE_ENV).is_some(),
            failures_dir: Some(failures_dir),
        }
    }
}

/// Why a golden check did not pass.
#[derive(Debug, thiserror::Error)]
pub enum GoldenError {
    /// The name is not a plain `*.png` file name (no folders, no `..`).
    #[error("bad golden name {0:?}: use letters, digits, `.`, `_` and `-`, ending in `.png`")]
    BadName(String),
    /// There is no golden file yet.
    #[error("golden {} is missing; create it with UPDATE_GOLDEN=1 (cargo xtask golden update)", path.display())]
    Missing {
        /// The file that was looked for.
        path: PathBuf,
    },
    /// The golden file (or the folder for it) could not be read or written, or is not an 8-bit RGBA PNG.
    #[error("golden {} cannot be used: {reason}", path.display())]
    Unreadable {
        /// The file.
        path: PathBuf,
        /// What went wrong.
        reason: String,
    },
    /// The golden and the rendered image have different sizes.
    #[error("golden is {}x{} but the rendered image is {}x{}; if the size change is intended, regenerate with UPDATE_GOLDEN=1 (cargo xtask golden update)", golden.0, golden.1, rendered.0, rendered.1)]
    SizeChanged {
        /// The golden's size.
        golden: (u32, u32),
        /// The rendered image's size.
        rendered: (u32, u32),
    },
    /// The images are the same size but differ by more than the tolerance.
    #[error("{name}: {} differing pixels of {} (a pixel differs when a channel is off by more than {} levels; {:.3} %), mean difference {:.2} (limits: {:.3} % and {:.2}){}; if the change is intended, regenerate with UPDATE_GOLDEN=1 (cargo xtask golden update)", stats.differing_pixels, stats.total_pixels, channel_threshold, stats.fraction() * 100.0, stats.mean_abs, max_fraction * 100.0, max_mean, evidence_text(actual, diff))]
    Differs {
        /// The golden's file name.
        name: String,
        /// How they differ.
        stats: Stats,
        /// The channel threshold used.
        channel_threshold: u8,
        /// The tolerance's limit of the differing share.
        max_fraction: f32,
        /// The tolerance's limit of the mean difference.
        max_mean: f32,
        /// Where the rendered image was left.
        actual: Option<PathBuf>,
        /// Where the difference image was left.
        diff: Option<PathBuf>,
    },
}

fn evidence_text(actual: &Option<PathBuf>, diff: &Option<PathBuf>) -> String {
    match (actual, diff) {
        (Some(a), Some(d)) => format!(
            "; rendered image: {}, difference: {}",
            a.display(),
            d.display()
        ),
        _ => String::new(),
    }
}

fn valid_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".png") else {
        return false;
    };
    !stem.is_empty()
        && !name.contains("..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn unreadable(path: &Path, reason: impl ToString) -> GoldenError {
    GoldenError::Unreadable {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}

/// Check `rendered` against the golden file `dir/name`, or in update mode write it there.
pub fn check_golden(
    dir: &Path,
    name: &str,
    rendered: &ImageRgba8,
    tolerance: &Tolerance,
    mode: &Mode,
) -> Result<(), GoldenError> {
    if !valid_name(name) {
        return Err(GoldenError::BadName(name.to_string()));
    }
    let path = dir.join(name);
    if mode.update {
        let bytes = encode_png(rendered).map_err(|e| unreadable(&path, e))?;
        std::fs::create_dir_all(dir).map_err(|e| unreadable(&path, e))?;
        return std::fs::write(&path, bytes).map_err(|e| unreadable(&path, e));
    }
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(GoldenError::Missing { path });
        }
        Err(e) => return Err(unreadable(&path, e)),
    };
    let golden = decode_png(&bytes).map_err(|e| unreadable(&path, e))?;
    if (golden.width, golden.height) != (rendered.width, rendered.height) {
        return Err(GoldenError::SizeChanged {
            golden: (golden.width, golden.height),
            rendered: (rendered.width, rendered.height),
        });
    }
    let stats = compare(rendered, &golden, tolerance.channel_threshold)
        .ok_or_else(|| unreadable(&path, "pixel data has the wrong length"))?;
    if tolerance.accepts(&stats) {
        return Ok(());
    }
    let (mut actual, mut diff) = (None, None);
    if let Some(dir) = &mode.failures_dir {
        let stem = name.strip_suffix(".png").unwrap_or(name);
        let (actual_path, diff_path) = (
            dir.join(format!("{stem}.actual.png")),
            dir.join(format!("{stem}.diff.png")),
        );
        let written = std::fs::create_dir_all(dir).is_ok()
            && encode_png(rendered).is_ok_and(|b| std::fs::write(&actual_path, b).is_ok())
            && diff_image(rendered, &golden)
                .and_then(|d| encode_png(&d).ok())
                .is_some_and(|b| std::fs::write(&diff_path, b).is_ok());
        if written {
            (actual, diff) = (Some(actual_path), Some(diff_path));
        }
    }
    Err(GoldenError::Differs {
        name: name.to_string(),
        stats,
        channel_threshold: tolerance.channel_threshold,
        max_fraction: tolerance.max_differing_fraction,
        max_mean: tolerance.mean_abs,
        actual,
        diff,
    })
}

/// The `force_fallback_adapter` values to try, in order: hardware first and then the software
/// fallback, or only the software adapter when it is forced (a machine-independent result).
pub fn adapter_modes(force_software: bool) -> &'static [bool] {
    if force_software {
        &[true]
    } else {
        &[false, true]
    }
}

/// Whether a value of [`FORCE_SOFTWARE_ENV`] asks for the software adapter.
pub fn force_software_requested_from(value: Option<&str>) -> bool {
    value.is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Whether [`FORCE_SOFTWARE_ENV`] asks for the software adapter.
pub fn force_software_requested() -> bool {
    force_software_requested_from(std::env::var(FORCE_SOFTWARE_ENV).ok().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ImageRgba8;

    fn solid(w: u32, h: u32, value: u8) -> ImageRgba8 {
        ImageRgba8 {
            width: w,
            height: h,
            pixels: vec![value; (w * h * 4) as usize],
        }
    }

    fn with_block(base: &ImageRgba8, x0: u32, y0: u32, w: u32, h: u32, value: u8) -> ImageRgba8 {
        let mut img = base.clone();
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let i = ((y * img.width + x) * 4) as usize;
                img.pixels[i..i + 3].fill(value);
            }
        }
        img
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ss2-golden-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ---- the statistics ---------------------------------------------------------------------

    #[test]
    fn identical_images_have_no_difference() {
        let a = solid(8, 8, 90);
        let stats = compare(&a, &a.clone(), 16).unwrap();
        assert_eq!(
            (
                stats.max_channel,
                stats.differing_pixels,
                stats.total_pixels
            ),
            (0, 0, 64)
        );
        assert_eq!(stats.mean_abs, 0.0);
        assert_eq!(stats.fraction(), 0.0);
        assert!(Tolerance::default().accepts(&stats));
    }

    #[test]
    fn the_statistics_count_pixels_over_the_channel_threshold_not_all_changes() {
        let a = solid(10, 10, 100);
        let mut b = a.clone();
        b.pixels[0] = 110; // one channel of pixel 0 changed by 10: under the threshold of 16
        b.pixels[4] = 130; // one channel of pixel 1 changed by 30: over it
        let stats = compare(&a, &b, 16).unwrap();
        assert_eq!(stats.max_channel, 30);
        assert_eq!(
            stats.differing_pixels, 1,
            "only the change over the threshold counts as a differing pixel"
        );
        assert!((stats.fraction() - 0.01).abs() < 1e-6);
    }

    #[test]
    fn a_change_of_exactly_the_threshold_is_not_a_differing_pixel_but_one_more_is() {
        let a = solid(2, 2, 100);
        let mut at = a.clone();
        at.pixels[0] = 116; // exactly 16
        assert_eq!(compare(&a, &at, 16).unwrap().differing_pixels, 0);
        let mut over = a.clone();
        over.pixels[0] = 117; // 17
        assert_eq!(compare(&a, &over, 16).unwrap().differing_pixels, 1);
        // The alpha channel counts too.
        let mut alpha = a.clone();
        alpha.pixels[3] = 140;
        assert_eq!(compare(&a, &alpha, 16).unwrap().differing_pixels, 1);
    }

    #[test]
    fn a_size_mismatch_is_not_comparable() {
        assert!(compare(&solid(4, 4, 0), &solid(4, 5, 0), 16).is_none());
        assert!(compare(&solid(4, 4, 0), &solid(5, 4, 0), 16).is_none());
    }

    #[test]
    fn a_missing_button_fails_even_though_the_mean_difference_is_tiny() {
        // A 720p screen with a 300 x 60 element missing (2 % of the pixels), changed by 120 levels.
        let golden = solid(1280, 720, 40);
        let broken = with_block(&golden, 400, 300, 300, 60, 160);
        let stats = compare(&broken, &golden, 16).unwrap();
        assert!(
            stats.mean_abs < 4.0,
            "the mean alone would pass it: {}",
            stats.mean_abs
        );
        assert!(stats.fraction() > 0.01);
        assert!(!Tolerance::default().accepts(&stats), "{stats:?}");
    }

    #[test]
    fn rasteriser_noise_passes_and_a_small_real_change_does_not() {
        // Anti-aliasing noise: every pixel off by three levels (under the mean limit of 4).
        let golden = solid(640, 360, 100);
        let noisy = ImageRgba8 {
            pixels: golden.pixels.iter().map(|p| p.wrapping_add(3)).collect(),
            ..golden.clone()
        };
        assert!(
            Tolerance::default().accepts(
                &compare(&noisy, &golden, Tolerance::default().channel_threshold).unwrap()
            )
        );
        // A 40 x 20 block (0.35 % of 640 x 360) changed a lot is over the default limit of 0.1 %.
        let broken = with_block(&golden, 10, 10, 40, 20, 220);
        assert!(
            !Tolerance::default().accepts(
                &compare(&broken, &golden, Tolerance::default().channel_threshold).unwrap()
            )
        );
        // ...and a tolerance can be loosened on purpose.
        let loose = Tolerance {
            max_differing_fraction: 0.01,
            ..Tolerance::default()
        };
        assert!(loose.accepts(&compare(&broken, &golden, loose.channel_threshold).unwrap()));
    }

    #[test]
    fn the_mean_limit_still_applies() {
        let golden = solid(100, 100, 0);
        let dim = solid(100, 100, 9); // every pixel off by 9: under the channel threshold, over the mean limit of 4
        let stats = compare(&dim, &golden, 16).unwrap();
        assert_eq!(stats.differing_pixels, 0);
        assert!(
            !Tolerance::default().accepts(&stats),
            "a uniform shift is a change too: {stats:?}"
        );
    }

    // ---- the difference image and PNG --------------------------------------------------------

    #[test]
    fn the_difference_image_is_black_where_nothing_changed_and_bright_where_it_did() {
        let a = solid(4, 4, 50);
        let b = with_block(&a, 1, 1, 2, 2, 150);
        let d = diff_image(&a, &b).unwrap();
        assert_eq!((d.width, d.height), (4, 4));
        let px = |x: u32, y: u32| {
            let i = ((y * 4 + x) * 4) as usize;
            [
                d.pixels[i],
                d.pixels[i + 1],
                d.pixels[i + 2],
                d.pixels[i + 3],
            ]
        };
        assert_eq!(px(0, 0), [0, 0, 0, 255], "unchanged is black and opaque");
        let changed = px(1, 1);
        assert!(
            changed[0] == 255 && changed[3] == 255,
            "a big change saturates the red: {changed:?}"
        );
        assert!(diff_image(&a, &solid(2, 2, 0)).is_none());
        // A small change is amplified so that it can be seen.
        let c = with_block(&a, 0, 0, 1, 1, 53);
        let small = diff_image(&a, &c).unwrap();
        assert!(
            small.pixels[0] >= 16,
            "a change of 3 is made visible: {}",
            small.pixels[0]
        );
    }

    #[test]
    fn png_round_trips_pixels_exactly() {
        let mut img = solid(7, 5, 0);
        for (i, p) in img.pixels.iter_mut().enumerate() {
            *p = (i * 37 % 251) as u8;
        }
        let bytes = encode_png(&img).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(decode_png(&bytes).unwrap(), img);
        assert!(decode_png(b"not a png").is_err());
        assert!(
            decode_png(&bytes[..20]).is_err(),
            "a truncated file is an error, not a panic"
        );
    }

    // ---- checking against a golden file -------------------------------------------------------

    #[test]
    fn a_matching_image_passes_and_leaves_nothing_behind() {
        let dir = temp_dir("match");
        let golden = with_block(&solid(32, 32, 10), 4, 4, 8, 8, 200);
        std::fs::write(dir.join("a.png"), encode_png(&golden).unwrap()).unwrap();
        let failures = dir.join("failures");
        check_golden(
            &dir,
            "a.png",
            &golden,
            &Tolerance::default(),
            &Mode {
                update: false,
                failures_dir: Some(failures.clone()),
            },
        )
        .unwrap();
        assert!(!failures.exists(), "nothing is written for a pass");
    }

    #[test]
    fn a_failure_writes_the_rendered_image_and_the_difference_and_says_where() {
        let dir = temp_dir("fail");
        let golden = solid(64, 64, 20);
        std::fs::write(dir.join("screen.png"), encode_png(&golden).unwrap()).unwrap();
        let broken = with_block(&golden, 8, 8, 32, 32, 220);
        let failures = dir.join("failures");
        let err = check_golden(
            &dir,
            "screen.png",
            &broken,
            &Tolerance::default(),
            &Mode {
                update: false,
                failures_dir: Some(failures.clone()),
            },
        )
        .unwrap_err();
        assert!(matches!(err, GoldenError::Differs { .. }), "{err}");
        let text = err.to_string();
        assert!(
            text.contains("screen.png")
                && text.contains("differing pixels")
                && text.contains("UPDATE_GOLDEN"),
            "{text}"
        );
        let actual =
            decode_png(&std::fs::read(failures.join("screen.actual.png")).unwrap()).unwrap();
        assert_eq!(actual, broken, "the rendered image is kept");
        let diff = decode_png(&std::fs::read(failures.join("screen.diff.png")).unwrap()).unwrap();
        assert_eq!((diff.width, diff.height), (64, 64));
        assert!(
            text.contains("screen.actual.png") && text.contains("screen.diff.png"),
            "{text}"
        );
    }

    #[test]
    fn a_missing_golden_and_a_size_change_are_clear_errors() {
        let dir = temp_dir("errors");
        let mode = Mode {
            update: false,
            failures_dir: Some(dir.join("f")),
        };
        let err = check_golden(
            &dir,
            "nope.png",
            &solid(4, 4, 0),
            &Tolerance::default(),
            &mode,
        )
        .unwrap_err();
        assert!(
            matches!(err, GoldenError::Missing { .. }) && err.to_string().contains("UPDATE_GOLDEN"),
            "{err}"
        );
        std::fs::write(dir.join("a.png"), encode_png(&solid(8, 8, 0)).unwrap()).unwrap();
        let err =
            check_golden(&dir, "a.png", &solid(4, 4, 0), &Tolerance::default(), &mode).unwrap_err();
        assert!(
            matches!(
                err,
                GoldenError::SizeChanged {
                    golden: (8, 8),
                    rendered: (4, 4)
                }
            ),
            "{err}"
        );
        std::fs::write(dir.join("broken.png"), b"garbage").unwrap();
        assert!(matches!(
            check_golden(
                &dir,
                "broken.png",
                &solid(4, 4, 0),
                &Tolerance::default(),
                &mode
            ),
            Err(GoldenError::Unreadable { .. })
        ));
    }

    #[test]
    fn update_mode_writes_the_golden_and_creates_its_folder_and_never_fails() {
        let dir = temp_dir("update").join("deep").join("er");
        let img = with_block(&solid(16, 16, 0), 2, 2, 4, 4, 255);
        check_golden(
            &dir,
            "new.png",
            &img,
            &Tolerance::default(),
            &Mode {
                update: true,
                failures_dir: None,
            },
        )
        .unwrap();
        assert_eq!(
            decode_png(&std::fs::read(dir.join("new.png")).unwrap()).unwrap(),
            img
        );
        // Updating over an existing, different golden replaces it.
        let other = solid(16, 16, 77);
        check_golden(
            &dir,
            "new.png",
            &other,
            &Tolerance::default(),
            &Mode {
                update: true,
                failures_dir: None,
            },
        )
        .unwrap();
        assert_eq!(
            decode_png(&std::fs::read(dir.join("new.png")).unwrap()).unwrap(),
            other
        );
    }

    #[test]
    fn names_cannot_escape_the_golden_folder() {
        let dir = temp_dir("names");
        for bad in [
            "../x.png",
            "a/../../x.png",
            "/etc/x.png",
            "x\\..\\y.png",
            "",
            "no_extension",
            ".png",
            "..x.png",
            "a..b.png",
            "a b.png",
            "x.png.exe",
            "a/b.png",
            "C:x.png",
        ] {
            let err = check_golden(
                &dir,
                bad,
                &solid(2, 2, 0),
                &Tolerance::default(),
                &Mode {
                    update: true,
                    failures_dir: None,
                },
            )
            .unwrap_err();
            assert!(matches!(err, GoldenError::BadName(_)), "{bad:?}: {err}");
        }
    }

    // ---- the adapter choice --------------------------------------------------------------------

    #[test]
    fn the_adapter_modes_try_hardware_first_unless_a_software_adapter_is_forced() {
        assert_eq!(
            adapter_modes(false),
            &[false, true],
            "hardware first, then the software fallback"
        );
        assert_eq!(
            adapter_modes(true),
            &[true],
            "forced: only the software adapter, for a machine-independent result"
        );
        assert!(force_software_requested_from(Some("1")));
        assert!(force_software_requested_from(Some("true")));
        assert!(!force_software_requested_from(Some("0")));
        assert!(!force_software_requested_from(Some("")));
        assert!(!force_software_requested_from(None));
    }
}
