//! Scoped game-mode runtime for hosted demos/rulesets.
//!
//! Rules may coexist in one app and gate their systems on the active room's mode
//! tag rather than owning a global state. Standalone compositions can install the
//! same rules ungated. Mode-owned entities carry `ModeScopedEntity` and are swept
//! when routing leaves the mode.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{despawn_scoped_entity, ModeScopedEntity};
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
};
use ambition_platformer2d_world::rooms::RoomSet;

/// Run condition: the active room belongs to the game mode `name`.
///
/// The absent-resource case is `false`: an app with no world installed is in no
/// mode, so a hosted ruleset stays asleep rather than panicking. `None` mode
/// metadata is the base game, and matches no named mode.
pub fn in_mode(
    name: &'static str,
) -> impl FnMut(Option<ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>>) -> bool
       + Clone {
    move |rooms: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>>| {
        rooms.is_some_and(|rooms| rooms.active_metadata().mode.as_deref() == Some(name))
    }
}

/// Run condition: a live session is in Ambition's OWN base mode — an active room
/// that carries no named demo mode tag.
///
/// The mirror of [`in_mode`]: where `in_mode("sanic")` wakes a hosted demo's rules
/// only inside that demo's rooms, `in_base_mode` wakes the host's OWN chrome only
/// when the live session is Ambition's, not a hosted Sanic/Mary-O/Pocket session.
/// The absent-resource case is `false` (no active room  no session  frontend /
/// title), so gating a host-only menu on this keeps it dormant on the title screen
/// AND inside a hosted demo — exactly "a live session exists AND it is Ambition's
/// mode". Pair it with the canonical session gate
/// [`ambition_platformer2d_shared_tangle::lifecycle::simulation_authorized`] when a
/// system also needs the full scope-identity guarantee.
pub fn in_base_mode(
    rooms: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>>,
) -> bool {
    rooms.is_some_and(|rooms| rooms.active_metadata().mode.is_none())
}

/// Despawn every [`ModeScopedEntity`] whose mode is not the active room's.
///
/// Runs only when `RoomSet` changes, which is a room publication or a world
/// replacement. A room change inside one mode leaves that mode's entities alone:
/// the sweep compares modes, not rooms, which is what makes a mode a lifetime
/// distinct from a room.
pub fn despawn_departed_mode_entities(
    mut commands: Commands,
    rooms: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>>,
    scoped: Query<(Entity, &ModeScopedEntity)>,
) {
    let Some(rooms) = rooms else { return };
    if !rooms.is_changed() {
        return;
    }
    let current = rooms.active_metadata().mode.as_deref();
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
        app.add_systems(
            sim,
            despawn_departed_mode_entities
                .in_set(Platformer2dSimulationPhaseMonolith::Progression),
        );
    }
}
