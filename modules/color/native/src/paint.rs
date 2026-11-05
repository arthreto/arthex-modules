// Everything here is pixel arithmetic: sizes well within what these types hold exactly.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

/// `--accent` of `src/ui/tokens.css`, as blue, green and red, the order of a Windows bitmap.
const ACCENT: [f32; 3] = [255.0, 194.0, 76.0];
/// The tag's `--surface-dialog`, in the same order.
pub const TAG_BACKGROUND: [u8; 3] = [46, 39, 36];
const SWATCH_RING: f32 = 0.45;

/// The mockup draws the fade on a screen of this height; other screens scale it.
const REFERENCE_HEIGHT: f32 = 800.0;
const EDGE_REACH: f32 = 72.0;
const EDGE_STRENGTH: f32 = 0.34;
const CORNER_REACH: f32 = 300.0;
const CORNER_STRENGTH: f32 = 0.38;
const OUTLINE_STRENGTH: f32 = 0.7;
/// A layered window lets clicks through where its alpha is zero: the picker must catch them all.
const FLOOR: f32 = 1.0 / 255.0;

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

/// The whole fade, premultiplied blue, green, red and alpha, from the top row down.
pub fn edge_fade(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let alpha = edge_alpha(x, y, width, height);
            pixels.extend([
                byte(ACCENT[0] * alpha),
                byte(ACCENT[1] * alpha),
                byte(ACCENT[2] * alpha),
                byte(alpha * 255.0),
            ]);
        }
    }
    pixels
}

fn segment_distance(px: f32, py: f32, from: (f32, f32), to: (f32, f32)) -> f32 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let along = (((px - from.0) * dx + (py - from.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (px - (from.0 + along * dx)).hypot(py - (from.1 + along * dy))
}

/// A straight stroke: where it starts, where it ends and its radius.
type Stroke = ((f32, f32), (f32, f32), f32);

/// A white pipette with a dark outline, in a 24 unit drawing: straight blue, green, red and alpha,
/// as a cursor wants. Its tip, the pixel it reads, is at `EYEDROPPER_TIP`.
pub fn eyedropper(size: u16) -> Vec<u8> {
    const SHAPES: [Stroke; 3] = [
        ((EYEDROPPER_TIP.0, EYEDROPPER_TIP.1), (13.0, 11.0), 1.1),
        ((14.5, 9.5), (19.5, 4.5), 2.5),
        ((11.0, 7.0), (17.0, 13.0), 1.3),
    ];
    const OUTLINE: f32 = 1.1;
    let unit = 24.0 / f32::from(size);
    let mut pixels = Vec::with_capacity(usize::from(size) * usize::from(size) * 4);
    for row in 0..size {
        for column in 0..size {
            let (x, y) = (
                (f32::from(column) + 0.5) * unit,
                (f32::from(row) + 0.5) * unit,
            );
            let distance = SHAPES
                .iter()
                .map(|(from, to, radius)| segment_distance(x, y, *from, *to) - radius)
                .fold(f32::MAX, f32::min);
            let body = clamp01((0.5 * unit - distance) / unit);
            let whole = clamp01((0.5 * unit + OUTLINE - distance) / unit);
            let level = byte(255.0 * body);
            pixels.extend([level, level, level, byte(255.0 * whole)]);
        }
    }
    pixels
}

/// Where the tip of `eyedropper` is, in its 24 unit drawing.
pub const EYEDROPPER_TIP: (f32, f32) = (2.5, 21.5);

/// The colour at a point of a screen copy in blue, green, red and alpha rows.
pub fn color_at(pixels: &[u8], width: u32, height: u32, x: i32, y: i32) -> Option<[u8; 3]> {
    let (x, y) = (u32::try_from(x).ok()?, u32::try_from(y).ok()?);
    if x >= width || y >= height {
        return None;
    }
    let start = (y as usize * width as usize + x as usize) * 4;
    let pixel = pixels.get(start..start + 3)?;
    Some([pixel[2], pixel[1], pixel[0]])
}

pub fn hex([red, green, blue]: [u8; 3]) -> String {
    format!("#{red:02X}{green:02X}{blue:02X}")
}

/// What goes under the text of the tag: its background and the colour it shows, opaque, so the
/// system can draw the text with its usual smoothing. `finish_tag` then cuts the shape out.
pub fn tag_base(width: u32, height: u32, [red, green, blue]: [u8; 3]) -> Vec<u8> {
    let inset = height as f32 * 0.19;
    let radius = height as f32 / 2.0 - inset;
    let centre = height as f32 / 2.0;
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let distance = (x as f32 + 0.5 - centre).hypot(y as f32 + 0.5 - centre);
            let swatch = clamp01(radius - distance + 0.5);
            // A thin light ring, so a colour close to the background still shows.
            let ring = clamp01(1.0 - (distance - radius).abs()) * SWATCH_RING;
            let mix = |ink: u8, back: u8| {
                let filled = f32::from(ink) * swatch + f32::from(back) * (1.0 - swatch);
                byte(filled + (255.0 - filled) * ring)
            };
            pixels.extend([
                mix(blue, TAG_BACKGROUND[0]),
                mix(green, TAG_BACKGROUND[1]),
                mix(red, TAG_BACKGROUND[2]),
                255,
            ]);
        }
    }
    pixels
}

/// Rounds the ends of the tag and premultiplies it, for a layered window. The system's text
/// clears alpha where it draws, so alpha is set anew for every pixel.
pub fn finish_tag(pixels: &mut [u8], width: u32, height: u32) {
    let radius = height as f32 / 2.0;
    for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = (index as u32 % width) as f32 + 0.5;
        let y = (index as u32 / width) as f32 + 0.5;
        let nearest_x = x.clamp(radius, width as f32 - radius);
        let coverage = clamp01(radius - (x - nearest_x).hypot(y - radius) + 0.5);
        for channel in &mut pixel[..3] {
            *channel = byte(f32::from(*channel) * coverage);
        }
        pixel[3] = byte(255.0 * coverage);
    }
}

/// Beside the cursor, but flipped to its other side rather than leaving the screen.
pub fn tag_position(
    cursor: (i32, i32),
    screen: (i32, i32, i32, i32),
    size: (i32, i32),
    gap: i32,
) -> (i32, i32) {
    let (left, top, right, bottom) = screen;
    let x = if cursor.0 + gap + size.0 > right {
        cursor.0 - gap - size.0
    } else {
        cursor.0 + gap
    };
    let y = if cursor.1 + gap + size.1 > bottom {
        cursor.1 - gap - size.1
    } else {
        cursor.1 + gap
    };
    (x.max(left), y.max(top))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: u32 = 1920;
    const HEIGHT: u32 = 1080;

    fn alpha(x: u32, y: u32) -> f32 {
        edge_alpha(x, y, WIDTH, HEIGHT)
    }

    #[test]
    fn the_fade_stays_clickable_everywhere_but_never_shows_in_the_middle() {
        assert!((alpha(WIDTH / 2, HEIGHT / 2) - FLOOR).abs() < 1e-6);
        let pixels = edge_fade(40, 30);
        assert!(pixels.as_chunks::<4>().0.iter().all(|pixel| pixel[3] >= 1));
    }

    #[test]
    fn the_fade_is_strongest_in_the_corners() {
        assert!(alpha(2, 2) > alpha(WIDTH / 2, 2));
        assert!(alpha(WIDTH / 2, 2) > alpha(WIDTH / 2, 60));
        assert!((alpha(WIDTH / 2, 500) - FLOOR).abs() < 1e-6);
    }

    #[test]
    fn the_fade_is_the_same_on_every_side() {
        assert!((alpha(5, 5) - alpha(WIDTH - 6, HEIGHT - 6)).abs() < 1e-5);
        assert!((alpha(5, 300) - alpha(WIDTH - 6, 300)).abs() < 1e-5);
    }

    #[test]
    fn the_fade_is_premultiplied() {
        for pixel in edge_fade(64, 48).as_chunks::<4>().0 {
            assert!(pixel[0] <= pixel[3] && pixel[1] <= pixel[3] && pixel[2] <= pixel[3]);
        }
    }

    #[test]
    fn the_eyedropper_is_drawn_up_to_its_tip() {
        let size = 32_u16;
        let pixels = eyedropper(size);
        assert_eq!(pixels.len(), 32 * 32 * 4);
        let tip_x = (EYEDROPPER_TIP.0 / 24.0 * f32::from(size)) as usize;
        let tip_y = (EYEDROPPER_TIP.1 / 24.0 * f32::from(size)) as usize;
        assert!(pixels[(tip_y * 32 + tip_x) * 4 + 3] > 0);
        assert_eq!(pixels[(31 * 32 + 31) * 4 + 3], 0);
    }

    #[test]
    fn reads_a_colour_where_the_cursor_is() {
        let pixels = [
            1, 2, 3, 255, //
            4, 5, 6, 255, //
            7, 8, 9, 255, //
            10, 11, 12, 255,
        ];
        assert_eq!(color_at(&pixels, 2, 2, 1, 1), Some([12, 11, 10]));
        assert_eq!(color_at(&pixels, 2, 2, 0, 0), Some([3, 2, 1]));
    }

    #[test]
    fn reads_nothing_off_the_screen() {
        let pixels = [0; 16];
        assert_eq!(color_at(&pixels, 2, 2, -1, 0), None);
        assert_eq!(color_at(&pixels, 2, 2, 2, 0), None);
        assert_eq!(color_at(&pixels, 2, 2, 0, 2), None);
    }

    #[test]
    fn writes_colours_as_web_codes() {
        assert_eq!(hex([76, 194, 255]), "#4CC2FF");
        assert_eq!(hex([0, 5, 10]), "#00050A");
    }

    #[test]
    fn a_colour_like_the_background_still_shows_by_its_ring() {
        let (width, height) = (100, 32);
        let [blue, green, red] = TAG_BACKGROUND;
        let pixels = tag_base(width, height, [red, green, blue]);
        let brightest = (0..32)
            .map(|x| pixels[(16 * width as usize + x) * 4])
            .max()
            .unwrap_or_default();
        assert!(brightest > blue + 40);
    }

    #[test]
    fn the_tag_shows_its_colour_and_rounds_its_ends() {
        let (width, height) = (100, 32);
        let mut pixels = tag_base(width, height, [255, 0, 0]);
        let at = |x: usize, y: usize| (y * width as usize + x) * 4;
        assert_eq!(pixels[at(16, 16)..at(16, 16) + 3], [0, 0, 255]);
        assert_eq!(pixels[at(60, 16)..at(60, 16) + 3], TAG_BACKGROUND);
        finish_tag(&mut pixels, width, height);
        assert_eq!(pixels[at(0, 0) + 3], 0);
        assert_eq!(pixels[at(99, 31) + 3], 0);
        assert_eq!(pixels[at(50, 16) + 3], 255);
    }

    #[test]
    fn the_tag_stays_on_the_screen() {
        let screen = (0, 0, 1920, 1080);
        let size = (140, 32);
        assert_eq!(tag_position((100, 100), screen, size, 18), (118, 118));
        assert_eq!(tag_position((1900, 100), screen, size, 18), (1742, 118));
        assert_eq!(tag_position((100, 1070), screen, size, 18), (118, 1020));
        assert_eq!(tag_position((5, 5), (0, 0, 100, 40), size, 18), (0, 0));
    }

    #[test]
    fn the_tag_follows_a_screen_left_of_the_main_one() {
        assert_eq!(
            tag_position((-500, 200), (-1920, 0, 0, 1080), (140, 32), 18),
            (-482, 218)
        );
    }
}
