//! 中文字体注入。

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

#[cfg(windows)]
const CANDIDATES: &[&str] = &[
    r"C:\Windows\Fonts\msyh.ttc",
    r"C:\Windows\Fonts\simhei.ttf",
    r"C:\Windows\Fonts\simsun.ttc",
];

#[cfg(not(windows))]
const CANDIDATES: &[&str] = &[
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/System/Library/Fonts/PingFang.ttc",
];

/// 配置 CJK 字体。
pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let Some((name, bytes)) = load_cjk() else {
        tracing::warn!("未找到系统中文字体");
        return;
    };
    fonts
        .font_data
        .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, name.clone());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push(name);
    ctx.set_fonts(fonts);
}

fn load_cjk() -> Option<(String, Vec<u8>)> {
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            let name = std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("cjk")
                .to_string();
            return Some((name, bytes));
        }
    }
    None
}
