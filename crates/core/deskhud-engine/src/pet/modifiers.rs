//! 修饰键快照。

/// Shift / Ctrl / Alt / Win（宿主归一化）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PetModifiers {
    /// Shift。
    pub shift: bool,
    /// Ctrl / Control。
    pub ctrl: bool,
    /// Alt / Option。
    pub alt: bool,
    /// Win / Meta / Super。
    pub meta: bool,
}

impl PetModifiers {
    /// 无修饰。
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
        meta: false,
    };

    /// 是否有任一修饰键。
    pub fn any(self) -> bool {
        self.shift || self.ctrl || self.alt || self.meta
    }
}
