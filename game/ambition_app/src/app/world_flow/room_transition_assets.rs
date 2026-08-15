//! Room-scoped construction/asset preparation, readiness, and prefetch.
//!
//! Ordinary transitions already have one correctness transaction in
//! `room_transition_loading`. This module contributes real Bevy asset evidence
//! to that transaction and performs bounded speculative construction and asset
//! preparation for rooms adjacent to the active room. A prefetched room is never
//! made authoritative; promotion reuses both the frozen construction plan and
//! the exact handles Bevy is already loading.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use bevy::asset::{LoadState, UntypedAssetId};
use bevy::image::TextureAtlasLayout;
use bevy::prelude::{
    AssetServer, Assets, DetectChanges, Handle, Image, Res, ResMut, Resource, Time,
};
use bevy::time::Real;

use ambition_platformer2d::actors::features::RoomContentStagingRegistry;
use ambition_platformer2d::actors::rooms::{InteractionKindSpec, RoomSet, RoomSpec};
use ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog;
use ambition_platformer2d::entity_catalog::placements::PlacementSchema;
use ambition_platformer2d::load::{
    LoadCoordinator, LoadEvent, LoadFailure, LoadWorkState, UnitProgress,
};
use ambition_platformer2d::platformer::lifecycle::{
    ActiveSessionScope, SessionScopeId, SessionWorldRef,
};
use ambition_platformer2d::render::quality::ResolvedVisualQuality;
use ambition_platformer2d::sprite_sheet::boss::BossSpriteAsset;
use ambition_platformer2d::sprite_sheet::character::CharacterSpriteAsset;
use ambition_platformer2d::sprite_sheet::game_assets::{
    ensure_parallax_layers_for_room, EntitySprite, GameAssets, ParallaxLayerAsset, ParallaxTheme,
};

use ambition_platformer2d::runtime::room_transition::{
    set_room_transition_work_state, RoomConstructionPlanPrefetch, RoomTransitionLoadPhase,
    RoomTransitionLoadState,
};

/// One concrete image handle whose successful load contributes to room visual
/// readiness. The label is deterministic and developer-facing; the Bevy asset
/// id is the runtime identity used for readiness polling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoomAssetDependency {
    pub(crate) label: String,
    pub(crate) asset_id: UntypedAssetId,
}

/// Deterministic dependency set for one target room under the currently
/// resolved asset profile and visual-quality handles.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RoomAssetManifest {
    pub(crate) room_id: String,
    pub(crate) dependencies: Vec<RoomAssetDependency>,
}

impl RoomAssetManifest {
    pub(crate) fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.dependencies.len()
    }
}

#[derive(Clone, Debug)]
struct PrefetchedRoomPreparation {
    manifest: RoomAssetManifest,
    /// True once the matching construction plan has been published into the
    /// engine's [`RoomConstructionPlanPrefetch`]. The PLAN itself lives there:
    /// it is an engine artifact keyed by engine identity, and a transition
    /// promotes it without asking this host anything.
    plan_published: bool,
    requested_at: Duration,
    settled_at: Option<Duration>,
}

/// Bounded speculative construction/asset cache for the active room's graph
/// neighbors.
///
/// Entries are valid only for the exact content-epoch/session/source-room
/// tuple. A transition promotes a cache entry only when a freshly-derived target
/// manifest compares
/// equal, so quality changes, hot reload, and asset-handle replacement become
/// safe misses rather than stale promotion.
#[derive(Resource, Default, Debug)]
pub(crate) struct RoomPreparationPrefetchState {
    content_epoch: u64,
    session_scope: Option<SessionScopeId>,
    source_room_id: Option<String>,
    entries: BTreeMap<String, PrefetchedRoomPreparation>,
    pub(crate) hits: u64,
    pub(crate) misses: u64,
    pub(crate) stale_misses: u64,
}

/// Optional presentation-side resources consumed by the simulation-side room
/// transition starter. Bundling them keeps the Bevy system below its parameter
/// arity limit while preserving a clean headless path where every field is
/// absent.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct RoomTransitionAssetContext<'w> {
    pub(crate) assets: Option<ResMut<'w, GameAssets>>,
    pub(crate) catalog: Option<Res<'w, Platformer2dAssetCatalog>>,
    pub(crate) character_catalog: Option<
        Res<'w, ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>,
    >,
    pub(crate) asset_server: Option<Res<'w, AssetServer>>,
    pub(crate) layouts: Option<ResMut<'w, Assets<TextureAtlasLayout>>>,
    pub(crate) quality: Option<Res<'w, ResolvedVisualQuality>>,
    /// The engine's per-character load ledger. Not optional: the engine plugin
    /// installs it unconditionally, so its absence would mean a composition with
    /// no materialization service at all — which the startup audit reports.
    pub(crate) character_load_states:
        Option<ResMut<'w, ambition_platformer2d::actors::character_runtime::CharacterLoadStates>>,
    /// Registered character definitions. A character may be declared ONLY through
    /// `register_character`, in which case this is the only place its sheet is
    /// named — so the synchronous room decode has to consult it or a
    /// registered-only fighter reaches the reveal barrier as a placeholder.
    pub(crate) prepared_characters: Option<
        Res<'w, ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>,
    >,
    /// Sheets this app's providers authored (queue U1) — the other place a
    /// character's sheet can be named, and the only one reachable from outside
    /// this workspace.
    pub(crate) authored_sheets:
        Res<'w, ambition_platformer2d::sprite_sheet::character::sheets::AuthoredSheets>,
    pub(crate) prefetch: Option<ResMut<'w, RoomPreparationPrefetchState>>,
    pub(crate) real_time: Option<Res<'w, Time<Real>>>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RoomAssetReadiness {
    pub(crate) settled: usize,
    pub(crate) total: usize,
    pub(crate) pending: Vec<String>,
    pub(crate) failed: Vec<String>,
}

impl RoomAssetReadiness {
    pub(crate) fn is_ready(&self) -> bool {
        self.pending.is_empty() && self.failed.is_empty()
    }

    fn is_terminal(&self) -> bool {
        self.pending.is_empty()
    }
}

fn add_image_handle(
    by_label: &mut BTreeMap<String, UntypedAssetId>,
    label: impl Into<String>,
    handle: &Handle<Image>,
) {
    by_label.insert(label.into(), UntypedAssetId::from(handle));
}

fn add_character_asset(
    by_label: &mut BTreeMap<String, UntypedAssetId>,
    label: &str,
    asset: &CharacterSpriteAsset,
) {
    if asset.pages.is_empty() {
        add_image_handle(by_label, format!("{label}:page:0"), &asset.texture);
        return;
    }
    for (index, page) in asset.pages.iter().enumerate() {
        add_image_handle(by_label, format!("{label}:page:{index}"), &page.texture);
    }
}

fn add_boss_asset(
    by_label: &mut BTreeMap<String, UntypedAssetId>,
    label: &str,
    asset: &BossSpriteAsset,
) {
    for (index, page) in asset.pages.iter().enumerate() {
        add_image_handle(by_label, format!("{label}:page:{index}"), &page.texture);
    }
}

fn add_named_character(
    by_label: &mut BTreeMap<String, UntypedAssetId>,
    assets: &GameAssets,
    character_id: &str,
) {
    if let Some(asset) = assets.characters.sheet(character_id) {
        add_character_asset(by_label, &format!("character:{character_id}"), asset);
    }
}

/// Name every character this room stages and hand the list to the ENGINE to
/// decode, before the manifest is built — so the reveal barrier waits on those
/// sheets exactly like it waits on parallax themes.
///
/// This function used to BE the materializer, which is the defect the character
/// plan opens on: the step that turns a declared character into loaded art lived
/// in an application crate, so `ambition_demo_mary_o_app` never ran it (Mary-O
/// rendered as a rectangle in her own game) and `ambition_demo_sanic_app`
/// hand-rolled a copy. All that is left here is naming what this room stages,
/// which is content knowledge the host legitimately has.
#[allow(clippy::too_many_arguments)]
pub(crate) fn demand_room_character_sheets(
    room: &RoomSpec,
    staged_actor_names: &[String],
    assets: &mut GameAssets,
    catalog: &Platformer2dAssetCatalog,
    character_catalog: &ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    quality: &ResolvedVisualQuality,
    states: &mut ambition_platformer2d::actors::character_runtime::CharacterLoadStates,
    registry: &ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry,
    // The provider-authored sheets (queue U1) — passed for the same reason the
    // catalog is: this host names what a room stages, and the ENGINE decodes it.
    authored_sheets: &ambition_platformer2d::sprite_sheet::character::sheets::AuthoredSheets,
) {
    let mut names: Vec<&str> = staged_actor_names.iter().map(String::as_str).collect();
    for placement in &room.placements {
        if let PlacementSchema::Interactable(spec) = &placement.schema {
            if let InteractionKindSpec::Npc {
                character_id: Some(character_id),
                ..
            } = &spec.kind
            {
                names.push(character_id);
            }
        }
    }
    // Authored enemies too: `add_room_specific_sprites` adds their sheets to
    // the manifest by name, but a sheet that is still only DECLARED is
    // invisible to that lookup — the enemy would silently render as the
    // goblin/rectangle fallback (GPT 5.6 review finding 4).
    for enemy in &room.enemy_spawns {
        names.push(&enemy.name);
    }
    // Submit DEMAND and let the engine decode it. The host names what this room
    // stages — that is content knowledge it legitimately has — but the decode
    // itself is the engine's, so every application gets it whether or not it has
    // a room-transition step at all.
    let mut demand =
        ambition_platformer2d::actors::character_runtime::CharacterLoadDemand::default();
    demand.request_all(names);
    ambition_platformer2d::actors::character_runtime::materialize_character_demand(
        &mut demand,
        states,
        &mut assets.characters,
        character_catalog,
        authored_sheets,
        registry,
        catalog,
        asset_server,
        layouts,
        Some(&quality.budget),
    );
}

fn add_room_specific_sprites(
    room: &RoomSpec,
    staged_actor_names: &[String],
    assets: &GameAssets,
    by_label: &mut BTreeMap<String, UntypedAssetId>,
) {
    // The static entity sheet set is small, shared by most rooms, and loaded as
    // the sandbox core. Including every present handle makes room reveal wait
    // for the common tiles/features it may instantiate without duplicating the
    // renderer's state-aware sprite-selection policy here.
    for &sprite in EntitySprite::ALL {
        if let Some(handle) = assets.entities.get(sprite) {
            add_image_handle(by_label, format!("entity:{sprite:?}"), handle);
        }
    }

    for prop in &room.props {
        if let Some(asset) = assets.characters.prop_asset_for_kind(&prop.kind) {
            add_character_asset(by_label, &format!("prop:{}", prop.kind), asset);
        }
    }

    for placement in &room.placements {
        match &placement.schema {
            PlacementSchema::Interactable(spec) => {
                if let InteractionKindSpec::Npc {
                    character_id: Some(character_id),
                    ..
                } = &spec.kind
                {
                    add_named_character(by_label, assets, &character_id);
                }
            }
            PlacementSchema::Pickup(spec) => {
                if let Some(kind) = spec.sprite.as_deref() {
                    if let Some(asset) = assets.characters.prop_asset_for_kind(kind) {
                        add_character_asset(by_label, &format!("pickup-prop:{kind}"), asset);
                    }
                }
            }
            PlacementSchema::Hazard(_)
            | PlacementSchema::Chest(_)
            | PlacementSchema::Breakable(_)
            | PlacementSchema::Portal(_) => {}
        }
    }

    // Legacy typed enemy rows and content-staged actors still identify their
    // presentation through the authored display name. The character loader
    // double-keys NPC sheets by catalog id and display name, so this lookup is
    // exact when the content supplied a dedicated sheet and safely falls back
    // otherwise.
    for enemy in &room.enemy_spawns {
        add_named_character(by_label, assets, &enemy.name);
    }
    // Staged actors' own sheets join the barrier: with startup deferral these
    // are materialized just before this walk, so the reveal now waits on them
    // the same way it waits on placement NPCs and parallax themes.
    for name in staged_actor_names {
        add_named_character(by_label, assets, name);
    }

    // No fallback-sheet row: §4.10 deleted the borrowed goblin sheet, so there is
    // no shared handle for the reveal barrier to wait on. An actor with no art of
    // its own draws the marked placeholder and says so.

    if !room.boss_spawns.is_empty() {
        if let Some(asset) = assets.boss.as_ref() {
            add_boss_asset(by_label, "boss:fallback", asset);
        }
        let mut boss_keys = assets.boss_sprites.keys().collect::<Vec<_>>();
        boss_keys.sort();
        for key in boss_keys {
            if let Some(asset) = assets.boss_sprites.get(key) {
                add_boss_asset(by_label, &format!("boss:{key}"), asset);
            }
        }
    }
}

/// Request and describe all currently-known presentation handles needed before
/// revealing `room`.
///
/// Optional catalog entries that resolve to no handle remain legitimate
/// placeholder fallbacks and therefore do not enter the manifest. Once a
/// concrete handle exists, it is activation-critical: it must load successfully
/// before reveal, and a failed load fails the room transaction while the source
/// room remains authoritative.
pub(crate) fn build_room_asset_manifest(
    room: &RoomSpec,
    staged_actor_names: &[String],
    assets: &mut GameAssets,
    catalog: &Platformer2dAssetCatalog,
    character_catalog: &ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    quality: &ResolvedVisualQuality,
    states: &mut ambition_platformer2d::actors::character_runtime::CharacterLoadStates,
    registry: &ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry,
    authored_sheets: &ambition_platformer2d::sprite_sheet::character::sheets::AuthoredSheets,
) -> RoomAssetManifest {
    ensure_parallax_layers_for_room(
        assets,
        catalog,
        asset_server,
        &room.metadata,
        Some(&quality.budget),
    );
    demand_room_character_sheets(
        room,
        staged_actor_names,
        assets,
        catalog,
        character_catalog,
        asset_server,
        layouts,
        quality,
        states,
        registry,
        authored_sheets,
    );

    build_loaded_room_asset_manifest(room, staged_actor_names, assets)
}

/// Describe the handles already selected for an active room without mutating
/// the cache. Direct startup uses this after `load_game_assets` has loaded the
/// active room's parallax theme; room transitions use
/// [`build_room_asset_manifest`] because a target room may need lazy handle
/// creation first.
pub(crate) fn build_loaded_room_asset_manifest(
    room: &RoomSpec,
    staged_actor_names: &[String],
    assets: &GameAssets,
) -> RoomAssetManifest {
    let mut by_label = BTreeMap::new();
    add_room_specific_sprites(room, staged_actor_names, assets, &mut by_label);

    let theme = ParallaxTheme::from_room_metadata(&room.metadata);
    for &layer in ParallaxLayerAsset::ALL {
        if let Some(handle) = assets.parallax_layers.get(theme, layer) {
            add_image_handle(
                &mut by_label,
                format!("parallax:{}:{}", theme.key(), layer.key()),
                handle,
            );
        }
    }

    // Multiple authored names can intentionally resolve to the same handle.
    // Keep one deterministic label per runtime asset id so progress totals are
    // about actual loads rather than aliases.
    let mut seen = Vec::<UntypedAssetId>::new();
    let dependencies = by_label
        .into_iter()
        .filter_map(|(label, asset_id)| {
            if seen.iter().any(|seen_id| seen_id == &asset_id) {
                return None;
            }
            seen.push(asset_id);
            Some(RoomAssetDependency { label, asset_id })
        })
        .collect();

    RoomAssetManifest {
        room_id: room.id.clone(),
        dependencies,
    }
}

pub(crate) fn inspect_room_asset_manifest(
    asset_server: &AssetServer,
    manifest: &RoomAssetManifest,
) -> RoomAssetReadiness {
    let mut readiness = RoomAssetReadiness {
        total: manifest.len(),
        ..Default::default()
    };
    for dependency in &manifest.dependencies {
        if asset_server.is_loaded_with_dependencies(dependency.asset_id.clone()) {
            readiness.settled += 1;
            continue;
        }
        match asset_server.load_state(dependency.asset_id.clone()) {
            LoadState::Failed(_) => {
                readiness.settled += 1;
                readiness.failed.push(dependency.label.clone());
            }
            LoadState::NotLoaded | LoadState::Loading => {
                readiness.pending.push(dependency.label.clone());
            }
            LoadState::Loaded => {
                // The root asset has loaded but one of its dependencies has not.
                readiness.pending.push(dependency.label.clone());
            }
        }
    }
    readiness
}

impl RoomPreparationPrefetchState {
    fn reset_for(
        &mut self,
        content_epoch: u64,
        session_scope: Option<SessionScopeId>,
        source_room_id: &str,
    ) -> bool {
        let changed = self.content_epoch != content_epoch
            || self.session_scope != session_scope
            || self.source_room_id.as_deref() != Some(source_room_id);
        if changed {
            self.entries.clear();
            self.content_epoch = content_epoch;
            self.session_scope = session_scope;
            self.source_room_id = Some(source_room_id.to_string());
        }
        changed
    }

    pub(crate) fn classify_promotion(
        &mut self,
        content_epoch: u64,
        session_scope: Option<SessionScopeId>,
        source_room_id: &str,
        manifest: &RoomAssetManifest,
        now: Option<Duration>,
    ) -> bool {
        self.reset_for(content_epoch, session_scope, source_room_id);
        match self.entries.get(&manifest.room_id) {
            Some(entry) if entry.manifest == *manifest => {
                self.hits = self.hits.saturating_add(1);
                match (now, entry.settled_at) {
                    (Some(now), Some(settled_at)) => {
                        let lead = now.saturating_sub(settled_at);
                        bevy::log::debug!(
                            target: "ambition_platformer2d::room_transition",
                            "promoted settled room asset prefetch for '{}' with {:.1} ms lead",
                            manifest.room_id,
                            lead.as_secs_f64() * 1000.0,
                        );
                    }
                    (Some(now), None) => {
                        let elapsed = now.saturating_sub(entry.requested_at);
                        bevy::log::debug!(
                            target: "ambition_platformer2d::room_transition",
                            "promoted in-flight room asset prefetch for '{}' after {:.1} ms",
                            manifest.room_id,
                            elapsed.as_secs_f64() * 1000.0,
                        );
                    }
                    (None, _) => {
                        bevy::log::debug!(
                            target: "ambition_platformer2d::room_transition",
                            "promoted room asset prefetch for '{}'",
                            manifest.room_id,
                        );
                    }
                }
                true
            }
            Some(_) => {
                self.stale_misses = self.stale_misses.saturating_add(1);
                self.misses = self.misses.saturating_add(1);
                bevy::log::debug!(
                    target: "ambition_platformer2d::room_transition",
                    "discarded stale room asset prefetch for '{}'",
                    manifest.room_id,
                );
                false
            }
            None => {
                self.misses = self.misses.saturating_add(1);
                bevy::log::debug!(
                    target: "ambition_platformer2d::room_transition",
                    "room asset prefetch miss for '{}'",
                    manifest.room_id,
                );
                false
            }
        }
    }
}

/// The manifest for the transition currently in flight, keyed by the engine's
/// transition `sequence`.
///
/// It lives here rather than on `ActiveRoomTransitionLoad` because a
/// `RoomAssetManifest` is a bag of Bevy asset ids under a resolved visual
/// quality — a fact about THIS host's presentation pipeline, which the engine
/// cannot name and has no use for. The engine owns the transaction and its
/// identity; the contributor owns its own evidence.
#[derive(Resource, Default, Debug)]
pub(crate) struct ContributedRoomAssets {
    sequence: Option<u64>,
    manifest: Option<Arc<RoomAssetManifest>>,
}

/// Build the destination room's dependency set the first time the engine hands
/// this host a transition, and report the contributor's opening state.
///
/// This is the half of the old in-`begin_room_transition_load_system` block that
/// only a host can do. The engine declares the asset work item and leaves it
/// `Running` whenever `RoomTransitionAssetContributor` is installed; this system
/// is what installs that marker's promise.
#[allow(clippy::too_many_arguments)]
pub(crate) fn contribute_room_transition_assets_system(
    room_set: SessionWorldRef<RoomSet>,
    mut transitions: ResMut<RoomTransitionLoadState>,
    mut contributed: ResMut<ContributedRoomAssets>,
    mut context: RoomTransitionAssetContext,
    mut loads: ResMut<LoadCoordinator>,
    mut load_events: bevy::prelude::MessageWriter<LoadEvent>,
) {
    let Some(active) = transitions.active.as_mut() else {
        contributed.sequence = None;
        contributed.manifest = None;
        return;
    };
    if contributed.sequence == Some(active.sequence) || active.asset_readiness_complete {
        return;
    }
    contributed.sequence = Some(active.sequence);
    contributed.manifest = None;

    let (
        Some(assets),
        Some(catalog),
        Some(character_catalog),
        Some(asset_server),
        Some(layouts),
        Some(quality),
        Some(character_load_states),
    ) = (
        context.assets.as_deref_mut(),
        context.catalog.as_deref(),
        context.character_catalog.as_deref(),
        context.asset_server.as_deref(),
        context.layouts.as_deref_mut(),
        context.quality.as_deref(),
        context.character_load_states.as_deref_mut(),
    )
    else {
        // The marker promised an answer this host cannot give. Say so instead of
        // stalling the barrier forever.
        active.asset_readiness_complete = true;
        set_room_transition_work_state(
            &mut loads,
            &mut load_events,
            &active.barrier.load_id,
            active.asset_work_id.clone(),
            LoadWorkState::Skipped,
        );
        return;
    };

    let prepared_characters = context
        .prepared_characters
        .as_deref()
        .cloned()
        .unwrap_or_default();

    let Some(target_spec) = room_set.rooms.get(active.target_room) else {
        return;
    };

    #[cfg(not(target_arch = "wasm32"))]
    let manifest_started = std::time::Instant::now();
    let manifest = build_room_asset_manifest(
        target_spec,
        &active.staged_actor_names,
        assets,
        catalog,
        character_catalog,
        asset_server,
        layouts,
        quality,
        character_load_states,
        &prepared_characters,
        &context.authored_sheets,
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        active.asset_manifest_duration = Some(manifest_started.elapsed());
    }
    let now = context.real_time.as_deref().map(|time| time.elapsed());
    if let Some(cache) = context.prefetch.as_deref_mut() {
        let assets_promoted = cache.classify_promotion(
            active.content_epoch,
            active.session_scope,
            &active.source_room_id,
            &manifest,
            now,
        );
        active.prefetch_hit &= assets_promoted;
    }
    let manifest_is_empty = manifest.is_empty();
    let readiness = inspect_room_asset_manifest(asset_server, &manifest);
    active.last_asset_progress = Some((readiness.settled, readiness.total));
    contributed.manifest = Some(Arc::new(manifest));

    if !readiness.failed.is_empty() {
        let detail = format!(
            "room '{}' failed to load {} activation-critical asset(s): {}",
            active.target_room_id,
            readiness.failed.len(),
            readiness.failed.join(", "),
        );
        active.asset_readiness_complete = true;
        set_room_transition_work_state(
            &mut loads,
            &mut load_events,
            &active.barrier.load_id,
            active.asset_work_id.clone(),
            LoadWorkState::Failed(
                LoadFailure::new(
                    "The destination room's visuals could not be loaded.",
                    detail.clone(),
                )
                .retryable(true),
            ),
        );
        bevy::log::error!(target: "ambition_platformer2d::room_transition", "{detail}");
    } else if manifest_is_empty || readiness.is_ready() {
        active.asset_readiness_complete = true;
        active.asset_ready_at = now;
        set_room_transition_work_state(
            &mut loads,
            &mut load_events,
            &active.barrier.load_id,
            active.asset_work_id.clone(),
            LoadWorkState::Complete,
        );
    } else {
        set_room_transition_work_state(
            &mut loads,
            &mut load_events,
            &active.barrier.load_id,
            active.asset_work_id.clone(),
            LoadWorkState::Running {
                progress: Some(UnitProgress::new(
                    readiness.settled as f32,
                    readiness.total.max(1) as f32,
                )),
            },
        );
    }
}

/// Poll the active transition's concrete room dependency set and publish real
/// unit progress into its required load work.
pub(crate) fn poll_room_transition_asset_readiness_system(
    asset_server: Res<AssetServer>,
    time: Res<Time<Real>>,
    mut transitions: ResMut<RoomTransitionLoadState>,
    contributed: Res<ContributedRoomAssets>,
    mut loads: ResMut<LoadCoordinator>,
    mut load_events: bevy::prelude::MessageWriter<LoadEvent>,
) {
    let Some(active) = transitions.active.as_mut() else {
        return;
    };
    if active.phase != RoomTransitionLoadPhase::AwaitingReadiness || active.asset_readiness_complete
    {
        return;
    }
    if contributed.sequence != Some(active.sequence) {
        return;
    }
    let Some(manifest) = contributed.manifest.as_ref() else {
        return;
    };

    let readiness = inspect_room_asset_manifest(&asset_server, manifest);
    let progress_key = (readiness.settled, readiness.total);
    if !readiness.failed.is_empty() {
        let detail = format!(
            "room '{}' failed to load {} activation-critical asset(s): {}",
            active.target_room_id,
            readiness.failed.len(),
            readiness.failed.join(", "),
        );
        set_room_transition_work_state(
            &mut loads,
            &mut load_events,
            &active.barrier.load_id,
            active.asset_work_id.clone(),
            LoadWorkState::Failed(
                LoadFailure::new(
                    "The destination room's visuals could not be loaded.",
                    detail.clone(),
                )
                .retryable(true),
            ),
        );
        active.last_asset_progress = Some(progress_key);
        active.asset_readiness_complete = true;
        bevy::log::error!(target: "ambition_platformer2d::room_transition", "{detail}");
        return;
    }

    if active.last_asset_progress != Some(progress_key) {
        let state = if readiness.is_ready() {
            LoadWorkState::Complete
        } else {
            LoadWorkState::Running {
                progress: Some(UnitProgress::new(
                    readiness.settled as f32,
                    readiness.total.max(1) as f32,
                )),
            }
        };
        set_room_transition_work_state(
            &mut loads,
            &mut load_events,
            &active.barrier.load_id,
            active.asset_work_id.clone(),
            state,
        );
        active.last_asset_progress = Some(progress_key);
    }

    if readiness.is_ready() {
        active.asset_readiness_complete = true;
        active.asset_ready_at.get_or_insert_with(|| time.elapsed());
    }
}

/// Speculatively prepare construction plans and poll exact asset manifests for graph-neighbor
/// rooms. The cache is bounded to the current active room's outgoing neighbors.
/// Promotion is an equality check against a freshly-derived manifest, so stale
/// content or quality variants are never trusted.
#[allow(clippy::too_many_arguments)]
/// **How many one-hop neighbours may have their preparation prefetched.**
///
/// ⛔ **this was unbounded, and it is the launch-time stutter.** (2026-07-30,
/// found from Jon's desktop timeline capture.) The prefetch reaches every room
/// ONE loading zone away and demands that room's whole cast — which is right for
/// a corridor and catastrophic for a hub. `central_hub_main` authors 21 loading
/// zones and `central_hub_basement` 18, so standing in the hub prefetches
/// essentially the entire game:
///
/// ```text
/// staged cast on entering the Ambition route:  162 characters
/// decoded in the 10-15s window:                +157 images, +357.8 MP
/// resident image memory:                       1803 MB
/// frames in that 5s window:                    91   (p99 1372ms, max 1437ms)
/// ```
///
/// Two things make it hurt rather than merely cost:
///
/// * **it is not covered.** The transition path demands the same art behind the
///   load cover (`build_room_asset_manifest` → `demand_room_character_sheets`),
///   where a wait is invisible. Prefetching converts that covered wait into an
///   uncovered multi-hundred-millisecond hitch DURING PLAY — for up to 21 rooms
///   the player may never walk into.
/// * **it grew silently.** Every room wired to the hub added its cast to the
///   cost of standing in the hub, and no instrument watched it: the boot budget
///   measures the title screen and this is entirely post-boot.
///
/// ⚠ **the cap SKIPS a room outright rather than prefetching it partially**, and
/// that is deliberate. A half-prefetched room would cache a manifest that does
/// not name the art it needs, and the transition promotes cached manifests — so
/// the room would reveal with its cast undecoded. Skipping produces an ordinary
/// prefetch MISS, which is the well-tested covered path.
///
/// Four is chosen against the world's real shape, not as a round number: every
/// corridor and lab in `sandbox.ldtk` has at most four exits, so ordinary
/// traversal is unaffected and only the hubs are trimmed.
const NEIGHBOR_PREFETCH_ROOM_BUDGET: usize = 4;

pub(crate) fn prefetch_neighbor_room_preparation_system(
    room_set: SessionWorldRef<RoomSet>,
    content_epoch: Res<ambition_platformer2d::runtime::room_transition::RoomTransitionContentEpoch>,
    placement_lowering: Res<
        ambition_platformer2d::actors::world::placements::PlacementLoweringRegistry,
    >,
    content_staging: Res<RoomContentStagingRegistry>,
    character_catalog: Res<
        ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog,
    >,
    boss_catalog: Res<ambition_platformer2d::actors::boss_encounter::BossCatalog>,
    // ⚠ **PAIRED with the recipes because a Bevy system stops at sixteen
    // params**, the same reason the covered transition path groups them. The
    // authorities travel together anyway: a placement names a character and may
    // name the policy that drives it.
    (construction_recipes, active_binding, brain_profiles, mut plan_prefetch): (
        Res<ambition_platformer2d::actors::construction::ActorConstructionRegistry>,
        Option<Res<ambition_platformer2d::actors::rooms::ActiveContentBinding>>,
        Option<
            Res<ambition_platformer2d::characters::actor::character_catalog::BrainProfileRegistry>,
        >,
        ResMut<RoomConstructionPlanPrefetch>,
    ),
    mut assets: ResMut<GameAssets>,
    catalog: Res<Platformer2dAssetCatalog>,
    asset_server: Res<AssetServer>,
    (mut layouts, mut character_load_states, prepared_characters, authored_sheets): (
        ResMut<Assets<TextureAtlasLayout>>,
        // Grouped with `layouts` to stay under Bevy's SystemParam arity limit.
        ResMut<ambition_platformer2d::actors::character_runtime::CharacterLoadStates>,
        Option<Res<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>>,
        Res<ambition_platformer2d::sprite_sheet::character::sheets::AuthoredSheets>,
    ),
    quality: Res<ResolvedVisualQuality>,
    time: Res<Time<Real>>,
    active_session: Option<Res<ActiveSessionScope>>,
    mut cache: ResMut<RoomPreparationPrefetchState>,
) {
    let empty_registry =
        ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry::default();
    let Some(source_room) = room_set.rooms.get(room_set.active) else {
        cache.entries.clear();
        cache.source_room_id = None;
        return;
    };
    let session_scope = active_session.as_deref().and_then(|scope| scope.current());
    let Some(spawn_scope) =
        ambition_platformer2d::platformer::lifecycle::SessionSpawnScope::for_optional_active_session(
            active_session.as_deref(),
        )
    else {
        cache.entries.clear();
        cache.source_room_id = None;
        return;
    };
    let identity_changed = cache.reset_for(content_epoch.get(), session_scope, &source_room.id);
    let refresh_manifests = identity_changed
        || room_set.is_changed()
        || placement_lowering.is_changed()
        || content_staging.is_changed()
        || character_catalog.is_changed()
        || boss_catalog.is_changed()
        || catalog.is_changed()
        || quality.is_changed();

    // **THE NEIGHBOURHOOD IS BOUNDED**, and the reason is the shape of this
    // world rather than a general principle about prefetching. See
    // [`NEIGHBOR_PREFETCH_ROOM_BUDGET`].
    let all_neighbors = room_set.neighboring_room_indices();
    let skipped_neighbors = all_neighbors
        .len()
        .saturating_sub(NEIGHBOR_PREFETCH_ROOM_BUDGET);
    let neighbor_indices = all_neighbors
        .iter()
        .copied()
        .take(NEIGHBOR_PREFETCH_ROOM_BUDGET)
        .collect::<Vec<_>>();
    if skipped_neighbors > 0 {
        // NOT silent. A cap that quietly drops work reads as "everything is
        // prefetched" to the next person measuring a transition.
        bevy::log::warn_once!(
            target: "ambition_platformer2d::room_transition",
            "room '{}' has {} neighbours; prefetching preparation for the first {} and \
             skipping {}. Those rooms take the ordinary covered transition path instead \
             (correct, just not preloaded). A hub with a large fan-out is the case this \
             budget exists for — see NEIGHBOR_PREFETCH_ROOM_BUDGET.",
            source_room.id,
            all_neighbors.len(),
            neighbor_indices.len(),
            skipped_neighbors,
        );
    }
    let neighbor_ids = neighbor_indices
        .iter()
        .filter_map(|&index| room_set.rooms.get(index))
        .map(|room| room.id.clone())
        .collect::<BTreeSet<_>>();
    cache.entries.retain(|room_id, entry| {
        let keep = neighbor_ids.contains(room_id);
        if !keep && entry.plan_published {
            plan_prefetch.forget(room_id);
        }
        keep
    });

    for index in neighbor_indices {
        let Some(room) = room_set.rooms.get(index) else {
            continue;
        };
        if !refresh_manifests
            && cache
                .entries
                .get(&room.id)
                .is_some_and(|entry| entry.plan_published)
        {
            continue;
        }
        let construction_plan =
            match ambition_platformer2d::actors::rooms::RoomConstructionPlan::prepare_from_parts(
                &room_set,
                index,
                &placement_lowering,
                &content_staging,
                &character_catalog,
                &authored_sheets,
                &boss_catalog,
                spawn_scope,
                // Prefetched plans state the LIVE binding too: if a hot reload
                // moves the session to a new generation before this plan
                // commits, the boundary refuses the stale prefetch — which is
                // exactly the invalidation the cache needs.
                //
                // ⛔⛔ **AND THIS ROAD CARRIED NEITHER THE CAST NOR THE
                // POLICIES**, which is what Jon's browser log was saying at
                // frame rate: *"`goblin` … which this composition has not
                // registered"* for a character that IS registered. An absent
                // prepared registry is an EMPTY cast, not an exemption, so every
                // neighbour containing a character-built body failed preflight,
                // was forgotten, and was re-prepared from scratch on the next
                // frame — a whole `RoomConstructionPlan` per neighbour per
                // frame, thrown away, for as long as you stood there. It also
                // meant the prefetch never covered exactly the rooms that cost
                // the most to prepare.
                ambition_platformer2d::actors::features::ActorConstructionContext::for_room_construction(
                    &construction_recipes,
                    ambition_platformer2d::engine_core::ContentEpoch(content_epoch.get()),
                    active_binding.as_deref(),
                    prepared_characters.as_deref(),
                    brain_profiles.as_deref(),
                ),
            ) {
                Ok(plan) => plan,
                Err(error) => {
                    cache.entries.remove(&room.id);
                    plan_prefetch.forget(&room.id);
                    bevy::log::warn!(
                        target: "ambition_platformer2d::room_transition",
                        "could not prefetch construction for neighbor room '{}': {error}",
                        room.id,
                    );
                    continue;
                }
            };
        let staged_names = construction_plan.content_staged_names();
        let manifest = build_room_asset_manifest(
            room,
            &staged_names,
            &mut assets,
            &catalog,
            &character_catalog,
            &asset_server,
            &mut layouts,
            &quality,
            &mut character_load_states,
            prepared_characters.as_deref().unwrap_or(&empty_registry),
            &authored_sheets,
        );
        let replace = refresh_manifests
            || cache.entries.get(&room.id).map_or(true, |entry| {
                entry.manifest != manifest || !entry.plan_published
            });
        if replace {
            plan_prefetch.publish(&room.id, Arc::new(construction_plan));
            cache.entries.insert(
                room.id.clone(),
                PrefetchedRoomPreparation {
                    manifest,
                    plan_published: true,
                    requested_at: time.elapsed(),
                    settled_at: None,
                },
            );
        }
    }

    for entry in cache.entries.values_mut() {
        if entry.settled_at.is_some() {
            continue;
        }
        if inspect_room_asset_manifest(&asset_server, &entry.manifest).is_terminal() {
            entry.settled_at = Some(time.elapsed());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_equality_is_the_prefetch_promotion_contract() {
        let empty = RoomAssetManifest {
            room_id: "hall".to_string(),
            dependencies: Vec::new(),
        };
        let mut cache = RoomPreparationPrefetchState::default();
        cache.reset_for(1, None, "hub");
        cache.entries.insert(
            "hall".to_string(),
            PrefetchedRoomPreparation {
                manifest: empty.clone(),
                plan_published: false,
                requested_at: Duration::ZERO,
                settled_at: Some(Duration::ZERO),
            },
        );
        assert!(cache.classify_promotion(1, None, "hub", &empty, Some(Duration::ZERO)));
        assert!(
            !cache.classify_promotion(2, None, "hub", &empty, Some(Duration::ZERO)),
            "a new content epoch must invalidate otherwise identical prefetched work",
        );

        let different_room = RoomAssetManifest {
            room_id: "basement".to_string(),
            dependencies: Vec::new(),
        };
        assert!(!cache.classify_promotion(1, None, "hub", &different_room, Some(Duration::ZERO)));
    }
}
