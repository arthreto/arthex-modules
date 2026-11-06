//! Draws the mascot from `mascot-art.json`, the same drawing as the interface's, as a program has
//! no browser to do it. Its movements follow the keyframes of `Mascot.svelte`.
#![allow(clippy::cast_precision_loss)]

use std::{collections::HashMap, sync::OnceLock};

use serde::Deserialize;
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Rect,
    Stroke, Transform,
};

use crate::progress::Species;

const DARK_INK: &str = "#241A17";
const UNITS: f32 = 200.0;
/// The tail and a hop go past the 200 units of the drawing, which a window would cut.
const ROOM: f32 = 24.0;

/// The window that holds a mascot of this size.
#[allow(clippy::cast_possible_truncation)]
pub fn window_size(size: i32) -> i32 {
    (size as f32 * (UNITS + 2.0 * ROOM) / UNITS).ceil() as i32
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Look {
    main: String,
    dark: String,
    light: String,
    paw: String,
    ear_color: String,
    inner: String,
    tip_color: String,
    head: String,
    mask: String,
    ear: String,
    ear_inner: String,
    tail: String,
    tail_tip: String,
    striped: bool,
}

/// A shape of an outfit or of a hiding place.
#[derive(Deserialize)]
struct Part {
    #[serde(default)]
    layer: String,
    d: String,
    fill: Option<String>,
    opacity: Option<f32>,
    stroke: Option<String>,
    width: Option<f32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Peek {
    tail: String,
    tip: String,
    fake_tail: String,
    fake_tip: String,
}

#[derive(Deserialize)]
struct Art {
    common: HashMap<String, String>,
    species: HashMap<String, Look>,
    wear: HashMap<String, Vec<Part>>,
    hiding: HashMap<String, Vec<Part>>,
    peek: Peek,
}

fn art() -> Option<&'static Art> {
    static ART: OnceLock<Option<Art>> = OnceLock::new();
    ART.get_or_init(|| serde_json::from_str(include_str!("../../mascot-art.json")).ok())
        .as_ref()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Idle,
    Happy,
    Walk,
}

/// Where the parts are at a moment of the animation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    lift: f32,
    lean: f32,
    all_scale: (f32, f32),
    breath: (f32, f32),
    head_drop: f32,
    tail: f32,
    ear: f32,
    blink: f32,
    heart: f32,
}

/// Goes from 0 to 1 and back to 0 over a period, as an `ease-in-out` keyframe does.
fn swing(seconds: f32, period: f32) -> f32 {
    (1.0 - (seconds / period * std::f32::consts::TAU).cos()) / 2.0
}

fn wag(seconds: f32, period: f32) -> f32 {
    -7.0 + 16.0 * swing(seconds, period)
}

/// The short flick of the ears, at the end of each period.
fn flick(seconds: f32, period: f32) -> f32 {
    let at = (seconds % period) / period;
    if at < 0.88 {
        0.0
    } else if at < 0.92 {
        (at - 0.88) / 0.04 * -9.0
    } else if at < 0.96 {
        -9.0 + (at - 0.92) / 0.04 * 12.0
    } else {
        3.0 - (at - 0.96) / 0.04 * 3.0
    }
}

pub fn pose(mood: Mood, seconds: f32) -> Pose {
    let breath = swing(seconds, 3.2);
    let at = (seconds % 4.8) / 4.8;
    let mut pose = Pose {
        lift: 0.0,
        lean: 0.0,
        all_scale: (1.0, 1.0),
        breath: (1.0 + 0.015 * breath, 1.0 - 0.025 * breath),
        head_drop: 2.5 * breath,
        tail: wag(seconds, 2.2),
        ear: flick(seconds, 5.4),
        blink: if (0.93..0.99).contains(&at) {
            0.08
        } else {
            1.0
        },
        heart: -1.0,
    };
    match mood {
        Mood::Idle => {}
        Mood::Happy => {
            let hop = swing(seconds, 0.62);
            pose.lift = hop * 16.0;
            pose.all_scale = (1.03 - 0.05 * hop, 0.97 + 0.06 * hop);
            pose.tail = wag(seconds, 0.34);
            pose.ear = flick(seconds, 1.2);
            pose.heart = (seconds % 1.4) / 1.4;
        }
        Mood::Walk => {
            let step = swing(seconds, 0.5);
            pose.lift = step * 6.0;
            pose.lean = -2.5 + 5.0 * step;
            pose.tail = wag(seconds, 0.5);
        }
    }
    pose
}

fn hex(color: &str) -> Color {
    let channel = |at: usize| {
        color
            .get(at..at + 2)
            .and_then(|part| u8::from_str_radix(part, 16).ok())
            .unwrap_or(0)
    };
    Color::from_rgba8(channel(1), channel(3), channel(5), 255)
}

fn paint(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    paint
}

fn number(text: &[u8], at: &mut usize) -> Option<f32> {
    while text.get(*at).is_some_and(u8::is_ascii_whitespace) {
        *at += 1;
    }
    let start = *at;
    while text
        .get(*at)
        .is_some_and(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'-'))
    {
        *at += 1;
    }
    std::str::from_utf8(&text[start..*at]).ok()?.parse().ok()
}

/// The absolute commands the drawing uses: M, L, H, V, C, Q and Z.
pub fn parse_path(data: &str) -> Option<Path> {
    let text = data.as_bytes();
    let mut builder = PathBuilder::new();
    let (mut at, mut command) = (0, b' ');
    let (mut x, mut y) = (0.0, 0.0);
    while at < text.len() {
        if text[at].is_ascii_whitespace() {
            at += 1;
            continue;
        }
        if text[at].is_ascii_alphabetic() {
            command = text[at];
            at += 1;
            if command == b'Z' {
                builder.close();
                continue;
            }
        }
        match command {
            b'M' | b'L' => {
                (x, y) = (number(text, &mut at)?, number(text, &mut at)?);
                if command == b'M' {
                    builder.move_to(x, y);
                } else {
                    builder.line_to(x, y);
                }
            }
            b'H' => {
                x = number(text, &mut at)?;
                builder.line_to(x, y);
            }
            b'V' => {
                y = number(text, &mut at)?;
                builder.line_to(x, y);
            }
            b'C' => {
                let (x1, y1) = (number(text, &mut at)?, number(text, &mut at)?);
                let (x2, y2) = (number(text, &mut at)?, number(text, &mut at)?);
                (x, y) = (number(text, &mut at)?, number(text, &mut at)?);
                builder.cubic_to(x1, y1, x2, y2, x, y);
            }
            b'Q' => {
                let (x1, y1) = (number(text, &mut at)?, number(text, &mut at)?);
                (x, y) = (number(text, &mut at)?, number(text, &mut at)?);
                builder.quad_to(x1, y1, x, y);
            }
            _ => return None,
        }
    }
    builder.finish()
}

fn oval(x: f32, y: f32, radius_x: f32, radius_y: f32) -> Option<Path> {
    Rect::from_xywh(x - radius_x, y - radius_y, radius_x * 2.0, radius_y * 2.0)
        .map(PathBuilder::from_oval)?
}

struct Brush<'a> {
    pixmap: &'a mut Pixmap,
}

impl Brush<'_> {
    fn fill(&mut self, path: Option<Path>, color: Color, at: Transform) {
        if let Some(path) = path {
            self.pixmap
                .fill_path(&path, &paint(color), FillRule::Winding, at, None);
        }
    }

    fn data(&mut self, data: &str, color: &str, at: Transform) {
        self.fill(parse_path(data), hex(color), at);
    }

    fn line(&mut self, data: &str, color: &str, width: f32, at: Transform) {
        if let Some(path) = parse_path(data) {
            let stroke = Stroke {
                width,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            };
            self.pixmap
                .stroke_path(&path, &paint(hex(color)), &stroke, at, None);
        }
    }

    fn spot(&mut self, center: (f32, f32), radius: (f32, f32), color: Color, at: Transform) {
        self.fill(oval(center.0, center.1, radius.0, radius.1), color, at);
    }

    fn part(&mut self, part: &Part, at: Transform) {
        if let (Some(color), Some(stroke)) = (&part.stroke, part.width)
            && part.fill.is_none()
        {
            self.line(&part.d, color, stroke, at);
            return;
        }
        if let Some(color) = &part.fill {
            self.fill(
                parse_path(&part.d),
                soft(color, part.opacity.unwrap_or(1.0)),
                at,
            );
        }
        if let (Some(color), Some(width)) = (&part.stroke, part.width) {
            self.line(&part.d, color, width, at);
        }
    }
}

fn soft(color: &str, alpha: f32) -> Color {
    let mut color = hex(color);
    color.set_alpha(alpha);
    color
}

fn common<'a>(art: &'a Art, name: &str) -> &'a str {
    art.common.get(name).map_or("", String::as_str)
}

fn draw_wear(brush: &mut Brush, art: &Art, wear: Option<&str>, layer: &str, at: Transform) {
    let Some(parts) = wear.and_then(|wear| art.wear.get(wear)) else {
        return;
    };
    for part in parts.iter().filter(|part| part.layer == layer) {
        brush.part(part, at);
    }
}

fn draw_body(brush: &mut Brush, art: &Art, look: &Look, wear: Option<&str>, at: Transform) {
    draw_wear(brush, art, wear, "back", at);
    brush.data(common(art, "body"), &look.main, at);
    brush.data(common(art, "bib"), &look.light, at);
    brush.data(common(art, "pawLeft"), &look.paw, at);
    brush.data(common(art, "pawRight"), &look.paw, at);
    draw_wear(brush, art, wear, "neck", at);
}

fn draw_eyes(brush: &mut Brush, art: &Art, happy: bool, blink: f32, head: Transform) {
    if happy {
        brush.line(common(art, "eyesHappy"), DARK_INK, 5.0, head);
        return;
    }
    let eyes = head
        .pre_translate(100.0, 90.0)
        .pre_scale(1.0, blink)
        .pre_translate(-100.0, -90.0);
    brush.spot((74.0, 90.0), (10.5, 12.5), hex(DARK_INK), eyes);
    brush.spot((126.0, 90.0), (10.5, 12.5), hex(DARK_INK), eyes);
    brush.spot((78.0, 85.0), (4.0, 4.0), hex("#FFFFFF"), eyes);
    brush.spot((130.0, 85.0), (4.0, 4.0), hex("#FFFFFF"), eyes);
    brush.spot((71.0, 95.0), (2.0, 2.0), soft("#FFFFFF", 0.8), eyes);
    brush.spot((123.0, 95.0), (2.0, 2.0), soft("#FFFFFF", 0.8), eyes);
}

fn draw_head(brush: &mut Brush, art: &Art, look: &Look, pose: &Pose, happy: bool, at: Transform) {
    let flicked = at.pre_concat(Transform::from_rotate_at(pose.ear, 76.0, 56.0));
    for ear in [flicked, at.pre_translate(200.0, 0.0).pre_scale(-1.0, 1.0)] {
        brush.data(&look.ear, &look.ear_color, ear);
        brush.line(&look.ear, &look.ear_color, 5.0, ear);
        brush.data(&look.ear_inner, &look.inner, ear);
    }
    brush.data(&look.head, &look.main, at);
    brush.data(&look.mask, &look.light, at);
    if look.striped {
        brush.line(common(art, "stripes"), &look.dark, 5.0, at);
    }
    let cheek = soft("#FF8FA3", 0.42);
    brush.spot((60.0, 108.0), (11.0, 7.0), cheek, at);
    brush.spot((140.0, 108.0), (11.0, 7.0), cheek, at);
    draw_eyes(brush, art, happy, pose.blink, at);
    brush.data(common(art, "nose"), DARK_INK, at);
    if happy {
        brush.data(common(art, "mouthOpen"), "#7A2E3A", at);
        brush.data(common(art, "tongue"), "#FF8FA3", at);
    } else {
        brush.line(common(art, "mouth"), DARK_INK, 2.6, at);
    }
    if look.striped {
        brush.line(common(art, "whiskers"), &look.dark, 2.2, at);
    }
}

/// A mascot of `size` pixels, in premultiplied red, green, blue and alpha. `None` if the drawing
/// cannot be read, which a test rules out.
pub fn render(
    species: Species,
    mood: Mood,
    wear: Option<&str>,
    seconds: f32,
    size: u32,
) -> Option<Pixmap> {
    let art = art()?;
    let look = art.species.get(species.key())?;
    let mut pixmap = Pixmap::new(size, size)?;
    let pose = pose(mood, seconds);
    let scale = size as f32 / (UNITS + 2.0 * ROOM);
    let canvas = Transform::from_scale(scale, scale).pre_translate(ROOM, ROOM);
    let all = canvas
        .pre_translate(0.0, -pose.lift)
        .pre_concat(Transform::from_rotate_at(pose.lean, 100.0, 186.0))
        .pre_translate(100.0, 186.0)
        .pre_scale(pose.all_scale.0, pose.all_scale.1)
        .pre_translate(-100.0, -186.0);
    let mut brush = Brush {
        pixmap: &mut pixmap,
    };
    brush.spot((100.0, 189.0), (56.0, 7.0), soft("#000000", 0.22), canvas);
    let tail = all.pre_concat(Transform::from_rotate_at(pose.tail, 126.0, 178.0));
    brush.data(&look.tail, &look.main, tail);
    brush.data(&look.tail_tip, &look.tip_color, tail);
    let body = all
        .pre_translate(100.0, 186.0)
        .pre_scale(pose.breath.0, pose.breath.1)
        .pre_translate(-100.0, -186.0);
    draw_body(&mut brush, art, look, wear, body);
    let head = all.pre_translate(0.0, pose.head_drop);
    draw_head(&mut brush, art, look, &pose, mood == Mood::Happy, head);
    draw_wear(&mut brush, art, wear, "head", head);
    if pose.heart >= 0.0 {
        let rise = 6.0 - 28.0 * pose.heart;
        let fade = if pose.heart < 0.3 {
            pose.heart / 0.3
        } else {
            (1.0 - pose.heart) / 0.7
        };
        brush.fill(
            parse_path(common(art, "heart")),
            soft("#FF6B86", fade),
            all.pre_translate(0.0, rise),
        );
    }
    Some(pixmap)
}

/// The room around a thing of hide and seek, in units of its drawing of 140 by 120: its tail, its
/// shadow and its shaking go past the drawing.
pub const THING_ROOM: f32 = 14.0;
const THING_WIDTH: f32 = 140.0;
const THING_HEIGHT: f32 = 120.0;

/// What sticks out of a hiding place at the moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sticking {
    Tail,
    Fake,
}

/// How a thing of hide and seek looks at a moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThingLook<'a> {
    pub art: &'a str,
    pub sticking: Option<Sticking>,
    /// A plant is narrow: what sticks out of it has to be closer.
    pub sticking_shift: f32,
    pub angle: f32,
    pub opacity: f32,
}

/// The window of a thing that is this wide on the screen.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn thing_window(width: i32) -> (i32, i32) {
    let unit = width as f32 / THING_WIDTH;
    (
        (unit * (THING_WIDTH + 2.0 * THING_ROOM)).ceil() as i32,
        (unit * (THING_HEIGHT + 2.0 * THING_ROOM)).ceil() as i32,
    )
}

/// A thing of hide and seek in its window, of `width` pixels without the room around it.
#[allow(clippy::cast_sign_loss)]
pub fn render_thing(species: Species, thing: &ThingLook, width: i32) -> Option<Pixmap> {
    let art = art()?;
    let look = art.species.get(species.key())?;
    let parts = art.hiding.get(thing.art)?;
    let (window_width, window_height) = thing_window(width);
    let mut pixmap = Pixmap::new(window_width as u32, window_height as u32)?;
    let unit = width as f32 / THING_WIDTH;
    let at = Transform::from_scale(unit, unit)
        .pre_translate(THING_ROOM, THING_ROOM)
        .pre_concat(Transform::from_rotate_at(thing.angle, 70.0, 116.0));
    let mut brush = Brush {
        pixmap: &mut pixmap,
    };
    brush.spot((70.0, 115.0), (56.0, 6.0), soft("#000000", 0.25), at);
    let stuck = at.pre_translate(thing.sticking_shift, 0.0);
    match thing.sticking {
        Some(Sticking::Tail) => {
            brush.data(&art.peek.tail, &look.main, stuck);
            brush.data(&art.peek.tip, &look.tip_color, stuck);
        }
        Some(Sticking::Fake) => {
            brush.data(&art.peek.fake_tail, &look.main, stuck);
            brush.data(&art.peek.fake_tip, "#8A5A2B", stuck);
        }
        None => {}
    }
    for part in parts {
        brush.part(part, at);
    }
    fade(&mut pixmap, thing.opacity);
    Some(pixmap)
}

fn fade(pixmap: &mut Pixmap, opacity: f32) {
    if opacity >= 1.0 {
        return;
    }
    for byte in pixmap.data_mut() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let faded = (f32::from(*byte) * opacity) as u8;
        *byte = faded;
    }
}

/// The mascot's own shape in cardboard: the shades of brown of a cut-out, standing still.
fn cardboard(species: Species, size: u32) -> Option<Pixmap> {
    let mut pixmap = render(species, Mood::Idle, None, 0.0, size)?;
    for pixel in pixmap.data_mut().as_chunks_mut::<4>().0 {
        let [red, green, blue, alpha] = *pixel;
        let shade = 0.3 * f32::from(red) + 0.59 * f32::from(green) + 0.11 * f32::from(blue);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let tone = |amount: f32| (shade * amount).min(f32::from(alpha)) as u8;
        *pixel = [tone(0.98), tone(0.76), tone(0.47), alpha];
    }
    Some(pixmap)
}

pub fn render_card(species: Species, size: u32) -> Option<Pixmap> {
    cardboard(species, size)
}

/// A cut-out once knocked over, on its back: twice as wide and half as high again, with the place
/// where its feet were in the middle of the bottom of its old height.
pub fn render_fallen_card(species: Species, size: u32) -> Option<Pixmap> {
    let card = cardboard(species, size)?;
    let mut pixmap = Pixmap::new(size * 2, size + size / 2)?;
    let half = size as f32 / 2.0;
    let at = Transform::from_rotate_at(-84.0, size as f32, size as f32).pre_translate(half, 0.0);
    pixmap.draw_pixmap(0, 0, card.as_ref(), &PixmapPaint::default(), at, None);
    fade(&mut pixmap, 0.55);
    Some(pixmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Species; 4] = [Species::Fox, Species::Dog, Species::Cat, Species::Wolf];
    const MOODS: [Mood; 3] = [Mood::Idle, Mood::Happy, Mood::Walk];

    fn alpha(pixmap: &Pixmap, x: u32, y: u32) -> u8 {
        pixmap
            .pixel(x, y)
            .map_or(0, tiny_skia::PremultipliedColorU8::alpha)
    }

    #[test]
    fn every_path_of_the_drawing_can_be_read() {
        let art = art().unwrap();
        for (name, data) in &art.common {
            assert!(parse_path(data).is_some(), "{name}");
        }
        for (name, parts) in art.wear.iter().chain(&art.hiding) {
            for part in parts {
                assert!(parse_path(&part.d).is_some(), "{name}: {}", part.d);
                assert!(part.fill.is_some() || part.stroke.is_some(), "{name}");
            }
        }
        for data in [
            &art.peek.tail,
            &art.peek.tip,
            &art.peek.fake_tail,
            &art.peek.fake_tip,
        ] {
            assert!(parse_path(data).is_some(), "{data}");
        }
        for (name, look) in &art.species {
            for data in [
                &look.head,
                &look.mask,
                &look.ear,
                &look.ear_inner,
                &look.tail,
                &look.tail_tip,
            ] {
                assert!(parse_path(data).is_some(), "{name}: {data}");
            }
        }
    }

    #[test]
    fn every_mascot_is_drawn_in_every_mood() {
        for species in ALL {
            for mood in MOODS {
                let pixmap = render(species, mood, Some("scarf"), 0.7, 200).unwrap();
                assert!(
                    alpha(&pixmap, 100, 90) > 200,
                    "{species:?} {mood:?}: the head"
                );
                assert_eq!(
                    alpha(&pixmap, 2, 2),
                    0,
                    "{species:?} {mood:?}: a corner stays clear"
                );
                for edge in 0..200 {
                    assert_eq!(
                        alpha(&pixmap, 199, edge),
                        0,
                        "{species:?} {mood:?}: the tail"
                    );
                    assert_eq!(alpha(&pixmap, edge, 0), 0, "{species:?} {mood:?}: the top");
                }
            }
        }
    }

    #[test]
    fn its_colour_comes_from_its_species() {
        let fox = render(Species::Fox, Mood::Idle, None, 0.0, 200).unwrap();
        let wolf = render(Species::Wolf, Mood::Idle, None, 0.0, 200).unwrap();
        assert_ne!(fox.pixel(100, 60), wolf.pixel(100, 60));
    }

    #[test]
    fn it_follows_the_size_asked_for() {
        let large = render(Species::Cat, Mood::Idle, None, 0.0, 300).unwrap();
        assert_eq!((large.width(), large.height()), (300, 300));
        assert!(render(Species::Cat, Mood::Idle, None, 0.0, 0).is_none());
    }

    #[test]
    fn an_accessory_changes_the_drawing() {
        let shop: serde_json::Value =
            serde_json::from_str(include_str!("../../shop.json")).unwrap();
        let plain = render(Species::Fox, Mood::Idle, None, 0.0, 200).unwrap();
        let items = shop["wear"].as_array().unwrap();
        assert_eq!(items.len(), art().unwrap().wear.len());
        for item in items {
            let wear = item["id"].as_str().unwrap();
            let dressed = render(Species::Fox, Mood::Idle, Some(wear), 0.0, 200).unwrap();
            assert_ne!(plain.data(), dressed.data(), "{wear}");
        }
    }

    #[test]
    fn no_outfit_is_cut_by_the_window() {
        for species in ALL {
            for wear in art().unwrap().wear.keys() {
                for mood in MOODS {
                    let pixmap = render(species, mood, Some(wear), 0.3, 200).unwrap();
                    for edge in 0..200 {
                        assert_eq!(alpha(&pixmap, edge, 0), 0, "{wear} {species:?}: the top");
                        assert_eq!(alpha(&pixmap, 0, edge), 0, "{wear} {species:?}: the left");
                    }
                }
            }
        }
    }

    fn thing(art: &'static str, sticking: Option<Sticking>) -> ThingLook<'static> {
        ThingLook {
            art,
            sticking,
            sticking_shift: 0.0,
            angle: 0.0,
            opacity: 1.0,
        }
    }

    #[test]
    fn every_thing_fits_its_window() {
        let (width, height) = thing_window(140);
        for art in ["box", "open", "plant", "basket", "cushion"] {
            for sticking in [None, Some(Sticking::Tail), Some(Sticking::Fake)] {
                let pixmap = render_thing(Species::Fox, &thing(art, sticking), 140).unwrap();
                assert_eq!(
                    (pixmap.width(), pixmap.height()),
                    (width.cast_unsigned(), height.cast_unsigned())
                );
                for edge in 0..pixmap.height() {
                    assert_eq!(alpha(&pixmap, pixmap.width() - 1, edge), 0, "{art}: right");
                    assert_eq!(alpha(&pixmap, 0, edge), 0, "{art}: left");
                }
                assert!(pixmap.data().iter().any(|byte| *byte > 0), "{art}");
            }
        }
    }

    #[test]
    fn a_tail_changes_a_thing_and_the_real_one_differs_from_the_fake() {
        let plain = render_thing(Species::Fox, &thing("box", None), 140).unwrap();
        let real = render_thing(Species::Fox, &thing("box", Some(Sticking::Tail)), 140).unwrap();
        let fake = render_thing(Species::Fox, &thing("box", Some(Sticking::Fake)), 140).unwrap();
        assert_ne!(plain.data(), real.data());
        assert_ne!(real.data(), fake.data());
    }

    #[test]
    fn a_cut_out_stays_valid_once_recoloured_and_fallen() {
        let card = render_card(Species::Dog, 120).unwrap();
        let fallen = render_fallen_card(Species::Dog, 120).unwrap();
        for pixmap in [&card, &fallen] {
            for pixel in pixmap.data().as_chunks::<4>().0 {
                assert!(pixel[..3].iter().all(|channel| *channel <= pixel[3]));
            }
            assert!(pixmap.data().iter().any(|byte| *byte > 0));
        }
    }

    #[test]
    fn it_moves_with_time() {
        let first = render(Species::Fox, Mood::Happy, None, 0.1, 200).unwrap();
        let later = render(Species::Fox, Mood::Happy, None, 0.3, 200).unwrap();
        assert_ne!(first.data(), later.data());
    }

    #[test]
    fn it_blinks_now_and_then() {
        assert_eq!(pose(Mood::Idle, 0.0).blink, 1.0);
        assert!(pose(Mood::Idle, 4.6).blink < 0.5);
    }

    #[test]
    fn a_broken_path_is_refused() {
        assert!(parse_path("M1 2 X3 4").is_none());
        assert!(parse_path("M1").is_none());
    }
}
