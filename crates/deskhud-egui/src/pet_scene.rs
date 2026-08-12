//! 将当前宠物契约转换为平台无关覆盖层场景。
//!
//! 此模块只保留既有圆形宠物的视觉语义；原生窗口、GDI 或 egui 细节不进入这里。

#![cfg_attr(not(windows), allow(dead_code))]

use deskhud_engine::{
    OverlayCircle, OverlayColor, OverlayDisplayTarget, OverlayEllipse, OverlayHitKind,
    OverlayHitRegion, OverlayHitShape, OverlayPoint, OverlayRect, OverlayRoundedRect, OverlayScene,
    OverlayText, OverlayVisual, PetBubbleStyle, PetPaint, PetTheme,
};
use deskhud_ui::GraphicsPreferences;

const PUPIL_COLOR: OverlayColor = OverlayColor {
    red: 28,
    green: 32,
    blue: 40,
    alpha: 255,
};

/// 根据既有 [`PetPaint`] 生成原生与跨平台后端均可消费的最小宠物场景。
///
/// 宠物身体与眼睛转换为中性绘制原语；对话气泡由独立覆盖窗消费。
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
        let eye_open = finite_or_zero(paint.eye_open).clamp(0.0, 1.0);
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
        if eye_open <= 0.06 {
            let eyelid_radius_x = eye_radius * 0.76;
            visuals.extend([
                ellipse(
                    "pet.eyelid.left",
                    left_eye,
                    eyelid_radius_x,
                    1.25,
                    PUPIL_COLOR,
                ),
                ellipse(
                    "pet.eyelid.right",
                    right_eye,
                    eyelid_radius_x,
                    1.25,
                    PUPIL_COLOR,
                ),
            ]);
        } else {
            let eye_radius_y = eye_radius * eye_open;
            let pupil_radius_y = pupil_radius * eye_open;
            visuals.extend([
                ellipse(
                    "pet.eye.left",
                    left_eye,
                    eye_radius,
                    eye_radius_y,
                    eye_color,
                ),
                ellipse(
                    "pet.eye.right",
                    right_eye,
                    eye_radius,
                    eye_radius_y,
                    eye_color,
                ),
                ellipse(
                    "pet.pupil.left",
                    add(left_eye, pupil_offset),
                    pupil_radius,
                    pupil_radius_y,
                    PUPIL_COLOR,
                ),
                ellipse(
                    "pet.pupil.right",
                    add(right_eye, pupil_offset),
                    pupil_radius,
                    pupil_radius_y,
                    PUPIL_COLOR,
                ),
            ]);
        }
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

/// Build a non-interactive dialogue scene for the host-owned transparent popup.
pub(crate) fn dialogue_scene_from_pet_paint(
    target: OverlayDisplayTarget,
    size: [f32; 2],
    paint: &PetPaint,
    theme: PetTheme,
    graphics: GraphicsPreferences,
) -> Option<OverlayScene> {
    if !graphics.effects {
        return None;
    }
    paint
        .bubble_text
        .as_deref()
        .filter(|text| !text.is_empty())
        .map(|text| {
            let bubble_rect = OverlayRect {
                origin: OverlayPoint { x: 3.0, y: 3.0 },
                width: (size[0] - 6.0).max(1.0),
                height: (size[1] - 6.0).max(1.0),
            };
            let (bubble_color, text_color, corner_radius) = bubble_appearance(paint, theme);
            let visuals = vec![
                OverlayVisual::RoundedRect(OverlayRoundedRect {
                    id: "pet.bubble.shadow".into(),
                    rect: OverlayRect {
                        origin: OverlayPoint { x: 5.0, y: 5.0 },
                        width: bubble_rect.width,
                        height: bubble_rect.height,
                    },
                    corner_radius: corner_radius + 1.0,
                    color: OverlayColor {
                        red: 0,
                        green: 0,
                        blue: 0,
                        alpha: 48,
                    },
                }),
                OverlayVisual::RoundedRect(OverlayRoundedRect {
                    id: "pet.bubble.background".into(),
                    rect: bubble_rect,
                    corner_radius,
                    color: bubble_color,
                }),
                OverlayVisual::Text(OverlayText {
                    id: "pet.bubble.text".into(),
                    rect: bubble_rect,
                    text: text.chars().take(18).collect(),
                    font_size: 14.0,
                    color: text_color,
                }),
            ];
            OverlayScene {
                target,
                visuals,
                hit_regions: Vec::new(),
            }
        })
}

fn bubble_appearance(paint: &PetPaint, theme: PetTheme) -> (OverlayColor, OverlayColor, f32) {
    match paint.bubble_style {
        PetBubbleStyle::FollowTheme => match theme {
            PetTheme::Light => (
                OverlayColor {
                    red: 248,
                    green: 248,
                    blue: 252,
                    alpha: 242,
                },
                OverlayColor {
                    red: 32,
                    green: 34,
                    blue: 40,
                    alpha: 255,
                },
                10.0,
            ),
            PetTheme::Dark => (
                OverlayColor {
                    red: 35,
                    green: 38,
                    blue: 45,
                    alpha: 242,
                },
                OverlayColor {
                    red: 250,
                    green: 250,
                    blue: 252,
                    alpha: 255,
                },
                10.0,
            ),
        },
        PetBubbleStyle::Custom {
            background_rgba,
            text_rgba,
            corner_radius,
        } => (
            color_from_rgba(background_rgba),
            color_from_rgba(text_rgba),
            finite_nonnegative(corner_radius).min(48.0),
        ),
    }
}

/// 返回宠物身体对应的命中形状，供平台在不栅格化整帧时执行命中判断。
pub(crate) fn pet_hit_shape(center: OverlayPoint, radius: f32) -> OverlayHitShape {
    OverlayHitShape::Circle { center, radius }
}

fn ellipse(
    id: &str,
    center: OverlayPoint,
    radius_x: f32,
    radius_y: f32,
    color: OverlayColor,
) -> OverlayVisual {
    OverlayVisual::Ellipse(OverlayEllipse {
        id: id.into(),
        center,
        radius_x,
        radius_y,
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

fn color_from_rgba(rgba: [f32; 4]) -> OverlayColor {
    OverlayColor {
        red: unit_to_u8(rgba[0]),
        green: unit_to_u8(rgba[1]),
        blue: unit_to_u8(rgba[2]),
        alpha: unit_to_u8(rgba[3]),
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
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use deskhud_engine::{OverlayDisplayTarget, OverlayPoint, OverlayVisual, PetPaint, PetTheme};
    use deskhud_ui::GraphicsPreferences;

    use super::{dialogue_scene_from_pet_paint, scene_from_pet_paint};

    #[test]
    fn converts_body_eyes_and_interactive_body_region() {
        let pet_scene = scene_from_pet_paint(
            OverlayDisplayTarget::Display("primary".into()),
            OverlayPoint { x: 100.0, y: 80.0 },
            40.0,
            &PetPaint::default(),
            [0.0, 0.0],
        );
        assert_eq!(pet_scene.visuals.len(), 5);
        assert_eq!(pet_scene.hit_regions.len(), 1);
        assert!(pet_scene.hit_regions[0].contains(OverlayPoint { x: 100.0, y: 80.0 }));
    }

    #[test]
    fn converts_bubble_background_tail_and_text() {
        let paint = PetPaint {
            bubble_text: Some("Ctrl+C".into()),
            ..PetPaint::default()
        };
        let pet_scene = scene_from_pet_paint(
            OverlayDisplayTarget::Display("primary".into()),
            OverlayPoint { x: 80.0, y: 80.0 },
            40.0,
            &paint,
            [0.0, 0.0],
        );

        let dialogue_scene = dialogue_scene_from_pet_paint(
            OverlayDisplayTarget::Display("primary".into()),
            [190.0, 50.0],
            &paint,
            PetTheme::Dark,
            GraphicsPreferences::default(),
        )
        .expect("bubble text should create a dialogue scene");

        assert_eq!(pet_scene.visuals.len(), 5);
        assert_eq!(dialogue_scene.visuals.len(), 3);
        assert!(matches!(
            dialogue_scene.visuals[1],
            OverlayVisual::RoundedRect(_)
        ));
        assert!(matches!(dialogue_scene.visuals[2], OverlayVisual::Text(_)));
    }

    #[test]
    fn applies_light_theme_to_default_bubble() {
        let paint = PetPaint {
            bubble_text: Some("hello".into()),
            ..PetPaint::default()
        };
        let scene = dialogue_scene_from_pet_paint(
            OverlayDisplayTarget::Display("primary".into()),
            [190.0, 50.0],
            &paint,
            PetTheme::Light,
            GraphicsPreferences::default(),
        )
        .expect("bubble text should create a dialogue scene");
        let OverlayVisual::RoundedRect(background) = &scene.visuals[1] else {
            panic!("first visual should be the bubble background");
        };
        assert_eq!(background.color.red, 248);
    }

    #[test]
    fn effects_off_hides_bubble_scene() {
        let paint = PetPaint {
            bubble_text: Some("hello".into()),
            ..PetPaint::default()
        };
        assert!(
            dialogue_scene_from_pet_paint(
                OverlayDisplayTarget::Display("primary".into()),
                [190.0, 50.0],
                &paint,
                PetTheme::Light,
                GraphicsPreferences {
                    effects: false,
                    ..GraphicsPreferences::default()
                },
            )
            .is_none()
        );
    }

    #[test]
    fn draws_closed_eyes_as_eyelids() {
        let paint = PetPaint {
            eye_open: 0.0,
            ..PetPaint::default()
        };
        let scene = scene_from_pet_paint(
            OverlayDisplayTarget::Display("primary".into()),
            OverlayPoint { x: 80.0, y: 80.0 },
            40.0,
            &paint,
            [0.0, 0.0],
        );

        assert_eq!(scene.visuals.len(), 3);
        assert!(matches!(scene.visuals[1], OverlayVisual::Ellipse(_)));
        assert!(matches!(scene.visuals[2], OverlayVisual::Ellipse(_)));
    }
}
