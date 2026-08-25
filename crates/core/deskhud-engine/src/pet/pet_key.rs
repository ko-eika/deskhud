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
    /// Insert（含 NumLock 关闭时的小键盘 0）。
    Insert,
    /// Clear（常见于 NumLock 关闭时的小键盘 5）。
    Clear,
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
    /// NumLock。
    NumLock,
    /// 小键盘 Enter。
    NumpadEnter,
    /// 小键盘数字 0..=9。
    NumpadDigit(u8),
    /// 小键盘加号。
    NumpadAdd,
    /// 小键盘减号。
    NumpadSubtract,
    /// 小键盘乘号。
    NumpadMultiply,
    /// 小键盘除号。
    NumpadDivide,
    /// 小键盘小数点。
    NumpadDecimal,
    /// 小键盘分隔符（取决于键盘布局）。
    NumpadSeparator,
    /// F1..=F12。
    Function(u8),
    /// 字母 A..=Z（统一大写）。
    Letter(char),
    /// 数字 '0'..='9'。
    Digit(char),
    /// 标点（按键主铭文，US 布局未按下 Shift 时的字符）。
    Punct(char),
}
