//! Shell-routed authority for the canonical live platformer session world.

use bevy::ecs::change_detection::DetectChanges as _;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_actors::avatar::StartingCharacter;
use ambition_actors::ldtk_world::LdtkRuntimeIndex;
use ambition_actors::rooms::{ActiveRoomMetadata, RoomMusicRequest, RoomSet};
use ambition_encounter::EncounterMusicRequest;
use ambition_engine_core::RoomGeometry;
use ambition_game_shell::{
    ActiveGameplaySession, GameplaySessionSet, GameplaySessionWorldRoot, ShellActivationId,
};
use ambition_platformer_primitives::lifecycle::SessionScopeSet;
use ambition_platformer_primitives::markers::ControlledSubject;
use ambition_platformer_primitives::schedule::{GameplaySimulationRoot, SimScheduleExt as _};
use ambition_runtime::PlatformerSessionWorld;

#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SessionWorldProjectionAuthority {
    pub owner: Option<Entity>,
    pub activation: Option<ShellActivationId>,
    pub complete: bool,
    pub synchronized_revision: Option<u64>,
}

impl SessionWorldProjectionAuthority {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn owns(&self, entity: Entity) -> bool {
        self.owner == Some(entity) && self.complete
    }
}

#[derive(SystemParam)]
pub struct ActivePlatformerSessionWorld<'w, 's> {
    active: Option<Res<'w, ActiveGameplaySession>>,
    worlds: Query<'w, 's, (&'static GameplaySessionWorldRoot, &'static PlatformerSessionWorld)>,
}

impl ActivePlatformerSessionWorld<'_, '_> {
    pub fn get(&self) -> Option<&PlatformerSessionWorld> {
        let instance = self.active.as_deref()?.0.as_ref()?;
        let entity = instance.world?;
        let (root, world) = self.worlds.get(entity).ok()?;
        (root.activation_id == instance.activation.activation_id
            && root.experience_id == instance.activation.experience_id
            && root.scope == instance.scope
            && root.audio == instance.audio
            && root.load == instance.load
            && root.prepared == instance.prepared
            && world.catalogs.audio_provider == instance.audio.provider_id)
            .then_some(world)
    }
}

#[derive(SystemParam)]
pub struct ActivePlatformerSessionWorldMut<'w, 's> {
    active: Option<Res<'w, ActiveGameplaySession>>,
    worlds: Query<
        'w,
        's,
        (
            &'static GameplaySessionWorldRoot,
            &'static mut PlatformerSessionWorld,
        ),
    >,
}

impl ActivePlatformerSessionWorldMut<'_, '_> {
    pub fn edit<R>(
        &mut self,
        edit: impl FnOnce(&mut PlatformerSessionWorld) -> R,
    ) -> Option<R> {
        let instance = self.active.as_deref()?.0.as_ref()?;
        let entity = instance.world?;
        let (root, mut world) = self.worlds.get_mut(entity).ok()?;
        if root.activation_id != instance.activation.activation_id
            || root.experience_id != instance.activation.experience_id
            || root.scope != instance.scope
            || root.audio != instance.audio
            || root.load != instance.load
            || root.prepared != instance.prepared
            || world.catalogs.audio_provider != instance.audio.provider_id
        {
            return None;
        }
        let result = edit(&mut world);
        world.revision = world.revision.wrapping_add(1);
        Some(result)
    }
}

#[derive(SystemParam)]
struct ProjectionWrite<'w> {
    geometry: Option<ResMut<'w, RoomGeometry>>,
    room_set: Option<ResMut<'w, RoomSet>>,
    active_room: Option<ResMut<'w, ActiveRoomMetadata>>,
    starting_character: Option<ResMut<'w, StartingCharacter>>,
    runtime_rooms: Option<ResMut<'w, LdtkRuntimeIndex>>,
    room_music: Option<ResMut<'w, RoomMusicRequest>>,
    encounter_music: Option<ResMut<'w, EncounterMusicRequest>>,
    controlled_subject: Option<ResMut<'w, ControlledSubject>>,
}

impl ProjectionWrite<'_> {
    fn publish(&mut self, world: &PlatformerSessionWorld) -> bool {
        let mut complete = true;
        macro_rules! publish {
            ($field:ident, $value:expr) => {
                if let Some(target) = self.$field.as_deref_mut() {
                    *target = $value;
                } else {
                    complete = false;
                }
            };
        }
        publish!(geometry, world.geometry.clone());
        publish!(room_set, world.room_set.clone());
        publish!(active_room, world.active_room.clone());
        publish!(starting_character, world.starting_character.clone());
        publish!(runtime_rooms, world.runtime_rooms.clone());
        publish!(room_music, world.requests.room_music.clone());
        publish!(encounter_music, world.requests.encounter_music.clone());
        complete
    }

    fn clear_frontend_state(&mut self) {
        if let Some(value) = self.room_music.as_deref_mut() {
            *value = Default::default();
        }
        if let Some(value) = self.encounter_music.as_deref_mut() {
            *value = Default::default();
        }
        if let Some(value) = self.controlled_subject.as_deref_mut() {
            value.0 = None;
        }
    }
}

#[derive(SystemParam)]
struct ProjectionRead<'w> {
    geometry: Option<Res<'w, RoomGeometry>>,
    room_set: Option<Res<'w, RoomSet>>,
    active_room: Option<Res<'w, ActiveRoomMetadata>>,
    starting_character: Option<Res<'w, StartingCharacter>>,
    runtime_rooms: Option<Res<'w, LdtkRuntimeIndex>>,
    room_music: Option<Res<'w, RoomMusicRequest>>,
    encounter_music: Option<Res<'w, EncounterMusicRequest>>,
}

impl ProjectionRead<'_> {
    fn complete(&self) -> bool {
        self.geometry.is_some()
            && self.room_set.is_some()
            && self.active_room.is_some()
            && self.starting_character.is_some()
            && self.runtime_rooms.is_some()
            && self.room_music.is_some()
            && self.encounter_music.is_some()
    }

    fn changed(&self) -> bool {
        self.geometry.as_ref().is_some_and(|value| value.is_changed())
            || self.room_set.as_ref().is_some_and(|value| value.is_changed())
            || self.active_room.as_ref().is_some_and(|value| value.is_changed())
            || self.starting_character.as_ref().is_some_and(|value| value.is_changed())
            || self.runtime_rooms.as_ref().is_some_and(|value| value.is_changed())
            || self.room_music.as_ref().is_some_and(|value| value.is_changed())
            || self.encounter_music.as_ref().is_some_and(|value| value.is_changed())
    }

    fn capture(&self, world: &mut PlatformerSessionWorld) {
        world.geometry = self.geometry.as_deref().unwrap().clone();
        world.room_set = self.room_set.as_deref().unwrap().clone();
        world.active_room = self.active_room.as_deref().unwrap().clone();
        world.starting_character = self.starting_character.as_deref().unwrap().clone();
        world.runtime_rooms = self.runtime_rooms.as_deref().unwrap().clone();
        world.requests.room_music = self.room_music.as_deref().unwrap().clone();
        world.requests.encounter_music = self.encounter_music.as_deref().unwrap().clone();
    }
}

#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SessionWorldProjectionSet {
    Publish,
    Capture,
}

pub struct PlatformerSessionWorldProjectionPlugin;

impl Plugin for PlatformerSessionWorldProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SessionWorldProjectionAuthority>();
        let sim = app.sim_schedule();
        if app.sim_is_fixed_tick() {
            app.configure_sets(
                sim,
                (
                    SessionWorldProjectionSet::Publish.before(GameplaySimulationRoot),
                    SessionWorldProjectionSet::Capture.after(GameplaySimulationRoot),
                ),
            )
            .add_systems(
                sim,
                (
                    reconcile_projection.in_set(SessionWorldProjectionSet::Publish),
                    capture_projection.in_set(SessionWorldProjectionSet::Capture),
                ),
            )
            .configure_sets(
                Update,
                SessionWorldProjectionSet::Publish
                    .after(GameplaySessionSet::Providers)
                    .before(SessionScopeSet::Presentation),
            )
            .add_systems(
                Update,
                reconcile_projection.in_set(SessionWorldProjectionSet::Publish),
            );
        } else {
            app.configure_sets(
                Update,
                (
                    SessionWorldProjectionSet::Publish
                        .after(GameplaySessionSet::Providers)
                        .before(GameplaySimulationRoot),
                    SessionWorldProjectionSet::Capture
                        .after(GameplaySimulationRoot)
                        .before(SessionScopeSet::Presentation),
                ),
            )
            .add_systems(
                Update,
                (
                    reconcile_projection.in_set(SessionWorldProjectionSet::Publish),
                    capture_projection.in_set(SessionWorldProjectionSet::Capture),
                ),
            );
        }
    }
}

fn reconcile_projection(
    active: Option<Res<ActiveGameplaySession>>,
    roots: Query<&GameplaySessionWorldRoot>,
    worlds: Query<&PlatformerSessionWorld>,
    mut projection: ProjectionWrite,
    mut authority: ResMut<SessionWorldProjectionAuthority>,
) {
    let Some(instance) = active.as_deref().and_then(|value| value.0.as_ref()) else {
        projection.clear_frontend_state();
        authority.clear();
        return;
    };
    let Some(entity) = instance.world else {
        projection.clear_frontend_state();
        authority.clear();
        return;
    };
    let Ok(root) = roots.get(entity) else {
        projection.clear_frontend_state();
        authority.clear();
        return;
    };
    let Ok(world) = worlds.get(entity) else {
        projection.clear_frontend_state();
        authority.clear();
        return;
    };
    if root.activation_id != instance.activation.activation_id
        || root.experience_id != instance.activation.experience_id
        || root.scope != instance.scope
        || root.audio != instance.audio
        || root.load != instance.load
        || root.prepared != instance.prepared
        || world.catalogs.audio_provider != instance.audio.provider_id
    {
        projection.clear_frontend_state();
        authority.clear();
        return;
    }

    if authority.owner != Some(entity)
        || !authority.complete
        || authority.synchronized_revision != Some(world.revision)
    {
        let complete = projection.publish(world);
        *authority = SessionWorldProjectionAuthority {
            owner: Some(entity),
            activation: Some(instance.activation.activation_id),
            complete,
            synchronized_revision: complete.then_some(world.revision),
        };
    }
}

fn capture_projection(
    active: Option<Res<ActiveGameplaySession>>,
    mut authority: ResMut<SessionWorldProjectionAuthority>,
    projection: ProjectionRead,
    mut worlds: Query<&mut PlatformerSessionWorld>,
) {
    let Some(entity) = active.as_deref().and_then(ActiveGameplaySession::active_world_entity) else {
        return;
    };
    if !authority.owns(entity) || !projection.complete() || !projection.changed() {
        return;
    }
    let Ok(mut world) = worlds.get_mut(entity) else {
        return;
    };
    projection.capture(&mut world);
    world.revision = world.revision.wrapping_add(1);
    authority.synchronized_revision = Some(world.revision);
}
