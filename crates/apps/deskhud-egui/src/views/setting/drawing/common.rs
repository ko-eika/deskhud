//! Shared helpers used by the settings pages.

use super::text_for_locale;
use crate::fonts;
use deskhud_ui::{CatalogStore, Locale, MessageKey, SettingsModel, SettingsTab};
use egui::{Color32, CornerRadius, RichText, Stroke, TextureHandle, Ui};

pub(crate) fn truncate_ui_text(ui: &Ui, text: &str, font: egui::FontId, max_width: f32) -> String {
    if ui.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(text.into(), font.clone(), Color32::WHITE)
            .size()
            .x
    }) <= max_width
    {
        return text.into();
    }
    let mut result = String::new();
    for ch in text.chars() {
        let mut candidate = result.clone();
        candidate.push(ch);
        candidate.push('…');
        if ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(candidate.clone(), font.clone(), Color32::WHITE)
                .size()
                .x
        }) > max_width
        {
            break;
        }
        result.push(ch);
    }
    if result.is_empty() {
        "…".into()
    } else {
        format!("{result}…")
    }
}

pub(crate) fn paint_preview_contain(ui: &Ui, stage: egui::Rect, texture: &TextureHandle) {
    let size = texture.size_vec2();
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    let scale = (stage.width() / size.x).min(stage.height() / size.y);
    let image_rect = egui::Rect::from_center_size(stage.center(), size * scale);
    ui.painter().with_clip_rect(stage).image(
        texture.id(),
        image_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

pub(crate) fn catalog_text(
    catalogs: &CatalogStore,
    locale: Locale,
    id: &str,
    field: &str,
    fallback: &str,
) -> String {
    let key = format!("{id}.{field}");
    let fallback = if fallback == field || fallback.contains('.') {
        key.as_str()
    } else {
        fallback
    };
    catalogs.t(locale, &key, fallback).to_owned()
}

pub(crate) fn tooltip_meta_row(ui: &mut Ui, locale: Locale, key: MessageKey, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(text_for_locale(locale, key))
                .font(fonts::scaled_font(ui, 0.82))
                .color(ui.visuals().weak_text_color()),
        );
        ui.label(RichText::new(value).font(fonts::scaled_font(ui, 0.86)));
    });
}

pub(crate) fn paint_preview_frame(ui: &Ui, preview: egui::Rect, scale: f32) {
    ui.painter().rect(
        preview,
        CornerRadius::same((10.0 * scale).round() as u8),
        ui.visuals().faint_bg_color,
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
}

pub(crate) fn draw_empty(ui: &mut Ui, model: &SettingsModel) {
    let message = match model.tab {
        SettingsTab::Hud => MessageKey::HudSettingsEmpty,
        _ => MessageKey::SettingsPetEmpty,
    };
    ui.label(text_for_locale(model.draft.locale, message));
}
