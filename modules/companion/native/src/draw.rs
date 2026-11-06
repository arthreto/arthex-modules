//! Draws the mascot from `mascot-art.json`, the same drawing as the interface's, as a program has
//! no browser to do it. Its movements follow the keyframes of `Mascot.svelte`.
#![allow(clippy::cast_precision_loss)]

use std::{collections::HashMap, sync::OnceLock};

use serde::Deserialize;
use tiny_skia::{
    Color, FillRule, LineCap, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform,
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

#[derive(Deserialize)]
struct Art {
    common: HashMap<String, String>,
    species: HashMap<String, Look>,
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
    Sniff,
    Walk,
}

/// Where the parts are at a moment of the animation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    lift: f32,
    lean: f32,
    all_scale: (f32, f32),
    breath: (f32, f32),
    head: (f32, f32),
    head_lean: f32,
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
        head: (0.0, 2.5 * breath),
        head_lean: 0.0,
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
        Mood::Sniff => {
            let turn = (seconds * std::f32::consts::TAU).sin();
            pose.head = (
                2.0 * turn,
                7.0 + 2.0 * (seconds * 2.0 * std::f32::consts::TAU).sin(),
            );
            pose.head_lean = 3.0 * turn - 1.5;
            pose.tail = wag(seconds, 1.0);
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
                ..Stroke::default()
            };
            self.pixmap
                .stroke_path(&path, &paint(hex(color)), &stroke, at, None);
        }
    }

    fn spot(&mut self, center: (f32, f32), radius: (f32, f32), color: Color, at: Transform) {
        self.fill(oval(center.0, center.1, radius.0, radius.1), color, at);
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

fn draw_body(brush: &mut Brush, art: &Art, look: &Look, wear: Option<&str>, at: Transform) {
    brush.data(common(art, "body"), &look.main, at);
    brush.data(common(art, "bib"), &look.light, at);
    brush.data(common(art, "pawLeft"), &look.paw, at);
    brush.data(common(art, "pawRight"), &look.paw, at);
    match wear {
        Some("scarf") => {
            brush.data(common(art, "scarf"), "#E5534B", at);
            brush.data(common(art, "scarfKnot"), "#C93F38", at);
        }
        Some("bow") => {
            brush.data(common(art, "bowLeft"), "#7C8CF8", at);
            brush.data(common(art, "bowRight"), "#7C8CF8", at);
            brush.spot((100.0, 134.0), (6.0, 6.0), hex("#5B6BE0"), at);
        }
        _ => {}
    }
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

fn draw_head_wear(brush: &mut Brush, art: &Art, wear: Option<&str>, at: Transform) {
    match wear {
        Some("glasses") => {
            for x in [74.0, 126.0] {
                brush.spot((x, 90.0), (17.0, 17.0), soft("#FFFFFF", 0.12), at);
                if let Some(ring) = oval(x, 90.0, 17.0, 17.0) {
                    let stroke = Stroke {
                        width: 4.0,
                        ..Stroke::default()
                    };
                    brush
                        .pixmap
                        .stroke_path(&ring, &paint(hex("#1F2430")), &stroke, at, None);
                }
            }
            brush.line(common(art, "glassesBridge"), "#1F2430", 4.0, at);
        }
        Some("crown") => {
            brush.line(common(art, "crown"), "#D9A520", 3.0, at);
            brush.data(common(art, "crown"), "#F5C542", at);
            brush.spot((100.0, 30.0), (4.0, 4.0), hex("#E5534B"), at);
        }
        _ => {}
    }
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
    let head = all
        .pre_translate(pose.head.0, pose.head.1)
        .pre_concat(Transform::from_rotate_at(pose.head_lean, 100.0, 124.0));
    draw_head(&mut brush, art, look, &pose, mood == Mood::Happy, head);
    draw_head_wear(&mut brush, art, wear, head);
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

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Species; 4] = [Species::Fox, Species::Dog, Species::Cat, Species::Wolf];
    const MOODS: [Mood; 4] = [Mood::Idle, Mood::Happy, Mood::Sniff, Mood::Walk];

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
        let plain = render(Species::Fox, Mood::Idle, None, 0.0, 200).unwrap();
        for wear in ["scarf", "glasses", "bow", "crown"] {
            let dressed = render(Species::Fox, Mood::Idle, Some(wear), 0.0, 200).unwrap();
            assert_ne!(plain.data(), dressed.data(), "{wear}");
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
