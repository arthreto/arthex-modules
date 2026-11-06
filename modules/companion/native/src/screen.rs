//! The mascot on the screen: its visits, and the game of hide and seek. Its windows never take the
//! focus and draw nothing while it is away, so that it costs nothing between two visits.
#![allow(clippy::cast_precision_loss)]

use std::{
    cell::RefCell,
    ptr,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicIsize, Ordering},
        mpsc::{Sender, channel},
    },
    thread,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
    Graphics::Gdi::{
        CreateFontW, DT_CENTER, DT_END_ELLIPSIS, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
        DeleteObject, DrawTextW, FW_MEDIUM, FW_SEMIBOLD, GetMonitorInfoW, GetTextExtentPoint32W,
        HFONT, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, MonitorFromWindow,
        SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
    },
    UI::{
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext},
        Shell::{QUNS_ACCEPTS_NOTIFICATIONS, SHQueryUserNotificationState},
        WindowsAndMessaging::{
            DefWindowProcW, DestroyWindow, DispatchMessageW, GetClassNameW, GetCursorPos,
            GetForegroundWindow, GetMessageW, GetWindowRect, HCURSOR, HWND_TOPMOST, IDC_ARROW,
            IDC_HAND, KillTimer, LoadCursorW, MSG, PostMessageW, RegisterClassW, SW_HIDE,
            SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetCursor, SetTimer,
            SetWindowPos, ShowWindow, WM_APP, WM_LBUTTONUP, WM_POWERBROADCAST, WM_SETCURSOR,
            WM_TIMER, WNDCLASSW,
        },
    },
};

use super::{
    Shared,
    canvas::{Canvas, create_window, main_screen_scale, module, scaled, wide},
    draw::{
        Mood, Sticking, THING_ROOM, ThingLook, render, render_card, render_fallen_card,
        render_thing, window_size,
    },
    places::{
        Area, Kind, Spot, Walk, clock, lay_out, shaking, tail_shows, wait_before_visit, walk_in,
        walk_out,
    },
    progress::{Game, Progress, Species},
};

const WAKE: u32 = WM_APP + 1;
const HIDE: u32 = WM_APP + 2;
const WAIT_TIMER: usize = 1;
const FRAME_TIMER: usize = 2;
const FRAME_MILLISECONDS: u32 = 40;
const TRY_AGAIN_MILLISECONDS: u32 = 300_000;
const VISIT_SIZE: f32 = 150.0;
const WALK_SPEED: f32 = 90.0;
const STAY_SECONDS: f32 = 25.0;
const HAPPY_SECONDS: f32 = 1.2;
const HIDE_SECONDS: u32 = 60;
const HIDE_TRIES: u32 = 4;
/// How long the mascot stays out at the end of the game, before Arthex comes back on its own.
const ENDING_SECONDS: f32 = 2.0;
/// Where nothing is put, as the counter is there.
const HIDE_TOP_ROOM: f32 = 84.0;
const OPENED_OPACITY: f32 = 0.35;
/// Its drawing of 140 units wide, as `places` knows the width of a thing.
const THING_UNITS: f32 = 140.0;
/// What sticks out of a narrow pot has to be closer to it.
const PLANT_SHIFT: f32 = -22.0;
/// Its feet are above the bottom of its window: this puts them on the taskbar.
const FEET_ROOM: f32 = 20.0;
/// Asking Windows if the moment is good costs more than a frame.
const FRAMES_BETWEEN_CHECKS: u32 = 25;
const TOP_MARGIN: f32 = 20.0;
const PANEL_HEIGHT: f32 = 46.0;
const PANEL_RADIUS: f32 = 14.0;
const PANEL_PADDING: f32 = 14.0;
const PANEL_GAP: f32 = 8.0;
const BUTTON_HEIGHT: f32 = 30.0;
const BUTTON_PADDING: f32 = 12.0;
const BUTTON_RADIUS: f32 = 10.0;
const CLOSE_WIDTH: f32 = 32.0;
const CLOSE_ICON: f32 = 5.0;
const TEXT_SIZE: f32 = 13.5;
const COUNTER_SIZE: f32 = 14.0;
/// The panel, its border and the primary button of `src/ui/tokens.css`, as blue, green and red.
const PANEL: [u8; 3] = [32, 26, 24];
const PANEL_ALPHA: f32 = 0.92;
const BORDER: [u8; 3] = [68, 62, 60];
const ACCENT: [u8; 3] = [255, 194, 76];
/// As Windows colours: 0x00BBGGRR.
const TEXT: u32 = 0x00F6_F4F3;
const ON_ACCENT: u32 = 0x002C_2005;
const COUNTER: u32 = 0x0074_BAFD;
const MUTED: u32 = 0x00D4_CDC9;
const XP: u32 = 0x00FF_C24C;
const CLOSE_LINE: [u8; 3] = [194, 185, 180];

/// How another thread reaches the screen's.
#[derive(Default)]
pub struct Remote {
    window: AtomicIsize,
    open_asked: AtomicBool,
    finder: Mutex<Option<Sender<bool>>>,
}

impl Remote {
    fn post(&self, message: u32) -> Result<(), String> {
        let window = self.window.load(Ordering::Relaxed) as HWND;
        // SAFETY: only posts a message; a window that is gone makes it fail harmlessly.
        if window.is_null() || unsafe { PostMessageW(window, message, 0, 0) } == 0 {
            return Err("the screen is not ready".to_string());
        }
        Ok(())
    }

    /// The settings or the mascot changed: the next visit is looked at again.
    pub fn changed(&self) {
        let _ = self.post(WAKE);
    }

    /// Plays until the mascot is found or the time is up, and says which.
    pub fn hide_and_seek(&self) -> Result<bool, String> {
        let (sender, receiver) = channel();
        *self.finder.lock().unwrap_or_else(PoisonError::into_inner) = Some(sender);
        self.post(HIDE)?;
        receiver.recv().map_err(|error| error.to_string())
    }

    pub fn take_open(&self) -> bool {
        self.open_asked.swap(false, Ordering::Relaxed)
    }

    fn found(&self, found: bool) {
        if let Some(sender) = self
            .finder
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            let _ = sender.send(found);
        }
    }
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| {
            time.as_secs().wrapping_mul(0x9E37_79B9) ^ u64::from(time.subsec_nanos())
        })
}

fn cursor() -> POINT {
    let mut position = POINT { x: 0, y: 0 };
    // SAFETY: writes into a local.
    unsafe { GetCursorPos(&raw mut position) };
    position
}

fn area_of(rect: RECT) -> Area {
    Area {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}

fn monitor_info(monitor: *mut std::ffi::c_void) -> MONITORINFO {
    let empty = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut info = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or_default(),
        rcMonitor: empty,
        rcWork: empty,
        dwFlags: 0,
    };
    // SAFETY: `info` is a local with its size set; a monitor that is gone only leaves it empty.
    unsafe { GetMonitorInfoW(monitor, &raw mut info) };
    info
}

fn work_area() -> Area {
    // SAFETY: asks for the monitor under the cursor, or the nearest one, which always exists.
    let monitor = unsafe { MonitorFromPoint(cursor(), MONITOR_DEFAULTTONEAREST) };
    area_of(monitor_info(monitor).rcWork)
}

/// The desktop fills the screen too, but it is the best moment of all.
fn is_desktop(window: HWND) -> bool {
    let mut class = [0_u16; 16];
    // SAFETY: the buffer outlives the call, and its size is given.
    let length = unsafe { GetClassNameW(window, class.as_mut_ptr(), 16) };
    let name = String::from_utf16_lossy(&class[..usize::try_from(length).unwrap_or(0)]);
    name == "Progman" || name == "WorkerW"
}

/// In a game, a film or a presentation, or under the overlay of Arthex itself: the moment is not
/// the mascot's. Windows knows the first ones; the second has a window that fills the screen.
fn someone_is_busy() -> bool {
    let mut state = 0;
    // SAFETY: writes into a local.
    let answered = unsafe { SHQueryUserNotificationState(&raw mut state) } == 0;
    if answered && state != QUNS_ACCEPTS_NOTIFICATIONS {
        return true;
    }
    // SAFETY: only reads about a window, which may be gone: the calls then fail harmlessly.
    unsafe {
        let window = GetForegroundWindow();
        if window.is_null() || is_desktop(window) {
            return false;
        }
        let mut rect = std::mem::zeroed();
        GetWindowRect(window, &raw mut rect);
        let screen = monitor_info(MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST)).rcMonitor;
        rect.left <= screen.left
            && rect.top <= screen.top
            && rect.right >= screen.right
            && rect.bottom >= screen.bottom
    }
}

fn font(size: f32, weight: u32, scale: f32) -> HFONT {
    let name = wide("Segoe UI Variable Text");
    // SAFETY: the name outlives the call; a missing font falls back to a system one.
    unsafe {
        CreateFontW(
            -scaled(size, scale),
            0,
            0,
            0,
            weight.cast_signed(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            name.as_ptr(),
        )
    }
}

fn text_width(font: HFONT, text: &str) -> i32 {
    let characters: Vec<u16> = text.encode_utf16().collect();
    let (Some(probe), Ok(length)) = (Canvas::new(1, 1), i32::try_from(characters.len())) else {
        return 0;
    };
    let mut size = SIZE { cx: 0, cy: 0 };
    // SAFETY: the context and font live through this block, and the text outlives the call.
    unsafe {
        let previous = SelectObject(probe.context, font);
        GetTextExtentPoint32W(probe.context, characters.as_ptr(), length, &raw mut size);
        SelectObject(probe.context, previous);
    }
    size.cx
}

fn write(canvas: &Canvas, font: HFONT, color: u32, text: &str, area: RECT, flags: u32) {
    let characters: Vec<u16> = text.encode_utf16().collect();
    let Ok(length) = i32::try_from(characters.len()) else {
        return;
    };
    let mut area = area;
    // SAFETY: the context and font live through this block, and the text and rectangle outlive
    // the call; these flags never make it write in the text.
    unsafe {
        let previous = SelectObject(canvas.context, font);
        SetBkMode(canvas.context, TRANSPARENT.cast_signed());
        SetTextColor(canvas.context, color);
        DrawTextW(
            canvas.context,
            characters.as_ptr(),
            length,
            &raw mut area,
            flags | DT_SINGLELINE | DT_NOPREFIX | DT_VCENTER,
        );
        SelectObject(canvas.context, previous);
    }
}

fn rounded_inside(x: f32, y: f32, rect: (f32, f32, f32, f32), radius: f32) -> f32 {
    let (left, top, right, bottom) = rect;
    let dx = (left + radius - x).max(x - (right - radius)).max(0.0);
    let dy = (top + radius - y).max(y - (bottom - radius)).max(0.0);
    radius - dx.hypot(dy)
}

fn blend(under: u8, over: u8, amount: f32) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = (f32::from(under) * (1.0 - amount) + f32::from(over) * amount).round() as u8;
    value
}

/// A rounded rectangle in opaque colours, before `finish_panel` makes the whole panel translucent.
fn fill_rounded(canvas: &mut Canvas, rect: (f32, f32, f32, f32), radius: f32, color: [u8; 3]) {
    let width = canvas.width.cast_unsigned();
    for (index, pixel) in canvas
        .pixels_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let index = u32::try_from(index).unwrap_or(0);
        let (x, y) = ((index % width) as f32 + 0.5, (index / width) as f32 + 0.5);
        let amount = (rounded_inside(x, y, rect, radius) + 0.5).clamp(0.0, 1.0);
        if amount > 0.0 {
            for channel in 0..3 {
                pixel[channel] = blend(pixel[channel], color[channel], amount);
            }
        }
    }
}

fn draw_close(canvas: &mut Canvas, center: (f32, f32), half: f32) {
    let width = canvas.width.cast_unsigned();
    for (index, pixel) in canvas
        .pixels_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let index = u32::try_from(index).unwrap_or(0);
        let (x, y) = (
            (index % width) as f32 + 0.5 - center.0,
            (index / width) as f32 + 0.5 - center.1,
        );
        if x.abs() > half + 1.0 || y.abs() > half + 1.0 {
            continue;
        }
        let distance = ((x - y).abs().min((x + y).abs())) / std::f32::consts::SQRT_2;
        let amount = (1.4 - distance).clamp(0.0, 1.0);
        for channel in 0..3 {
            pixel[channel] = blend(pixel[channel], CLOSE_LINE[channel], amount);
        }
    }
}

/// Text drawn by Windows leaves the alpha alone: everything gets the panel's here, once drawn.
fn finish_panel(canvas: &mut Canvas, scale: f32) {
    let (width, height) = (canvas.width.cast_unsigned(), canvas.height.cast_unsigned());
    let rect = (0.0, 0.0, width as f32, height as f32);
    for (index, pixel) in canvas
        .pixels_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let index = u32::try_from(index).unwrap_or(0);
        let (x, y) = ((index % width) as f32 + 0.5, (index / width) as f32 + 0.5);
        let alpha =
            (rounded_inside(x, y, rect, PANEL_RADIUS * scale) + 0.5).clamp(0.0, 1.0) * PANEL_ALPHA;
        for channel in &mut pixel[..3] {
            *channel = blend(0, *channel, alpha);
        }
        pixel[3] = blend(0, 255, alpha);
    }
}

/// The base of a panel of this width: its border, then its surface.
fn panel_base(width: i32, scale: f32) -> Option<Canvas> {
    let height = scaled(PANEL_HEIGHT, scale);
    let mut canvas = Canvas::new(width, height)?;
    let (right, bottom) = (width as f32, height as f32);
    fill_rounded(
        &mut canvas,
        (0.0, 0.0, right, bottom),
        PANEL_RADIUS * scale,
        BORDER,
    );
    let edge = scale.max(1.0);
    fill_rounded(
        &mut canvas,
        (edge, edge, right - edge, bottom - edge),
        (PANEL_RADIUS * scale - edge).max(0.0),
        PANEL,
    );
    Some(canvas)
}

#[derive(Clone, Copy)]
struct Texts {
    french: bool,
}

impl Texts {
    const fn shall_we_play(self) -> &'static str {
        if self.french {
            "On joue ?"
        } else {
            "Shall we play?"
        }
    }

    const fn play(self) -> &'static str {
        if self.french { "Jouer" } else { "Play" }
    }

    fn find(self, name: &str) -> String {
        if self.french {
            format!("Trouve {name}")
        } else {
            format!("Find {name}")
        }
    }

    fn tries(self, count: u32) -> String {
        match (self.french, count) {
            (true, 0 | 1) => format!("{count} essai"),
            (true, _) => format!("{count} essais"),
            (false, 1) => "1 try".to_string(),
            (false, _) => format!("{count} tries"),
        }
    }

    const fn missed(self) -> &'static str {
        if self.french { "Raté" } else { "Missed" }
    }

    fn kibble(self, count: u32) -> String {
        if self.french {
            format!("+{count} croquettes")
        } else {
            format!("+{count} kibble")
        }
    }
}

/// A panel with the button that is on it, and where to click.
struct Panel {
    canvas: Canvas,
    button: (i32, i32),
    close: i32,
}

fn bubble(texts: Texts, scale: f32) -> Option<Panel> {
    let (text_font, button_font) = (
        font(TEXT_SIZE, FW_MEDIUM, scale),
        font(13.0, FW_MEDIUM, scale),
    );
    let text_width_px = text_width(text_font, texts.shall_we_play());
    let button_width = text_width(button_font, texts.play()) + 2 * scaled(BUTTON_PADDING, scale);
    let padding = scaled(PANEL_PADDING, scale);
    let gap = scaled(PANEL_GAP, scale);
    let close_width = scaled(CLOSE_WIDTH, scale);
    let width = padding + text_width_px + gap + button_width + gap / 2 + close_width + gap / 2;
    let panel = panel_base(width, scale).map(|mut canvas| {
        let height = canvas.height;
        let button_left = padding + text_width_px + gap;
        let button_right = button_left + button_width;
        let button_top = (height - scaled(BUTTON_HEIGHT, scale)) / 2;
        fill_rounded(
            &mut canvas,
            (
                button_left as f32,
                button_top as f32,
                button_right as f32,
                (button_top + scaled(BUTTON_HEIGHT, scale)) as f32,
            ),
            BUTTON_RADIUS * scale,
            ACCENT,
        );
        let close_left = button_right + gap / 2;
        draw_close(
            &mut canvas,
            ((close_left + close_width / 2) as f32, height as f32 / 2.0),
            CLOSE_ICON * scale,
        );
        write(
            &canvas,
            text_font,
            TEXT,
            texts.shall_we_play(),
            RECT {
                left: padding,
                top: 0,
                right: button_left,
                bottom: height,
            },
            DT_END_ELLIPSIS,
        );
        write(
            &canvas,
            button_font,
            ON_ACCENT,
            texts.play(),
            RECT {
                left: button_left,
                top: 0,
                right: button_right,
                bottom: height,
            },
            DT_CENTER,
        );
        finish_panel(&mut canvas, scale);
        Panel {
            canvas,
            button: (button_left, button_right),
            close: close_left,
        }
    });
    // SAFETY: created above, and no longer selected in any context.
    unsafe {
        DeleteObject(text_font);
        DeleteObject(button_font);
    }
    panel
}

/// A line of coloured texts, with a close button at the end if it asks for one.
fn text_panel(texts: &[(&str, u32)], with_close: bool, scale: f32) -> Option<Panel> {
    let bold = font(COUNTER_SIZE, FW_SEMIBOLD, scale);
    let padding = scaled(PANEL_PADDING, scale);
    let gap = scaled(12.0, scale);
    let widths: Vec<i32> = texts
        .iter()
        .map(|(text, _)| text_width(bold, text))
        .collect();
    let line = widths.iter().sum::<i32>() + gap * (i32::try_from(texts.len()).unwrap_or(1) - 1);
    let close_width = if with_close {
        scaled(CLOSE_WIDTH, scale)
    } else {
        0
    };
    let width = padding
        + line
        + if with_close {
            gap / 2 + close_width
        } else {
            padding
        };
    let panel = panel_base(width, scale).map(|mut canvas| {
        let height = canvas.height;
        let mut left = padding;
        for ((text, color), text_width) in texts.iter().zip(&widths) {
            write(
                &canvas,
                bold,
                *color,
                text,
                RECT {
                    left,
                    top: 0,
                    right: left + text_width,
                    bottom: height,
                },
                0,
            );
            left += text_width + gap;
        }
        let close = width - close_width;
        if with_close {
            draw_close(
                &mut canvas,
                ((close + close_width / 2) as f32, height as f32 / 2.0),
                CLOSE_ICON * scale,
            );
        }
        finish_panel(&mut canvas, scale);
        Panel {
            canvas,
            button: (0, 0),
            close,
        }
    });
    // SAFETY: created above, and no longer selected in any context.
    unsafe { DeleteObject(bold) };
    panel
}

/// A drawing as a bitmap for a window.
fn canvas_of(pixmap: &tiny_skia::Pixmap) -> Option<Canvas> {
    let mut canvas = Canvas::new(
        i32::try_from(pixmap.width()).ok()?,
        i32::try_from(pixmap.height()).ok()?,
    )?;
    for (to, from) in canvas
        .pixels_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(pixmap.data().as_chunks::<4>().0)
    {
        // A Windows bitmap holds blue, green, red and alpha, both premultiplied.
        *to = [from[2], from[1], from[0], from[3]];
    }
    Some(canvas)
}

/// The drawn mascot, as the bitmap of its window.
fn present_mascot(window: HWND, look: &Look, mood: Mood, seconds: f32, x: i32, y: i32) {
    let pixmap = render(
        look.species,
        mood,
        look.wear.as_deref(),
        seconds,
        look.size.cast_unsigned(),
    );
    if let Some(canvas) = pixmap.as_ref().and_then(canvas_of) {
        canvas.present(window, x, y);
    }
}

/// What the mascot looks like at the moment.
#[derive(Clone)]
struct Look {
    species: Species,
    wear: Option<String>,
    name: String,
    size: i32,
}

fn look_of(progress: &Progress, size: i32) -> Option<Look> {
    Some(Look {
        species: progress.species?,
        wear: progress.wear.clone(),
        name: progress.name.clone(),
        size,
    })
}

enum Step {
    Coming(Walk),
    Staying { x: i32, until: f32 },
    Leaving(Walk),
}

struct Visit {
    look: Look,
    work: Area,
    started: Instant,
    step: Step,
    happy_until: f32,
    y: i32,
    panel: Option<Panel>,
}

/// A thing on the screen, in its own window: only what shakes or changes is ever drawn again.
struct Thing {
    window: HWND,
    spot: Spot,
    drawn: Option<(i32, Option<Sticking>, bool, bool)>,
}

impl Thing {
    fn sticking(&self, seconds: f32, reduced: bool) -> Option<Sticking> {
        let tail = self.spot.tail.filter(|_| !self.spot.opened)?;
        (reduced || tail_shows(tail, seconds)).then_some(if tail.fake {
            Sticking::Fake
        } else {
            Sticking::Tail
        })
    }

    fn present(&mut self, species: Species, seconds: f32, reduced: bool, out: bool) {
        if self.spot.kind == Kind::Card {
            self.present_card(species);
            return;
        }
        let sticking = if out {
            None
        } else {
            self.sticking(seconds, reduced)
        };
        let shake = self
            .spot
            .shake
            .filter(|_| !reduced && !out && !self.spot.opened);
        let angle = shake.map_or(0.0, |shake| shaking(shake, seconds));
        #[allow(clippy::cast_possible_truncation)]
        let state = ((angle * 10.0) as i32, sticking, self.spot.opened, out);
        if self.drawn == Some(state) {
            return;
        }
        self.drawn = Some(state);
        let art = match self.spot.kind {
            Kind::Box if self.spot.opened || out => "open",
            Kind::Box => "box",
            Kind::Plant => "plant",
            Kind::Basket => "basket",
            Kind::Cushion | Kind::Card => "cushion",
        };
        let look = ThingLook {
            art,
            sticking,
            sticking_shift: if self.spot.kind == Kind::Plant {
                PLANT_SHIFT
            } else {
                0.0
            },
            angle,
            opacity: if self.spot.opened {
                OPENED_OPACITY
            } else {
                1.0
            },
        };
        let room = scaled_room(self.spot.width);
        if let Some(canvas) = render_thing(species, &look, self.spot.width)
            .as_ref()
            .and_then(canvas_of)
        {
            canvas.present(self.window, self.spot.x - room, self.spot.y - room);
        }
    }

    /// Standing, or knocked over on its back once clicked.
    fn present_card(&mut self, species: Species) {
        let state = (0, None, self.spot.opened, false);
        if self.drawn == Some(state) {
            return;
        }
        self.drawn = Some(state);
        let size = self.spot.width.cast_unsigned();
        let (pixmap, x) = if self.spot.opened {
            (
                render_fallen_card(species, size),
                self.spot.x - self.spot.width / 2,
            )
        } else {
            (render_card(species, size), self.spot.x)
        };
        if let Some(canvas) = pixmap.as_ref().and_then(canvas_of) {
            canvas.present(self.window, x, self.spot.y);
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn scaled_room(width: i32) -> i32 {
    (width as f32 / THING_UNITS * THING_ROOM).round() as i32
}

struct Ending {
    found: bool,
    started: Instant,
    x: i32,
    y: i32,
}

struct Hunt {
    look: Look,
    things: Vec<Thing>,
    work: Area,
    started: Instant,
    shown_seconds: Option<u32>,
    tries: u32,
    ending: Option<Ending>,
    panel: Option<Panel>,
}

impl Drop for Hunt {
    fn drop(&mut self) {
        for thing in &self.things {
            // SAFETY: our own window, used by nothing else once its game is over.
            unsafe { DestroyWindow(thing.window) };
        }
    }
}

struct Screen {
    shared: Arc<Shared>,
    texts: Texts,
    /// Hidden, and only there to wait: top-level windows hear when the computer wakes up, and a
    /// timer needs no polling.
    waiter: HWND,
    mascot: HWND,
    panel: HWND,
    class: Vec<u16>,
    hand: HCURSOR,
    arrow: HCURSOR,
    scale: f32,
    visit: Option<Visit>,
    hunt: Option<Hunt>,
    frames_drawn: u32,
}

thread_local! {
    static SCREEN: RefCell<Option<Screen>> = const { RefCell::new(None) };
}

impl Screen {
    /// Waits for the next visit, without looking in the meantime.
    fn plan(&self, milliseconds: Option<u32>) {
        let progress = self.shared.progress();
        let wanted = progress.on_screen && progress.species.is_some();
        let minutes = progress.frequency.minutes();
        drop(progress);
        // SAFETY: only starts or stops our own timer.
        unsafe {
            if wanted {
                let wait = milliseconds.unwrap_or_else(|| wait_before_visit(minutes, seed()));
                SetTimer(self.waiter, WAIT_TIMER, wait, None);
            } else {
                KillTimer(self.waiter, WAIT_TIMER);
            }
        }
    }

    fn reduced_motion(&self) -> bool {
        self.shared.progress().reduced_motion
    }

    fn frames(&self, on: bool) {
        // SAFETY: only starts or stops our own timer.
        unsafe {
            if on && !self.reduced_motion() {
                SetTimer(self.waiter, FRAME_TIMER, FRAME_MILLISECONDS, None);
            } else {
                KillTimer(self.waiter, FRAME_TIMER);
            }
        }
    }

    fn hide_windows(&self) {
        // SAFETY: our own windows.
        unsafe {
            ShowWindow(self.mascot, SW_HIDE);
            ShowWindow(self.panel, SW_HIDE);
        }
    }

    fn begin_visit(&mut self) {
        if self.visit.is_some() || self.hunt.is_some() {
            return;
        }
        if someone_is_busy() {
            self.plan(Some(TRY_AGAIN_MILLISECONDS));
            return;
        }
        let size = window_size(scaled(VISIT_SIZE, self.scale));
        let Some(look) = look_of(&self.shared.progress(), size) else {
            return;
        };
        let work = work_area();
        let walk = walk_in(work, size, WALK_SPEED * self.scale, seed());
        let reduced = self.reduced_motion();
        let step = if reduced {
            Step::Staying {
                x: walk.to,
                until: STAY_SECONDS,
            }
        } else {
            Step::Coming(walk)
        };
        self.visit = Some(Visit {
            look,
            work,
            started: Instant::now(),
            step,
            happy_until: 0.0,
            y: work.bottom - size + scaled(FEET_ROOM, self.scale),
            panel: None,
        });
        // SAFETY: shows our own window without taking the focus.
        unsafe { ShowWindow(self.mascot, SW_SHOWNOACTIVATE) };
        if reduced {
            // With no frames, nothing else ends the visit: the timer is only started to end it.
            // SAFETY: only starts our own timer.
            unsafe { SetTimer(self.waiter, FRAME_TIMER, 1000, None) };
        } else {
            self.frames(true);
        }
        self.frame();
    }

    fn end_visit(&mut self) {
        self.visit = None;
        self.frames(false);
        self.hide_windows();
        self.plan(None);
    }

    /// Takes the mascot out by the way it came.
    fn send_away(&mut self) {
        let reduced = self.reduced_motion();
        let scale = self.scale;
        let Some(visit) = &mut self.visit else {
            return;
        };
        let Step::Staying { x, .. } = visit.step else {
            return;
        };
        visit.panel = None;
        // SAFETY: our own window.
        unsafe { ShowWindow(self.panel, SW_HIDE) };
        if reduced {
            self.end_visit();
            return;
        }
        visit.started = Instant::now();
        visit.happy_until = 0.0;
        visit.step = Step::Leaving(walk_out(x, visit.work, WALK_SPEED * scale));
    }

    fn show_bubble(&mut self, x: i32) {
        let panel = bubble(self.texts, self.scale);
        let margin = scaled(30.0, self.scale);
        let Some(visit) = &mut self.visit else {
            return;
        };
        if let Some(panel) = panel {
            let left = (x + visit.look.size - margin - panel.canvas.width).max(visit.work.left);
            panel.canvas.present(
                self.panel,
                left,
                visit.y - panel.canvas.height + scaled(6.0, self.scale),
            );
            visit.panel = Some(panel);
            // SAFETY: shows our own window without taking the focus.
            unsafe { ShowWindow(self.panel, SW_SHOWNOACTIVATE) };
        }
    }

    fn visit_frame(&mut self) {
        self.frames_drawn = self.frames_drawn.wrapping_add(1);
        if self.frames_drawn.is_multiple_of(FRAMES_BETWEEN_CHECKS) && someone_is_busy() {
            self.end_visit();
            return;
        }
        let Some(visit) = &mut self.visit else {
            return;
        };
        let seconds = visit.started.elapsed().as_secs_f32();
        let reduced = self.shared.progress().reduced_motion;
        let (x, mood, finished, arrived) = match visit.step {
            Step::Coming(walk) => {
                let (x, done) = walk.position(seconds);
                (x, Mood::Walk, false, done.then_some(x))
            }
            Step::Staying { x, until } => {
                let mood = if seconds < visit.happy_until {
                    Mood::Happy
                } else {
                    Mood::Idle
                };
                (x, mood, seconds >= until, None)
            }
            Step::Leaving(walk) => {
                let (x, done) = walk.position(seconds);
                (x, Mood::Walk, done, None)
            }
        };
        let frame_seconds = if reduced { 0.0 } else { seconds };
        present_mascot(self.mascot, &visit.look, mood, frame_seconds, x, visit.y);
        if let Some(x) = arrived {
            visit.started = Instant::now();
            visit.step = Step::Staying {
                x,
                until: STAY_SECONDS,
            };
            self.show_bubble(x);
        } else if finished {
            if matches!(
                self.visit.as_ref().map(|visit| &visit.step),
                Some(Step::Leaving(_))
            ) {
                self.end_visit();
            } else {
                self.send_away();
            }
        } else if reduced
            && let Some(visit) = &self.visit
            && visit.panel.is_none()
            && let Step::Staying { x, .. } = visit.step
        {
            self.show_bubble(x);
        }
    }

    fn begin_hunt(&mut self) {
        if self.visit.is_some() {
            self.end_visit();
        }
        let size = window_size(scaled(VISIT_SIZE, self.scale));
        let Some(look) = look_of(&self.shared.progress(), size) else {
            self.shared.screen.found(false);
            return;
        };
        let work = work_area();
        let top = scaled(HIDE_TOP_ROOM, self.scale);
        let things: Vec<Thing> = lay_out(work, self.scale, top, seed())
            .into_iter()
            .map(|spot| Thing {
                window: create_window(self.class.as_ptr()),
                spot,
                drawn: None,
            })
            .collect();
        let complete = things.iter().all(|thing| !thing.window.is_null());
        self.hunt = Some(Hunt {
            look,
            things,
            work,
            started: Instant::now(),
            shown_seconds: None,
            tries: HIDE_TRIES,
            ending: None,
            panel: None,
        });
        if !complete {
            self.end_hunt(false);
            return;
        }
        self.frame();
        if let Some(hunt) = &self.hunt {
            for thing in &hunt.things {
                // SAFETY: shows our own window without taking the focus.
                unsafe { ShowWindow(thing.window, SW_SHOWNOACTIVATE) };
            }
        }
        self.frames(true);
        // With no frames to draw, the counter still has to move and the time to run out.
        if self.reduced_motion() {
            // SAFETY: only starts our own timer.
            unsafe { SetTimer(self.waiter, FRAME_TIMER, 1000, None) };
        }
    }

    fn end_hunt(&mut self, found: bool) {
        self.hunt = None;
        self.frames(false);
        self.hide_windows();
        self.shared.screen.found(found);
        self.plan(None);
    }

    fn place_counter(&self, panel: &Panel) {
        let work = work_area();
        panel.canvas.present(
            self.panel,
            i32::midpoint(work.left, work.right) - panel.canvas.width / 2,
            work.top + scaled(TOP_MARGIN, self.scale),
        );
        // SAFETY: shows our own window without taking the focus.
        unsafe { ShowWindow(self.panel, SW_SHOWNOACTIVATE) };
    }

    fn show_counter(&mut self, seconds_left: u32) {
        let (scale, texts) = (self.scale, self.texts);
        let Some(hunt) = &self.hunt else {
            return;
        };
        let panel = text_panel(
            &[
                (&texts.find(&hunt.look.name), TEXT),
                (&clock(seconds_left), COUNTER),
                (&texts.tries(hunt.tries), MUTED),
            ],
            true,
            scale,
        );
        if let Some(panel) = panel {
            self.place_counter(&panel);
            if let Some(hunt) = &mut self.hunt {
                hunt.panel = Some(panel);
            }
        }
    }

    /// The mascot comes out of its hiding place, says how it went and leaves on its own.
    fn finish_hunt(&mut self, found: bool) {
        let (scale, texts) = (self.scale, self.texts);
        let Some(hunt) = &mut self.hunt else {
            return;
        };
        let Some(spot) = hunt
            .things
            .iter()
            .find(|thing| thing.spot.hides)
            .map(|thing| thing.spot)
        else {
            return;
        };
        let size = hunt.look.size;
        let x = (spot.x + spot.width / 2 - size / 2).clamp(hunt.work.left, hunt.work.right - size);
        let y = (spot.y + spot.height() - size + scaled(FEET_ROOM, scale)).max(hunt.work.top);
        hunt.ending = Some(Ending {
            found,
            started: Instant::now(),
            x,
            y,
        });
        let (xp, kibble) = Game::HideAndSeek.reward();
        let panel = if found {
            text_panel(
                &[(&format!("+{xp} XP"), XP), (&texts.kibble(kibble), COUNTER)],
                false,
                scale,
            )
        } else {
            text_panel(&[(texts.missed(), TEXT)], false, scale)
        };
        if let Some(panel) = panel {
            let left = (x + size / 2 - panel.canvas.width / 2)
                .clamp(hunt.work.left, hunt.work.right - panel.canvas.width);
            let top = (y + scaled(12.0, scale) - panel.canvas.height).max(hunt.work.top);
            panel.canvas.present(self.panel, left, top);
            hunt.panel = Some(panel);
        }
        self.hunt_frame();
        // SAFETY: our own windows, in front of the things that were opened after them.
        unsafe {
            ShowWindow(self.mascot, SW_SHOWNOACTIVATE);
            SetWindowPos(
                self.mascot,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            SetWindowPos(
                self.panel,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    fn hunt_frame(&mut self) {
        let reduced = self.reduced_motion();
        let Some(hunt) = &mut self.hunt else {
            return;
        };
        let seconds = hunt.started.elapsed().as_secs_f32();
        let species = hunt.look.species;
        let ending = hunt.ending.is_some();
        for thing in &mut hunt.things {
            thing.present(species, seconds, reduced, ending && thing.spot.hides);
        }
        if let Some(ending) = &hunt.ending {
            let frame_seconds = if reduced { 0.0 } else { seconds };
            let mood = if ending.found {
                Mood::Happy
            } else {
                Mood::Idle
            };
            present_mascot(
                self.mascot,
                &hunt.look,
                mood,
                frame_seconds,
                ending.x,
                ending.y,
            );
            if ending.started.elapsed().as_secs_f32() >= ENDING_SECONDS {
                let found = ending.found;
                self.end_hunt(found);
            }
            return;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let whole = seconds as u32;
        if whole >= HIDE_SECONDS {
            self.finish_hunt(false);
        } else if hunt.shown_seconds != Some(whole) {
            hunt.shown_seconds = Some(whole);
            self.show_counter(HIDE_SECONDS - whole);
        }
    }

    fn click_thing(&mut self, window: HWND) {
        let Some(hunt) = &mut self.hunt else {
            return;
        };
        if hunt.ending.is_some() {
            return;
        }
        let Some(thing) = hunt.things.iter_mut().find(|thing| thing.window == window) else {
            return;
        };
        if thing.spot.opened {
            return;
        }
        if thing.spot.hides {
            self.finish_hunt(true);
            return;
        }
        thing.spot.opened = true;
        hunt.tries -= 1;
        let tries = hunt.tries;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let left = HIDE_SECONDS.saturating_sub(hunt.started.elapsed().as_secs_f32() as u32);
        self.hunt_frame();
        if tries == 0 {
            self.finish_hunt(false);
        } else {
            self.show_counter(left);
        }
    }

    fn thing_cursor(&self, window: HWND) -> Option<HCURSOR> {
        let hunt = self.hunt.as_ref()?;
        let thing = hunt.things.iter().find(|thing| thing.window == window)?;
        Some(if thing.spot.opened || hunt.ending.is_some() {
            self.arrow
        } else {
            self.hand
        })
    }

    fn frame(&mut self) {
        if self.hunt.is_some() {
            self.hunt_frame();
        } else if self.visit.is_some() {
            self.visit_frame();
        }
    }

    fn click_mascot(&mut self) {
        if let Some(visit) = &mut self.visit
            && matches!(visit.step, Step::Staying { .. })
        {
            visit.happy_until = visit.started.elapsed().as_secs_f32() + HAPPY_SECONDS;
        }
    }

    fn click_panel(&mut self, x: i32) {
        if let Some(hunt) = &self.hunt {
            let closes =
                hunt.ending.is_none() && hunt.panel.as_ref().is_some_and(|panel| x >= panel.close);
            if closes {
                self.end_hunt(false);
            }
            return;
        }
        let Some(panel) = self.visit.as_ref().and_then(|visit| visit.panel.as_ref()) else {
            return;
        };
        let (button, close) = (panel.button, panel.close);
        if (button.0..button.1).contains(&x) {
            self.end_visit();
            self.shared.screen.open_asked.store(true, Ordering::Relaxed);
            // Lost, these only leave the overlay closed, and a click brings the mascot back.
            let _ = self.shared.events.set_overlay_visible(true);
            let _ = self.shared.events.emit("open", serde_json::Value::Null);
        } else if x >= close {
            self.send_away();
        }
    }
}

fn low_word(value: LPARAM) -> i32 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    let word = (value & 0xFFFF) as u16 as i16;
    i32::from(word)
}

unsafe extern "system" fn window_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Taken out while it works: showing a window calls this procedure again at once, and those
    // calls have nothing to do here but the default.
    let handled = SCREEN.take().and_then(|mut screen| {
        let result = match message {
            WAKE => {
                if screen.visit.is_some() && !screen.shared.progress().on_screen {
                    screen.end_visit();
                }
                screen.plan(None);
                Some(0)
            }
            HIDE => {
                screen.begin_hunt();
                Some(0)
            }
            WM_TIMER if wparam == WAIT_TIMER => {
                // SAFETY: only stops our own timer: the next wait is set when the visit ends.
                unsafe { KillTimer(window, WAIT_TIMER) };
                screen.begin_visit();
                Some(0)
            }
            WM_TIMER if wparam == FRAME_TIMER => {
                screen.frame();
                Some(0)
            }
            // The timers lost track of the time while the computer was asleep.
            WM_POWERBROADCAST => {
                if screen.visit.is_none() && screen.hunt.is_none() {
                    screen.plan(None);
                }
                Some(1)
            }
            WM_LBUTTONUP if window == screen.mascot => {
                screen.click_mascot();
                Some(0)
            }
            WM_LBUTTONUP if window == screen.panel => {
                screen.click_panel(low_word(lparam));
                Some(0)
            }
            WM_LBUTTONUP => {
                screen.click_thing(window);
                Some(0)
            }
            WM_SETCURSOR if window == screen.mascot || window == screen.panel => {
                // SAFETY: a system cursor, which lives as long as the program.
                unsafe { SetCursor(screen.hand) };
                Some(1)
            }
            WM_SETCURSOR => screen.thing_cursor(window).map(|cursor| {
                // SAFETY: a system cursor, which lives as long as the program.
                unsafe { SetCursor(cursor) };
                1
            }),
            _ => None,
        };
        SCREEN.set(Some(screen));
        result
    });
    // SAFETY: forwards the arguments Windows gave us.
    handled.unwrap_or_else(|| unsafe { DefWindowProcW(window, message, wparam, lparam) })
}

fn listen(shared: &Arc<Shared>) -> Option<()> {
    // A program is scaled by Windows unless it says otherwise, and the mascot would look blurred.
    // SAFETY: changes how Windows treats this thread's windows, before it opens any.
    unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let class = wide("ArthexCompanionScreen");
    // SAFETY: the class name outlives the call.
    unsafe {
        let description = WNDCLASSW {
            lpfnWndProc: Some(window_procedure),
            hInstance: module(),
            lpszClassName: class.as_ptr(),
            // SAFETY: every other field of this C struct is valid when zero.
            ..std::mem::zeroed()
        };
        RegisterClassW(&raw const description);
    }
    let (waiter, mascot, panel) = (
        create_window(class.as_ptr()),
        create_window(class.as_ptr()),
        create_window(class.as_ptr()),
    );
    if waiter.is_null() || mascot.is_null() || panel.is_null() {
        // Nobody will ever answer: whoever waits for a game must not wait forever.
        shared.screen.found(false);
        return None;
    }
    let screen = Screen {
        shared: Arc::clone(shared),
        texts: Texts {
            french: shared.french,
        },
        waiter,
        mascot,
        panel,
        class,
        // SAFETY: system cursors, shared and never destroyed.
        hand: unsafe { LoadCursorW(ptr::null_mut(), IDC_HAND) },
        // SAFETY: as above.
        arrow: unsafe { LoadCursorW(ptr::null_mut(), IDC_ARROW) },
        scale: main_screen_scale(),
        visit: None,
        hunt: None,
        frames_drawn: 0,
    };
    screen.plan(None);
    SCREEN.set(Some(screen));
    shared
        .screen
        .window
        .store(waiter as isize, Ordering::Relaxed);
    // SAFETY: the windows and this message loop belong to this thread, for as long as the program.
    unsafe {
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&raw mut message, ptr::null_mut(), 0, 0) > 0 {
            DispatchMessageW(&raw const message);
        }
        DestroyWindow(waiter);
    }
    Some(())
}

/// Waits for the visits for as long as the program runs.
pub fn start(shared: Arc<Shared>) {
    thread::spawn(move || listen(&shared));
}
