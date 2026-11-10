//! Game-side menu host stack: backend-agnostic page model, dispatcher, item
//! effects, and the flat-grid / 3D-cube presentation hosts.
//!
//! `ambition_platformer2d::menu` (the `ambition_menu` crate) owns the Map tab; this module keeps
//! backend selector; this crate owns app-level wiring and effects.

/// Run Grid navigation before Cube navigation. The active backend consumes only
/// its menu owner's edges. A backend switch cannot apply those edges twice.
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GridMenuNav;

/// The cube backend's half of [`GridMenuNav`]'s ordering.
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct KaleidoscopeMenuNav;

#[cfg(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
mod build_label;
#[cfg(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
mod regions;
#[cfg(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
pub(crate) mod lifecycle;
#[cfg(feature = "input")]
mod bindings;
#[cfg(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
mod departure;
#[cfg(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
mod system_cache;

#[cfg(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
#[derive(bevy::prelude::SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MenuActionsApplied;

pub mod dispatch;
pub mod effects;
#[cfg(feature = "bevy_ui_menu")]
pub mod grid_backend;
// Always compiled, despite the name. (repair_wasm §1)
//
// Everything else is the backend-neutral menu host — the cursor, the system-menu navigation,
// item actions, page building — which `dispatch.rs` and the bevy_ui `grid_backend.rs` import
// unconditionally because they genuinely need it. So a build without the cube (the web persona)
// failed to compile the FLAT menu, which has nothing to do with Lunex.
//
// The cube renderer itself is still gated, item by item, inside the module. A
// headless or web build pays for no 3D UI toolkit; it just gets to have a menu.
pub mod kaleidoscope_app;
pub mod map_page;
pub mod model;
pub(crate) mod quality_confirm;
#[cfg(test)]
mod test_support;

#[cfg(all(test, feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
mod parity_tests;

/// State owned by one inventory menu. Both backends use these components.
#[derive(bevy::prelude::Component, Default)]
#[require(kaleidoscope_app::KaleidoscopeCursor, kaleidoscope_app::KaleidoscopeSystemNav,
    kaleidoscope_app::RebindCapture, quality_confirm::VisualQualityConfirmState)]
#[cfg_attr(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"), require(lifecycle::MenuLifecycleState))]
#[cfg_attr(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"), require(regions::MenuRegion, system_cache::CachedSystemMenu))]
#[cfg_attr(feature = "bevy_ui_menu", require(grid_backend::GridMenuTabState))]
#[cfg_attr(feature = "kaleidoscope_menu", require(kaleidoscope_app::KaleidoscopeScroll,
    kaleidoscope_app::MenuPublication, ambition_menu_kaleidoscope::KaleidoscopeOpenState))]
pub(crate) struct MenuInstance;

#[cfg(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
fn inventory_input_allowed(
    access: &ambition_platformer2d::input::ParticipantContextAccess,
    participant: ambition_platformer2d::input::ParticipantId,
) -> bool {
    use ambition_platformer2d::input::{ContextClaim, INVENTORY_CONTEXT, participant::context_priority};
    access.permits(participant, ContextClaim::capturing(INVENTORY_CONTEXT, context_priority::INVENTORY))
}
