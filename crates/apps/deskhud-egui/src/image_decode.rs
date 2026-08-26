use egui::ColorImage;
use resvg::{tiny_skia, usvg};

pub fn decode(bytes: &[u8], edge: u32) -> Option<ColorImage> {
    let text = std::str::from_utf8(bytes).ok().map(str::trim_start);
    if text.is_some_and(|s| s.starts_with('<')) {
        let tree = usvg::Tree::from_data(bytes, &usvg::Options::default()).ok()?;
        let size = tree.size();
        let scale = (edge as f32 / size.width()).min(edge as f32 / size.height());
        let mut pixmap = tiny_skia::Pixmap::new(
            (size.width() * scale).max(1.0) as u32,
            (size.height() * scale).max(1.0) as u32,
        )?;
        resvg::render(
            &tree,
            tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        return Some(ColorImage::from_rgba_premultiplied(
            [pixmap.width() as usize, pixmap.height() as usize],
            pixmap.data(),
        ));
    }
    let rgba = image::load_from_memory(bytes).ok()?.into_rgba8();
    Some(ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}
