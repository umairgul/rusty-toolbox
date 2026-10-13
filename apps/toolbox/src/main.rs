//! Toolbox desktop app: a sidebar of tools, with the selected tool's UI on the right.

mod color_space;
mod eyedropper;
mod tools;

use tools::{Category, Tool};

/// Regenerate with `cargo xtask icon` after editing `assets/icon/toolbox.svg`.
const ICON_PNG: &[u8] = include_bytes!("../../../assets/icon/toolbox-512.png");

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Toolbox")
        .with_inner_size([900.0, 640.0])
        .with_min_inner_size([600.0, 420.0]);
    match eframe::icon_data::from_png_bytes(ICON_PNG) {
        Ok(icon) => viewport = viewport.with_icon(icon),
        Err(error) => eprintln!("couldn't load the app icon: {error}"),
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Toolbox",
        options,
        Box::new(|cc| {
            color_space::use_srgb(cc);
            Ok(Box::new(ToolboxApp::new()))
        }),
    )
}

struct ToolboxApp {
    tools: Vec<Box<dyn Tool>>,
    selected: usize,
}

impl ToolboxApp {
    fn new() -> Self {
        Self {
            tools: tools::all(),
            selected: 0,
        }
    }
}

impl eframe::App for ToolboxApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

impl ToolboxApp {
    fn show(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("tool_list")
            .resizable(false)
            .exact_size(180.0)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.heading("Toolbox");
                ui.separator();
                for category in Category::ALL {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(category.label()).small().weak());
                    for (index, tool) in self
                        .tools
                        .iter()
                        .enumerate()
                        .filter(|(_, tool)| tool.category() == category)
                    {
                        if ui
                            .selectable_label(self.selected == index, tool.name())
                            .clicked()
                        {
                            self.selected = index;
                        }
                    }
                }
            });

        egui::CentralPanel::default().show(ui, |ui| {
            let tool = &mut self.tools[self.selected];
            ui.heading(tool.name());
            ui.label(tool.description());
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.add_space(8.0);
                    tool.ui(ui);
                });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Opens every tool and draws a few frames, which catches panics in UI code. Set
    /// `TOOLBOX_SCREENSHOTS=<dir>` to also save a PNG of each tool there.
    #[test]
    fn every_tool_renders() {
        let screenshots = std::env::var_os("TOOLBOX_SCREENSHOTS").map(PathBuf::from);
        for index in 0..tools::all().len() {
            let mut app = ToolboxApp::new();
            app.selected = index;
            let name = app.tools[index]
                .name()
                .to_lowercase()
                .replace([' ', '/'], "-");
            let mut harness = egui_kittest::Harness::builder()
                .with_size([900.0, 640.0])
                .build_ui_state(|ui, app: &mut ToolboxApp| app.show(ui), app);
            harness.run_steps(4);
            if let Some(dir) = &screenshots {
                let image = harness.render().expect("rendering needs a GPU adapter");
                image
                    .save(dir.join(format!("{index:02}-{name}.png")))
                    .expect("screenshot saved");
            }
        }
    }

    /// egui's built-in fonts lack many symbols, such as check marks and arrows, which then
    /// draw as empty boxes. Every non-ASCII character in the UI source must be one the fonts can draw.
    #[test]
    fn ui_text_uses_only_drawable_characters() {
        let ctx = egui::Context::default();
        // One frame loads the fonts. Its texture updates go nowhere in a test, so clear them.
        ctx.run_ui(Default::default(), |_| {})
            .textures_delta
            .clear();
        // The proportional fonts are the UI's fallback chain; monospace loads lazily, so its
        // glyphs aren't known up front.
        let font = egui::FontId::proportional(14.0);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut missing = Vec::new();
        let mut files = vec![dir.join("main.rs")];
        files.extend(
            std::fs::read_dir(dir.join("tools"))
                .unwrap()
                .map(|entry| entry.unwrap().path()),
        );
        for file in files {
            let source = std::fs::read_to_string(&file).unwrap();
            for c in source.chars().filter(|c| !c.is_ascii()) {
                let drawable = ctx.fonts_mut(|f| f.has_glyph(&font, c));
                if !drawable {
                    missing.push(format!(
                        "{c:?} in {}",
                        file.file_name().unwrap().to_string_lossy()
                    ));
                }
            }
        }
        missing.dedup();
        assert!(
            missing.is_empty(),
            "characters egui can't draw: {missing:?}"
        );
    }
}
