//! The overlay's window on macOS: borderless, letting the mouse through, on every Space, showing the picture as its
//! layer's contents.

use super::draw::Image;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSScreen, NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_core_foundation::{CFData, CFRetained, CFType, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo, kCGColorSpaceSRGB,
};
use std::ptr;

pub struct NativeWindow {
    window: Retained<NSWindow>,
    main_thread: MainThreadMarker,
    /// Where the window is: its top-left corner and size in pixels, and the scale
    placed: Option<(i32, i32, usize, usize, u64)>,
    visible: bool,
}

impl NativeWindow {
    pub fn new() -> Option<Self> {
        let main_thread = MainThreadMarker::new()?;
        let frame = CGRect::new(CGPoint::ZERO, CGSize::new(1.0, 1.0));
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                main_thread.alloc(),
                frame,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // Owned here: closing it must not free it behind this handle
        unsafe { window.setReleasedWhenClosed(false) };
        window.setOpaque(false);
        window.setBackgroundColor(Some(&NSColor::clearColor()));
        window.setHasShadow(false);
        window.setIgnoresMouseEvents(true);
        window.setLevel(NSStatusWindowLevel);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        window.contentView()?.setWantsLayer(true);
        Some(Self {
            window,
            main_thread,
            placed: None,
            visible: false,
        })
    }

    /// Shows `image` with its top-left corner at (x, y), in the pixels of the monitors' positions. Whether the window
    /// has just come up, in front of the others.
    pub fn show(&mut self, image: &Image, x: i32, y: i32, scale: f64, opacity: f32) -> bool {
        let Some(layer) = self.window.contentView().and_then(|view| view.layer()) else {
            return false;
        };
        let Some(picture) = picture(image) else {
            return false;
        };
        let contents: &CFType = &picture;
        unsafe { layer.setContents(Some(contents.as_ref())) };

        let place = (x, y, image.width, image.height, scale.to_bits());
        if self.placed != Some(place) {
            layer.setContentsScale(scale);
            // Cocoa counts from the bottom of the main screen, the monitors' positions from its top
            let screens = NSScreen::screens(self.main_thread);
            let main_height = screens.firstObject().map_or(0.0, |screen| screen.frame().size.height);
            let size = CGSize::new(image.width as f64 / scale, image.height as f64 / scale);
            let origin = CGPoint::new(x as f64 / scale, main_height - y as f64 / scale - size.height);
            self.window.setFrame_display(CGRect::new(origin, size), false);
            self.placed = Some(place);
        }
        self.window.setAlphaValue(opacity.into());
        let appears = !self.visible;
        if appears {
            self.window.orderFrontRegardless();
            self.visible = true;
        }
        appears
    }

    /// In front of the other windows at its level, if it shows.
    pub fn raise(&mut self) {
        if self.visible {
            self.window.orderFrontRegardless();
        }
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.window.setAlphaValue(opacity.into());
    }

    pub fn hide(&mut self) {
        self.window.orderOut(None);
        self.visible = false;
    }
}

fn picture(image: &Image) -> Option<CFRetained<CGImage>> {
    let data = CFData::from_bytes(&image.pixels);
    let provider = CGDataProvider::with_cf_data(Some(&data))?;
    let srgb = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }))?;
    unsafe {
        CGImage::new(
            image.width,
            image.height,
            8,
            32,
            image.width * 4,
            Some(&srgb),
            CGBitmapInfo(CGImageAlphaInfo::PremultipliedLast.0),
            Some(&provider),
            ptr::null(),
            false,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    }
}
