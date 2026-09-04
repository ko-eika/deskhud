//! HUD 视觉效果调整控件。

use super::*;
use crate::components;

pub(super) fn draw_effects_group(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    instance_id: &deskhud_engine::HudInstanceId,
    item: &HudRenderItem,
) -> bool {
    let mut changed = false;

    components::config_card(
        ui,
        Some(
            egui::RichText::new(deskhud_ui::i18n::t(
                prefs.locale,
                MessageKey::HudAdjustGlobalEffects,
            ))
            .strong()
            .into(),
        ),
        |ui| {
            changed |= draw_effect_detail_link_row(
                ui,
                prefs,
                instance_id,
                layout,
                MessageKey::HudAdjustWindowShadow,
                "shadow_enabled",
                item.shadow_enabled,
                ShadowTarget::Global,
                false,
            );
        },
        None,
    );
    ui.add_space(8.0);

    components::config_card(
        ui,
        Some(
            egui::RichText::new(deskhud_ui::i18n::t(
                prefs.locale,
                MessageKey::HudAdjustWindowEffects,
            ))
            .strong()
            .into(),
        ),
        |ui| {
            changed |= draw_effect_slider_config_row(
                ui,
                prefs,
                instance_id,
                "corner_radius",
                MessageKey::HudAdjustCornerRadius,
                item.corner_radius,
                HUD_CORNER_RADIUS_MAX,
                " px",
                true,
            );
            changed |= draw_effect_detail_link_row(
                ui,
                prefs,
                instance_id,
                layout,
                MessageKey::HudAdjustWindowShadow,
                "window_shadow_enabled",
                true,
                ShadowTarget::Window,
                true,
            );
            changed |= draw_effect_detail_link_row(
                ui,
                prefs,
                instance_id,
                layout,
                MessageKey::HudAdjustBorderEffects,
                "border_enabled",
                item.border_enabled,
                ShadowTarget::Border,
                true,
            );
            changed |= draw_effect_detail_link_row(
                ui,
                prefs,
                instance_id,
                layout,
                MessageKey::HudAdjustBackgroundEffects,
                "background_enabled",
                item.background_enabled,
                ShadowTarget::Background,
                false,
            );
        },
        None,
    );
    ui.add_space(8.0);

    components::config_card(
        ui,
        Some(
            egui::RichText::new(deskhud_ui::i18n::t(
                prefs.locale,
                MessageKey::HudAdjustContentEffects,
            ))
            .strong()
            .into(),
        ),
        |ui| {
            changed |= draw_effect_color_config_row(
                ui,
                prefs,
                instance_id,
                MessageKey::HudAdjustContentColor,
                item.content_color,
                ["content_red", "content_green", "content_blue"],
                true,
            );
            changed |= draw_effect_slider_config_row(
                ui,
                prefs,
                instance_id,
                "content_opacity",
                MessageKey::HudAdjustContentOpacity,
                item.content_opacity,
                1.0,
                "",
                true,
            );
            changed |= draw_effect_detail_link_row(
                ui,
                prefs,
                instance_id,
                layout,
                MessageKey::HudAdjustContentShadow,
                "content_shadow_enabled",
                true,
                ShadowTarget::Content,
                false,
            );
        },
        None,
    );
    changed
}

#[allow(clippy::too_many_arguments)]
fn draw_effect_detail_link_row(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    instance_id: &deskhud_engine::HudInstanceId,
    layout: &mut LayoutState,
    key: MessageKey,
    enable_name: &str,
    default_enabled: bool,
    target: ShadowTarget,
    show_divider: bool,
) -> bool {
    let mut enabled = prefs.hud.instance_visual_value(
        instance_id,
        enable_name,
        if default_enabled { 1.0 } else { 0.0 },
    ) >= 0.5;
    let before = enabled;
    let mode_name = match target {
        ShadowTarget::Window => Some("window_shadow_mode"),
        ShadowTarget::Content => Some("content_shadow_mode"),
        _ => None,
    };
    let mut mode_global =
        mode_name.is_some_and(|name| prefs.hud.instance_visual_value(instance_id, name, 0.0) < 0.5);
    let before_mode_global = mode_global;
    let mut open_target = None;
    components::config_row_with_divider(
        ui,
        deskhud_ui::i18n::t(prefs.locale, key),
        None::<egui::RichText>,
        show_divider,
        |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(42.0, 24.0), egui::Sense::hover());
            crate::components::toggle_switch_with_id(
                ui,
                rect,
                &mut enabled,
                ("effect-detail", instance_id.as_str(), enable_name),
            );
            ui.add_enabled_ui(enabled, |ui| {
                if mode_name.is_some() {
                    let custom_clicked = ui
                        .add_sized(
                            egui::vec2(78.0, ADJUST_ROW_HEIGHT),
                            egui::Button::new(deskhud_ui::i18n::t(
                                prefs.locale,
                                MessageKey::HudAdjustShadowCustom,
                            ))
                            .selected(!mode_global),
                        )
                        .clicked();
                    let global_clicked = ui
                        .add_sized(
                            egui::vec2(78.0, ADJUST_ROW_HEIGHT),
                            egui::Button::new(deskhud_ui::i18n::t(
                                prefs.locale,
                                MessageKey::HudAdjustShadowGlobal,
                            ))
                            .selected(mode_global),
                        )
                        .clicked();
                    if global_clicked {
                        mode_global = true;
                        open_target = Some(ShadowTarget::Global);
                    } else if custom_clicked {
                        mode_global = false;
                        open_target = Some(target);
                    }
                } else if ui
                    .add_sized(
                        egui::vec2(166.0, ADJUST_ROW_HEIGHT),
                        egui::Button::new(deskhud_ui::i18n::t(
                            prefs.locale,
                            MessageKey::HudAdjustOpenSettings,
                        )),
                    )
                    .clicked()
                {
                    open_target = Some(target);
                }
            });
        },
    );
    if before != enabled {
        prefs.hud.set_instance_visual_value(
            instance_id,
            enable_name,
            if enabled { 1.0 } else { 0.0 },
        );
    }
    if before_mode_global != mode_global
        && let Some(mode_name) = mode_name
    {
        prefs.hud.set_instance_visual_value(
            instance_id,
            mode_name,
            if mode_global { 0.0 } else { 1.0 },
        );
    }
    if let Some(open_target) = open_target {
        if matches!(open_target, ShadowTarget::Window) {
            prefs
                .hud
                .set_instance_visual_value(instance_id, "window_shadow_mode", 1.0);
        } else if matches!(open_target, ShadowTarget::Content) {
            prefs
                .hud
                .set_instance_visual_value(instance_id, "content_shadow_mode", 1.0);
        }
        layout.shadow_open = true;
        layout.shadow_target = Some(open_target);
    }
    before != enabled || before_mode_global != mode_global || open_target.is_some()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_effect_slider_config_row(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    instance_id: &deskhud_engine::HudInstanceId,
    name: &str,
    message: MessageKey,
    default: f32,
    display_scale: f32,
    suffix: &str,
    show_divider: bool,
) -> bool {
    let mut value = prefs.hud.instance_visual_value(instance_id, name, default);
    let mut shown = value * display_scale;
    let mut changed = false;
    components::config_row_with_divider(
        ui,
        deskhud_ui::i18n::t(prefs.locale, message),
        None::<egui::RichText>,
        show_divider,
        |ui| {
            let input_changed = ui
                .add_sized(
                    egui::vec2(ADJUST_VALUE_WIDTH, ADJUST_ROW_HEIGHT),
                    egui::DragValue::new(&mut shown)
                        .fixed_decimals(2)
                        .range(0.0..=display_scale)
                        .speed((display_scale / 100.0).max(0.01))
                        .suffix(suffix),
                )
                .changed();
            let slider_changed = ui
                .add_sized(
                    egui::vec2(
                        216.0 - ADJUST_VALUE_WIDTH - ui.spacing().item_spacing.x,
                        ADJUST_ROW_HEIGHT,
                    ),
                    egui::Slider::new(&mut value, 0.0..=1.0)
                        .step_by(0.01 / display_scale as f64)
                        .handle_shape(egui::style::HandleShape::Circle)
                        .show_value(false),
                )
                .changed();
            if input_changed {
                value = (shown / display_scale.max(f32::EPSILON)).clamp(0.0, 1.0);
            }
            changed = slider_changed || input_changed;
        },
    );
    if changed {
        prefs
            .hud
            .set_instance_visual_value(instance_id, name, value);
    }
    changed
}

pub(super) fn draw_effect_color_config_row(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    instance_id: &deskhud_engine::HudInstanceId,
    message: MessageKey,
    mut color: [u8; 3],
    names: [&str; 3],
    show_divider: bool,
) -> bool {
    for (channel, name) in names.iter().enumerate() {
        color[channel] =
            (prefs
                .hud
                .instance_visual_value(instance_id, name, color[channel] as f32 / 255.0)
                * 255.0)
                .round() as u8;
    }
    let mut changed = false;
    components::config_row_with_divider(
        ui,
        deskhud_ui::i18n::t(prefs.locale, message),
        None::<egui::RichText>,
        show_divider,
        |ui| {
            changed = draw_hex_color_control(
                ui,
                &mut color,
                ui.make_persistent_id(("hud-effect-color", instance_id.as_str(), names[0])),
            );
        },
    );
    if changed {
        for (name, channel) in names.into_iter().zip(color) {
            prefs
                .hud
                .set_instance_visual_value(instance_id, name, channel as f32 / 255.0);
        }
    }
    changed
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub(super) fn draw_effect_slider_row(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    instance_id: &deskhud_engine::HudInstanceId,
    name: &str,
    message: MessageKey,
    default: f32,
    display_scale: f32,
    suffix: &str,
) -> bool {
    let mut changed = false;
    let (label_rect, control_rect, value_rect) = allocate_effect_row(ui);
    draw_effect_label(ui, label_rect, deskhud_ui::i18n::t(prefs.locale, message));
    let mut value = prefs.hud.instance_visual_value(instance_id, name, default);
    let mut slider_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("hud-effect-slider", instance_id.as_str(), name))
            .max_rect(control_rect)
            .layout(egui::Layout::centered_and_justified(
                egui::Direction::TopDown,
            )),
    );
    slider_ui.spacing_mut().slider_width = control_rect.width();
    let slider_changed = slider_ui
        .add(
            egui::Slider::new(&mut value, 0.0..=1.0)
                .step_by(0.01 / display_scale as f64)
                .handle_shape(egui::style::HandleShape::Circle)
                .show_value(false),
        )
        .changed();
    let mut shown = value * display_scale;
    let mut value_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("hud-effect-value", instance_id.as_str(), name))
            .max_rect(value_rect)
            .layout(egui::Layout::centered_and_justified(
                egui::Direction::TopDown,
            )),
    );
    let input_changed = value_ui
        .add(
            egui::DragValue::new(&mut shown)
                .fixed_decimals(2)
                .range(0.0..=display_scale)
                .speed((display_scale / 100.0).max(0.01))
                .suffix(suffix),
        )
        .changed();
    if input_changed {
        value = (shown / display_scale.max(f32::EPSILON)).clamp(0.0, 1.0);
    }
    if slider_changed || input_changed {
        prefs
            .hud
            .set_instance_visual_value(instance_id, name, value);
        changed = true;
    }
    ui.add_space(ADJUST_ROW_GAP);
    changed
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub(super) fn draw_effect_color_row(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    instance_id: &deskhud_engine::HudInstanceId,
    message: MessageKey,
    mut color: [u8; 3],
    names: [&str; 3],
) -> bool {
    let mut changed = false;
    for (channel, name) in names.iter().enumerate() {
        color[channel] =
            (prefs
                .hud
                .instance_visual_value(instance_id, name, color[channel] as f32 / 255.0)
                * 255.0)
                .round() as u8;
    }
    let (label_rect, control_rect, value_rect) = allocate_effect_row(ui);
    draw_effect_label(ui, label_rect, deskhud_ui::i18n::t(prefs.locale, message));
    let mut color_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("hud-effect-color", instance_id.as_str(), names[0]))
            .max_rect(control_rect)
            .layout(egui::Layout::centered_and_justified(
                egui::Direction::TopDown,
            )),
    );
    color_ui.spacing_mut().interact_size = control_rect.size();
    let color_input_id =
        ui.make_persistent_id(("hud-effect-color-text", instance_id.as_str(), names[0]));
    let picker_changed = color_ui.color_edit_button_srgb(&mut color).changed();
    if picker_changed {
        for (name, channel) in names.into_iter().zip(color) {
            prefs
                .hud
                .set_instance_visual_value(instance_id, name, channel as f32 / 255.0);
        }
        changed = true;
    }
    let canonical = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
    let mut input = ui.ctx().data_mut(|data| {
        data.get_temp::<String>(color_input_id)
            .unwrap_or_else(|| canonical.clone())
    });
    if picker_changed {
        input.clone_from(&canonical);
    }
    let mut value_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("hud-effect-color-value", instance_id.as_str(), names[0]))
            .max_rect(value_rect)
            .layout(egui::Layout::centered_and_justified(
                egui::Direction::TopDown,
            )),
    );
    let response = value_ui.add(
        egui::TextEdit::singleline(&mut input)
            .id(color_input_id.with("edit"))
            .font(egui::TextStyle::Monospace)
            .horizontal_align(egui::Align::Center)
            .vertical_align(egui::Align::Center)
            .desired_width(value_rect.width()),
    );
    if response.changed()
        && let Some(parsed) = parse_hex_color(&input)
    {
        color = parsed;
        for (name, channel) in names.into_iter().zip(color) {
            prefs
                .hud
                .set_instance_visual_value(instance_id, name, channel as f32 / 255.0);
        }
        changed = true;
    }
    if response.lost_focus() && parse_hex_color(&input).is_none() {
        input = canonical;
    }
    ui.ctx()
        .data_mut(|data| data.insert_temp(color_input_id, input));
    ui.add_space(ADJUST_ROW_GAP);
    changed
}
