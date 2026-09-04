//! HUD 绘制流程编排。

use super::{HudDragState, HudLayoutTarget, HudRenderItem, LayoutState, ShadowTarget};
use deskhud_engine::HudInstanceId;
use deskhud_ui::{HUD_SIZE_FACTOR_MAX, HUD_SIZE_FACTOR_MIN, MessageKey, UiPreferences};

mod adjustment;
mod frame;
mod layout;
mod overlay;

use adjustment::draw_adjust_window;
use frame::draw_frame;
use layout::{draw_alignment_grid, layout_slot, set_layout_slot, snap_coordinate, sync_layouts};
use overlay::{
    EditorOverlay, GroupDropFeedback, draw_border, draw_editor_overlays, draw_group_drop_feedback,
};

const HUD_PADDING: f32 = 8.0;
const GRID_STEP: f32 = 0.05;
const ADJUST_PANEL_WIDTH: f32 = 440.0;
const ADJUST_ROW_HEIGHT: f32 = 32.0;
const ADJUST_ROW_GAP: f32 = 8.0;
const ADJUST_LABEL_INDENT: f32 = 36.0;
const ADJUST_LABEL_WIDTH: f32 = 108.0;
const ADJUST_VALUE_WIDTH: f32 = 82.0;
const RESIZE_EDGE_GRAB: f32 = 7.0;
const RESIZE_CORNER_GRAB: f32 = 14.0;
const HUD_BORDER_WIDTH_MAX: f32 = 6.0;
// Allow the radius to reach half of a HUD's short side even at 300% scale,
// so the maximum value can produce a true capsule rather than a rounded
// rectangle capped at the old 32 px threshold.
const HUD_CORNER_RADIUS_MAX: f32 = 160.0;

pub(super) struct DrawResult {
    pub(super) size: [f32; 2],
    pub(super) move_by: Option<[f32; 2]>,
    pub(super) changed: bool,
}

enum EditorAction {
    CreateGroup {
        member: Option<HudInstanceId>,
        position: egui::Pos2,
    },
    DeleteGroup(String),
    BeginHudDrag {
        member: HudInstanceId,
        source_group_id: Option<String>,
        source_group_rect: Option<egui::Rect>,
        position: egui::Pos2,
        size: egui::Vec2,
        initial_delta: egui::Vec2,
    },
    RemoveMember {
        member: HudInstanceId,
        position: egui::Pos2,
    },
}

/// 绘制 HUD 子窗口并返回根据子窗口计算出的 HUD 尺寸。
pub(super) fn draw(
    ui: &mut egui::Ui,
    time: f32,
    layout: &mut LayoutState,
    items: &[HudRenderItem],
    prefs: &mut UiPreferences,
) -> DrawResult {
    let mut bounds = egui::Rect::NOTHING;
    let mut changed = false;
    let mut editor_overlays = Vec::new();
    let mut editor_action = None;
    let drag_released =
        layout.active_hud_drag.is_some() && ui.input(|input| input.pointer.primary_released());
    advance_hud_drag(layout, ui.input(|input| input.pointer.delta()));
    let canvas_response = ui.interact(
        ui.max_rect(),
        ui.make_persistent_id("hud-layout-canvas"),
        egui::Sense::click(),
    );
    if layout.layout_mode {
        let position = canvas_response
            .interact_pointer_pos()
            .unwrap_or(egui::Pos2::ZERO);
        canvas_response.context_menu(|ui| {
            if ui
                .button(deskhud_ui::i18n::t(
                    prefs.locale,
                    MessageKey::HudGroupCreate,
                ))
                .clicked()
            {
                editor_action = Some(EditorAction::CreateGroup {
                    member: None,
                    position,
                });
                ui.close();
            }
        });
    }
    if layout.layout_mode
        && layout.snap_to_grid
        && let Some(activity) = layout.activity_size
    {
        draw_alignment_grid(ui, activity);
    }
    for item in items {
        let position = layout
            .positions
            .entry(item.key.clone())
            .or_insert(item.initial_position);
        let base_size = egui::vec2(item.base_size.width, item.base_size.height);
        let preferred_size = item.container_size.unwrap_or_else(|| {
            egui::vec2(
                base_size.x * item.width.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
                base_size.y * item.height.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
            )
        });
        let mut window = egui::Window::new(egui::RichText::new(&item.key).small())
            .id(egui::Id::new((
                "hud-item",
                &item.key,
                layout.adjust_session,
                layout.window_revision,
            )))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .movable(false)
            // The HUD visual owns the background. The egui window frame must
            // stay transparent, otherwise its padding/background becomes a
            // second rectangle around the actual HUD panel.
            .frame(egui::Frame::NONE)
            .fixed_pos(*position);
        if layout.layout_mode {
            // DeskHud owns all four resize edges so left/top resizing can
            // update both the slot position and size. egui disables those
            // edges for fixed, immovable windows.
            window = window.fixed_size(preferred_size);
        } else {
            // Layout mode may leave a remembered large Window rectangle in
            // egui memory. Compact HUDs must size themselves to their content
            // again when the editor is closed.
            window = window.auto_sized();
        }
        let response = window.show(ui.ctx(), |ui| {
            draw_frame(ui, item, layout.layout_mode, prefs)
        });
        let Some(response) = response else { continue };
        let Some(frame) = response.inner else {
            continue;
        };
        let member_active = frame.members.iter().any(|member| {
            member.response.clicked()
                || member.response.drag_started()
                || member.response.dragged()
                || member.response.secondary_clicked()
        });
        if let HudLayoutTarget::Group(group_id) = &item.target {
            for member in &frame.members {
                let member_key = format!("instance/{}", member.instance_id.as_str());
                if let Some(resize) = member.resize_drag {
                    let old_size = member.rect.size();
                    let mut next_size = old_size;
                    if resize.edges.left {
                        next_size.x = (old_size.x - resize.delta.x).max(1.0);
                    } else if resize.edges.right {
                        next_size.x = (old_size.x + resize.delta.x).max(1.0);
                    }
                    if resize.edges.top {
                        next_size.y = (old_size.y - resize.delta.y).max(1.0);
                    } else if resize.edges.bottom {
                        next_size.y = (old_size.y + resize.delta.y).max(1.0);
                    }
                    if layout.lock_ratio {
                        let ratio = old_size.y / old_size.x.max(1.0);
                        if resize.edges.left || resize.edges.right {
                            next_size.y = (next_size.x * ratio).max(1.0);
                        } else if resize.edges.top || resize.edges.bottom {
                            next_size.x = (next_size.y / ratio.max(0.001)).max(1.0);
                        }
                    }
                    if let Some(instance) = prefs
                        .hud
                        .instances
                        .iter_mut()
                        .find(|instance| instance.id == member.instance_id)
                    {
                        instance.layout.width = next_size.x / member.base_size.width.max(1.0);
                        instance.layout.height = next_size.y / member.base_size.height.max(1.0);
                    }
                    if let Some(group) = prefs
                        .hud
                        .groups
                        .iter_mut()
                        .find(|group| group.children.contains(&member.instance_id))
                        && let Some(saved) = group
                            .member_layouts
                            .iter_mut()
                            .find(|saved| saved.instance_id == member.instance_id)
                    {
                        if resize.edges.left {
                            saved.x = (saved.x + old_size.x - next_size.x).max(0.0);
                        }
                        if resize.edges.top {
                            saved.y = (saved.y + old_size.y - next_size.y).max(0.0);
                        }
                    }
                    changed = true;
                }
                if member.response.clicked()
                    || member.response.drag_started()
                    || member.response.secondary_clicked()
                {
                    layout.selected = Some(member_key.clone());
                }
                if member.response.secondary_clicked() {
                    layout.adjust_open = true;
                }
                if member.response.drag_started() {
                    editor_action = Some(EditorAction::BeginHudDrag {
                        member: member.instance_id.clone(),
                        source_group_id: Some(group_id.clone()),
                        source_group_rect: Some(frame.body.rect),
                        position: member.rect.min,
                        size: member.rect.size(),
                        initial_delta: member.response.drag_delta(),
                    });
                }
                member.response.context_menu(|ui| {
                    if ui
                        .button(deskhud_ui::i18n::t(
                            prefs.locale,
                            MessageKey::HudGroupRemoveMember,
                        ))
                        .clicked()
                    {
                        editor_action = Some(EditorAction::RemoveMember {
                            member: member.instance_id.clone(),
                            position: member.rect.min,
                        });
                        ui.close();
                    }
                });
                editor_overlays.push(EditorOverlay {
                    key: member_key,
                    rect: member.rect,
                    layer_id: member.response.layer_id,
                    corner_radius: member.corner_radius,
                });
            }
        }
        if layout.layout_mode
            && !member_active
            && (frame.body.clicked() || frame.body.drag_started() || frame.resize_started)
        {
            layout.selected = Some(item.key.clone());
        }
        if layout.layout_mode && !member_active && frame.body.secondary_clicked() {
            layout.selected = Some(item.key.clone());
            layout.adjust_open = true;
        }
        if layout.layout_mode && !member_active {
            let position = frame.body.interact_pointer_pos().unwrap_or(*position);
            frame.body.context_menu(|ui| match &item.target {
                HudLayoutTarget::Instance(id) => {
                    if ui
                        .button(deskhud_ui::i18n::t(
                            prefs.locale,
                            MessageKey::HudGroupCreate,
                        ))
                        .clicked()
                    {
                        editor_action = Some(EditorAction::CreateGroup {
                            member: Some(id.clone()),
                            position,
                        });
                        ui.close();
                    }
                }
                HudLayoutTarget::Group(id) => {
                    if ui
                        .button(deskhud_ui::i18n::t(prefs.locale, MessageKey::HudGroupEdit))
                        .clicked()
                    {
                        layout.selected = Some(item.key.clone());
                        layout.adjust_open = true;
                        ui.close();
                    }
                    if ui
                        .button(deskhud_ui::i18n::t(
                            prefs.locale,
                            MessageKey::HudGroupDelete,
                        ))
                        .clicked()
                    {
                        editor_action = Some(EditorAction::DeleteGroup(id.clone()));
                        ui.close();
                    }
                }
            });
        }
        let drag_response = frame.group_drag.as_ref().unwrap_or(&frame.body);
        let item_is_active_drag = matches!(
            (&item.target, layout.active_hud_drag.as_ref()),
            (HudLayoutTarget::Instance(id), Some(drag)) if id == &drag.instance_id
        );
        if layout.layout_mode
            && !member_active
            && frame.resize_drag.is_none()
            && !item_is_active_drag
        {
            match &item.target {
                HudLayoutTarget::Instance(id)
                    if layout.active_hud_drag.is_none() && drag_response.drag_started() =>
                {
                    editor_action = Some(EditorAction::BeginHudDrag {
                        member: id.clone(),
                        source_group_id: None,
                        source_group_rect: None,
                        position: *position,
                        size: frame.body.rect.size(),
                        initial_delta: drag_response.drag_delta(),
                    });
                }
                HudLayoutTarget::Group(_) if drag_response.dragged() => {
                    *position += drag_response.drag_delta();
                    if layout.snap_to_grid
                        && let Some(activity) = layout.activity_size
                    {
                        position.x = snap_coordinate(position.x, activity.x);
                        position.y = snap_coordinate(position.y, activity.y);
                    }
                    changed = true;
                }
                _ => {}
            }
        }
        if layout.layout_mode {
            editor_overlays.push(EditorOverlay {
                key: item.key.clone(),
                rect: frame.body.rect,
                layer_id: frame.body.layer_id,
                corner_radius: item.corner_radius,
            });
            if let Some(resize) = frame.resize_drag
                && let Some(mut slot) = layout_slot(prefs, item)
            {
                let old_size = frame.size;
                let is_group = matches!(item.target, HudLayoutTarget::Group(_));
                let min_size = if is_group {
                    egui::vec2(1.0, 1.0)
                } else {
                    base_size * HUD_SIZE_FACTOR_MIN
                };
                let max_size = if is_group {
                    egui::vec2(f32::MAX, f32::MAX)
                } else {
                    base_size * HUD_SIZE_FACTOR_MAX
                };
                let mut next_size = old_size;
                if resize.edges.left {
                    next_size.x = (old_size.x - resize.delta.x).clamp(min_size.x, max_size.x);
                    position.x += old_size.x - next_size.x;
                } else if resize.edges.right {
                    next_size.x = (old_size.x + resize.delta.x).clamp(min_size.x, max_size.x);
                }
                if resize.edges.top {
                    next_size.y = (old_size.y - resize.delta.y).clamp(min_size.y, max_size.y);
                    position.y += old_size.y - next_size.y;
                } else if resize.edges.bottom {
                    next_size.y = (old_size.y + resize.delta.y).clamp(min_size.y, max_size.y);
                }
                if layout.lock_ratio {
                    let ratio = old_size.y / old_size.x.max(1.0);
                    if resize.edges.left || resize.edges.right {
                        next_size.y = (next_size.x * ratio).clamp(min_size.y, max_size.y);
                    } else if resize.edges.top || resize.edges.bottom {
                        next_size.x =
                            (next_size.y / ratio.max(0.001)).clamp(min_size.x, max_size.x);
                    }
                }
                if let HudLayoutTarget::Group(group_id) = &item.target {
                    if let Some(group) = prefs
                        .hud
                        .groups
                        .iter_mut()
                        .find(|group| &group.id == group_id)
                    {
                        group.size = [next_size.x.max(1.0), next_size.y.max(1.0)];
                        let horizontal_limit = group.size[0] * 0.25;
                        let vertical_limit = group.size[1] * 0.25;
                        group.inner.padding[0] = group.inner.padding[0].min(vertical_limit);
                        group.inner.padding[2] = group.inner.padding[2].min(vertical_limit);
                        group.inner.padding[1] = group.inner.padding[1].min(horizontal_limit);
                        group.inner.padding[3] = group.inner.padding[3].min(horizontal_limit);
                    }
                } else {
                    slot.width = (next_size.x / base_size.x.max(1.0))
                        .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
                    slot.height = (next_size.y / base_size.y.max(1.0))
                        .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
                    set_layout_slot(prefs, item, slot);
                }
                changed = true;
            }
        }
        if let Some(activity_size) = layout.activity_size {
            let panel_size = response.response.rect.size();
            position.x = position.x.clamp(
                HUD_PADDING,
                (activity_size.x - panel_size.x - HUD_PADDING).max(HUD_PADDING),
            );
            position.y = position.y.clamp(
                HUD_PADDING,
                (activity_size.y - panel_size.y - HUD_PADDING).max(HUD_PADDING),
            );
        }
        bounds = bounds.union(egui::Rect::from_min_size(
            *position,
            response.response.rect.size(),
        ));
    }

    if let Some(action) = editor_action {
        changed |= apply_editor_action(action, layout, prefs, &editor_overlays);
    }

    let active_drop = layout.active_hud_drag.as_ref().map(|drag| {
        let hud_rect = egui::Rect::from_min_size(drag.position, drag.size);
        let target_group_id = group_drop_target(&editor_overlays, hud_rect)
            .map(str::to_owned)
            .or_else(|| {
                drag.source_group_id
                    .as_ref()
                    .zip(drag.source_group_rect)
                    .and_then(|(group_id, group_rect)| {
                        rect_contains_rect(group_rect, hud_rect).then(|| group_id.clone())
                    })
            });
        (
            drag.source_group_id.clone(),
            drag.source_group_rect,
            target_group_id,
            hud_rect,
        )
    });
    if let Some((source_group_id, source_group_rect, target_group_id, _)) = &active_drop {
        if let Some(source_group_id) = source_group_id
            && target_group_id.as_deref() != Some(source_group_id.as_str())
            && let Some(overlay) = group_overlay(&editor_overlays, source_group_id)
        {
            let mut overlay = overlay.clone();
            if let Some(source_group_rect) = source_group_rect {
                overlay.rect = *source_group_rect;
            }
            draw_group_drop_feedback(ui, time, &overlay, GroupDropFeedback::Remove);
        }
        if let Some(target_group_id) = target_group_id
            && source_group_id.as_deref() != Some(target_group_id.as_str())
            && let Some(overlay) = group_overlay(&editor_overlays, target_group_id)
        {
            draw_group_drop_feedback(ui, time, overlay, GroupDropFeedback::Add);
        }
    }
    if drag_released && let Some((_, _, target_group_id, _)) = active_drop {
        changed |= finish_hud_drag(layout, prefs, &editor_overlays, target_group_id.as_deref());
    }

    if layout.layout_mode {
        draw_editor_overlays(ui, time, &editor_overlays, layout.selected.as_deref());
    }

    if layout.layout_mode
        && layout.adjust_open
        && let Some(selected) = layout.selected.clone()
    {
        if selected.starts_with("instance/") {
            layout.hud_adjust_open = true;
            layout.hud_adjust_key = Some(selected.clone());
        }
        let selected_group = selected
            .strip_prefix("group/")
            .map(str::to_owned)
            .or_else(|| {
                selected.strip_prefix("instance/").and_then(|id| {
                    prefs
                        .hud
                        .groups
                        .iter()
                        .find(|group| group.children.iter().any(|child| child.as_str() == id))
                        .map(|group| group.id.clone())
                })
            });
        if let Some(group_id) = selected_group {
            layout.group_adjust_open = true;
            layout.group_adjust_key = Some(format!("group/{group_id}"));
        }
        if layout.hud_adjust_open
            && let Some(key) = layout.hud_adjust_key.clone()
        {
            changed |= draw_adjust_window(ui, layout, items, prefs, key, "hud-adjust", false);
        }
        if layout.group_adjust_open
            && let Some(key) = layout.group_adjust_key.clone()
        {
            changed |= draw_adjust_window(ui, layout, items, prefs, key, "group-adjust", true);
        }
    }

    if !bounds.is_positive() {
        return DrawResult {
            size: [160.0, 100.0],
            move_by: None,
            changed,
        };
    }

    // 布局模式是覆盖活动区的辅助界面：用户排列面板时暂时扩大原生 HUD 窗口，
    // 按 Escape 退出后，下面的紧凑布局分支会将窗口恢复为实际内容尺寸。
    if layout.layout_mode
        && let Some(activity_size) = layout.activity_size
    {
        draw_border(
            ui,
            time,
            egui::Rect::from_min_size(egui::Pos2::ZERO, activity_size),
        );
        return DrawResult {
            size: [activity_size.x, activity_size.y],
            move_by: None,
            changed: changed || sync_layouts(prefs, layout, items, activity_size),
        };
    }

    let offset = egui::vec2(HUD_PADDING, HUD_PADDING) - bounds.min.to_vec2();
    if offset != egui::Vec2::ZERO {
        for item in items {
            if let Some(position) = layout.positions.get_mut(&item.key) {
                *position += offset;
            }
        }
    }
    let border_size = bounds.size() + egui::vec2(HUD_PADDING * 2.0, HUD_PADDING * 2.0);
    if layout.layout_mode {
        draw_border(
            ui,
            time,
            egui::Rect::from_min_size(egui::Pos2::ZERO, border_size),
        );
    }
    if layout.compact_pending {
        layout.compact_pending = false;
        layout.activity_size = None;
    }
    DrawResult {
        size: [border_size.x.max(160.0), border_size.y.max(100.0)],
        move_by: if offset == egui::Vec2::ZERO {
            None
        } else {
            Some([-offset.x, -offset.y])
        },
        changed: changed || sync_layouts(prefs, layout, items, egui::Vec2::ZERO),
    }
}

fn clamp_hud_position(position: egui::Pos2, size: egui::Vec2, activity: egui::Vec2) -> egui::Pos2 {
    egui::pos2(
        position.x.clamp(
            HUD_PADDING,
            (activity.x - size.x - HUD_PADDING).max(HUD_PADDING),
        ),
        position.y.clamp(
            HUD_PADDING,
            (activity.y - size.y - HUD_PADDING).max(HUD_PADDING),
        ),
    )
}

fn advance_hud_drag(layout: &mut LayoutState, delta: egui::Vec2) {
    let Some(drag) = layout.active_hud_drag.as_mut() else {
        return;
    };
    drag.position += delta;
    if let Some(activity) = layout.activity_size {
        drag.position = clamp_hud_position(drag.position, drag.size, activity);
    }
    layout.positions.insert(
        format!("instance/{}", drag.instance_id.as_str()),
        drag.position,
    );
}

fn group_overlay<'a>(overlays: &'a [EditorOverlay], group_id: &str) -> Option<&'a EditorOverlay> {
    overlays
        .iter()
        .find(|overlay| overlay.key == format!("group/{group_id}"))
}

fn group_drop_target(overlays: &[EditorOverlay], hud_rect: egui::Rect) -> Option<&str> {
    overlays.iter().find_map(|overlay| {
        let group_id = overlay.key.strip_prefix("group/")?;
        rect_contains_rect(overlay.rect, hud_rect).then_some(group_id)
    })
}

fn rect_contains_rect(container: egui::Rect, contained: egui::Rect) -> bool {
    contained.min.x >= container.min.x
        && contained.min.y >= container.min.y
        && contained.max.x <= container.max.x
        && contained.max.y <= container.max.y
}

pub(super) fn finish_active_hud_drag_as_screen(
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
) -> bool {
    finish_hud_drag(layout, prefs, &[], None)
}

fn finish_hud_drag(
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    overlays: &[EditorOverlay],
    target_group_id: Option<&str>,
) -> bool {
    let Some(drag) = layout.active_hud_drag.take() else {
        return false;
    };
    let key = format!("instance/{}", drag.instance_id.as_str());
    if let Some(group_id) = target_group_id
        && let Some(group_rect) = if drag.source_group_id.as_deref() == Some(group_id) {
            drag.source_group_rect
                .or_else(|| group_overlay(overlays, group_id).map(|overlay| overlay.rect))
        } else {
            group_overlay(overlays, group_id).map(|overlay| overlay.rect)
        }
    {
        let changed = prefs.hud.add_instance_to_group(group_id, &drag.instance_id);
        if changed
            && let Some(group) = prefs
                .hud
                .groups
                .iter_mut()
                .find(|group| group.id == group_id)
        {
            let [top, _, _, left] = group.inner.clone().normalized().padding;
            if let Some(member) = group
                .member_layouts
                .iter_mut()
                .find(|member| member.instance_id == drag.instance_id)
            {
                member.x = (drag.position.x - group_rect.min.x - left).max(0.0);
                member.y = (drag.position.y - group_rect.min.y - top).max(0.0);
            }
        }
        layout.positions.remove(&key);
        layout.selected = Some(key);
        layout.window_revision = layout.window_revision.wrapping_add(1);
        return changed;
    }

    let Some(activity) = layout.activity_size else {
        return false;
    };
    let mut x = (drag.position.x / activity.x.max(1.0)).clamp(0.0, 1.0);
    let mut y = (drag.position.y / activity.y.max(1.0)).clamp(0.0, 1.0);
    if layout.snap_to_grid {
        x = layout::snap_normalized(x);
        y = layout::snap_normalized(y);
    }
    if let Some(instance) = prefs
        .hud
        .instances
        .iter_mut()
        .find(|instance| instance.id == drag.instance_id)
    {
        instance.layout.x = x;
        instance.layout.y = y;
    }
    layout
        .positions
        .insert(key.clone(), egui::pos2(x * activity.x, y * activity.y));
    layout.selected = Some(key);
    layout.window_revision = layout.window_revision.wrapping_add(1);
    true
}

fn apply_editor_action(
    action: EditorAction,
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    _overlays: &[EditorOverlay],
) -> bool {
    match action {
        EditorAction::CreateGroup { member, position } => {
            let index = prefs.hud.groups.len() + 1;
            let name = format!(
                "{} {index}",
                deskhud_ui::i18n::t(prefs.locale, MessageKey::HudGroupDefaultName)
            );
            let id = prefs.hud.create_group(name);
            if let Some(activity) = layout.activity_size
                && let Some(group) = prefs.hud.groups.iter_mut().find(|group| group.id == id)
            {
                group.layout.x = (position.x / activity.x.max(1.0)).clamp(0.0, 1.0);
                group.layout.y = (position.y / activity.y.max(1.0)).clamp(0.0, 1.0);
            }
            if let Some(member) = member {
                prefs.hud.add_instance_to_group(&id, &member);
            }
            let key = format!("group/{id}");
            layout.positions.insert(key.clone(), position);
            layout.selected = Some(key);
            layout.adjust_open = true;
            layout.window_revision = layout.window_revision.wrapping_add(1);
            true
        }
        EditorAction::DeleteGroup(id) => {
            let changed = prefs.hud.delete_group(&id);
            if changed {
                let key = format!("group/{id}");
                layout.positions.remove(&key);
                if layout.selected.as_deref() == Some(key.as_str()) {
                    layout.selected = None;
                    layout.adjust_open = false;
                }
                layout.window_revision = layout.window_revision.wrapping_add(1);
            }
            changed
        }
        EditorAction::BeginHudDrag {
            member,
            source_group_id,
            source_group_rect,
            position,
            size,
            initial_delta,
        } => {
            let display = source_group_id
                .as_deref()
                .and_then(|group_id| {
                    prefs
                        .hud
                        .groups
                        .iter()
                        .find(|group| group.id == group_id)
                        .map(|group| group.layout.display.clone())
                })
                .unwrap_or_else(|| "primary".to_owned());
            let detached = source_group_id
                .as_ref()
                .is_some_and(|_| prefs.hud.remove_instance_from_group(&member));
            if let Some(instance) = prefs
                .hud
                .instances
                .iter_mut()
                .find(|instance| instance.id == member)
            {
                instance.layout.display = display;
            }
            let mut position = position + initial_delta;
            if let Some(activity) = layout.activity_size {
                position = clamp_hud_position(position, size, activity);
            }
            let key = format!("instance/{}", member.as_str());
            layout.positions.insert(key.clone(), position);
            layout.selected = Some(key);
            layout.active_hud_drag = Some(HudDragState {
                instance_id: member,
                source_group_id,
                source_group_rect,
                position,
                size,
            });
            if detached {
                layout.window_revision = layout.window_revision.wrapping_add(1);
            }
            detached
        }
        EditorAction::RemoveMember { member, position } => {
            let Some(activity) = layout.activity_size else {
                return false;
            };
            let display = prefs
                .hud
                .groups
                .iter()
                .find(|group| group.children.contains(&member))
                .map(|group| group.layout.display.clone())
                .unwrap_or_else(|| "primary".to_owned());
            if let Some(instance) = prefs
                .hud
                .instances
                .iter_mut()
                .find(|instance| instance.id == member)
            {
                instance.layout.display = display;
                instance.layout.x = (position.x / activity.x.max(1.0)).clamp(0.0, 1.0);
                instance.layout.y = (position.y / activity.y.max(1.0)).clamp(0.0, 1.0);
            }
            let changed = prefs.hud.remove_instance_from_group(&member);
            if changed {
                let key = format!("instance/{}", member.as_str());
                layout.positions.insert(key.clone(), position);
                layout.selected = Some(key);
                layout.window_revision = layout.window_revision.wrapping_add(1);
            }
            changed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefs_with_instances() -> (UiPreferences, HudInstanceId, HudInstanceId) {
        let mut prefs = UiPreferences::default();
        let first_source = deskhud_engine::HudSourceId::new("hud.test.one", "clock");
        let second_source = deskhud_engine::HudSourceId::new("hud.test.two", "meter");
        prefs
            .hud
            .ensure_default_instances([(first_source, true), (second_source, true)]);
        let first = prefs.hud.instances[0].id.clone();
        let second = prefs.hud.instances[1].id.clone();
        (prefs, first, second)
    }

    #[test]
    fn grouped_drag_detaches_then_rejoins_with_group_relative_coordinates() {
        let (mut prefs, first, _) = prefs_with_instances();
        let mut layout = LayoutState {
            layout_mode: true,
            activity_size: Some(egui::vec2(1000.0, 800.0)),
            ..LayoutState::default()
        };
        let group_id = prefs.hud.create_group("Free");
        prefs.hud.add_instance_to_group(&group_id, &first);
        prefs.hud.groups[0].inner.padding = [10.0, 0.0, 0.0, 12.0];
        let overlays = [EditorOverlay {
            key: format!("group/{group_id}"),
            rect: egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(400.0, 300.0)),
            layer_id: egui::LayerId::background(),
            corner_radius: 0.0,
        }];
        assert!(apply_editor_action(
            EditorAction::BeginHudDrag {
                member: first.clone(),
                source_group_id: Some(group_id.clone()),
                source_group_rect: Some(overlays[0].rect),
                position: egui::pos2(150.0, 150.0),
                size: egui::vec2(80.0, 40.0),
                initial_delta: egui::vec2(5.0, 3.0),
            },
            &mut layout,
            &mut prefs,
            &overlays,
        ));
        assert!(!prefs.hud.groups[0].children.contains(&first));
        assert_eq!(
            layout.active_hud_drag.as_ref().unwrap().position,
            egui::pos2(155.0, 153.0)
        );

        advance_hud_drag(&mut layout, egui::vec2(20.0, 7.0));
        advance_hud_drag(&mut layout, egui::vec2(15.0, 10.0));
        let hud_rect = egui::Rect::from_min_size(
            layout.active_hud_drag.as_ref().unwrap().position,
            egui::vec2(80.0, 40.0),
        );
        assert_eq!(
            group_drop_target(&overlays, hud_rect),
            Some(group_id.as_str())
        );
        assert!(finish_hud_drag(
            &mut layout,
            &mut prefs,
            &overlays,
            Some(&group_id),
        ));
        let group = &prefs.hud.groups[0];
        assert!(group.children.contains(&first));
        let member = group
            .member_layouts
            .iter()
            .find(|member| member.instance_id == first)
            .unwrap();
        assert_eq!((member.x, member.y), (78.0, 60.0));
    }

    #[test]
    fn drop_target_requires_the_entire_hud_rect_inside_the_group() {
        let overlays = [EditorOverlay {
            key: "group/one".to_owned(),
            rect: egui::Rect::from_min_max(egui::pos2(100.0, 100.0), egui::pos2(300.0, 300.0)),
            layer_id: egui::LayerId::background(),
            corner_radius: 0.0,
        }];
        let inside = egui::Rect::from_min_size(egui::pos2(120.0, 130.0), egui::vec2(80.0, 40.0));
        let partial = egui::Rect::from_min_size(egui::pos2(250.0, 130.0), egui::vec2(80.0, 40.0));
        assert_eq!(group_drop_target(&overlays, inside), Some("one"));
        assert_eq!(group_drop_target(&overlays, partial), None);
    }

    #[test]
    fn outside_drop_writes_layout_window_coordinates_to_the_screen_slot() {
        let (mut prefs, first, _) = prefs_with_instances();
        let mut layout = LayoutState {
            layout_mode: true,
            activity_size: Some(egui::vec2(1000.0, 800.0)),
            active_hud_drag: Some(HudDragState {
                instance_id: first.clone(),
                source_group_id: None,
                source_group_rect: None,
                position: egui::pos2(500.0, 320.0),
                size: egui::vec2(100.0, 50.0),
            }),
            ..LayoutState::default()
        };
        assert!(finish_hud_drag(&mut layout, &mut prefs, &[], None));
        let instance = prefs
            .hud
            .instances
            .iter()
            .find(|instance| instance.id == first)
            .unwrap();
        assert_eq!((instance.layout.x, instance.layout.y), (0.5, 0.4));
        assert!(layout.active_hud_drag.is_none());
    }
}
