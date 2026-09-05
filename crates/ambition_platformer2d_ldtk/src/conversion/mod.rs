//! LDtk → Ambition runtime conversion.
//!
//! Materializes the typed [`ambition_platformer2d_world::rooms::RoomSet`] graph from a
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

use ambition_platformer2d_core as ae;

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
use ambition_platformer2d_world::rooms::{
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
    pub fn to_room_set(
        &self,
        manifest: &ambition_platformer2d_world::world_manifest::WorldManifest,
        vocabulary: &LdtkVocabulary,
    ) -> Result<RoomSet, Vec<String>> {
        // The caller's WorldManifest names where play starts and which baked
        // `ron-room` docs join the graph.
        self.build_room_set(&manifest.entry_room, &manifest.ron_rooms, vocabulary)
    }

    /// Convert a SELF-CONTAINED project — a game crate's own embedded world
    /// file (a demo's standalone level). Play starts in the caller's
    /// `entry_room` and no manifest-registered auxiliary rooms are appended,
    /// so the conversion needs no `WorldManifest` at all.
    pub fn to_room_set_with_entry(
        &self,
        entry_room: &str,
        vocabulary: &LdtkVocabulary,
    ) -> Result<RoomSet, Vec<String>> {
        self.build_room_set(entry_room, &[], vocabulary)
    }

    fn build_room_set(
        &self,
        entry_room: &str,
        ron_rooms: &[ambition_platformer2d_world::ron_room::RonRoomSource],
        vocabulary: &LdtkVocabulary,
    ) -> Result<RoomSet, Vec<String>> {
        let report = self.validate(vocabulary);
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

        // A project without the named entry area (synthetic fixtures, partial
        // checkouts) starts in its first composed area.
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
            rooms.push(self.compose_runtime_area(&area_id, &levels, vocabulary)?);
        }
        // Baked `ron-room` docs: rooms that enter the graph as serialized IR, no authoring
        // backend behind them.
        for doc in ambition_platformer2d_world::ron_room::load_ron_rooms(ron_rooms)? {
            links.extend(doc.links);
            rooms.push(doc.spec);
        }
        Ok(RoomSet::from_parts(start_room, rooms, links))
    }

    pub(crate) fn collect_room_links(&self) -> Vec<RoomLink> {
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
        vocabulary: &LdtkVocabulary,
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
        let mut moving_platforms: Vec<ambition_platformer2d_world::platforms::MovingPlatformSpec> =
            Vec::new();
        let mut camera_zones: Vec<CameraZoneSpec> = Vec::new();
        let mut kinematic_paths: Vec<KinematicPathSpec> = Vec::new();
        let mut props: Vec<PropSpec> = Vec::new();
        let mut ground_items: Vec<ambition_platformer2d_world::rooms::GroundItemSpec> = Vec::new();
        let mut portal_gun_spawns: Vec<ambition_platformer2d_world::rooms::PortalGunSpawnSpec> =
            Vec::new();
        let mut shrines: Vec<ambition_platformer2d_world::rooms::ShrineSpec> = Vec::new();
        let mut gravity_zones: Vec<ambition_platformer2d_world::rooms::GravityZoneSpec> =
            Vec::new();
        let mut enemy_spawns: Vec<
            ambition_platformer2d_world::rooms::Authored<
                ambition_platformer2d_world::rooms::EnemySpawnSpec,
            >,
        > = Vec::new();
        let mut boss_spawns: Vec<
            ambition_platformer2d_world::rooms::Authored<
                ambition_entity_catalog::placements::BossBrain,
            >,
        > = Vec::new();
        let mut debug_labels: Vec<
            ambition_platformer2d_world::rooms::Authored<
                ambition_platformer2d_world::debug_label::DebugLabel,
            >,
        > = Vec::new();
        let mut mount_links: Vec<(String, String)> = Vec::new();
        let mut chains: Vec<ae::SurfaceChain> = Vec::new();
        let mut placements: Vec<ambition_platformer2d_world::placements::PlacementRecord> =
            Vec::new();
        let mut encounter_triggers: Vec<ambition_platformer2d_world::rooms::EncounterTriggerSpec> =
            Vec::new();
        let mut lock_walls: Vec<ambition_platformer2d_world::rooms::EncounterLockWallSpec> =
            Vec::new();
        let mut switch_commands: Vec<ambition_platformer2d_world::rooms::SwitchCommandSpec> =
            Vec::new();
        let mut metadata = ambition_platformer2d_world::rooms::RoomMetadata::default();
        // Indexed BEFORE any conversion: a `path_ref` may name a path authored
        // later in the file, or in a sibling level of the same active area.
        let kinematic_path_ids = kinematic_path_ids_by_iid(levels);
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
                match entity_to_runtime(entity, offset, vocabulary, &kinematic_path_ids) {
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
                        shrines.extend(emission.shrines);
                        gravity_zones.extend(emission.gravity_zones);
                        enemy_spawns.extend(emission.enemy_spawns);
                        boss_spawns.extend(emission.boss_spawns);
                        debug_labels.extend(emission.debug_labels);
                        mount_links.extend(emission.mount_links);
                        chains.extend(emission.chains);
                        placements.extend(emission.placements);
                        encounter_triggers.extend(emission.encounter_triggers);
                        lock_walls.extend(emission.lock_walls);
                        switch_commands.extend(emission.switch_commands);
                    }
                    // name the LEVEL. An iid is not something an author can
                    // search for; the level is what they open to fix it, and
                    // every other diagnostic on this path already says which one.
                    Err(error) => errors.push(format!(
                        "level '{}' {} {}: {error}",
                        level.identifier, entity.identifier, entity.iid
                    )),
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
            .with_chains(chains)
            // The room's own out-of-bounds margins, authored on the level, in
            // ONE call so a fourth axis cannot be added to the metadata and
            // silently not forwarded. Absent, the engine defaults stand — which
            // is what every room had when the fall margin was a literal inside
            // the movement kernel and the other two did not exist.
            .with_edge_margins(
                metadata.fall_out_margin.map(|px| px as f32),
                metadata.side_out_margin.map(|px| px as f32),
                metadata.rise_out_margin.map(|px| px as f32),
            ),
            loading_zones,
            metadata,
            camera_zones,
            kinematic_paths,
            moving_platforms: resolved_moving_platforms,
            props,
            ground_items,
            portal_gun_spawns,
            shrines,
            gravity_zones,
            enemy_spawns,
            boss_spawns,
            debug_labels,
            mount_links,
            placements,
            encounter_triggers,
            lock_walls,
            switch_commands,
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
    pub moving_platforms: Vec<ambition_platformer2d_world::platforms::MovingPlatformSpec>,
    pub camera_zones: Vec<CameraZoneSpec>,
    pub kinematic_paths: Vec<KinematicPathSpec>,
    /// LDtk-authored decorative props emitted by this entity. Most
    /// entities emit zero; `Prop` emits one. Render-only — see
    /// [`PropSpec`].
    pub props: Vec<PropSpec>,
    /// LDtk-authored ground held-items emitted by this entity. Most emit
    /// zero; `GroundItem` emits one. See [`ambition_platformer2d_world::rooms::GroundItemSpec`].
    pub ground_items: Vec<ambition_platformer2d_world::rooms::GroundItemSpec>,
    /// LDtk-authored portal-gun pickups. Most emit zero; `PortalGunSpawn` emits
    /// one. See [`ambition_platformer2d_world::rooms::PortalGunSpawnSpec`].
    pub portal_gun_spawns: Vec<ambition_platformer2d_world::rooms::PortalGunSpawnSpec>,
    /// LDtk-authored heal/save shrines. Most emit zero; `ShrineSpawn` emits one.
    pub shrines: Vec<ambition_platformer2d_world::rooms::ShrineSpec>,
    /// LDtk-authored localized-gravity zones. Most emit zero; `GravityZone` emits
    /// one. See [`ambition_platformer2d_world::rooms::GravityZoneSpec`].
    pub gravity_zones: Vec<ambition_platformer2d_world::rooms::GravityZoneSpec>,
    // --- Per-family authored entity emissions:
    // interactables migrated to the `placements` channel (fable audit F9.2).
    pub enemy_spawns: Vec<
        ambition_platformer2d_world::rooms::Authored<
            ambition_platformer2d_world::rooms::EnemySpawnSpec,
        >,
    >,
    pub boss_spawns: Vec<
        ambition_platformer2d_world::rooms::Authored<
            ambition_entity_catalog::placements::BossBrain,
        >,
    >,
    pub debug_labels: Vec<
        ambition_platformer2d_world::rooms::Authored<
            ambition_platformer2d_world::debug_label::DebugLabel,
        >,
    >,
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
    pub placements: Vec<ambition_platformer2d_world::placements::PlacementRecord>,
    /// An authored encounter's trigger volume. At most one per area; most entities emit zero.
    /// See [`ambition_platformer2d_world:rooms:EncounterTriggerSpec`] for why these now join
    /// the emission stream instead of being read off the raw project.
    pub encounter_triggers: Vec<ambition_platformer2d_world::rooms::EncounterTriggerSpec>,
    /// An authored encounter's lock wall. At most one per area.
    pub lock_walls: Vec<ambition_platformer2d_world::rooms::EncounterLockWallSpec>,
    /// A `Switch`'s authored `on_activate` line. Most switches emit none.
    pub switch_commands: Vec<ambition_platformer2d_world::rooms::SwitchCommandSpec>,
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

    pub fn moving_platform(
        spec: ambition_platformer2d_world::platforms::MovingPlatformSpec,
    ) -> Self {
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

    pub fn ground_item(spec: ambition_platformer2d_world::rooms::GroundItemSpec) -> Self {
        Self {
            ground_items: vec![spec],
            ..Self::default()
        }
    }

    #[cfg(feature = "portal_ldtk")]
    pub fn portal_gun_spawn(spec: ambition_platformer2d_world::rooms::PortalGunSpawnSpec) -> Self {
        Self {
            portal_gun_spawns: vec![spec],
            ..Self::default()
        }
    }

    pub fn shrine(spec: ambition_platformer2d_world::rooms::ShrineSpec) -> Self {
        Self {
            shrines: vec![spec],
            ..Self::default()
        }
    }

    pub fn gravity_zone(spec: ambition_platformer2d_world::rooms::GravityZoneSpec) -> Self {
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
    pub fn placement(record: ambition_platformer2d_world::placements::PlacementRecord) -> Self {
        Self {
            placements: vec![record],
            ..Self::default()
        }
    }

    pub fn from_compiled(compiled: SurfaceCompiled) -> Self {
        // Breakables lower through the single `placements` channel (fable audit
        // F9.2). The surface compiler still yields typed `Authored<BreakableSpec>`
        // internally; convert each to a placement record here at the emission edge.
        let placements = compiled
            .breakables
            .into_iter()
            .map(|authored| {
                let mut record = ambition_platformer2d_world::placements::PlacementRecord::new(
                    authored.id,
                    ambition_entity_catalog::placements::PlacementSchema::Breakable(
                        authored.payload,
                    ),
                    authored.aabb,
                );
                record.name = authored.name;
                record
            })
            .collect();
        Self {
            blocks: compiled.blocks,
            placements,
            ..Self::default()
        }
    }

    pub fn enemy_spawn(
        authored: ambition_platformer2d_world::rooms::Authored<
            ambition_platformer2d_world::rooms::EnemySpawnSpec,
        >,
    ) -> Self {
        Self {
            enemy_spawns: vec![authored],
            ..Self::default()
        }
    }

    pub fn boss_spawn(
        authored: ambition_platformer2d_world::rooms::Authored<
            ambition_entity_catalog::placements::BossBrain,
        >,
    ) -> Self {
        Self {
            boss_spawns: vec![authored],
            ..Self::default()
        }
    }

    pub fn debug_label(
        authored: ambition_platformer2d_world::rooms::Authored<
            ambition_platformer2d_world::debug_label::DebugLabel,
        >,
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

/// The stable lookup id conversion gives a `KinematicPath`: its authored `id`
/// field, else the slug of its display name, else the LDtk iid.
///
/// Public because a validator reading raw LDtk JSON needs the id conversion
/// WILL produce, and re-deriving it is how the game-side content validator came
/// to disagree with the runtime about which paths exist. Ask, do not model.
///
/// Nothing referenced the compacted spelling; it existed only to be bridged back by a third
/// resolution alias, and that bridge was implemented in three places and got it wrong in two, which
/// is how sandbox's basement patroller stood still for months while two validators called it
/// healthy. The rule now has ONE owner and this road asks it.
pub fn kinematic_path_lookup_id(entity: &LdtkEntityInstance, name: &str) -> String {
    field_string(entity, "id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| ambition_platformer2d_world::rooms::kinematic_path_name_slug(name))
        .unwrap_or_else(|| entity.iid.clone())
}

/// An entity's display name: its authored `name`, else the LDtk identifier.
///
/// One rule, because two roads need it — the per-entity conversion context and
/// the pre-pass that indexes an area's `KinematicPath`s by iid. Both feed
/// [`kinematic_path_lookup_id`], whose answer changes with the name, so a second
/// spelling of "what is this entity called" would mint a second lookup id.
fn entity_display_name(entity: &LdtkEntityInstance) -> String {
    field_string(entity, "name").unwrap_or_else(|| entity.identifier.clone())
}

/// Every `KinematicPath` in an active area, by LDtk `iid` → the lookup id its
/// [`KinematicPathSpec`] will carry.
///
/// this is what makes a native `EntityRef` to a path resolvable at all. A
/// ref stores the target's `iid`; the room's path table is keyed by the lookup
/// id. Resolving one to the other needs the TARGET entity, which a per-entity
/// converter does not have, so the area builds the index once and the ref is
/// resolved at conversion — the same shape the Python `set-field` road uses when
/// it reads a ref target's containers out of the project instead of trusting a
/// spec to carry them.
///
/// scoped to the AREA, matching the runtime lookup table's scope exactly. A
/// ref pointing at a path in some other area is not in this map, and the
/// converter refuses it out loud rather than degrading to "no motion".
fn kinematic_path_ids_by_iid(levels: &[&LdtkLevel]) -> BTreeMap<String, String> {
    levels
        .iter()
        .copied()
        .flat_map(|level| level.all_entity_instances())
        .filter(|entity| entity.identifier == "KinematicPath")
        .map(|entity| {
            let name = entity_display_name(entity);
            (entity.iid.clone(), kinematic_path_lookup_id(entity, &name))
        })
        .collect()
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
    /// This active area's `KinematicPath`s, by iid → the lookup id each one's
    /// spec carries. Read through [`Self::kinematic_path_ref`], never directly.
    pub kinematic_path_ids: &'a BTreeMap<String, String>,
}

impl LdtkEntityCtx<'_> {
    /// The `(entity, name, min, size)` tuple most converters consume.
    pub fn parts(&self) -> (&LdtkEntityInstance, String, ae::Vec2, ae::Vec2) {
        (self.entity, self.name.clone(), self.min, self.size)
    }

    /// Resolve a native `EntityRef` field naming a `KinematicPath` into the
    /// lookup id every path resolver answers to.
    ///
    /// `Ok(None)` = the field is unset, which is authoring nothing. An `Err` is
    /// a ref that names something this area has no path for — a dangling or
    /// mistyped link, which is content the author must fix. it is refused
    /// rather than dropped: a path reference that silently resolves to nothing
    /// degrades to "the body does not move", and a level that looks finished
    /// while an actor stands still is the exact failure this repo has paid for
    /// on this relationship twice.
    ///
    /// this re-derives no resolution rule. The id comes from
    /// [`kinematic_path_lookup_id`] — the same call that mints the spec's own id
    /// — so the reference and the target agree by construction rather than by a
    /// spelling convention both sides implement.
    pub fn kinematic_path_ref(&self, field: &str) -> Result<Option<String>, String> {
        let Some(target) = field_entity_ref(self.entity, field) else {
            return Ok(None);
        };
        match self.kinematic_path_ids.get(&target) {
            Some(id) => Ok(Some(id.clone())),
            None => Err(format!(
                "`{field}` references entity `{target}`, which is not a \
                 KinematicPath in this active area"
            )),
        }
    }
}

/// One LDtk entity converter: `identifier → emission`. Pure `fn` — content
/// registers additional converters via [`install_ldtk_entity_converters`];
/// everything a game-specific converter needs must come from the entity's
/// authored fields (the ctx), never from ambient state.
pub type LdtkEntityConverter = fn(&LdtkEntityCtx<'_>) -> Result<RoomEmission, String>;

/// The LDtk nouns one conversion understands: the engine's standard
/// vocabulary, plus whatever the caller's game adds.
///
/// The reason given for the global was real: conversion runs from pure non-system code
/// (`to_room_set`, validators, tools) with no `World` in hand, so a Bevy `Resource` could not reach
/// it. But "no `World`" argues for a PARAMETER, not for ambient state — and a value passed in is
/// exactly as reachable from a tool as from a system.
///
/// the vocabulary is now part of the question. Asking "what rooms does
/// this project describe?" without saying which nouns you understand was always
/// an incomplete question; it only looked complete because one answer was
/// installed behind everyone's back.
#[derive(Clone, Default)]
pub struct LdtkVocabulary {
    extensions: BTreeMap<String, LdtkEntityConverter>,
}

impl LdtkVocabulary {
    /// The engine's vocabulary and nothing else — the right answer for the
    /// sandbox, for tools, and for any game that authors only standard nouns.
    pub fn engine() -> Self {
        Self::default()
    }

    /// The engine's vocabulary plus a game's own converters.
    ///
    /// a game cannot override a standard identifier. The engine's table
    /// wins on lookup, which keeps `Solid` meaning `Solid` in every world file
    /// anyone loads. Extending the vocabulary and redefining it are different
    /// permissions, and only the first is on offer.
    pub fn extended_by<I>(converters: I) -> Self
    where
        I: IntoIterator<Item = (String, LdtkEntityConverter)>,
    {
        Self {
            extensions: converters.into_iter().collect(),
        }
    }

    /// Resolve the converter for an identifier: the engine's standard
    /// vocabulary first, then this game's extensions. `None` = an unknown
    /// entity, which is a validation error rather than a silent skip.
    pub(super) fn converter_for(&self, identifier: &str) -> Option<LdtkEntityConverter> {
        standard_converters()
            .get(identifier)
            .or_else(|| self.extensions.get(identifier))
            .copied()
    }

    /// Every identifier this vocabulary can convert, in canonical order — what
    /// a validator reports against and what a tool can print.
    pub fn identifiers(&self) -> impl Iterator<Item = &str> {
        standard_converters()
            .keys()
            .copied()
            .chain(self.extensions.keys().map(String::as_str))
    }
}

impl std::fmt::Debug for LdtkVocabulary {
    /// Function pointers have no useful `Debug`, and the IDENTIFIERS are what
    /// anyone comparing two vocabularies actually means.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LdtkVocabulary")
            .field("extensions", &self.extensions.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// The engine's standard LDtk vocabulary, registered through the SAME
/// registry shape content extensions use.
///
/// ⛔⛔ THIS SAID THE KEYS MIRROR the marker-registration list *"exactly (pinned
/// by a test)"*. MEASURED 2026-09-05: **neither half was true.** This table had
/// 34 keys and the list had 32 — `SurfaceLoop` and `SurfaceRamp` had converters
/// and no marker registration — and no test anywhere pinned the two.
///
/// ⭐ THE LIST IS GONE. `bevy_runtime::AmbitionLdtkRegistrationPlugin` now
/// DERIVES its registrations from this vocabulary, so the two cannot disagree:
/// there is nothing left to disagree with. What remains is a named exclusion,
/// `MARKERLESS_IDENTIFIERS`, holding back the same pair the drift had held back
/// by accident — behaviour-identical on purpose, because registering them is a
/// separate decision (awaiting-maintainer-decision #64).
///
/// ⚠ THE DRIFT WAS FOUND BY CHECKING A COMMENT, not by a failure, and the
/// comment claiming a test was what made it invisible: a reader who believed
/// the sentence had no reason to count either side. What IS pinned, and really
/// is, is the CONTRACT against these converters: `contract/prover.rs` runs
/// `ldtk_entity_contract.json` (34 entities) against the real parsers in both
/// directions.
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
        map.insert("SurfaceRamp", convert_surface_ramp);
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
        map.insert("EncounterTrigger", convert_encounter_trigger);
        map.insert("LockWall", convert_lock_wall);
        // Read by its own consumer off the raw LdtkProject; it never joins the
        // emission stream.
        map.insert("StitchedBoundary", convert_consumed_elsewhere);
        map
    })
}

pub(super) fn entity_to_runtime(
    entity: &LdtkEntityInstance,
    offset: ae::Vec2,
    vocabulary: &LdtkVocabulary,
    kinematic_path_ids: &BTreeMap<String, String>,
) -> Result<RoomEmission, String> {
    let (min, size) = entity_min_size(entity, offset);
    let ctx = LdtkEntityCtx {
        entity,
        name: entity_display_name(entity),
        min,
        size,
        offset,
        kinematic_path_ids,
    };
    let Some(converter) = vocabulary.converter_for(&entity.identifier) else {
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
    use ambition_asset_manager::AssetId;
    use ambition_platformer2d_world::world_manifest::{WorldManifest, WorldSource};
    use serde_json::Value;

    fn test_fixture_manifest() -> WorldManifest {
        let worlds_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../game/ambition_content/assets/worlds");
        let source = |id: &str, file: &str, required: bool| WorldSource {
            id: AssetId::new(id),
            asset_path: format!("game://worlds/{file}"),
            loose_path: Some(worlds_dir.join(file)),
            embedded_text: None,
            embedded_bevy_path: None,
            required,
        };
        WorldManifest {
            entry_room: "central_hub_complex".to_string(),
            ron_rooms: Vec::new(),
            worlds: vec![
                source("world.sandbox_ldtk", "sandbox.ldtk", true),
                source("world.intro_ldtk", "intro.ldtk", false),
                source(
                    "world.cut_rope_ldtk",
                    "you_have_to_cut_the_rope.ldtk",
                    false,
                ),
                source("world.hall_ldtk", "hall_of_characters.ldtk", false),
            ],
        }
    }

    // ---- Restored ruled-contract tests (fable final audit F7): these were dropped in the
    // carve. They pin [W-b] dual emission, the §3.6 tile GeoId determinism contract, the sanic
    // IR proof, and the F7 fixes (record display name; inline-motion hazards stay legacy-only).

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

    /// A level field the same synthetic project can carry, so the round-trip
    /// under test is level metadata rather than an entity.
    fn level_field(name: &str, value: Value) -> LdtkFieldInstance {
        LdtkFieldInstance {
            identifier: name.to_string(),
            value,
            real_editor_values: vec![],
        }
    }

    /// A stage authors where it ends.
    ///
    /// `WorldEdgeMargins::fall` was a `200.0` literal inside the movement kernel —
    /// duplicated across two copies of the out-of-bounds gate — so no room could
    /// disagree with it, which made a platform fighter's blast zone (a per-stage
    /// number that IS the loss condition of the genre) unauthorable. Rust rooms
    /// gained `with_fall_out_margin`; this is the LDtk half, because "missing a
    /// concept means ADD IT TO LDTK" and a number only Rust can set is not
    /// authored, it is hard-coded somewhere newer.
    #[test]
    fn a_level_authors_its_own_fall_out_margin() {
        let mut project = synthetic_level(Vec::new());
        project.levels[0]
            .field_instances
            .push(level_field("fall_out_margin", Value::Number(64.into())));
        let room_set = project
            .to_room_set_with_entry("central_hub_complex", &LdtkVocabulary::engine())
            .expect("the project composes");
        assert_eq!(
            room_set.rooms[0].world.edges.fall, 64.0,
            "the level authored a 64px fall-out margin and the composed world did \
             not take it — the field is declared, read, and dropped"
        );
        assert_eq!(
            room_set.rooms[0].metadata.fall_out_margin,
            Some(64),
            "the metadata channel must carry it too; the world is downstream of it"
        );
    }

    /// The whole migration is worthless if adding an authoring channel silently re-tunes every
    /// room that never used it.
    #[test]
    fn a_level_that_authors_no_margin_keeps_the_engine_default() {
        let room_set = synthetic_level(Vec::new())
            .to_room_set_with_entry("central_hub_complex", &LdtkVocabulary::engine())
            .expect("the project composes");
        assert_eq!(
            room_set.rooms[0].world.edges.fall,
            ae::World::DEFAULT_FALL_OUT_MARGIN
        );
        assert_eq!(room_set.rooms[0].metadata.fall_out_margin, None);
    }

    /// A level authors where finishing it leads.
    ///
    /// this was an if/else chain in a game crate, and it was the LAST Rust
    /// cost of authoring a level. Mary-O's `exit_for_room` read
    /// *"1-1 → 1-2, else if 1-2 → 1-3, else if 1-3 → 1-1, else replay"*: every
    /// other property of a level — geometry, blocks, enemies, links, goal pole,
    /// roster entry — came off the LDtk file, and the successor did not. It had
    /// already cost a test, which had pinned *"finishing 1-2 returns to 1-1"* —
    /// true only while 1-2 was the last level authored.
    ///
    /// the id is NOT resolved here, and that is the design. A level states
    /// a name; only the loaded `RoomSet` knows which rooms a session holds, so
    /// refusing an unknown id at conversion would refuse a room that names a
    /// sibling living in another world file. The consumer warns.
    #[test]
    fn a_level_authors_where_finishing_it_leads() {
        let mut project = synthetic_level(Vec::new());
        project.levels[0]
            .field_instances
            .push(level_field("next_room", Value::String("cinder_ferry".into())));
        let room_set = project
            .to_room_set_with_entry("registry_lab", &LdtkVocabulary::engine())
            .expect("the project composes");
        assert_eq!(
            room_set.rooms[0].metadata.next_room.as_deref(),
            Some("cinder_ferry"),
            "the level named its successor and the composed room dropped it — \
             the field is declared and read but never lands, which is the `mode` \
             bug wearing a new name"
        );
    }

    /// A level that names no successor has none — the arcade loop, which is a
    /// real answer rather than the absence of one. An EMPTY string is the same
    /// answer as an unset field, because clearing the box in the editor is how
    /// an author retires an exit.
    #[test]
    fn a_level_that_names_no_successor_has_none() {
        let room_set = synthetic_level(Vec::new())
            .to_room_set_with_entry("registry_lab", &LdtkVocabulary::engine())
            .expect("the project composes");
        assert_eq!(room_set.rooms[0].metadata.next_room, None);

        let mut cleared = synthetic_level(Vec::new());
        cleared.levels[0]
            .field_instances
            .push(level_field("next_room", Value::String("   ".into())));
        let room_set = cleared
            .to_room_set_with_entry("registry_lab", &LdtkVocabulary::engine())
            .expect("the project composes");
        assert_eq!(
            room_set.rooms[0].metadata.next_room, None,
            "a blank `next_room` is 'no successor', not a room whose id is spaces"
        );
    }

    /// A negative margin would put the kill line INSIDE the room, so every body
    /// would be out of bounds standing on the floor. Rejected at the reader,
    /// not clamped, because a clamp turns an authoring mistake into a room that
    /// merely behaves oddly.
    #[test]
    fn a_negative_out_margin_is_refused_rather_than_clamped() {
        // All THREE margins, because they share one `take_px` closure now.
        let mut project = synthetic_level(Vec::new());
        for name in ["fall_out_margin", "side_out_margin", "rise_out_margin"] {
            project.levels[0]
                .field_instances
                .push(level_field(name, Value::Number((-32).into())));
        }
        let room = &project
            .to_room_set_with_entry("central_hub_complex", &LdtkVocabulary::engine())
            .expect("the project composes")
            .rooms[0];
        assert_eq!(room.metadata.fall_out_margin, None);
        assert_eq!(room.metadata.side_out_margin, None);
        assert_eq!(room.metadata.rise_out_margin, None);
        assert_eq!(room.world.edges.fall, ae::World::DEFAULT_FALL_OUT_MARGIN);
        assert_eq!(room.world.edges.side, None);
        assert_eq!(room.world.edges.rise, None);
    }

    /// Zero is not "unset". A stage that authors `0` is saying "you are out the
    /// instant you cross my edge", and the refusal above must not swallow it —
    /// `filter(>= 0)` and `filter(> 0)` differ by exactly this case.
    #[test]
    fn a_zero_margin_is_authored_and_not_mistaken_for_absence() {
        let mut project = synthetic_level(Vec::new());
        project.levels[0]
            .field_instances
            .push(level_field("fall_out_margin", Value::Number(0.into())));
        project.levels[0]
            .field_instances
            .push(level_field("side_out_margin", Value::Number(0.into())));
        let room = &project
            .to_room_set_with_entry("central_hub_complex", &LdtkVocabulary::engine())
            .expect("the project composes")
            .rooms[0];
        assert_eq!(room.metadata.fall_out_margin, Some(0));
        assert_eq!(room.world.edges.fall, 0.0);
        assert_eq!(room.world.edges.side, Some(0.0));
    }

    /// A stage can declare that its SIDES are a blast zone, and a corridor
    /// can decline. The fall direction always kills — every room has a pit
    /// whether it wanted one or not — but the sides mean opposite things in
    /// the two genres this engine serves, so they are `Option` and absent by
    /// default.
    #[test]
    fn a_level_authors_its_optional_out_margins() {
        let mut project = synthetic_level(Vec::new());
        project.levels[0]
            .field_instances
            .push(level_field("side_out_margin", Value::Number(48.into())));
        project.levels[0].field_instances.push(level_field(
            "rise_out_margin",
            Value::Number(96.into()),
        ));
        let room = &project
            .to_room_set_with_entry("central_hub_complex", &LdtkVocabulary::engine())
            .expect("the project composes")
            .rooms[0];
        assert_eq!(room.world.edges.side, Some(48.0));
        assert_eq!(room.world.edges.rise, Some(96.0));
        assert_eq!(room.metadata.side_out_margin, Some(48));
        assert_eq!(room.metadata.rise_out_margin, Some(96));
    }

    /// A room that says nothing has NO side or ceiling blast zone. This is the
    /// case that protects every existing corridor in the game: if absent meant
    /// "some default distance", walking off the left edge of a room would start
    /// killing players the moment this field shipped.
    #[test]
    fn a_level_that_declines_the_optional_zones_has_none() {
        let room = &synthetic_level(Vec::new())
            .to_room_set_with_entry("central_hub_complex", &LdtkVocabulary::engine())
            .expect("the project composes")
            .rooms[0];
        assert_eq!(room.world.edges.side, None);
        assert_eq!(room.world.edges.rise, None);
    }

    /// A level can declare its game mode. `RoomMetadata::mode` documented
    /// itself as "authored as the LDtk level string field `mode`" while no
    /// project declared the field and no level set it — every mode in the repo
    /// is assigned in Rust. The doc was describing a channel that did not
    /// exist; this is the channel.
    #[test]
    fn a_level_authors_its_own_game_mode() {
        let mut project = synthetic_level(Vec::new());
        project.levels[0]
            .field_instances
            .push(level_field("mode", Value::String("sanic".into())));
        let room_set = project
            .to_room_set_with_entry("central_hub_complex", &LdtkVocabulary::engine())
            .expect("the project composes");
        assert_eq!(
            room_set.rooms[0].metadata.mode.as_deref(),
            Some("sanic"),
            "a level that declares its mode must reach the rules gate that reads it"
        );
    }

    /// [W-b] / F9.2 arc exit: a `DamageVolume` emits a single `PlacementRecord`
    /// (the ONLY channel now — no typed hazard Vec), carrying the authored
    /// display name (F7: lowering must not label hazards by iid).
    #[test]
    fn damage_volume_emits_a_named_hazard_placement_record() {
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
        let room_set = project
            .to_room_set_with_entry("central_hub_complex", &LdtkVocabulary::engine())
            .expect("hazard project composes");
        let room = &room_set.rooms[0];
        assert_eq!(
            room.placements.len(),
            1,
            "the placements channel is the only hazard spawn path"
        );
        let record = &room.placements[0];
        assert_eq!(
            record.name, "Spike Run",
            "authored display name rides the record"
        );
        let PlacementSchema::Hazard(spec) = &record.schema else {
            panic!("expected a hazard placement schema");
        };
        assert_eq!(spec.damage, 3);
        assert_eq!(spec.kind, DamageKind::Hazard);
        assert_eq!(spec.team, DamageTeam::Environment);
        assert_eq!(spec.path_id.as_deref(), Some("spike_run"));
    }

    /// A `PickupSpawn` may author an optional animated sprite sheet: the reward
    /// stays on `kind`, and the presentation override rides `PickupSpec.sprite`
    /// (the pickup renderer binds it as a looping character sheet). Absent field
    ///  `None`  the static per-kind sprite.
    #[test]
    fn pickup_spawn_carries_an_optional_animated_sprite() {
        use ambition_entity_catalog::placements::PlacementSchema;
        use ambition_entity_catalog::PickupKind;
        let project = synthetic_level(vec![
            entity_at(
                "PickupSpawn",
                [64, 320],
                [30, 30],
                &[
                    ("name", Value::String("ring".into())),
                    ("kind", Value::String("currency:1".into())),
                    ("sprite", Value::String("sanic_ring_prop".into())),
                ],
            ),
            entity_at(
                "PickupSpawn",
                [128, 320],
                [24, 24],
                &[
                    ("name", Value::String("coin".into())),
                    ("kind", Value::String("currency:1".into())),
                ],
            ),
        ]);
        let room_set = project
            .to_room_set_with_entry("central_hub_complex", &LdtkVocabulary::engine())
            .expect("pickup project composes");
        let room = &room_set.rooms[0];
        let sprite_of = |name: &str| {
            room.placements
                .iter()
                .find(|r| r.name == name)
                .and_then(|r| match &r.schema {
                    PlacementSchema::Pickup(p) => Some((p.kind.clone(), p.sprite.clone())),
                    _ => None,
                })
                .expect("pickup lowered")
        };
        let (ring_kind, ring_sprite) = sprite_of("ring");
        assert!(matches!(ring_kind, PickupKind::Currency { amount: 1 }));
        assert_eq!(
            ring_sprite.as_deref(),
            Some("sanic_ring_prop"),
            "the authored sprite field rides the pickup spec"
        );
        assert_eq!(
            sprite_of("coin").1,
            None,
            "a pickup without a sprite field stays on the static per-kind art"
        );
    }

    /// F7 dissolution (F9.2 arc exit): an INLINE-motion hazard is LIFTED to a
    /// room-level `KinematicPath` at conversion — it emits a normal hazard
    /// placement whose `path_id` references the synthesized path, so the
    /// lowering resolves the motion instead of silently dropping it.
    #[test]
    fn inline_motion_hazards_lift_to_a_room_kinematic_path() {
        use ambition_entity_catalog::placements::PlacementSchema;
        let project = synthetic_level(vec![entity_at(
            "DamageVolume",
            [96, 416],
            [64, 32],
            &[
                ("id", Value::String("spikes".into())),
                ("damage", Value::Number(2.into())),
                ("path_points", Value::String("0,0; 100,0".into())),
            ],
        )]);
        let room_set = project
            .to_room_set_with_entry("central_hub_complex", &LdtkVocabulary::engine())
            .expect("composes");
        let room = &room_set.rooms[0];
        // Exactly one hazard placement, no typed hazard Vec (deleted).
        assert_eq!(
            room.placements.len(),
            1,
            "inline-motion hazard emits a record"
        );
        let PlacementSchema::Hazard(spec) = &room.placements[0].schema else {
            panic!("expected a hazard placement schema");
        };
        let path_id = spec
            .path_id
            .as_deref()
            .expect("inline motion lifted to a path_id reference");
        // The synthesized room-level KinematicPath exists and is what path_id
        // points at, so `new_with_paths` will resolve the motion.
        assert!(
            room.kinematic_paths.iter().any(|p| p.id == path_id),
            "the lifted room KinematicPath '{path_id}' is present in the room"
        );
    }

    #[test]
    fn the_sanic_area_round_trips_as_a_ron_room() {
        let manifest = test_fixture_manifest();
        let project =
            LdtkProject::load_default_for_dev(&manifest).expect("sandbox LDtk should load");
        let room_set = project
            .to_room_set(&manifest, &LdtkVocabulary::engine())
            .expect("sandbox composes");
        let sanic = room_set
            .rooms
            .iter()
            .find(|room| room.id == "sanic_sandbox")
            .expect("the sanic area exists in the sandbox world");
        assert!(
            !sanic.world.chains.is_empty(),
            "fixture: the sanic area exercises the chains channel"
        );
        let doc = ambition_platformer2d_world::ron_room::RonRoomDoc {
            spec: sanic.clone(),
            links: Vec::new(),
        };
        let baked = ambition_platformer2d_world::ron_room::room_doc_to_ron(&doc).expect("bakes");
        let reloaded =
            ambition_platformer2d_world::ron_room::room_doc_from_ron(&baked).expect("parses");
        let rebaked =
            ambition_platformer2d_world::ron_room::room_doc_to_ron(&reloaded).expect("re-bakes");
        assert_eq!(baked, rebaked, "serialize∘parse is a fixed point");
        let twin_set = ambition_platformer2d_world::rooms::RoomSet::from_parts(
            reloaded.spec.id.clone(),
            vec![reloaded.spec],
            reloaded.links,
        );
        assert_eq!(twin_set.active_spec().id, "sanic_sandbox");
    }

    /// An author can NAME a moving platform, and gets the iid when they do
    /// not.
    ///
    /// this converter went straight to the iid, alone among the ones that take an identity:
    /// `LoadingZone`, `CameraZone`, `Portal` and `ShrineSpawn` all read `field_string(entity,
    /// "id")` first.
    ///
    /// both halves, because the fallback is what keeps it additive. No
    /// world authors an `id` on a `MovingPlatform` today, so every existing
    /// platform has to keep the iid it already had — a test that only checked
    /// the new field would pass over a change that broke every current level.
    #[test]
    fn a_moving_platform_takes_the_authored_id_and_falls_back_to_its_iid() {
        use crate::project::{LdtkEntityInstance, LdtkFieldInstance};

        let platform = |fields: Vec<LdtkFieldInstance>| LdtkEntityInstance {
            iid: "MovingPlatform-4242".into(),
            identifier: "MovingPlatform".into(),
            pivot: Vec::new(),
            px: [64, 96],
            width: 96,
            height: 16,
            field_instances: fields,
        };
        let named = |identifier: &str, value: &str| LdtkFieldInstance {
            identifier: identifier.into(),
            value: serde_json::Value::String(value.into()),
            real_editor_values: Vec::new(),
        };
        let no_paths = BTreeMap::new();
        let convert = |entity: &LdtkEntityInstance| {
            let ctx = LdtkEntityCtx {
                entity,
                name: "Underground Ferry".to_string(),
                min: ae::Vec2::new(64.0, 96.0),
                size: ae::Vec2::new(96.0, 16.0),
                offset: ae::Vec2::ZERO,
                kinematic_path_ids: &no_paths,
            };
            super::entity_converters::convert_moving_platform(&ctx)
                .expect("a MovingPlatform converts")
                .moving_platforms
                .remove(0)
        };

        let authored = platform(vec![named("id", "mary_o_1_2_ferry")]);
        assert_eq!(
            convert(&authored).id,
            "mary_o_1_2_ferry",
            "the authored id wins, so a room can address the platform it means"
        );

        let anonymous = platform(Vec::new());
        assert_eq!(
            convert(&anonymous).id,
            "MovingPlatform-4242",
            "and a platform that names nothing keeps its iid — every world \
             authored before this field existed depends on that"
        );
    }

    /// An enemy's BEHAVIOUR and its ART are authored separately — both roads.
    ///
    /// the ABSENT case is the one that matters. Every world authored
    /// before the field existed names no `character_id`, and a test that only
    /// checked the new field would sail past a change that broke all of them —
    /// the lesson the sibling `MovingPlatform` id row wrote down. So both roads
    /// are asserted, and the fallback is asserted to be the NAME rather than
    /// merely "not the id".
    #[test]
    fn an_enemy_authors_its_art_identity_and_falls_back_to_its_name() {
        let enemy = |fields: Vec<LdtkFieldInstance>| LdtkEntityInstance {
            iid: "EnemySpawn-8080".into(),
            identifier: "EnemySpawn".into(),
            pivot: Vec::new(),
            px: [32, 48],
            width: 16,
            height: 16,
            field_instances: fields,
        };
        let named = |identifier: &str, value: &str| LdtkFieldInstance {
            identifier: identifier.into(),
            value: serde_json::Value::String(value.into()),
            real_editor_values: Vec::new(),
        };
        let no_paths = BTreeMap::new();
        let convert = |entity: &LdtkEntityInstance| {
            let ctx = LdtkEntityCtx {
                entity,
                name: "Solid Snake".to_string(),
                min: ae::Vec2::new(32.0, 48.0),
                size: ae::Vec2::new(16.0, 16.0),
                offset: ae::Vec2::ZERO,
                kinematic_path_ids: &no_paths,
            };
            super::entity_converters::convert_enemy_spawn(&ctx)
                .expect("an EnemySpawn converts")
                .enemy_spawns
                .remove(0)
        };
        let convert_err = |entity: &LdtkEntityInstance| {
            let ctx = LdtkEntityCtx {
                entity,
                name: "Solid Snake".to_string(),
                min: ae::Vec2::new(32.0, 48.0),
                size: ae::Vec2::new(16.0, 16.0),
                offset: ae::Vec2::ZERO,
                kinematic_path_ids: &no_paths,
            };
            super::entity_converters::convert_enemy_spawn(&ctx)
                .expect_err("an EnemySpawn with no character must be refused")
        };

        let authored = convert(&enemy(vec![
            named("brain", "mary_o_snake"),
            named("character_id", "solid_snake"),
        ]));
        assert_eq!(
            authored.payload.character_id.as_str(),
            "solid_snake",
            "the authored identity did not survive conversion"
        );
        assert_eq!(
            authored.payload.presentation_identity(),
            "solid_snake",
            "one field answers art and gameplay alike; a rename cannot un-art the \
             level and cannot swap the creature either"
        );
        assert_eq!(
            authored.name, "Solid Snake",
            "the label is still the label; the id did not swallow it"
        );
        assert_eq!(
            authored.payload.facing,
            ambition_platformer2d_world::rooms::SpawnFacing::Right,
            "an older placement with no facing field keeps the historical +1/right default"
        );

        let left = convert(&enemy(vec![
            named("brain", "mary_o_snake"),
            named("character_id", "solid_snake"),
            named("facing", "Left"),
        ]));
        assert_eq!(
            left.payload.facing,
            ambition_platformer2d_world::rooms::SpawnFacing::Left,
            "initial orientation is authored by this occurrence, not by its character or brain"
        );

        let bad_facing = convert_err(&enemy(vec![
            named("brain", "mary_o_snake"),
            named("character_id", "solid_snake"),
            named("facing", "West-ish"),
        ]));
        assert!(
            bad_facing.contains("not one of Left / Right"),
            "a misspelled orientation must refuse rather than silently choose a direction: {bad_facing}"
        );

        // the display-name road is REFUSED, not defaulted. With the field required an authored
        // entity that names no creature cannot be lowered at all, and the conversion says which
        // entity and why.
        let missing = convert_err(&enemy(vec![named("brain", "mary_o_snake")]));
        assert!(
            missing.contains("authors no `character_id`"),
            "a placement naming no creature must be refused by name: {missing}"
        );

        // an authored-but-BLANK field is what the LDtk editor writes for a
        // field a human tabbed through. It reads as absent — which is now a
        // refusal rather than an identity nothing in the catalog can match.
        let blank = convert_err(&enemy(vec![
            named("brain", "mary_o_snake"),
            named("character_id", "   "),
        ]));
        assert!(
            blank.contains("authors no `character_id`"),
            "a whitespace-only field must refuse exactly as an absent one does: {blank}"
        );
    }

    /// They disagreed, so conversion minted `enemy_patrol_a` for a path nothing referenced by that
    /// name, and the gap was papered over downstream until sandbox's basement patroller stood still
    /// for months with two validators calling it healthy.
    ///
    /// asserted as AGREEMENT rather than against a literal, because a literal is exactly what a
    /// second copy of the rule would also satisfy.
    #[test]
    fn a_derived_path_id_is_a_spelling_the_resolvers_accept() {
        use crate::project::{LdtkEntityInstance, LdtkFieldInstance};

        let path = |fields: Vec<LdtkFieldInstance>| LdtkEntityInstance {
            iid: "KinematicPath-0139".into(),
            identifier: "KinematicPath".into(),
            pivot: Vec::new(),
            px: [0, 0],
            width: 8,
            height: 8,
            field_instances: fields,
        };

        // Sandbox's shipped basement path: no authored `id`, display name
        // `enemy patrol path A`, referenced as `Patrol:enemy_patrol_path_a`.
        let name = "enemy patrol path A";
        let derived = kinematic_path_lookup_id(&path(Vec::new()), name);
        assert!(
            ambition_platformer2d_world::rooms::kinematic_path_aliases(&derived, name)
                .any(|alias| alias == derived),
            "conversion minted `{derived}`, which the resolvers do not accept — \
             a path nothing can reference is a patrol that never moves"
        );
        assert_ne!(
            derived, "enemy_patrol_a",
            "`enemy_patrol_a` is what the DELETED second slug rule minted for \
             this name; if it is back, so is the second authority"
        );

        // An authored id still wins outright, and an unnameable one falls back
        // to the iid rather than to an empty key that collides with everything.
        let authored = path(vec![LdtkFieldInstance {
            identifier: "id".into(),
            value: serde_json::Value::String("lab_patrol_line".into()),
            real_editor_values: Vec::new(),
        }]);
        assert_eq!(kinematic_path_lookup_id(&authored, name), "lab_patrol_line");
        assert_eq!(
            kinematic_path_lookup_id(&path(Vec::new()), "  !! "),
            "KinematicPath-0139"
        );
    }

    /// `EnemySpawn` at `px`, patrolling the `KinematicPath` with the given iid.
    fn patroller(path_iid: Option<&str>, brain: Option<&str>) -> Vec<LdtkFieldInstance> {
        let mut fields = vec![LdtkFieldInstance {
            identifier: "character_id".into(),
            value: Value::String("goblin".into()),
            real_editor_values: Vec::new(),
        }];
        if let Some(iid) = path_iid {
            fields.push(LdtkFieldInstance {
                identifier: "path_ref".into(),
                // LDtk's canonical EntityRef shape; the other three keys are
                // the file's business and `field_entity_ref` reads only this one.
                value: serde_json::json!({ "entityIid": iid }),
                real_editor_values: Vec::new(),
            });
        }
        if let Some(brain) = brain {
            fields.push(LdtkFieldInstance {
                identifier: "brain".into(),
                value: Value::String(brain.into()),
                real_editor_values: Vec::new(),
            });
        }
        fields
    }

    /// Where `patrol_project` puts its path. `synthetic_level` is 640x480 and
    /// the converter rejects an out-of-bounds placement before it ever looks at a
    /// field, so a fixture that overflows the level fails every reference test
    /// for a reason that has nothing to do with references.
    const PATROL_PATH_PX: [i32; 2] = [120, 400];

    /// The iid `entity_at` will mint for that path — derived, so moving the
    /// fixture cannot leave a reference pointing at where it used to be.
    fn patrol_path_iid() -> String {
        format!(
            "KinematicPath-test-{}-{}",
            PATROL_PATH_PX[0], PATROL_PATH_PX[1]
        )
    }

    fn patrol_project(path_iid: Option<&str>, brain: Option<&str>) -> LdtkProject {
        let mut spawn = entity_at("EnemySpawn", [160, 380], [44, 58], &[]);
        spawn.field_instances = patroller(path_iid, brain);
        synthetic_level(vec![
            // Sandbox's shipped basement path: NO authored `id`, so its lookup
            // id is derived from the display name. The reference must land on
            // whatever that derivation produced, which is the whole point of
            // resolving through the target instead of through a spelling.
            entity_at(
                "KinematicPath",
                PATROL_PATH_PX,
                [360, 12],
                &[
                    ("name", Value::String("enemy patrol path A".into())),
                    ("points", Value::String("140,420;460,420".into())),
                    ("speed", Value::from(95.0)),
                ],
            ),
            spawn,
        ])
    }

    /// A native `path_ref` names the path the room actually built.
    ///
    /// this is the migration's whole claim. An `EntityRef` names the ENTITY, and conversion
    /// resolves it through the same `kinematic_path_lookup_id` that minted the target's own id, so
    /// the two cannot disagree.
    ///
    /// asserted as AGREEMENT with the room's own path table rather than
    /// against a literal id — a literal is exactly what a second derivation
    /// would also satisfy.
    #[test]
    fn a_native_path_ref_resolves_to_the_id_the_room_built_for_that_path() {
        let room = &patrol_project(Some(&patrol_path_iid()), None)
            .to_room_set_with_entry("registry_lab", &LdtkVocabulary::engine())
            .expect("the project composes")
            .rooms[0];

        let path_spec = room
            .kinematic_paths
            .first()
            .expect("precondition: the room built the KinematicPath");
        let brain = &room
            .enemy_spawns
            .first()
            .expect("precondition: the room built the EnemySpawn")
            .payload
            .brain;
        let ambition_entity_catalog::placements::CharacterBrain::Patrol { path_id } = brain else {
            panic!("a `path_ref` IS the patrol brain, and this is {brain:?}");
        };
        let path_id = path_id.as_deref().expect("a resolved ref names a path");
        assert_eq!(
            path_id, path_spec.id,
            "the reference resolved to `{path_id}`, and the room built the path \
             as `{}` — a reference nothing can resolve is a patrol that never moves",
            path_spec.id
        );
        // …and the poison the id must survive: the lookup table lowering builds
        // is generated from the spec, and THAT is what the body rides.
        assert!(
            ambition_platformer2d_world::rooms::kinematic_path_lookup(&room.kinematic_paths)
                .iter()
                .any(|(spelling, _)| spelling.as_str() == path_id),
            "`{path_id}` is not a spelling the runtime lookup table answers to"
        );
    }

    /// A dangling native ref is the one verdict this road owns outright — it is LDtk's own
    /// referential integrity, not a re-derived engine rule.
    #[test]
    fn a_path_ref_at_something_that_is_not_a_path_is_refused_by_name() {
        let errors = patrol_project(Some("KinematicPath-nobody-minted-this"), None)
            .to_room_set_with_entry("registry_lab", &LdtkVocabulary::engine())
            .expect_err("a dangling path_ref must not compose");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("KinematicPath-nobody-minted-this")),
            "the refusal must name the target an author can search for: {errors:?}"
        );
    }

    /// The retired string spelling is refused, not reinterpreted. With the
    /// `Patrol:` branch deleted, an un-migrated placement would otherwise parse
    /// as `CharacterBrain::Custom("Patrol:…")` and look exactly like a healthy
    /// one — which is the same silence the migration exists to end.
    #[test]
    fn the_retired_patrol_string_is_refused_out_loud() {
        let errors = patrol_project(None, Some("Patrol:enemy_patrol_path_a"))
            .to_room_set_with_entry("registry_lab", &LdtkVocabulary::engine())
            .expect_err("a `Patrol:` brain must not compose");
        assert!(
            errors.iter().any(|error| error.contains("path_ref")),
            "the refusal must say what to author instead: {errors:?}"
        );
        // And the other half: a placement may not say it twice.
        let both = patrol_project(Some(&patrol_path_iid()), Some("Guard:96"))
            .to_room_set_with_entry("registry_lab", &LdtkVocabulary::engine())
            .expect_err("`path_ref` beside a brain is two answers to one question");
        assert!(
            both.iter().any(|error| error.contains("Guard:96")),
            "the refusal must name the contradicting brain: {both:?}"
        );
    }
}
