//! The pictures that come with the module, converted to the format and the size asked for.

use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use image::{DynamicImage, ImageFormat, RgbImage, codecs::jpeg::JpegEncoder, imageops::FilterType};

/// The pictures of `bundle/pictures/`, all in the public domain.
pub const PICTURES: [&str; 8] = [
    "landscape",
    "portrait",
    "city",
    "animal",
    "food",
    "object",
    "alpha",
    "text",
];

/// The formats an image can be created in.
pub const FORMATS: [&str; 7] = ["png", "jpg", "webp", "gif", "bmp", "ico", "tiff"];

/// An `.ico` holds images of 256 pixels at most.
const ICON_SIDE: u32 = 256;

pub fn folder() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|program| program.parent().map(Path::to_path_buf))
        .unwrap_or_default()
        .join("pictures")
}

pub fn load(folder: &Path, id: &str) -> Result<DynamicImage, String> {
    if !PICTURES.contains(&id) {
        return Err(format!("unknown picture {id}"));
    }
    image::open(folder.join(format!("{id}.webp"))).map_err(|error| error.to_string())
}

/// What is under a transparent picture once it is saved without transparency.
fn on_white(image: &DynamicImage) -> RgbImage {
    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let blend = |channel: u8| {
            let mixed = (u16::from(channel) * u16::from(a) + 255 * (255 - u16::from(a))) / 255;
            u8::try_from(mixed).unwrap_or(u8::MAX)
        };
        image::Rgb([blend(r), blend(g), blend(b)])
    })
}

pub fn encode_jpeg(image: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, quality)
        .encode_image(&DynamicImage::ImageRgb8(on_white(image)))
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

/// The picture at `size` (cropped to fill it), or at its own size, in `format`.
pub fn convert(
    image: &DynamicImage,
    size: Option<(u32, u32)>,
    format: &str,
) -> Result<Vec<u8>, String> {
    let mut image = match size {
        Some((width, height)) => image.resize_to_fill(width, height, FilterType::Lanczos3),
        None => image.clone(),
    };
    if format == "ico" && (image.width() > ICON_SIDE || image.height() > ICON_SIDE) {
        image = image.resize(ICON_SIDE, ICON_SIDE, FilterType::Lanczos3);
    }
    let kind = match format {
        "jpg" => return encode_jpeg(&image, 90),
        "png" => ImageFormat::Png,
        "webp" => ImageFormat::WebP,
        "gif" => ImageFormat::Gif,
        "bmp" => ImageFormat::Bmp,
        "ico" => ImageFormat::Ico,
        "tiff" => ImageFormat::Tiff,
        other => return Err(format!("unknown image format {other}")),
    };
    let image = if kind == ImageFormat::Gif || kind == ImageFormat::WebP {
        DynamicImage::ImageRgba8(image.to_rgba8())
    } else {
        image
    };
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, kind)
        .map_err(|error| error.to_string())?;
    Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, Rgba, RgbaImage};

    fn sample() -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_fn(300, 200, |x, y| {
            Rgba([
                u8::try_from(x % 256).unwrap(),
                u8::try_from(y).unwrap(),
                90,
                if x < 150 { 255 } else { 0 },
            ])
        }))
    }

    #[test]
    fn writes_every_format_at_the_size_asked() {
        for format in FORMATS {
            let bytes = convert(&sample(), Some((64, 48)), format).unwrap();
            let read = image::load_from_memory(&bytes).unwrap();
            assert_eq!(read.dimensions(), (64, 48), "{format}");
        }
    }

    #[test]
    fn keeps_its_size_or_fits_an_icon() {
        let png = image::load_from_memory(&convert(&sample(), None, "png").unwrap()).unwrap();
        assert_eq!(png.dimensions(), (300, 200));
        let ico = image::load_from_memory(&convert(&sample(), Some((1920, 1080)), "ico").unwrap())
            .unwrap();
        assert_eq!(ico.dimensions(), (256, 144));
    }

    #[test]
    fn puts_white_under_transparency_in_a_jpeg() {
        let jpeg = image::load_from_memory(&convert(&sample(), None, "jpg").unwrap()).unwrap();
        let [r, g, b, _] = jpeg.get_pixel(290, 10).0;
        assert!(r > 240 && g > 240 && b > 240);
    }

    #[test]
    fn refuses_an_unknown_picture_or_format() {
        assert!(load(Path::new("."), "../secret").is_err());
        assert!(convert(&sample(), None, "psd").is_err());
    }
}
