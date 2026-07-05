//! The engine's standard LDtk entity converters — the built-in vocabulary
//! registered in `standard_converters()`. One `convert_*` per LDtk entity
//! type, all with the uniform [`LdtkEntityConverter`] signature so the
//! registry treats engine and content converters identically. Helpers +
//! `RuntimeEntityEmission` stay in the parent and are reached via `super::*`
//! (descendant visibility).

use super::*;

/// `PlayerStart` — the area's spawn point (box center).
pub(super) fn convert_player_start(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    Ok(RuntimeEntityEmission::spawn(ctx.min + ctx.size * 0.5))
}

/// Surface-shaped identifiers (`Solid`, `OneWayPlatform`, `BlinkWall`,
/// `HazardBlock`, `PogoOrb`, `ReboundPad`, `BreakablePlatform`,
/// `BreakablePogoOrb`) all share one typed parse → compile pipeline, so
/// collision/contact systems consume a single runtime IR.
pub(super) fn convert_surface(ctx: &LdtkEntityCtx<'_>) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let spec = parse_surface_spec(entity, min, size, name)?;
    let compiled = compile_surface(&spec)?;
    Ok(RuntimeEntityEmission::from_compiled(compiled))
}

/// `StitchedBoundary` / `EncounterTrigger` / `LockWall` are read by their own
/// consumers off the raw `LdtkProject` and never join the emission stream.
pub(super) fn convert_consumed_elsewhere(
    _ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    Ok(RuntimeEntityEmission::ignored())
}

pub(super) fn convert_loading_zone(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    Ok(RuntimeEntityEmission::zone(LoadingZone {
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
    }))
}

pub(super) fn convert_damage_volume(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let offset = ctx.offset;
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
    Ok(RuntimeEntityEmission::hazard(crate::rooms::Authored::new(
        entity.iid.clone(),
        name,
        aabb,
        volume,
    )))
}

/// `SurfaceChain` — a rideable surface polyline (demo plan S3, momentum
/// locomotion). Fields: `points` (semicolon `x,y` pairs, level-local — the
/// KinematicPath convention), optional `closed: bool` (a loop; the closing
/// segment is implicit). One-sided by winding (author floors left→right);
/// the engine validator runs at conversion so inverted joins / degenerate
/// segments / self-intersections fail LOUDLY here instead of masquerading as
/// physics bugs in play.
pub(super) fn convert_surface_chain(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, _min, _size) = ctx.parts();
    let points = offset_points(
        parse_points(&field_string(entity, "points").unwrap_or_default()),
        ctx.offset,
    );
    let closed = field_bool(entity, "closed").unwrap_or(false);
    let chain = if closed {
        ae::SurfaceChain::closed_loop(name, points)
    } else {
        ae::SurfaceChain::open(name, points)
    };
    let problems = chain.validate();
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    Ok(RuntimeEntityEmission::chain(chain))
}

pub(super) fn convert_kinematic_path(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let offset = ctx.offset;
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
    let path = ambition_characters::actor::KinematicPath {
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

pub(super) fn convert_prop(ctx: &LdtkEntityCtx<'_>) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
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

pub(super) fn convert_npc_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
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
    let interactable = ambition_interaction::Interactable::new(
        entity.iid.clone(),
        field_string(entity, "prompt").unwrap_or_else(|| "Talk".to_string()),
        object_aabb(min, size),
        ambition_interaction::InteractionKind::Npc {
            character_id: (!character_id.is_empty()).then(|| character_id.clone()),
            dialogue_id: field_string(entity, "dialogue_id"),
            // Optional `patrol_radius` field on NpcSpawn. 0 (or unset)
            // → static NPC unless `path_id` is set.
            patrol_radius: field_f32(entity, "patrol_radius").unwrap_or(0.0),
            patrol_path_id: field_string(entity, "path_id")
                .or_else(|| field_string(entity, "patrol_path_id")),
        },
    );
    let (id, name, aabb) = authored_triple(entity, display_name, min, size);
    Ok(RuntimeEntityEmission::interactable(
        crate::rooms::Authored::new(id, name, aabb, interactable),
    ))
}

pub(super) fn convert_pickup_spawn(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let pickup = ambition_interaction::Pickup::new(
        entity.iid.clone(),
        parse_pickup_kind(&field_string(entity, "kind").unwrap_or_else(|| "health:1".to_string())),
    );
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    Ok(RuntimeEntityEmission::pickup(crate::rooms::Authored::new(
        id, name, aabb, pickup,
    )))
}

pub(super) fn convert_ground_item(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
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
pub(super) fn convert_portal_gun_spawn(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // Field-less marker (id/name optional): the box gives the pickup's center +
    // half-extent. Spawns an already-armed `PortalGunPickup` at room load.
    let id = field_string(entity, "id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entity.iid.clone());
    Ok(RuntimeEntityEmission::portal_gun_spawn(
        crate::rooms::PortalGunSpawnSpec {
            id,
            name,
            pos: min + size * 0.5,
            half_extent: size * 0.5,
        },
    ))
}

#[cfg(feature = "portal_ldtk")]
pub(super) fn convert_portal(ctx: &LdtkEntityCtx<'_>) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
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

// Portal-authored entities require the `portal_ldtk` feature. Per the
// refactor anti-goal ("do NOT make LDtk silently ignore portal-authored
// entities when portal is disabled — fail loudly"), a portal-OFF /
// portal_ldtk-OFF build registers explicit ERROR converters rather than
// dropping the entities.
#[cfg(not(feature = "portal_ldtk"))]
pub(super) fn convert_portal_gun_spawn(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    portal_compiled_out(ctx)
}

#[cfg(not(feature = "portal_ldtk"))]
pub(super) fn convert_portal(ctx: &LdtkEntityCtx<'_>) -> Result<RuntimeEntityEmission, String> {
    portal_compiled_out(ctx)
}

#[cfg(not(feature = "portal_ldtk"))]
fn portal_compiled_out(ctx: &LdtkEntityCtx<'_>) -> Result<RuntimeEntityEmission, String> {
    Err(format!(
        "portal-authored entity '{}' encountered, but the portal \
         LDtk converter is compiled out (enable the `portal_ldtk` cargo \
         feature to author portal entities)",
        ctx.entity.identifier
    ))
}

fn authored_id(entity: &LdtkEntityInstance) -> String {
    field_string(entity, "id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entity.iid.clone())
}

pub(super) fn convert_shrine(ctx: &LdtkEntityCtx<'_>) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    Ok(RuntimeEntityEmission::shrine(crate::rooms::ShrineSpec {
        id: authored_id(entity),
        name,
        pos: min + size * 0.5,
        half_extent: size * 0.5,
    }))
}

pub(super) fn convert_gravity_zone(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // `dir` names the gravity direction inside the zone; default up (the demo).
    let dir = match field_string(entity, "dir").as_deref().map(str::trim) {
        Some("down") => ae::Vec2::new(0.0, 1.0),
        Some("left") => ae::Vec2::new(-1.0, 0.0),
        Some("right") => ae::Vec2::new(1.0, 0.0),
        _ => ae::Vec2::new(0.0, -1.0),
    };
    Ok(RuntimeEntityEmission::gravity_zone(
        crate::rooms::GravityZoneSpec {
            id: authored_id(entity),
            name,
            center: min + size * 0.5,
            half_extent: size * 0.5,
            dir,
            oscillate_amplitude: field_f32(entity, "oscillate_amplitude").unwrap_or(0.0),
            oscillate_freq: field_f32(entity, "oscillate_freq").unwrap_or(1.0),
        },
    ))
}

pub(super) fn convert_chest_spawn(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let chest = ambition_interaction::Chest::new(
        entity.iid.clone(),
        field_string(entity, "reward").map(|value| parse_pickup_kind(&value)),
    );
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    Ok(RuntimeEntityEmission::chest(crate::rooms::Authored::new(
        id, name, aabb, chest,
    )))
}

pub(super) fn convert_enemy_spawn(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let mut brain =
        parse_enemy_brain(&field_string(entity, "brain").unwrap_or_else(|| "Passive".to_string()));
    if let Some(path_id) =
        field_string(entity, "path_id").or_else(|| field_string(entity, "patrol_path_id"))
    {
        if !path_id.trim().is_empty() {
            brain = ambition_characters::actor::CharacterBrain::Patrol {
                path_id: Some(path_id.trim().to_string()),
            };
        }
    }
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    let mut emission = RuntimeEntityEmission::enemy_spawn(crate::rooms::Authored::new(
        id.clone(),
        name,
        aabb,
        brain,
    ));
    // ADR 0020: a rider EnemySpawn carrying a `mounted_on` entity-ref emits an
    // authored mount link `(rider_id, mount_id)`. The ref stores the mount's
    // LDtk `iid`; authored linked pairs carry no explicit `id` field, so the
    // mount's `FeatureId` equals its `iid` and resolution matches on it.
    if let Some(mount_id) = field_entity_ref(entity, "mounted_on") {
        emission.mount_links.push((id, mount_id));
    }
    Ok(emission)
}

pub(super) fn convert_boss_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let brain =
        parse_boss_brain(&field_string(entity, "brain").unwrap_or_else(|| "Dormant".to_string()));
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    Ok(RuntimeEntityEmission::boss_spawn(
        crate::rooms::Authored::new(id, name, aabb, brain),
    ))
}

pub(super) fn convert_debug_label(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let pos = min + size * 0.5;
    let aabb = ae::Aabb::new(pos, ae::Vec2::splat(1.0));
    let label = crate::debug_label::DebugLabel::new(
        field_string(entity, "text").unwrap_or_else(|| entity.identifier.clone()),
        pos,
        parse_debug_label_kind(
            &field_string(entity, "category").unwrap_or_else(|| "Custom".to_string()),
        ),
    );
    Ok(RuntimeEntityEmission::debug_label(
        crate::rooms::Authored::new(entity.iid.clone(), name, aabb, label),
    ))
}

pub(super) fn convert_water_volume(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, _name, min, size) = ctx.parts();
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
    Ok(RuntimeEntityEmission::water_region(ae::WaterRegion::new(
        object_aabb(min, size),
        ae::WaterKind::Clear,
        spec,
    )))
}

pub(super) fn convert_moving_platform(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // LDtk entity bounds define platform size and, for the legacy sweep
    // mode, starting AABB. When `path_id` is authored, the platform
    // follows the referenced active-area-local `KinematicPathSpec`
    // instead and uses its first point as the runtime center.
    let start_pos = min + size * 0.5;
    let sweep_dx = field_f32(entity, "sweep_dx").unwrap_or(240.0);
    let speed = field_f32(entity, "speed").unwrap_or(130.0);
    let path_id =
        field_string(entity, "path_id").or_else(|| field_string(entity, "patrol_path_id"));
    Ok(RuntimeEntityEmission::moving_platform(
        crate::world::platforms::MovingPlatformSpec::from_authored(
            entity.iid.clone(),
            name,
            start_pos,
            size,
            sweep_dx,
            speed,
            path_id,
        ),
    ))
}

pub(super) fn convert_camera_zone(
    ctx: &LdtkEntityCtx<'_>,
) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    Ok(RuntimeEntityEmission::camera_zone(CameraZoneSpec {
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
    }))
}

/// Convert an LDtk `Switch` entity into a runtime [`ambition_interaction::Interactable`]
/// carrying the wire-format custom payload.
///
/// The `SwitchFeature` spawn path re-parses the payload into a typed
/// [`crate::encounter::SwitchActivation`] once, so downstream gameplay
/// systems never touch the string form.
pub(super) fn convert_switch(ctx: &LdtkEntityCtx<'_>) -> Result<RuntimeEntityEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let activation = crate::encounter::SwitchActivation {
        id: field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
        action: field_string(entity, "action").unwrap_or_else(|| "ResetEncounter".into()),
        target_encounter: field_string(entity, "target_encounter").unwrap_or_default(),
    };
    let aabb = object_aabb(min, size);
    let interactable = ambition_interaction::Interactable::new(
        activation.id.clone(),
        field_string(entity, "prompt").unwrap_or_else(|| "Activate".into()),
        aabb,
        ambition_interaction::InteractionKind::Custom(activation.to_custom_payload()),
    );
    // Use the LDtk field `id` (carried on activation) for the
    // authored entity id so the SwitchRuntime id matches the
    // SwitchActivation id. The entity.iid would default to something
    // like "Switch-4072"; that mismatch silently no-op'd switch state
    // updates and left the switch sprite stuck red.
    Ok(RuntimeEntityEmission::interactable(
        crate::rooms::Authored::new(activation.id, name, aabb, interactable),
    ))
}
