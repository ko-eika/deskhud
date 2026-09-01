//! Switch row and toggle control.

use egui::{CornerRadius, RichText, Sense, TextStyle, Ui, Vec2};

/// A two-column setting row with an optional supporting description.
pub(crate) fn switch_row(
    ui: &mut Ui,
    title: impl Into<RichText>,
    description: Option<impl Into<RichText>>,
    value: &mut bool,
) -> egui::Response {
    let title = title.into();
    let description = description.map(Into::into);
    let line_height = ui.text_style_height(&TextStyle::Body);
    let description_height = ui.text_style_height(&TextStyle::Small);
    let row_gap = 4.0;
    let height = if description.is_some() {
        (line_height + row_gap + description_height + 12.0).max(48.0)
    } else {
        (line_height + 12.0).max(36.0)
    };
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let text_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 2.0, rect.top()),
        egui::pos2(rect.right() - 58.0, rect.bottom()),
    );
    super::centered_label(ui, text_rect, title, description);
    let switch_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 22.0, rect.center().y),
        Vec2::new(42.0, 24.0),
    );
    response.union(toggle_switch(ui, switch_rect, value))
}

pub(crate) fn toggle_switch(ui: &mut Ui, rect: egui::Rect, value: &mut bool) -> egui::Response {
    let mut response = ui.interact(
        rect,
        ui.id()
            .with(("switch", rect.top().to_bits(), rect.left().to_bits())),
        Sense::click(),
    );
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool(response.id, *value);
    let mut fill = super::lerp_color(
        ui.visuals().widgets.inactive.bg_fill,
        ui.visuals().selection.bg_fill,
        t,
    );
    if response.hovered() {
        let hover_target = if *value {
            ui.visuals().selection.bg_fill.gamma_multiply(1.16)
        } else {
            ui.visuals().widgets.hovered.bg_fill
        };
        fill = super::lerp_color(fill, hover_target, 0.68);
    }
    ui.painter().rect_filled(rect, CornerRadius::same(12), fill);
    let knob_x = egui::lerp((rect.left() + 12.0)..=(rect.right() - 12.0), t);
    ui.painter().circle_filled(
        egui::pos2(knob_x, rect.center().y),
        8.0,
        ui.visuals().extreme_bg_color,
    );
    response
}
