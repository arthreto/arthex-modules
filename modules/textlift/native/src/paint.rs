// Everything here is pixel arithmetic: sizes well within what these types hold exactly.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

/// `--accent` of `src/ui/tokens.css`, as blue, green and red, the order of a Windows bitmap.
const ACCENT: [f32; 3] = [255.0, 194.0, 76.0];
/// `--surface-dialog`, in the same order.
const SURFACE: [u8; 3] = [46, 39, 36];
const TILE: [u8; 3] = [58, 51, 48];
const TILE_BORDER: [u8; 3] = [78, 72, 69];
/// `--tint-mint`, the colour of the module.
const MINT: [u8; 3] = [153, 211, 52];
const MUTED: [u8; 3] = [194, 185, 180];

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

/// The size of the bubble's parts, in a drawing at 100 %.
pub const BUBBLE_HEIGHT: f32 = 60.0;
pub const BUBBLE_TEXT_LEFT: f32 = 58.0;
pub const BUBBLE_TEXT_GAP: f32 = 14.0;
pub const BUBBLE_CLOSE_WIDTH: f32 = 40.0;
const BUBBLE_RADIUS: f32 = 14.0;
const TILE_LEFT: f32 = 16.0;
const TILE_SIZE: f32 = 32.0;
const TILE_RADIUS: f32 = 9.0;
const ICON_SIZE: f32 = 18.0;
const CLOSE_ICON_SIZE: f32 = 16.0;

type Segment = ((f32, f32), (f32, f32));

const ICON: [Segment; 11] = [
    ((4.0, 8.0), (4.0, 4.0)),
    ((4.0, 4.0), (8.0, 4.0)),
    ((16.0, 4.0), (20.0, 4.0)),
    ((20.0, 4.0), (20.0, 8.0)),
    ((20.0, 16.0), (20.0, 20.0)),
    ((20.0, 20.0), (16.0, 20.0)),
    ((8.0, 20.0), (4.0, 20.0)),
    ((4.0, 20.0), (4.0, 16.0)),
    ((8.0, 9.0), (16.0, 9.0)),
    ((8.0, 12.0), (16.0, 12.0)),
    ((8.0, 15.0), (13.0, 15.0)),
];
const CLOSE: [Segment; 2] = [((6.0, 6.0), (18.0, 18.0)), ((18.0, 6.0), (6.0, 18.0))];

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

/// How far a point is inside a rectangle with round corners, negative outside.
fn rounded_inside(x: f32, y: f32, rect: (f32, f32, f32, f32), radius: f32) -> f32 {
    let (left, top, right, bottom) = rect;
    let dx = (left + radius - x).max(x - (right - radius)).max(0.0);
    let dy = (top + radius - y).max(y - (bottom - radius)).max(0.0);
    radius - dx.hypot(dy)
}

fn segment_distance(px: f32, py: f32, from: (f32, f32), to: (f32, f32)) -> f32 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let along = (((px - from.0) * dx + (py - from.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (px - (from.0 + along * dx)).hypot(py - (from.1 + along * dy))
}

/// How much a stroked pictogram of 24 units covers a point.
fn stroke_coverage(x: f32, y: f32, origin: (f32, f32), size: f32, segments: &[Segment]) -> f32 {
    let unit = size / 24.0;
    let (ux, uy) = ((x - origin.0) / unit, (y - origin.1) / unit);
    let nearest = segments
        .iter()
        .map(|(from, to)| segment_distance(ux, uy, *from, *to))
        .fold(f32::MAX, f32::min);
    clamp01((0.9 - nearest) * unit + 0.5)
}

fn mix(under: [u8; 3], over: [u8; 3], amount: f32) -> [u8; 3] {
    [0, 1, 2].map(|channel| {
        byte(f32::from(under[channel]) * (1.0 - amount) + f32::from(over[channel]) * amount)
    })
}

/// The opaque bubble without its text: the tile, its pictogram and the close button.
pub fn bubble_base(width: u32, height: u32, scale: f32) -> Vec<u8> {
    let tile_top = (height as f32 - TILE_SIZE * scale) / 2.0;
    let tile = (
        TILE_LEFT * scale,
        tile_top,
        (TILE_LEFT + TILE_SIZE) * scale,
        tile_top + TILE_SIZE * scale,
    );
    let icon_origin = (
        tile.0 + (TILE_SIZE - ICON_SIZE) * scale / 2.0,
        tile.1 + (TILE_SIZE - ICON_SIZE) * scale / 2.0,
    );
    let close_origin = (
        width as f32 - (BUBBLE_CLOSE_WIDTH + CLOSE_ICON_SIZE) * scale / 2.0,
        (height as f32 - CLOSE_ICON_SIZE * scale) / 2.0,
    );
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for row in 0..height {
        for column in 0..width {
            let (x, y) = (column as f32 + 0.5, row as f32 + 0.5);
            let inside = rounded_inside(x, y, tile, TILE_RADIUS * scale);
            let mut color = mix(SURFACE, TILE_BORDER, clamp01(inside + 0.5));
            color = mix(color, TILE, clamp01(inside - scale + 0.5));
            color = mix(
                color,
                MINT,
                stroke_coverage(x, y, icon_origin, ICON_SIZE * scale, &ICON),
            );
            color = mix(
                color,
                MUTED,
                stroke_coverage(x, y, close_origin, CLOSE_ICON_SIZE * scale, &CLOSE),
            );
            pixels.extend([color[0], color[1], color[2], u8::MAX]);
        }
    }
    pixels
}

/// Rounds the corners and premultiplies, for a layered window. The system's text clears alpha where
/// it draws, so alpha is set anew for every pixel.
pub fn finish_bubble(pixels: &mut [u8], width: u32, height: u32, scale: f32) {
    let rect = (0.0, 0.0, width as f32, height as f32);
    for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = (index as u32 % width) as f32 + 0.5;
        let y = (index as u32 / width) as f32 + 0.5;
        let coverage = clamp01(rounded_inside(x, y, rect, BUBBLE_RADIUS * scale) + 0.5);
        for channel in &mut pixel[..3] {
            *channel = byte(f32::from(*channel) * coverage);
        }
        pixel[3] = byte(255.0 * coverage);
    }
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

    #[test]
    fn the_bubble_has_round_corners() {
        let (width, height) = (200, 60);
        let mut pixels = bubble_base(width, height, 1.0);
        finish_bubble(&mut pixels, width, height, 1.0);
        assert_eq!(pixels[3], 0);
        let centre = (30 * 200 + 100) * 4 + 3;
        assert_eq!(pixels[centre], 255);
    }

    #[test]
    fn the_bubble_shows_its_pictogram_and_close_button() {
        let (width, height) = (200u32, 60u32);
        let pixels = bubble_base(width, height, 1.0);
        let at = |x: u32, y: u32| -> [u8; 3] {
            let index = ((y * width + x) * 4) as usize;
            [pixels[index], pixels[index + 1], pixels[index + 2]]
        };
        // A line of the pictogram, and the centre of the cross.
        assert_ne!(at(32, 30), TILE);
        assert_ne!(at(180, 30), SURFACE);
    }
}
