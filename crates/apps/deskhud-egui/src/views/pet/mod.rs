//! Pet 主视口的 UI 入口。

mod drawing;
pub(crate) mod menu;
mod window;

pub(crate) use menu::{PetMenu, PetMenuAction};
pub(crate) use window::PetWindow;

use std::time::Duration;

use deskhud_engine::{
    DockState, DragState, MouseState, PetConfigBag, PetKind, PetPaintCtx, PetTheme,
};
use deskhud_ui::UiPreferences;
use egui::{Context, RawInput};

use crate::views::ViewOutput;

/// 构建透明、可拖动并带有右键菜单的 Pet 视图。
pub(crate) fn run(
    context: &Context,
    raw_input: RawInput,
    pet: &dyn PetKind,
    prefs: &UiPreferences,
    elapsed: f32,
) -> ViewOutput {
    let full_output = context.run_ui(raw_input, |ctx| {
        ctx.request_repaint_after(Duration::from_millis(16));
        let info = pet.info();
        let options: Vec<(&str, bool)> = pet
            .config_options()
            .iter()
            .map(|option| (option.key, option.default))
            .collect();
        let map = prefs.pet.short_map_for(info.id, &options);
        pet.apply_config(PetConfigBag::new(&map));
        let paint = pet.paint(PetPaintCtx {
            time_secs: elapsed as f64,
            pointer_dir: [0.0, 0.0],
            status_line: "",
            dock: DockState::FREE,
            drag: DragState::IDLE,
            mouse: MouseState::IDLE,
            config: PetConfigBag::new(&map),
            theme: PetTheme::default(),
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| drawing::draw(ui, &paint));
    });

    ViewOutput {
        full_output,
        ..Default::default()
    }
}
