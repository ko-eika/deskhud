//! Interface font configuration.
#![allow(clippy::collapsible_if)]

use super::text;
use crate::{components, fonts};
use deskhud_ui::{Locale, MessageKey, SettingsModel};
use egui::RichText;

pub(super) fn draw(ui: &mut egui::Ui, model: &mut SettingsModel) {
    let preview_text = text(model, MessageKey::SettingsUiFontPreview);
    let preview_size = fonts::scaled_size(ui, 1.0);
    components::config_card(
        ui,
        Some(
            RichText::new(text(model, MessageKey::SettingsUiFont))
                .font(fonts::scaled_font(ui, 1.14))
                .into(),
        ),
        |ui| {
            let locale = crate::fonts::language_tag_for(model.draft.locale);
            let families = crate::fonts::list_font_families_for(&locale);
            let mut family_key = model.draft.shell.ui_font_family.clone();
            let locale_id = ui.make_persistent_id("settings_font_locale");
            let previous_locale = ui.ctx().data_mut(|data| data.get_temp::<Locale>(locale_id));
            let language_changed =
                previous_locale.is_some_and(|previous| previous != model.draft.locale);
            ui.ctx()
                .data_mut(|data| data.insert_temp(locale_id, model.draft.locale));
            // 语言变化会重新得到兼容字体列表。若当前字体族或样式已不可用，
            // 始终从该字体族中优先选择 Regular / Normal 等正常样式。
            let current_family = families
                .iter()
                .find(|family| family.family_key == family_key);
            if language_changed {
                if let Some(family) = current_family {
                    apply_default_font_face(model, family);
                } else if let Some(family) = families.first() {
                    family_key = family.family_key.clone();
                    model.draft.shell.ui_font_family = family_key.clone();
                    apply_default_font_face(model, family);
                }
            } else if current_family.is_none() {
                if let Some(family) = families.first() {
                    family_key = family.family_key.clone();
                    model.draft.shell.ui_font_family = family_key.clone();
                    apply_default_font_face(model, family);
                }
            } else if let Some(family) = current_family
                && !family
                    .faces
                    .iter()
                    .any(|face| face.style == model.draft.shell.ui_font_style)
            {
                apply_default_font_face(model, family);
            }
            // 字体数据不能在 egui 当前绘制帧中重新注册，否则会重建字体图集并阻塞窗口调度。
            // 这里先使用当前界面字体绘制名称，实际字体切换仍在下一帧统一应用。
            let family_options: Vec<components::DropdownOption> = families
                .iter()
                .map(|family| (family.family_key.clone(), family.label_for_locale(&locale)))
                .collect();
            components::config_row_with_divider(
                ui,
                text(model, MessageKey::SettingsUiFontFamily),
                None::<RichText>,
                true,
                |ui| {
                    if let Some(selected) = components::dropdown(
                        ui,
                        "settings_font_family",
                        &family_key,
                        &family_options,
                        true,
                    ) {
                        model.draft.shell.ui_font_family = selected.clone();
                        if let Some(family) =
                            families.iter().find(|family| family.family_key == selected)
                        {
                            apply_default_font_face(model, family);
                        }
                    }
                },
            );
            let selected_family = families
                .iter()
                .find(|f| f.family_key == model.draft.shell.ui_font_family);
            let styles =
                selected_family.map_or_else(|| vec!["Regular".to_owned()], |f| f.style_names());
            let style_options: Vec<components::DropdownOption> = styles
                .iter()
                // 样式名称直接使用字体扫描结果，不再做本地化或规范化显示。
                .map(|style| (style.clone(), style.clone()))
                .collect();
            components::config_row_with_divider(
                ui,
                text(model, MessageKey::SettingsUiFontStyle),
                None::<RichText>,
                true,
                |ui| {
                    if let Some(style) = components::dropdown(
                        ui,
                        "settings_font_style",
                        &model.draft.shell.ui_font_style,
                        &style_options,
                        false,
                    ) {
                        model.draft.shell.ui_font_style = style.clone();
                        if let Some(face) = selected_family.and_then(|f| f.face_for(&style)) {
                            model.draft.shell.ui_font_id =
                                crate::fonts::persistable_font_id(&face.font_id);
                        }
                    }
                },
            );
            let size_options: Vec<components::DropdownOption> =
                [12.0, 13.0, 14.0, 15.0, 16.0, 18.0, 20.0]
                    .into_iter()
                    .map(|size| (format!("{size:.0}"), format!("{size:.0}")))
                    .collect();
            components::config_row_with_divider(
                ui,
                text(model, MessageKey::SettingsUiFontSize),
                None::<RichText>,
                false,
                |ui| {
                    let selected_size = format!("{:.0}", model.draft.shell.ui_font_size);
                    if let Some(size) = components::dropdown(
                        ui,
                        "settings_font_size",
                        &selected_size,
                        &size_options,
                        false,
                    ) {
                        if let Ok(size) = size.parse::<f32>() {
                            model.draft.shell.ui_font_size = size;
                        }
                    }
                },
            );
        },
        Some(Box::new(move |ui: &mut egui::Ui| {
            ui.label(RichText::new(preview_text).font(egui::FontId::proportional(preview_size)));
        })),
    );
}

pub(super) fn apply_default_font_face(
    model: &mut SettingsModel,
    family: &deskhud_ui::font::FontFamilyEntry,
) {
    if let Some(face) = preferred_default_face(family) {
        model.draft.shell.ui_font_style = face.style.clone();
        model.draft.shell.ui_font_id = crate::fonts::persistable_font_id(&face.font_id);
    }
}

fn preferred_default_face(
    family: &deskhud_ui::font::FontFamilyEntry,
) -> Option<&deskhud_ui::font::FontFace> {
    family
        .faces
        .iter()
        .find(|face| {
            let raw = face.style.trim().to_ascii_lowercase().replace(' ', "");
            raw == "regular"
                || raw == "normal"
                || raw == "book"
                || raw == "roman"
                || raw == "plain"
                || deskhud_ui::font::normalize_style_name(&face.style) == "Regular"
        })
        .or_else(|| family.faces.first())
}
