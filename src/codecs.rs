use std::path::{Path, PathBuf};

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageError, ImageReader};

pub const READ_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff"];

pub const WRITE_FORMATS: &[(&str, &[&str])] = &[
    ("PNG", &["png"]),
    ("JPEG", &["jpg", "jpeg"]),
    ("BMP", &["bmp"]),
    ("TIFF", &["tiff", "tif"]),
    ("WebP", &["webp"]),
];

pub fn supports(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            READ_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

pub fn open(path: &Path) -> Result<DynamicImage, ImageError> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(image)
}

pub fn save(image: &DynamicImage, path: &Path) -> Result<(), ImageError> {
    image.save(path)
}

/// Agrega una extensión predeterminada cuando el usuario escribió sólo el
/// nombre del archivo. Una extensión vacía (`foto.`) se trata del mismo modo.
pub fn with_default_extension(mut path: PathBuf, extension: &str) -> PathBuf {
    let has_extension = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| !value.is_empty());
    if !has_extension {
        path.set_extension(extension);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_detection_is_case_insensitive() {
        assert!(supports(Path::new("photo.JPEG")));
        assert!(supports(Path::new("photo.webp")));
        assert!(!supports(Path::new("notes.txt")));
    }

    #[test]
    fn adds_default_extension_only_when_missing() {
        assert_eq!(
            with_default_extension(PathBuf::from("foto"), "png"),
            PathBuf::from("foto.png")
        );
        assert_eq!(
            with_default_extension(PathBuf::from("foto."), "png"),
            PathBuf::from("foto.png")
        );
        assert_eq!(
            with_default_extension(PathBuf::from("foto.jpg"), "png"),
            PathBuf::from("foto.jpg")
        );
    }
}
