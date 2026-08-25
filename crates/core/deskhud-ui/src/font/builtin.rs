//! Bundled font assets and their toolkit-independent metadata.

use super::{FontCatalog, FontContainerFace, FontFace, FontFamilyEntry};

/// One font file embedded in the application.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinFontAsset {
    /// File name used as the stable source name.
    pub file_name: &'static str,
    /// Embedded font container bytes.
    pub bytes: &'static [u8],
}

static BUILTIN_FONT_ASSETS: &[BuiltinFontAsset] = &[BuiltinFontAsset {
    file_name: "Inter.ttc",
    bytes: include_bytes!("../../../../../assets/fonts/Inter.ttc"),
}];

/// Returns all bundled font files without exposing any UI framework type.
pub fn builtin_font_assets() -> &'static [BuiltinFontAsset] {
    BUILTIN_FONT_ASSETS
}

/// Returns the embedded bytes and face index for a bundled font identifier.
pub fn builtin_font_data(id: &str) -> Option<(&'static [u8], u32)> {
    let (source, index) = if let Some((source, index)) = id.split_once("#face=") {
        (source, index.parse().ok()?)
    } else {
        (id, 0)
    };
    let asset = BUILTIN_FONT_ASSETS.iter().find(|asset| {
        asset.file_name == source
            || std::path::Path::new(asset.file_name)
                .file_stem()
                .and_then(|stem| stem.to_str())
                == Some(source)
    })?;
    Some((asset.bytes, index))
}

/// Scans bundled files and groups all faces by family and style.
pub fn builtin_font_families() -> Vec<FontFamilyEntry> {
    let mut catalog = FontCatalog::default();
    for asset in BUILTIN_FONT_ASSETS {
        let Ok(faces) = super::inspect_font_bytes(asset.bytes) else {
            continue;
        };
        for face in faces {
            add_face(asset, face, &mut catalog);
        }
    }
    catalog.into_entries()
}

fn add_face(asset: &BuiltinFontAsset, face: FontContainerFace, catalog: &mut FontCatalog) {
    let family_stem = face.family.as_deref().unwrap_or(asset.file_name);
    let (family_key, label, parsed_style, aliases) = super::classify_stem(family_stem);
    catalog.upsert(
        family_key,
        label,
        aliases,
        FontFace {
            style: face.subfamily.unwrap_or(parsed_style),
            font_id: format!(
                "{}#face={}",
                asset.file_name.trim_end_matches(".ttc"),
                face.face_index
            ),
            builtin: true,
        },
    );
}
