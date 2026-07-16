//! The shared provider lifecycle: preparation, prepared-session ownership,
//! and activation into the live session world.
//!
//! Every experience registered through
//! [`PlatformerExperienceAuthoring::install`](crate::authoring::PlatformerExperienceAuthoring::install)
//! shares these systems; the provider contributes only its session-world
//! source. The answers to the lifecycle questions live here:
//!
//! - **What does a provider prepare?** A [`PlatformerSessionWorld`], validated
//!   against its [`AuthoredCatalogFragments`](crate::authoring::AuthoredCatalogFragments)
//!   into a [`PlatformerPreparationReport`].
//! - **What identity proves activation matches preparation?** The
//!   [`PreparedSessionIdentity`] published through the shell's
//!   [`PreparedSessionRegistry`].
//! - **Who owns the prepared value?** [`PreparedPlatformerSessions`], keyed by
//!   the load transaction.
//! - **When is live session state created?** Preparation owns the validated
//!   world value; activation consumes it by exact identity and creates the live
//!   session root and scoped entities through [`PlatformerSessionBuilder::build`].

use std::collections::BTreeMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_game_shell::{
    ActiveGameplaySession, ActiveShellExperience, GameplayInputOwner, GameplaySessionEvent,
    GameplaySessionSet, PreparedSessionIdentity, PreparedSessionRegistry, ProviderLoadTransaction,
    ShellEvent, PREPARE_ADAPTIVE_WORK_ID, PREPARE_CATALOGS_WORK_ID, PREPARE_DEFAULTS_WORK_ID,
    PREPARE_MUSIC_WORK_ID, PREPARE_PACKED_SFX_WORK_ID, PREPARE_SESSION_WORK_ID,
    PREPARE_SFX_WORK_ID, PREPARE_SPRITES_WORK_ID, PREPARE_WORLD_WORK_ID,
};
use ambition_load::AmbitionLoadSet;
use ambition_platformer_primitives::lifecycle::{SessionScopeId, SessionSpawnScope};
use ambition_runtime::PlatformerSessionWorld;

use crate::authoring::PlatformerAuthoredCatalogRegistry;

/// Every provider's preparation system runs in this set (inside
/// `AmbitionLoadSet::Contributors`); the shared prepared-session cleanup runs
/// after it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlatformerPreparationSet;

/// What preparation proved about the session it published.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformerPreparationReport {
    pub starting_character: String,
    pub sprite_manifest: String,
    pub sprite_asset: String,
    /// Whether the host's optional shared presentation cache already has a bound sheet.
    /// A provider may still activate with authored paths and use the renderer fallback while
    /// its own presentation assets stream or are supplied by a standalone host.
    pub sprite_bound: bool,
    pub music_ready: bool,
    pub procedural_sfx_ready: bool,
    pub adaptive_cues_ready: bool,
    pub packed_sfx_streamable: bool,
    pub deliberate_silence: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingPackedSfxReadiness {
    pub(crate) provider_id: String,
}

#[derive(Resource, Default)]
pub(crate) struct PlatformerStreamingReadiness {
    pub(crate) pending_packed_sfx: BTreeMap<ambition_load::LoadId, PendingPackedSfxReadiness>,
}

/// The once-per-app runtime half of the provider protocol: the shared
/// prepared-session store, its cleanup, packed-SFX streaming readiness, and
/// the one activation system every installed experience shares.
pub(crate) struct PlatformerProviderRuntimePlugin;

impl Plugin for PlatformerProviderRuntimePlugin {
    fn build(&self, app: &mut App) {
        // The provider owns the session lifecycle: it constructs the live session
        // on activation and must retire its resource mirrors on teardown. This
        // plugin is composed only by shell hosts, which also install
        // `SessionScopePlugin`, so `SessionScopeSet::Cleanup` and
        // `SessionScopeRetired` are available.
        app.add_plugins(ambition_actors::session::SessionTeardownPlugin);
        app.init_resource::<PlatformerStreamingReadiness>()
            .init_resource::<PreparedPlatformerSessions>()
            .configure_sets(
                Update,
                PlatformerPreparationSet.in_set(AmbitionLoadSet::Contributors),
            )
            .add_systems(
                Update,
                (
                    update_streamable_packed_sfx.in_set(AmbitionLoadSet::Contributors),
                    cleanup_prepared_platformer_sessions
                        .after(PlatformerPreparationSet)
                        .in_set(AmbitionLoadSet::Contributors),
                    activate_prepared_platformer_sessions.in_set(GameplaySessionSet::Providers),
                ),
            );
    }
}

fn update_streamable_packed_sfx(
    loads: Res<ambition_load::LoadCoordinator>,
    banks: Option<Res<ambition_audio::catalog::SfxBankRegistry>>,
    mut readiness: ResMut<PlatformerStreamingReadiness>,
    mut commands: MessageWriter<ambition_load::LoadCommand>,
) {
    readiness
        .pending_packed_sfx
        .retain(|load_id, _| loads.contains(load_id));
    let Some(banks) = banks else {
        return;
    };
    let ready = readiness
        .pending_packed_sfx
        .iter()
        .filter_map(|(load_id, pending)| {
            (!banks.ids_for(pending.provider_id.as_str()).is_empty()).then_some(load_id.clone())
        })
        .collect::<Vec<_>>();
    for load_id in ready {
        commands.write(ambition_load::LoadCommand::SetWorkState {
            load_id: load_id.clone(),
            work_id: ambition_load::LoadWorkId::new(PREPARE_PACKED_SFX_WORK_ID),
            state: ambition_load::LoadWorkState::Complete,
        });
        readiness.pending_packed_sfx.remove(&load_id);
    }
}

/// True when the shell requested preparation for `experience_id` this frame —
/// the run gate that keeps a provider's session-world source from building
/// worlds on frames nobody asked for.
pub(crate) fn preparation_requested(
    experience_id: String,
) -> impl FnMut(MessageReader<ShellEvent>) -> bool {
    move |mut events: MessageReader<ShellEvent>| {
        events.read().any(|event| {
            matches!(
                event,
                ShellEvent::PreparationRequested(transaction)
                    if transaction.experience_id.as_str() == experience_id
            )
        })
    }
}

/// The shared preparation system body: piped after a provider's session-world
/// source (tagged with its experience id), it gives every matching transaction
/// an owned copy of that authored world value.
pub(crate) fn prepare_requested_sessions(
    In((experience_id, world)): In<(String, PlatformerSessionWorld)>,
    mut events: MessageReader<ShellEvent>,
    mut preparation: PlatformerPreparation,
) {
    for event in events.read() {
        let ShellEvent::PreparationRequested(transaction) = event else {
            continue;
        };
        if transaction.experience_id.as_str() != experience_id {
            continue;
        }
        preparation.prepare(transaction, world.clone());
    }
}

/// Catalog validation + prepared-session publication for one transaction.
#[derive(SystemParam)]
pub(crate) struct PlatformerPreparation<'w> {
    authored_catalogs: Res<'w, PlatformerAuthoredCatalogRegistry>,
    character_catalog: Res<'w, ambition_characters::actor::character_catalog::CharacterCatalog>,
    audio_catalogs: Res<'w, ambition_audio::catalog::AudioCatalogRegistry>,
    #[cfg(feature = "audio")]
    adaptive_catalogs: Option<Res<'w, ambition_audio::music::AdaptiveMusicCatalogRegistry>>,
    sfx_banks: Option<Res<'w, ambition_audio::catalog::SfxBankRegistry>>,
    game_assets: Option<Res<'w, ambition_sprite_sheet::game_assets::GameAssets>>,
    registry: ResMut<'w, PreparedSessionRegistry>,
    sessions: ResMut<'w, PreparedPlatformerSessions>,
    streaming: ResMut<'w, PlatformerStreamingReadiness>,
    commands: MessageWriter<'w, ambition_load::LoadCommand>,
}

impl PlatformerPreparation<'_> {
    pub(crate) fn prepare(
        &mut self,
        transaction: &ProviderLoadTransaction,
        world: PlatformerSessionWorld,
    ) -> Option<PreparedSessionIdentity> {
        let Some(authored) = self
            .authored_catalogs
            .get(transaction.experience_id.as_str())
            .cloned()
        else {
            self.fail(
                transaction,
                PREPARE_CATALOGS_WORK_ID,
                ambition_load::LoadFailure::new(
                    "Provider catalogs are unavailable",
                    format!(
                        "experience '{}' has no registered platformer authoring fragments",
                        transaction.experience_id.as_str(),
                    ),
                )
                .retryable(false),
            );
            return None;
        };
        for work_id in [
            PREPARE_CATALOGS_WORK_ID,
            PREPARE_WORLD_WORK_ID,
            PREPARE_SPRITES_WORK_ID,
            PREPARE_MUSIC_WORK_ID,
            PREPARE_SFX_WORK_ID,
            PREPARE_ADAPTIVE_WORK_ID,
            PREPARE_DEFAULTS_WORK_ID,
            PREPARE_SESSION_WORK_ID,
        ] {
            self.set_state(
                transaction,
                work_id,
                ambition_load::LoadWorkState::Running { progress: None },
            );
        }

        if let Some((work_id, failure)) =
            authored.validate(&self.character_catalog, &self.audio_catalogs)
        {
            self.fail(transaction, work_id, failure);
            return None;
        }
        self.complete(transaction, PREPARE_CATALOGS_WORK_ID);

        if world.active_room_id().trim().is_empty()
            || world.catalogs.world_provider.trim().is_empty()
            || world.catalogs.character_provider.trim().is_empty()
            || world.catalogs.audio_provider.trim().is_empty()
        {
            self.fail(
                transaction,
                PREPARE_WORLD_WORK_ID,
                ambition_load::LoadFailure::new(
                    "World data is incomplete",
                    "prepared platformer world has an empty active room or provider identity",
                )
                .retryable(true),
            );
            return None;
        }
        self.complete(transaction, PREPARE_WORLD_WORK_ID);

        let (sprite_asset, sprite_manifest) = {
            let entry = self
                .character_catalog
                .get(authored.starting_character.as_str())
                .expect("catalog validation already proved the starting character exists");
            (entry.spritesheet.clone(), entry.manifest.clone())
        };
        if sprite_asset.trim().is_empty() || sprite_manifest.trim().is_empty() {
            self.fail(
                transaction,
                PREPARE_SPRITES_WORK_ID,
                ambition_load::LoadFailure::new(
                    "Character presentation is incomplete",
                    format!(
                        "character '{}' has no spritesheet or manifest path",
                        authored.starting_character
                    ),
                )
                .retryable(true),
            );
            return None;
        }
        // `GameAssets` is a host-owned optional presentation cache, not provider
        // authority. Standalone providers may intentionally use the renderer's
        // fallback while their own art streams, and a shared host must not reject
        // a provider merely because Ambition's resident cache does not contain
        // that provider's private character. The required preparation work is
        // resolving and validating the provider-authored paths above; cache
        // binding is recorded as evidence, never used as an activation gate.
        let sprite_bound = self.game_assets.as_deref().is_some_and(|assets| {
            assets
                .characters
                .asset_for_character_id(authored.starting_character.as_str())
                .is_some()
        });
        self.complete(transaction, PREPARE_SPRITES_WORK_ID);

        let music_ready = self
            .audio_catalogs
            .music_for(authored.audio_provider.as_str())
            .is_some();
        let procedural_sfx_ready = self
            .audio_catalogs
            .sfx_for(authored.audio_provider.as_str())
            .is_some();
        if authored.expects_music && !music_ready {
            self.fail(
                transaction,
                PREPARE_MUSIC_WORK_ID,
                ambition_load::LoadFailure::new(
                    "Provider music is not ready",
                    format!(
                        "provider '{}' requires a music fragment but registered none",
                        authored.audio_provider
                    ),
                )
                .retryable(true),
            );
            return None;
        }
        self.complete(transaction, PREPARE_MUSIC_WORK_ID);
        if authored.expects_procedural_sfx && !procedural_sfx_ready {
            self.fail(
                transaction,
                PREPARE_SFX_WORK_ID,
                ambition_load::LoadFailure::new(
                    "Provider procedural SFX are not ready",
                    format!(
                        "provider '{}' requires procedural SFX but registered none",
                        authored.audio_provider
                    ),
                )
                .retryable(true),
            );
            return None;
        }
        self.complete(transaction, PREPARE_SFX_WORK_ID);
        let deliberate_silence = !authored.expects_music
            && !authored.expects_procedural_sfx
            && !music_ready
            && !procedural_sfx_ready;

        #[cfg(feature = "audio")]
        let adaptive_cues_ready = self
            .adaptive_catalogs
            .as_deref()
            .and_then(|catalogs| catalogs.catalog_for(authored.audio_provider.as_str()))
            .is_some();
        #[cfg(not(feature = "audio"))]
        let adaptive_cues_ready = false;
        #[cfg(feature = "audio")]
        if authored.expects_adaptive_cues && !adaptive_cues_ready {
            self.fail(
                transaction,
                PREPARE_ADAPTIVE_WORK_ID,
                ambition_load::LoadFailure::new(
                    "Adaptive music is not ready",
                    format!(
                        "provider '{}' requires adaptive cues but registered none",
                        authored.audio_provider
                    ),
                )
                .retryable(true),
            );
            return None;
        }
        self.complete(transaction, PREPARE_ADAPTIVE_WORK_ID);

        let effective_character = world
            .starting_character
            .effective_id(authored.starting_character.as_str());
        if effective_character != authored.starting_character.as_str()
            || world.catalogs.audio_provider.as_str() != authored.audio_provider.as_str()
        {
            self.fail(
                transaction,
                PREPARE_DEFAULTS_WORK_ID,
                ambition_load::LoadFailure::new(
                    "Provider defaults do not match the prepared world",
                    format!(
                        "expected character '{}' and audio provider '{}', got '{}' and '{}'",
                        authored.starting_character,
                        authored.audio_provider,
                        effective_character,
                        world.catalogs.audio_provider,
                    ),
                )
                .retryable(false),
            );
            return None;
        }
        self.complete(transaction, PREPARE_DEFAULTS_WORK_ID);

        let packed_ids = self
            .sfx_banks
            .as_deref()
            .map(|banks| banks.ids_for(authored.audio_provider.as_str()))
            .unwrap_or_default();
        let packed_sfx_streamable = authored.expects_packed_sfx && packed_ids.is_empty();
        self.set_state(
            transaction,
            PREPARE_PACKED_SFX_WORK_ID,
            if packed_sfx_streamable {
                ambition_load::LoadWorkState::Running { progress: None }
            } else if packed_ids.is_empty() {
                ambition_load::LoadWorkState::Skipped
            } else {
                ambition_load::LoadWorkState::Complete
            },
        );
        if packed_sfx_streamable {
            self.streaming.pending_packed_sfx.insert(
                transaction.barrier.load_id.clone(),
                PendingPackedSfxReadiness {
                    provider_id: authored.audio_provider.clone(),
                },
            );
        } else {
            self.streaming
                .pending_packed_sfx
                .remove(&transaction.barrier.load_id);
        }

        let report = PlatformerPreparationReport {
            starting_character: authored.starting_character.clone(),
            sprite_manifest,
            sprite_asset,
            sprite_bound,
            music_ready,
            procedural_sfx_ready,
            adaptive_cues_ready,
            packed_sfx_streamable,
            deliberate_silence,
        };
        let identity =
            self.sessions
                .publish_with_report(transaction, world, report, &mut self.registry)?;
        self.complete(transaction, PREPARE_SESSION_WORK_ID);
        self.commands
            .write(ambition_load::LoadCommand::SetDiscovery {
                load_id: transaction.barrier.load_id.clone(),
                barrier_id: transaction.barrier.barrier_id.clone(),
                open: false,
                forecast: None,
            });
        Some(identity)
    }

    fn complete(&mut self, transaction: &ProviderLoadTransaction, work_id: &'static str) {
        self.set_state(transaction, work_id, ambition_load::LoadWorkState::Complete);
    }

    fn fail(
        &mut self,
        transaction: &ProviderLoadTransaction,
        work_id: &'static str,
        failure: ambition_load::LoadFailure,
    ) {
        self.set_state(
            transaction,
            work_id,
            ambition_load::LoadWorkState::Failed(failure),
        );
        self.commands
            .write(ambition_load::LoadCommand::SetDiscovery {
                load_id: transaction.barrier.load_id.clone(),
                barrier_id: transaction.barrier.barrier_id.clone(),
                open: false,
                forecast: None,
            });
    }

    fn set_state(
        &mut self,
        transaction: &ProviderLoadTransaction,
        work_id: &'static str,
        state: ambition_load::LoadWorkState,
    ) {
        self.commands
            .write(ambition_load::LoadCommand::SetWorkState {
                load_id: transaction.barrier.load_id.clone(),
                work_id: ambition_load::LoadWorkId::new(work_id),
                state,
            });
    }
}

#[derive(Clone)]
struct PreparedPlatformerRecord {
    transaction: ProviderLoadTransaction,
    world: PlatformerSessionWorld,
    report: PlatformerPreparationReport,
}

/// The owner of every prepared-but-not-yet-activated session world, keyed by
/// its load transaction. One resource serves all installed experiences: load
/// ids are globally unique, and activation retires records by exact
/// [`PreparedSessionIdentity`], so providers cannot observe each other's
/// prepared worlds.
#[derive(Resource, Default)]
pub struct PreparedPlatformerSessions {
    records: BTreeMap<ambition_load::LoadId, PreparedPlatformerRecord>,
}

impl PreparedPlatformerSessions {
    pub(crate) fn publish_with_report(
        &mut self,
        transaction: &ProviderLoadTransaction,
        world: PlatformerSessionWorld,
        report: PlatformerPreparationReport,
        registry: &mut PreparedSessionRegistry,
    ) -> Option<PreparedSessionIdentity> {
        let identity = registry.publish(transaction)?;
        let old = self.records.insert(
            transaction.barrier.load_id.clone(),
            PreparedPlatformerRecord {
                transaction: transaction.clone(),
                world,
                report,
            },
        );
        assert!(old.is_none(), "fresh prepared transaction was reused");
        Some(identity)
    }

    pub fn report(
        &self,
        identity: &PreparedSessionIdentity,
    ) -> Option<&PlatformerPreparationReport> {
        let record = self.records.get(&identity.transaction.barrier.load_id)?;
        (record.transaction == identity.transaction).then_some(&record.report)
    }

    pub(crate) fn take(
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

    pub(crate) fn retain_requested(&mut self, registry: &PreparedSessionRegistry) {
        self.records
            .retain(|load_id, _| registry.contains_load(load_id));
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

fn cleanup_prepared_platformer_sessions(
    registry: Res<PreparedSessionRegistry>,
    mut sessions: ResMut<PreparedPlatformerSessions>,
) {
    sessions.retain_requested(&registry);
}

/// The one activation system. For every activated experience with authored
/// platformer catalogs, it takes the prepared world by exact identity and
/// constructs the live session; the prepared report's starting character is
/// the session's default character (preparation proved it matches the world).
fn activate_prepared_platformer_sessions(
    mut events: MessageReader<GameplaySessionEvent>,
    authored_catalogs: Res<PlatformerAuthoredCatalogRegistry>,
    mut sessions: ResMut<PreparedPlatformerSessions>,
    mut registry: ResMut<PreparedSessionRegistry>,
    mut builder: PlatformerSessionBuilder,
) {
    for event in events.read() {
        let GameplaySessionEvent::Activated { activation, scope } = event else {
            continue;
        };
        let experience_id = activation.experience_id.as_str();
        if authored_catalogs.get(experience_id).is_none() {
            continue;
        }
        let prepared = activation.prepared_session.as_ref().unwrap_or_else(|| {
            panic!("experience '{experience_id}' requires an exact prepared-session publication")
        });
        let default_character = sessions
            .report(prepared)
            .unwrap_or_else(|| {
                panic!(
                    "experience '{experience_id}' prepared data must match the authorized transaction"
                )
            })
            .starting_character
            .clone();
        let live_world = sessions.take(prepared, &mut registry).unwrap_or_else(|| {
            panic!(
                "experience '{experience_id}' prepared data must match the authorized transaction"
            )
        });
        builder.build(activation, *scope, live_world, default_character.as_str());
    }
}

/// Constructs the live session for a prepared world: the simulation world and
/// player under the activation's session scope, the input-owner binding, and
/// the session-root publication through [`ActiveGameplaySession`].
#[derive(SystemParam)]
pub struct PlatformerSessionBuilder<'w, 's> {
    commands: Commands<'w, 's>,
    editable_abilities: Res<'w, ambition_dev_tools::dev_tools::EditableAbilitySet>,
    editable_tuning: Res<'w, ambition_dev_tools::dev_tools::EditableMovementTuning>,
    asset_server: Res<'w, AssetServer>,
    character_catalog: Res<'w, ambition_characters::actor::character_catalog::CharacterCatalog>,
    character_roster: Res<'w, ambition_actors::features::CharacterRoster>,
    boss_catalog: Res<'w, ambition_actors::boss_encounter::BossCatalog>,
    placement_lowering: Res<'w, ambition_actors::world::placements::PlacementLoweringRegistry>,
    content_staging: Res<'w, ambition_actors::features::RoomContentStagingRegistry>,
    sandbox_data_asset: Option<Res<'w, ambition_actors::session::data::SandboxDataAsset>>,
    sandbox_asset_collection:
        Option<Res<'w, ambition_actors::assets::loading::SandboxAssetCollection>>,
    moving_platforms: ResMut<'w, ambition_world::collision::MovingPlatformSet>,
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
        // Live moving-platform state derives from the activating room. Rooms
        // without authored platforms (every current demo) reset it to empty.
        self.moving_platforms.0 = ambition_actors::world::platforms::moving_platforms_for_room(
            live_world.room_set.active_spec(),
        );

        let player = ambition_actors::session::setup::simulation_world(
            &mut self.commands,
            SessionSpawnScope::scoped(scope),
            ambition_actors::session::setup::SimulationSetup {
                world: &live_world.geometry,
                room_set: &live_world.room_set,
                ldtk_index: &live_world.runtime_rooms,
                editable_abilities: &self.editable_abilities,
                editable_tuning: &self.editable_tuning,
                starting_character: &live_world.starting_character,
                character_catalog: &self.character_catalog,
                character_roster: &self.character_roster,
                placement_lowering: &self.placement_lowering,
                content_staging: &self.content_staging,
                boss_catalog: &self.boss_catalog,
                default_character_id,
                sandbox_data_asset: self.sandbox_data_asset.as_deref(),
                sandbox_asset_collection: self.sandbox_asset_collection.as_deref(),
                asset_server: &self.asset_server,
            },
        );

        self.commands.entity(player).insert(GameplayInputOwner {
            activation_id: activation.activation_id,
            scope,
        });

        let world = self
            .active_session
            .spawn_world_for(&mut self.commands, activation, scope, live_world)
            .expect("provider activation still owns the session it is constructing");

        SessionBuildResult { player, world }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authoring::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};

    #[test]
    fn authoring_installation_registers_preparation_resources_synchronously() {
        let mut app = App::new();
        PlatformerExperienceAuthoring::new(
            "fixture",
            "fixture_gameplay",
            "Fixture",
            "Provider fixture",
            "Prepare fixture",
            AuthoredCatalogFragments::new("fixture_character", "fixture"),
        )
        .install(&mut app, || -> PlatformerSessionWorld {
            unreachable!("the fixture never receives a preparation request")
        });

        assert!(app
            .world()
            .contains_resource::<PlatformerAuthoredCatalogRegistry>());
        assert!(app
            .world()
            .contains_resource::<PlatformerStreamingReadiness>());
        assert!(app
            .world()
            .contains_resource::<PreparedPlatformerSessions>());
    }

    #[test]
    fn packed_bank_readiness_completes_only_the_matching_streamable_transaction() {
        let load_id = ambition_load::LoadId::new("provider-load");
        let other_load_id = ambition_load::LoadId::new("other-load");
        let mut loads = ambition_load::LoadCoordinator::default();
        loads.apply(ambition_load::LoadCommand::Begin(
            ambition_load::LoadPlanSpec::new(load_id.clone(), "Provider load"),
        ));
        loads.apply(ambition_load::LoadCommand::Begin(
            ambition_load::LoadPlanSpec::new(other_load_id.clone(), "Other load"),
        ));

        let mut pending = PlatformerStreamingReadiness::default();
        pending.pending_packed_sfx.insert(
            load_id.clone(),
            PendingPackedSfxReadiness {
                provider_id: "provider".to_owned(),
            },
        );
        pending.pending_packed_sfx.insert(
            other_load_id.clone(),
            PendingPackedSfxReadiness {
                provider_id: "other".to_owned(),
            },
        );

        let mut app = App::new();
        app.add_message::<ambition_load::LoadCommand>()
            .insert_resource(loads)
            .insert_resource(pending)
            .insert_resource(ambition_audio::catalog::SfxBankRegistry::default())
            .add_systems(Update, update_streamable_packed_sfx);
        app.update();
        assert!(app
            .world_mut()
            .resource_mut::<Messages<ambition_load::LoadCommand>>()
            .drain()
            .next()
            .is_none());

        app.world_mut()
            .resource_mut::<ambition_audio::catalog::SfxBankRegistry>()
            .register(
                "provider",
                BTreeMap::from([(ambition_sfx::SfxId::from_static("provider.ready"), 7)]),
            )
            .expect("fixture bank fragment registers");
        app.update();
        let commands = app
            .world_mut()
            .resource_mut::<Messages<ambition_load::LoadCommand>>()
            .drain()
            .collect::<Vec<_>>();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            ambition_load::LoadCommand::SetWorkState {
                load_id: completed_load,
                work_id,
                state: ambition_load::LoadWorkState::Complete,
            } if completed_load == &load_id && work_id.as_str() == PREPARE_PACKED_SFX_WORK_ID
        ));
        let readiness = app.world().resource::<PlatformerStreamingReadiness>();
        assert!(!readiness.pending_packed_sfx.contains_key(&load_id));
        assert!(readiness.pending_packed_sfx.contains_key(&other_load_id));
    }
}
