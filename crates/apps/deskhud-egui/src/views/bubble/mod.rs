//! 宿主管理的独立宠物对话气泡视图。

use crate::{
    runtime::{
        viewport::{UserEvent, Viewport, WindowLayer},
        viewport_config::ViewportConfig,
    },
    views::ViewOutput,
};
use deskhud_ui::UiPreferences;
use winit::{
    dpi::PhysicalPosition,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::WindowId,
};

#[derive(Clone)]
pub(crate) struct BubbleContent {
    pub(crate) text: String,
    pub(crate) color: [f32; 4],
    pub(crate) background: [f32; 4],
    pub(crate) corner_radius: f32,
}
pub(crate) struct PetBubbleWindow {
    viewport: Viewport,
    content: Option<BubbleContent>,
    font_size: f32,
    last_anchor: Option<PhysicalPosition<i32>>,
}
impl PetBubbleWindow {
    pub(crate) fn create(event_loop: &ActiveEventLoop, proxy: &EventLoopProxy<UserEvent>) -> Self {
        let mut viewport = Viewport::new(event_loop, ViewportConfig::pet_bubble(), proxy);
        viewport.set_cursor_hittest(false);
        Self {
            viewport,
            content: None,
            font_size: 14.0,
            last_anchor: None,
        }
    }
    pub(crate) fn window_id(&self) -> WindowId {
        self.viewport.window_id()
    }
    pub(crate) fn window_handle(&self) -> std::sync::Arc<winit::window::Window> {
        self.viewport.window_handle()
    }
    pub(crate) fn set_window_layer(&mut self, layer: WindowLayer) {
        self.viewport.set_window_layer(layer);
    }
    pub(crate) fn handle_event(&mut self, event: &WindowEvent) {
        self.viewport.handle_event(event);
    }
    pub(crate) fn update(
        &mut self,
        content: Option<BubbleContent>,
        anchor: PhysicalPosition<i32>,
        prefs: &UiPreferences,
    ) {
        let was_visible = self.content.is_some();
        self.content = content;
        self.font_size = prefs.shell.ui_font_size.max(10.0);
        self.viewport.apply_ui_preferences(prefs);
        if self.content.is_some() && self.last_anchor != Some(anchor) {
            self.viewport
                .request_outer_position(PhysicalPosition::new(anchor.x - 90, anchor.y - 26));
            self.last_anchor = Some(anchor);
        } else if self.content.is_none() {
            self.last_anchor = None;
        }
        if was_visible != self.content.is_some() {
            self.viewport
                .set_visible_without_focus(self.content.is_some());
        }
        self.viewport.set_cursor_hittest(false);
    }
    pub(crate) fn render(&mut self) {
        let content = self.content.clone();
        let font_size = self.font_size;
        self.viewport.render(|context, raw_input| {
            let full_output = context.run_ui(raw_input, |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        if let Some(content) = &content {
                            let rect = ui.max_rect();
                            let color = |rgba: [f32; 4]| {
                                egui::Color32::from_rgba_unmultiplied(
                                    (rgba[0].clamp(0.0, 1.0) * 255.0) as u8,
                                    (rgba[1].clamp(0.0, 1.0) * 255.0) as u8,
                                    (rgba[2].clamp(0.0, 1.0) * 255.0) as u8,
                                    (rgba[3].clamp(0.0, 1.0) * 255.0) as u8,
                                )
                            };
                            ui.painter().rect_filled(
                                rect,
                                content.corner_radius,
                                color(content.background),
                            );
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                &content.text,
                                egui::FontId::proportional(font_size),
                                color(content.color),
                            );
                        }
                    });
            });
            ViewOutput {
                full_output,
                ..Default::default()
            }
        });
    }
    pub(crate) fn is_visible(&self) -> bool {
        self.content.is_some()
    }
    pub(crate) fn hide(&mut self) {
        self.content = None;
        self.last_anchor = None;
        self.viewport.set_visible_without_focus(false);
    }
    pub(crate) fn destroy(&mut self) {
        self.viewport.destroy();
    }
}
