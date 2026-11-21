//! Stage 2, structure: local contrast, which brings out texture and detail
//! without changing the overall tones.
//!
//! Detail is measured at two scales, as the difference between the image and
//! a blurred copy of it, in perceptual space. It is weighted towards the
//! midtones, so that pure blacks and whites neither clip nor grow halos.

use rayon::prelude::*;

use crate::blur;
use crate::image::{GrayImage, linear_to_srgb, srgb_to_linear};

/// Blur radius of each scale, as a fraction of the image's longer side, and
/// its share of the detail.
const SCALES: [(f32, f32); 2] = [(0.002, 0.6), (0.01, 0.4)];

/// Gain at full strength: positive values add detail more boldly than
/// negative values remove it, since -1 already smooths the detail away.
const GAIN_UP: f32 = 2.0;
const GAIN_DOWN: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StructureParams {
    /// From -1 (smooth) to 1 (strong texture).
    pub amount: f32,
}

pub fn apply(image: &GrayImage, params: &StructureParams) -> GrayImage {
    let amount = params.amount.clamp(-1.0, 1.0);
    if amount == 0.0 {
        return image.clone();
    }
    let gain = amount * if amount > 0.0 { GAIN_UP } else { GAIN_DOWN };
    let perceptual = image.map(|l| linear_to_srgb(l.clamp(0.0, 1.0)));
    let longer_side = image.width().max(image.height()) as f32;

    let mut detail = vec![0.0; perceptual.data().len()];
    for (radius, share) in SCALES {
        let blurred = blur::gaussian(&perceptual, radius * longer_side);
        for ((d, p), b) in detail.iter_mut().zip(perceptual.data()).zip(blurred.data()) {
            *d += share * (p - b);
        }
    }

    let data = perceptual
        .data()
        .par_iter()
        .zip(&detail)
        .map(|(&p, &d)| {
            let midtones = 4.0 * p * (1.0 - p);
            srgb_to_linear((p + gain * d * midtones).clamp(0.0, 1.0))
        })
        .collect();
    GrayImage::new(image.width(), image.height(), data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A horizontal step from dark to light grey, `width` pixels wide.
    fn step(width: u32) -> GrayImage {
        let row: Vec<f32> = (0..width).map(|x| if x < width / 2 { 0.1 } else { 0.4 }).collect();
        GrayImage::new(width, 4, row.repeat(4))
    }

    fn structure(image: &GrayImage, amount: f32) -> GrayImage {
        apply(image, &StructureParams { amount })
    }

    /// Difference across the edge, just either side of it.
    fn edge_contrast(image: &GrayImage) -> f32 {
        let half = image.width() as usize / 2;
        image.data()[half] - image.data()[half - 1]
    }

    #[test]
    fn zero_changes_nothing() {
        let image = step(200);
        assert_eq!(structure(&image, 0.0), image);
    }

    #[test]
    fn a_flat_image_has_no_detail_to_change() {
        let image = GrayImage::new(50, 50, vec![0.3; 2500]);
        let out = structure(&image, 1.0);
        assert!(out.data().iter().all(|&v| (v - 0.3).abs() < 1e-5));
    }

    #[test]
    fn positive_structure_strengthens_an_edge() {
        let image = step(400);
        assert!(edge_contrast(&structure(&image, 0.8)) > edge_contrast(&image));
    }

    #[test]
    fn negative_structure_softens_an_edge() {
        let image = step(400);
        assert!(edge_contrast(&structure(&image, -0.8)) < edge_contrast(&image));
    }

    #[test]
    fn pure_black_and_white_are_left_alone() {
        let row: Vec<f32> = (0..200).map(|x| if x < 100 { 0.0 } else { 1.0 }).collect();
        let image = GrayImage::new(200, 2, row.repeat(2));
        assert_eq!(structure(&image, 1.0), image);
    }

    #[test]
    fn looks_the_same_at_any_resolution() {
        // Each pixel of the small image covers two pixels of the large one.
        let (small, large) = (step(1000), step(2000));
        let (small, large) = (structure(&small, 1.0), structure(&large, 1.0));
        for x in 0..1000 {
            let a = small.data()[x];
            let b = (large.data()[2 * x] + large.data()[2 * x + 1]) / 2.0;
            assert!((a - b).abs() < 0.01, "at {x}: {a} vs {b}");
        }
    }
}
