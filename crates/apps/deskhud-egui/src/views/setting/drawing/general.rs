//! General settings groups.

use super::{text, text_for_locale};
use crate::components;
use deskhud_ui::{CatalogStore, Locale, MessageKey, SettingsModel, UiTheme};
use egui::RichText;

pub(super) fn draw(ui: &mut egui::Ui, model: &mut SettingsModel, catalogs: &CatalogStore) {
    components::config_card(
        ui,
        None,
        |ui| {
            components::config_row(
                ui,
                text(model, MessageKey::SettingsTheme),
                None::<RichText>,
                |ui| theme_combo(ui, model),
            );
        },
        None,
    );
    ui.add_space(14.0);
    components::config_card(
        ui,
        None,
        |ui| {
            components::config_row(
                ui,
                text(model, MessageKey::SettingsLocale),
                None::<RichText>,
                |ui| locale_combo(ui, model, catalogs),
            );
        },
        None,
    );
    ui.add_space(14.0);
    super::font::draw(ui, model);
}

fn theme_combo(ui: &mut egui::Ui, model: &mut SettingsModel) {
    let options = vec![
        (
            "system".into(),
            text(model, MessageKey::OptThemeSystem).into(),
        ),
        (
            "light".into(),
            text(model, MessageKey::OptThemeLight).into(),
        ),
        ("dark".into(), text(model, MessageKey::OptThemeDark).into()),
    ];
    let selected = match model.draft.shell.ui_theme {
        UiTheme::Light => "light",
        UiTheme::Dark => "dark",
        UiTheme::System => "system",
    };
    if let Some(value) = components::dropdown(ui, "settings_theme", selected, &options, false) {
        model.draft.shell.ui_theme = match value.as_str() {
            "light" => UiTheme::Light,
            "dark" => UiTheme::Dark,
            _ => UiTheme::System,
        };
    }
}

fn locale_combo(ui: &mut egui::Ui, model: &mut SettingsModel, catalogs: &CatalogStore) {
    let current = model.draft.locale;
    let options = vec![
        (
            "system".into(),
            text_for_locale(current, MessageKey::OptLocaleSystem).into(),
        ),
        (
            "zh-CN".into(),
            text_for_locale(current, MessageKey::OptLocaleZh).into(),
        ),
        (
            "en-US".into(),
            text_for_locale(current, MessageKey::OptLocaleEn).into(),
        ),
    ];
    let selected = match &current {
        Locale::ZhCn => "zh-CN",
        Locale::En => "en-US",
        Locale::System => "system",
        Locale::Custom(tag) => tag,
    };
    let mut options = options;
    for tag in catalogs.locales() {
        if matches!(tag.as_str(), "en" | "en-US" | "zh" | "zh-CN") {
            continue;
        }
        if !options.iter().any(|(id, _)| id == &tag) {
            options.push((tag.clone(), tag));
        }
    }
    if let Some(value) = components::dropdown(ui, "settings_locale", selected, &options, false) {
        model.draft.locale = Locale::from_tag(&value).unwrap_or(Locale::System);
    }
}
