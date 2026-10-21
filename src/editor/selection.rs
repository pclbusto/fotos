use std::collections::VecDeque;

use image::{Rgba, RgbaImage};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionMask {
    width: u32,
    height: u32,
    alpha: Vec<u8>,
}

impl SelectionMask {
    /// Selects only the color region connected to `seed`, then expands and
    /// softens its edge. Equal colors separated by another region stay intact.
    pub fn magic_wand(
        image: &RgbaImage,
        seed: (u32, u32),
        tolerance: f32,
        grow: u32,
        feather: u32,
    ) -> Option<Self> {
        let (width, height) = image.dimensions();
        if seed.0 >= width || seed.1 >= height {
            return None;
        }

        let target = image.get_pixel(seed.0, seed.1).0;
        let tolerance_sq = tolerance.max(0.0).powi(2);
        let mut alpha = vec![0; (width * height) as usize];
        let mut visited = vec![false; alpha.len()];
        let mut queue = VecDeque::from([seed]);

        while let Some((x, y)) = queue.pop_front() {
            let index = (y * width + x) as usize;
            if visited[index] {
                continue;
            }
            visited[index] = true;

            let pixel = image.get_pixel(x, y).0;
            let dr = pixel[0] as f32 - target[0] as f32;
            let dg = pixel[1] as f32 - target[1] as f32;
            let db = pixel[2] as f32 - target[2] as f32;
            let same_alpha_kind = (pixel[3] == 0) == (target[3] == 0);
            if !same_alpha_kind || dr * dr + dg * dg + db * db > tolerance_sq {
                continue;
            }

            alpha[index] = u8::MAX;
            if x > 0 {
                queue.push_back((x - 1, y));
            }
            if x + 1 < width {
                queue.push_back((x + 1, y));
            }
            if y > 0 {
                queue.push_back((x, y - 1));
            }
            if y + 1 < height {
                queue.push_back((x, y + 1));
            }
        }

        let mut mask = Self {
            width,
            height,
            alpha,
        };
        mask.grow(grow.min(64));
        mask.feather(feather.min(64));
        Some(mask)
    }

    pub fn selected_pixels(&self) -> usize {
        self.alpha.iter().filter(|&&alpha| alpha != 0).count()
    }

    pub fn overlay(&self) -> RgbaImage {
        let mut output = RgbaImage::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x, y);
                let alpha = self.alpha[index];
                if alpha != 0 {
                    output.put_pixel(
                        x,
                        y,
                        Rgba([64, 140, 255, ((alpha as u16 * 96) / 255) as u8]),
                    );
                }
            }
        }

        for y in 0..self.height {
            for x in 0..self.width {
                if self.is_boundary(x, y) {
                    let value = if ((x + y) / 3) % 2 == 0 { 255 } else { 0 };
                    output.put_pixel(x, y, Rgba([value, value, value, 255]));
                }
            }
        }
        output
    }

    pub fn delete_from(&self, image: &mut RgbaImage) {
        if image.dimensions() != (self.width, self.height) {
            return;
        }
        for (pixel, selected) in image.pixels_mut().zip(&self.alpha) {
            pixel.0[3] = ((pixel.0[3] as u16 * (255 - *selected as u16)) / 255) as u8;
        }
    }

    fn grow(&mut self, radius: u32) {
        if radius != 0 {
            self.alpha = box_max(&self.alpha, self.width, self.height, radius, true);
            self.alpha = box_max(&self.alpha, self.width, self.height, radius, false);
        }
    }

    fn feather(&mut self, radius: u32) {
        if radius != 0 {
            self.alpha = box_blur(&self.alpha, self.width, self.height, radius, true);
            self.alpha = box_blur(&self.alpha, self.width, self.height, radius, false);
        }
    }

    fn is_boundary(&self, x: u32, y: u32) -> bool {
        if self.alpha[self.index(x, y)] == 0 {
            return false;
        }
        x == 0
            || y == 0
            || x + 1 == self.width
            || y + 1 == self.height
            || self.alpha[self.index(x - 1, y)] == 0
            || self.alpha[self.index(x + 1, y)] == 0
            || self.alpha[self.index(x, y - 1)] == 0
            || self.alpha[self.index(x, y + 1)] == 0
    }

    fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }
}

fn box_blur(source: &[u8], width: u32, height: u32, radius: u32, horizontal: bool) -> Vec<u8> {
    let mut output = vec![0; source.len()];
    let line_count = if horizontal { height } else { width };
    let line_len = if horizontal { width } else { height };
    for line in 0..line_count {
        let mut prefix = vec![0_u32; line_len as usize + 1];
        for position in 0..line_len {
            let (x, y) = if horizontal {
                (position, line)
            } else {
                (line, position)
            };
            prefix[position as usize + 1] =
                prefix[position as usize] + source[(y * width + x) as usize] as u32;
        }
        for position in 0..line_len {
            let start = position.saturating_sub(radius);
            let end = (position + radius).min(line_len - 1);
            let sum = prefix[end as usize + 1] - prefix[start as usize];
            let (x, y) = if horizontal {
                (position, line)
            } else {
                (line, position)
            };
            output[(y * width + x) as usize] = (sum / (end - start + 1)) as u8;
        }
    }
    output
}

fn box_max(source: &[u8], width: u32, height: u32, radius: u32, horizontal: bool) -> Vec<u8> {
    let mut output = vec![0; source.len()];
    let line_count = if horizontal { height } else { width };
    let line_len = if horizontal { width } else { height };
    for line in 0..line_count {
        let mut prefix = vec![0_u32; line_len as usize + 1];
        for position in 0..line_len {
            let (x, y) = if horizontal {
                (position, line)
            } else {
                (line, position)
            };
            prefix[position as usize + 1] =
                prefix[position as usize] + u32::from(source[(y * width + x) as usize] != 0);
        }
        for position in 0..line_len {
            let start = position.saturating_sub(radius);
            let end = (position + radius).min(line_len - 1);
            let (x, y) = if horizontal {
                (position, line)
            } else {
                (line, position)
            };
            if prefix[end as usize + 1] != prefix[start as usize] {
                output[(y * width + x) as usize] = u8::MAX;
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wand_selects_only_the_connected_region() {
        let mut image = RgbaImage::from_pixel(5, 1, Rgba([0, 0, 255, 255]));
        image.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        image.put_pixel(1, 0, Rgba([250, 0, 0, 255]));
        image.put_pixel(4, 0, Rgba([255, 0, 0, 255]));

        let mask = SelectionMask::magic_wand(&image, (0, 0), 10.0, 0, 0).unwrap();
        assert_eq!(mask.selected_pixels(), 2);
    }

    #[test]
    fn deleting_a_region_preserves_the_same_color_behind_a_contour() {
        let mut image = RgbaImage::from_pixel(5, 1, Rgba([255, 255, 255, 255]));
        image.put_pixel(2, 0, Rgba([0, 0, 0, 255]));
        let mask = SelectionMask::magic_wand(&image, (0, 0), 0.0, 0, 0).unwrap();
        mask.delete_from(&mut image);

        assert_eq!(image.get_pixel(0, 0).0[3], 0);
        assert_eq!(image.get_pixel(4, 0).0[3], 255);
    }
}
