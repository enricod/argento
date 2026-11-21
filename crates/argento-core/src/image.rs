//! Image buffers in the linear working space, and conversions in and out of it.

use rayon::prelude::*;

/// An RGB image in linear light: three interleaved `f32` samples per pixel,
/// row by row, with 0.0 as black and 1.0 as diffuse white.
#[derive(Clone, Debug, PartialEq)]
pub struct RgbImage {
    width: u32,
    height: u32,
    data: Vec<f32>,
}

impl RgbImage {
    /// Wraps samples that are already linear.
    ///
    /// # Panics
    /// If `data` does not hold exactly `width * height * 3` samples.
    pub fn from_linear(width: u32, height: u32, data: Vec<f32>) -> Self {
        assert_eq!(
            data.len(),
            width as usize * height as usize * 3,
            "an {width}x{height} RGB image needs width * height * 3 samples"
        );
        Self { width, height, data }
    }

    /// Converts sRGB-encoded samples in `[0, 1]` to the linear working space.
    ///
    /// # Panics
    /// If `data` does not hold exactly `width * height * 3` samples.
    pub fn from_srgb(width: u32, height: u32, mut data: Vec<f32>) -> Self {
        data.par_iter_mut().for_each(|v| *v = srgb_to_linear(*v));
        Self::from_linear(width, height, data)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The linear samples, three per pixel.
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// Encodes the image as 8-bit sRGB, three bytes per pixel, for display.
    /// Values outside `[0, 1]` are clipped.
    pub fn to_srgb8(&self) -> Vec<u8> {
        self.data
            .par_iter()
            .map(|&v| (linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8)
            .collect()
    }
}

/// The sRGB transfer function, from encoded value to linear light.
pub fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// The inverse sRGB transfer function, from linear light to encoded value.
pub fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_functions_keep_black_and_white() {
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-6);
        assert_eq!(linear_to_srgb(0.0), 0.0);
        assert!((linear_to_srgb(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn srgb_mid_grey_is_about_21_percent_linear() {
        assert!((srgb_to_linear(0.5) - 0.214_041).abs() < 1e-5);
    }

    #[test]
    fn transfer_functions_are_inverse() {
        for i in 0..=1000 {
            let v = i as f32 / 1000.0;
            assert!((linear_to_srgb(srgb_to_linear(v)) - v).abs() < 1e-5, "at {v}");
        }
    }

    #[test]
    fn every_8_bit_value_survives_the_round_trip() {
        let encoded: Vec<u8> = (0..=255).flat_map(|v| [v, v, v]).collect();
        let samples = encoded.iter().map(|&v| v as f32 / 255.0).collect();
        let image = RgbImage::from_srgb(256, 1, samples);
        assert_eq!(image.to_srgb8(), encoded);
    }

    #[test]
    fn to_srgb8_clips_out_of_range_values() {
        let image = RgbImage::from_linear(1, 1, vec![-0.5, 2.0, 1.0]);
        assert_eq!(image.to_srgb8(), [0, 255, 255]);
    }

    #[test]
    #[should_panic(expected = "width * height * 3")]
    fn from_linear_rejects_the_wrong_number_of_samples() {
        RgbImage::from_linear(2, 2, vec![0.0; 11]);
    }
}
