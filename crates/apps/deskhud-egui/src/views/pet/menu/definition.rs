//! Pet 菜单项定义和业务动作转换。

use crate::{
    menu::{MenuDefinition, MenuItem},
    runtime::viewport::WindowLayer,
};
use deskhud_ui::{CatalogStore, Locale, MessageKey};

const PET_LAYER: &str = "pet.layer";
const PET_LAYER_TOP: &str = "pet.layer.top";
const PET_LAYER_NORMAL: &str = "pet.layer.normal";
const PET_LAYER_BOTTOM: &str = "pet.layer.bottom";
const OPEN_SETTINGS: &str = "pet.open_settings";
const OPEN_HUD: &str = "pet.open_hud";
const HUD_LAYOUT: &str = "pet.hud_layout";
const HUD_LAYER: &str = "pet.hud_layer";
const HUD_LAYER_TOP: &str = "pet.hud_layer.top";
const HUD_LAYER_NORMAL: &str = "pet.hud_layer.normal";
const HUD_LAYER_BOTTOM: &str = "pet.hud_layer.bottom";
const EXIT_APPLICATION: &str = "pet.exit_application";

/// Pet 菜单返回给窗口协调层的业务动作。
pub(crate) enum Action {
    /// 切换 Pet 置顶状态。
    SetPetLayer(WindowLayer),
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
    locale: Locale,
    pet_layer: WindowLayer,
    hud_layer: WindowLayer,
    hud_enabled: bool,
) -> MenuDefinition {
    MenuDefinition::new(vec![
        MenuItem::new(
            OPEN_SETTINGS,
            CatalogStore::t_shell(locale, MessageKey::MenuSettings),
        )
        .with_icon("brightness"),
        MenuItem::new(
            PET_LAYER,
            CatalogStore::t_shell(locale, MessageKey::SettingsPetLayer),
        )
        .with_submenu(MenuDefinition::new(vec![
            MenuItem::checkable(
                PET_LAYER_TOP,
                CatalogStore::t_shell(locale, MessageKey::MenuLayerTop),
                pet_layer == WindowLayer::AlwaysOnTop,
            ),
            MenuItem::checkable(
                PET_LAYER_NORMAL,
                CatalogStore::t_shell(locale, MessageKey::MenuLayerNormal),
                pet_layer == WindowLayer::Normal,
            ),
            MenuItem::checkable(
                PET_LAYER_BOTTOM,
                CatalogStore::t_shell(locale, MessageKey::MenuLayerBottom),
                pet_layer == WindowLayer::AlwaysOnBottom,
            ),
        ]))
        .with_icon("layers-subtract"),
        MenuItem::checkable(
            OPEN_HUD,
            CatalogStore::t_shell(locale, MessageKey::SettingsNavHud),
            hud_enabled,
        )
        .with_icon("puzzle")
        .with_separator_before(),
        MenuItem::new(
            HUD_LAYER,
            CatalogStore::t_shell(locale, MessageKey::MenuPluginLayer),
        )
        .with_enabled(hud_enabled)
        .with_submenu(MenuDefinition::new(vec![
            MenuItem::checkable(
                HUD_LAYER_TOP,
                CatalogStore::t_shell(locale, MessageKey::MenuLayerTop),
                hud_layer == WindowLayer::AlwaysOnTop,
            ),
            MenuItem::checkable(
                HUD_LAYER_NORMAL,
                CatalogStore::t_shell(locale, MessageKey::MenuLayerNormal),
                hud_layer == WindowLayer::Normal,
            ),
            MenuItem::checkable(
                HUD_LAYER_BOTTOM,
                CatalogStore::t_shell(locale, MessageKey::MenuLayerBottom),
                hud_layer == WindowLayer::AlwaysOnBottom,
            ),
        ]))
        .with_icon("layers-subtract"),
        MenuItem::new(
            HUD_LAYOUT,
            CatalogStore::t_shell(locale, MessageKey::MenuHudLayout),
        )
        .with_icon("window")
        .with_enabled(hud_enabled),
        MenuItem::new(
            EXIT_APPLICATION,
            CatalogStore::t_shell(locale, MessageKey::MenuQuit),
        )
        .with_icon("close")
        .with_separator_before(),
    ])
}

/// 将通用菜单返回的标识转换为 Pet 业务动作。
pub(crate) fn parse_action(id: &str) -> Option<Action> {
    match id {
        PET_LAYER_TOP => Some(Action::SetPetLayer(WindowLayer::AlwaysOnTop)),
        PET_LAYER_NORMAL => Some(Action::SetPetLayer(WindowLayer::Normal)),
        PET_LAYER_BOTTOM => Some(Action::SetPetLayer(WindowLayer::AlwaysOnBottom)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localized_menu_keeps_product_order_and_states() {
        let menu = definition(
            Locale::ZhCn,
            WindowLayer::AlwaysOnTop,
            WindowLayer::Normal,
            false,
        );
        let labels = menu.items.iter().map(|item| item.label).collect::<Vec<_>>();
        assert_eq!(
            labels,
            ["设置", "宠物层级", "插件", "插件层级", "插件布局", "退出"]
        );
        assert!(menu.items[1].submenu.is_some());
        assert!(!menu.items[2].checked);
        assert!(menu.items[2].separator_before);
        assert!(!menu.items[3].enabled);
        assert!(!menu.items[4].enabled);
        assert!(menu.items[5].separator_before);
        assert_eq!(menu.items[0].icon, Some("brightness"));
        assert_eq!(menu.items[1].icon, Some("layers-subtract"));
        assert_eq!(menu.items[2].icon, Some("puzzle"));
        assert_eq!(menu.items[3].icon, Some("layers-subtract"));
        assert_eq!(menu.items[4].icon, Some("window"));
        assert_eq!(menu.items[5].icon, Some("close"));

        let pet_layer_items = &menu.items[1]
            .submenu
            .as_ref()
            .expect("pet layer submenu")
            .items;
        assert!(pet_layer_items[0].checked);

        let layer_items = &menu.items[3]
            .submenu
            .as_ref()
            .expect("plugin layer submenu")
            .items;
        assert_eq!(
            layer_items
                .iter()
                .map(|item| item.label)
                .collect::<Vec<_>>(),
            ["置顶", "正常", "置底"]
        );
        assert!(layer_items[1].checked);
    }

    #[test]
    fn english_menu_uses_shell_catalog() {
        let menu = definition(
            Locale::En,
            WindowLayer::Normal,
            WindowLayer::AlwaysOnTop,
            true,
        );
        assert_eq!(menu.items[0].label, "Settings");
        assert_eq!(menu.items[3].label, "Plugin layer");
        assert_eq!(menu.items[5].label, "Quit");
    }
}
