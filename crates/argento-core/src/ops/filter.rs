//! Stage 1: from RGB to luminance through a coloured filter, as a filter on
//! the lens does with black-and-white film.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::image::{GrayImage, RgbImage};

/// Rec. 709 luminance: how the eye weighs red, green and blue.
const NEUTRAL: [f32; 3] = [0.2126, 0.7152, 0.0722];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Filter {
    /// No filter: plain luminance.
    #[default]
    Neutral,
    /// Darkens blue skies strongly, lightens skin and red.
    Red,
    Orange,
    /// Darkens blue skies a little: the classic landscape filter.
    Yellow,
    /// Lightens foliage, darkens red.
    Green,
    /// Lightens skies and blue, darkens skin and red.
    Blue,
}

impl Filter {
    pub const ALL: [Filter; 6] = [
        Filter::Neutral,
        Filter::Red,
        Filter::Orange,
        Filter::Yellow,
        Filter::Green,
        Filter::Blue,
    ];

    /// Channel weights at full strength. Every set sums to 1, so a neutral
    /// grey keeps its value whatever the filter.
    fn weights(self) -> [f32; 3] {
        match self {
            Filter::Neutral => NEUTRAL,
            Filter::Red => [0.70, 0.25, 0.05],
            Filter::Orange => [0.55, 0.40, 0.05],
            Filter::Yellow => [0.40, 0.55, 0.05],
            Filter::Green => [0.10, 0.85, 0.05],
            Filter::Blue => [0.10, 0.25, 0.65],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FilterParams {
    pub filter: Filter,
    /// From 0 (no effect, plain luminance) to 1 (the full filter).
    pub strength: f32,
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            filter: Filter::Neutral,
            strength: 1.0,
        }
    }
}

impl FilterParams {
    /// Channel weights: plain luminance blended towards the filter's by the
    /// strength.
    pub fn weights(&self) -> [f32; 3] {
        let t = self.strength.clamp(0.0, 1.0);
        let full = self.filter.weights();
        std::array::from_fn(|c| NEUTRAL[c] + (full[c] - NEUTRAL[c]) * t)
    }
}

pub fn apply(image: &RgbImage, params: &FilterParams) -> GrayImage {
    let [r, g, b] = params.weights();
    let data = image
        .data()
        .par_chunks_exact(3)
        .map(|p| p[0] * r + p[1] * g + p[2] * b)
        .collect();
    GrayImage::new(image.width(), image.height(), data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_pixel(rgb: [f32; 3], filter: Filter, strength: f32) -> f32 {
        let image = RgbImage::from_linear(1, 1, rgb.to_vec());
        apply(&image, &FilterParams { filter, strength }).data()[0]
    }

    #[test]
    fn a_neutral_grey_keeps_its_value_through_every_filter() {
        for filter in Filter::ALL {
            for strength in [0.0, 0.5, 1.0] {
                let v = one_pixel([0.3, 0.3, 0.3], filter, strength);
                assert!((v - 0.3).abs() < 1e-6, "{filter:?} at {strength}: {v}");
            }
        }
    }

    #[test]
    fn neutral_is_rec_709_luminance() {
        let v = one_pixel([1.0, 0.0, 0.0], Filter::Neutral, 1.0);
        assert!((v - 0.2126).abs() < 1e-6);
    }

    #[test]
    fn a_red_filter_lightens_red_and_darkens_blue() {
        let red = [0.8, 0.1, 0.1];
        let blue = [0.1, 0.2, 0.8];
        assert!(one_pixel(red, Filter::Red, 1.0) > one_pixel(red, Filter::Neutral, 1.0));
        assert!(one_pixel(blue, Filter::Red, 1.0) < one_pixel(blue, Filter::Neutral, 1.0));
    }

    #[test]
    fn a_blue_filter_lightens_blue() {
        let blue = [0.1, 0.2, 0.8];
        assert!(one_pixel(blue, Filter::Blue, 1.0) > one_pixel(blue, Filter::Neutral, 1.0));
    }

    #[test]
    fn zero_strength_is_plain_luminance() {
        let colour = [0.7, 0.2, 0.4];
        let v = one_pixel(colour, Filter::Red, 0.0);
        assert!((v - one_pixel(colour, Filter::Neutral, 1.0)).abs() < 1e-6);
    }

    #[test]
    fn half_strength_lies_between_none_and_full() {
        let colour = [0.8, 0.1, 0.1];
        let none = one_pixel(colour, Filter::Red, 0.0);
        let half = one_pixel(colour, Filter::Red, 0.5);
        let full = one_pixel(colour, Filter::Red, 1.0);
        assert!(none < half && half < full);
    }
}
