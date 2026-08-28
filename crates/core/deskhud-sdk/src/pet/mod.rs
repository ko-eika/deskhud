//! Pet Guest authoring helpers.
//!
//! Implement the generated [`crate::Guest`] trait. The WIT types are the ABI;
//! this module retains the small metadata types used by package tooling.

/// Legacy authoring shape retained for source compatibility; new components
/// should implement [`crate::Guest`] instead.
pub trait PetGuest {
    /// 稳定 ID（与 manifest.id 一致）。
    fn id(&self) -> &str;

    /// 每帧或定时推进行为状态。
    fn tick(&mut self, _dt_secs: f32) {}

    /// Legacy event hook.
    fn on_event_placeholder(&mut self) {}
}
