//! Opaque egui pet context menu rendered by the direct native host.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use deskhud_ui::{MessageKey, UiPreferences};
use egui::{Align2, Color32, CornerRadius, FontId, Frame, Margin, Sense, Stroke, Vec2};

const ROW_HEIGHT: f32 = 34.0;
const SEPARATOR_HEIGHT: f32 = 9.0;
const MENU_WIDTH: f32 = 180.0;
const MENU_PADDING_X: f32 = 8.0;
const MENU_PADDING_Y: f32 = 8.0;
const TEXT_INSET: f32 = 10.0;
const LABEL_FONT: f32 = 13.5;

pub(crate) fn menu_height() -> f32 {
    MENU_PADDING_Y * 2.0 + 5.0 * ROW_HEIGHT + 2.0 * SEPARATOR_HEIGHT + 2.0
}

#[derive(Clone)]
pub(crate) struct PetMenuHost {
    inner: Arc<Mutex<PetMenuState>>,
}

pub(crate) struct PetMenuState {
    pub open: bool,
    pub opened_at: Instant,
    pub anchor: egui::Pos2,
    pub menu_width: f32,
    prefs: UiPreferences,
    pub(crate) master_enabled: bool,
    pub(crate) pet_topmost: bool,
    pub open_settings: bool,
    pub begin_hud_layout: bool,
    pub toggle_master: Option<bool>,
    pub toggle_topmost: Option<bool>,
    pub quit: bool,
}

impl PetMenuHost {
    pub(crate) fn new(prefs: UiPreferences) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PetMenuState {
                open: false,
                opened_at: Instant::now(),
                anchor: egui::Pos2::ZERO,
                menu_width: MENU_WIDTH,
                prefs,
                master_enabled: true,
                pet_topmost: true,
                open_settings: false,
                begin_hud_layout: false,
                toggle_master: None,
                toggle_topmost: None,
                quit: false,
            })),
        }
    }

    pub(crate) fn lock(&self) -> std::sync::MutexGuard<'_, PetMenuState> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }

    pub(crate) fn open_at(
        &self,
        prefs: &UiPreferences,
        cursor_points: egui::Pos2,
        pixels_per_point: f32,
        master_enabled: bool,
        pet_topmost: bool,
    ) {
        let (x, y) = crate::platform::fit_popup_pos_points(
            (cursor_points.x, cursor_points.y),
            MENU_WIDTH,
            menu_height(),
            pixels_per_point,
        );
        let mut state = self.lock();
        state.open = true;
        state.opened_at = Instant::now();
        state.anchor = egui::pos2(x, y);
        state.menu_width = MENU_WIDTH;
        state.prefs = prefs.clone();
        state.master_enabled = master_enabled;
        state.pet_topmost = pet_topmost;
        state.open_settings = false;
        state.begin_hud_layout = false;
        state.toggle_master = None;
        state.toggle_topmost = None;
        state.quit = false;
    }

    pub(crate) fn draw_native(&self, ui: &mut egui::Ui) {
        let context = ui.ctx().clone();
        let theme = self.lock().prefs.shell.ui_theme;
        crate::theme::apply(&context, theme);
        if context.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.lock().open = false;
            return;
        }

        let dark = matches!(context.theme(), egui::Theme::Dark);
        let fill = if dark {
            Color32::from_rgb(43, 45, 49)
        } else {
            Color32::from_rgb(248, 248, 252)
        };
        ui.painter()
            .rect_filled(ui.max_rect(), CornerRadius::ZERO, fill);

        let mut close = false;
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(fill).inner_margin(Margin::symmetric(
                MENU_PADDING_X as i8,
                MENU_PADDING_Y as i8,
            )))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let mut state = self.lock();
                let settings = state.prefs.t(MessageKey::MenuSettings).to_owned();
                let topmost = state.prefs.t(MessageKey::SettingsTopmost).to_owned();
                let plugins = state.prefs.t(MessageKey::SettingsNavHud).to_owned();
                let layout = state.prefs.t(MessageKey::MenuHudLayout).to_owned();
                let quit = state.prefs.t(MessageKey::MenuQuit).to_owned();
                let master_enabled = state.master_enabled;
                let pet_topmost = state.pet_topmost;

                if action_row(ui, &settings, dark).clicked() {
                    state.open_settings = true;
                    close = true;
                }
                if check_row(ui, &topmost, pet_topmost, dark).clicked() {
                    state.toggle_topmost = Some(!pet_topmost);
                    close = true;
                }
                separator(ui, dark);
                if check_row(ui, &plugins, master_enabled, dark).clicked() {
                    state.toggle_master = Some(!master_enabled);
                    close = true;
                }
                ui.add_enabled_ui(master_enabled && !cfg!(target_os = "macos"), |ui| {
                    if action_row(ui, &layout, dark).clicked() {
                        state.begin_hud_layout = true;
                        close = true;
                    }
                });
                separator(ui, dark);
                if action_row(ui, &quit, dark).clicked() {
                    state.quit = true;
                    close = true;
                }
            });
        if close {
            self.lock().open = false;
        }
    }
}

fn text_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(232, 234, 237)
    } else {
        Color32::from_rgb(28, 28, 32)
    }
}

fn hover_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(52, 54, 60)
    } else {
        Color32::from_rgb(232, 236, 244)
    }
}

fn action_row(ui: &mut egui::Ui, label: &str, dark: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_HEIGHT), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), hover_color(dark));
    }
    ui.painter().text(
        egui::pos2(rect.left() + TEXT_INSET, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(LABEL_FONT),
        text_color(dark),
    );
    response
}

fn check_row(ui: &mut egui::Ui, label: &str, checked: bool, dark: bool) -> egui::Response {
    let response = action_row(ui, label, dark);
    if checked {
        ui.painter().text(
            egui::pos2(response.rect.right() - 12.0, response.rect.center().y),
            Align2::RIGHT_CENTER,
            "✓",
            FontId::proportional(14.0),
            Color32::from_rgb(70, 135, 230),
        );
    }
    response
}

fn separator(ui: &mut egui::Ui, dark: bool) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), SEPARATOR_HEIGHT),
        Sense::hover(),
    );
    let color = if dark {
        Color32::from_rgb(60, 64, 72)
    } else {
        Color32::from_rgb(220, 222, 230)
    };
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::pos2(rect.right() - 4.0, rect.center().y),
        ],
        Stroke::new(1.0, color),
    );
}
