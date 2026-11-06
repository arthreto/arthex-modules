// Everything here is pixel arithmetic: sizes well within what these types hold exactly.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

/// The HUD's background, as blue, green and red, the order of a Windows bitmap.
const SURFACE: [u8; 3] = [32, 26, 24];
const SURFACE_ALPHA: f32 = 0.92;
const TILE: [u8; 3] = [47, 41, 39];
const BORDER: [u8; 3] = [68, 62, 60];
const BUTTON: [u8; 3] = [54, 48, 46];
const MUTED: [u8; 3] = [194, 185, 180];

/// The size of the HUD's parts, in a drawing at 100 %.
pub const HUD_WIDTH: f32 = 440.0;
pub const HUD_HEIGHT: f32 = 66.0;
pub const HUD_HEIGHT_PROGRESS: f32 = 78.0;
pub const TEXT_LEFT: f32 = 66.0;
pub const CLOSE_LEFT_FROM_END: f32 = 42.0;
pub const BUTTON_HEIGHT: f32 = 32.0;
pub const BUTTON_GAP: f32 = 12.0;
const RADIUS: f32 = 16.0;
const TILE_LEFT: f32 = 14.0;
const TILE_SIZE: f32 = 40.0;
const TILE_RADIUS: f32 = 12.0;
const ICON_SIZE: f32 = 20.0;
const CLOSE_SIZE: f32 = 16.0;
const CLOSE_BOX: f32 = 32.0;
const BUTTON_RADIUS: f32 = 10.0;
const BAR_HEIGHT: f32 = 5.0;
const BAR_BOTTOM: f32 = 13.0;

type Segment = ((f32, f32), (f32, f32));

const CALENDAR: [Segment; 11] = [
    ((8.0, 2.0), (8.0, 6.0)),
    ((16.0, 2.0), (16.0, 6.0)),
    ((3.0, 10.0), (21.0, 10.0)),
    ((5.0, 4.0), (19.0, 4.0)),
    ((21.0, 6.0), (21.0, 20.0)),
    ((19.0, 22.0), (5.0, 22.0)),
    ((3.0, 20.0), (3.0, 6.0)),
    ((3.0, 6.0), (5.0, 4.0)),
    ((19.0, 4.0), (21.0, 6.0)),
    ((21.0, 20.0), (19.0, 22.0)),
    ((5.0, 22.0), (3.0, 20.0)),
];
const CLOSE: [Segment; 2] = [((6.0, 6.0), (18.0, 18.0)), ((18.0, 6.0), (6.0, 18.0))];

const fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn byte(value: f32) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}

/// The tints of `src/ui/tokens.css`, in the bitmap's order. They are the only ones an item can have.
pub fn tint_color(name: &str) -> [u8; 3] {
    match name {
        "violet" => [253, 181, 196],
        "green" => [183, 231, 110],
        "orange" => [116, 186, 253],
        "pink" => [212, 168, 249],
        "red" => [165, 165, 252],
        _ => [252, 211, 125],
    }
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

/// A rectangle with a border, and the fill inside it.
fn framed(under: [u8; 3], inside: f32, border: [u8; 3], fill: [u8; 3], scale: f32) -> [u8; 3] {
    let color = mix(under, border, clamp01(inside + 0.5));
    mix(color, fill, clamp01(inside - scale + 0.5))
}

/// Where the parts of a HUD go, from its size and the width of its button's text.
#[derive(Clone, Copy)]
pub struct HudLayout {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub button_left: f32,
    pub button_right: f32,
}

/// The opaque HUD without its text: the tile, the button, the close cross and the progress.
pub fn hud_base(layout: HudLayout, tint: [u8; 3], progress: Option<f32>) -> Vec<u8> {
    let HudLayout {
        width,
        height,
        scale,
        button_left,
        button_right,
    } = layout;
    let (full_width, full_height) = (width as f32, height as f32);
    let tile_top = if progress.is_some() {
        12.0 * scale
    } else {
        (full_height - TILE_SIZE * scale) / 2.0
    };
    let tile = (
        TILE_LEFT * scale,
        tile_top,
        (TILE_LEFT + TILE_SIZE) * scale,
        tile_top + TILE_SIZE * scale,
    );
    let icon = (
        tile.0 + (TILE_SIZE - ICON_SIZE) * scale / 2.0,
        tile.1 + (TILE_SIZE - ICON_SIZE) * scale / 2.0,
    );
    let button_top = (full_height - BUTTON_HEIGHT * scale) / 2.0;
    let button = (
        button_left,
        button_top,
        button_right,
        button_top + BUTTON_HEIGHT * scale,
    );
    let close_centre = full_width - (10.0 + CLOSE_BOX / 2.0) * scale;
    let close = (
        close_centre - CLOSE_SIZE * scale / 2.0,
        (full_height - CLOSE_SIZE * scale) / 2.0,
    );
    let bar_top = full_height - (BAR_BOTTOM + BAR_HEIGHT) * scale;
    let bar = (
        TEXT_LEFT * scale,
        bar_top,
        button_left - BUTTON_GAP * scale,
        bar_top + BAR_HEIGHT * scale,
    );
    let bar_radius = BAR_HEIGHT * scale / 2.0;
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for row in 0..height {
        for column in 0..width {
            let (x, y) = (column as f32 + 0.5, row as f32 + 0.5);
            let mut color = framed(
                SURFACE,
                rounded_inside(x, y, tile, TILE_RADIUS * scale),
                BORDER,
                TILE,
                scale,
            );
            color = framed(
                color,
                rounded_inside(x, y, button, BUTTON_RADIUS * scale),
                BORDER,
                BUTTON,
                scale,
            );
            color = mix(
                color,
                tint,
                stroke_coverage(x, y, icon, ICON_SIZE * scale, &CALENDAR),
            );
            color = mix(
                color,
                MUTED,
                stroke_coverage(x, y, close, CLOSE_SIZE * scale, &CLOSE),
            );
            if let Some(done) = progress {
                let on_bar = clamp01(rounded_inside(x, y, bar, bar_radius) + 0.5);
                let filled = x <= bar.0 + (bar.2 - bar.0) * done;
                let bar_color = if filled {
                    tint
                } else {
                    mix(SURFACE, tint, 0.18)
                };
                color = mix(color, bar_color, on_bar);
            }
            pixels.extend([color[0], color[1], color[2], u8::MAX]);
        }
    }
    pixels
}

/// Rounds the corners and premultiplies, for a layered window. The system's text clears alpha where
/// it draws, so alpha is set anew for every pixel. No pixel inside is fully clear, or clicks would
/// go through to what is behind.
pub fn finish_hud(pixels: &mut [u8], width: u32, height: u32, scale: f32) {
    let rect = (0.0, 0.0, width as f32, height as f32);
    for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = (index as u32 % width) as f32 + 0.5;
        let y = (index as u32 / width) as f32 + 0.5;
        let alpha = clamp01(rounded_inside(x, y, rect, RADIUS * scale) + 0.5) * SURFACE_ALPHA;
        for channel in &mut pixel[..3] {
            *channel = byte(f32::from(*channel) * alpha);
        }
        pixel[3] = byte(255.0 * alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAYOUT: HudLayout = HudLayout {
        width: 440,
        height: 78,
        scale: 1.0,
        button_left: 312.0,
        button_right: 390.0,
    };

    fn at(pixels: &[u8], x: u32, y: u32) -> [u8; 3] {
        let index = ((y * LAYOUT.width + x) * 4) as usize;
        [pixels[index], pixels[index + 1], pixels[index + 2]]
    }

    #[test]
    fn the_pictogram_takes_the_colour_of_the_item() {
        let tint = tint_color("orange");
        let pixels = hud_base(LAYOUT, tint, None);
        // The line under the top of the calendar, in the middle of the tile.
        assert_eq!(at(&pixels, 34, 37), tint);
    }

    #[test]
    fn the_progress_fills_from_the_left() {
        let tint = tint_color("sky");
        let pixels = hud_base(LAYOUT, tint, Some(0.5));
        let top = LAYOUT.height - 13 - 3;
        assert_eq!(at(&pixels, 70, top), tint);
        assert_ne!(at(&pixels, 290, top), tint);
    }

    #[test]
    fn without_progress_there_is_no_bar() {
        let tint = tint_color("sky");
        let pixels = hud_base(LAYOUT, tint, None);
        assert_eq!(at(&pixels, 70, LAYOUT.height - 13 - 3), SURFACE);
    }

    #[test]
    fn the_hud_has_round_corners_and_catches_clicks() {
        let mut pixels = hud_base(LAYOUT, tint_color("sky"), None);
        finish_hud(&mut pixels, LAYOUT.width, LAYOUT.height, 1.0);
        assert_eq!(pixels[3], 0);
        let centre = ((39 * LAYOUT.width + 220) * 4 + 3) as usize;
        assert!(pixels[centre] > 200);
    }

    #[test]
    fn an_unknown_tint_falls_back_on_sky() {
        assert_eq!(tint_color("nope"), tint_color("sky"));
    }
}
