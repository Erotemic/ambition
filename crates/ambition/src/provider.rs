//! Compact Bevy-native authoring surface for platformer experience providers.

use std::{collections::BTreeMap, marker::PhantomData};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::game_shell::{
    standard_platformer_preparation_plan, ActiveGameplaySession, ActiveShellExperience,
    ExperienceRegistration, GameplaySessionAppExt, PreparedSessionIdentity,
    PreparedSessionRegistry, ProviderLoadTransaction, ShellCompletionPolicy, ShellRouteSpec,
    PREPARE_ADAPTIVE_WORK_ID, PREPARE_AUDIO_WORK_ID, PREPARE_CATALOGS_WORK_ID,
    PREPARE_DEFAULTS_WORK_ID, PREPARE_MUSIC_WORK_ID, PREPARE_PACKED_SFX_WORK_ID,
    PREPARE_SESSION_WORK_ID, PREPARE_SFX_WORK_ID, PREPARE_SPRITES_WORK_ID,
    PREPARE_WORLD_WORK_ID,
};
use crate::platformer::lifecycle::{SessionScopeId, SessionSpawnScope};
use crate::runtime::PlatformerSessionWorld;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredCatalogFragments {
    pub starting_character: String,
    pub audio_provider: String,
    pub expects_music: bool,
    pub expects_procedural_sfx: bool,
    pub expects_adaptive_cues: bool,
    pub expects_packed_sfx: bool,
}

impl AuthoredCatalogFragments {
    pub fn new(
        starting_character: impl Into<String>,
        audio_provider: impl Into<String>,
    ) -> Self {
        Self {
            starting_character: starting_character.into(),
            audio_provider: audio_provider.into(),
            expects_music: false,
            expects_procedural_sfx: false,
            expects_adaptive_cues: false,
            expects_packed_sfx: false,
        }
    }

    pub fn with_music(mut self) -> Self {
        self.expects_music = true;
        self
    }

    pub fn with_procedural_sfx(mut self) -> Self {
        self.expects_procedural_sfx = true;
        self
    }

    pub fn with_adaptive_cues(mut self) -> Self {
        self.expects_adaptive_cues = true;
        self
    }

    pub fn with_packed_sfx(mut self) -> Self {
        self.expects_packed_sfx = true;
        self
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

#[derive(Resource, Default)]
pub struct PlatformerAuthoredCatalogRegistry {
    by_experience: BTreeMap<String, AuthoredCatalogFragments>,
}

impl PlatformerAuthoredCatalogRegistry {
    pub fn get(&self, experience_id: &str) -> Option<&AuthoredCatalogFragments> {
        self.by_experience.get(experience_id)
    }

    fn register(&mut self, experience_id: &str, fragments: AuthoredCatalogFragments) {
        if let Some(existing) = self.by_experience.get(experience_id) {
            assert_eq!(
                existing, &fragments,
                "platformer experience '{experience_id}' registered conflicting authored catalogs",
            );
            return;
        }
        self.by_experience
            .insert(experience_id.to_owned(), fragments);
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
    pub loading: Option<crate::load_presentation::LoadExperienceSpec>,
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
            loading: None,
        }
    }

    pub fn with_loading_activity(mut self, activity_id: impl Into<String>) -> Self {
        let mut loading = crate::load_presentation::LoadExperienceSpec::basic(format!(
            "{}.loading", self.experience_id
        ));
        loading.activity = Some(crate::load_presentation::LoadActivityId::new(activity_id));
        loading.ready_policy =
            crate::load_presentation::ReadyTransitionPolicy::AutoUnlessEngaged;
        self.loading = Some(loading);
        self
    }

    pub fn with_loading_spec(
        mut self,
        loading: crate::load_presentation::LoadExperienceSpec,
    ) -> Self {
        self.loading = Some(loading);
        self
    }

    pub fn register(&self, app: &mut App) {
        // Provider registration is the authoritative composition seam. Install
        // both preparation resources synchronously here before any provider
        // systems can be initialized. The runtime plugin also uses `init`, but
        // relying on a nested plugin build to publish the private streaming
        // resource left thin standalone hosts vulnerable to first-update
        // SystemParam validation failures.
        app.init_resource::<PlatformerAuthoredCatalogRegistry>()
            .init_resource::<PlatformerStreamingReadiness>();
        if !app.is_plugin_added::<PlatformerProviderRuntimePlugin>() {
            app.add_plugins(PlatformerProviderRuntimePlugin);
        }
        app.world_mut()
            .resource_mut::<PlatformerAuthoredCatalogRegistry>()
            .register(self.experience_id.as_str(), self.catalogs.clone());
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
        if let Some(loading) = self.loading.clone() {
            app.init_resource::<crate::load_presentation::LoadPresentationCatalog>();
            app.world_mut()
                .resource_mut::<crate::load_presentation::LoadPresentationCatalog>()
                .by_route
                .insert(
                    crate::game_shell::ShellRouteId::new(self.route_id.clone()),
                    loading,
                );
        }
    }
}

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
struct PendingPackedSfxReadiness {
    provider_id: String,
}

#[derive(Resource, Default)]
struct PlatformerStreamingReadiness {
    pending_packed_sfx: BTreeMap<crate::load::LoadId, PendingPackedSfxReadiness>,
}

struct PlatformerProviderRuntimePlugin;

impl Plugin for PlatformerProviderRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlatformerStreamingReadiness>()
            .add_systems(
                Update,
                update_streamable_packed_sfx
                    .in_set(crate::load::AmbitionLoadSet::Contributors),
            );
    }
}

fn update_streamable_packed_sfx(
    loads: Res<crate::load::LoadCoordinator>,
    banks: Option<Res<crate::audio::catalog::SfxBankRegistry>>,
    mut readiness: ResMut<PlatformerStreamingReadiness>,
    mut commands: MessageWriter<crate::load::LoadCommand>,
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
        commands.write(crate::load::LoadCommand::SetWorkState {
            load_id: load_id.clone(),
            work_id: crate::load::LoadWorkId::new(PREPARE_PACKED_SFX_WORK_ID),
            state: crate::load::LoadWorkState::Complete,
        });
        readiness.pending_packed_sfx.remove(&load_id);
    }
}

#[derive(SystemParam)]
pub struct PlatformerPreparation<'w> {
    authored_catalogs: Res<'w, PlatformerAuthoredCatalogRegistry>,
    character_catalog:
        Res<'w, crate::characters::actor::character_catalog::CharacterCatalog>,
    audio_catalogs: Res<'w, crate::audio::catalog::AudioCatalogRegistry>,
    #[cfg(feature = "audio")]
    adaptive_catalogs: Option<Res<'w, crate::audio::music::AdaptiveMusicCatalogRegistry>>,
    sfx_banks: Option<Res<'w, crate::audio::catalog::SfxBankRegistry>>,
    game_assets: Option<Res<'w, crate::sprite_sheet::game_assets::GameAssets>>,
    registry: ResMut<'w, PreparedSessionRegistry>,
    streaming: ResMut<'w, PlatformerStreamingReadiness>,
    commands: MessageWriter<'w, crate::load::LoadCommand>,
}

impl PlatformerPreparation<'_> {
    pub fn prepare<M: Send + Sync + 'static>(
        &mut self,
        transaction: &ProviderLoadTransaction,
        world: PlatformerSessionWorld,
        sessions: &mut PreparedPlatformerSessions<M>,
    ) -> Option<PreparedSessionIdentity> {
        let Some(authored) = self
            .authored_catalogs
            .get(transaction.experience_id.as_str())
            .cloned()
        else {
            self.fail(
                transaction,
                PREPARE_CATALOGS_WORK_ID,
                crate::load::LoadFailure::new(
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
                crate::load::LoadWorkState::Running { progress: None },
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
                crate::load::LoadFailure::new(
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
                crate::load::LoadFailure::new(
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
                crate::load::LoadFailure::new(
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
                crate::load::LoadFailure::new(
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
                crate::load::LoadFailure::new(
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
                crate::load::LoadFailure::new(
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
                crate::load::LoadWorkState::Running { progress: None }
            } else if packed_ids.is_empty() {
                crate::load::LoadWorkState::Skipped
            } else {
                crate::load::LoadWorkState::Complete
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
        let identity = sessions.publish_with_report(
            transaction,
            world,
            report,
            &mut self.registry,
        )?;
        self.complete(transaction, PREPARE_SESSION_WORK_ID);
        self.commands.write(crate::load::LoadCommand::SetDiscovery {
            load_id: transaction.barrier.load_id.clone(),
            barrier_id: transaction.barrier.barrier_id.clone(),
            open: false,
            forecast: None,
        });
        Some(identity)
    }

    fn complete(&mut self, transaction: &ProviderLoadTransaction, work_id: &'static str) {
        self.set_state(transaction, work_id, crate::load::LoadWorkState::Complete);
    }

    fn fail(
        &mut self,
        transaction: &ProviderLoadTransaction,
        work_id: &'static str,
        failure: crate::load::LoadFailure,
    ) {
        self.set_state(transaction, work_id, crate::load::LoadWorkState::Failed(failure));
        self.commands.write(crate::load::LoadCommand::SetDiscovery {
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
        state: crate::load::LoadWorkState,
    ) {
        self.commands.write(crate::load::LoadCommand::SetWorkState {
            load_id: transaction.barrier.load_id.clone(),
            work_id: crate::load::LoadWorkId::new(work_id),
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
    pub fn publish_with_report(
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

        self.commands.entity(player).insert(crate::game_shell::GameplayInputOwner {
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

    #[test]
    fn authoring_registration_installs_preparation_resources_synchronously() {
        let mut app = App::new();
        PlatformerExperienceAuthoring::new(
            "fixture",
            "fixture_gameplay",
            "Fixture",
            "Provider fixture",
            "Prepare fixture",
            AuthoredCatalogFragments::new("fixture_character", "fixture"),
        )
        .register(&mut app);

        assert!(app
            .world()
            .contains_resource::<PlatformerAuthoredCatalogRegistry>());
        assert!(app
            .world()
            .contains_resource::<PlatformerStreamingReadiness>());
    }

    #[test]
    fn packed_bank_readiness_completes_only_the_matching_streamable_transaction() {
        let load_id = crate::load::LoadId::new("provider-load");
        let other_load_id = crate::load::LoadId::new("other-load");
        let mut loads = crate::load::LoadCoordinator::default();
        loads.apply(crate::load::LoadCommand::Begin(crate::load::LoadPlanSpec::new(
            load_id.clone(),
            "Provider load",
        )));
        loads.apply(crate::load::LoadCommand::Begin(crate::load::LoadPlanSpec::new(
            other_load_id.clone(),
            "Other load",
        )));

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
        app.add_message::<crate::load::LoadCommand>()
            .insert_resource(loads)
            .insert_resource(pending)
            .insert_resource(crate::audio::catalog::SfxBankRegistry::default())
            .add_systems(Update, update_streamable_packed_sfx);
        app.update();
        assert!(app
            .world_mut()
            .resource_mut::<Messages<crate::load::LoadCommand>>()
            .drain()
            .next()
            .is_none());

        app.world_mut()
            .resource_mut::<crate::audio::catalog::SfxBankRegistry>()
            .register(
                "provider",
                BTreeMap::from([(crate::sfx::SfxId::from_static("provider.ready"), 7)]),
            )
            .expect("fixture bank fragment registers");
        app.update();
        let commands = app
            .world_mut()
            .resource_mut::<Messages<crate::load::LoadCommand>>()
            .drain()
            .collect::<Vec<_>>();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            crate::load::LoadCommand::SetWorkState {
                load_id: completed_load,
                work_id,
                state: crate::load::LoadWorkState::Complete,
            } if completed_load == &load_id && work_id.as_str() == PREPARE_PACKED_SFX_WORK_ID
        ));
        let readiness = app.world().resource::<PlatformerStreamingReadiness>();
        assert!(!readiness.pending_packed_sfx.contains_key(&load_id));
        assert!(readiness.pending_packed_sfx.contains_key(&other_load_id));
    }
}
