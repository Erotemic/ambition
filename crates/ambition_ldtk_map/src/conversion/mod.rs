//! LDtk → Ambition runtime conversion.
//!
//! Materializes the typed [`ambition_world::rooms::RoomSet`] graph from a
//! validated [`super::project::LdtkProject`]. Per-entity routing goes
//! through the [`LdtkEntityConverter`] REGISTRY (ADR 0009): the engine
//! registers the standard vocabulary (`Solid`, `LoadingZone`, `Portal`,
//! `GravityZone`, `EnemySpawn`, …) and a game installs additional
//! converters at plugin-build time via
//! [`install_ldtk_entity_converters`] — the loader itself never learns
//! a content identifier. IntGrid → block / water / climbable emission
//! also lives here.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use ambition_engine_core as ae;

use super::fields::{
    field_bool, field_entity_ref, field_f32, field_i32, field_string, parse_boss_brain,
    parse_debug_label_kind, parse_enemy_brain, parse_optional_path, parse_path_mode,
    parse_pickup_kind, parse_points,
};
use super::intgrid::{
    emit_climbable_regions_from_intgrid, emit_collision_blocks_from_intgrid,
    emit_water_regions_from_intgrid,
};
use super::project::{LdtkEntityInstance, LdtkLevel, LdtkProject};
use super::surfaces::{
    compile_surface, parse_surface_spec, SurfaceCompiled, SURFACE_LIKE_IDENTIFIERS,
};
use ambition_world::rooms::{
    CameraClampMode, CameraZoneSpec, KinematicPathSpec, LoadingZone, LoadingZoneActivation,
    PropSpec, RoomLink, RoomSet, RoomSpec,
};

impl LdtkProject {
    /// Build the sandbox runtime room set from LDtk.
    ///
    /// This is a direct LDtk-native runtime builder. LDtk does not
    /// round-trip through a RON-shaped world manifest before it becomes
    /// playable data. `RoomSet` remains the runtime graph, but LDtk
    /// materializes `RoomSpec`, `ae::World`, loading zones, and graph links
    /// directly here.
    pub fn to_room_set(&self) -> Result<RoomSet, Vec<String>> {
        let report = self.validate();
        if !report.is_ok() {
            return Err(report.errors);
        }

        let mut area_levels: BTreeMap<String, Vec<&LdtkLevel>> = BTreeMap::new();
        for level in &self.levels {
            area_levels
                .entry(level.active_area())
                .or_default()
                .push(level);
        }

        // The game's installed WorldManifest names where play starts; a
        // project without that area (synthetic fixtures, partial checkouts)
        // starts in its first composed area.
        let entry_room = super::manifest::world_manifest().entry_room.as_str();
        let start_room = if area_levels.contains_key(entry_room) {
            entry_room.to_string()
        } else {
            area_levels
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| entry_room.to_string())
        };

        let mut links = self.collect_room_links();
        let mut rooms = Vec::new();
        for (area_id, levels) in area_levels {
            rooms.push(self.compose_runtime_area(&area_id, &levels)?);
        }
        // Baked `ron-room` docs (W2): rooms that enter the graph as
        // serialized IR, no authoring backend behind them.
        for doc in
            ambition_world::ron_room::load_ron_rooms(&super::manifest::world_manifest().ron_rooms)?
        {
            links.extend(doc.links);
            rooms.push(doc.spec);
        }
        Ok(RoomSet::from_parts(start_room, rooms, links))
    }

    fn collect_room_links(&self) -> Vec<RoomLink> {
        let mut links = Vec::new();
        for level in &self.levels {
            let from_room = level.active_area();
            for entity in level.all_entity_instances() {
                if entity.identifier != "LoadingZone" {
                    continue;
                }
                let Some(target_room) = field_string(entity, "target_room") else {
                    continue;
                };
                let Some(target_zone) = field_string(entity, "target_zone") else {
                    continue;
                };
                links.push(RoomLink {
                    from_room: from_room.clone(),
                    from_zone: field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
                    to_room: target_room,
                    to_zone: target_zone,
                    bidirectional: field_bool(entity, "bidirectional").unwrap_or(false),
                });
            }
        }
        links
    }

    fn compose_runtime_area(
        &self,
        area_id: &str,
        levels: &[&LdtkLevel],
    ) -> Result<RoomSpec, Vec<String>> {
        let mut errors = Vec::new();
        let min_x = levels.iter().map(|level| level.world_x).min().unwrap_or(0) as f32;
        let min_y = levels.iter().map(|level| level.world_y).min().unwrap_or(0) as f32;
        let max_x = levels
            .iter()
            .map(|level| level.world_x + level.px_wid)
            .max()
            .unwrap_or(0) as f32;
        let max_y = levels
            .iter()
            .map(|level| level.world_y + level.px_hei)
            .max()
            .unwrap_or(0) as f32;
        let mut spawn = None;
        let mut blocks = Vec::new();
        let mut loading_zones = Vec::new();
        let mut water_regions = Vec::new();
        let mut climbable_regions = Vec::new();
        let mut moving_platforms: Vec<ambition_world::platforms::MovingPlatformSpec> = Vec::new();
        let mut camera_zones: Vec<CameraZoneSpec> = Vec::new();
        let mut kinematic_paths: Vec<KinematicPathSpec> = Vec::new();
        let mut props: Vec<PropSpec> = Vec::new();
        let mut ground_items: Vec<ambition_world::rooms::GroundItemSpec> = Vec::new();
        let mut portal_gun_spawns: Vec<ambition_world::rooms::PortalGunSpawnSpec> = Vec::new();
        let mut portals: Vec<ambition_world::rooms::PortalSpec> = Vec::new();
        let mut shrines: Vec<ambition_world::rooms::ShrineSpec> = Vec::new();
        let mut gravity_zones: Vec<ambition_world::rooms::GravityZoneSpec> = Vec::new();
        // Per-family authored entity lists. Each LDtk entity emits into
        // exactly one of these (or into one of the non-authored Vecs
        // above).
        let mut hazards: Vec<
            ambition_world::rooms::Authored<ambition_world::rooms::HazardVolumeSpec>,
        > = Vec::new();
        let mut pickups: Vec<ambition_world::rooms::Authored<ambition_world::rooms::PickupSpec>> =
            Vec::new();
        let mut chests: Vec<ambition_world::rooms::Authored<ambition_world::rooms::ChestSpec>> =
            Vec::new();
        let mut breakables: Vec<
            ambition_world::rooms::Authored<ambition_world::rooms::BreakableSpec>,
        > = Vec::new();
        let mut enemy_spawns: Vec<
            ambition_world::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>,
        > = Vec::new();
        let mut boss_spawns: Vec<
            ambition_world::rooms::Authored<ambition_entity_catalog::placements::BossBrain>,
        > = Vec::new();
        let mut debug_labels: Vec<
            ambition_world::rooms::Authored<ambition_world::debug_label::DebugLabel>,
        > = Vec::new();
        let mut mount_links: Vec<(String, String)> = Vec::new();
        let mut chains: Vec<ae::SurfaceChain> = Vec::new();
        let mut placements: Vec<ambition_world::placements::PlacementRecord> = Vec::new();
        let mut metadata = ambition_world::rooms::RoomMetadata::default();
        for level in levels {
            // First-non-empty wins so author intent is predictable when
            // an active area spans multiple levels (e.g. central hub +
            // basement). The level order here is the LDtk-file order.
            metadata.merge(level.level_metadata());
            // AMBITION_REVIEW(spatial): LDtk world coordinates are flattened into
            // active-area-local Ambition coordinates here. Wall openings, edge
            // exits, transition arrivals, and camera bounds all depend on this
            // convention staying stable.
            let offset = ae::Vec2::new(level.world_x as f32 - min_x, level.world_y as f32 - min_y);
            if level.ambition_layer().is_none() {
                errors.push(format!(
                    "level '{}' missing Ambition layer",
                    level.identifier
                ));
                continue;
            }
            // Iterate every Entities-type layer in the level, not
            // just `"Ambition"`. A side layer like `"AmbitionCameras"`
            // holding only `CameraZone` entities is still picked up.
            for entity in level.all_entity_instances() {
                match entity_to_runtime(entity, offset) {
                    Ok(emission) => {
                        if emission.ignored {
                            continue;
                        }
                        if let Some(value) = emission.spawn {
                            spawn = Some(value);
                        }
                        blocks.extend(emission.blocks);
                        loading_zones.extend(emission.zones);
                        water_regions.extend(emission.water_regions);
                        moving_platforms.extend(emission.moving_platforms);
                        camera_zones.extend(emission.camera_zones);
                        kinematic_paths.extend(emission.kinematic_paths);
                        props.extend(emission.props);
                        ground_items.extend(emission.ground_items);
                        portal_gun_spawns.extend(emission.portal_gun_spawns);
                        portals.extend(emission.portals);
                        shrines.extend(emission.shrines);
                        gravity_zones.extend(emission.gravity_zones);
                        hazards.extend(emission.hazards);
                        pickups.extend(emission.pickups);
                        chests.extend(emission.chests);
                        breakables.extend(emission.breakables);
                        enemy_spawns.extend(emission.enemy_spawns);
                        boss_spawns.extend(emission.boss_spawns);
                        debug_labels.extend(emission.debug_labels);
                        mount_links.extend(emission.mount_links);
                        chains.extend(emission.chains);
                        placements.extend(emission.placements);
                    }
                    Err(error) => {
                        errors.push(format!("{} {}: {error}", entity.identifier, entity.iid))
                    }
                }
            }

            // IntGrid `Collision` layer: greedy-merge runs of same-value
            // cells into rectangles before emitting engine blocks. Per-cell
            // blocks introduced perceptible friction during ground-walk
            // because every 16px boundary became a potential snag against
            // the bespoke sweep logic (path_forward step D); merging
            // collapses a typical floor of N cells into one block while
            // keeping the IntGrid as the authoring representation.
            if let Some(layer) = level.collision_layer() {
                let geo_layer_key = format!("{}/{}", level.identifier, layer.identifier);
                match emit_collision_blocks_from_intgrid(layer, offset, &geo_layer_key) {
                    Ok(layer_blocks) => blocks.extend(layer_blocks),
                    Err(message) => {
                        errors.push(format!("level '{}' Collision: {message}", level.identifier))
                    }
                }
            }

            // IntGrid `Water` layer: each cell becomes a swimmable
            // region. Source-agnostic with entity `WaterVolume`; both
            // populate `World::water_regions`.
            if let Some(layer) = level.water_layer() {
                match emit_water_regions_from_intgrid(layer, offset) {
                    Ok(layer_regions) => water_regions.extend(layer_regions),
                    Err(message) => {
                        errors.push(format!("level '{}' Water: {message}", level.identifier))
                    }
                }
            }

            // IntGrid `Climbable` layer: each cell becomes a ladder /
            // vine / climbable wall region.
            if let Some(layer) = level.climbable_layer() {
                match emit_climbable_regions_from_intgrid(layer, offset) {
                    Ok(layer_regions) => climbable_regions.extend(layer_regions),
                    Err(message) => {
                        errors.push(format!("level '{}' Climbable: {message}", level.identifier))
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        let mut resolved_moving_platforms = Vec::new();
        for platform in moving_platforms {
            match platform.resolve(&kinematic_paths) {
                Ok(platform) => resolved_moving_platforms.push(platform),
                Err(error) => errors.push(error),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(RoomSpec {
            id: area_id.to_string(),
            world: ae::World::new(
                format!("Ambition: {}", area_id.replace('_', " ")),
                ae::Vec2::new(max_x - min_x, max_y - min_y),
                spawn.unwrap_or_else(|| ae::Vec2::new(96.0, 96.0)),
                blocks,
            )
            .with_water_regions(water_regions)
            .with_climbable_regions(climbable_regions)
            .with_chains(chains),
            loading_zones,
            metadata,
            camera_zones,
            kinematic_paths,
            moving_platforms: resolved_moving_platforms,
            props,
            ground_items,
            portal_gun_spawns,
            portals,
            shrines,
            gravity_zones,
            hazards,
            pickups,
            chests,
            breakables,
            enemy_spawns,
            boss_spawns,
            debug_labels,
            mount_links,
            placements,
        })
    }

    pub(super) fn area_has_player_start(&self, area: &str) -> bool {
        self.levels.iter().any(|level| {
            level.active_area() == area
                && level
                    .all_entity_instances()
                    .any(|entity| entity.identifier == "PlayerStart")
        })
    }
}

/// Aggregated runtime emission for one LDtk entity instance.
///
/// LDtk entities historically mapped 1:1 to a single emitted runtime piece.
/// With `Surface`, a single LDtk entity can compile into multiple emissions
/// (e.g. a `Block` for static collision plus a typed authored entity for the
/// breakable lifetime), so the conversion API yields a struct rather than a
/// one-of enum. Per-family Vecs replace the retired generic
/// `Vec<ae::RoomObject>` so the room composer can route each family into
/// its own `RoomSpec` field without re-dispatching on a kind enum.
#[derive(Clone, Debug, Default)]
pub struct RoomEmission {
    pub spawn: Option<ae::Vec2>,
    pub blocks: Vec<ae::Block>,
    pub zones: Vec<LoadingZone>,
    pub water_regions: Vec<ae::WaterRegion>,
    /// LDtk-authored moving platforms emitted by this entity.
    ///
    /// Most entities emit zero platforms; `MovingPlatform` emits one. The room
    /// composer concatenates these so active areas can own multiple authored
    /// moving solids.
    pub moving_platforms: Vec<ambition_world::platforms::MovingPlatformSpec>,
    pub camera_zones: Vec<CameraZoneSpec>,
    pub kinematic_paths: Vec<KinematicPathSpec>,
    /// LDtk-authored decorative props emitted by this entity. Most
    /// entities emit zero; `Prop` emits one. Render-only — see
    /// [`PropSpec`].
    pub props: Vec<PropSpec>,
    /// LDtk-authored ground held-items emitted by this entity. Most emit
    /// zero; `GroundItem` emits one. See [`ambition_world::rooms::GroundItemSpec`].
    pub ground_items: Vec<ambition_world::rooms::GroundItemSpec>,
    /// LDtk-authored portal-gun pickups. Most emit zero; `PortalGunSpawn` emits
    /// one. See [`ambition_world::rooms::PortalGunSpawnSpec`].
    pub portal_gun_spawns: Vec<ambition_world::rooms::PortalGunSpawnSpec>,
    /// LDtk-authored static portals. Most emit zero; `Portal` emits one. See
    /// [`ambition_world::rooms::PortalSpec`].
    pub portals: Vec<ambition_world::rooms::PortalSpec>,
    /// LDtk-authored heal/save shrines. Most emit zero; `ShrineSpawn` emits one.
    pub shrines: Vec<ambition_world::rooms::ShrineSpec>,
    /// LDtk-authored localized-gravity zones. Most emit zero; `GravityZone` emits
    /// one. See [`ambition_world::rooms::GravityZoneSpec`].
    pub gravity_zones: Vec<ambition_world::rooms::GravityZoneSpec>,
    // --- Per-family authored entity emissions:
    pub hazards: Vec<ambition_world::rooms::Authored<ambition_world::rooms::HazardVolumeSpec>>,
    // interactables migrated to the `placements` channel (fable audit F9.2).
    pub pickups: Vec<ambition_world::rooms::Authored<ambition_world::rooms::PickupSpec>>,
    pub chests: Vec<ambition_world::rooms::Authored<ambition_world::rooms::ChestSpec>>,
    pub breakables: Vec<ambition_world::rooms::Authored<ambition_world::rooms::BreakableSpec>>,
    pub enemy_spawns:
        Vec<ambition_world::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>>,
    pub boss_spawns:
        Vec<ambition_world::rooms::Authored<ambition_entity_catalog::placements::BossBrain>>,
    pub debug_labels: Vec<ambition_world::rooms::Authored<ambition_world::debug_label::DebugLabel>>,
    /// ADR 0020 authored mount links: `(rider_id, mount_id)` pairs emitted by a
    /// rider `EnemySpawn` carrying a `mounted_on` entity-ref. Resolved into a
    /// `RidingOn`/`MountSlot` link after both actors spawn (`FeatureId` match).
    pub mount_links: Vec<(String, String)>,
    /// Rideable surface chains (demo plan S3/Q17 — the momentum-locomotion
    /// geometry). Most entities emit zero; `SurfaceChain` emits one, and
    /// generated-geometry converters (e.g. a content `SurfaceLoop` marker)
    /// may emit many. Folded into `World::chains`; collision geometry ONLY
    /// for surface-momentum bodies.
    pub chains: Vec<ae::SurfaceChain>,
    /// Authored placement RECORDS (the [W-b] shape): the schema-over-record
    /// channel every family converges onto as W-queue step 3 converts spawn
    /// branches to lowering interpreters. During the migration a converter
    /// may DUAL-emit (its legacy typed family + the record); records are
    /// inert until an interpreter is registered for their kind.
    pub placements: Vec<ambition_world::placements::PlacementRecord>,
    pub ignored: bool,
}

impl RoomEmission {
    pub fn ignored() -> Self {
        Self {
            ignored: true,
            ..Self::default()
        }
    }

    pub fn spawn(value: ae::Vec2) -> Self {
        Self {
            spawn: Some(value),
            ..Self::default()
        }
    }

    pub fn zone(zone: LoadingZone) -> Self {
        Self {
            zones: vec![zone],
            ..Self::default()
        }
    }

    pub fn chain(chain: ae::SurfaceChain) -> Self {
        Self {
            chains: vec![chain],
            ..Self::default()
        }
    }

    pub fn water_region(region: ae::WaterRegion) -> Self {
        Self {
            water_regions: vec![region],
            ..Self::default()
        }
    }

    pub fn moving_platform(spec: ambition_world::platforms::MovingPlatformSpec) -> Self {
        Self {
            moving_platforms: vec![spec],
            ..Self::default()
        }
    }

    pub fn camera_zone(zone: CameraZoneSpec) -> Self {
        Self {
            camera_zones: vec![zone],
            ..Self::default()
        }
    }

    pub fn prop(spec: PropSpec) -> Self {
        Self {
            props: vec![spec],
            ..Self::default()
        }
    }

    pub fn ground_item(spec: ambition_world::rooms::GroundItemSpec) -> Self {
        Self {
            ground_items: vec![spec],
            ..Self::default()
        }
    }

    #[cfg(feature = "portal_ldtk")]
    pub fn portal_gun_spawn(spec: ambition_world::rooms::PortalGunSpawnSpec) -> Self {
        Self {
            portal_gun_spawns: vec![spec],
            ..Self::default()
        }
    }

    #[cfg(feature = "portal_ldtk")]
    pub fn portal(spec: ambition_world::rooms::PortalSpec) -> Self {
        Self {
            portals: vec![spec],
            ..Self::default()
        }
    }

    pub fn shrine(spec: ambition_world::rooms::ShrineSpec) -> Self {
        Self {
            shrines: vec![spec],
            ..Self::default()
        }
    }

    pub fn gravity_zone(spec: ambition_world::rooms::GravityZoneSpec) -> Self {
        Self {
            gravity_zones: vec![spec],
            ..Self::default()
        }
    }

    pub fn kinematic_path(spec: KinematicPathSpec) -> Self {
        Self {
            kinematic_paths: vec![spec],
            ..Self::default()
        }
    }

    /// Emit a single authored placement RECORD (the [W-b] schema-over-record
    /// channel). Families migrated off their typed `RoomSpec` list (fable audit
    /// F9.2 — interactables so far) emit through here only.
    pub fn placement(record: ambition_world::placements::PlacementRecord) -> Self {
        Self {
            placements: vec![record],
            ..Self::default()
        }
    }

    pub fn from_compiled(compiled: SurfaceCompiled) -> Self {
        Self {
            blocks: compiled.blocks,
            breakables: compiled.breakables,
            ..Self::default()
        }
    }

    // Per-family typed emitters. The conversion sites use these instead of
    // wrapping payloads in a generic `RoomObject { kind: ... }`.
    pub fn hazard(
        authored: ambition_world::rooms::Authored<ambition_world::rooms::HazardVolumeSpec>,
    ) -> Self {
        Self {
            hazards: vec![authored],
            ..Self::default()
        }
    }

    pub fn pickup(
        authored: ambition_world::rooms::Authored<ambition_world::rooms::PickupSpec>,
    ) -> Self {
        Self {
            pickups: vec![authored],
            ..Self::default()
        }
    }

    pub fn chest(
        authored: ambition_world::rooms::Authored<ambition_world::rooms::ChestSpec>,
    ) -> Self {
        Self {
            chests: vec![authored],
            ..Self::default()
        }
    }

    pub fn enemy_spawn(
        authored: ambition_world::rooms::Authored<
            ambition_entity_catalog::placements::CharacterBrain,
        >,
    ) -> Self {
        Self {
            enemy_spawns: vec![authored],
            ..Self::default()
        }
    }

    pub fn boss_spawn(
        authored: ambition_world::rooms::Authored<ambition_entity_catalog::placements::BossBrain>,
    ) -> Self {
        Self {
            boss_spawns: vec![authored],
            ..Self::default()
        }
    }

    pub fn debug_label(
        authored: ambition_world::rooms::Authored<ambition_world::debug_label::DebugLabel>,
    ) -> Self {
        Self {
            debug_labels: vec![authored],
            ..Self::default()
        }
    }
}

fn entity_min_size(entity: &LdtkEntityInstance, offset: ae::Vec2) -> (ae::Vec2, ae::Vec2) {
    (
        ae::Vec2::new(entity.px[0] as f32, entity.px[1] as f32) + offset,
        ae::Vec2::new(entity.width as f32, entity.height as f32),
    )
}

fn object_aabb(min: ae::Vec2, size: ae::Vec2) -> ae::Aabb {
    ae::aabb_from_min_size(min, size)
}

fn offset_points(points: Vec<ae::Vec2>, offset: ae::Vec2) -> Vec<ae::Vec2> {
    points.into_iter().map(|point| point + offset).collect()
}

fn path_lookup_id(entity: &LdtkEntityInstance, name: &str) -> String {
    field_string(entity, "id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| compact_path_name(name))
        .unwrap_or_else(|| entity.iid.clone())
}

fn compact_path_name(name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut previous_was_sep = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_sep = false;
        } else if !previous_was_sep && !slug.is_empty() {
            slug.push('_');
            previous_was_sep = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        return None;
    }
    let slug = slug.replace("_path_", "_");
    Some(slug.strip_suffix("_path").unwrap_or(&slug).to_string())
}

fn authored_triple(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> (String, String, ae::Aabb) {
    (entity.iid.clone(), name, object_aabb(min, size))
}

/// Everything a converter receives about one LDtk entity instance,
/// pre-resolved into active-area-local coordinates.
pub struct LdtkEntityCtx<'a> {
    pub entity: &'a LdtkEntityInstance,
    /// Resolved display name (the `name` field, else the LDtk identifier).
    pub name: String,
    /// Active-area-local top-left corner (the level offset is applied).
    pub min: ae::Vec2,
    pub size: ae::Vec2,
    /// The level's active-area offset. Apply it to any ADDITIONAL points a
    /// converter parses out of entity fields (e.g. path points) — `min` has
    /// it applied already.
    pub offset: ae::Vec2,
}

impl LdtkEntityCtx<'_> {
    /// The `(entity, name, min, size)` tuple most converters consume.
    pub fn parts(&self) -> (&LdtkEntityInstance, String, ae::Vec2, ae::Vec2) {
        (self.entity, self.name.clone(), self.min, self.size)
    }
}

/// One LDtk entity converter: `identifier → emission`. Pure `fn` — content
/// registers additional converters via [`install_ldtk_entity_converters`];
/// everything a game-specific converter needs must come from the entity's
/// authored fields (the ctx), never from ambient state.
pub type LdtkEntityConverter = fn(&LdtkEntityCtx<'_>) -> Result<RoomEmission, String>;

/// Content-installed LDtk entity converters (ADR 0009). Set once at
/// plugin-build time; first install wins (same seam contract as
/// `install_enemy_roster`). Deliberately a process-global `OnceLock`, not a
/// Bevy `Resource`: conversion runs from pure non-system code
/// (`LdtkProject::to_room_set`, validators, tools) with no `World` in hand.
static EXTRA_ENTITY_CONVERTERS: OnceLock<BTreeMap<String, LdtkEntityConverter>> = OnceLock::new();

/// Install game-specific LDtk entity converters — the content layer calls
/// this at plugin-build time (before any world load). Installed identifiers
/// pass validation and convert exactly like the engine's standard vocabulary;
/// a standard identifier cannot be overridden (the standard table wins on
/// lookup). First install wins; later calls are ignored.
pub fn install_ldtk_entity_converters<I>(converters: I)
where
    I: IntoIterator<Item = (String, LdtkEntityConverter)>,
{
    let _ = EXTRA_ENTITY_CONVERTERS.set(converters.into_iter().collect());
}

/// The engine's standard LDtk vocabulary, registered through the SAME
/// registry shape content extensions use. Keys mirror
/// [`super::bevy_runtime::AMBITION_LDTK_ENTITY_IDENTIFIERS`] exactly
/// (pinned by a test) — the marker-registration list and the converter
/// table must not drift.
fn standard_converters() -> &'static BTreeMap<&'static str, LdtkEntityConverter> {
    static STANDARD: OnceLock<BTreeMap<&'static str, LdtkEntityConverter>> = OnceLock::new();
    STANDARD.get_or_init(|| {
        let mut map: BTreeMap<&'static str, LdtkEntityConverter> = BTreeMap::new();
        // Surface-shaped identifiers (one typed parse → compile pipeline).
        for identifier in SURFACE_LIKE_IDENTIFIERS {
            map.insert(identifier, convert_surface as LdtkEntityConverter);
        }
        map.insert("PlayerStart", convert_player_start);
        map.insert("LoadingZone", convert_loading_zone);
        map.insert("DamageVolume", convert_damage_volume);
        map.insert("KinematicPath", convert_kinematic_path);
        map.insert("SurfaceChain", convert_surface_chain);
        map.insert("SurfaceLoop", convert_surface_loop);
        map.insert("Prop", convert_prop);
        map.insert("NpcSpawn", convert_npc_spawn);
        map.insert("PickupSpawn", convert_pickup_spawn);
        map.insert("GroundItem", convert_ground_item);
        // Under `portal_ldtk` these are the real converters; compiled out,
        // they are loud-error converters (fail, never silently drop).
        map.insert("PortalGunSpawn", convert_portal_gun_spawn);
        map.insert("Portal", convert_portal);
        map.insert("ShrineSpawn", convert_shrine);
        map.insert("GravityZone", convert_gravity_zone);
        map.insert("ChestSpawn", convert_chest_spawn);
        map.insert("EnemySpawn", convert_enemy_spawn);
        map.insert("BossSpawn", convert_boss_spawn);
        map.insert("DebugLabel", convert_debug_label);
        map.insert("WaterVolume", convert_water_volume);
        map.insert("MovingPlatform", convert_moving_platform);
        map.insert("CameraZone", convert_camera_zone);
        map.insert("Switch", convert_switch);
        // Read by their own consumers off the raw LdtkProject; they never
        // join the emission stream.
        for identifier in ["StitchedBoundary", "EncounterTrigger", "LockWall"] {
            map.insert(identifier, convert_consumed_elsewhere);
        }
        map
    })
}

/// Resolve the converter for an LDtk identifier: the engine's standard
/// vocabulary first, then content-installed extensions. `None` = unknown
/// entity (validation error).
pub(super) fn converter_for(identifier: &str) -> Option<LdtkEntityConverter> {
    standard_converters()
        .get(identifier)
        .or_else(|| {
            EXTRA_ENTITY_CONVERTERS
                .get()
                .and_then(|extra| extra.get(identifier))
        })
        .copied()
}

pub(super) fn entity_to_runtime(
    entity: &LdtkEntityInstance,
    offset: ae::Vec2,
) -> Result<RoomEmission, String> {
    let (min, size) = entity_min_size(entity, offset);
    let ctx = LdtkEntityCtx {
        entity,
        name: field_string(entity, "name").unwrap_or_else(|| entity.identifier.clone()),
        min,
        size,
        offset,
    };
    let Some(converter) = converter_for(&entity.identifier) else {
        return Err(format!(
            "unsupported entity identifier '{}'",
            entity.identifier
        ));
    };
    converter(&ctx)
}

mod entity_converters;
use entity_converters::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{LdtkFieldInstance, LdtkLayerInstance, LdtkLevel, LdtkProject};
    use serde_json::Value;

    // ---- Restored ruled-contract tests (fable final audit F7): these were
    // dropped in the W3 carve. They pin [W-b] dual emission, the §3.6 tile
    // GeoId determinism contract, the W2 sanic IR proof, and the F7 fixes
    // (record display name; inline-motion hazards stay legacy-only).

    fn entity_at(
        identifier: &str,
        px: [i32; 2],
        size: [i32; 2],
        fields: &[(&str, Value)],
    ) -> crate::project::LdtkEntityInstance {
        crate::project::LdtkEntityInstance {
            iid: format!("{identifier}-test-{}-{}", px[0], px[1]),
            identifier: identifier.to_string(),
            pivot: vec![0.0, 0.0],
            px,
            width: size[0],
            height: size[1],
            field_instances: fields
                .iter()
                .map(|(name, value)| LdtkFieldInstance {
                    identifier: name.to_string(),
                    value: value.clone(),
                    real_editor_values: vec![Value::Null],
                })
                .collect(),
        }
    }

    fn synthetic_level(entities: Vec<crate::project::LdtkEntityInstance>) -> LdtkProject {
        let mut instances = vec![entity_at("PlayerStart", [32, 400], [16, 32], &[])];
        instances.extend(entities);
        LdtkProject {
            json_version: "1.5.3".into(),
            levels: vec![LdtkLevel {
                iid: "level-iid".into(),
                identifier: "registry_lab".into(),
                world_x: 0,
                world_y: 0,
                px_wid: 640,
                px_hei: 480,
                field_instances: vec![LdtkFieldInstance {
                    identifier: "activeArea".into(),
                    value: Value::String("registry_lab".into()),
                    real_editor_values: vec![],
                }],
                layer_instances: vec![LdtkLayerInstance {
                    identifier: "Ambition".into(),
                    layer_type: "Entities".into(),
                    c_wid: 40,
                    c_hei: 30,
                    grid_size: 16,
                    entity_instances: instances,
                    int_grid_csv: Vec::new(),
                    grid_tiles: Vec::new(),
                }],
            }],
        }
    }

    /// [W-b]: a `DamageVolume` DUAL-emits — the plain hazard spawn payload plus
    /// the `PlacementRecord` twin, joined by the same placement id AND carrying
    /// the authored display name (F7: lowering must not label hazards by iid).
    #[test]
    fn damage_volume_dual_emits_a_named_hazard_placement_record() {
        use ambition_entity_catalog::placements::{DamageKind, DamageTeam, PlacementSchema};
        let project = synthetic_level(vec![entity_at(
            "DamageVolume",
            [96, 416],
            [64, 32],
            &[
                ("damage", Value::Number(3.into())),
                ("name", Value::String("Spike Run".into())),
                ("path_id", Value::String("spike_run".into())),
            ],
        )]);
        let room_set = project.to_room_set().expect("hazard project composes");
        let room = &room_set.rooms[0];
        assert_eq!(
            room.hazards.len(),
            1,
            "plain hazard channel still feeds spawning"
        );
        assert_eq!(room.placements.len(), 1, "record channel carries the twin");
        let record = &room.placements[0];
        assert_eq!(record.id.as_str(), room.hazards[0].id, "same placement id");
        assert_eq!(
            record.name, "Spike Run",
            "authored display name rides the record"
        );
        assert_eq!(record.aabb, room.hazards[0].aabb, "same authored footprint");
        let PlacementSchema::Hazard(spec) = &record.schema else {
            panic!("expected a hazard placement schema");
        };
        assert_eq!(spec.damage, 3);
        assert_eq!(spec.kind, DamageKind::Hazard);
        assert_eq!(spec.team, DamageTeam::Environment);
        assert_eq!(spec.path_id.as_deref(), Some("spike_run"));
    }

    /// F7: an INLINE-motion hazard cannot be represented by
    /// `HazardSpec` — it must NOT emit a record (else the lowering path wins
    /// the dual-spawn guard and silently drops the motion).
    #[test]
    fn inline_motion_hazards_stay_legacy_only() {
        let project = synthetic_level(vec![entity_at(
            "DamageVolume",
            [96, 416],
            [64, 32],
            &[
                ("damage", Value::Number(2.into())),
                ("path_points", Value::String("0,0; 100,0".into())),
            ],
        )]);
        let room_set = project.to_room_set().expect("composes");
        let room = &room_set.rooms[0];
        assert_eq!(room.hazards.len(), 1);
        assert!(
            room.hazards[0].payload.motion.is_some(),
            "fixture: the inline path parsed"
        );
        assert!(
            room.placements.is_empty(),
            "an inline-motion hazard must not emit a record until dissolution \
             lifts the path to a room-level KinematicPath"
        );
    }

    /// THE W2 IR PROOF, restored: the sanic area (richest IR surface — the
    /// chains channel) round-trips serialize∘parse as a string fixed point
    /// and re-enters a RoomSet with no LDtk in the second path.
    #[test]
    fn the_sanic_area_round_trips_as_a_ron_room() {
        let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
        let room_set = project.to_room_set().expect("sandbox composes");
        let sanic = room_set
            .rooms
            .iter()
            .find(|room| room.id == "sanic_sandbox")
            .expect("the sanic area exists in the sandbox world");
        assert!(
            !sanic.world.chains.is_empty(),
            "fixture: the sanic area exercises the chains channel"
        );
        let doc = ambition_world::ron_room::RonRoomDoc {
            spec: sanic.clone(),
            links: Vec::new(),
        };
        let baked = ambition_world::ron_room::room_doc_to_ron(&doc).expect("bakes");
        let reloaded = ambition_world::ron_room::room_doc_from_ron(&baked).expect("parses");
        let rebaked = ambition_world::ron_room::room_doc_to_ron(&reloaded).expect("re-bakes");
        assert_eq!(baked, rebaked, "serialize∘parse is a fixed point");
        let twin_set = ambition_world::rooms::RoomSet::from_parts(
            reloaded.spec.id.clone(),
            vec![reloaded.spec],
            reloaded.links,
        );
        assert_eq!(twin_set.active_spec().id, "sanic_sandbox");
    }

    #[test]
    fn compact_path_name_slugifies_and_strips_path_noise() {
        assert_eq!(
            compact_path_name("Moving Platform Path").as_deref(),
            Some("moving_platform")
        );
        assert_eq!(compact_path_name("gate-path-a").as_deref(), Some("gate_a"));
        assert_eq!(compact_path_name("Patrol Path").as_deref(), Some("patrol"));
        assert_eq!(
            compact_path_name("Already_Slug").as_deref(),
            Some("already_slug")
        );
        // Collapses runs of separators and trims trailing ones.
        assert_eq!(compact_path_name("  a -- b  ").as_deref(), Some("a_b"));
        // No alphanumerics → no usable slug.
        assert_eq!(compact_path_name("  !! "), None);
        assert_eq!(compact_path_name(""), None);
    }
}
