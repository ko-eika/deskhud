use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) fn user_data_dir() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(appdata).join("DeskHud"))
}

#[allow(clippy::permissions_set_readonly_false)]
pub(crate) fn ensure_writable(path: &Path) -> std::io::Result<()> {
    if let Ok(metadata) = fs::metadata(path)
        && metadata.permissions().readonly()
    {
        let mut permissions = metadata.permissions();
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

pub(crate) fn write_config(tmp: &Path, path: &Path, text: &str) -> std::io::Result<()> {
    // Windows rename() cannot reliably replace a file being inspected by the
    // shell or antivirus, so write and flush the exact target first.
    let direct_write = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .and_then(|mut file| {
            file.write_all(text.as_bytes())?;
            file.sync_all()
        });
    if direct_write.is_err() {
        // Some Windows file providers reject truncation but allow replacement
        // by rename after the old entry is removed.
        fs::remove_file(path)?;
        fs::rename(tmp, path)?;
    } else {
        let _ = fs::remove_file(tmp);
    }
    Ok(())
}
