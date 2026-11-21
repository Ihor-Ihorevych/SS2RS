//! Image comparison for golden-image tests.

use crate::ImageRgba8;

/// Mean absolute per-channel difference between two images, in 0..=255 units.
/// `None` if the images differ in size.
pub fn mean_abs_diff(a: &ImageRgba8, b: &ImageRgba8) -> Option<f32> {
    if a.width != b.width || a.height != b.height || a.pixels.len() != b.pixels.len() {
        return None;
    }
    if a.pixels.is_empty() {
        return Some(0.0);
    }
    let total: u64 = a
        .pixels
        .iter()
        .zip(&b.pixels)
        .map(|(&x, &y)| x.abs_diff(y) as u64)
        .sum();
    Some(total as f32 / a.pixels.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ImageRgba8;

    fn img(w: u32, h: u32, fill: u8) -> ImageRgba8 {
        ImageRgba8 {
            width: w,
            height: h,
            pixels: vec![fill; (w * h * 4) as usize],
        }
    }

    #[test]
    fn mean_abs_diff_identical_is_zero() {
        assert_eq!(mean_abs_diff(&img(4, 4, 10), &img(4, 4, 10)), Some(0.0));
    }

    #[test]
    fn mean_abs_diff_detects_change() {
        let a = img(2, 2, 0);
        let mut b = img(2, 2, 0);
        b.pixels[0] = 255; // one channel of one pixel, 16 channels in total
        let d = mean_abs_diff(&a, &b).unwrap();
        assert!((d - 255.0 / 16.0).abs() < 1e-4, "{d}");
    }

    #[test]
    fn mean_abs_diff_size_mismatch_is_none() {
        assert_eq!(mean_abs_diff(&img(2, 2, 0), &img(2, 3, 0)), None);
    }
}
