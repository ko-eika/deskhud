//! Shared settings dropdown.
//!
//! The popup intentionally does not use `ComboBox`: the old settings page used
//! an independently positioned `Area` so it could control its exact width,
//! height, padding, scrolling and option painting. Searchable dropdowns keep
//! the same state machine: opening selects the current text, normal typing
//! performs prefix matching and selects the completion suffix, while delete
//! and IME preedit are never overwritten by matching.
#![allow(clippy::clone_on_copy, clippy::if_same_then_else)]

use egui::{
    Align2, Area, Color32, CornerRadius, Frame, Margin, Order, ScrollArea, Sense, Stroke, TextEdit,
    TextStyle, Ui, Vec2,
};

const DROPDOWN_MAX_HEIGHT: f32 = 320.0;
const DROPDOWN_OPTION_HEIGHT: f32 = 36.0;
const DROPDOWN_OPTION_GAP: f32 = 6.0;

/// Visual metrics for a dropdown while retaining the shared interaction and
/// popup behavior. Different surfaces can match their neighboring controls
/// without duplicating the dropdown state machine.
#[derive(Clone, Copy)]
pub(crate) struct DropdownStyle {
    pub(crate) width: f32,
    pub(crate) button_height: f32,
    pub(crate) button_radius: u8,
    pub(crate) popup_radius: u8,
    pub(crate) option_radius: u8,
    pub(crate) horizontal_padding: f32,
    pub(crate) vertical_padding: f32,
}

impl DropdownStyle {
    /// Standard settings-page dropdown metrics.
    pub(crate) const SETTINGS: Self = Self {
        width: 200.0,
        button_height: 40.0,
        button_radius: 11,
        popup_radius: 11,
        option_radius: 8,
        horizontal_padding: 14.0,
        vertical_padding: 10.0,
    };

    /// Compact metrics for HUD adjustment cards, matching their 216×32 inputs.
    pub(crate) const ADJUSTMENT: Self = Self {
        width: 216.0,
        button_height: 32.0,
        button_radius: 4,
        popup_radius: 8,
        option_radius: 4,
        horizontal_padding: 12.0,
        vertical_padding: 6.0,
    };
}

/// A key/label pair rendered by [`dropdown`].
pub(crate) type DropdownOption = (String, String);

#[derive(Clone, Default)]
struct DropdownState {
    open: bool,
    query: String,
    highlight: String,
    select_all: bool,
    suppress_match: bool,
    ime_composing: bool,
    scroll_to_highlight: bool,
}

/// Shows a consistent dropdown and returns the selected key when it changes.
pub(crate) fn dropdown(
    ui: &mut Ui,
    id_source: impl std::hash::Hash + std::fmt::Debug,
    selected: &str,
    options: &[DropdownOption],
    searchable: bool,
) -> Option<String> {
    dropdown_with_style(
        ui,
        id_source,
        selected,
        options,
        searchable,
        DropdownStyle::SETTINGS,
    )
}

/// Shows a dropdown using shared behavior with caller-selected visual metrics.
pub(crate) fn dropdown_with_style(
    ui: &mut Ui,
    id_source: impl std::hash::Hash + std::fmt::Debug,
    selected: &str,
    options: &[DropdownOption],
    searchable: bool,
    style: DropdownStyle,
) -> Option<String> {
    dropdown_impl(ui, id_source, selected, options, searchable, style)
}

fn dropdown_impl(
    ui: &mut Ui,
    id_source: impl std::hash::Hash + std::fmt::Debug,
    selected: &str,
    options: &[DropdownOption],
    searchable: bool,
    style: DropdownStyle,
) -> Option<String> {
    let id = ui.make_persistent_id(id_source);
    let mut state = ui.ctx().data_mut(|data| {
        data.get_temp_mut_or_insert_with(id, DropdownState::default)
            .clone()
    });
    let selected_label = options
        .iter()
        .find(|(key, _)| key == selected)
        .map_or(selected, |(_, label)| label.as_str());
    state.ime_composing = update_ime_composing(ui, state.ime_composing);
    let mut changed = None;

    let (button_rect, button_response) =
        ui.allocate_exact_size(Vec2::new(style.width, style.button_height), Sense::click());
    let opened_now = !state.open && button_response.clicked();
    if opened_now {
        state.open = true;
        state.query = selected_label.to_owned();
        state.highlight = selected.to_owned();
        state.select_all = searchable;
        state.suppress_match = false;
        state.scroll_to_highlight = true;
    } else if state.open && button_response.clicked() && !searchable {
        state.open = false;
    }

    let visuals = if state.open {
        ui.visuals().widgets.open.clone()
    } else {
        ui.style().interact(&button_response).clone()
    };
    ui.painter().rect(
        button_rect,
        CornerRadius::same(style.button_radius),
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );

    let inner = button_rect.shrink2(Vec2::new(style.horizontal_padding, style.vertical_padding));
    let arrow_rect =
        egui::Rect::from_min_max(egui::pos2(inner.right() - 24.0, inner.top()), inner.max);
    let text_rect = egui::Rect::from_min_max(
        inner.min,
        egui::pos2(arrow_rect.left() - 6.0, inner.bottom()),
    );
    let edit_id = id.with("search");

    if state.open && searchable {
        let mut query = state.query.clone();
        let response = ui.put(
            text_rect,
            TextEdit::singleline(&mut query)
                .id(edit_id)
                .frame(Frame::NONE)
                .desired_width(text_rect.width()),
        );
        if state.select_all {
            let mut edit_state = TextEdit::load_state(ui.ctx(), edit_id).unwrap_or_default();
            edit_state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::default(),
                    egui::text::CCursor::new(state.query.chars().count()),
                )));
            edit_state.store(ui.ctx(), edit_id);
            response.request_focus();
            state.select_all = false;
        }
        if response.changed() {
            apply_search_input(ui, &mut state, &mut query, options, edit_id, selected);
        }
        state.query = query;
    } else {
        let text = truncate_text(
            ui,
            selected_label,
            text_rect.width(),
            TextStyle::Button.resolve(ui.style()),
        );
        ui.painter().with_clip_rect(text_rect).text(
            egui::pos2(text_rect.left(), text_rect.center().y),
            Align2::LEFT_CENTER,
            text,
            TextStyle::Button.resolve(ui.style()),
            ui.visuals().text_color(),
        );
    }
    super::icons::paint(
        ui,
        "chevron-down",
        arrow_rect.shrink(2.0),
        ui.visuals().weak_text_color(),
        state.open,
    );

    if state.open {
        let popup_id = id.with("popup");
        let item_count = options.len();
        let content_height = 2.0 * style.vertical_padding
            + item_count as f32 * DROPDOWN_OPTION_HEIGHT
            + item_count.saturating_sub(1) as f32 * DROPDOWN_OPTION_GAP;
        let popup_height = content_height.min(DROPDOWN_MAX_HEIGHT);
        let popup_pos = egui::pos2(button_rect.left(), button_rect.bottom() + 2.0);
        let highlighted = state.highlight.clone();
        let highlight_index = options
            .iter()
            .position(|(key, _)| key == &highlighted)
            .unwrap_or(0);
        let popup = Area::new(popup_id)
            .order(Order::Foreground)
            .fixed_pos(popup_pos)
            .default_size(Vec2::new(button_rect.width(), popup_height))
            .sense(Sense::click())
            .show(ui.ctx(), |ui| {
                ui.set_width(button_rect.width());
                ui.set_min_width(button_rect.width());
                Frame::NONE
                    .fill(ui.visuals().window_fill)
                    .stroke(Stroke::new(1.0, ui.visuals().window_stroke.color))
                    .corner_radius(CornerRadius::same(style.popup_radius))
                    .inner_margin(Margin::symmetric(
                        style.horizontal_padding as i8,
                        style.vertical_padding as i8,
                    ))
                    .shadow(ui.visuals().popup_shadow)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = DROPDOWN_OPTION_GAP;
                        let draw_options = |ui: &mut Ui, changed: &mut Option<String>| {
                            for (key, label) in options {
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), DROPDOWN_OPTION_HEIGHT),
                                    Sense::click(),
                                );
                                let active = *key == highlighted;
                                let fill = if active {
                                    super::with_alpha(
                                        ui.visuals().selection.bg_fill,
                                        if response.hovered() { 112 } else { 64 },
                                    )
                                } else if response.hovered() {
                                    ui.visuals().widgets.hovered.bg_fill
                                } else {
                                    Color32::TRANSPARENT
                                };
                                if fill != Color32::TRANSPARENT {
                                    ui.painter().rect_filled(
                                        rect,
                                        CornerRadius::same(style.option_radius),
                                        fill,
                                    );
                                }
                                let option_font = TextStyle::Button.resolve(ui.style());
                                let label = truncate_text(
                                    ui,
                                    label,
                                    (rect.width() - style.horizontal_padding * 2.0).max(0.0),
                                    option_font.clone(),
                                );
                                ui.painter().with_clip_rect(rect).text(
                                    egui::pos2(
                                        rect.left() + style.horizontal_padding,
                                        rect.center().y,
                                    ),
                                    Align2::LEFT_CENTER,
                                    label,
                                    option_font,
                                    if active {
                                        ui.visuals().selection.stroke.color
                                    } else {
                                        ui.visuals().text_color()
                                    },
                                );
                                if response.clicked() {
                                    *changed = Some(key.clone());
                                }
                            }
                        };
                        if content_height > DROPDOWN_MAX_HEIGHT {
                            let viewport_height =
                                (DROPDOWN_MAX_HEIGHT - style.vertical_padding * 2.0).max(0.0);
                            let mut scroll = ScrollArea::vertical()
                                .max_height(DROPDOWN_MAX_HEIGHT)
                                .auto_shrink([false, true]);
                            if state.scroll_to_highlight {
                                let item_y = highlight_index as f32
                                    * (DROPDOWN_OPTION_HEIGHT + DROPDOWN_OPTION_GAP);
                                scroll = scroll.vertical_scroll_offset(
                                    (item_y - (viewport_height - DROPDOWN_OPTION_HEIGHT) * 0.5)
                                        .max(0.0),
                                );
                                state.scroll_to_highlight = false;
                            }
                            scroll.show(ui, |ui| draw_options(ui, &mut changed));
                        } else {
                            draw_options(ui, &mut changed);
                        }
                    });
            });
        if changed.is_some() {
            state.open = false;
        } else if ui.input(|input| input.pointer.any_click())
            && button_response.clicked_elsewhere()
            && popup.response.clicked_elsewhere()
        {
            state.open = false;
        }
    }

    if let Some(key) = changed.as_ref() {
        state.highlight = key.clone();
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, state));
    changed
}

fn apply_search_input(
    ui: &Ui,
    state: &mut DropdownState,
    query: &mut String,
    options: &[DropdownOption],
    edit_id: egui::Id,
    selected: &str,
) {
    if state.ime_composing {
        return;
    }
    let deleting = ui.input(|input| {
        input.key_pressed(egui::Key::Backspace) || input.key_pressed(egui::Key::Delete)
    });
    if deleting {
        state.suppress_match = true;
        state.highlight = selected.to_owned();
        return;
    }
    state.suppress_match = false;
    if query.is_empty() {
        return;
    }
    let typed = query.clone();
    let typed_lower = typed.to_lowercase();
    let Some((key, label)) = options.iter().find(|(key, label)| {
        key.to_lowercase().starts_with(&typed_lower)
            || label.to_lowercase().starts_with(&typed_lower)
    }) else {
        return;
    };
    state.highlight = key.clone();
    state.scroll_to_highlight = true;
    if label.to_lowercase().starts_with(&typed_lower) {
        *query = label.clone();
        select_text_suffix(
            ui.ctx(),
            edit_id,
            typed.chars().count(),
            query.chars().count(),
        );
    }
}

fn select_text_suffix(ctx: &egui::Context, id: egui::Id, start: usize, end: usize) {
    let mut state = TextEdit::load_state(ctx, id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(start),
            egui::text::CCursor::new(end),
        )));
    state.store(ctx, id);
}

fn update_ime_composing(ui: &Ui, mut composing: bool) -> bool {
    ui.input(|input| {
        for event in &input.events {
            match event {
                egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => {
                    composing = !text.is_empty();
                }
                egui::Event::Ime(egui::ImeEvent::Commit(_)) => composing = false,
                _ => {}
            }
        }
    });
    composing
}

fn truncate_text(ui: &Ui, text: &str, max_width: f32, font: egui::FontId) -> String {
    if max_width <= 8.0 {
        return String::new();
    }
    let full = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), ui.visuals().text_color())
        .size()
        .x;
    if full <= max_width {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while chars.len() > 1 {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "…";
        let width = ui
            .painter()
            .layout_no_wrap(candidate.clone(), font.clone(), ui.visuals().text_color())
            .size()
            .x;
        if width <= max_width {
            return candidate;
        }
    }
    "…".to_owned()
}
