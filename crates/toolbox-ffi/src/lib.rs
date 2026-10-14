//! UniFFI bindings for toolbox-core, used by the SwiftUI app in `macos/`.
//!
//! Each module wraps one tool's core API in FFI-friendly types: records instead of borrowed
//! data, enums without data where Swift needs `CaseIterable`, and [`ToolError`] for failures.
//! The tool logic itself stays in toolbox-core.

uniffi::setup_scaffolding!();

mod color;
mod contrast;
mod cron;
mod data_formats;
mod encode;
mod error;
mod fake_data;
mod gradients;
mod hashes;
mod icon_generator;
mod ids;
mod image_base64;
mod image_converter;
mod image_metadata;
mod images;
mod json;
mod jwt;
mod lorem;
mod markdown;
mod names;
mod number_base;
mod palettes;
mod password;
mod qr;
mod regex_tester;
mod sql_formatter;
mod text_case;
mod text_diff;
mod time_zones;
mod timestamps;

pub use error::ToolError;
