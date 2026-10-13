use super::{Category, Tool, copy_button, output_box};
use toolbox_core::lorem::{self, Unit};

pub struct LoremTool {
    options: lorem::Options,
    text: String,
}

impl Default for LoremTool {
    fn default() -> Self {
        let options = lorem::Options::default();
        Self {
            text: lorem::generate(&options),
            options,
        }
    }
}

fn max_count(unit: Unit) -> usize {
    match unit {
        Unit::Paragraphs => 50,
        Unit::Sentences => 100,
        Unit::Words => 1000,
    }
}

impl Tool for LoremTool {
    fn name(&self) -> &'static str {
        "Lorem Ipsum"
    }

    fn description(&self) -> &'static str {
        "Placeholder text by paragraph, sentence or word, as plain text or HTML."
    }

    fn category(&self) -> Category {
        Category::Generators
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let before = self.options.clone();

        ui.horizontal(|ui| {
            for unit in Unit::ALL {
                ui.selectable_value(&mut self.options.unit, unit, unit.label());
            }
        });
        ui.add_space(8.0);
        let max = max_count(self.options.unit);
        self.options.count = self.options.count.min(max);
        ui.spacing_mut().slider_width = 260.0;
        ui.add(
            egui::Slider::new(&mut self.options.count, 1..=max)
                .logarithmic(max > 100)
                .text("Count"),
        );
        ui.checkbox(
            &mut self.options.classic_start,
            "Start with \"Lorem ipsum dolor sit amet…\"",
        );
        ui.checkbox(&mut self.options.html, "Wrap paragraphs in <p> tags");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button("Regenerate").clicked() {
                self.text = lorem::generate(&self.options);
            }
            copy_button(ui, "copy_lorem", "Copy", &self.text);
            let words = self.text.split_whitespace().count();
            ui.weak(format!(
                "{words} words · {} characters",
                self.text.chars().count()
            ));
        });
        if self.options != before {
            self.text = lorem::generate(&self.options);
        }
        ui.add_space(8.0);

        output_box(ui, "lorem_output", &self.text, 360.0);
    }
}
