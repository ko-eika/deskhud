//! HUD 阴影设置及预览。

use super::effects::{draw_custom_color_config_row, draw_effect_slider_config_row};
use super::*;
use crate::components;
use crate::views::hud::drawing::frame::paint_window_shadow;

#[allow(dead_code)]
pub(super) struct ShadowControlRow<'a> {
    pub(super) instance_id: &'a deskhud_engine::HudInstanceId,
    pub(super) global: bool,
    pub(super) master: bool,
    pub(super) target: ShadowTarget,
    pub(super) preview: Option<(f32, f32, f32, f32, [u8; 3])>,
}

#[allow(dead_code)]
pub(super) fn draw_shadow_control_row(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    row: ShadowControlRow<'_>,
) -> (bool, Option<ShadowTarget>) {
    let ShadowControlRow {
        instance_id,
        global,
        master,
        target,
        preview,
    } = row;
    let label_key = MessageKey::HudAdjustWindowShadow;
    let (mode_name, enable_name) = match target {
        ShadowTarget::Window => ("window_shadow_mode", "window_shadow_enabled"),
        ShadowTarget::Content => ("content_shadow_mode", "content_shadow_enabled"),
        ShadowTarget::Global => ("shadow_enabled", "shadow_enabled"),
        ShadowTarget::Border => ("border_enabled", "border_enabled"),
        ShadowTarget::Background => ("background_enabled", "background_enabled"),
    };
    let mut mode_global = global;
    let mut enabled = if master {
        global
    } else {
        prefs
            .hud
            .instance_visual_value(instance_id, enable_name, 1.0)
            >= 0.5
    };
    let before_mode_global = mode_global;
    let before_enabled = enabled;
    let right_width = if master { 42.0 } else { 82.0 };
    let row_width = ui.available_width();
    let (row_rect, _) = ui.allocate_exact_size(
        egui::vec2(row_width, ADJUST_ROW_HEIGHT),
        egui::Sense::hover(),
    );
    let label_rect = egui::Rect::from_min_size(
        row_rect.min + egui::vec2(ADJUST_LABEL_INDENT, 0.0),
        egui::vec2(ADJUST_LABEL_WIDTH - ADJUST_LABEL_INDENT, ADJUST_ROW_HEIGHT),
    );
    let value_rect = egui::Rect::from_min_size(
        egui::pos2(row_rect.right() - right_width, row_rect.top()),
        egui::vec2(right_width, ADJUST_ROW_HEIGHT),
    );
    let control_rect = egui::Rect::from_min_max(
        egui::pos2(
            label_rect.right() + ui.spacing().item_spacing.x,
            row_rect.top(),
        ),
        egui::pos2(
            value_rect.left() - ui.spacing().item_spacing.x,
            row_rect.bottom(),
        ),
    );
    draw_effect_label(ui, label_rect, deskhud_ui::i18n::t(prefs.locale, label_key));
    let clicked = if master {
        let response = ui
            .interact(
                control_rect,
                ui.make_persistent_id(("hud-shadow-preview", instance_id.as_str())),
                egui::Sense::click(),
            )
            .on_hover_text(deskhud_ui::i18n::t(
                prefs.locale,
                MessageKey::HudAdjustShadowSettings,
            ));
        if let Some((opacity, blur, distance, angle, color)) = preview {
            draw_shadow_preview_inline(ui, control_rect, opacity, blur, distance, angle, color);
        }
        if response.clicked() {
            Some(ShadowTarget::Global)
        } else {
            None
        }
    } else {
        let split = control_rect.width() * 0.5;
        let global_rect = egui::Rect::from_min_max(
            control_rect.min,
            egui::pos2(control_rect.left() + split, control_rect.bottom()),
        );
        let custom_rect = egui::Rect::from_min_max(
            egui::pos2(
                control_rect.left() + split + ui.spacing().item_spacing.x,
                control_rect.top(),
            ),
            control_rect.max,
        );
        let global_clicked = ui
            .put(
                global_rect,
                egui::Button::new(deskhud_ui::i18n::t(
                    prefs.locale,
                    MessageKey::HudAdjustShadowGlobal,
                ))
                .selected(mode_global),
            )
            .clicked();
        let custom_clicked = ui
            .put(
                custom_rect,
                egui::Button::new(deskhud_ui::i18n::t(
                    prefs.locale,
                    MessageKey::HudAdjustShadowCustom,
                ))
                .selected(!mode_global),
            )
            .clicked();
        if global_clicked {
            mode_global = true;
        } else if custom_clicked {
            mode_global = false;
        }
        if global_clicked {
            Some(ShadowTarget::Global)
        } else if custom_clicked {
            Some(target)
        } else {
            None
        }
    };
    let mut right_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(value_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    if !master {
        right_ui.label(deskhud_ui::i18n::t(
            prefs.locale,
            MessageKey::HudAdjustShadowGlobal,
        ));
    }
    let switch_rect = egui::Rect::from_min_size(
        egui::pos2(value_rect.right() - 42.0, value_rect.top() + 4.0),
        egui::vec2(42.0, 24.0),
    );
    crate::components::toggle_switch_with_id(
        &mut right_ui,
        switch_rect,
        &mut enabled,
        ("shadow", instance_id.as_str(), enable_name),
    );
    let mode_changed = !master && before_mode_global != mode_global;
    let enable_changed = before_enabled != enabled;
    if mode_changed {
        prefs.hud.set_instance_visual_value(
            instance_id,
            mode_name,
            if mode_global { 0.0 } else { 1.0 },
        );
    }
    if enable_changed {
        prefs.hud.set_instance_visual_value(
            instance_id,
            enable_name,
            if enabled { 1.0 } else { 0.0 },
        );
    }
    ui.add_space(ADJUST_ROW_GAP);
    (mode_changed || enable_changed, clicked)
}

#[allow(dead_code)]
fn draw_shadow_preview_inline(
    ui: &egui::Ui,
    rect: egui::Rect,
    opacity: f32,
    blur: f32,
    distance: f32,
    angle: f32,
    color: [u8; 3],
) {
    let painter = ui.painter();
    // Keep the preview's visible height aligned with position/size inputs;
    // only inset horizontally so the preview still has breathing room beside
    // the label and switch.
    let panel = rect.shrink2(egui::vec2(4.0, 0.0));
    if opacity > f32::EPSILON {
        paint_window_shadow(painter, panel, 6.0, opacity, blur, distance, angle, color);
    }
    painter.rect_filled(panel, 6.0, ui.visuals().window_fill());
    painter.text(
        panel.center(),
        egui::Align2::CENTER_CENTER,
        "Aa",
        egui::FontId::proportional(16.0),
        ui.visuals().text_color(),
    );
}

pub(super) fn draw_shadow_window(
    ui: &egui::Ui,
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    item: &HudRenderItem,
    instance_id: &deskhud_engine::HudInstanceId,
    target: ShadowTarget,
) -> bool {
    if matches!(target, ShadowTarget::Border | ShadowTarget::Background) {
        return draw_surface_effect_window(ui, layout, prefs, item, instance_id, target);
    }
    let mut open = layout.shadow_open;
    let mut changed = false;
    let position = layout
        .activity_size
        .map(|size| egui::pos2((size.x - EDITOR_PANEL_WIDTH - 336.0).max(24.0), 32.0))
        .unwrap_or(egui::pos2(24.0, 32.0));
    let (
        title_key,
        opacity_name,
        blur_name,
        distance_name,
        angle_name,
        color_names,
        color_enabled_name,
        opacity,
        blur,
        distance,
        angle,
        color,
    ) = match target {
        ShadowTarget::Global => (
            MessageKey::HudAdjustGlobalShadow,
            "shadow_opacity",
            "shadow_blur",
            "shadow_distance",
            "shadow_angle",
            ["shadow_red", "shadow_green", "shadow_blue"],
            "shadow_color_enabled",
            item.window_shadow,
            item.window_shadow_blur,
            item.window_shadow_distance,
            item.window_shadow_angle,
            item.window_shadow_color,
        ),
        ShadowTarget::Window => (
            MessageKey::HudAdjustCustomShadow,
            "window_shadow",
            "window_shadow_blur",
            "window_shadow_distance",
            "window_shadow_angle",
            [
                "window_shadow_red",
                "window_shadow_green",
                "window_shadow_blue",
            ],
            "window_shadow_color_enabled",
            item.window_custom_shadow,
            item.window_custom_shadow_blur,
            item.window_custom_shadow_distance,
            item.window_custom_shadow_angle,
            item.window_custom_shadow_color,
        ),
        ShadowTarget::Content => (
            MessageKey::HudAdjustCustomShadow,
            "content_shadow",
            "content_shadow_blur",
            "content_shadow_distance",
            "content_shadow_angle",
            [
                "content_shadow_red",
                "content_shadow_green",
                "content_shadow_blue",
            ],
            "content_shadow_color_enabled",
            item.content_custom_shadow,
            item.content_custom_shadow_blur,
            item.content_custom_shadow_distance,
            item.content_custom_shadow_angle,
            item.content_custom_shadow_color,
        ),
        ShadowTarget::Border | ShadowTarget::Background => {
            unreachable!("surface effect target was handled before shadow settings")
        }
    };
    egui::Window::new(egui::RichText::new(deskhud_ui::i18n::t(prefs.locale, title_key)).strong())
        .id(egui::Id::new(("hud-shadow-window", layout.adjust_session)))
        .default_pos(position)
        .default_width(EDITOR_PANEL_WIDTH)
        .min_width(EDITOR_PANEL_WIDTH)
        .max_width(EDITOR_PANEL_WIDTH)
        .resizable(false)
        .collapsible(false)
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            components::config_card(
                ui,
                None,
                |ui| {
                    changed |= draw_custom_color_config_row(
                        ui,
                        prefs,
                        instance_id,
                        MessageKey::HudAdjustShadowColor,
                        color,
                        color_names,
                        color_enabled_name,
                        true,
                    );
                    changed |= draw_effect_slider_config_row(
                        ui,
                        prefs,
                        instance_id,
                        opacity_name,
                        MessageKey::HudAdjustShadowOpacity,
                        opacity,
                        1.0,
                        "",
                        true,
                    );
                    changed |= draw_effect_slider_config_row(
                        ui,
                        prefs,
                        instance_id,
                        blur_name,
                        MessageKey::HudAdjustShadowBlur,
                        blur,
                        24.0,
                        " px",
                        true,
                    );
                    changed |= draw_effect_slider_config_row(
                        ui,
                        prefs,
                        instance_id,
                        distance_name,
                        MessageKey::HudAdjustShadowDistance,
                        distance,
                        12.0,
                        " px",
                        true,
                    );
                    changed |= draw_effect_slider_config_row(
                        ui,
                        prefs,
                        instance_id,
                        angle_name,
                        MessageKey::HudAdjustShadowAngle,
                        angle,
                        360.0,
                        "°",
                        false,
                    );
                },
                None,
            );
        });
    layout.shadow_open = open;
    changed
}

fn draw_surface_effect_window(
    ui: &egui::Ui,
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    item: &HudRenderItem,
    instance_id: &deskhud_engine::HudInstanceId,
    target: ShadowTarget,
) -> bool {
    let mut open = layout.shadow_open;
    let mut changed = false;
    let (title, window_id) = match target {
        ShadowTarget::Border => (MessageKey::HudAdjustBorderEffects, "hud-border-window"),
        ShadowTarget::Background => (
            MessageKey::HudAdjustBackgroundEffects,
            "hud-background-window",
        ),
        _ => unreachable!("surface effect window requires border or background"),
    };
    let position = layout
        .activity_size
        .map(|size| egui::pos2((size.x - EDITOR_PANEL_WIDTH - 336.0).max(24.0), 32.0))
        .unwrap_or(egui::pos2(24.0, 32.0));
    egui::Window::new(egui::RichText::new(deskhud_ui::i18n::t(prefs.locale, title)).strong())
        .id(egui::Id::new((window_id, layout.adjust_session)))
        .default_pos(position)
        .default_width(EDITOR_PANEL_WIDTH)
        .min_width(EDITOR_PANEL_WIDTH)
        .max_width(EDITOR_PANEL_WIDTH)
        .resizable(false)
        .collapsible(false)
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            components::config_card(
                ui,
                None,
                |ui| match target {
                    ShadowTarget::Border => {
                        changed |= draw_custom_color_config_row(
                            ui,
                            prefs,
                            instance_id,
                            MessageKey::HudAdjustBorderColor,
                            item.border_color,
                            ["border_red", "border_green", "border_blue"],
                            "border_color_enabled",
                            true,
                        );
                        changed |= draw_effect_slider_config_row(
                            ui,
                            prefs,
                            instance_id,
                            "border_width",
                            MessageKey::HudAdjustBorderWidth,
                            item.border_width,
                            HUD_BORDER_WIDTH_MAX,
                            " px",
                            true,
                        );
                        changed |= draw_effect_slider_config_row(
                            ui,
                            prefs,
                            instance_id,
                            "border_opacity",
                            MessageKey::HudAdjustBorderOpacity,
                            item.border_opacity,
                            1.0,
                            "",
                            false,
                        );
                    }
                    ShadowTarget::Background => {
                        changed |= draw_background_color_config_row(ui, prefs, instance_id, item);
                        changed |= draw_effect_slider_config_row(
                            ui,
                            prefs,
                            instance_id,
                            "background_opacity",
                            MessageKey::HudAdjustBackgroundOpacity,
                            item.background_opacity,
                            1.0,
                            "",
                            true,
                        );
                        changed |= draw_effect_slider_config_row(
                            ui,
                            prefs,
                            instance_id,
                            "background_blur",
                            MessageKey::HudAdjustBackgroundBlur,
                            item.background_blur,
                            1.0,
                            "",
                            false,
                        );
                    }
                    _ => unreachable!("surface effect window requires border or background"),
                },
                None,
            );
        });
    layout.shadow_open = open;
    changed
}

fn draw_background_color_config_row(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    instance_id: &deskhud_engine::HudInstanceId,
    item: &HudRenderItem,
) -> bool {
    let default_color = item
        .layers
        .iter()
        .flat_map(|layer| layer.frame.visuals.iter())
        .find_map(|visual| match visual {
            deskhud_engine::HudVisual::Panel { color, .. } => Some([color[0], color[1], color[2]]),
            _ => None,
        })
        .unwrap_or([0; 3]);
    let mut color = std::array::from_fn(|channel| {
        let names = ["background_red", "background_green", "background_blue"];
        (prefs.hud.instance_visual_value(
            instance_id,
            names[channel],
            default_color[channel] as f32 / 255.0,
        ) * 255.0)
            .round() as u8
    });
    let mut custom = prefs
        .hud
        .instance_visual_value(instance_id, "background_color_enabled", 0.0)
        >= 0.5;
    let before_custom = custom;
    let mut color_changed = false;
    components::config_row_with_divider(
        ui,
        deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustBorderColor),
        None::<egui::RichText>,
        true,
        |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(42.0, 24.0), egui::Sense::hover());
            components::toggle_switch_with_id(
                ui,
                rect,
                &mut custom,
                ("background-color", instance_id.as_str()),
            );
            ui.add_space(8.0);
            ui.add_enabled_ui(custom, |ui| {
                color_changed = draw_hex_color_control(
                    ui,
                    &mut color,
                    ui.make_persistent_id(("hud-background-color", instance_id.as_str())),
                );
            });
        },
    );
    if before_custom != custom {
        prefs.hud.set_instance_visual_value(
            instance_id,
            "background_color_enabled",
            if custom { 1.0 } else { 0.0 },
        );
    }
    if color_changed {
        for (name, channel) in ["background_red", "background_green", "background_blue"]
            .into_iter()
            .zip(color)
        {
            prefs
                .hud
                .set_instance_visual_value(instance_id, name, channel as f32 / 255.0);
        }
    }
    before_custom != custom || color_changed
}
