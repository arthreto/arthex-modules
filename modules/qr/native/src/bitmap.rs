// Everything here is pixel arithmetic: sizes well within what these types hold exactly.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

const BYTES_PER_PIXEL: usize = 4;

/// An image in memory: blue, green, red and alpha, from the top row down, as Windows draws them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Bitmap {
    /// The screen often says nothing useful in alpha: it is ignored.
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

    /// A PNG file of the image, which the interface shows and reads.
    pub fn png(&self) -> Result<Vec<u8>, String> {
        let mut file = Vec::new();
        let mut encoder = png::Encoder::new(&mut file, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let rgba: Vec<u8> = self
            .pixels
            .as_chunks::<BYTES_PER_PIXEL>()
            .0
            .iter()
            .flat_map(|&[blue, green, red, alpha]| [red, green, blue, alpha])
            .collect();
        encoder
            .write_header()
            .and_then(|mut writer| writer.write_image_data(&rgba))
            .map_err(|error| error.to_string())?;
        Ok(file)
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

    #[test]
    fn opaque_ignores_alpha() {
        let image = Bitmap::opaque(1, 1, vec![1, 2, 3, 0]);
        assert_eq!(image.pixels, [1, 2, 3, 255]);
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
    fn png_holds_the_colours_in_their_usual_order() {
        let image = Bitmap::opaque(2, 1, vec![1, 2, 3, 255, 4, 5, 6, 255]);
        let file = image.png().unwrap();
        let mut reader = png::Decoder::new(file.as_slice()).read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size()];
        reader.next_frame(&mut pixels).unwrap();
        assert_eq!(pixels, [3, 2, 1, 255, 6, 5, 4, 255]);
    }
}
