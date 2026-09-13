//! The shared provider lifecycle: preparation, prepared-session ownership,
//! and activation into the live session world.
//!
//! Every experience registered through
//! [`PlatformerExperienceAuthoring::install`](crate::authoring::PlatformerExperienceAuthoring::install)
//! shares these systems; the provider contributes only its session-world
//! source. The answers to the lifecycle questions live here:
//!
//! - What does a provider prepare? A [`PreparedPlatformerSource`], validated
//!   against its [`AuthoredCatalogFragments`](crate::authoring::AuthoredCatalogFragments)
//!   into a [`PlatformerPreparationReport`].
//! - What identity proves activation matches preparation? The
//!   [`PreparedSessionIdentity`] published through the shell's
//!   [`PreparedSessionRegistry`].
//! - Who owns the prepared value? [`PreparedPlatformerSessions`], keyed by
//!   the load transaction.
//! - When is live session state created? Preparation owns the validated
//!   world value; activation consumes it by exact identity and creates the live
//!   session root and scoped entities through [`PlatformerSessionBuilder::build`].

use std::collections::BTreeMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_game_shell::{
    ActiveGameplaySession, ActiveShellExperience, GameplayInputOwner, GameplaySessionEvent,
    GameplaySessionSet, PreparedSessionIdentity, PreparedSessionRegistry, ProviderLoadTransaction,
    ShellEvent, PREPARE_ADAPTIVE_WORK_ID, PREPARE_CATALOGS_WORK_ID, PREPARE_DEFAULTS_WORK_ID,
    PREPARE_FIRST_ROOM_ART_WORK_ID, PREPARE_MUSIC_WORK_ID, PREPARE_PACKED_SFX_WORK_ID,
    PREPARE_SESSION_WORK_ID, PREPARE_SFX_WORK_ID, PREPARE_SPRITES_WORK_ID, PREPARE_WORLD_WORK_ID,
};
use ambition_load::AmbitionLoadSet;
use ambition_platformer2d_runtime::{
    ContentDiagnostic, ContentEpochSequence, ContentOwner, PlatformerSessionWorld, PreparedContent,
    PreparedContentBuilder, PreparedContentIdentity, PreparedPlatformerSource,
};
use ambition_platformer2d_shared_tangle::lifecycle::{SessionScopeId, SessionSpawnScope};

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
        app.add_plugins(ambition_platformer2d_actor_monolith::session::SessionTeardownPlugin);
        app.init_resource::<PlatformerStreamingReadiness>()
            .init_resource::<PreparedPlatformerSessions>()
            .init_resource::<ContentEpochSequence>()
            .init_resource::<
                ambition_platformer2d_shared_tangle::gameplay_presentation::ActiveGameplayPresentationProfiles,
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
                ambition_platformer2d_shared_tangle::gameplay_presentation::ActiveHudDeclaration,
            >()
            .init_resource::<
                ambition_platformer2d_shared_tangle::gameplay_presentation::ActiveDefensePresentationPolicy,
            >()
            .init_resource::<
                ambition_platformer2d_shared_tangle::gameplay_presentation::HudReadouts,
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
                            ambition_platformer2d_shared_tangle::gameplay_presentation::GameplayPresentationSet,
                        ),
                    // The HUD declaration follows the route on the same
                    // schedule and for the same reason: a switch must not show
                    // one frame of the previous game's readouts.
                    crate::authoring::select_active_hud_declaration
                        .after(activate_prepared_platformer_sessions)
                        .before(
                            ambition_platformer2d_shared_tangle::gameplay_presentation::GameplayPresentationSet,
                        ),
                    // Defense presentation is route-owned too: a multi-game host may
                    // keep Mary-O and Smash installed simultaneously, so build-order
                    // insertion cannot be the authority.
                    //
                    // ⛔⛔ AND IT MAY NOT ALSO ASK TO PRECEDE `PresentationVisualSync`.
                    // That set runs EARLIER IN THE FRAME than session activation —
                    // `PresentationVisualSync → RoomTransitionCoverSet → Observe →
                    // Activity → … → Bridge → Providers` is the host's own chain — so
                    // the extra edge closed a fifteen-hop cycle and the whole `Update`
                    // schedule failed to build. The cue systems read the policy
                    // published on the previous tick, which is the same one-frame
                    // relationship both siblings above have.
                    crate::authoring::select_active_defense_presentation
                        .after(activate_prepared_platformer_sessions)
                        .before(
                            ambition_platformer2d_shared_tangle::gameplay_presentation::GameplayPresentationSet,
                        ),
                    // After the profile SELECTION rather than before the layout resolve: it
                    // reads what that system just published, and the camera resolve it feeds
                    // runs on its own schedule.
                    crate::authoring::publish_active_camera_feel
                        .after(crate::authoring::select_active_presentation_profiles),
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
            banks
                .has_ids(pending.provider_id.as_str())
                .then_some(load_id.clone())
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
    snapshot_registry: Option<Res<'w, ambition_platformer2d_runtime::rollback::RollbackRegistry>>,
    placement_lowering: Option<
        Res<'w, ambition_platformer2d_actor_monolith::world::placements::PlacementLoweringRegistry>,
    >,
    content_staging:
        Option<Res<'w, ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry>>,
    //  This brings the struct to Bevy's 16-parameter `SystemParam` ceiling.
    // The next field added here must bundle something first.
    //
    // ⭐ AND THE NEXT FIELD DID ARRIVE, so this is the bundle that comment asked
    // for: the construction schema catalog, and the identity of the authored
    // content PACK this composition selected. Both are inputs to one thing — the
    // prepared content's fingerprint — so pairing them is not an arbitrary
    // grouping made to fit a ceiling.
    content_inputs: (
        Option<
            Res<'w, ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog>,
        >,
        Option<Res<'w, ambition_platformer2d_runtime::SelectedContentIdentity>>,
        // ⛔ THE CANDIDATE IDENTITY OF ONE TRANSACTION, not of this App. See
        // `PendingGenerationInputs`: a pending generation used to overwrite the
        // App-wide selection to make preparation see it, which handed the
        // candidate's stamp to every UNRELATED preparation running in the same
        // window.
        Option<Res<'w, ambition_platformer2d_runtime::PendingGenerationInputs>>,
        // ⛔⛤ **THE THREE MECHANICAL REGISTRIES SESSION CONSTRUCTION CONSUMES
        // AND NOTHING FINGERPRINTED**, bundled here for the same reason the
        // three above are: they are all inputs to ONE thing, the prepared
        // content's identity. The prepared cast (`max_health`, motion model,
        // abilities, contact damage, brain policy), the authored sheets (body
        // metrics and authored attack geometry), and the boss catalog
        // (behaviours, encounters, fallbacks). See `MechanicalRegistries`.
        Option<Res<'w, ambition_characters::prepared::StagedCharacterOverrides>>,
        Option<Res<'w, ambition_sprite_sheet::character::sheets::AuthoredSheets>>,
        Option<Res<'w, ambition_boss_encounter::BossCatalog>>,
        // ⛔ THE FOLD, not the pre-fold source above: the fingerprint is over
        // `StagedCharacterOverrides` (lossless) while session construction reads
        // the published registry, so FREEZING has to capture the value the
        // builder will actually use. See `SessionMechanics`.
        Option<Res<'w, ambition_characters::prepared::PreparedCharacterRegistry>>,
        // ⛔⛤ **THE IMMUTABLE DEVELOPER CONSTRUCTION KNOBS**, in this bundle for
        // the same reason as the rest: they are inputs to ONE thing, the
        // prepared content's identity. Both change the authoritative ROSTER a
        // room admits, and neither reached any fingerprint — see
        // `MechanicalRegistries::developer_construction`.
        Option<Res<'w, ambition_characters::brain::AuthoredBrainOverride>>,
        Option<Res<'w, ambition_characters::actor::AuthoredPopulationCap>>,
    ),
    epochs: ResMut<'w, ContentEpochSequence>,
    audio_catalogs: Res<'w, ambition_audio::catalog::AudioCatalogRegistry>,
    #[cfg(feature = "audio")]
    adaptive_catalogs: Option<Res<'w, ambition_audio::music::AdaptiveMusicCatalogRegistry>>,
    sfx_banks: Option<Res<'w, ambition_audio::catalog::SfxBankRegistry>>,
    game_assets: Option<Res<'w, ambition_sprite_sheet::game_assets::GameAssets>>,
    registry: ResMut<'w, PreparedSessionRegistry>,
    // Paired because a `SystemParam` stops at sixteen: the store, and whether
    // a HOST owns `prepare-first-room-art` (see `FirstRoomArtContributor`).
    sessions: (
        ResMut<'w, PreparedPlatformerSessions>,
        Option<Res<'w, FirstRoomArtContributor>>,
    ),
    streaming: ResMut<'w, PlatformerStreamingReadiness>,
    commands: MessageWriter<'w, ambition_load::LoadCommand>,
}

impl PlatformerPreparation<'_> {
    /// This transaction's content identity — see [`content_identity_for`].
    ///
    /// ⛔⛤ **A METHOD RATHER THAN THREE ARGUMENTS AT THE CALL SITE, AND A POISON
    /// IS WHY.** Poisoning the free function fails its arm; poisoning the CALL
    /// SITE — passing `None` for the claim and `""` for the load — stayed green
    /// across 429 tests, because witnessing that hop means running a real
    /// preparation and comparing the resulting fingerprint. Nothing here can
    /// witness it, so the honest fix is to leave nothing to get wrong: the
    /// inputs are no longer spellable at the call site, and the only remaining
    /// mistake is not calling this at all.
    fn content_identity_for(&self, transaction: &ProviderLoadTransaction) -> Option<String> {
        content_identity_for(
            self.content_inputs.1.as_deref(),
            self.content_inputs.2.as_deref(),
            transaction.barrier.load_id.as_str(),
        )
    }

    /// The cast this transaction freezes. See [`candidate_cast_for`].
    fn candidate_cast_for(
        &self,
        transaction: &ProviderLoadTransaction,
    ) -> Option<ambition_characters::prepared::PreparedCharacterRegistry> {
        candidate_cast_for(
            self.content_inputs.6.as_deref(),
            self.content_inputs.2.as_deref(),
            transaction.barrier.load_id.as_str(),
        )
    }

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
            PREPARE_FIRST_ROOM_ART_WORK_ID,
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
                .sheet(authored.starting_character.as_str())
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

        // So `AMBITION_START_CHARACTER=<anything but the default>` failed this work item,
        // preparation returned before publishing anything, and the session NEVER ACTIVATED — no
        // world, no body, no error a player sees.
        //
        // Two sites asked one question; one was corrected and the other kept the conflation for ten
        // days — and the corrected one runs at PREPARE_SESSION, downstream of this early `return`,
        // so it could never be reached by the case it was written for.
        //
        // The two facts, and who owns each now:
        // * the provider's DEFAULT must exist — `AuthoredCatalogFragments::validate`,
        //   at PREPARE_CATALOGS above;
        // * the session's SELECTION must RESOLVE — `prepare_platformer_content`,
        //   at PREPARE_SESSION below. ONE owner, deliberately not re-asked here.
        //
        // What is left is the question this work item genuinely owns: the prepared
        // world's audio provider is the one this experience authored fragments for.
        if source.catalogs().audio_provider.as_str() != authored.audio_provider.as_str() {
            self.fail(
                transaction,
                PREPARE_DEFAULTS_WORK_ID,
                ambition_load::LoadFailure::new(
                    "Provider defaults do not match the prepared world",
                    format!(
                        "expected audio provider '{}', got '{}'",
                        authored.audio_provider,
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
            .map(ambition_platformer2d_runtime::rollback::RollbackRegistry::schema_fingerprint)
            .unwrap_or_else(|| {
                ambition_platformer2d_runtime::rollback::RollbackRegistry::default()
                    .schema_fingerprint()
            });
        // ⛔ THE MECHANICAL MATERIAL IS RENDERED FIRST AND ITS FAILURE IS A
        // PREPARATION FAILURE, routed through the SAME arm as every other
        // diagnostic below — see `canonical`. A dump that cannot be rendered has
        // no identity, so this refuses rather than hashing an error string.
        let content = match canonical(
            self.content_inputs.3.as_deref(),
            ambition_characters::prepared::StagedCharacterOverrides::deterministic_dump,
            "characters.definitions",
        )
        .and_then(|prepared_cast| {
            Ok(MechanicalRegistries {
                construction_recipes: self
                    .content_inputs
                    .0
                    .as_deref()
                    .map(ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog::deterministic_dump),
                content_pack: self.content_identity_for(transaction),
                prepared_cast,
                // ⚠ INFALLIBLE: the sheet dump is the provider's own declaration
                // TEXT, already retained, with no serialization step to fail.
                authored_sheets: self
                    .content_inputs
                    .4
                    .as_deref()
                    .map(ambition_sprite_sheet::character::sheets::AuthoredSheets::deterministic_dump),
                boss_catalog: canonical(
                    self.content_inputs.5.as_deref(),
                    ambition_boss_encounter::BossCatalog::deterministic_dump,
                    "boss.catalog",
                )?,
                developer_construction: developer_construction_dump(
                    self.content_inputs.7.as_deref(),
                    self.content_inputs.8.as_deref(),
                ),
            })
        })
        .and_then(|mechanical| {
            prepare_platformer_content(
                source,
                &authored,
                self.character_catalog_registry.as_deref(),
                self.placement_lowering.as_deref(),
                self.content_staging.as_deref(),
                mechanical,
                snapshot_schema,
                &mut self.epochs,
            )
        }) {
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
        // ⚠ TAKEN BEFORE THE PUBLISH BORROW, not for style: `publish` borrows
        // `self.registry` mutably.
        let frozen_cast = self.candidate_cast_for(transaction);
        let identity = self.sessions.0.publish(
            transaction,
            PreparedPlatformerSession {
                content,
                report,
                // ⛔ FROZEN HERE, in the same system that took the identity, so
                // the two cannot describe different worlds.
                mechanical: SessionMechanics {
                    // ⛔ THE TRANSACTION'S OWN CANDIDATE, never the App's
                    // published registry — see `candidate_cast_for`.
                    characters: frozen_cast,
                    sheets: self
                        .content_inputs
                        .4
                        .as_deref()
                        .cloned()
                        .unwrap_or_default(),
                    bosses: self
                        .content_inputs
                        .5
                        .as_deref()
                        .cloned()
                        .unwrap_or_default(),
                },
            },
            &mut self.registry,
        )?;
        self.complete(transaction, PREPARE_SESSION_WORK_ID);
        // The first room's art is the host's question: it has the sprite
        // catalog, the asset server and the resolved quality, and it reads the
        // published record to answer it (`PreparedPlatformerSessions::published`).
        // A composition without that contributor has nobody to answer, so the
        // item completes here rather than holding activation forever.
        if self.sessions.1.is_none() {
            self.complete(transaction, PREPARE_FIRST_ROOM_ART_WORK_ID);
        }
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

    // Include deterministic area membership and bounds when an LDtk index exists.
    // Mutable revision/sync cursors are excluded; RON-authored games have no index.
    #[cfg(feature = "ldtk")]
    let area_rows = |area_id: &str| -> (Vec<String>, String) {
        let index = source.installed_ldtk_index();
        let iids = index.map(|i| i.level_iids_for(area_id)).unwrap_or_default();
        let bounds = index.and_then(|i| i.area_bounds(area_id)).map_or_else(
            || "\t-".to_string(),
            |b| format!("\t{},{},{},{}", b.min_x, b.min_y, b.max_x, b.max_y),
        );
        (iids, bounds)
    };
    #[cfg(not(feature = "ldtk"))]
    let area_rows = |_area_id: &str| -> (Vec<String>, String) { (Vec::new(), "\t-".to_string()) };
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
        let (mut level_iids, bounds) = area_rows(area_id);
        level_iids.sort();
        runtime_index.push_str("area\t");
        runtime_index.push_str(area_id);
        runtime_index.push('\t');
        runtime_index.push_str(&level_iids.join(","));
        runtime_index.push_str(&bounds);
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
    snapshot_schema: ambition_platformer2d_runtime::SnapshotSchemaFingerprint,
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
        .get_resource::<ambition_platformer2d_actor_monolith::world::placements::PlacementLoweringRegistry>()
        .cloned();
    let content_staging = app
        .world()
        .get_resource::<ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry>(
        )
        .cloned();
    let construction_recipes = app
        .world()
        .get_resource::<ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog>()
        .map(|catalog| catalog.deterministic_dump());
    let content_pack = app
        .world()
        .get_resource::<ambition_platformer2d_runtime::SelectedContentIdentity>()
        .map(|identity| identity.0.clone());
    // ⛔⛤ THE THREE MECHANICAL REGISTRIES SESSION CONSTRUCTION CONSUMES AND
    // NOTHING FINGERPRINTED. See `MechanicalRegistries`.
    let prepared_cast = ambition_characters::prepared::staged_cast_declaration(app.world())
        .transpose()
        .map_err(|error| ContentDiagnostic::new("characters.definitions", error))?;
    // ⚠ INFALLIBLE: the sheet dump is the provider's own declaration TEXT,
    // already retained, with no serialization step to fail.
    let authored_sheets = app
        .world()
        .get_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>()
        .map(ambition_sprite_sheet::character::sheets::AuthoredSheets::deterministic_dump);
    let boss_catalog = canonical(
        app.world().get_resource::<ambition_boss_encounter::BossCatalog>(),
        ambition_boss_encounter::BossCatalog::deterministic_dump,
        "boss.catalog",
    )?;
    let snapshot_schema = app
        .world()
        .get_resource::<ambition_platformer2d_runtime::rollback::RollbackRegistry>()
        .map(ambition_platformer2d_runtime::rollback::RollbackRegistry::schema_fingerprint)
        .unwrap_or_else(|| {
            ambition_platformer2d_runtime::rollback::RollbackRegistry::default()
                .schema_fingerprint()
        });
    // ⚠ TAKEN BEFORE THE EPOCH BORROW: `resource_mut` holds the world mutably.
    let developer_construction = developer_construction_dump(
        app.world()
            .get_resource::<ambition_characters::brain::AuthoredBrainOverride>(),
        app.world()
            .get_resource::<ambition_characters::actor::AuthoredPopulationCap>(),
    );
    app.init_resource::<ContentEpochSequence>();
    let mut epochs = app.world_mut().resource_mut::<ContentEpochSequence>();
    prepare_platformer_content(
        source,
        authored,
        character_registry.as_ref(),
        placement_lowering.as_ref(),
        content_staging.as_ref(),
        MechanicalRegistries {
            construction_recipes,
            content_pack,
            prepared_cast,
            authored_sheets,
            boss_catalog,
            developer_construction,
        },
        snapshot_schema,
        &mut epochs,
    )
}

/// Which content identity does THIS preparation fingerprint against?
///
/// ⛔⛤ **THE APP'S SELECTION WAS THE ONLY ANSWER, AND A HOT RELOAD HAD TO
/// OVERWRITE IT.** A pending generation needs preparation to see the CANDIDATE,
/// so it replaced `SelectedContentIdentity` App-wide — and every UNRELATED route
/// preparation running in that window inherited a stamp for content it never
/// prepared. The identity is exactly what the rollback timeline contract
/// compares, so a stranger's session carried a generation identity that was not
/// its own.
///
/// ⛔ THE CLAIM MUST NAME A TRANSACTION, and a claim naming a DIFFERENT one is
/// not a fallback — it is a stranger's, and the active selection is the right
/// answer. Returning the pending identity whenever one exists would restore the
/// defect with an extra step.
///
/// ⚠ FACTORED OUT SO IT HAS A WITNESS. Inline, this resolution was reachable
/// only by running a real preparation, and 868 tests stayed green with the claim
/// ignored entirely — the consumer half of the fix had no arm at all.
pub(crate) fn content_identity_for(
    active: Option<&ambition_platformer2d_runtime::SelectedContentIdentity>,
    pending: Option<&ambition_platformer2d_runtime::PendingGenerationInputs>,
    load_id: &str,
) -> Option<String> {
    pending
        .and_then(|claim| claim.identity_for(load_id))
        .map(str::to_string)
        .or_else(|| active.map(|identity| identity.0.clone()))
}

/// The cast THIS transaction must be built from.
///
/// ⛔⛤ **THE FREEZE USED TO READ THE APP'S PUBLISHED REGISTRY, AND ON THE ONE
/// ROAD THAT CHANGES THE CAST THAT IS THE WRONG GENERATION.** A cast-changing
/// reload admits the N+1 registry at REQUEST time and withholds it from the App
/// until the commit boundary — correctly, so a candidate never becomes live
/// before its transaction commits. Preparation running inside that window
/// therefore saw N. The session's identity said N+1 and its fighters were N's:
/// the exact mixed-generation class the prepared-generation work exists to
/// remove.
///
/// ⚠ **THE FALLBACK IS NOT A GUESS.** No claim, or a stranger's claim, means
/// this preparation is not inside a pending generation at all, and the App's
/// published registry is then its own transaction's value rather than somebody
/// else's candidate. A claim that IS ours and carries no cast means the same
/// thing for a different reason: the candidate does not change the cast, so N is
/// N+1's cast. Both are stated rather than inferred — see
/// `PendingGenerationInputs::characters_for`.
pub(crate) fn candidate_cast_for(
    active: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    pending: Option<&ambition_platformer2d_runtime::PendingGenerationInputs>,
    load_id: &str,
) -> Option<ambition_characters::prepared::PreparedCharacterRegistry> {
    match pending.and_then(|claim| claim.characters_for(load_id)) {
        // Our transaction, and it publishes a new cast: build from THAT.
        Some(Some(candidate)) => Some(candidate.clone()),
        // Our transaction, publishing no new cast; or no claim of ours at all.
        Some(None) | None => active.cloned(),
    }
}

/// The MECHANICAL App authorities that session construction consumes, as
/// canonical bytes.
///
/// ⛔⛤ **THIS EXISTS BECAUSE THE LIST WAS A PARAMETER PAIR AND THE LIST WAS
/// INCOMPLETE.** `PreparedContentIdentity` bound the construction recipes and
/// the content pack and stopped there, while `PlatformerSessionBuilder` went on
/// to build the session out of THREE more App registries that nothing
/// fingerprinted: the prepared cast, the authored sheets, and the boss catalog.
/// Two compositions could therefore differ in a character's `max_health`, in the
/// body geometry an authored sheet declares, or in how a boss fights, and share
/// one identity — which is exactly the identity the rollback timeline contract
/// compares to decide whether a snapshot may be restored.
///
/// ⭐⭐ **A NAMED STRUCT RATHER THAN MORE PARAMETERS, BECAUSE THE FAILURE MODE
/// IS OMISSION.** A twelve-argument call is where the next mechanical registry
/// gets forgotten; a struct is a place to add a field, and the compiler names
/// every construction site when one appears.
///
/// ⚠ **OPAQUE STRINGS ON PURPOSE**, the same contract `construction_recipes` and
/// `content_pack` already had: this crate must not name `ambition_content_pack`,
/// `ambition_sprite_sheet` or `ambition_boss_encounter` to hash their contents.
/// The composition that owns each authority renders its own canonical material
/// and passes it down. `None` means "this composition installed none", which is
/// a real state and not a missing value.
///
/// ⚠ **WHAT IS STILL NOT BOUND, said so the next reader does not assume this
/// list is closed:** the rest of `PlatformerSessionBuilder`'s inputs have not
/// been audited against the mechanical/derived/presentation classification. This
/// covers the three the review named and measured.
/// ⛔⛤ **A CANONICAL DUMP THAT CANNOT BE RENDERED IS A REFUSAL, NOT A STRING.**
/// The first version of these dumps embedded `<unserializable: {error}>` and
/// carried on, which fails OPEN into the identity machinery: the fingerprint
/// becomes a hash of a FAILURE MESSAGE, and two generations that fail the same
/// way are declared identical — which is exactly the claim
/// `RollbackTimelineContract` uses to decide a snapshot may be restored into a
/// world. ⇒ A generation whose mechanical material cannot be rendered has no
/// identity, so the preparation refuses.
/// The canonical form of the immutable developer construction configuration.
///
/// ⛔⛤ **RENDERED HERE RATHER THAN AS A `deterministic_dump` ON EITHER TYPE,
/// BECAUSE NEITHER TYPE OWNS THE CLASS.** `AuthoredBrainOverride` and
/// `AuthoredPopulationCap` live in `ambition_characters` and know nothing about
/// each other; what binds them is that BOTH are read once from the environment
/// by the developer plugin and BOTH change the roster a room construction
/// admits. The class is a property of this identity, so the rendering is too.
///
/// ⚠ **EVERY FIELD IS SPELLED, INCLUDING THE ABSENT ONES.** A dump that omits an
/// unset field cannot distinguish *"no preset"* from *"no developer tools"* from
/// a field that was added and forgotten — and a fingerprint that cannot tell
/// those apart is a fingerprint two different worlds can share. `-` is the
/// absent spelling and is not a legal value of any of these fields.
fn developer_construction_dump(
    brains: Option<&ambition_characters::brain::AuthoredBrainOverride>,
    population: Option<&ambition_characters::actor::AuthoredPopulationCap>,
) -> Option<String> {
    // ⛔ ABSENT ONLY WHEN BOTH ARE, so a composition that installs one knob and
    // not the other is not silently reported as installing neither.
    if brains.is_none() && population.is_none() {
        return None;
    }
    let spell = |value: Option<&str>| value.unwrap_or("-").to_string();
    Some(format!(
        "brain.preset={}\nbrain.profile={}\npopulation.cap={}\n",
        spell(brains.and_then(ambition_characters::brain::AuthoredBrainOverride::preset)),
        spell(brains.and_then(ambition_characters::brain::AuthoredBrainOverride::profile)),
        population
            .and_then(|cap| cap.cap())
            .map_or_else(|| "-".to_string(), |cap| cap.to_string()),
    ))
}

fn canonical<T>(
    source: Option<&T>,
    render: impl Fn(&T) -> Result<String, String>,
    section: &'static str,
) -> Result<Option<String>, ContentDiagnostic> {
    source
        .map(|value| render(value).map_err(|error| ContentDiagnostic::new(section, error)))
        .transpose()
}

#[derive(Clone, Debug, Default)]
pub struct MechanicalRegistries {
    /// Canonical descriptor-only dump of every installed construction domain.
    /// Executable recipe dispatch remains typed and closed inside each domain;
    /// the fingerprint needs only stable schema metadata, never function
    /// pointers.
    pub construction_recipes: Option<String>,
    /// The selected authored content pack's identity.
    pub content_pack: Option<String>,
    /// The pre-fold staged cast — `ambition_characters::prepared::staged_cast_declaration`.
    pub prepared_cast: Option<String>,
    /// Authored sheet declarations, which carry body metrics and authored attack
    /// geometry — `AuthoredSheets::deterministic_dump`.
    pub authored_sheets: Option<String>,
    /// Boss behaviours, encounters, sheets and fallbacks —
    /// `BossCatalog::deterministic_dump`.
    pub boss_catalog: Option<String>,
    /// ⛔⛤ **IMMUTABLE DEVELOPER CONSTRUCTION CONFIGURATION, WHICH IS STILL
    /// MECHANICAL.** `AuthoredBrainOverride` chooses a forced preset/profile in
    /// the NPC construction road and `AuthoredPopulationCap` is spent by
    /// `RoomFeatureConstructionPlan::prepare` BEFORE the construction rows
    /// exist, so different values produce a different authoritative ROSTER and
    /// different autonomous behaviour.
    ///
    /// ⛔ **"WRITTEN ONCE FROM THE ENVIRONMENT" ANSWERS THE SNAPSHOT QUESTION
    /// AND NOT THE IDENTITY ONE**, and the project had been conflating them.
    /// `rollback_coverage` waives both because a resimulated frame never rereads
    /// them — correct, and about STORAGE. Two Apps could still share one
    /// `PreparedContentIdentity` with different rosters, and that identity is
    /// what `RollbackTimelineContract` compares to decide a snapshot may be
    /// restored into a world.
    ///
    /// ⚠ **ONE SECTION FOR BOTH, because they are one class**: developer
    /// construction configuration. `None` means this composition installs no
    /// developer tools, which is what an unset environment variable has always
    /// meant and is a real state rather than a missing value.
    pub developer_construction: Option<String>,
}

pub fn prepare_platformer_content(
    source: PreparedPlatformerSource,
    authored: &crate::authoring::AuthoredCatalogFragments,
    character_registry: Option<
        &ambition_characters::actor::character_catalog::CharacterCatalogRegistry,
    >,
    placement_lowering: Option<
        &ambition_platformer2d_actor_monolith::world::placements::PlacementLoweringRegistry,
    >,
    content_staging: Option<
        &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry,
    >,
    mechanical: MechanicalRegistries,
    snapshot_schema: ambition_platformer2d_runtime::SnapshotSchemaFingerprint,
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
    // Found by trying to photograph the player wearing an older incarnation of itself.
    //
    // Two different facts were wearing one check. The provider's DEFAULT is an
    // authoring fact and is still validated: it must be non-empty and must name
    // a character this composition actually has, or an untouched build spawns
    // something nothing can describe. Which character a session SELECTED is a
    // runtime choice, and the whole point of a playable cast is that it may be
    // any member of it.
    //
    // What is checked now is the property that actually matters — the effective character RESOLVES.
    //
    // One question at two sites is the shape; if a third site ever needs it, route it through
    // here instead of copying the test.
    let effective_character = source
        .starting_character()
        .effective_id(authored.starting_character.as_str());
    if source.catalogs().audio_provider.as_str() != authored.audio_provider.as_str() {
        return Err(ContentDiagnostic::new(
            "provider.defaults",
            format!(
                "expected audio provider '{}', got '{}'",
                authored.audio_provider,
                source.catalogs().audio_provider,
            ),
        ));
    }
    // An assembly failure is not this check's to report — the caller already
    // surfaces it — so a registry that will not assemble simply leaves the
    // resolvability question unanswered rather than answering it wrongly.
    if let Some(catalog) = character_registry
        .map(|registry| registry.assemble())
        .and_then(Result::ok)
    {
        let catalog = &catalog.catalog;
        for (role, id) in [
            ("authored default", authored.starting_character.as_str()),
            ("selected starting character", effective_character),
        ] {
            if catalog.display_name(id).is_none() {
                return Err(ContentDiagnostic::new(
                    "provider.defaults",
                    format!(
                        "the {role} '{id}' names no character in this composition's \
                         assembled catalog — a body would spawn wearing an identity \
                         nothing can describe"
                    ),
                ));
            }
        }
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
    //  Only what the dump carries is hashed: for a recipe, its id + owner +
    // source + schema id; for a relation, its kind + owner + source + schema id.
    //
    // Neither the wiring function nor the postcondition verifier is hashed, and
    // neither can be: a `fn` address is process-local, so hashing one would make
    // the fingerprint differ between two runs of the same binary. Bumping the
    // schema id is therefore the ONLY way a behaviour change reaches the
    // fingerprint, which is the same rule that governs relation registration
    // identity — see `try_register_relation`, which deliberately does not
    // compare function addresses either. Postcondition verification exists
    // partly because of this gap: a relation whose wiring silently stopped
    // working under an unchanged schema id is invisible here, and visible there.
    let MechanicalRegistries {
        construction_recipes,
        content_pack,
        prepared_cast,
        authored_sheets,
        boss_catalog,
        developer_construction,
    } = mechanical;
    // ⛔ DESTRUCTURED EXHAUSTIVELY, so a field added to `MechanicalRegistries`
    // and not bound below is a compile error rather than a silent omission —
    // which is the defect this whole struct exists to close.
    for (section, material) in [
        ("construction.recipes", construction_recipes),
        ("content.pack", content_pack),
        // ⛔⛤ THE THREE THAT SESSION CONSTRUCTION CONSUMED AND NOTHING
        // FINGERPRINTED. See `MechanicalRegistries`.
        ("characters.definitions", prepared_cast),
        ("characters.authored-sheets", authored_sheets),
        ("boss.catalog", boss_catalog),
        // ⛔ THE IMMUTABLE DEVELOPER KNOBS THAT CHANGE THE CONSTRUCTED ROSTER.
        // See `MechanicalRegistries::developer_construction`.
        ("construction.developer", developer_construction),
    ] {
        builder
            .add_section(section, material.map_or_else(Vec::new, String::into_bytes))
            .map_err(|error| ContentDiagnostic::new(section, error.to_string()))?;
    }

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
    /// ⛔⛤ **THE MECHANICAL VALUES THIS GENERATION WAS FINGERPRINTED AGAINST,
    /// FROZEN AT PREPARATION.**
    ///
    /// `PreparedContentIdentity` binds the prepared cast, the authored sheets and
    /// the boss catalog — and session construction USED TO RE-READ THE SAME
    /// MUTABLE App registries at activation, so a generation could be prepared
    /// against cast N and have its world built from N+1 with the identity still
    /// claiming N. MEASURED: `activate_staged_revision` inserts a fresh
    /// `PreparedCharacterRegistry`, which is exactly the hot-reload road.
    ///
    /// ⇒ **A prepared generation now MEANS the frozen values**, not
    /// *"`PreparedContent` says N, and at activation query whatever these
    /// resources contain now"*.
    pub mechanical: SessionMechanics,
}

/// The mechanical registries a session is constructed from.
///
/// ⛔⛤ **IT MOVED DOWN A CRATE, AND THAT IS THE FIX.** As a provider-private
/// struct it could only ever be a PREPARED-session field, handed to `build` and
/// dropped — so a generation's frozen values governed the first construction
/// call and every later road (a door, a death, a reset) went back to the App.
/// Living in `actor_monolith` it can also be the RESOURCE those roads read. One
/// type, two lifetimes: frozen in the prepared record, promoted at activation.
pub use ambition_platformer2d_actor_monolith::session::mechanics::SessionMechanics;

#[derive(Resource, Default)]
pub struct PreparedPlatformerSessions {
    records: BTreeMap<ambition_load::LoadId, PreparedPlatformerRecord>,
}

/// Installed by a host that answers `prepare-first-room-art` itself: it reads
/// [`PreparedPlatformerSessions::published`] for the start room and the
/// starting character, demands and decodes their art, and completes the work
/// item through `LoadCommand::SetWorkState`. Without it the provider completes
/// the item at publish time (a thin host has no sprite catalog to ask).
#[derive(Resource, Default, Debug)]
pub struct FirstRoomArtContributor;

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

    /// Every published, not yet activated session with the transaction that
    /// prepared it — what a host reads to prepare the first room's art before
    /// activation.
    pub fn published(
        &self,
    ) -> impl Iterator<Item = (&ProviderLoadTransaction, &PreparedPlatformerSession)> {
        self.records
            .values()
            .map(|record| (&record.transaction, &record.prepared))
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
        // ⛔⛤ **PROMOTED, NOT JUST CONSUMED.** `take` removes the prepared record
        // and `build` borrows the frozen values for one construction call; every
        // LATER road that rebuilds a room — a door, a death, a reset — used to go
        // back to whatever registries the App held by then. Installing the
        // generation's mechanics as a resource is what makes "this session runs
        // under generation N" a statement about its CONTENT and not only about
        // its stamp. Session-scoped teardown removes it with the rest.
        builder
            .commands
            .insert_resource(prepared.mechanical.clone());
        builder.build(
            activation,
            *scope,
            prepared.content,
            &prepared.mechanical,
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
    tuning: Res<'w, ambition_platformer2d_core::ActiveMovementTuning>,
    character_catalog: Res<'w, ambition_characters::actor::character_catalog::CharacterCatalog>,
    /// The prepared cast, when this composition registered one. Activation builds
    /// the player's BODY, and a prepared character states what a body physically
    /// is — its health pool, its mass, its authored box.

    /// The published controller policies, so an enemy placement may name one
    /// (`EnemySpawnSpec::brain_profile`).
    brain_profiles:
        Option<Res<'w, ambition_characters::actor::character_catalog::BrainProfileRegistry>>,
    /// What a DEVELOPER has forced every authored actor's brain to. Absent =
    /// no developer tools = the author decides. Beside the policies because it
    /// is the same class of authority: lowering consults it while BUILDING a
    /// brain, which the actor kernel used to do by calling into
    /// `ambition_dev_tools` directly.
    /// Paired with the population cap because this `SystemParam` is at Bevy's
    /// sixteen: both are developer overrides of the authored cast, threaded
    /// into lowering as a snapshot rather than read from a developer crate.
    forced_brains: (
        Option<Res<'w, ambition_characters::brain::AuthoredBrainOverride>>,
        Option<Res<'w, ambition_characters::actor::AuthoredPopulationCap>>,
    ),
    /// Provider-authored sheets (U1): activation sizes each seated body
    /// from its sheet, so the builder needs it beside the catalog.

    placement_lowering:
        Res<'w, ambition_platformer2d_actor_monolith::world::placements::PlacementLoweringRegistry>,
    content_staging:
        Res<'w, ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry>,
    construction_recipes:
        Res<'w, ambition_platformer2d_actor_monolith::construction::ActorConstructionRegistry>,
    moving_platforms: ResMut<'w, ambition_platformer2d_world::collision::MovingPlatformSet>,
    active_session: ResMut<'w, ActiveGameplaySession>,
    /// ⭐⭐ WHERE THE FILE SAYS THINGS ARE, AT THE MOMENT THE WORLD IS BUILT.
    /// Activation used to pass no continuity at all, so a load authored an
    /// object the save says is lying next door and a later checkpoint resume
    /// rebuilt the room to take it back out. The ledger is adopted at
    /// `SessionScopeSet::Activate` now, which is before this runs.
    ///
    /// `Option` because a composition with no durable horizon registers neither
    /// resource, and an empty ledger is the honest answer there — not a reason
    /// to refuse to build a world.
    occurrences:
        Option<Res<'w, ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
    minted: Option<
        Res<
            'w,
            ambition_platformer2d_actor_monolith::items::pickup::minted_horizon::MintedItemBaseline,
        >,
    >,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionBuildResult {
    /// The session's home body, when the experience declared one.
    ///
    /// See `InitialBodyPolicy`.
    pub player: Option<Entity>,
    pub world: Entity,
}

impl PlatformerSessionBuilder<'_, '_> {
    pub fn build(
        &mut self,
        activation: &ActiveShellExperience,
        scope: SessionScopeId,
        prepared_content: PreparedContent,
        // ⛔⛤ **THE FROZEN MECHANICAL STATE, NOT THIS App's CURRENT REGISTRIES.**
        // The three `Res` handles that used to serve these reads are GONE from
        // the `SystemParam`, so building from whatever the world contains now is
        // not something this function can express. See `SessionMechanics`.
        mechanical: &SessionMechanics,
        default_character_id: &str,
    ) -> SessionBuildResult {
        let live_world: PlatformerSessionWorld = prepared_content.source().instantiate_live();
        // The authoring format's own session state, installed beside the
        // canonical bundle rather than inside it. `None` for every
        // RON-authored game, and the component is then simply ABSENT from the
        // session root — which is what the LDtk systems' `SessionWorldRef`
        // reads as "no LDtk world here", instead of an empty index that made
        // them run and find nothing every tick.
        #[cfg(feature = "ldtk")]
        let installed_ldtk_index = prepared_content.source().installed_ldtk_index().cloned();
        let prepared_identity: PreparedContentIdentity = prepared_content.identity();
        // Live moving-platform state derives from the activating room. Rooms
        // without authored platforms (every current demo) reset it to empty.
        self.moving_platforms.0 = ambition_platformer2d_world::platforms::moving_platforms_for_room(
            live_world.room_set.active_spec(),
        );

        let player = ambition_platformer2d_actor_monolith::session::setup::simulation_world(
            &mut self.commands,
            SessionSpawnScope::scoped(scope),
            ambition_platformer2d_actor_monolith::session::setup::SimulationSetup {
                world: &live_world.geometry,
                room_set: &live_world.room_set,
                // The CALLER converts: who edits the set is a developer
                // facility, and construction needs only the set.
                fallback_abilities: self.editable_abilities.as_engine(),
                tuning: &self.tuning,
                initial_body: &live_world.initial_body,
                prepared_characters: mechanical.characters.as_ref(),
                placement_lowering: &self.placement_lowering,
                content_staging: &self.content_staging,
                // Activation is the one place that holds the exact prepared
                // definition, so it is the one place a construction plan can
                // state a REAL activation generation rather than defaulting.
                //  the cast was two lines away and not handed over.
                // Planning asks the CHARACTER whether a placement is a limbed
                // `"giant"`-class host before it asks the roster, and with no
                // cast it can only ask the roster — so the shipped sandbox's
                // giant, which authors its mount class on its definition,
                // planned as an ordinary enemy and failed relation verification
                // (*"is the mount of relation `ambition.mount` but is
                // constructed as a `authored-enemy`"*). Naming the authorities
                // as arguments is what stopped that being a thing a road can
                // forget.
                construction:
                    ambition_platformer2d_actor_monolith::features::ActorConstructionContext::for_room_construction(
                        &self.construction_recipes,
                        &self.character_catalog,
                        // ⛔ THE GENERATION BEING ACTIVATED, WITH NO FALLBACK.
                        // See `GenerationMechanics::of`.
                        &ambition_platformer2d_actor_monolith::session::mechanics::
                            GenerationMechanics::of(mechanical),
                        prepared_identity.epoch,
                        None,
                        self.brain_profiles.as_deref(),
                        // ⭐ THE SAVE'S LEDGER, AT CONSTRUCTION. A fresh session
                        // has an empty one and builds exactly what it always
                        // built; a LOAD has the file's rows here, so the room it
                        // opens in is built right the first time instead of
                        // built wrong and corrected. The world's definitions
                        // ride along because a row may name an object minted by
                        // a record next door — see `OccurrenceContinuity`.
                        self.occurrences.as_deref().map(|remembered| {
                            ambition_platformer2d_actor_monolith::features::OccurrenceContinuity {
                                remembered,
                                world: &live_world.room_set.rooms,
                                minted: self.minted.as_deref(),
                            }
                        }),
                        self.forced_brains.0.as_deref(),
                        self.forced_brains.1.as_deref(),
                    ),
                boss_catalog: &mechanical.bosses,
                default_character_id,
            },
        );

        let world = self
            .active_session
            .spawn_world_for(
                &mut self.commands,
                activation,
                scope,
                // The bare epoch rides alongside the identity that defines it,
                // from this single value, so layers below `ambition_platformer2d_runtime`
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

        // SESSION INPUT OWNERSHIP GOES ON THE WORLD, NOT ON A BODY.
        //
        //  `GameplayInputOwner` documents itself as *"Marker and exact owner
        // facts on the canonical live gameplay-world entity"* and was then
        // inserted onto the player. Nothing read it for routing — its only
        // consumer counts instances — so the disagreement was invisible right up
        // until a session could legitimately have no body at all, at which point
        // "no home avatar" would silently have meant "no input owner". A
        // human-versus-CPU match has no home body and certainly has input
        // authority; those are different facts and now live in different places.
        self.commands.entity(world).insert(GameplayInputOwner {
            activation_id: activation.activation_id,
            scope,
        });

        // A format installs its own index; without the capability there is none
        // to install, and the session root simply has no such component — which
        // is exactly what the LDtk systems read as "no LDtk world here".
        #[cfg(feature = "ldtk")]
        if let Some(index) = installed_ldtk_index {
            self.commands.entity(world).insert(index);
        }

        SessionBuildResult { player, world }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authoring::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};

    fn active(line: &str) -> ambition_platformer2d_runtime::SelectedContentIdentity {
        ambition_platformer2d_runtime::SelectedContentIdentity(line.to_string())
    }

    fn claim(load: &str, line: &str) -> ambition_platformer2d_runtime::PendingGenerationInputs {
        ambition_platformer2d_runtime::PendingGenerationInputs {
            load_id: load.to_string(),
            identity: line.to_string(),
            characters: None,
        }
    }

    /// ⛔⛔ **A CANDIDATE IDENTITY BELONGS TO ONE TRANSACTION AND NOBODY ELSE.**
    ///
    /// ⛤ THIS ARM EXISTS BECAUSE ITS ABSENCE WAS MEASURED. Poisoning the
    /// resolution to ignore the claim entirely left 868 tests green across this
    /// crate and `ambition_app`: the consumer half of the per-transaction
    /// identity had no witness anywhere in the tree.
    #[test]
    fn a_pending_identity_is_read_only_by_the_transaction_that_claimed_it() {
        // ⭐ THE CONTROL FIRST: with no claim at all, the App's selection is the
        // answer — or "a stranger reads the active identity" is satisfied by a
        // resolution that never returns a candidate.
        assert_eq!(
            content_identity_for(Some(&active("pack 1 cfp1:aa")), None, "shell.game.1"),
            Some("pack 1 cfp1:aa".to_string()),
            "with no pending claim the active selection must be the answer"
        );
        // The owner reads the candidate.
        assert_eq!(
            content_identity_for(
                Some(&active("pack 1 cfp1:aa")),
                Some(&claim("shell.game.7", "pack 2 cfp1:bb")),
                "shell.game.7",
            ),
            Some("pack 2 cfp1:bb".to_string()),
            "the claiming transaction did not read its own candidate identity"
        );
        // ⛔ THE ASSERTION THE WHOLE ARM IS FOR — and both loads name the SAME
        // ROUTE, because two generations can target one route and a route
        // comparison would pass this.
        assert_eq!(
            content_identity_for(
                Some(&active("pack 1 cfp1:aa")),
                Some(&claim("shell.game.7", "pack 2 cfp1:bb")),
                "shell.game.8",
            ),
            Some("pack 1 cfp1:aa".to_string()),
            "an unrelated transaction was fingerprinted against a candidate it \
             never prepared — the rollback timeline contract compares exactly \
             this identity"
        );
        // ⚠ ABSENT IS A REAL ANSWER: a composition with no content pack selects
        // none, and a stranger's claim must not invent one for it.
        assert_eq!(
            content_identity_for(
                None,
                Some(&claim("shell.game.7", "pack 2 cfp1:bb")),
                "shell.game.8"
            ),
            None,
            "a stranger's claim became the answer for an App that selected no pack"
        );
    }

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
        let room = ambition_platformer2d_world::rooms::RoomSpec::new(
            "same-room",
            ambition_platformer2d_core::World::new(
                "same-room",
                ambition_platformer2d_core::Vec2::new(width, 128.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        let room_set = ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "same-room",
            vec![room],
            Vec::new(),
        );
        PreparedPlatformerSource::new(
            "same-provider",
            room_set.clone(),
            ambition_platformer2d_core::RoomGeometry(room_set.active_world().clone()),
            ambition_platformer2d_world::rooms::ActiveRoomMetadata(
                room_set.active_spec().metadata.clone(),
            ),
            ambition_platformer2d_actor_monolith::avatar::StartingCharacter::new("alpha"),
        )
    }

    fn isolated_room_fixture_source(room_id: &str) -> PreparedPlatformerSource {
        let room = ambition_platformer2d_world::rooms::RoomSpec::new(
            room_id,
            ambition_platformer2d_core::World::new(
                room_id,
                ambition_platformer2d_core::Vec2::new(128.0, 128.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        let room_set = ambition_platformer2d_world::rooms::RoomSet::from_parts(
            room_id,
            vec![room],
            Vec::new(),
        );
        PreparedPlatformerSource::new(
            "same-provider",
            room_set.clone(),
            ambition_platformer2d_core::RoomGeometry(room_set.active_world().clone()),
            ambition_platformer2d_world::rooms::ActiveRoomMetadata(
                room_set.active_spec().metadata.clone(),
            ),
            ambition_platformer2d_actor_monolith::avatar::StartingCharacter::new("alpha"),
        )
    }

    fn two_room_fixture_source(active_room: &str) -> PreparedPlatformerSource {
        let first = ambition_platformer2d_world::rooms::RoomSpec::new(
            "same-room",
            ambition_platformer2d_core::World::new(
                "same-room",
                ambition_platformer2d_core::Vec2::new(128.0, 128.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        let second = ambition_platformer2d_world::rooms::RoomSpec::new(
            "second-room",
            ambition_platformer2d_core::World::new(
                "second-room",
                ambition_platformer2d_core::Vec2::new(160.0, 128.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        let mut room_set = ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "same-room",
            vec![first, second],
            Vec::new(),
        );
        room_set.active = room_set.room_index_by_id(active_room).unwrap();
        PreparedPlatformerSource::new(
            "same-provider",
            room_set.clone(),
            ambition_platformer2d_core::RoomGeometry(room_set.active_world().clone()),
            ambition_platformer2d_world::rooms::ActiveRoomMetadata(
                room_set.active_spec().metadata.clone(),
            ),
            ambition_platformer2d_actor_monolith::avatar::StartingCharacter::new("alpha"),
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

    fn staging_registry(
        reverse: bool,
    ) -> ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry {
        let mut registry =
            ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry::default();
        let register_a = |registry: &mut ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry| {
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
        let register_b = |registry: &mut ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry| {
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
        staging: &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry,
    ) -> PreparedContent {
        fixture_content_with_recipes(source, characters, staging, None)
    }

    fn fixture_content_with_recipes(
        source: PreparedPlatformerSource,
        characters: &ambition_characters::actor::character_catalog::CharacterCatalogRegistry,
        staging: &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry,
        construction_recipes: Option<String>,
    ) -> PreparedContent {
        let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
        let snapshot_schema = ambition_platformer2d_runtime::rollback::RollbackRegistry::default()
            .schema_fingerprint();
        let mut epochs = ContentEpochSequence::default();
        prepare_platformer_content(
            source,
            &authored,
            Some(characters),
            None,
            Some(staging),
            MechanicalRegistries {
                construction_recipes,
                ..Default::default()
            },
            snapshot_schema,
            &mut epochs,
        )
        .unwrap()
    }

    /// Choosing a character other than the default is not a content error.
    ///
    /// `capture_scene --character <id>` has a usage example in its own header — `--character
    /// npc_pirate_admiral` — and it had never worked for any id, that one included (found by
    /// trying to photograph the player wearing an older incarnation of itself).
    ///
    /// What preparation legitimately owns is that the selection RESOLVES, which
    /// the second half of this asserts.
    #[test]
    fn a_starting_character_other_than_the_default_prepares() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);
        let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
        let snapshot_schema = ambition_platformer2d_runtime::rollback::RollbackRegistry::default()
            .schema_fingerprint();

        let prepare = |selected: &str| {
            let base = fixture_source(128.0);
            let source = PreparedPlatformerSource::new(
                "same-provider",
                base.room_set().clone(),
                ambition_platformer2d_core::RoomGeometry(base.room_set().active_world().clone()),
                ambition_platformer2d_world::rooms::ActiveRoomMetadata(
                    base.room_set().active_spec().metadata.clone(),
                ),
                ambition_platformer2d_actor_monolith::avatar::StartingCharacter::new(
                    selected.to_string(),
                ),
            );
            let mut epochs = ContentEpochSequence::default();
            prepare_platformer_content(
                source,
                &authored,
                Some(&characters),
                None,
                Some(&staging),
                MechanicalRegistries::default(),
                snapshot_schema,
                &mut epochs,
            )
        };

        assert!(
            prepare("beta").is_ok(),
            "selecting a real character that is not the provider default must \
             prepare — the whole point of a playable cast is that it has members \
             other than the first one"
        );
        let rejected = prepare("no_such_character")
            .expect_err("a selection that names nothing must still be fatal");
        assert_eq!(rejected.section, "provider.defaults");
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
        let mut registry =
            ambition_platformer2d_actor_monolith::construction::ActorConstructionRegistry::default(
            );
        registry
            .try_register_relation(
                ambition_platformer2d_actor_monolith::construction::relation_grudge(),
                "ambition_platformer2d_actor_monolith",
                "aggression",
                schema,
            )
            .unwrap();
        registry.deterministic_dump()
    }

    /// A real registry, dumped — the same value the app path contributes.
    fn construction_dump(schema: &str) -> String {
        let mut registry =
            ambition_platformer2d_actor_monolith::construction::ActorConstructionRegistry::default(
            );
        registry
            .try_register_recipe(
                ambition_platformer2d_actor_monolith::construction::recipe_staged_actor(),
                "ambition_platformer2d_actor_monolith",
                "content-staging",
                schema,
            )
            .unwrap();
        registry.deterministic_dump()
    }

    fn construction_dump_ordered(reverse: bool) -> String {
        let mut registry =
            ambition_platformer2d_actor_monolith::construction::ActorConstructionRegistry::default(
            );
        let ids = [
            ambition_platformer2d_actor_monolith::construction::recipe_staged_actor(),
            ambition_platformer2d_actor_monolith::construction::recipe_summoned_minion(),
        ];
        let ids: Vec<_> = if reverse {
            ids.into_iter().rev().collect()
        } else {
            ids.into_iter().collect()
        };
        for id in ids {
            registry
                .try_register_recipe(id, "ambition_platformer2d_actor_monolith", "src", "v1")
                .unwrap();
        }
        registry.deterministic_dump()
    }

    /// ⛔⛤ **TWO SESSIONS PREPARED UNDER DIFFERENT CONTENT PACKS ARE DIFFERENT
    /// CONTENT GENERATIONS, AND UNTIL 2026-09-11 THEY WERE NOT.**
    ///
    /// Every other section of a `PreparedContent` is an App REGISTRY, so the
    /// authored content PACK — move tables, item catalog, encounter waves —
    /// reached the game without reaching this fingerprint. The consequence is
    /// not academic: `RollbackTimelineContract` stores a `PreparedContentIdentity`
    /// and the GGRS session refuses *"prepared content changed while the session
    /// was active"* by comparing exactly that, so the guard could not see the
    /// content most likely to change during development.
    ///
    /// ⚠ THE SIBLING ARM BELOW IS WHAT MAKES THIS ONE MEAN SOMETHING: two
    /// preparations that agree on everything INCLUDING the pack still share a
    /// fingerprint, so this is not "any two preparations differ".
    #[test]
    fn two_preparations_under_different_content_packs_are_different_generations() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);
        let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
        let snapshot_schema = ambition_platformer2d_runtime::rollback::RollbackRegistry::default()
            .schema_fingerprint();
        let mut epochs = ContentEpochSequence::default();
        let prepare = |pack: Option<&str>, epochs: &mut ContentEpochSequence| {
            prepare_platformer_content(
                fixture_source(128.0),
                &authored,
                Some(&characters),
                None,
                Some(&staging),
                MechanicalRegistries {
                    content_pack: pack.map(str::to_string),
                    ..Default::default()
                },
                snapshot_schema,
                epochs,
            )
            .unwrap()
        };

        let first = prepare(Some("ambition 1.0.0 aaaaaaaaaaaaaaaa"), &mut epochs);
        let second = prepare(Some("ambition 1.0.0 bbbbbbbbbbbbbbbb"), &mut epochs);
        assert_ne!(
            first.fingerprint(),
            second.fingerprint(),
            "a different content pack did not reach the prepared content's identity"
        );

        // ⛔ AND ABSENT IS ITS OWN ANSWER, distinct from either pack: a
        // composition that selected no pack is not the same generation as one
        // that selected one.
        let none = prepare(None, &mut epochs);
        assert_ne!(none.fingerprint(), first.fingerprint());
        assert_ne!(none.fingerprint(), second.fingerprint());
    }

    /// ⭐ THE CONTROL. The same pack twice is the same definition, so the arm
    /// above is about the PACK and not about preparation being nondeterministic.
    #[test]
    fn two_preparations_under_one_content_pack_share_a_definition_identity() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);
        let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
        let snapshot_schema = ambition_platformer2d_runtime::rollback::RollbackRegistry::default()
            .schema_fingerprint();
        let mut epochs = ContentEpochSequence::default();
        let prepare = |epochs: &mut ContentEpochSequence| {
            prepare_platformer_content(
                fixture_source(128.0),
                &authored,
                Some(&characters),
                None,
                Some(&staging),
                MechanicalRegistries {
                    content_pack: Some("ambition 1.0.0 aaaaaaaaaaaaaaaa".to_string()),
                    ..Default::default()
                },
                snapshot_schema,
                epochs,
            )
            .unwrap()
        };
        assert_eq!(
            prepare(&mut epochs).fingerprint(),
            prepare(&mut epochs).fingerprint()
        );
    }

    #[test]
    fn sequential_preparations_share_definition_identity_but_not_epoch() {
        let characters = character_registry(false, CHARACTER_B);
        let staging = staging_registry(false);
        let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
        let snapshot_schema = ambition_platformer2d_runtime::rollback::RollbackRegistry::default()
            .schema_fingerprint();
        let mut epochs = ContentEpochSequence::default();
        let first = prepare_platformer_content(
            fixture_source(128.0),
            &authored,
            Some(&characters),
            None,
            Some(&staging),
            MechanicalRegistries::default(),
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
            MechanicalRegistries::default(),
            snapshot_schema,
            &mut epochs,
        )
        .unwrap();

        assert_eq!(first.fingerprint(), second.fingerprint());
        assert_ne!(first.epoch(), second.epoch());
        assert_eq!(
            first.epoch(),
            ambition_platformer2d_runtime::ContentEpoch(1)
        );
        assert_eq!(
            second.epoch(),
            ambition_platformer2d_runtime::ContentEpoch(2)
        );
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

        let committed = changed.with_epoch(ambition_platformer2d_runtime::ContentEpoch(9));
        assert_eq!(
            committed.epoch(),
            ambition_platformer2d_runtime::ContentEpoch(9)
        );
        assert_eq!(
            active.epoch(),
            ambition_platformer2d_runtime::ContentEpoch(1)
        );
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

#[cfg(test)]
mod mechanical_registries_reach_the_identity {
    //! ⛔⛤ **THREE MECHANICAL APP REGISTRIES BUILT THE SESSION AND NONE OF THEM
    //! REACHED `PreparedContentIdentity`.**
    //!
    //! `PlatformerSessionBuilder` consumes the prepared cast, the authored
    //! sheets and the boss catalog while constructing the world. The identity
    //! bound `CharacterCatalogRegistry::canonical_fragments()` — **a DIFFERENT
    //! ROAD**, measured 2026-09-12: the provider API `App::register_character`
    //! goes to `stage_authored_character` and lands in `StagedCharacterOverrides`,
    //! never in that registry. So two compositions could differ in a character's
    //! `max_health`, in the body geometry an authored sheet declares, or in how
    //! a boss fights, and share one identity.
    //!
    //! ⛔⛔ **AND THE IDENTITY IS NOT A LABEL.** `RollbackTimelineContract` stores
    //! it to state which world a timeline is valid for, so "the same generation"
    //! was a claim two mechanically different worlds could both make.
    //!
    //! ⭐ **EACH ARM CHANGES EXACTLY ONE THING AND SHARES EVERYTHING ELSE WITH
    //! ITS PAIR**, so a difference cannot come from anywhere but the registry
    //! named — and the CONTROL below proves two Apps told the same things agree,
    //! without which every arm here is satisfied by an identity that is simply
    //! unstable.

    use super::*;
    use crate::authoring::AuthoredCatalogFragments;

    fn source() -> PreparedPlatformerSource {
        let world = ambition_platformer2d_core::World::new(
            "room",
            ambition_platformer2d_core::Vec2::new(128.0, 128.0),
            ambition_platformer2d_core::Vec2::new(16.0, 16.0),
            Vec::new(),
        );
        let room = ambition_platformer2d_world::rooms::RoomSpec::new("room", world.clone());
        PreparedPlatformerSource::new(
            "fixture",
            ambition_platformer2d_world::rooms::RoomSet::from_parts(
                "room",
                vec![room],
                Vec::new(),
            ),
            ambition_platformer2d_core::RoomGeometry(world),
            ambition_platformer2d_world::rooms::ActiveRoomMetadata::default(),
            ambition_platformer2d_actor_monolith::avatar::StartingCharacter::new("alpha"),
        )
    }

    /// Prepare an App whose only variation is whatever `stage` did to it.
    fn identity_of(stage: impl FnOnce(&mut bevy::app::App)) -> String {
        let mut app = bevy::app::App::new();
        stage(&mut app);
        prepare_platformer_content_for_app(
            &mut app,
            source(),
            &AuthoredCatalogFragments::new("alpha", "fixture"),
        )
        .expect("the fixture composition prepares")
        .fingerprint()
        .to_string()
    }

    fn a_character(max_health: i32) -> ambition_characters::actor::definition::CharacterDefinition {
        let mut definition =
            ambition_characters::actor::definition::CharacterDefinition::new("alpha", "Alpha", "fixture");
        definition.vitals.max_health = Some(max_health);
        definition
    }

    /// The cast an App PUBLISHES after staging one character with this health.
    ///
    /// ⚠ The published registry, not the staged overrides: the freeze captures
    /// the value the builder actually reads, and those are different resources.
    fn published_cast(
        max_health: i32,
    ) -> ambition_characters::prepared::PreparedCharacterRegistry {
        let mut app = bevy::app::App::new();
        ambition_characters::prepared::stage_authored_character(
            &mut app,
            a_character(max_health),
            &Default::default(),
        )
        .expect("stages");
        ambition_characters::prepared::close_preparation_barrier(app.world_mut());
        app.world()
            .get_resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
            .cloned()
            .expect("the barrier publishes a cast")
    }

    fn health_of(cast: &ambition_characters::prepared::PreparedCharacterRegistry) -> Option<i32> {
        cast.get("alpha").and_then(|definition| definition.vitals.max_health)
    }

    /// ⛔⛤ **PREPARE N+1, FREEZE N+1 — NOT THE CAST THE APP HAS STILL GOT
    /// PUBLISHED.**
    ///
    /// ⛤ THIS ARM EXISTS BECAUSE A REVIEW FOUND THE OPPOSITE SHIPPED. A
    /// cast-changing reload admits the N+1 registry at REQUEST time and
    /// deliberately withholds it from the App until the commit boundary, so
    /// throughout the transaction the published `PreparedCharacterRegistry` is
    /// still N. The freeze read that published registry — giving a prepared
    /// session whose identity names N+1 and whose fighters are N's.
    ///
    /// ⭐ **THE DISCRIMINATOR IS A MECHANICAL VALUE, NOT A GENERATION COUNTER.**
    /// `max_health` is read by the thing that builds the body; a counter would
    /// let a resolver that returns the right STAMP with the wrong CONTENTS pass.
    #[test]
    fn a_transaction_freezes_its_own_candidate_cast_not_the_apps_published_one() {
        let live = published_cast(3);
        let candidate = published_cast(9);
        // ⭐ THE PREMISE FIRST: these two casts must actually differ, or every
        // assertion below is satisfied by a resolver that returns anything.
        assert_eq!(health_of(&live), Some(3));
        assert_eq!(health_of(&candidate), Some(9));

        let mine = ambition_platformer2d_runtime::PendingGenerationInputs {
            load_id: "shell.game.7".to_string(),
            identity: "pack 2 cfp1:bb".to_string(),
            characters: Some(candidate.clone()),
        };

        // ⛔ THE ASSERTION THE ARM IS FOR.
        assert_eq!(
            health_of(
                &candidate_cast_for(Some(&live), Some(&mine), "shell.game.7")
                    .expect("the claiming transaction has a cast")
            ),
            Some(9),
            "the transaction froze the App's PUBLISHED cast while its OWN \
             admitted candidate was the one it would publish at commit — a \
             session whose identity names N+1 built from N's fighters",
        );

        // ⛔ A STRANGER'S CANDIDATE IS NOT A FALLBACK. Both loads name the same
        // ROUTE, because two generations can target one route.
        assert_eq!(
            health_of(
                &candidate_cast_for(Some(&live), Some(&mine), "shell.game.8")
                    .expect("a stranger still has the App's cast")
            ),
            Some(3),
            "an unrelated preparation was built from a candidate cast it never \
             prepared",
        );

        // ⚠ OUR CLAIM, CARRYING NO CAST, IS NOT "USE ANYTHING": the candidate
        // does not change the cast, so the App's published cast IS this
        // transaction's own value.
        let castless = ambition_platformer2d_runtime::PendingGenerationInputs {
            characters: None,
            ..mine.clone()
        };
        assert_eq!(
            health_of(
                &candidate_cast_for(Some(&live), Some(&castless), "shell.game.7")
                    .expect("a cast-preserving transaction still has a cast")
            ),
            Some(3),
        );

        // ⚠ ABSENT IS A REAL ANSWER: a composition that published no cast at
        // all must not be handed a stranger's.
        assert!(
            candidate_cast_for(None, Some(&mine), "shell.game.8").is_none(),
            "a stranger's candidate became the cast for an App that published none",
        );
    }

    /// ⛔⛤ **A DEVELOPER'S POPULATION CAP CHANGES THE ROSTER AND MUST CHANGE THE
    /// IDENTITY.**
    ///
    /// ⛤ FROM THE 2026-09-13 REVIEW. `AuthoredPopulationCap` is spent by
    /// `RoomFeatureConstructionPlan::prepare` BEFORE the construction rows
    /// exist, so a capped App and an uncapped one build DIFFERENT authoritative
    /// rosters — and they shared one `PreparedContentIdentity`, which is the
    /// value `RollbackTimelineContract` compares to decide whether a snapshot
    /// from one world may be restored into another.
    ///
    /// ⚠ **"WRITTEN ONCE FROM THE ENVIRONMENT" IS AN ANSWER TO A DIFFERENT
    /// QUESTION.** `rollback_coverage` waives this resource because a
    /// resimulated frame never rereads it. That is about STORAGE. This is about
    /// IDENTITY, and the project had been treating one as the other.
    #[test]
    fn a_developer_population_cap_reaches_the_identity() {
        let uncapped = identity_of(|app| {
            ambition_characters::prepared::stage_authored_character(
                app,
                a_character(7),
                &Default::default(),
            )
            .expect("stages");
            app.insert_resource(ambition_characters::actor::AuthoredPopulationCap::UNCAPPED);
        });
        let capped = identity_of(|app| {
            ambition_characters::prepared::stage_authored_character(
                app,
                a_character(7),
                &Default::default(),
            )
            .expect("stages");
            app.insert_resource(ambition_characters::actor::AuthoredPopulationCap::capped_at(1));
        });
        assert_ne!(
            uncapped, capped,
            "two compositions that admit different numbers of authored actors \
             share one PreparedContentIdentity — a snapshot taken under one \
             would be declared restorable into the other",
        );
    }

    /// ⛔ THE SAME FOR THE FORCED BRAIN, and BOTH FIELDS, because a dump that
    /// spells one and forgets the other is the omission this section exists to
    /// close.
    #[test]
    fn a_forced_developer_brain_reaches_the_identity() {
        let authored = identity_of(|app| {
            ambition_characters::prepared::stage_authored_character(
                app,
                a_character(7),
                &Default::default(),
            )
            .expect("stages");
            app.insert_resource(ambition_characters::brain::AuthoredBrainOverride::default());
        });
        let forced_preset = identity_of(|app| {
            ambition_characters::prepared::stage_authored_character(
                app,
                a_character(7),
                &Default::default(),
            )
            .expect("stages");
            app.insert_resource(ambition_characters::brain::AuthoredBrainOverride {
                preset: Some("stand_still".to_string()),
                profile: None,
            });
        });
        let forced_profile = identity_of(|app| {
            ambition_characters::prepared::stage_authored_character(
                app,
                a_character(7),
                &Default::default(),
            )
            .expect("stages");
            app.insert_resource(ambition_characters::brain::AuthoredBrainOverride {
                preset: None,
                profile: Some("fighter".to_string()),
            });
        });
        assert_ne!(
            authored, forced_preset,
            "a composition whose every actor's brain PRESET is forced shares one \
             identity with the composition that lets the author decide",
        );
        assert_ne!(
            authored, forced_profile,
            "a composition whose every actor's autonomous PROFILE is forced \
             shares one identity with the composition that lets the author decide",
        );
        // ⛔ AND THE TWO FIELDS ARE NOT INTERCHANGEABLE. A dump that rendered
        // "something is forced" rather than WHICH field would pass both arms
        // above and fail this one — and the two roads reach genuinely different
        // brains (see `AuthoredBrainOverride::profile`).
        assert_ne!(
            forced_preset, forced_profile,
            "forcing the PRESET and forcing the PROFILE produced one identity, so \
             the dump records that a knob is set and not which",
        );
    }

    /// ⛔⛤ **AND TWO DIFFERENT VALUES OF ONE KNOB ARE TWO DIFFERENT WORLDS.**
    ///
    /// ⛤ **THIS ARM EXISTS BECAUSE A POISON DECLINED TO FIRE.** With the dump
    /// rewritten to spell `set` instead of the value, the two arms above BOTH
    /// stayed green: they vary WHICH FIELD is populated and never WHICH VALUE it
    /// holds, so a fingerprint recording only *"a knob is set"* satisfies them.
    /// A composition forcing every brain to `stand_still` and one forcing every
    /// brain to `charge` would have shared an identity, and a cap of 1 and a cap
    /// of 2 with it.
    ///
    /// ⇒ A poison that passes is a finding about the COVERAGE, not a licence to
    /// keep the guard.
    #[test]
    fn two_different_values_of_one_developer_knob_are_two_identities() {
        let forced_to = |preset: &str| {
            let preset = preset.to_string();
            identity_of(move |app| {
                ambition_characters::prepared::stage_authored_character(
                    app,
                    a_character(7),
                    &Default::default(),
                )
                .expect("stages");
                app.insert_resource(ambition_characters::brain::AuthoredBrainOverride {
                    preset: Some(preset),
                    profile: None,
                });
            })
        };
        assert_ne!(
            forced_to("stand_still"),
            forced_to("charge"),
            "two compositions whose every actor is forced to a DIFFERENT brain \
             preset share one identity — the dump records that the knob is set \
             rather than what it is set to",
        );

        let capped_at = |cap: usize| {
            identity_of(move |app| {
                ambition_characters::prepared::stage_authored_character(
                    app,
                    a_character(7),
                    &Default::default(),
                )
                .expect("stages");
                app.insert_resource(
                    ambition_characters::actor::AuthoredPopulationCap::capped_at(cap),
                );
            })
        };
        assert_ne!(
            capped_at(1),
            capped_at(2),
            "a room that admits one authored actor and a room that admits two \
             share one identity",
        );
    }

    /// ⭐ THE CONTROL, and it runs first because every arm below depends on it:
    /// two Apps told the same things must agree, or "they differ" proves only
    /// that the fingerprint is unstable.
    #[test]
    fn two_apps_told_the_same_things_agree() {
        let left = identity_of(|app| {
            ambition_characters::prepared::stage_authored_character(
                app,
                a_character(7),
                &Default::default(),
            )
            .expect("stages");
        });
        let right = identity_of(|app| {
            ambition_characters::prepared::stage_authored_character(
                app,
                a_character(7),
                &Default::default(),
            )
            .expect("stages");
        });
        assert_eq!(
            left, right,
            "two identical compositions disagreed, so every arm below would \
             pass for the wrong reason",
        );
    }

    #[test]
    fn a_registered_characters_max_health_reaches_the_identity() {
        let two = identity_of(|app| {
            ambition_characters::prepared::stage_authored_character(
                app,
                a_character(2),
                &Default::default(),
            )
            .expect("stages");
        });
        let five = identity_of(|app| {
            ambition_characters::prepared::stage_authored_character(
                app,
                a_character(5),
                &Default::default(),
            )
            .expect("stages");
        });
        assert_ne!(
            two, five,
            "two compositions whose character has different MAX HEALTH share one \
             PreparedContentIdentity — and the rollback timeline contract \
             compares exactly that identity to decide whether a snapshot from \
             one may be restored into the other",
        );
    }

    #[test]
    fn a_registered_characters_contact_damage_reaches_the_identity() {
        let harmless = identity_of(|app| {
            ambition_characters::prepared::stage_authored_character(
                app,
                a_character(3),
                &Default::default(),
            )
            .expect("stages");
        });
        let hazardous = identity_of(|app| {
            let mut definition = a_character(3);
            definition.contact_damage = Some(ambition_characters::actor::intrinsics::ContactDamage {
                amount: 9,
                ..Default::default()
            });
            ambition_characters::prepared::stage_authored_character(
                app,
                definition,
                &Default::default(),
            )
            .expect("stages");
        });
        assert_ne!(
            harmless, hazardous,
            "whether touching this body hurts is mechanical and did not reach \
             the identity",
        );
    }

    #[test]
    fn an_authored_sheets_declaration_reaches_the_identity() {
        // The declaration TEXT is the canonical material, so two sheets that
        // differ anywhere in what the provider said must differ here.
        fn sheets(
            ron: &str,
        ) -> ambition_sprite_sheet::character::sheets::AuthoredSheets {
            let mut sheets =
                ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
            sheets
                .insert_ron("alpha", ron)
                .expect("the fixture sheet parses");
            sheets
        }
        fn one_record(frame_width: u32) -> String {
            format!(
                r#"[(
                    target: "alpha",
                    image: "alpha.png",
                    label_width: 0,
                    frame_width: {frame_width},
                    frame_height: 32,
                    rows: [
                        (animation: "idle", row_index: 0, frame_count: 1,
                         duration_ms: 100, duration_secs: 0.1, page: 0, rects: []),
                    ],
                )]"#
            )
        }
        let narrow = identity_of(|app| {
            app.insert_resource(sheets(&one_record(24)));
        });
        let wide = identity_of(|app| {
            app.insert_resource(sheets(&one_record(48)));
        });
        assert_ne!(
            narrow, wide,
            "authored sheet records carry the BODY METRICS the collision body is \
             built from and the authored attack geometry the character-sprite \
             road resolves; two compositions with different ones shared an \
             identity",
        );
    }

    #[test]
    fn a_boss_behaviour_reaches_the_identity() {
        // ⭐ REGISTERED THROUGH THE PROVIDER'S OWN API, not by inserting the
        // resource: the road this arm is about is the one a third-party
        // composition actually takes.
        fn register(app: &mut bevy::app::App, contact_damage: i32) {
            use ambition_boss_encounter::BossCatalogAppExt as _;
            // ⭐ THE SHIPPED PROFILE, MUTATED IN ONE FIELD AND RE-SERIALIZED.
            // Hand-writing a minimal profile RON would pin a hand-listed field
            // set that a new required field silently invalidates; round-tripping
            // a real one keeps this arm about the ONE difference it varies.
            let mut profile = ambition_boss_encounter::test_boss_catalog()
                .behavior("clockwork_warden")
                .expect("the shipped fixture catalog has this boss")
                .clone();
            profile.body_damage = contact_damage;
            let behaviors = format!(
                "{{\"clockwork_warden\": {}}}",
                ron::to_string(&profile).expect("a profile that parsed serializes")
            );
            // Every behaviour needs its encounter, so the fixture's own goes
            // along unchanged — it is the CONSTANT half of this pair.
            let encounter = ron::to_string(
                ambition_boss_encounter::test_boss_catalog()
                    .encounter("clockwork_warden")
                    .expect("the shipped fixture catalog has this encounter"),
            )
            .expect("an encounter that parsed serializes");
            app.register_boss_catalog_fragment(
                ambition_boss_encounter::BossCatalogFragment::from_ron(
                    "fixture",
                    // ⚠ NO FALLBACK: naming one requires an encounter for it,
                    // and this arm is about the BEHAVIOUR, not the fallback.
                    None::<String>,
                    None::<String>,
                    &behaviors,
                    &[encounter.as_str()],
                    "{}",
                    Default::default(),
                    Default::default(),
                )
                .expect("the fixture boss fragment parses"),
            );
        }
        let gentle = identity_of(|app| register(app, 1));
        let brutal = identity_of(|app| register(app, 40));
        assert_ne!(
            gentle, brutal,
            "how much a boss HURTS is mechanical and did not reach the identity",
        );
    }
}
