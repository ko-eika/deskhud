//! HUD/plugin settings page.

use super::hud_list;
use super::{catalog_text, text};
use crate::components;
use deskhud_engine::{EngineRegistry, HudConfigKind, HudContribution, HudSourceId};
use deskhud_ui::{CatalogStore, HudConfigValue, LayerPreference, MessageKey, SettingsModel};
use egui::{CornerRadius, Frame, Margin, RichText, Sense, Stroke, Ui, Vec2};
use std::sync::Arc;

pub(super) fn draw(
    ui: &mut Ui,
    registry: &Arc<EngineRegistry>,
    catalogs: &CatalogStore,
    model: &mut SettingsModel,
) {
    let plugins = registry.plugin_infos();
    let initial_master_enabled = model.draft.hud.is_master_enabled();
    let mut master_enabled = initial_master_enabled;
    components::config_card(
        ui,
        Some(
            RichText::new(text(model, MessageKey::HudGlobalConfig))
                .strong()
                .into(),
        ),
        |ui| {
            components::switch_row_with_divider(
                ui,
                RichText::new(text(model, MessageKey::HudMasterEnable)).strong(),
                Some(
                    RichText::new(if master_enabled {
                        text(model, MessageKey::HudMasterEnableHint)
                    } else {
                        text(model, MessageKey::HudMasterDisabledHint)
                    })
                    .small(),
                ),
                &mut master_enabled,
                true,
            );
            components::config_row_with_divider(
                ui,
                text(model, MessageKey::MenuPluginLayer),
                None::<RichText>,
                false,
                |ui| {
                    let options = [
                        (
                            "top".to_owned(),
                            text(model, MessageKey::MenuLayerTop).to_owned(),
                        ),
                        (
                            "normal".to_owned(),
                            text(model, MessageKey::MenuLayerNormal).to_owned(),
                        ),
                        (
                            "bottom".to_owned(),
                            text(model, MessageKey::MenuLayerBottom).to_owned(),
                        ),
                    ];
                    let selected = match model.draft.hud.layer {
                        LayerPreference::Top => "top",
                        LayerPreference::Normal => "normal",
                        LayerPreference::Bottom => "bottom",
                    };
                    if let Some(value) =
                        components::dropdown(ui, "settings_hud_layer", selected, &options, false)
                    {
                        model.draft.hud.layer = match value.as_str() {
                            "bottom" => LayerPreference::Bottom,
                            "normal" => LayerPreference::Normal,
                            _ => LayerPreference::Top,
                        };
                    }
                },
            );
        },
        None,
    );
    if master_enabled != initial_master_enabled {
        model.draft.hud.set_master_enabled(master_enabled);
    }
    ui.add_space(16.0);
    if plugins.is_empty() {
        components::section_card(ui, |ui| {
            ui.label(text(model, MessageKey::HudSettingsEmpty));
        });
        return;
    }
    let selection_id = egui::Id::new("settings.hud.selected_plugin");
    let selected_id = ui.ctx().data(|data| data.get_temp::<String>(selection_id));
    let mut selected_id = selected_id
        .filter(|id| plugins.iter().any(|plugin| plugin.id == id))
        .unwrap_or_else(|| plugins[0].id.to_owned());
    ui.ctx()
        .data_mut(|data| data.insert_temp(selection_id, selected_id.clone()));
    components::section_card(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new(text(model, MessageKey::HudPluginList)).strong());
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(12.0);
        Frame::NONE
            .fill(ui.visuals().widgets.inactive.bg_fill)
            .stroke(Stroke::new(
                1.0,
                ui.visuals()
                    .widgets
                    .noninteractive
                    .bg_stroke
                    .color
                    .gamma_multiply(if ui.visuals().dark_mode { 0.62 } else { 0.82 }),
            ))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let gap = 12.0;
                let card_width = ((ui.available_width() - gap * 2.0) / 3.0).max(1.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(gap);
                    for plugin in &plugins {
                        let initial_enabled = model.draft.hud.is_plugin_enabled(plugin.id);
                        let mut enabled = initial_enabled;
                        let selected = hud_list::draw_plugin_list_card(
                            ui,
                            catalogs,
                            model,
                            plugin,
                            &selected_id,
                            card_width,
                            &mut enabled,
                        );
                        if selected {
                            selected_id = plugin.id.to_owned();
                            ui.ctx().data_mut(|data| {
                                data.insert_temp(selection_id, selected_id.clone())
                            });
                        }
                        if enabled != initial_enabled {
                            model.draft.hud.set_plugin_enabled(plugin.id, enabled);
                        }
                    }
                });
            });
    });
    let Some(plugin) = plugins.iter().find(|plugin| plugin.id == selected_id) else {
        return;
    };
    let contributions: Vec<_> = registry
        .all_hud_contributions()
        .into_iter()
        .filter(|(id, _)| *id == plugin.id)
        .map(|(_, contribution)| contribution)
        .collect();
    ui.add_space(16.0);
    let plugin_name = catalog_text(
        catalogs,
        model.draft.locale,
        plugin.id,
        "display_name",
        plugin.display_name,
    );
    let plugin_config_title = format!(
        "{} · {}",
        plugin_name,
        text(model, MessageKey::HudPluginConfig)
    );
    let empty_contributions_label = text(model, MessageKey::HudSettingsEmpty).to_owned();
    let rows_enabled = model.draft.hud.is_plugin_enabled(plugin.id);
    ui.label(RichText::new(&plugin_config_title).strong().size(16.0));
    ui.add_space(8.0);
    ui.add_enabled_ui(rows_enabled, |ui| {
        if contributions.is_empty() {
            components::section_card(ui, |ui| {
                ui.label(&empty_contributions_label);
            });
        } else {
            for contribution in &contributions {
                components::section_card(ui, |ui| {
                    let initial_item_enabled = model.draft.hud.is_enabled(
                        plugin.id,
                        contribution.id,
                        contribution.default_enabled,
                    );
                    let mut item_enabled = initial_item_enabled;
                    let label = catalog_text(
                        catalogs,
                        model.draft.locale,
                        plugin.id,
                        &format!("{}.label", contribution.id),
                        contribution.label,
                    );
                    let description = catalog_text(
                        catalogs,
                        model.draft.locale,
                        plugin.id,
                        &format!("{}.description", contribution.id),
                        "",
                    );
                    let icon = hud_list::hud_contribution_icon_texture(ui, plugin.id, contribution);
                    components::config_row_with_icon(
                        ui,
                        icon.as_ref(),
                        RichText::new(label).strong(),
                        (!description.is_empty()).then_some(RichText::new(description).small()),
                        |ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(42.0, 24.0), Sense::hover());
                            components::toggle_switch(ui, rect, &mut item_enabled);
                        },
                    );
                    if item_enabled != initial_item_enabled {
                        model
                            .draft
                            .hud
                            .set_enabled(plugin.id, contribution.id, item_enabled);
                    }
                    if item_enabled && !contribution.config.is_empty() {
                        ui.add_space(2.0);
                        ui.separator();
                        ui.add_space(2.0);
                        ui.scope(|ui| {
                            ui.spacing_mut().item_spacing.y = 10.0;
                            draw_contribution_config(
                                ui,
                                registry,
                                catalogs,
                                model,
                                plugin.id,
                                contribution,
                            );
                        });
                    }
                });
                ui.add_space(10.0);
            }
        }
    });
}

fn draw_contribution_config(
    ui: &mut Ui,
    registry: &EngineRegistry,
    catalogs: &CatalogStore,
    model: &mut SettingsModel,
    plugin_id: &str,
    contribution: &HudContribution,
) {
    let source = HudSourceId::new(plugin_id, contribution.id);
    let instance_id = deskhud_ui::HudPrefs::default_instance_id(&source);
    let Some(instance_index) = model
        .draft
        .hud
        .instances
        .iter()
        .position(|instance| instance.id == instance_id && instance.source == source)
    else {
        return;
    };
    for option in contribution.config {
        let locale = model.draft.locale;
        let label = catalog_text(
            catalogs,
            locale,
            plugin_id,
            &format!("{}.{}.label", contribution.id, option.key),
            option.label,
        );
        let description = catalog_text(
            catalogs,
            locale,
            plugin_id,
            &format!("{}.{}.description", contribution.id, option.key),
            option.description,
        );
        components::config_row_with_divider(
            ui,
            RichText::new(label),
            (!description.is_empty()).then_some(RichText::new(description).small()),
            true,
            |ui| {
                let config = &mut model.draft.hud.instances[instance_index].config;
                match option.kind {
                    HudConfigKind::Bool { default } => {
                        let mut value = config
                            .get(option.key)
                            .and_then(HudConfigValue::as_bool)
                            .unwrap_or(default);
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(42.0, 24.0), Sense::hover());
                        if components::toggle_switch(ui, rect, &mut value).changed() {
                            config.insert(option.key.to_owned(), HudConfigValue::Bool(value));
                        }
                    }
                    HudConfigKind::Number {
                        default,
                        min,
                        max,
                        step,
                    } => {
                        let mut value = config
                            .get(option.key)
                            .and_then(HudConfigValue::as_f32)
                            .unwrap_or(default)
                            .clamp(min, max);
                        if ui
                            .add(
                                egui::DragValue::new(&mut value)
                                    .range(min..=max)
                                    .speed(step),
                            )
                            .changed()
                        {
                            config
                                .insert(option.key.to_owned(), HudConfigValue::Float(value as f64));
                        }
                    }
                    HudConfigKind::Text { default, max_len } => {
                        let mut value = config
                            .get(option.key)
                            .and_then(HudConfigValue::as_str)
                            .unwrap_or(default)
                            .to_owned();
                        if ui
                            .add(egui::TextEdit::singleline(&mut value).desired_width(220.0))
                            .changed()
                        {
                            value = value.chars().take(max_len).collect();
                            config.insert(option.key.to_owned(), HudConfigValue::String(value));
                        }
                    }
                    HudConfigKind::Choice { default, choices } => {
                        let selected = config
                            .get(option.key)
                            .and_then(HudConfigValue::as_str)
                            .filter(|value| choices.iter().any(|choice| choice.value == *value))
                            .unwrap_or(default);
                        let choices = choices
                            .iter()
                            .map(|choice| {
                                (
                                    choice.value.to_owned(),
                                    catalog_text(
                                        catalogs,
                                        model.draft.locale,
                                        plugin_id,
                                        &format!("unit.{}", choice.value),
                                        choice.label,
                                    ),
                                )
                            })
                            .collect::<Vec<_>>();
                        if let Some(value) = components::dropdown(
                            ui,
                            ("hud-config", plugin_id, contribution.id, option.key),
                            selected,
                            &choices,
                            false,
                        ) {
                            config.insert(option.key.to_owned(), HudConfigValue::String(value));
                        }
                    }
                    HudConfigKind::DynamicChoice { default } => {
                        let selected = config
                            .get(option.key)
                            .and_then(HudConfigValue::as_str)
                            .unwrap_or(default)
                            .to_owned();
                        let placeholder = catalog_text(
                            catalogs,
                            locale,
                            plugin_id,
                            &format!("{}.{}.placeholder", contribution.id, option.key),
                            option.label,
                        );
                        let mut choices = vec![(String::new(), placeholder)];
                        choices.extend(
                            registry
                                .hud_config_choices(plugin_id, contribution.id, option.key)
                                .into_iter()
                                .map(|choice| (choice.value, choice.label)),
                        );
                        if !selected.is_empty()
                            && !choices.iter().any(|(value, _)| value == &selected)
                        {
                            choices.push((selected.clone(), selected.clone()));
                        }
                        if let Some(value) = components::dropdown(
                            ui,
                            ("hud-dynamic-config", plugin_id, contribution.id, option.key),
                            &selected,
                            &choices,
                            true,
                        ) {
                            config.insert(option.key.to_owned(), HudConfigValue::String(value));
                        }
                    }
                }
            },
        );
    }
}
