//! 中性鼠标按键（无平台虚拟码）。

/// 宠物可感知的鼠标键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PetMouseButton {
    /// 左键 / 主按钮。
    Primary,
    /// 右键 / 次按钮。
    Secondary,
    /// 中键。
    Middle,
}
