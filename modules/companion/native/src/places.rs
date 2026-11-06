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

/// A hiding place: where the mascot's window goes, and which part of it stays hidden, from the
/// left or from the right, as if something covered it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hiding {
    pub x: i32,
    pub y: i32,
    pub shown: i32,
    pub hidden_on_left: bool,
}

/// Small windows make poor hiding places, and most are not what they seem.
const SMALLEST_WINDOW: i32 = 200;

/// As much of the mascot as can be seen once hidden, out of its size.
const fn shown_part(size: i32) -> i32 {
    size * 7 / 10
}

fn at_window_sides(window: Area, work: Area, size: i32, height_at: u64) -> Vec<Hiding> {
    let shown = shown_part(size);
    if window.width() < SMALLEST_WINDOW || window.height() < SMALLEST_WINDOW {
        return Vec::new();
    }
    let lowest = (window.bottom.min(work.bottom) - size).max(work.top);
    let highest = window.top.max(work.top);
    let room = u64::try_from((lowest - highest).max(0)).unwrap_or(0);
    let y = highest + i32::try_from(height_at % (room + 1)).unwrap_or(0);
    let mut places = Vec::new();
    if window.left - shown >= work.left {
        places.push(Hiding {
            x: window.left - shown,
            y,
            shown,
            hidden_on_left: false,
        });
    }
    if window.right + shown <= work.right {
        places.push(Hiding {
            x: window.right,
            y,
            shown,
            hidden_on_left: true,
        });
    }
    places
}

fn at_screen_sides(work: Area, size: i32, height_at: u64) -> [Hiding; 2] {
    let shown = shown_part(size);
    let room = u64::try_from((work.height() - size).max(0)).unwrap_or(0);
    let y = work.top + i32::try_from(height_at % (room + 1)).unwrap_or(0);
    [
        Hiding {
            x: work.left,
            y,
            shown,
            hidden_on_left: true,
        },
        Hiding {
            x: work.right - shown,
            y,
            shown,
            hidden_on_left: false,
        },
    ]
}

/// Behind one of the open windows, or at an edge of the screen. The edges are always possible, so
/// that the game works on an empty desktop.
pub fn hiding_place(windows: &[Area], work: Area, size: i32, seed: u64) -> Hiding {
    let mut places: Vec<Hiding> = windows
        .iter()
        .flat_map(|window| at_window_sides(*window, work, size, seed / 7))
        .collect();
    places.extend(at_screen_sides(work, size, seed / 7));
    let count = places.len() as u64;
    places[usize::try_from(seed % count).unwrap_or(0)]
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

/// `M:SS`, as the counter of hide and seek shows it.
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
    const WINDOW: Area = Area {
        left: 500,
        top: 200,
        right: 1400,
        bottom: 800,
    };

    fn on_screen(place: Hiding, size: i32) {
        assert!(
            place.y >= WORK.top && place.y + size <= WORK.bottom,
            "{place:?}"
        );
        assert!(
            place.x >= WORK.left && place.x + place.shown <= WORK.right,
            "{place:?}"
        );
    }

    #[test]
    fn hides_on_the_side_of_a_window() {
        let places = at_window_sides(WINDOW, WORK, 110, 40);
        assert_eq!(places.len(), 2);
        assert_eq!(places[0].x + places[0].shown, WINDOW.left);
        assert!(!places[0].hidden_on_left);
        assert_eq!(places[1].x, WINDOW.right);
        assert!(places[1].hidden_on_left);
        assert!(
            places
                .iter()
                .all(|place| place.y >= WINDOW.top && place.y + 110 <= WINDOW.bottom)
        );
    }

    #[test]
    fn a_window_that_fills_the_screen_has_no_side_to_hide_behind() {
        assert_eq!(at_window_sides(WORK, WORK, 110, 0), Vec::new());
    }

    #[test]
    fn a_tiny_window_is_no_hiding_place() {
        let tiny = Area {
            left: 300,
            top: 300,
            right: 400,
            bottom: 400,
        };
        assert_eq!(at_window_sides(tiny, WORK, 110, 0), Vec::new());
    }

    #[test]
    fn a_window_partly_off_the_screen_keeps_the_mascot_on_it() {
        let low = Area {
            left: 500,
            top: 900,
            right: 1400,
            bottom: 1400,
        };
        for seed in 0..500 {
            for place in at_window_sides(low, WORK, 110, seed) {
                on_screen(place, 110);
            }
        }
    }

    #[test]
    fn an_empty_desktop_still_has_hiding_places() {
        for seed in 0..500 {
            on_screen(hiding_place(&[], WORK, 110, seed), 110);
        }
    }

    #[test]
    fn every_place_can_come_up() {
        let places: Vec<_> = (0..400)
            .map(|seed| hiding_place(&[WINDOW], WORK, 110, seed).hidden_on_left)
            .collect();
        assert!(places.contains(&true) && places.contains(&false));
        assert!((0..400).any(|seed| hiding_place(&[WINDOW], WORK, 110, seed).x == WINDOW.right));
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
