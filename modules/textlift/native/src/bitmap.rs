// Everything here is pixel arithmetic: sizes well within what these types hold exactly.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

const BYTES_PER_PIXEL: usize = 4;
const HEADER_MIN: usize = 40;
const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;

/// An opaque image in memory: blue, green, red and alpha, from the top row down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Bitmap {
    /// Screens and the clipboard often say nothing useful in alpha: it is ignored.
    pub fn opaque(width: u32, height: u32, mut pixels: Vec<u8>) -> Self {
        for pixel in pixels.as_chunks_mut::<BYTES_PER_PIXEL>().0 {
            pixel[3] = u8::MAX;
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// From premultiplied pixels, on white: dark text on a transparent image must stay readable.
    pub fn over_white(width: u32, height: u32, mut pixels: Vec<u8>) -> Self {
        for pixel in pixels.as_chunks_mut::<BYTES_PER_PIXEL>().0 {
            let rest = u8::MAX - pixel[3];
            pixel[0] = pixel[0].saturating_add(rest);
            pixel[1] = pixel[1].saturating_add(rest);
            pixel[2] = pixel[2].saturating_add(rest);
            pixel[3] = u8::MAX;
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// The part inside the image of a rectangle, or nothing when none of it is.
    pub fn crop(&self, left: i32, top: i32, right: i32, bottom: i32) -> Option<Self> {
        let (x_from, y_from) = (left.max(0) as u32, top.max(0) as u32);
        let (x_to, y_to) = (
            right.clamp(0, self.width as i32) as u32,
            bottom.clamp(0, self.height as i32) as u32,
        );
        if x_to <= x_from || y_to <= y_from {
            return None;
        }
        let stride = self.width as usize * BYTES_PER_PIXEL;
        let mut pixels = Vec::with_capacity(
            (x_to - x_from) as usize * (y_to - y_from) as usize * BYTES_PER_PIXEL,
        );
        for y in y_from as usize..y_to as usize {
            let row = y * stride;
            pixels.extend_from_slice(
                &self.pixels[row + x_from as usize * BYTES_PER_PIXEL
                    ..row + x_to as usize * BYTES_PER_PIXEL],
            );
        }
        Some(Self {
            width: x_to - x_from,
            height: y_to - y_from,
            pixels,
        })
    }

    /// Enlarged by a whole factor, smoothly: the recognition reads small text better this way.
    pub fn enlarged(&self, factor: u32) -> Self {
        if factor <= 1 {
            return self.clone();
        }
        let (width, height) = (self.width * factor, self.height * factor);
        let mut pixels = Vec::with_capacity(width as usize * height as usize * BYTES_PER_PIXEL);
        let sample = |x: u32, y: u32, channel: usize| {
            let index = (y as usize * self.width as usize + x as usize) * BYTES_PER_PIXEL + channel;
            f32::from(self.pixels[index])
        };
        for y in 0..height {
            let source_y = ((y as f32 + 0.5) / factor as f32 - 0.5).max(0.0);
            let (top, y_part) = (source_y as u32, source_y.fract());
            let bottom = (top + 1).min(self.height - 1);
            for x in 0..width {
                let source_x = ((x as f32 + 0.5) / factor as f32 - 0.5).max(0.0);
                let (left, x_part) = (source_x as u32, source_x.fract());
                let right = (left + 1).min(self.width - 1);
                for channel in 0..BYTES_PER_PIXEL {
                    let upper = sample(left, top, channel) * (1.0 - x_part)
                        + sample(right, top, channel) * x_part;
                    let lower = sample(left, bottom, channel) * (1.0 - x_part)
                        + sample(right, bottom, channel) * x_part;
                    pixels.push((upper * (1.0 - y_part) + lower * y_part).round() as u8);
                }
            }
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// From what the clipboard holds as `CF_DIB`: a header, then rows of 24 or 32 bits.
    pub fn from_dib(bytes: &[u8]) -> Option<Self> {
        let read_u32 = |at: usize| {
            bytes
                .get(at..at + 4)
                .and_then(|part| part.try_into().ok())
                .map(u32::from_le_bytes)
        };
        let header = read_u32(0)? as usize;
        if header < HEADER_MIN {
            return None;
        }
        let width = read_u32(4)? as i32;
        let height = read_u32(8)? as i32;
        let bits = u16::from_le_bytes(bytes.get(14..16)?.try_into().ok()?);
        let compression = read_u32(16)?;
        if width <= 0 || height == 0 || !matches!(bits, 24 | 32) {
            return None;
        }
        // The three masks of this form follow a short header, and are part of a longer one.
        let masks = match compression {
            BI_BITFIELDS if header == HEADER_MIN => 12,
            BI_RGB | BI_BITFIELDS => 0,
            _ => return None,
        };
        let (rows, top_down) = (height.unsigned_abs() as usize, height < 0);
        let columns = width as usize;
        let depth = usize::from(bits) / 8;
        let stride = (columns * depth).div_ceil(4) * 4;
        let content = bytes.get(header + masks..)?;
        if content.len() < stride.checked_mul(rows)? {
            return None;
        }
        let mut pixels = Vec::with_capacity(columns * rows * BYTES_PER_PIXEL);
        for row in 0..rows {
            let source = if top_down { row } else { rows - 1 - row };
            let line = &content[source * stride..source * stride + columns * depth];
            for pixel in line.chunks_exact(depth) {
                pixels.extend_from_slice(&[pixel[0], pixel[1], pixel[2], u8::MAX]);
            }
        }
        Some(Self {
            width: width as u32,
            height: rows as u32,
            pixels,
        })
    }
}

/// How much to enlarge an image before reading it: Windows misses text in very small images.
pub const fn reading_scale(width: u32, height: u32) -> u32 {
    let shortest = if width < height { width } else { height };
    if shortest >= 400 {
        1
    } else if shortest >= 100 {
        2
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(width: u32, height: u32) -> Bitmap {
        let pixels = (0..width * height)
            .flat_map(|index| [index as u8, 0, 0, u8::MAX])
            .collect();
        Bitmap {
            width,
            height,
            pixels,
        }
    }

    fn dib(height: i32, bits: u16, compression: u32, content: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0; HEADER_MIN];
        bytes[0..4].copy_from_slice(&(HEADER_MIN as u32).to_le_bytes());
        bytes[4..8].copy_from_slice(&1_i32.to_le_bytes());
        bytes[8..12].copy_from_slice(&height.to_le_bytes());
        bytes[14..16].copy_from_slice(&bits.to_le_bytes());
        bytes[16..20].copy_from_slice(&compression.to_le_bytes());
        bytes.extend_from_slice(content);
        bytes
    }

    #[test]
    fn opaque_ignores_alpha() {
        let image = Bitmap::opaque(1, 1, vec![1, 2, 3, 0]);
        assert_eq!(image.pixels, [1, 2, 3, 255]);
    }

    #[test]
    fn transparency_turns_white() {
        let image = Bitmap::over_white(2, 1, vec![0, 0, 0, 0, 10, 20, 30, 128]);
        assert_eq!(image.pixels, [255, 255, 255, 255, 137, 147, 157, 255]);
    }

    #[test]
    fn crop_keeps_the_rows_asked_for() {
        let cropped = square(3, 3).crop(1, 1, 3, 2);
        let cropped = cropped
            .as_ref()
            .map(|image| (image.width, image.height, image.pixels[0]));
        assert_eq!(cropped, Some((2, 1, 4)));
    }

    #[test]
    fn crop_stops_at_the_edges_or_finds_nothing() {
        let image = square(3, 3);
        assert_eq!(image.crop(-5, -5, 1, 1).map(|part| part.width), Some(1));
        assert_eq!(image.crop(5, 5, 9, 9), None);
    }

    #[test]
    fn enlarging_keeps_a_flat_image_flat() {
        let flat = Bitmap::opaque(2, 2, [9, 8, 7, 255].repeat(4));
        let large = flat.enlarged(3);
        assert_eq!((large.width, large.height), (6, 6));
        assert_eq!(large.pixels, [9, 8, 7, 255].repeat(36));
    }

    #[test]
    fn reading_scale_grows_for_small_images() {
        assert_eq!(reading_scale(1920, 1080), 1);
        assert_eq!(reading_scale(500, 120), 2);
        assert_eq!(reading_scale(500, 30), 4);
    }

    #[test]
    fn dib_rows_go_from_the_bottom_up_by_default() {
        let image = Bitmap::from_dib(&dib(2, 32, BI_RGB, &[1, 1, 1, 0, 2, 2, 2, 0]));
        assert_eq!(
            image.map(|image| image.pixels),
            Some(vec![2, 2, 2, 255, 1, 1, 1, 255])
        );
    }

    #[test]
    fn dib_can_go_from_the_top_down() {
        let image = Bitmap::from_dib(&dib(-2, 32, BI_RGB, &[1, 1, 1, 0, 2, 2, 2, 0]));
        assert_eq!(
            image.map(|image| image.pixels),
            Some(vec![1, 1, 1, 255, 2, 2, 2, 255])
        );
    }

    #[test]
    fn dib_rows_of_24_bits_are_padded() {
        let image = Bitmap::from_dib(&dib(1, 24, BI_RGB, &[3, 2, 1, 0]));
        assert_eq!(image.map(|image| image.pixels), Some(vec![3, 2, 1, 255]));
    }

    #[test]
    fn dib_masks_are_skipped() {
        let content = [[0; 12].as_slice(), &[4, 5, 6, 0]].concat();
        let image = Bitmap::from_dib(&dib(1, 32, BI_BITFIELDS, &content));
        assert_eq!(image.map(|image| image.pixels), Some(vec![4, 5, 6, 255]));
    }

    #[test]
    fn dib_refuses_what_it_cannot_read() {
        assert_eq!(Bitmap::from_dib(&[1, 2, 3]), None);
        assert_eq!(Bitmap::from_dib(&dib(1, 8, BI_RGB, &[0; 4])), None);
        assert_eq!(Bitmap::from_dib(&dib(4, 32, BI_RGB, &[0; 4])), None);
    }
}
