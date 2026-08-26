//! Pet 菜单窗口与通用菜单控制器之间的适配层。
#![cfg_attr(target_os = "macos", allow(dead_code))]

use deskhud_ui::UiTheme;
use winit::{
    dpi::PhysicalPosition,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::WindowId,
};

use crate::{
    menu::{MenuConfig, MenuController},
    runtime::viewport::{UserEvent, WindowLayer},
};

use super::{PetMenuAction, definition, parse_action};

/// Pet 菜单窗口及其子菜单树。
pub(crate) struct PetMenu {
    /// 通用菜单控制器。
    controller: MenuController,
}

impl PetMenu {
    /// 创建 Pet 菜单，并预热固定数量的子菜单窗口。
    pub(crate) fn create(event_loop: &ActiveEventLoop, proxy: &EventLoopProxy<UserEvent>) -> Self {
        let mut controller = MenuController::new(
            event_loop,
            MenuConfig {
                title: "Pet menu",
                show_title: false,
                egui_id: egui::ViewportId::from_hash_of("pet-menu"),
                #[cfg(target_os = "linux")]
                size: [260.0, 340.0],
                #[cfg(target_os = "linux")]
                focus_on_show: true,
                #[cfg(not(target_os = "linux"))]
                focus_on_show: true,
                ..Default::default()
            },
            proxy,
        );
        controller.prewarm_submenus(event_loop);
        Self { controller }
    }

    /// 判断窗口是否属于 Pet 菜单树。
    pub(crate) fn contains_window(&self, window_id: WindowId) -> bool {
        self.controller.contains_window(window_id)
    }

    /// 返回根菜单窗口标识。
    pub(crate) fn window_id(&self) -> WindowId {
        self.controller.window_id()
    }

    /// 返回菜单树中所有原生窗口句柄。
    pub(crate) fn window_handles(&self) -> Vec<(WindowId, std::sync::Arc<winit::window::Window>)> {
        self.controller.window_handles()
    }

    /// 判断根菜单是否可见。
    pub(crate) fn is_visible(&self) -> bool {
        self.controller.is_visible()
    }

    /// 根据当前窗口状态，在指定屏幕位置打开菜单树。
    pub(crate) fn open(
        &mut self,
        anchor: PhysicalPosition<i32>,
        pet_layer: WindowLayer,
        hud_layer: WindowLayer,
        hud_open: bool,
    ) {
        let menu_definition = definition(pet_layer, hud_layer, hud_open);
        self.controller.open(anchor, &menu_definition);
    }

    /// 关闭整棵菜单树。
    pub(crate) fn close(&mut self) {
        self.controller.close();
    }

    /// 将原生窗口事件路由到对应层级的菜单。
    pub(crate) fn handle_event(&mut self, window_id: WindowId, event: &WindowEvent) {
        self.controller.handle_event(window_id, event);
    }

    /// 绘制菜单树，并将点击结果转换为 Pet 业务动作。
    pub(crate) fn render(
        &mut self,
        pet_layer: WindowLayer,
        hud_layer: WindowLayer,
        hud_open: bool,
        theme: UiTheme,
    ) -> (Option<PetMenuAction>, bool) {
        let menu_definition = definition(pet_layer, hud_layer, hud_open);
        let (selected_item, should_close) = self.controller.render(&menu_definition, theme);
        let action = selected_item.as_deref().and_then(parse_action);
        (action, should_close)
    }

    /// 释放菜单树的全部 OpenGL 资源。
    pub(crate) fn destroy(&mut self) {
        self.controller.destroy();
    }
}
