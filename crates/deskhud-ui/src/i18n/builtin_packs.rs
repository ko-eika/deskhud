//! 内置宠 / 演示插件的多语言文案（键与包约定一致：`pet|hud.<id>.*`）。

use std::collections::BTreeMap;

use super::CatalogStore;

/// 把内置包文案写入目录（zh-CN + en）。
pub fn seed_builtin_packs(store: &mut CatalogStore) {
    store.merge_layer("zh-CN", &zh_cn());
    store.merge_layer("en", &en());
}

fn zh_cn() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    // pet.deskhud.specs
    m.insert("pet.deskhud.specs.display_name".into(), "大眼球".into());
    m.insert(
        "pet.deskhud.specs.description".into(),
        "全局跟鼠标看；键鼠短提示；悬停高亮".into(),
    );
    m.insert(
        "pet.deskhud.specs.follow_eyes.label".into(),
        "眼睛跟随指针".into(),
    );
    m.insert(
        "pet.deskhud.specs.follow_eyes.description".into(),
        "瞳孔跟随桌面光标方向转动".into(),
    );
    m.insert(
        "pet.deskhud.specs.key_tips.label".into(),
        "按键提示".into(),
    );
    m.insert(
        "pet.deskhud.specs.key_tips.description".into(),
        "键盘按下时显示短气泡（如 Ctrl+C）".into(),
    );
    m.insert(
        "pet.deskhud.specs.mouse_tips.label".into(),
        "鼠标提示".into(),
    );
    m.insert(
        "pet.deskhud.specs.mouse_tips.description".into(),
        "全局鼠标按键 / 滚轮时显示短气泡".into(),
    );
    m.insert(
        "pet.deskhud.specs.hover_highlight.label".into(),
        "悬停高亮".into(),
    );
    m.insert(
        "pet.deskhud.specs.hover_highlight.description".into(),
        "指针停在宠上时身体略提亮".into(),
    );
    m.insert(
        "pet.deskhud.specs.dock_tint.label".into(),
        "贴边变色".into(),
    );
    m.insert(
        "pet.deskhud.specs.dock_tint.description".into(),
        "吸附屏幕边缘时改变身体颜色".into(),
    );

    // pet.deskhud.blob
    m.insert("pet.deskhud.blob.display_name".into(), "蓝点".into());
    m.insert(
        "pet.deskhud.blob.description".into(),
        "简洁圆点；拖动/贴边略变形态".into(),
    );
    m.insert(
        "pet.deskhud.blob.hover_pulse.label".into(),
        "悬停轻弹".into(),
    );
    m.insert(
        "pet.deskhud.blob.hover_pulse.description".into(),
        "指针停在宠上时略微放大呼吸".into(),
    );
    m.insert(
        "pet.deskhud.blob.dock_tint.label".into(),
        "贴边变色".into(),
    );
    m.insert(
        "pet.deskhud.blob.dock_tint.description".into(),
        "吸附边缘时身体颜色变化".into(),
    );
    m.insert(
        "pet.deskhud.blob.drag_react.label".into(),
        "拖动反馈".into(),
    );
    m.insert(
        "pet.deskhud.blob.drag_react.description".into(),
        "拖动时加强弹跳与提亮".into(),
    );

    // hud.deskhud.demo
    m.insert("hud.deskhud.demo.display_name".into(), "演示 HUD".into());
    m.insert(
        "hud.deskhud.demo.description".into(),
        "示例插件：开关后在宠窗底部显示演示条（非真实数据源）".into(),
    );
    m.insert(
        "hud.deskhud.demo.clock.label".into(),
        "演示时钟条".into(),
    );
    m.insert(
        "hud.deskhud.demo.tip.label".into(),
        "演示提示条".into(),
    );
    m
}

fn en() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("pet.deskhud.specs.display_name".into(), "Big Eyes".into());
    m.insert(
        "pet.deskhud.specs.description".into(),
        "Follows the cursor; key/mouse tip bubbles; hover highlight".into(),
    );
    m.insert(
        "pet.deskhud.specs.follow_eyes.label".into(),
        "Eyes follow pointer".into(),
    );
    m.insert(
        "pet.deskhud.specs.follow_eyes.description".into(),
        "Pupils track desktop cursor direction".into(),
    );
    m.insert(
        "pet.deskhud.specs.key_tips.label".into(),
        "Key tips".into(),
    );
    m.insert(
        "pet.deskhud.specs.key_tips.description".into(),
        "Show short bubbles on key press (e.g. Ctrl+C)".into(),
    );
    m.insert(
        "pet.deskhud.specs.mouse_tips.label".into(),
        "Mouse tips".into(),
    );
    m.insert(
        "pet.deskhud.specs.mouse_tips.description".into(),
        "Show short bubbles for global mouse buttons / wheel".into(),
    );
    m.insert(
        "pet.deskhud.specs.hover_highlight.label".into(),
        "Hover highlight".into(),
    );
    m.insert(
        "pet.deskhud.specs.hover_highlight.description".into(),
        "Slightly brighten the body while hovered".into(),
    );
    m.insert(
        "pet.deskhud.specs.dock_tint.label".into(),
        "Dock tint".into(),
    );
    m.insert(
        "pet.deskhud.specs.dock_tint.description".into(),
        "Change body color when snapped to screen edges".into(),
    );

    m.insert("pet.deskhud.blob.display_name".into(), "Blue Dot".into());
    m.insert(
        "pet.deskhud.blob.description".into(),
        "Simple blob; reacts to drag and docking".into(),
    );
    m.insert(
        "pet.deskhud.blob.hover_pulse.label".into(),
        "Hover pulse".into(),
    );
    m.insert(
        "pet.deskhud.blob.hover_pulse.description".into(),
        "Slight scale pulse while hovered".into(),
    );
    m.insert(
        "pet.deskhud.blob.dock_tint.label".into(),
        "Dock tint".into(),
    );
    m.insert(
        "pet.deskhud.blob.dock_tint.description".into(),
        "Body tint changes when docked".into(),
    );
    m.insert(
        "pet.deskhud.blob.drag_react.label".into(),
        "Drag feedback".into(),
    );
    m.insert(
        "pet.deskhud.blob.drag_react.description".into(),
        "Stronger bounce and brightness while dragging".into(),
    );

    m.insert("hud.deskhud.demo.display_name".into(), "Demo HUD".into());
    m.insert(
        "hud.deskhud.demo.description".into(),
        "Sample plugin: demo strips at the bottom of the pet window".into(),
    );
    m.insert(
        "hud.deskhud.demo.clock.label".into(),
        "Demo clock strip".into(),
    );
    m.insert(
        "hud.deskhud.demo.tip.label".into(),
        "Demo tip strip".into(),
    );
    m
}
