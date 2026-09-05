//! Plugin list cards and metadata previews.

use super::{catalog_text, paint_preview_contain, tooltip_meta_row, truncate_ui_text};
use crate::{components, fonts};
use deskhud_ui::{CatalogStore, Locale, MessageKey, SettingsModel};
use egui::{RichText, Sense, Stroke, Ui, Vec2};

fn plugin_tooltip(
    response: egui::Response,
    info: &deskhud_engine::PluginInfo,
    name: &str,
    description: &str,
    locale: Locale,
) -> egui::Response {
    let name = name.to_owned();
    let description = description.to_owned();
    response.on_hover_ui(|ui| {
        ui.set_min_width(250.0);
        ui.set_max_width(320.0);
        ui.label(
            RichText::new(&name)
                .font(fonts::scaled_font(ui, 1.15))
                .strong(),
        );
        ui.add_space(4.0);
        ui.label(RichText::new(&description).color(ui.visuals().weak_text_color()));
        ui.add_space(8.0);
        ui.separator();
        ui.label(info.id);
        tooltip_meta_row(ui, locale, MessageKey::MetaAuthor, info.author);
        tooltip_meta_row(ui, locale, MessageKey::MetaVersion, info.version);
        tooltip_meta_row(ui, locale, MessageKey::MetaEngine, info.engine);
        if let Some(homepage) = info.homepage {
            ui.add(egui::Hyperlink::from_label_and_url(homepage, homepage))
                .on_hover_text(homepage);
        }
    })
}

fn plugin_icon_texture(ui: &Ui, info: &deskhud_engine::PluginInfo) -> Option<egui::TextureHandle> {
    let bytes = info.icon?;
    let cache_id = ui.make_persistent_id(("plugin-icon", info.id));
    if let Some(texture) = ui.ctx().data(|data| data.get_temp(cache_id)) {
        return Some(texture);
    }
    let image = crate::image_decode::decode(bytes, 1536)?;
    let texture = ui.ctx().load_texture(
        format!("plugin-icon-{}", info.id),
        image,
        egui::TextureOptions::LINEAR,
    );
    ui.ctx()
        .data_mut(|data| data.insert_temp(cache_id, texture.clone()));
    Some(texture)
}

pub(super) fn hud_contribution_icon_texture(
    ui: &Ui,
    plugin_id: &str,
    contribution: &deskhud_engine::HudContribution,
) -> Option<egui::TextureHandle> {
    let bytes = contribution.icon?;
    let cache_id = ui.make_persistent_id(("hud-contribution-icon", plugin_id, contribution.id));
    if let Some(texture) = ui.ctx().data(|data| data.get_temp(cache_id)) {
        return Some(texture);
    }
    let image = crate::image_decode::decode(bytes, 1536)?;
    let texture = ui.ctx().load_texture(
        format!("hud-contribution-icon-{plugin_id}-{}", contribution.id),
        image,
        egui::TextureOptions::LINEAR,
    );
    ui.ctx()
        .data_mut(|data| data.insert_temp(cache_id, texture.clone()));
    Some(texture)
}

pub(super) fn draw_plugin_list_card(
    ui: &mut egui::Ui,
    catalogs: &CatalogStore,
    model: &SettingsModel,
    plugin: &deskhud_engine::PluginInfo,
    selected_id: &str,
    card_width: f32,
    enabled: &mut bool,
) -> bool {
    const HEIGHT: f32 = 82.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(card_width, HEIGHT), Sense::click());
    let selected = plugin.id == selected_id;
    let fill = if selected {
        ui.visuals().selection.bg_fill.gamma_multiply(0.18)
    } else if response.hovered() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().faint_bg_color
    };
    ui.painter().rect(
        rect.shrink(0.5),
        9.0,
        fill,
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
    let name = catalog_text(
        catalogs,
        model.draft.locale,
        plugin.id,
        "display_name",
        plugin.display_name,
    );
    let description = catalog_text(
        catalogs,
        model.draft.locale,
        plugin.id,
        "description",
        plugin.description,
    );
    let icon_rect =
        egui::Rect::from_min_size(rect.left_top() + Vec2::new(12.0, 13.0), Vec2::splat(56.0));
    ui.painter()
        .rect_filled(icon_rect, 9.0, ui.visuals().faint_bg_color);
    if let Some(texture) = plugin_icon_texture(ui, plugin) {
        paint_preview_contain(ui, icon_rect.shrink(6.0), &texture);
    }
    let text_left = icon_rect.right() + 10.0;
    let switch_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 33.0, rect.center().y),
        Vec2::new(42.0, 24.0),
    );
    let text_width = (switch_rect.left() - text_left - 10.0).max(0.0);
    ui.painter().text(
        egui::pos2(text_left, rect.top() + 14.0),
        egui::Align2::LEFT_TOP,
        truncate_ui_text(ui, &name, fonts::scaled_font(ui, 1.0), text_width),
        fonts::scaled_font(ui, 1.0),
        ui.visuals().text_color(),
    );
    ui.painter().text(
        egui::pos2(text_left, rect.top() + 39.0),
        egui::Align2::LEFT_TOP,
        format!("{} · {}", plugin.author, plugin.version),
        fonts::scaled_font(ui, 0.82),
        ui.visuals().weak_text_color(),
    );
    ui.painter().text(
        egui::pos2(text_left, rect.top() + 57.0),
        egui::Align2::LEFT_TOP,
        truncate_ui_text(ui, plugin.id, fonts::scaled_font(ui, 0.72), text_width),
        fonts::scaled_font(ui, 0.72),
        ui.visuals().weak_text_color(),
    );
    let toggle_response = components::toggle_switch_with_id(
        ui,
        switch_rect,
        enabled,
        ("hud-plugin-enable", plugin.id),
    );
    let response = plugin_tooltip(response, plugin, &name, &description, model.draft.locale);
    response.clicked() || toggle_response.clicked()
}
