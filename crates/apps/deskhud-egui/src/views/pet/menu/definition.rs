//! Pet 菜单项定义和业务动作转换。

use crate::{
    menu::{MenuDefinition, MenuItem},
    runtime::viewport::WindowLayer,
};

const TOGGLE_ALWAYS_ON_TOP: &str = "pet.toggle_always_on_top";
const OPEN_SETTINGS: &str = "pet.open_settings";
const OPEN_HUD: &str = "pet.open_hud";
const HUD_LAYOUT: &str = "pet.hud_layout";
const HUD_LAYER: &str = "pet.hud_layer";
const HUD_LAYER_TOP: &str = "pet.hud_layer.top";
const HUD_LAYER_NORMAL: &str = "pet.hud_layer.normal";
const HUD_LAYER_BOTTOM: &str = "pet.hud_layer.bottom";
const HUD_LAYER_MORE: &str = "pet.hud_layer.more";
const HUD_LAYER_DETAILS: &str = "pet.hud_layer.details";
const HUD_LAYER_INFO: &str = "pet.hud_layer.info";
const EXIT_APPLICATION: &str = "pet.exit_application";

/// Pet 菜单返回给窗口协调层的业务动作。
pub(crate) enum Action {
    /// 切换 Pet 置顶状态。
    ToggleAlwaysOnTop,
    /// 打开 Settings 窗口。
    OpenSettings,
    /// 显示或隐藏 HUD。
    OpenHud,
    /// 进入 HUD 布局模式。
    HudLayout,
    /// 修改 HUD 窗口层级。
    SetHudLayer(WindowLayer),
    /// 请求退出应用。
    ExitApplication,
}

/// 根据当前 Pet/HUD 状态创建菜单定义。
pub(crate) fn definition(
    pet_layer: WindowLayer,
    hud_layer: WindowLayer,
    hud_open: bool,
) -> MenuDefinition {
    MenuDefinition::new(vec![
        MenuItem::checkable(
            TOGGLE_ALWAYS_ON_TOP,
            "Toggle always on top",
            pet_layer == WindowLayer::AlwaysOnTop,
        ),
        MenuItem::new(OPEN_SETTINGS, "Open settings"),
        MenuItem::checkable(OPEN_HUD, "HUD", hud_open).with_separator_before(),
        MenuItem::new(HUD_LAYER, "HUD layer")
            .with_enabled(hud_open)
            .with_submenu(MenuDefinition::new(vec![
                MenuItem::checkable(
                    HUD_LAYER_TOP,
                    "Always on top",
                    hud_layer == WindowLayer::AlwaysOnTop,
                ),
                MenuItem::checkable(HUD_LAYER_NORMAL, "Normal", hud_layer == WindowLayer::Normal),
                MenuItem::checkable(
                    HUD_LAYER_BOTTOM,
                    "Always on bottom",
                    hud_layer == WindowLayer::AlwaysOnBottom,
                ),
            ])),
        MenuItem::new(HUD_LAYOUT, "HUD layout").with_enabled(hud_open),
        MenuItem::new(HUD_LAYER_MORE, "More options")
            .with_separator_before()
            .with_submenu(MenuDefinition::new(vec![
                MenuItem::new(HUD_LAYER_DETAILS, "Display details").with_submenu(
                    MenuDefinition::new(vec![MenuItem::new(HUD_LAYER_INFO, "Window info")]),
                ),
                MenuItem::new("pet.debug.monitor", "Monitor information").with_enabled(false),
                MenuItem::new("pet.debug.position", "Window position").with_enabled(false),
                MenuItem::new("pet.debug.size", "Window size").with_enabled(false),
                MenuItem::new("pet.debug.renderer", "Renderer status").with_enabled(false),
                MenuItem::new("pet.debug.input", "Input state").with_enabled(false),
                MenuItem::new("pet.debug.layout", "Layout diagnostics").with_enabled(false),
            ])),
        MenuItem::new(EXIT_APPLICATION, "Close application").with_separator_before(),
    ])
}

/// 将通用菜单返回的标识转换为 Pet 业务动作。
pub(crate) fn parse_action(id: &str) -> Option<Action> {
    match id {
        TOGGLE_ALWAYS_ON_TOP => Some(Action::ToggleAlwaysOnTop),
        OPEN_SETTINGS => Some(Action::OpenSettings),
        OPEN_HUD => Some(Action::OpenHud),
        HUD_LAYOUT => Some(Action::HudLayout),
        HUD_LAYER_TOP => Some(Action::SetHudLayer(WindowLayer::AlwaysOnTop)),
        HUD_LAYER_NORMAL => Some(Action::SetHudLayer(WindowLayer::Normal)),
        HUD_LAYER_BOTTOM => Some(Action::SetHudLayer(WindowLayer::AlwaysOnBottom)),
        EXIT_APPLICATION => Some(Action::ExitApplication),
        _ => None,
    }
}
