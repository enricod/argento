//! Gaussian blur of a grey image, approximated by three box blurs.

use rayon::prelude::*;

use crate::image::GrayImage;

/// Blurs with a Gaussian of standard deviation `sigma` pixels. Edges are
/// extended by repeating the border pixels. A `sigma` too small to matter
/// returns the image unchanged.
pub fn gaussian(image: &GrayImage, sigma: f32) -> GrayImage {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let radii = box_radii(sigma);
    if radii.iter().all(|&r| r == 0) {
        return image.clone();
    }

    let mut data = image.data().to_vec();
    blur_rows(&mut data, w, &radii);
    let mut columns = transpose(&data, w, h);
    blur_rows(&mut columns, h, &radii);
    GrayImage::new(image.width(), image.height(), transpose(&columns, h, w))
}

/// Radii of the three box blurs whose succession best matches a Gaussian
/// of standard deviation `sigma` (Kovesi's method).
fn box_radii(sigma: f32) -> [usize; 3] {
    if sigma.is_nan() || sigma <= 0.0 {
        return [0; 3];
    }
    let n = 3.0;
    let ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut lower = ideal.floor() as i64;
    if lower % 2 == 0 {
        lower -= 1;
    }
    let lower = lower.max(1);
    let upper = lower + 2;
    let (l, s2) = (lower as f32, sigma * sigma);
    let use_lower =
        ((12.0 * s2 - n * l * l - 4.0 * n * l - 3.0 * n) / (-4.0 * l - 4.0)).round() as i64;
    std::array::from_fn(|i| {
        let width = if (i as i64) < use_lower { lower } else { upper };
        ((width - 1) / 2) as usize
    })
}

fn blur_rows(data: &mut [f32], width: usize, radii: &[usize; 3]) {
    data.par_chunks_mut(width).for_each(|row| {
        let mut scratch = vec![0.0; width];
        for &r in radii {
            box_blur(row, &mut scratch, r);
            row.copy_from_slice(&scratch);
        }
    });
}

/// Mean over `[i - r, i + r]`, clamping indices to the row.
fn box_blur(src: &[f32], dst: &mut [f32], r: usize) {
    if r == 0 {
        dst.copy_from_slice(src);
        return;
    }
    let last = src.len() as isize - 1;
    let at = |i: isize| src[i.clamp(0, last) as usize];
    let r = r as isize;
    let mut sum: f32 = (-r..=r).map(at).sum();
    let scale = 1.0 / (2 * r + 1) as f32;
    for (i, out) in dst.iter_mut().enumerate() {
        let i = i as isize;
        *out = sum * scale;
        sum += at(i + r + 1) - at(i - r);
    }
}

fn transpose(data: &[f32], width: usize, height: usize) -> Vec<f32> {
    let mut out = vec![0.0; data.len()];
    out.par_chunks_mut(height)
        .enumerate()
        .for_each(|(x, column)| {
            for (y, v) in column.iter_mut().enumerate() {
                *v = data[y * width + x];
            }
        });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impulse(size: u32) -> GrayImage {
        let mut data = vec![0.0; (size * size) as usize];
        let centre = (size / 2) as usize;
        data[centre * size as usize + centre] = 1.0;
        GrayImage::new(size, size, data)
    }

    #[test]
    fn a_flat_image_stays_flat() {
        let image = GrayImage::new(30, 20, vec![0.4; 600]);
        let blurred = gaussian(&image, 5.0);
        assert!(blurred.data().iter().all(|&v| (v - 0.4).abs() < 1e-5));
    }

    #[test]
    fn zero_sigma_changes_nothing() {
        let image = impulse(9);
        assert_eq!(gaussian(&image, 0.0), image);
    }

    #[test]
    fn keeps_the_total_away_from_the_edges() {
        let blurred = gaussian(&impulse(101), 4.0);
        let total: f32 = blurred.data().iter().sum();
        assert!((total - 1.0).abs() < 1e-3, "{total}");
    }

    #[test]
    fn spreads_an_impulse_by_about_sigma() {
        let sigma = 6.0;
        let blurred = gaussian(&impulse(101), sigma);
        let row = &blurred.data()[50 * 101..51 * 101];
        let total: f32 = row.iter().sum();
        let variance: f32 = row
            .iter()
            .enumerate()
            .map(|(x, v)| v * (x as f32 - 50.0).powi(2))
            .sum::<f32>()
            / total;
        assert!((variance.sqrt() - sigma).abs() < 0.6, "{}", variance.sqrt());
    }

    #[test]
    fn blurs_both_directions() {
        let blurred = gaussian(&impulse(21), 2.0);
        let at = |x: usize, y: usize| blurred.data()[y * 21 + x];
        assert!(at(10, 13) > 0.0);
        assert!((at(13, 10) - at(10, 13)).abs() < 1e-6);
    }
}
