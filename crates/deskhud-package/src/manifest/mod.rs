//! 包清单（`manifest.toml`）。

use serde::{Deserialize, Serialize};

use crate::PackageError;

/// 包类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackKind {
    /// 宠物包：皮肤 + 行为。
    Pet,
    /// HUD 功能插件。
    Plugin,
}

/// `.deskhud` 根清单。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackManifest {
    /// 稳定 ID。宠物必须 `pet.<组织>.<标识>`；HUD 必须 `hud.<组织>.<标识>`。
    pub id: String,
    /// 宠物或插件。
    pub kind: PackKind,
    /// 与宿主 Guest ABI 对齐的主版本。
    pub api_version: u32,
    /// 显示名回退（无 i18n 时用）。
    pub display_name: String,
    /// 说明回退。
    #[serde(default)]
    pub description: String,
    /// 作者 / 来源（社区包必填更友好；缺省空）。
    #[serde(default)]
    pub author: String,
    /// 主页或仓库 URL。
    #[serde(default)]
    pub homepage: Option<String>,
    /// WASM 入口相对路径；内置原生包可空。
    #[serde(default)]
    pub entry: Option<String>,
    /// 宠物包主窗宽（逻辑像素）；插件包可忽略。缺省 140。
    #[serde(default = "default_window_dim")]
    pub window_width: u32,
    /// 宠物包主窗高（逻辑像素）。缺省 140。
    #[serde(default = "default_window_dim")]
    pub window_height: u32,
    /// 设置页预览图相对路径（如 `assets/preview.png`；支持 png/jpeg/gif/webp）；缺省无。
    #[serde(default)]
    pub preview: Option<String>,
    /// 包图标相对路径（宠物/插件通用；如 `assets/icon.png`）。
    #[serde(default)]
    pub icon: Option<String>,
    /// HUD 插件条目图标映射（`id` ↔ 包内图标路径）；与 Guest 声明的条目 id 对齐。
    #[serde(default)]
    pub hud: Vec<PackHudEntry>,
}

/// 清单中声明的一条 HUD 条目资源（至少用于图标；文案可由 Guest / i18n 提供）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackHudEntry {
    /// 与 `HudContribution.id` / Guest 条目 id 一致。
    pub id: String,
    /// 条目图标相对路径；缺省则宿主用默认图标。
    #[serde(default)]
    pub icon: Option<String>,
}

fn default_window_dim() -> u32 {
    140
}

/// `kind.org.pack`：至少 `min_segments` 段，首段为 `kind`，段内仅 `[a-zA-Z0-9_-]`。
fn validate_namespaced_id(
    id: &str,
    kind: &str,
    min_segments: usize,
) -> Result<(), PackageError> {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < min_segments || parts[0] != kind {
        return Err(PackageError::InvalidManifest(format!(
            "id `{id}` must be `{kind}.<org>.<id>` (at least {min_segments} segments)"
        )));
    }
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty()
            || !part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(PackageError::InvalidManifest(format!(
                "id `{id}` segment[{i}] `{part}` invalid (use [a-zA-Z0-9_-])"
            )));
        }
    }
    Ok(())
}

impl PackManifest {
    /// 当前宿主支持的 API 主版本。
    pub const SUPPORTED_API_VERSION: u32 = 1;

    /// 校验必填与版本。
    pub fn validate(&self) -> Result<(), PackageError> {
        if self.id.trim().is_empty() {
            return Err(PackageError::InvalidManifest("id is empty".into()));
        }
        if self.display_name.trim().is_empty() {
            return Err(PackageError::InvalidManifest(
                "display_name is empty".into(),
            ));
        }
        if self.api_version != Self::SUPPORTED_API_VERSION {
            return Err(PackageError::InvalidManifest(format!(
                "api_version {} unsupported (need {})",
                self.api_version,
                Self::SUPPORTED_API_VERSION
            )));
        }
        match self.kind {
            PackKind::Plugin => validate_namespaced_id(&self.id, "hud", 3)?,
            PackKind::Pet => validate_namespaced_id(&self.id, "pet", 3)?,
        }
        Ok(())
    }

    /// 从 TOML 文本解析并校验。
    pub fn parse_toml(text: &str) -> Result<Self, PackageError> {
        let m: Self = toml::from_str(text)?;
        m.validate()?;
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pet_manifest() {
        let m = PackManifest::parse_toml(
            r#"
id = "pet.community.cool_cat"
kind = "pet"
api_version = 1
display_name = "Cool Cat"
entry = "guest.wasm"
"#,
        )
        .unwrap();
        assert_eq!(m.kind, PackKind::Pet);
        assert_eq!(m.entry.as_deref(), Some("guest.wasm"));
    }

    #[test]
    fn parse_plugin_manifest_requires_hud_prefix() {
        let m = PackManifest::parse_toml(
            r#"
id = "hud.acme.clock"
kind = "plugin"
api_version = 1
display_name = "Clock"
"#,
        )
        .unwrap();
        assert_eq!(m.kind, PackKind::Plugin);

        let err = PackManifest::parse_toml(
            r#"
id = "demo.hud"
kind = "plugin"
api_version = 1
display_name = "Bad"
"#,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("hud.<org>.<id>"));
    }

    #[test]
    fn parse_pet_manifest_requires_pet_prefix() {
        let err = PackManifest::parse_toml(
            r#"
id = "builtin.specs"
kind = "pet"
api_version = 1
display_name = "Bad"
"#,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("pet.<org>.<id>"));
    }

    #[test]
    fn parse_plugin_hud_icon_entries() {
        let m = PackManifest::parse_toml(
            r#"
id = "hud.acme.clock"
kind = "plugin"
api_version = 1
display_name = "Clock"
icon = "assets/icon.png"

[[hud]]
id = "clock"
icon = "assets/clock.png"

[[hud]]
id = "tip"
"#,
        )
        .unwrap();
        assert_eq!(m.icon.as_deref(), Some("assets/icon.png"));
        assert_eq!(m.hud.len(), 2);
        assert_eq!(m.hud[0].id, "clock");
        assert_eq!(m.hud[0].icon.as_deref(), Some("assets/clock.png"));
        assert!(m.hud[1].icon.is_none());
    }
}
