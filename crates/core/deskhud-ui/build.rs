use std::{fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../../i18n");
    println!("cargo:rerun-if-changed={}", root.display());
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    for locale in ["en-US", "zh-CN"] {
        let dir = root.join(locale);
        let mut files = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|v| v.to_str()) == Some("po"))
            .collect::<Vec<_>>();
        files.sort();
        let text = files
            .into_iter()
            .map(|p| fs::read_to_string(p).unwrap())
            .collect::<Vec<_>>()
            .join("\n\n");
        fs::write(out.join(format!("shell-{locale}.po")), text).unwrap();
    }
}
