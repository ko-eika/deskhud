//! 平台无关的宠物场景帧契约。

#![allow(missing_docs)]

/// 场景资源的稳定包内标识；渲染器负责把它解析为实际资源。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetId(pub String);

/// 二维仿射变换（平移、旋转、缩放）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub translation: [f32; 2],
    pub rotation_radians: f32,
    pub scale: [f32; 2],
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0],
            rotation_radians: 0.0,
            scale: [1.0, 1.0],
        }
    }
}

/// RGBA 颜色，分量应在 `0..=1`。
pub type SceneColor = [f32; 4];

/// 基础矢量路径。
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub points: Vec<[f32; 2]>,
    pub closed: bool,
    pub fill: Option<SceneColor>,
    pub stroke: Option<SceneColor>,
    /// 描边宽度，使用与 `points` 相同的场景坐标单位。
    pub stroke_width: f32,
}

/// 基础图形，不含任何宠物特征语义。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Circle { radius: f32 },
    Rect { size: [f32; 2], corner_radius: f32 },
    Ellipse { radii: [f32; 2] },
}

/// 一个场景节点；`z_index` 决定同帧绘制顺序。
#[derive(Debug, Clone, PartialEq)]
pub enum SceneNode {
    Sprite {
        asset: AssetId,
        size: [f32; 2],
        opacity: f32,
    },
    AtlasFrame {
        asset: AssetId,
        frame: u32,
        size: [f32; 2],
        opacity: f32,
    },
    Path(Path),
    GradientPath {
        path: Path,
        top_color: SceneColor,
        bottom_color: SceneColor,
    },
    Shape {
        shape: Shape,
        color: SceneColor,
    },
    Text {
        text: String,
        color: SceneColor,
        size: f32,
    },
    Bubble {
        text: String,
        color: SceneColor,
        background: SceneColor,
        corner_radius: f32,
    },
    HitRegion {
        shape: Shape,
    },
}

/// 带变换和层级的节点。
#[derive(Debug, Clone, PartialEq)]
pub struct SceneItem {
    pub transform: Transform2D,
    pub z_index: i32,
    pub node: SceneNode,
}

/// 一帧宠物场景；由宠物程序逐帧生成，宿主只校验并解释它。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PetScene {
    pub items: Vec<SceneItem>,
}

/// 场景校验失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneValidationError {
    TooManyNodes,
    TextTooLong,
    InvalidNumber,
    InvalidGeometry,
}

impl PetScene {
    pub const MAX_NODES: usize = 4096;
    pub const MAX_TEXT_LENGTH: usize = 4096;

    /// 校验节点数量、字符串、坐标、尺寸和颜色，防止异常包破坏渲染器。
    pub fn validate(&self) -> Result<(), SceneValidationError> {
        if self.items.len() > Self::MAX_NODES {
            return Err(SceneValidationError::TooManyNodes);
        }
        for item in &self.items {
            if !valid_transform(item.transform) {
                return Err(SceneValidationError::InvalidNumber);
            }
            validate_node(&item.node)?;
        }
        Ok(())
    }

    /// Tests a point in scene coordinates against the package-declared hit regions.
    pub fn hit_test(&self, point: [f32; 2]) -> bool {
        self.items.iter().any(|item| {
            let SceneNode::HitRegion { shape } = &item.node else {
                return false;
            };
            let x = (point[0] - item.transform.translation[0])
                / item.transform.scale[0].abs().max(f32::EPSILON);
            let y = (point[1] - item.transform.translation[1])
                / item.transform.scale[1].abs().max(f32::EPSILON);
            match shape {
                Shape::Circle { radius } => x * x + y * y <= radius * radius,
                Shape::Ellipse { radii } => {
                    (x / radii[0].max(f32::EPSILON)).powi(2)
                        + (y / radii[1].max(f32::EPSILON)).powi(2)
                        <= 1.0
                }
                Shape::Rect { size, .. } => x.abs() <= size[0] / 2.0 && y.abs() <= size[1] / 2.0,
            }
        })
    }
}

fn valid_transform(t: Transform2D) -> bool {
    t.translation.iter().all(|v| v.is_finite())
        && t.rotation_radians.is_finite()
        && t.scale
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
}
fn valid_color(c: SceneColor) -> bool {
    c.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v))
}
fn valid_size(s: [f32; 2]) -> bool {
    s.iter()
        .all(|v| v.is_finite() && *v >= 0.0 && *v <= 1_000_000.0)
}
fn valid_shape(s: &Shape) -> bool {
    match s {
        Shape::Circle { radius } => valid_size([*radius, 0.0]),
        Shape::Rect {
            size,
            corner_radius,
        } => valid_size(*size) && valid_size([*corner_radius, 0.0]),
        Shape::Ellipse { radii } => valid_size(*radii),
    }
}
fn validate_node(node: &SceneNode) -> Result<(), SceneValidationError> {
    let (text, colors, geometry_ok) = match node {
        SceneNode::Sprite { size, opacity, .. } | SceneNode::AtlasFrame { size, opacity, .. } => (
            None,
            &[][..],
            valid_size(*size) && opacity.is_finite() && (0.0..=1.0).contains(opacity),
        ),
        SceneNode::Path(p) => (
            None,
            &[][..],
            p.points.iter().all(|p| p.iter().all(|v| v.is_finite()))
                && p.stroke_width.is_finite()
                && p.stroke_width >= 0.0
                && p.fill.is_none_or(valid_color)
                && p.stroke.is_none_or(valid_color),
        ),
        SceneNode::GradientPath {
            path,
            top_color,
            bottom_color,
        } => (
            None,
            &[(*top_color), (*bottom_color)][..],
            path.points.iter().all(|p| p.iter().all(|v| v.is_finite()))
                && path.stroke_width.is_finite()
                && path.stroke_width >= 0.0
                && path.fill.is_none_or(valid_color)
                && path.stroke.is_none_or(valid_color),
        ),
        SceneNode::Shape { shape, color } => {
            (None, std::slice::from_ref(color), valid_shape(shape))
        }
        SceneNode::Text { text, color, size } => (
            Some(text),
            std::slice::from_ref(color),
            size.is_finite() && *size >= 0.0,
        ),
        SceneNode::Bubble {
            text,
            color,
            background,
            corner_radius,
        } => (
            Some(text),
            &[(*color), (*background)][..],
            corner_radius.is_finite() && *corner_radius >= 0.0,
        ),
        SceneNode::HitRegion { shape } => (None, &[][..], valid_shape(shape)),
    };
    if text.is_some_and(|s| s.chars().count() > PetScene::MAX_TEXT_LENGTH) {
        return Err(SceneValidationError::TextTooLong);
    }
    if !colors.iter().all(|c| valid_color(*c)) {
        return Err(SceneValidationError::InvalidNumber);
    }
    if !geometry_ok {
        return Err(SceneValidationError::InvalidGeometry);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_vector_and_atlas_nodes() {
        let scene = PetScene {
            items: vec![
                SceneItem {
                    transform: Transform2D::default(),
                    z_index: 0,
                    node: SceneNode::Shape {
                        shape: Shape::Circle { radius: 10.0 },
                        color: [1.0, 0.0, 0.0, 1.0],
                    },
                },
                SceneItem {
                    transform: Transform2D::default(),
                    z_index: 1,
                    node: SceneNode::AtlasFrame {
                        asset: AssetId("pet/run".into()),
                        frame: 2,
                        size: [32.0, 32.0],
                        opacity: 1.0,
                    },
                },
            ],
        };
        assert!(scene.validate().is_ok());
    }
    #[test]
    fn rejects_oversized_text() {
        let scene = PetScene {
            items: vec![SceneItem {
                transform: Transform2D::default(),
                z_index: 0,
                node: SceneNode::Text {
                    text: "x".repeat(PetScene::MAX_TEXT_LENGTH + 1),
                    color: [0.0; 4],
                    size: 12.0,
                },
            }],
        };
        assert_eq!(scene.validate(), Err(SceneValidationError::TextTooLong));
    }

    #[test]
    fn hit_test_uses_declared_region_only() {
        let scene = PetScene {
            items: vec![SceneItem {
                transform: Transform2D::default(),
                z_index: 0,
                node: SceneNode::HitRegion {
                    shape: Shape::Circle { radius: 1.0 },
                },
            }],
        };
        assert!(scene.hit_test([0.5, 0.0]));
        assert!(!scene.hit_test([1.1, 0.0]));
    }
}
