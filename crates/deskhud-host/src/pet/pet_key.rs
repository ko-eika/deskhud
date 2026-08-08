//! 中性按键（宠窗获焦转发 / 全局子集采样）。

/// 宠物可感知的按键；未列出的键不转发。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PetKey {
    /// Escape。
    Escape,
    /// Tab。
    Tab,
    /// Enter / Return。
    Enter,
    /// Space。
    Space,
    /// Backspace。
    Backspace,
    /// Delete。
    Delete,
    /// 方向上。
    ArrowUp,
    /// 方向下。
    ArrowDown,
    /// 方向左。
    ArrowLeft,
    /// 方向右。
    ArrowRight,
    /// Home。
    Home,
    /// End。
    End,
    /// PageUp。
    PageUp,
    /// PageDown。
    PageDown,
    /// 左/右 Shift（合并为一种）。
    Shift,
    /// 左/右 Ctrl。
    Ctrl,
    /// 左/右 Alt。
    Alt,
    /// Win / Meta。
    Super,
    /// CapsLock。
    CapsLock,
    /// F1..=F12。
    Function(u8),
    /// 字母 A..=Z（统一大写）。
    Letter(char),
    /// 数字 '0'..='9'。
    Digit(char),
    /// 标点（按键主铭文，US 布局未按下 Shift 时的字符）。
    Punct(char),
}
