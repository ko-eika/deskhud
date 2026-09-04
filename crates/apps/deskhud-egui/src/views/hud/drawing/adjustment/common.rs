//! HUD 调整窗口共用的标签、输入与单位控件。

use super::*;

pub(super) fn draw_effect_label(ui: &mut egui::Ui, rect: egui::Rect, text: &str) {
    let mut label_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    label_ui.label(text);
}

pub(super) fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ])
}

/// Draws the HUD editor's shared colour swatch and editable hexadecimal value.
/// The same editor is used for both a HUD's visual overrides and group labels,
/// so the two adjustment panels do not drift into separate colour UIs.
pub(super) fn draw_hex_color_control(ui: &mut egui::Ui, color: &mut [u8; 3], id: egui::Id) -> bool {
    const VALUE_WIDTH: f32 = ADJUST_VALUE_WIDTH;
    // Keep the compound editor within the standard 216 px settings-control
    // footprint, so its left edge aligns with the other group fields.
    const TOTAL_WIDTH: f32 = 216.0;
    let picker_width =
        (ui.available_width().min(TOTAL_WIDTH) - VALUE_WIDTH - ui.spacing().item_spacing.x)
            .max(32.0);
    let input_id = id.with("text");
    let canonical = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
    let mut input = ui.ctx().data_mut(|data| {
        data.get_temp::<String>(input_id)
            .unwrap_or_else(|| canonical.clone())
    });

    let response = ui.add_sized(
        egui::vec2(VALUE_WIDTH, ADJUST_ROW_HEIGHT),
        egui::TextEdit::singleline(&mut input)
            .id(input_id.with("edit"))
            .font(egui::TextStyle::Monospace)
            .horizontal_align(egui::Align::Center)
            .vertical_align(egui::Align::Center),
    );
    let mut changed = false;
    if response.changed()
        && let Some(parsed) = parse_hex_color(&input)
    {
        *color = parsed;
        changed = true;
    }
    if response.lost_focus() && parse_hex_color(&input).is_none() {
        input = canonical.clone();
    }

    let picker_rect = ui
        .allocate_exact_size(
            egui::vec2(picker_width, ADJUST_ROW_HEIGHT),
            egui::Sense::hover(),
        )
        .0;
    let mut picker_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(id.with("picker"))
            .max_rect(picker_rect)
            .layout(egui::Layout::centered_and_justified(
                egui::Direction::TopDown,
            )),
    );
    picker_ui.spacing_mut().interact_size = picker_rect.size();
    if picker_ui.color_edit_button_srgb(color).changed() {
        input = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
        changed = true;
    }
    ui.ctx().data_mut(|data| data.insert_temp(input_id, input));
    changed
}

pub(super) fn allocate_effect_row(ui: &mut egui::Ui) -> (egui::Rect, egui::Rect, egui::Rect) {
    let row_width = ui.available_width();
    let spacing = ui.spacing().item_spacing.x;
    let (row_rect, _) = ui.allocate_exact_size(
        egui::vec2(row_width, ADJUST_ROW_HEIGHT),
        egui::Sense::hover(),
    );
    let label_rect = egui::Rect::from_min_size(
        row_rect.min + egui::vec2(ADJUST_LABEL_INDENT, 0.0),
        egui::vec2(ADJUST_LABEL_WIDTH - ADJUST_LABEL_INDENT, ADJUST_ROW_HEIGHT),
    );
    let value_rect = egui::Rect::from_min_size(
        egui::pos2(row_rect.right() - ADJUST_VALUE_WIDTH, row_rect.top()),
        egui::vec2(ADJUST_VALUE_WIDTH, ADJUST_ROW_HEIGHT),
    );
    let control_rect = egui::Rect::from_min_max(
        egui::pos2(label_rect.right() + spacing, row_rect.top()),
        egui::pos2(value_rect.left() - spacing, row_rect.bottom()),
    );
    (label_rect, control_rect, value_rect)
}
