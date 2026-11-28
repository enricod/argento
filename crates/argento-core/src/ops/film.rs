//! Film stock: how a given emulsion sees colour (stage 1, in place of the
//! colour filter), how it renders tones (stage 4) and its grain (stage 4).

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::blur;
use crate::image::{GrayImage, linear_to_srgb, srgb_to_linear};
use crate::ops::tone::ToneParams;

/// Grain amplitude at full strength, as a fraction of the perceptual range.
/// Small on purpose: grain should be felt in the midtones, not dominate.
const GRAIN_AMPLITUDE: f32 = 0.06;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stock {
    /// Sees all colours roughly as the eye does.
    #[default]
    Panchromatic,
    /// Blind to red: reds render dark, blues and greens light.
    Orthochromatic,
    /// Steep curve, deep blacks and clean whites.
    HighContrast,
    /// Flat curve with lifted shadows, for soft portraits.
    Soft,
}

impl Stock {
    pub const ALL: [Stock; 4] = [
        Stock::Panchromatic,
        Stock::Orthochromatic,
        Stock::HighContrast,
        Stock::Soft,
    ];

    /// Channel weights of the emulsion's spectral response. Each set sums
    /// to 1, so a neutral grey keeps its value, as with the colour filters.
    pub fn spectrum(self) -> [f32; 3] {
        match self {
            Stock::Panchromatic | Stock::Soft => [0.30, 0.59, 0.11],
            Stock::Orthochromatic => [0.00, 0.75, 0.25],
            Stock::HighContrast => [0.35, 0.50, 0.15],
        }
    }

    /// The characteristic curve, as the brightness and contrast it adds to
    /// the image.
    pub fn curve(self) -> ToneParams {
        match self {
            Stock::Panchromatic => ToneParams::default(),
            Stock::Orthochromatic => ToneParams {
                brightness: 0.0,
                contrast: 0.2,
            },
            Stock::HighContrast => ToneParams {
                brightness: -0.1,
                contrast: 0.6,
            },
            Stock::Soft => ToneParams {
                brightness: 0.15,
                contrast: -0.4,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FilmParams {
    pub stock: Stock,
    /// From 0 (no grain) to 1 (heavy grain).
    pub grain: f32,
    /// Grain size as a fraction of the longer side of the image, so that
    /// the preview and the full-resolution export look the same.
    pub grain_size: f32,
}

impl Default for FilmParams {
    fn default() -> Self {
        Self {
            stock: Stock::default(),
            grain: 0.2,
            grain_size: 0.001,
        }
    }
}

/// Stage 4, grain: a random pattern, blurred to the grain size, added in
/// proportion to how midtone the pixel is. Black and white stay untouched.
pub fn grain(image: &GrayImage, params: &FilmParams) -> GrayImage {
    let amount = params.grain.clamp(0.0, 1.0);
    if amount == 0.0 {
        return image.clone();
    }
    let (w, h) = (image.width(), image.height());
    let longer_side = w.max(h) as f32;
    let noise = noise(w, h);
    let pattern = blur::gaussian(&noise, params.grain_size.max(0.0) * longer_side);
    let pattern = normalised(pattern.data());

    let data = image
        .data()
        .par_iter()
        .zip(&pattern)
        .map(|(&l, &n)| {
            let p = linear_to_srgb(l.clamp(0.0, 1.0));
            let midtones = 4.0 * p * (1.0 - p);
            srgb_to_linear((p + amount * GRAIN_AMPLITUDE * midtones * n).clamp(0.0, 1.0))
        })
        .collect();
    GrayImage::new(w, h, data)
}

/// Uniform noise, one value per pixel, from a hash of its position: the same
/// photo always gets the same grain.
fn noise(width: u32, height: u32) -> GrayImage {
    let data = (0..height)
        .flat_map(|y| (0..width).map(move |x| hash(x, y)))
        .collect();
    GrayImage::new(width, height, data)
}

fn hash(x: u32, y: u32) -> f32 {
    let mut h = u64::from(x).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::from(y).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 32;
    (h >> 40) as f32 / (1u64 << 24) as f32 - 0.5
}

/// Rescales to zero mean and unit variance, so that the grain amplitude does
/// not depend on how much the blur smoothed the noise.
fn normalised(data: &[f32]) -> Vec<f32> {
    let n = data.len() as f32;
    let mean = data.iter().sum::<f32>() / n;
    let variance = data.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    let scale = if variance > 0.0 {
        variance.sqrt().recip()
    } else {
        0.0
    };
    data.iter().map(|v| (v - mean) * scale).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::tone;

    fn grey(l: f32) -> GrayImage {
        GrayImage::new(1, 1, vec![l])
    }

    fn lum(stock: Stock, rgb: [f32; 3]) -> f32 {
        let [r, g, b] = stock.spectrum();
        rgb[0] * r + rgb[1] * g + rgb[2] * b
    }

    /// A flat patch of `l` of the given size.
    fn flat(width: u32, height: u32, l: f32) -> GrayImage {
        GrayImage::new(width, height, vec![l; (width * height) as usize])
    }

    #[test]
    fn every_stock_keeps_a_neutral_grey() {
        for stock in Stock::ALL {
            let sum: f32 = stock.spectrum().iter().sum();
            assert!((sum - 1.0).abs() < 1e-6, "{stock:?} sums to {sum}");
            assert!((lum(stock, [0.3, 0.3, 0.3]) - 0.3).abs() < 1e-6);
        }
    }

    #[test]
    fn orthochromatic_film_darkens_red() {
        let red = [0.8, 0.1, 0.1];
        assert!(lum(Stock::Orthochromatic, red) < lum(Stock::Panchromatic, red));
    }

    #[test]
    fn high_contrast_steepens_and_soft_flattens_the_curve() {
        let dark = grey(0.05).data()[0];
        let high = tone::apply(&grey(0.05), &Stock::HighContrast.curve()).data()[0];
        let soft = tone::apply(&grey(0.05), &Stock::Soft.curve()).data()[0];
        assert!(high < dark, "high contrast should darken shadows");
        assert!(soft > dark, "soft should lift shadows");
    }

    #[test]
    fn panchromatic_has_a_neutral_curve() {
        assert_eq!(Stock::Panchromatic.curve(), ToneParams::default());
    }

    #[test]
    fn zero_grain_changes_nothing() {
        let image = flat(8, 8, 0.4);
        let params = FilmParams {
            grain: 0.0,
            ..FilmParams::default()
        };
        assert_eq!(grain(&image, &params), image);
    }

    #[test]
    fn grain_leaves_black_and_white_alone() {
        let params = FilmParams {
            grain: 1.0,
            ..FilmParams::default()
        };
        let image = GrayImage::new(2, 1, vec![0.0, 1.0]);
        assert_eq!(grain(&image, &params).data(), [0.0, 1.0]);
    }

    #[test]
    fn grain_is_visible_in_the_midtones() {
        let params = FilmParams {
            grain: 1.0,
            grain_size: 0.0,
            ..FilmParams::default()
        };
        let out = grain(&flat(64, 64, 0.2), &params);
        let spread = out
            .data()
            .iter()
            .map(|v| (v - 0.2).abs())
            .fold(0.0, f32::max);
        assert!(spread > 0.005, "grain too faint: {spread}");
    }

    #[test]
    fn grain_is_the_same_every_time() {
        let params = FilmParams {
            grain: 0.5,
            ..FilmParams::default()
        };
        let image = flat(32, 32, 0.5);
        assert_eq!(grain(&image, &params), grain(&image, &params));
    }

    /// Share of horizontal neighbours that move the same way from the
    /// midtone: a proxy for how coarse the grain is.
    fn neighbours_agree(size: f32) -> f32 {
        let params = FilmParams {
            grain: 1.0,
            grain_size: size,
            ..FilmParams::default()
        };
        let out = grain(&flat(128, 128, 0.5), &params);
        let d = out.data();
        let pairs = 128 * 127;
        let agree = (0..128)
            .flat_map(|y| (0..127).map(move |x| y * 128 + x))
            .filter(|&i| (d[i] - 0.5).signum() == (d[i + 1] - 0.5).signum())
            .count();
        agree as f32 / pairs as f32
    }

    #[test]
    fn a_larger_grain_size_gives_coarser_grain() {
        assert!(neighbours_agree(0.05) > neighbours_agree(0.0));
    }
}
