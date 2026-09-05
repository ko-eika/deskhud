//! Small rasterized SVG icons used by the settings controls.
#![allow(clippy::collapsible_if)]

use egui::{Color32, ColorImage, Rect, TextureHandle, TextureOptions, Ui};

const CHEVRON_DOWN: &[u8] = include_bytes!("../../../../../assets/svg/chevron-down.svg");
const GRID: &[u8] = include_bytes!("../../../../../assets/svg/grid.svg");
const LIST: &[u8] = include_bytes!("../../../../../assets/svg/list.svg");
const LAYOUT_GRID: &[u8] = include_bytes!("../../../../../assets/svg/layout-grid.svg");
const LIST_DETAILS: &[u8] = include_bytes!("../../../../../assets/svg/list-details.svg");
const ADJUST_HORIZONTAL: &[u8] = include_bytes!("../../../../../assets/svg/adjust-horizontal.svg");
const BRIGHTNESS: &[u8] = include_bytes!("../../../../../assets/svg/brightness.svg");
const LAYERS_SUBTRACT: &[u8] = include_bytes!("../../../../../assets/svg/layers-subtract.svg");
const CREATE_FILLED: &[u8] = include_bytes!("../../../../../assets/svg/create-filled.svg");
const CHECK: &[u8] = include_bytes!("../../../../../assets/svg/check.svg");
const CLOSE: &[u8] = include_bytes!("../../../../../assets/svg/close.svg");
const CIRCLE_CHECK: &[u8] = include_bytes!("../../../../../assets/svg/circle-check.svg");
const CLOSE_CIRCLE: &[u8] = include_bytes!("../../../../../assets/svg/close-circle.svg");
const ANALYTICS: &[u8] = include_bytes!("../../../../../assets/svg/analytics.svg");
const PUZZLE: &[u8] = include_bytes!("../../../../../assets/svg/puzzle.svg");
const WINDOW: &[u8] = include_bytes!("../../../../../assets/svg/window.svg");
const INFO: &[u8] = include_bytes!("../../../../../assets/svg/info.svg");
const LINK: &[u8] = include_bytes!("../../../../../assets/svg/link.svg");
const RESET: &[u8] = include_bytes!("../../../../../assets/svg/reset.svg");

/// Paints a bundled SVG icon without relying on a font glyph being available.
pub(crate) fn paint(ui: &Ui, name: &'static str, rect: Rect, color: Color32, flip_y: bool) {
    let Some(texture) = texture(ui, name) else {
        return;
    };
    let uv = if flip_y {
        Rect::from_min_max(egui::pos2(0.0, 1.0), egui::pos2(1.0, 0.0))
    } else {
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
    };
    ui.painter().image(texture.id(), rect, uv, color);
}

fn texture(ui: &Ui, name: &'static str) -> Option<TextureHandle> {
    let id = ui.make_persistent_id(("settings-svg", name));
    if let Some(texture) = ui.ctx().data(|data| data.get_temp::<TextureHandle>(id)) {
        return Some(texture);
    }
    let bytes = match name {
        "chevron-down" => CHEVRON_DOWN,
        "grid" => GRID,
        "list" => LIST,
        "layout-grid" => LAYOUT_GRID,
        "list-details" => LIST_DETAILS,
        "adjust-horizontal" => ADJUST_HORIZONTAL,
        "brightness" => BRIGHTNESS,
        "layers-subtract" => LAYERS_SUBTRACT,
        "create-filled" => CREATE_FILLED,
        "check" => CHECK,
        "close" => CLOSE,
        "circle-check" => CIRCLE_CHECK,
        "close-circle" => CLOSE_CIRCLE,
        "analytics" => ANALYTICS,
        "puzzle" => PUZZLE,
        "window" => WINDOW,
        "info" => INFO,
        "link" => LINK,
        "reset" => RESET,
        _ => return None,
    };
    let pixmap = rasterize(bytes)?;
    let image = ColorImage::from_rgba_unmultiplied(
        [pixmap.width() as usize, pixmap.height() as usize],
        pixmap.data(),
    );
    let texture = ui.ctx().load_texture(name, image, TextureOptions::LINEAR);
    ui.ctx()
        .data_mut(|data| data.insert_temp(id, texture.clone()));
    Some(texture)
}

fn rasterize(bytes: &[u8]) -> Option<resvg::tiny_skia::Pixmap> {
    let options = resvg::usvg::Options::default();
    // Rasterize every icon as white so egui's painter tint can apply the active
    // theme color at draw time. The desktop icon set omits `fill`, which would
    // otherwise make SVG paths default to black.
    let mut svg = String::from_utf8_lossy(bytes).replace("currentColor", "#ffffff");
    if let Some(svg_start) = svg.find("<svg") {
        if let Some(tag_end) = svg[svg_start..].find('>') {
            svg.insert_str(svg_start + tag_end, " fill=\"#ffffff\"");
        }
    }
    let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &options).ok()?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    Some(pixmap)
}
