//! HUD 屏幕槽位、吸附网格与布局同步。

use super::overlay::with_alpha;
use super::*;

pub(super) fn snap_coordinate(value: f32, _activity: f32) -> f32 {
    (value / GRID_STEP).round() * GRID_STEP
}

pub(super) fn draw_alignment_grid(ui: &egui::Ui, activity: egui::Vec2) {
    let color = with_alpha(ui.visuals().widgets.noninteractive.fg_stroke.color, 48);
    let stroke = egui::Stroke::new(1.0, color);
    let mut x = GRID_STEP;
    while x < activity.x {
        ui.painter()
            .line_segment([egui::pos2(x, 0.0), egui::pos2(x, activity.y)], stroke);
        x += GRID_STEP;
    }
    let mut y = GRID_STEP;
    while y < activity.y {
        ui.painter()
            .line_segment([egui::pos2(0.0, y), egui::pos2(activity.x, y)], stroke);
        y += GRID_STEP;
    }
}

pub(super) fn layout_slot(
    prefs: &UiPreferences,
    item: &HudRenderItem,
) -> Option<deskhud_ui::HudSlotLayout> {
    match &item.target {
        HudLayoutTarget::Instance(id) => prefs
            .hud
            .instances
            .iter()
            .find(|instance| &instance.id == id)
            .map(|instance| instance.layout.clone()),
        HudLayoutTarget::Group(id) => prefs
            .hud
            .groups
            .iter()
            .find(|group| &group.id == id)
            .map(|group| group.layout.clone()),
    }
}

pub(super) fn set_layout_slot(
    prefs: &mut UiPreferences,
    item: &HudRenderItem,
    slot: deskhud_ui::HudSlotLayout,
) {
    match &item.target {
        HudLayoutTarget::Instance(id) => {
            if let Some(instance) = prefs
                .hud
                .instances
                .iter_mut()
                .find(|instance| &instance.id == id)
            {
                instance.layout = slot.clamp01();
            }
        }
        HudLayoutTarget::Group(id) => {
            if let Some(group) = prefs.hud.groups.iter_mut().find(|group| &group.id == id) {
                // Group layout.size is an actual pixel container size, not
                // an instance scale factor.
                group.layout = slot.clamp_position();
            }
        }
    }
}
