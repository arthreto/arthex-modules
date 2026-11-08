use std::{ffi::c_void, ptr, slice};

use windows_sys::Win32::{
    Foundation::{HINSTANCE, HWND, POINT, SIZE},
    Graphics::Gdi::{
        AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
        CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
        GetDeviceCaps, HBITMAP, HDC, HGDIOBJ, LOGPIXELSX, RGBQUAD, ReleaseDC, SelectObject,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, GetCursorPos, ULW_ALPHA, UpdateLayeredWindow, WS_EX_LAYERED,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    },
};

pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

pub fn module() -> HINSTANCE {
    // SAFETY: asks for the handle of this program, which always exists.
    unsafe { GetModuleHandleW(ptr::null()) }
}

pub fn cursor_position() -> POINT {
    let mut position = POINT { x: 0, y: 0 };
    // SAFETY: writes into a local.
    unsafe { GetCursorPos(&raw mut position) };
    position
}

#[allow(clippy::cast_possible_truncation)]
pub fn scaled(value: f32, scale: f32) -> i32 {
    (value * scale).round() as i32
}

pub fn main_screen_scale() -> f32 {
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

pub fn create_window(class: *const u16, extended: u32) -> HWND {
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

/// A 32-bit bitmap selected in its own drawing context, with its pixels in memory.
pub struct Canvas {
    pub context: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut c_void,
    pub width: i32,
    pub height: i32,
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
    pub fn new(width: i32, height: i32) -> Option<Self> {
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

    pub fn pixels(&self) -> &[u8] {
        // SAFETY: the bitmap holds `length` bytes for as long as `self` lives.
        unsafe { slice::from_raw_parts(self.bits.cast::<u8>(), self.length()) }
    }

    pub fn pixels_mut(&mut self) -> &mut [u8] {
        // SAFETY: as in `pixels`, and `&mut self` makes this the only access.
        unsafe { slice::from_raw_parts_mut(self.bits.cast::<u8>(), self.length()) }
    }

    /// Shows the bitmap as the content of a layered window.
    pub fn present(&self, window: HWND, x: i32, y: i32) {
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
