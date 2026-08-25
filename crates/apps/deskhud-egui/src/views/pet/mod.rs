//! Pet 主视口的 UI 入口。

mod drawing;
pub(crate) mod menu;
mod window;

pub(crate) use menu::{PetMenu, PetMenuAction};
pub(crate) use window::PetWindow;

use std::time::Duration;

use egui::{Context, RawInput};

use crate::views::ViewOutput;

/// 构建透明、可拖动并带有右键菜单的 Pet 视图。
pub(crate) fn run(context: &Context, raw_input: RawInput) -> ViewOutput {
    let full_output = context.run_ui(raw_input, |ctx| {
        ctx.request_repaint_after(Duration::from_millis(16));
        let time = ctx.input(|input| input.time) as f32;

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| drawing::draw(ui, time));
    });

    ViewOutput {
        full_output,
        ..Default::default()
    }
}
