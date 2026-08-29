//! 启动引导：内置注册 + 本地包发现 + 社区 WASM 宠物注册。

use std::sync::Arc;

use deskhud_engine::{EngineRegistry, PetKind, Plugin};
use deskhud_package::PackKind;
use hud_deskhud_demo::DemoHudPlugin;
use pet_deskhud_mochi::BuiltinMochiPet;
use pet_deskhud_sesame::BuiltinSesamePet;
use tracing::{info, warn};

use crate::{DiscoveredPack, PackageLoader, RuntimeError, WasmLimits, WasmPet};

/// 引导结果：可运行的宿主注册表 + 已发现包清单。
pub struct Bootstrap {
    /// 内置宠 / 插件与可加载的社区 WASM 宠物。
    pub registry: EngineRegistry,
    /// 本地扫描到的包（含目录与 `.deskhud`），包括实例化失败的包。
    pub discovered: Vec<DiscoveredPack>,
}

fn register_builtins(registry: &mut EngineRegistry) {
    // This is the single source of truth for compile-in pets. To make a pet
    // external, remove it here and ship a WASM Component with `entry` in its
    // manifest. Keeping the list explicit avoids package-folder contents
    // accidentally changing the built-in product surface.
    const BUILTIN_PETS: &[&str] = &["pet.deskhud.mochi", "pet.deskhud.sesame"];
    for id in BUILTIN_PETS {
        match *id {
            "pet.deskhud.mochi" => {
                registry.register_pet(Arc::new(BuiltinMochiPet::default()) as Arc<dyn PetKind>);
            }
            "pet.deskhud.sesame" => {
                registry.register_pet(Arc::new(BuiltinSesamePet::default()) as Arc<dyn PetKind>);
            }
            _ => unreachable!("unknown configured built-in pet: {id}"),
        }
    }
    registry.register_plugin(Arc::new(DemoHudPlugin) as Arc<dyn Plugin>);
}

/// 启动时的默认宿主注册表（内置宠 + 演示 HUD），并扫描 profile / 用户包目录。
///
/// - 与内置 **同 ID** 的清单：仅记录发现（元数据覆盖留给后续）；不卸载内置实现。
/// - 其它宠物包：通过沙箱化 Component Model Guest 后注册；失败只隔离该包。
pub fn bootstrap_registry() -> Bootstrap {
    bootstrap_registry_result().unwrap_or_else(|err| {
        warn!(%err, "package discover failed; using builtins only");
        let mut registry = EngineRegistry::new();
        register_builtins(&mut registry);
        Bootstrap {
            registry,
            discovered: Vec::new(),
        }
    })
}

/// 可返回错误的引导（测试用）。
pub fn bootstrap_registry_result() -> Result<Bootstrap, RuntimeError> {
    let mut registry = EngineRegistry::new();
    register_builtins(&mut registry);
    let loader = PackageLoader::new();
    let discovered = loader.discover()?;
    let builtin_pet_ids: Vec<String> = registry
        .pet_infos()
        .into_iter()
        .map(|p| p.id.to_string())
        .collect();
    let builtin_plugin_ids: Vec<String> = registry
        .plugin_infos()
        .into_iter()
        .map(|p| p.id.to_string())
        .collect();
    for pack in &discovered {
        let id = &pack.manifest.id;
        if let Some(reason) = &pack.incompatible_reason {
            warn!(%id, %reason, "skip future register (incompatible)");
            continue;
        }
        let mapped =
            builtin_pet_ids.iter().any(|b| b == id) || builtin_plugin_ids.iter().any(|b| b == id);
        if mapped {
            info!(%id, "discovered pack maps to builtin (native)");
        } else if let (PackKind::Pet, Some(entry)) =
            (pack.manifest.kind, pack.manifest.entry.as_deref())
            && pack.manifest.is_external()
        {
            match std::fs::read(pack.root.join(entry)).and_then(|bytes| {
                let preview = pack
                    .manifest
                    .preview
                    .as_deref()
                    .map(|path| std::fs::read(pack.root.join(path)))
                    .transpose()?;
                WasmPet::load_with_preview(&bytes, WasmLimits::default(), preview)
                    .map_err(|e| std::io::Error::other(e.to_string()))
            }) {
                Ok(pet) => {
                    registry.register_pet(pet as Arc<dyn PetKind>);
                    info!(%id, "registered community WASM pet");
                }
                Err(error) => warn!(%id, %error, "community WASM pet rejected"),
            }
        } else if pack.manifest.entry.is_some() {
            info!(%id, "discovered community plugin (WASM plugin ABI pending)");
        } else {
            info!(%id, "discovered pack without entry (metadata only)");
        }
    }
    Ok(Bootstrap {
        registry,
        discovered,
    })
}
