use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = manifest_dir.join("../../../assets/fonts");
    println!("cargo:rerun-if-changed={}", source.display());

    if !source.is_dir() {
        return;
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let target_profile = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR should be inside target/<profile>/build");
    let destination = target_profile.join("fonts");
    copy_tree(&source, &destination).expect("copy DeskHud fonts to target profile");
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
