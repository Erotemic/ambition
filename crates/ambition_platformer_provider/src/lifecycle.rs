//! The shared provider lifecycle: preparation, prepared-session ownership,
//! and activation into the live session world.
//!
//! Every experience registered through
//! [`PlatformerExperienceAuthoring::install`](crate::authoring::PlatformerExperienceAuthoring::install)
//! shares these systems; the provider contributes only its session-world
//! source. The answers to the lifecycle questions live here:
//!
//! - **What does a provider prepare?** A [`PreparedPlatformerSource`], validated
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
use ambition_runtime::{
    ContentDiagnostic, ContentEpochSequence, ContentOwner, PlatformerSessionWorld, PreparedContent,
    PreparedContentBuilder, PreparedContentIdentity, PreparedPlatformerSource,
};

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
            .init_resource::<ContentEpochSequence>()
            .init_resource::<
                ambition_platformer_primitives::gameplay_presentation::ActiveGameplayPresentationProfiles,
            >()
            // The HUD's two route-following channels, owned here for the same
            // reason as the profile above: this layer REGISTERS the systems
            // that write them, so this layer must guarantee they exist.
            // `DeclaredHudPlugin` also inits both, which hid the gap until a
            // composition ran a HUD-declaring route WITHOUT the renderer —
            // `shell_host_lifecycle` then panicked with "Resource does not
            // exist", first on `HudReadouts` (the game's publisher) and then on
            // `ActiveHudDeclaration` (`select_active_hud_declaration`, right
            // below). A headless host must be able to run a game that has a HUD.
            .init_resource::<
                ambition_platformer_primitives::gameplay_presentation::ActiveHudDeclaration,
            >()
            .init_resource::<
                ambition_platformer_primitives::gameplay_presentation::HudReadouts,
            >()
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
                    // Presentation follows the route, so it must settle after
                    // activation and BEFORE the host resolves this frame's
                    // layout — otherwise every experience switch shows one
                    // frame of the previous game's viewport.
                    crate::authoring::select_active_presentation_profiles
                        .after(activate_prepared_platformer_sessions)
                        .before(
                            ambition_platformer_primitives::gameplay_presentation::GameplayPresentationSet,
                        ),
                    // The HUD declaration follows the route on the same
                    // schedule and for the same reason: a switch must not show
                    // one frame of the previous game's readouts.
                    crate::authoring::select_active_hud_declaration
                        .after(activate_prepared_platformer_sessions)
                        .before(
                            ambition_platformer_primitives::gameplay_presentation::GameplayPresentationSet,
                        ),
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
    In((experience_id, source)): In<(String, PreparedPlatformerSource)>,
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
        preparation.prepare(transaction, source.clone());
    }
}

/// Catalog validation + prepared-session publication for one transaction.
#[derive(SystemParam)]
pub(crate) struct PlatformerPreparation<'w> {
    authored_catalogs: Res<'w, PlatformerAuthoredCatalogRegistry>,
    character_catalog: Res<'w, ambition_characters::actor::character_catalog::CharacterCatalog>,
    character_catalog_registry:
        Option<Res<'w, ambition_characters::actor::character_catalog::CharacterCatalogRegistry>>,
    snapshot_registry: Option<Res<'w, ambition_runtime::rollback::RollbackRegistry>>,
    placement_lowering:
        Option<Res<'w, ambition_actors::world::placements::PlacementLoweringRegistry>>,
    content_staging: Option<Res<'w, ambition_actors::features::RoomContentStagingRegistry>>,
    // ⚠ This brings the struct to Bevy's 16-parameter `SystemParam` ceiling.
    // The next field added here must bundle something first.
    construction_recipes: Option<Res<'w, ambition_actors::construction::ActorConstructionRegistry>>,
    epochs: ResMut<'w, ContentEpochSequence>,
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
        source: PreparedPlatformerSource,
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

        if source.active_room_id().trim().is_empty()
            || source.catalogs().world_provider.trim().is_empty()
            || source.catalogs().character_provider.trim().is_empty()
            || source.catalogs().audio_provider.trim().is_empty()
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

        let effective_character = source
            .starting_character()
            .effective_id(authored.starting_character.as_str());
        if effective_character != authored.starting_character.as_str()
            || source.catalogs().audio_provider.as_str() != authored.audio_provider.as_str()
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
                        source.catalogs().audio_provider,
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
        let snapshot_schema = self
            .snapshot_registry
            .as_deref()
            .map(ambition_runtime::rollback::RollbackRegistry::schema_fingerprint)
            .unwrap_or_else(|| {
                ambition_runtime::rollback::RollbackRegistry::default().schema_fingerprint()
            });
        let content = match prepare_platformer_content(
            source,
            &authored,
            self.character_catalog_registry.as_deref(),
            self.placement_lowering.as_deref(),
            self.content_staging.as_deref(),
            self.construction_recipes
                .as_deref()
                .map(ambition_actors::construction::ActorConstructionRegistry::deterministic_dump),
            snapshot_schema,
            &mut self.epochs,
        ) {
            Ok(content) => content,
            Err(diagnostic) => {
                self.fail(
                    transaction,
                    PREPARE_SESSION_WORK_ID,
                    ambition_load::LoadFailure::new(
                        "Prepared content assembly failed",
                        diagnostic.to_string(),
                    )
                    .retryable(false),
                );
                return None;
            }
        };
        let identity = self.sessions.publish(
            transaction,
            PreparedPlatformerSession { content, report },
            &mut self.registry,
        )?;
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

fn add_world_fingerprint_sections(
    builder: &mut PreparedContentBuilder,
    source: &PreparedPlatformerSource,
) -> Result<(), ContentDiagnostic> {
    let mut rooms = source.room_set().rooms.clone();
    rooms.sort_by(|a, b| a.id.cmp(&b.id));
    let rooms = ron::ser::to_string(&rooms).map_err(|error| {
        ContentDiagnostic::new(
            "world.rooms",
            format!("canonical room serialization failed: {error}"),
        )
    })?;
    builder
        .add_section("world.rooms", rooms.into_bytes())
        .map_err(|error| ContentDiagnostic::new("world.rooms", error.to_string()))?;
    let links = ron::ser::to_string(&source.room_set().canonical_links()).map_err(|error| {
        ContentDiagnostic::new(
            "world.graph",
            format!("canonical room-link serialization failed: {error}"),
        )
    })?;
    builder
        .add_section("world.graph", links.into_bytes())
        .map_err(|error| ContentDiagnostic::new("world.graph", error.to_string()))?;
    let active_geometry = ron::ser::to_string(&source.geometry().0).map_err(|error| {
        ContentDiagnostic::new(
            "world.active-geometry",
            format!("canonical active geometry serialization failed: {error}"),
        )
    })?;
    builder
        .add_section("world.active-geometry", active_geometry.into_bytes())
        .map_err(|error| ContentDiagnostic::new("world.active-geometry", error.to_string()))?;
    let active_metadata = ron::ser::to_string(&source.active_room().0).map_err(|error| {
        ContentDiagnostic::new(
            "world.active-metadata",
            format!("canonical active metadata serialization failed: {error}"),
        )
    })?;
    builder
        .add_section("world.active-metadata", active_metadata.into_bytes())
        .map_err(|error| ContentDiagnostic::new("world.active-metadata", error.to_string()))?;
    let start_room = source
        .room_set()
        .rooms
        .get(source.room_set().start)
        .map(|room| room.id.as_str())
        .unwrap_or("<missing>");
    builder
        .add_section(
            "world.initial-state",
            format!(
                "active_room={}\nstart_room={}\nstarting_character={}\n",
                source.active_room_id(),
                start_room,
                source.starting_character().character_id,
            )
            .into_bytes(),
        )
        .map_err(|error| ContentDiagnostic::new("world.initial-state", error.to_string()))?;

    // The LDtk index carries deterministic area-to-level membership and area
    // bounds used by streaming/level selection. Its mutable revision/sync
    // cursors are deliberately excluded.
    let mut runtime_index = String::new();
    let mut area_ids = source
        .room_set()
        .rooms
        .iter()
        .map(|room| room.id.as_str())
        .collect::<Vec<_>>();
    area_ids.sort_unstable();
    area_ids.dedup();
    for area_id in area_ids {
        let mut level_iids = source.runtime_rooms().level_iids_for(area_id);
        level_iids.sort();
        runtime_index.push_str("area\t");
        runtime_index.push_str(area_id);
        runtime_index.push('\t');
        runtime_index.push_str(&level_iids.join(","));
        if let Some(bounds) = source.runtime_rooms().area_bounds(area_id) {
            runtime_index.push_str(&format!(
                "\t{},{},{},{}",
                bounds.min_x, bounds.min_y, bounds.max_x, bounds.max_y,
            ));
        } else {
            runtime_index.push_str("\t-");
        }
        runtime_index.push('\n');
    }
    builder
        .add_section("world.runtime-index", runtime_index.into_bytes())
        .map_err(|error| ContentDiagnostic::new("world.runtime-index", error.to_string()))?;
    Ok(())
}

/// Build an LDtk replacement candidate from the active immutable definition.
/// Non-world sections are copied byte-for-byte; world sections are regenerated
/// by the same canonical function used during initial provider preparation.
pub fn prepare_world_replacement_candidate(
    active: &PreparedContent,
    source: PreparedPlatformerSource,
    snapshot_schema: ambition_runtime::SnapshotSchemaFingerprint,
) -> Result<PreparedContent, ContentDiagnostic> {
    // The incoming replacement may have been assembled while the live session
    // was in a room other than the definition's original activation room.
    // Normalize that mutable cursor before hashing so ordinary room movement
    // cannot manufacture a new content identity.
    let definition_room = active.source().active_room_id();
    let source = source
        .with_definition_active_room(definition_room)
        .ok_or_else(|| {
            ContentDiagnostic::new(
                "world.definition-active-room",
                format!(
                    "replacement world does not contain the active definition room '{}'",
                    definition_room,
                ),
            )
        })?;
    let mut builder = PreparedContentBuilder::default();
    for owner in active.owners() {
        builder.add_owner(owner.clone());
    }
    for section in active.sections() {
        if !section.name.starts_with("world.") {
            builder
                .add_section(section.name.clone(), section.canonical_bytes().to_vec())
                .map_err(|error| ContentDiagnostic::new(section.name.clone(), error.to_string()))?;
        }
    }
    add_world_fingerprint_sections(&mut builder, &source)?;
    Ok(builder.finish(active.epoch(), snapshot_schema, source))
}

/// Assemble exact immutable content for a direct-entry app after its plugins
/// have installed construction and snapshot registries. Direct demos use this
/// instead of hand-building an un-fingerprinted live session root.
pub fn prepare_platformer_content_for_app(
    app: &mut App,
    source: PreparedPlatformerSource,
    authored: &crate::authoring::AuthoredCatalogFragments,
) -> Result<PreparedContent, ContentDiagnostic> {
    let character_registry = app
        .world()
        .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalogRegistry>()
        .cloned();
    let placement_lowering = app
        .world()
        .get_resource::<ambition_actors::world::placements::PlacementLoweringRegistry>()
        .cloned();
    let content_staging = app
        .world()
        .get_resource::<ambition_actors::features::RoomContentStagingRegistry>()
        .cloned();
    let construction_recipes = app
        .world()
        .get_resource::<ambition_actors::construction::ActorConstructionRegistry>()
        .map(|registry| registry.deterministic_dump());
    let snapshot_schema = app
        .world()
        .get_resource::<ambition_runtime::rollback::RollbackRegistry>()
        .map(ambition_runtime::rollback::RollbackRegistry::schema_fingerprint)
        .unwrap_or_else(|| {
            ambition_runtime::rollback::RollbackRegistry::default().schema_fingerprint()
        });
    app.init_resource::<ContentEpochSequence>();
    let mut epochs = app.world_mut().resource_mut::<ContentEpochSequence>();
    prepare_platformer_content(
        source,
        authored,
        character_registry.as_ref(),
        placement_lowering.as_ref(),
        content_staging.as_ref(),
        construction_recipes,
        snapshot_schema,
        &mut epochs,
    )
}

pub fn prepare_platformer_content(
    source: PreparedPlatformerSource,
    authored: &crate::authoring::AuthoredCatalogFragments,
    character_registry: Option<
        &ambition_characters::actor::character_catalog::CharacterCatalogRegistry,
    >,
    placement_lowering: Option<&ambition_actors::world::placements::PlacementLoweringRegistry>,
    content_staging: Option<&ambition_actors::features::RoomContentStagingRegistry>,
    // Canonical dump of the construction registry, when the app has one. A dump
    // rather than the registry itself: `ConstructionRegistry` is not `Clone` (it
    // holds relation `fn` pointers), and the fingerprint wants only its stable
    // semantic metadata anyway. Nothing process-local is hashed.
    construction_recipes: Option<String>,
    snapshot_schema: ambition_runtime::SnapshotSchemaFingerprint,
    epochs: &mut ContentEpochSequence,
) -> Result<PreparedContent, ContentDiagnostic> {
    if source.active_room_id().trim().is_empty()
        || source.catalogs().world_provider.trim().is_empty()
        || source.catalogs().character_provider.trim().is_empty()
        || source.catalogs().audio_provider.trim().is_empty()
    {
        return Err(ContentDiagnostic::new(
            "provider.source",
            "active room and provider identities must not be empty",
        ));
    }
    let effective_character = source
        .starting_character()
        .effective_id(authored.starting_character.as_str());
    if effective_character != authored.starting_character.as_str()
        || source.catalogs().audio_provider.as_str() != authored.audio_provider.as_str()
    {
        return Err(ContentDiagnostic::new(
            "provider.defaults",
            format!(
                "expected character '{}' and audio provider '{}', got '{}' and '{}'",
                authored.starting_character,
                authored.audio_provider,
                effective_character,
                source.catalogs().audio_provider,
            ),
        ));
    }
    for room in &source.room_set().rooms {
        if !room.placements.is_empty() && placement_lowering.is_none() {
            return Err(ContentDiagnostic::new(
                format!("world.room.{}.placements", room.id),
                "authored placements require an installed placement-lowering registry",
            ));
        }
        if let Some(registry) = placement_lowering {
            registry
                .validate_room(room.id.as_str(), &room.placements)
                .map_err(|error| {
                    ContentDiagnostic::new(
                        format!("world.room.{}.placements", room.id),
                        error.to_string(),
                    )
                })?;
        }
        if let Some(registry) = content_staging {
            registry.try_requests_for(room).map_err(|error| {
                ContentDiagnostic::new(
                    format!("world.room.{}.content-staging", room.id),
                    error.to_string(),
                )
            })?;
        }
    }

    let mut builder = PreparedContentBuilder::default();
    builder.add_owner(ContentOwner::new(
        source.catalogs().world_provider.clone(),
        "provider-session-source",
        "world",
    ));
    builder.add_owner(ContentOwner::new(
        source.catalogs().character_provider.clone(),
        "character-catalog",
        "characters",
    ));
    builder.add_owner(ContentOwner::new(
        source.catalogs().audio_provider.clone(),
        "audio-catalog",
        "audio",
    ));

    builder
        .add_section(
            "provider.catalogs",
            format!(
                "world={}\ncharacters={}\naudio={}\n",
                source.catalogs().world_provider,
                source.catalogs().character_provider,
                source.catalogs().audio_provider,
            )
            .into_bytes(),
        )
        .map_err(|error| ContentDiagnostic::new("provider.catalogs", error.to_string()))?;
    builder.add_section(
        "provider.authored-defaults",
        format!(
            "starting_character={}\naudio_provider={}\nexpects_music={}\nexpects_procedural_sfx={}\nexpects_adaptive_cues={}\nexpects_packed_sfx={}\n",
            authored.starting_character,
            authored.audio_provider,
            authored.expects_music,
            authored.expects_procedural_sfx,
            authored.expects_adaptive_cues,
            authored.expects_packed_sfx,
        ).into_bytes(),
).map_err(|error| ContentDiagnostic::new("provider.authored-defaults", error.to_string()))?;

    add_world_fingerprint_sections(&mut builder, &source)?;

    // Conservative Phase-1 contract: all linked character fragments contribute,
    // because rooms and content stagers may select characters dynamically. This
    // is broader than the eventual dependency closure but never under-binds.
    if let Some(registry) = character_registry {
        for (provider, default, source_ron) in registry.canonical_fragments() {
            builder.add_owner(ContentOwner::new(
                &provider,
                "registered-character-fragment",
                "characters",
            ));
            let mut bytes = format!(
                "provider={provider}\ndefault={}\n",
                default.as_deref().unwrap_or("-")
            )
            .into_bytes();
            bytes.extend_from_slice(source_ron.as_bytes());
            let section = format!("characters.fragment.{provider}");
            builder
                .add_section(section.clone(), bytes)
                .map_err(|error| ContentDiagnostic::new(section, error.to_string()))?;
        }
    }
    if let Some(registry) = placement_lowering {
        for (_, owner, source_id, _) in registry.schema_descriptors() {
            builder.add_owner(ContentOwner::new(owner, source_id, "placement-lowering"));
        }
    }
    builder
        .add_section(
            "construction.placement-lowering",
            placement_lowering.map_or_else(Vec::new, |registry| {
                registry.deterministic_dump().into_bytes()
            }),
        )
        .map_err(|error| {
            ContentDiagnostic::new("construction.placement-lowering", error.to_string())
        })?;
    if let Some(registry) = content_staging {
        for (_, owner, source_id, _) in registry.schema_descriptors() {
            builder.add_owner(ContentOwner::new(owner, source_id, "content-staging"));
        }
    }
    builder
        .add_section(
            "construction.content-staging",
            content_staging.map_or_else(Vec::new, |registry| {
                registry.deterministic_dump().into_bytes()
            }),
        )
        .map_err(|error| {
            ContentDiagnostic::new("construction.content-staging", error.to_string())
        })?;

    // The construction recipe table decides how authoritative entities are
    // built, so a change to it is a change to the content — two sessions whose
    // recipe schemas differ are not interchangeable, and a snapshot taken under
    // one is not safe to restore under the other. It was documented as
    // contributing to the fingerprint well before it actually did.
    //
    // ⚠ Only what the dump carries is hashed: for a recipe, its id + owner +
    // source + schema id; for a relation, its kind + owner + source + schema id.
    //
    // Neither the wiring function nor the postcondition verifier is hashed, and
    // neither can be: a `fn` address is process-local, so hashing one would make
    // the fingerprint differ between two runs of the same binary. **Bumping the
    // schema id is therefore the ONLY way a behaviour change reaches the
    // fingerprint**, which is the same rule that governs relation registration
    // identity — see `try_register_relation`, which deliberately does not
    // compare function addresses either. Postcondition verification exists
    // partly because of this gap: a relation whose wiring silently stopped
    // working under an unchanged schema id is invisible here, and visible there.
    builder
        .add_section(
            "construction.recipes",
            construction_recipes.map_or_else(Vec::new, |dump| dump.into_bytes()),
        )
        .map_err(|error| ContentDiagnostic::new("construction.recipes", error.to_string()))?;

    // Epoch allocation is the final non-fallible step: a rejected candidate
    // never consumes or publishes an activation generation.
    Ok(builder.finish(epochs.allocate(), snapshot_schema, source))
}

#[derive(Clone)]
struct PreparedPlatformerRecord {
    transaction: ProviderLoadTransaction,
    prepared: PreparedPlatformerSession,
}

/// One coherent publication: immutable content and the report produced by the
/// same successful validation/assembly transaction.
#[derive(Clone)]
pub struct PreparedPlatformerSession {
    pub content: PreparedContent,
    pub report: PlatformerPreparationReport,
}

#[derive(Resource, Default)]
pub struct PreparedPlatformerSessions {
    records: BTreeMap<ambition_load::LoadId, PreparedPlatformerRecord>,
}

impl PreparedPlatformerSessions {
    pub(crate) fn publish(
        &mut self,
        transaction: &ProviderLoadTransaction,
        prepared: PreparedPlatformerSession,
        registry: &mut PreparedSessionRegistry,
    ) -> Option<PreparedSessionIdentity> {
        let identity = registry.publish(transaction)?;
        let old = self.records.insert(
            transaction.barrier.load_id.clone(),
            PreparedPlatformerRecord {
                transaction: transaction.clone(),
                prepared,
            },
        );
        assert!(old.is_none(), "fresh prepared transaction was reused");
        Some(identity)
    }

    pub(crate) fn take(
        &mut self,
        identity: &PreparedSessionIdentity,
        registry: &mut PreparedSessionRegistry,
    ) -> Option<PreparedPlatformerSession> {
        let load_id = &identity.transaction.barrier.load_id;
        let record = self.records.remove(load_id)?;
        if record.transaction != identity.transaction || !registry.retire_prepared(identity) {
            self.records.insert(load_id.clone(), record);
            return None;
        }
        Some(record.prepared)
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
        let prepared = sessions.take(prepared, &mut registry).unwrap_or_else(|| {
            panic!(
                "experience '{experience_id}' prepared data must match the authorized transaction"
            )
        });
        let default_character = prepared.report.starting_character.clone();
        builder.build(
            activation,
            *scope,
            prepared.content,
            default_character.as_str(),
        );
    }
}

/// Constructs the live session for a prepared world: the simulation world and
/// player under the activation's session scope, the input-owner binding, and
/// the session-root publication through [`ActiveGameplaySession`].
#[derive(SystemParam)]
pub struct PlatformerSessionBuilder<'w, 's> {
    commands: Commands<'w, 's>,
    editable_abilities: Res<'w, ambition_dev_tools::dev_tools::EditableAbilitySet>,
    tuning: Res<'w, ambition_engine_core::ActiveMovementTuning>,
    asset_server: Res<'w, AssetServer>,
    character_catalog: Res<'w, ambition_characters::actor::character_catalog::CharacterCatalog>,
    character_roster: Res<'w, ambition_actors::features::CharacterRoster>,
    boss_catalog: Res<'w, ambition_actors::boss_encounter::BossCatalog>,
    placement_lowering: Res<'w, ambition_actors::world::placements::PlacementLoweringRegistry>,
    content_staging: Res<'w, ambition_actors::features::RoomContentStagingRegistry>,
    construction_recipes: Res<'w, ambition_actors::construction::ActorConstructionRegistry>,
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
        prepared_content: PreparedContent,
        default_character_id: &str,
    ) -> SessionBuildResult {
        let live_world: PlatformerSessionWorld = prepared_content.source().instantiate_live();
        let prepared_identity: PreparedContentIdentity = prepared_content.identity();
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
                tuning: &self.tuning,
                starting_character: &live_world.starting_character,
                character_catalog: &self.character_catalog,
                character_roster: &self.character_roster,
                placement_lowering: &self.placement_lowering,
                content_staging: &self.content_staging,
                // Activation is the one place that holds the exact prepared
                // definition, so it is the one place a construction plan can
                // state a REAL activation generation rather than defaulting.
                construction: ambition_actors::features::ActorConstructionContext::new(
                    &self.construction_recipes,
                    prepared_identity.epoch,
                ),
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
            .spawn_world_for(
                &mut self.commands,
                activation,
                scope,
                // The bare epoch rides alongside the identity that defines it,
                // from this single value, so layers below `ambition_runtime`
                // (construction planning) can read the activation generation
                // without naming prepared-content identity.
                (
                    live_world,
                    prepared_content,
                    prepared_identity,
                    prepared_identity.epoch,
                ),
            )
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
        .install(&mut app, || -> PreparedPlatformerSource {
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

    const CHARACTER_A: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "alpha": (
                display_name: "Alpha", spritesheet: "alpha.png", manifest: "alpha.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    const CHARACTER_B: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Float) },
        characters: {
            "beta": (
                display_name: "Beta", spritesheet: "beta.png", manifest: "beta.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    fn fixture_source(width: f32) -> PreparedPlatformerSource {
        let room = ambition_world::rooms::RoomSpec::new(
            "same-room",
            ambition_engine_core::World::new(
                "same-room",
                ambition_engine_core::Vec2::new(width, 128.0),
                ambition_engine_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        let room_set =
            ambition_world::rooms::RoomSet::from_parts("same-room", vec![room], Vec::new());
        PreparedPlatformerSource::new(
            "same-provider",
            room_set.clone(),
            ambition_engine_core::RoomGeometry(room_set.active_world().clone()),
            ambition_world::rooms::ActiveRoomMetadata(room_set.active_spec().metadata.clone()),
            ambition_actors::avatar::StartingCharacter::new("alpha"),
            ambition_actors::ldtk_world::LdtkRuntimeIndex::default(),
        )
    }

    fn isolated_room_fixture_source(room_id: &str) -> PreparedPlatformerSource {
        let room = ambition_world::rooms::RoomSpec::new(
            room_id,
            ambition_engine_core::World::new(
                room_id,
                ambition_engine_core::Vec2::new(128.0, 128.0),
                ambition_engine_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        let room_set = ambition_world::rooms::RoomSet::from_parts(room_id, vec![room], Vec::new());
        PreparedPlatformerSource::new(
            "same-provider",
            room_set.clone(),
            ambition_engine_core::RoomGeometry(room_set.active_world().clone()),
            ambition_world::rooms::ActiveRoomMetadata(room_set.active_spec().metadata.clone()),
            ambition_actors::avatar::StartingCharacter::new("alpha"),
            ambition_actors::ldtk_world::LdtkRuntimeIndex::default(),
        )
    }

    fn two_room_fixture_source(active_room: &str) -> PreparedPlatformerSource {
        let first = ambition_world::rooms::RoomSpec::new(
            "same-room",
            ambition_engine_core::World::new(
                "same-room",
                ambition_engine_core::Vec2::new(128.0, 128.0),
                ambition_engine_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        let second = ambition_world::rooms::RoomSpec::new(
            "second-room",
            ambition_engine_core::World::new(
                "second-room",
                ambition_engine_core::Vec2::new(160.0, 128.0),
                ambition_engine_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        let mut room_set = ambition_world::rooms::RoomSet::from_parts(
            "same-room",
            vec![first, second],
            Vec::new(),
        );
        room_set.active = room_set.room_index_by_id(active_room).unwrap();
        PreparedPlatformerSource::new(
            "same-provider",
            room_set.clone(),
            ambition_engine_core::RoomGeometry(room_set.active_world().clone()),
            ambition_world::rooms::ActiveRoomMetadata(room_set.active_spec().metadata.clone()),
            ambition_actors::avatar::StartingCharacter::new("alpha"),
            ambition_actors::ldtk_world::LdtkRuntimeIndex::default(),
        )
    }

    fn character_registry(
        reverse: bool,
        beta_ron: &str,
    ) -> ambition_characters::actor::character_catalog::CharacterCatalogRegistry {
        use ambition_characters::actor::character_catalog::{
            CharacterCatalogFragment, CharacterCatalogRegistry,
        };
        let a =
            CharacterCatalogFragment::from_ron("provider-a", Some("alpha"), CHARACTER_A).unwrap();
        let b = CharacterCatalogFragment::from_ron("provider-b", Some("beta"), beta_ron).unwrap();
        let mut registry = CharacterCatalogRegistry::default();
        if reverse {
            registry.register(b).unwrap();
            registry.register(a).unwrap();
        } else {
            registry.register(a).unwrap();
            registry.register(b).unwrap();
        }
        registry
    }

    fn staging_registry(reverse: bool) -> ambition_actors::features::RoomContentStagingRegistry {
        let mut registry = ambition_actors::features::RoomContentStagingRegistry::default();
        let register_a = |registry: &mut ambition_actors::features::RoomContentStagingRegistry| {
            registry
                .register(
                    "same-room",
                    "provider-a",
                    "fixture-a",
                    "fixture-a.v1",
                    |_| Vec::new(),
                )
                .unwrap();
        };
        let register_b = |registry: &mut ambition_actors::features::RoomContentStagingRegistry| {
            registry
                .register(
                    "same-room",
                    "provider-b",
                    "fixture-b",
                    "fixture-b.v1",
                    |_| Vec::new(),
                )
                .unwrap();
        };
        if reverse {
            register_b(&mut registry);
            register_a(&mut registry);
        } else {
            register_a(&mut registry);
            register_b(&mut registry);
        }
        registry
    }

    fn fixture_content(
        source: PreparedPlatformerSource,
        characters: &ambition_characters::actor::character_catalog::CharacterCatalogRegistry,
        staging: &ambition_actors::features::RoomContentStagingRegistry,
    ) -> PreparedContent {
        fixture_content_with_recipes(source, characters, staging, None)
    }

    fn fixture_content_with_recipes(
        source: PreparedPlatformerSource,
        characters: &ambition_characters::actor::character_catalog::CharacterCatalogRegistry,
        staging: &ambition_actors::features::RoomContentStagingRegistry,
        construction_recipes: Option<String>,
    ) -> PreparedContent {
        let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
        let snapshot_schema =
            ambition_runtime::rollback::RollbackRegistry::default().schema_fingerprint();
        let mut epochs = ContentEpochSequence::default();
        prepare_platformer_content(
            source,
            &authored,
            Some(characters),
            None,
            Some(staging),
            construction_recipes,
            snapshot_schema,
            &mut epochs,
        )
        .unwrap()
    }

    #[test]
    fn prepared_content_is_order_independent_and_dump_stable() {
        let first_characters = character_registry(false, CHARACTER_B);
        let second_characters = character_registry(true, CHARACTER_B);
        let first_staging = staging_registry(false);
        let second_staging = staging_registry(true);
        let first = fixture_content(fixture_source(128.0), &first_characters, &first_staging);
        let second = fixture_content(fixture_source(128.0), &second_characters, &second_staging);
        assert_eq!(first.fingerprint(), second.fingerprint());
        assert_eq!(first.deterministic_dump(), second.deterministic_dump());
    }

    #[test]
    fn prepared_content_detects_geometry_and_character_action_changes() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);
        let baseline = fixture_content(fixture_source(128.0), &characters, &staging);
        let changed_geometry = fixture_content(fixture_source(192.0), &characters, &staging);
        assert_ne!(baseline.fingerprint(), changed_geometry.fingerprint());

        let changed_character_ron = CHARACTER_B.replace("move_style: Float", "move_style: Slither");
        let changed_characters = character_registry(false, &changed_character_ron);
        let changed_character =
            fixture_content(fixture_source(128.0), &changed_characters, &staging);
        assert_ne!(baseline.fingerprint(), changed_character.fingerprint());
    }

    /// The construction recipe table decides how authoritative entities are
    /// built, so a change to it is a change to the content. This was DOCUMENTED
    /// as contributing to the fingerprint long before it did — `prepare_platformer_content`
    /// did not take the registry at all.
    #[test]
    fn a_construction_recipe_schema_change_moves_the_fingerprint() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);

        let baseline = fixture_content_with_recipes(
            fixture_source(128.0),
            &characters,
            &staging,
            Some(construction_dump("actor-construction-v1")),
        );
        let bumped_schema = fixture_content_with_recipes(
            fixture_source(128.0),
            &characters,
            &staging,
            Some(construction_dump("actor-construction-v2")),
        );
        assert_ne!(
            baseline.fingerprint(),
            bumped_schema.fingerprint(),
            "a recipe schema bump is a content change"
        );

        let absent =
            fixture_content_with_recipes(fixture_source(128.0), &characters, &staging, None);
        assert_ne!(
            baseline.fingerprint(),
            absent.fingerprint(),
            "having a recipe table at all differs from having none"
        );
    }

    /// ...but registration ORDER does not, because the registry is ordered
    /// storage. Both halves matter: a fingerprint that ignores real changes is
    /// useless, and one that reacts to plugin insertion order is unusable.
    #[test]
    fn construction_registration_order_does_not_move_the_fingerprint() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);

        let forward = fixture_content_with_recipes(
            fixture_source(128.0),
            &characters,
            &staging,
            Some(construction_dump_ordered(false)),
        );
        let reversed = fixture_content_with_recipes(
            fixture_source(128.0),
            &characters,
            &staging,
            Some(construction_dump_ordered(true)),
        );
        assert_eq!(forward.fingerprint(), reversed.fingerprint());
    }

    /// A relation's wiring behaviour can change while its kind and owner stay
    /// the same. The schema id is what makes that visible to content identity,
    /// so it must reach the fingerprint just as a recipe's does.
    #[test]
    fn a_relation_schema_change_moves_the_fingerprint() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);

        let baseline = fixture_content_with_recipes(
            fixture_source(128.0),
            &characters,
            &staging,
            Some(relation_dump("v1")),
        );
        let bumped = fixture_content_with_recipes(
            fixture_source(128.0),
            &characters,
            &staging,
            Some(relation_dump("v2")),
        );
        assert_ne!(baseline.fingerprint(), bumped.fingerprint());
    }

    fn relation_dump(schema: &str) -> String {
        let mut registry = ambition_actors::construction::ActorConstructionRegistry::default();
        registry
            .try_register_relation(
                ambition_actors::construction::relation_grudge(),
                "ambition_actors",
                "aggression",
                schema,
                ambition_actors::construction::grudge_ops_for_tests(),
            )
            .unwrap();
        registry.deterministic_dump()
    }

    /// A real registry, dumped — the same value the app path contributes.
    fn construction_dump(schema: &str) -> String {
        let mut registry = ambition_actors::construction::ActorConstructionRegistry::default();
        registry
            .try_register_recipe(
                ambition_actors::construction::recipe_staged_actor(),
                "ambition_actors",
                "content-staging",
                schema,
            )
            .unwrap();
        registry.deterministic_dump()
    }

    fn construction_dump_ordered(reverse: bool) -> String {
        let mut registry = ambition_actors::construction::ActorConstructionRegistry::default();
        let ids = [
            ambition_actors::construction::recipe_staged_actor(),
            ambition_actors::construction::recipe_summoned_minion(),
        ];
        let ids: Vec<_> = if reverse {
            ids.into_iter().rev().collect()
        } else {
            ids.into_iter().collect()
        };
        for id in ids {
            registry
                .try_register_recipe(id, "ambition_actors", "src", "v1")
                .unwrap();
        }
        registry.deterministic_dump()
    }

    #[test]
    fn sequential_preparations_share_definition_identity_but_not_epoch() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);
        let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
        let snapshot_schema =
            ambition_runtime::rollback::RollbackRegistry::default().schema_fingerprint();
        let mut epochs = ContentEpochSequence::default();
        let first = prepare_platformer_content(
            fixture_source(128.0),
            &authored,
            Some(&characters),
            None,
            Some(&staging),
            None,
            snapshot_schema,
            &mut epochs,
        )
        .unwrap();
        let second = prepare_platformer_content(
            fixture_source(128.0),
            &authored,
            Some(&characters),
            None,
            Some(&staging),
            None,
            snapshot_schema,
            &mut epochs,
        )
        .unwrap();

        assert_eq!(first.fingerprint(), second.fingerprint());
        assert_ne!(first.epoch(), second.epoch());
        assert_eq!(first.epoch(), ambition_runtime::ContentEpoch(1));
        assert_eq!(second.epoch(), ambition_runtime::ContentEpoch(2));
    }

    #[test]
    fn reload_candidate_is_detached_and_equivalent_reload_preserves_epoch() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);
        let active = fixture_content(fixture_source(128.0), &characters, &staging);
        let schema = active.snapshot_schema();

        let equivalent =
            prepare_world_replacement_candidate(&active, fixture_source(128.0), schema).unwrap();
        assert_eq!(equivalent.fingerprint(), active.fingerprint());
        assert_eq!(equivalent.epoch(), active.epoch());

        let changed =
            prepare_world_replacement_candidate(&active, fixture_source(192.0), schema).unwrap();
        assert_ne!(changed.fingerprint(), active.fingerprint());
        assert_eq!(
            changed.epoch(),
            active.epoch(),
            "candidate is not committed yet"
        );
        assert_eq!(active.source().geometry().0.size.x, 128.0);

        let committed = changed.with_epoch(ambition_runtime::ContentEpoch(9));
        assert_eq!(committed.epoch(), ambition_runtime::ContentEpoch(9));
        assert_eq!(active.epoch(), ambition_runtime::ContentEpoch(1));
    }

    #[test]
    fn replacement_ignores_the_live_active_room_cursor() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);
        let active = fixture_content(two_room_fixture_source("same-room"), &characters, &staging);
        let live_room_candidate = two_room_fixture_source("second-room");
        let candidate = prepare_world_replacement_candidate(
            &active,
            live_room_candidate,
            active.snapshot_schema(),
        )
        .unwrap();

        assert_eq!(candidate.fingerprint(), active.fingerprint());
        assert_eq!(candidate.epoch(), active.epoch());
        assert_eq!(candidate.source().active_room_id(), "same-room");
    }

    #[test]
    fn failed_reload_candidate_leaves_active_content_unchanged() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);
        let active = fixture_content(two_room_fixture_source("same-room"), &characters, &staging);
        let before = active.identity();

        let error = prepare_world_replacement_candidate(
            &active,
            isolated_room_fixture_source("second-room"),
            active.snapshot_schema(),
        )
        .unwrap_err();

        assert_eq!(error.section, "world.definition-active-room");
        assert_eq!(active.identity(), before);
        assert_eq!(active.source().active_room_id(), "same-room");
    }

    #[test]
    fn ordinary_room_movement_does_not_change_prepared_identity() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);
        let content = fixture_content(two_room_fixture_source("same-room"), &characters, &staging);
        let before = content.identity();
        let mut live = content.source().instantiate_live();
        live.room_set.active = live.room_set.room_index_by_id("second-room").unwrap();

        assert_eq!(live.active_room_id(), "second-room");
        assert_eq!(content.identity(), before);
        assert_eq!(content.source().active_room_id(), "same-room");
    }

    #[test]
    fn mutable_live_requests_do_not_change_prepared_identity() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);
        let content = fixture_content(fixture_source(128.0), &characters, &staging);
        let before = content.identity();
        let mut live = content.source().instantiate_live();
        live.requests.room_music.desired_track = Some("runtime-track".to_owned());
        live.requests.encounter_music.priority_track = Some("runtime-boss".to_owned());
        assert_eq!(content.identity(), before);
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
