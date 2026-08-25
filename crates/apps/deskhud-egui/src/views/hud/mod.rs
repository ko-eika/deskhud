//! HUD 视口的 UI 入口。

mod drawing;
mod window;

pub(crate) use window::HudWindow;

use std::time::Duration;

use egui::{Context, RawInput};

use crate::views::ViewOutput;

/// HUD 内部子窗口的布局状态。
pub(crate) struct LayoutState {
    /// 两个 HUD 子面板的逻辑坐标。
    pub(crate) positions: [egui::Pos2; 2],
    /// 是否处于可拖动布局模式。
    pub(crate) layout_mode: bool,
    /// 当前显示器活动区域的逻辑尺寸。
    pub(crate) activity_size: Option<egui::Vec2>,
    /// 是否等待下一帧切回紧凑窗口尺寸。
    pub(crate) compact_pending: bool,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            positions: [egui::pos2(24.0, 24.0), egui::pos2(180.0, 76.0)],
            layout_mode: false,
            activity_size: None,
            compact_pending: false,
        }
    }
}

/// 构建透明、无边框并带有动态虚线边框的 HUD 视图。
pub(crate) fn run(context: &Context, raw_input: RawInput, layout: &mut LayoutState) -> ViewOutput {
    let mut content_size = [320.0, 180.0];
    let mut move_by = None;
    let full_output = context.run_ui(raw_input, |ctx| {
        ctx.request_repaint_after(Duration::from_millis(16));
        let time = ctx.input(|input| input.time) as f32;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let result = drawing::draw(ui, time, layout);
                content_size = result.size;
                move_by = result.move_by;
            });
    });

    ViewOutput {
        full_output,
        resize_to: Some(content_size),
        move_by,
        ..Default::default()
    }
}
