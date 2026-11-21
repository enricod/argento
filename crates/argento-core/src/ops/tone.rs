//! Stage 2, global tone: brightness and contrast.
//!
//! Both are curves in perceptual space (sRGB-encoded luminance) that keep
//! black and white where they are, so they never clip.

use crate::image::{GrayImage, linear_to_srgb, srgb_to_linear};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ToneParams {
    /// From -1 (darker midtones) to 1 (lighter midtones).
    pub brightness: f32,
    /// From -1 (flatter) to 1 (punchier), pivoting on the middle grey.
    pub contrast: f32,
}

pub fn apply(image: &GrayImage, params: &ToneParams) -> GrayImage {
    if *params == ToneParams::default() {
        return image.clone();
    }
    let gamma = (-params.brightness.clamp(-1.0, 1.0)).exp2();
    let steepness = params.contrast.clamp(-1.0, 1.0).exp2();
    image.map(|l| {
        // Black and white are returned exactly: the sRGB round trip leaves
        // white at 0.99999994, and the curves, very steep near the ends when
        // contrast is negative, would turn that into a visible drop.
        if l <= 0.0 {
            return 0.0;
        }
        if l >= 1.0 {
            return 1.0;
        }
        let v = linear_to_srgb(l).powf(gamma);
        srgb_to_linear(s_curve(v, steepness))
    })
}

/// An S-curve through (0, 0), (0.5, 0.5) and (1, 1): steeper than the
/// diagonal in the middle when `k > 1`, flatter when `k < 1`.
fn s_curve(v: f32, k: f32) -> f32 {
    if v < 0.5 {
        0.5 * (2.0 * v).powf(k)
    } else {
        1.0 - 0.5 * (2.0 * (1.0 - v)).powf(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MID: f32 = 0.214_041; // sRGB 0.5 in linear light

    fn tone(l: f32, brightness: f32, contrast: f32) -> f32 {
        let image = GrayImage::new(1, 1, vec![l]);
        apply(&image, &ToneParams { brightness, contrast }).data()[0]
    }

    #[test]
    fn default_parameters_change_nothing() {
        for l in [0.0, 0.05, MID, 0.7, 1.0] {
            assert_eq!(tone(l, 0.0, 0.0), l);
        }
    }

    #[test]
    fn black_and_white_never_move() {
        for (b, c) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (0.7, -0.4)] {
            assert!(tone(0.0, b, c).abs() < 1e-6);
            assert!((tone(1.0, b, c) - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn brightness_lifts_or_lowers_the_midtones() {
        assert!(tone(MID, 0.5, 0.0) > MID);
        assert!(tone(MID, -0.5, 0.0) < MID);
    }

    #[test]
    fn contrast_pivots_on_the_middle_grey() {
        assert!((tone(MID, 0.0, 0.8) - MID).abs() < 1e-5);
        let (dark, light) = (0.05, 0.6);
        assert!(tone(dark, 0.0, 0.8) < dark && tone(light, 0.0, 0.8) > light);
        assert!(tone(dark, 0.0, -0.8) > dark && tone(light, 0.0, -0.8) < light);
    }

    #[test]
    fn the_curves_keep_the_order_of_tones() {
        for (b, c) in [(1.0, 1.0), (-1.0, -1.0), (0.3, -1.0)] {
            let mut previous = -1.0;
            for i in 0..=100 {
                let v = tone(i as f32 / 100.0, b, c);
                assert!(v >= previous, "not monotonic at {i} for {b}, {c}");
                previous = v;
            }
        }
    }
}
