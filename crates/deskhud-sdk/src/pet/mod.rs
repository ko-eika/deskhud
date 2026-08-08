//! 宠物包 Guest API（骨架）。
//!
//! Phase 3 将提供与 `deskhud-host::PetKind` / `PetEvent` 对齐的导出宏。
//! 扩展说明见仓库 `docs/extension-guide.md`。

/// 宠物 Guest 应实现的逻辑钩子（设计稿；WASM 导出尚未接线）。
pub trait PetGuest {
    /// 稳定 ID（与 manifest.id 一致）。
    fn id(&self) -> &str;

    /// 每帧或定时推进行为状态。
    fn tick(&mut self, _dt_secs: f32) {}

    /// 宿主事件入口（贴边 / 拖拽 / 键鼠）；形状将与 host `PetEvent` 对齐。
    fn on_event_placeholder(&mut self) {}
}
