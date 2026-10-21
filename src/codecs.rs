use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_detection_is_case_insensitive() {
        assert!(supports(Path::new("photo.JPEG")));
        assert!(supports(Path::new("photo.webp")));
        assert!(!supports(Path::new("notes.txt")));
    }
}
