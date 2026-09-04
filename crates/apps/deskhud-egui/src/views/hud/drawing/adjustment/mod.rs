//! HUD 调整窗口流程编排。

mod common;
mod effects;
mod shadow;

use common::*;
use effects::draw_effects_group;
use shadow::draw_shadow_window;

use super::layout::layout_slot;
use super::*;
use crate::components;

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_adjust_window(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    items: &[HudRenderItem],
    prefs: &mut UiPreferences,
    key: String,
    window_id: &'static str,
    group_window: bool,
    interactive: bool,
    panel_top: f32,
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
            item.plugin_name = layer.plugin_name.clone();
            item.contribution_name = layer.contribution_name.clone();
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
    // Top-level HUDs and groups move in the transient layout canvas. Their
    // persisted x/y intentionally remain unchanged until layout mode ends,
    // so the adjustment panel must show the live canvas position instead of
    // the stale value from prefs. Group members keep their own local values.
    if layout.layout_mode
        && !grouped_member
        && let Some(position) = layout.positions.get(&key)
    {
        slot.x = position.x;
        slot.y = position.y;
    }
    let initial_width = slot.width;
    let initial_height = slot.height;
    let initial_x = slot.x;
    let initial_y = slot.y;
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
        .id(egui::Id::new((
            window_id,
            layout.adjust_session,
            layout.adjustment_window_revision,
        )))
        .default_pos(
            layout
                .activity_size
                .map_or(egui::pos2(24.0, panel_top), |size| {
                    // Keep adjustment windows in the same right-aligned column.
                    let right = (size.x - ADJUST_PANEL_WIDTH - 24.0).max(24.0);
                    egui::pos2(right, panel_top)
                }),
        )
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
            ui.add_enabled_ui(interactive, |ui| {
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
                            let size = match &item.target {
                                HudLayoutTarget::Group(_) => item
                                    .container_size
                                    .unwrap_or_else(|| {
                                        egui::vec2(item.base_size.width, item.base_size.height)
                                    })
                                    .max(group_min_size()),
                                HudLayoutTarget::Instance(_) => egui::vec2(
                                    item.base_size.width * slot.width,
                                    item.base_size.height * slot.height,
                                ),
                            };
                            changed |= draw_position_editor(
                                ui,
                                layout,
                                prefs.locale,
                                PositionTarget::Slot {
                                    slot: &mut slot,
                                    size,
                                },
                            );
                            ui.add_space(8.0);
                        } else if let HudLayoutTarget::Instance(instance_id) = &item.target {
                            let locale = prefs.locale;
                            if let Some(geometry) = grouped_member_position_geometry(
                                layout,
                                prefs,
                                item,
                                instance_id,
                                &slot,
                            ) {
                                let mut screen_position = geometry.position;
                                let position_changed = draw_position_editor(
                                    ui,
                                    layout,
                                    locale,
                                    PositionTarget::Member {
                                        x: &mut screen_position.x,
                                        y: &mut screen_position.y,
                                        minimum: geometry.minimum,
                                        maximum: geometry.maximum,
                                    },
                                );
                                if position_changed
                                    && let Some(instance) = prefs
                                        .hud
                                        .instances
                                        .iter_mut()
                                        .find(|instance| &instance.id == instance_id)
                                {
                                    let local = (screen_position - geometry.minimum).round_ui();
                                    instance.layout.x = local.x.max(0.0);
                                    instance.layout.y = local.y.max(0.0);
                                }
                                changed |= position_changed;
                            }
                            ui.add_space(8.0);
                        }
                        let (size_changed, width_was_changed, height_was_changed) =
                            if let HudLayoutTarget::Group(id) = &item.target {
                                (
                                    draw_group_size_group(ui, layout, prefs, id, item, &slot),
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
        });
    if layout.shadow_open
        && interactive
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
        let limits = hud_adjustment_size_factor_limits(layout, prefs, &slot, item);
        if width_changed && !height_changed {
            slot.height = (slot.width * ratio).clamp(HUD_SIZE_FACTOR_MIN, limits.y);
            slot.width = (slot.height / ratio).clamp(HUD_SIZE_FACTOR_MIN, limits.x);
            changed = true;
        } else if height_changed && !width_changed {
            slot.width = (slot.height / ratio).clamp(HUD_SIZE_FACTOR_MIN, limits.x);
            slot.height = (slot.width * ratio).clamp(HUD_SIZE_FACTOR_MIN, limits.y);
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
            // physical-pixel window coordinates used by a top-level slot.
            // Do not pass them through set_layout_slot(), which clamps them.
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
            // Top-level positions are transient editor coordinates during
            // layout mode. Keep prefs in window-local coordinates and move
            // the editor's global position by the adjustment delta instead.
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
            if let Some(position) = layout.positions.get_mut(&key) {
                *position += egui::vec2(slot_x - initial_x, slot_y - initial_y);
            }
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
            let padding_limit =
                (group.layout.width.min(group.layout.height).max(0.0) * 0.25).floor();
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
                    let selected = group_arrangement_label(locale);
                    let options = vec![("free".to_owned(), selected.to_owned())];
                    let _ = components::dropdown_with_style(
                        ui,
                        ("hud-group-arrangement", group_id),
                        "free",
                        &options,
                        false,
                        components::DropdownStyle::ADJUSTMENT,
                    );
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

fn group_arrangement_label(locale: deskhud_ui::Locale) -> &'static str {
    deskhud_ui::i18n::t(locale, MessageKey::HudGroupArrangementFree)
}

enum PositionTarget<'a> {
    Slot {
        slot: &'a mut deskhud_ui::HudSlotLayout,
        size: egui::Vec2,
    },
    Member {
        x: &'a mut f32,
        y: &'a mut f32,
        minimum: egui::Pos2,
        maximum: egui::Pos2,
    },
}

#[derive(Clone, Copy)]
struct GroupedMemberPositionGeometry {
    position: egui::Pos2,
    minimum: egui::Pos2,
    maximum: egui::Pos2,
}

fn grouped_member_position_geometry(
    layout: &LayoutState,
    prefs: &UiPreferences,
    item: &HudRenderItem,
    instance_id: &deskhud_engine::HudInstanceId,
    slot: &deskhud_ui::HudSlotLayout,
) -> Option<GroupedMemberPositionGeometry> {
    let group = prefs
        .hud
        .groups
        .iter()
        .find(|group| group.children.contains(instance_id))?;
    let group_position = layout
        .positions
        .get(&format!("group/{}", group.id))
        .copied()
        .unwrap_or(item.initial_position);
    let group_size = layout
        .transient_group_sizes
        .get(&group.id)
        .copied()
        .or(item.container_size)
        .unwrap_or_else(|| egui::vec2(group.layout.width, group.layout.height));
    let [top, right, bottom, left] = effective_group_padding(group, group_size);
    let member_size = egui::vec2(
        item.base_size.width * slot.width,
        item.base_size.height * slot.height,
    );
    Some(grouped_member_position_geometry_from_parts(
        group_position,
        group_size,
        [top, right, bottom, left],
        member_size,
        egui::pos2(slot.x, slot.y),
    ))
}

fn grouped_member_position_geometry_from_parts(
    group_position: egui::Pos2,
    group_size: egui::Vec2,
    [top, right, bottom, left]: [f32; 4],
    member_size: egui::Vec2,
    local_position: egui::Pos2,
) -> GroupedMemberPositionGeometry {
    let minimum = group_position + egui::vec2(left, top);
    let maximum = egui::pos2(
        (group_position.x + group_size.x - right - member_size.x).max(minimum.x),
        (group_position.y + group_size.y - bottom - member_size.y).max(minimum.y),
    );
    GroupedMemberPositionGeometry {
        position: minimum + local_position.to_vec2(),
        minimum,
        maximum,
    }
}

fn draw_position_editor(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    locale: deskhud_ui::Locale,
    target: PositionTarget<'_>,
) -> bool {
    let mut changed = false;
    components::config_card_with_header(
        ui,
        |ui| {
            ui.label(
                egui::RichText::new(deskhud_ui::i18n::t(locale, MessageKey::HudAdjustPosition))
                    .strong(),
            );
        },
        |ui| {
            let activity = layout.activity_size.unwrap_or(egui::vec2(1.0, 1.0));
            let (x, y, minimum, maximum, snap) = match target {
                PositionTarget::Slot { slot, size } => (
                    &mut slot.x,
                    &mut slot.y,
                    egui::pos2(HUD_PADDING, HUD_PADDING),
                    top_level_position_max(activity, size),
                    true,
                ),
                PositionTarget::Member {
                    x,
                    y,
                    minimum,
                    maximum,
                } => (x, y, minimum, maximum, false),
            };
            for (index, (label, value, pixels)) in [
                (MessageKey::HudAdjustX, x, activity.x),
                (MessageKey::HudAdjustY, y, activity.y),
            ]
            .into_iter()
            .enumerate()
            {
                let mut shown = *value;
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
                    let min = if index == 0 { minimum.x } else { minimum.y };
                    let max = if index == 0 { maximum.x } else { maximum.y };
                    *value = shown.clamp(min, max);
                    if snap && layout.snap_to_grid {
                        *value = snap_coordinate(*value, pixels).clamp(min, max);
                    }
                    changed = true;
                }
            }
        },
    );
    changed
}

fn draw_group_size_group(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    group_id: &str,
    item: &HudRenderItem,
    slot: &deskhud_ui::HudSlotLayout,
) -> bool {
    let content_min_size = group_adjustment_min_size(prefs, group_id, item);
    let screen_max_size = layout
        .activity_size
        .map_or(egui::Vec2::splat(f32::MAX), |activity| {
            egui::vec2(
                activity.x - HUD_PADDING - slot.x,
                activity.y - HUD_PADDING - slot.y,
            )
        });
    let Some(group) = prefs
        .hud
        .groups
        .iter_mut()
        .find(|group| group.id == group_id)
    else {
        return false;
    };
    let mut changed = false;
    if group.layout.width <= 0.0 || group.layout.height <= 0.0 {
        group.layout.width = (item.base_size.width * slot.width.max(0.5))
            .max(item.base_size.width)
            .max(content_min_size.x);
        group.layout.height = (item.base_size.height * slot.height.max(0.5))
            .max(item.base_size.height)
            .max(content_min_size.y);
        changed = true;
    } else {
        let width = group.layout.width.max(content_min_size.x);
        let height = group.layout.height.max(content_min_size.y);
        changed |= width != group.layout.width || height != group.layout.height;
        group.layout.width = width;
        group.layout.height = height;
    }
    let ratio = group.layout.height / group.layout.width.max(1.0);
    let (minimum, maximum) = group_adjustment_size_limits(
        content_min_size,
        screen_max_size,
        layout.lock_ratio.then_some(ratio),
    );
    let old_size = egui::vec2(group.layout.width, group.layout.height);
    if layout.lock_ratio {
        group.layout.width = group.layout.width.clamp(minimum.x, maximum.x);
        group.layout.height = group.layout.width * ratio;
    } else {
        group.layout.width = group.layout.width.clamp(minimum.x, maximum.x);
        group.layout.height = group.layout.height.clamp(minimum.y, maximum.y);
    }
    changed |= old_size != egui::vec2(group.layout.width, group.layout.height);
    components::config_card_with_header(
        ui,
        |ui| {
            ui.label(
                egui::RichText::new(deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustSize))
                    .strong(),
            );
        },
        |ui| {
            for (index, label) in [MessageKey::HudAdjustWidth, MessageKey::HudAdjustHeight]
                .into_iter()
                .enumerate()
            {
                let mut value = if index == 0 {
                    group.layout.width
                } else {
                    group.layout.height
                };
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
                                        .range(
                                            if index == 0 { minimum.x } else { minimum.y }
                                                ..=if index == 0 { maximum.x } else { maximum.y },
                                        )
                                        .suffix(" px"),
                                )
                                .changed()
                            })
                            .inner;
                    },
                );
                if value_changed {
                    value = value.clamp(
                        if index == 0 { minimum.x } else { minimum.y },
                        if index == 0 { maximum.x } else { maximum.y },
                    );
                    if layout.lock_ratio {
                        if index == 0 {
                            group.layout.width = value;
                            group.layout.height = value * ratio;
                        } else {
                            group.layout.height = value;
                            group.layout.width = value / ratio.max(0.001);
                        }
                    } else {
                        if index == 0 {
                            group.layout.width = value;
                        } else {
                            group.layout.height = value;
                        }
                    }
                    let horizontal_limit = group.layout.width * 0.25;
                    let vertical_limit = group.layout.height * 0.25;
                    group.inner.padding[0] = group.inner.padding[0].min(vertical_limit).floor();
                    group.inner.padding[2] = group.inner.padding[2].min(vertical_limit).floor();
                    group.inner.padding[1] = group.inner.padding[1].min(horizontal_limit).floor();
                    group.inner.padding[3] = group.inner.padding[3].min(horizontal_limit).floor();
                    changed = true;
                }
            }
        },
    );
    changed
}

fn top_level_position_max(activity: egui::Vec2, size: egui::Vec2) -> egui::Pos2 {
    egui::pos2(
        (activity.x - size.x - HUD_PADDING).max(HUD_PADDING),
        (activity.y - size.y - HUD_PADDING).max(HUD_PADDING),
    )
}

fn group_adjustment_size_limits(
    minimum: egui::Vec2,
    maximum: egui::Vec2,
    aspect_ratio: Option<f32>,
) -> (egui::Vec2, egui::Vec2) {
    let maximum = maximum.max(minimum);
    let Some(ratio) = aspect_ratio.filter(|ratio| ratio.is_finite() && *ratio > 0.0) else {
        return (minimum, maximum);
    };
    let min_width = minimum.x.max(minimum.y / ratio);
    let max_width = maximum.x.min(maximum.y / ratio).max(min_width);
    (
        egui::vec2(min_width, min_width * ratio),
        egui::vec2(max_width, max_width * ratio),
    )
}

fn group_adjustment_min_size(
    prefs: &UiPreferences,
    group_id: &str,
    item: &HudRenderItem,
) -> egui::Vec2 {
    let Some(group) = prefs.hud.groups.iter().find(|group| group.id == group_id) else {
        return group_min_size();
    };
    let [top, right, bottom, left] = group.inner.padding;
    let mut minimum = group_min_size();
    for layer in &item.layers {
        let Some(instance) = prefs
            .hud
            .instances
            .iter()
            .find(|instance| instance.id == layer.instance_id)
        else {
            continue;
        };
        let width = layer.base_size.width
            * instance
                .layout
                .width
                .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
        let height = layer.base_size.height
            * instance
                .layout
                .height
                .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
        minimum = include_group_member_in_minimum(
            minimum,
            [top, right, bottom, left],
            egui::pos2(instance.layout.x.max(0.0), instance.layout.y.max(0.0)),
            egui::vec2(width, height),
        );
    }
    minimum
}

fn include_group_member_in_minimum(
    minimum: egui::Vec2,
    [top, right, bottom, left]: [f32; 4],
    position: egui::Pos2,
    size: egui::Vec2,
) -> egui::Vec2 {
    egui::vec2(
        minimum.x.max(left + position.x + size.x + right),
        minimum.y.max(top + position.y + size.y + bottom),
    )
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
    let mut width_changed = false;
    let mut height_changed = false;
    let base = egui::vec2(item.base_size.width, item.base_size.height);
    let limits = hud_adjustment_size_factor_limits(layout, prefs, slot, item);
    components::config_card_with_header(
        ui,
        |ui| {
            ui.label(
                egui::RichText::new(deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustSize))
                    .strong(),
            );
        },
        |ui| {
            let width_max = if layout.lock_ratio {
                limits
                    .x
                    .min(limits.y / ratio.max(0.001))
                    .max(HUD_SIZE_FACTOR_MIN)
            } else {
                limits.x
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
            let height_max = if layout.lock_ratio {
                limits.y.min(limits.x * ratio).max(HUD_SIZE_FACTOR_MIN)
            } else {
                limits.y
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
    (changed, width_changed, height_changed)
}

fn hud_adjustment_size_factor_limits(
    layout: &LayoutState,
    prefs: &UiPreferences,
    slot: &deskhud_ui::HudSlotLayout,
    item: &HudRenderItem,
) -> egui::Vec2 {
    let base = egui::vec2(item.base_size.width, item.base_size.height);
    let bottom_right = if let HudLayoutTarget::Instance(instance_id) = &item.target
        && let Some(group) = prefs
            .hud
            .groups
            .iter()
            .find(|group| group.children.contains(instance_id))
    {
        let group_size = layout
            .transient_group_sizes
            .get(&group.id)
            .copied()
            .or(item.container_size)
            .unwrap_or_else(|| egui::vec2(group.layout.width, group.layout.height));
        let [top, right, bottom, left] = effective_group_padding(group, group_size);
        egui::vec2(
            (group_size.x - left - right).max(0.0),
            (group_size.y - top - bottom).max(0.0),
        )
    } else if let Some(activity) = layout.activity_size {
        activity - egui::Vec2::splat(HUD_PADDING)
    } else {
        egui::vec2(
            slot.x + base.x * HUD_SIZE_FACTOR_MAX,
            slot.y + base.y * HUD_SIZE_FACTOR_MAX,
        )
    };
    remaining_size_factor_limits(base, egui::pos2(slot.x, slot.y), bottom_right)
}

fn remaining_size_factor_limits(
    base: egui::Vec2,
    position: egui::Pos2,
    bottom_right: egui::Vec2,
) -> egui::Vec2 {
    egui::vec2(
        ((bottom_right.x - position.x) / base.x.max(1.0))
            .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
        ((bottom_right.y - position.y) / base.y.max(1.0))
            .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_adjustment_minimum_includes_member_position_size_and_padding() {
        let minimum = include_group_member_in_minimum(
            group_min_size(),
            [8.0, 12.0, 10.0, 6.0],
            egui::pos2(175.0, 90.0),
            egui::vec2(80.0, 40.0),
        );
        assert_eq!(minimum, egui::vec2(273.0, 148.0));
    }

    #[test]
    fn hud_adjustment_limits_size_at_the_available_bottom_right_edges() {
        let limits = remaining_size_factor_limits(
            egui::vec2(100.0, 50.0),
            egui::pos2(220.0, 160.0),
            egui::vec2(500.0, 300.0),
        );
        assert_eq!(limits, egui::vec2(2.8, 2.8));
    }

    #[test]
    fn group_adjustment_position_and_locked_size_stop_at_screen_edges() {
        assert_eq!(
            top_level_position_max(egui::vec2(800.0, 600.0), egui::vec2(240.0, 140.0)),
            egui::pos2(552.0, 452.0),
        );
        let (minimum, maximum) = group_adjustment_size_limits(
            egui::vec2(120.0, 80.0),
            egui::vec2(300.0, 160.0),
            Some(0.75),
        );
        assert_eq!(minimum, egui::vec2(120.0, 90.0));
        assert_eq!(maximum, egui::vec2(160.0 / 0.75, 160.0));
    }

    #[test]
    fn grouped_member_editor_exposes_screen_position_but_keeps_local_bounds() {
        let geometry = grouped_member_position_geometry_from_parts(
            egui::pos2(100.0, 100.0),
            egui::vec2(300.0, 200.0),
            [10.0, 20.0, 30.0, 40.0],
            egui::vec2(80.0, 50.0),
            egui::pos2(25.0, 15.0),
        );
        assert_eq!(geometry.position, egui::pos2(165.0, 125.0));
        assert_eq!(geometry.minimum, egui::pos2(140.0, 110.0));
        assert_eq!(geometry.maximum, egui::pos2(300.0, 220.0));
        assert_eq!(geometry.position - geometry.minimum, egui::vec2(25.0, 15.0));
    }
}
