//! The demo-hosting seam (decomposition D-C, vision §5): **scoped game modes**.
//!
//! Ambition hosts its own demos. A demo's rules crate exposes a
//! `<Demo>RulesPlugin` whose systems are gated on the ACTIVE ROOM's mode tag —
//! not on a global `State`, which only one ruleset could ever own at a time.
//! Several hosted rulesets therefore coexist in one binary, each awake only
//! inside the rooms that opted into it:
//!
//! ```ignore
//! impl Plugin for SanicRulesPlugin {
//!     fn build(&self, app: &mut App) {
//!         let sim = app.sim_schedule();
//!         let rules = (accumulate_momentum, ride_the_loop);
//!         if self.hosted {
//!             app.add_systems(sim, rules.run_if(in_mode("sanic")));
//!         } else {
//!             app.add_systems(sim, rules);   // standalone: the whole game IS the demo
//!         }
//!     }
//! }
//! ```
//!
//! `SanicRulesPlugin::hosted()` vs `::global()` is a CONSTRUCTOR FLAG, not two
//! plugins — the rules are one list, and only their gating differs.
//!
//! Mode-owned state rides an entity carrying [`ModeScopedEntity`] (spawn it with
//! `Commands::spawn_mode_scoped`), so leaving the mode's rooms tears it down
//! through the same lifetime-scope vocabulary a room-scoped entity uses. The
//! sweep lives here rather than beside the marker because it reads the active
//! room's metadata, which is a tier above the lifecycle primitives.

use bevy::prelude::*;

use ambition_platformer_primitives::lifecycle::{despawn_scoped_entity, ModeScopedEntity};
use ambition_platformer_primitives::schedule::{SandboxSet, SimScheduleExt as _};
use ambition_world::rooms::ActiveRoomMetadata;

/// Run condition: the active room belongs to the game mode `name`.
///
/// The absent-resource case is `false`: an app with no world installed is in no
/// mode, so a hosted ruleset stays asleep rather than panicking. `None` mode
/// metadata is the base game, and matches no named mode.
pub fn in_mode(name: &'static str) -> impl FnMut(Option<ambition_platformer_primitives::lifecycle::SessionWorldRef<ActiveRoomMetadata>>) -> bool + Clone {
    move |active: Option<ambition_platformer_primitives::lifecycle::SessionWorldRef<ActiveRoomMetadata>>| {
        active.is_some_and(|active| active.0.mode.as_deref() == Some(name))
    }
}

/// Despawn every [`ModeScopedEntity`] whose mode is not the active room's.
///
/// Runs only when `ActiveRoomMetadata` changes — `sync_active_room_metadata`
/// writes it behind a `PartialEq` guard, so "changed" already means the active
/// room's metadata really differs. Entities of the mode we just entered survive;
/// so does everything belonging to a mode we never left, which is exactly what
/// makes a mode a lifetime distinct from a room.
pub fn despawn_departed_mode_entities(
    mut commands: Commands,
    active: Option<ambition_platformer_primitives::lifecycle::SessionWorldRef<ActiveRoomMetadata>>,
    scoped: Query<(Entity, &ModeScopedEntity)>,
) {
    let Some(active) = active else { return };
    if !active.is_changed() {
        return;
    }
    let current = active.0.mode.as_deref();
    for (entity, scope) in scoped.iter() {
        if current != Some(scope.0.as_str()) {
            despawn_scoped_entity(&mut commands, entity);
        }
    }
}

/// Owns the mode-scope lifetime: the sweep that retires a departed mode's
/// entities. The run condition [`in_mode`] is a free function because a rules
/// plugin attaches it to its OWN systems.
pub struct ModeScopePlugin;

impl Plugin for ModeScopePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // After the canonical metadata component publishes this frame's active
        // room, so a transition INTO a different mode tears the old mode down
        // on the same frame it becomes stale.
        app.add_systems(
            sim,
            despawn_departed_mode_entities
                .after(ambition_actors::rooms::sync_active_room_metadata)
                .in_set(SandboxSet::Progression),
        );
    }
}
