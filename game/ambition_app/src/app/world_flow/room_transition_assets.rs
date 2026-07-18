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
use bevy::prelude::{AssetServer, DetectChanges, Handle, Image, Res, ResMut, Resource, Time};
use bevy::time::Real;

use ambition::actors::features::RoomContentStagingRegistry;
use ambition::actors::rooms::{InteractionKindSpec, RoomSet, RoomSpec};
use ambition::asset_manager::sandbox_assets::SandboxAssetCatalog;
use ambition::entity_catalog::placements::PlacementSchema;
use ambition::load::{LoadCoordinator, LoadEvent, LoadFailure, LoadWorkState, UnitProgress};
use ambition::platformer::lifecycle::{ActiveSessionScope, SessionScopeId, SessionWorldRef};
use ambition::render::quality::ResolvedVisualQuality;
use ambition::sprite_sheet::boss::BossSpriteAsset;
use ambition::sprite_sheet::character::CharacterSpriteAsset;
use ambition::sprite_sheet::game_assets::{
    ensure_parallax_layers_for_room, EntitySprite, GameAssets, ParallaxLayerAsset, ParallaxTheme,
};

use super::room_transition_loading::{
    set_room_transition_work_state, RoomTransitionLoadPhase, RoomTransitionLoadState,
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
    construction_plan: Option<Arc<ambition::actors::rooms::RoomConstructionPlan>>,
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
    pub(crate) catalog: Option<Res<'w, SandboxAssetCatalog>>,
    pub(crate) asset_server: Option<Res<'w, AssetServer>>,
    pub(crate) quality: Option<Res<'w, ResolvedVisualQuality>>,
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
    if let Some(asset) = assets.characters.asset_for_character_id(character_id) {
        add_character_asset(by_label, &format!("character:{character_id}"), asset);
    } else if let Some(asset) = assets.characters.npc_asset_for_name(character_id) {
        add_character_asset(by_label, &format!("character-name:{character_id}"), asset);
    }
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
    for name in staged_actor_names {
        add_named_character(by_label, assets, name);
    }

    if !room.enemy_spawns.is_empty() || !staged_actor_names.is_empty() {
        if let Some(asset) = assets.characters.goblin.as_ref() {
            add_character_asset(by_label, "character-fallback:goblin", asset);
        }
    }

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
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    quality: &ResolvedVisualQuality,
) -> RoomAssetManifest {
    ensure_parallax_layers_for_room(
        assets,
        catalog,
        asset_server,
        &room.metadata,
        Some(&quality.budget),
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

    /// Promote the exact immutable construction artifact prepared for a graph
    /// neighbor. A same-id hot reload, provider/catalog replacement, or session
    /// change becomes a miss because the prepared spec/scope must still match.
    pub(crate) fn promote_construction_plan(
        &mut self,
        content_epoch: u64,
        session_scope: Option<SessionScopeId>,
        source_room_id: &str,
        target: &RoomSpec,
    ) -> Option<Arc<ambition::actors::rooms::RoomConstructionPlan>> {
        self.reset_for(content_epoch, session_scope, source_room_id);
        let entry = self.entries.get(&target.id)?;
        let plan = entry.construction_plan.as_ref()?;
        if !plan.matches_room_spec(target) || plan.session_scope().id() != session_scope {
            return None;
        }
        Some(Arc::clone(plan))
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
                            target: "ambition::room_transition",
                            "promoted settled room asset prefetch for '{}' with {:.1} ms lead",
                            manifest.room_id,
                            lead.as_secs_f64() * 1000.0,
                        );
                    }
                    (Some(now), None) => {
                        let elapsed = now.saturating_sub(entry.requested_at);
                        bevy::log::debug!(
                            target: "ambition::room_transition",
                            "promoted in-flight room asset prefetch for '{}' after {:.1} ms",
                            manifest.room_id,
                            elapsed.as_secs_f64() * 1000.0,
                        );
                    }
                    (None, _) => {
                        bevy::log::debug!(
                            target: "ambition::room_transition",
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
                    target: "ambition::room_transition",
                    "discarded stale room asset prefetch for '{}'",
                    manifest.room_id,
                );
                false
            }
            None => {
                self.misses = self.misses.saturating_add(1);
                bevy::log::debug!(
                    target: "ambition::room_transition",
                    "room asset prefetch miss for '{}'",
                    manifest.room_id,
                );
                false
            }
        }
    }
}

/// Poll the active transition's concrete room dependency set and publish real
/// unit progress into its required load work.
pub(crate) fn poll_room_transition_asset_readiness_system(
    asset_server: Res<AssetServer>,
    time: Res<Time<Real>>,
    mut transitions: ResMut<RoomTransitionLoadState>,
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
    let Some(manifest) = active.asset_manifest.as_ref() else {
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
        bevy::log::error!(target: "ambition::room_transition", "{detail}");
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
pub(crate) fn prefetch_neighbor_room_preparation_system(
    room_set: SessionWorldRef<RoomSet>,
    content_epoch: Res<super::room_transition_loading::RoomTransitionContentEpoch>,
    placement_lowering: Res<ambition::actors::world::placements::PlacementLoweringRegistry>,
    content_staging: Res<RoomContentStagingRegistry>,
    character_catalog: Res<ambition::characters::actor::character_catalog::CharacterCatalog>,
    character_roster: Res<ambition::actors::features::CharacterRoster>,
    boss_catalog: Res<ambition::actors::boss_encounter::BossCatalog>,
    mut assets: ResMut<GameAssets>,
    catalog: Res<SandboxAssetCatalog>,
    asset_server: Res<AssetServer>,
    quality: Res<ResolvedVisualQuality>,
    time: Res<Time<Real>>,
    active_session: Option<Res<ActiveSessionScope>>,
    mut cache: ResMut<RoomPreparationPrefetchState>,
) {
    let Some(source_room) = room_set.rooms.get(room_set.active) else {
        cache.entries.clear();
        cache.source_room_id = None;
        return;
    };
    let session_scope = active_session.as_deref().and_then(|scope| scope.current());
    let Some(spawn_scope) =
        ambition::platformer::lifecycle::SessionSpawnScope::for_optional_active_session(
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
        || character_roster.is_changed()
        || boss_catalog.is_changed()
        || catalog.is_changed()
        || quality.is_changed();

    let neighbor_indices = room_set.neighboring_room_indices();
    let neighbor_ids = neighbor_indices
        .iter()
        .filter_map(|&index| room_set.rooms.get(index))
        .map(|room| room.id.clone())
        .collect::<BTreeSet<_>>();
    cache
        .entries
        .retain(|room_id, _| neighbor_ids.contains(room_id));

    for index in neighbor_indices {
        let Some(room) = room_set.rooms.get(index) else {
            continue;
        };
        if !refresh_manifests
            && cache
                .entries
                .get(&room.id)
                .is_some_and(|entry| entry.construction_plan.is_some())
        {
            continue;
        }
        let construction_plan =
            match ambition::actors::rooms::RoomConstructionPlan::prepare_from_parts(
                &room_set,
                index,
                &placement_lowering,
                &content_staging,
                &character_catalog,
                &character_roster,
                &boss_catalog,
                spawn_scope,
            ) {
                Ok(plan) => plan,
                Err(error) => {
                    cache.entries.remove(&room.id);
                    bevy::log::warn!(
                        target: "ambition::room_transition",
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
            &asset_server,
            &quality,
        );
        let replace = refresh_manifests
            || cache.entries.get(&room.id).map_or(true, |entry| {
                entry.manifest != manifest || entry.construction_plan.is_none()
            });
        if replace {
            cache.entries.insert(
                room.id.clone(),
                PrefetchedRoomPreparation {
                    manifest,
                    construction_plan: Some(Arc::new(construction_plan)),
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
                construction_plan: None,
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
