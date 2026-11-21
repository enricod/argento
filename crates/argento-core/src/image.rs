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

    /// A copy whose longer side is at most `max_side` pixels, each pixel the
    /// average of the source pixels it covers. An image already small enough
    /// is copied unchanged.
    pub fn downscale_to_fit(&self, max_side: u32) -> RgbImage {
        let (sw, sh) = (self.width as usize, self.height as usize);
        let longest = sw.max(sh);
        let max_side = max_side.max(1) as usize;
        if longest <= max_side {
            return self.clone();
        }
        let scaled = |side: usize| (side * max_side).div_ceil(longest).max(1);
        let (w, h) = (scaled(sw), scaled(sh));

        let mut data = vec![0.0; w * h * 3];
        data.par_chunks_mut(w * 3).enumerate().for_each(|(y, row)| {
            let (y0, y1) = (y * sh / h, ((y + 1) * sh / h).max(y * sh / h + 1));
            for x in 0..w {
                let (x0, x1) = (x * sw / w, ((x + 1) * sw / w).max(x * sw / w + 1));
                let mut sum = [0.0f32; 3];
                for sy in y0..y1 {
                    let line = &self.data[(sy * sw + x0) * 3..(sy * sw + x1) * 3];
                    for pixel in line.as_chunks::<3>().0 {
                        for (total, v) in sum.iter_mut().zip(pixel) {
                            *total += v;
                        }
                    }
                }
                let count = ((y1 - y0) * (x1 - x0)) as f32;
                for c in 0..3 {
                    row[x * 3 + c] = sum[c] / count;
                }
            }
        });
        RgbImage::from_linear(w as u32, h as u32, data)
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

/// A single-channel image in linear light: one `f32` luminance sample per
/// pixel, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct GrayImage {
    width: u32,
    height: u32,
    data: Vec<f32>,
}

impl GrayImage {
    /// # Panics
    /// If `data` does not hold exactly `width * height` samples.
    pub fn new(width: u32, height: u32, data: Vec<f32>) -> Self {
        assert_eq!(
            data.len(),
            width as usize * height as usize,
            "an {width}x{height} grey image needs width * height samples"
        );
        Self { width, height, data }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// Applies `f` to every sample.
    pub fn map(&self, f: impl Fn(f32) -> f32 + Sync) -> GrayImage {
        let data = self.data.par_iter().map(|&v| f(v)).collect();
        GrayImage::new(self.width, self.height, data)
    }

    /// The same grey in all three channels.
    pub fn to_rgb(&self) -> RgbImage {
        let data = self.data.par_iter().flat_map_iter(|&v| [v, v, v]).collect();
        RgbImage::from_linear(self.width, self.height, data)
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
    fn downscale_keeps_the_aspect_ratio_and_fits_the_longer_side() {
        let image = RgbImage::from_linear(4000, 3000, vec![0.5; 4000 * 3000 * 3]);
        let proxy = image.downscale_to_fit(2048);
        assert_eq!((proxy.width(), proxy.height()), (2048, 1536));
        assert!(proxy.data().iter().all(|&v| (v - 0.5).abs() < 1e-6));
    }

    #[test]
    fn downscale_averages_in_linear_light() {
        // A black and a white pixel become one mid-grey pixel (0.5 linear).
        let image = RgbImage::from_linear(2, 1, vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        assert_eq!(image.downscale_to_fit(1).data(), [0.5, 0.5, 0.5]);
    }

    #[test]
    fn downscale_leaves_a_small_image_alone() {
        let image = RgbImage::from_linear(3, 2, (0..18).map(|v| v as f32).collect());
        assert_eq!(image.downscale_to_fit(2048), image);
    }

    #[test]
    fn downscale_never_reaches_zero_pixels() {
        let image = RgbImage::from_linear(1000, 1, vec![0.2; 3000]);
        let proxy = image.downscale_to_fit(10);
        assert_eq!((proxy.width(), proxy.height()), (10, 1));
    }

    #[test]
    fn grey_to_rgb_repeats_the_value_in_each_channel() {
        let grey = GrayImage::new(2, 1, vec![0.1, 0.9]);
        assert_eq!(grey.to_rgb().data(), [0.1, 0.1, 0.1, 0.9, 0.9, 0.9]);
    }

    #[test]
    #[should_panic(expected = "width * height * 3")]
    fn from_linear_rejects_the_wrong_number_of_samples() {
        RgbImage::from_linear(2, 2, vec![0.0; 11]);
    }
}
