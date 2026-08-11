//! 将当前宠物契约转换为平台无关覆盖层场景。
//!
//! 此模块只保留既有圆形宠物的视觉语义；原生窗口、GDI 或 egui 细节不进入这里。

use deskhud_engine::{
    OverlayCircle, OverlayColor, OverlayDisplayTarget, OverlayHitKind, OverlayHitRegion,
    OverlayHitShape, OverlayPoint, OverlayScene, OverlayVisual, PetPaint,
};

const PUPIL_COLOR: OverlayColor = OverlayColor {
    red: 28,
    green: 32,
    blue: 40,
    alpha: 255,
};

/// 根据既有 [`PetPaint`] 生成原生与跨平台后端均可消费的最小宠物场景。
///
/// 对话气泡需文本原语支持，当前不在本阶段转换；调用方仍可在现有 egui 路径显示它。
pub(crate) fn scene_from_pet_paint(
    target: OverlayDisplayTarget,
    center: OverlayPoint,
    base_radius: f32,
    paint: &PetPaint,
    pupil_offset: [f32; 2],
) -> OverlayScene {
    let radius = (base_radius * finite_nonnegative(paint.bounce)).max(1.0);
    let mut visuals = vec![OverlayVisual::Circle(OverlayCircle {
        id: "pet.body".into(),
        center,
        radius,
        color: color_from_rgb(paint.body_rgb),
    })];

    if paint.draw_eyes {
        let eye_y = -radius * 0.12;
        let eye_x = radius * 0.28;
        let eye_radius = radius * 0.16;
        let pupil_radius = eye_radius * 0.48;
        let left_eye = OverlayPoint {
            x: center.x - eye_x,
            y: center.y + eye_y,
        };
        let right_eye = OverlayPoint {
            x: center.x + eye_x,
            y: center.y + eye_y,
        };
        let pupil_offset = OverlayPoint {
            x: finite_or_zero(pupil_offset[0]),
            y: finite_or_zero(pupil_offset[1]),
        };
        let eye_color = color_from_rgb(paint.eye_rgb);
        visuals.extend([
            circle("pet.eye.left", left_eye, eye_radius, eye_color),
            circle("pet.eye.right", right_eye, eye_radius, eye_color),
            circle(
                "pet.pupil.left",
                add(left_eye, pupil_offset),
                pupil_radius,
                PUPIL_COLOR,
            ),
            circle(
                "pet.pupil.right",
                add(right_eye, pupil_offset),
                pupil_radius,
                PUPIL_COLOR,
            ),
        ]);
    }

    OverlayScene {
        target,
        visuals,
        hit_regions: vec![OverlayHitRegion {
            id: "pet.body".into(),
            shape: pet_hit_shape(center, radius),
            kind: OverlayHitKind::Interactive,
        }],
    }
}

/// 返回宠物身体对应的命中形状，供平台在不栅格化整帧时执行命中判断。
pub(crate) fn pet_hit_shape(center: OverlayPoint, radius: f32) -> OverlayHitShape {
    OverlayHitShape::Circle { center, radius }
}

fn circle(id: &str, center: OverlayPoint, radius: f32, color: OverlayColor) -> OverlayVisual {
    OverlayVisual::Circle(OverlayCircle {
        id: id.into(),
        center,
        radius,
        color,
    })
}

fn add(point: OverlayPoint, offset: OverlayPoint) -> OverlayPoint {
    OverlayPoint {
        x: point.x + offset.x,
        y: point.y + offset.y,
    }
}

fn color_from_rgb(rgb: [f32; 3]) -> OverlayColor {
    OverlayColor {
        red: unit_to_u8(rgb[0]),
        green: unit_to_u8(rgb[1]),
        blue: unit_to_u8(rgb[2]),
        alpha: 255,
    }
}

fn unit_to_u8(value: f32) -> u8 {
    (finite_or_zero(value).clamp(0.0, 1.0) * 255.0).round() as u8
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        1.0
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use deskhud_engine::{OverlayDisplayTarget, OverlayPoint, PetPaint};

    use super::scene_from_pet_paint;

    #[test]
    fn converts_body_eyes_and_interactive_body_region() {
        let scene = scene_from_pet_paint(
            OverlayDisplayTarget::Display("primary".into()),
            OverlayPoint { x: 100.0, y: 80.0 },
            40.0,
            &PetPaint::default(),
            [0.0, 0.0],
        );
        assert_eq!(scene.visuals.len(), 5);
        assert_eq!(scene.hit_regions.len(), 1);
        assert!(scene.hit_regions[0].contains(OverlayPoint { x: 100.0, y: 80.0 }));
    }
}
