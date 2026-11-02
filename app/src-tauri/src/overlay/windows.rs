//! The overlay's window on Windows: a layered window, which takes the picture with its transparency at each update,
//! lets the mouse through, never takes the focus and stays out of the taskbar and Alt+Tab.

use super::draw::Image;
use std::mem;
use std::ptr::{null, null_mut};
use std::slice;
use windows_sys::Win32::Foundation::{GetLastError, HWND, POINT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
    CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, HWND_TOPMOST, RegisterClassW, SW_HIDE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_SHOWWINDOW, SetWindowPos, ShowWindow, ULW_ALPHA, UpdateLayeredWindow, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows_sys::w;

pub struct NativeWindow {
    window: HWND,
    /// The picture's memory, kept while its size stays the same
    canvas: Option<Canvas>,
    shown: bool,
    /// Whether a failed update was already reported, so a lasting failure is reported once
    failing: bool,
}

/// A bitmap the window's pictures are copied into, selected in a memory device context
struct Canvas {
    memory: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    pixels: *mut u8,
    width: i32,
    height: i32,
}

impl NativeWindow {
    pub fn new() -> Option<Self> {
        let class_name = w!("DrezimoOverlay");
        unsafe {
            let instance = GetModuleHandleW(null());
            let class = WNDCLASSW {
                lpfnWndProc: Some(DefWindowProcW),
                hInstance: instance,
                lpszClassName: class_name,
                ..mem::zeroed()
            };
            // Fails harmlessly when the class is already there, from a window made before
            RegisterClassW(&class);
            let window = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_name,
                w!("Drezimo overlay"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                instance,
                null(),
            );
            (!window.is_null()).then_some(Self {
                window,
                canvas: None,
                shown: false,
                failing: false,
            })
        }
    }

    /// Shows `image` with its top-left corner at (x, y), in pixels.
    pub fn show(&mut self, image: &Image, x: i32, y: i32, _scale: f64, opacity: f32) {
        let (width, height) = (image.width as i32, image.height as i32);
        let fits = self
            .canvas
            .as_ref()
            .is_some_and(|canvas| (canvas.width, canvas.height) == (width, height));
        if !fits {
            // The old bitmap goes first: two at once could fail where one fits
            self.canvas = None;
            self.canvas = Canvas::new(width, height);
        }
        let Some(canvas) = &self.canvas else {
            self.report("cannot make the overlay's bitmap");
            return;
        };

        let target = unsafe { slice::from_raw_parts_mut(canvas.pixels, image.pixels.len()) };
        for (to, from) in target
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(image.pixels.as_chunks::<4>().0)
        {
            *to = [from[2], from[1], from[0], from[3]];
        }
        let position = POINT { x, y };
        let size = SIZE { cx: width, cy: height };
        let origin = POINT { x: 0, y: 0 };
        let blend = blend(opacity);
        let updated = unsafe {
            UpdateLayeredWindow(
                self.window,
                null_mut(),
                &position,
                &size,
                canvas.memory,
                &origin,
                0,
                &blend,
                ULW_ALPHA,
            )
        };
        if updated == 0 {
            self.report("cannot update the overlay");
            return;
        }
        self.failing = false;
        if !self.shown {
            // Above the windows made topmost since, without taking the focus
            let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW;
            unsafe { SetWindowPos(self.window, HWND_TOPMOST, 0, 0, 0, 0, flags) };
            self.shown = true;
        }
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        let blend = blend(opacity);
        unsafe {
            UpdateLayeredWindow(
                self.window,
                null_mut(),
                null(),
                null(),
                null_mut(),
                null(),
                0,
                &blend,
                ULW_ALPHA,
            );
        }
    }

    pub fn hide(&mut self) {
        unsafe { ShowWindow(self.window, SW_HIDE) };
        self.shown = false;
    }

    fn report(&mut self, what: &str) {
        if !self.failing {
            let code = unsafe { GetLastError() };
            eprintln!("{what} (Windows error {code})");
            self.failing = true;
        }
    }
}

impl Canvas {
    fn new(width: i32, height: i32) -> Option<Self> {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                // Negative: the rows from the top
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..unsafe { mem::zeroed() }
            },
            ..unsafe { mem::zeroed() }
        };
        unsafe {
            let memory = CreateCompatibleDC(null_mut());
            if memory.is_null() {
                return None;
            }
            let mut pixels = null_mut();
            let bitmap = CreateDIBSection(memory, &info, DIB_RGB_COLORS, &mut pixels, null_mut(), 0);
            if bitmap.is_null() || pixels.is_null() {
                DeleteDC(memory);
                return None;
            }
            let previous = SelectObject(memory, bitmap);
            Some(Self {
                memory,
                bitmap,
                previous,
                pixels: pixels.cast(),
                width,
                height,
            })
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.memory, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.memory);
        }
    }
}

fn blend(opacity: f32) -> BLENDFUNCTION {
    BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: (opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
        AlphaFormat: AC_SRC_ALPHA as u8,
    }
}
