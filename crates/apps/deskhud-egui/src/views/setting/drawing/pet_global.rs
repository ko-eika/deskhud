//! Pet global configuration group.

use super::text;
use crate::components;
use deskhud_ui::{LayerPreference, MessageKey, SettingsModel};
use egui::{RichText, Sense, Stroke, Ui, Vec2};

pub(super) fn draw(ui: &mut Ui, model: &mut SettingsModel) {
    components::config_row_with_divider(
        ui,
        text(model, MessageKey::SettingsPetLayer),
        Some(RichText::new(text(model, MessageKey::SettingsPetLayerHint)).small()),
        true,
        |ui| {
            let options = [
                (
                    "top".to_owned(),
                    text(model, MessageKey::MenuLayerTop).to_owned(),
                ),
                (
                    "normal".to_owned(),
                    text(model, MessageKey::MenuLayerNormal).to_owned(),
                ),
                (
                    "bottom".to_owned(),
                    text(model, MessageKey::MenuLayerBottom).to_owned(),
                ),
            ];
            let selected = match model.draft.pet.layer {
                LayerPreference::Top => "top",
                LayerPreference::Normal => "normal",
                LayerPreference::Bottom => "bottom",
            };
            if let Some(value) =
                components::dropdown(ui, "settings_pet_layer", selected, &options, false)
            {
                model.draft.pet.layer = match value.as_str() {
                    "bottom" => LayerPreference::Bottom,
                    "normal" => LayerPreference::Normal,
                    _ => LayerPreference::Top,
                };
            }
        },
    );
    components::switch_row_with_divider(
        ui,
        RichText::new(text(model, MessageKey::SettingsPetBubbles)),
        Some(RichText::new(text(model, MessageKey::SettingsPetBubblesHint)).small()),
        &mut model.draft.pet.bubbles,
        true,
    );
    components::switch_row_with_divider(
        ui,
        RichText::new(text(model, MessageKey::SettingsPetKeyboardInput)),
        Some(RichText::new(text(model, MessageKey::SettingsPetKeyboardInputHint)).small()),
        &mut model.draft.pet.global_keyboard_input,
        true,
    );
    components::switch_row_with_divider(
        ui,
        RichText::new(text(model, MessageKey::SettingsPetMouseInput)),
        Some(RichText::new(text(model, MessageKey::SettingsPetMouseInputHint)).small()),
        &mut model.draft.pet.global_mouse_input,
        false,
    );
}

pub(super) fn draw_view_modes(ui: &mut Ui, model: &mut SettingsModel) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 28.0), Sense::hover());
    ui.painter().rect(
        rect,
        8.0,
        ui.visuals().extreme_bg_color,
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
    let (left, right) = (
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.center().x, rect.max.y)),
        egui::Rect::from_min_max(egui::pos2(rect.center().x, rect.min.y), rect.max),
    );
    let l = ui.interact(left, ui.id().with("pet-grid"), Sense::click());
    let r = ui.interact(right, ui.id().with("pet-list"), Sense::click());
    let active = if model.draft.pet.picker_mode == deskhud_ui::PetPickerMode::Grid {
        left
    } else {
        right
    };
    ui.painter().rect_filled(
        active.shrink(1.0),
        7.0,
        ui.visuals().selection.bg_fill.gamma_multiply(0.18),
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.center().x, rect.top() + 5.0),
            egui::pos2(rect.center().x, rect.bottom() - 5.0),
        ],
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
    let color = ui.visuals().selection.stroke.color;
    let icon_rect = |area: egui::Rect| {
        let side = area.width().min(area.height()) - 12.0;
        egui::Rect::from_center_size(area.center(), Vec2::splat(side.max(1.0)))
    };
    components::icons::paint(ui, "layout-grid", icon_rect(left), color, false);
    components::icons::paint(ui, "list-details", icon_rect(right), color, false);
    if l.clicked() {
        model.draft.pet.picker_mode = deskhud_ui::PetPickerMode::Grid;
    }
    if r.clicked() {
        model.draft.pet.picker_mode = deskhud_ui::PetPickerMode::List;
    }
}
