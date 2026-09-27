//! Scoped game-mode runtime for hosted demos/rulesets.
//!
//! Rules may coexist in one app and gate their systems on the active room's mode
//! tag rather than owning a global state. Standalone compositions can install the
//! same rules ungated. Mode-owned entities carry `ModeScopedEntity` and are swept
//! when routing leaves the mode.

use bevy::prelude::*;

use ambition_combat::scoped_rules::RulesScope;
use ambition_platformer2d_shared_tangle::lifecycle::{
    despawn_scoped_entity, ModeScopedEntity, ModeVisit, SessionWorldRef,
};
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
};
use ambition_platformer2d_actor_monolith::session::governing_rules::CurrentRoom;
use ambition_platformer2d_world::rooms::RoomSet;

/// Run condition: the active room belongs to the game mode `name`.
///
/// The absent-resource case is `false`: an app with no world installed is in no
/// mode, so a hosted ruleset stays asleep rather than panicking. `None` mode
/// metadata is the base game, and matches no named mode.
pub fn in_mode(name: &'static str) -> impl FnMut(CurrentRoom) -> bool + Clone {
    in_rules_scope(RulesScope::Mode(name))
}

/// Run condition: the active room is one that `scope` governs, the same rooms a
/// `declare_rules(scope, ..)` statement governs. A game states its scope once
/// and gates both its declared rules and its systems with that one value, so a
/// hosted and a standalone composition differ in the scope and in nothing else.
///
/// It asks [`RulesScope::governs`], the question `GoverningRules` asks, so a
/// gated system runs exactly where the same scope's rules govern. `EveryRoom`
/// is `true` with no world at all, because a standalone game's rules are the
/// binary's rules.
pub fn in_rules_scope(scope: RulesScope) -> impl FnMut(CurrentRoom) -> bool + Clone {
    move |room: CurrentRoom| room.in_scope(scope)
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
pub fn in_base_mode(room: CurrentRoom) -> bool {
    room.in_scope(RulesScope::UntaggedRooms)
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

/// Project the active room's declared rule `T` into `R`, a resource that a
/// crate which cannot see rooms reads.
///
/// `R` is rebuilt from the declaration and the active room every time this
/// runs, so it is a read model: nothing else writes it. It is written only when
/// the answer changes, so a reader that gates on change detection sees one
/// change per room change. An app without `R` has no reader, and this does
/// nothing.
pub fn project_room_rule<T, R>(
    rule: ambition_platformer2d_actor_monolith::session::governing_rules::GoverningRules<T>,
    out: Option<ResMut<R>>,
) where
    T: Clone + std::fmt::Debug + Send + Sync + 'static,
    R: Resource
        + bevy::ecs::component::Component<Mutability = bevy::ecs::component::Mutable>
        + PartialEq
        + From<Option<T>>,
{
    let Some(mut out) = out else { return };
    let next = R::from(rule.get());
    if *out != next {
        *out = next;
    }
}

/// The set in which every declared mode owner is brought into being and told
/// which room it is in ([`ModeVisit`]). It runs in `GameplayEffects`, and a
/// game's rules that read their owner on the tick it is born run `.after` it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModeOwnersSpawned;

/// Record on each mode owner the room it is in this tick, and how it came there.
///
/// The one reading of "has the mode arrived in a room": each game read it by
/// remembering a room on its own owner, one by index and one by id.
pub fn follow_mode_owner_rooms(
    rooms: Option<SessionWorldRef<RoomSet>>,
    mut owners: Query<&mut ModeVisit>,
) {
    let Some(rooms) = rooms else {
        return;
    };
    let active = rooms.active_spec().id.as_str();
    for mut visit in &mut owners {
        let next = visit.after(active);
        if *visit != next {
            *visit = next;
        }
    }
}

/// The two steps of [`ModeOwnersSpawned`]: owners are born, then each is told
/// the room it is in, so an owner born this tick sees `Arrival::First`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ModeOwnerStep {
    Born,
    Placed,
}

fn configure_mode_owner_sets(app: &mut App) {
    let sim = app.sim_schedule();
    app.configure_sets(
        sim,
        ModeOwnersSpawned.in_set(Platformer2dSimulationPhaseMonolith::GameplayEffects),
    );
    app.configure_sets(
        sim,
        (ModeOwnerStep::Born, ModeOwnerStep::Placed)
            .chain()
            .in_set(ModeOwnersSpawned),
    );
}

/// Bring `mode`'s owner into being on the first tick the rooms `scope` governs
/// are live.
///
/// A hosted game keeps its level or act state on one entity per mode. The
/// owner is spawned with `spawn_mode_owner`, so the mode sweep retires it when
/// the mode ends, the session retires it on a relaunch, and its `SimId` names
/// it for the rollback order. That `SimId` is also how this system finds an
/// owner that already exists. No owner is spawned while no session is live (at
/// the launcher), so stale room metadata cannot bring one back.
///
/// `owner` is a function, not a value, so each spawn starts from the authored
/// state and never from a copy an earlier owner changed.
pub fn install_mode_owner<B: Bundle>(
    app: &mut App,
    scope: RulesScope,
    mode: &'static str,
    owner: fn() -> B,
) {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt as _,
    };
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    let identity = SimId::singleton("mode_owner", mode);
    let spawn_owner = move |mut commands: Commands,
                            owners: Query<&SimId, With<ModeScopedEntity>>,
                            session: Option<Res<ActiveSessionScope>>| {
        let session_live = session.as_ref().is_none_or(|scope| scope.current().is_some());
        if !session_live || owners.iter().any(|existing| *existing == identity) {
            return;
        }
        let spawn_scope = session
            .as_ref()
            .map_or(SessionSpawnScope::UNSCOPED, |scope| scope.spawn_scope());
        commands.spawn_mode_owner(spawn_scope, mode, owner());
    };
    configure_mode_owner_sets(app);
    let sim = app.sim_schedule();
    app.add_systems(
        sim,
        spawn_owner
            .in_set(ModeOwnerStep::Born)
            .run_if(in_rules_scope(scope)),
    );
}

/// Owns the mode-scope lifetime: the sweep that retires a departed mode's
/// entities, and [`follow_mode_owner_rooms`], which tells each mode owner the
/// room it is in. Installed once with the engine; a second follow in one tick
/// would call every arrival a stay. The run condition [`in_mode`] is a free
/// function because a rules plugin attaches it to its OWN systems.
pub struct ModeScopePlugin;

impl Plugin for ModeScopePlugin {
    fn build(&self, app: &mut App) {
        configure_mode_owner_sets(app);
        let sim = app.sim_schedule();
        app.add_systems(sim, follow_mode_owner_rooms.in_set(ModeOwnerStep::Placed));
        app.add_systems(
            sim,
            despawn_departed_mode_entities
                .in_set(Platformer2dSimulationPhaseMonolith::Progression),
        );
    }
}
