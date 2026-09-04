//! HUD 屏幕槽位、吸附网格与布局同步。

use super::overlay::with_alpha;
use super::*;

pub(super) fn snap_normalized(value: f32) -> f32 {
    (value / GRID_STEP).round() * GRID_STEP
}

pub(super) fn snap_coordinate(value: f32, activity: f32) -> f32 {
    snap_normalized(value / activity.max(1.0)) * activity
}

pub(super) fn draw_alignment_grid(ui: &egui::Ui, activity: egui::Vec2) {
    let color = with_alpha(ui.visuals().widgets.noninteractive.fg_stroke.color, 48);
    let stroke = egui::Stroke::new(1.0, color);
    let divisions = (1.0 / GRID_STEP) as i32;
    for index in 1..divisions {
        let x = activity.x * index as f32 / divisions as f32;
        let y = activity.y * index as f32 / divisions as f32;
        ui.painter()
            .line_segment([egui::pos2(x, 0.0), egui::pos2(x, activity.y)], stroke);
        ui.painter()
            .line_segment([egui::pos2(0.0, y), egui::pos2(activity.x, y)], stroke);
    }
}

pub(super) fn draw_snap_grid_control(
    ui: &mut egui::Ui,
    snap_to_grid: &std::cell::Cell<bool>,
    locale: deskhud_ui::Locale,
) -> bool {
    // Match the combo-box row height; the icon itself remains 24 px and is
    // centered inside this 28 px interaction area.
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(28.0, ADJUST_ROW_HEIGHT), egui::Sense::click());
    if response.clicked() {
        snap_to_grid.set(!snap_to_grid.get());
    }
    let color = if snap_to_grid.get() {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    draw_toggle_icon(ui, rect, "grid", color, snap_to_grid.get());
    let clicked = response.clicked();
    response.on_hover_text(deskhud_ui::i18n::t(locale, MessageKey::HudAdjustSnapGrid));
    clicked
}

pub(super) fn draw_ratio_lock_control(
    ui: &mut egui::Ui,
    lock_ratio: &std::cell::Cell<bool>,
    locale: deskhud_ui::Locale,
) -> bool {
    // Match the combo-box row height; the icon itself remains 24 px and is
    // centered inside this 28 px interaction area.
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(28.0, ADJUST_ROW_HEIGHT), egui::Sense::click());
    if response.clicked() {
        lock_ratio.set(!lock_ratio.get());
    }
    let color = if lock_ratio.get() {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    draw_toggle_icon(ui, rect, "link", color, lock_ratio.get());
    let clicked = response.clicked();
    response.on_hover_text(deskhud_ui::i18n::t(locale, MessageKey::HudAdjustLockRatio));
    clicked
}

fn draw_toggle_icon(
    ui: &egui::Ui,
    rect: egui::Rect,
    icon: &'static str,
    color: egui::Color32,
    active: bool,
) {
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(24.0, 24.0));
    ui.painter().rect_filled(
        icon_rect,
        4.0,
        if active {
            ui.visuals().selection.bg_fill
        } else {
            ui.visuals().widgets.inactive.bg_fill
        },
    );
    crate::components::icons::paint(ui, icon, icon_rect.shrink(4.0), color, false);
}

pub(super) fn layout_slot(
    prefs: &UiPreferences,
    item: &HudRenderItem,
) -> Option<deskhud_ui::HudSlotLayout> {
    match &item.target {
        HudLayoutTarget::Instance(id) => {
            if let Some(group) = prefs
                .hud
                .groups
                .iter()
                .find(|group| group.children.contains(id))
            {
                let member = group
                    .member_layouts
                    .iter()
                    .find(|member| &member.instance_id == id);
                return Some(deskhud_ui::HudSlotLayout {
                    display: group.layout.display.clone(),
                    x: member.map(|member| member.x).unwrap_or(0.0),
                    y: member.map(|member| member.y).unwrap_or(0.0),
                    width: prefs
                        .hud
                        .instances
                        .iter()
                        .find(|instance| &instance.id == id)
                        .map(|instance| instance.layout.width)
                        .unwrap_or(1.0),
                    height: prefs
                        .hud
                        .instances
                        .iter()
                        .find(|instance| &instance.id == id)
                        .map(|instance| instance.layout.height)
                        .unwrap_or(1.0),
                });
            }
            prefs
                .hud
                .instances
                .iter()
                .find(|instance| &instance.id == id)
                .map(|instance| instance.layout.clone())
        }
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
            if let Some(group) = prefs
                .hud
                .groups
                .iter_mut()
                .find(|group| group.children.contains(id))
            {
                if group
                    .member_layouts
                    .iter()
                    .all(|member| &member.instance_id != id)
                {
                    group.member_layouts.push(deskhud_ui::HudGroupMemberLayout {
                        instance_id: id.clone(),
                        x: slot.x.max(0.0),
                        y: slot.y.max(0.0),
                    });
                }
                let member = group
                    .member_layouts
                    .iter_mut()
                    .find(|member| &member.instance_id == id)
                    .expect("inserted missing group member layout");
                member.x = slot.x.max(0.0);
                member.y = slot.y.max(0.0);
                return;
            }
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
                group.layout = slot.clamp01();
            }
        }
    }
}

pub(super) fn sync_layouts(
    prefs: &mut UiPreferences,
    layout: &LayoutState,
    items: &[HudRenderItem],
    activity: egui::Vec2,
) -> bool {
    if !layout.layout_mode || activity.x <= 0.0 || activity.y <= 0.0 {
        return false;
    }
    let mut changed = false;
    for item in items {
        if matches!(
            (&item.target, layout.active_hud_drag.as_ref()),
            (HudLayoutTarget::Instance(id), Some(drag)) if id == &drag.instance_id
        ) {
            continue;
        }
        // A grouped member uses logical pixel coordinates relative to its
        // group canvas. It must never be fed through the normalized screen
        // slot synchronizer below, or every frame would clamp it back to 0..1
        // (making the position appear permanently locked at zero).
        if let HudLayoutTarget::Instance(id) = &item.target
            && prefs
                .hud
                .groups
                .iter()
                .any(|group| group.children.contains(id))
        {
            continue;
        }
        if let Some(pos) = layout.positions.get(&item.key)
            && let Some(mut slot) = layout_slot(prefs, item)
        {
            let x = (pos.x / activity.x).clamp(0.0, 1.0);
            let y = (pos.y / activity.y).clamp(0.0, 1.0);
            let x = if layout.snap_to_grid {
                snap_normalized(x)
            } else {
                x
            };
            let y = if layout.snap_to_grid {
                snap_normalized(y)
            } else {
                y
            };
            if (slot.x - x).abs() > 0.0001 || (slot.y - y).abs() > 0.0001 {
                slot.x = x;
                slot.y = y;
                set_layout_slot(prefs, item, slot);
                changed = true;
            }
        }
    }
    changed
}
