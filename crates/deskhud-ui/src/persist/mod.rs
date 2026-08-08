//! 偏好持久化（TOML → 用户数据目录）。

use std::fs;
use std::path::PathBuf;

use thiserror::Error;

use crate::UiPreferences;

/// 持久化错误。
#[derive(Debug, Error)]
pub enum PersistError {
    /// IO。
    #[error("prefs io: {0}")]
    Io(#[from] std::io::Error),
    /// TOML 解析。
    #[error("prefs parse: {0}")]
    Parse(#[from] toml::de::Error),
    /// TOML 序列化。
    #[error("prefs serialize: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// 用户数据根：`%APPDATA%/DeskHud` 或 `~/.local/share/DeskHud`。
pub fn user_data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let appdata = std::env::var_os("APPDATA")?;
        Some(PathBuf::from(appdata).join("DeskHud"))
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("DeskHud"),
        )
    }
}

/// 偏好文件路径。
pub fn prefs_path() -> Option<PathBuf> {
    Some(user_data_dir()?.join("prefs.toml"))
}

/// 从磁盘加载；文件不存在或损坏时返回 `Default`。
pub fn load_or_default() -> UiPreferences {
    match load() {
        Ok(p) => p,
        Err(e) => {
            // 调用方可用 tracing；此处保持 ui crate 无日志依赖
            let _ = e;
            UiPreferences::default()
        }
    }
}

/// 从磁盘加载。
pub fn load() -> Result<UiPreferences, PersistError> {
    let path = prefs_path().ok_or_else(|| {
        PersistError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "user data dir unavailable",
        ))
    })?;
    if !path.exists() {
        return Ok(UiPreferences::default());
    }
    let text = fs::read_to_string(&path)?;
    let mut prefs: UiPreferences = toml::from_str(&text)?;
    prefs.normalize_ids();
    Ok(prefs)
}

/// 写入磁盘（自动创建目录）。
pub fn save(prefs: &UiPreferences) -> Result<(), PersistError> {
    let dir = user_data_dir().ok_or_else(|| {
        PersistError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "user data dir unavailable",
        ))
    })?;
    fs::create_dir_all(&dir)?;
    let path = dir.join("prefs.toml");
    let text = toml::to_string_pretty(prefs)?;
    let tmp = dir.join("prefs.toml.tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HudPrefs, Locale, PetPrefs, ShellPrefs};

    #[test]
    fn roundtrip_toml() {
        let mut prefs = UiPreferences {
            locale: Locale::En,
            shell: ShellPrefs {
                active_pet_kind_id: "pet.deskhud.blob".into(),
                pet_width: 96.0,
                pet_height: 96.0,
                pet_pos_x: Some(120.0),
                pet_pos_y: Some(340.0),
                pet_topmost: false,
                pet_picker_mode: crate::PetPickerMode::Grid,
                settings_width: Some(800.0),
                settings_height: Some(600.0),
                settings_pos_x: Some(80.0),
                settings_pos_y: Some(60.0),
                ..ShellPrefs::default()
            },
            pet: PetPrefs::default(),
            hud: HudPrefs::default(),
        };
        prefs.hud.set_enabled("hud.deskhud.demo", "clock", false);
        prefs
            .pet
            .set_bool("pet.deskhud.specs.config1", true);
        let text = toml::to_string_pretty(&prefs).unwrap();
        assert!(text.contains("[hud.config]"));
        assert!(text.contains("hud.deskhud.demo.clock.enable"));
        assert!(text.contains("[pet.config]"));
        assert!(text.contains("pet.deskhud.specs.config1"));
        let back: UiPreferences = toml::from_str(&text).unwrap();
        assert_eq!(back, prefs);
    }

    #[test]
    fn migrate_old_pet_id() {
        assert_eq!(
            crate::migrate_pet_id("builtin.specs"),
            "pet.deskhud.specs"
        );
    }
}
