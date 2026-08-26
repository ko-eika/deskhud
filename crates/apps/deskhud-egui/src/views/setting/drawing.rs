//! Settings 页面布局与控件绘制。

use std::sync::Arc;

use crate::components;
use deskhud_engine::EngineRegistry;
use deskhud_ui::{
    AnimationQuality, FpsLimit, Locale, MessageKey, PowerMode, SettingsCommand, SettingsModel,
    SettingsTab, UiPreferences, UiTheme,
};
use egui::{
    Align, CentralPanel, Color32, CornerRadius, Frame, Layout, Margin, Panel, Pos2, RichText,
    ScrollArea, Sense, Stroke, Ui, Vec2,
};

/// 绘制设置窗口，返回是否请求关闭窗口。
pub(super) fn draw(
    ui: &mut Ui,
    registry: &Arc<EngineRegistry>,
    model: &mut SettingsModel,
) -> (bool, Option<UiPreferences>) {
    let mut applied_preferences = None;
    ensure_font_compatible_with_locale(ui, model);
    let locale = model.draft.locale;

    Panel::left("settings_nav")
        .exact_size(168.0)
        .resizable(false)
        .frame(
            Frame::NONE
                .fill(ui.visuals().faint_bg_color)
                .inner_margin(Margin::same(12)),
        )
        .show(ui, |ui| draw_navigation(ui, model, locale));

    Panel::bottom("settings_footer")
        .exact_size(54.0)
        .frame(
            Frame::NONE
                .fill(ui.visuals().extreme_bg_color)
                .inner_margin(Margin::symmetric(18, 10)),
        )
        .show(ui, |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if footer_button(
                    ui,
                    text(model, MessageKey::ActionApply),
                    model.is_dirty(),
                    ui.visuals().selection.bg_fill,
                    ui.visuals().selection.bg_fill.gamma_multiply(1.16),
                )
                .clicked()
                {
                    applied_preferences = model.command(SettingsCommand::ApplyKeepOpen);
                }
                if footer_button(
                    ui,
                    text(model, MessageKey::ActionReset),
                    model.is_dirty(),
                    ui.visuals().widgets.inactive.bg_fill,
                    ui.visuals().widgets.hovered.bg_fill,
                )
                .clicked()
                {
                    model.command(SettingsCommand::Reset);
                }
            });
        });

    CentralPanel::default()
        .frame(
            Frame::NONE
                .fill(ui.visuals().extreme_bg_color)
                .inner_margin(Margin::same(24)),
        )
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("settings_content_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| match model.tab {
                    SettingsTab::General => draw_general(ui, model),
                    SettingsTab::Performance => draw_performance(ui, model),
                    SettingsTab::Hud => draw_hud(ui, registry, model),
                    _ => draw_placeholder(ui, model),
                });
        });

    (false, applied_preferences)
}

/// Ensures that language controls are never drawn with a font that cannot
/// render the newly selected language. This runs before the sidebar and locale
/// dropdown, rather than after the font section, so the first frame after a
/// language change already has a compatible face.
fn ensure_font_compatible_with_locale(ui: &mut Ui, model: &mut SettingsModel) {
    let locale = crate::fonts::language_tag_for(model.draft.locale);
    let families = crate::fonts::list_font_families_for(&locale);
    let locale_id = ui.make_persistent_id("settings_font_locale_guard");
    let previous_locale = ui.ctx().data_mut(|data| data.get_temp::<Locale>(locale_id));
    let language_changed = previous_locale.is_some_and(|previous| previous != model.draft.locale);
    ui.ctx()
        .data_mut(|data| data.insert_temp(locale_id, model.draft.locale));

    let current_family = families
        .iter()
        .find(|family| family.family_key == model.draft.shell.ui_font_family);
    let current_style_available = current_family.is_some_and(|family| {
        family
            .faces
            .iter()
            .any(|face| face.style == model.draft.shell.ui_font_style)
    });
    if !language_changed && current_family.is_some() && current_style_available {
        return;
    }

    let Some(family) = current_family.or_else(|| families.first()) else {
        return;
    };
    model.draft.shell.ui_font_family = family.family_key.clone();
    apply_default_font_face(model, family);
}

fn draw_navigation(ui: &mut egui::Ui, model: &mut SettingsModel, locale: Locale) {
    ui.heading(RichText::new(text(model, MessageKey::SettingsTitle)).size(20.0));
    ui.label(
        RichText::new(text(model, MessageKey::AppName))
            .size(12.0)
            .color(Color32::from_gray(165)),
    );
    ui.add_space(22.0);
    for tab in SettingsTab::ALL {
        let selected = model.tab == tab;
        if nav_button(
            ui,
            tab,
            selected,
            text_for_locale(locale, tab.nav_message()),
        )
        .clicked()
        {
            model.command(SettingsCommand::Navigate(tab));
        }
    }
}

fn nav_button(ui: &mut egui::Ui, tab: SettingsTab, selected: bool, label: &str) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(144.0, 36.0), Sense::hover());
    let response = ui.interact(rect, ui.id().with(tab.nav_message()), Sense::click());
    let hovered = response.hovered() || ui.rect_contains_pointer(rect);
    if selected {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(8),
            components::with_alpha(
                ui.visuals().selection.bg_fill,
                if hovered { 112 } else { 96 },
            ),
        );
    } else if hovered {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(8),
            ui.visuals().widgets.hovered.bg_fill,
        );
    }
    let color = if selected {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().text_color()
    };
    draw_nav_icon(ui, rect.left_center() + Vec2::new(18.0, 0.0), tab, color);
    ui.painter().text(
        Pos2::new(rect.left() + 42.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        color,
    );
    response
}

fn draw_nav_icon(ui: &egui::Ui, center: Pos2, tab: SettingsTab, color: Color32) {
    let painter = ui.painter();
    match tab {
        SettingsTab::General => {
            for y in [-5.0, 0.0, 5.0] {
                painter.line_segment(
                    [center + Vec2::new(-7.0, y), center + Vec2::new(7.0, y)],
                    Stroke::new(1.5, color),
                );
            }
            for x in [-3.0, 2.0, -1.0] {
                painter.circle_filled(center + Vec2::new(x, 0.0), 2.0, color);
            }
        }
        SettingsTab::Performance => {
            for (x, h) in [(-6.0, 5.0), (0.0, 9.0), (6.0, 13.0)] {
                painter.rect_filled(
                    egui::Rect::from_center_size(
                        center + Vec2::new(x, 3.0 - h * 0.5),
                        Vec2::new(3.0, h),
                    ),
                    CornerRadius::same(1),
                    color,
                );
            }
        }
        SettingsTab::Pet => {
            painter.circle_stroke(center, 7.0, Stroke::new(1.8, color));
            painter.circle_filled(center, 2.0, color);
        }
        SettingsTab::Hud => {
            for y in [-5.0, 0.0, 5.0] {
                painter.line_segment(
                    [center + Vec2::new(-6.0, y), center + Vec2::new(7.0, y)],
                    Stroke::new(1.8, color),
                );
            }
        }
        SettingsTab::About => {
            painter.circle_stroke(center, 7.0, Stroke::new(1.8, color));
            painter.text(
                center + Vec2::new(0.0, 0.5),
                egui::Align2::CENTER_CENTER,
                "i",
                egui::FontId::proportional(11.0),
                color,
            );
        }
    }
}

fn draw_general(ui: &mut egui::Ui, model: &mut SettingsModel) {
    ui.heading(RichText::new(text(model, MessageKey::SettingsNavGeneral)).size(24.0));
    ui.add_space(18.0);
    components::config_card(
        ui,
        None,
        |ui| {
            components::switch_row(
                ui,
                RichText::new(text(model, MessageKey::SettingsTopmost)),
                Some(RichText::new(text(model, MessageKey::SettingsTopmostHint)).small()),
                &mut model.draft.shell.topmost,
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
                |ui| locale_combo(ui, model),
            );
        },
        None,
    );
    ui.add_space(14.0);
    draw_font_section(ui, model);
}

fn draw_performance(ui: &mut egui::Ui, model: &mut SettingsModel) {
    ui.heading(RichText::new(text(model, MessageKey::SettingsNavPerformance)).size(24.0));
    ui.add_space(8.0);
    ui.label(
        RichText::new(text(model, MessageKey::SettingsPerformanceIntro))
            .color(ui.visuals().weak_text_color()),
    );
    ui.add_space(14.0);

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
                RichText::new(text(model, MessageKey::SettingsPerformanceBubbles)),
                None::<RichText>,
                &mut model.draft.graphics.bubbles,
            );
            ui.separator();
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

fn draw_hud(ui: &mut egui::Ui, registry: &Arc<EngineRegistry>, model: &mut SettingsModel) {
    ui.heading(RichText::new(text(model, MessageKey::SettingsNavHud)).size(24.0));
    ui.add_space(8.0);
    ui.label(
        RichText::new(text(model, MessageKey::HudSettingsIntro))
            .color(ui.visuals().weak_text_color()),
    );
    ui.add_space(14.0);
    let initial_master_enabled = model.draft.hud.is_master_enabled();
    let mut master_enabled = initial_master_enabled;
    components::switch_group(
        ui,
        "hud.master",
        RichText::new(text(model, MessageKey::HudMasterEnable)).size(15.0),
        Some(
            RichText::new(if model.draft.hud.is_master_enabled() {
                text(model, MessageKey::HudMasterEnableHint)
            } else {
                text(model, MessageKey::HudMasterDisabledHint)
            })
            .small(),
        ),
        &mut master_enabled,
        |ui| {
            let plugins = registry.plugin_infos();
            if plugins.is_empty() {
                ui.label(text(model, MessageKey::HudSettingsEmpty));
                return;
            }
            for plugin in plugins {
                let contributions: Vec<_> = registry
                    .all_hud_contributions()
                    .into_iter()
                    .filter(|(id, _)| *id == plugin.id)
                    .map(|(_, contribution)| contribution)
                    .collect();
                if contributions.is_empty() {
                    continue;
                }
                let initial_plugin_enabled = model.draft.hud.is_plugin_enabled(plugin.id);
                let mut plugin_enabled = initial_plugin_enabled;
                let plugin_id = plugin.id;
                components::switch_group(
                    ui,
                    egui::Id::new(("hud.plugin", plugin_id)),
                    RichText::new(plugin.display_name).size(14.0),
                    Some(RichText::new(format!("{} · {}", plugin.author, plugin.version)).small()),
                    &mut plugin_enabled,
                    |ui| {
                        for (index, contribution) in contributions.iter().enumerate() {
                            if index > 0 {
                                ui.separator();
                            }
                            let initial_item_enabled = model.draft.hud.is_enabled(
                                plugin_id,
                                contribution.id,
                                contribution.default_enabled,
                            );
                            let mut item_enabled = initial_item_enabled;
                            components::switch_row(
                                ui,
                                RichText::new(contribution.label),
                                None::<RichText>,
                                &mut item_enabled,
                            );
                            if item_enabled != initial_item_enabled {
                                model.draft.hud.set_enabled(
                                    plugin_id,
                                    contribution.id,
                                    item_enabled,
                                );
                            }
                        }
                    },
                );
                if plugin_enabled != initial_plugin_enabled {
                    model
                        .draft
                        .hud
                        .set_plugin_enabled(plugin_id, plugin_enabled);
                }
            }
        },
    );
    if master_enabled != initial_master_enabled {
        model.draft.hud.set_master_enabled(master_enabled);
    }
}

fn draw_font_section(ui: &mut egui::Ui, model: &mut SettingsModel) {
    let preview_text = text(model, MessageKey::SettingsUiFontPreview);
    let preview_size = model.draft.shell.ui_font_size.clamp(11.0, 22.0);
    components::config_card(
        ui,
        Some(
            RichText::new(text(model, MessageKey::SettingsUiFont))
                .size(16.0)
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
            components::config_row(
                ui,
                text(model, MessageKey::SettingsUiFontFamily),
                None::<RichText>,
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
            ui.separator();
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
            components::config_row(
                ui,
                text(model, MessageKey::SettingsUiFontStyle),
                None::<RichText>,
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
                            model.draft.shell.ui_font_id = face.font_id.clone();
                        }
                    }
                },
            );
            ui.separator();
            let size_options: Vec<components::DropdownOption> =
                [12.0, 13.0, 14.0, 15.0, 16.0, 18.0, 20.0]
                    .into_iter()
                    .map(|size| (format!("{size:.0}"), format!("{size:.0}")))
                    .collect();
            components::config_row(
                ui,
                text(model, MessageKey::SettingsUiFontSize),
                None::<RichText>,
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
            ui.label(RichText::new(preview_text).size(preview_size));
        })),
    );
}

fn apply_default_font_face(model: &mut SettingsModel, family: &deskhud_ui::font::FontFamilyEntry) {
    if let Some(face) = preferred_default_face(family) {
        model.draft.shell.ui_font_style = face.style.clone();
        model.draft.shell.ui_font_id = face.font_id.clone();
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

fn draw_placeholder(ui: &mut egui::Ui, model: &SettingsModel) {
    ui.heading(
        RichText::new(text_for_locale(model.draft.locale, model.tab.nav_message())).size(24.0),
    );
    ui.add_space(18.0);
    let intro = match model.tab {
        SettingsTab::Performance => MessageKey::SettingsPerformanceIntro,
        SettingsTab::Pet => MessageKey::SettingsPetIntro,
        SettingsTab::Hud => MessageKey::HudSettingsEmpty,
        SettingsTab::About => MessageKey::SettingsAboutIntro,
        SettingsTab::General => MessageKey::SettingsNavGeneral,
    };
    ui.label(text_for_locale(model.draft.locale, intro));
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

fn locale_combo(ui: &mut egui::Ui, model: &mut SettingsModel) {
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
    let selected = match current {
        Locale::ZhCn => "zh-CN",
        Locale::En => "en-US",
        Locale::System => "system",
    };
    if let Some(value) = components::dropdown(ui, "settings_locale", selected, &options, false) {
        model.draft.locale = match value.as_str() {
            "zh-CN" => Locale::ZhCn,
            "en-US" => Locale::En,
            _ => Locale::System,
        };
    }
}

fn footer_button(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    fill: Color32,
    _hover_fill: Color32,
) -> egui::Response {
    let button = egui::Button::new(RichText::new(label).size(14.0))
        .min_size(Vec2::new(88.0, 32.0))
        .fill(fill)
        .corner_radius(CornerRadius::same(8));
    ui.add_enabled(enabled, button)
}

fn text(model: &SettingsModel, key: MessageKey) -> &'static str {
    text_for_locale(model.draft.locale, key)
}

fn text_for_locale(locale: Locale, key: MessageKey) -> &'static str {
    match locale.resolved() {
        Locale::ZhCn => deskhud_ui::i18n::t(Locale::ZhCn, key),
        Locale::En | Locale::System => deskhud_ui::i18n::t(Locale::En, key),
    }
}
