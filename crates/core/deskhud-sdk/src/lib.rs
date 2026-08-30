//! Guest-side bindings for the DeskHud WASM Component Model contract.
//!
//! Community components only see neutral records and never receive egui,
//! HWND, or other host implementation types.

#![deny(missing_docs)]

pub mod pet;
pub mod plugin;

/// Generated WIT bindings and the `Guest` trait implemented by a pet.
pub mod bindings {
    #![allow(missing_docs)]
    wit_bindgen::generate!({
        path: "wit",
        world: "pet-guest",
    });
}

/// Guest ABI main version; this must match the package manifest.
/// Must match `PackManifest::SUPPORTED_API_VERSION` and the WIT contract
/// consumed by the host runtime.
pub const API_VERSION: u32 = 3;

/// Guest ABI trait implemented by a community pet.
pub use bindings::exports::deskhud::guest::pet_api::Guest;

/// Generated WIT records and variants used by a Guest implementation.
pub use bindings::exports::deskhud::guest::pet_api;

/// Exports a Guest implementation as the `pet-guest` world.
#[macro_export]
macro_rules! export_pet {
    ($guest:ty) => {
        $crate::bindings::__export_pet_guest_impl!($guest);
    };
}
