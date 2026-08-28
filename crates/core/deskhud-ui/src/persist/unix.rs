use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn user_data_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("DeskHud"),
    )
}

pub(crate) fn ensure_writable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

pub(crate) fn write_config(tmp: &Path, path: &Path, _text: &str) -> std::io::Result<()> {
    fs::rename(tmp, path)
}
