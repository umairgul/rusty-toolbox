//! Colour-manages the window as sRGB on macOS.
//!
//! egui draws sRGB colors, but wgpu leaves the window's Metal layer without a color space, and
//! macOS then shows the pixels as the display's native colors. On a display that isn't sRGB (a
//! MacBook's P3 screen, many external monitors) every color comes out slightly off. Tagging
//! the window as sRGB makes macOS convert it like any other app, so swatches show their true
//! colors and the eyedropper reads back the same values Toolbox drew.

/// Call once, from the app creator, while the window exists.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn use_srgb(cc: &eframe::CreationContext<'_>) {
    use objc2_app_kit::{NSColorSpace, NSView};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = cc.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };
    // SAFETY: on macOS, eframe's window handle points to the window's content NSView, which
    // lives as long as the window. The app creator runs on the main thread, as AppKit requires.
    let view: &NSView = unsafe { appkit.ns_view.cast().as_ref() };
    if let Some(window) = view.window() {
        window.setColorSpace(Some(&NSColorSpace::sRGBColorSpace()));
    }
}

#[cfg(not(target_os = "macos"))]
pub fn use_srgb(_cc: &eframe::CreationContext<'_>) {}
