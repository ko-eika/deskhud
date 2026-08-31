use std::{
    fs,
    path::{Path, PathBuf},
};

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../packs");
    println!("cargo:rerun-if-changed={}", root.display());
    let workspace_i18n =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../../i18n");
    println!("cargo:rerun-if-changed={}", workspace_i18n.display());
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    for locale in ["zh-CN", "en-US"] {
        let mut input_catalog = String::new();
        for name in ["keys.po", "settings.po"] {
            let path = workspace_i18n.join(locale).join(name);
            if let Ok(text) = fs::read_to_string(&path) {
                if !input_catalog.is_empty() {
                    input_catalog.push_str("\n\n");
                }
                input_catalog.push_str(&text);
            }
        }
        fs::write(
            out.join(format!("builtin-input-{locale}.po")),
            input_catalog,
        )
        .unwrap();
    }
    for (name, pack) in [
        ("mochi", "pet-deskhud-mochi"),
        ("sesame", "pet-deskhud-sesame"),
        ("demo", "hud-deskhud-demo"),
    ] {
        for locale in ["zh-CN", "en-US"] {
            let dir = root.join(pack).join("i18n").join(locale);
            let mut files = fs::read_dir(&dir)
                .unwrap()
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|v| v.to_str()) == Some("po"))
                .collect::<Vec<_>>();
            files.sort();
            let text = files
                .into_iter()
                .map(|p| read(&p))
                .collect::<Vec<_>>()
                .join("\n\n");
            fs::write(out.join(format!("builtin-{name}-{locale}.po")), text).unwrap();
        }
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}
