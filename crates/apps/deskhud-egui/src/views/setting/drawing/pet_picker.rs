//! Pet picker cards.

use super::pet::{pet_preview_rect, pet_preview_texture, pet_tooltip};
use super::{catalog_text, paint_preview_contain, paint_preview_frame, truncate_ui_text};
use crate::components;
use deskhud_ui::{CatalogStore, SettingsModel};
use egui::{Sense, Stroke, Ui, Vec2};

pub(super) fn draw_pet_grid_card(
    ui: &mut Ui,
    catalogs: &CatalogStore,
    model: &mut SettingsModel,
    info: &deskhud_engine::PetKindInfo,
    layout: deskhud_ui::PetCardLayout,
) {
    let scale = layout.content_scale;
    let pad = 12.0 * scale;
    let selected = model.draft.pet.kind == info.id;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(layout.card_width, layout.card_height),
        Sense::click(),
    );
    let name = catalog_text(
        catalogs,
        model.draft.locale,
        info.id,
        "display_name",
        info.display_name,
    );
    let description = catalog_text(
        catalogs,
        model.draft.locale,
        info.id,
        "description",
        info.description,
    );
    let response = pet_tooltip(response, info, &name, &description, model.draft.locale);
    let draw = rect.shrink(1.0);
    let fill = if selected {
        ui.visuals().selection.bg_fill.gamma_multiply(0.18)
    } else if response.hovered() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().extreme_bg_color
    };
    let stroke = if selected {
        Stroke::new(
            1.0,
            components::lerp_color(
                ui.visuals().widgets.noninteractive.bg_stroke.color,
                ui.visuals().selection.stroke.color,
                0.32,
            ),
        )
    } else {
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
    };
    ui.painter()
        .rect(draw, 12.0 * scale, fill, stroke, egui::StrokeKind::Inside);

    let stage = egui::Rect::from_center_size(
        egui::pos2(
            draw.center().x,
            draw.top() + pad + layout.preview_side * 0.5,
        ),
        Vec2::splat(layout.preview_side),
    );
    let preview = pet_preview_rect(stage);
    paint_preview_frame(ui, preview, scale);
    if let Some(texture) = pet_preview_texture(ui, info) {
        paint_preview_contain(ui, preview, &texture);
    }

    let title = ui.text_style_height(&egui::TextStyle::Body) * scale * 1.08;
    let small = ui.text_style_height(&egui::TextStyle::Small) * scale;
    // Use a more breathable baseline gap inside the bottom information group;
    // it still scales with the rest of the card content.
    let line_gap = 8.0 * scale;
    let left = draw.left() + pad;
    // Keep the information group bottom-anchored. This makes the preview and
    // the text two stable vertical groups instead of a chain whose gaps vary
    // with the amount of text in each card.
    let description_bottom = draw.bottom() - pad;
    let metadata_bottom = description_bottom - small - line_gap;
    let title_bottom = metadata_bottom - small - line_gap;
    ui.painter().text(
        egui::pos2(left, title_bottom),
        egui::Align2::LEFT_BOTTOM,
        truncate_ui_text(
            ui,
            &name,
            egui::FontId::proportional(title),
            draw.width() - pad * 2.0,
        ),
        egui::FontId::proportional(title),
        ui.visuals().text_color(),
    );
    ui.painter().text(
        egui::pos2(left, metadata_bottom),
        egui::Align2::LEFT_BOTTOM,
        truncate_ui_text(
            ui,
            &format!(
                "{}  ·  {:.0}×{:.0}",
                info.author, info.window_width, info.window_height
            ),
            egui::FontId::proportional(small),
            draw.width() - pad * 2.0,
        ),
        egui::FontId::proportional(small),
        ui.visuals().weak_text_color(),
    );
    ui.painter().text(
        egui::pos2(left, description_bottom),
        egui::Align2::LEFT_BOTTOM,
        truncate_ui_text(
            ui,
            &description,
            egui::FontId::proportional(small),
            draw.width() - pad * 2.0,
        ),
        egui::FontId::proportional(small),
        ui.visuals().weak_text_color(),
    );
    if response.clicked() {
        let mode = model.draft.pet.picker_mode;
        deskhud_ui::apply_pet_selection(&mut model.draft, info.id.to_string(), mode);
        model
            .draft
            .pet
            .apply_window_size(info.window_width, info.window_height);
    }
}

pub(super) fn draw_pet_list_row(
    ui: &mut Ui,
    catalogs: &CatalogStore,
    model: &mut SettingsModel,
    info: &deskhud_engine::PetKindInfo,
) {
    const PAD: f32 = 12.0;
    let selected = model.draft.pet.kind == info.id;
    let body = ui.text_style_height(&egui::TextStyle::Body);
    let small = ui.text_style_height(&egui::TextStyle::Small);
    const LINE_GAP: f32 = 8.0;
    let text_block_height = body + small * 2.0 + LINE_GAP * 2.0;
    // The preview square exactly matches the complete three-line text group:
    // title, metadata, description, plus the two scaled vertical gaps.
    let thumb_side = text_block_height;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), thumb_side + PAD * 2.0),
        Sense::click(),
    );
    let name = catalog_text(
        catalogs,
        model.draft.locale,
        info.id,
        "display_name",
        info.display_name,
    );
    let description = catalog_text(
        catalogs,
        model.draft.locale,
        info.id,
        "description",
        info.description,
    );
    let response = pet_tooltip(response, info, &name, &description, model.draft.locale);
    let draw = rect.shrink(0.5);
    let fill = if selected {
        ui.visuals().selection.bg_fill.gamma_multiply(0.18)
    } else if response.hovered() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().extreme_bg_color
    };
    let stroke = if selected {
        Stroke::new(
            1.0,
            components::lerp_color(
                ui.visuals().widgets.noninteractive.bg_stroke.color,
                ui.visuals().selection.stroke.color,
                0.32,
            ),
        )
    } else {
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
    };
    ui.painter()
        .rect(draw, 10.0, fill, stroke, egui::StrokeKind::Inside);
    let thumb = egui::Rect::from_min_size(draw.min + Vec2::splat(PAD), Vec2::splat(thumb_side));
    let preview = pet_preview_rect(thumb);
    paint_preview_frame(ui, preview, 1.0);
    if let Some(texture) = pet_preview_texture(ui, info) {
        paint_preview_contain(ui, preview, &texture);
    }
    let left = thumb.right() + PAD;
    let top = thumb.top();
    let description_bottom = thumb.bottom();
    let metadata_top = top + body + LINE_GAP;
    ui.painter().text(
        egui::pos2(left, top),
        egui::Align2::LEFT_TOP,
        truncate_ui_text(
            ui,
            &name,
            egui::FontId::proportional(body),
            draw.right() - left - PAD,
        ),
        egui::FontId::proportional(body),
        ui.visuals().text_color(),
    );
    ui.painter().text(
        egui::pos2(left, metadata_top),
        egui::Align2::LEFT_TOP,
        truncate_ui_text(
            ui,
            &format!(
                "{}  ·  {:.0}×{:.0}",
                info.author, info.window_width, info.window_height
            ),
            egui::FontId::proportional(small),
            draw.right() - left - PAD,
        ),
        egui::FontId::proportional(small),
        ui.visuals().weak_text_color(),
    );
    ui.painter().text(
        egui::pos2(left, description_bottom),
        egui::Align2::LEFT_BOTTOM,
        truncate_ui_text(
            ui,
            &description,
            egui::FontId::proportional(small),
            draw.right() - left - PAD,
        ),
        egui::FontId::proportional(small),
        ui.visuals().weak_text_color(),
    );
    if response.clicked() {
        let mode = model.draft.pet.picker_mode;
        deskhud_ui::apply_pet_selection(&mut model.draft, info.id.to_string(), mode);
        model
            .draft
            .pet
            .apply_window_size(info.window_width, info.window_height);
    }
}
