//! EngineRegistry 扩展测试。

use std::sync::Arc;

use deskhud_engine::{
    EngineRegistry, PetKind, PetKindInfo, PetPaint, PetPaintCtx, Plugin, PluginInfo,
};

struct ExtraPet;
impl PetKind for ExtraPet {
    fn info(&self) -> PetKindInfo {
        PetKindInfo {
            id: "pet.test.extra",
            display_name: "Extra",
            description: "t",
            author: "test",
            homepage: None,
            version: "0.0.1",
            engine: "0.2",
            window_width: 120.0,
            window_height: 120.0,
            preview_png: None,
        }
    }
    fn paint(&self, _: PetPaintCtx<'_>) -> PetPaint {
        PetPaint::default()
    }
}

struct ExtraPlugin;
impl Plugin for ExtraPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "hud.test.extra",
            display_name: "P",
            description: "t",
            author: "test",
            homepage: None,
            version: "0.0.1",
            engine: "0.2",
            icon_png: None,
        }
    }
}

#[test]
fn empty_registry_has_no_builtins() {
    let host = EngineRegistry::new();
    assert!(host.pet_infos().is_empty());
    assert!(host.plugin_infos().is_empty());
    assert!(host.all_hud_contributions().is_empty());
}

#[test]
fn register_extra() {
    let mut host = EngineRegistry::new();
    host.register_pet(Arc::new(ExtraPet));
    host.register_plugin(Arc::new(ExtraPlugin));
    assert_eq!(host.active_pet_id(), "pet.test.extra");
    assert!(host.set_active_pet("pet.test.extra"));
    assert!(host.plugin_infos().iter().any(|p| p.id == "hud.test.extra"));
}
