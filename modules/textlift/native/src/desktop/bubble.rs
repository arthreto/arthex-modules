use std::{cell::RefCell, ptr, sync::Arc, thread};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
    Graphics::Gdi::{
        CreateFontW, DeleteObject, FW_MEDIUM, GetMonitorInfoW, GetTextExtentPoint32W, HFONT,
        MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, SelectObject, SetBkMode,
        SetTextColor, TRANSPARENT, TextOutW,
    },
    System::DataExchange::AddClipboardFormatListener,
    UI::WindowsAndMessaging::{
        DefWindowProcW, DispatchMessageW, GetMessageW, HCURSOR, IDC_HAND, KillTimer, LoadCursorW,
        MSG, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE, SetCursor, SetTimer, ShowWindow,
        WM_CLIPBOARDUPDATE, WM_LBUTTONUP, WM_SETCURSOR, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE,
    },
};

use super::{
    Shared,
    canvas::{
        Canvas, create_window, cursor_position, exclude_from_capture, main_screen_scale, module,
        scaled, wide,
    },
    clipboard,
};
use crate::paint::{
    BUBBLE_CLOSE_WIDTH, BUBBLE_HEIGHT, BUBBLE_TEXT_GAP, BUBBLE_TEXT_LEFT, bubble_base,
    finish_bubble,
};

const VISIBLE_MS: u32 = 6000;
const TIMER: usize = 1;
const SCREEN_MARGIN: f32 = 16.0;
const FONT_SIZE: f32 = 14.0;
/// `--text` of `src/ui/tokens.css`, as a Windows colour: 0x00BBGGRR.
const TEXT_COLOR: u32 = 0x00F6_F4F3;

/// What the window procedure needs; everything runs on the bubble's thread.
struct Bubble {
    shared: Arc<Shared>,
    window: HWND,
    canvas: Canvas,
    close_from: i32,
    margin: i32,
    cursor: HCURSOR,
}

thread_local! {
    static BUBBLE: RefCell<Option<Bubble>> = const { RefCell::new(None) };
}

impl Bubble {
    /// In the corner of the screen where the user just worked.
    fn show(&self) {
        let cursor = cursor_position();
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
                MonitorFromPoint(
                    POINT {
                        x: cursor.x,
                        y: cursor.y,
                    },
                    MONITOR_DEFAULTTONEAREST,
                ),
                &raw mut info,
            );
        }
        self.canvas.present(
            self.window,
            info.rcWork.right - self.canvas.width - self.margin,
            info.rcWork.bottom - self.canvas.height - self.margin,
        );
        // SAFETY: shows our own window without taking the focus, and starts its own timer.
        unsafe {
            ShowWindow(self.window, SW_SHOWNOACTIVATE);
            SetTimer(self.window, TIMER, VISIBLE_MS, None);
        }
    }

    fn hide(&self) {
        // SAFETY: stops our own timer and hides our own window.
        unsafe {
            KillTimer(self.window, TIMER);
            ShowWindow(self.window, SW_HIDE);
        }
    }

    fn click(&self, x: i32) {
        self.hide();
        if x >= self.close_from {
            return;
        }
        if let Some(image) = clipboard::image() {
            self.shared.show_image(image);
        }
    }
}

/// The part of a mouse message that holds x, which is negative left of the window.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn low_word(value: LPARAM) -> i32 {
    i32::from((value & 0xFFFF) as u16 as i16)
}

unsafe extern "system" fn window_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let handled = BUBBLE.with_borrow(|bubble| {
        let bubble = bubble.as_ref()?;
        match message {
            WM_CLIPBOARDUPDATE => {
                if bubble.shared.proposes() && clipboard::holds_screenshot() {
                    bubble.show();
                }
                Some(0)
            }
            WM_TIMER => {
                bubble.hide();
                Some(0)
            }
            WM_SETCURSOR => {
                // SAFETY: a system cursor, which lives as long as the program.
                unsafe { SetCursor(bubble.cursor) };
                Some(1)
            }
            WM_LBUTTONUP => {
                bubble.click(low_word(lparam));
                Some(0)
            }
            _ => None,
        }
    });
    // SAFETY: forwards the arguments Windows gave us.
    handled.unwrap_or_else(|| unsafe { DefWindowProcW(window, message, wparam, lparam) })
}

fn font(scale: f32) -> HFONT {
    let name = wide("Segoe UI Variable Text");
    // SAFETY: the name outlives the call; a missing font falls back to a system one.
    unsafe {
        CreateFontW(
            -scaled(FONT_SIZE, scale),
            0,
            0,
            0,
            FW_MEDIUM.cast_signed(),
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

fn text_size(font: HFONT, text: &[u16]) -> Option<SIZE> {
    let probe = Canvas::new(1, 1)?;
    let mut size = SIZE { cx: 0, cy: 0 };
    let length = i32::try_from(text.len()).ok()?;
    // SAFETY: the context and font live through this block, and the text outlives the call.
    unsafe {
        let previous = SelectObject(probe.context, font);
        GetTextExtentPoint32W(probe.context, text.as_ptr(), length, &raw mut size);
        SelectObject(probe.context, previous);
    }
    Some(size)
}

fn draw(label: &str, scale: f32) -> Option<(Canvas, i32)> {
    let text: Vec<u16> = label.encode_utf16().collect();
    let font = font(scale);
    let measured = text_size(font, &text);
    let drawn = measured.and_then(|size| {
        let close = scaled(BUBBLE_CLOSE_WIDTH, scale);
        let text_left = scaled(BUBBLE_TEXT_LEFT, scale);
        let width = text_left + size.cx + scaled(BUBBLE_TEXT_GAP, scale) + close;
        let height = scaled(BUBBLE_HEIGHT, scale);
        let mut canvas = Canvas::new(width, height)?;
        let (width_px, height_px) = (width.cast_unsigned(), height.cast_unsigned());
        canvas
            .pixels_mut()
            .copy_from_slice(&bubble_base(width_px, height_px, scale));
        let length = i32::try_from(text.len()).ok()?;
        // SAFETY: the context and font live through this block, and the text outlives the call.
        unsafe {
            let previous = SelectObject(canvas.context, font);
            SetBkMode(canvas.context, TRANSPARENT.cast_signed());
            SetTextColor(canvas.context, TEXT_COLOR);
            TextOutW(
                canvas.context,
                text_left,
                (height - size.cy) / 2,
                text.as_ptr(),
                length,
            );
            SelectObject(canvas.context, previous);
        }
        finish_bubble(canvas.pixels_mut(), width_px, height_px, scale);
        Some((canvas, width - close))
    });
    // SAFETY: created above, and no longer selected in any context.
    unsafe { DeleteObject(font) };
    drawn
}

fn listen(shared: Arc<Shared>, label: &str) -> Option<()> {
    let scale = main_screen_scale();
    let class = wide("ArthexTextLiftBubble");
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
    let window = create_window(class.as_ptr(), WS_EX_NOACTIVATE);
    if window.is_null() {
        return None;
    }
    exclude_from_capture(window);
    let (canvas, close_from) = draw(label, scale)?;
    BUBBLE.set(Some(Bubble {
        shared,
        window,
        canvas,
        close_from,
        margin: scaled(SCREEN_MARGIN, scale),
        // SAFETY: a system cursor, shared and never destroyed.
        cursor: unsafe { LoadCursorW(ptr::null_mut(), IDC_HAND) },
    }));
    // SAFETY: the window and this message loop belong to this thread, for as long as the program.
    unsafe {
        AddClipboardFormatListener(window);
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&raw mut message, ptr::null_mut(), 0, 0) > 0 {
            DispatchMessageW(&raw const message);
        }
    }
    Some(())
}

/// Listens to the clipboard for as long as the program runs. It waits for Windows between messages.
pub fn start(shared: Arc<Shared>, label: String) {
    thread::spawn(move || listen(shared, &label));
}
