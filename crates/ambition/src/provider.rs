//! Compact Bevy-native authoring surface for platformer experience providers.

use std::{collections::BTreeMap, marker::PhantomData};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::game_shell::{
    standard_platformer_preparation_plan, ActiveGameplaySession, ActiveShellExperience,
    ExperienceRegistration, GameplaySessionAppExt, PreparedSessionIdentity,
    PreparedSessionRegistry, ProviderLoadTransaction, ShellCompletionPolicy, ShellRouteSpec,
    PREPARE_AUDIO_WORK_ID, PREPARE_CATALOGS_WORK_ID,
};
use crate::platformer::lifecycle::{SessionScopeId, SessionSpawnScope};
use crate::runtime::PlatformerSessionWorld;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredCatalogFragments {
    pub starting_character: String,
    pub audio_provider: String,
}

impl AuthoredCatalogFragments {
    pub fn new(
        starting_character: impl Into<String>,
        audio_provider: impl Into<String>,
    ) -> Self {
        Self {
            starting_character: starting_character.into(),
            audio_provider: audio_provider.into(),
        }
    }

    pub fn validate(
        &self,
        character_catalog: &crate::characters::actor::character_catalog::CharacterCatalog,
        audio_catalogs: &crate::audio::catalog::AudioCatalogRegistry,
    ) -> Option<(&'static str, crate::load::LoadFailure)> {
        if character_catalog.get(self.starting_character.as_str()).is_none() {
            return Some((
                PREPARE_CATALOGS_WORK_ID,
                crate::load::LoadFailure::new(
                    "Starting character data is unavailable",
                    format!(
                        "character catalog has no '{}' row",
                        self.starting_character
                    ),
                )
                .retryable(true),
            ));
        }
        if !audio_catalogs.has_provider(self.audio_provider.as_str()) {
            return Some((
                PREPARE_AUDIO_WORK_ID,
                crate::load::LoadFailure::new(
                    "Provider audio intent is unavailable",
                    format!(
                        "provider '{}' registered no explicit audio fragment",
                        self.audio_provider
                    ),
                )
                .retryable(true),
            ));
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct PlatformerExperienceAuthoring {
    pub experience_id: String,
    pub route_id: String,
    pub label: String,
    pub description: String,
    pub preparation_label: String,
    pub catalogs: AuthoredCatalogFragments,
}

impl PlatformerExperienceAuthoring {
    pub fn new(
        experience_id: impl Into<String>,
        route_id: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
        preparation_label: impl Into<String>,
        catalogs: AuthoredCatalogFragments,
    ) -> Self {
        Self {
            experience_id: experience_id.into(),
            route_id: route_id.into(),
            label: label.into(),
            description: description.into(),
            preparation_label: preparation_label.into(),
            catalogs,
        }
    }

    pub fn register(&self, app: &mut App) {
        app.register_gameplay_experience(
            ExperienceRegistration::new(
                self.experience_id.clone(),
                self.label.clone(),
                self.route_id.clone(),
            )
            .with_description(self.description.clone()),
            ShellRouteSpec::new(self.route_id.clone(), self.experience_id.clone())
                .preparing_with(standard_platformer_preparation_plan(
                    self.preparation_label.clone(),
                ))
                .on_complete(ShellCompletionPolicy::ReturnHome),
        );
    }
}

#[derive(Clone)]
struct PreparedPlatformerRecord {
    transaction: ProviderLoadTransaction,
    world: PlatformerSessionWorld,
}

#[derive(Resource)]
pub struct PreparedPlatformerSessions<M: Send + Sync + 'static> {
    records: BTreeMap<crate::load::LoadId, PreparedPlatformerRecord>,
    marker: PhantomData<fn() -> M>,
}

impl<M: Send + Sync + 'static> Default for PreparedPlatformerSessions<M> {
    fn default() -> Self {
        Self {
            records: BTreeMap::new(),
            marker: PhantomData,
        }
    }
}

impl<M: Send + Sync + 'static> PreparedPlatformerSessions<M> {
    pub fn publish(
        &mut self,
        transaction: &ProviderLoadTransaction,
        world: PlatformerSessionWorld,
        registry: &mut PreparedSessionRegistry,
    ) -> Option<PreparedSessionIdentity> {
        let identity = registry.publish(transaction)?;
        let old = self.records.insert(
            transaction.barrier.load_id.clone(),
            PreparedPlatformerRecord {
                transaction: transaction.clone(),
                world,
            },
        );
        assert!(old.is_none(), "fresh prepared transaction was reused");
        Some(identity)
    }

    pub fn take(
        &mut self,
        identity: &PreparedSessionIdentity,
        registry: &mut PreparedSessionRegistry,
    ) -> Option<PlatformerSessionWorld> {
        let load_id = &identity.transaction.barrier.load_id;
        let record = self.records.remove(load_id)?;
        if record.transaction != identity.transaction || !registry.retire_prepared(identity) {
            self.records.insert(load_id.clone(), record);
            return None;
        }
        Some(record.world)
    }

    pub fn retain_requested(&mut self, registry: &PreparedSessionRegistry) {
        self.records
            .retain(|load_id, _| registry.contains_load(load_id));
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

pub fn cleanup_prepared_platformer_sessions<M: Send + Sync + 'static>(
    registry: Res<PreparedSessionRegistry>,
    mut sessions: ResMut<PreparedPlatformerSessions<M>>,
) {
    sessions.retain_requested(&registry);
}

#[derive(SystemParam)]
pub struct PlatformerSessionBuilder<'w, 's> {
    commands: Commands<'w, 's>,
    editable_abilities: Res<'w, crate::dev_tools::dev_tools::EditableAbilitySet>,
    editable_tuning: Res<'w, crate::dev_tools::dev_tools::EditableMovementTuning>,
    asset_server: Res<'w, AssetServer>,
    character_catalog:
        Res<'w, crate::characters::actor::character_catalog::CharacterCatalog>,
    character_roster: Res<'w, crate::actors::features::CharacterRoster>,
    boss_catalog: Res<'w, crate::actors::boss_encounter::BossCatalog>,
    sandbox_data_asset: Option<Res<'w, crate::actors::session::data::SandboxDataAsset>>,
    sandbox_asset_collection:
        Option<Res<'w, crate::actors::assets::loading::SandboxAssetCollection>>,
    geometry: ResMut<'w, crate::engine_core::RoomGeometry>,
    room_set: ResMut<'w, crate::actors::rooms::RoomSet>,
    metadata: ResMut<'w, crate::actors::rooms::ActiveRoomMetadata>,
    starting_character: ResMut<'w, crate::actors::avatar::StartingCharacter>,
    active_session: ResMut<'w, ActiveGameplaySession>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionBuildResult {
    pub player: Entity,
    pub world: Entity,
}

impl PlatformerSessionBuilder<'_, '_> {
    pub fn build(
        &mut self,
        activation: &ActiveShellExperience,
        scope: SessionScopeId,
        live_world: PlatformerSessionWorld,
        default_character_id: &str,
    ) -> SessionBuildResult {
        let player = crate::actors::session::setup::simulation_world(
            &mut self.commands,
            SessionSpawnScope::scoped(scope),
            crate::actors::session::setup::SimulationSetup {
                world: &live_world.geometry,
                room_set: &live_world.room_set,
                ldtk_index: &live_world.runtime_rooms,
                editable_abilities: &self.editable_abilities,
                editable_tuning: &self.editable_tuning,
                starting_character: &live_world.starting_character,
                character_catalog: &self.character_catalog,
                character_roster: &self.character_roster,
                boss_catalog: &self.boss_catalog,
                default_character_id,
                sandbox_data_asset: self.sandbox_data_asset.as_deref(),
                sandbox_asset_collection: self.sandbox_asset_collection.as_deref(),
                asset_server: &self.asset_server,
            },
        );

        let world = self
            .active_session
            .spawn_world_for(&mut self.commands, activation, scope, live_world.clone())
            .expect("provider activation still owns the session it is constructing");

        *self.geometry = live_world.geometry;
        *self.room_set = live_world.room_set;
        *self.metadata = live_world.active_room;
        *self.starting_character = live_world.starting_character;

        SessionBuildResult { player, world }
    }
}
