//! The engine's standard LDtk entity converters — the built-in vocabulary
//! registered in `standard_converters()`. One `convert_*` per LDtk entity
//! type, all with the uniform [`LdtkEntityConverter`] signature so the
//! registry treats engine and content converters identically. Helpers +
//! `RoomEmission` stay in the parent and are reached via `super::*`
//! (descendant visibility).

use super::*;
use ambition_entity_catalog::placements::{HazardSpec, PlacementSchema};

/// `PlayerStart` — the area's spawn point (box center).
pub(super) fn convert_player_start(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    Ok(RoomEmission::spawn(ctx.min + ctx.size * 0.5))
}

/// Surface-shaped identifiers (`Solid`, `OneWayPlatform`, `BlinkWall`,
/// `HazardBlock`, `PogoOrb`, `ReboundPad`, `BreakablePlatform`,
/// `BreakablePogoOrb`) all share one typed parse → compile pipeline, so
/// collision/contact systems consume a single runtime IR.
pub(super) fn convert_surface(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let spec = parse_surface_spec(entity, min, size, name)?;
    let compiled = compile_surface(&spec)?;
    Ok(RoomEmission::from_compiled(compiled))
}

/// `StitchedBoundary` / `EncounterTrigger` / `LockWall` are read by their own
/// consumers off the raw `LdtkProject` and never join the emission stream.
pub(super) fn convert_consumed_elsewhere(_ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    Ok(RoomEmission::ignored())
}

pub(super) fn convert_loading_zone(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    Ok(RoomEmission::zone(LoadingZone {
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

pub(super) fn convert_damage_volume(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let offset = ctx.offset;
    let aabb = object_aabb(min, size);
    let mut volume =
        ambition_world::rooms::HazardVolumeSpec::new(field_i32(entity, "damage").unwrap_or(1));
    volume.path_id = field_string(entity, "path_id")
        .or_else(|| field_string(entity, "patrol_path_id"))
        .and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });
    let inline_motion = parse_optional_path(entity).map(|mut path| {
        path.points = offset_points(path.points, offset);
        path
    });
    // Hazards flow through the single `placements` channel (fable audit F9.2 arc
    // exit). The Tier-0 `HazardSpec` carries `path_id` only, so a legacy INLINE
    // motion path is LIFTED here to a room-level `KinematicPath`: synthesize a
    // path entry keyed by `{iid}__inline_motion`, reference it via `path_id`, and
    // emit a normal hazard placement. This is behavior-preserving —
    // `HazardRuntime::new_with_paths` resolves `path_id` to that room path and
    // sets `volume.motion`, identical to the old inline case. (No live map
    // authored inline-motion hazards — F7 audited all four .ldtk files — so this
    // is the F7 dissolution completing, not a live behavior change.)
    let mut kinematic_paths = Vec::new();
    let path_id = if let Some(motion) = inline_motion {
        let synth_id = format!("{}__inline_motion", entity.iid);
        kinematic_paths.push(KinematicPathSpec {
            id: synth_id.clone(),
            name: synth_id.clone(),
            aabb,
            path: motion,
        });
        Some(synth_id)
    } else {
        volume.path_id.clone()
    };
    let mut record = ambition_world::placements::PlacementRecord::new(
        entity.iid.clone(),
        PlacementSchema::Hazard(HazardSpec {
            damage: volume.damage,
            knockback: volume.knockback,
            kind: volume.kind,
            team: volume.team,
            hitstop_seconds: volume.hitstop_seconds,
            respawn: volume.respawn,
            path_id,
        }),
        aabb,
    );
    record.name = name.clone();
    Ok(RoomEmission {
        placements: vec![record],
        kinematic_paths,
        ..Default::default()
    })
}

/// `SurfaceChain` — a rideable surface polyline (demo plan S3, momentum
/// locomotion). Fields: `points` (semicolon `x,y` pairs, level-local — the
/// KinematicPath convention), optional `closed: bool` (a loop; the closing
/// segment is implicit). One-sided by winding (author floors left→right);
/// the engine validator runs at conversion so inverted joins / degenerate
/// segments / self-intersections fail LOUDLY here instead of masquerading as
/// physics bugs in play.
pub(super) fn convert_surface_chain(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
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
    Ok(RoomEmission::chain(chain))
}

/// `SurfaceLoop` — a rideable full LOOP (the Sonic loop), authored as a MARKER
/// so a level need not hand-plot 24 points (demo plan S3b / Q17). Fields:
/// `radius` (px, required) and optional `segments` (polygon resolution, default
/// 24, min 3). The converter GENERATES the closed polygon chain at conversion
/// time and emits it into the same `chains` channel `SurfaceChain` uses — the
/// second real consumer of the content-registered converter seam, strictly
/// better than a script-injection escape hatch.
///
/// Winding is fixed INTERIOR-rideable: vertices wind so each segment's
/// `normal = (t.y, -t.x)` points toward the loop center, so a momentum body
/// rides the INSIDE of the loop (up the wall, across the ceiling, and down).
/// The marker's center (`min + size/2`) is the loop center.
pub(super) fn convert_surface_loop(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let radius = field_f32(entity, "radius").unwrap_or(0.0);
    if radius <= 0.0 {
        return Err("SurfaceLoop requires a positive `radius`".to_string());
    }
    let segments = field_i32(entity, "segments").unwrap_or(24).max(3) as usize;
    let center = min + size * 0.5;
    // Decreasing-angle winding → inward normals (interior-rideable). Vertex k at
    // angle -2πk/n: the first segment heads "up" the right wall, and the shared
    // `(t.y,-t.x)` rule points its normal toward the center.
    let points: Vec<ae::Vec2> = (0..segments)
        .map(|k| {
            let theta = -std::f32::consts::TAU * (k as f32) / (segments as f32);
            center + ae::Vec2::new(theta.cos(), theta.sin()) * radius
        })
        .collect();
    let points = offset_points(points, ctx.offset);
    let loop_chain = ae::SurfaceChain::closed_loop(name, points);
    let problems = loop_chain.validate();
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    Ok(RoomEmission::chain(loop_chain))
}

/// The four corners a [`convert_surface_ramp`] fillet can round.
///
/// Named by the two surfaces it joins, in travel order: `FloorToRightWall` runs
/// along a floor in `+x` and turns up a wall on its right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RampOrientation {
    FloorToRightWall,
    FloorToLeftWall,
    CeilingToRightWall,
    CeilingToLeftWall,
}

impl RampOrientation {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "FloorToRightWall" => Some(Self::FloorToRightWall),
            "FloorToLeftWall" => Some(Self::FloorToLeftWall),
            "CeilingToRightWall" => Some(Self::CeilingToRightWall),
            "CeilingToLeftWall" => Some(Self::CeilingToLeftWall),
            _ => None,
        }
    }

    /// The SOLID corner the fillet rounds, within the marker's box.
    fn corner(self, min: ae::Vec2, max: ae::Vec2) -> ae::Vec2 {
        // `+y` is DOWN, so a floor is at `max.y` and a ceiling at `min.y`.
        match self {
            Self::FloorToRightWall => ae::Vec2::new(max.x, max.y),
            Self::FloorToLeftWall => ae::Vec2::new(min.x, max.y),
            Self::CeilingToRightWall => ae::Vec2::new(max.x, min.y),
            Self::CeilingToLeftWall => ae::Vec2::new(min.x, min.y),
        }
    }

    /// Unit vector from the corner toward the arc CENTER — i.e. into the open
    /// room. Each component is `±1`, so the base arc table mirrors by sign alone.
    fn into_room(self) -> ae::Vec2 {
        match self {
            Self::FloorToRightWall => ae::Vec2::new(-1.0, -1.0),
            Self::FloorToLeftWall => ae::Vec2::new(1.0, -1.0),
            Self::CeilingToRightWall => ae::Vec2::new(-1.0, 1.0),
            Self::CeilingToLeftWall => ae::Vec2::new(1.0, 1.0),
        }
    }
}

/// `SurfaceRamp` — the **quarter-circle fillet** that lets a momentum body carry
/// its speed from a floor onto a wall (Q27; pinned math in
/// `docs/planning/engine/spatial-model.md` §`SurfaceRamp`).
///
/// Fields: `radius` (px, required), `orientation` (one of the four
/// [`RampOrientation`] names, default `FloorToRightWall`), `segments` (polygon
/// resolution, default 8, min 2).
///
/// AMBITION_REVIEW(spatial): the arc table is the doc's, verbatim, for the base
/// orientation. **The winding sign is NOT hand-derived per orientation** — the
/// doc explicitly forbids that, and it is right to: a floor's outward normal must
/// point up, a ceiling's down, and mirroring the point list flips the sign of
/// `normal = (t.y, -t.x)` in a way that is genuinely hard to see. Instead the
/// converter DERIVES the order: a fillet's surface normal points from the arc
/// toward its center, so the point list is reversed whenever the first segment's
/// normal points away from it. One code path, four orientations, and the winding
/// oracle (`surface_ramp_winding_oracle`) proves each one by riding it.
pub(super) fn convert_surface_ramp(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let radius = field_f32(entity, "radius").unwrap_or(0.0);
    if radius <= 0.0 {
        return Err("SurfaceRamp requires a positive `radius`".to_string());
    }
    let orientation = match field_string(entity, "orientation") {
        Some(s) => RampOrientation::parse(&s)
            .ok_or_else(|| format!("SurfaceRamp: unknown orientation `{s}`"))?,
        None => RampOrientation::FloorToRightWall,
    };
    let segments = field_i32(entity, "segments").unwrap_or(8).max(2) as usize;

    let points = surface_ramp_points(
        orientation.corner(min, min + size),
        radius,
        orientation,
        segments,
    );
    let points = offset_points(points, ctx.offset);
    let chain = ae::SurfaceChain::open(name, points);
    let problems = chain.validate();
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    Ok(RoomEmission::chain(chain))
}

/// The arc, in the order that makes its normals point into the room.
///
/// Base table (spatial-model.md, `+y` is DOWN, `FloorToRightWall`, corner
/// `(x1, y0)`):
///
/// ```text
/// C    = (x1 − r, y0 − r)
/// P(θ) = C + r · (sin θ, cos θ),  θ ∈ [0°, 90°]
/// P(0°)  = (x1 − r, y0)   — tangent on the floor
/// P(90°) = (x1, y0 − r)   — tangent on the wall
/// ```
///
/// The other three are axis mirrors of the SAME table, which `into_room()`'s
/// `±1` components apply.
pub fn surface_ramp_points(
    corner: ae::Vec2,
    radius: f32,
    orientation: RampOrientation,
    segments: usize,
) -> Vec<ae::Vec2> {
    let sign = orientation.into_room();
    let center = corner + sign * radius;
    let n = segments.max(2);
    let mut points: Vec<ae::Vec2> = (0..=n)
        .map(|k| {
            let theta = std::f32::consts::FRAC_PI_2 * (k as f32) / (n as f32);
            // Base table, then mirrored by `sign`. `sin` runs along the wall axis
            // and `cos` along the floor axis, exactly as the doc writes it.
            center + ae::Vec2::new(-sign.x * theta.sin(), -sign.y * theta.cos()) * radius
        })
        .collect();

    // Derive the winding rather than tabulate it. `SurfaceChain`'s outward normal
    // is `(t.y, -t.x)`; on a fillet it must point from the surface toward the arc
    // center. If the first segment's does not, the list runs the wrong way.
    let t = (points[1] - points[0]).normalize_or_zero();
    let normal = ae::Vec2::new(t.y, -t.x);
    if normal.dot(center - points[0]) < 0.0 {
        points.reverse();
    }
    points
}

pub(super) fn convert_kinematic_path(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
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
    let path = ambition_engine_core::KinematicPath {
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
    Ok(RoomEmission::kinematic_path(KinematicPathSpec::new(
        path_lookup_id(entity, &name),
        name,
        object_aabb(min, size),
        path,
    )))
}

pub(super) fn convert_prop(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // Decorative-only entity. Renders a sprite via `PropRegistry`, but
    // never grows an `Interactable` or a `RoomObject` — so the player
    // can walk past with no dialogue prompt and the engine never sees
    // it.
    let kind = field_string(entity, "kind").unwrap_or_default();
    if kind.trim().is_empty() {
        return Err("Prop requires non-empty `kind` field".to_string());
    }
    Ok(RoomEmission::prop(PropSpec {
        id: entity.iid.clone(),
        name,
        kind: kind.trim().to_string(),
        pos: min + size * 0.5,
        size,
    }))
}

pub(super) fn convert_npc_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
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
        character_id.clone()
    };
    let interactable = ambition_world::rooms::InteractableSpec::new(
        field_string(entity, "prompt").unwrap_or_else(|| "Talk".to_string()),
        ambition_world::rooms::InteractionKindSpec::Npc {
            character_id: (!character_id.is_empty()).then(|| character_id.clone()),
            dialogue_id: field_string(entity, "dialogue_id"),
            // Optional `patrol_radius` field on NpcSpawn. 0 (or unset)
            // -> static NPC unless `path_id` is set.
            patrol_radius: field_f32(entity, "patrol_radius").unwrap_or(0.0),
            patrol_path_id: field_string(entity, "path_id")
                .or_else(|| field_string(entity, "patrol_path_id")),
        },
    );
    let (id, name, aabb) = authored_triple(entity, display_name, min, size);
    let mut record = ambition_world::placements::PlacementRecord::new(
        id,
        PlacementSchema::Interactable(interactable),
        aabb,
    );
    record.name = name;
    Ok(RoomEmission::placement(record))
}

pub(super) fn convert_pickup_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let pickup = ambition_world::rooms::PickupSpec::new(parse_pickup_kind(
        &field_string(entity, "kind").unwrap_or_else(|| "health:1".to_string()),
    ));
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    let mut record =
        ambition_world::placements::PlacementRecord::new(id, PlacementSchema::Pickup(pickup), aabb);
    record.name = name;
    Ok(RoomEmission::placement(record))
}

pub(super) fn convert_ground_item(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
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
    Ok(RoomEmission::ground_item(
        ambition_world::rooms::GroundItemSpec {
            id,
            name,
            held_item: held_item.trim().to_string(),
            pos: min + size * 0.5,
            half_extent: size * 0.5,
        },
    ))
}

#[cfg(feature = "portal_ldtk")]
pub(super) fn convert_portal_gun_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // Field-less marker (id/name optional): the box gives the pickup's center +
    // half-extent. Spawns an already-armed `PortalGunPickup` at room load.
    let id = field_string(entity, "id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entity.iid.clone());
    Ok(RoomEmission::portal_gun_spawn(
        ambition_world::rooms::PortalGunSpawnSpec {
            id,
            name,
            pos: min + size * 0.5,
            half_extent: size * 0.5,
        },
    ))
}

#[cfg(feature = "portal_ldtk")]
pub(super) fn convert_portal(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // `color` names the pair (its partner is the linked exit); `normal` is the
    // surface the portal sits on (up = floor, down = ceiling, left = right-wall,
    // right = left-wall — y is down in world space). The box center is the face.
    let color_str = field_string(entity, "color").unwrap_or_default();
    let color = ambition_world::rooms::PortalChannelColorSpec::from_name(&color_str)
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
    // The face center (`pos = min + size*0.5`) is exactly the placement record's
    // `aabb.center()`, so the Tier-0 mirror stores only `normal` (+ color/link/
    // half_length) and the actor lowering derives `pos` from the record aabb.
    let aabb = object_aabb(min, size);
    let mut record = ambition_world::placements::PlacementRecord::new(
        authored_id(entity),
        PlacementSchema::Portal(ambition_entity_catalog::placements::PortalSchema {
            color,
            normal: [normal.x, normal.y],
            link,
            half_length,
        }),
        aabb,
    );
    record.name = name;
    Ok(RoomEmission::placement(record))
}

// Portal-authored entities require the `portal_ldtk` feature. Per the
// refactor anti-goal ("do NOT make LDtk silently ignore portal-authored
// entities when portal is disabled — fail loudly"), a portal-OFF /
// portal_ldtk-OFF build registers explicit ERROR converters rather than
// dropping the entities.
#[cfg(not(feature = "portal_ldtk"))]
pub(super) fn convert_portal_gun_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    portal_compiled_out(ctx)
}

#[cfg(not(feature = "portal_ldtk"))]
pub(super) fn convert_portal(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    portal_compiled_out(ctx)
}

#[cfg(not(feature = "portal_ldtk"))]
fn portal_compiled_out(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
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

pub(super) fn convert_shrine(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    Ok(RoomEmission::shrine(ambition_world::rooms::ShrineSpec {
        id: authored_id(entity),
        name,
        pos: min + size * 0.5,
        half_extent: size * 0.5,
    }))
}

pub(super) fn convert_gravity_zone(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // `dir` names the gravity direction inside the zone; default up (the demo).
    let dir = match field_string(entity, "dir").as_deref().map(str::trim) {
        Some("down") => ae::Vec2::new(0.0, 1.0),
        Some("left") => ae::Vec2::new(-1.0, 0.0),
        Some("right") => ae::Vec2::new(1.0, 0.0),
        _ => ae::Vec2::new(0.0, -1.0),
    };
    Ok(RoomEmission::gravity_zone(
        ambition_world::rooms::GravityZoneSpec {
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

pub(super) fn convert_chest_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let chest = ambition_world::rooms::ChestSpec::new(
        field_string(entity, "reward").map(|value| parse_pickup_kind(&value)),
    );
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    let mut record =
        ambition_world::placements::PlacementRecord::new(id, PlacementSchema::Chest(chest), aabb);
    record.name = name;
    Ok(RoomEmission::placement(record))
}

pub(super) fn convert_enemy_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let mut brain =
        parse_enemy_brain(&field_string(entity, "brain").unwrap_or_else(|| "Passive".to_string()));
    if let Some(path_id) =
        field_string(entity, "path_id").or_else(|| field_string(entity, "patrol_path_id"))
    {
        if !path_id.trim().is_empty() {
            brain = ambition_entity_catalog::placements::CharacterBrain::Patrol {
                path_id: Some(path_id.trim().to_string()),
            };
        }
    }
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    let mut emission = RoomEmission::enemy_spawn(ambition_world::rooms::Authored::new(
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

pub(super) fn convert_boss_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let brain =
        parse_boss_brain(&field_string(entity, "brain").unwrap_or_else(|| "Dormant".to_string()));
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    let mut emission = RoomEmission::boss_spawn(ambition_world::rooms::Authored::new(
        id.clone(),
        name,
        aabb,
        brain,
    ));
    // ADR 0020 (G4): a BOSS authored as a mount RIDER (GNU-ton the scholar aboard
    // his `giant_gnu` mount) carries a `mounted_on` EntityRef exactly like a rider
    // `EnemySpawn` does — mirror of `convert_enemy_spawn` above. `spawn_boss`
    // attaches the boss's `CanPilot` (fork#2); `resolve_pending_mount_links`
    // installs the `RidingOn`/`MountSlot` link from this authored `(rider, mount)`
    // pair, matching the mount's `FeatureId` (== its `iid`).
    if let Some(mount_id) = field_entity_ref(entity, "mounted_on") {
        emission.mount_links.push((id, mount_id));
    }
    Ok(emission)
}

pub(super) fn convert_debug_label(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let pos = min + size * 0.5;
    let aabb = ae::Aabb::new(pos, ae::Vec2::splat(1.0));
    let label = ambition_world::debug_label::DebugLabel::new(
        field_string(entity, "text").unwrap_or_else(|| entity.identifier.clone()),
        pos,
        parse_debug_label_kind(
            &field_string(entity, "category").unwrap_or_else(|| "Custom".to_string()),
        ),
    );
    Ok(RoomEmission::debug_label(
        ambition_world::rooms::Authored::new(entity.iid.clone(), name, aabb, label),
    ))
}

pub(super) fn convert_water_volume(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
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
    Ok(RoomEmission::water_region(ae::WaterRegion::new(
        object_aabb(min, size),
        ae::WaterKind::Clear,
        spec,
    )))
}

pub(super) fn convert_moving_platform(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
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
    Ok(RoomEmission::moving_platform(
        ambition_world::platforms::MovingPlatformSpec::from_authored(
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

pub(super) fn convert_camera_zone(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    Ok(RoomEmission::camera_zone(CameraZoneSpec {
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

/// Convert an LDtk `Switch` entity into a room-owned interaction spec
/// carrying the wire-format custom payload.
///
/// The `SwitchFeature` spawn path re-parses the payload into a typed
/// the sim-side switch activation parser once, so downstream gameplay
/// systems never touch the string form.
pub(super) fn convert_switch(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let id = field_string(entity, "id").unwrap_or_else(|| entity.iid.clone());
    let action = field_string(entity, "action").unwrap_or_else(|| "ResetEncounter".into());
    let target_encounter = field_string(entity, "target_encounter").unwrap_or_default();
    let aabb = object_aabb(min, size);
    let interactable = ambition_world::rooms::InteractableSpec::new(
        field_string(entity, "prompt").unwrap_or_else(|| "Activate".into()),
        ambition_world::rooms::InteractionKindSpec::Custom(format!(
            "switch:{id}:{action}:{target_encounter}"
        )),
    );
    // Use the LDtk field `id` (carried on activation) for the
    // authored entity id so the SwitchRuntime id matches the
    // SwitchActivation id. The entity.iid would default to something
    // like "Switch-4072"; that mismatch silently no-op'd switch state
    // updates and left the switch sprite stuck red.
    let mut record = ambition_world::placements::PlacementRecord::new(
        id,
        PlacementSchema::Interactable(interactable),
        aabb,
    );
    record.name = name;
    Ok(RoomEmission::placement(record))
}

/// **The winding oracle for `SurfaceRamp` (Q27).**
///
/// `docs/planning/engine/spatial-model.md`: *"Do NOT hand-derive the winding sign
/// per orientation … let the WINDING ORACLE decide correctness — a 4-case
/// parameterized headless test in which a momentum body enters each ramp along
/// the floor at speed and must EXIT moving up the wall … A sign error flips a case
/// from 'climbs the wall' to 'launches off/clips' — the test catches what
/// inspection doesn't."*
///
/// So it is not inspected. It is ridden.
#[cfg(test)]
mod surface_ramp_winding_oracle {
    use super::{surface_ramp_points, RampOrientation};
    use ambition_engine_core as ae;

    const R: f32 = 200.0;
    const SEGMENTS: usize = 8;
    const CORNER: ae::Vec2 = ae::Vec2::new(600.0, 400.0);
    const DT: f32 = 1.0 / 60.0;
    /// Fast enough to be momentum, slow enough that a 200px fillet can supply the
    /// centripetal demand (`v²·angle/r_body` vs `stick_factor · press`). A body
    /// that launches off the ramp is a level-design fact, not a winding bug.
    const SPEED: f32 = 300.0;

    fn corner_for(o: RampOrientation) -> ae::Vec2 {
        let x = match o {
            RampOrientation::FloorToRightWall | RampOrientation::CeilingToRightWall => CORNER.x,
            _ => CORNER.x - 2.0 * R,
        };
        let y = match o {
            RampOrientation::FloorToRightWall | RampOrientation::FloorToLeftWall => CORNER.y,
            _ => CORNER.y - 2.0 * R,
        };
        ae::Vec2::new(x, y)
    }

    /// A fillet plus the straight flat that leads into it and the straight wall it
    /// leaves along — one chain, in the order `surface_ramp_points` chose.
    fn ramp_chain(o: RampOrientation) -> ae::SurfaceChain {
        let corner = corner_for(o);
        let arc = surface_ramp_points(corner, R, o, SEGMENTS);
        let room = o.into_room();

        // Long lead-ins on purpose: the joint at the fillet's mouth then sits at an
        // arc length where a fixed-epsilon joint nudge used to round away (see
        // `ambition_engine_core::surface::joint_nudge`). The oracle should ride a
        // realistic chain, not a convenient one.
        let flat_far = ae::Vec2::new(corner.x + room.x * 8.0 * R, corner.y);
        let wall_far = ae::Vec2::new(corner.x, corner.y + room.y * 8.0 * R);

        let first_is_flat = (arc[0].y - corner.y).abs() < 1.0;
        let mut points = Vec::with_capacity(arc.len() + 2);
        if first_is_flat {
            points.push(flat_far);
            points.extend(arc.iter().copied());
            points.push(wall_far);
        } else {
            points.push(wall_far);
            points.extend(arc.iter().copied());
            points.push(flat_far);
        }
        ae::SurfaceChain::open(format!("{o:?}"), points)
    }

    /// A ramp's local "down" is the opposite of the flat surface's outward normal:
    /// `+y` under a floor, `−y` above a ceiling. This is the C4 gravity conjugation
    /// the doc asks the mirrored cases to be ridden under.
    fn gravity_for(o: RampOrientation) -> ae::Vec2 {
        ae::Vec2::new(0.0, -o.into_room().y) * 1450.0
    }

    /// Ride the chain from the flat end into the corner; report the velocity the
    /// moment the body clears the fillet onto the wall.
    fn ride_into_the_corner(o: RampOrientation) -> ae::Vec2 {
        let chain = ramp_chain(o);
        let corner = corner_for(o);
        let room = o.into_room();
        let world = ae::World::new(
            "ramp",
            ae::Vec2::new(6000.0, 6000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        )
        .with_chains(vec![chain.clone()]);
        // Frictionless: this test is about winding, not about whether the body has
        // enough speed left after 1600px of floor.
        let params = ae::surface::MomentumParams {
            friction: 0.0,
            ..Default::default()
        };
        let total = chain.total_length();

        // Start near the FLAT end, moving toward the arc. Which end of the chain
        // that is depends on the winding the converter derived, so ask the chain.
        let flat_at_zero = (chain.points[0].y - corner.y).abs() < 1.0;
        let (s, v_t) = if flat_at_zero {
            (30.0, SPEED)
        } else {
            (total - 30.0, -SPEED)
        };

        let f = chain.frame_at(s);
        let radius = 14.0;
        let mut body = ae::surface::SurfaceBody {
            pos: f.point + f.normal * radius,
            vel: ae::Vec2::ZERO,
            radius,
            motion: ae::surface::SurfaceMotion::Riding {
                on: ae::surface::SurfaceRef::Chain(0),
                s,
                v_t,
            },
        };
        let gravity = gravity_for(o);
        // `run` is along the CHAIN's tangent (increasing arc length), not along
        // world `+x`. Which world direction that is depends on the winding the
        // converter derived — so hold the stick in the direction we are already
        // travelling, and let the chain say what that means. Passing `-room.x` here
        // braked the ceiling cases into a stop, which is a fact about `run`, not
        // about the ramp.
        let run = v_t.signum();
        // Sample the moment the body clears the fillet onto the wall — NOT seconds
        // later. A body that climbed the wall correctly decelerates under gravity
        // and comes back down, and "it is falling" is not a winding bug.
        for _ in 0..2000 {
            ae::surface::step_surface_body(
                &mut body,
                &world,
                &params,
                gravity,
                ae::surface::SurfaceInputs {
                    run,
                    jump_pressed: false,
                },
                DT,
                None,
            );
            // Past the fillet's far tangent point, measured along the wall axis.
            if (body.pos.y - corner.y) * room.y > R * 1.25 {
                break;
            }
        }
        body.vel
    }

    /// **The oracle.** A body that runs into the fillet at speed leaves along the
    /// WALL, in the direction the room opens — up, for a floor; down, for a
    /// ceiling. A winding sign error turns this into a launch or a clip, and no
    /// amount of reading the arc table would show it.
    #[test]
    fn a_momentum_body_carries_its_speed_from_the_flat_onto_the_wall() {
        for o in [
            RampOrientation::FloorToRightWall,
            RampOrientation::FloorToLeftWall,
            RampOrientation::CeilingToRightWall,
            RampOrientation::CeilingToLeftWall,
        ] {
            let vel = ride_into_the_corner(o);
            let expected_sign = o.into_room().y; // floors exit up (−y), ceilings down (+y)
            assert!(
                vel.y * expected_sign > 0.0,
                "{o:?}: exited with vel {vel:?}; expected the wall-axis component to \
                 run toward {expected_sign:+}"
            );
            assert!(
                vel.y.abs() > vel.x.abs(),
                "{o:?}: exited with vel {vel:?} — that is still along the flat, not up \
                 the wall"
            );
        }
    }

    /// The arc itself: endpoints tangent to the two surfaces, `segments + 1` points,
    /// every point on the circle of radius `r` about the fillet's center. The doc's
    /// table, checked.
    #[test]
    fn the_arc_is_the_documented_quarter_circle() {
        for o in [
            RampOrientation::FloorToRightWall,
            RampOrientation::FloorToLeftWall,
            RampOrientation::CeilingToRightWall,
            RampOrientation::CeilingToLeftWall,
        ] {
            let corner = ae::Vec2::new(600.0, 400.0);
            let pts = surface_ramp_points(corner, R, o, SEGMENTS);
            assert_eq!(pts.len(), SEGMENTS + 1, "{o:?}");

            let center = corner + o.into_room() * R;
            for p in &pts {
                assert!(
                    ((*p - center).length() - R).abs() < 0.01,
                    "{o:?}: {p:?} is not on the fillet's circle"
                );
            }
            // One endpoint is tangent on the flat surface, the other on the wall.
            let on_flat = pts.iter().filter(|p| (p.y - corner.y).abs() < 0.01).count();
            let on_wall = pts.iter().filter(|p| (p.x - corner.x).abs() < 0.01).count();
            assert_eq!((on_flat, on_wall), (1, 1), "{o:?}");
        }
    }

    /// The winding is DERIVED, not tabulated: every orientation's first segment has
    /// its outward normal pointing into the room. That is the property the
    /// converter enforces, and the reason there is one code path for four cases.
    #[test]
    fn every_orientation_winds_so_its_normals_point_into_the_room() {
        for o in [
            RampOrientation::FloorToRightWall,
            RampOrientation::FloorToLeftWall,
            RampOrientation::CeilingToRightWall,
            RampOrientation::CeilingToLeftWall,
        ] {
            let corner = ae::Vec2::new(600.0, 400.0);
            let pts = surface_ramp_points(corner, R, o, SEGMENTS);
            let center = corner + o.into_room() * R;
            for w in pts.windows(2) {
                let t = (w[1] - w[0]).normalize();
                let normal = ae::Vec2::new(t.y, -t.x);
                let midpoint = (w[0] + w[1]) * 0.5;
                assert!(
                    normal.dot(center - midpoint) > 0.0,
                    "{o:?}: a segment's normal points into the solid"
                );
            }
        }
    }
}
