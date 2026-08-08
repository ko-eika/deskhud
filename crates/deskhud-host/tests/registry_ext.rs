//! HostRegistry 扩展测试。

use std::sync::Arc;

use deskhud_host::{HostRegistry, PetKind, PetKindInfo, PetPaint, PetPaintCtx, Plugin, PluginInfo};

struct ExtraPet;
impl PetKind for ExtraPet {
    fn info(&self) -> PetKindInfo {
        PetKindInfo {
            id: "pet.test.extra",
            display_name: "Extra",
            description: "t",
            author: "test",
            homepage: None,
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
            icon_png: None,
        }
    }
}

#[test]
fn default_has_specs_and_demo_hud() {
    let host = HostRegistry::new();
    assert!(host.pet_infos().iter().any(|p| p.id == "pet.deskhud.specs"));
    assert!(host.plugin_infos().iter().any(|p| p.id == "hud.deskhud.demo"));
    assert!(!host.all_hud_contributions().is_empty());
}

#[test]
fn register_extra() {
    let mut host = HostRegistry::new();
    host.register_pet(Arc::new(ExtraPet));
    host.register_plugin(Arc::new(ExtraPlugin));
    assert!(host.set_active_pet("pet.test.extra"));
}
