//! Pet 图形绘制职责。

/// 解释宠物包输出的中性场景节点。
pub(super) struct EguiSceneRenderer;

impl EguiSceneRenderer {
    pub(super) fn render(ui: &mut egui::Ui, scene: &deskhud_engine::PetScene) {
        draw_scene(ui, scene);
    }
}

fn draw_scene(ui: &mut egui::Ui, scene: &deskhud_engine::PetScene) {
    let rect = ui.max_rect();
    let base = rect.width().min(rect.height()) * 0.32;
    let center = ui.max_rect().center();
    let painter = ui.painter();
    let mut items = scene.items.iter().collect::<Vec<_>>();
    items.sort_by_key(|item| item.z_index);
    for item in items {
        let [x, y] = item.transform.translation;
        let position = center + egui::vec2(x * base, y * base);
        let [sx, sy] = item.transform.scale;
        let color = |rgba: [f32; 4]| {
            egui::Color32::from_rgba_unmultiplied(
                (rgba[0].clamp(0.0, 1.0) * 255.0) as u8,
                (rgba[1].clamp(0.0, 1.0) * 255.0) as u8,
                (rgba[2].clamp(0.0, 1.0) * 255.0) as u8,
                (rgba[3].clamp(0.0, 1.0) * 255.0) as u8,
            )
        };
        if let deskhud_engine::SceneNode::Shape { shape, color: fill } = &item.node {
            match shape {
                deskhud_engine::Shape::Circle { radius } => {
                    painter.circle_filled(
                        position,
                        radius * base * sx.abs().min(sy.abs()),
                        color(*fill),
                    );
                }
                deskhud_engine::Shape::Ellipse { radii } => {
                    let pixel_radius_x = radii[0] * base * sx.abs();
                    let pixel_radius_y = radii[1] * base * sy.abs();
                    let segment_count = ((pixel_radius_x.max(pixel_radius_y) * 1.25).ceil()
                        as usize)
                        .clamp(32, 128);
                    let points = (0..segment_count)
                        .map(|i| {
                            let angle = i as f32 * std::f32::consts::TAU / segment_count as f32;
                            position
                                + egui::vec2(
                                    angle.cos() * pixel_radius_x,
                                    angle.sin() * pixel_radius_y,
                                )
                        })
                        .collect();
                    painter.add(egui::epaint::Shape::convex_polygon(
                        points,
                        color(*fill),
                        egui::Stroke::NONE,
                    ));
                }
                deskhud_engine::Shape::Rect {
                    size,
                    corner_radius,
                } => {
                    painter.rect_filled(
                        egui::Rect::from_center_size(
                            position,
                            egui::vec2(size[0] * base * sx.abs(), size[1] * base * sy.abs()),
                        ),
                        *corner_radius,
                        color(*fill),
                    );
                }
            }
        } else if let deskhud_engine::SceneNode::Path(path) = &item.node {
            let points: Vec<_> = path
                .points
                .iter()
                .map(|[x, y]| center + egui::vec2(x * base, y * base))
                .collect();
            if let Some(fill) = path.fill {
                if points.len() >= 3 {
                    painter.add(filled_polygon(&points, color(fill)));
                }
            }
            if let Some(stroke) = path.stroke {
                let stroke_color = color(stroke);
                let stroke_width = path.stroke_width * base;
                if !path.closed {
                    if let Some(first) = points.first() {
                        painter.circle_filled(*first, stroke_width * 0.5, stroke_color);
                    }
                    if let Some(last) = points.last() {
                        painter.circle_filled(*last, stroke_width * 0.5, stroke_color);
                    }
                }
                painter.line(points, egui::Stroke::new(stroke_width, stroke_color));
            }
        } else if let deskhud_engine::SceneNode::GradientPath {
            path,
            top_color,
            bottom_color,
        } = &item.node
        {
            let points: Vec<_> = path
                .points
                .iter()
                .map(|[x, y]| center + egui::vec2(x * base, y * base))
                .collect();
            if points.len() >= 3 {
                let min_y = path
                    .points
                    .iter()
                    .map(|point| point[1])
                    .fold(f32::INFINITY, f32::min);
                let max_y = path
                    .points
                    .iter()
                    .map(|point| point[1])
                    .fold(f32::NEG_INFINITY, f32::max);
                let gradient_color = |y: f32| {
                    let t = ((y - min_y) / (max_y - min_y).max(f32::EPSILON)).clamp(0.0, 1.0);
                    let rgba = std::array::from_fn(|index| {
                        top_color[index] + (bottom_color[index] - top_color[index]) * t
                    });
                    color(rgba)
                };
                let mut mesh = egui::epaint::Mesh::default();
                for [a, b, c] in triangulate_polygon(&points) {
                    let a_index = mesh.vertices.len() as u32;
                    mesh.colored_vertex(points[a], gradient_color(path.points[a][1]));
                    let b_index = mesh.vertices.len() as u32;
                    mesh.colored_vertex(points[b], gradient_color(path.points[b][1]));
                    let c_index = mesh.vertices.len() as u32;
                    mesh.colored_vertex(points[c], gradient_color(path.points[c][1]));
                    mesh.add_triangle(a_index, b_index, c_index);
                }
                painter.add(egui::Shape::mesh(mesh));
            }
            if let Some(stroke) = path.stroke {
                let stroke_color = color(stroke);
                let stroke_width = path.stroke_width * base;
                if !path.closed {
                    if let Some(first) = points.first() {
                        painter.circle_filled(*first, stroke_width * 0.5, stroke_color);
                    }
                    if let Some(last) = points.last() {
                        painter.circle_filled(*last, stroke_width * 0.5, stroke_color);
                    }
                }
                painter.line(points, egui::Stroke::new(stroke_width, stroke_color));
            }
        } else if let deskhud_engine::SceneNode::Text {
            text,
            color: fill,
            size,
        } = &item.node
        {
            painter.text(
                position,
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(*size),
                color(*fill),
            );
        }
    }
}

fn filled_polygon(points: &[egui::Pos2], fill: egui::Color32) -> egui::Shape {
    let mut mesh = egui::epaint::Mesh::default();
    for [a, b, c] in triangulate_polygon(points) {
        let a_index = mesh.vertices.len() as u32;
        mesh.colored_vertex(points[a], fill);
        let b_index = mesh.vertices.len() as u32;
        mesh.colored_vertex(points[b], fill);
        let c_index = mesh.vertices.len() as u32;
        mesh.colored_vertex(points[c], fill);
        mesh.add_triangle(a_index, b_index, c_index);
    }
    egui::Shape::mesh(mesh)
}

/// Ear-clips a simple closed polygon so concave package artwork is filled
/// correctly instead of being forced through egui's convex-only helper.
fn triangulate_polygon(points: &[egui::Pos2]) -> Vec<[usize; 3]> {
    let point_count = if points.len() > 1 && points.first() == points.last() {
        points.len() - 1
    } else {
        points.len()
    };
    if point_count < 3 {
        return Vec::new();
    }
    let mut indices: Vec<usize> = (0..point_count).collect();
    if polygon_area(points) < 0.0 {
        indices.reverse();
    }
    let mut triangles = Vec::with_capacity(point_count.saturating_sub(2));
    let mut guard = 0;
    while indices.len() > 2 && guard < point_count * point_count {
        let mut clipped = false;
        for cursor in 0..indices.len() {
            let previous = indices[(cursor + indices.len() - 1) % indices.len()];
            let current = indices[cursor];
            let next = indices[(cursor + 1) % indices.len()];
            if cross(points[previous], points[current], points[next]) <= 0.0 {
                continue;
            }
            if indices.iter().any(|&candidate| {
                candidate != previous
                    && candidate != current
                    && candidate != next
                    && point_in_triangle(
                        points[candidate],
                        points[previous],
                        points[current],
                        points[next],
                    )
            }) {
                continue;
            }
            triangles.push([previous, current, next]);
            indices.remove(cursor);
            clipped = true;
            break;
        }
        if !clipped {
            return Vec::new();
        }
        guard += 1;
    }
    triangles
}

fn polygon_area(points: &[egui::Pos2]) -> f32 {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            point.x * next.y - next.x * point.y
        })
        .sum::<f32>()
        * 0.5
}

fn cross(a: egui::Pos2, b: egui::Pos2, c: egui::Pos2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn point_in_triangle(point: egui::Pos2, a: egui::Pos2, b: egui::Pos2, c: egui::Pos2) -> bool {
    cross(a, b, point) >= 0.0 && cross(b, c, point) >= 0.0 && cross(c, a, point) >= 0.0
}
