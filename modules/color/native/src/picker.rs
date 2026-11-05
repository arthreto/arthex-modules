use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    ptr, slice,
};

use windows_sys::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
    Graphics::Gdi::{
        AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, BitBlt,
        CAPTUREBLT, CreateBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW,
        DIB_RGB_COLORS, DeleteDC, DeleteObject, EnumDisplayMonitors, FW_SEMIBOLD, GetDC,
        GetDeviceCaps, GetMonitorInfoW, HBITMAP, HDC, HFONT, HGDIOBJ, HMONITOR, LOGPIXELSX,
        MONITORINFO, RGBQUAD, ReleaseDC, SRCCOPY, SelectObject, SetBkMode, SetTextColor,
        TRANSPARENT, TextOutW,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
        Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE},
        WindowsAndMessaging::{
            CreateIconIndirect, CreateWindowExW, DefWindowProcW, DestroyCursor, DestroyWindow,
            DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics, HCURSOR, ICONINFO, MSG,
            PostQuitMessage, RegisterClassW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
            SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_SHOWNOACTIVATE, SetCursor,
            SetForegroundWindow, SetWindowDisplayAffinity, ShowWindow, ULW_ALPHA,
            UpdateLayeredWindow, WDA_EXCLUDEFROMCAPTURE, WM_ACTIVATEAPP, WM_KEYDOWN, WM_LBUTTONUP,
            WM_MOUSEMOVE, WM_RBUTTONUP, WM_SETCURSOR, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
};

use super::paint::{
    EYEDROPPER_TIP, color_at, edge_fade, eyedropper, finish_tag, hex, tag_base, tag_position,
};

const CURSOR_SIZE: i32 = 32;
const TAG_WIDTH: f32 = 128.0;
const TAG_HEIGHT: f32 = 32.0;
const TAG_GAP: f32 = 18.0;
const TAG_TEXT_LEFT: f32 = 34.0;
const TAG_FONT: f32 = 14.0;
/// `--text` of `src/ui/tokens.css`, as a Windows colour: 0x00BBGGRR.
const TAG_TEXT_COLOR: u32 = 0x00F6_F4F3;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn module() -> HINSTANCE {
    // SAFETY: asks for the handle of this program, which always exists.
    unsafe { GetModuleHandleW(ptr::null()) }
}

/// A 32-bit bitmap selected in its own drawing context, with its pixels in memory.
struct Canvas {
    context: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut c_void,
    width: i32,
    height: i32,
}

fn bitmap_info(width: i32, height: i32) -> BITMAPINFO {
    BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).unwrap_or_default(),
            biWidth: width,
            // Negative: the rows go from the top down, as everything here reads and writes them.
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD::default()],
    }
}

impl Canvas {
    fn new(width: i32, height: i32) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
        let info = bitmap_info(width, height);
        // SAFETY: the objects created here are released in `drop`; `info` outlives the call.
        unsafe {
            let screen = GetDC(ptr::null_mut());
            let context = CreateCompatibleDC(screen);
            ReleaseDC(ptr::null_mut(), screen);
            let mut bits = ptr::null_mut();
            let bitmap = CreateDIBSection(
                context,
                &raw const info,
                DIB_RGB_COLORS,
                &raw mut bits,
                ptr::null_mut(),
                0,
            );
            if bitmap.is_null() || bits.is_null() {
                DeleteDC(context);
                return None;
            }
            let previous = SelectObject(context, bitmap);
            Some(Self {
                context,
                bitmap,
                previous,
                bits,
                width,
                height,
            })
        }
    }

    fn length(&self) -> usize {
        usize::try_from(self.width).unwrap_or(0) * usize::try_from(self.height).unwrap_or(0) * 4
    }

    fn pixels(&self) -> &[u8] {
        // SAFETY: the bitmap holds `length` bytes for as long as `self` lives.
        unsafe { slice::from_raw_parts(self.bits.cast::<u8>(), self.length()) }
    }

    fn pixels_mut(&mut self) -> &mut [u8] {
        // SAFETY: as in `pixels`, and `&mut self` makes this the only access.
        unsafe { slice::from_raw_parts_mut(self.bits.cast::<u8>(), self.length()) }
    }

    /// Shows the bitmap as the content of a layered window.
    fn present(&self, window: HWND, x: i32, y: i32) {
        let position = POINT { x, y };
        let extent = SIZE {
            cx: self.width,
            cy: self.height,
        };
        let source = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: u8::try_from(AC_SRC_OVER).unwrap_or_default(),
            BlendFlags: 0,
            SourceConstantAlpha: u8::MAX,
            AlphaFormat: u8::try_from(AC_SRC_ALPHA).unwrap_or_default(),
        };
        // SAFETY: every pointer refers to a local that outlives the call.
        unsafe {
            let screen = GetDC(ptr::null_mut());
            UpdateLayeredWindow(
                window,
                screen,
                &raw const position,
                &raw const extent,
                self.context,
                &raw const source,
                0,
                &raw const blend,
                ULW_ALPHA,
            );
            ReleaseDC(ptr::null_mut(), screen);
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        // SAFETY: these objects were created in `new` and are used nowhere else.
        unsafe {
            SelectObject(self.context, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.context);
        }
    }
}

/// The screen as it is now, across every monitor.
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

    fn color_under(&self, cursor: POINT) -> Option<[u8; 3]> {
        color_at(
            self.canvas.pixels(),
            self.canvas.width.cast_unsigned(),
            self.canvas.height.cast_unsigned(),
            cursor.x - self.left,
            cursor.y - self.top,
        )
    }
}

/// One pixel of the screen as it is at this moment. The windows of the picker must be left out of
/// captures for this to be the real colour.
struct LiveReader {
    pixel: Canvas,
}

impl LiveReader {
    fn new() -> Option<Self> {
        Some(Self {
            pixel: Canvas::new(1, 1)?,
        })
    }

    fn color_at(&self, cursor: POINT) -> Option<[u8; 3]> {
        // SAFETY: both contexts are valid for this block and the size is that of the bitmap.
        let copied = unsafe {
            let screen = GetDC(ptr::null_mut());
            let copied = BitBlt(
                self.pixel.context,
                0,
                0,
                1,
                1,
                screen,
                cursor.x,
                cursor.y,
                SRCCOPY | CAPTUREBLT,
            );
            ReleaseDC(ptr::null_mut(), screen);
            copied
        };
        if copied == 0 {
            return None;
        }
        color_at(self.pixel.pixels(), 1, 1, 0, 0)
    }
}

/// Leaves a window out of what a capture of the screen sees. Windows before 10 version 2004 refuse.
fn exclude_from_capture(window: HWND) -> bool {
    // SAFETY: only changes how Windows treats our own window.
    unsafe { SetWindowDisplayAffinity(window, WDA_EXCLUDEFROMCAPTURE) != 0 }
}

/// What the window procedure needs; everything runs on the thread of `run`.
struct Session {
    copy: ScreenCopy,
    /// Reads the screen as it is now. Without it, what changed after the copy keeps its old colour.
    live: Option<LiveReader>,
    screens: Vec<RECT>,
    cursor: HCURSOR,
    tag: HWND,
    font: HFONT,
    scale: f32,
    picked: Cell<Option<String>>,
}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

fn cursor_position() -> POINT {
    let mut position = POINT { x: 0, y: 0 };
    // SAFETY: writes into a local.
    unsafe { GetCursorPos(&raw mut position) };
    position
}

#[allow(clippy::cast_possible_truncation)]
fn scaled(value: f32, scale: f32) -> i32 {
    (value * scale).round() as i32
}

fn draw_tag(session: &Session, color: [u8; 3]) -> Option<Canvas> {
    let (width, height) = (
        scaled(TAG_WIDTH, session.scale),
        scaled(TAG_HEIGHT, session.scale),
    );
    let mut canvas = Canvas::new(width, height)?;
    let (width_px, height_px) = (width.cast_unsigned(), height.cast_unsigned());
    canvas
        .pixels_mut()
        .copy_from_slice(&tag_base(width_px, height_px, color));
    let text: Vec<u16> = hex(color).encode_utf16().collect();
    let length = i32::try_from(text.len()).unwrap_or(0);
    // SAFETY: the context and font live through this block, and the text outlives the call.
    unsafe {
        let previous = SelectObject(canvas.context, session.font);
        SetBkMode(canvas.context, TRANSPARENT.cast_signed());
        SetTextColor(canvas.context, TAG_TEXT_COLOR);
        let top = (height - scaled(TAG_FONT * 1.3, session.scale)) / 2;
        TextOutW(
            canvas.context,
            scaled(TAG_TEXT_LEFT, session.scale),
            top,
            text.as_ptr(),
            length,
        );
        SelectObject(canvas.context, previous);
    }
    finish_tag(canvas.pixels_mut(), width_px, height_px);
    Some(canvas)
}

fn color_under_cursor(session: &Session, cursor: POINT) -> Option<[u8; 3]> {
    session
        .live
        .as_ref()
        .and_then(|live| live.color_at(cursor))
        .or_else(|| session.copy.color_under(cursor))
}

fn follow_cursor(session: &Session) {
    let cursor = cursor_position();
    let Some(color) = color_under_cursor(session, cursor) else {
        return;
    };
    let Some(canvas) = draw_tag(session, color) else {
        return;
    };
    let screen = session
        .screens
        .iter()
        .find(|screen| {
            (screen.left..screen.right).contains(&cursor.x)
                && (screen.top..screen.bottom).contains(&cursor.y)
        })
        .map_or((i32::MIN, i32::MIN, i32::MAX, i32::MAX), |screen| {
            (screen.left, screen.top, screen.right, screen.bottom)
        });
    let (x, y) = tag_position(
        (cursor.x, cursor.y),
        screen,
        (canvas.width, canvas.height),
        scaled(TAG_GAP, session.scale),
    );
    canvas.present(session.tag, x, y);
}

fn finish(session: &Session, picked: Option<String>) {
    session.picked.set(picked);
    // SAFETY: ends the message loop of this thread.
    unsafe { PostQuitMessage(0) };
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
                // SAFETY: the cursor lives as long as the session.
                unsafe { SetCursor(session.cursor) };
                Some(1)
            }
            WM_MOUSEMOVE => {
                follow_cursor(session);
                Some(0)
            }
            WM_LBUTTONUP => {
                let color = color_under_cursor(session, cursor_position());
                finish(session, color.map(hex));
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
            // Another program took over: the picker must never stay in the way.
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
    let mut info = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or_default(),
        rcMonitor: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        rcWork: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
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

fn create_eyedropper() -> HCURSOR {
    let size = u16::try_from(CURSOR_SIZE).unwrap_or(32);
    let pixels = eyedropper(size);
    let info = bitmap_info(CURSOR_SIZE, CURSOR_SIZE);
    // SAFETY: the bitmaps are created and released here, and a cursor keeps its own copies. The
    // color bitmap holds `CURSOR_SIZE` squared pixels of 4 bytes, which is what `eyedropper` returns.
    unsafe {
        let mut bits = ptr::null_mut();
        let color = CreateDIBSection(
            ptr::null_mut(),
            &raw const info,
            DIB_RGB_COLORS,
            &raw mut bits,
            ptr::null_mut(),
            0,
        );
        if color.is_null() || bits.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len());
        let mask = CreateBitmap(CURSOR_SIZE, CURSOR_SIZE, 1, 1, ptr::null());
        let hot_x = EYEDROPPER_TIP.0 / 24.0 * f32::from(size);
        let hot_y = EYEDROPPER_TIP.1 / 24.0 * f32::from(size);
        let icon = ICONINFO {
            fIcon: 0,
            xHotspot: scaled(hot_x, 1.0).cast_unsigned(),
            yHotspot: scaled(hot_y, 1.0).cast_unsigned(),
            hbmMask: mask,
            hbmColor: color,
        };
        let cursor = CreateIconIndirect(&raw const icon);
        DeleteObject(mask);
        DeleteObject(color);
        cursor
    }
}

fn create_window(class: *const u16, extended: u32) -> HWND {
    // SAFETY: the class name outlives the call and a null window only makes the caller stop.
    unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | extended,
            class,
            ptr::null(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            module(),
            ptr::null(),
        )
    }
}

fn main_screen_scale() -> f32 {
    // SAFETY: reads the scale of the main screen from a context released at once.
    let dpi = unsafe {
        let screen = GetDC(ptr::null_mut());
        let dpi = GetDeviceCaps(screen, LOGPIXELSX.cast_signed());
        ReleaseDC(ptr::null_mut(), screen);
        dpi
    };
    #[allow(clippy::cast_precision_loss)]
    let scale = dpi as f32 / 96.0;
    scale
}

fn tag_font(scale: f32) -> HFONT {
    let font_name = wide("Cascadia Mono");
    // SAFETY: the name outlives the call; a missing font falls back to a system one.
    unsafe {
        CreateFontW(
            -scaled(TAG_FONT, scale),
            0,
            0,
            0,
            FW_SEMIBOLD.cast_signed(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            font_name.as_ptr(),
        )
    }
}

/// Covers every screen with the fade until the user clicks or cancels, on the calling thread.
pub fn run() -> Option<String> {
    // A program is scaled by Windows unless it says otherwise: on a screen at 125 %, the cursor,
    // the windows and the copy of the screen would not line up, and the wrong pixel would be read.
    // SAFETY: changes how Windows treats this process; refused harmlessly if already set.
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let copy = ScreenCopy::take()?;
    let class = wide("ArthexScreenColor");
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
    let scale = main_screen_scale();
    let font = tag_font(scale);
    let tag = create_window(class.as_ptr(), WS_EX_TRANSPARENT | WS_EX_NOACTIVATE);
    let cursor = create_eyedropper();
    let fades: Vec<HWND> = screens
        .iter()
        .map(|_| create_window(class.as_ptr(), 0))
        .collect();
    let first = fades.first().copied().filter(|first| !first.is_null());
    let ready = first.is_some() && !tag.is_null() && !cursor.is_null();

    let mut picked = None;
    if ready {
        let hidden_from_captures = fades
            .iter()
            .chain(std::iter::once(&tag))
            .all(|window| exclude_from_capture(*window));
        SESSION.set(Some(Session {
            copy,
            live: hidden_from_captures.then(LiveReader::new).flatten(),
            screens: screens.clone(),
            cursor,
            tag,
            font,
            scale,
            picked: Cell::new(None),
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
        SESSION.with_borrow(|session| {
            if let Some(session) = session {
                follow_cursor(session);
            }
        });
        // SAFETY: the windows, the session and this message loop all belong to this thread. The
        // overlay had the focus until a moment ago, so Windows lets us take it for Escape.
        unsafe {
            ShowWindow(tag, SW_SHOWNOACTIVATE);
            if let Some(first) = first {
                SetForegroundWindow(first);
                SetFocus(first);
            }
            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&raw mut message, ptr::null_mut(), 0, 0) > 0 {
                DispatchMessageW(&raw const message);
            }
        }
        picked = SESSION.take().and_then(|session| session.picked.take());
    } else {
        drop(copy);
    }

    // SAFETY: everything here was created above; destroying a null handle only fails.
    unsafe {
        for window in fades {
            DestroyWindow(window);
        }
        DestroyWindow(tag);
        DestroyCursor(cursor);
        DeleteObject(font);
    }
    picked
}
