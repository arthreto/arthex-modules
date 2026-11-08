use std::{cell::Cell, cell::RefCell, ptr};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        BitBlt, CAPTUREBLT, EnumDisplayMonitors, GetDC, GetMonitorInfoW, HDC, HMONITOR,
        MONITORINFO, ReleaseDC, SRCCOPY,
    },
    UI::{
        Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, SetFocus, VK_ESCAPE},
        WindowsAndMessaging::{
            DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, GetSystemMetrics,
            HCURSOR, IDC_CROSS, LoadCursorW, MSG, PostQuitMessage, RegisterClassW,
            SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE,
            SW_SHOWNOACTIVATE, SetCursor, SetForegroundWindow, ShowWindow, WM_ACTIVATEAPP,
            WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONUP, WM_SETCURSOR,
            WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TRANSPARENT,
        },
    },
};

use super::canvas::{
    Canvas, create_window, cursor_position, main_screen_scale, module, scaled, wide,
};
use crate::{
    bitmap::Bitmap,
    paint::{edge_fade, zone_frame},
};

const ZONE_BORDER: f32 = 2.0;
/// Below this, in pixels on a screen at 100 %, the button only moved by accident.
const SMALLEST_ZONE: f32 = 6.0;

/// The screen as it is now, across every monitor, before anything of ours is on it.
struct ScreenCopy {
    canvas: Canvas,
    left: i32,
    top: i32,
}

impl ScreenCopy {
    fn take() -> Option<Self> {
        // SAFETY: these calls only read the layout of the screens.
        let (left, top, width, height) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        };
        let canvas = Canvas::new(width, height)?;
        // SAFETY: both contexts are valid for this block and the size is that of the bitmap.
        let copied = unsafe {
            let screen = GetDC(ptr::null_mut());
            let copied = BitBlt(
                canvas.context,
                0,
                0,
                width,
                height,
                screen,
                left,
                top,
                SRCCOPY | CAPTUREBLT,
            );
            ReleaseDC(ptr::null_mut(), screen);
            copied
        };
        (copied != 0).then_some(Self { canvas, left, top })
    }

    fn part(&self, zone: RECT) -> Option<Bitmap> {
        let whole = Bitmap {
            width: self.canvas.width.cast_unsigned(),
            height: self.canvas.height.cast_unsigned(),
            pixels: self.canvas.pixels().to_vec(),
        };
        let part = whole.crop(
            zone.left - self.left,
            zone.top - self.top,
            zone.right - self.left,
            zone.bottom - self.top,
        )?;
        Some(Bitmap::opaque(part.width, part.height, part.pixels))
    }
}

/// What the window procedure needs; everything runs on the thread of `run`.
struct Session {
    zone: HWND,
    cursor: HCURSOR,
    scale: f32,
    from: Cell<Option<POINT>>,
    chosen: Cell<Option<RECT>>,
}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

fn zone_between(from: POINT, to: POINT) -> RECT {
    RECT {
        left: from.x.min(to.x),
        top: from.y.min(to.y),
        right: from.x.max(to.x),
        bottom: from.y.max(to.y),
    }
}

fn draw_zone(session: &Session, zone: RECT) {
    let (width, height) = (zone.right - zone.left, zone.bottom - zone.top);
    let Some(mut canvas) = Canvas::new(width, height) else {
        return;
    };
    let border = scaled(ZONE_BORDER, session.scale).cast_unsigned();
    canvas.pixels_mut().copy_from_slice(&zone_frame(
        width.cast_unsigned(),
        height.cast_unsigned(),
        border,
    ));
    canvas.present(session.zone, zone.left, zone.top);
    // SAFETY: shows our own window.
    unsafe { ShowWindow(session.zone, SW_SHOWNOACTIVATE) };
}

fn finish(session: &Session, chosen: Option<RECT>) {
    session.chosen.set(chosen);
    // SAFETY: ends the message loop of this thread.
    unsafe { PostQuitMessage(0) };
}

fn release(session: &Session) {
    let Some(from) = session.from.take() else {
        return;
    };
    let zone = zone_between(from, cursor_position());
    let smallest = scaled(SMALLEST_ZONE, session.scale);
    // SAFETY: gives back the mouse this thread took.
    unsafe { ReleaseCapture() };
    if zone.right - zone.left >= smallest && zone.bottom - zone.top >= smallest {
        finish(session, Some(zone));
    } else {
        // SAFETY: hides our own window.
        unsafe { ShowWindow(session.zone, SW_HIDE) };
    }
}

unsafe extern "system" fn window_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let handled = SESSION.with_borrow(|session| {
        let session = session.as_ref()?;
        match message {
            WM_SETCURSOR => {
                // SAFETY: a system cursor, which lives as long as the program.
                unsafe { SetCursor(session.cursor) };
                Some(1)
            }
            WM_LBUTTONDOWN => {
                session.from.set(Some(cursor_position()));
                // SAFETY: keeps the mouse on our window while the button is down.
                unsafe { SetCapture(window) };
                Some(0)
            }
            WM_MOUSEMOVE => {
                if let Some(from) = session.from.get() {
                    draw_zone(session, zone_between(from, cursor_position()));
                }
                Some(0)
            }
            WM_LBUTTONUP => {
                release(session);
                Some(0)
            }
            WM_RBUTTONUP => {
                finish(session, None);
                Some(0)
            }
            WM_KEYDOWN if wparam == usize::from(VK_ESCAPE) => {
                finish(session, None);
                Some(0)
            }
            // Another program took over: the selection must never stay in the way.
            WM_ACTIVATEAPP if wparam == 0 => {
                finish(session, None);
                Some(0)
            }
            _ => None,
        }
    });
    // SAFETY: forwards the arguments Windows gave us.
    handled.unwrap_or_else(|| unsafe { DefWindowProcW(window, message, wparam, lparam) })
}

unsafe extern "system" fn collect_screen(
    monitor: HMONITOR,
    _context: HDC,
    _bounds: *mut RECT,
    screens: LPARAM,
) -> i32 {
    // SAFETY: `screens` is the vector `all_screens` passes, alive during the enumeration.
    let screens = unsafe { &mut *(screens as *mut Vec<RECT>) };
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
    // SAFETY: the monitor comes from Windows and `info` is a local with its size set.
    if unsafe { GetMonitorInfoW(monitor, &raw mut info) } != 0 {
        screens.push(info.rcMonitor);
    }
    1
}

fn all_screens() -> Vec<RECT> {
    let mut screens: Vec<RECT> = Vec::new();
    // SAFETY: the callback only uses the vector, which outlives this call.
    unsafe {
        EnumDisplayMonitors(
            ptr::null_mut(),
            ptr::null(),
            Some(collect_screen),
            (&raw mut screens) as LPARAM,
        );
    }
    screens
}

/// Covers every screen with the fade until the user traces a zone or cancels, on the calling
/// thread. What the zone shows is read from a copy taken first, so the fade is not part of it.
pub fn run() -> Option<Bitmap> {
    let copy = ScreenCopy::take()?;
    let class = wide("ArthexQrSelection");
    // SAFETY: the class name outlives the call; registering it twice only fails.
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
    let screens = all_screens();
    let zone = create_window(class.as_ptr(), WS_EX_TRANSPARENT | WS_EX_NOACTIVATE);
    // SAFETY: a system cursor, shared and never destroyed.
    let cursor = unsafe { LoadCursorW(ptr::null_mut(), IDC_CROSS) };
    let fades: Vec<HWND> = screens
        .iter()
        .map(|_| create_window(class.as_ptr(), 0))
        .collect();
    let first = fades.first().copied().filter(|first| !first.is_null());

    let mut chosen = None;
    if let Some(first) = first.filter(|_| !zone.is_null()) {
        SESSION.set(Some(Session {
            zone,
            cursor,
            scale: main_screen_scale(),
            from: Cell::new(None),
            chosen: Cell::new(None),
        }));
        for (window, screen) in fades.iter().zip(&screens) {
            let (width, height) = (screen.right - screen.left, screen.bottom - screen.top);
            if let Some(mut canvas) = Canvas::new(width, height) {
                let pixels = edge_fade(width.cast_unsigned(), height.cast_unsigned());
                canvas.pixels_mut().copy_from_slice(&pixels);
                canvas.present(*window, screen.left, screen.top);
            }
            // SAFETY: shows our own window.
            unsafe { ShowWindow(*window, SW_SHOWNOACTIVATE) };
        }
        // SAFETY: the windows, the session and this message loop all belong to this thread. The
        // overlay had the focus until a moment ago, so Windows lets us take it for Escape.
        unsafe {
            SetForegroundWindow(first);
            SetFocus(first);
            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&raw mut message, ptr::null_mut(), 0, 0) > 0 {
                DispatchMessageW(&raw const message);
            }
        }
        chosen = SESSION.take().and_then(|session| session.chosen.take());
    }

    // SAFETY: everything here was created above; destroying a null handle only fails.
    unsafe {
        for window in fades {
            DestroyWindow(window);
        }
        DestroyWindow(zone);
    }
    chosen.and_then(|zone| copy.part(zone))
}
