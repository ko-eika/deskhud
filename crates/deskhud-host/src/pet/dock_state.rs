//! 宠窗相对显示器工作区的贴边状态（中性，无 HWND）。

/// 四边贴靠标志；可同时为真（角落）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DockState {
    /// 贴工作区左边。
    pub left: bool,
    /// 贴工作区右边。
    pub right: bool,
    /// 贴工作区顶边。
    pub top: bool,
    /// 贴工作区底边。
    pub bottom: bool,
}

impl DockState {
    /// 未贴任何边。
    pub const FREE: Self = Self {
        left: false,
        right: false,
        top: false,
        bottom: false,
    };

    /// 是否完全自由（未贴边）。
    pub fn is_free(self) -> bool {
        !self.left && !self.right && !self.top && !self.bottom
    }

    /// 是否贴在角落（至少两条相邻边）。
    pub fn is_corner(self) -> bool {
        (self.left || self.right) && (self.top || self.bottom)
    }
}
