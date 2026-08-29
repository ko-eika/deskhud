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

/// Controls whether a package is compile-in native code or a disk-loaded
/// WASM Component. `Auto` preserves compatibility with older manifests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PackLoadMode {
    /// Infer from `entry`: no entry means builtin, an entry means external.
    #[default]
    Auto,
    /// Package metadata belongs to a compile-in native implementation.
    Builtin,
    /// Load the package's `entry` as a WASM Component from disk.
    External,
}

/// `.deskhud` 根清单。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackManifest {
    /// 稳定 ID。宠物必须 `pet.<组织>.<标识>`；HUD 必须 `hud.<组织>.<标识>`。
    pub id: String,
    /// 宠物或插件。
    pub kind: PackKind,
    /// Deployment mode; this controls packaging and discovery, not native registration.
    #[serde(default)]
    pub load: PackLoadMode,
    /// 包自身 SemVer（仅展示 / 更新比较，不参与加载门闸）。
    pub version: String,
    /// 引擎兼容族，如 `"0.3"` 或 `"1"`（见 `docs/versioning.md`）。
    pub engine: String,
    /// 与引擎 Guest ABI 对齐的主版本。
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
    /// 设置页预览图相对路径（如 `assets/preview.svg`；支持 svg/png/jpeg/gif/webp）；缺省无。
    #[serde(default)]
    pub preview: Option<String>,
    /// 包图标相对路径（宠物/插件通用；如 `assets/icon.svg`）。
    #[serde(default)]
    pub icon: Option<String>,
    /// HUD 插件条目图标映射（`id` ↔ 包内图标路径）；与 Guest 声明的条目 id 对齐。
    #[serde(default)]
    pub hud: Vec<PackHudEntry>,
    /// 包内可被场景引用的资源索引。资源路径必须相对包根且唯一。
    #[serde(default)]
    pub resources: Vec<PackResource>,
}

/// 包资源的用途；首版只解释位图和 atlas/序列帧，不承诺骨骼或 3D。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackResourceKind {
    /// 普通位图。
    Image,
    /// 包含多个矩形帧的 atlas 位图。
    Atlas,
    /// 逐张图片组成的序列帧资源。
    Sequence,
    /// 宿主不解释内容的资源。
    Other,
}

/// 清单中的一个资源索引项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackResource {
    /// 场景中的稳定资源 ID。
    pub id: String,
    /// 相对包根路径。
    pub path: String,
    /// 资源用途。
    pub kind: PackResourceKind,
    /// 可选的期望像素尺寸；非零时加载后必须匹配。
    #[serde(default)]
    pub width: u32,
    /// 可选的期望像素尺寸；非零时加载后必须匹配。
    #[serde(default)]
    pub height: u32,
    /// atlas/序列帧引用，坐标和尺寸均为像素。
    #[serde(default)]
    pub frames: Vec<PackFrame>,
}

/// atlas 或序列帧中的一个矩形。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackFrame {
    /// 左上角 X。
    pub x: u32,
    /// 左上角 Y。
    pub y: u32,
    /// 帧宽度。
    pub width: u32,
    /// 帧高度。
    pub height: u32,
}

/// 清单中声明的一条 HUD 条目资源（至少用于图标；文案可由 Guest / i18n 提供）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackHudEntry {
    /// 与 `HudContribution.id` / Guest 条目 id 一致。
    pub id: String,
    /// 条目图标相对路径；缺省则引擎用默认图标。
    #[serde(default)]
    pub icon: Option<String>,
}

fn default_window_dim() -> u32 {
    140
}

fn version_looks_ok(version: &str) -> bool {
    let v = version.trim();
    !v.is_empty() && v.chars().any(|c| c.is_ascii_digit())
}

/// `kind.org.pack`：至少 `min_segments` 段，首段为 `kind`，段内仅 `[a-zA-Z0-9_-]`。
fn validate_namespaced_id(id: &str, kind: &str, min_segments: usize) -> Result<(), PackageError> {
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
    /// 当前引擎支持的 API 主版本。
    pub const SUPPORTED_API_VERSION: u32 = 3;

    /// Whether this manifest describes a disk-loaded WASM package.
    pub fn is_external(&self) -> bool {
        matches!(self.load, PackLoadMode::External)
            || (matches!(self.load, PackLoadMode::Auto) && self.entry.is_some())
    }

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
        if !version_looks_ok(&self.version) {
            return Err(PackageError::InvalidManifest(
                "version is empty or missing a digit".into(),
            ));
        }
        if self.engine.trim().is_empty() {
            return Err(PackageError::InvalidManifest("engine is empty".into()));
        }
        if self.api_version != Self::SUPPORTED_API_VERSION {
            return Err(PackageError::InvalidManifest(format!(
                "api_version {} unsupported (need {})",
                self.api_version,
                Self::SUPPORTED_API_VERSION
            )));
        }
        if matches!(self.load, PackLoadMode::External) && self.entry.is_none() {
            return Err(PackageError::InvalidManifest(
                "external package requires entry = `guest.wasm`".into(),
            ));
        }
        if matches!(self.load, PackLoadMode::Builtin) && self.entry.is_some() {
            return Err(PackageError::InvalidManifest(
                "builtin package must not declare a WASM entry".into(),
            ));
        }
        match self.kind {
            PackKind::Plugin => validate_namespaced_id(&self.id, "hud", 3)?,
            PackKind::Pet => validate_namespaced_id(&self.id, "pet", 3)?,
        }
        let mut ids = std::collections::HashSet::new();
        for resource in &self.resources {
            if resource.id.trim().is_empty() || !ids.insert(&resource.id) {
                return Err(PackageError::InvalidManifest(format!(
                    "resource id `{}` is empty or duplicated",
                    resource.id
                )));
            }
            validate_relative_path(&resource.path)?;
            if resource.kind == PackResourceKind::Atlas
                || resource.kind == PackResourceKind::Sequence
            {
                if resource.frames.is_empty() {
                    return Err(PackageError::InvalidManifest(format!(
                        "resource `{}` requires at least one frame",
                        resource.id
                    )));
                }
                for frame in &resource.frames {
                    if frame.width == 0 || frame.height == 0 {
                        return Err(PackageError::InvalidManifest(format!(
                            "resource `{}` has a zero-sized frame",
                            resource.id
                        )));
                    }
                }
            } else if !resource.frames.is_empty() {
                return Err(PackageError::InvalidManifest(format!(
                    "resource `{}` has frames but is not atlas/sequence",
                    resource.id
                )));
            }
        }
        for path in [&self.entry, &self.preview, &self.icon]
            .into_iter()
            .flatten()
        {
            validate_relative_path(path)?;
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

/// Validates a package-relative path without touching the filesystem.
pub fn validate_relative_path(path: &str) -> Result<(), PackageError> {
    let p = std::path::Path::new(path);
    if path.trim().is_empty() || p.is_absolute() || path.contains('\\') || path.contains('\0') {
        return Err(PackageError::InvalidManifest(format!(
            "path `{path}` must be a non-empty relative POSIX path"
        )));
    }
    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(PackageError::InvalidManifest(format!(
            "path `{path}` must not contain `..`"
        )));
    }
    Ok(())
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
version = "0.3.0"
engine = "0.3"
api_version = 3
display_name = "Cool Cat"
entry = "guest.wasm"
"#,
        )
        .unwrap();
        assert_eq!(m.kind, PackKind::Pet);
        assert_eq!(m.version, "0.3.0");
        assert_eq!(m.engine, "0.3");
        assert_eq!(m.entry.as_deref(), Some("guest.wasm"));
    }

    #[test]
    fn parse_plugin_manifest_requires_hud_prefix() {
        let m = PackManifest::parse_toml(
            r#"
id = "hud.acme.clock"
kind = "plugin"
version = "0.3.0"
engine = "0.3"
api_version = 3
display_name = "Clock"
"#,
        )
        .unwrap();
        assert_eq!(m.kind, PackKind::Plugin);

        let err = PackManifest::parse_toml(
            r#"
id = "demo.hud"
kind = "plugin"
version = "0.3.0"
engine = "0.3"
api_version = 3
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
version = "0.3.0"
engine = "0.3"
api_version = 3
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
version = "0.3.0"
engine = "0.3"
api_version = 3
display_name = "Clock"
icon = "assets/icon.svg"

[[hud]]
id = "clock"
icon = "assets/clock.svg"

[[hud]]
id = "tip"
"#,
        )
        .unwrap();
        assert_eq!(m.icon.as_deref(), Some("assets/icon.svg"));
        assert_eq!(m.hud.len(), 2);
        assert_eq!(m.hud[0].id, "clock");
        assert_eq!(m.hud[0].icon.as_deref(), Some("assets/clock.svg"));
        assert!(m.hud[1].icon.is_none());
    }

    #[test]
    fn reject_empty_version_or_engine() {
        let err = PackManifest::parse_toml(
            r#"
id = "pet.community.cool_cat"
kind = "pet"
version = ""
engine = "0.3"
api_version = 3
display_name = "Cool Cat"
"#,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("version"));

        let err = PackManifest::parse_toml(
            r#"
id = "pet.community.cool_cat"
kind = "pet"
version = "0.3.0"
engine = ""
api_version = 3
display_name = "Cool Cat"
"#,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("engine"));
    }
}
