//! 右键菜单：设置 / 退出（轻量；配置集中在统一设置窗）。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Frame, Margin, Sense, Stroke, Vec2};
use deskhud_ui::{MessageKey, UiPreferences};

use crate::win_chrome;

fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("deskhud_pet_menu")
}

const ROW_H: f32 = 34.0;
const MENU_MIN_W: f32 = 140.0;
/// 内容区最大宽（不含边距）；更长则「…」截断。
const MENU_MAX_CONTENT_W: f32 = 220.0;
const MENU_PAD_X: f32 = 8.0;
const TEXT_INSET: f32 = 10.0;
const DISMISS_GRACE: Duration = Duration::from_millis(280);
const LABEL_FONT: f32 = 13.5;

fn menu_height() -> f32 {
    10.0 + ROW_H + 4.0 + ROW_H + 10.0
}

fn menu_labels(prefs: &UiPreferences) -> [String; 2] {
    [
        prefs.t(MessageKey::MenuSettings).to_string(),
        prefs.t(MessageKey::MenuQuit).to_string(),
    ]
}

fn measure_text_width(ctx: &egui::Context, text: &str) -> f32 {
    let font = FontId::proportional(LABEL_FONT);
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font, Color32::WHITE)
            .size()
            .x
    })
}

fn compute_menu_width(ctx: &egui::Context, labels: &[String]) -> f32 {
    let mut content = 64.0_f32;
    for label in labels {
        content = content.max(measure_text_width(ctx, label));
    }
    let content = content.min(MENU_MAX_CONTENT_W);
    (content + TEXT_INSET + MENU_PAD_X * 2.0).clamp(MENU_MIN_W, MENU_MAX_CONTENT_W + TEXT_INSET + MENU_PAD_X * 2.0)
}

fn ellipsize(ctx: &egui::Context, text: &str, max_w: f32) -> String {
    if measure_text_width(ctx, text) <= max_w {
        return text.to_string();
    }
    let ell = "…";
    let ell_w = measure_text_width(ctx, ell);
    if ell_w >= max_w {
        return ell.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let cand: String = chars[..mid].iter().collect::<String>() + ell;
        if measure_text_width(ctx, &cand) <= max_w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>() + ell
}

/// 与主壳共享的右键菜单状态。
#[derive(Clone)]
pub struct PetMenuHost {
    inner: Arc<Mutex<PetMenuState>>,
}

#[derive(Debug)]
pub struct PetMenuState {
    pub open: bool,
    pub focus_once: bool,
    pub opened_at: Instant,
    pub menu_hwnd: Option<isize>,
    pub pet_hwnd: Option<isize>,
    pub popup_chrome_done: bool,
    pub anchor: egui::Pos2,
    pub cursor: egui::Pos2,
    pub ppp: f32,
    pub menu_w: f32,
    pub width_ready: bool,
    pub locale_prefs: UiPreferences,
    pub open_settings: bool,
    pub quit: bool,
}

impl PetMenuHost {
    pub fn new(prefs: UiPreferences) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PetMenuState {
                open: false,
                focus_once: false,
                opened_at: Instant::now(),
                menu_hwnd: None,
                pet_hwnd: None,
                popup_chrome_done: false,
                anchor: egui::pos2(0.0, 0.0),
                cursor: egui::pos2(0.0, 0.0),
                ppp: 1.0,
                menu_w: MENU_MIN_W,
                width_ready: false,
                locale_prefs: prefs,
                open_settings: false,
                quit: false,
            })),
        }
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, PetMenuState> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn is_open(&self) -> bool {
        self.lock().open
    }

    pub fn dismiss(&self, ctx: &egui::Context) {
        if self.lock().open {
            self.close_viewport(ctx);
        }
    }

    pub fn open_at(&self, prefs: &UiPreferences, cursor_points: egui::Pos2, ppp: f32) {
        // 首帧前用保守宽度贴边；字体测量后在 show 里再校正
        let provisional_w = MENU_MIN_W.max(180.0);
        let (x, y) = win_chrome::fit_popup_pos_points(
            (cursor_points.x, cursor_points.y),
            provisional_w,
            menu_height(),
            ppp,
        );
        let mut s = self.lock();
        s.open = true;
        s.focus_once = true;
        s.opened_at = Instant::now();
        s.menu_hwnd = None;
        s.popup_chrome_done = false;
        s.cursor = cursor_points;
        s.ppp = ppp;
        s.menu_w = provisional_w;
        s.width_ready = false;
        s.anchor = egui::pos2(x, y);
        s.locale_prefs = prefs.clone();
        s.open_settings = false;
        s.quit = false;
    }

    pub fn show(&self, ctx: &egui::Context, pet_hwnd: Option<isize>) {
        if !self.lock().open {
            return;
        }
        self.lock().pet_hwnd = pet_hwnd;
        self.maybe_dismiss_outside(ctx, pet_hwnd);
        if !self.lock().open {
            return;
        }

        if !self.lock().width_ready {
            let labels = {
                let s = self.lock();
                menu_labels(&s.locale_prefs)
            };
            let w = compute_menu_width(ctx, &labels);
            let mut s = self.lock();
            s.menu_w = w;
            s.width_ready = true;
            let (x, y) = win_chrome::fit_popup_pos_points(
                (s.cursor.x, s.cursor.y),
                w,
                menu_height(),
                s.ppp,
            );
            s.anchor = egui::pos2(x, y);
        }

        let (anchor, focus_once, menu_w) = {
            let s = self.lock();
            (s.anchor, s.focus_once, s.menu_w)
        };
        let height = menu_height();
        let shared = self.clone();
        ctx.show_viewport_deferred(
            viewport_id(),
            egui::ViewportBuilder::default()
                .with_title("")
                .with_decorations(false)
                .with_title_shown(false)
                .with_titlebar_shown(false)
                .with_titlebar_buttons_shown(false)
                .with_position(anchor)
                .with_inner_size([menu_w, height])
                .with_transparent(false)
                .with_resizable(false)
                .with_taskbar(false)
                .with_always_on_top()
                .with_active(true)
                .with_visible(true)
                .with_window_level(egui::WindowLevel::AlwaysOnTop),
            move |ui, _| shared.draw(ui),
        );

        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::OuterPosition(anchor));
        ctx.send_viewport_cmd_to(
            viewport_id(),
            egui::ViewportCommand::InnerSize(egui::vec2(menu_w, height)),
        );
        if focus_once {
            self.lock().focus_once = false;
            ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Focus);
        }
    }

    fn maybe_dismiss_outside(&self, ctx: &egui::Context, pet_hwnd: Option<isize>) {
        let (opened_at, menu_hwnd) = {
            let s = self.lock();
            if !s.open {
                return;
            }
            (s.opened_at, s.menu_hwnd)
        };
        if opened_at.elapsed() < DISMISS_GRACE {
            return;
        }
        if win_chrome::foreground_is_outside(pet_hwnd, menu_hwnd) {
            self.close_viewport(ctx);
        }
    }

    fn draw(&self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        {
            let mut s = self.lock();
            let pet = s.pet_hwnd;
            if s.menu_hwnd.is_none() {
                if let Some(h) = win_chrome::foreground_hwnd() {
                    if Some(h) != pet {
                        s.menu_hwnd = Some(h);
                    }
                }
            }
            if let Some(h) = s.menu_hwnd {
                if !s.popup_chrome_done {
                    win_chrome::ensure_acrylic_popup(h, pet);
                    s.popup_chrome_done = true;
                }
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape))
            || ctx.input(|i| i.viewport().close_requested())
        {
            self.close_viewport(&ctx);
            return;
        }
        let focused = ctx.input(|i| i.viewport().focused).unwrap_or(true);
        if self.lock().opened_at.elapsed() >= DISMISS_GRACE && !focused {
            self.close_viewport(&ctx);
            return;
        }

        let fill = Color32::from_rgb(248, 248, 252);
        ui.painter()
            .rect_filled(ui.max_rect(), CornerRadius::ZERO, fill);

        let mut close = false;
        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(fill)
                    .stroke(Stroke::NONE)
                    .inner_margin(Margin::symmetric(MENU_PAD_X as i8, 8)),
            )
            .show(ui, |ui| {
                let mut s = self.lock();
                let settings = s.locale_prefs.t(MessageKey::MenuSettings).to_string();
                let quit = s.locale_prefs.t(MessageKey::MenuQuit).to_string();
                let max_text_w = (ui.available_width() - TEXT_INSET).max(24.0);
                let settings_draw = ellipsize(&ctx, &settings, max_text_w);
                let quit_draw = ellipsize(&ctx, &quit, max_text_w);
                if action_row(ui, &settings_draw).clicked() {
                    s.open_settings = true;
                    close = true;
                }
                if action_row(ui, &quit_draw).clicked() {
                    s.quit = true;
                    close = true;
                }
            });

        if close {
            self.close_viewport(&ctx);
        }
    }

    fn close_viewport(&self, ctx: &egui::Context) {
        let mut s = self.lock();
        s.open = false;
        s.menu_hwnd = None;
        s.popup_chrome_done = false;
        s.width_ready = false;
        drop(s);
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::CancelClose);
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Visible(false));
    }
}

fn action_row(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(6),
            Color32::from_rgb(232, 236, 244),
        );
    }
    ui.painter().text(
        egui::pos2(rect.left() + TEXT_INSET, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(LABEL_FONT),
        Color32::from_rgb(28, 28, 32),
    );
    response
}
