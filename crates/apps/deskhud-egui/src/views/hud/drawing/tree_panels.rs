//! HUD layout-editor information and selection trees.

use std::collections::{HashMap, HashSet};

use deskhud_engine::{EngineRegistry, HudInstanceId, HudSourceId};
use deskhud_ui::{CatalogStore, MessageKey, UiPreferences};

use super::*;
use crate::components;

const INFORMATION_TREE: &str = "information-tree";
const ACTIVE_TREE: &str = "active-tree";

pub(super) fn draw(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    registry: &EngineRegistry,
    catalogs: &CatalogStore,
    prefs: &mut UiPreferences,
) -> bool {
    sync_panel_order(layout);
    let order = layout.tree_panel_order.clone();
    let mut changed = false;
    // Match the right-side adjustment column: draw bottom/later panels first
    // so the first-opened, topmost panel wins if the windows overlap.
    for panel in order.iter().rev() {
        match *panel {
            INFORMATION_TREE => {
                changed |= draw_information_tree(ui, layout, registry, catalogs, prefs);
            }
            ACTIVE_TREE => draw_active_tree(ui, layout, registry, catalogs, prefs),
            _ => {}
        }
    }
    sync_panel_order(layout);
    changed
}

fn draw_information_tree(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    registry: &EngineRegistry,
    catalogs: &CatalogStore,
    prefs: &mut UiPreferences,
) -> bool {
    let mut changed = false;
    let session = layout.adjust_session;
    let revision = layout.tree_window_revision;
    let panel_top = panel_top(layout, INFORMATION_TREE);
    let mut open = layout.information_tree_open;
    let window_frame = tree_window_frame(ui);
    egui::Window::new(
        egui::RichText::new(deskhud_ui::i18n::t(
            prefs.locale,
            MessageKey::HudLayoutInformationTree,
        ))
        .strong(),
    )
    .id(egui::Id::new((
        "hud-layout-information-tree",
        session,
        revision,
    )))
    .default_pos(egui::pos2(EDITOR_PANEL_LEFT_MARGIN, panel_top))
    .default_width(EDITOR_PANEL_WIDTH)
    .default_height(EDITOR_PANEL_HEIGHT)
    .min_width(EDITOR_PANEL_WIDTH)
    .max_width(EDITOR_PANEL_WIDTH)
    .min_height(320.0)
    .resizable([false, true])
    .movable(true)
    .collapsible(true)
    .open(&mut open)
    .frame(window_frame)
    .show(ui.ctx(), |ui| {
        draw_panel_summary(
            ui,
            "puzzle",
            prefs
                .hud
                .instances
                .iter()
                .filter(|instance| instance.enabled)
                .count(),
            prefs.hud.instances.len(),
        );
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for plugin in registry.plugin_infos() {
                    let contributions = registry
                        .all_hud_contributions()
                        .into_iter()
                        .filter(|(plugin_id, _)| *plugin_id == plugin.id)
                        .map(|(_, contribution)| contribution)
                        .collect::<Vec<_>>();
                    if contributions.is_empty() {
                        continue;
                    }
                    let plugin_name = plugin_name(catalogs, prefs, plugin.id, plugin.display_name);
                    let mut plugin_enabled = prefs.hud.is_plugin_enabled(plugin.id);
                    let enabled_count = contributions
                        .iter()
                        .filter(|contribution| {
                            prefs.hud.is_enabled(
                                plugin.id,
                                contribution.id,
                                contribution.default_enabled,
                            )
                        })
                        .count();
                    tree_card(ui, None, |ui| {
                        let header =
                            egui::collapsing_header::CollapsingState::load_with_default_open(
                                ui.ctx(),
                                ui.make_persistent_id(("hud-info-plugin", plugin.id)),
                                true,
                            )
                            .show_header(ui, |ui| {
                                paint_inline_icon(ui, "puzzle");
                                ui.label(egui::RichText::new(plugin_name).strong());
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        changed |= compact_toggle(
                                            ui,
                                            ("hud-info-plugin-toggle", plugin.id),
                                            &mut plugin_enabled,
                                        );
                                        count_badge(ui, enabled_count, contributions.len());
                                    },
                                );
                            });
                        header.body(|ui| {
                            ui.add_enabled_ui(plugin_enabled, |ui| {
                                for contribution in &contributions {
                                    let mut enabled = prefs.hud.is_enabled(
                                        plugin.id,
                                        contribution.id,
                                        contribution.default_enabled,
                                    );
                                    let label = contribution_name(
                                        catalogs,
                                        prefs,
                                        plugin.id,
                                        contribution.id,
                                        contribution.label,
                                    );
                                    tree_value_row(ui, enabled, |ui| {
                                        paint_inline_icon(ui, "window");
                                        ui.label(label);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if compact_toggle(
                                                    ui,
                                                    (
                                                        "hud-info-contribution-toggle",
                                                        plugin.id,
                                                        contribution.id,
                                                    ),
                                                    &mut enabled,
                                                ) {
                                                    prefs.hud.set_enabled(
                                                        plugin.id,
                                                        contribution.id,
                                                        enabled,
                                                    );
                                                    changed = true;
                                                }
                                            },
                                        );
                                    });
                                }
                            });
                        });
                    });
                    ui.add_space(8.0);
                    if plugin_enabled != prefs.hud.is_plugin_enabled(plugin.id) {
                        prefs.hud.set_plugin_enabled(plugin.id, plugin_enabled);
                        changed = true;
                    }
                }
            });
    });
    layout.information_tree_open = open;
    changed
}

fn draw_active_tree(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    registry: &EngineRegistry,
    catalogs: &CatalogStore,
    prefs: &UiPreferences,
) {
    let available: HashSet<_> = registry
        .all_hud_contributions()
        .into_iter()
        .map(|(plugin_id, contribution)| HudSourceId::new(plugin_id, contribution.id))
        .collect();
    let membership: HashMap<_, _> = prefs
        .hud
        .groups
        .iter()
        .flat_map(|group| {
            group
                .children
                .iter()
                .map(move |child| (child.clone(), group.id.as_str()))
        })
        .collect();
    let is_active = |id: &HudInstanceId| {
        prefs.hud.is_master_enabled()
            && prefs.hud.instances.iter().any(|instance| {
                &instance.id == id
                    && instance.enabled
                    && prefs.hud.is_plugin_enabled(&instance.source.plugin_id)
                    && available.contains(&instance.source)
            })
    };
    let ungrouped = prefs
        .hud
        .instances
        .iter()
        .filter(|instance| !membership.contains_key(&instance.id) && is_active(&instance.id))
        .collect::<Vec<_>>();
    let active_groups = prefs
        .hud
        .groups
        .iter()
        .filter(|group| prefs.hud.is_master_enabled() && group.enabled)
        .collect::<Vec<_>>();
    let active_count = ungrouped.len()
        + active_groups
            .iter()
            .map(|group| {
                group
                    .children
                    .iter()
                    .filter(|child| is_active(child))
                    .count()
            })
            .sum::<usize>();
    let session = layout.adjust_session;
    let revision = layout.tree_window_revision;
    let panel_top = panel_top(layout, ACTIVE_TREE);
    let mut open = layout.active_tree_open;
    let window_frame = tree_window_frame(ui);

    egui::Window::new(
        egui::RichText::new(deskhud_ui::i18n::t(
            prefs.locale,
            MessageKey::HudLayoutActiveTree,
        ))
        .strong(),
    )
    .id(egui::Id::new(("hud-layout-active-tree", session, revision)))
    .default_pos(egui::pos2(EDITOR_PANEL_LEFT_MARGIN, panel_top))
    .default_width(EDITOR_PANEL_WIDTH)
    .default_height(EDITOR_PANEL_HEIGHT)
    .min_width(EDITOR_PANEL_WIDTH)
    .max_width(EDITOR_PANEL_WIDTH)
    .min_height(320.0)
    .resizable([false, true])
    .movable(true)
    .collapsible(true)
    .open(&mut open)
    .frame(window_frame)
    .show(ui.ctx(), |ui| {
        draw_panel_summary(
            ui,
            "layers-subtract",
            active_count + active_groups.len(),
            prefs.hud.instances.len() + prefs.hud.groups.len(),
        );
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if ungrouped.is_empty() && active_groups.is_empty() {
                    ui.label(
                        egui::RichText::new(deskhud_ui::i18n::t(
                            prefs.locale,
                            MessageKey::HudLayoutActiveEmpty,
                        ))
                        .color(ui.visuals().weak_text_color()),
                    );
                    return;
                }
                if !ungrouped.is_empty() {
                    tree_card(ui, None, |ui| {
                        egui::CollapsingHeader::new(
                            egui::RichText::new(deskhud_ui::i18n::t(
                                prefs.locale,
                                MessageKey::HudLayoutUngrouped,
                            ))
                            .strong(),
                        )
                        .id_salt("hud-active-ungrouped")
                        .default_open(true)
                        .show(ui, |ui| {
                            for instance in &ungrouped {
                                draw_instance_row(ui, layout, registry, catalogs, prefs, instance);
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
                for group in active_groups {
                    let key = format!("group/{}", group.id);
                    let selected = layout.selected.as_deref() == Some(key.as_str());
                    let title = if group.name.is_empty() {
                        deskhud_ui::i18n::t(prefs.locale, MessageKey::HudGroupDefaultName)
                            .to_owned()
                    } else {
                        group.name.clone()
                    };
                    let group_color =
                        egui::Color32::from_rgb(group.color[0], group.color[1], group.color[2]);
                    let active_child_count = group
                        .children
                        .iter()
                        .filter(|child| is_active(child))
                        .count();
                    tree_card(ui, Some(group_color), |ui| {
                        let header =
                            egui::collapsing_header::CollapsingState::load_with_default_open(
                                ui.ctx(),
                                ui.make_persistent_id(("hud-active-group", &group.id)),
                                true,
                            )
                            .show_header(ui, |ui| {
                                color_dot(ui, group_color);
                                if ui
                                    .selectable_label(selected, egui::RichText::new(title).strong())
                                    .clicked()
                                {
                                    select(layout, key.clone());
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| count_badge(ui, active_child_count, group.children.len()),
                                );
                            });
                        header.body(|ui| {
                            for child in &group.children {
                                if !is_active(child) {
                                    continue;
                                }
                                if let Some(instance) = prefs
                                    .hud
                                    .instances
                                    .iter()
                                    .find(|instance| &instance.id == child)
                                {
                                    draw_instance_row(
                                        ui, layout, registry, catalogs, prefs, instance,
                                    );
                                }
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
            });
    });
    layout.active_tree_open = open;
}

fn sync_panel_order(layout: &mut LayoutState) {
    let previous = layout.tree_panel_order.clone();
    layout.tree_panel_order.retain(|panel| match *panel {
        INFORMATION_TREE => layout.information_tree_open,
        ACTIVE_TREE => layout.active_tree_open,
        _ => false,
    });
    if layout.information_tree_open && !layout.tree_panel_order.contains(&INFORMATION_TREE) {
        layout.tree_panel_order.push(INFORMATION_TREE);
    }
    if layout.active_tree_open && !layout.tree_panel_order.contains(&ACTIVE_TREE) {
        layout.tree_panel_order.push(ACTIVE_TREE);
    }
    if layout.tree_panel_order != previous {
        layout.tree_window_revision = layout.tree_window_revision.wrapping_add(1);
    }
}

fn panel_top(layout: &LayoutState, panel: &str) -> f32 {
    let max_height = layout
        .activity_size
        .map(|size| (size.y - 64.0).max(360.0))
        .unwrap_or(720.0);
    let mut top = EDITOR_PANEL_TOP;
    for current in &layout.tree_panel_order {
        if *current == panel {
            break;
        }
        top += EDITOR_PANEL_HEIGHT.min(max_height) + EDITOR_PANEL_GAP;
    }
    top
}

fn draw_instance_row(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    registry: &EngineRegistry,
    catalogs: &CatalogStore,
    prefs: &UiPreferences,
    instance: &deskhud_ui::HudInstance,
) {
    let fallback = registry
        .all_hud_contributions()
        .into_iter()
        .find(|(plugin_id, contribution)| {
            *plugin_id == instance.source.plugin_id
                && contribution.id == instance.source.contribution_id
        })
        .map_or(instance.source.contribution_id.as_str(), |(_, item)| {
            item.label
        });
    let label = contribution_name(
        catalogs,
        prefs,
        &instance.source.plugin_id,
        &instance.source.contribution_id,
        fallback,
    );
    let key = format!("instance/{}", instance.id.as_str());
    let selected = layout.selected.as_deref() == Some(key.as_str());
    if tree_selectable_row(ui, selected, &label)
        .on_hover_text(format!(
            "{}\n{}",
            instance.source.plugin_id,
            instance.id.as_str()
        ))
        .clicked()
    {
        select(layout, key);
    }
}

fn tree_window_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::window(ui.style())
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(12))
}

fn draw_panel_summary(ui: &mut egui::Ui, icon: &'static str, current: usize, total: usize) {
    egui::Frame::NONE
        .fill(ui.visuals().selection.bg_fill.gamma_multiply(0.10))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                paint_inline_icon(ui, icon);
                ui.label(egui::RichText::new(format!("{current} / {total}")).strong());
            });
        });
}

fn tree_card(ui: &mut egui::Ui, accent: Option<egui::Color32>, add: impl FnOnce(&mut egui::Ui)) {
    let border = ui
        .visuals()
        .widgets
        .noninteractive
        .bg_stroke
        .color
        .gamma_multiply(if ui.visuals().dark_mode { 0.64 } else { 0.88 });
    let response = egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(egui::CornerRadius::same(9))
        .inner_margin(egui::Margin::symmetric(9, 7))
        .show(ui, add);
    if let Some(color) = accent {
        ui.painter().line_segment(
            [
                response.response.rect.left_top() + egui::vec2(2.0, 8.0),
                response.response.rect.left_bottom() + egui::vec2(2.0, -8.0),
            ],
            egui::Stroke::new(3.0, color),
        );
    }
}

fn tree_value_row(ui: &mut egui::Ui, enabled: bool, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::NONE
        .fill(if enabled {
            ui.visuals().selection.bg_fill.gamma_multiply(0.06)
        } else {
            egui::Color32::TRANSPARENT
        })
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(7, 5))
        .show(ui, |ui| {
            ui.horizontal(add);
        });
}

fn tree_selectable_row(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let height = 30.0_f32.max(ui.text_style_height(&egui::TextStyle::Body) + 10.0);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click(),
    );
    let fill = if selected {
        ui.visuals().selection.bg_fill.gamma_multiply(0.24)
    } else if response.hovered() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    let icon_rect = egui::Rect::from_center_size(
        rect.left_center() + egui::vec2(12.0, 0.0),
        egui::Vec2::splat(16.0),
    );
    components::icons::paint(ui, "window", icon_rect, ui.visuals().text_color(), false);
    ui.painter().text(
        rect.left_center() + egui::vec2(26.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::TextStyle::Body.resolve(ui.style()),
        ui.visuals().text_color(),
    );
    response
}

fn paint_inline_icon(ui: &mut egui::Ui, name: &'static str) {
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(17.0), egui::Sense::hover());
    components::icons::paint(ui, name, rect, ui.visuals().text_color(), false);
}

fn color_dot(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(14.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
}

fn count_badge(ui: &mut egui::Ui, current: usize, total: usize) {
    egui::Frame::NONE
        .fill(ui.visuals().widgets.inactive.bg_fill)
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(format!("{current}/{total}"))
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });
}

fn select(layout: &mut LayoutState, key: String) {
    layout.selected = Some(key);
    layout.adjust_open = true;
}

fn compact_toggle(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    value: &mut bool,
) -> bool {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(42.0, 24.0), egui::Sense::hover());
    components::toggle_switch_with_id(ui, rect, value, id).changed()
}

fn plugin_name(
    catalogs: &CatalogStore,
    prefs: &UiPreferences,
    plugin_id: &str,
    fallback: &str,
) -> String {
    catalogs
        .t(prefs.locale, &format!("{plugin_id}.display_name"), fallback)
        .to_owned()
}

fn contribution_name(
    catalogs: &CatalogStore,
    prefs: &UiPreferences,
    plugin_id: &str,
    contribution_id: &str,
    fallback: &str,
) -> String {
    catalogs
        .t(
            prefs.locale,
            &format!("{plugin_id}.{contribution_id}.label"),
            fallback,
        )
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_tree_panels_stack_from_the_shared_top_edge() {
        let mut layout = LayoutState {
            information_tree_open: true,
            active_tree_open: true,
            ..LayoutState::default()
        };
        sync_panel_order(&mut layout);

        assert_eq!(layout.tree_panel_order, [INFORMATION_TREE, ACTIVE_TREE]);
        assert_eq!(panel_top(&layout, INFORMATION_TREE), EDITOR_PANEL_TOP);
        assert_eq!(
            panel_top(&layout, ACTIVE_TREE),
            EDITOR_PANEL_TOP + EDITOR_PANEL_HEIGHT + EDITOR_PANEL_GAP
        );
    }

    #[test]
    fn closing_reflows_and_reopening_appends_to_the_column() {
        let mut layout = LayoutState {
            information_tree_open: true,
            active_tree_open: true,
            tree_panel_order: vec![INFORMATION_TREE, ACTIVE_TREE],
            ..LayoutState::default()
        };
        let initial_revision = layout.tree_window_revision;

        layout.information_tree_open = false;
        sync_panel_order(&mut layout);
        assert_eq!(layout.tree_panel_order, [ACTIVE_TREE]);
        assert_eq!(panel_top(&layout, ACTIVE_TREE), EDITOR_PANEL_TOP);
        assert_ne!(layout.tree_window_revision, initial_revision);

        layout.information_tree_open = true;
        sync_panel_order(&mut layout);
        assert_eq!(layout.tree_panel_order, [ACTIVE_TREE, INFORMATION_TREE]);
        assert_eq!(
            panel_top(&layout, INFORMATION_TREE),
            EDITOR_PANEL_TOP + EDITOR_PANEL_HEIGHT + EDITOR_PANEL_GAP
        );
    }
}
