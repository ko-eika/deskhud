//! Current pet configuration group.

use super::{catalog_text, draw_empty, text};
use crate::components;
use deskhud_engine::EngineRegistry;
use deskhud_ui::{CatalogStore, MessageKey, SettingsModel};
use egui::{RichText, Sense, Ui, Vec2};
use std::sync::Arc;

pub(super) fn draw(
    ui: &mut Ui,
    registry: &Arc<EngineRegistry>,
    catalogs: &CatalogStore,
    model: &mut SettingsModel,
) {
    let active_id = model.draft.pet.kind.clone();
    let Some(pet) = registry
        .pets()
        .into_iter()
        .find(|pet| pet.info().id == active_id)
    else {
        draw_empty(ui, model);
        return;
    };
    let options = pet.config_options();
    if options.is_empty() {
        return;
    }
    let pet_info = pet.info();
    let pet_name = catalog_text(
        catalogs,
        model.draft.locale,
        &active_id,
        "display_name",
        pet_info.display_name,
    );

    components::config_card(
        ui,
        Some(
            RichText::new(format!(
                "{} · {}",
                pet_name,
                text(model, MessageKey::SettingsPetConfig)
            ))
            .strong()
            .into(),
        ),
        |ui| {
            for (index, option) in options.iter().enumerate() {
                if index > 0 {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                }
                let mut enabled =
                    model
                        .draft
                        .pet
                        .get_option(&active_id, option.key, option.default);
                let label = catalog_text(
                    catalogs,
                    model.draft.locale,
                    &active_id,
                    &format!("{}.label", option.key),
                    option.label,
                );
                let description = catalog_text(
                    catalogs,
                    model.draft.locale,
                    &active_id,
                    &format!("{}.description", option.key),
                    option.description,
                );
                let mut changed = false;
                components::config_row(
                    ui,
                    RichText::new(label).strong(),
                    Some(
                        RichText::new(description)
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    ),
                    |ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(42.0, 24.0), Sense::hover());
                        changed = components::toggle_switch(ui, rect, &mut enabled).changed();
                    },
                );
                if changed {
                    model.draft.pet.set_option(&active_id, option.key, enabled);
                }
            }
        },
        None,
    );
}
