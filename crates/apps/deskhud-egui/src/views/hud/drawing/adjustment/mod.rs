//! HUD 调整窗口流程编排。

mod common;
mod effects;
mod shadow;

use common::*;
use effects::draw_effects_group;
use shadow::draw_shadow_window;

use super::layout::{
    draw_ratio_lock_control, draw_snap_grid_control, layout_slot, set_layout_slot, snap_normalized,
};
use super::*;
use crate::components;

pub(super) fn draw_adjust_window(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    items: &[HudRenderItem],
    prefs: &mut UiPreferences,
    key: String,
    window_id: &'static str,
    group_window: bool,
) -> bool {
    let direct_item = items.iter().find(|item| item.key == key);
    let owned_item = direct_item
        .is_none()
        .then(|| {
            let instance_id = key.strip_prefix("instance/")?;
            let (group_item, layer) = items.iter().find_map(|item| {
                item.layers
                    .iter()
                    .find(|layer| layer.instance_id.as_str() == instance_id)
                    .map(|layer| (item, layer))
            })?;
            let mut item = group_item.clone();
            item.key = key.clone();
            item.target = HudLayoutTarget::Instance(layer.instance_id.clone());
            item.source = Some(layer.source.clone());
            item.layers = vec![layer.clone()];
            item.base_size = layer.base_size;
            Some(item)
        })
        .flatten();
    let Some(item) = direct_item.or(owned_item.as_ref()) else {
        return false;
    };
    let grouped_member = matches!(&item.target, HudLayoutTarget::Instance(id) if prefs.hud.groups.iter().any(|group| group.children.contains(id)));
    let Some(mut slot) = layout_slot(prefs, item) else {
        return false;
    };
    let initial_width = slot.width;
    let initial_height = slot.height;
    let initial_ratio = layout
        .locked_ratio
        .unwrap_or(initial_height / initial_width.max(0.001));
    let initial_lock_ratio = layout.lock_ratio;
    let mut changed = false;
    let mut width_changed = false;
    let mut height_changed = false;
    let mut open = if group_window {
        layout.group_adjust_open
    } else {
        layout.hud_adjust_open
    };
    let max_panel_height = layout
        .activity_size
        .map(|size| (size.y - 64.0).max(360.0))
        .unwrap_or(720.0);
    // Default to the space the current editor needs, but keep first-open
    // windows compact. `max_height` below remains the activity-area limit,
    // so users can still manually expand a window past 500 px.
    let content_height: f32 = if matches!(item.target, HudLayoutTarget::Group(_)) {
        390.0
    } else {
        620.0
    };
    let default_panel_height = content_height.min(500.0).min(max_panel_height);

    let title_key = if matches!(item.target, HudLayoutTarget::Group(_)) {
        MessageKey::HudGroupAdjustTitle
    } else {
        MessageKey::HudAdjustTitle
    };
    egui::Window::new(egui::RichText::new(deskhud_ui::i18n::t(prefs.locale, title_key)).strong())
        // Window placement belongs to the editor type, not to an individual
        // group or HUD. Switching between group 1 and group 2 should keep
        // the same group-editor location.
        .id(egui::Id::new((window_id, layout.adjust_session)))
        .default_pos(layout.activity_size.map_or(egui::pos2(24.0, 32.0), |size| {
            if group_window {
                egui::pos2(24.0, 32.0)
            } else {
                egui::pos2((size.x - ADJUST_PANEL_WIDTH - 24.0).max(24.0), 32.0)
            }
        }))
        .default_width(ADJUST_PANEL_WIDTH)
        .default_height(default_panel_height)
        .min_width(ADJUST_PANEL_WIDTH)
        .max_width(ADJUST_PANEL_WIDTH)
        .min_height(320.0)
        .max_height(max_panel_height)
        .resizable([false, true])
        .collapsible(false)
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let HudLayoutTarget::Group(id) = &item.target {
                        changed |= draw_group_settings(ui, prefs, id);
                        ui.add_space(8.0);
                    }
                    if let HudLayoutTarget::Instance(instance_id) = &item.target {
                        draw_hud_info(ui, prefs.locale, item, instance_id);
                        ui.add_space(8.0);
                    }
                    if !grouped_member {
                        changed |= draw_position_editor(
                            ui,
                            layout,
                            prefs.locale,
                            PositionTarget::Slot(&mut slot),
                        );
                        ui.add_space(8.0);
                    } else if let HudLayoutTarget::Instance(instance_id) = &item.target {
                        let locale = prefs.locale;
                        if let Some((x, y)) = prefs.hud.groups.iter_mut().find_map(|group| {
                            if !group.children.iter().any(|id| id == instance_id) {
                                return None;
                            }
                            group.member_layouts.iter_mut().find_map(|member| {
                                (&member.instance_id == instance_id)
                                    .then_some((&mut member.x, &mut member.y))
                            })
                        }) {
                            changed |= draw_position_editor(
                                ui,
                                layout,
                                locale,
                                PositionTarget::Member { x, y },
                            );
                        }
                        ui.add_space(8.0);
                    }
                    let (size_changed, width_was_changed, height_was_changed) =
                        if let HudLayoutTarget::Group(id) = &item.target {
                            (
                                draw_group_size_group(
                                    ui,
                                    layout,
                                    prefs,
                                    id,
                                    item.base_size,
                                    item.width,
                                    item.height,
                                ),
                                false,
                                false,
                            )
                        } else {
                            draw_size_group(ui, layout, prefs, &mut slot, item, initial_ratio)
                        };
                    changed |= size_changed;
                    width_changed = width_was_changed;
                    height_changed = height_was_changed;
                    if let HudLayoutTarget::Instance(instance_id) = &item.target {
                        ui.add_space(8.0);
                        changed |= draw_effects_group(ui, layout, prefs, instance_id, item);
                    }
                });
        });
    if layout.shadow_open
        && let HudLayoutTarget::Instance(instance_id) = &item.target
    {
        changed |= draw_shadow_window(
            ui,
            layout,
            prefs,
            item,
            instance_id,
            layout.shadow_target.unwrap_or(ShadowTarget::Global),
        );
    }
    if group_window {
        layout.group_adjust_open = open;
    } else {
        layout.hud_adjust_open = open;
    }
    if layout.lock_ratio && !initial_lock_ratio {
        layout.locked_ratio = Some(initial_height / initial_width.max(0.001));
    } else if !layout.lock_ratio {
        layout.locked_ratio = None;
    }
    if layout.lock_ratio {
        let ratio = layout.locked_ratio.unwrap_or(initial_ratio).max(0.001);
        if width_changed && !height_changed {
            slot.height = (slot.width * ratio).clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
            slot.width = (slot.height / ratio).clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
            changed = true;
        } else if height_changed && !width_changed {
            slot.width = (slot.height / ratio).clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
            slot.height = (slot.width * ratio).clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
            changed = true;
        }
    }
    if changed {
        if (slot.width - initial_width).abs() > 0.0001
            || (slot.height - initial_height).abs() > 0.0001
        {
            layout.window_revision = layout.window_revision.wrapping_add(1);
        }
        let (slot_x, slot_y) = (slot.x, slot.y);
        if grouped_member {
            // Group-member coordinates are logical pixels, unlike the
            // normalized 0..1 screen coordinates used by a top-level slot.
            // Do not pass them through set_layout_slot(), which clamps them.
            if let HudLayoutTarget::Instance(instance_id) = &item.target
                && let Some(group) = prefs
                    .hud
                    .groups
                    .iter_mut()
                    .find(|group| group.children.contains(instance_id))
                && let Some(member) = group
                    .member_layouts
                    .iter_mut()
                    .find(|member| &member.instance_id == instance_id)
            {
                member.x = slot_x.max(0.0);
                member.y = slot_y.max(0.0);
            }
            if let HudLayoutTarget::Instance(instance_id) = &item.target
                && let Some(instance) = prefs
                    .hud
                    .instances
                    .iter_mut()
                    .find(|instance| &instance.id == instance_id)
            {
                instance.layout.width = slot.width;
                instance.layout.height = slot.height;
            }
        } else {
            set_layout_slot(prefs, item, slot);
        }
        if !grouped_member
            && let Some(pos) = layout.positions.get_mut(&key)
            && let Some(activity) = layout.activity_size
        {
            pos.x = slot_x * activity.x;
            pos.y = slot_y * activity.y;
        }
    }
    changed
}

fn draw_hud_info(
    ui: &mut egui::Ui,
    locale: deskhud_ui::Locale,
    item: &HudRenderItem,
    instance_id: &deskhud_engine::HudInstanceId,
) {
    components::config_card(
        ui,
        Some(
            egui::RichText::new(deskhud_ui::i18n::t(locale, MessageKey::HudAdjustInfo))
                .strong()
                .into(),
        ),
        |ui| {
            draw_read_only_info_row(
                ui,
                locale,
                MessageKey::HudAdjustInstance,
                instance_id.as_str(),
                true,
            );
            draw_read_only_info_row(
                ui,
                locale,
                MessageKey::HudAdjustPlugin,
                &item.plugin_name,
                true,
            );
            draw_read_only_info_row(
                ui,
                locale,
                MessageKey::HudAdjustContribution,
                &item.contribution_name,
                false,
            );
        },
        None,
    );
}

fn draw_read_only_info_row(
    ui: &mut egui::Ui,
    locale: deskhud_ui::Locale,
    label: MessageKey,
    value: &str,
    show_divider: bool,
) {
    components::config_row_with_divider(
        ui,
        deskhud_ui::i18n::t(locale, label),
        None::<egui::RichText>,
        show_divider,
        |ui| {
            let mut value = value.to_owned();
            ui.add_sized(
                egui::vec2(216.0, ADJUST_ROW_HEIGHT),
                egui::TextEdit::singleline(&mut value)
                    .interactive(false)
                    .font(egui::TextStyle::Monospace)
                    .horizontal_align(egui::Align::Min)
                    .vertical_align(egui::Align::Center),
            )
            .on_hover_text(value);
        },
    );
}

fn draw_group_settings(ui: &mut egui::Ui, prefs: &mut UiPreferences, group_id: &str) -> bool {
    use deskhud_engine::HudGroupArrangement;

    let locale = prefs.locale;
    let Some(group) = prefs
        .hud
        .groups
        .iter_mut()
        .find(|group| group.id == group_id)
    else {
        return false;
    };
    let mut changed = false;
    components::config_card(
        ui,
        Some(
            egui::RichText::new(deskhud_ui::i18n::t(locale, MessageKey::HudGroupInfo))
                .strong()
                .into(),
        ),
        |ui| {
            components::config_row_with_divider(
                ui,
                deskhud_ui::i18n::t(locale, MessageKey::HudGroupName),
                None::<egui::RichText>,
                true,
                |ui| {
                    ui.add_space(8.0);
                    changed |= ui
                        .add_sized(
                            egui::vec2(200.0, ADJUST_ROW_HEIGHT),
                            egui::TextEdit::singleline(&mut group.name)
                                .vertical_align(egui::Align::Center),
                        )
                        .changed();
                    ui.add_space(8.0);
                },
            );
            components::config_row_with_divider(
                ui,
                deskhud_ui::i18n::t(locale, MessageKey::HudGroupColor),
                None::<egui::RichText>,
                true,
                |ui| {
                    changed |= draw_hex_color_control(
                        ui,
                        &mut group.color,
                        ui.make_persistent_id(("hud-group-color", group_id)),
                    );
                },
            );
            let mut padding = group.inner.padding[0];
            let padding_limit = (group.size[0].min(group.size[1]).max(0.0) * 0.25).floor();
            components::config_row_with_divider(
                ui,
                deskhud_ui::i18n::t(locale, MessageKey::HudGroupPadding),
                None::<egui::RichText>,
                true,
                |ui| {
                    if ui
                        .add_sized(
                            egui::vec2(216.0, ADJUST_ROW_HEIGHT),
                            egui::DragValue::new(&mut padding)
                                .fixed_decimals(0)
                                .range(0.0..=padding_limit)
                                .suffix(" px"),
                        )
                        .changed()
                    {
                        group.inner.padding = [padding.clamp(0.0, padding_limit).floor(); 4];
                        changed = true;
                    }
                },
            );
            group.inner.arrangement = HudGroupArrangement::Free;
            components::config_row_with_divider(
                ui,
                deskhud_ui::i18n::t(locale, MessageKey::HudGroupArrangement),
                None::<egui::RichText>,
                false,
                |ui| {
                    let selected = "free";
                    let options = vec![(
                        selected.to_owned(),
                        group_arrangement_label(locale, HudGroupArrangement::Free).to_owned(),
                    )];
                    if components::dropdown_with_style(
                        ui,
                        ("hud-group-arrangement", group_id),
                        selected,
                        &options,
                        false,
                        components::DropdownStyle::ADJUSTMENT,
                    )
                    .is_some()
                    {
                        group.inner.arrangement = HudGroupArrangement::Free;
                    }
                },
            );
        },
        None,
    );
    if changed {
        group.inner = group.inner.clone().normalized();
    }
    changed
}

fn group_arrangement_label(
    locale: deskhud_ui::Locale,
    arrangement: deskhud_engine::HudGroupArrangement,
) -> &'static str {
    let key = match arrangement {
        deskhud_engine::HudGroupArrangement::Free => MessageKey::HudGroupArrangementFree,
        deskhud_engine::HudGroupArrangement::Horizontal => {
            MessageKey::HudGroupArrangementHorizontal
        }
        deskhud_engine::HudGroupArrangement::Vertical => MessageKey::HudGroupArrangementVertical,
        deskhud_engine::HudGroupArrangement::Grid => MessageKey::HudGroupArrangementGrid,
    };
    deskhud_ui::i18n::t(locale, key)
}

enum PositionTarget<'a> {
    Slot(&'a mut deskhud_ui::HudSlotLayout),
    Member { x: &'a mut f32, y: &'a mut f32 },
}

fn draw_position_editor(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    locale: deskhud_ui::Locale,
    target: PositionTarget<'_>,
) -> bool {
    let snap_to_grid = std::cell::Cell::new(layout.snap_to_grid);
    let snap_control_changed = std::cell::Cell::new(false);
    let mut changed = false;
    components::config_card_with_header(
        ui,
        |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), ADJUST_ROW_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new(deskhud_ui::i18n::t(
                            locale,
                            MessageKey::HudAdjustPosition,
                        ))
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        snap_control_changed.set(draw_snap_grid_control(ui, &snap_to_grid, locale));
                    });
                },
            );
        },
        |ui| {
            let activity = layout.activity_size.unwrap_or(egui::vec2(1.0, 1.0));
            let member_target = matches!(&target, PositionTarget::Member { .. });
            let (x, y) = match target {
                PositionTarget::Slot(slot) => (&mut slot.x, &mut slot.y),
                PositionTarget::Member { x, y } => (x, y),
            };
            for (index, (label, value, pixels)) in [
                (MessageKey::HudAdjustX, x, activity.x),
                (MessageKey::HudAdjustY, y, activity.y),
            ]
            .into_iter()
            .enumerate()
            {
                let mut shown = if member_target {
                    *value
                } else {
                    *value * pixels
                };
                let mut value_changed = false;
                components::config_row_with_divider(
                    ui,
                    deskhud_ui::i18n::t(locale, label),
                    None::<egui::RichText>,
                    index == 0,
                    |ui| {
                        value_changed = ui
                            .push_id(("hud-position-value", index), |ui| {
                                ui.add_sized(
                                    egui::vec2(216.0, ADJUST_ROW_HEIGHT),
                                    egui::DragValue::new(&mut shown).speed(1.0).suffix(" px"),
                                )
                                .changed()
                            })
                            .inner;
                    },
                );
                if value_changed {
                    if member_target {
                        *value = shown.max(0.0);
                    } else {
                        *value = (shown / pixels.max(1.0)).clamp(0.0, 1.0);
                        if snap_to_grid.get() {
                            *value = snap_normalized(*value);
                        }
                    }
                    changed = true;
                }
            }
        },
    );
    if snap_control_changed.get() {
        layout.snap_to_grid = snap_to_grid.get();
        changed = true;
    }
    changed
}

fn draw_group_size_group(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    group_id: &str,
    base_size: deskhud_engine::HudLogicalSize,
    legacy_width: f32,
    legacy_height: f32,
) -> bool {
    let Some(group) = prefs
        .hud
        .groups
        .iter_mut()
        .find(|group| group.id == group_id)
    else {
        return false;
    };
    if group.size[0] <= 0.0 || group.size[1] <= 0.0 {
        group.size[0] = (base_size.width * legacy_width.max(0.5)).max(1.0);
        group.size[1] = (base_size.height * legacy_height.max(0.5)).max(1.0);
    }
    let mut changed = false;
    let lock_ratio = std::cell::Cell::new(layout.lock_ratio);
    let ratio_control_changed = std::cell::Cell::new(false);
    let ratio = group.size[1] / group.size[0].max(1.0);
    components::config_card_with_header(
        ui,
        |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), ADJUST_ROW_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new(deskhud_ui::i18n::t(
                            prefs.locale,
                            MessageKey::HudAdjustSize,
                        ))
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ratio_control_changed.set(draw_ratio_lock_control(
                            ui,
                            &lock_ratio,
                            prefs.locale,
                        ));
                    });
                },
            );
        },
        |ui| {
            for (index, label) in [MessageKey::HudAdjustWidth, MessageKey::HudAdjustHeight]
                .into_iter()
                .enumerate()
            {
                let mut value = group.size[index];
                let mut value_changed = false;
                components::config_row_with_divider(
                    ui,
                    deskhud_ui::i18n::t(prefs.locale, label),
                    None::<egui::RichText>,
                    index == 0,
                    |ui| {
                        value_changed = ui
                            .push_id(("hud-group-size", group_id, index), |ui| {
                                ui.add_sized(
                                    egui::vec2(216.0, ADJUST_ROW_HEIGHT),
                                    egui::DragValue::new(&mut value)
                                        .fixed_decimals(1)
                                        .speed(1.0)
                                        .range(1.0..=f32::MAX)
                                        .suffix(" px"),
                                )
                                .changed()
                            })
                            .inner;
                    },
                );
                if value_changed {
                    group.size[index] = value.max(1.0);
                    if lock_ratio.get() {
                        if index == 0 {
                            group.size[1] = (group.size[0] * ratio).max(1.0);
                        } else {
                            group.size[0] = (group.size[1] / ratio.max(0.001)).max(1.0);
                        }
                    }
                    let horizontal_limit = group.size[0] * 0.25;
                    let vertical_limit = group.size[1] * 0.25;
                    group.inner.padding[0] = group.inner.padding[0].min(vertical_limit).floor();
                    group.inner.padding[2] = group.inner.padding[2].min(vertical_limit).floor();
                    group.inner.padding[1] = group.inner.padding[1].min(horizontal_limit).floor();
                    group.inner.padding[3] = group.inner.padding[3].min(horizontal_limit).floor();
                    changed = true;
                }
            }
        },
    );
    if ratio_control_changed.get() {
        layout.lock_ratio = lock_ratio.get();
        changed = true;
    }
    changed
}

fn draw_size_group(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    prefs: &UiPreferences,
    slot: &mut deskhud_ui::HudSlotLayout,
    item: &HudRenderItem,
    ratio: f32,
) -> (bool, bool, bool) {
    let mut changed = false;
    let lock_ratio = std::cell::Cell::new(layout.lock_ratio);
    let ratio_control_changed = std::cell::Cell::new(false);
    let mut width_changed = false;
    let mut height_changed = false;
    let base = egui::vec2(item.base_size.width, item.base_size.height);
    components::config_card_with_header(
        ui,
        |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), ADJUST_ROW_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new(deskhud_ui::i18n::t(
                            prefs.locale,
                            MessageKey::HudAdjustSize,
                        ))
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ratio_control_changed.set(draw_ratio_lock_control(
                            ui,
                            &lock_ratio,
                            prefs.locale,
                        ));
                    });
                },
            );
        },
        |ui| {
            let width_max = if lock_ratio.get() {
                (HUD_SIZE_FACTOR_MAX / ratio.max(0.001))
                    .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX)
            } else {
                HUD_SIZE_FACTOR_MAX
            };
            {
                let mut shown = slot.width * base.x;
                components::config_row_with_divider(
                    ui,
                    deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustWidth),
                    None::<egui::RichText>,
                    true,
                    |ui| {
                        width_changed = ui
                            .add_sized(
                                egui::vec2(216.0, ADJUST_ROW_HEIGHT),
                                egui::DragValue::new(&mut shown)
                                    .speed(1.0)
                                    .range((base.x * HUD_SIZE_FACTOR_MIN)..=(base.x * width_max))
                                    .suffix(" px"),
                            )
                            .changed();
                    },
                );
                if width_changed {
                    slot.width = (shown / base.x.max(1.0)).clamp(HUD_SIZE_FACTOR_MIN, width_max);
                }
                changed |= width_changed;
            }
            let height_max = if lock_ratio.get() {
                (HUD_SIZE_FACTOR_MAX * ratio).clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX)
            } else {
                HUD_SIZE_FACTOR_MAX
            };
            {
                let mut shown = slot.height * base.y;
                components::config_row_with_divider(
                    ui,
                    deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustHeight),
                    None::<egui::RichText>,
                    false,
                    |ui| {
                        height_changed = ui
                            .add_sized(
                                egui::vec2(216.0, ADJUST_ROW_HEIGHT),
                                egui::DragValue::new(&mut shown)
                                    .speed(1.0)
                                    .range((base.y * HUD_SIZE_FACTOR_MIN)..=(base.y * height_max))
                                    .suffix(" px"),
                            )
                            .changed();
                    },
                );
                if height_changed {
                    slot.height = (shown / base.y.max(1.0)).clamp(HUD_SIZE_FACTOR_MIN, height_max);
                }
                changed |= height_changed;
            }
        },
    );
    if ratio_control_changed.get() {
        layout.lock_ratio = lock_ratio.get();
        changed = true;
    }
    (changed, width_changed, height_changed)
}
