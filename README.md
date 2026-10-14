<p align="center">
  <img src="assets/icon/toolbox-512.png" alt="Toolbox icon" width="128" height="128">
</p>

<h1 align="center">Toolbox</h1>

<p align="center">
  <strong>27 everyday tools for developers and designers in one fast, native desktop app.</strong><br>
  A SwiftUI app on a Rust core. Works offline, and your data never leaves your computer.
</p>

<p align="center">
  <img alt="Rust 2024 edition" src="https://img.shields.io/badge/rust-2024_edition-orange?logo=rust">
  <img alt="Platform: macOS" src="https://img.shields.io/badge/platform-macOS_14%2B-lightgrey?logo=apple">
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
- **Native.** A SwiftUI app with the system's controls, text editing, dark mode and eyedropper, on a fast Rust core that stays responsive with large inputs.
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

Toolbox needs macOS 14 (Sonoma) or later and runs natively on both Apple silicon and Intel.

> **Note:** If a build isn't signed with a Developer ID, macOS Gatekeeper warns on first launch. Right-click the app, choose **Open**, then confirm.

### From source

See [Building from source](#building-from-source) below.

## Building from source

### Prerequisites

- **macOS 14 or later.**
- **Rust**, a recent stable toolchain that supports the 2024 edition (1.85 or newer), installed with [rustup](https://rustup.rs).
- **Swift 6.2 or newer.** The Xcode Command Line Tools are enough (`xcode-select --install`). Full Xcode isn't required, but it gives you SwiftUI previews, Instruments and the view debugger.

### Build and run

```sh
git clone https://github.com/umairgul/rusty-toolbox.git
cd rusty-toolbox

# Build dist/Toolbox.app for this Mac and open it
scripts/build-macos.sh --dev
```

The script builds the Rust core as a static library, generates its Swift bindings, builds the Swift package and assembles the app bundle. After the first run, `swift build --package-path macos` works on its own for Swift-only changes. Run the script again whenever you change Rust code.

## Project structure

```
rusty-toolbox/
├── crates/
│   ├── toolbox-core/            # All tool logic, with no UI dependencies
│   │   ├── data/                # Bundled data (EFF passphrase wordlist)
│   │   └── src/                 # One module per tool: hash, jwt, cron, color/, images/, ...
│   └── toolbox-ffi/             # UniFFI bindings: exposes toolbox-core to Swift
│       └── src/                 # One module per tool, plus shared color, images, time_zones
├── macos/                       # The SwiftUI app (a Swift package)
│   ├── Package.swift
│   └── Sources/
│       ├── Toolbox/
│       │   ├── App/             # Entry point, sidebar, tool registry (Tool.swift), ToolStore
│       │   ├── Components/      # Shared views: CodeEditor, CopyButton, ValueRow, ColorField, ...
│       │   └── Tools/           # One file per tool: its @Observable model and its view
│       ├── ToolboxBindings/     # Generated Swift bindings (not committed)
│       └── toolbox_ffiFFI/      # C target for the generated header (header not committed)
├── uniffi-bindgen/              # The bindings generator, run by the build script
├── xtask/                       # Project automation (`cargo xtask <command>`)
├── scripts/
│   └── build-macos.sh           # Builds Toolbox.app and a signed, notarized .dmg
├── assets/icon/                 # App icon source (SVG) and rendered PNG/ICNS
└── Cargo.toml                   # Workspace manifest: shared dependencies, lints and profiles
```

## Architecture

The code is split into three layers:

```
┌──────────────────────────────────────────────┐
│  macos/ (SwiftUI)                            │
│  Sidebar · tool views · files · clipboard    │
└──────────────────────┬───────────────────────┘
                       │ generated Swift bindings
┌──────────────────────▼───────────────────────┐
│  crates/toolbox-ffi (UniFFI)                 │
│  FFI-friendly records, enums and errors      │
└──────────────────────┬───────────────────────┘
                       │ plain Rust calls
┌──────────────────────▼───────────────────────┐
│  crates/toolbox-core                         │
│  Hashing · encoding · parsing · color math · │
│  image processing · generators               │
└──────────────────────────────────────────────┘
```

- **`toolbox-core`** holds every algorithm and data transformation. It has no UI or FFI dependencies, and it's fully unit-tested.
- **`toolbox-ffi`** wraps the core for Swift with [UniFFI](https://mozilla.github.io/uniffi-rs/) proc macros. It converts core types to records and enums that cross the boundary (strings, numbers, `Vec<u8>` as `Data`), and reports failures as `ToolError`. It's built as a static library that's linked into the app, so there's no dynamic library to ship.
- **The SwiftUI app** is a presentation layer. Each tool is one file in `macos/Sources/Toolbox/Tools/` with an `@Observable` model, which holds the tool's state, and a view. `ToolStore` keeps every model alive for the whole session, so switching tools doesn't lose what you typed.

Slow work, such as converting images, hashing files or formatting large inputs, runs off the main thread through `offMainThread` or `.task(id:)`, and stale results are discarded.

## Development guide

### Everyday commands

| Task | Command |
| --- | --- |
| Build and open the app | `scripts/build-macos.sh --dev` |
| Rebuild only Swift changes | `swift build --package-path macos` |
| Run all Rust tests | `cargo test --workspace` |
| Test only the core logic | `cargo test -p toolbox-core` |
| Lint | `cargo clippy --workspace --all-targets` |
| Format Rust | `cargo fmt --all` |
| Regenerate the app icon | `cargo xtask icon` |
| Build a macOS release | `scripts/build-macos.sh` |

> `xtask` and `uniffi-bindgen` aren't default workspace members, so plain `cargo build` and `cargo test` skip them. Use `--workspace` to include them.

### Adding a new tool

1. **Write the logic in `toolbox-core`.** Add `crates/toolbox-core/src/my_tool.rs`, export it from `lib.rs` with `pub mod my_tool;`, and cover it with unit tests in the same file.

   If the tool needs a new crate, add it under `[workspace.dependencies]` in the root `Cargo.toml`, then reference it with `name.workspace = true` in the member's `Cargo.toml`.

2. **Expose it to Swift in `toolbox-ffi`.** Add `crates/toolbox-ffi/src/my_tool.rs` and declare it in `lib.rs`:

   ```rust
   use crate::ToolError;

   #[uniffi::export]
   pub fn my_tool_transform(input: &str) -> Result<String, ToolError> {
       toolbox_core::my_tool::transform(input).map_err(ToolError::new)
   }
   ```

   UniFFI has one namespace for everything it exports, so prefix every function and type with the tool's name (`my_tool_transform`, `MyToolOptions`), never generic names like `Options` or `Mode`.

3. **Build the view.** Add `macos/Sources/Toolbox/Tools/MyTool.swift`:

   ```swift
   import SwiftUI
   import ToolboxBindings

   @Observable
   final class MyToolModel {
       var input = ""
   }

   struct MyToolView: View {
       @Bindable var model: MyToolModel

       var body: some View {
           Form {
               Section {
                   TextField("Input", text: $model.input)
               }
               Section("Result") {
                   switch Result(catching: { try myToolTransform(input: model.input) }) {
                   case .success(let output): ValueRow("Output", value: output)
                   case .failure(let error): StatusLabel(ok: false, message: error.message)
                   }
               }
           }
           .formStyle(.grouped)
       }
   }
   ```

4. **Register it.** Add a case to `Tool` in `App/Tool.swift` with its name, summary, category and SF Symbol. Add a model to `ToolStore` and to `ToolStore.model(for:)`, and add the view to `ToolDetail` in `App/ContentView.swift`.

5. **Reuse the shared components** in `Components/` rather than writing your own:

   | Component | Use it for |
   | --- | --- |
   | `CodeEditor` | Multi-line code or data input and output. Unlike `TextEditor`, it never substitutes smart quotes or dashes. |
   | `Pane`, `StatusBar` | Titled editor panes and a bottom status line, as in JSON Formatter |
   | `CopyButton`, `ValueRow` | Copying, and label/value rows with a copy button |
   | `StatusLabel`, `FormNote` | Success or failure messages, and explanatory footers |
   | `SliderField` | A whole-number setting as a slider plus a number field |
   | `ColorField`, `Swatch`, `CopySwatch` | Color input with the system eyedropper, and swatches |
   | `ImageDropZone`, `ImageFiles`, `Files` | Opening, dropping, saving and revealing files |
   | `ZoneList`, `ZonePickerButton` | A searchable IANA time zone picker |
   | `offMainThread` | Running slow Rust calls without blocking the UI |

   A tool that takes files should accept drops, and conform its model to `OpensFiles` so files opened from Finder reach it.

### Testing

- **Core unit tests** (`crates/toolbox-core`): over 130 tests covering parsing, encoding, hashing, color math, image conversion and more. QR output is verified by decoding it again with `rqrr`.
- **FFI tests** (`crates/toolbox-ffi`): check the conversions between core and FFI types, such as byte offsets to UTF-16 ranges and option mapping, plus end-to-end calls like converting an image file.

To check UI changes, build with `scripts/build-macos.sh --dev`. The app also accepts launch arguments that help with screenshots: `-selectedTool <case>` opens a tool, such as `-selectedTool json`, and `-appearance dark` or `-appearance light` overrides the system appearance:

```sh
open dist/Toolbox.app --args -selectedTool palettes -appearance dark
```

### Code conventions

- **Keep logic out of the UI.** Anything that can be unit-tested belongs in `toolbox-core`. The FFI layer only adapts types.
- **No `unsafe`.** `unsafe_code` is forbidden workspace-wide.
- **Clippy** warns on leftover `dbg!` and `todo!`. Keep `cargo clippy --workspace --all-targets` clean.
- **Swift 6** with strict concurrency and `MainActor` as the default isolation. The build should produce no Swift warnings.
- **Use `@ViewState`, not `@State`.** Since the macOS 27 SDK, `@State` is a macro whose plugin ships only with Xcode, so it doesn't compile with just the Command Line Tools. `@ViewState` is a typealias for the same property wrapper (see `App/StateWorkaround.swift`). Once the project requires Xcode, switch back and delete that file.
- **Error messages:** show `error.message`, not `localizedDescription`, which UniFFI fills with the enum's debug description.
- **Follow the macOS Human Interface Guidelines.** Use Title Case for buttons and menus, give toolbar items tooltips, and use semantic colors so everything works in dark mode.
- **Shared dependencies** are declared once in `[workspace.dependencies]`, so every crate uses the same versions.
- **Builds are reproducible.** `Cargo.lock` is committed, and release builds use `--locked`.

### Updating the app icon

The icon's source is `assets/icon/toolbox.svg`. After editing it, run:

```sh
cargo xtask icon
```

This renders `assets/icon/toolbox-512.png`, which this README uses, and `assets/icon/Toolbox.icns`, which goes in the app bundle. Commit the regenerated files.

## Releasing for macOS

`scripts/build-macos.sh` builds `dist/Toolbox.app` and `dist/Toolbox-<version>.dmg`. The version comes from `[workspace.package]` in `Cargo.toml`.

```sh
# Universal app (Apple silicon + Intel), ad-hoc signed, with a disk image
scripts/build-macos.sh

# Faster local build for this Mac's CPU only, without a disk image
scripts/build-macos.sh --native --no-dmg

# Like --native --no-dmg, but builds Swift in debug and opens the app
scripts/build-macos.sh --dev
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

Toolbox is a macOS app (macOS 14 or later). Its UI is SwiftUI and AppKit, and it uses these system features:

| Feature | Provided by |
| --- | --- |
| Screen eyedropper (color tools) | `NSColorSampler`. No screen-recording permission is needed. |
| Opening HEIC/HEIF images | `sips` |
| Markdown preview | WebKit (`WKWebView`) |
| Opening images from Finder or the Dock | The app's document types (`public.image`) |

`toolbox-core` itself is platform-independent Rust, apart from HEIC decoding.

## Contributing

Contributions are welcome. Before opening a pull request:

1. Put new logic in `toolbox-core` and give it unit tests.
2. Run `cargo fmt --all`, `cargo clippy --workspace --all-targets` and `cargo test --workspace`, and make sure all three pass cleanly. Build the app with `scripts/build-macos.sh --dev` and make sure Swift reports no warnings.
3. Keep each commit focused, with a message that explains the change.
4. For UI changes, include before and after screenshots in light and dark mode (see [Testing](#testing)).

Use [GitHub Issues](https://github.com/umairgul/rusty-toolbox/issues) to report a bug or suggest a tool.

## License

Toolbox is dual-licensed under either of the following, at your option:

- MIT License ([`LICENSE-MIT`](LICENSE-MIT))
- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))

Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in this work, as defined in the Apache-2.0 license, is dual-licensed as above, without any additional terms or conditions.

### Third-party data

The passphrase generator uses the [EFF Long Wordlist](https://www.eff.org/dice) by the Electronic Frontier Foundation, licensed under [CC BY 3.0 US](https://creativecommons.org/licenses/by/3.0/us/). See [`NOTICE`](NOTICE) for details.
