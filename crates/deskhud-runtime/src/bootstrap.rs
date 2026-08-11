//! 启动引导：内置注册 + 本地包发现（社区 WASM 仍后接）。

use std::sync::Arc;

use deskhud_engine::{EngineRegistry, PetKind, Plugin};
use hud_deskhud_demo::DemoHudPlugin;
use pet_deskhud_blob::BuiltinBlobPet;
use pet_deskhud_specs::BuiltinSpecsPet;
use tracing::{info, warn};

use crate::{DiscoveredPack, PackageLoader, RuntimeError};

/// 引导结果：可运行的宿主注册表 + 已发现包清单。
pub struct Bootstrap {
    /// 内置宠 / 插件（及日后 WASM 注册）。
    pub registry: EngineRegistry,
    /// 本地扫描到的包（含目录与 `.deskhud`）；尚未 WASM 实例化的社区包也会在此。
    pub discovered: Vec<DiscoveredPack>,
}

fn register_builtins(registry: &mut EngineRegistry) {
    registry.register_pet(Arc::new(BuiltinSpecsPet::default()) as Arc<dyn PetKind>);
    registry.register_pet(Arc::new(BuiltinBlobPet::default()) as Arc<dyn PetKind>);
    registry.register_plugin(Arc::new(DemoHudPlugin) as Arc<dyn Plugin>);
}

/// 启动时的默认宿主注册表（内置宠 + 演示 HUD），并扫描 `packages/`。
///
/// - 与内置 **同 ID** 的清单：仅记录发现（元数据覆盖留给后续）；不卸载内置实现。
/// - 其它包：记入 [`Bootstrap::discovered`]，待 WASM 接入后再 `register_*`。
pub fn bootstrap_registry() -> Bootstrap {
    match bootstrap_registry_result() {
        Ok(b) => b,
        Err(err) => {
            warn!(%err, "package discover failed; using builtins only");
            let mut registry = EngineRegistry::new();
            register_builtins(&mut registry);
            Bootstrap {
                registry,
                discovered: Vec::new(),
            }
        }
    }
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
        } else if pack.manifest.entry.is_some() {
            info!(%id, "discovered community pack (wasm pending)");
        } else {
            info!(%id, "discovered pack without entry (metadata only)");
        }
    }
    Ok(Bootstrap {
        registry,
        discovered,
    })
}
