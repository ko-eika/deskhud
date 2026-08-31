//! Settings 页面布局与控件绘制。

use std::sync::Arc;

use crate::{components, fonts};
use deskhud_engine::EngineRegistry;
use deskhud_ui::{
    AnimationQuality, CatalogStore, FpsLimit, LayerPreference, Locale, MessageKey, PowerMode,
    SettingsCommand, SettingsModel, SettingsTab, UiPreferences, UiTheme,
};
use egui::{
    Align, CentralPanel, Color32, CornerRadius, Frame, Layout, Margin, Panel, Pos2, RichText,
    ScrollArea, Sense, Stroke, TextureOptions, Ui, Vec2,
};

/// 绘制设置窗口，返回是否请求关闭窗口。
pub(super) fn draw(
    ui: &mut Ui,
    registry: &Arc<EngineRegistry>,
    catalogs: &CatalogStore,
    model: &mut SettingsModel,
) -> (bool, Option<UiPreferences>) {
    let mut applied_preferences = None;
    ensure_font_compatible_with_locale(ui, model);
    let locale = model.draft.locale;

    Panel::left("settings_nav")
        .exact_size(220.0)
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
                    SettingsTab::General => draw_general(ui, model, catalogs),
                    SettingsTab::Performance => draw_performance(ui, model),
                    SettingsTab::Pet => draw_pet(ui, registry, catalogs, model),
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
    ui.heading(
        RichText::new(text(model, MessageKey::SettingsTitle)).font(fonts::scaled_font(ui, 1.43)),
    );
    ui.label(
        RichText::new(text(model, MessageKey::AppName))
            .font(fonts::scaled_font(ui, 0.86))
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
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::hover());
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
    let label_font = fonts::scaled_font(ui, 1.0);
    let display_label = truncate_ui_text(ui, label, label_font.clone(), rect.width() - 54.0);
    draw_nav_icon(ui, rect.left_center() + Vec2::new(18.0, 0.0), tab, color);
    ui.painter().text(
        Pos2::new(rect.left() + 42.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        display_label.as_str(),
        label_font,
        color,
    );
    if display_label != label {
        response.on_hover_text(label)
    } else {
        response
    }
}

fn draw_nav_icon(ui: &egui::Ui, center: Pos2, tab: SettingsTab, color: Color32) {
    let icon = match tab {
        SettingsTab::General => "adjust-horizontal",
        SettingsTab::Performance => "analytics",
        SettingsTab::Pet => "create-filled",
        SettingsTab::Hud => "puzzle",
        SettingsTab::About => "info",
    };
    components::icons::paint(
        ui,
        icon,
        egui::Rect::from_center_size(center, Vec2::splat(18.0)),
        color,
        false,
    );
}

fn draw_general(ui: &mut egui::Ui, model: &mut SettingsModel, catalogs: &CatalogStore) {
    ui.heading(
        RichText::new(text(model, MessageKey::SettingsNavGeneral))
            .font(fonts::scaled_font(ui, 1.71)),
    );
    ui.add_space(18.0);
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
    draw_font_section(ui, model);
}

fn draw_performance(ui: &mut egui::Ui, model: &mut SettingsModel) {
    ui.heading(
        RichText::new(text(model, MessageKey::SettingsNavPerformance))
            .font(fonts::scaled_font(ui, 1.71)),
    );
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
                RichText::new(text(model, MessageKey::SettingsPerformanceShadows)),
                None::<RichText>,
                &mut model.draft.graphics.shadows,
            );
        },
        None,
    );
}

fn draw_pet(
    ui: &mut Ui,
    registry: &Arc<EngineRegistry>,
    catalogs: &CatalogStore,
    model: &mut SettingsModel,
) {
    ui.heading(
        RichText::new(text(model, MessageKey::SettingsNavPet)).font(fonts::scaled_font(ui, 1.71)),
    );
    // Match the standard settings page title → description rhythm.
    ui.add_space(8.0);
    ui.label(
        RichText::new(text(model, MessageKey::SettingsPetIntro))
            .color(ui.visuals().weak_text_color()),
    );
    ui.add_space(14.0);

    let infos = registry.pet_infos();
    let mode = model.draft.pet.picker_mode;
    components::section_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(text(model, MessageKey::SettingsPetList)).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                draw_pet_view_modes(ui, model)
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
                        draw_pet_grid_card(ui, catalogs, model, info, card_layout);
                    }
                });
            }
            deskhud_ui::PetPickerMode::List => {
                for info in &infos {
                    draw_pet_list_row(ui, catalogs, model, info);
                    ui.add_space(8.0);
                }
            }
        }
    });
    ui.add_space(16.0);
    components::config_card(
        ui,
        Some(
            RichText::new(text(model, MessageKey::SettingsPetGlobal))
                .strong()
                .into(),
        ),
        |ui| draw_pet_global(ui, model),
        None,
    );
    ui.add_space(16.0);

    let active_id = model.draft.pet.kind.clone();
    let Some(pet) = registry
        .pets()
        .into_iter()
        .find(|pet| pet.info().id == active_id)
    else {
        draw_placeholder(ui, model);
        return;
    };
    let options = pet.config_options();
    if options.is_empty() {
        return;
    }

    components::config_card(
        ui,
        Some(
            RichText::new(text(model, MessageKey::SettingsPetConfig))
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
                let label = pet_catalog_text(
                    catalogs,
                    model.draft.locale,
                    &active_id,
                    &format!("{}.label", option.key),
                    option.label,
                );
                let description = pet_catalog_text(
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

fn draw_pet_global(ui: &mut Ui, model: &mut SettingsModel) {
    components::config_row(
        ui,
        text(model, MessageKey::SettingsPetLayer),
        Some(RichText::new(text(model, MessageKey::SettingsPetLayerHint)).small()),
        |ui| {
            let options = [
                (
                    "top".to_owned(),
                    text(model, MessageKey::MenuLayerTop).to_owned(),
                ),
                (
                    "normal".to_owned(),
                    text(model, MessageKey::MenuLayerNormal).to_owned(),
                ),
                (
                    "bottom".to_owned(),
                    text(model, MessageKey::MenuLayerBottom).to_owned(),
                ),
            ];
            let selected = match model.draft.pet.layer {
                LayerPreference::Top => "top",
                LayerPreference::Normal => "normal",
                LayerPreference::Bottom => "bottom",
            };
            if let Some(value) =
                components::dropdown(ui, "settings_pet_layer", selected, &options, false)
            {
                model.draft.pet.layer = match value.as_str() {
                    "bottom" => LayerPreference::Bottom,
                    "normal" => LayerPreference::Normal,
                    _ => LayerPreference::Top,
                };
            }
        },
    );
    ui.separator();
    components::switch_row(
        ui,
        RichText::new(text(model, MessageKey::SettingsPetBubbles)),
        Some(RichText::new(text(model, MessageKey::SettingsPetBubblesHint)).small()),
        &mut model.draft.pet.bubbles,
    );
    ui.separator();
    components::switch_row(
        ui,
        RichText::new(text(model, MessageKey::SettingsPetKeyboardInput)),
        Some(RichText::new(text(model, MessageKey::SettingsPetKeyboardInputHint)).small()),
        &mut model.draft.pet.global_keyboard_input,
    );
    ui.separator();
    components::switch_row(
        ui,
        RichText::new(text(model, MessageKey::SettingsPetMouseInput)),
        Some(RichText::new(text(model, MessageKey::SettingsPetMouseInputHint)).small()),
        &mut model.draft.pet.global_mouse_input,
    );
}

fn draw_pet_view_modes(ui: &mut Ui, model: &mut SettingsModel) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 28.0), Sense::hover());
    ui.painter().rect(
        rect,
        8.0,
        ui.visuals().extreme_bg_color,
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
    let (left, right) = (
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.center().x, rect.max.y)),
        egui::Rect::from_min_max(egui::pos2(rect.center().x, rect.min.y), rect.max),
    );
    let l = ui.interact(left, ui.id().with("pet-grid"), Sense::click());
    let r = ui.interact(right, ui.id().with("pet-list"), Sense::click());
    let active = if model.draft.pet.picker_mode == deskhud_ui::PetPickerMode::Grid {
        left
    } else {
        right
    };
    ui.painter().rect_filled(
        active.shrink(1.0),
        7.0,
        ui.visuals().selection.bg_fill.gamma_multiply(0.18),
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.center().x, rect.top() + 5.0),
            egui::pos2(rect.center().x, rect.bottom() - 5.0),
        ],
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
    let color = ui.visuals().selection.stroke.color;
    let icon_rect = |area: egui::Rect| {
        let side = area.width().min(area.height()) - 12.0;
        egui::Rect::from_center_size(area.center(), Vec2::splat(side.max(1.0)))
    };
    components::icons::paint(ui, "layout-grid", icon_rect(left), color, false);
    components::icons::paint(ui, "list-details", icon_rect(right), color, false);
    if l.clicked() {
        model.draft.pet.picker_mode = deskhud_ui::PetPickerMode::Grid;
    }
    if r.clicked() {
        model.draft.pet.picker_mode = deskhud_ui::PetPickerMode::List;
    }
}

#[allow(dead_code)] // Removed after the new grid/list renderers have passed visual QA.
fn draw_pet_choice(
    ui: &mut Ui,
    model: &mut SettingsModel,
    info: &deskhud_engine::PetKindInfo,
    list: bool,
) {
    let selected = model.draft.pet.kind == info.id;
    let body_size = ui
        .style()
        .text_styles
        .get(&egui::TextStyle::Body)
        .map(|font| font.size)
        .unwrap_or(14.0);
    let small_size = ui
        .style()
        .text_styles
        .get(&egui::TextStyle::Small)
        .map(|font| font.size)
        .unwrap_or(body_size * 0.8);
    let height = if list {
        94.0
    } else if ui.available_width() > 300.0 {
        350.0
    } else {
        208.0
    };
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    let fill = if selected {
        ui.visuals().selection.bg_fill.gamma_multiply(0.38)
    } else if response.hovered() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().faint_bg_color
    };
    let stroke = if selected {
        Stroke::new(2.0, ui.visuals().selection.stroke.color)
    } else {
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
    };
    ui.painter()
        .rect(rect, 12.0, fill, stroke, egui::StrokeKind::Inside);
    let preview = if list {
        egui::Rect::from_min_size(rect.min + Vec2::splat(10.0), Vec2::splat(74.0))
    } else {
        egui::Rect::from_min_max(
            rect.min + Vec2::splat(10.0),
            egui::pos2(rect.max.x - 10.0, rect.min.y + rect.width() - 10.0),
        )
    };
    ui.painter()
        .rect_filled(preview, 8.0, ui.visuals().extreme_bg_color);
    if let Some(bytes) = info.preview {
        let cache_id = ui.make_persistent_id(("pet-preview", info.id));
        let cached = ui
            .ctx()
            .data(|data| data.get_temp::<egui::TextureHandle>(cache_id));
        let texture = cached.or_else(|| {
            let image = crate::image_decode::decode(bytes, 768)?;
            let texture = ui.ctx().load_texture(
                format!("pet-preview-{}", info.id),
                image,
                TextureOptions::LINEAR,
            );
            ui.ctx()
                .data_mut(|data| data.insert_temp(cache_id, texture.clone()));
            Some(texture)
        });
        if let Some(texture) = texture {
            let source = texture.size_vec2();
            let scale = (preview.width() / source.x).min(preview.height() / source.y);
            let fitted = egui::Rect::from_center_size(preview.center(), source * scale);
            ui.painter().image(
                texture.id(),
                fitted,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }
    let text_x = if list {
        rect.left() + 96.0
    } else {
        rect.left() + 14.0
    };
    let title_y = if list {
        rect.top() + 20.0
    } else {
        preview.bottom() + 22.0
    };
    ui.painter().text(
        egui::pos2(text_x, title_y),
        egui::Align2::LEFT_CENTER,
        info.display_name,
        egui::FontId::proportional(body_size),
        ui.visuals().text_color(),
    );
    ui.painter().text(
        egui::pos2(
            text_x,
            if list {
                rect.top() + 40.0
            } else {
                preview.bottom() + 43.0
            },
        ),
        egui::Align2::LEFT_CENTER,
        info.description,
        egui::FontId::proportional(small_size),
        ui.visuals().weak_text_color(),
    );
    ui.painter().text(
        egui::pos2(
            text_x,
            if list {
                rect.top() + 61.0
            } else {
                rect.bottom() - 14.0
            },
        ),
        egui::Align2::LEFT_CENTER,
        format!(
            "{}  ·  {:.0}×{:.0}",
            info.author, info.window_width, info.window_height
        ),
        egui::FontId::proportional(small_size * 0.9),
        ui.visuals().weak_text_color(),
    );
    if response.clicked() {
        let picker_mode = model.draft.pet.picker_mode;
        deskhud_ui::apply_pet_selection(&mut model.draft, info.id.to_string(), picker_mode);
        model
            .draft
            .pet
            .apply_window_size(info.window_width, info.window_height);
    }
}

fn draw_pet_grid_card(
    ui: &mut Ui,
    catalogs: &CatalogStore,
    model: &mut SettingsModel,
    info: &deskhud_engine::PetKindInfo,
    layout: deskhud_ui::PetCardLayout,
) {
    let scale = layout.content_scale;
    let pad = 12.0 * scale;
    let selected = model.draft.pet.kind == info.id;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(layout.card_width, layout.card_height),
        Sense::click(),
    );
    let name = pet_catalog_text(
        catalogs,
        model.draft.locale,
        info.id,
        "display_name",
        info.display_name,
    );
    let description = pet_catalog_text(
        catalogs,
        model.draft.locale,
        info.id,
        "description",
        info.description,
    );
    let response = pet_tooltip(response, info, &name, &description, model.draft.locale);
    let draw = rect.shrink(1.0);
    let fill = if selected {
        ui.visuals().selection.bg_fill.gamma_multiply(0.18)
    } else if response.hovered() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().extreme_bg_color
    };
    let stroke = if selected {
        Stroke::new(
            1.0,
            components::lerp_color(
                ui.visuals().widgets.noninteractive.bg_stroke.color,
                ui.visuals().selection.stroke.color,
                0.32,
            ),
        )
    } else {
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
    };
    ui.painter()
        .rect(draw, 12.0 * scale, fill, stroke, egui::StrokeKind::Inside);

    let stage = egui::Rect::from_center_size(
        egui::pos2(
            draw.center().x,
            draw.top() + pad + layout.preview_side * 0.5,
        ),
        Vec2::splat(layout.preview_side),
    );
    let preview = pet_preview_rect(stage);
    paint_preview_frame(ui, preview, scale);
    if let Some(texture) = pet_preview_texture(ui, info) {
        paint_preview_contain(ui, preview, &texture);
    }

    let title = ui.text_style_height(&egui::TextStyle::Body) * scale * 1.08;
    let small = ui.text_style_height(&egui::TextStyle::Small) * scale;
    // Use a more breathable baseline gap inside the bottom information group;
    // it still scales with the rest of the card content.
    let line_gap = 8.0 * scale;
    let left = draw.left() + pad;
    // Keep the information group bottom-anchored. This makes the preview and
    // the text two stable vertical groups instead of a chain whose gaps vary
    // with the amount of text in each card.
    let description_bottom = draw.bottom() - pad;
    let metadata_bottom = description_bottom - small - line_gap;
    let title_bottom = metadata_bottom - small - line_gap;
    ui.painter().text(
        egui::pos2(left, title_bottom),
        egui::Align2::LEFT_BOTTOM,
        truncate_ui_text(
            ui,
            &name,
            egui::FontId::proportional(title),
            draw.width() - pad * 2.0,
        ),
        egui::FontId::proportional(title),
        ui.visuals().text_color(),
    );
    ui.painter().text(
        egui::pos2(left, metadata_bottom),
        egui::Align2::LEFT_BOTTOM,
        truncate_ui_text(
            ui,
            &format!(
                "{}  ·  {:.0}×{:.0}",
                info.author, info.window_width, info.window_height
            ),
            egui::FontId::proportional(small),
            draw.width() - pad * 2.0,
        ),
        egui::FontId::proportional(small),
        ui.visuals().weak_text_color(),
    );
    ui.painter().text(
        egui::pos2(left, description_bottom),
        egui::Align2::LEFT_BOTTOM,
        truncate_ui_text(
            ui,
            &description,
            egui::FontId::proportional(small),
            draw.width() - pad * 2.0,
        ),
        egui::FontId::proportional(small),
        ui.visuals().weak_text_color(),
    );
    if response.clicked() {
        let mode = model.draft.pet.picker_mode;
        deskhud_ui::apply_pet_selection(&mut model.draft, info.id.to_string(), mode);
        model
            .draft
            .pet
            .apply_window_size(info.window_width, info.window_height);
    }
}

fn draw_pet_list_row(
    ui: &mut Ui,
    catalogs: &CatalogStore,
    model: &mut SettingsModel,
    info: &deskhud_engine::PetKindInfo,
) {
    const PAD: f32 = 12.0;
    let selected = model.draft.pet.kind == info.id;
    let body = ui.text_style_height(&egui::TextStyle::Body);
    let small = ui.text_style_height(&egui::TextStyle::Small);
    const LINE_GAP: f32 = 8.0;
    let text_block_height = body + small * 2.0 + LINE_GAP * 2.0;
    // The preview square exactly matches the complete three-line text group:
    // title, metadata, description, plus the two scaled vertical gaps.
    let thumb_side = text_block_height;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), thumb_side + PAD * 2.0),
        Sense::click(),
    );
    let name = pet_catalog_text(
        catalogs,
        model.draft.locale,
        info.id,
        "display_name",
        info.display_name,
    );
    let description = pet_catalog_text(
        catalogs,
        model.draft.locale,
        info.id,
        "description",
        info.description,
    );
    let response = pet_tooltip(response, info, &name, &description, model.draft.locale);
    let draw = rect.shrink(0.5);
    let fill = if selected {
        ui.visuals().selection.bg_fill.gamma_multiply(0.18)
    } else if response.hovered() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().extreme_bg_color
    };
    let stroke = if selected {
        Stroke::new(
            1.0,
            components::lerp_color(
                ui.visuals().widgets.noninteractive.bg_stroke.color,
                ui.visuals().selection.stroke.color,
                0.32,
            ),
        )
    } else {
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
    };
    ui.painter()
        .rect(draw, 10.0, fill, stroke, egui::StrokeKind::Inside);
    let thumb = egui::Rect::from_min_size(draw.min + Vec2::splat(PAD), Vec2::splat(thumb_side));
    let preview = pet_preview_rect(thumb);
    paint_preview_frame(ui, preview, 1.0);
    if let Some(texture) = pet_preview_texture(ui, info) {
        paint_preview_contain(ui, preview, &texture);
    }
    let left = thumb.right() + PAD;
    let top = thumb.top();
    let description_bottom = thumb.bottom();
    let metadata_top = top + body + LINE_GAP;
    ui.painter().text(
        egui::pos2(left, top),
        egui::Align2::LEFT_TOP,
        truncate_ui_text(
            ui,
            &name,
            egui::FontId::proportional(body),
            draw.right() - left - PAD,
        ),
        egui::FontId::proportional(body),
        ui.visuals().text_color(),
    );
    ui.painter().text(
        egui::pos2(left, metadata_top),
        egui::Align2::LEFT_TOP,
        truncate_ui_text(
            ui,
            &format!(
                "{}  ·  {:.0}×{:.0}",
                info.author, info.window_width, info.window_height
            ),
            egui::FontId::proportional(small),
            draw.right() - left - PAD,
        ),
        egui::FontId::proportional(small),
        ui.visuals().weak_text_color(),
    );
    ui.painter().text(
        egui::pos2(left, description_bottom),
        egui::Align2::LEFT_BOTTOM,
        truncate_ui_text(
            ui,
            &description,
            egui::FontId::proportional(small),
            draw.right() - left - PAD,
        ),
        egui::FontId::proportional(small),
        ui.visuals().weak_text_color(),
    );
    if response.clicked() {
        let mode = model.draft.pet.picker_mode;
        deskhud_ui::apply_pet_selection(&mut model.draft, info.id.to_string(), mode);
        model
            .draft
            .pet
            .apply_window_size(info.window_width, info.window_height);
    }
}

fn pet_preview_texture(ui: &Ui, info: &deskhud_engine::PetKindInfo) -> Option<egui::TextureHandle> {
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

fn pet_preview_rect(container: egui::Rect) -> egui::Rect {
    // Keep the preview window identical for every card. The pet artwork is
    // fitted inside it using its own standard window ratio below.
    container
}

fn paint_preview_frame(ui: &Ui, preview: egui::Rect, scale: f32) {
    ui.painter().rect(
        preview,
        CornerRadius::same((10.0 * scale).round() as u8),
        ui.visuals().faint_bg_color,
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
}

fn paint_preview_contain(ui: &Ui, stage: egui::Rect, texture: &egui::TextureHandle) {
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

fn truncate_ui_text(ui: &Ui, text: &str, font: egui::FontId, max_width: f32) -> String {
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

fn pet_catalog_text(
    catalogs: &CatalogStore,
    locale: Locale,
    id: &str,
    field: &str,
    fallback: &str,
) -> String {
    let key = format!("{id}.{field}");
    // Package metadata and config entries carry the stable PO msgid here.
    // Use the complete key as the fallback so a missing package catalog is
    // handled by the shared safe fallback instead of exposing the msgid.
    let fallback = if fallback == field || fallback.contains('.') {
        key.as_str()
    } else {
        fallback
    };
    catalogs.t(locale, &key, fallback).to_owned()
}

fn pet_tooltip(
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

fn tooltip_meta_row(ui: &mut Ui, locale: Locale, key: MessageKey, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(text_for_locale(locale, key))
                .font(fonts::scaled_font(ui, 0.82))
                .color(ui.visuals().weak_text_color()),
        );
        ui.label(RichText::new(value).font(fonts::scaled_font(ui, 0.86)));
    });
}

fn draw_hud(ui: &mut egui::Ui, registry: &Arc<EngineRegistry>, model: &mut SettingsModel) {
    ui.heading(
        RichText::new(text(model, MessageKey::SettingsNavHud)).font(fonts::scaled_font(ui, 1.71)),
    );
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
        RichText::new(text(model, MessageKey::HudMasterEnable)).font(fonts::scaled_font(ui, 1.07)),
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
                    RichText::new(plugin.display_name).font(fonts::scaled_font(ui, 1.0)),
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
                            model.draft.shell.ui_font_id =
                                crate::fonts::persistable_font_id(&face.font_id);
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
            ui.label(RichText::new(preview_text).font(egui::FontId::proportional(preview_size)));
        })),
    );
}

fn apply_default_font_face(model: &mut SettingsModel, family: &deskhud_ui::font::FontFamilyEntry) {
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

fn draw_placeholder(ui: &mut egui::Ui, model: &SettingsModel) {
    ui.heading(
        RichText::new(text_for_locale(model.draft.locale, model.tab.nav_message()))
            .font(fonts::scaled_font(ui, 1.71)),
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

fn footer_button(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    fill: Color32,
    _hover_fill: Color32,
) -> egui::Response {
    let button = egui::Button::new(RichText::new(label).font(fonts::scaled_font(ui, 1.0)))
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
        Locale::En | Locale::System | Locale::Custom(_) => deskhud_ui::i18n::t(Locale::En, key),
    }
}
