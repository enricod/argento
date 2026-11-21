//! Loading JPEG, PNG and TIFF files into the linear working space.
//!
//! Files are assumed to be sRGB: embedded ICC profiles are not read yet.

use std::path::Path;

use argento_core::image::RgbImage;
use image::{DynamicImage, ImageDecoder, ImageReader};

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("cannot read the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Decode(#[from] image::ImageError),
}

/// Loads a JPEG, PNG or TIFF file (8 or 16 bits per channel, colour or
/// greyscale), turned upright according to its EXIF orientation.
///
/// The format is recognised from the file's content, not its extension.
pub fn load(path: &Path) -> Result<RgbImage, LoadError> {
    let mut decoder = ImageReader::open(path)?
        .with_guessed_format()?
        .into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);

    let rgb = image.into_rgb32f();
    Ok(RgbImage::from_srgb(
        rgb.width(),
        rgb.height(),
        rgb.into_raw(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use argento_core::image::srgb_to_linear;
    use image::{ImageBuffer, ImageFormat, Luma, Rgb};

    fn dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn loads_an_8_bit_png_into_linear_light() {
        let dir = dir();
        let path = dir.path().join("photo.png");
        ImageBuffer::from_raw(2, 1, vec![0u8, 128, 255, 10, 20, 30])
            .map(|b: ImageBuffer<Rgb<u8>, _>| b.save(&path).unwrap())
            .unwrap();

        let image = load(&path).unwrap();

        assert_eq!((image.width(), image.height()), (2, 1));
        let expected = [0u8, 128, 255, 10, 20, 30].map(|v| srgb_to_linear(v as f32 / 255.0));
        for (got, want) in image.data().iter().zip(expected) {
            assert!((got - want).abs() < 1e-6);
        }
        assert_eq!(image.to_srgb8(), [0, 128, 255, 10, 20, 30]);
    }

    #[test]
    fn keeps_the_precision_of_a_16_bit_tiff() {
        let dir = dir();
        let path = dir.path().join("scan.tif");
        let raw = vec![1000u16, 1001, 1002];
        ImageBuffer::<Rgb<u16>, _>::from_raw(1, 1, raw.clone())
            .unwrap()
            .save_with_format(&path, ImageFormat::Tiff)
            .unwrap();

        let image = load(&path).unwrap();

        for (got, v) in image.data().iter().zip(raw) {
            let want = srgb_to_linear(v as f32 / 65535.0);
            assert!((got - want).abs() < 1e-7, "{got} != {want}");
        }
        assert_ne!(image.data()[0], image.data()[1]);
    }

    #[test]
    fn expands_greyscale_to_rgb() {
        let dir = dir();
        let path = dir.path().join("grey.png");
        ImageBuffer::<Luma<u8>, _>::from_raw(1, 1, vec![200u8])
            .unwrap()
            .save(&path)
            .unwrap();

        assert_eq!(load(&path).unwrap().to_srgb8(), [200, 200, 200]);
    }

    #[test]
    fn loads_a_jpeg() {
        let dir = dir();
        let path = dir.path().join("photo.jpg");
        ImageBuffer::<Rgb<u8>, _>::from_pixel(8, 4, Rgb([90, 120, 150]))
            .save(&path)
            .unwrap();

        let image = load(&path).unwrap();

        assert_eq!((image.width(), image.height()), (8, 4));
    }

    #[test]
    fn recognises_the_format_by_content_not_extension() {
        let dir = dir();
        let png = dir.path().join("photo.png");
        ImageBuffer::<Rgb<u8>, _>::from_pixel(1, 1, Rgb([1, 2, 3]))
            .save(&png)
            .unwrap();
        let misnamed = dir.path().join("photo.jpg");
        std::fs::rename(&png, &misnamed).unwrap();

        assert_eq!(load(&misnamed).unwrap().to_srgb8(), [1, 2, 3]);
    }

    #[test]
    fn rejects_a_file_that_is_not_an_image() {
        let dir = dir();
        let path = dir.path().join("notes.jpg");
        std::fs::write(&path, "not a photo").unwrap();

        assert!(matches!(load(&path), Err(LoadError::Decode(_))));
    }

    #[test]
    fn reports_a_missing_file() {
        let dir = dir();
        assert!(matches!(
            load(&dir.path().join("gone.png")),
            Err(LoadError::Io(_))
        ));
    }
}
