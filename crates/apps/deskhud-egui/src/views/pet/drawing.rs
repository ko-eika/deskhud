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
                    let radius = 24;
                    let points = (0..radius)
                        .map(|i| {
                            let angle = i as f32 * std::f32::consts::TAU / radius as f32;
                            position
                                + egui::vec2(
                                    angle.cos() * radii[0] * base * sx.abs(),
                                    angle.sin() * radii[1] * base * sy.abs(),
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
                    painter.add(egui::epaint::Shape::convex_polygon(
                        points.clone(),
                        color(fill),
                        egui::Stroke::NONE,
                    ));
                }
            }
            if let Some(stroke) = path.stroke {
                painter.line(points, egui::Stroke::new(path.stroke_width, color(stroke)));
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
