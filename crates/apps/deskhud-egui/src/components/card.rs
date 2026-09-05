//! Settings card container.
#![allow(clippy::type_complexity)]

use egui::{
    Align, CornerRadius, FontId, Frame, Layout, Margin, RichText, Stroke, TextureHandle, Ui, Vec2,
};

/// Draws a rounded card with a theme-aware, low-contrast border.
pub(crate) fn section_card(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let border = ui
        .visuals()
        .widgets
        .noninteractive
        .bg_stroke
        .color
        .gamma_multiply(if ui.visuals().dark_mode { 0.62 } else { 0.9 });
    Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(16, 12))
        .show(ui, add);
}

/// Draws a configuration card with optional header/footer and content.
pub(crate) fn config_card(
    ui: &mut Ui,
    title: Option<egui::WidgetText>,
    add_content: impl FnOnce(&mut Ui),
    add_footer: Option<Box<dyn FnOnce(&mut Ui)>>,
) {
    let has_header = title.is_some();
    let has_footer = add_footer.is_some();

    section_card(ui, |ui| {
        if let Some(title) = title {
            ui.label(title);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
        }
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            let content = |ui: &mut Ui| add_content(ui);
            if has_header || has_footer {
                Frame::NONE
                    .inner_margin(Margin::symmetric(12, 0))
                    .show(ui, content);
            } else {
                content(ui);
            }
        });
        if let Some(add_footer) = add_footer {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            add_footer(ui);
            ui.add_space(4.0);
        }
    });
}

/// Draws a configuration card with a caller-controlled header while keeping
/// the standard configuration-card spacing for its rows.
pub(crate) fn config_card_with_header(
    ui: &mut Ui,
    draw_header: impl FnOnce(&mut Ui),
    add_content: impl FnOnce(&mut Ui),
) {
    section_card(ui, |ui| {
        draw_header(ui);
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            Frame::NONE
                .inner_margin(Margin::symmetric(12, 0))
                .show(ui, add_content);
        });
    });
}

/// Draws a title and optional description centered within the supplied row.
pub(crate) fn centered_label(
    ui: &mut Ui,
    rect: egui::Rect,
    title: RichText,
    description: Option<RichText>,
) {
    let title_height = ui.text_style_height(&egui::TextStyle::Body);
    let body_size = ui
        .style()
        .text_styles
        .get(&egui::TextStyle::Body)
        .map_or(14.0, |font| font.size);
    // Small/Body are both normalized to the user's configured size, so use an
    // explicit scale to keep descriptions visibly subordinate to their labels.
    let description_size = body_size * 0.78;
    let description_height = description
        .as_ref()
        .map(|_| ui.text_style_height(&egui::TextStyle::Small) * description_size / body_size)
        .unwrap_or(0.0);
    let content_height = if description.is_some() {
        title_height + 6.0 + description_height
    } else {
        title_height
    };
    let centered_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.center().y - (content_height * 0.5)),
        egui::vec2(rect.width(), content_height),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(centered_rect)
            .layout(Layout::top_down(Align::Min)),
        |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.label(title);
                if let Some(description) = description {
                    ui.label(
                        description
                            .font(FontId::proportional(description_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                }
            });
        },
    );
}

/// Draws a full-width setting row with a vertically centered label block and control.
///
/// Keeping this layout in the card component ensures switches, dropdowns and future
/// controls share the same alignment without each settings page inventing its own row.
pub(crate) fn config_row(
    ui: &mut Ui,
    title: impl Into<RichText>,
    description: Option<impl Into<RichText>>,
    add_control: impl FnOnce(&mut Ui),
) {
    config_row_with_icon(ui, None, title, description, add_control);
}

/// Draws a configuration row and, when requested, its bottom divider.
/// Callers choose the divider so a card can separate successive settings
/// without leaving an unnecessary rule below its final item.
pub(crate) fn config_row_with_divider(
    ui: &mut Ui,
    title: impl Into<RichText>,
    description: Option<impl Into<RichText>>,
    show_divider: bool,
    add_control: impl FnOnce(&mut Ui),
) {
    config_row(ui, title, description, add_control);
    if show_divider {
        ui.separator();
    }
}

/// Draws a setting row with an optional leading icon before its label block.
pub(crate) fn config_row_with_icon(
    ui: &mut Ui,
    icon: Option<&TextureHandle>,
    title: impl Into<RichText>,
    description: Option<impl Into<RichText>>,
    add_control: impl FnOnce(&mut Ui),
) {
    let title = title.into();
    let description = description.map(Into::into);
    let body_height = ui.text_style_height(&egui::TextStyle::Body);
    let small_height = ui.text_style_height(&egui::TextStyle::Small);
    let row_height = if description.is_some() {
        (body_height + small_height + 12.0).max(body_height * 2.6)
    } else {
        (body_height + 12.0).max(body_height * 1.9)
    };
    let width = ui.available_width();
    ui.allocate_ui_with_layout(
        Vec2::new(width, row_height),
        Layout::left_to_right(Align::Center),
        |ui| {
            if let Some(icon) = icon {
                ui.add(
                    egui::Image::new(icon)
                        .fit_to_exact_size(Vec2::splat(28.0))
                        .corner_radius(6.0),
                );
                ui.add_space(10.0);
            }
            let icon_width = icon.map_or(0.0, |_| 38.0);
            let label_width = (ui.available_width() - 216.0 - icon_width).max(0.0);
            let label_rect = ui.allocate_space(Vec2::new(label_width, row_height)).1;
            centered_label(ui, label_rect, title, description);
            ui.with_layout(Layout::right_to_left(Align::Center), add_control);
        },
    );
}
