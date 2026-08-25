//! Settings 视口的 UI 入口。

mod drawing;
mod window;

pub(crate) use window::SettingsWindow;

use std::time::Duration;

use egui::{Context, RawInput};

use crate::views::ViewOutput;

/// 构建带有系统装饰和渲染状态动画的 Settings 视图。
pub(crate) fn run(context: &Context, raw_input: RawInput) -> ViewOutput {
    let mut should_close = false;
    let full_output = context.run_ui(raw_input, |ctx| {
        ctx.request_repaint_after(Duration::from_millis(16));
        let time = ctx.input(|input| input.time) as f32;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Settings");
            ui.separator();
            ui.label("This is a normal decorated window.");
            let (font_id, family, style, size) = crate::fonts::default_font();
            let locale = crate::fonts::current_locale();
            let all_families = crate::fonts::list_all_font_families();
            let families = crate::fonts::list_font_families();
            let selected = crate::fonts::default_font_selection();
            ui.label(format!(
                "Default font: {family} / {style} ({size:.0}px, {font_id})"
            ));
            ui.label(format!("Compatible font families: {}", families.len()));
            ui.label(format!(
                "Locale: {}-{}; compatible families: {}/{}; selected: {}",
                locale.language,
                locale.region.as_deref().unwrap_or(""),
                families.len(),
                all_families.len(),
                selected
                    .as_ref()
                    .map(|font| font.family_label.as_str())
                    .unwrap_or("none")
            ));
            egui::CollapsingHeader::new("Available system fonts")
                .default_open(false)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            for entry in families.iter() {
                                ui.label(format!(
                                    "{} ({})",
                                    entry.label,
                                    entry.style_names().join(", ")
                                ));
                            }
                        });
                });
            drawing::draw_status(ui, time);
            if ui.button("Close settings").clicked() {
                should_close = true;
            }
        });
    });

    ViewOutput {
        full_output,
        should_close,
        ..Default::default()
    }
}
