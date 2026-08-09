//! 设置页图标 / 预览：SVG 与光栅图统一解码为 RGBA。

use eframe::egui::ColorImage;
use resvg::tiny_skia;
use resvg::usvg;

/// 图标默认栅格边长（逻辑像素，纹理像素）。
pub const ICON_RASTER_EDGE: u32 = 128;
/// 宠物预览默认栅格边长。
pub const PREVIEW_RASTER_EDGE: u32 = 512;

/// 将资源字节解码为 egui [`ColorImage`]。
///
/// - 嗅探为 SVG → `resvg` 栅格化到不超过 `max_edge` 的矩形（保持宽高比）
/// - 否则 → `image` crate（png/jpeg/gif/webp）
pub fn decode_to_color_image(bytes: &[u8], max_edge: u32) -> Option<ColorImage> {
    if looks_like_svg(bytes) {
        return rasterize_svg(bytes, max_edge);
    }
    match image::load_from_memory(bytes) {
        Ok(img) => {
            let rgba = img.into_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            Some(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
        }
        Err(_) => {
            // 扩展名/嗅探失败时再试 SVG（部分文件无前导空白）
            rasterize_svg(bytes, max_edge)
        }
    }
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let t = std::str::from_utf8(bytes)
        .ok()
        .map(|s| s.trim_start())
        .unwrap_or("");
    t.starts_with("<?xml") || t.starts_with("<svg") || t.starts_with("<!DOCTYPE svg")
}

fn rasterize_svg(bytes: &[u8], max_edge: u32) -> Option<ColorImage> {
    let max_edge = max_edge.max(1);
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(bytes, &opt).ok()?;
    let size = tree.size();
    let sw = size.width().max(1.0);
    let sh = size.height().max(1.0);
    let scale = (max_edge as f32 / sw).min(max_edge as f32 / sh);
    let pw = (sw * scale).round().max(1.0) as u32;
    let ph = (sh * scale).round().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(pw, ph)?;
    // 透明底，适配深/浅色设置页
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let w = pixmap.width() as usize;
    let h = pixmap.height() as usize;
    Some(ColorImage::from_rgba_unmultiplied([w, h], pixmap.data()))
}
