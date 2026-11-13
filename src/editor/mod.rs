use cosmic::iced::widget::image::Handle;
use cosmic::iced::Point;
use image::DynamicImage;

use crate::state::EditOperation;

pub mod selection;
pub mod update;

use selection::SelectionMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAlgorithm {
    Nearest,
    Bilinear,
    Bicubic,
    Gaussian,
    Lanczos,
}

impl ResizeAlgorithm {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Nearest,
            1 => Self::Bilinear,
            2 => Self::Bicubic,
            3 => Self::Gaussian,
            _ => Self::Lanczos,
        }
    }

    fn filter(self) -> image::imageops::FilterType {
        match self {
            Self::Nearest => image::imageops::FilterType::Nearest,
            Self::Bilinear => image::imageops::FilterType::Triangle,
            Self::Bicubic => image::imageops::FilterType::CatmullRom,
            Self::Gaussian => image::imageops::FilterType::Gaussian,
            Self::Lanczos => image::imageops::FilterType::Lanczos3,
        }
    }
}

pub fn to_handle(image: &DynamicImage) -> Handle {
    let rgba = image.to_rgba8();
    Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw())
}

pub fn rotate_clockwise(image: &DynamicImage) -> DynamicImage {
    image.rotate90()
}

pub fn rotate_counterclockwise(image: &DynamicImage) -> DynamicImage {
    image.rotate270()
}

pub fn flip_horizontal(image: &DynamicImage) -> DynamicImage {
    image.fliph()
}

pub fn flip_vertical(image: &DynamicImage) -> DynamicImage {
    image.flipv()
}

pub(crate) fn crop_rect(
    first: Point,
    second: Point,
    image_width: u32,
    image_height: u32,
) -> Option<(u32, u32, u32, u32)> {
    let x0 = first.x.min(second.x).floor().clamp(0.0, image_width as f32) as u32;
    let y0 = first
        .y
        .min(second.y)
        .floor()
        .clamp(0.0, image_height as f32) as u32;
    let x1 = first.x.max(second.x).ceil().clamp(0.0, image_width as f32) as u32;
    let y1 = first.y.max(second.y).ceil().clamp(0.0, image_height as f32) as u32;
    let width = x1.saturating_sub(x0);
    let height = y1.saturating_sub(y0);
    (width > 0 && height > 0).then_some((x0, y0, width, height))
}

pub fn crop(image: &DynamicImage, first: Point, second: Point) -> Option<DynamicImage> {
    let (x, y, width, height) = crop_rect(first, second, image.width(), image.height())?;
    Some(image.crop_imm(x, y, width, height))
}

/// Combina varias regiones conservando su posición relativa. Las zonas no
/// seleccionadas quedan transparentes dentro del rectángulo envolvente.
pub fn crop_multiple(image: &DynamicImage, regions: &[(Point, Point)]) -> Option<DynamicImage> {
    let rects: Vec<_> = regions
        .iter()
        .filter_map(|(first, second)| crop_rect(*first, *second, image.width(), image.height()))
        .collect();
    let min_x = rects.iter().map(|rect| rect.0).min()?;
    let min_y = rects.iter().map(|rect| rect.1).min()?;
    let max_x = rects.iter().map(|rect| rect.0 + rect.2).max()?;
    let max_y = rects.iter().map(|rect| rect.1 + rect.3).max()?;
    let mut output = image::RgbaImage::new(max_x - min_x, max_y - min_y);
    for (x, y, width, height) in rects {
        let fragment = image.crop_imm(x, y, width, height).to_rgba8();
        image::imageops::overlay(
            &mut output,
            &fragment,
            i64::from(x - min_x),
            i64::from(y - min_y),
        );
    }
    Some(DynamicImage::ImageRgba8(output))
}

pub fn resize(image: &DynamicImage, factor: f32, algorithm: ResizeAlgorithm) -> DynamicImage {
    let width = ((image.width() as f32 * factor).round() as u32).max(1);
    let height = ((image.height() as f32 * factor).round() as u32).max(1);
    image.resize_exact(width, height, algorithm.filter())
}

/// Makes only the color region connected to `seed` transparent.
pub fn remove_background(
    image: &DynamicImage,
    seed: (u32, u32),
    tolerance: f32,
    grow: u32,
    feather: u32,
) -> DynamicImage {
    let mut output = image.to_rgba8();
    if let Some(mask) = SelectionMask::magic_wand(&output, seed, tolerance, grow, feather) {
        mask.delete_from(&mut output);
    }
    DynamicImage::ImageRgba8(output)
}

pub fn replay(mut image: DynamicImage, operations: &[EditOperation]) -> DynamicImage {
    for operation in operations {
        image = match operation {
            EditOperation::Crop(first, second) => crop(&image, *first, *second).unwrap_or(image),
            EditOperation::MultiCrop(regions) => crop_multiple(&image, regions).unwrap_or(image),
            EditOperation::RotateClockwise => rotate_clockwise(&image),
            EditOperation::RotateCounterclockwise => rotate_counterclockwise(&image),
            EditOperation::FlipHorizontal => flip_horizontal(&image),
            EditOperation::FlipVertical => flip_vertical(&image),
            EditOperation::Resize { factor, filter } => {
                resize(&image, *factor, ResizeAlgorithm::from_index(*filter))
            }
            EditOperation::RemoveBackground {
                seed,
                tolerance,
                grow,
                feather,
            } => remove_background(&image, *seed, *tolerance, *grow, *feather),
        };
    }
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_normalizes_reversed_points() {
        let image = DynamicImage::new_rgba8(100, 80);
        let cropped = crop(&image, Point::new(70.0, 60.0), Point::new(10.0, 20.0)).unwrap();
        assert_eq!((cropped.width(), cropped.height()), (60, 40));
    }

    #[test]
    fn crop_clamps_to_image_bounds() {
        let image = DynamicImage::new_rgba8(100, 80);
        let cropped = crop(&image, Point::new(-20.0, -10.0), Point::new(150.0, 100.0)).unwrap();
        assert_eq!((cropped.width(), cropped.height()), (100, 80));
    }

    #[test]
    fn multiple_crop_preserves_relative_positions_and_transparency() {
        let mut source = image::RgbaImage::from_pixel(6, 4, image::Rgba([20, 30, 40, 255]));
        source.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        source.put_pixel(5, 3, image::Rgba([0, 0, 255, 255]));
        let image = DynamicImage::ImageRgba8(source);
        let result = crop_multiple(
            &image,
            &[
                (Point::new(0.0, 0.0), Point::new(2.0, 2.0)),
                (Point::new(4.0, 2.0), Point::new(6.0, 4.0)),
            ],
        )
        .unwrap()
        .to_rgba8();

        assert_eq!(result.dimensions(), (6, 4));
        assert_eq!(result.get_pixel(0, 0), &image::Rgba([255, 0, 0, 255]));
        assert_eq!(result.get_pixel(5, 3), &image::Rgba([0, 0, 255, 255]));
        assert_eq!(result.get_pixel(3, 1).0[3], 0);
    }

    #[test]
    fn resize_never_produces_zero_sized_images() {
        let image = DynamicImage::new_rgba8(1, 1);
        let resized = resize(&image, 0.01, ResizeAlgorithm::Nearest);
        assert_eq!((resized.width(), resized.height()), (1, 1));
    }

    #[test]
    fn rotations_swap_image_dimensions() {
        let image = DynamicImage::new_rgba8(12, 7);
        let clockwise = rotate_clockwise(&image);
        let counterclockwise = rotate_counterclockwise(&image);
        assert_eq!((clockwise.width(), clockwise.height()), (7, 12));
        assert_eq!(
            (counterclockwise.width(), counterclockwise.height()),
            (7, 12)
        );
    }

    #[test]
    fn flips_preserve_image_dimensions() {
        let image = DynamicImage::new_rgba8(12, 7);
        let horizontal = flip_horizontal(&image);
        let vertical = flip_vertical(&image);
        assert_eq!((horizontal.width(), horizontal.height()), (12, 7));
        assert_eq!((vertical.width(), vertical.height()), (12, 7));
    }

    #[test]
    fn replay_applies_multiple_operations_in_order() {
        let image = DynamicImage::new_rgba8(100, 80);
        let operations = vec![
            EditOperation::Crop(Point::new(10.0, 10.0), Point::new(70.0, 50.0)),
            EditOperation::RotateClockwise,
            EditOperation::FlipHorizontal,
        ];

        let result = replay(image, &operations);
        assert_eq!((result.width(), result.height()), (40, 60));
    }

    #[test]
    fn remove_background_stops_at_a_color_boundary() {
        let mut image = image::RgbaImage::from_pixel(5, 1, image::Rgba([255, 255, 255, 255]));
        image.put_pixel(2, 0, image::Rgba([0, 0, 0, 255]));
        let result =
            remove_background(&DynamicImage::ImageRgba8(image), (0, 0), 0.0, 0, 0).to_rgba8();

        assert_eq!(result.get_pixel(0, 0).0[3], 0);
        assert_eq!(result.get_pixel(4, 0).0[3], 255);
    }
}
