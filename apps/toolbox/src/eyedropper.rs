//! Screen eyedropper: the user clicks any pixel on screen and gets its color.
//!
//! On macOS this is AppKit's `NSColorSampler`, the magnifying loupe from the system color
//! panel. It needs no screen-recording permission. Other platforms don't support it yet.

use std::sync::{Arc, Mutex};
use toolbox_core::color::Color;

/// Whether this platform has an eyedropper.
pub const SUPPORTED: bool = cfg!(target_os = "macos");

/// The result of a sampling session: `Some(Some(color))` when picked, `Some(None)` when
/// cancelled, `None` while still running.
type Slot = Arc<Mutex<Option<Option<Color>>>>;

#[derive(Default)]
pub struct Eyedropper {
    slot: Slot,
    active: bool,
}

impl Eyedropper {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn start(&mut self, ctx: &egui::Context) {
        if !self.active {
            self.active = true;
            platform::start(self.slot.clone(), ctx.clone());
        }
    }

    /// The picked color, once the session has ended. Cancelling yields nothing.
    pub fn poll(&mut self) -> Option<Color> {
        let result = self.slot.lock().ok()?.take()?;
        self.active = false;
        // Screen pixels are 8-bit; the system reports them as fractions like 0.99998, so round
        // to the 8-bit value to match hex codes and named colors exactly.
        result.map(|color| {
            let [r, g, b, a] = color.to_rgba8();
            Color::from_rgba8(r, g, b, a)
        })
    }
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod platform {
    use super::Slot;
    use block2::RcBlock;
    use objc2_app_kit::{NSColor, NSColorSampler, NSColorSpace};
    use toolbox_core::color::Color;

    pub fn start(slot: Slot, ctx: egui::Context) {
        let handler = RcBlock::new(move |picked: *mut NSColor| {
            // SAFETY: AppKit passes nil when the user cancels, and otherwise a valid NSColor
            // that stays alive for the duration of this call.
            let picked = unsafe { picked.as_ref() };
            let color = picked
                .and_then(|color| color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace()))
                .map(|srgb| Color {
                    r: srgb.redComponent(),
                    g: srgb.greenComponent(),
                    b: srgb.blueComponent(),
                    a: srgb.alphaComponent(),
                });
            if let Ok(mut slot) = slot.lock() {
                *slot = Some(color);
            }
            ctx.request_repaint();
        });
        let sampler = NSColorSampler::new();
        // SAFETY: egui runs the UI on the main thread on macOS, as AppKit requires. AppKit
        // copies and retains the handler block until the session ends, and the sampler keeps
        // itself alive for the session too.
        unsafe { sampler.showSamplerWithSelectionHandler(&handler) };
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    /// Unsupported: end the session at once, as if cancelled.
    pub fn start(slot: super::Slot, _ctx: egui::Context) {
        if let Ok(mut slot) = slot.lock() {
            *slot = Some(None);
        }
    }
}
