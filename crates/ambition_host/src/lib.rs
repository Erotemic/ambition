//! The windowed-HOST face — [the windowed host] (decomposition E5 step 5):
//! [`PlatformerHostPlugins`], a Bevy [`PluginGroup`] that assembles the
//! per-frame wiring a VISIBLE platformer host layers on top of
//! [`ambition_runtime::PlatformerEnginePlugins`] — the player-input /
//! player-simulation (engine-generic half) / room-transition registrations,
//! the portal-schedule wiring, the camera follow/shake cluster, and the input
//! plugins.
//!
//! ## Why this crate
//!
//! A VISIBLE game (Ambition, or a demo) builds its host App by adding the
//! engine group + this host group + its own content crate:
//!
//! ```ignore
//! let mut app = App::new();
//! ambition_runtime::add_headless_foundation(&mut app); // or DefaultPlugins
//! app.add_plugins(ambition_runtime::PlatformerEnginePlugins)
//!    .add_plugins(ambition_host::PlatformerHostPlugins)   // <- this group
//!    .add_plugins(my_content::MyGameContentPlugin);
//! ```
//!
//! The host MAY dep `ambition_render` / `ambition_input` / `leafwing-input-
//! manager` / `ambition_gameplay_core`; it must NEVER dep `ambition_content`
//! (enforced by `tests/host_names_no_content.rs`). A headless / RL entry point
//! adds only the engine group; a windowed host adds this one too.
//!
//! ## SCAFFOLD status (opus 2026-07-06)
//!
//! Minted EMPTY so the E5-step-5 carve is a pure system-move: fable fills
//! [`PlatformerHostPlugins`] from the movable `register_*` fns in
//! `ambition_app/src/app/plugins.rs`, one plugin per domain (anti-god rule 2),
//! per the **E5 step-5 readiness brief** in `docs/planning/engine/
//! decomposition.md` (the exact host-generic vs app-local split + the three
//! `wire_portal_schedule` ordering landmines to preserve). The existing portal
//! / gravity / camera-continuity app suites are the parity harness — port
//! boldly, they catch an ordering break.

use bevy::app::{App, Plugin, PluginGroup, PluginGroupBuilder};

/// The windowed-host plugin group. EMPTY scaffold today (see the crate docs):
/// fable adds the per-frame host plugins here in E5 step 5. Kept non-empty by a
/// single no-op marker plugin so `PluginGroupBuilder` has a home and consumers
/// can already `add_plugins(PlatformerHostPlugins)` against a stable seam.
pub struct PlatformerHostPlugins;

impl PluginGroup for PlatformerHostPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            // The seam. Fable replaces this with the real host plugins:
            //   .add(HostInputPlugin)          // register_player_input_systems
            //   .add(HostRoomTransitionPlugin) // register_room_transition_systems
            //   .add(HostPortalSchedulePlugin) // wire_portal_schedule (feature = "portal")
            //   .add(HostCameraPlugin)         // camera follow/shake cluster
            //   .add(HostInputBindingsPlugin)  // add_input_plugins
            .add(HostSeamPlugin)
    }
}

/// No-op marker plugin holding the [`PlatformerHostPlugins`] seam open while the
/// group is an empty scaffold. Registering it is harmless; fable removes it once
/// the real host plugins land.
struct HostSeamPlugin;

impl Plugin for HostSeamPlugin {
    fn build(&self, _app: &mut App) {}
}
