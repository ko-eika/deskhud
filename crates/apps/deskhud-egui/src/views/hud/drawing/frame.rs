//! HUD 内容帧绘制、样式合成与缩放命中。

use super::overlay::with_alpha;
use super::*;
use crate::views::hud::DEFAULT_SHADOW_BLUR;
use crate::views::hud::HudRenderLayer;
use deskhud_engine::{HudVisual, ThemePalette};

pub(super) struct FrameResponse {
    pub(super) body: egui::Response,
    pub(super) group_drag: Option<egui::Response>,
    pub(super) resize_drag: Option<ResizeDrag>,
    pub(super) resize_started: bool,
    pub(super) group_inner_rect: Option<egui::Rect>,
    pub(super) members: Vec<MemberResponse>,
}

pub(super) struct MemberResponse {
    pub(super) instance_id: deskhud_engine::HudInstanceId,
    pub(super) response: egui::Response,
    pub(super) rect: egui::Rect,
    pub(super) corner_radius: f32,
    pub(super) resize_drag: Option<ResizeDrag>,
    pub(super) base_size: deskhud_engine::HudLogicalSize,
}

#[derive(Clone, Copy)]
pub(super) struct ResizeEdges {
    pub(super) left: bool,
    pub(super) right: bool,
    pub(super) top: bool,
    pub(super) bottom: bool,
}

#[derive(Clone, Copy)]
pub(super) struct ResizeDrag {
    pub(super) edges: ResizeEdges,
    pub(super) delta: egui::Vec2,
}

struct LayerAppearance {
    background_enabled: bool,
    background_color: [u8; 3],
    content_custom_color_enabled: bool,
    background_opacity: f32,
    background_blur: f32,
    content_opacity: f32,
    content_color: [u8; 3],
    shadow_enabled: bool,
    window_shadow_global: bool,
    content_shadow_global: bool,
    window_shadow_enabled: bool,
    content_shadow_enabled: bool,
    window_shadow: f32,
    window_shadow_blur: f32,
    window_shadow_distance: f32,
    window_shadow_angle: f32,
    window_shadow_color: [u8; 3],
    window_shadow_color_custom: bool,
    window_custom_shadow: f32,
    window_custom_shadow_blur: f32,
    window_custom_shadow_distance: f32,
    window_custom_shadow_angle: f32,
    window_custom_shadow_color: [u8; 3],
    window_custom_shadow_color_custom: bool,
    content_custom_shadow: f32,
    content_custom_shadow_blur: f32,
    content_custom_shadow_distance: f32,
    content_custom_shadow_angle: f32,
    content_custom_shadow_color: [u8; 3],
    content_custom_shadow_color_custom: bool,
    border_enabled: bool,
    border_opacity: f32,
    border_width: f32,
    corner_radius: f32,
    border_color: [u8; 3],
    border_custom_color_enabled: bool,
}

fn resolve_layer_appearance(
    prefs: &UiPreferences,
    layer: &HudRenderLayer,
    palette: ThemePalette,
) -> LayerAppearance {
    let default_radius = layer
        .frame
        .visuals
        .iter()
        .find_map(|visual| match visual {
            HudVisual::Panel { radius, .. } => {
                Some((radius / HUD_CORNER_RADIUS_MAX).clamp(0.0, 1.0))
            }
            _ => None,
        })
        .unwrap_or(6.0 / HUD_CORNER_RADIUS_MAX);
    let value = |name: &str, default: f32| {
        layer
            .config
            .get(name)
            .and_then(config_f32)
            .unwrap_or_else(|| {
                prefs.hud.visual_value(
                    &layer.source.plugin_id,
                    &layer.source.contribution_id,
                    name,
                    default,
                )
            })
            .clamp(0.0, 1.0)
    };
    let legacy_corner_radius = value("border_radius", default_radius);
    let legacy_window_shadow = value("window_shadow", 0.0);
    let legacy_content_shadow = value("content_shadow", 0.0);
    let shadow_opacity = value(
        "shadow_opacity",
        0.75_f32.max(legacy_window_shadow.max(legacy_content_shadow)),
    );
    let rgb = |names: [&str; 3], defaults: [u8; 3]| {
        std::array::from_fn(|channel| {
            (value(names[channel], defaults[channel] as f32 / 255.0) * 255.0).round() as u8
        })
    };
    let theme_background = color3(palette.surface);
    let theme_content = color3(palette.text);
    let theme_border = color3(palette.border);
    let background_custom_color_enabled = value("background_color_enabled", 0.0) >= 0.5;
    let background_color = rgb(
        ["background_red", "background_green", "background_blue"],
        theme_background,
    );
    LayerAppearance {
        background_enabled: value("background_enabled", 1.0) >= 0.5,
        background_color: if background_custom_color_enabled {
            background_color
        } else {
            theme_background
        },
        background_opacity: value("background_opacity", 1.0),
        background_blur: value("background_blur", 0.0),
        content_opacity: value("content_opacity", 1.0),
        content_custom_color_enabled: value("content_color_enabled", 0.0) >= 0.5,
        content_color: rgb(
            ["content_red", "content_green", "content_blue"],
            theme_content,
        ),
        shadow_enabled: value(
            "shadow_enabled",
            if shadow_opacity > f32::EPSILON {
                1.0
            } else {
                0.0
            },
        ) >= 0.5,
        window_shadow_global: value("window_shadow_mode", 0.0) < 0.5,
        content_shadow_global: value("content_shadow_mode", 0.0) < 0.5,
        window_shadow_enabled: value("window_shadow_enabled", 1.0) >= 0.5,
        content_shadow_enabled: value("content_shadow_enabled", 1.0) >= 0.5,
        window_shadow: shadow_opacity,
        window_shadow_blur: value(
            "shadow_blur",
            value("window_shadow_blur", DEFAULT_SHADOW_BLUR),
        ),
        window_shadow_distance: value(
            "shadow_distance",
            value("window_shadow_distance", 5.0 / 12.0),
        ),
        window_shadow_angle: value("shadow_angle", value("window_shadow_angle", 0.125)),
        window_shadow_color_custom: value("shadow_color_enabled", 0.0) >= 0.5,
        window_shadow_color: rgb(
            ["shadow_red", "shadow_green", "shadow_blue"],
            color3(palette.shadow),
        ),
        window_custom_shadow: value("window_shadow", 0.75),
        window_custom_shadow_blur: value("window_shadow_blur", DEFAULT_SHADOW_BLUR),
        window_custom_shadow_distance: value("window_shadow_distance", 5.0 / 12.0),
        window_custom_shadow_angle: value("window_shadow_angle", 0.125),
        window_custom_shadow_color: rgb(
            [
                "window_shadow_red",
                "window_shadow_green",
                "window_shadow_blue",
            ],
            color3(palette.shadow),
        ),
        window_custom_shadow_color_custom: value("window_shadow_color_enabled", 0.0) >= 0.5,
        content_custom_shadow: value("content_shadow", 0.75),
        content_custom_shadow_blur: value("content_shadow_blur", DEFAULT_SHADOW_BLUR),
        content_custom_shadow_distance: value("content_shadow_distance", 5.0 / 12.0),
        content_custom_shadow_angle: value("content_shadow_angle", 0.125),
        content_custom_shadow_color: rgb(
            [
                "content_shadow_red",
                "content_shadow_green",
                "content_shadow_blue",
            ],
            color3(palette.shadow),
        ),
        content_custom_shadow_color_custom: value("content_shadow_color_enabled", 0.0) >= 0.5,
        border_enabled: value("border_enabled", 1.0) >= 0.5,
        border_opacity: value("border_opacity", 1.0),
        border_width: value("border_width", 1.0 / 6.0),
        corner_radius: value("corner_radius", legacy_corner_radius),
        border_custom_color_enabled: value("border_color_enabled", 0.0) >= 0.5,
        border_color: rgb(["border_red", "border_green", "border_blue"], theme_border),
    }
}

fn config_f32(value: &deskhud_ui::HudConfigValue) -> Option<f32> {
    match value {
        deskhud_ui::HudConfigValue::Float(value) => Some(*value as f32),
        deskhud_ui::HudConfigValue::Int(value) => Some(*value as f32),
        _ => None,
    }
}

fn color3(color: deskhud_engine::OverlayColor) -> [u8; 3] {
    [color.red, color.green, color.blue]
}

pub(super) fn draw_frame(
    ui: &mut egui::Ui,
    item: &HudRenderItem,
    layout_mode: bool,
    custom_resize: bool,
    prefs: &UiPreferences,
) -> FrameResponse {
    let base_size = egui::vec2(item.base_size.width, item.base_size.height);
    let available = ui.available_size_before_wrap();
    let is_group = matches!(item.target, HudLayoutTarget::Group(_));
    let size = if layout_mode
        && available.x.is_finite()
        && available.y.is_finite()
        && available.x > 1.0
        && available.y > 1.0
    {
        available
    } else {
        item.container_size.unwrap_or_else(|| {
            egui::vec2(
                base_size.x * item.width.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
                base_size.y * item.height.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
            )
        })
    };
    let (rect, response) = ui.allocate_exact_size(
        size,
        if layout_mode && !is_group {
            egui::Sense::click_and_drag()
        } else if layout_mode {
            // The group remains selectable/draggable; child rectangles are
            // registered later and take precedence for member dragging.
            egui::Sense::click_and_drag()
        } else {
            egui::Sense::hover()
        },
    );
    let ui_font_scale =
        egui::TextStyle::Body.resolve(ui.style()).size / deskhud_ui::DEFAULT_UI_FONT_SIZE.max(1.0);
    let hud_painter = ui.ctx().layer_painter(response.layer_id);
    let group_target = is_group;
    let scale_x = if group_target {
        1.0
    } else {
        rect.width() / item.base_size.width.max(1.0)
    };
    let scale_y = if group_target {
        1.0
    } else {
        rect.height() / item.base_size.height.max(1.0)
    };
    let mut member_responses = Vec::new();
    let mut group_inner_rect = None;
    if layout_mode && let Some([red, green, blue]) = item.group_color {
        let radius = (item.corner_radius.clamp(0.0, 1.0) * HUD_CORNER_RADIUS_MAX)
            .round()
            .clamp(0.0, 255.0) as u8;
        let corner_radius = egui::CornerRadius::same(radius);
        hud_painter.rect_filled(
            rect,
            corner_radius,
            egui::Color32::from_rgba_unmultiplied(red, green, blue, 32),
        );
        hud_painter.rect_stroke(
            rect,
            corner_radius,
            egui::Stroke::new(
                1.5,
                egui::Color32::from_rgba_unmultiplied(red, green, blue, 180),
            ),
            egui::StrokeKind::Inside,
        );
        if let Some([top, right, bottom, left]) = item.group_padding {
            let horizontal_limit = rect.width() * 0.25;
            let vertical_limit = rect.height() * 0.25;
            let top = top.clamp(0.0, vertical_limit).floor();
            let bottom = bottom.clamp(0.0, vertical_limit).floor();
            let left = left.clamp(0.0, horizontal_limit).floor();
            let right = right.clamp(0.0, horizontal_limit).floor();
            let inner = egui::Rect::from_min_max(
                rect.min + egui::vec2(left, top),
                rect.max - egui::vec2(right, bottom),
            );
            group_inner_rect = Some(inner);
            hud_painter.rect_stroke(
                inner,
                corner_radius,
                egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(red, green, blue, 120),
                ),
                egui::StrokeKind::Inside,
            );
        }
    }
    for layer in &item.layers {
        let palette = crate::views::theme::palette(ui.visuals());
        let appearance = resolve_layer_appearance(prefs, layer, palette);
        let foreground_color = if appearance.content_custom_color_enabled {
            appearance.content_color
        } else {
            color3(palette.text)
        };
        let border_color = if appearance.border_custom_color_enabled {
            appearance.border_color
        } else {
            color3(palette.border)
        };
        let child_rect = egui::Rect::from_min_size(
            rect.min + egui::vec2(layer.rect.x * scale_x, layer.rect.y * scale_y),
            egui::vec2(layer.rect.width * scale_x, layer.rect.height * scale_y),
        );
        let child_clip = egui::Rect::from_min_size(
            rect.min + egui::vec2(layer.clip.x * scale_x, layer.clip.y * scale_y),
            egui::vec2(layer.clip.width * scale_x, layer.clip.height * scale_y),
        )
        .intersect(rect);
        let child_painter = hud_painter.with_clip_rect(child_clip);
        let window_radius = appearance.corner_radius.clamp(0.0, 1.0) * HUD_CORNER_RADIUS_MAX;
        let (window_shadow, window_blur, window_distance, window_angle, window_color) =
            if appearance.window_shadow_global {
                (
                    appearance.window_shadow,
                    appearance.window_shadow_blur,
                    appearance.window_shadow_distance,
                    appearance.window_shadow_angle,
                    if appearance.window_shadow_color_custom {
                        appearance.window_shadow_color
                    } else {
                        color3(palette.shadow)
                    },
                )
            } else {
                (
                    appearance.window_custom_shadow,
                    appearance.window_custom_shadow_blur,
                    appearance.window_custom_shadow_distance,
                    appearance.window_custom_shadow_angle,
                    if appearance.window_custom_shadow_color_custom {
                        appearance.window_custom_shadow_color
                    } else {
                        color3(palette.shadow)
                    },
                )
            };
        let (content_shadow, content_blur, content_distance, content_angle, content_color) =
            if appearance.content_shadow_global {
                (
                    appearance.window_shadow,
                    appearance.window_shadow_blur,
                    appearance.window_shadow_distance,
                    appearance.window_shadow_angle,
                    if appearance.window_shadow_color_custom {
                        appearance.window_shadow_color
                    } else {
                        color3(palette.shadow)
                    },
                )
            } else {
                (
                    appearance.content_custom_shadow,
                    appearance.content_custom_shadow_blur,
                    appearance.content_custom_shadow_distance,
                    appearance.content_custom_shadow_angle,
                    if appearance.content_custom_shadow_color_custom {
                        appearance.content_custom_shadow_color
                    } else {
                        color3(palette.shadow)
                    },
                )
            };
        let window_shadow_enabled = if appearance.window_shadow_global {
            appearance.shadow_enabled && appearance.window_shadow_enabled
        } else {
            appearance.window_shadow_enabled && window_shadow > f32::EPSILON
        };
        let content_shadow_enabled = if appearance.content_shadow_global {
            appearance.shadow_enabled && appearance.content_shadow_enabled
        } else {
            appearance.content_shadow_enabled && content_shadow > f32::EPSILON
        };
        if window_shadow_enabled {
            paint_window_shadow(
                // The shadow intentionally extends outside the HUD rect. A
                // rect-sized clip would cut away the entire visible part of
                // the window shadow before it reaches the desktop backdrop.
                &hud_painter,
                child_rect,
                window_radius,
                window_shadow,
                window_blur,
                window_distance,
                window_angle,
                window_color,
            );
        }
        let child_scale_x = (child_rect.width() / layer.base_size.width.max(1.0))
            .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
        let child_scale_y = (child_rect.height() / layer.base_size.height.max(1.0))
            .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
        let child_scale = child_scale_x.min(child_scale_y);
        for visual in &layer.frame.visuals {
            match visual {
                HudVisual::Panel {
                    width: _,
                    height: _,
                    radius: _,
                    color,
                } => {
                    if appearance.background_enabled {
                        paint_acrylic_background(
                            &child_painter,
                            child_rect,
                            window_radius,
                            [
                                appearance.background_color[0],
                                appearance.background_color[1],
                                appearance.background_color[2],
                                color[3],
                            ],
                            appearance.background_opacity,
                            appearance.background_blur,
                        );
                    }
                }
                HudVisual::Text {
                    text,
                    font_size,
                    color,
                } => {
                    paint_hud_text(
                        &child_painter,
                        child_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::proportional(
                            (font_size * child_scale * ui_font_scale).clamp(8.0, 96.0),
                        ),
                        foreground_color,
                        color[3],
                        appearance.content_opacity,
                        if content_shadow_enabled {
                            content_shadow
                        } else {
                            0.0
                        },
                        content_blur,
                        content_distance,
                        content_angle,
                        content_color,
                    );
                }
                HudVisual::Label {
                    text,
                    x,
                    y,
                    align,
                    font_size,
                    color,
                } => {
                    let align = match align {
                        deskhud_engine::HudTextAlign::Left => egui::Align2::LEFT_CENTER,
                        deskhud_engine::HudTextAlign::Center => egui::Align2::CENTER_CENTER,
                        deskhud_engine::HudTextAlign::Right => egui::Align2::RIGHT_CENTER,
                    };
                    paint_hud_text(
                        &child_painter,
                        child_rect.min + egui::vec2(x * child_scale_x, y * child_scale_y),
                        align,
                        text,
                        egui::FontId::proportional(
                            (font_size * child_scale * ui_font_scale).clamp(8.0, 96.0),
                        ),
                        foreground_color,
                        color[3],
                        appearance.content_opacity,
                        if content_shadow_enabled {
                            content_shadow
                        } else {
                            0.0
                        },
                        content_blur,
                        content_distance,
                        content_angle,
                        content_color,
                    );
                }
                HudVisual::ProgressBar {
                    x,
                    y,
                    width,
                    height,
                    radius,
                    value,
                    background,
                    fill,
                } => {
                    let bar = egui::Rect::from_min_size(
                        child_rect.min + egui::vec2(x * child_scale_x, y * child_scale_y),
                        egui::vec2(width * child_scale_x, height * child_scale_y),
                    )
                    .intersect(child_rect);
                    let radius = (radius * child_scale).clamp(0.0, bar.height() * 0.5);
                    child_painter.rect_filled(
                        bar,
                        radius,
                        rgba_with_alpha(*background, appearance.content_opacity),
                    );
                    let fill_rect = egui::Rect::from_min_size(
                        bar.min,
                        egui::vec2(bar.width() * value.clamp(0.0, 1.0), bar.height()),
                    );
                    child_painter.rect_filled(
                        fill_rect,
                        radius,
                        rgba_with_alpha(*fill, appearance.content_opacity),
                    );
                }
                HudVisual::LineChart {
                    x,
                    y,
                    width,
                    height,
                    values,
                    min,
                    max,
                    stroke_width,
                    color,
                } => {
                    let chart = egui::Rect::from_min_size(
                        child_rect.min + egui::vec2(x * child_scale_x, y * child_scale_y),
                        egui::vec2(width * child_scale_x, height * child_scale_y),
                    )
                    .intersect(child_rect);
                    let range = (max - min).max(f32::EPSILON);
                    if values.len() > 1 && chart.is_positive() {
                        let last = (values.len() - 1) as f32;
                        let points = values.iter().enumerate().map(|(index, value)| {
                            egui::pos2(
                                chart.left() + chart.width() * index as f32 / last,
                                chart.bottom()
                                    - chart.height() * ((*value - min) / range).clamp(0.0, 1.0),
                            )
                        });
                        child_painter.add(egui::Shape::line(
                            points.collect(),
                            egui::Stroke::new(
                                (stroke_width * child_scale).clamp(0.5, 32.0),
                                rgba_with_alpha(*color, appearance.content_opacity),
                            ),
                        ));
                    }
                }
            }
        }
        if appearance.border_enabled {
            paint_hud_border(
                &child_painter,
                child_rect,
                appearance.border_opacity,
                appearance.border_width,
                appearance.corner_radius,
                egui::Color32::from_rgb(border_color[0], border_color[1], border_color[2]),
            );
        }
        if layout_mode && matches!(item.target, HudLayoutTarget::Group(_)) {
            let response = ui.interact(
                child_rect,
                ui.make_persistent_id(("hud-group-member", layer.instance_id.as_str())),
                egui::Sense::drag(),
            );
            let (resize_drag, _resize_started) = hud_resize_interaction(
                ui,
                &format!("hud-member/{}", layer.instance_id.as_str()),
                child_rect,
            );
            member_responses.push(MemberResponse {
                instance_id: layer.instance_id.clone(),
                response,
                rect: child_rect,
                corner_radius: appearance.corner_radius,
                resize_drag,
                base_size: layer.base_size,
            });
        }
    }
    let (resize_drag, resize_started) = if custom_resize {
        hud_resize_interaction(ui, &item.key, rect)
    } else {
        (None, false)
    };
    FrameResponse {
        body: response,
        group_drag: None,
        resize_drag,
        resize_started,
        group_inner_rect,
        members: member_responses,
    }
}

fn hud_resize_interaction(
    ui: &mut egui::Ui,
    key: &str,
    rect: egui::Rect,
) -> (Option<ResizeDrag>, bool) {
    let edge = RESIZE_EDGE_GRAB
        .min(rect.width() * 0.25)
        .min(rect.height() * 0.25);
    let corner = RESIZE_CORNER_GRAB
        .min(rect.width() * 0.4)
        .min(rect.height() * 0.4);
    let sides = [
        (
            "left",
            egui::Rect::from_min_max(
                rect.left_top() + egui::vec2(0.0, corner),
                rect.left_bottom() + egui::vec2(edge, -corner),
            ),
            ResizeEdges {
                left: true,
                right: false,
                top: false,
                bottom: false,
            },
            egui::CursorIcon::ResizeHorizontal,
        ),
        (
            "right",
            egui::Rect::from_min_max(
                rect.right_top() + egui::vec2(-edge, corner),
                rect.right_bottom() + egui::vec2(0.0, -corner),
            ),
            ResizeEdges {
                left: false,
                right: true,
                top: false,
                bottom: false,
            },
            egui::CursorIcon::ResizeHorizontal,
        ),
        (
            "top",
            egui::Rect::from_min_max(
                rect.left_top() + egui::vec2(corner, 0.0),
                rect.right_top() + egui::vec2(-corner, edge),
            ),
            ResizeEdges {
                left: false,
                right: false,
                top: true,
                bottom: false,
            },
            egui::CursorIcon::ResizeVertical,
        ),
        (
            "bottom",
            egui::Rect::from_min_max(
                rect.left_bottom() + egui::vec2(corner, -edge),
                rect.right_bottom() + egui::vec2(-corner, 0.0),
            ),
            ResizeEdges {
                left: false,
                right: false,
                top: false,
                bottom: true,
            },
            egui::CursorIcon::ResizeVertical,
        ),
    ];
    let corners = [
        (
            "top-left",
            egui::Rect::from_min_size(rect.left_top(), egui::Vec2::splat(corner)),
            ResizeEdges {
                left: true,
                right: false,
                top: true,
                bottom: false,
            },
            egui::CursorIcon::ResizeNwSe,
        ),
        (
            "top-right",
            egui::Rect::from_min_max(
                rect.right_top() + egui::vec2(-corner, 0.0),
                rect.right_top() + egui::vec2(0.0, corner),
            ),
            ResizeEdges {
                left: false,
                right: true,
                top: true,
                bottom: false,
            },
            egui::CursorIcon::ResizeNeSw,
        ),
        (
            "bottom-left",
            egui::Rect::from_min_max(
                rect.left_bottom() + egui::vec2(0.0, -corner),
                rect.left_bottom() + egui::vec2(corner, 0.0),
            ),
            ResizeEdges {
                left: true,
                right: false,
                top: false,
                bottom: true,
            },
            egui::CursorIcon::ResizeNeSw,
        ),
        (
            "bottom-right",
            egui::Rect::from_min_max(
                rect.right_bottom() - egui::Vec2::splat(corner),
                rect.right_bottom(),
            ),
            ResizeEdges {
                left: false,
                right: true,
                top: false,
                bottom: true,
            },
            egui::CursorIcon::ResizeNwSe,
        ),
    ];

    let mut drag = None;
    let mut started = false;
    for (name, hit_rect, edges, cursor) in sides.into_iter().chain(corners) {
        let response = ui
            .interact(
                hit_rect,
                ui.make_persistent_id(("hud-resize", key, name)),
                egui::Sense::drag(),
            )
            .on_hover_cursor(cursor);
        started |= response.drag_started();
        if response.dragged() {
            drag = Some(ResizeDrag {
                edges,
                // The stored geometry already contains every previous frame
                // of this gesture, so apply only the current pointer step.
                // Response::drag_delta is cumulative and would accelerate
                // into a boundary, producing the apparent squeeze/offset.
                delta: ui.input(|input| input.pointer.delta()),
            });
        }
    }
    (drag, started)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_window_shadow(
    painter: &egui::Painter,
    rect: egui::Rect,
    radius: f32,
    opacity: f32,
    blur: f32,
    distance: f32,
    angle: f32,
    color: [u8; 3],
) {
    let opacity = opacity.clamp(0.0, 1.0);
    if opacity <= f32::EPSILON {
        return;
    }
    let blur = blur.clamp(0.0, 1.0);
    let distance = distance.clamp(0.0, 1.0) * 12.0;
    let angle = angle.clamp(0.0, 1.0) * std::f32::consts::TAU;
    let offset = egui::vec2(angle.cos(), angle.sin()) * distance;
    for step in (1..=6).rev() {
        let spread = blur * step as f32 * 4.0;
        let alpha = (opacity * 80.0 / 6.0).round() as u8;
        painter.rect_filled(
            rect.translate(offset).expand(spread),
            (radius + spread).round().clamp(0.0, 255.0) as u8,
            egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], alpha),
        );
    }
}

fn paint_acrylic_background(
    painter: &egui::Painter,
    rect: egui::Rect,
    radius: f32,
    color: [u8; 4],
    opacity: f32,
    acrylic: f32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
    let acrylic = acrylic.clamp(0.0, 1.0);
    if acrylic > f32::EPSILON {
        // The portable Glow renderer cannot sample pixels behind the native
        // window. Layered tint, softened edges and an inner highlight provide
        // a stable acrylic-like treatment on every supported platform.
        for step in (1..=3).rev() {
            let spread = acrylic * step as f32 * 3.0;
            painter.rect_filled(
                rect.expand(spread),
                (radius + spread).round().clamp(0.0, 255.0) as u8,
                rgba_with_alpha(color, opacity * acrylic * (0.10 / step as f32)),
            );
        }
    }
    painter.rect_filled(
        rect,
        radius.round().clamp(0.0, 255.0) as u8,
        rgba_with_alpha(color, opacity),
    );
    if acrylic > f32::EPSILON {
        let [red, green, blue, _] = color;
        let luminance = red as f32 * 0.2126 + green as f32 * 0.7152 + blue as f32 * 0.0722;
        let tint = if luminance < 150.0 {
            egui::Color32::from_white_alpha((acrylic * opacity * 34.0).round() as u8)
        } else {
            egui::Color32::from_black_alpha((acrylic * opacity * 24.0).round() as u8)
        };
        painter.rect_filled(rect, radius.round().clamp(0.0, 255.0) as u8, tint);
        painter.rect_stroke(
            rect.shrink(0.5),
            radius,
            egui::Stroke::new(
                1.0,
                egui::Color32::from_white_alpha((acrylic * opacity * 64.0).round() as u8),
            ),
            egui::StrokeKind::Inside,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_hud_text(
    painter: &egui::Painter,
    position: egui::Pos2,
    align: egui::Align2,
    text: &str,
    font: egui::FontId,
    color: [u8; 3],
    source_alpha: u8,
    opacity: f32,
    shadow_opacity: f32,
    shadow_blur: f32,
    shadow_distance: f32,
    shadow_angle: f32,
    shadow_color: [u8; 3],
) {
    let opacity = opacity.clamp(0.0, 1.0);
    let shadow_opacity = shadow_opacity.clamp(0.0, 1.0);
    if shadow_opacity > f32::EPSILON {
        let blur = shadow_blur.clamp(0.0, 1.0);
        let distance = shadow_distance.clamp(0.0, 1.0) * 8.0;
        let angle = shadow_angle.clamp(0.0, 1.0) * std::f32::consts::TAU;
        let offset = egui::vec2(angle.cos(), angle.sin()) * distance;
        let steps = 5;
        for step in (0..steps).rev() {
            let angle = step as f32 * std::f32::consts::TAU / steps as f32;
            let spread = blur * 4.0;
            let delta = egui::vec2(angle.cos(), angle.sin()) * spread;
            let alpha = (source_alpha as f32 * opacity * shadow_opacity * 0.75 / steps as f32)
                .round() as u8;
            painter.text(
                position + offset + delta,
                align,
                text,
                font.clone(),
                egui::Color32::from_rgba_unmultiplied(
                    shadow_color[0],
                    shadow_color[1],
                    shadow_color[2],
                    alpha,
                ),
            );
        }
    }
    painter.text(
        position,
        align,
        text,
        font,
        egui::Color32::from_rgba_unmultiplied(
            color[0],
            color[1],
            color[2],
            (source_alpha as f32 * opacity).round() as u8,
        ),
    );
}

fn paint_hud_border(
    painter: &egui::Painter,
    rect: egui::Rect,
    opacity: f32,
    width: f32,
    radius: f32,
    color: egui::Color32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
    let width = width.clamp(0.0, 1.0) * HUD_BORDER_WIDTH_MAX;
    if opacity <= f32::EPSILON || width <= f32::EPSILON {
        return;
    }
    painter.rect_stroke(
        rect,
        radius.clamp(0.0, 1.0) * HUD_CORNER_RADIUS_MAX,
        egui::Stroke::new(width, with_alpha(color, (opacity * 255.0).round() as u8)),
        egui::StrokeKind::Inside,
    );
}

fn rgba_with_alpha([red, green, blue, alpha]: [u8; 4], opacity: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        red,
        green,
        blue,
        ((alpha as f32) * opacity.clamp(0.0, 1.0)) as u8,
    )
}
