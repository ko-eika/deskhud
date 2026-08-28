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
