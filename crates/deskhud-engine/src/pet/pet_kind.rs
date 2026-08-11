//! [`PetKind`]：可替换的桌宠外观 / 轻行为。

use std::collections::HashMap;

use super::{
    DockState, DragState, MouseState, PetConfigOption, PetEvent, PetKindInfo, PetPaint, PetTheme,
};

/// 当前帧可读的宠配置（短键 → 布尔；由壳从 `[pet.config]` 解析）。
#[derive(Debug, Clone, Copy)]
pub struct PetConfigBag<'a> {
    map: &'a HashMap<String, bool>,
}

impl<'a> PetConfigBag<'a> {
    /// 包装已解析的短键表。
    pub fn new(map: &'a HashMap<String, bool>) -> Self {
        Self { map }
    }

    /// 读取；缺省 `default`。
    pub fn get(self, key: &str, default: bool) -> bool {
        self.map.get(key).copied().unwrap_or(default)
    }
}

/// 一帧绘制上下文（由 UI 壳填充）。
///
/// 扩展行为请优先读 `dock` / `drag` / `mouse` / `config` 等中性状态，或在 [`PetKind::on_event`] 里跟变化。
#[derive(Debug, Clone, Copy)]
pub struct PetPaintCtx<'a> {
    /// 运行时间（秒）。
    pub time_secs: f64,
    /// 指针相对宠窗中心的方向，大致落在 [-1, 1]。
    ///
    /// **始终来自桌面全局光标**（不要求悬停在宠上）；默认宠「眼睛跟着鼠标」即读此字段。
    pub pointer_dir: [f32; 2],
    /// 宿主状态短文案（插件 HUD 摘要等，可空）。
    pub status_line: &'a str,
    /// 当前贴边状态（壳根据工作区几何计算）。
    pub dock: DockState,
    /// 当前拖拽状态。
    pub drag: DragState,
    /// 鼠标快照：含宠上局部交互 + 桌面全局按键。
    pub mouse: MouseState,
    /// 当前宠的布尔配置（短键）。
    pub config: PetConfigBag<'a>,
    /// 宿主已解析的明暗方案，供宠物决定是否跟随主题绘制气泡等附属内容。
    pub theme: PetTheme,
}

/// 桌宠类型扩展点。
///
/// 社区包日后经 runtime 适配实现同一 trait；勿在实现里依赖 HWND / 屏幕坐标。
pub trait PetKind: Send + Sync {
    /// 元数据。
    fn info(&self) -> PetKindInfo;

    /// 设置页可调选项（缺省无）。
    fn config_options(&self) -> &'static [PetConfigOption] {
        &[]
    }

    /// 壳在每帧事件前同步配置（便于 `on_event` 读取开关）。
    fn apply_config(&self, _config: PetConfigBag<'_>) {}

    /// 自主状态 / 动画推进（默认空）。
    fn tick(&self, _dt_secs: f32) {}

    /// 响应宿主事件（默认空）。需要可变状态时用内部 `Mutex` 等。
    fn on_event(&self, _event: PetEvent) {}

    /// 根据上下文生成一帧外观。
    fn paint(&self, ctx: PetPaintCtx<'_>) -> PetPaint;
}
