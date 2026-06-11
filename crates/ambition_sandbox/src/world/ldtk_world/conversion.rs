//! LDtk → Ambition runtime conversion.
//!
//! Materializes the typed [`crate::rooms::RoomSet`] graph from a
//! validated [`super::project::LdtkProject`]. Per-entity routing
//! (`entity_to_runtime`) plus IntGrid → block / water / climbable
//! emission live here.

use std::collections::BTreeMap;

use crate::engine_core as ae;

use super::fields::{
    field_bool, field_f32, field_i32, field_string, parse_boss_brain, parse_debug_label_kind,
    parse_enemy_brain, parse_optional_path, parse_path_mode, parse_pickup_kind, parse_points,
};
use super::intgrid::{
    emit_climbable_regions_from_intgrid, emit_collision_blocks_from_intgrid,
    emit_water_regions_from_intgrid,
};
use super::project::{LdtkEntityInstance, LdtkLevel, LdtkProject};
use super::surfaces::{
    compile_surface, is_surface_like_identifier, parse_surface_spec, SurfaceCompiled,
};
use crate::rooms::{
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

        let start_room = if area_levels.contains_key("central_hub_complex") {
            "central_hub_complex".to_string()
        } else {
            area_levels
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| "central_hub_complex".to_string())
        };

        let links = self.collect_room_links();
        let mut rooms = Vec::new();
        for (area_id, levels) in area_levels {
            rooms.push(self.compose_runtime_area(&area_id, &levels)?);
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
        let mut moving_platforms: Vec<crate::world::platforms::MovingPlatformSpec> = Vec::new();
        let mut camera_zones: Vec<CameraZoneSpec> = Vec::new();
        let mut kinematic_paths: Vec<KinematicPathSpec> = Vec::new();
        let mut props: Vec<PropSpec> = Vec::new();
        let mut ground_items: Vec<crate::rooms::GroundItemSpec> = Vec::new();
        #[cfg(feature = "portal")]
        let mut portal_gun_spawns: Vec<crate::rooms::PortalGunSpawnSpec> = Vec::new();
        #[cfg(feature = "portal")]
        let mut portals: Vec<crate::rooms::PortalSpec> = Vec::new();
        let mut shrines: Vec<crate::rooms::ShrineSpec> = Vec::new();
        let mut gravity_zones: Vec<crate::rooms::GravityZoneSpec> = Vec::new();
        // Per-family authored entity lists. Each LDtk entity emits into
        // exactly one of these (or into one of the non-authored Vecs
        // above).
        let mut hazards: Vec<crate::rooms::Authored<crate::combat::DamageVolume>> = Vec::new();
        let mut interactables: Vec<crate::rooms::Authored<crate::interaction::Interactable>> =
            Vec::new();
        let mut pickups: Vec<crate::rooms::Authored<crate::interaction::Pickup>> = Vec::new();
        let mut chests: Vec<crate::rooms::Authored<crate::interaction::Chest>> = Vec::new();
        let mut breakables: Vec<crate::rooms::Authored<crate::interaction::Breakable>> = Vec::new();
        let mut enemy_spawns: Vec<crate::rooms::Authored<crate::actor::EnemyBrain>> = Vec::new();
        let mut boss_spawns: Vec<crate::rooms::Authored<crate::actor::BossBrain>> = Vec::new();
        let mut debug_labels: Vec<crate::rooms::Authored<crate::debug_label::DebugLabel>> =
            Vec::new();
        let mut metadata = crate::rooms::RoomMetadata::default();
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
                        #[cfg(feature = "portal")]
                        portal_gun_spawns.extend(emission.portal_gun_spawns);
                        #[cfg(feature = "portal")]
                        portals.extend(emission.portals);
                        shrines.extend(emission.shrines);
                        gravity_zones.extend(emission.gravity_zones);
                        hazards.extend(emission.hazards);
                        interactables.extend(emission.interactables);
                        pickups.extend(emission.pickups);
                        chests.extend(emission.chests);
                        breakables.extend(emission.breakables);
                        enemy_spawns.extend(emission.enemy_spawns);
                        boss_spawns.extend(emission.boss_spawns);
                        debug_labels.extend(emission.debug_labels);
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
                match emit_collision_blocks_from_intgrid(layer, offset) {
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
            .with_climbable_regions(climbable_regions),
            loading_zones,
            metadata,
            camera_zones,
            kinematic_paths,
            moving_platforms: resolved_moving_platforms,
            props,
            ground_items,
            #[cfg(feature = "portal")]
            portal_gun_spawns,
            #[cfg(feature = "portal")]
            portals,
            shrines,
            gravity_zones,
            hazards,
            interactables,
            pickups,
            chests,
            breakables,
            enemy_spawns,
            boss_spawns,
            debug_labels,
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
pub(super) struct RuntimeEntityEmission {
    pub(super) spawn: Option<ae::Vec2>,
    pub(super) blocks: Vec<ae::Block>,
    pub(super) zones: Vec<LoadingZone>,
    pub(super) water_regions: Vec<ae::WaterRegion>,
    /// LDtk-authored moving platforms emitted by this entity.
    ///
    /// Most entities emit zero platforms; `MovingPlatform` emits one. The room
    /// composer concatenates these so active areas can own multiple authored
    /// moving solids.
    pub(super) moving_platforms: Vec<crate::world::platforms::MovingPlatformSpec>,
    pub(super) camera_zones: Vec<CameraZoneSpec>,
    pub(super) kinematic_paths: Vec<KinematicPathSpec>,
    /// LDtk-authored decorative props emitted by this entity. Most
    /// entities emit zero; `Prop` emits one. Render-only — see
    /// [`PropSpec`].
    pub(super) props: Vec<PropSpec>,
    /// LDtk-authored ground held-items emitted by this entity. Most emit
    /// zero; `GroundItem` emits one. See [`crate::rooms::GroundItemSpec`].
    pub(super) ground_items: Vec<crate::rooms::GroundItemSpec>,
    /// LDtk-authored portal-gun pickups. Most emit zero; `PortalGunSpawn` emits
    /// one. See [`crate::rooms::PortalGunSpawnSpec`].
    #[cfg(feature = "portal")]
    pub(super) portal_gun_spawns: Vec<crate::rooms::PortalGunSpawnSpec>,
    /// LDtk-authored static portals. Most emit zero; `Portal` emits one. See
    /// [`crate::rooms::PortalSpec`].
    #[cfg(feature = "portal")]
    pub(super) portals: Vec<crate::rooms::PortalSpec>,
    /// LDtk-authored heal/save shrines. Most emit zero; `ShrineSpawn` emits one.
    pub(super) shrines: Vec<crate::rooms::ShrineSpec>,
    /// LDtk-authored localized-gravity zones. Most emit zero; `GravityZone` emits
    /// one. See [`crate::rooms::GravityZoneSpec`].
    pub(super) gravity_zones: Vec<crate::rooms::GravityZoneSpec>,
    // --- Per-family authored entity emissions:
    pub(super) hazards: Vec<crate::rooms::Authored<crate::combat::DamageVolume>>,
    pub(super) interactables: Vec<crate::rooms::Authored<crate::interaction::Interactable>>,
    pub(super) pickups: Vec<crate::rooms::Authored<crate::interaction::Pickup>>,
    pub(super) chests: Vec<crate::rooms::Authored<crate::interaction::Chest>>,
    pub(super) breakables: Vec<crate::rooms::Authored<crate::interaction::Breakable>>,
    pub(super) enemy_spawns: Vec<crate::rooms::Authored<crate::actor::EnemyBrain>>,
    pub(super) boss_spawns: Vec<crate::rooms::Authored<crate::actor::BossBrain>>,
    pub(super) debug_labels: Vec<crate::rooms::Authored<crate::debug_label::DebugLabel>>,
    pub(super) ignored: bool,
}

impl RuntimeEntityEmission {
    fn ignored() -> Self {
        Self {
            ignored: true,
            ..Self::default()
        }
    }

    fn spawn(value: ae::Vec2) -> Self {
        Self {
            spawn: Some(value),
            ..Self::default()
        }
    }

    fn zone(zone: LoadingZone) -> Self {
        Self {
            zones: vec![zone],
            ..Self::default()
        }
    }

    fn water_region(region: ae::WaterRegion) -> Self {
        Self {
            water_regions: vec![region],
            ..Self::default()
        }
    }

    fn moving_platform(spec: crate::world::platforms::MovingPlatformSpec) -> Self {
        Self {
            moving_platforms: vec![spec],
            ..Self::default()
        }
    }

    fn camera_zone(zone: CameraZoneSpec) -> Self {
        Self {
            camera_zones: vec![zone],
            ..Self::default()
        }
    }

    fn prop(spec: PropSpec) -> Self {
        Self {
            props: vec![spec],
            ..Self::default()
        }
    }

    fn ground_item(spec: crate::rooms::GroundItemSpec) -> Self {
        Self {
            ground_items: vec![spec],
            ..Self::default()
        }
    }

    #[cfg(feature = "portal_ldtk")]
    fn portal_gun_spawn(spec: crate::rooms::PortalGunSpawnSpec) -> Self {
        Self {
            portal_gun_spawns: vec![spec],
            ..Self::default()
        }
    }

    #[cfg(feature = "portal_ldtk")]
    fn portal(spec: crate::rooms::PortalSpec) -> Self {
        Self {
            portals: vec![spec],
            ..Self::default()
        }
    }

    fn shrine(spec: crate::rooms::ShrineSpec) -> Self {
        Self {
            shrines: vec![spec],
            ..Self::default()
        }
    }

    fn gravity_zone(spec: crate::rooms::GravityZoneSpec) -> Self {
        Self {
            gravity_zones: vec![spec],
            ..Self::default()
        }
    }

    fn kinematic_path(spec: KinematicPathSpec) -> Self {
        Self {
            kinematic_paths: vec![spec],
            ..Self::default()
        }
    }

    fn from_compiled(compiled: SurfaceCompiled) -> Self {
        Self {
            blocks: compiled.blocks,
            breakables: compiled.breakables,
            ..Self::default()
        }
    }

    // Per-family typed emitters. The conversion sites use these instead of
    // wrapping payloads in a generic `RoomObject { kind: ... }`.
    fn hazard(authored: crate::rooms::Authored<crate::combat::DamageVolume>) -> Self {
        Self {
            hazards: vec![authored],
            ..Self::default()
        }
    }

    fn interactable(authored: crate::rooms::Authored<crate::interaction::Interactable>) -> Self {
        Self {
            interactables: vec![authored],
            ..Self::default()
        }
    }

    fn pickup(authored: crate::rooms::Authored<crate::interaction::Pickup>) -> Self {
        Self {
            pickups: vec![authored],
            ..Self::default()
        }
    }

    fn chest(authored: crate::rooms::Authored<crate::interaction::Chest>) -> Self {
        Self {
            chests: vec![authored],
            ..Self::default()
        }
    }

    fn enemy_spawn(authored: crate::rooms::Authored<crate::actor::EnemyBrain>) -> Self {
        Self {
            enemy_spawns: vec![authored],
            ..Self::default()
        }
    }

    fn boss_spawn(authored: crate::rooms::Authored<crate::actor::BossBrain>) -> Self {
        Self {
            boss_spawns: vec![authored],
            ..Self::default()
        }
    }

    fn debug_label(authored: crate::rooms::Authored<crate::debug_label::DebugLabel>) -> Self {
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

pub(super) fn entity_to_runtime(
    entity: &LdtkEntityInstance,
    offset: ae::Vec2,
) -> Result<RuntimeEntityEmission, String> {
    let (min, size) = entity_min_size(entity, offset);
    let name = field_string(entity, "name").unwrap_or_else(|| entity.identifier.clone());

    // Surface-shaped identifiers (canonical `Surface` plus legacy aliases) all
    // share a typed parse → compile pipeline. Old JSON paths for `Solid`,
    // `OneWayPlatform`, `BlinkWall`, `HazardBlock`, `PogoOrb`, `ReboundPad`,
    // and `Breakable` are now routed through `LdtkSurfaceSpec` so future
    // collision/contact systems consume one typed runtime IR.
    if is_surface_like_identifier(&entity.identifier) {
        let spec = parse_surface_spec(entity, min, size, name)?;
        let compiled = compile_surface(&spec)?;
        return Ok(RuntimeEntityEmission::from_compiled(compiled));
    }

    match entity.identifier.as_str() {
        "PlayerStart" => Ok(RuntimeEntityEmission::spawn(min + size * 0.5)),
        "LoadingZone" => Ok(convert_loading_zone(entity, name, min, size)),
        "DamageVolume" => Ok(convert_damage_volume(entity, name, min, size, offset)),
        "KinematicPath" => convert_kinematic_path(entity, name, min, size, offset),
        "Prop" => convert_prop(entity, name, min, size),
        "NpcSpawn" => Ok(convert_npc_spawn(entity, name, min, size)),
        "PickupSpawn" => Ok(convert_pickup_spawn(entity, name, min, size)),
        "GroundItem" => convert_ground_item(entity, name, min, size),
        // Portal-authored entities require the `portal_ldtk` feature. Per the
        // refactor anti-goal ("do NOT make LDtk silently ignore portal-authored
        // entities when portal is disabled — fail loudly"), a portal-OFF /
        // portal_ldtk-OFF build returns an explicit conversion error here rather
        // than dropping the entity.
        #[cfg(feature = "portal_ldtk")]
        "PortalGunSpawn" => Ok(convert_portal_gun_spawn(entity, name, min, size)),
        #[cfg(feature = "portal_ldtk")]
        "Portal" => convert_portal(entity, name, min, size),
        #[cfg(not(feature = "portal_ldtk"))]
        ident @ ("PortalGunSpawn" | "Portal") => Err(format!(
            "portal-authored entity '{ident}' ('{}') encountered, but the portal \
             LDtk converter is compiled out (enable the `portal_ldtk` cargo \
             feature to author portal entities)",
            entity.identifier
        )),
        "ShrineSpawn" => Ok(convert_shrine(entity, name, min, size)),
        "GravityZone" => Ok(convert_gravity_zone(entity, name, min, size)),
        "ChestSpawn" => Ok(convert_chest_spawn(entity, name, min, size)),
        "EnemySpawn" => Ok(convert_enemy_spawn(entity, name, min, size)),
        "BossSpawn" => Ok(convert_boss_spawn(entity, name, min, size)),
        "DebugLabel" => Ok(convert_debug_label(entity, name, min, size)),
        "WaterVolume" => Ok(convert_water_volume(entity, min, size)),
        "MovingPlatform" => Ok(convert_moving_platform(entity, name, min, size)),
        "CameraZone" => Ok(convert_camera_zone(entity, name, min, size)),
        // StitchedBoundary / EncounterTrigger / LockWall are read by
        // their own consumers off the raw LdtkProject and never join
        // the generic RoomObject stream — emit nothing here.
        "StitchedBoundary" | "EncounterTrigger" | "LockWall" => {
            Ok(RuntimeEntityEmission::ignored())
        }
        "Switch" => Ok(convert_switch(entity, name, min, size)),
        _ => Err(format!(
            "unsupported entity identifier '{}'",
            entity.identifier
        )),
    }
}

fn convert_loading_zone(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    RuntimeEntityEmission::zone(LoadingZone {
        id: field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
        name,
        activation: match field_string(entity, "activation")
            .unwrap_or_else(|| "Door".to_string())
            .as_str()
        {
            "EdgeExit" => LoadingZoneActivation::EdgeExit,
            "Walk" | "walk" => LoadingZoneActivation::Walk,
            _ => LoadingZoneActivation::Door,
        },
        aabb: object_aabb(min, size),
    })
}

fn convert_damage_volume(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
    offset: ae::Vec2,
) -> RuntimeEntityEmission {
    let aabb = object_aabb(min, size);
    let mut volume = crate::combat::DamageVolume::new(
        entity.iid.clone(),
        aabb,
        field_i32(entity, "damage").unwrap_or(1),
    );
    volume.path_id = field_string(entity, "path_id")
        .or_else(|| field_string(entity, "patrol_path_id"))
        .and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });
    volume.motion = parse_optional_path(entity).map(|mut path| {
        path.points = offset_points(path.points, offset);
        path
    });
    RuntimeEntityEmission::hazard(crate::rooms::Authored::new(
        entity.iid.clone(),
        name,
        aabb,
        volume,
    ))
}

fn convert_kinematic_path(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
    offset: ae::Vec2,
) -> Result<RuntimeEntityEmission, String> {
    let points = offset_points(
        parse_points(&field_string(entity, "points").unwrap_or_default()),
        offset,
    );
    if points.len() < 2 {
        return Err("KinematicPath requires at least two points".to_string());
    }
    let speed = field_f32(entity, "speed").unwrap_or(100.0);
    if speed <= 0.0 {
        return Err("KinematicPath speed must be positive".to_string());
    }
    let path = crate::actor::KinematicPath {
        points,
        speed,
        mode: parse_path_mode(
            &field_string(entity, "mode").unwrap_or_else(|| "PingPong".to_string()),
        ),
        start_offset_seconds: field_f32(entity, "start_offset_seconds")
            .or_else(|| field_f32(entity, "start_offset"))
            .unwrap_or(0.0)
            .max(0.0),
    };
    Ok(RuntimeEntityEmission::kinematic_path(
        KinematicPathSpec::new(
            path_lookup_id(entity, &name),
            name,
            object_aabb(min, size),
            path,
        ),
    ))
}

fn convert_prop(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> Result<RuntimeEntityEmission, String> {
    // Decorative-only entity. Renders a sprite via `PropRegistry`, but
    // never grows an `Interactable` or a `RoomObject` — so the player
    // can walk past with no dialogue prompt and the engine never sees
    // it.
    let kind = field_string(entity, "kind").unwrap_or_default();
    if kind.trim().is_empty() {
        return Err("Prop requires non-empty `kind` field".to_string());
    }
    Ok(RuntimeEntityEmission::prop(PropSpec {
        id: entity.iid.clone(),
        name,
        kind: kind.trim().to_string(),
        pos: min + size * 0.5,
        size,
    }))
}

fn convert_npc_spawn(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    // Post-Phase 2: LDtk NpcSpawns carry a stable `character_id`
    // field that keys into `assets/data/character_catalog.ron`. The
    // resolved display name (`catalog.display_name`) becomes the
    // `Authored.name` so downstream sprite / banter / dialog lookups
    // — which still match on display name today — work unchanged.
    // Phase 6 lifts those consumers off display-name lookups in
    // favor of character_id keys; until then this translation is the
    // bridge.
    let character_id = field_string(entity, "character_id").unwrap_or_default();
    let display_name = if character_id.is_empty() {
        name
    } else {
        crate::character_roster::display_name_for_character_id(&character_id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| character_id.clone())
    };
    let interactable = crate::interaction::Interactable::new(
        entity.iid.clone(),
        field_string(entity, "prompt").unwrap_or_else(|| "Talk".to_string()),
        object_aabb(min, size),
        crate::interaction::InteractionKind::Npc {
            dialogue_id: field_string(entity, "dialogue_id"),
            // Optional `patrol_radius` field on NpcSpawn. 0 (or unset)
            // → static NPC unless `path_id` is set.
            patrol_radius: field_f32(entity, "patrol_radius").unwrap_or(0.0),
            patrol_path_id: field_string(entity, "path_id")
                .or_else(|| field_string(entity, "patrol_path_id")),
        },
    );
    let (id, name, aabb) = authored_triple(entity, display_name, min, size);
    RuntimeEntityEmission::interactable(crate::rooms::Authored::new(id, name, aabb, interactable))
}

fn convert_pickup_spawn(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    let pickup = crate::interaction::Pickup::new(
        entity.iid.clone(),
        parse_pickup_kind(&field_string(entity, "kind").unwrap_or_else(|| "health:1".to_string())),
    );
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    RuntimeEntityEmission::pickup(crate::rooms::Authored::new(id, name, aabb, pickup))
}

fn convert_ground_item(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> Result<RuntimeEntityEmission, String> {
    // Authored held-item pickup. `held_item` is a brain held-item registry id
    // (`meteor`, `bomb`, `puppy_slug_gun`, `gun_sword`, ...); resolution to a
    // `HeldItemSpec` happens at spawn, where an unregistered / feature-gated id
    // is tolerated (the item simply doesn't appear) rather than failing the
    // whole room load.
    let held_item = field_string(entity, "held_item").unwrap_or_default();
    if held_item.trim().is_empty() {
        return Err("GroundItem requires non-empty `held_item` field".to_string());
    }
    let id = field_string(entity, "id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entity.iid.clone());
    Ok(RuntimeEntityEmission::ground_item(
        crate::rooms::GroundItemSpec {
            id,
            name,
            held_item: held_item.trim().to_string(),
            pos: min + size * 0.5,
            half_extent: size * 0.5,
        },
    ))
}

#[cfg(feature = "portal_ldtk")]
fn convert_portal_gun_spawn(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    // Field-less marker (id/name optional): the box gives the pickup's center +
    // half-extent. Spawns an already-armed `PortalGunPickup` at room load.
    let id = field_string(entity, "id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entity.iid.clone());
    RuntimeEntityEmission::portal_gun_spawn(crate::rooms::PortalGunSpawnSpec {
        id,
        name,
        pos: min + size * 0.5,
        half_extent: size * 0.5,
    })
}

#[cfg(feature = "portal_ldtk")]
fn convert_portal(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> Result<RuntimeEntityEmission, String> {
    // `color` names the pair (its partner is the linked exit); `normal` is the
    // surface the portal sits on (up = floor, down = ceiling, left = right-wall,
    // right = left-wall — y is down in world space). The box center is the face.
    let color_str = field_string(entity, "color").unwrap_or_default();
    let color = crate::portal::PortalChannelColor::from_name(&color_str)
        .ok_or_else(|| format!("Portal '{name}' has unknown color '{color_str}'"))?;
    let normal = match field_string(entity, "normal").as_deref().map(str::trim) {
        Some("down") => ae::Vec2::new(0.0, 1.0),
        Some("left") => ae::Vec2::new(-1.0, 0.0),
        Some("right") => ae::Vec2::new(1.0, 0.0),
        Some("up") | None => ae::Vec2::new(0.0, -1.0),
        Some(other) => return Err(format!("Portal '{name}' has unknown normal '{other}'")),
    };
    // Explicit link id (preferred pairing); empty/absent ⇒ legacy color pairing.
    let link = field_string(entity, "link")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    // Authored opening size: the box dimension ALONG the surface (perpendicular
    // to the normal). Floor/ceiling (vertical normal) → width; wall → height.
    let along = if normal.x.abs() > 0.5 { size.y } else { size.x };
    let half_length = (along > 1.0).then_some(along * 0.5);
    Ok(RuntimeEntityEmission::portal(crate::rooms::PortalSpec {
        id: authored_id(entity),
        name,
        color,
        pos: min + size * 0.5,
        normal,
        link,
        half_length,
    }))
}

fn authored_id(entity: &LdtkEntityInstance) -> String {
    field_string(entity, "id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entity.iid.clone())
}

fn convert_shrine(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    RuntimeEntityEmission::shrine(crate::rooms::ShrineSpec {
        id: authored_id(entity),
        name,
        pos: min + size * 0.5,
        half_extent: size * 0.5,
    })
}

fn convert_gravity_zone(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    // `dir` names the gravity direction inside the zone; default up (the demo).
    let dir = match field_string(entity, "dir").as_deref().map(str::trim) {
        Some("down") => ae::Vec2::new(0.0, 1.0),
        Some("left") => ae::Vec2::new(-1.0, 0.0),
        Some("right") => ae::Vec2::new(1.0, 0.0),
        _ => ae::Vec2::new(0.0, -1.0),
    };
    RuntimeEntityEmission::gravity_zone(crate::rooms::GravityZoneSpec {
        id: authored_id(entity),
        name,
        center: min + size * 0.5,
        half_extent: size * 0.5,
        dir,
        oscillate_amplitude: field_f32(entity, "oscillate_amplitude").unwrap_or(0.0),
        oscillate_freq: field_f32(entity, "oscillate_freq").unwrap_or(1.0),
    })
}

fn convert_chest_spawn(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    let chest = crate::interaction::Chest::new(
        entity.iid.clone(),
        field_string(entity, "reward").map(|value| parse_pickup_kind(&value)),
    );
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    RuntimeEntityEmission::chest(crate::rooms::Authored::new(id, name, aabb, chest))
}

fn convert_enemy_spawn(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    let mut brain =
        parse_enemy_brain(&field_string(entity, "brain").unwrap_or_else(|| "Passive".to_string()));
    if let Some(path_id) =
        field_string(entity, "path_id").or_else(|| field_string(entity, "patrol_path_id"))
    {
        if !path_id.trim().is_empty() {
            brain = crate::actor::EnemyBrain::Patrol {
                path_id: Some(path_id.trim().to_string()),
            };
        }
    }
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    RuntimeEntityEmission::enemy_spawn(crate::rooms::Authored::new(id, name, aabb, brain))
}

fn convert_boss_spawn(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    let brain =
        parse_boss_brain(&field_string(entity, "brain").unwrap_or_else(|| "Dormant".to_string()));
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    RuntimeEntityEmission::boss_spawn(crate::rooms::Authored::new(id, name, aabb, brain))
}

fn convert_debug_label(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    let pos = min + size * 0.5;
    let aabb = ae::Aabb::new(pos, ae::Vec2::splat(1.0));
    let label = crate::debug_label::DebugLabel::new(
        field_string(entity, "text").unwrap_or_else(|| entity.identifier.clone()),
        pos,
        parse_debug_label_kind(
            &field_string(entity, "category").unwrap_or_else(|| "Custom".to_string()),
        ),
    );
    RuntimeEntityEmission::debug_label(crate::rooms::Authored::new(
        entity.iid.clone(),
        name,
        aabb,
        label,
    ))
}

fn convert_water_volume(
    entity: &LdtkEntityInstance,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    // Entity-authored water: source-agnostic, lands in the same
    // `World::water_regions` list IntGrid Water cells populate.
    // Reserved for irregular pools the per-cell IntGrid layer can't
    // shape.
    let mut spec = ae::WaterVolumeSpec::default();
    if let Some(value) = field_f32(entity, "gravity_scale") {
        spec.gravity_scale = value;
    }
    if let Some(value) = field_f32(entity, "drag") {
        spec.drag = value;
    }
    if let Some(value) = field_f32(entity, "max_fall_speed") {
        spec.max_fall_speed = value;
    }
    if let Some(value) = field_f32(entity, "swim_up_impulse") {
        spec.swim_up_impulse = value;
    }
    // Entity water defaults to Clear. The IntGrid Water layer is the
    // canonical authoring path for distinct kinds; if a future entity
    // field needs Murky, add a `kind` field via
    // `register_ldtk_entity_def.py` and route it here.
    RuntimeEntityEmission::water_region(ae::WaterRegion::new(
        object_aabb(min, size),
        ae::WaterKind::Clear,
        spec,
    ))
}

fn convert_moving_platform(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    // LDtk entity bounds define platform size and, for the legacy sweep
    // mode, starting AABB. When `path_id` is authored, the platform
    // follows the referenced active-area-local `KinematicPathSpec`
    // instead and uses its first point as the runtime center.
    let start_pos = min + size * 0.5;
    let sweep_dx = field_f32(entity, "sweep_dx").unwrap_or(240.0);
    let speed = field_f32(entity, "speed").unwrap_or(130.0);
    let path_id =
        field_string(entity, "path_id").or_else(|| field_string(entity, "patrol_path_id"));
    RuntimeEntityEmission::moving_platform(
        crate::world::platforms::MovingPlatformSpec::from_authored(
            entity.iid.clone(),
            name,
            start_pos,
            size,
            sweep_dx,
            speed,
            path_id,
        ),
    )
}

fn convert_camera_zone(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    RuntimeEntityEmission::camera_zone(CameraZoneSpec {
        id: field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
        name,
        aabb: object_aabb(min, size),
        priority: field_i32(entity, "priority").unwrap_or(0),
        zoom: field_f32(entity, "zoom").or_else(|| field_f32(entity, "camera_zoom")),
        target_offset: ae::Vec2::new(
            field_f32(entity, "target_offset_x").unwrap_or(0.0),
            field_f32(entity, "target_offset_y").unwrap_or(0.0),
        ),
        easing_hz: field_f32(entity, "easing_hz"),
        cinematic_lock: field_bool(entity, "cinematic_lock")
            .or_else(|| field_bool(entity, "lock_to_zone"))
            .unwrap_or(false),
        clamp_mode: CameraClampMode::from_author_value(
            field_string(entity, "clamp_mode").as_deref(),
        ),
    })
}

/// Convert an LDtk `Switch` entity into a runtime [`crate::interaction::Interactable`]
/// carrying the wire-format custom payload.
///
/// The `SwitchFeature` spawn path re-parses the payload into a typed
/// [`crate::encounter::SwitchActivation`] once, so downstream gameplay
/// systems never touch the string form.
fn convert_switch(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
) -> RuntimeEntityEmission {
    let activation = crate::encounter::SwitchActivation {
        id: field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
        action: field_string(entity, "action").unwrap_or_else(|| "ResetEncounter".into()),
        target_encounter: field_string(entity, "target_encounter").unwrap_or_default(),
    };
    let aabb = object_aabb(min, size);
    let interactable = crate::interaction::Interactable::new(
        activation.id.clone(),
        field_string(entity, "prompt").unwrap_or_else(|| "Activate".into()),
        aabb,
        crate::interaction::InteractionKind::Custom(activation.to_custom_payload()),
    );
    // Use the LDtk field `id` (carried on activation) for the
    // authored entity id so the SwitchRuntime id matches the
    // SwitchActivation id. The entity.iid would default to something
    // like "Switch-4072"; that mismatch silently no-op'd switch state
    // updates and left the switch sprite stuck red.
    RuntimeEntityEmission::interactable(crate::rooms::Authored::new(
        activation.id,
        name,
        aabb,
        interactable,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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
