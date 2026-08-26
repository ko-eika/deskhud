//! Switch row and expandable switch group.

use egui::{CornerRadius, Frame, Margin, RichText, Sense, Stroke, TextStyle, Ui, Vec2};

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
    response.union(draw_switch(ui, switch_rect, value))
}

/// A styled master switch with an animated, collapsible child area.
pub(crate) fn switch_group(
    ui: &mut Ui,
    id_source: impl std::hash::Hash + std::fmt::Debug,
    title: impl Into<RichText>,
    description: Option<impl Into<RichText>>,
    enabled: &mut bool,
    add_children: impl FnOnce(&mut Ui),
) {
    let title = title.into();
    let description = description.map(Into::into);
    let border = ui
        .visuals()
        .widgets
        .noninteractive
        .bg_stroke
        .color
        .gamma_multiply(0.62);
    Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            let state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                ui.make_persistent_id(id_source),
                true,
            );
            let header = state.show_header(ui, |ui| switch_row(ui, title, description, enabled));
            let _ = header.body(|ui| {
                ui.add_enabled_ui(*enabled, |ui| {
                    ui.add_space(4.0);
                    ui.indent(ui.id().with("switch-group-content"), add_children);
                });
            });
        });
}

fn draw_switch(ui: &mut Ui, rect: egui::Rect, value: &mut bool) -> egui::Response {
    let response = ui.interact(
        rect,
        ui.id()
            .with(("switch", rect.top().to_bits(), rect.left().to_bits())),
        Sense::click(),
    );
    if response.clicked() {
        *value = !*value;
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
