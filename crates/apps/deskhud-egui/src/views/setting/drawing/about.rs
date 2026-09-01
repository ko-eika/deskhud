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
            components::config_row(
                ui,
                text(model, MessageKey::SettingsAboutVersion),
                None::<RichText>,
                |ui| value(ui, &info.version),
            );
            ui.separator();
            components::config_row(
                ui,
                text(model, MessageKey::SettingsAboutAuthors),
                None::<RichText>,
                |ui| value(ui, &info.authors),
            );
            ui.separator();
            components::config_row(
                ui,
                text(model, MessageKey::SettingsAboutLicense),
                None::<RichText>,
                |ui| value(ui, &info.license),
            );
            ui.separator();
            components::config_row(
                ui,
                text(model, MessageKey::SettingsAboutStackLabel),
                None::<RichText>,
                |ui| value(ui, &info.stack),
            );
            ui.separator();
            components::config_row(
                ui,
                text(model, MessageKey::SettingsAboutHomepage),
                None::<RichText>,
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
