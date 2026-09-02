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

use bevy::asset::{AssetId, LoadState};
use bevy::image::TextureAtlasLayout;
use bevy::prelude::{
    AssetServer, Assets, DetectChanges, Handle, Image, Res, ResMut, Resource, Time,
};
use bevy::time::Real;

use ambition_platformer2d::actors::features::RoomContentStagingRegistry;
use ambition_platformer2d::asset_manager::image_stages::{AppGpuPreparedImages, RenderWorldPresent};
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
use ambition_platformer2d::world::rooms::{InteractionKindSpec, RoomSet, RoomSpec};

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
    pub(crate) asset_id: AssetId<Image>,
}

/// Deterministic dependency set for one target room under the currently
/// resolved asset profile and visual-quality handles.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RoomAssetManifest {
    pub(crate) room_id: String,
    pub(crate) dependencies: Vec<RoomAssetDependency>,
    /// Labels of presentation the room semantically REQUIRES and that has no
    /// handle to wait on — a spec naming a page its realization does not hold.
    ///
    /// why this lives in the manifest instead of a `Result` out of the
    /// builder. The manifest is already the artifact that travels to both
    /// parties who can refuse a reveal (the transition's contribute/poll pair
    /// and startup loading), it is already the prefetch cache's equality key —
    /// so a truncated realization can never be promoted as equal to a healthy
    /// one — and the only decision site that can act on a failure is the one
    /// that already turns a `readiness.failed` row into
    /// `LoadWorkState::Failed`. A `Result` would have to be caught at build
    /// time, stashed beside the manifest, and re-joined with it at exactly that
    /// site; carrying the unresolved requirement AS a dependency-shaped row
    /// keeps one artifact, one equality contract, and one refusal.
    ///
    /// not the inverse of the sparse-pack fix. A pack page no frame
    /// samples is not a dependency and never enters here — `used_pages()` is
    /// the semantic authority for which pages a character can actually draw
    /// from, and only a page it names can become unresolved.
    pub(crate) unresolved: Vec<String>,
}

impl RoomAssetManifest {
    pub(crate) fn is_empty(&self) -> bool {
        self.dependencies.is_empty() && self.unresolved.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.dependencies.len() + self.unresolved.len()
    }
}

/// Accumulator for one manifest under construction: the handles a room will
/// wait on, plus the required presentation that has no handle at all.
#[derive(Debug, Default)]
struct RoomManifestDraft {
    by_label: BTreeMap<String, AssetId<Image>>,
    /// Sorted and deduplicated so a manifest stays deterministic — the prefetch
    /// cache compares whole manifests for equality.
    unresolved: BTreeSet<String>,
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
    /// How many room preparations this cache has actually performed.
    ///
    /// the other three counters describe PROMOTIONS — what a transition got out of the cache.
    /// None of them describe what putting things in it cost, which is the question asks: a
    /// prefetch that re-prepares its neighbours every frame is a cache that is pure overhead,
    /// and it would look identical from the hit rate.
    pub(crate) preparations: u64,
}

/// Optional presentation-side resources consumed by the simulation-side room
/// transition starter. Bundling them keeps the Bevy system below its parameter
/// arity limit while preserving a clean headless path where every field is
/// absent.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct RoomTransitionAssetContext<'w, 's> {
    pub(crate) assets: Option<ResMut<'w, GameAssets>>,
    pub(crate) catalog: Option<Res<'w, Platformer2dAssetCatalog>>,
    pub(crate) character_catalog: Option<
        Res<'w, ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>,
    >,
    pub(crate) asset_server: Option<Res<'w, AssetServer>>,
    /// Whether THIS App has a render world, so the GPU-upload readiness term
    /// asks about a GPU that exists. ⛔ Absent means no render world: the
    /// resource is inserted only by the App that installs the census's render
    /// systems, and a headless sibling in the same process must not inherit it.
    pub(crate) render_world: Option<Res<'w, RenderWorldPresent>>,
    /// What THIS App has actually uploaded. Beside `render_world` because they
    /// are one question in two halves — does this App draw, and has it drawn
    /// THIS image — and because the process ledger can answer neither.
    pub(crate) prepared_here: Option<Res<'w, AppGpuPreparedImages>>,
    /// Main-world images are the readiness authority for handles that were
    /// inserted directly instead of requested through `AssetServer`.
    pub(crate) images: Option<Res<'w, Assets<Image>>>,
    pub(crate) layouts: Option<ResMut<'w, Assets<TextureAtlasLayout>>>,
    pub(crate) quality: Option<Res<'w, ResolvedVisualQuality>>,
    /// The engine's per-character load ledger. Not optional: the engine plugin
    /// installs it unconditionally, so its absence would mean a composition with
    /// no materialization service at all — which the startup audit reports.
    pub(crate) character_load_states:
        Option<ResMut<'w, ambition_platformer2d::actors::character_runtime::CharacterLoadStates>>,
    /// The engine's GLOBAL demand, drained one character per frame by
    /// `materialize_demanded_character_sheets`. The transition hands it the
    /// cast the per-frame ration did not realize on the transition frame.
    pub(crate) character_load_demand:
        Option<ResMut<'w, ambition_platformer2d::actors::character_runtime::CharacterLoadDemand>>,
    /// Registered character definitions. A character may be declared ONLY through
    /// `register_character`, in which case this is the only place its sheet is
    /// named — so the synchronous room decode has to consult it or a
    /// registered-only fighter reaches the reveal barrier as a placeholder.
    pub(crate) prepared_characters:
        Option<Res<'w, ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>>,
    /// Sheets this app's providers authored — the other place a
    /// character's sheet can be named, and the only one reachable from outside
    /// this workspace.
    /// The boss catalog, for the boss sheets a boss room demands on preparation.
    pub(crate) boss_catalog: Option<Res<'w, ambition_platformer2d::boss_encounter::BossCatalog>>,
    /// What the PLAYER population and driven bodies wear: the characters a
    /// room's placements never list but that travel with the transition. Not
    /// every body — every NPC wears its character too, and asking for all of
    /// them re-demanded the whole gallery at Full on the way out (measured).
    pub(crate) worn: bevy::prelude::Query<
        'w,
        's,
        &'static ambition_platformer2d::characters::actor::WornCharacter,
        bevy::prelude::Or<(
            bevy::prelude::With<ambition_platformer2d::platformer::markers::PlayerEntity>,
            bevy::prelude::With<ambition_platformer2d::characters::control::DrivingParticipant>,
        )>,
    >,
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
    draft: &mut RoomManifestDraft,
    label: impl Into<String>,
    handle: &Handle<Image>,
) {
    // A default handle is a placeholder, not a load request. Sparse packed
    // character sheets intentionally leave unused page slots defaulted so the
    // animator can keep indexing `pages[frame_page]` without decoding pages it
    // never samples. Putting that placeholder into an activation manifest makes
    // the barrier wait forever: AssetServer quite correctly reports that no
    // such load exists and nothing can ever advance it.
    if handle == &Handle::default() {
        return;
    }
    draft.by_label.insert(label.into(), handle.id());
}

fn add_character_asset(draft: &mut RoomManifestDraft, label: &str, asset: &CharacterSpriteAsset) {
    if asset.pages.is_empty() {
        add_image_handle(draft, format!("{label}:page:0"), &asset.texture);
        return;
    }

    // `pages` is indexed by the source page number and therefore includes
    // placeholder slots for sparse packed sheets. The sheet spec is the
    // semantic authority for which pages any frame can actually draw from.
    // Manifest only those pages; an unused pack page is not a presentation
    // dependency of this character.
    for index in asset.spec.used_pages() {
        // Logging and continuing omitted the character from the barrier entirely, so the
        // barrier could report Ready and reveal the room with missing presentation.
        let page = asset.pages.get(index as usize);
        let texture = page.map(|page| &page.texture);
        match texture {
            Some(texture) if texture != &Handle::default() => {
                add_image_handle(draft, format!("{label}:page:{index}"), texture);
            }
            _ => {
                let reason = if page.is_some() {
                    "its realization holds no image for that page"
                } else {
                    "its realization has fewer page slots than that"
                };
                bevy::log::error!(
                    target: "ambition_platformer2d::room_transition",
                    "character asset '{label}' draws from page {index}, but {reason} ({} slot(s))",
                    asset.pages.len(),
                );
                draft
                    .unresolved
                    .insert(format!("{label}:page:{index} (required, unrealized)"));
            }
        }
    }
}

fn add_boss_asset(draft: &mut RoomManifestDraft, label: &str, asset: &BossSpriteAsset) {
    for (index, page) in asset.pages.iter().enumerate() {
        add_image_handle(draft, format!("{label}:page:{index}"), &page.texture);
    }
}

fn add_named_character(draft: &mut RoomManifestDraft, assets: &GameAssets, character_id: &str) {
    if let Some(asset) = assets.characters.sheet(character_id) {
        add_character_asset(draft, &format!("character:{character_id}"), asset);
    }
}

/// Every character token the destination room asks art for: the plan's staged
/// actors, the placements' catalog NPCs, and the authored enemies. ONE list,
/// used both to DEMAND (`demand_room_character_sheets`) and to WAIT
/// (`inspect_demanded_characters`) — two lists would let the barrier wait on
/// something other than what was asked for.
pub(crate) fn room_character_tokens(room: &RoomSpec, staged_actor_names: &[String]) -> Vec<String> {
    let mut names: Vec<String> = staged_actor_names.to_vec();
    for placement in &room.placements {
        if let PlacementSchema::Interactable(spec) = &placement.schema {
            if let InteractionKindSpec::Npc {
                character_id: Some(character_id),
                ..
            } = &spec.kind
            {
                names.push(character_id.clone());
            }
        }
    }
    // Authored enemies too: a sheet that is still only DECLARED is invisible
    // to the manifest's by-name lookup, so the enemy must be demanded here.
    for enemy in &room.enemy_spawns {
        names.push(enemy.name.clone());
    }
    names.sort();
    names.dedup();
    names
}

/// Fold the DEMANDED-BUT-NOT-REALIZED characters into a readiness answer.
///
/// ⛔⛔ THE BARRIER WAITED ON THE PAGES OF REALIZED SHEETS, AND LOADS ARE
/// RATIONED TO ONE CHARACTER PER FRAME. `materialize_character_demand` stages
/// every token at once but REALIZES one per frame
/// (`MAX_CHARACTERS_MATERIALIZED_PER_FRAME`), and a sheet the table only
/// DECLARES contributes no handle to the manifest — so on the transition frame
/// the manifest held one character's pages, the barrier waited 3 ms for them,
/// and the hall's other 111 arrived in the open over three seconds as nine
/// frames of 89-355 ms (`desktop-timeline-run-20260902T015909Z`). The comment
/// on that bound said the curtain would stay down because
/// `character_reveal_ready` blocks on unsettled tokens; nothing in the ROOM
/// barrier ever called it. This is that call.
///
/// A token whose load reached a terminal outcome is settled either way: art
/// that cannot load is the sprites system's placeholder, not a reveal that
/// never comes.
pub(crate) fn inspect_demanded_characters(
    tokens: &[String],
    assets: &GameAssets,
    states: Option<&ambition_platformer2d::actors::character_runtime::CharacterLoadStates>,
    readiness: &mut RoomAssetReadiness,
) {
    use ambition_platformer2d::sprite_sheet::character::CharacterSheetState;
    for token in tokens {
        match assets.characters.sheet_state(token) {
            // Its pages are in the manifest (rebuilt as sheets realize).
            CharacterSheetState::Ready(_) => {}
            CharacterSheetState::Declared { character_id } => {
                readiness.total += 1;
                let terminal = states.is_some_and(|states| {
                    states.outcome(character_id).is_some() || states.outcome(token).is_some()
                });
                if terminal {
                    readiness.settled += 1;
                } else {
                    readiness
                        .pending
                        .push(format!("character:{token} (not yet decoded)"));
                }
            }
            // Nothing declares it; nothing will ever arrive. The sprites system
            // names the typo once. Not counted, so it cannot hold a reveal.
            CharacterSheetState::Unknown => {}
        }
    }
}

/// How many of `tokens` the table holds a realized sheet for.
pub(crate) fn realized_character_count(tokens: &[String], assets: &GameAssets) -> usize {
    tokens
        .iter()
        .filter(|token| assets.characters.sheet(token).is_some())
        .count()
}

/// Name every character this room stages and hand the list to the ENGINE to
/// decode, before the manifest is built — so the reveal barrier waits on those
/// sheets exactly like it waits on parallax themes.
///
/// All that is left here is naming what this room stages, which is content knowledge the host
/// legitimately has.
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
    registry: &ambition_platformer2d::characters::prepared::PreparedCharacterRegistry,
    // The provider-authored sheets — passed for the same reason the
    // catalog is: this host names what a room stages, and the ENGINE decodes it.
    authored_sheets: &ambition_platformer2d::sprite_sheet::character::sheets::AuthoredSheets,
    // Characters some body WEARS right now. They go everywhere the body goes,
    // so a retire must keep them — the room's placements never list the player.
    worn: &[String],
    // `Some(owners)` for the transition INTO this room (covered): every
    // realization whose character is not in `owners` — this room's cast, the
    // worn ones, the one-hop neighbours' casts — is retired here, under the
    // cover, and leaves memory with the room it was for. `None` for a
    // neighbour PREFETCH, which runs in the open: retiring the current room's
    // sheets there would draw placeholders live.
    retire_all_but: Option<&RoomResidencyOwners>,
) -> RoomCharacterRemainder {
    let names = room_character_tokens(room, staged_actor_names);
    // Every character is realized at the user's tier; the room has no say
    // (Jon, 2026-09-02: no lower tier for gallery previews). What a commit
    // decides is OWNERSHIP: who stays resident.
    let retired = match retire_all_but {
        Some(owners) => assets.characters.retire_realizations_except(&owners.0),
        None => Default::default(),
    };
    if !retired.is_empty() {
        bevy::log::info!(
            target: "ambition_platformer2d::room_transition",
            "room '{}': retired {} character realization(s) no room, body or neighbour \
             owns before the reveal",
            room.id,
            retired.len(),
        );
    }
    // Submit DEMAND and let the engine decode it. The host names what this room
    // stages — that is content knowledge it legitimately has — but the decode
    // itself is the engine's, so every application gets it whether or not it has
    // a room-transition step at all.
    let mut demand =
        ambition_platformer2d::actors::character_runtime::CharacterLoadDemand::default();
    demand.request_all(names.iter().map(String::as_str));
    // A retired sheet this room or a worn body needs comes straight back (a
    // quality change between two commits can leave one at the wrong tier);
    // everything else stays retired: their actors left with the room, and the
    // next room that places one demands it then.
    let wanted: std::collections::BTreeSet<String> = names
        .iter()
        .chain(worn.iter())
        .map(|token| {
            ambition_platformer2d::actors::character_runtime::canonical_character_id(
                registry,
                character_catalog,
                token,
            )
            .to_string()
        })
        .collect();
    let retired_but_wanted: Vec<String> = retired
        .into_iter()
        .filter(|id| wanted.contains(id))
        .collect();
    demand.request_all(retired_but_wanted.iter().map(String::as_str));
    ambition_platformer2d::actors::character_runtime::materialize_character_demand(
        &mut demand,
        states,
        &mut assets.characters,
        &mut assets.fx,
        character_catalog,
        authored_sheets,
        registry,
        catalog,
        asset_server,
        layouts,
        Some(&quality.budget),
    );
    // ⛔⛔ THE REMAINDER USED TO DIE HERE. `materialize_character_demand` STAGES
    // every token but REALIZES at most `MAX_CHARACTERS_MATERIALIZED_PER_FRAME`
    // per call, and this `demand` was a local that went out of scope — so a
    // room's cast beyond the first character was never loaded by the transition
    // at all. Those characters were loaded later, one per frame, when their
    // actors spawned and demanded them through the GLOBAL demand: after the
    // reveal, in the open. Measured on the host 2026-09-02 as 111 placeholder
    // rectangles at the hall's reveal and 434 MP arriving over three seconds.
    // The caller forwards this remainder into the global `CharacterLoadDemand`
    // and the reveal barrier waits on it (`inspect_demanded_characters`).
    RoomCharacterRemainder {
        tokens: demand.pending().map(str::to_string).collect(),
    }
}

/// What a room's demand could not realize on the frame it was made.
#[derive(Debug, Default)]
pub(crate) struct RoomCharacterRemainder {
    pub(crate) tokens: Vec<String>,
}

impl RoomCharacterRemainder {
    /// Hand the remainder to the engine's global demand.
    pub(crate) fn forward_into(
        &self,
        demand: &mut ambition_platformer2d::actors::character_runtime::CharacterLoadDemand,
    ) {
        demand.request_all(self.tokens.iter().map(String::as_str));
    }
}

/// The character ids that stay resident across a room commit: the destination's
/// cast, whatever a body wears, and the one-hop neighbours' placed casts (the
/// prefetch decodes those in the open, and retiring them would only make it
/// decode them again). A resident character page belongs to a realization and
/// a realization belongs to one of these owners; anything else leaves with the
/// room it was for. Canonical ids, because the table is retired by id.
pub(crate) struct RoomResidencyOwners(pub(crate) std::collections::BTreeSet<String>);

impl RoomResidencyOwners {
    pub(crate) fn for_room(
        room_set: &RoomSet,
        room_index: usize,
        staged_actor_names: &[String],
        worn: &[String],
        registry: &ambition_platformer2d::characters::prepared::PreparedCharacterRegistry,
        character_catalog: &ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog,
    ) -> Self {
        let mut tokens: Vec<String> = match room_set.rooms.get(room_index) {
            Some(room) => room_character_tokens(room, staged_actor_names),
            None => Vec::new(),
        };
        tokens.extend(worn.iter().cloned());
        for index in room_set.neighboring_room_indices_of(room_index) {
            tokens.extend(room_character_tokens(&room_set.rooms[index], &[]));
        }
        Self(
            tokens
                .iter()
                .map(|token| {
                    ambition_platformer2d::actors::character_runtime::canonical_character_id(
                        registry,
                        character_catalog,
                        token,
                    )
                    .to_string()
                })
                .collect(),
        )
    }
}

fn add_room_specific_sprites(
    room: &RoomSpec,
    staged_actor_names: &[String],
    assets: &GameAssets,
    draft: &mut RoomManifestDraft,
) {
    // The static entity sheet set is small, shared by most rooms, and loaded as
    // the sandbox core. Including every present handle makes room reveal wait
    // for the common tiles/features it may instantiate without duplicating the
    // renderer's state-aware sprite-selection policy here.
    for &sprite in EntitySprite::ALL {
        if let Some(handle) = assets.entities.get(sprite) {
            add_image_handle(draft, format!("entity:{sprite:?}"), handle);
        }
    }

    for prop in &room.props {
        if let Some(asset) = assets.characters.prop_asset_for_kind(&prop.kind) {
            add_character_asset(draft, &format!("prop:{}", prop.kind), asset);
        }
    }

    // Every vfx sheet the process has DEMANDED so far: the core set at boot,
    // plus the sheets a realized character owns (demanded the frame its sprite
    // realized, `character_sprites::demand_character_fx_sheets`). A fighter
    // revealed before its own effects decoded draws its first hit blank; the
    // manifest is rebuilt whenever a realization lands, so a late-realizing
    // cast member's sheet joins the barrier too.
    for target in assets.fx.targets() {
        if let Some(asset) = assets.fx.get(target) {
            add_character_asset(draft, &format!("fx:{target}"), asset);
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
                    add_named_character(draft, assets, &character_id);
                }
            }
            PlacementSchema::Pickup(spec) => {
                if let Some(kind) = spec.sprite.as_deref() {
                    if let Some(asset) = assets.characters.prop_asset_for_kind(kind) {
                        add_character_asset(draft, &format!("pickup-prop:{kind}"), asset);
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
        add_named_character(draft, assets, &enemy.name);
    }
    // Staged actors' own sheets join the barrier: with startup deferral these
    // are materialized just before this walk, so the reveal now waits on them
    // the same way it waits on placement NPCs and parallax themes.
    for name in staged_actor_names {
        add_named_character(draft, assets, name);
    }

    // No fallback-sheet row: §4.10 deleted the borrowed goblin sheet, so there is
    // no shared handle for the reveal barrier to wait on. An actor with no art of
    // its own draws the marked placeholder and says so.

    if !room.boss_spawns.is_empty() {
        if let Some(asset) = assets.boss.as_ref() {
            add_boss_asset(draft, "boss:fallback", asset);
        }
        let mut boss_keys = assets.boss_sprites.keys().collect::<Vec<_>>();
        boss_keys.sort();
        for key in boss_keys {
            if let Some(asset) = assets.boss_sprites.get(key) {
                add_boss_asset(draft, &format!("boss:{key}"), asset);
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
    registry: &ambition_platformer2d::characters::prepared::PreparedCharacterRegistry,
    authored_sheets: &ambition_platformer2d::sprite_sheet::character::sheets::AuthoredSheets,
    boss_catalog: Option<&ambition_platformer2d::boss_encounter::BossCatalog>,
    worn: &[String],
    retire_all_but: Option<&RoomResidencyOwners>,
) -> (RoomAssetManifest, RoomCharacterRemainder) {
    ensure_parallax_layers_for_room(
        assets,
        catalog,
        asset_server,
        &room.metadata,
        Some(&quality.budget),
    );
    // A room that authors a boss demands the dedicated boss sheets HERE, the
    // first time such a room is prepared — not at boot for every room (asset
    // open work 2: 30 MP resident in the hall for bosses it does not have).
    // The manifest below then lists them and the reveal waits on them.
    if !room.boss_spawns.is_empty() {
        if let Some(boss_catalog) = boss_catalog {
            let keys = ambition_platformer2d::actors::assets::game_assets::boss_sheet_keys_for_room(
                room,
                boss_catalog,
            );
            ambition_platformer2d::actors::assets::game_assets::ensure_boss_sheets_loaded(
                assets,
                boss_catalog,
                Some(&keys),
                catalog,
                asset_server,
                layouts,
                Some(&quality.budget),
            );
        }
    }
    let remainder = demand_room_character_sheets(
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
        worn,
        retire_all_but,
    );
    (
        build_loaded_room_asset_manifest(room, staged_actor_names, assets),
        remainder,
    )
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
    let mut draft = RoomManifestDraft::default();
    add_room_specific_sprites(room, staged_actor_names, assets, &mut draft);

    let theme = ParallaxTheme::from_room_metadata(&room.metadata);
    for &layer in ParallaxLayerAsset::ALL {
        if let Some(handle) = assets.parallax_layers.get(theme, layer) {
            add_image_handle(
                &mut draft,
                format!("parallax:{}:{}", theme.key(), layer.key()),
                handle,
            );
        }
    }

    // Multiple authored names can intentionally resolve to the same handle.
    // Keep one deterministic label per runtime asset id so progress totals are
    // about actual loads rather than aliases.
    let mut seen = Vec::<AssetId<Image>>::new();
    let dependencies = draft
        .by_label
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
        unresolved: draft.unresolved.into_iter().collect(),
    }
}

pub(crate) fn inspect_room_asset_manifest(
    asset_server: &AssetServer,
    images: Option<&Assets<Image>>,
    // ⛔ THIS App's fact, not the process's. The ledger is a `static` shared by
    // every App in the process; only the caller knows whether ITS App has a
    // render world that will ever stamp stage 3.
    render_world: RenderWorldPresent,
    // ⛔ AND THIS App's PREPARED SET, for the same reason one step further in.
    // `RenderWorldPresent` says whether this App draws; this says what it has
    // actually uploaded. The process ledger cannot answer the second question
    // either — asset ids are App-local and collide across Apps, so a sibling
    // App's upload was able to satisfy this App's reveal.
    prepared_here: Option<&AppGpuPreparedImages>,
    manifest: &RoomAssetManifest,
) -> RoomAssetReadiness {
    let mut readiness = RoomAssetReadiness {
        total: manifest.len(),
        ..Default::default()
    };
    // A required page with no realization can never settle, so it is terminal
    // the moment the manifest is built: counted as settled (nothing will ever
    // arrive for it) AND failed (the reveal must be refused). Waiting on it
    // instead would resurrect the permanent spinner this file was repaired for.
    for label in &manifest.unresolved {
        readiness.settled += 1;
        readiness.failed.push(label.clone());
    }
    for dependency in &manifest.dependencies {
        // `AssetServer` is authoritative only for handles it owns. A directly
        // inserted/procedural image has no server load state; for that case the main-world
        // `Assets<Image>` collection is the readiness authority.
        let inserted = match asset_server.get_load_state(dependency.asset_id) {
            Some(_) if asset_server.is_loaded_with_dependencies(dependency.asset_id) => true,
            Some(LoadState::Failed(_)) => {
                readiness.settled += 1;
                readiness.failed.push(dependency.label.clone());
                continue;
            }
            Some(LoadState::NotLoaded | LoadState::Loading | LoadState::Loaded) => false,
            None => images.is_some_and(|images| images.contains(dependency.asset_id)),
        };
        if !inserted {
            readiness.pending.push(dependency.label.clone());
            continue;
        }
        // Decoded and inserted. ⭐ AND UPLOADED, when a render world is there
        // to upload: a page whose GPU copy is still owed would otherwise be
        // prepared on the first frame AFTER the cover lifts — measured as every
        // sheet of the hall's reveal in one render frame. Waiting here turns
        // that frame into cover time. Headless (no render world) the term is
        // always false; see `AppGpuPreparedImages::is_awaiting_gpu`.
        //
        // ⛔ THE APP-LOCAL SET DECIDES; THE LEDGER ONLY MIRRORS. A missing set
        // beside a present render world is a composition the census plugin does
        // not build — it inserts both together — so the fallback exists to keep
        // today's behaviour rather than to be relied on, and it is the global
        // answer with the global flaw.
        let awaiting = match prepared_here {
            Some(prepared) => prepared.is_awaiting_gpu(dependency.asset_id.untyped(), render_world),
            None => ambition_platformer2d::asset_manager::image_stages::ledger()
                .is_awaiting_gpu(dependency.asset_id.untyped(), render_world),
        };
        if awaiting {
            readiness
                .pending
                .push(format!("{} (gpu upload)", dependency.label));
        } else {
            readiness.settled += 1;
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
    /// The destination room and the plan's staged names, kept so the poll can
    /// REBUILD the manifest as rationed sheets realize (a sheet realized after
    /// the first build has pages the first manifest never saw).
    room: Option<Arc<RoomSpec>>,
    staged_actor_names: Vec<String>,
    /// Every character token the transition demanded; see `inspect_demanded_characters`.
    demanded_characters: Vec<String>,
    /// How many of `demanded_characters` were realized when `manifest` was built.
    realized_at_build: usize,
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
        contributed.room = None;
        contributed.demanded_characters.clear();
        return;
    };
    if contributed.sequence == Some(active.sequence) || active.asset_readiness_complete {
        return;
    }
    contributed.sequence = Some(active.sequence);
    contributed.manifest = None;

    let worn: Vec<String> = context
        .worn
        .iter()
        .map(|worn| worn.0.as_str().to_string())
        .collect();
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

    // the browser recorded NOTHING here, and this is the burst that most
    // needed measuring: `build_room_asset_manifest` STAGES the whole cast
    // synchronously (Hall of Characters stages 129 distinct ones) and realizes
    // them one per frame from then on.
    // `bevy::platform::time::Instant` is sub-frame on wasm and native alike.
    let manifest_started = bevy::platform::time::Instant::now();
    let owners = RoomResidencyOwners::for_room(
        &room_set,
        active.target_room,
        &active.staged_actor_names,
        &worn,
        &prepared_characters,
        character_catalog,
    );
    let (manifest, remainder) = build_room_asset_manifest(
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
        context.boss_catalog.as_deref(),
        &worn,
        Some(&owners),
    );
    active.asset_manifest_duration = Some(manifest_started.elapsed());
    // The characters the ration did not realize this frame: hand them to the
    // engine's global demand so `materialize_demanded_character_sheets` loads
    // them one per frame BEHIND the cover, which the barrier below holds until
    // they are in.
    if let Some(demand) = context.character_load_demand.as_deref_mut() {
        remainder.forward_into(demand);
    } else if !remainder.tokens.is_empty() {
        bevy::log::warn!(
            target: "ambition_platformer2d::room_transition",
            "room '{}': {} character(s) beyond the per-frame ration have no global \
             CharacterLoadDemand to be handed to; they will load when their actors spawn",
            active.target_room_id,
            remainder.tokens.len(),
        );
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
    let demanded = room_character_tokens(target_spec, &active.staged_actor_names);
    let mut readiness = inspect_room_asset_manifest(
        asset_server,
        context.images.as_deref(),
        RenderWorldPresent::from_option(context.render_world.as_deref()),
        context.prepared_here.as_deref(),
        &manifest,
    );
    inspect_demanded_characters(
        &demanded,
        assets,
        Some(&*character_load_states),
        &mut readiness,
    );
    // Empty means NOTHING to wait for — including no character still decoding.
    let manifest_is_empty = manifest.is_empty() && readiness.total == 0;
    active.observe_asset_progress(readiness.settled, readiness.total, now.unwrap_or_default());
    contributed.realized_at_build = realized_character_count(&demanded, assets);
    contributed.room = Some(Arc::new(target_spec.clone()));
    contributed.staged_actor_names = active.staged_actor_names.clone();
    contributed.demanded_characters = demanded;
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
    images: Option<Res<Assets<Image>>>,
    render_world: Option<Res<RenderWorldPresent>>,
    prepared_here: Option<Res<AppGpuPreparedImages>>,
    assets: Option<Res<GameAssets>>,
    character_load_states: Option<
        Res<ambition_platformer2d::actors::character_runtime::CharacterLoadStates>,
    >,
    time: Res<Time<Real>>,
    mut transitions: ResMut<RoomTransitionLoadState>,
    mut contributed: ResMut<ContributedRoomAssets>,
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
    if contributed.manifest.is_none() {
        return;
    }
    // Sheets realize one per frame AFTER the manifest was built, and each brings
    // page handles the first build never saw. Rebuild the (non-mutating)
    // description whenever the realized count moved, so the barrier waits on
    // those pages too and not only on "is it realized yet".
    if let (Some(assets), Some(room)) = (assets.as_deref(), contributed.room.clone()) {
        let realized = realized_character_count(&contributed.demanded_characters, assets);
        if realized != contributed.realized_at_build {
            let manifest =
                build_loaded_room_asset_manifest(&room, &contributed.staged_actor_names, assets);
            contributed.manifest = Some(Arc::new(manifest));
            contributed.realized_at_build = realized;
        }
    }
    let Some(manifest) = contributed.manifest.as_ref() else {
        return;
    };
    let mut readiness = inspect_room_asset_manifest(
        &asset_server,
        images.as_deref(),
        RenderWorldPresent::from_option(render_world.as_deref()),
        prepared_here.as_deref(),
        manifest,
    );
    if let Some(assets) = assets.as_deref() {
        inspect_demanded_characters(
            &contributed.demanded_characters,
            assets,
            character_load_states.as_deref(),
            &mut readiness,
        );
    }
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
        active.observe_asset_progress(readiness.settled, readiness.total, time.elapsed());
        active.asset_readiness_complete = true;
        bevy::log::error!(target: "ambition_platformer2d::room_transition", "{detail}");
        return;
    }

    if active.observe_asset_progress(readiness.settled, readiness.total, time.elapsed()) {
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
    }

    if readiness.is_ready() {
        active.asset_readiness_complete = true;
        active.asset_ready_at.get_or_insert_with(|| time.elapsed());
        return;
    }

    // SAY WHAT IT IS WAITING FOR. `readiness.pending` names every activation-critical asset
    // that has not settled, and this poll computed it and threw it away every frame while the
    // foreground showed 99% — a number that means only *"not Ready"*, because
    // `LoadPresentationModel` clamps an un-Ready barrier to `0.999`.
    //
    // once per stall, not once per frame. The names are stable while the
    // barrier is stuck, so repeating them is the log spam this file has been
    // burned by before; a barrier that starts moving again clears the flag above
    // and earns a fresh report if it stalls again.
    let stalled_for = active
        .asset_progress_since
        .map(|since| time.elapsed().saturating_sub(since))
        .unwrap_or_default();
    if active.asset_stall_report.is_none() && stalled_for >= ASSET_READINESS_STALL_REPORT {
        let report = asset_stall_report(
            &active.target_room_id,
            stalled_for,
            &readiness.pending,
            readiness.total,
        );
        bevy::log::warn!(target: "ambition_platformer2d::room_transition", "{report}");
        active.asset_stall_report = Some(report);
    }
}

/// The explanation a stalled barrier owes: the room, how long, and the NAMES
/// of what is outstanding — a stall report that says only "still waiting" is
/// the 99% problem with more words. Pure, so the naming contract is tested
/// without a composition whose loads never finish (there is none: images
/// decode in every host now that the no-window builder finishes its plugins).
pub(crate) fn asset_stall_report(
    target_room_id: &str,
    stalled_for: Duration,
    pending: &[String],
    total: usize,
) -> String {
    const NAMED: usize = 12;
    let named = pending
        .iter()
        .take(NAMED)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let and_more = pending.len().saturating_sub(NAMED);
    format!(
        "room '{target_room_id}' has been waiting {:.1}s for {} of {total} activation-critical \
         asset(s) and has not settled one in that time. Still pending: {named}{}",
        stalled_for.as_secs_f32(),
        pending.len(),
        if and_more > 0 {
            format!(" (+{and_more} more)")
        } else {
            String::new()
        },
    )
}

/// How long a room's asset barrier may sit at the SAME settled count before it
/// owes an explanation.
///
/// not a timeout — nothing is cancelled and no transition fails. A slow
/// connection legitimately spends this long on a large room, and the report is
/// how a maintainer tells that apart from a barrier that will never move. Chosen
/// well above an ordinary covered transition (sub-second on a warm desktop) so a
/// healthy load never files one.
const ASSET_READINESS_STALL_REPORT: Duration = Duration::from_secs(5);

/// Speculatively prepare construction plans and poll exact asset manifests for graph-neighbor
/// rooms. The cache is bounded to the current active room's outgoing neighbors.
/// Promotion is an equality check against a freshly-derived manifest, so stale
/// content or quality variants are never trusted.
#[allow(clippy::too_many_arguments)]
/// Maximum number of one-hop rooms prefetched from the active room.
///
/// Excess neighbors are skipped as complete rooms rather than partially
/// prefetched because cached manifests are promoted only when complete. Four
/// covers ordinary corridor/lab branching while bounding uncovered decode work
/// at high-degree hubs.
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
    boss_catalog: Res<ambition_platformer2d::boss_encounter::BossCatalog>,
    // PAIRED with the recipes because a Bevy system stops at sixteen
    // params, the same reason the covered transition path groups them. The
    // authorities travel together anyway: a placement names a character and may
    // name the policy that drives it.
    (
        construction_recipes,
        active_binding,
        brain_profiles,
        forced_brains,
        population_cap,
        mut plan_prefetch,
    ): (
        Res<ambition_platformer2d::actors::construction::ActorConstructionRegistry>,
        Option<Res<ambition_platformer2d::actors::rooms::ActiveContentBinding>>,
        Option<
            Res<ambition_platformer2d::characters::actor::character_catalog::BrainProfileRegistry>,
        >,
        // ⭐ AND WHAT A DEVELOPER FORCED THEM TO. The PREFETCH builds the same
        // plan the transition will commit, so a plan built without the override
        // and a transition built with it would disagree about the cast — the
        // prefetch would be discarded, silently, on every forced run. The
        // population cap rides for the same reason: a capped hall prefetched
        // uncapped is a different room.
        Option<Res<ambition_platformer2d::characters::brain::AuthoredBrainOverride>>,
        Option<Res<ambition_platformer2d::characters::actor::AuthoredPopulationCap>>,
        ResMut<RoomConstructionPlanPrefetch>,
    ),
    mut assets: ResMut<GameAssets>,
    catalog: Res<Platformer2dAssetCatalog>,
    // Grouped because they are one question — is this dependency ready, and for
    // an App with which render world — and because a Bevy system stops at
    // sixteen params.
    (asset_server, images, render_world, prepared_here): (
        Res<AssetServer>,
        Option<Res<Assets<Image>>>,
        Option<Res<RenderWorldPresent>>,
        // Grouped with the render-world fact because they are one question:
        // does this App draw, and has it uploaded THIS image.
        Option<Res<AppGpuPreparedImages>>,
    ),
    (mut layouts, mut character_load_states, prepared_characters, authored_sheets): (
        ResMut<Assets<TextureAtlasLayout>>,
        // Grouped with `layouts` to stay under Bevy's SystemParam arity limit.
        ResMut<ambition_platformer2d::actors::character_runtime::CharacterLoadStates>,
        Option<Res<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>>,
        Res<ambition_platformer2d::sprite_sheet::character::sheets::AuthoredSheets>,
    ),
    quality: Res<ResolvedVisualQuality>,
    time: Res<Time<Real>>,
    active_session: Option<Res<ActiveSessionScope>>,
    mut cache: ResMut<RoomPreparationPrefetchState>,
) {
    let empty_registry =
        ambition_platformer2d::characters::prepared::PreparedCharacterRegistry::default();
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

    // THE NEIGHBOURHOOD IS BOUNDED, and the reason is the shape of this
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
        cache.preparations = cache.preparations.saturating_add(1);
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
                // An absent prepared registry is an EMPTY cast, not an exemption, so every
                // neighbour containing a character-built body failed preflight, was forgotten, and
                // was re-prepared from scratch on the next frame — a whole `RoomConstructionPlan`
                // per neighbour per frame, thrown away, for as long as you stood there. It also
                // meant the prefetch never covered exactly the rooms that cost the most to prepare.
                ambition_platformer2d::actors::features::ActorConstructionContext::for_room_construction(
                    &construction_recipes,
                    ambition_platformer2d::engine_core::ContentEpoch(content_epoch.get()),
                    active_binding.as_deref(),
                    prepared_characters.as_deref(),
                    brain_profiles.as_deref(),
                    // THE PREFETCH DELIBERATELY REMEMBERS NOTHING, and
                    // the promotion check is what makes that safe: a plan
                    // states the dispositions it was prepared against, and the
                    // transition refuses to promote one prepared against
                    // anything but what the world remembers at the door. So
                    // while a body is carrying an authored object, every
                    // neighbour plan is a MISS and the transition prepares a
                    // fresh one — correct, one preparation, and no cache
                    // keyed on a value that changes when somebody bends down.
                    //
                    // AND THAT MISS IS NO LONGER TRANSIENT. Cross-room reinstatement made a
                    // `Placed` row visible to EVERY room's outlook — `Reinstated` where the
                    // object lies, `Suppressed` everywhere else, because those are one decision
                    // — so an object put down anywhere disables this cache for the whole world
                    // for the rest of the session, not just for the room holding it.
                    // Correctness is unaffected; what is lost is the preloading.
                    None,
                    forced_brains.as_deref(),
                    population_cap.as_deref(),
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
        // The prefetch stages and declares a neighbour's cast and realizes the
        // ration's worth; the remainder is deliberately NOT forwarded to the
        // global demand — loading a neighbour's whole cast in the open, one per
        // frame, is the hitch this file exists to avoid. The transition into that
        // room forwards it, behind its cover.
        let (manifest, _not_forwarded) = build_room_asset_manifest(
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
            Some(&boss_catalog),
            &[],
            None,
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
        if inspect_room_asset_manifest(
            &asset_server,
            images.as_deref(),
            RenderWorldPresent::from_option(render_world.as_deref()),
            prepared_here.as_deref(),
            &entry.manifest,
        )
        .is_terminal()
        {
            entry.settled_at = Some(time.elapsed());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `init_asset` is a trait method, and this module imports Bevy by name
    // rather than by prelude glob — so the trait has to be named too.
    use bevy::asset::AssetApp;
    use bevy::prelude::App;

    /// A character the room DEMANDED but the ration has not yet realized holds
    /// the reveal; one nothing declares does not (nothing will ever arrive for
    /// it, and the sprites system names the typo). Ready ones are answered by
    /// their pages in the manifest, not counted twice here.
    #[test]
    fn a_declared_but_unrealized_character_is_pending_and_an_unknown_one_is_not() {
        let mut assets = GameAssets::default();
        assets.characters.declare("npc_busy_beaver", "Busy Beaver");
        let tokens = vec![
            "npc_busy_beaver".to_string(),
            "Busy Beaver".to_string(),
            "nobody_declares_this".to_string(),
        ];
        let mut readiness = RoomAssetReadiness::default();
        inspect_demanded_characters(&tokens, &assets, None, &mut readiness);
        assert_eq!(
            readiness.total, 2,
            "both tokens of the declared character count"
        );
        assert_eq!(readiness.settled, 0);
        assert_eq!(
            readiness.pending,
            vec![
                "character:npc_busy_beaver (not yet decoded)".to_string(),
                "character:Busy Beaver (not yet decoded)".to_string(),
            ]
        );
        assert!(
            !readiness.is_ready(),
            "a declared, unrealized character holds the reveal"
        );
    }

    #[test]
    fn placeholder_image_handles_never_enter_room_manifests() {
        let mut images = Assets::<Image>::default();
        let real = images.add(Image::default());

        let mut draft = RoomManifestDraft::default();
        add_image_handle(
            &mut draft,
            "character:packed:unused-page",
            &Handle::<Image>::default(),
        );
        add_image_handle(&mut draft, "character:packed:used-page", &real);

        assert_eq!(draft.by_label.len(), 1);
        assert_eq!(
            draft.by_label.get("character:packed:used-page"),
            Some(&real.id()),
            "a sparse packed sheet's placeholder page is not a load dependency",
        );
    }

    /// One synthetic authored sheet, built through the public authoring seam so
    /// the fixture is a real `CharacterSheetSpec` rather than a hand-poked one.
    ///
    /// `pages_named` is how many page images the sheet declares; `frame_pages`
    /// is which pages its frame rects actually sample. The two differ for a
    /// sparse pack — the whole reason `used_pages()` exists.
    fn a_synthetic_spec(
        pages_named: usize,
        frame_pages: &[u32],
    ) -> ambition_platformer2d::sprite_sheet::character::sheets::CharacterSheetSpec {
        use ambition_platformer2d::sprite_sheet::character::sheets::{
            try_load_spec_for_target_authored, AuthoredSheets, SheetTuning,
        };

        // A target the baked index cannot hold, because the authored lookup
        // FALLS BACK to it — a name collision would silently test a real sheet.
        let target = "d153_synthetic_sheet";
        let images = (0..pages_named)
            .map(|page| format!("\"page_{page}.png\""))
            .collect::<Vec<_>>()
            .join(", ");
        let rows = frame_pages
            .iter()
            .enumerate()
            .map(|(index, page)| {
                // Every sheet needs an Idle row or the loader refuses it.
                let animation = if index == 0 { "idle" } else { "run" };
                format!(
                    "(animation: \"{animation}\", row_index: {index}, frame_count: 1, \
                     duration_ms: 100, duration_secs: 0.1, \
                     rects: [(x: 0, y: 0, w: 64, h: 64, page: {page})])"
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let ron = format!(
            "[(target: \"{target}\", image: \"page_0.png\", images: [{images}], \
              label_width: 0, frame_width: 64, frame_height: 64, rows: [{rows}])]"
        );

        let mut authored = AuthoredSheets::default();
        authored
            .insert_ron(target, &ron)
            .expect("the synthetic sheet parses");
        try_load_spec_for_target_authored(&authored, target, &SheetTuning::new(1.0, 1))
            .expect("the synthetic sheet resolves to a spec")
    }

    /// A room staging one character whose realization is `pages`, built through
    /// the real manifest builder.
    fn a_room_manifest_staging(
        pages: Vec<ambition_platformer2d::sprite_sheet::character::CharacterSpritePage>,
        spec: ambition_platformer2d::sprite_sheet::character::sheets::CharacterSheetSpec,
    ) -> RoomAssetManifest {
        use ambition_platformer2d::world::prelude::{AuthoredWorld, Vec2};

        let representative = pages
            .first()
            .map(|page| page.texture.clone())
            .unwrap_or_default();
        let asset = ambition_platformer2d::sprite_sheet::character::CharacterSpriteAsset {
            texture: representative,
            layout: Handle::default(),
            spec,
            pages,
            requested_tier: Default::default(),
            resolved_tier: Default::default(),
        };
        let mut assets = GameAssets::default();
        assets.characters.declare("d153_fighter", "D153 Fighter");
        assets.characters.publish("d153_fighter", asset);

        let world = AuthoredWorld::new(
            "D153 Room",
            Vec2::new(640.0, 360.0),
            Vec2::new(64.0, 256.0),
            Vec::new(),
        );
        let room = RoomSpec::new("d153_room", world);
        build_loaded_room_asset_manifest(&room, &["d153_fighter".to_owned()], &assets)
    }

    /// A page the SPEC requires and the REALIZATION does not have must refuse the reveal.
    #[test]
    fn a_required_page_the_realization_lacks_refuses_the_room() {
        let mut app = App::new();
        app.add_plugins(bevy::app::TaskPoolPlugin::default());
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        let asset_server = app.world().resource::<AssetServer>().clone();
        let realized = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());

        // The spec draws from pages 0 and 1; the realization holds one slot.
        let spec = a_synthetic_spec(2, &[0, 1]);
        assert_eq!(
            spec.used_pages().into_iter().collect::<Vec<_>>(),
            vec![0, 1],
            "the fixture is useless unless the spec actually names two pages",
        );
        let manifest = a_room_manifest_staging(
            vec![
                ambition_platformer2d::sprite_sheet::character::CharacterSpritePage {
                    texture: realized,
                    layout: Handle::default(),
                },
            ],
            spec,
        );

        assert_eq!(
            manifest.unresolved,
            vec!["character:d153_fighter:page:1 (required, unrealized)".to_owned()],
            "the page the realization lacks must be recorded, not skipped",
        );
        let readiness = inspect_room_asset_manifest(
            &asset_server,
            Some(app.world().resource::<Assets<Image>>()),
            // This App builds no render world, so nothing is ever owed a GPU.
            RenderWorldPresent(false),
            // No render world in this fixture, so no App-local prepared set
            // either — the GPU term is off and nothing may wait on it.
            None,
            &manifest,
        );
        assert!(
            !readiness.is_ready(),
            "a room missing required art must never report Ready",
        );
        assert!(
            readiness.is_terminal(),
            "an unrealizable page can never settle, so the barrier must not WAIT \
             on it — that is the permanent spinner this file was repaired for",
        );
        assert_eq!(readiness.failed.len(), 1, "{:?}", readiness.failed);
    }

    /// The regression this could have caused. An ordinary sparse pack names
    /// more pages than its frames sample; those slots are legitimately
    /// placeholders and must stay out of the manifest entirely — neither waited
    /// on (the permanent spinner) nor failed (this change's own hazard).
    #[test]
    fn an_unsampled_pack_page_is_neither_waited_on_nor_failed() {
        let mut app = App::new();
        app.add_plugins(bevy::app::TaskPoolPlugin::default());
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        let asset_server = app.world().resource::<AssetServer>().clone();
        let realized = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());

        // Three pack pages declared, every frame on page 0.
        let spec = a_synthetic_spec(3, &[0, 0]);
        assert_eq!(
            spec.page_count(),
            3,
            "the fixture must actually be a sparse pack",
        );
        assert_eq!(
            spec.used_pages().into_iter().collect::<Vec<_>>(),
            vec![0],
            "the fixture must actually leave pages unsampled",
        );
        let placeholder = || ambition_platformer2d::sprite_sheet::character::CharacterSpritePage {
            texture: Handle::default(),
            layout: Handle::default(),
        };
        let manifest = a_room_manifest_staging(
            vec![
                ambition_platformer2d::sprite_sheet::character::CharacterSpritePage {
                    texture: realized,
                    layout: Handle::default(),
                },
                placeholder(),
                placeholder(),
            ],
            spec,
        );

        assert!(
            manifest.unresolved.is_empty(),
            "an unsampled pack page is not a requirement: {:?}",
            manifest.unresolved,
        );
        assert_eq!(
            manifest.dependencies.len(),
            1,
            "only the sampled page is a load dependency: {:?}",
            manifest.dependencies,
        );
        let readiness = inspect_room_asset_manifest(
            &asset_server,
            Some(app.world().resource::<Assets<Image>>()),
            // This App builds no render world, so nothing is ever owed a GPU.
            RenderWorldPresent(false),
            // No render world in this fixture, so no App-local prepared set
            // either — the GPU term is off and nothing may wait on it.
            None,
            &manifest,
        );
        assert!(readiness.failed.is_empty(), "{:?}", readiness.failed);
        assert!(readiness.is_ready(), "{:?}", readiness.pending);
    }

    /// A decoded page whose GPU copy is still owed holds the reveal — when a
    /// render world exists to owe it — and releases it the frame the upload
    /// lands. Without a render world the same page is simply ready.
    ///
    /// ⭐ THE RENDER-WORLD FACT IS NOW AN ARGUMENT, so this test states it per
    /// arm instead of setting a process-global for its span and clearing it
    /// before returning. The stamps it makes are still on the shared ledger; the
    /// id is a fresh handle nobody else awaits.
    #[test]
    fn room_readiness_waits_for_the_gpu_copy_only_while_a_render_world_owes_it() {
        use ambition_platformer2d::asset_manager::image_stages;
        let mut app = App::new();
        app.add_plugins(bevy::app::TaskPoolPlugin::default());
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        let asset_server = app.world().resource::<AssetServer>().clone();
        let page = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        let manifest = a_room_manifest_staging(
            vec![
                ambition_platformer2d::sprite_sheet::character::CharacterSpritePage {
                    texture: page.clone(),
                    layout: Handle::default(),
                },
            ],
            a_synthetic_spec(1, &[0]),
        );
        let inspect = |app: &App, render_world: RenderWorldPresent| {
            inspect_room_asset_manifest(
                &asset_server,
                Some(app.world().resource::<Assets<Image>>()),
                render_world,
                // This fixture parameterises the render-world fact and never
                // stamps a GPU copy, so there is no App-local set to consult.
                None,
                &manifest,
            )
        };
        let headless = RenderWorldPresent(false);
        let rendering = RenderWorldPresent(true);

        // ⭐ THE RACE ARM: the page is in `Assets<Image>` but the ledger has not
        // stamped it inserted yet — which is every image on the frame it lands,
        // because the insertion stamp runs in `Last` and this poll in `Update`.
        // With a render world present that is OWED, not ready: the old reading
        // of the awaiting list called it ready here, latched the reveal, and
        // let a paced upload land after the cover lifted.
        let unstamped = inspect(&app, rendering);
        assert!(
            !unstamped.is_ready()
                && unstamped
                    .pending
                    .iter()
                    .any(|label| label.ends_with("(gpu upload)")),
            "loaded but not yet proven on the GPU must hold the reveal: {:?}",
            unstamped.pending
        );

        // Inserted, no render world: ready (a headless run never waits on a GPU).
        image_stages::ledger().inserted(
            page.id().untyped(),
            1.0,
            None,
            None,
            std::time::Instant::now(),
        );
        assert!(
            inspect(&app, headless).is_ready(),
            "no render world: {:?}",
            inspect(&app, headless).pending
        );

        // Inserted, render world present, upload owed: the page holds the reveal
        // under its own label.
        let held = inspect(&app, rendering);
        assert!(!held.is_ready());
        assert!(
            held.pending
                .iter()
                .any(|label| label.ends_with("(gpu upload)")),
            "the pending label names the stage: {:?}",
            held.pending
        );
        assert_eq!(held.settled, 0);

        // The render world stamps it prepared: ready on the next inspection.
        image_stages::ledger().gpu_prepared(page.id().untyped(), std::time::Instant::now());
        let ready = inspect(&app, rendering);
        assert!(ready.is_ready(), "{:?}", ready.pending);
        assert_eq!(ready.settled, 1);
        // ⛔⛔ AND THE POINT OF THE ARGUMENT: the SAME ledger, the SAME id, in
        // the SAME breath, answers a headless App differently. While this was a
        // field on the process-global ledger these two could not disagree, so
        // one rendering App in the process made every headless sibling wait for
        // an upload nothing would ever stamp.
        assert!(
            inspect(&app, headless).is_ready(),
            "a headless App must not inherit a rendering sibling's GPU debt"
        );
    }

    #[test]
    fn room_readiness_asks_the_owner_of_an_image_handle() {
        let mut app = App::new();
        app.add_plugins(bevy::app::TaskPoolPlugin::default());
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        let asset_server = app.world().resource::<AssetServer>().clone();

        let reserved = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .reserve_handle();
        let present = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());

        let manifest = RoomAssetManifest {
            room_id: "direct-image-room".to_owned(),
            dependencies: vec![
                RoomAssetDependency {
                    label: "reserved".to_owned(),
                    asset_id: reserved.id(),
                },
                RoomAssetDependency {
                    label: "present".to_owned(),
                    asset_id: present.id(),
                },
            ],
            ..Default::default()
        };
        let readiness = inspect_room_asset_manifest(
            &asset_server,
            Some(app.world().resource::<Assets<Image>>()),
            // This App builds no render world, so nothing is ever owed a GPU.
            RenderWorldPresent(false),
            // No render world in this fixture, so no App-local prepared set
            // either — the GPU term is off and nothing may wait on it.
            None,
            &manifest,
        );

        assert_eq!(readiness.settled, 1);
        assert_eq!(readiness.pending, vec!["reserved".to_owned()]);
        assert!(readiness.failed.is_empty());
    }

    #[test]
    fn manifest_equality_is_the_prefetch_promotion_contract() {
        let empty = RoomAssetManifest {
            room_id: "hall".to_string(),
            ..Default::default()
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
            ..Default::default()
        };
        assert!(!cache.classify_promotion(1, None, "hub", &different_room, Some(Duration::ZERO)));
    }

    /// The stall report names the room and the outstanding assets, and caps the
    /// list so a 129-character hall does not print 129 names per stall.
    #[test]
    fn a_stall_report_names_the_room_and_what_is_outstanding() {
        let pending: Vec<String> = (0..15).map(|i| format!("page_{i}.png")).collect();
        let report =
            asset_stall_report("hall_of_characters", Duration::from_secs(5), &pending, 141);
        assert!(report.contains("room 'hall_of_characters'"), "{report}");
        assert!(report.contains("15 of 141"), "{report}");
        assert!(report.contains("Still pending: page_0.png"), "{report}");
        assert!(report.contains("page_11.png (+3 more)"), "{report}");
        assert!(!report.contains("page_12.png"), "{report}");
    }
}
