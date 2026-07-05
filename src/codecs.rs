use std::path::Path;

use image::{DynamicImage, ImageError};

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
    image::open(path)
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
