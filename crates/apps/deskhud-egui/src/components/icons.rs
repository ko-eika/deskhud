//! Small rasterized SVG icons used by the settings controls.

use egui::{Color32, ColorImage, Rect, TextureHandle, TextureOptions, Ui};

const CHEVRON_DOWN: &[u8] = include_bytes!("../../../../../assets/svg/chevron-down.svg");
const GRID: &[u8] = include_bytes!("../../../../../assets/svg/grid.svg");
const LIST: &[u8] = include_bytes!("../../../../../assets/svg/list.svg");

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
    // The bundled SVGs intentionally use `currentColor`. Rasterize them as white
    // first so egui's painter tint can apply the active theme color at draw time.
    let svg = String::from_utf8_lossy(bytes).replace("currentColor", "#ffffff");
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
