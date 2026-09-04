//! HUD/plugin settings page.

use super::hud_list;
use super::{catalog_text, text};
use crate::components;
use deskhud_engine::EngineRegistry;
use deskhud_ui::{CatalogStore, LayerPreference, MessageKey, SettingsModel};
use egui::{RichText, Sense, Ui, Vec2};
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
    let selected_id = ui
        .ctx()
        .data(|data| data.get_temp::<String>(ui.make_persistent_id("hud.selected_plugin")));
    let selected_id = selected_id
        .filter(|id| plugins.iter().any(|plugin| plugin.id == id))
        .unwrap_or_else(|| plugins[0].id.to_owned());
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            ui.make_persistent_id("hud.selected_plugin"),
            selected_id.clone(),
        )
    });
    components::section_card(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new(text(model, MessageKey::HudPluginList)).strong());
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
        let gap = 12.0;
        let card_width = ((ui.available_width() - gap * 2.0) / 3.0).max(1.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(gap);
            for plugin in &plugins {
                let initial_enabled = model.draft.hud.is_plugin_enabled(plugin.id);
                let mut enabled = initial_enabled;
                hud_list::draw_plugin_list_card(
                    ui,
                    catalogs,
                    model,
                    plugin,
                    &selected_id,
                    card_width,
                    &mut enabled,
                );
                if enabled != initial_enabled {
                    model.draft.hud.set_plugin_enabled(plugin.id, enabled);
                }
            }
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
    components::config_card(
        ui,
        Some(RichText::new(&plugin_config_title).strong().into()),
        |ui| {
            ui.add_enabled_ui(rows_enabled, |ui| {
                for (index, contribution) in contributions.iter().enumerate() {
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
                    components::config_row_with_icon_and_divider(
                        ui,
                        icon.as_ref(),
                        RichText::new(label).strong(),
                        (!description.is_empty()).then_some(RichText::new(description).small()),
                        index + 1 < contributions.len(),
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
                }
            });
            if contributions.is_empty() {
                ui.label(&empty_contributions_label);
            }
        },
        None,
    );
}
