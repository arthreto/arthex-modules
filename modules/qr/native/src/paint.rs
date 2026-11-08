// Everything here is pixel arithmetic: sizes well within what these types hold exactly.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

/// `--accent` of `src/ui/tokens.css`, as blue, green and red, the order of a Windows bitmap.
const ACCENT: [f32; 3] = [255.0, 194.0, 76.0];

/// The mockup draws the fade on a screen of this height; other screens scale it.
const REFERENCE_HEIGHT: f32 = 800.0;
const EDGE_REACH: f32 = 72.0;
const EDGE_STRENGTH: f32 = 0.34;
const CORNER_REACH: f32 = 300.0;
const CORNER_STRENGTH: f32 = 0.38;
const OUTLINE_STRENGTH: f32 = 0.7;
/// A layered window lets clicks through where its alpha is zero: the selection must catch them all.
const FLOOR: f32 = 1.0 / 255.0;
const ZONE_FILL: f32 = 0.14;

const fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn byte(value: f32) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}

/// A soft fade from the edges of a screen, stronger in the corners, gone well before the centre.
fn edge_alpha(x: u32, y: u32, width: u32, height: u32) -> f32 {
    let scale = height as f32 / REFERENCE_HEIGHT;
    let (from_left, from_top) = (x as f32, y as f32);
    let (from_right, from_bottom) = ((width - 1 - x) as f32, (height - 1 - y) as f32);
    let nearest = from_left.min(from_top).min(from_right).min(from_bottom);
    if nearest < 1.0 {
        return OUTLINE_STRENGTH;
    }
    let edge = clamp01(1.0 - nearest / (EDGE_REACH * scale)).powi(2) * EDGE_STRENGTH;
    let corner_distance = from_left.min(from_right).hypot(from_top.min(from_bottom));
    let corner = clamp01(1.0 - corner_distance / (CORNER_REACH * scale)).powi(2) * CORNER_STRENGTH;
    (1.0 - (1.0 - edge) * (1.0 - corner)).max(FLOOR)
}

fn accent(alpha: f32) -> [u8; 4] {
    [
        byte(ACCENT[0] * alpha),
        byte(ACCENT[1] * alpha),
        byte(ACCENT[2] * alpha),
        byte(alpha * 255.0),
    ]
}

/// The whole fade, premultiplied blue, green, red and alpha, from the top row down.
pub fn edge_fade(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            pixels.extend(accent(edge_alpha(x, y, width, height)));
        }
    }
    pixels
}

/// The zone being traced: a light fill inside a solid border, premultiplied.
pub fn zone_frame(width: u32, height: u32, border: u32) -> Vec<u8> {
    let (fill, line) = (accent(ZONE_FILL), accent(1.0));
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let on_border = x < border || y < border || x + border >= width || y + border >= height;
            pixels.extend(if on_border { line } else { fill });
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fade_never_lets_a_click_through() {
        assert!(
            edge_fade(200, 100)
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| pixel[3] > 0)
        );
    }

    #[test]
    fn the_zone_has_a_solid_border_and_a_light_fill() {
        let pixels = zone_frame(6, 6, 2);
        assert_eq!(pixels[3], 255);
        let centre = (3 * 6 + 3) * 4 + 3;
        assert_eq!(pixels[centre], byte(ZONE_FILL * 255.0));
    }
}
