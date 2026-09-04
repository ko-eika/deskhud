//! About page information.

use super::text;
use crate::{components, fonts};
use deskhud_ui::{AboutInfo, MessageKey, SettingsModel};
use egui::{RichText, Ui};

pub(super) fn draw(ui: &mut Ui, model: &SettingsModel) {
    let info = AboutInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        authors: env!("CARGO_PKG_AUTHORS").to_owned(),
        license: "Apache-2.0".to_owned(),
        stack: text(model, MessageKey::SettingsAboutStack).to_owned(),
        homepage: "https://github.com/ko-eika/deskhud".to_owned(),
    };

    components::config_card(
        ui,
        Some(
            RichText::new(text(model, MessageKey::SettingsNavAbout))
                .strong()
                .into(),
        ),
        |ui| {
            components::config_row_with_divider(
                ui,
                text(model, MessageKey::SettingsAboutVersion),
                None::<RichText>,
                true,
                |ui| value(ui, &info.version),
            );
            components::config_row_with_divider(
                ui,
                text(model, MessageKey::SettingsAboutAuthors),
                None::<RichText>,
                true,
                |ui| value(ui, &info.authors),
            );
            components::config_row_with_divider(
                ui,
                text(model, MessageKey::SettingsAboutLicense),
                None::<RichText>,
                true,
                |ui| value(ui, &info.license),
            );
            components::config_row_with_divider(
                ui,
                text(model, MessageKey::SettingsAboutStackLabel),
                None::<RichText>,
                true,
                |ui| value(ui, &info.stack),
            );
            components::config_row_with_divider(
                ui,
                text(model, MessageKey::SettingsAboutHomepage),
                None::<RichText>,
                false,
                |ui| {
                    ui.add(egui::Hyperlink::from_label_and_url(
                        RichText::new(&info.homepage).font(fonts::scaled_font(ui, 0.92)),
                        &info.homepage,
                    ));
                },
            );
        },
        None,
    );
}

fn value(ui: &mut Ui, value: &str) {
    ui.label(RichText::new(value).font(fonts::scaled_font(ui, 0.92)));
}
