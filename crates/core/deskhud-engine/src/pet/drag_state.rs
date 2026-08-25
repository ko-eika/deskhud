//! 宠窗拖拽交互状态（中性，无 HWND）。

/// 是否正在被用户拖动。
///
/// 日后可扩展速度等字段；社区包通过 [`crate::PetPaintCtx::drag`] / [`crate::PetEvent`] 消费。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DragState {
    /// 主按钮拖动进行中。
    pub active: bool,
}

impl DragState {
    /// 空闲。
    pub const IDLE: Self = Self { active: false };
    /// 拖动中。
    pub const ACTIVE: Self = Self { active: true };

    /// 是否正在拖动。
    pub fn is_dragging(self) -> bool {
        self.active
    }
}
