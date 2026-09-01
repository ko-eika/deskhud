//! Settings 页面布局与控件绘制。

use std::sync::Arc;

use crate::{components, fonts};
use deskhud_engine::EngineRegistry;
use deskhud_ui::{
    CatalogStore, Locale, MessageKey, SettingsCommand, SettingsModel, SettingsTab, UiPreferences,
};
use egui::{
    Align, CentralPanel, Color32, CornerRadius, Frame, Layout, Margin, Panel, Pos2, RichText,
    ScrollArea, Sense, Ui, Vec2,
};

#[path = "about.rs"]
mod about;
#[path = "common.rs"]
mod common;
#[path = "font.rs"]
mod font;
#[path = "general.rs"]
mod general;
#[path = "hud.rs"]
mod hud;
#[path = "hud_list.rs"]
mod hud_list;
#[path = "performance.rs"]
mod performance;
#[path = "pet.rs"]
mod pet;
#[path = "pet_config.rs"]
mod pet_config;
#[path = "pet_global.rs"]
mod pet_global;
#[path = "pet_picker.rs"]
mod pet_picker;

pub(super) use common::{
    catalog_text, draw_empty, paint_preview_contain, paint_preview_frame, tooltip_meta_row,
    truncate_ui_text,
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
            draw_page_header(ui, model);
            ui.add_space(14.0);
            ScrollArea::vertical()
                .id_salt("settings_content_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| match model.tab {
                    SettingsTab::General => general::draw(ui, model, catalogs),
                    SettingsTab::Performance => performance::draw(ui, model),
                    SettingsTab::Pet => pet::draw(ui, registry, catalogs, model),
                    SettingsTab::Hud => hud::draw(ui, registry, catalogs, model),
                    SettingsTab::About => about::draw(ui, model),
                });
        });

    (false, applied_preferences)
}

fn draw_page_header(ui: &mut Ui, model: &SettingsModel) {
    ui.heading(
        RichText::new(text(model, model.tab.nav_message())).font(fonts::scaled_font(ui, 1.71)),
    );
    let intro = match model.tab {
        SettingsTab::General => MessageKey::SettingsGeneralIntro,
        SettingsTab::Performance => MessageKey::SettingsPerformanceIntro,
        SettingsTab::Pet => MessageKey::SettingsPetIntro,
        SettingsTab::Hud => MessageKey::HudSettingsIntro,
        SettingsTab::About => MessageKey::SettingsAboutIntro,
    };
    ui.add_space(8.0);
    ui.label(RichText::new(text(model, intro)).color(ui.visuals().weak_text_color()));
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
    font::apply_default_font_face(model, family);
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
