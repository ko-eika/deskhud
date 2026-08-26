//! Application-wide egui theme tokens.

use deskhud_ui::{SystemTheme, UiTheme, resolve_theme};
use egui::{Color32, Context, Stroke};

/// Applies the application's resolved theme to an egui context.
pub(crate) fn apply(ctx: &Context, theme: UiTheme) {
    let dark = matches!(
        resolve_theme(theme, Some(SystemTheme::Dark)),
        SystemTheme::Dark
    );
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    if dark {
        visuals.override_text_color = Some(Color32::from_rgb(232, 235, 241));
        visuals.weak_text_color = Some(Color32::from_rgb(164, 171, 182));
        visuals.faint_bg_color = Color32::from_rgb(38, 41, 47);
        visuals.extreme_bg_color = Color32::from_rgb(26, 28, 33);
        visuals.panel_fill = Color32::from_rgb(30, 33, 38);
        visuals.window_fill = Color32::from_rgb(30, 33, 38);
        visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(67, 72, 82));
        visuals.selection.bg_fill = Color32::from_rgb(42, 119, 224);
        visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
        set_widget_colors(
            &mut visuals,
            [49, 53, 61],
            [60, 68, 82],
            [41, 45, 52],
            [221, 226, 235],
            [67, 72, 82],
        );
    } else {
        visuals.override_text_color = Some(Color32::from_rgb(42, 46, 54));
        visuals.weak_text_color = Some(Color32::from_rgb(102, 109, 121));
        visuals.faint_bg_color = Color32::from_rgb(243, 245, 248);
        visuals.extreme_bg_color = Color32::from_rgb(252, 252, 253);
        visuals.panel_fill = Color32::from_rgb(247, 248, 250);
        visuals.window_fill = Color32::from_rgb(252, 252, 253);
        visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(202, 207, 216));
        visuals.selection.bg_fill = Color32::from_rgb(126, 194, 244);
        visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(25, 77, 119));
        set_widget_colors(
            &mut visuals,
            [238, 241, 245],
            [226, 237, 248],
            [232, 235, 240],
            [42, 46, 54],
            [202, 207, 216],
        );
    }
    visuals.button_frame = true;
    visuals.collapsing_header_frame = false;
    visuals.disabled_alpha = 0.48;
    ctx.set_visuals(visuals);
}

fn set_widget_colors(
    visuals: &mut egui::Visuals,
    inactive: [u8; 3],
    hovered: [u8; 3],
    active: [u8; 3],
    text: [u8; 3],
    border: [u8; 3],
) {
    let color = |rgb: [u8; 3]| Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    let stroke = Stroke::new(1.0, color(border));
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.fg_stroke = Stroke::new(1.0, color(text));
        widget.bg_stroke = stroke;
    }
    visuals.widgets.inactive.bg_fill = color(inactive);
    visuals.widgets.inactive.weak_bg_fill = color(inactive);
    visuals.widgets.hovered.bg_fill = color(hovered);
    visuals.widgets.hovered.weak_bg_fill = color(hovered);
    visuals.widgets.active.bg_fill = color(active);
    visuals.widgets.active.weak_bg_fill = color(active);
    visuals.widgets.open.bg_fill = color(hovered);
    visuals.widgets.open.weak_bg_fill = color(hovered);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, color(text));
    visuals.widgets.noninteractive.bg_stroke = stroke;
}
