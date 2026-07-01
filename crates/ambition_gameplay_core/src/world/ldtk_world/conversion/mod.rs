//! LDtk → Ambition runtime conversion.
//!
//! Materializes the typed [`crate::rooms::RoomSet`] graph from a
//! validated [`super::project::LdtkProject`]. Per-entity routing
//! (`entity_to_runtime`) plus IntGrid → block / water / climbable
//! emission live here.

use std::collections::BTreeMap;

use ambition_engine_core as ae;

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
        let mut interactables: Vec<crate::rooms::Authored<ambition_interaction::Interactable>> =
            Vec::new();
        let mut pickups: Vec<crate::rooms::Authored<ambition_interaction::Pickup>> = Vec::new();
        let mut chests: Vec<crate::rooms::Authored<ambition_interaction::Chest>> = Vec::new();
        let mut breakables: Vec<crate::rooms::Authored<ambition_interaction::Breakable>> =
            Vec::new();
        let mut enemy_spawns: Vec<
            crate::rooms::Authored<ambition_characters::actor::CharacterBrain>,
        > = Vec::new();
        let mut boss_spawns: Vec<crate::rooms::Authored<ambition_characters::actor::BossBrain>> =
            Vec::new();
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
    pub(super) interactables: Vec<crate::rooms::Authored<ambition_interaction::Interactable>>,
    pub(super) pickups: Vec<crate::rooms::Authored<ambition_interaction::Pickup>>,
    pub(super) chests: Vec<crate::rooms::Authored<ambition_interaction::Chest>>,
    pub(super) breakables: Vec<crate::rooms::Authored<ambition_interaction::Breakable>>,
    pub(super) enemy_spawns:
        Vec<crate::rooms::Authored<ambition_characters::actor::CharacterBrain>>,
    pub(super) boss_spawns: Vec<crate::rooms::Authored<ambition_characters::actor::BossBrain>>,
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

    fn interactable(authored: crate::rooms::Authored<ambition_interaction::Interactable>) -> Self {
        Self {
            interactables: vec![authored],
            ..Self::default()
        }
    }

    fn pickup(authored: crate::rooms::Authored<ambition_interaction::Pickup>) -> Self {
        Self {
            pickups: vec![authored],
            ..Self::default()
        }
    }

    fn chest(authored: crate::rooms::Authored<ambition_interaction::Chest>) -> Self {
        Self {
            chests: vec![authored],
            ..Self::default()
        }
    }

    fn enemy_spawn(
        authored: crate::rooms::Authored<ambition_characters::actor::CharacterBrain>,
    ) -> Self {
        Self {
            enemy_spawns: vec![authored],
            ..Self::default()
        }
    }

    fn boss_spawn(authored: crate::rooms::Authored<ambition_characters::actor::BossBrain>) -> Self {
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

mod entity_converters;
use entity_converters::*;

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
