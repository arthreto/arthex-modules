//! Where the mascot goes on the screen, away from anything that talks to Windows.

/// A rectangle of the screen, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Area {
    const fn width(self) -> i32 {
        self.right - self.left
    }

    const fn height(self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Box,
    Plant,
    Basket,
    Cushion,
    Card,
}

/// What sticks out of a hiding place, now and then.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tail {
    pub fake: bool,
    pub offset: f32,
}

/// The shaking of a hiding place: how often, and from when.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shake {
    pub period: f32,
    pub delay: f32,
}

/// A thing on the screen in the game of hide and seek.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub kind: Kind,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub hides: bool,
    pub tail: Option<Tail>,
    pub shake: Option<Shake>,
    pub opened: bool,
}

impl Spot {
    /// Its height without the room around its drawing, of 140 by 120 but for a cut-out.
    pub const fn height(&self) -> i32 {
        match self.kind {
            Kind::Card => self.width,
            _ => self.width * 120 / 140,
        }
    }
}

/// Every thing on the screen, with its width at the normal scale.
const THINGS: [(Kind, f32); 10] = [
    (Kind::Box, 112.0),
    (Kind::Box, 132.0),
    (Kind::Box, 100.0),
    (Kind::Box, 120.0),
    (Kind::Plant, 96.0),
    (Kind::Plant, 104.0),
    (Kind::Basket, 118.0),
    (Kind::Cushion, 112.0),
    (Kind::Card, 110.0),
    (Kind::Card, 96.0),
];
const COLUMNS: i32 = 4;
const ROWS: i32 = 3;
const FAKE_TAILS: usize = 2;
const TAIL_PERIOD: f32 = 7.0;
const FAKE_TAIL_PERIOD: f32 = 9.0;
const TAIL_SHOWN: f32 = 1.5;
const REAL_SHAKE: Shake = Shake {
    period: 5.2,
    delay: 0.0,
};
const FAKE_SHAKE: Shake = Shake {
    period: 7.4,
    delay: 1.6,
};

/// A small generator that is good enough to scatter things.
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[allow(clippy::cast_possible_truncation)]
fn pick(state: &mut u64, below: usize) -> usize {
    (next(state) % below.max(1) as u64) as usize
}

fn shuffled<T>(items: &mut [T], state: &mut u64) {
    for last in (1..items.len()).rev() {
        items.swap(last, pick(state, last + 1));
    }
}

#[allow(clippy::cast_precision_loss)]
fn offset(state: &mut u64, period: f32) -> f32 {
    (next(state) % 1000) as f32 / 1000.0 * period
}

/// Scatters the things on the screen, each in its own cell of a grid so that none covers another,
/// under `top`, where the counter is. The mascot hides in anything but a cut-out, and two other
/// things show a fake tail of their own.
pub fn lay_out(work: Area, scale: f32, top: i32, seed: u64) -> Vec<Spot> {
    let mut state = seed | 1;
    let (cell_width, cell_height) = (work.width() / COLUMNS, (work.height() - top).max(1) / ROWS);
    let mut cells: Vec<(i32, i32)> = (0..ROWS)
        .flat_map(|row| (0..COLUMNS).map(move |column| (column, row)))
        .collect();
    shuffled(&mut cells, &mut state);
    let biggest = cell_width.min(cell_height) * 3 / 4;
    let mut spots: Vec<Spot> = THINGS
        .iter()
        .zip(&cells)
        .map(|((kind, width), (column, row))| {
            let width = scaled_width(*width, scale).min(biggest);
            let mut spot = Spot {
                kind: *kind,
                x: 0,
                y: 0,
                width,
                hides: false,
                tail: None,
                shake: None,
                opened: false,
            };
            let room_x = u64::try_from(cell_width - width).unwrap_or(0) / 2 + 1;
            let room_y = u64::try_from(cell_height - spot.height()).unwrap_or(0) / 2 + 1;
            spot.x = work.left
                + column * cell_width
                + (cell_width - width) / 4
                + i32::try_from(next(&mut state) % room_x).unwrap_or(0);
            spot.y = work.top
                + top
                + row * cell_height
                + (cell_height - spot.height()) / 4
                + i32::try_from(next(&mut state) % room_y).unwrap_or(0);
            spot
        })
        .collect();
    let mut hiding_places: Vec<usize> = spots
        .iter()
        .enumerate()
        .filter(|(_, spot)| spot.kind != Kind::Card)
        .map(|(index, _)| index)
        .collect();
    shuffled(&mut hiding_places, &mut state);
    let (real, others) = hiding_places.split_at(1);
    spots[real[0]].hides = true;
    spots[real[0]].shake = Some(REAL_SHAKE);
    spots[real[0]].tail = Some(Tail {
        fake: false,
        offset: offset(&mut state, TAIL_PERIOD),
    });
    for index in others.iter().take(FAKE_TAILS) {
        spots[*index].shake = Some(FAKE_SHAKE);
        spots[*index].tail = Some(Tail {
            fake: true,
            offset: offset(&mut state, FAKE_TAIL_PERIOD),
        });
    }
    spots
}

#[allow(clippy::cast_possible_truncation)]
fn scaled_width(width: f32, scale: f32) -> i32 {
    (width * scale).round() as i32
}

/// Whether a tail shows at this moment: it only does for a short while, so it has to be watched for.
pub fn tail_shows(tail: Tail, seconds: f32) -> bool {
    let period = if tail.fake {
        FAKE_TAIL_PERIOD
    } else {
        TAIL_PERIOD
    };
    (seconds + tail.offset) % period < TAIL_SHOWN
}

/// The angle of a thing at this moment, still for most of its period.
pub fn shaking(shake: Shake, seconds: f32) -> f32 {
    const KEYS: [(f32, f32); 5] = [
        (0.88, 0.0),
        (0.91, -2.6),
        (0.94, 2.2),
        (0.97, -1.2),
        (1.0, 0.0),
    ];
    let at = ((seconds - shake.delay).max(0.0) % shake.period) / shake.period;
    let mut before = KEYS[0];
    for key in KEYS {
        if at < key.0 {
            let along = ((at - before.0) / (key.0 - before.0)).max(0.0);
            return before.1 + (key.1 - before.1) * along;
        }
        before = key;
    }
    0.0
}

/// A share of the usual wait, from half of it to one and a half times, so that visits never come
/// like clockwork.
pub fn wait_before_visit(minutes: u32, seed: u64) -> u32 {
    let usual = u64::from(minutes) * 60_000;
    u32::try_from(usual / 2 + seed % (usual + 1)).unwrap_or(u32::MAX)
}

/// How a visit goes along the bottom of the screen, from the moment the mascot comes in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Walk {
    pub from: i32,
    pub to: i32,
    pub speed: f32,
}

impl Walk {
    /// Where the mascot is, and whether it has arrived.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn position(self, seconds: f32) -> (i32, bool) {
        let distance = (self.to - self.from) as f32;
        let travelled = self.speed * seconds;
        if travelled >= distance.abs() {
            (self.to, true)
        } else {
            (self.from + (distance.signum() * travelled) as i32, false)
        }
    }
}

/// It comes in from the right and stops in the right half of the screen, clear of the corner.
pub fn walk_in(work: Area, size: i32, speed: f32, chance: u64) -> Walk {
    let room = u64::try_from((work.width() / 2 - size).max(1)).unwrap_or(1);
    let to = work.right - size - i32::try_from(chance % room).unwrap_or(0);
    Walk {
        from: work.right,
        to,
        speed,
    }
}

/// It leaves the way it came.
pub fn walk_out(at: i32, work: Area, speed: f32) -> Walk {
    Walk {
        from: at,
        to: work.right,
        speed,
    }
}

/// `M:SS`, as the counter of hide and seek shows the time left.
pub fn clock(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Area = Area {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1032,
    };
    fn things(seed: u64) -> Vec<Spot> {
        lay_out(WORK, 1.0, 80, seed)
    }

    #[test]
    fn everything_stays_on_the_screen_under_the_counter() {
        for (work, scale) in [
            (WORK, 1.0),
            (WORK, 1.5),
            (
                Area {
                    left: -1280,
                    top: 40,
                    right: 0,
                    bottom: 760,
                },
                1.0,
            ),
        ] {
            for seed in 0..800 {
                for spot in lay_out(work, scale, 80, seed) {
                    assert!(
                        spot.x >= work.left && spot.x + spot.width <= work.right,
                        "{spot:?}"
                    );
                    assert!(
                        spot.y >= work.top + 80 && spot.y + spot.height() <= work.bottom,
                        "{spot:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn nothing_covers_anything_else() {
        for seed in 0..800 {
            let spots = things(seed);
            assert_eq!(spots.len(), THINGS.len());
            for (index, one) in spots.iter().enumerate() {
                for other in &spots[index + 1..] {
                    let apart = one.x + one.width <= other.x
                        || other.x + other.width <= one.x
                        || one.y + one.height() <= other.y
                        || other.y + other.height() <= one.y;
                    assert!(apart, "{one:?} {other:?}");
                }
            }
        }
    }

    #[test]
    fn it_hides_in_one_thing_never_a_cut_out_with_two_fake_tails_elsewhere() {
        for seed in 0..800 {
            let spots = things(seed);
            let hiding: Vec<_> = spots.iter().filter(|spot| spot.hides).collect();
            assert_eq!(hiding.len(), 1);
            assert_ne!(hiding[0].kind, Kind::Card);
            assert!(!hiding[0].tail.unwrap().fake);
            let fakes = spots
                .iter()
                .filter(|spot| spot.tail.is_some_and(|tail| tail.fake));
            assert_eq!(fakes.clone().count(), FAKE_TAILS);
            assert!(
                fakes
                    .clone()
                    .all(|spot| spot.kind != Kind::Card && !spot.hides)
            );
        }
    }

    #[test]
    fn it_can_hide_in_every_kind_of_thing() {
        let kinds: Vec<Kind> = (0..400)
            .flat_map(|seed| things(seed).into_iter().filter(|spot| spot.hides))
            .map(|spot| spot.kind)
            .collect();
        for kind in [Kind::Box, Kind::Plant, Kind::Basket, Kind::Cushion] {
            assert!(kinds.contains(&kind), "{kind:?}");
        }
    }

    #[test]
    fn a_tiny_screen_makes_the_things_smaller_rather_than_on_top_of_each_other() {
        let tiny = Area {
            left: 0,
            top: 0,
            right: 640,
            bottom: 400,
        };
        for spot in lay_out(tiny, 2.0, 80, 7) {
            assert!(spot.width <= 80, "{spot:?}");
        }
    }

    #[test]
    fn a_tail_shows_only_for_a_while() {
        let tail = Tail {
            fake: false,
            offset: 0.0,
        };
        assert!(tail_shows(tail, 0.5));
        assert!(!tail_shows(tail, 3.0));
        assert!(tail_shows(tail, TAIL_PERIOD + 0.5));
    }

    #[test]
    fn a_thing_stays_still_most_of_the_time() {
        let shake = Shake {
            period: 10.0,
            delay: 0.0,
        };
        assert_eq!(shaking(shake, 0.0), 0.0);
        assert_eq!(shaking(shake, 8.0), 0.0);
        assert!((shaking(shake, 9.1) + 2.6).abs() < 0.01);
        assert!((shaking(shake, 9.4) - 2.2).abs() < 0.01);
        assert_eq!(shaking(shake, 10.0), 0.0);
    }

    #[test]
    fn visits_keep_to_their_frequency() {
        assert_eq!(wait_before_visit(10, 0), 300_000);
        assert_eq!(wait_before_visit(10, 600_000), 900_000);
        for seed in [1, 77, 1_000_000, u64::MAX] {
            let wait = wait_before_visit(45, seed);
            assert!((1_350_000..=4_050_000).contains(&wait), "{wait}");
        }
    }

    #[test]
    fn walks_in_and_stops_in_the_right_half() {
        for seed in 0..2000 {
            let walk = walk_in(WORK, 225, 90.0, seed);
            assert_eq!(walk.from, WORK.right);
            assert!(
                walk.to >= WORK.right / 2 && walk.to + 225 <= WORK.right,
                "{walk:?}"
            );
        }
    }

    #[test]
    fn walking_ends_where_it_was_going() {
        let walk = walk_in(WORK, 225, 100.0, 0);
        assert_eq!(walk.position(0.0), (WORK.right, false));
        assert_eq!(walk.position(1.0), (WORK.right - 100, false));
        assert_eq!(walk.position(60.0), (walk.to, true));
        let out = walk_out(walk.to, WORK, 100.0);
        assert_eq!(out.position(1.0), (walk.to + 100, false));
        assert_eq!(out.position(60.0), (WORK.right, true));
    }

    #[test]
    fn the_counter_reads_like_a_clock() {
        assert_eq!(clock(12), "0:12");
        assert_eq!(clock(60), "1:00");
    }
}
