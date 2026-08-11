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

/// 与渲染器无关的 RGBA 颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayColor {
    /// 红色通道。
    pub red: u8,
    /// 绿色通道。
    pub green: u8,
    /// 蓝色通道。
    pub blue: u8,
    /// 不透明度。
    pub alpha: u8,
}

/// 场景中的圆形绘制原语。
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayCircle {
    /// 用于诊断与事件映射的稳定标识。
    pub id: String,
    /// 圆心的逻辑坐标。
    pub center: OverlayPoint,
    /// 半径；调用方应提供非负值。
    pub radius: f32,
    /// 填充颜色。
    pub color: OverlayColor,
}

/// An axis-aligned solid ellipse in an overlay scene.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayEllipse {
    /// Stable identifier for diagnostics and event mapping.
    pub id: String,
    /// Center in logical coordinates.
    pub center: OverlayPoint,
    /// Horizontal radius; callers provide a non-negative value.
    pub radius_x: f32,
    /// Vertical radius; callers provide a non-negative value.
    pub radius_y: f32,
    /// Fill color.
    pub color: OverlayColor,
}

/// 场景中的纯色圆角矩形绘制原语。
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayRoundedRect {
    /// 用于诊断的稳定标识。
    pub id: String,
    /// 矩形范围。
    pub rect: OverlayRect,
    /// 圆角半径。
    pub corner_radius: f32,
    /// 填充颜色。
    pub color: OverlayColor,
}

/// 场景中的单行居中文本绘制原语。
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayText {
    /// 用于诊断的稳定标识。
    pub id: String,
    /// 文本布局范围。
    pub rect: OverlayRect,
    /// UTF-8 文本内容。
    pub text: String,
    /// 字号（逻辑像素）。
    pub font_size: f32,
    /// 文本颜色。
    pub color: OverlayColor,
}

/// 覆盖层后端必须支持的最小绘制原语。
///
/// 这是场景描述而非图形 API；平台后端可用 GDI、Direct2D、Core Animation 或其它
/// 实现将其栅格化。
#[derive(Debug, Clone, PartialEq)]
pub enum OverlayVisual {
    /// 纯色圆形。
    Circle(OverlayCircle),
    /// Solid axis-aligned ellipse.
    Ellipse(OverlayEllipse),
    /// 纯色圆角矩形。
    RoundedRect(OverlayRoundedRect),
    /// 单行居中文本。
    Text(OverlayText),
}

/// 命中区域的交互语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayHitKind {
    /// 宠物或 HUD 控件消费指针输入。
    Interactive,
    /// 只绘制信息，必须让指针落到下层应用。
    Passthrough,
}

/// 命中区域的几何形状。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverlayHitShape {
    /// 轴对齐矩形。
    Rect(OverlayRect),
    /// 圆形；坐标与半径均使用目标显示器逻辑坐标。
    Circle {
        /// 圆心。
        center: OverlayPoint,
        /// 半径；调用方应提供非负值。
        radius: f32,
    },
}

impl OverlayHitShape {
    /// 判断逻辑坐标点是否位于形状内。
    pub fn contains(self, point: OverlayPoint) -> bool {
        match self {
            Self::Rect(rect) => {
                point.x >= rect.origin.x
                    && point.x <= rect.origin.x + rect.width
                    && point.y >= rect.origin.y
                    && point.y <= rect.origin.y + rect.height
            }
            Self::Circle { center, radius } => {
                let dx = point.x - center.x;
                let dy = point.y - center.y;
                dx * dx + dy * dy <= radius * radius
            }
        }
    }
}

/// 一个由壳解释的命中区域。
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayHitRegion {
    /// 用于将平台事件路由回宠物或 HUD 条目的稳定标识。
    pub id: String,
    /// 区域在目标显示器逻辑坐标中的几何形状。
    pub shape: OverlayHitShape,
    /// 区域是否消费指针输入。
    pub kind: OverlayHitKind,
}

impl OverlayHitRegion {
    /// 判断逻辑坐标点是否命中该区域。
    pub fn contains(&self, point: OverlayPoint) -> bool {
        self.shape.contains(point)
    }
}

/// 交给平台后端的单帧覆盖层状态。
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayScene {
    /// 该帧目标显示器。
    pub target: OverlayDisplayTarget,
    /// 后端需要绘制的、按顺序叠放的视觉原语。
    pub visuals: Vec<OverlayVisual>,
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

#[cfg(test)]
mod tests {
    use super::{OverlayHitShape, OverlayPoint, OverlayRect};

    #[test]
    fn circle_includes_boundary_and_excludes_corner() {
        let shape = OverlayHitShape::Circle {
            center: OverlayPoint { x: 10.0, y: 10.0 },
            radius: 5.0,
        };
        assert!(shape.contains(OverlayPoint { x: 15.0, y: 10.0 }));
        assert!(!shape.contains(OverlayPoint { x: 14.0, y: 14.0 }));
    }

    #[test]
    fn rectangle_includes_edges() {
        let shape = OverlayHitShape::Rect(OverlayRect {
            origin: OverlayPoint { x: 2.0, y: 3.0 },
            width: 4.0,
            height: 5.0,
        });
        assert!(shape.contains(OverlayPoint { x: 6.0, y: 8.0 }));
        assert!(!shape.contains(OverlayPoint { x: 6.1, y: 8.0 }));
    }
}
