//! Performance settings groups.

use super::text;
use crate::components;
use deskhud_ui::{AnimationQuality, FpsLimit, MessageKey, PowerMode, SettingsModel};
use egui::RichText;

pub(super) fn draw(ui: &mut egui::Ui, model: &mut SettingsModel) {
    components::config_card(
        ui,
        None,
        |ui| {
            components::config_row(
                ui,
                text(model, MessageKey::SettingsPerformanceFps),
                None::<RichText>,
                |ui| {
                    let options: Vec<components::DropdownOption> = [
                        ("auto", text(model, MessageKey::SettingsPerformanceAuto)),
                        ("30", "30"),
                        ("60", "60"),
                        ("120", "120"),
                    ]
                    .into_iter()
                    .map(|(key, label)| (key.to_owned(), label.to_owned()))
                    .collect();
                    let selected = match model.draft.graphics.fps_limit {
                        FpsLimit::Auto => "auto",
                        FpsLimit::Fps30 => "30",
                        FpsLimit::Fps60 => "60",
                        FpsLimit::Fps120 => "120",
                    };
                    if let Some(value) = components::dropdown(
                        ui,
                        "settings_performance_fps",
                        selected,
                        &options,
                        false,
                    ) {
                        model.draft.graphics.fps_limit = match value.as_str() {
                            "30" => FpsLimit::Fps30,
                            "60" => FpsLimit::Fps60,
                            "120" => FpsLimit::Fps120,
                            _ => FpsLimit::Auto,
                        };
                    }
                },
            );
            ui.separator();
            components::config_row(
                ui,
                text(model, MessageKey::SettingsPerformanceAnimation),
                None::<RichText>,
                |ui| {
                    let options: Vec<components::DropdownOption> = [
                        ("low", text(model, MessageKey::SettingsPerformanceLow)),
                        (
                            "standard",
                            text(model, MessageKey::SettingsPerformanceStandard),
                        ),
                        ("high", text(model, MessageKey::SettingsPerformanceHigh)),
                    ]
                    .into_iter()
                    .map(|(key, label)| (key.to_owned(), label.to_owned()))
                    .collect();
                    let selected = match model.draft.graphics.animation_quality {
                        AnimationQuality::Low => "low",
                        AnimationQuality::Standard => "standard",
                        AnimationQuality::High => "high",
                    };
                    if let Some(value) = components::dropdown(
                        ui,
                        "settings_performance_animation",
                        selected,
                        &options,
                        false,
                    ) {
                        model.draft.graphics.animation_quality = match value.as_str() {
                            "low" => AnimationQuality::Low,
                            "high" => AnimationQuality::High,
                            _ => AnimationQuality::Standard,
                        };
                    }
                },
            );
            ui.separator();
            components::config_row(
                ui,
                text(model, MessageKey::SettingsPerformancePower),
                None::<RichText>,
                |ui| {
                    let options: Vec<components::DropdownOption> = [
                        ("saving", text(model, MessageKey::SettingsPerformanceSaving)),
                        (
                            "balanced",
                            text(model, MessageKey::SettingsPerformanceBalanced),
                        ),
                        ("smooth", text(model, MessageKey::SettingsPerformanceSmooth)),
                    ]
                    .into_iter()
                    .map(|(key, label)| (key.to_owned(), label.to_owned()))
                    .collect();
                    let selected = match model.draft.graphics.power_mode {
                        PowerMode::Saving => "saving",
                        PowerMode::Balanced => "balanced",
                        PowerMode::Smooth => "smooth",
                    };
                    if let Some(value) = components::dropdown(
                        ui,
                        "settings_performance_power",
                        selected,
                        &options,
                        false,
                    ) {
                        model.draft.graphics.power_mode = match value.as_str() {
                            "saving" => PowerMode::Saving,
                            "smooth" => PowerMode::Smooth,
                            _ => PowerMode::Balanced,
                        };
                    }
                },
            );
        },
        None,
    );
    ui.add_space(14.0);
    components::config_card(
        ui,
        Some(text(model, MessageKey::SettingsPerformanceEffects).into()),
        |ui| {
            components::switch_row(
                ui,
                RichText::new(text(model, MessageKey::SettingsPerformanceShadows)),
                None::<RichText>,
                &mut model.draft.graphics.shadows,
            );
        },
        None,
    );
}
