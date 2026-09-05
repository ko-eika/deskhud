//! HUD 绘制流程编排。

use super::{HudDragState, HudLayoutTarget, HudRenderItem, LayoutState, ShadowTarget};
use deskhud_engine::HudInstanceId;
use deskhud_ui::{HUD_SIZE_FACTOR_MAX, HUD_SIZE_FACTOR_MIN, MessageKey, UiPreferences};
use egui::emath::GuiRounding as _;

mod adjustment;
mod frame;
mod layout;
mod overlay;
mod tree_panels;

use adjustment::draw_adjust_window;
use frame::draw_frame;
use layout::{draw_alignment_grid, layout_slot, set_layout_slot, snap_coordinate};
use overlay::{
    EditorOverlay, GroupDropFeedback, draw_alignment_guides, draw_border, draw_editor_overlays,
    draw_group_drop_feedback, draw_preview_background, draw_preview_border,
};

const HUD_PADDING: f32 = 8.0;
/// Grid spacing in the same pixel coordinate system as layout x/y.
const GRID_STEP: f32 = 32.0;
/// Maximum distance at which a moving edge or centre is attracted to another
/// HUD/group alignment line.
const ALIGNMENT_SNAP_DISTANCE: f32 = 8.0;
const EDITOR_PANEL_WIDTH: f32 = 440.0;
const EDITOR_PANEL_HEIGHT: f32 = 420.0;
const EDITOR_PANEL_LEFT_MARGIN: f32 = 24.0;
const EDITOR_PANEL_TOP: f32 = 32.0;
const EDITOR_PANEL_GAP: f32 = 10.0;
const ADJUST_ROW_HEIGHT: f32 = 32.0;
const ADJUST_ROW_GAP: f32 = 8.0;
const ADJUST_LABEL_INDENT: f32 = 36.0;
const ADJUST_LABEL_WIDTH: f32 = 108.0;
const ADJUST_VALUE_WIDTH: f32 = 82.0;
const RESIZE_EDGE_GRAB: f32 = 7.0;
const RESIZE_CORNER_GRAB: f32 = 14.0;
const GROUP_MIN_WIDTH: f32 = 120.0;
const GROUP_MIN_HEIGHT: f32 = 80.0;
const HUD_BORDER_WIDTH_MAX: f32 = 6.0;
// Allow the radius to reach half of a HUD's short side even at 300% scale,
// so the maximum value can produce a true capsule rather than a rounded
// rectangle capped at the old 32 px threshold.
const HUD_CORNER_RADIUS_MAX: f32 = 160.0;

const CANVAS_CONTEXT_MENU_KEYS: &[MessageKey] = &[
    MessageKey::HudLayoutInformationTree,
    MessageKey::HudLayoutActiveTree,
    MessageKey::HudAdjustSnapGrid,
    MessageKey::HudAdjustLockRatio,
    MessageKey::HudGroupCreate,
    MessageKey::HudLayoutDone,
    MessageKey::HudLayoutCancel,
];
const GROUP_CONTEXT_MENU_KEYS: &[MessageKey] = &[
    MessageKey::HudAdjustSnapGrid,
    MessageKey::HudAdjustLockRatio,
    MessageKey::HudAdjustResetPosition,
    MessageKey::HudAdjustResetSize,
    MessageKey::HudGroupDelete,
];
const HUD_CONTEXT_MENU_KEYS: &[MessageKey] = &[
    MessageKey::HudAdjustSnapGrid,
    MessageKey::HudAdjustLockRatio,
    MessageKey::HudAdjustResetPosition,
    MessageKey::HudAdjustResetSize,
    MessageKey::HudLayoutCloseInformation,
];

pub(super) struct DrawResult {
    pub(super) size: [f32; 2],
    pub(super) move_by: Option<[f32; 2]>,
    pub(super) changed: bool,
}

#[derive(Clone, Copy, Default)]
pub(super) struct AlignmentSnap {
    pub(super) delta: egui::Vec2,
    pub(super) vertical: Option<f32>,
    pub(super) horizontal: Option<f32>,
}

impl AlignmentSnap {
    fn is_active(self) -> bool {
        self.vertical.is_some() || self.horizontal.is_some()
    }
}

enum EditorAction {
    CreateGroup {
        member: Option<HudInstanceId>,
        position: egui::Pos2,
    },
    DeleteGroup(String),
    ResetTopLevelPosition {
        key: String,
        position: egui::Pos2,
    },
    ResetMemberPosition(HudInstanceId),
    ResetHudSize(HudInstanceId),
    ResetGroupSize(String),
    DisableHud(HudInstanceId),
    BeginHudDrag {
        member: HudInstanceId,
        source_group_id: Option<String>,
        source_group_rect: Option<egui::Rect>,
        position: egui::Pos2,
        size: egui::Vec2,
        initial_delta: egui::Vec2,
    },
}

fn draw_layout_options_menu(
    ui: &mut egui::Ui,
    snap_to_grid: &mut bool,
    lock_ratio: &mut bool,
    locale: deskhud_ui::Locale,
) -> bool {
    let mut changed = false;
    if crate::menu::embedded_menu_item(
        ui,
        deskhud_ui::i18n::t(locale, MessageKey::HudAdjustSnapGrid),
        None,
        *snap_to_grid,
    )
    .clicked()
    {
        *snap_to_grid = !*snap_to_grid;
        changed = true;
    }
    crate::menu::embedded_menu_gap(ui);
    if crate::menu::embedded_menu_item(
        ui,
        deskhud_ui::i18n::t(locale, MessageKey::HudAdjustLockRatio),
        None,
        *lock_ratio,
    )
    .clicked()
    {
        *lock_ratio = !*lock_ratio;
        changed = true;
    }
    changed
}

fn style_context_menu(ui: &mut egui::Ui, locale: deskhud_ui::Locale, labels: &[MessageKey]) {
    let labels = labels
        .iter()
        .map(|key| deskhud_ui::i18n::t(locale, *key))
        .collect::<Vec<_>>();
    crate::menu::embedded_menu_begin(ui, &labels);
}

fn style_canvas_context_menu(ui: &mut egui::Ui, locale: deskhud_ui::Locale) {
    style_context_menu(ui, locale, CANVAS_CONTEXT_MENU_KEYS);
}

fn style_hud_context_menu(ui: &mut egui::Ui, locale: deskhud_ui::Locale) {
    style_context_menu(ui, locale, HUD_CONTEXT_MENU_KEYS);
}

fn style_group_context_menu(ui: &mut egui::Ui, locale: deskhud_ui::Locale) {
    style_context_menu(ui, locale, GROUP_CONTEXT_MENU_KEYS);
}

fn draw_tree_panel_menu(
    ui: &mut egui::Ui,
    information_tree_open: &mut bool,
    active_tree_open: &mut bool,
    locale: deskhud_ui::Locale,
) {
    if crate::menu::embedded_menu_item(
        ui,
        deskhud_ui::i18n::t(locale, MessageKey::HudLayoutInformationTree),
        None,
        *information_tree_open,
    )
    .clicked()
    {
        *information_tree_open = true;
        ui.close();
    }
    if crate::menu::embedded_menu_item(
        ui,
        deskhud_ui::i18n::t(locale, MessageKey::HudLayoutActiveTree),
        None,
        *active_tree_open,
    )
    .clicked()
    {
        *active_tree_open = true;
        ui.close();
    }
}

fn context_menu_button(
    ui: &mut egui::Ui,
    label: &str,
    icon: Option<&'static str>,
) -> egui::Response {
    crate::menu::embedded_menu_item(ui, label, icon, false)
}

fn draw_layout_exit_context_menu(ui: &mut egui::Ui, locale: deskhud_ui::Locale) -> Option<bool> {
    crate::menu::embedded_menu_separator(ui);
    if context_menu_button(
        ui,
        deskhud_ui::i18n::t(locale, MessageKey::HudLayoutDone),
        Some("circle-check"),
    )
    .clicked()
    {
        return Some(false);
    }
    crate::menu::embedded_menu_gap(ui);
    context_menu_button(
        ui,
        deskhud_ui::i18n::t(locale, MessageKey::HudLayoutCancel),
        Some("close-circle"),
    )
    .clicked()
    .then_some(true)
}

fn sync_adjustment_targets(layout: &mut LayoutState, prefs: &UiPreferences) {
    let reopening_all = layout.adjust_open
        && layout.adjustment_selection == layout.selected
        && !layout.hud_adjust_open
        && !layout.group_adjust_open;
    if layout.adjustment_selection == layout.selected && !reopening_all {
        return;
    }
    layout.adjustment_selection = layout.selected.clone();
    layout.adjust_open = layout.selected.is_some();

    let Some(selected) = layout.selected.clone() else {
        return;
    };
    if selected.starts_with("instance/") {
        if !layout.hud_adjust_open {
            record_adjustment_open(layout, "hud-adjust");
        }
        layout.hud_adjust_open = true;
        layout.hud_adjust_key = Some(selected.clone());
        if let Some(instance_id) = selected.strip_prefix("instance/")
            && let Some(group) = prefs.hud.groups.iter().find(|group| {
                group
                    .children
                    .iter()
                    .any(|child| child.as_str() == instance_id)
            })
        {
            if !layout.group_adjust_open {
                record_adjustment_open(layout, "group-adjust");
            }
            layout.group_adjust_open = true;
            layout.group_adjust_key = Some(format!("group/{}", group.id));
        }
    } else if selected.starts_with("group/") {
        if !layout.group_adjust_open {
            record_adjustment_open(layout, "group-adjust");
        }
        layout.group_adjust_open = true;
        layout.group_adjust_key = Some(selected);
    }
}

fn record_adjustment_open(layout: &mut LayoutState, panel: &'static str) {
    layout.adjustment_order.retain(|current| *current != panel);
    layout.adjustment_order.push(panel);
    layout.adjustment_window_revision = layout.adjustment_window_revision.wrapping_add(1);
}

fn adjustment_panel_top(layout: &LayoutState, panel: &str) -> f32 {
    let max_height = layout
        .activity_size
        .map(|size| (size.y - 64.0).max(360.0))
        .unwrap_or(720.0);
    let mut top = EDITOR_PANEL_TOP;
    for current in &layout.adjustment_order {
        if *current == panel {
            break;
        }
        top += EDITOR_PANEL_HEIGHT.min(max_height) + EDITOR_PANEL_GAP;
    }
    top
}

/// 绘制 HUD 子窗口并返回根据子窗口计算出的 HUD 尺寸。
pub(super) fn draw(
    ui: &mut egui::Ui,
    time: f32,
    layout: &mut LayoutState,
    items: &[HudRenderItem],
    registry: &deskhud_engine::EngineRegistry,
    catalogs: &deskhud_ui::CatalogStore,
    prefs: &mut UiPreferences,
) -> DrawResult {
    let mut bounds = egui::Rect::NOTHING;
    let mut changed = false;
    let mut finish_layout_requested = false;
    let mut discard_layout_requested = false;
    project_absolute_positions(layout);
    if layout.layout_mode {
        layout.rendered_rects.clear();
    }
    let mut editor_overlays = Vec::new();
    let mut editor_action = None;
    let mut active_group_drag: Option<(String, egui::Vec2, bool)> = None;
    let mut alignment_guides = AlignmentSnap::default();
    let drag_released =
        layout.active_hud_drag.is_some() && ui.input(|input| input.pointer.primary_released());
    let root_drag_released =
        layout.root_dragging && ui.input(|input| input.pointer.primary_released());
    advance_hud_drag(layout, ui.input(|input| input.pointer.delta()));
    let canvas_response = ui.interact(
        ui.max_rect(),
        ui.make_persistent_id("hud-layout-canvas"),
        egui::Sense::click_and_drag(),
    );
    if layout.layout_mode {
        let canvas_rect = ui.max_rect();
        let position = ui
            .input(|input| input.pointer.interact_pos())
            .map(|point| point - canvas_rect.min.to_vec2())
            .or_else(|| canvas_response.interact_pointer_pos())
            .unwrap_or(egui::Pos2::ZERO);
        let mut menu_snap_to_grid = layout.snap_to_grid;
        let mut menu_lock_ratio = layout.lock_ratio;
        let mut menu_information_tree_open = layout.information_tree_open;
        let mut menu_active_tree_open = layout.active_tree_open;
        let mut menu_changed = false;
        canvas_response.context_menu(|ui| {
            style_canvas_context_menu(ui, prefs.locale);
            draw_tree_panel_menu(
                ui,
                &mut menu_information_tree_open,
                &mut menu_active_tree_open,
                prefs.locale,
            );
            crate::menu::embedded_menu_separator(ui);
            menu_changed |= draw_layout_options_menu(
                ui,
                &mut menu_snap_to_grid,
                &mut menu_lock_ratio,
                prefs.locale,
            );
            crate::menu::embedded_menu_separator(ui);
            if context_menu_button(
                ui,
                deskhud_ui::i18n::t(prefs.locale, MessageKey::HudGroupCreate),
                Some("create-filled"),
            )
            .clicked()
            {
                editor_action = Some(EditorAction::CreateGroup {
                    member: None,
                    position,
                });
                ui.close();
            }
            if let Some(discard) = draw_layout_exit_context_menu(ui, prefs.locale) {
                discard_layout_requested = discard;
                finish_layout_requested = !discard;
                ui.close();
            }
        });
        if menu_changed {
            layout.snap_to_grid = menu_snap_to_grid;
            layout.lock_ratio = menu_lock_ratio;
            changed = true;
        }
        layout.information_tree_open = menu_information_tree_open;
        layout.active_tree_open = menu_active_tree_open;
    }
    if layout.layout_mode {
        draw_preview_background(ui, preview_bounds(layout, items).expand(HUD_PADDING));
    }
    if layout.layout_mode
        && layout.snap_to_grid
        && let Some(activity) = layout.activity_size
    {
        draw_alignment_grid(ui, activity);
    }
    for item in items {
        let reset_position =
            adjustment::adjustment_default_position(layout, prefs, items, item, &item.key, false);
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
        let preferred_size = if matches!(item.target, HudLayoutTarget::Group(_)) {
            preferred_size.max(group_min_size())
        } else {
            preferred_size
        };
        let min_size = if matches!(item.target, HudLayoutTarget::Group(_)) {
            group_min_size()
        } else {
            base_size * HUD_SIZE_FACTOR_MIN
        };
        let item_max_size = if matches!(item.target, HudLayoutTarget::Group(_)) {
            egui::Vec2::splat(f32::MAX)
        } else {
            base_size * HUD_SIZE_FACTOR_MAX
        };
        let canvas_bounds = layout.activity_size.map(|activity| {
            egui::Rect::from_min_max(
                egui::pos2(HUD_PADDING, HUD_PADDING),
                egui::pos2(activity.x - HUD_PADDING, activity.y - HUD_PADDING),
            )
        });
        let mut window = egui::Window::new(egui::RichText::new(&item.key).small())
            .id(egui::Id::new((
                "hud-item",
                &item.key,
                layout.adjust_session,
            )))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .movable(false)
            // The HUD visual owns the background. The egui window frame must
            // stay transparent, otherwise its padding/background becomes a
            // second rectangle around the actual HUD panel.
            .frame(egui::Frame::NONE)
            .constrain(false)
            .fixed_pos(*position);
        if layout.layout_mode {
            window = window.fixed_size(preferred_size);
        } else {
            // Layout mode may leave a remembered large Window rectangle in
            // egui memory. Compact HUDs must size themselves to their content
            // again when the editor is closed.
            window = window.auto_sized();
        }
        let response = window.show(ui.ctx(), |ui| {
            draw_frame(ui, item, layout.layout_mode, layout.layout_mode, prefs)
        });
        let Some(response) = response else { continue };
        let Some(frame) = response.inner else {
            continue;
        };
        if layout.layout_mode {
            layout
                .rendered_rects
                .insert(item.key.clone(), frame.body.rect);
        }
        let member_active = frame.members.iter().any(|member| {
            member.response.clicked_by(egui::PointerButton::Primary)
                || member
                    .response
                    .drag_started_by(egui::PointerButton::Primary)
                || member.response.dragged_by(egui::PointerButton::Primary)
                || member.context_response.secondary_clicked()
                || member.resize_drag.is_some()
        });
        if let HudLayoutTarget::Group(group_id) = &item.target {
            for member in &frame.members {
                let member_key = format!("instance/{}", member.instance_id.as_str());
                if let Some(resize) = member.resize_drag {
                    let old_rect = member.rect;
                    let member_min = egui::vec2(
                        member.base_size.width * HUD_SIZE_FACTOR_MIN,
                        member.base_size.height * HUD_SIZE_FACTOR_MIN,
                    );
                    let member_max = egui::vec2(
                        member.base_size.width * HUD_SIZE_FACTOR_MAX,
                        member.base_size.height * HUD_SIZE_FACTOR_MAX,
                    );
                    let inner_bounds = frame.group_inner_rect.unwrap_or(frame.body.rect);
                    let next_rect = constrained_resize_rect(
                        old_rect,
                        resize,
                        member_min,
                        member_max,
                        inner_bounds,
                        layout
                            .lock_ratio
                            .then_some(old_rect.height() / old_rect.width().max(1.0)),
                    );
                    let next_size = next_rect.size();
                    if let Some(instance) = prefs
                        .hud
                        .instances
                        .iter_mut()
                        .find(|instance| instance.id == member.instance_id)
                    {
                        instance.layout.width = next_size.x / member.base_size.width.max(1.0);
                        instance.layout.height = next_size.y / member.base_size.height.max(1.0);
                    }
                    if let Some(instance) = prefs
                        .hud
                        .instances
                        .iter_mut()
                        .find(|instance| instance.id == member.instance_id)
                    {
                        instance.layout.x =
                            (instance.layout.x + next_rect.min.x - old_rect.min.x).max(0.0);
                        instance.layout.y =
                            (instance.layout.y + next_rect.min.y - old_rect.min.y).max(0.0);
                    }
                    changed = true;
                }
                if member.response.clicked_by(egui::PointerButton::Primary)
                    || member
                        .response
                        .drag_started_by(egui::PointerButton::Primary)
                {
                    layout.selected = Some(member_key.clone());
                    layout.adjust_open = true;
                }
                if member.context_response.secondary_clicked() {
                    layout.selected = Some(member_key.clone());
                }
                if member
                    .response
                    .drag_started_by(egui::PointerButton::Primary)
                {
                    editor_action = Some(EditorAction::BeginHudDrag {
                        member: member.instance_id.clone(),
                        source_group_id: Some(group_id.clone()),
                        source_group_rect: Some(frame.body.rect),
                        position: member.rect.min,
                        size: member.rect.size(),
                        initial_delta: member.response.drag_delta(),
                    });
                }
                let mut menu_snap_to_grid = layout.snap_to_grid;
                let mut menu_lock_ratio = layout.lock_ratio;
                let mut menu_changed = false;
                member.context_response.context_menu(|ui| {
                    style_hud_context_menu(ui, prefs.locale);
                    menu_changed |= draw_layout_options_menu(
                        ui,
                        &mut menu_snap_to_grid,
                        &mut menu_lock_ratio,
                        prefs.locale,
                    );
                    crate::menu::embedded_menu_separator(ui);
                    if context_menu_button(
                        ui,
                        deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustResetPosition),
                        Some("reset"),
                    )
                    .clicked()
                    {
                        editor_action = Some(EditorAction::ResetMemberPosition(
                            member.instance_id.clone(),
                        ));
                        ui.close();
                    }
                    if context_menu_button(
                        ui,
                        deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustResetSize),
                        Some("reset"),
                    )
                    .clicked()
                    {
                        editor_action =
                            Some(EditorAction::ResetHudSize(member.instance_id.clone()));
                        ui.close();
                    }
                    crate::menu::embedded_menu_separator(ui);
                    if context_menu_button(
                        ui,
                        deskhud_ui::i18n::t(prefs.locale, MessageKey::HudLayoutCloseInformation),
                        Some("close"),
                    )
                    .clicked()
                    {
                        editor_action = Some(EditorAction::DisableHud(member.instance_id.clone()));
                        ui.close();
                    }
                });
                if menu_changed {
                    layout.snap_to_grid = menu_snap_to_grid;
                    layout.lock_ratio = menu_lock_ratio;
                    changed = true;
                }
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
            && (frame.body.clicked_by(egui::PointerButton::Primary)
                || frame.body.drag_started_by(egui::PointerButton::Primary)
                || frame.resize_started)
        {
            layout.selected = Some(item.key.clone());
            if frame.body.clicked_by(egui::PointerButton::Primary) {
                layout.adjust_open = true;
            }
        }
        if layout.layout_mode && !member_active && frame.context_response.secondary_clicked() {
            layout.selected = Some(item.key.clone());
        }
        if layout.layout_mode && !member_active {
            let mut menu_snap_to_grid = layout.snap_to_grid;
            let mut menu_lock_ratio = layout.lock_ratio;
            let mut menu_changed = false;
            frame
                .context_response
                .context_menu(|ui| match &item.target {
                    HudLayoutTarget::Instance(id) => {
                        style_hud_context_menu(ui, prefs.locale);
                        menu_changed |= draw_layout_options_menu(
                            ui,
                            &mut menu_snap_to_grid,
                            &mut menu_lock_ratio,
                            prefs.locale,
                        );
                        crate::menu::embedded_menu_separator(ui);
                        if context_menu_button(
                            ui,
                            deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustResetPosition),
                            Some("reset"),
                        )
                        .clicked()
                        {
                            editor_action = Some(EditorAction::ResetTopLevelPosition {
                                key: item.key.clone(),
                                position: reset_position,
                            });
                            ui.close();
                        }
                        if context_menu_button(
                            ui,
                            deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustResetSize),
                            Some("reset"),
                        )
                        .clicked()
                        {
                            editor_action = Some(EditorAction::ResetHudSize(id.clone()));
                            ui.close();
                        }
                        crate::menu::embedded_menu_separator(ui);
                        if context_menu_button(
                            ui,
                            deskhud_ui::i18n::t(
                                prefs.locale,
                                MessageKey::HudLayoutCloseInformation,
                            ),
                            Some("close"),
                        )
                        .clicked()
                        {
                            editor_action = Some(EditorAction::DisableHud(id.clone()));
                            ui.close();
                        }
                    }
                    HudLayoutTarget::Group(id) => {
                        style_group_context_menu(ui, prefs.locale);
                        menu_changed |= draw_layout_options_menu(
                            ui,
                            &mut menu_snap_to_grid,
                            &mut menu_lock_ratio,
                            prefs.locale,
                        );
                        crate::menu::embedded_menu_separator(ui);
                        if context_menu_button(
                            ui,
                            deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustResetPosition),
                            Some("reset"),
                        )
                        .clicked()
                        {
                            editor_action = Some(EditorAction::ResetTopLevelPosition {
                                key: item.key.clone(),
                                position: reset_position,
                            });
                            ui.close();
                        }
                        if context_menu_button(
                            ui,
                            deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustResetSize),
                            Some("reset"),
                        )
                        .clicked()
                        {
                            editor_action = Some(EditorAction::ResetGroupSize(id.clone()));
                            ui.close();
                        }
                        crate::menu::embedded_menu_separator(ui);
                        if context_menu_button(
                            ui,
                            deskhud_ui::i18n::t(prefs.locale, MessageKey::HudGroupDelete),
                            Some("close"),
                        )
                        .clicked()
                        {
                            editor_action = Some(EditorAction::DeleteGroup(id.clone()));
                            ui.close();
                        }
                    }
                });
            if menu_changed {
                layout.snap_to_grid = menu_snap_to_grid;
                layout.lock_ratio = menu_lock_ratio;
                changed = true;
            }
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
                    if layout.active_hud_drag.is_none()
                        && drag_response.drag_started_by(egui::PointerButton::Primary) =>
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
                HudLayoutTarget::Group(group_id)
                    if drag_response.dragged_by(egui::PointerButton::Primary)
                        || drag_response.drag_stopped_by(egui::PointerButton::Primary) =>
                {
                    if drag_response.dragged_by(egui::PointerButton::Primary) {
                        // `Response::drag_delta()` is cumulative. Applying it
                        // every frame accelerates the group and can skip past
                        // the activity edge, so use the pointer's frame delta.
                        *position += ui.input(|input| input.pointer.delta());
                        if let Some(activity) = layout.activity_size {
                            *position =
                                clamp_hud_position(*position, frame.body.rect.size(), activity);
                        }
                    }
                    active_group_drag = Some((
                        format!("group/{group_id}"),
                        frame.body.rect.size(),
                        drag_response.drag_stopped_by(egui::PointerButton::Primary),
                    ));
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
            if !member_active
                && let Some(resize) = frame.resize_drag
                && let Some(mut slot) = layout_slot(prefs, item)
            {
                // Use the rectangle egui actually painted, not the requested
                // cache position. Window placement is rounded internally;
                // mixing the two coordinate sources accumulates tiny member
                // offsets during a continuous resize.
                let old_rect = frame.body.rect;
                let resize_min_size = if matches!(item.target, HudLayoutTarget::Group(_)) {
                    group_resize_min_size(&frame, old_rect, resize.edges)
                } else {
                    min_size
                };
                let next_rect = constrained_resize_rect(
                    old_rect,
                    resize,
                    resize_min_size,
                    item_max_size,
                    canvas_bounds.unwrap_or(egui::Rect::EVERYTHING),
                    layout
                        .lock_ratio
                        .then_some(old_rect.height() / old_rect.width().max(1.0)),
                )
                .round_ui();
                *position = next_rect.min;
                let next_size = next_rect.size();
                if let HudLayoutTarget::Group(group_id) = &item.target {
                    let old_member_rects = frame
                        .members
                        .iter()
                        .map(|member| (member.instance_id.clone(), member.rect))
                        .collect::<Vec<_>>();
                    let mut next_padding = [0.0; 4];
                    if let Some(group) = prefs
                        .hud
                        .groups
                        .iter_mut()
                        .find(|group| &group.id == group_id)
                    {
                        group.layout.width = next_size.x;
                        group.layout.height = next_size.y;
                        let horizontal_limit = group.layout.width * 0.25;
                        let vertical_limit = group.layout.height * 0.25;
                        group.inner.padding[0] = group.inner.padding[0].min(vertical_limit).floor();
                        group.inner.padding[2] = group.inner.padding[2].min(vertical_limit).floor();
                        group.inner.padding[1] =
                            group.inner.padding[1].min(horizontal_limit).floor();
                        group.inner.padding[3] =
                            group.inner.padding[3].min(horizontal_limit).floor();
                        next_padding = effective_group_padding(group, next_size);
                    }
                    // Resizing the top or left group edge changes the group
                    // origin. Members are independent content, so translate
                    // their local coordinates by the opposite amount to keep
                    // every HUD at the same screen position.
                    let [top, _, _, left] = next_padding;
                    for (instance_id, member_rect) in old_member_rects {
                        if let Some(instance) = prefs
                            .hud
                            .instances
                            .iter_mut()
                            .find(|instance| instance.id == instance_id)
                        {
                            let local = stable_member_local_position(
                                member_rect.min,
                                next_rect.min,
                                egui::vec2(left, top),
                            );
                            instance.layout.x = local.x;
                            instance.layout.y = local.y;
                        }
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
        bounds = bounds.union(egui::Rect::from_min_size(
            *position,
            response.response.rect.size(),
        ));
    }

    // All overlays are available only after the item pass. Apply group
    // alignment here so a group can attract to HUDs that are painted later
    // in preference order as well as to those painted earlier.
    if let Some((key, size, released)) = active_group_drag
        && let Some(mut position) = layout.positions.get(&key).copied()
    {
        if released
            && layout.snap_to_grid
            && let Some(activity) = layout.activity_size
        {
            position.x = snap_coordinate(position.x, activity.x);
            position.y = snap_coordinate(position.y, activity.y);
        }
        let alignment = alignment_snap_for_drag(
            egui::Rect::from_min_size(position, size),
            &editor_overlays,
            &key,
            layout.activity_size,
            prefs,
            layout.snap_to_grid,
        );
        if released {
            position += alignment.delta;
        } else {
            alignment_guides = alignment;
        }
        if let Some(activity) = layout.activity_size {
            position = clamp_hud_position(position, size, activity);
        }
        if layout.positions.get(&key).copied() != Some(position) {
            layout.positions.insert(key, position);
            changed = true;
        }
    }

    // A dragged member is temporarily represented as a top-level HUD. Snap
    // its screen rectangle against every other visible HUD/group before the
    // drop target is evaluated, so the persisted group-local coordinates use
    // the aligned position too.
    if let Some(drag) = layout.active_hud_drag.as_ref() {
        let key = format!("instance/{}", drag.instance_id.as_str());
        let size = drag.size;
        let mut position = drag.position;
        if drag_released
            && layout.snap_to_grid
            && let Some(activity) = layout.activity_size
        {
            position.x = snap_coordinate(position.x, activity.x);
            position.y = snap_coordinate(position.y, activity.y);
        }
        let alignment = alignment_snap_for_drag(
            egui::Rect::from_min_size(position, size),
            &editor_overlays,
            &key,
            layout.activity_size,
            prefs,
            layout.snap_to_grid,
        );
        if drag_released {
            position += alignment.delta;
        } else {
            alignment_guides = alignment;
        }
        if let Some(activity) = layout.activity_size {
            position = clamp_hud_position(position, size, activity);
        }
        if let Some(drag) = layout.active_hud_drag.as_mut() {
            if drag.position != position {
                changed = true;
            }
            drag.position = position;
        }
        layout.positions.insert(key, position);
    }

    if layout.layout_mode
        && alignment_guides.is_active()
        && let Some(activity) = layout.activity_size
    {
        draw_alignment_guides(ui, alignment_guides, activity);
    }

    if let Some(action) = editor_action {
        changed |= apply_editor_action(action, layout, prefs, &editor_overlays);
    }

    // The compact preview acts as a virtual root group. Its empty surface
    // moves all top-level HUDs/groups together while preserving child
    // geometry. Child overlays win the hit test, so HUD drags remain precise.
    if layout.layout_mode {
        let preview_rect = bounds.expand(HUD_PADDING);
        let pointer_pos = ui.input(|input| input.pointer.interact_pos());
        let pointer_over_child = pointer_pos.is_some_and(|pointer| {
            editor_overlays
                .iter()
                .any(|overlay| overlay.rect.contains(pointer))
        });
        if canvas_response.drag_started_by(egui::PointerButton::Primary)
            && pointer_pos.is_some_and(|pointer| preview_rect.contains(pointer))
            && !pointer_over_child
        {
            layout.root_dragging = true;
        }
        if !ui.input(|input| input.pointer.primary_down()) {
            layout.root_dragging = false;
        }
        if layout.root_dragging && canvas_response.dragged_by(egui::PointerButton::Primary) {
            let delta = root_drag_delta(
                preview_rect,
                ui.input(|input| input.pointer.delta()),
                layout.activity_size,
            );
            if delta != egui::Vec2::ZERO {
                for position in layout.positions.values_mut() {
                    *position += delta;
                }
                changed = true;
            }
        }
        if root_drag_released && layout.snap_to_grid {
            // The preview is a virtual root group. Snap that root rectangle,
            // not each child independently, so the internal HUD geometry is
            // preserved while the whole preview lands on the grid.
            let preview_rect = preview_bounds(layout, items).expand(HUD_PADDING);
            if let Some(activity) = layout.activity_size {
                let delta = root_grid_snap_delta(preview_rect, activity);
                if delta != egui::Vec2::ZERO {
                    for position in layout.positions.values_mut() {
                        *position += delta;
                    }
                    changed = true;
                }
            }
        }
    }

    // Commit the latest canvas positions before rendering the adjustment
    // windows. Otherwise a drag and a panel read can happen in the same
    // frame with different position sources, making groups visibly flicker.
    if layout.layout_mode {
        sync_absolute_positions(layout);
    }

    let active_drop = layout.active_hud_drag.as_ref().map(|drag| {
        let hud_rect = egui::Rect::from_min_size(drag.position, drag.size);
        let target_group_id = group_drop_target(&editor_overlays, hud_rect, prefs)
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
        changed |= finish_hud_drag_after_release(
            layout,
            prefs,
            &editor_overlays,
            target_group_id.as_deref(),
        );
    }

    if layout.layout_mode {
        draw_editor_overlays(ui, time, &editor_overlays, layout.selected.as_deref());
        changed |= tree_panels::draw(ui, layout, registry, catalogs, prefs);
    }

    if layout.layout_mode {
        sync_adjustment_targets(layout, prefs);
    }
    if layout.layout_mode && layout.adjust_open {
        let selected_is_group = layout
            .selected
            .as_deref()
            .is_some_and(|selected| selected.starts_with("group/"));
        let panel_order = layout.adjustment_order.clone();
        // Render later-opened panels first so the first-opened panel remains
        // visually on top, matching their vertical order.
        for panel in panel_order.iter().rev() {
            let (open, key, group_window, interactive) = match *panel {
                "hud-adjust" => (
                    layout.hud_adjust_open,
                    layout.hud_adjust_key.clone(),
                    false,
                    !selected_is_group,
                ),
                "group-adjust" => (
                    layout.group_adjust_open,
                    layout.group_adjust_key.clone(),
                    true,
                    selected_is_group,
                ),
                _ => continue,
            };
            if !open {
                continue;
            }
            let Some(key) = key else { continue };
            let panel_top = adjustment_panel_top(layout, panel);
            changed |= draw_adjust_window(
                ui,
                layout,
                items,
                prefs,
                key,
                panel,
                group_window,
                interactive,
                panel_top,
            );
        }
        layout.adjust_open = layout.hud_adjust_open || layout.group_adjust_open;
        if !layout.adjust_open {
            layout.adjustment_order.clear();
            layout.adjustment_reset_sizes.clear();
        }
    }

    layout.finish_layout_requested |= finish_layout_requested;
    layout.discard_layout_requested |= discard_layout_requested;

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
        sync_absolute_positions(layout);
        // The outer border is the active editing canvas. The inner border is
        // the compact window that will be restored after leaving layout mode.
        // Keeping both visible makes the pending normal-mode geometry clear.
        draw_preview_border(ui, bounds.expand(HUD_PADDING));
        draw_border(
            ui,
            time,
            egui::Rect::from_min_size(egui::Pos2::ZERO, activity_size),
        );
        return DrawResult {
            size: [activity_size.x, activity_size.y],
            move_by: None,
            changed,
        };
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
        // Normal mode uses the persisted native-window preset and the
        // persisted window-local item coordinates. It must not apply the
        // legacy "move content to padding, then move the native window"
        // correction a second time.
        move_by: None,
        changed,
    }
}

fn preview_bounds(layout: &mut LayoutState, items: &[HudRenderItem]) -> egui::Rect {
    let mut bounds = egui::Rect::NOTHING;
    for item in items {
        let position = layout
            .positions
            .entry(item.key.clone())
            .or_insert(item.initial_position);
        let base_size = egui::vec2(item.base_size.width, item.base_size.height);
        let size = item.container_size.unwrap_or_else(|| {
            egui::vec2(
                base_size.x * item.width.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
                base_size.y * item.height.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
            )
        });
        let size = if matches!(item.target, HudLayoutTarget::Group(_)) {
            size.max(group_min_size())
        } else {
            size
        };
        bounds = bounds.union(egui::Rect::from_min_size(*position, size));
    }
    bounds
}

fn root_drag_delta(
    preview_rect: egui::Rect,
    delta: egui::Vec2,
    activity_size: Option<egui::Vec2>,
) -> egui::Vec2 {
    let Some(activity) = activity_size else {
        return delta;
    };
    let activity_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, activity);
    let min_delta = activity_rect.min - preview_rect.min;
    let max_delta = activity_rect.max - preview_rect.max;
    egui::vec2(
        clamp_root_axis(delta.x, min_delta.x, max_delta.x),
        clamp_root_axis(delta.y, min_delta.y, max_delta.y),
    )
}

fn clamp_root_axis(value: f32, min: f32, max: f32) -> f32 {
    if min <= max {
        value.clamp(min, max)
    } else {
        0.0
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

/// Returns the translation needed to align the moving rectangle's nearest
/// edge/centre with another visible HUD or group. X and Y are solved
/// independently, allowing corner, edge, and centre alignment in one drag.
fn alignment_snap(
    moving: egui::Rect,
    overlays: &[EditorOverlay],
    moving_key: &str,
    activity_size: Option<egui::Vec2>,
    prefs: &UiPreferences,
) -> AlignmentSnap {
    let moving_group_id = moving_key.strip_prefix("group/");
    let moving_x = [moving.left(), moving.center().x, moving.right()];
    let moving_y = [moving.top(), moving.center().y, moving.bottom()];
    let mut target_x = Vec::new();
    let mut target_y = Vec::new();

    // The layout window is a virtual root: it contributes only its centre
    // lines, while concrete HUD/group rectangles contribute all three
    // edge/centre lines below.
    if let Some(activity) = activity_size {
        target_x.push(activity.x * 0.5);
        target_y.push(activity.y * 0.5);
    }

    for overlay in overlays {
        // The compact layout preview is a virtual root, not an alignable HUD.
        // Only concrete instance/group overlays may provide alignment lines.
        if !is_alignable_overlay(&overlay.key)
            || overlay.key == moving_key
            || moving_group_id.is_some_and(|group_id| {
                overlay
                    .key
                    .strip_prefix("instance/")
                    .is_some_and(|instance_id| {
                        prefs.hud.groups.iter().any(|group| {
                            group.id == group_id
                                && group
                                    .children
                                    .iter()
                                    .any(|child| child.as_str() == instance_id)
                        })
                    })
            })
        {
            continue;
        }
        target_x.extend([
            overlay.rect.left(),
            overlay.rect.center().x,
            overlay.rect.right(),
        ]);
        target_y.extend([
            overlay.rect.top(),
            overlay.rect.center().y,
            overlay.rect.bottom(),
        ]);
    }

    let x = nearest_alignment(&moving_x, &target_x);
    let y = nearest_alignment(&moving_y, &target_y);
    AlignmentSnap {
        delta: egui::vec2(
            x.map_or(0.0, |(delta, _)| delta),
            y.map_or(0.0, |(delta, _)| delta),
        ),
        vertical: x.map(|(_, target)| target),
        horizontal: y.map(|(_, target)| target),
    }
}

fn is_alignable_overlay(key: &str) -> bool {
    key.starts_with("instance/") || key.starts_with("group/")
}

fn alignment_snap_for_drag(
    moving: egui::Rect,
    overlays: &[EditorOverlay],
    moving_key: &str,
    activity_size: Option<egui::Vec2>,
    prefs: &UiPreferences,
    snap_to_grid: bool,
) -> AlignmentSnap {
    if snap_to_grid {
        AlignmentSnap::default()
    } else {
        alignment_snap(moving, overlays, moving_key, activity_size, prefs)
    }
}

fn nearest_alignment(moving: &[f32; 3], targets: &[f32]) -> Option<(f32, f32)> {
    moving
        .iter()
        .flat_map(|moving| targets.iter().map(move |target| (target - moving, *target)))
        .filter(|(delta, _)| delta.abs() <= ALIGNMENT_SNAP_DISTANCE)
        .min_by(|(left, _), (right, _)| left.abs().total_cmp(&right.abs()))
}

fn root_grid_snap_delta(preview_rect: egui::Rect, activity: egui::Vec2) -> egui::Vec2 {
    let desired = egui::vec2(
        snap_coordinate(preview_rect.min.x, activity.x) - preview_rect.min.x,
        snap_coordinate(preview_rect.min.y, activity.y) - preview_rect.min.y,
    );
    root_drag_delta(preview_rect, desired, Some(activity))
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

fn group_drop_target<'a>(
    overlays: &'a [EditorOverlay],
    hud_rect: egui::Rect,
    prefs: &UiPreferences,
) -> Option<&'a str> {
    // Groups are painted in preference order, so the last matching group is
    // the visible/topmost one. Choosing the first match makes overlapping
    // groups steal drops from the group the user is actually pointing at.
    overlays.iter().rev().find_map(|overlay| {
        let group_id = overlay.key.strip_prefix("group/")?;
        let content_rect = if let Some(group) =
            prefs.hud.groups.iter().find(|group| group.id == group_id)
        {
            let [top, right, bottom, left] = effective_group_padding(group, overlay.rect.size());
            egui::Rect::from_min_max(
                overlay.rect.min + egui::vec2(left, top),
                overlay.rect.max - egui::vec2(right, bottom),
            )
        } else {
            // Keep editor hit-testing tolerant of a frame that has not yet
            // been reflected in prefs during the same interaction frame.
            overlay.rect
        };
        rect_contains_rect(content_rect, hud_rect).then_some(group_id)
    })
}

fn rect_contains_rect(container: egui::Rect, contained: egui::Rect) -> bool {
    contained.min.x >= container.min.x
        && contained.min.y >= container.min.y
        && contained.max.x <= container.max.x
        && contained.max.y <= container.max.y
}

fn effective_group_padding(group: &deskhud_ui::HudGroup, size: egui::Vec2) -> [f32; 4] {
    let [top, right, bottom, left] = group.inner.clone().normalized().padding;
    [
        top.min(size.y * 0.25).floor(),
        right.min(size.x * 0.25).floor(),
        bottom.min(size.y * 0.25).floor(),
        left.min(size.x * 0.25).floor(),
    ]
}

/// Converts a member's canvas rectangle into the only coordinate that is
/// persisted for a grouped instance: the member's position in its parent
/// group's content space. Both arguments are egui canvas coordinates.
fn group_member_local_position(
    member_min: egui::Pos2,
    group_rect: egui::Rect,
    padding: [f32; 4],
) -> egui::Pos2 {
    let [top, _, _, left] = padding;
    egui::pos2(
        (member_min.x - group_rect.min.x - left).max(0.0),
        (member_min.y - group_rect.min.y - top).max(0.0),
    )
}

pub(super) fn finish_active_hud_drag_as_screen(
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
) -> bool {
    finish_hud_drag(layout, prefs, &[], None)
}

fn finish_hud_drag_after_release(
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    overlays: &[EditorOverlay],
    target_group_id: Option<&str>,
) -> bool {
    finish_hud_drag_inner(layout, prefs, overlays, target_group_id, false)
}

fn finish_hud_drag(
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    overlays: &[EditorOverlay],
    target_group_id: Option<&str>,
) -> bool {
    finish_hud_drag_inner(layout, prefs, overlays, target_group_id, true)
}

fn finish_hud_drag_inner(
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    overlays: &[EditorOverlay],
    target_group_id: Option<&str>,
    snap_to_grid: bool,
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
        if let Some(source_group_id) = drag.source_group_id.as_deref() {
            layout.transient_group_sizes.remove(source_group_id);
        }
        layout.transient_group_sizes.remove(group_id);
        let changed = prefs.hud.add_instance_to_group(group_id, &drag.instance_id);
        if changed
            && let Some(group) = prefs
                .hud
                .groups
                .iter_mut()
                .find(|group| group.id == group_id)
        {
            let padding = effective_group_padding(group, group_rect.size());
            let drop_position = if snap_to_grid && layout.snap_to_grid {
                let activity = layout.activity_size.unwrap_or(egui::vec2(1.0, 1.0));
                egui::pos2(
                    layout::snap_coordinate(drag.position.x, activity.x),
                    layout::snap_coordinate(drag.position.y, activity.y),
                )
            } else {
                drag.position
            };
            if let Some(instance) = prefs
                .hud
                .instances
                .iter_mut()
                .find(|instance| instance.id == drag.instance_id)
            {
                let local = group_member_local_position(drop_position, group_rect, padding);
                instance.layout.x = local.x;
                instance.layout.y = local.y;
            }
        }
        layout.positions.remove(&key);
        // A grouped member is no longer a top-level layout slot. Remove its
        // old screen-space cache as well, otherwise leave_layout_mode can
        // write that stale global position back as group-local coordinates.
        layout.absolute_positions.remove(&key);
        layout.selected = Some(key);
        layout.window_revision = layout.window_revision.wrapping_add(1);
        return changed;
    }

    let Some(activity) = layout.activity_size else {
        if let Some(source_group_id) = drag.source_group_id.as_deref() {
            layout.transient_group_sizes.remove(source_group_id);
        }
        return false;
    };
    if let Some(source_group_id) = drag.source_group_id.as_deref() {
        layout.transient_group_sizes.remove(source_group_id);
    }
    let mut x = drag.position.x.max(0.0);
    let mut y = drag.position.y.max(0.0);
    if snap_to_grid && layout.snap_to_grid {
        x = layout::snap_coordinate(x, activity.x);
        y = layout::snap_coordinate(y, activity.y);
    }
    layout.positions.insert(key.clone(), egui::pos2(x, y));
    sync_absolute_positions(layout);
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
            if let Some(member) = member {
                prefs.hud.add_instance_to_group(&id, &member);
                // An instance that becomes the first member of a newly
                // created group must start in group-content coordinates.
                // Its old layout is a window-local position and cannot be
                // reused after the ownership change.
                if let Some(instance) = prefs
                    .hud
                    .instances
                    .iter_mut()
                    .find(|instance| instance.id == member)
                {
                    instance.layout.x = 0.0;
                    instance.layout.y = 0.0;
                }
                let member_key = format!("instance/{}", member.as_str());
                layout.positions.remove(&member_key);
                layout.absolute_positions.remove(&member_key);
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
        EditorAction::ResetTopLevelPosition { key, position } => {
            layout.positions.insert(key.clone(), position);
            sync_absolute_positions(layout);
            layout.adjustment_reset_sizes.remove(&key);
            layout.window_revision = layout.window_revision.wrapping_add(1);
            true
        }
        EditorAction::ResetMemberPosition(member) => {
            let Some(instance) = prefs
                .hud
                .instances
                .iter_mut()
                .find(|instance| instance.id == member)
            else {
                return false;
            };
            instance.layout.x = 0.0;
            instance.layout.y = 0.0;
            layout
                .adjustment_reset_sizes
                .remove(&format!("instance/{}", member.as_str()));
            layout.window_revision = layout.window_revision.wrapping_add(1);
            true
        }
        EditorAction::ResetHudSize(instance_id) => {
            let Some(instance) = prefs
                .hud
                .instances
                .iter_mut()
                .find(|instance| instance.id == instance_id)
            else {
                return false;
            };
            instance.layout.width = 1.0;
            instance.layout.height = 1.0;
            layout
                .adjustment_reset_sizes
                .remove(&format!("instance/{}", instance_id.as_str()));
            layout.window_revision = layout.window_revision.wrapping_add(1);
            true
        }
        EditorAction::ResetGroupSize(id) => {
            let Some(group) = prefs.hud.groups.iter_mut().find(|group| group.id == id) else {
                return false;
            };
            // A zero group size means content-sized automatic layout.
            group.layout.width = 0.0;
            group.layout.height = 0.0;
            layout.transient_group_sizes.remove(&id);
            layout.adjustment_reset_sizes.remove(&format!("group/{id}"));
            layout.window_revision = layout.window_revision.wrapping_add(1);
            true
        }
        EditorAction::DisableHud(instance_id) => {
            let Some(instance) = prefs
                .hud
                .instances
                .iter_mut()
                .find(|instance| instance.id == instance_id)
            else {
                return false;
            };
            if !instance.enabled {
                return false;
            }
            instance.enabled = false;
            let key = format!("instance/{}", instance_id.as_str());
            layout.positions.remove(&key);
            layout.absolute_positions.remove(&key);
            if layout.selected.as_deref() == Some(key.as_str()) {
                layout.selected = None;
                layout.adjustment_selection = None;
                layout.adjust_open = false;
                layout.hud_adjust_open = false;
                layout.hud_adjust_key = None;
                layout.shadow_open = false;
            }
            layout.window_revision = layout.window_revision.wrapping_add(1);
            true
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
            let detached = source_group_id.as_ref().is_some_and(|group_id| {
                let changed = prefs.hud.remove_instance_from_group(&member);
                if changed {
                    // Keep the group at its current visible size while the
                    // member is being dragged. This prevents the group from
                    // resizing on every drag frame. This is transient and is
                    // deliberately not written to the group's persisted size.
                    if let Some(rect) = source_group_rect {
                        layout
                            .transient_group_sizes
                            .insert(group_id.clone(), rect.size());
                    }
                }
                changed
            });
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
    }
}

/// Keeps the transient global screen-coordinate map in sync with the
/// activity-canvas projection used by egui interaction.
pub(super) fn sync_absolute_positions(layout: &mut LayoutState) {
    let Some(origin) = layout.activity_origin else {
        return;
    };
    let scale = layout.scale_factor.max(1.0);
    // absolute_positions is only a derived cache for top-level slots. Never
    // retain a member that has just been moved into a group.
    layout
        .absolute_positions
        .retain(|key, _| layout.positions.contains_key(key));
    for (key, position) in &layout.positions {
        layout
            .absolute_positions
            .insert(key.clone(), origin + position.to_vec2() * scale);
    }
}

/// Projects the authoritative global positions into the expanded activity
/// canvas used by egui. Global coordinates must remain the source of truth;
/// the canvas map is only a transient rendering/hit-testing projection.
fn project_absolute_positions(layout: &mut LayoutState) {
    let Some(origin) = layout.activity_origin else {
        return;
    };
    let scale = layout.scale_factor.max(1.0);
    for (key, absolute) in &layout.absolute_positions {
        layout
            .positions
            .insert(key.clone(), (*absolute - origin.to_vec2()) / scale);
    }
}

/// Smallest group size for the active edges while keeping every member inside
/// the padded inner border at its current screen position.
fn group_resize_min_size(
    frame: &frame::FrameResponse,
    group_rect: egui::Rect,
    edges: frame::ResizeEdges,
) -> egui::Vec2 {
    let Some(member_bounds) = frame
        .members
        .iter()
        .map(|member| member.rect)
        .reduce(|bounds, rect| bounds.union(rect))
    else {
        return group_min_size();
    };
    let inner = frame.group_inner_rect.unwrap_or(group_rect);
    group_resize_min_size_for_bounds(group_rect, inner, member_bounds, edges).max(group_min_size())
}

fn group_min_size() -> egui::Vec2 {
    egui::vec2(GROUP_MIN_WIDTH, GROUP_MIN_HEIGHT)
}

fn group_resize_min_size_for_bounds(
    group_rect: egui::Rect,
    inner: egui::Rect,
    member_bounds: egui::Rect,
    edges: frame::ResizeEdges,
) -> egui::Vec2 {
    let top = inner.top() - group_rect.top();
    let right = group_rect.right() - inner.right();
    let bottom = group_rect.bottom() - inner.bottom();
    let left = inner.left() - group_rect.left();
    let width = if edges.left {
        group_rect.right() - (member_bounds.left() - left)
    } else {
        member_bounds.right() + right - group_rect.left()
    };
    let height = if edges.top {
        group_rect.bottom() - (member_bounds.top() - top)
    } else {
        member_bounds.bottom() + bottom - group_rect.top()
    };
    egui::vec2(width.max(1.0), height.max(1.0))
}

fn stable_member_local_position(
    member_screen: egui::Pos2,
    group_screen: egui::Pos2,
    padding: egui::Vec2,
) -> egui::Pos2 {
    let local = member_screen - group_screen - padding;
    egui::pos2(local.x.max(0.0), local.y.max(0.0)).round_ui()
}

/// Applies one resize gesture without ever allowing egui's outer-window
/// constraint to push the opposite edge. When an aspect ratio is supplied,
/// both axes share one scale interval: reaching any edge stops both axes.
fn constrained_resize_rect(
    old: egui::Rect,
    resize: frame::ResizeDrag,
    min_size: egui::Vec2,
    max_size: egui::Vec2,
    bounds: egui::Rect,
    aspect_ratio: Option<f32>,
) -> egui::Rect {
    let horizontal = resize.edges.left || resize.edges.right;
    let vertical = resize.edges.top || resize.edges.bottom;
    let width_limit = if resize.edges.left {
        old.right() - bounds.left()
    } else {
        bounds.right() - old.left()
    };
    let height_limit = if resize.edges.top {
        old.bottom() - bounds.top()
    } else {
        bounds.bottom() - old.top()
    };
    let max_size = egui::vec2(
        max_size.x.min(width_limit).max(min_size.x),
        max_size.y.min(height_limit).max(min_size.y),
    );

    let desired_width = if resize.edges.left {
        old.width() - resize.delta.x
    } else if resize.edges.right {
        old.width() + resize.delta.x
    } else {
        old.width()
    };
    let desired_height = if resize.edges.top {
        old.height() - resize.delta.y
    } else if resize.edges.bottom {
        old.height() + resize.delta.y
    } else {
        old.height()
    };

    let size = if let Some(ratio) = aspect_ratio.filter(|ratio| ratio.is_finite() && *ratio > 0.0) {
        let min_width = min_size.x.max(min_size.y / ratio);
        let max_width = max_size.x.min(max_size.y / ratio).max(min_width);
        let requested_width = if horizontal {
            desired_width
        } else if vertical {
            desired_height / ratio
        } else {
            old.width()
        };
        let width = requested_width.clamp(min_width, max_width);
        egui::vec2(width, width * ratio)
    } else {
        egui::vec2(
            desired_width.clamp(min_size.x, max_size.x),
            desired_height.clamp(min_size.y, max_size.y),
        )
    };

    let min = egui::pos2(
        if resize.edges.left {
            old.right() - size.x
        } else {
            old.left()
        },
        if resize.edges.top {
            old.bottom() - size.y
        } else {
            old.top()
        },
    );
    egui::Rect::from_min_size(min, size)
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
    fn context_menus_keep_the_requested_actions_and_order() {
        assert_eq!(
            CANVAS_CONTEXT_MENU_KEYS,
            &[
                MessageKey::HudLayoutInformationTree,
                MessageKey::HudLayoutActiveTree,
                MessageKey::HudAdjustSnapGrid,
                MessageKey::HudAdjustLockRatio,
                MessageKey::HudGroupCreate,
                MessageKey::HudLayoutDone,
                MessageKey::HudLayoutCancel,
            ]
        );
        assert_eq!(
            GROUP_CONTEXT_MENU_KEYS,
            &[
                MessageKey::HudAdjustSnapGrid,
                MessageKey::HudAdjustLockRatio,
                MessageKey::HudAdjustResetPosition,
                MessageKey::HudAdjustResetSize,
                MessageKey::HudGroupDelete,
            ]
        );
        assert_eq!(
            HUD_CONTEXT_MENU_KEYS,
            &[
                MessageKey::HudAdjustSnapGrid,
                MessageKey::HudAdjustLockRatio,
                MessageKey::HudAdjustResetPosition,
                MessageKey::HudAdjustResetSize,
                MessageKey::HudLayoutCloseInformation,
            ]
        );
    }

    #[test]
    fn context_menu_reset_and_close_actions_update_the_selected_records() {
        let (mut prefs, first, _) = prefs_with_instances();
        let group_id = prefs.hud.create_group("Group");
        prefs.hud.instances[0].layout.x = 24.0;
        prefs.hud.instances[0].layout.y = 36.0;
        prefs.hud.instances[0].layout.width = 2.0;
        prefs.hud.instances[0].layout.height = 2.5;
        prefs.hud.groups[0].layout.width = 400.0;
        prefs.hud.groups[0].layout.height = 300.0;
        let mut layout = LayoutState::default();
        let key = format!("instance/{}", first.as_str());
        layout.positions.insert(key.clone(), egui::pos2(90.0, 80.0));

        assert!(apply_editor_action(
            EditorAction::ResetTopLevelPosition {
                key: key.clone(),
                position: egui::pos2(8.0, 8.0),
            },
            &mut layout,
            &mut prefs,
            &[],
        ));
        assert_eq!(layout.positions[&key], egui::pos2(8.0, 8.0));
        assert!(apply_editor_action(
            EditorAction::ResetHudSize(first.clone()),
            &mut layout,
            &mut prefs,
            &[],
        ));
        assert_eq!(prefs.hud.instances[0].layout.width, 1.0);
        assert_eq!(prefs.hud.instances[0].layout.height, 1.0);
        assert!(apply_editor_action(
            EditorAction::ResetGroupSize(group_id),
            &mut layout,
            &mut prefs,
            &[],
        ));
        assert_eq!(prefs.hud.groups[0].layout.width, 0.0);
        assert_eq!(prefs.hud.groups[0].layout.height, 0.0);
        assert!(apply_editor_action(
            EditorAction::DisableHud(first),
            &mut layout,
            &mut prefs,
            &[],
        ));
        assert!(!prefs.hud.instances[0].enabled);
    }

    #[test]
    fn adjustment_panels_use_the_shared_editor_size_and_stack_step() {
        let layout = LayoutState {
            adjustment_order: vec!["hud-adjust", "group-adjust"],
            ..LayoutState::default()
        };

        assert_eq!(
            adjustment_panel_top(&layout, "hud-adjust"),
            EDITOR_PANEL_TOP
        );
        assert_eq!(
            adjustment_panel_top(&layout, "group-adjust"),
            EDITOR_PANEL_TOP + EDITOR_PANEL_HEIGHT + EDITOR_PANEL_GAP
        );
    }

    #[test]
    fn alignment_snap_matches_edges_and_centres_independently() {
        let overlays = [EditorOverlay {
            key: "instance/target".to_owned(),
            rect: egui::Rect::from_min_max(egui::pos2(100.0, 100.0), egui::pos2(200.0, 200.0)),
            layer_id: egui::LayerId::background(),
            corner_radius: 0.0,
        }];
        let moving = egui::Rect::from_min_size(egui::pos2(123.0, 203.0), egui::vec2(40.0, 40.0));
        let delta = alignment_snap(
            moving,
            &overlays,
            "instance/moving",
            None,
            &UiPreferences::default(),
        )
        .delta;

        // The moving centre is 7 px from the target centre, while its top is
        // 3 px below the target bottom.
        assert_eq!(delta, egui::vec2(7.0, -3.0));
        let snap = alignment_snap(
            moving,
            &overlays,
            "instance/moving",
            None,
            &UiPreferences::default(),
        );
        assert_eq!(snap.vertical, Some(150.0));
        assert_eq!(snap.horizontal, Some(200.0));
    }

    #[test]
    fn alignment_snap_does_not_use_a_groups_own_members() {
        let (mut prefs, first, _) = prefs_with_instances();
        let group_id = prefs.hud.create_group("Group");
        prefs.hud.add_instance_to_group(&group_id, &first);
        let overlays = [EditorOverlay {
            key: format!("instance/{}", first.as_str()),
            rect: egui::Rect::from_min_max(egui::pos2(100.0, 100.0), egui::pos2(200.0, 200.0)),
            layer_id: egui::LayerId::background(),
            corner_radius: 0.0,
        }];

        assert_eq!(
            alignment_snap(
                egui::Rect::from_min_size(egui::pos2(103.0, 103.0), egui::vec2(40.0, 40.0)),
                &overlays,
                &format!("group/{group_id}"),
                None,
                &prefs,
            )
            .delta,
            egui::Vec2::ZERO
        );
    }

    #[test]
    fn alignment_snap_ignores_the_virtual_layout_preview() {
        let overlays = [EditorOverlay {
            key: "layout-preview".to_owned(),
            rect: egui::Rect::from_min_max(egui::pos2(100.0, 100.0), egui::pos2(200.0, 200.0)),
            layer_id: egui::LayerId::background(),
            corner_radius: 0.0,
        }];

        let snap = alignment_snap(
            egui::Rect::from_min_size(egui::pos2(203.0, 203.0), egui::vec2(40.0, 40.0)),
            &overlays,
            "instance/moving",
            None,
            &UiPreferences::default(),
        );
        assert!(!snap.is_active());
        assert_eq!(snap.delta, egui::Vec2::ZERO);
    }

    #[test]
    fn alignment_snap_uses_the_layout_window_centre() {
        let moving = egui::Rect::from_min_size(egui::pos2(303.0, 223.0), egui::vec2(40.0, 40.0));
        let snap = alignment_snap(
            moving,
            &[],
            "instance/moving",
            Some(egui::vec2(640.0, 480.0)),
            &UiPreferences::default(),
        );

        assert_eq!(snap.delta, egui::vec2(-3.0, -3.0));
        assert_eq!(snap.vertical, Some(320.0));
        assert_eq!(snap.horizontal, Some(240.0));
    }

    #[test]
    fn grouped_hud_can_align_to_its_own_group_edges_and_centres() {
        let (mut prefs, first, _) = prefs_with_instances();
        let group_id = prefs.hud.create_group("Group");
        prefs.hud.add_instance_to_group(&group_id, &first);
        let overlays = [EditorOverlay {
            key: format!("group/{group_id}"),
            rect: egui::Rect::from_min_max(egui::pos2(100.0, 100.0), egui::pos2(200.0, 200.0)),
            layer_id: egui::LayerId::background(),
            corner_radius: 0.0,
        }];

        let snap = alignment_snap(
            egui::Rect::from_min_size(egui::pos2(203.0, 203.0), egui::vec2(40.0, 40.0)),
            &overlays,
            &format!("instance/{}", first.as_str()),
            None,
            &prefs,
        );
        assert_eq!(snap.delta, egui::vec2(-3.0, -3.0));
        assert_eq!(snap.vertical, Some(200.0));
        assert_eq!(snap.horizontal, Some(200.0));
    }

    #[test]
    fn grid_snap_disables_hud_alignment_guides() {
        let overlays = [EditorOverlay {
            key: "instance/target".to_owned(),
            rect: egui::Rect::from_min_max(egui::pos2(100.0, 100.0), egui::pos2(200.0, 200.0)),
            layer_id: egui::LayerId::background(),
            corner_radius: 0.0,
        }];
        let snap = alignment_snap_for_drag(
            egui::Rect::from_min_size(egui::pos2(123.0, 203.0), egui::vec2(40.0, 40.0)),
            &overlays,
            "instance/moving",
            None,
            &UiPreferences::default(),
            true,
        );
        assert!(!snap.is_active());
        assert_eq!(snap.delta, egui::Vec2::ZERO);
    }

    #[test]
    fn root_grid_snap_aligns_the_preview_as_one_rectangle() {
        let preview = egui::Rect::from_min_size(egui::pos2(45.0, 77.0), egui::vec2(240.0, 120.0));
        assert_eq!(
            root_grid_snap_delta(preview, egui::vec2(640.0, 480.0)),
            egui::vec2(-13.0, -13.0)
        );
    }

    #[test]
    fn grouped_drag_detaches_then_rejoins_with_group_relative_coordinates() {
        let (mut prefs, first, _) = prefs_with_instances();
        let mut layout = LayoutState {
            layout_mode: true,
            activity_size: Some(egui::vec2(1000.0, 800.0)),
            activity_origin: Some(egui::pos2(0.0, 0.0)),
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
            group_drop_target(&overlays, hud_rect, &prefs),
            Some(group_id.as_str())
        );
        let member_key = format!("instance/{}", first.as_str());
        layout
            .absolute_positions
            .insert(member_key.clone(), egui::pos2(900.0, 700.0));
        assert!(finish_hud_drag(
            &mut layout,
            &mut prefs,
            &overlays,
            Some(&group_id),
        ));
        let group = &prefs.hud.groups[0];
        assert!(group.children.contains(&first));
        let instance = prefs
            .hud
            .instances
            .iter()
            .find(|instance| instance.id == first)
            .unwrap();
        assert_eq!((instance.layout.x, instance.layout.y), (78.0, 60.0));
        assert!(!layout.absolute_positions.contains_key(&member_key));
        assert!(layout.transient_group_sizes.is_empty());
    }

    #[test]
    fn creating_group_converts_existing_instance_position_to_group_coordinates() {
        let (mut prefs, first, _) = prefs_with_instances();
        prefs
            .hud
            .instances
            .iter_mut()
            .find(|instance| instance.id == first)
            .unwrap()
            .layout
            .x = 640.0;
        prefs
            .hud
            .instances
            .iter_mut()
            .find(|instance| instance.id == first)
            .unwrap()
            .layout
            .y = 360.0;
        let mut layout = LayoutState {
            layout_mode: true,
            activity_size: Some(egui::vec2(1200.0, 800.0)),
            ..LayoutState::default()
        };

        assert!(apply_editor_action(
            EditorAction::CreateGroup {
                member: Some(first.clone()),
                position: egui::pos2(640.0, 360.0),
            },
            &mut layout,
            &mut prefs,
            &[],
        ));

        let instance = prefs
            .hud
            .instances
            .iter()
            .find(|instance| instance.id == first)
            .unwrap();
        assert_eq!((instance.layout.x, instance.layout.y), (0.0, 0.0));
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
        let prefs = UiPreferences::default();
        assert_eq!(group_drop_target(&overlays, inside, &prefs), Some("one"));
        assert_eq!(group_drop_target(&overlays, partial, &prefs), None);
    }

    #[test]
    fn drop_target_prefers_the_topmost_matching_group() {
        let overlays = [
            EditorOverlay {
                key: "group/first".to_owned(),
                rect: egui::Rect::from_min_max(egui::pos2(100.0, 100.0), egui::pos2(320.0, 320.0)),
                layer_id: egui::LayerId::background(),
                corner_radius: 0.0,
            },
            EditorOverlay {
                key: "group/second".to_owned(),
                rect: egui::Rect::from_min_max(egui::pos2(140.0, 140.0), egui::pos2(300.0, 300.0)),
                layer_id: egui::LayerId::background(),
                corner_radius: 0.0,
            },
        ];
        let hud = egui::Rect::from_min_size(egui::pos2(180.0, 180.0), egui::vec2(60.0, 40.0));
        let prefs = UiPreferences::default();
        assert_eq!(group_drop_target(&overlays, hud, &prefs), Some("second"));
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
            activity_origin: Some(egui::pos2(0.0, 0.0)),
            ..LayoutState::default()
        };
        assert!(finish_hud_drag(&mut layout, &mut prefs, &[], None));
        let instance = prefs
            .hud
            .instances
            .iter()
            .find(|instance| instance.id == first)
            .unwrap();
        assert_eq!((instance.layout.x, instance.layout.y), (8.0, 8.0));
        assert_eq!(
            layout
                .absolute_positions
                .get(&format!("instance/{}", first.as_str())),
            Some(&egui::pos2(500.0, 320.0))
        );
        assert_eq!(
            layout
                .positions
                .get(&format!("instance/{}", first.as_str())),
            Some(&egui::pos2(500.0, 320.0))
        );
        assert!(layout.active_hud_drag.is_none());
    }

    #[test]
    fn layout_canvas_projection_roundtrips_physical_slot_coordinates() {
        let mut layout = LayoutState {
            activity_origin: Some(egui::pos2(100.0, 200.0)),
            scale_factor: 2.0,
            ..LayoutState::default()
        };
        layout
            .absolute_positions
            .insert("group/one".to_owned(), egui::pos2(500.0, 700.0));

        project_absolute_positions(&mut layout);
        assert_eq!(layout.positions["group/one"], egui::pos2(200.0, 250.0));

        sync_absolute_positions(&mut layout);
        assert_eq!(
            layout.absolute_positions["group/one"],
            egui::pos2(500.0, 700.0)
        );
    }

    #[test]
    fn ratio_resize_stops_both_axes_at_the_first_boundary() {
        let old = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(100.0, 50.0));
        let resized = constrained_resize_rect(
            old,
            frame::ResizeDrag {
                edges: frame::ResizeEdges {
                    left: false,
                    right: true,
                    top: false,
                    bottom: false,
                },
                delta: egui::vec2(200.0, 0.0),
            },
            egui::vec2(20.0, 10.0),
            egui::vec2(400.0, 400.0),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(500.0, 100.0)),
            Some(0.5),
        );
        assert_eq!(resized.size(), egui::vec2(180.0, 90.0));
        assert_eq!(resized.min, old.min);
    }

    #[test]
    fn resize_at_minimum_does_not_move_the_opposite_edge() {
        let old = egui::Rect::from_min_size(egui::pos2(40.0, 30.0), egui::vec2(80.0, 40.0));
        let resized = constrained_resize_rect(
            old,
            frame::ResizeDrag {
                edges: frame::ResizeEdges {
                    left: true,
                    right: false,
                    top: false,
                    bottom: false,
                },
                delta: egui::vec2(100.0, 0.0),
            },
            egui::vec2(60.0, 20.0),
            egui::vec2(300.0, 300.0),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(500.0, 500.0)),
            None,
        );
        assert_eq!(resized.width(), 60.0);
        assert_eq!(resized.right(), old.right());
    }

    #[test]
    fn group_edges_stop_when_the_inner_border_reaches_a_member() {
        let group = egui::Rect::from_min_max(egui::pos2(100.0, 100.0), egui::pos2(400.0, 300.0));
        let inner = group.shrink(10.0);
        let member = egui::Rect::from_min_max(egui::pos2(150.0, 140.0), egui::pos2(250.0, 190.0));
        let right_min = group_resize_min_size_for_bounds(
            group,
            inner,
            member,
            frame::ResizeEdges {
                left: false,
                right: true,
                top: false,
                bottom: false,
            },
        );
        let top_min = group_resize_min_size_for_bounds(
            group,
            inner,
            member,
            frame::ResizeEdges {
                left: false,
                right: false,
                top: true,
                bottom: false,
            },
        );
        assert_eq!(right_min.x, 160.0);
        assert_eq!(top_min.y, 170.0);
    }

    #[test]
    fn group_fixed_minimum_stops_resize() {
        let old = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(240.0, 140.0));
        let resized = constrained_resize_rect(
            old,
            frame::ResizeDrag {
                edges: frame::ResizeEdges {
                    left: false,
                    right: true,
                    top: false,
                    bottom: true,
                },
                delta: egui::vec2(-500.0, -500.0),
            },
            group_min_size(),
            egui::Vec2::splat(f32::MAX),
            egui::Rect::EVERYTHING,
            None,
        );
        assert_eq!(
            resized.size(),
            egui::vec2(GROUP_MIN_WIDTH, GROUP_MIN_HEIGHT),
        );
    }

    #[test]
    fn repeated_group_resizes_keep_member_screen_position_stable() {
        let member_screen = egui::pos2(183.75, 147.5);
        let padding = egui::vec2(8.0, 8.0);
        let mut group =
            egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(240.0, 140.0));
        for _ in 0..24 {
            group = constrained_resize_rect(
                group,
                frame::ResizeDrag {
                    edges: frame::ResizeEdges {
                        left: false,
                        right: true,
                        top: true,
                        bottom: false,
                    },
                    delta: egui::vec2(0.375, -0.375),
                },
                egui::vec2(120.0, 70.0),
                egui::vec2(800.0, 800.0),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1200.0, 800.0)),
                Some(140.0 / 240.0),
            )
            .round_ui();
            let local = stable_member_local_position(member_screen, group.min, padding);
            let reconstructed = group.min + padding + local.to_vec2();
            assert!((reconstructed - member_screen).length_sq() < 1e-6);
        }
    }
}
