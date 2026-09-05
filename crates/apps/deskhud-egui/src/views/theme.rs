//! Application-wide egui theme tokens.

use deskhud_engine::ThemePalette;
use deskhud_ui::{SystemTheme, UiTheme, resolve_theme};
use egui::{Color32, Context, Stroke};

/// Applies the application's resolved theme to an egui context.
pub(crate) fn apply(ctx: &Context, theme: UiTheme, system_theme: Option<SystemTheme>) {
    let system_theme = system_theme.or_else(|| {
        ctx.system_theme().map(|theme| match theme {
            egui::Theme::Dark => SystemTheme::Dark,
            egui::Theme::Light => SystemTheme::Light,
        })
    });
    let dark = matches!(resolve_theme(theme, system_theme), SystemTheme::Dark);
    ctx.set_theme(if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let palette = if dark {
        ThemePalette::dark()
    } else {
        ThemePalette::light()
    };
    visuals.override_text_color = Some(to_color(palette.text));
    visuals.weak_text_color = Some(to_color(palette.muted_text));
    visuals.faint_bg_color = to_color(palette.surface_alt);
    visuals.extreme_bg_color = to_color(palette.background);
    visuals.panel_fill = to_color(palette.surface);
    visuals.window_fill = to_color(palette.surface);
    visuals.window_stroke = Stroke::new(1.0, to_color(palette.border));
    // Keep text fields on the same recessed control surface as dropdowns and
    // switches instead of falling back to `extreme_bg_color`.
    visuals.text_edit_bg_color = Some(to_color(palette.control));
    visuals.selection.bg_fill = to_color(palette.selection);
    visuals.selection.stroke = Stroke::new(1.0, to_color(palette.selection_text));
    set_widget_colors(&mut visuals, palette);
    visuals.button_frame = true;
    visuals.collapsing_header_frame = false;
    visuals.disabled_alpha = 0.48;
    ctx.set_visuals(visuals);
}

/// Resolves the active egui visuals into the renderer-neutral engine palette.
pub(crate) fn palette(visuals: &egui::Visuals) -> ThemePalette {
    let color = |color: Color32| {
        let [red, green, blue, alpha] = color.to_array();
        deskhud_engine::OverlayColor {
            red,
            green,
            blue,
            alpha,
        }
    };
    let base = if visuals.dark_mode {
        ThemePalette::dark()
    } else {
        ThemePalette::light()
    };
    ThemePalette {
        accent: color(visuals.selection.bg_fill),
        accent_hover: color(visuals.selection.bg_fill),
        accent_active: color(visuals.selection.bg_fill),
        background: color(visuals.extreme_bg_color),
        surface: color(visuals.window_fill()),
        surface_alt: color(visuals.faint_bg_color),
        control: color(visuals.widgets.inactive.bg_fill),
        surface_hover: color(visuals.widgets.hovered.bg_fill),
        surface_active: color(visuals.widgets.active.bg_fill),
        border: color(visuals.widgets.noninteractive.bg_stroke.color),
        divider: color(visuals.window_stroke.color),
        focus: color(visuals.selection.stroke.color),
        text: color(visuals.text_color()),
        muted_text: color(visuals.weak_text_color()),
        disabled_text: color(visuals.widgets.noninteractive.fg_stroke.color),
        selection: color(visuals.selection.bg_fill),
        text_on_accent: color(visuals.selection.stroke.color),
        selection_text: color(visuals.selection.stroke.color),
        info: base.info,
        success: base.success,
        warning: base.warning,
        danger: base.danger,
        shadow: base.shadow,
    }
}

fn set_widget_colors(visuals: &mut egui::Visuals, palette: ThemePalette) {
    let stroke = Stroke::new(1.0, to_color(palette.border));
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.fg_stroke = Stroke::new(1.0, to_color(palette.text));
        widget.bg_stroke = stroke;
    }
    visuals.widgets.inactive.bg_fill = to_color(palette.control);
    visuals.widgets.inactive.weak_bg_fill = to_color(palette.control);
    visuals.widgets.hovered.bg_fill = to_color(palette.surface_hover);
    visuals.widgets.hovered.weak_bg_fill = to_color(palette.surface_hover);
    visuals.widgets.active.bg_fill = to_color(palette.surface_active);
    visuals.widgets.active.weak_bg_fill = to_color(palette.surface_active);
    visuals.widgets.open.bg_fill = to_color(palette.surface_hover);
    visuals.widgets.open.weak_bg_fill = to_color(palette.surface_hover);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, to_color(palette.text));
    visuals.widgets.noninteractive.bg_stroke = stroke;
}

fn to_color(color: deskhud_engine::OverlayColor) -> Color32 {
    Color32::from_rgba_unmultiplied(color.red, color.green, color.blue, color.alpha)
}
