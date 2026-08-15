//! 应用主题：浅色 / 深色 / 跟随系统（宠窗保持透明底）。

use deskhud_ui::UiTheme;
use egui::{Color32, Context, Shadow, Stroke, Theme, ThemePreference, Visuals};

/// 按偏好配置 Light/Dark 视觉（透明宠窗）并设置 ThemePreference。
pub fn apply(ctx: &Context, theme: UiTheme) {
    configure_transparent_visuals(ctx);
    ctx.set_theme(preference(theme));
}

fn preference(theme: UiTheme) -> ThemePreference {
    match theme {
        UiTheme::System => ThemePreference::System,
        UiTheme::Light => ThemePreference::Light,
        UiTheme::Dark => ThemePreference::Dark,
    }
}

fn configure_transparent_visuals(ctx: &Context) {
    for theme in [Theme::Light, Theme::Dark] {
        let prev = (*ctx.style_of(theme)).clone();
        let mut visuals = match theme {
            Theme::Light => Visuals::light(),
            Theme::Dark => Visuals::dark(),
        };
        visuals.panel_fill = Color32::TRANSPARENT;
        visuals.window_fill = Color32::TRANSPARENT;
        visuals.extreme_bg_color = Color32::TRANSPARENT;
        visuals.popup_shadow = Shadow::NONE;
        visuals.window_stroke = Stroke::NONE;
        ctx.set_visuals_of(theme, visuals);

        let mut style = (*ctx.style_of(theme)).clone();
        // 保留已应用的字号等 text_styles
        style.text_styles = prev.text_styles;
        style.visuals.panel_fill = Color32::TRANSPARENT;
        style.visuals.window_fill = Color32::TRANSPARENT;
        style.visuals.extreme_bg_color = Color32::TRANSPARENT;
        style.visuals.popup_shadow = Shadow::NONE;
        style.visuals.window_stroke = Stroke::NONE;
        ctx.set_style_of(theme, style);
    }
}
