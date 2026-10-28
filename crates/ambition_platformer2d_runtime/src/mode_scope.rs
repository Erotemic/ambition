//! Scoped game-mode runtime for hosted demos/rulesets.
//!
//! Rules may coexist in one app and gate their systems on the active room's mode
//! tag rather than owning a global state. Standalone compositions can install the
//! same rules ungated. Mode-owned entities carry `ModeScopedEntity` and are swept
//! when routing leaves the mode.

use bevy::prelude::*;

use ambition_combat::scoped_rules::{ActiveRoom, RulesScope};
use ambition_platformer2d_shared_tangle::lifecycle::{
    despawn_scoped_entity, ModeScopedEntity, ModeVisit,
};
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhase, SimScheduleExt as _,
};
use ambition_combat::governing_rules::{CurrentRoom, GoverningRules};
use ambition_platformer2d_shared_tangle::capability::{
    Capability, CapabilityPresentationSet, CapabilitySet, UsedCapabilities,
};

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

/// The systems that serve the rooms one [`RulesScope`] governs: a game's rules,
/// for the rooms of that game. The set runs only while an active room is in
/// its scope ([`in_rules_scope`]), so a game the hub has installed but is not
/// playing does no work. Add systems with
/// [`ScopedSystemsAppExt::add_scoped_systems`], which gives the set its
/// condition in each schedule.
///
/// The scope follows the room, not the frontend route: a session can visit
/// another game's room, and a session being prepared or retired is served by
/// systems outside the set. Inside the simulation the condition reads room
/// state that rollback restores, so a resimulated tick runs the same systems.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RulesScopeSet(pub RulesScope);

/// The (schedule, scope) pairs whose [`RulesScopeSet`] has its condition.
#[derive(Resource, Default)]
struct ConfiguredScopeSets(std::collections::HashSet<(bevy::ecs::schedule::InternedScheduleLabel, RulesScope)>);

pub trait ScopedSystemsAppExt {
    /// Add `systems` to `schedule` in the [`RulesScopeSet`] of `scope`: they
    /// run only while an active room is in `scope`.
    fn add_scoped_systems<M>(
        &mut self,
        schedule: impl bevy::ecs::schedule::ScheduleLabel,
        scope: RulesScope,
        systems: impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
    ) -> &mut Self;
}

impl ScopedSystemsAppExt for App {
    fn add_scoped_systems<M>(
        &mut self,
        schedule: impl bevy::ecs::schedule::ScheduleLabel,
        scope: RulesScope,
        systems: impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
    ) -> &mut Self {
        let schedule = schedule.intern();
        let first = self
            .world_mut()
            .get_resource_or_insert_with(ConfiguredScopeSets::default)
            .0
            .insert((schedule, scope));
        if first {
            self.configure_sets(schedule, RulesScopeSet(scope).run_if(in_rules_scope(scope)));
        }
        self.add_systems(schedule, systems.in_set(RulesScopeSet(scope)))
    }
}

/// Run condition: a live room uses `capability`: its game's declared
/// [`UsedCapabilities`] name it, or its game declared none. With no session,
/// only a whole-process declaration governs, so a capability runs unless the
/// binary's one game left it out.
pub fn room_uses(
    capability: Capability,
) -> impl FnMut(GoverningRules<UsedCapabilities>) -> bool + Clone {
    move |rules: GoverningRules<UsedCapabilities>| rules.any_live_room_uses(capability)
}

/// Run condition: [`room_uses`], or it held on the frame before. The one frame
/// more is for a [`CapabilityPresentationSet`]: its reconcile pass, with no
/// subjects left, takes down what it drew.
pub fn room_uses_or_did(
    capability: Capability,
) -> impl FnMut(GoverningRules<UsedCapabilities>, Local<bool>) -> bool + Clone {
    move |rules: GoverningRules<UsedCapabilities>, mut used_before: Local<bool>| {
        let used = rules.any_live_room_uses(capability);
        let run = used || *used_before;
        *used_before = used;
        run
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
pub fn in_base_mode(room: CurrentRoom) -> bool {
    room.in_scope(RulesScope::UntaggedRooms)
}

/// The rooms each mode's game governs, as the game declared them with
/// [`install_mode_owner`]. A mode with no declaration governs the rooms tagged
/// with its name.
///
/// This is the one statement of where a mode lives: the owner is born where its
/// scope governs, and [`despawn_departed_mode_entities`] retires the mode's
/// entities where it does not. Authored constants, written when a plugin is
/// built, so a rewind has nothing to restore.
#[derive(Resource, Debug, Default)]
pub struct ModeScopes(Vec<(&'static str, RulesScope)>);

impl ModeScopes {
    /// Record that `mode`'s game governs the rooms `scope` names.
    ///
    /// Panics on a second, different scope for one mode: a mode whose entities
    /// live in two sets of rooms has no single lifetime.
    pub fn declare(&mut self, mode: &'static str, scope: RulesScope) {
        match self.0.iter().find(|(declared, _)| *declared == mode) {
            Some((_, declared)) => assert_eq!(
                *declared, scope,
                "mode `{mode}` was declared in two scopes"
            ),
            None => self.0.push((mode, scope)),
        }
    }

    /// The rooms `mode`'s game governs.
    pub fn scope_of(&self, mode: &str) -> Option<RulesScope> {
        self.0
            .iter()
            .find(|(declared, _)| *declared == mode)
            .map(|(_, scope)| *scope)
    }
}

/// Whether `mode` governs a room that a rule scope sees as `room`: the mode's
/// declared scope ([`ModeScopes`]), else the rooms tagged with its name.
fn mode_governs(scopes: Option<&ModeScopes>, mode: &str, room: ActiveRoom<'_>) -> bool {
    match scopes.and_then(|scopes| scopes.scope_of(mode)) {
        Some(scope) => scope.governs(room),
        None => room == ActiveRoom::Mode(mode),
    }
}

/// Each live room, in instance order, as a rule scope sees it, with its id.
fn live_rule_rooms<'a>(
    rooms: &'a ambition_platformer2d_world::rooms::LiveRoomSpecs<'_, '_>,
) -> Vec<(&'a str, ActiveRoom<'a>)> {
    let mut live: Vec<_> = rooms.live_rooms().collect();
    live.sort_by_key(|(instance, _)| *instance);
    live.into_iter()
        .map(|(_, definition)| {
            let spec = rooms.rooms().spec(definition);
            (spec.id.as_str(), ActiveRoom::live(spec.metadata.mode.as_deref()))
        })
        .collect()
}

/// Despawn every [`ModeScopedEntity`] whose mode governs no live room.
///
/// A room change inside one mode leaves that mode's entities alone: the sweep
/// compares scopes, not rooms, which is what makes a mode a lifetime distinct
/// from a room. With two rooms live (OW1), a mode lives while any live room is
/// in its scope, the question `CurrentRoom::in_scope` asks for the mode's
/// systems.
///
/// ⛔ IT RUNS ON EVERY TICK, WITH NO GATE. It ran only when the live rooms
/// changed: `RoomSet` or a root's `LiveRoomDefinition` marked changed, or a
/// root removed. A rollback load writes the room set and every root again, so
/// under a resimulating host the gate opened on each load frame and on no
/// other. Measured (sync test, 4/10): the gate opened on 60 of 300 runs, and
/// never on a host with no rollback. So a mode-scoped entity whose mode
/// governs no live room was swept on a peer that loaded a snapshot and kept on
/// a peer that did not. The sweep is a pure function of the live rooms and
/// the mode-scoped entities, and there are few of them, so it runs on every
/// tick on every host.
pub fn despawn_departed_mode_entities(
    mut commands: Commands,
    rooms: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    scopes: Option<Res<ModeScopes>>,
    scoped: Query<(Entity, &ModeScopedEntity)>,
) {
    let Some(rooms) = rooms else { return };
    if scoped.is_empty() {
        return;
    }
    let live = live_rule_rooms(&rooms);
    if live.is_empty() {
        return;
    }
    for (entity, mode) in scoped.iter() {
        let governs = live
            .iter()
            .any(|(_, room)| mode_governs(scopes.as_deref(), &mode.0, *room));
        if !governs {
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
    rule: ambition_combat::governing_rules::GoverningRules<T>,
    out: Option<ResMut<R>>,
) where
    T: Clone + std::fmt::Debug + Send + Sync + 'static,
    R: Resource
        + bevy::ecs::component::Component<Mutability = bevy::ecs::component::Mutable>
        + PartialEq
        + From<Option<T>>,
{
    let Some(mut out) = out else { return };
    let next = R::from(rule.get().cloned());
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
///
/// An owner is in a live room its mode governs. With two rooms live (OW1), it
/// stays in its room while that room is live and governed, else it goes to the
/// first governed room in instance order. While no live room is governed, the
/// visit does not change: the mode sweep retires the owner. One owner per mode
/// follows one room; an owner per (mode, live room) is a later OW cut.
pub fn follow_mode_owner_rooms(
    rooms: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    scopes: Option<Res<ModeScopes>>,
    mut owners: Query<(&mut ModeVisit, Option<&ModeScopedEntity>)>,
) {
    let Some(rooms) = rooms else {
        return;
    };
    let live = live_rule_rooms(&rooms);
    for (mut visit, mode) in &mut owners {
        let mut governed = live.iter().filter(|(_, room)| {
            mode.is_none_or(|mode| mode_governs(scopes.as_deref(), &mode.0, *room))
        });
        let first = governed.clone().next().map(|(id, _)| *id);
        let Some(active) = governed
            .find(|(id, _)| visit.room() == Some(*id))
            .map(|(id, _)| *id)
            .or(first)
        else {
            continue;
        };
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
        ModeOwnersSpawned.in_set(Platformer2dSimulationPhase::GameplayEffects),
    );
    app.configure_sets(
        sim,
        (ModeOwnerStep::Born, ModeOwnerStep::Placed)
            .chain()
            .in_set(ModeOwnersSpawned),
    );
}

/// Bring `mode`'s owner into being on the first tick the rooms `scope` governs
/// are live. `scope` is also the owner's lifetime: it is recorded in
/// [`ModeScopes`], and the mode sweep retires the owner where `scope` does not
/// govern.
///
/// A hosted game keeps its level or act state on one entity per mode. The
/// owner is spawned with `spawn_mode_owner`, so the mode sweep retires it when
/// its scope stops governing, the session retires it on a relaunch, and its `SimId` names
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
    app.init_resource::<ModeScopes>();
    app.world_mut().resource_mut::<ModeScopes>().declare(mode, scope);
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
        use bevy::ecs::schedule::ScheduleLabel as _;
        // Each capability's systems run where the active room uses it. A set
        // with no members in a schedule costs nothing there.
        for schedule in [sim, PreUpdate.intern(), Update.intern(), PostUpdate.intern()] {
            for capability in Capability::ALL {
                app.configure_sets(
                    schedule,
                    CapabilitySet(capability).run_if(room_uses(capability)),
                );
            }
        }
        for schedule in [Update.intern(), PostUpdate.intern()] {
            for capability in Capability::ALL {
                app.configure_sets(
                    schedule,
                    CapabilityPresentationSet(capability).run_if(room_uses_or_did(capability)),
                );
            }
        }
        app.add_systems(sim, follow_mode_owner_rooms.in_set(ModeOwnerStep::Placed));
        app.add_systems(
            sim,
            despawn_departed_mode_entities
                .in_set(Platformer2dSimulationPhase::Progression),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat::governing_rules::test_fixtures::two_game_session;
    use ambition_combat::scoped_rules::{DeclaredRules, RulesScope};

    #[derive(Resource, Default)]
    struct Runs(u32);

    fn uses(app: &mut App, portals: bool) {
        let used = if portals { UsedCapabilities::ALL } else { UsedCapabilities::ALL.without(Capability::Portals) };
        let mut rules = DeclaredRules::<UsedCapabilities>::default();
        rules.declare(RulesScope::UntaggedRooms, used);
        app.insert_resource(rules);
    }

    /// A capability's presentation runs while a live room uses it, once more
    /// after none does (its pass takes the drawn things down), and then sleeps.
    #[test]
    fn a_capabilitys_presentation_runs_one_frame_after_its_rooms_stop_using_it() {
        let mut app = App::new();
        two_game_session(&mut app, false);
        uses(&mut app, true);
        app.init_resource::<Runs>();
        app.configure_sets(
            Update,
            CapabilityPresentationSet(Capability::Portals).run_if(room_uses_or_did(Capability::Portals)),
        );
        app.add_systems(
            Update,
            (|mut runs: ResMut<Runs>| runs.0 += 1).in_set(CapabilityPresentationSet(Capability::Portals)),
        );
        let step = |app: &mut App| {
            let before = app.world().resource::<Runs>().0;
            app.update();
            app.world().resource::<Runs>().0 - before
        };
        assert_eq!(step(&mut app), 1, "it did not run where the room uses portals");
        uses(&mut app, false);
        assert_eq!(step(&mut app), 1, "it did not run the frame after the room stopped using portals");
        assert_eq!(step(&mut app), 0, "it ran two frames after the room stopped using portals");
        assert_eq!(step(&mut app), 0, "it ran while no room uses portals");
        uses(&mut app, true);
        assert_eq!(step(&mut app), 1, "it did not wake when the room used portals again");
    }
}
