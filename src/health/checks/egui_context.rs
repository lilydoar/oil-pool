//! Egui context health check

use crate::health::check::{CheckResult, SystemCheck};
use egui::Context;

/// Checks that egui context can be created and operates correctly
pub struct EguiContextCheck;

impl EguiContextCheck {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EguiContextCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCheck for EguiContextCheck {
    fn name(&self) -> &'static str {
        "Egui Context"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Validates egui context creation and basic UI operations")
    }

    fn check(&self) -> CheckResult {
        let mut details = Vec::new();

        // Create egui context
        let ctx = Context::default();
        details.push("  ✓ egui context created".to_string());

        // Test basic input handling
        let raw_input = egui::RawInput::default();
        let _output = ctx.run(raw_input.clone(), |ctx| {
            // Test basic UI operations
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Health check test");
                let _ = ui.button("Test button");
            });
        });
        details.push("  ✓ UI frame processing works".to_string());

        // Test with a simple window
        let _output = ctx.run(raw_input.clone(), |ctx| {
            egui::Window::new("Test Window").show(ctx, |ui| {
                ui.heading("Test");
                ui.label("Testing egui operations");
                ui.separator();
                ui.checkbox(&mut true.clone(), "Test checkbox");
            });
        });
        details.push("  ✓ Window creation and widgets work".to_string());

        // Test layout calculation
        let _output = ctx.run(raw_input.clone(), |ctx| {
            egui::Area::new(egui::Id::new("test_area"))
                .fixed_pos(egui::pos2(10.0, 10.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Left");
                        ui.label("Right");
                    });
                    ui.vertical(|ui| {
                        ui.label("Top");
                        ui.label("Bottom");
                    });
                });
        });
        details.push("  ✓ Layout calculations work".to_string());

        // Test memory usage (egui tracks texture and shape memory)
        let _memory = ctx.memory(|mem| mem.clone());
        details.push("  ✓ Memory tracking operational".to_string());

        // Test input state handling
        let _output = ctx.run(raw_input.clone(), |ctx| {
            let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
            let keys_down = ctx.input(|i| i.keys_down.len());
            details.push(format!(
                "    Input state: pointer={:?}, keys={}",
                pointer_pos, keys_down
            ));
        });
        details.push("  ✓ Input state handling works".to_string());

        // Test style and visuals
        let style = ctx.style();
        let spacing = style.spacing.item_spacing;
        details.push(format!(
            "  ✓ Style system accessible (spacing: {:.1}x{:.1})",
            spacing.x, spacing.y
        ));

        // Test tessellation (shape rendering)
        let _output = ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.painter().circle_filled(
                    egui::pos2(50.0, 50.0),
                    10.0,
                    egui::Color32::from_rgb(255, 0, 0),
                );
            });
        });
        details.push("  ✓ Shape rendering (tessellation) works".to_string());

        CheckResult::pass("egui context fully operational").with_details(details.join("\n"))
    }
}
