//! The engine's standard LDtk entity converters, registered in
//! `standard_converters()`. One `convert_*` per LDtk entity type, all with the
//! [`LdtkEntityConverter`] signature, so engine and content converters are the
//! same to the registry. Helpers and `RoomEmission` are in the parent
//! (`super::*`).

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

/// `StitchedBoundary` is read by its own consumer off the raw `LdtkProject` and
/// never joins the emission stream.
pub(super) fn convert_consumed_elsewhere(_ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    Ok(RoomEmission::ignored())
}

/// An encounter's trigger volume, into the room IR. The encounter loader reads
/// rooms, not the `LdtkProject`, so it does not depend on the LDtk crate.
pub(super) fn convert_encounter_trigger(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, _name, min, size) = ctx.parts();
    Ok(RoomEmission {
        encounter_triggers: vec![ambition_platformer2d_world::rooms::EncounterTriggerSpec {
            // Empty is meaningful: the loader falls back to the area id, which the IR
            // does not have.
            id: field_string(entity, "id").unwrap_or_default(),
            min,
            size,
            camera_zoom: field_f32(entity, "camera_zoom"),
        }],
        ..RoomEmission::default()
    })
}

/// An encounter's lock wall, into the room IR. See
/// [`convert_encounter_trigger`].
pub(super) fn convert_lock_wall(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, _name, min, size) = ctx.parts();
    Ok(RoomEmission {
        lock_walls: vec![ambition_platformer2d_world::rooms::EncounterLockWallSpec {
            id: field_string(entity, "id").unwrap_or_default(),
            // Empty and absent both mean "not a gate": `authored_gated_lock_walls`
            // skips a wall without a condition because it belongs to an encounter.
            gated_by: field_string(entity, "gated_by")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            min,
            size,
        }],
        ..RoomEmission::default()
    })
}

pub(super) fn convert_loading_zone(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // An unknown spelling is refused, not defaulted, so a typo does not give a
    // door where the author meant a walk-through. See
    // `LoadingZoneActivation::from_authored`.
    let authored = field_string(entity, "activation").unwrap_or_else(|| "Door".to_string());
    let activation = LoadingZoneActivation::from_authored(&authored).ok_or_else(|| {
        format!(
            "LoadingZone '{name}' has activation '{authored}', which is not one of {:?}",
            LoadingZoneActivation::AUTHORED_SPELLINGS
        )
    })?;
    Ok(RoomEmission::zone(LoadingZone {
        id: field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
        name,
        activation,
        aabb: object_aabb(min, size),
    }))
}

pub(super) fn convert_damage_volume(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let offset = ctx.offset;
    let aabb = object_aabb(min, size);
    let mut volume = ambition_platformer2d_world::rooms::HazardVolumeSpec::new(
        field_i32(entity, "damage").unwrap_or(1),
    );
    volume.path_id = field_string(entity, "path_id").and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });
    let inline_motion = parse_optional_path(entity).map(|mut path| {
        path.points = offset_points(path.points, offset);
        path
    });
    // Hazards go through the `placements` channel. `HazardSpec` carries only
    // `path_id`, so an inline motion path is lifted to a room-level
    // `KinematicPath` keyed `{iid}__inline_motion` and referenced by `path_id`.
    // `HazardRuntime::new_with_paths` resolves it and sets `volume.motion`, the
    // same result as an inline path.
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
    let mut record = ambition_platformer2d_world::placements::PlacementRecord::new(
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

/// `SurfaceChain`: a rideable surface polyline (momentum locomotion). Fields:
/// `points` (semicolon `x,y` pairs, level-local, as for KinematicPath) and
/// optional `closed: bool` (a loop; the closing segment is implicit). One-sided
/// by winding: author floors left→right. The engine validator runs at
/// conversion, so inverted joins, degenerate segments, and self-intersections
/// fail here and not as physics bugs in play.
pub(super) fn convert_surface_chain(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, _min, _size) = ctx.parts();
    let points = offset_points(
        parse_points(&field_string(entity, "points").unwrap_or_default()),
        ctx.offset,
    );
    let closed = field_bool(entity, "closed").unwrap_or(false);
    let mut chain = if closed {
        ae::SurfaceChain::closed_loop(name, points)
    } else {
        ae::SurfaceChain::open(name, points)
    };
    // `fill`: the chain is the top of the ground, drawn filled beneath.
    chain.filled = field_bool(entity, "fill").unwrap_or(false);
    let problems = chain.validate();
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    Ok(RoomEmission::chain(chain))
}

/// `SurfaceLoop`: a rideable full loop. **The entity's box is the loop's
/// circle**: its centre is the loop's centre and half its side is the radius,
/// so an author draws a bigger box for a bigger loop and sees in the editor
/// where it will be. Optional `segments` (polygon resolution, default 24, min
/// 3). The converter generates the closed polygon at conversion and emits it
/// into the `chains` channel that `SurfaceChain` uses.
///
/// ⛔ There is no `radius` field. It was the loop's size while the box was a
/// 16 px marker, so the editor showed a dot where a 400 px loop would be; two
/// sizes for one loop is one too many.
pub(super) fn convert_surface_loop(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    if (size.x - size.y).abs() > 1.0 {
        return Err(format!(
            "SurfaceLoop's box is its circle and must be square, not {}x{}",
            size.x, size.y
        ));
    }
    let radius = size.x.min(size.y) * 0.5;
    if radius <= 0.0 {
        return Err("SurfaceLoop's box is its circle and must have a size".to_string());
    }
    let center = min + size * 0.5;
    // ATTACHED: a runnable loop joined to a floor chain (ramp → revolution →
    // crossover deck → runout), built where the box is. The ramp climbs from
    // the floor to the circle's bottom. The spans default to the proportions
    // of the Sanic speedway's loop (radius 180: approach 460, deck 280,
    // runout 720).
    let attach_to = field_string(entity, "attach_to").unwrap_or_default();
    if !attach_to.is_empty() {
        let span = |field: &str, default: f32| -> Result<f32, String> {
            let value = field_f32(entity, field).unwrap_or(default);
            if value < 0.0 {
                return Err(format!("SurfaceLoop `{field}` must not be negative"));
            }
            Ok(value)
        };
        let center_x = center.x + ctx.offset.x;
        let approach = span("approach", radius * 460.0 / 180.0)?;
        let deck = span("deck", radius * 280.0 / 180.0)?;
        let runout = span("runout", radius * 720.0 / 180.0)?;
        return Ok(RoomEmission {
            loop_attachments: vec![super::LoopAttachment {
                name: name.to_string(),
                floor: attach_to,
                center: ae::Vec2::new(center_x, center.y + ctx.offset.y),
                ramp_start_x: center_x - approach,
                radius,
                overpass_end_x: center_x + deck,
                runout_end_x: center_x + runout,
            }],
            ..Default::default()
        });
    }
    let segments = field_i32(entity, "segments").unwrap_or(24).max(3) as usize;
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

    /// Unit vector from the corner toward the arc center (into the open room).
    /// Each component is `±1`, so the base arc table mirrors by sign.
    fn into_room(self) -> ae::Vec2 {
        match self {
            Self::FloorToRightWall => ae::Vec2::new(-1.0, -1.0),
            Self::FloorToLeftWall => ae::Vec2::new(1.0, -1.0),
            Self::CeilingToRightWall => ae::Vec2::new(-1.0, 1.0),
            Self::CeilingToLeftWall => ae::Vec2::new(1.0, 1.0),
        }
    }
}

/// `SurfaceRamp`: the quarter-circle fillet that lets a momentum body carry its
/// speed from a floor onto a wall (math in `docs/architecture/spatial-model.md`
/// §`SurfaceRamp`).
///
/// Fields: `radius` (px, required), `orientation` (one of the four
/// [`RampOrientation`] names, default `FloorToRightWall`), `segments` (polygon
/// resolution, default 8, min 2).
///
/// AMBITION_REVIEW(spatial): the arc table is the doc's table for the base
/// orientation. Do not hand-derive the winding per orientation: mirroring the
/// point list flips the sign of `normal = (t.y, -t.x)`. The converter derives
/// the order instead. A fillet's normal points from the arc toward its center,
/// so the list is reversed when the first segment's normal points away. The
/// winding oracle (`surface_ramp_winding_oracle`) rides each orientation.
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
/// The other three orientations are axis mirrors of this table, applied by the
/// `±1` components of `into_room()`.
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
            // Base table, mirrored by `sign`. `sin` runs along the wall axis and `cos`
            // along the floor axis, as in the doc.
            center + ae::Vec2::new(-sign.x * theta.sin(), -sign.y * theta.cos()) * radius
        })
        .collect();

    // Derive the winding. `SurfaceChain`'s outward normal is `(t.y, -t.x)`; on a
    // fillet it must point from the surface toward the arc center. If the first
    // segment's normal does not, reverse the list.
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
    let path = ambition_platformer2d_core::KinematicPath {
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
        kinematic_path_lookup_id(entity, &name),
        name,
        object_aabb(min, size),
        path,
    )))
}

pub(super) fn convert_prop(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // Decorative-only entity. It renders a sprite through `PropRegistry` but gets
    // no `Interactable` or `RoomObject`, so it has no prompt and the engine does
    // not see it.
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
        // LDtk props are plain decoration: unmirrored, character-sized,
        // behind the cast.
        flip_y: false,
        draw: Default::default(),
    }))
}

pub(super) fn convert_npc_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // An NpcSpawn carries a `character_id` that keys into
    // `assets/data/character_catalog.ron`. This crate has no catalog dependency,
    // so `Authored.name` holds the character_id (the entity name is always
    // "NpcSpawn"). `spawn_static::lower_interactable_placement` resolves the
    // display name from the catalog at spawn, using the character_id in
    // `InteractionKindSpec::Npc`.
    let character_id = field_string(entity, "character_id").unwrap_or_default();
    let display_name = if character_id.is_empty() {
        name
    } else {
        character_id.clone()
    };
    let interactable = ambition_platformer2d_world::rooms::InteractableSpec::new(
        field_string(entity, "prompt").unwrap_or_else(|| "Talk".to_string()),
        ambition_platformer2d_world::rooms::InteractionKindSpec::Npc {
            character_id: (!character_id.is_empty()).then(|| character_id.clone()),
            dialogue_id: field_string(entity, "dialogue_id"),
            // Optional `patrol_radius`: a lane-radius parameter for a patrol brain
            // preset. It does not select the brain.
            patrol_radius: field_f32(entity, "patrol_radius").unwrap_or(0.0),
            patrol_path_id: field_string(entity, "path_id"),
            // Optional initial brain preset override. Absent or empty uses the
            // character's catalog `default_brain`.
            brain_override: field_string(entity, "brain_override"),
        },
    );
    let (id, name, aabb) = authored_triple(entity, display_name, min, size);
    let mut record = ambition_platformer2d_world::placements::PlacementRecord::new(
        id,
        PlacementSchema::Interactable(interactable),
        aabb,
    );
    record.name = name;
    Ok(RoomEmission::placement(record))
}

pub(super) fn convert_pickup_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let mut pickup = ambition_platformer2d_world::rooms::PickupSpec::new(parse_pickup_kind(
        &field_string(entity, "kind").unwrap_or_else(|| "health:1".to_string()),
    ));
    // Optional animated sprite sheet (a `GameAssets` prop kind), for example a
    // spinning ring. The reward stays on `kind`.
    if let Some(sprite) = field_string(entity, "sprite").filter(|s| !s.trim().is_empty()) {
        pickup.sprite = Some(sprite.trim().to_string());
    }
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    let mut record = ambition_platformer2d_world::placements::PlacementRecord::new(
        id,
        PlacementSchema::Pickup(pickup),
        aabb,
    );
    record.name = name;
    Ok(RoomEmission::placement(record))
}

pub(super) fn convert_ground_item(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // Authored held-item pickup. `held_item` is a held-item registry id (`meteor`,
    // `bomb`, `puppy_slug_gun`, `gun_sword`, ...). It resolves to a `HeldItemSpec`
    // at spawn. An unregistered or feature-gated id makes the item not appear; the
    // room load does not fail.
    let held_item = field_string(entity, "held_item").unwrap_or_default();
    if held_item.trim().is_empty() {
        return Err("GroundItem requires non-empty `held_item` field".to_string());
    }
    let id = field_string(entity, "id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entity.iid.clone());
    Ok(RoomEmission::ground_item(
        ambition_platformer2d_world::rooms::GroundItemSpec {
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
    // The box gives the pickup's center and half-extent. `id`, `name`, and `pair`
    // are optional. Spawns an armed `PortalGunPickup` at room load.
    let id = field_string(entity, "id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entity.iid.clone());
    // The portal pair this gun owns. Absent means 0, the classic blue/orange gun.
    // For a second gun of a different color, place another `PortalGunSpawn` with
    // a different `pair`.
    //
    // An out-of-range value wraps and is not refused: a wrong number gives a gun
    // of another color, not a room that fails to load. This code only fits the
    // value into a `u8`. `PortalGunColor` wraps the pair space, and this layer
    // does not depend on the portal crate (see the portal-api FIXME about the gun
    // palette).
    let pair = field_i32(entity, "pair").unwrap_or(0).rem_euclid(256) as u8;
    Ok(RoomEmission::portal_gun_spawn(
        ambition_platformer2d_world::rooms::PortalGunSpawnSpec {
            id,
            name,
            pos: min + size * 0.5,
            half_extent: size * 0.5,
            pair,
        },
    ))
}

#[cfg(feature = "portal_ldtk")]
pub(super) fn convert_portal(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // `color` names the pair (its partner is the linked exit). `normal` is the
    // surface the portal sits on (up = floor, down = ceiling, left = right wall,
    // right = left wall; y is down). The box center is the face.
    let color_str = field_string(entity, "color").unwrap_or_default();
    let color = ambition_platformer2d_world::rooms::PortalChannelColorSpec::from_name(&color_str)
        .ok_or_else(|| format!("Portal '{name}' has unknown color '{color_str}'"))?;
    let normal = match field_string(entity, "normal").as_deref().map(str::trim) {
        Some("down") => ae::Vec2::new(0.0, 1.0),
        Some("left") => ae::Vec2::new(-1.0, 0.0),
        Some("right") => ae::Vec2::new(1.0, 0.0),
        Some("up") | None => ae::Vec2::new(0.0, -1.0),
        Some(other) => return Err(format!("Portal '{name}' has unknown normal '{other}'")),
    };
    // Explicit link id (preferred pairing). Empty or absent uses color pairing.
    let link = field_string(entity, "link")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    // Authored opening size: the box dimension along the surface. Floor or
    // ceiling (vertical normal) → width; wall → height.
    let along = if normal.x.abs() > 0.5 { size.y } else { size.x };
    let half_length = (along > 1.0).then_some(along * 0.5);
    // The face center (`min + size*0.5`) is the placement record's
    // `aabb.center()`, so the Tier-0 mirror stores only `normal`, color, link, and
    // half_length. The actor lowering derives `pos` from the record aabb.
    let aabb = object_aabb(min, size);
    let mut record = ambition_platformer2d_world::placements::PlacementRecord::new(
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

// Portal entities need the `portal_ldtk` feature. Without it, register
// converters that fail with an error, so LDtk does not drop portal entities
// without notice.
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
    Ok(RoomEmission::shrine(
        ambition_platformer2d_world::rooms::ShrineSpec {
            id: authored_id(entity),
            name,
            pos: min + size * 0.5,
            half_extent: size * 0.5,
        },
    ))
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
        ambition_platformer2d_world::rooms::GravityZoneSpec {
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
    let chest = ambition_platformer2d_world::rooms::ChestSpec::new(
        field_string(entity, "reward").map(|value| parse_pickup_kind(&value)),
    );
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    let mut record = ambition_platformer2d_world::placements::PlacementRecord::new(
        id,
        PlacementSchema::Chest(chest),
        aabb,
    );
    record.name = name;
    Ok(RoomEmission::placement(record))
}

pub(super) fn convert_enemy_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let authored_brain = field_string(entity, "brain").unwrap_or_default();
    let authored_brain = authored_brain.trim();
    // The patrol path is a native reference. `path_ref` is an LDtk `EntityRef` to
    // a `KinematicPath`. The editor draws the link, tools find it from the schema,
    // and LDtk's referential integrity catches a missing target.
    //
    // The old `Patrol:<id>` string is refused. If it fell through to
    // `CharacterBrain::Custom`, an unmigrated placement would look valid. Delete
    // this arm when no `.ldtk` in the repo uses the prefix:
    //   grep -l '"Patrol:' game/*/assets/worlds/*.ldtk
    if authored_brain.starts_with("Patrol:") {
        return Err(format!(
            "EnemySpawn `{name}` authors brain `{authored_brain}`. The `Patrol:` \
             prefix is retired: author the patrol path as a `path_ref` EntityRef \
             pointing at the KinematicPath, and clear `brain`."
        ));
    }
    let path_ref = ctx
        .kinematic_path_ref("path_ref")
        .map_err(|problem| format!("EnemySpawn `{name}` {problem}"))?;
    // One authority per placement. A `path_ref` together with `Guard:96` gives two
    // answers to "what drives this body"; refuse instead of picking one.
    if path_ref.is_some() && !authored_brain.is_empty() {
        return Err(format!(
            "EnemySpawn `{name}` authors both `path_ref` and brain \
             `{authored_brain}`. A placement states what drives it once: a \
             `path_ref` IS the patrol brain, so clear `brain`."
        ));
    }
    let brain = match path_ref {
        Some(path_id) => ambition_entity_catalog::placements::CharacterBrain::Patrol {
            path_id: Some(path_id),
        },
        None if authored_brain.is_empty() => {
            ambition_entity_catalog::placements::CharacterBrain::Passive
        }
        None => parse_enemy_brain(authored_brain),
    };
    let (id, name, aabb) = authored_triple(entity, name, min, size);
    let character_id = field_string(entity, "character_id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "EnemySpawn `{name}` authors no `character_id`. A placement states                  which creature it places; the display name is a label, and using                  it was a coincidence the engine acted on."
            )
        })?;
    let mut payload = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(brain, character_id);
    // Initial facing is placement context: two instances of the same character
    // can enter a room facing opposite ways. Missing or blank keeps the default
    // (`Right` / +1). A misspelling is refused.
    if let Some(facing) = field_string(entity, "facing") {
        match facing.trim() {
            "" | "Right" => {}
            "Left" => payload.facing = ambition_platformer2d_world::rooms::SpawnFacing::Left,
            other => {
                return Err(format!(
                    "EnemySpawn `{name}` authors facing `{other}`, which is not one of Left / Right"
                ))
            }
        }
    }
    // Respawn policy for this placement, when it has one (ADR 0022). A migrated
    // character has no archetype row to inherit from, and the same creature can
    // be permanent in a story room and repopulating in a corridor.
    if let Some(respawn) = field_string(entity, "respawn") {
        match parse_respawn_policy(respawn.trim()) {
            Some(policy) => payload.respawn = Some(policy),
            // Refuse a misspelled policy. Falling back to the archetype's policy would
            // look the same as authoring nothing.
            None if !respawn.trim().is_empty() => {
                return Err(format!(
                    "EnemySpawn `{name}` authors respawn `{}`, which is not one of \
                     DeadStaysDead / OnRoomReenter / OnRest / InPlace(seconds)",
                    respawn.trim()
                ))
            }
            None => {}
        }
    }
    // Hostility is authored here, not inherited from a creature.
    if let Some(disposition) = field_string(entity, "disposition") {
        match disposition.trim() {
            "" => {}
            "Hostile" => {
                payload.disposition =
                    Some(ambition_entity_catalog::placements::SpawnDisposition::Hostile)
            }
            "Peaceful" => {
                payload.disposition =
                    Some(ambition_entity_catalog::placements::SpawnDisposition::Peaceful)
            }
            other => {
                return Err(format!(
                    "EnemySpawn `{name}` authors disposition `{other}`, which is \
                     not one of Hostile / Peaceful"
                ))
            }
        }
    }
    // Brain override, when this placement wants a policy other than the
    // character's own. See `EnemySpawnSpec::brain_profile`: the same body can be a
    // patroller in one corridor and a door guard in the next.
    //
    // Provider-relative, like other authored profile references: a level author
    // writes `medium_striker`, not `ambition::medium_striker`.
    if let Some(profile) = field_string(entity, "brain_profile") {
        let profile = profile.trim();
        if !profile.is_empty() {
            payload.brain_profile = Some(ambition_entity_catalog::BrainProfileRef::new(profile));
        }
    }
    let mut emission = RoomEmission::enemy_spawn(
        ambition_platformer2d_world::rooms::Authored::new(id.clone(), name, aabb, payload),
    );
    // ADR 0020: a rider EnemySpawn with a `mounted_on` entity-ref emits a mount
    // link `(rider_id, mount_id)`. The ref stores the mount's LDtk `iid`. Linked
    // pairs have no explicit `id`, so the mount's `FeatureId` equals its `iid`.
    if let Some(mount_id) = field_entity_ref(entity, "mounted_on") {
        emission.mount_links.push((id, mount_id));
    }
    Ok(emission)
}

/// Parse an authored respawn policy. The vocabulary is the engine's
/// [`RespawnPolicy`](ambition_entity_catalog::placements::RespawnPolicy), spelled
/// as a level author would write it in an LDtk string field.
fn parse_respawn_policy(text: &str) -> Option<ambition_entity_catalog::placements::RespawnPolicy> {
    use ambition_entity_catalog::placements::RespawnPolicy;
    match text {
        "" => None,
        "DeadStaysDead" => Some(RespawnPolicy::DeadStaysDead),
        "OnRoomReenter" => Some(RespawnPolicy::OnRoomReenter),
        "OnRest" => Some(RespawnPolicy::OnRest),
        other => other
            .strip_prefix("InPlace(")
            .and_then(|rest| rest.strip_suffix(')'))
            .and_then(|seconds| seconds.trim().parse::<f32>().ok())
            .map(RespawnPolicy::InPlace),
    }
}

pub(super) fn convert_boss_spawn(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let brain =
        parse_boss_brain(&field_string(entity, "brain").unwrap_or_else(|| "Dormant".to_string()));
    let (_iid, name, aabb) = authored_triple(entity, name, min, size);
    // The authored encounter id is the placement's id, not a separate field.
    // Boss progress is keyed only by stable authored encounter/placement ids
    // (maintainer ruling, question 57), and `boss.cleared(id)` asks about this
    // encounter. The durable record is written under this id
    // (`boss_encounter/src/systems.rs`), and a Yarn author can only type a name
    // they know, so the id must be the authored one, not the LDtk iid.
    //
    // Resolve it here so there is one authority: the save key, `FeatureId`, the
    // duplicate-id check, and mount links all read this `id`. An unauthored
    // placement falls back to the iid.
    //
    // Do not derive the id from the level identifier: a save key would then
    // change when the level is renamed.
    let id = crate::fields::boss_placement_id(entity);
    let mut emission = RoomEmission::boss_spawn(ambition_platformer2d_world::rooms::Authored::new(
        id.clone(),
        name,
        aabb,
        brain,
    ));
    // ADR 0020 (G4): a boss authored as a mount rider (GNU-ton on his
    // `giant_gnu`) carries a `mounted_on` EntityRef, as a rider `EnemySpawn` does
    // (see `convert_enemy_spawn`). `spawn_boss` attaches the boss's `CanPilot`.
    // The planned `ambition.mount` relation installs the `RidingOn`/`MountSlot`
    // link from this `(rider, mount)` pair, matching the mount's `FeatureId`
    // (== its `iid`).
    if let Some(mount_id) = field_entity_ref(entity, "mounted_on") {
        emission.mount_links.push((id, mount_id));
    }
    Ok(emission)
}

pub(super) fn convert_debug_label(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let pos = min + size * 0.5;
    let aabb = ae::Aabb::new(pos, ae::Vec2::splat(1.0));
    let label = ambition_platformer2d_world::debug_label::DebugLabel::new(
        field_string(entity, "text").unwrap_or_else(|| entity.identifier.clone()),
        pos,
        parse_debug_label_kind(
            &field_string(entity, "category").unwrap_or_else(|| "Custom".to_string()),
        ),
    );
    Ok(RoomEmission::debug_label(
        ambition_platformer2d_world::rooms::Authored::new(entity.iid.clone(), name, aabb, label),
    ))
}

pub(super) fn convert_water_volume(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, _name, min, size) = ctx.parts();
    // Entity water goes into the same `World::water_regions` list as IntGrid
    // Water cells. Use it for irregular pools that the per-cell layer cannot shape.
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
    // Entity water defaults to Clear. The IntGrid Water layer is the canonical
    // path for other kinds. To add Murky here, add a `kind` field with
    // `register_ldtk_entity_def.py` and route it here.
    Ok(RoomEmission::water_region(ae::WaterRegion::new(
        object_aabb(min, size),
        ae::WaterKind::Clear,
        spec,
    )))
}

pub(super) fn convert_moving_platform(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    // LDtk entity bounds give the platform size and, for sweep mode, the start
    // AABB. When `path_id` is authored, the platform follows that
    // active-area-local `KinematicPathSpec` and starts at its first point.
    let start_pos = min + size * 0.5;
    // Give the engine only what the author wrote (`None` for an unset field). The
    // engine selects the motion and owns the defaults.
    //
    // `sweep_dx` and `loop_dy` are deltas, so they take no level offset.
    // `loop_min_y` is a position, so it takes the offset (see
    // `LdtkEntityCtx::offset`). Without it the anchor is wrong by the level's
    // offset, which is not visible in a level at the origin.
    let motion = ambition_platformer2d_world::platforms::AuthoredPlatformMotion {
        sweep_dx: field_f32(entity, "sweep_dx"),
        speed: field_f32(entity, "speed"),
        path_id: field_string(entity, "path_id"),
        loop_dy: field_f32(entity, "loop_dy"),
        loop_anchor_y: field_f32(entity, "loop_min_y").map(|y| y + ctx.offset.y),
    }
    .classify()?;
    Ok(RoomEmission::moving_platform(
        ambition_platformer2d_world::platforms::MovingPlatformSpec::new(
            // Read the authored `id` first and fall back to the iid, as `LoadingZone`,
            // `CameraZone`, `Portal`, and `ShrineSpawn` do. A room needs a stable id to
            // address a platform; the name is presentation only (`FeatureName`) and must
            // not key gameplay.
            field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
            name,
            start_pos,
            size,
            motion,
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
        scroll_policy: ambition_platformer2d_world::rooms::CameraScrollPolicy::from_author_value(
            field_string(entity, "scroll_policy").as_deref(),
        ),
        clamp_mode: CameraClampMode::from_author_value(
            field_string(entity, "clamp_mode").as_deref(),
        ),
    }))
}

/// Convert an LDtk `Switch` entity into a room-owned interaction spec with the
/// wire-format custom payload. The `SwitchFeature` spawn path parses the
/// payload once, so gameplay systems do not see the string form.
pub(super) fn convert_switch(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let id = field_string(entity, "id").unwrap_or_else(|| entity.iid.clone());
    let action = field_string(entity, "action").unwrap_or_else(|| "ResetEncounter".into());
    let target_encounter = field_string(entity, "target_encounter").unwrap_or_default();
    let aabb = object_aabb(min, size);
    let interactable = ambition_platformer2d_world::rooms::InteractableSpec::new(
        field_string(entity, "prompt").unwrap_or_else(|| "Activate".into()),
        ambition_platformer2d_world::rooms::InteractionKindSpec::Custom(format!(
            "switch:{id}:{action}:{target_encounter}"
        )),
    );
    // Use the LDtk field `id` as the authored entity id, so the SwitchRuntime id
    // matches the SwitchActivation id. The iid (for example "Switch-4072") would
    // not match, and switch state updates would do nothing. Keep it before `id`
    // moves into the record: `SwitchActivation` joins on it, and command lines
    // address it.
    let id_for_command = id.clone();
    let mut record = ambition_platformer2d_world::placements::PlacementRecord::new(
        id,
        PlacementSchema::Interactable(interactable),
        aabb,
    );
    record.name = name;
    // Not part of `InteractableSpec`: that type is a variant of the closed Tier-0
    // `PlacementSchema`, and only one consumer reads this string.
    let switch_commands = field_string(entity, "on_activate")
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .map(
            |line| ambition_platformer2d_world::rooms::SwitchCommandSpec {
                switch_id: id_for_command,
                line,
            },
        )
        .into_iter()
        .collect();
    Ok(RoomEmission {
        switch_commands,
        ..RoomEmission::placement(record)
    })
}

/// The winding oracle for `SurfaceRamp`.
///
/// `docs/architecture/spatial-model.md` says not to hand-derive the winding per
/// orientation. Instead, a momentum body enters each of the four ramps along
/// the floor at speed and must exit moving up the wall. A sign error makes it
/// launch off or clip.
#[cfg(test)]
mod surface_ramp_winding_oracle;
