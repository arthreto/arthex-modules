use std::{
    cell::RefCell,
    ptr,
    sync::{
        Arc,
        atomic::{AtomicIsize, Ordering},
    },
    thread,
};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
    Graphics::Gdi::{
        CreateFontW, DT_CENTER, DT_END_ELLIPSIS, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
        DeleteObject, DrawTextW, FW_MEDIUM, FW_NORMAL, FW_SEMIBOLD, GetMonitorInfoW,
        GetTextExtentPoint32W, HFONT, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
        ScreenToClient, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
    },
    System::SystemInformation::GetLocalTime,
    UI::{
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext},
        WindowsAndMessaging::{
            DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, HCURSOR,
            IDC_ARROW, IDC_HAND, KillTimer, LoadCursorW, MSG, PostMessageW, RegisterClassW,
            SW_SHOWNOACTIVATE, SetCursor, SetTimer, ShowWindow, WM_APP, WM_LBUTTONUP,
            WM_POWERBROADCAST, WM_SETCURSOR, WM_TIMECHANGE, WM_TIMER, WNDCLASSW,
        },
    },
};

use super::{
    Shared,
    canvas::{Canvas, create_window, main_screen_scale, module, scaled, wide},
    moment::{Moment, moment_of},
    paint::{
        BUTTON_GAP, CLOSE_LEFT_FROM_END, HUD_HEIGHT, HUD_HEIGHT_PROGRESS, HUD_WIDTH, HudLayout,
        TEXT_LEFT, finish_hud, hud_base, tint_color,
    },
    text::{Reminder, Texts},
};

const WAKE: u32 = WM_APP + 1;
const TIMER: usize = 1;
/// A long wait is cut in pieces, which also catches a clock that drifted.
const LONGEST_WAIT_MINUTES: i64 = 360;
const SCREEN_TOP: f32 = 20.0;
const GAP: f32 = 10.0;
const BUTTON_PADDING: f32 = 12.0;
const TITLE_SIZE: f32 = 15.0;
const WHEN_SIZE: f32 = 12.5;
const BUTTON_SIZE: f32 = 13.0;
const TITLE_TOP: f32 = 13.0;
const TITLE_TOP_PROGRESS: f32 = 11.0;
const LINE_GAP: f32 = 21.0;
const LINE_HEIGHT: f32 = 20.0;
/// `--text` and `--text-muted` of `src/ui/tokens.css`, as Windows colours: 0x00BBGGRR.
const TEXT_COLOR: u32 = 0x00F6_F4F3;
const MUTED_COLOR: u32 = 0x00C2_B9B4;

/// Lets another thread tell the HUD's thread that the items changed.
#[derive(Default)]
pub struct Wake {
    window: AtomicIsize,
}

impl Wake {
    pub fn now(&self) {
        let window = self.window.load(Ordering::Relaxed) as HWND;
        if !window.is_null() {
            // SAFETY: only posts a message; a window that is gone makes it fail harmlessly.
            unsafe { PostMessageW(window, WAKE, 0, 0) };
        }
    }
}

/// One reminder on the screen.
struct Hud {
    window: HWND,
    id: String,
    canvas: Canvas,
    button_left: i32,
    button_right: i32,
    close_left: i32,
}

/// What the window procedure needs; everything runs on the HUD's thread.
struct Screen {
    shared: Arc<Shared>,
    texts: Texts,
    /// Hidden, and only there to wait: top-level windows hear when the computer wakes up or the
    /// clock changes, and a timer needs no polling.
    waiter: HWND,
    class: Vec<u16>,
    huds: Vec<Hud>,
    scale: f32,
    hand: HCURSOR,
    arrow: HCURSOR,
}

thread_local! {
    static SCREEN: RefCell<Option<Screen>> = const { RefCell::new(None) };
}

fn now() -> Moment {
    // SAFETY: fills a local of the expected type.
    let time = unsafe {
        let mut time = std::mem::zeroed();
        GetLocalTime(&raw mut time);
        time
    };
    moment_of(
        i64::from(time.wYear),
        i64::from(time.wMonth),
        i64::from(time.wDay),
        i64::from(time.wHour),
        i64::from(time.wMinute),
    )
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

/// Text in a rectangle: on one line, with an ellipsis rather than over what comes next.
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

#[allow(clippy::cast_precision_loss)]
fn draw(reminder: &Reminder, texts: Texts, scale: f32) -> Option<(Canvas, i32, i32, i32)> {
    let has_progress = reminder.progress.is_some();
    let width = scaled(HUD_WIDTH, scale);
    let height = scaled(
        if has_progress {
            HUD_HEIGHT_PROGRESS
        } else {
            HUD_HEIGHT
        },
        scale,
    );
    let close_left = width - scaled(CLOSE_LEFT_FROM_END, scale);
    let (title_font, when_font, button_font) = (
        font(TITLE_SIZE, FW_SEMIBOLD, scale),
        font(WHEN_SIZE, FW_NORMAL, scale),
        font(BUTTON_SIZE, FW_MEDIUM, scale),
    );
    let button_right = close_left - scaled(4.0, scale);
    let button_left =
        button_right - text_width(button_font, texts.later()) - 2 * scaled(BUTTON_PADDING, scale);
    let drawn = Canvas::new(width, height).map(|mut canvas| {
        let layout = HudLayout {
            width: width.cast_unsigned(),
            height: height.cast_unsigned(),
            scale,
            button_left: button_left as f32,
            button_right: button_right as f32,
        };
        canvas.pixels_mut().copy_from_slice(&hud_base(
            layout,
            tint_color(&reminder.tint),
            reminder.progress,
        ));
        let text_left = scaled(TEXT_LEFT, scale);
        let text_right = button_left - scaled(BUTTON_GAP, scale);
        let title_top = scaled(
            if has_progress {
                TITLE_TOP_PROGRESS
            } else {
                TITLE_TOP
            },
            scale,
        );
        let line = |top: i32| RECT {
            left: text_left,
            top,
            right: text_right,
            bottom: top + scaled(LINE_HEIGHT, scale),
        };
        write(
            &canvas,
            title_font,
            TEXT_COLOR,
            &reminder.title,
            line(title_top),
            DT_END_ELLIPSIS,
        );
        write(
            &canvas,
            when_font,
            MUTED_COLOR,
            &reminder.when,
            line(title_top + scaled(LINE_GAP, scale)),
            DT_END_ELLIPSIS,
        );
        write(
            &canvas,
            button_font,
            TEXT_COLOR,
            texts.later(),
            RECT {
                left: button_left,
                top: 0,
                right: button_right,
                bottom: height,
            },
            DT_CENTER,
        );
        finish_hud(
            canvas.pixels_mut(),
            width.cast_unsigned(),
            height.cast_unsigned(),
            scale,
        );
        canvas
    });
    // SAFETY: created above, and no longer selected in any context.
    unsafe {
        DeleteObject(title_font);
        DeleteObject(when_font);
        DeleteObject(button_font);
    }
    drawn.map(|canvas| (canvas, button_left, button_right, close_left))
}

fn cursor() -> POINT {
    let mut position = POINT { x: 0, y: 0 };
    // SAFETY: writes into a local.
    unsafe { GetCursorPos(&raw mut position) };
    position
}

impl Screen {
    /// One under the other, at the top of the screen the user is on.
    fn place(&self) {
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
        // SAFETY: `info` is a local with its size set; this monitor always exists.
        unsafe {
            GetMonitorInfoW(
                MonitorFromPoint(cursor(), MONITOR_DEFAULTTONEAREST),
                &raw mut info,
            );
        }
        let middle = i32::midpoint(info.rcWork.left, info.rcWork.right);
        let mut top = info.rcWork.top + scaled(SCREEN_TOP, self.scale);
        for hud in &self.huds {
            hud.canvas
                .present(hud.window, middle - hud.canvas.width / 2, top);
            // SAFETY: shows our own window without taking the focus.
            unsafe { ShowWindow(hud.window, SW_SHOWNOACTIVATE) };
            top += hud.canvas.height + scaled(GAP, self.scale);
        }
    }

    fn show(&mut self, reminder: &Reminder) {
        let window = create_window(self.class.as_ptr());
        if window.is_null() {
            return;
        }
        let Some((canvas, button_left, button_right, close_left)) =
            draw(reminder, self.texts, self.scale)
        else {
            // SAFETY: created just above, and used nowhere else.
            unsafe { DestroyWindow(window) };
            return;
        };
        self.huds.push(Hud {
            window,
            id: reminder.id.clone(),
            canvas,
            button_left,
            button_right,
            close_left,
        });
    }

    /// Shows what is due and waits for what comes next, without looking in the meantime.
    fn check(&mut self) {
        let moment = now();
        let (due, next, saved) = {
            let mut store = self.shared.store();
            let due = store.take_due(moment);
            let saved = due.is_empty() || store.save(&self.shared.folder).is_ok();
            let next = store.next_wake(moment);
            drop(store);
            (due, next, saved)
        };
        // Better to say nothing than to repeat a reminder at each start when the disk is full.
        if saved && !due.is_empty() {
            for item in &due {
                if let Some(reminder) = self.texts.reminder(item, moment) {
                    self.show(&reminder);
                }
            }
            self.place();
        }
        // SAFETY: only starts or stops our own timer.
        unsafe {
            match next {
                Some(next) => {
                    let minutes = (next - moment).clamp(1, LONGEST_WAIT_MINUTES);
                    let milliseconds = u32::try_from(minutes * 60_000).unwrap_or(u32::MAX);
                    SetTimer(self.waiter, TIMER, milliseconds, None);
                }
                None => {
                    KillTimer(self.waiter, TIMER);
                }
            }
        }
    }

    fn close(&mut self, window: HWND) {
        self.huds.retain(|hud| hud.window != window);
        // SAFETY: our own window, which nothing refers to any more.
        unsafe { DestroyWindow(window) };
        self.place();
    }

    fn click(&mut self, window: HWND, x: i32) {
        let Some(hud) = self.huds.iter().find(|hud| hud.window == window) else {
            return;
        };
        if (hud.button_left..hud.button_right).contains(&x) {
            let id = hud.id.clone();
            let mut store = self.shared.store();
            store.snooze(&id, now());
            // Only a convenience: at worst, it comes back at the next start.
            let _ = store.save(&self.shared.folder);
            drop(store);
            self.close(window);
            self.check();
        } else if x >= hud.close_left {
            self.close(window);
        }
    }

    fn cursor_for(&self, window: HWND) -> HCURSOR {
        let mut position = cursor();
        // SAFETY: writes into a local, from our own window.
        unsafe { ScreenToClient(window, &raw mut position) };
        let on_button = self
            .huds
            .iter()
            .find(|hud| hud.window == window)
            .is_some_and(|hud| {
                (hud.button_left..hud.button_right).contains(&position.x)
                    || position.x >= hud.close_left
            });
        if on_button { self.hand } else { self.arrow }
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
    // Taken out while it works: creating or closing a window calls this procedure again at once,
    // and those calls have nothing to do here but the default.
    let handled = SCREEN.take().and_then(|mut screen| {
        let result = match message {
            WAKE | WM_TIMER | WM_TIMECHANGE => {
                screen.check();
                Some(0)
            }
            // The timer lost track of the time while the computer was asleep.
            WM_POWERBROADCAST => {
                screen.check();
                Some(1)
            }
            WM_LBUTTONUP => {
                screen.click(window, low_word(lparam));
                Some(0)
            }
            WM_SETCURSOR => {
                // SAFETY: system cursors, which live as long as the program.
                unsafe { SetCursor(screen.cursor_for(window)) };
                Some(1)
            }
            _ => None,
        };
        SCREEN.set(Some(screen));
        result
    });
    // SAFETY: forwards the arguments Windows gave us.
    handled.unwrap_or_else(|| unsafe { DefWindowProcW(window, message, wparam, lparam) })
}

fn listen(shared: &Arc<Shared>, texts: Texts) -> Option<()> {
    // A program is scaled by Windows unless it says otherwise, and the HUD would look blurred.
    // SAFETY: changes how Windows treats this thread's windows, before it opens any.
    unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let class = wide("ArthexCalendarHud");
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
    let waiter = create_window(class.as_ptr());
    if waiter.is_null() {
        return None;
    }
    // SAFETY: system cursors, shared and never destroyed.
    let (hand, arrow) = unsafe {
        (
            LoadCursorW(ptr::null_mut(), IDC_HAND),
            LoadCursorW(ptr::null_mut(), IDC_ARROW),
        )
    };
    let mut screen = Screen {
        shared: Arc::clone(shared),
        texts,
        waiter,
        class,
        huds: Vec::new(),
        scale: main_screen_scale(),
        hand,
        arrow,
    };
    screen.check();
    SCREEN.set(Some(screen));
    shared.wake.window.store(waiter as isize, Ordering::Relaxed);
    // SAFETY: the windows and this message loop belong to this thread, for as long as the program.
    unsafe {
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&raw mut message, ptr::null_mut(), 0, 0) > 0 {
            DispatchMessageW(&raw const message);
        }
    }
    Some(())
}

/// Waits for reminders for as long as the program runs.
pub fn start(shared: Arc<Shared>, texts: Texts) {
    thread::spawn(move || listen(&shared, texts));
}
