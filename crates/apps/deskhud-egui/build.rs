use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let icon_ico = manifest_dir.join("../../../assets/icon.ico");
    let icon_icns = manifest_dir.join("../../../assets/icon.icns");
    let icon_png = manifest_dir.join("../../../assets/icon.png");
    let icon_svg = manifest_dir.join("../../../assets/icon.svg");
    println!("cargo:rerun-if-changed={}", icon_ico.display());
    println!("cargo:rerun-if-changed={}", icon_icns.display());
    println!("cargo:rerun-if-changed={}", icon_png.display());
    println!("cargo:rerun-if-changed={}", icon_svg.display());

    #[cfg(windows)]
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_icon(&icon_ico);
    }

    let source = manifest_dir.join("../../../assets/fonts");
    println!("cargo:rerun-if-changed={}", source.display());

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let target_profile = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR should be inside target/<profile>/build");
    if source.is_dir() {
        let destination = target_profile.join("fonts");
        copy_tree(&source, &destination).expect("copy DeskHud fonts to target profile");
    }
    let locales = manifest_dir.join("../../../i18n");
    println!("cargo:rerun-if-changed={}", locales.display());
    compile_locales(&locales, &target_profile.join("i18n")).expect("compile PO catalogs");
}

fn compile_locales(source: &Path, destination: &Path) -> std::io::Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    for locale in fs::read_dir(source)? {
        let locale = locale?;
        if !locale.path().is_dir() {
            continue;
        }
        let out = destination.join(locale.file_name());
        fs::create_dir_all(&out)?;
        for entry in fs::read_dir(locale.path())? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("po") {
                continue;
            }
            let mo = out.join(path.file_stem().unwrap()).with_extension("mo");
            let po = fs::read_to_string(&path)?;
            // Keep the build self-contained. The built-in compiler emits a
            // standard GNU gettext MO file from the supported PO syntax.
            fs::write(&mo, compile_mo(&po))?;
        }
    }
    Ok(())
}

fn compile_mo(po: &str) -> Vec<u8> {
    let mut entries = Vec::new();
    let mut id: Option<String> = None;
    let mut value: Option<String> = None;
    let mut target = false;
    for line in po.lines().chain(std::iter::once("")) {
        let line = line.trim();
        if line.is_empty() {
            if let (Some(i), Some(v)) = (id.take(), value.take())
                && !i.is_empty()
                && !v.is_empty()
            {
                entries.push((i, v));
            }
            target = false;
            continue;
        }
        if let Some(v) = line.strip_prefix("msgid ") {
            id = Some(unquote(v));
            target = false;
        } else if let Some(v) = line.strip_prefix("msgstr ") {
            value = Some(unquote(v));
            target = true;
        } else if line.starts_with('"') {
            let v = unquote(line);
            if target {
                value.get_or_insert_default().push_str(&v);
            } else {
                id.get_or_insert_default().push_str(&v);
            }
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let n = entries.len() as u32;
    let ot = 28;
    let tt = ot + n * 8;
    let mut data = tt + n * 8;
    let mut originals = Vec::new();
    let mut translations = Vec::new();
    let mut strings = Vec::new();
    for (i, v) in &entries {
        originals.push((i.len() as u32, data));
        strings.extend(i.as_bytes());
        strings.push(0);
        data += i.len() as u32 + 1;
        translations.push((v.len() as u32, data));
        strings.extend(v.as_bytes());
        strings.push(0);
        data += v.len() as u32 + 1;
    }
    let mut out = Vec::with_capacity(data as usize);
    for x in [0x9504_12de_u32, 0, n, ot, tt, 0, 0] {
        out.extend(x.to_le_bytes());
    }
    for (l, o) in originals.into_iter().chain(translations) {
        out.extend(l.to_le_bytes());
        out.extend(o.to_le_bytes());
    }
    out.extend(strings);
    out
}

fn unquote(raw: &str) -> String {
    let raw = raw.trim();
    let inner = raw
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or("");
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            out.push(match chars.next().unwrap_or('\\') {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '"' => '"',
                '\\' => '\\',
                x => x,
            });
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(windows)]
fn embed_windows_icon(icon: &Path) {
    let mut resource = winresource::WindowsResource::new();
    let icon_path = icon.to_string_lossy();
    resource.set_icon(icon_path.as_ref());
    if let Err(error) = resource.compile() {
        // A non-MSVC resource toolchain can still build the application;
        // only the Explorer/executable icon falls back in that case.
        println!("cargo:warning=embed icon.ico failed: {error}");
    }
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else if source_path.is_file()
            && source_path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| {
                    matches!(
                        extension.to_ascii_lowercase().as_str(),
                        "ttf" | "otf" | "ttc"
                    )
                })
        {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}
