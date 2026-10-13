//! Project automation. Run with `cargo xtask <command>`:
//!
//! - `icon`: renders `assets/icon/toolbox.svg` to `assets/icon/toolbox-512.png` (the window
//!   icon) and, on macOS, `assets/icon/Toolbox.icns` (the app bundle icon).

use resvg::{tiny_skia, usvg};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

/// The sizes macOS expects in an `.iconset`: (file name, pixels).
const ICONSET: [(&str, u32); 10] = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
];

fn main() -> ExitCode {
    let result = match std::env::args().nth(1).as_deref() {
        Some("icon") => icon(),
        _ => {
            eprintln!("usage: cargo xtask icon");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is inside the workspace")
        .to_owned()
}

fn icon() -> Result {
    let root = workspace_root();
    let icon_dir = root.join("assets/icon");
    let svg = fs::read(icon_dir.join("toolbox.svg"))?;
    let tree = usvg::Tree::from_data(&svg, &usvg::Options::default())?;

    let window_icon = icon_dir.join("toolbox-512.png");
    render(&tree, 512)?.save_png(&window_icon)?;
    println!("wrote {}", window_icon.display());

    if cfg!(target_os = "macos") {
        let iconset = root.join("target/xtask/Toolbox.iconset");
        fs::create_dir_all(&iconset)?;
        for (name, size) in ICONSET {
            render(&tree, size)?.save_png(iconset.join(name))?;
        }
        let icns = icon_dir.join("Toolbox.icns");
        let status = Command::new("iconutil")
            .arg("-c")
            .arg("icns")
            .arg(&iconset)
            .arg("-o")
            .arg(&icns)
            .status()?;
        if !status.success() {
            return Err(format!("iconutil failed with {status}").into());
        }
        println!("wrote {}", icns.display());
    } else {
        println!("skipped Toolbox.icns: iconutil is only available on macOS");
    }
    Ok(())
}

fn render(tree: &usvg::Tree, size: u32) -> Result<tiny_skia::Pixmap> {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).ok_or("icon size must be non-zero")?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}
