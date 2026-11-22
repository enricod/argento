//! Writing the finished photo to a file.
//!
//! The format follows the file's extension: JPEG (8 bits), or TIFF and PNG
//! at 16 bits per channel, which keep smooth gradients for further editing
//! or printing. Files are written as sRGB, without metadata for now.

use std::fs;
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};

use argento_core::image::RgbImage;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::tiff::TiffEncoder;
use image::{ExtendedColorType, ImageEncoder};

/// Quality of exported JPEG files, from 1 to 100.
pub const JPEG_QUALITY: u8 = 92;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Jpeg,
    Tiff,
    Png,
}

impl Format {
    /// The format named by the extension of `path`, if Argento writes it.
    pub fn from_path(path: &Path) -> Option<Format> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "jpg" | "jpeg" => Some(Format::Jpeg),
            "tif" | "tiff" => Some(Format::Tiff),
            "png" => Some(Format::Png),
            _ => None,
        }
    }
}

/// Where `photo` is exported by default: a 16-bit TIFF in the same folder,
/// named after it (`photo.jpg` → `photo-bw.tif`). The name always differs
/// from the photo's, so the original is never overwritten.
pub fn beside(photo: &Path) -> PathBuf {
    let mut name = photo.file_stem().unwrap_or_default().to_owned();
    name.push("-bw.tif");
    photo.with_file_name(name)
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("unsupported file type: use .jpg, .tif or .png")]
    UnknownFormat,
    #[error("cannot write the file: {0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Encode(#[from] image::ImageError),
}

/// Writes `image` to `path`, in the format given by its extension.
///
/// The file is written under a temporary name, then renamed: an existing
/// file is never left half overwritten.
pub fn export(image: &RgbImage, path: &Path) -> Result<(), ExportError> {
    let format = Format::from_path(path).ok_or(ExportError::UnknownFormat)?;
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".part");

    let result = write(image, format, Path::new(&temporary));
    match result {
        Ok(()) => Ok(fs::rename(&temporary, path)?),
        Err(e) => {
            let _ = fs::remove_file(&temporary);
            Err(e)
        }
    }
}

fn write(image: &RgbImage, format: Format, path: &Path) -> Result<(), ExportError> {
    let (width, height) = (image.width(), image.height());
    let mut file = BufWriter::new(fs::File::create(path)?);
    match format {
        Format::Jpeg => {
            let pixels = image.to_srgb8();
            JpegEncoder::new_with_quality(&mut file, JPEG_QUALITY).write_image(
                &pixels,
                width,
                height,
                ExtendedColorType::Rgb8,
            )?;
        }
        Format::Tiff | Format::Png => {
            let bytes: Vec<u8> = image
                .to_srgb16()
                .iter()
                .flat_map(|v| v.to_ne_bytes())
                .collect();
            if format == Format::Tiff {
                TiffEncoder::new(&mut file).write_image(
                    &bytes,
                    width,
                    height,
                    ExtendedColorType::Rgb16,
                )?;
            } else {
                PngEncoder::new(&mut file).write_image(
                    &bytes,
                    width,
                    height,
                    ExtendedColorType::Rgb16,
                )?;
            }
        }
    }
    file.into_inner().map_err(|e| e.into_error())?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load;

    /// A horizontal ramp from black to white.
    fn ramp() -> RgbImage {
        let (w, h) = (64, 8);
        let data = (0..h)
            .flat_map(|_| (0..w).flat_map(|x| [x as f32 / (w - 1) as f32; 3]))
            .collect();
        RgbImage::from_srgb(w, h, data)
    }

    #[test]
    fn the_extension_chooses_the_format() {
        assert_eq!(Format::from_path(Path::new("a.JPG")), Some(Format::Jpeg));
        assert_eq!(Format::from_path(Path::new("a.jpeg")), Some(Format::Jpeg));
        assert_eq!(Format::from_path(Path::new("a.tif")), Some(Format::Tiff));
        assert_eq!(Format::from_path(Path::new("a.png")), Some(Format::Png));
        assert_eq!(Format::from_path(Path::new("a.webp")), None);
        assert_eq!(Format::from_path(Path::new("a")), None);
    }

    #[test]
    fn the_default_export_is_a_tiff_beside_the_photo() {
        assert_eq!(beside(Path::new("/p/a.jpg")), Path::new("/p/a-bw.tif"));
        assert_eq!(beside(Path::new("/p/a.b.png")), Path::new("/p/a.b-bw.tif"));
        // A TIFF original keeps its own name.
        assert_eq!(beside(Path::new("/p/a.tif")), Path::new("/p/a-bw.tif"));
        assert_eq!(
            beside(Path::new("/p/a-bw.tif")),
            Path::new("/p/a-bw-bw.tif")
        );
    }

    #[test]
    fn an_unknown_extension_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.webp");
        assert!(matches!(
            export(&ramp(), &path),
            Err(ExportError::UnknownFormat)
        ));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn lossless_formats_round_trip_exactly() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a.tif", "a.png"] {
            let path = dir.path().join(name);
            export(&ramp(), &path).unwrap();
            let back = load(&path).unwrap();
            assert_eq!(back.to_srgb16(), ramp().to_srgb16(), "{name}");
        }
        // No temporary files are left behind.
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn jpeg_round_trips_closely() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jpg");
        export(&ramp(), &path).unwrap();
        let back = load(&path).unwrap();
        assert_eq!((back.width(), back.height()), (64, 8));
        for (a, b) in back.to_srgb8().iter().zip(ramp().to_srgb8()) {
            assert!(a.abs_diff(b) <= 4, "{a} vs {b}");
        }
    }

    #[test]
    fn an_existing_file_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.png");
        fs::write(&path, b"old").unwrap();
        export(&ramp(), &path).unwrap();
        assert!(load(&path).is_ok());
    }
}
