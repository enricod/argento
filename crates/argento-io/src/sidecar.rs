//! Sidecar files: the recipe of a photo, saved as TOML next to it
//! (`photo.jpg` → `photo.jpg.argento`). The photo itself is never modified.
//!
//! Missing fields take their default value, so a sidecar written by an older
//! version still opens; one written by a newer version is refused rather
//! than half understood.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use argento_core::recipe::Recipe;
use serde::{Deserialize, Serialize};

/// Version of the file format written.
pub const VERSION: u32 = 1;

const EXTENSION: &str = "argento";

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("cannot access the sidecar file: {0}")]
    Io(#[from] io::Error),
    #[error("the sidecar file is damaged: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("cannot write the sidecar file: {0}")]
    Format(#[from] toml::ser::Error),
    #[error("the sidecar file was written by a newer version of Argento (format {0})")]
    TooNew(u32),
}

#[derive(Serialize, Deserialize)]
struct File {
    version: u32,
    #[serde(flatten)]
    recipe: Recipe,
}

/// Where the sidecar of `photo` lives.
pub fn path_for(photo: &Path) -> PathBuf {
    let mut name = photo.as_os_str().to_owned();
    name.push(".");
    name.push(EXTENSION);
    PathBuf::from(name)
}

/// The recipe saved for `photo`, or `None` if it has no sidecar.
pub fn read(photo: &Path) -> Result<Option<Recipe>, SidecarError> {
    let text = match fs::read_to_string(path_for(photo)) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let file: File = toml::from_str(&text)?;
    if file.version > VERSION {
        return Err(SidecarError::TooNew(file.version));
    }
    Ok(Some(file.recipe))
}

/// Saves `recipe` as the sidecar of `photo`.
///
/// The file is written under a temporary name, then renamed: a crash never
/// leaves a truncated sidecar behind.
pub fn write(photo: &Path, recipe: &Recipe) -> Result<(), SidecarError> {
    let text = toml::to_string(&File {
        version: VERSION,
        recipe: *recipe,
    })?;
    let path = path_for(photo);
    let mut temporary = path.clone().into_os_string();
    temporary.push(".tmp");

    let mut file = fs::File::create(&temporary)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argento_core::ops::filter::Filter;

    fn edited() -> Recipe {
        let mut recipe = Recipe::default();
        recipe.filter.filter = Filter::Orange;
        recipe.filter.strength = 0.7;
        recipe.global.tone.brightness = 0.1;
        recipe.global.tone.contrast = -0.25;
        recipe.global.structure.amount = 0.5;
        recipe
    }

    #[test]
    fn the_sidecar_sits_next_to_the_photo() {
        assert_eq!(
            path_for(Path::new("/photos/a.jpg")),
            Path::new("/photos/a.jpg.argento")
        );
    }

    #[test]
    fn a_photo_without_sidecar_has_no_recipe() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(&dir.path().join("a.jpg")).unwrap().is_none());
    }

    #[test]
    fn a_written_recipe_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("a.jpg");
        write(&photo, &edited()).unwrap();
        assert_eq!(read(&photo).unwrap(), Some(edited()));
        // No temporary file is left behind.
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn a_film_reads_back_with_its_stock_and_grain() {
        use argento_core::ops::film::{FilmParams, Stock};
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("a.jpg");
        let mut recipe = edited();
        recipe.film = Some(FilmParams {
            stock: Stock::HighContrast,
            grain: 0.4,
            grain_size: 0.002,
        });
        write(&photo, &recipe).unwrap();
        assert_eq!(read(&photo).unwrap(), Some(recipe));
    }

    #[test]
    fn the_file_is_readable_toml() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("a.jpg");
        write(&photo, &edited()).unwrap();
        let text = fs::read_to_string(path_for(&photo)).unwrap();
        assert!(text.contains("version = 1"), "{text}");
        assert!(text.contains(r#"filter = "orange""#), "{text}");
        assert!(text.contains("strength = 0.7\n"), "{text}");
    }

    #[test]
    fn missing_fields_take_their_default() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("a.jpg");
        fs::write(
            path_for(&photo),
            "version = 1\n[global.tone]\ncontrast = 0.5\n",
        )
        .unwrap();
        let mut expected = Recipe::default();
        expected.global.tone.contrast = 0.5;
        assert_eq!(read(&photo).unwrap(), Some(expected));
    }

    #[test]
    fn a_newer_format_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("a.jpg");
        fs::write(path_for(&photo), "version = 99\n").unwrap();
        assert!(matches!(read(&photo), Err(SidecarError::TooNew(99))));
    }

    #[test]
    fn a_damaged_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("a.jpg");
        fs::write(path_for(&photo), "version = \n").unwrap();
        assert!(matches!(read(&photo), Err(SidecarError::Parse(_))));
    }
}
