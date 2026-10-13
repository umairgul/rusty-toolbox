<p align="center">
  <img src="assets/icon/toolbox-512.png" alt="Toolbox icon" width="128" height="128">
</p>

<h1 align="center">Toolbox</h1>

<p align="center">
  <strong>27 everyday tools for developers and designers in one fast, native desktop app.</strong><br>
  Built in Rust with <a href="https://github.com/emilk/egui">egui</a>. Works offline, and your data never leaves your computer.
</p>

<p align="center">
  <img alt="Rust 2024 edition" src="https://img.shields.io/badge/rust-2024_edition-orange?logo=rust">
  <img alt="Platform: macOS" src="https://img.shields.io/badge/platform-macOS_11%2B-lightgrey?logo=apple">
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue">
</p>

---

## Table of contents

- [Why Toolbox](#why-toolbox)
- [Features](#features)
- [Installation](#installation)
- [Building from source](#building-from-source)
- [Project structure](#project-structure)
- [Architecture](#architecture)
- [Development guide](#development-guide)
  - [Everyday commands](#everyday-commands)
  - [Adding a new tool](#adding-a-new-tool)
  - [Testing](#testing)
  - [Code conventions](#code-conventions)
  - [Updating the app icon](#updating-the-app-icon)
- [Releasing for macOS](#releasing-for-macos)
- [Platform support](#platform-support)
- [Contributing](#contributing)
- [License](#license)

## Why Toolbox

Developers often paste tokens, passwords and customer data into random websites just to decode a JWT or format some JSON. Toolbox puts those utilities in one native app instead:

- **Private.** Everything runs on your machine, with no network calls, telemetry or accounts.
- **Fast.** It's a native Rust binary that starts quickly and stays responsive with large inputs.
- **Exact.** Formatters keep your numbers, escapes and key order as written, and lossless operations stay lossless.
- **One place.** Generators, converters, formatters, color tools and image tools share one sidebar.

## Features

### Generators

| Tool | What it does |
| --- | --- |
| **Passwords** | Random passwords and memorable passphrases (EFF wordlist) from a cryptographically secure RNG, with an entropy-based strength meter. |
| **Names** | Random names for projects, usernames, test people and companies. |
| **IDs** | UUIDs (v4, v7), ULIDs and Nano IDs, one at a time or in bulk. |
| **QR Codes** | QR codes for text, links and Wi-Fi networks. Export as PNG or SVG. |
| **Lorem Ipsum** | Placeholder text by paragraph, sentence or word, as plain text or HTML. |
| **Fake Data** | Realistic test records for seeding databases, exported as CSV, JSON or SQL. Emails and phone numbers are fictional. |

### Encoders & converters

| Tool | What it does |
| --- | --- |
| **Hashes** | MD5, SHA-1, SHA-2 and BLAKE3 hashes of text or files, plus checksum verification. |
| **Encode / Decode** | Base64, URL percent-encoding, HTML entities and hex. |
| **Timestamps** | Convert between Unix timestamps and dates, in any IANA time zone. |
| **JWT Decoder** | Decode JSON Web Tokens, check expiry and verify HMAC signatures, all offline. |
| **Data Formats** | Convert between JSON, YAML, TOML and CSV. |
| **Number Bases** | Convert integers between decimal, hex, octal, binary and any base up to 36. |
| **Text Case** | Convert identifiers between camelCase, snake_case, kebab-case and more, line by line. |
| **Cron** | Explain a cron expression in plain English and list its next run times. |

### Formatters & testers

| Tool | What it does |
| --- | --- |
| **JSON Formatter** | Format, minify and validate JSON. Syntax errors include their position. |
| **Regex Tester** | Live match highlighting, capture groups and replace. Supports lookaround and backreferences. |
| **Text Diff** | Compare two texts side by side, with changed words highlighted. |
| **SQL Formatter** | Format SQL for reading, or compact it onto one line. |
| **Markdown Preview** | Write Markdown with a live preview, then copy or save the result as HTML. |

### Color & design

| Tool | What it does |
| --- | --- |
| **Color Picker** | Pick a color anywhere on screen with the eyedropper, or enter any CSS color, and get it in every format. |
| **Contrast Checker** | Check foreground and background colors against WCAG 2 levels and get passing alternatives. |
| **Palettes** | Build a 50–950 shade scale and color harmonies from one color. Click a swatch to copy it. |
| **Gradients** | Blend colors in OKLab, OKLCH or sRGB, then copy the CSS or the intermediate steps. |

### Images

| Tool | What it does |
| --- | --- |
| **Image Converter** | Convert between PNG, JPEG, WebP, AVIF, GIF, BMP, ICO and TIFF, with bulk resizing and compression. Also opens HEIC and SVG. |
| **Metadata Remover** | See what a photo reveals (camera, date, GPS location) and save a clean copy. JPEG, PNG and WebP are cleaned without re-encoding, so quality is unchanged. |
| **Icon Generator** | Make website favicons, a macOS `.icns`, a Windows `.ico` and PNGs at every size from one image or SVG. |
| **Base64 Images** | Turn images into data URIs for HTML, CSS and Markdown, or turn data URIs back into image files. |

## Installation

### macOS (prebuilt)

1. Download `Toolbox-<version>.dmg` from the [Releases](https://github.com/umairgul/rusty-toolbox/releases) page.
2. Open the disk image and drag **Toolbox** into **Applications**.

Toolbox needs macOS 11 (Big Sur) or later and runs natively on both Apple silicon and Intel.

> **Note:** If a build isn't signed with a Developer ID, macOS Gatekeeper warns on first launch. Right-click the app, choose **Open**, then confirm.

### From source

See [Building from source](#building-from-source) below.

## Building from source

### Prerequisites

- **Rust**, a recent stable toolchain that supports the 2024 edition (1.85 or newer), installed with [rustup](https://rustup.rs).
- **macOS:** Xcode Command Line Tools (`xcode-select --install`).
- **Linux:** the system libraries egui/eframe needs for windowing and OpenGL, such as the X11/Wayland and `libxkbcommon` development packages.

### Build and run

```sh
git clone https://github.com/umairgul/rusty-toolbox.git
cd rusty-toolbox

# Run a development build
cargo run -p toolbox

# Run an optimized build
cargo run -p toolbox --release
```

Dependencies are compiled with optimizations even in debug builds (see `[profile.dev.package."*"]` in `Cargo.toml`), so development builds stay responsive while your own code compiles quickly.

## Project structure

Toolbox is a Cargo workspace with three members:

```
rusty-toolbox/
├── apps/
│   └── toolbox/                 # Desktop app (egui/eframe): the window, sidebar and tool UIs
│       └── src/
│           ├── main.rs          # App entry point, layout and UI tests
│           ├── eyedropper.rs    # Screen color sampling (macOS, NSColorSampler)
│           ├── color_space.rs   # Tags the window as sRGB on macOS
│           └── tools/           # One module per tool, plus shared widgets (mod.rs)
├── crates/
│   └── toolbox-core/            # All tool logic, with no GUI dependencies
│       ├── data/                # Bundled data (EFF passphrase wordlist)
│       └── src/                 # One module per tool: hash, jwt, cron, color/, images/, ...
├── xtask/                       # Project automation (`cargo xtask <command>`)
├── scripts/
│   └── build-macos.sh           # Builds Toolbox.app and a signed, notarized .dmg
├── assets/icon/                 # App icon source (SVG) and rendered PNG/ICNS
├── .cargo/config.toml           # Defines the `cargo xtask` alias
└── Cargo.toml                   # Workspace manifest: shared dependencies, lints and profiles
```

## Architecture

The code is split into two layers with a strict boundary:

```
┌──────────────────────────────────────────────┐
│  apps/toolbox (egui)                         │
│  Sidebar · tool UIs · file dialogs · copy    │
└──────────────────────┬───────────────────────┘
                       │ calls plain Rust functions
┌──────────────────────▼───────────────────────┐
│  crates/toolbox-core                         │
│  Hashing · encoding · parsing · color math · │
│  image processing · generators               │
└──────────────────────────────────────────────┘
```

- **`toolbox-core`** holds every algorithm and data transformation. It has no UI dependencies, so it is fully unit-tested and could back another frontend such as a CLI, a web build or a different GUI toolkit.
- **`toolbox`** (the app) is a thin presentation layer. Each tool implements the `Tool` trait, owns its UI state and calls into `toolbox-core` to do the work.

```rust
pub trait Tool {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn category(&self) -> Category;
    fn ui(&mut self, ui: &mut egui::Ui);
}
```

`tools::all()` returns every tool. The sidebar groups them by `Category` (Generators, Encoders & Converters, Formatters & Testers, Color & Design, Images) in the order they appear in that list.

## Development guide

### Everyday commands

| Task | Command |
| --- | --- |
| Run the app | `cargo run -p toolbox` |
| Run all tests | `cargo test --workspace` |
| Test only the core logic | `cargo test -p toolbox-core` |
| Lint | `cargo clippy --workspace --all-targets` |
| Format | `cargo fmt --all` |
| Regenerate the app icon | `cargo xtask icon` |
| Build a macOS release | `scripts/build-macos.sh` |

> `xtask` isn't a default workspace member, so plain `cargo build` and `cargo test` skip it. Use `--workspace` to include it.

### Adding a new tool

1. **Write the logic in `toolbox-core`.** Add `crates/toolbox-core/src/my_tool.rs`, export it from `lib.rs` with `pub mod my_tool;`, and cover it with unit tests in the same file:

   ```rust
   pub fn transform(input: &str) -> Result<String, Error> { /* ... */ }

   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn transforms_simple_input() {
           assert_eq!(transform("abc").unwrap(), "...");
       }
   }
   ```

   If the tool needs a new crate, add it under `[workspace.dependencies]` in the root `Cargo.toml`, then reference it with `name.workspace = true` in the member's `Cargo.toml`.

2. **Build the UI in the app.** Add `apps/toolbox/src/tools/my_tool.rs`:

   ```rust
   use super::{Category, Tool};

   #[derive(Default)]
   pub struct MyTool {
       input: String,
   }

   impl Tool for MyTool {
       fn name(&self) -> &'static str { "My Tool" }
       fn description(&self) -> &'static str { "One sentence on what it does." }
       fn category(&self) -> Category { Category::Converters }

       fn ui(&mut self, ui: &mut egui::Ui) {
           ui.text_edit_multiline(&mut self.input);
           match toolbox_core::my_tool::transform(&self.input) {
               Ok(output) => super::output_box(ui, "my_tool_output", &output, 240.0),
               Err(error) => super::error_label(ui, error),
           }
       }
   }
   ```

3. **Register it.** Add `mod my_tool;` to `apps/toolbox/src/tools/mod.rs` and a `Box::new(my_tool::MyTool::default())` entry to `all()`, positioned where it should appear within its category.

4. **Reuse the shared widgets** in `tools/mod.rs` rather than writing your own:

   | Helper | Use it for |
   | --- | --- |
   | `copy_button` | A copy button that briefly reads "Copied!" |
   | `output_box` | Read-only, selectable monospace output |
   | `value_rows` | A table of label, copy button and value |
   | `error_label` / `status_label` | Error messages and green or red status lines |
   | `zone_picker` | A searchable IANA time zone picker |
   | `save_file` | A native save dialog that writes bytes |
   | `strength_bar` | An entropy-based strength meter |
   | `human_size` | File sizes such as "1.5 MB" |

   The color and image tools also share widgets in `color_widgets.rs` and `image_widgets.rs`.

5. **Run the tests.** `every_tool_renders` opens your new tool automatically, so UI panics surface without any extra test code.

### Testing

The suite has two layers:

- **Core unit tests** (`crates/toolbox-core`): over 130 tests covering parsing, encoding, hashing, color math, image conversion and more. QR output is verified by decoding it again with `rqrr`.
- **UI tests** (`apps/toolbox/src/main.rs`), built on [`egui_kittest`](https://github.com/emilk/egui/tree/master/crates/egui_kittest):
  - `every_tool_renders` opens every tool and draws several frames to catch panics in UI code.
  - `ui_text_uses_only_drawable_characters` fails if UI source contains a character egui's built-in fonts can't draw, which would otherwise appear as an empty box.

To save a screenshot of every tool, which is useful for reviewing UI changes or updating documentation, run:

```sh
mkdir -p screenshots
TOOLBOX_SCREENSHOTS=screenshots cargo test -p toolbox every_tool_renders
```

Screenshots require a GPU adapter (via `wgpu`).

### Code conventions

- **Keep logic out of the UI.** Anything that can be unit-tested belongs in `toolbox-core`.
- **No `unsafe` by default.** `unsafe_code` is denied workspace-wide. The only exceptions are the two small macOS FFI modules (`eyedropper.rs` and `color_space.rs`), and each opts in with a documented `#[allow(unsafe_code)]`.
- **Clippy** warns on leftover `dbg!` and `todo!`. Keep `cargo clippy --workspace --all-targets` clean.
- **Shared dependencies** are declared once in `[workspace.dependencies]`, so every crate uses the same versions.
- **UI text** should only use characters egui's fonts can draw. The test suite enforces this.
- **Builds are reproducible.** `Cargo.lock` is committed, and release builds use `--locked`.

### Updating the app icon

The icon's source is `assets/icon/toolbox.svg`. After editing it, run:

```sh
cargo xtask icon
```

This renders `assets/icon/toolbox-512.png`, which is embedded in the binary as the window icon. On macOS it also builds `assets/icon/Toolbox.icns` for the app bundle. Commit the regenerated files.

## Releasing for macOS

`scripts/build-macos.sh` builds `dist/Toolbox.app` and `dist/Toolbox-<version>.dmg`. The version comes from `[workspace.package]` in `Cargo.toml`.

```sh
# Universal binary (Apple silicon + Intel), ad-hoc signed, with a disk image
scripts/build-macos.sh

# Faster local build for this Mac's CPU only, without a disk image
scripts/build-macos.sh --native --no-dmg
```

For a build you can distribute, sign it with a Developer ID and notarize it:

```sh
BUNDLE_ID=com.yourcompany.toolbox \
SIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)" \
NOTARY_PROFILE=toolbox-notary \
scripts/build-macos.sh
```

| Variable | Purpose |
| --- | --- |
| `BUNDLE_ID` | Reverse-DNS app identifier. Defaults to `com.example.toolbox`; set it to a domain you own before distributing. |
| `SIGN_IDENTITY` | Code-signing identity. Without it the app is ad-hoc signed and only runs without warnings on the Mac that built it. |
| `NOTARY_PROFILE` | Keychain profile created with `xcrun notarytool store-credentials`. Requires `SIGN_IDENTITY`. The disk image is notarized and stapled. |

The script installs any missing Rust targets with `rustup`, renders the icon if it's missing, and verifies the signature before it finishes. Run `scripts/build-macos.sh --help` to see all options.

## Platform support

Toolbox is developed and released for **macOS 11+**. The core library and most of the UI are platform-independent, but these features depend on macOS:

| Feature | macOS | Other platforms |
| --- | --- | --- |
| Screen eyedropper (Color Picker) | Supported (`NSColorSampler`) | Unavailable; colors can still be typed in |
| Opening HEIC/HEIF images | Supported (via `sips`) | Unsupported |
| `.icns` generation in `cargo xtask icon` | Supported (via `iconutil`) | Skipped; the PNG icon is still rendered |
| sRGB-tagged window | Supported | Not applicable |
| `.app` / `.dmg` packaging | Supported | Not provided |

## Contributing

Contributions are welcome. Before opening a pull request:

1. Put new logic in `toolbox-core` and give it unit tests.
2. Run `cargo fmt --all`, `cargo clippy --workspace --all-targets` and `cargo test --workspace`, and make sure all three pass cleanly.
3. Keep each commit focused, with a message that explains the change.
4. For UI changes, include before and after screenshots (see [Testing](#testing)).

Use [GitHub Issues](https://github.com/umairgul/rusty-toolbox/issues) to report a bug or suggest a tool.

## License

Toolbox is dual-licensed under either of the following, at your option:

- MIT License ([`LICENSE-MIT`](LICENSE-MIT))
- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))

Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in this work, as defined in the Apache-2.0 license, is dual-licensed as above, without any additional terms or conditions.

### Third-party data

The passphrase generator uses the [EFF Long Wordlist](https://www.eff.org/dice) by the Electronic Frontier Foundation, licensed under [CC BY 3.0 US](https://creativecommons.org/licenses/by/3.0/us/). See [`NOTICE`](NOTICE) for details.
