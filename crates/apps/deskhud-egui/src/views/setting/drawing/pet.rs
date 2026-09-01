//! Pet settings page.

use super::pet_config;
use super::pet_global;
use super::pet_picker;
use super::{text, text_for_locale, tooltip_meta_row, truncate_ui_text};
use crate::{components, fonts};
use std::sync::Arc;

use deskhud_engine::EngineRegistry;
use deskhud_ui::{CatalogStore, Locale, MessageKey, SettingsModel};
use egui::{Align, Layout, RichText, TextureOptions, Ui, Vec2};

pub(super) fn draw(
    ui: &mut Ui,
    registry: &Arc<EngineRegistry>,
    catalogs: &CatalogStore,
    model: &mut SettingsModel,
) {
    components::config_card(
        ui,
        Some(
            RichText::new(text(model, MessageKey::SettingsPetGlobal))
                .strong()
                .into(),
        ),
        |ui| pet_global::draw(ui, model),
        None,
    );
    ui.add_space(16.0);

    let infos = registry.pet_infos();
    let mode = model.draft.pet.picker_mode;
    components::section_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(text(model, MessageKey::SettingsPetList)).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                pet_global::draw_view_modes(ui, model)
            });
        });
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(12.0);
        match mode {
            deskhud_ui::PetPickerMode::Grid => {
                let card_layout = deskhud_ui::pet_card_layout_with_font(
                    ui.available_width(),
                    fonts::base_size(ui),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);
                    for info in &infos {
                        pet_picker::draw_pet_grid_card(ui, catalogs, model, info, card_layout);
                    }
                });
            }
            deskhud_ui::PetPickerMode::List => {
                for (index, info) in infos.iter().enumerate() {
                    pet_picker::draw_pet_list_row(ui, catalogs, model, info);
                    if index + 1 < infos.len() {
                        ui.add_space(8.0);
                    }
                }
            }
        }
    });
    ui.add_space(16.0);

    pet_config::draw(ui, registry, catalogs, model);
}

pub(super) fn pet_preview_texture(
    ui: &Ui,
    info: &deskhud_engine::PetKindInfo,
) -> Option<egui::TextureHandle> {
    let bytes = info.preview?;
    let cache_id = ui.make_persistent_id(("pet-preview", info.id));
    if let Some(texture) = ui.ctx().data(|data| data.get_temp(cache_id)) {
        return Some(texture);
    }
    let image = crate::image_decode::decode(bytes, 1536)?;
    let texture = ui.ctx().load_texture(
        format!("pet-preview-{}", info.id),
        image,
        TextureOptions::LINEAR,
    );
    ui.ctx()
        .data_mut(|data| data.insert_temp(cache_id, texture.clone()));
    Some(texture)
}

pub(super) fn pet_preview_rect(container: egui::Rect) -> egui::Rect {
    // Keep the preview window identical for every card. The pet artwork is
    // fitted inside it using its own standard window ratio below.
    container
}

pub(super) fn pet_tooltip(
    response: egui::Response,
    info: &deskhud_engine::PetKindInfo,
    name: &str,
    description: &str,
    locale: Locale,
) -> egui::Response {
    let name = name.to_owned();
    let description = description.to_owned();
    response.on_hover_ui(|ui| {
        let base = fonts::base_size(ui);
        let scale = (base / 14.0).clamp(0.8, 1.35);
        let max_width = 320.0 * scale;
        ui.set_min_width(250.0 * scale);
        ui.set_max_width(max_width);
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = fonts::scaled_size(ui, 0.24);
            ui.spacing_mut().item_spacing.x = fonts::scaled_size(ui, 0.55);
            ui.label(
                RichText::new(&name)
                    .font(fonts::scaled_font(ui, 1.15))
                    .strong(),
            );
            ui.add_space(fonts::scaled_size(ui, 0.25));
            ui.label(
                RichText::new(&description)
                    .font(fonts::scaled_font(ui, 0.9))
                    .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(fonts::scaled_size(ui, 0.55));
            ui.separator();
            ui.add_space(fonts::scaled_size(ui, 0.4));
            ui.label(RichText::new(info.id).font(fonts::scaled_font(ui, 0.86)));
            tooltip_meta_row(ui, locale, MessageKey::MetaAuthor, info.author);
            tooltip_meta_row(ui, locale, MessageKey::MetaVersion, info.version);
            tooltip_meta_row(ui, locale, MessageKey::MetaEngine, info.engine);
            tooltip_meta_row(
                ui,
                locale,
                MessageKey::SettingsPetWindowSize,
                &format!("{:.0}×{:.0}", info.window_width, info.window_height),
            );
            if let Some(homepage) = info.homepage {
                ui.add_space(fonts::scaled_size(ui, 0.15));
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(text_for_locale(locale, MessageKey::MetaHomepage))
                            .font(fonts::scaled_font(ui, 0.82))
                            .color(ui.visuals().weak_text_color()),
                    );
                    let link_font = fonts::scaled_font(ui, 0.86);
                    let link = truncate_ui_text(
                        ui,
                        homepage,
                        link_font.clone(),
                        (max_width - fonts::scaled_size(ui, 5.0)).max(80.0),
                    );
                    ui.add(egui::Hyperlink::from_label_and_url(
                        RichText::new(link).font(link_font),
                        homepage,
                    ))
                    .on_hover_text(homepage);
                });
            }
        });
    })
}
