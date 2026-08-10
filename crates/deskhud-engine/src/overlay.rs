//! 平台无关的桌面覆盖层契约。
//!
//! 该模块只描述目标显示器、命中区域与后端能力；它不选择窗口系统、渲染器或
//! 输入 API。这样宠物包和 HUD 插件无需了解 HWND、Cocoa 或 Wayland。

/// 覆盖层要绘制到的显示器范围。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayDisplayTarget {
    /// 一台由稳定平台标识符指定的显示器。
    Display(String),
    /// 所有显示器构成的虚拟桌面。
    ///
    /// 仅当后端明确报告支持时才可使用；它必须处理混合 DPI、负坐标和热插拔。
    VirtualDesktop,
}

/// 逻辑坐标中的二维点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayPoint {
    /// 横坐标。
    pub x: f32,
    /// 纵坐标。
    pub y: f32,
}

/// 逻辑坐标中的轴对齐矩形。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayRect {
    /// 左上角。
    pub origin: OverlayPoint,
    /// 宽度；调用方应提供非负值。
    pub width: f32,
    /// 高度；调用方应提供非负值。
    pub height: f32,
}

/// 命中区域的交互语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayHitKind {
    /// 宠物或 HUD 控件消费指针输入。
    Interactive,
    /// 只绘制信息，必须让指针落到下层应用。
    Passthrough,
}

/// 一个由壳解释的命中区域。
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayHitRegion {
    /// 用于将平台事件路由回宠物或 HUD 条目的稳定标识。
    pub id: String,
    /// 区域在目标显示器逻辑坐标中的范围。
    pub bounds: OverlayRect,
    /// 区域是否消费指针输入。
    pub kind: OverlayHitKind,
}

/// 交给平台后端的单帧覆盖层状态。
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayScene {
    /// 该帧目标显示器。
    pub target: OverlayDisplayTarget,
    /// 需要平台按区域路由的输入范围。
    pub hit_regions: Vec<OverlayHitRegion>,
}

/// 平台后端显式声明的桌面覆盖层能力。
///
/// 壳必须依据这些标记选择正式模式或降级模式，不能假定各操作系统都有相同的
/// 透明、置顶及跨应用指针穿透行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OverlayBackendCapabilities {
    /// 可创建桌面级透明覆盖层。
    pub desktop_transparency: bool,
    /// 可将空白或被动区域的输入交给其他应用。
    pub per_region_passthrough: bool,
    /// 可安全覆盖用户选择的一台显示器。
    pub selected_display: bool,
    /// 可安全覆盖包含混合 DPI 与负坐标的虚拟桌面。
    pub virtual_desktop: bool,
}
