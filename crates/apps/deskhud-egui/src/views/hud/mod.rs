//! HUD 视口的 UI 入口。

mod drawing;
mod window;

pub(crate) use window::HudWindow;

use std::{collections::HashMap, time::Duration};

use deskhud_engine::{EngineRegistry, HudFrame};
use deskhud_ui::{HudSlotLayout, UiPreferences};
use egui::{Context, RawInput};

use crate::views::ViewOutput;

/// HUD 内部子窗口的布局状态。
#[derive(Default)]
pub(crate) struct LayoutState {
    /// 按 `plugin_id/contribution_id` 保留的 HUD 条目逻辑坐标。
    pub(crate) positions: HashMap<String, egui::Pos2>,
    /// 是否处于可拖动布局模式。
    pub(crate) layout_mode: bool,
    /// 当前显示器活动区域的逻辑尺寸。
    pub(crate) activity_size: Option<egui::Vec2>,
    /// 是否等待下一帧切回紧凑窗口尺寸。
    pub(crate) compact_pending: bool,
    /// 当前高亮的 HUD 条目；右键调整窗口也绑定到此条目。
    pub(crate) selected: Option<String>,
    pub(crate) adjust_open: bool,
    pub(crate) shadow_open: bool,
    pub(crate) shadow_target: Option<ShadowTarget>,
    /// Whether layout positions should snap to the visible alignment grid.
    pub(crate) snap_to_grid: bool,
    /// Recreates HUD egui windows when entering a new editing session or
    /// when their size is changed from the adjustment panel.
    pub(crate) window_revision: u64,
    pub(crate) adjust_session: u64,
    /// Whether the adjustment panel should preserve the selected HUD aspect ratio.
    pub(crate) lock_ratio: bool,
    pub(crate) locked_ratio: Option<f32>,
    pub(crate) position_unit: AdjustmentUnit,
    pub(crate) size_unit: AdjustmentUnit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShadowTarget {
    Global,
    Window,
    Content,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum AdjustmentUnit {
    #[default]
    Percent,
    Pixels,
}

/// 已通过全局、插件和条目开关筛选的一条真实 HUD 帧。
pub(crate) struct HudRenderItem {
    pub(crate) key: String,
    pub(crate) frame: HudFrame,
    pub(crate) initial_position: egui::Pos2,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) background_enabled: bool,
    pub(crate) background_opacity: f32,
    pub(crate) background_blur: f32,
    pub(crate) content_opacity: f32,
    pub(crate) shadow_enabled: bool,
    pub(crate) window_shadow_global: bool,
    pub(crate) content_shadow_global: bool,
    pub(crate) window_shadow_enabled: bool,
    pub(crate) content_shadow_enabled: bool,
    pub(crate) window_shadow: f32,
    pub(crate) window_shadow_blur: f32,
    pub(crate) window_shadow_distance: f32,
    pub(crate) window_shadow_angle: f32,
    pub(crate) window_shadow_color: [u8; 3],
    pub(crate) window_custom_shadow: f32,
    pub(crate) window_custom_shadow_blur: f32,
    pub(crate) window_custom_shadow_distance: f32,
    pub(crate) window_custom_shadow_angle: f32,
    pub(crate) window_custom_shadow_color: [u8; 3],
    pub(crate) content_custom_shadow: f32,
    pub(crate) content_custom_shadow_blur: f32,
    pub(crate) content_custom_shadow_distance: f32,
    pub(crate) content_custom_shadow_angle: f32,
    pub(crate) content_custom_shadow_color: [u8; 3],
    pub(crate) content_color: [u8; 3],
    pub(crate) border_enabled: bool,
    pub(crate) border_opacity: f32,
    pub(crate) border_width: f32,
    pub(crate) corner_radius: f32,
    pub(crate) border_color: [u8; 3],
}

struct ActiveHudFrame {
    plugin_id: &'static str,
    contribution_id: &'static str,
    frame: HudFrame,
    layout: HudSlotLayout,
}

fn active_hud_frames(
    registry: &EngineRegistry,
    prefs: &UiPreferences,
    elapsed_secs: f32,
) -> Vec<ActiveHudFrame> {
    registry
        .all_hud_contributions()
        .into_iter()
        .enumerate()
        .filter_map(|(index, (plugin_id, contribution))| {
            if !prefs
                .hud
                .is_active(plugin_id, contribution.id, contribution.default_enabled)
            {
                return None;
            }
            let frame = registry.hud_frame(plugin_id, contribution.id, elapsed_secs);
            (!frame.is_empty()).then(|| ActiveHudFrame {
                plugin_id,
                contribution_id: contribution.id,
                frame,
                layout: prefs.hud.slot_layout(plugin_id, contribution.id, index),
            })
        })
        .collect()
}

/// 构建透明、无边框并带有动态虚线边框的 HUD 视图。
pub(crate) fn run(
    context: &Context,
    raw_input: RawInput,
    layout: &mut LayoutState,
    items: &[HudRenderItem],
    prefs: &mut UiPreferences,
) -> ViewOutput {
    let mut content_size = [320.0, 180.0];
    let mut move_by = None;
    let mut changed = false;
    let full_output = context.run_ui(raw_input, |ctx| {
        ctx.request_repaint_after(Duration::from_millis(16));
        let time = ctx.input(|input| input.time) as f32;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let result = drawing::draw(ui, time, layout, items, prefs);
                content_size = result.size;
                move_by = result.move_by;
                changed = result.changed;
            });
    });

    ViewOutput {
        full_output,
        resize_to: Some(content_size),
        move_by,
        applied_preferences: changed.then(|| prefs.clone()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::active_hud_frames;
    use deskhud_engine::HudVisual;

    #[test]
    fn registry_contributions_follow_all_three_enable_levels() {
        let bootstrap = deskhud_runtime::bootstrap_registry();
        let mut prefs = deskhud_ui::UiPreferences::default();
        let initial = active_hud_frames(&bootstrap.registry, &prefs, 1.0);
        assert_eq!(initial.len(), 1);
        assert_eq!(initial[0].contribution_id, "clock");
        assert!(initial[0].frame.visuals.iter().any(|visual| {
            matches!(visual, HudVisual::Text { text, .. } if text.starts_with("DeskHud"))
        }));

        prefs.hud.set_enabled("hud.deskhud.demo", "tip", true);
        assert_eq!(active_hud_frames(&bootstrap.registry, &prefs, 1.0).len(), 2);

        prefs.hud.set_plugin_enabled("hud.deskhud.demo", false);
        assert!(active_hud_frames(&bootstrap.registry, &prefs, 1.0).is_empty());
    }
}
