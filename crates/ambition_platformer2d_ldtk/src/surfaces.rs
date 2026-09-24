//! Typed `Surface` authoring primitive: parse, then compile to engine collision.
//!
//! `parse_surface_spec` reads a Surface-shaped entity's fields into
//! `LdtkSurfaceSpec` and the `Surface*` enums. `compile_surface` lowers a spec
//! to a `SurfaceCompiled` (typed `Block`/`Breakable`/contact data). Used by
//! sibling `conversion`.

use ambition_platformer2d_core as ae;

use super::fields::{field_f32, field_i32, field_string};
use super::project::LdtkEntityInstance;

/// Collision behavior of an LDtk-authored `Surface`.
///
/// A designer places one rectangular entity and sets its `collision`,
/// `breakability`, `contact`, and `respawn` fields. The compile step turns this
/// into typed engine `Block`/`Breakable`/contact data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SurfaceCollision {
    /// Pure trigger volume; bodies pass through.
    #[default]
    None,
    /// Hard wall on both axes (legacy `Solid`).
    Solid,
    /// One-way landing: solid only when crossed from above (legacy `OneWayPlatform`).
    OneWayUp,
    /// Soft blink wall: solid until the player has the matching blink upgrade.
    BlinkSoft,
    /// Hard blink wall: solid until the player has the stronger blink upgrade.
    BlinkHard,
}

/// Whether and how a `Surface` can be destroyed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SurfaceBreakability {
    #[default]
    Indestructible,
    BreakOnHit,
    BreakOnStand,
    BreakOnHitOrStand,
}

/// Side-effect applied to bodies that touch a `Surface`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SurfaceContact {
    #[default]
    None,
    /// Return the toucher to spawn: the pit floor (`HazardBlock`). It does not
    /// damage.
    ///
    /// A surface that hurts is a `DamageVolume`. It lowers to a hazard placement,
    /// ticks in `ambition_platformer2d::combat::hazards`, and publishes a normal
    /// `HitEvent`, so i-frames, wallet shield, knockback, and death all apply. It
    /// can be static or moving; the motion path is optional.
    ResetToSpawn,
    /// Refreshes pogo / movement resources (legacy `PogoOrb`).
    PogoRefresh,
    Rebound {
        impulse: ae::Vec2,
    },
}

/// When a destroyed `Surface` returns.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SurfaceRespawn {
    #[default]
    Never,
    OnRoomReload,
    AfterSeconds(f32),
}

/// Typed intermediate representation for one LDtk `Surface` (or a legacy
/// alias such as `Solid`, `OneWayPlatform`, `BlinkWall`, `HazardBlock`,
/// `PogoOrb`, `ReboundPad`, `Breakable`).
///
/// `compile_surface` lowers it into engine runtime pieces (`ae::Block`,
/// `ae::RoomObject`), so collision and contact systems do not parse strings or
/// JSON.
#[derive(Clone, Debug, PartialEq)]
pub struct LdtkSurfaceSpec {
    /// LDtk-stable instance id.
    pub iid: String,
    /// Display name (defaults to identifier when not provided).
    pub name: String,
    /// Top-left in active-area-local Ambition coordinates (post-offset).
    pub min: ae::Vec2,
    /// Width and height in pixels.
    pub size: ae::Vec2,
    pub collision: SurfaceCollision,
    pub breakability: SurfaceBreakability,
    pub contact: SurfaceContact,
    pub respawn: SurfaceRespawn,
    /// Hit points for breakable surfaces. Ignored when `Indestructible`.
    pub max_hp: i32,
}

impl LdtkSurfaceSpec {
    /// Build an indestructible solid wall with no contact behavior. For tests and
    /// migration shims.
    pub fn solid_wall(
        iid: impl Into<String>,
        name: impl Into<String>,
        min: ae::Vec2,
        size: ae::Vec2,
    ) -> Self {
        Self {
            iid: iid.into(),
            name: name.into(),
            min,
            size,
            collision: SurfaceCollision::Solid,
            breakability: SurfaceBreakability::Indestructible,
            contact: SurfaceContact::None,
            respawn: SurfaceRespawn::Never,
            max_hp: 0,
        }
    }
}

/// Result of compiling a single `LdtkSurfaceSpec` into runtime engine data.
#[derive(Clone, Debug, Default)]
pub struct SurfaceCompiled {
    pub blocks: Vec<ae::Block>,
    pub breakables: Vec<
        ambition_platformer2d_world::rooms::Authored<
            ambition_platformer2d_world::rooms::BreakableSpec,
        >,
    >,
}

/// LDtk identifiers that lower through the typed surface pipeline.
///
/// The editor keeps these as distinct entities so designers pick the right
/// primitive. The parser collapses them to one `LdtkSurfaceSpec`, so
/// collision, contact, and breakability have one conversion path. There is no
/// generic `Surface` authoring entity.
///
/// `HazardBlock` is the reset volume (the pit floor). It does no damage. A
/// surface that hurts is a `DamageVolume`, which does not use this pipeline.
pub(super) const SURFACE_LIKE_IDENTIFIERS: &[&str] = &[
    "Solid",
    "OneWayPlatform",
    "BlinkWall",
    "HazardBlock",
    "PogoOrb",
    "ReboundPad",
    "BreakablePlatform",
    "BreakablePogoOrb",
];

/// True if `identifier` lowers into `LdtkSurfaceSpec` via `parse_surface_spec`.
pub(super) fn is_surface_like_identifier(identifier: &str) -> bool {
    SURFACE_LIKE_IDENTIFIERS.contains(&identifier)
}

/// Build an `LdtkSurfaceSpec` from a Surface-shaped LDtk entity.
///
/// Dispatch by identifier:
/// - `Surface`: parse fields directly.
/// - `Solid`/`OneWayPlatform`/`BlinkWall`/`HazardBlock`/`PogoOrb`/`ReboundPad`/`Breakable`:
///   legacy aliases. Their fields map onto the Surface model, so the same
///   compile path gives the same runtime data.
pub(super) fn parse_surface_spec(
    entity: &LdtkEntityInstance,
    min: ae::Vec2,
    size: ae::Vec2,
    name: String,
) -> Result<LdtkSurfaceSpec, String> {
    let mut spec = LdtkSurfaceSpec {
        iid: entity.iid.clone(),
        name,
        min,
        size,
        collision: SurfaceCollision::None,
        breakability: SurfaceBreakability::Indestructible,
        contact: SurfaceContact::None,
        respawn: SurfaceRespawn::Never,
        max_hp: 0,
    };

    match entity.identifier.as_str() {
        "Solid" => {
            spec.collision = SurfaceCollision::Solid;
        }
        "OneWayPlatform" => {
            spec.collision = SurfaceCollision::OneWayUp;
        }
        "BlinkWall" => {
            spec.collision = match field_string(entity, "tier")
                .unwrap_or_else(|| "Soft".to_string())
                .as_str()
            {
                "Soft" => SurfaceCollision::BlinkSoft,
                "Hard" => SurfaceCollision::BlinkHard,
                other => return Err(format!("invalid BlinkWall tier '{other}'")),
            };
        }
        "HazardBlock" => {
            spec.collision = SurfaceCollision::None;
            // No `damage` field is read: `HazardBlock` does not damage, and the shared
            // defs have no such field. See [`SurfaceContact::ResetToSpawn`].
            spec.contact = SurfaceContact::ResetToSpawn;
        }
        "PogoOrb" => {
            spec.collision = SurfaceCollision::None;
            spec.contact = SurfaceContact::PogoRefresh;
        }
        "ReboundPad" => {
            let impulse_x =
                field_f32(entity, "impulseX").ok_or_else(|| "missing impulseX".to_string())?;
            let impulse_y =
                field_f32(entity, "impulseY").ok_or_else(|| "missing impulseY".to_string())?;
            spec.collision = SurfaceCollision::None;
            spec.contact = SurfaceContact::Rebound {
                impulse: ae::Vec2::new(impulse_x, impulse_y),
            };
        }
        "BreakablePlatform" => {
            // Constrained breakable: `collision` must be Solid or OneWayUp (the LDtk
            // enum has no None option), so OnStand+None cannot be authored.
            spec.collision = match field_string(entity, "collision").as_deref() {
                Some("Solid") | None => SurfaceCollision::Solid,
                Some("OneWayUp") => SurfaceCollision::OneWayUp,
                Some(other) => {
                    return Err(format!("invalid BreakablePlatform collision '{other}'"));
                }
            };
            spec.breakability = match field_string(entity, "trigger")
                .as_deref()
                .unwrap_or("OnHit")
            {
                "OnHit" => SurfaceBreakability::BreakOnHit,
                "OnStand" => SurfaceBreakability::BreakOnStand,
                "Either" => SurfaceBreakability::BreakOnHitOrStand,
                other => return Err(format!("invalid BreakablePlatform trigger '{other}'")),
            };
            spec.respawn = parse_breakable_respawn(entity)?;
            spec.max_hp = field_i32(entity, "max_hp").unwrap_or(3);
        }
        "BreakablePogoOrb" => {
            // Pogo orb with health. No body collision. While intact, the collision world
            // gets a `BlockKind::PogoOrb` block from `world_with_sandbox_solids`, and each
            // pogo bounce damages the orb until it breaks.
            spec.collision = SurfaceCollision::None;
            spec.breakability = SurfaceBreakability::BreakOnHit;
            spec.contact = SurfaceContact::PogoRefresh;
            spec.respawn = parse_breakable_respawn(entity)?;
            spec.max_hp = field_i32(entity, "max_hp").unwrap_or(3);
        }
        other => {
            return Err(format!(
                "parse_surface_spec called for non-surface identifier '{other}'"
            ));
        }
    }

    Ok(spec)
}

/// Parse the `Breakable.respawn` field and its companion `respawn_seconds`.
///
/// Accepted forms:
/// - `"Never"` (default), `"OnRoomReload"`
/// - `"AfterSeconds"` with a positive `respawn_seconds` float field
/// - legacy inline `"AfterSeconds:<n>"` (instances saved before `respawn_seconds`)
/// - legacy `"Persistent"`, mapped to `Never`
fn parse_breakable_respawn(entity: &LdtkEntityInstance) -> Result<SurfaceRespawn, String> {
    let raw = field_string(entity, "respawn").unwrap_or_else(|| "Never".to_string());
    let trimmed = raw.trim();
    if let Some(seconds) = trimmed
        .strip_prefix("AfterSeconds:")
        .and_then(|text| text.parse::<f32>().ok())
    {
        if seconds <= 0.0 || seconds.is_nan() {
            return Err(format!(
                "AfterSeconds respawn requires positive seconds, got {seconds}"
            ));
        }
        return Ok(SurfaceRespawn::AfterSeconds(seconds));
    }
    match trimmed {
        "Never" | "Persistent" | "" => Ok(SurfaceRespawn::Never),
        "OnRoomReload" => Ok(SurfaceRespawn::OnRoomReload),
        "AfterSeconds" => {
            let seconds = field_f32(entity, "respawn_seconds")
                .ok_or_else(|| "AfterSeconds respawn requires respawn_seconds".to_string())?;
            if seconds <= 0.0 || seconds.is_nan() {
                return Err(format!(
                    "AfterSeconds respawn requires positive respawn_seconds, got {seconds}"
                ));
            }
            Ok(SurfaceRespawn::AfterSeconds(seconds))
        }
        other => Err(format!("invalid Breakable respawn '{other}'")),
    }
}

/// Lower a typed `LdtkSurfaceSpec` into engine runtime data.
///
/// Supported combinations:
///
/// - `Indestructible` + collision (or static contact) → one `ae::Block`.
/// - Any breakable collision/`None` contact → a `RoomObjectKind::Breakable`,
///   whose engine `BreakableCollision` mirrors the authored `SurfaceCollision`.
///
/// Other combinations (for example breakable + damage contact, or breakable +
/// blink wall) return an error that tells the author what is wrong.
pub fn compile_surface(spec: &LdtkSurfaceSpec) -> Result<SurfaceCompiled, String> {
    if spec.size.x <= 0.0 || spec.size.y <= 0.0 {
        return Err(format!(
            "Surface {} has non-positive size {}x{}",
            spec.iid, spec.size.x, spec.size.y
        ));
    }

    let mut blocks = Vec::new();
    let mut breakables: Vec<
        ambition_platformer2d_world::rooms::Authored<
            ambition_platformer2d_world::rooms::BreakableSpec,
        >,
    > = Vec::new();

    match spec.breakability {
        SurfaceBreakability::Indestructible => {
            if let Some(mut block) = compile_static_surface_block(spec)? {
                // Durable identity (§3.6): entity-authored geometry is named
                // by its placement (the LDtk iid); one block per surface.
                block.id = ae::GeoId::placement(ae::PlacementId::new(spec.iid.clone()), 0);
                blocks.push(block);
            }
        }
        breakable_kind => {
            // Only one breakable+contact combination is allowed: BreakablePogoOrb
            // (BreakOnHit, collision=None, PogoRefresh contact). While the orb is intact,
            // `world_with_sandbox_solids` emits a `BlockKind::PogoOrb` block, and each
            // pogo bounce damages the orb.
            let pogo_orb_combo = matches!(spec.contact, SurfaceContact::PogoRefresh)
                && matches!(spec.collision, SurfaceCollision::None)
                && matches!(breakable_kind, SurfaceBreakability::BreakOnHit);
            if !matches!(spec.contact, SurfaceContact::None) && !pogo_orb_combo {
                return Err(format!(
                    "Surface {} combines breakability with contact; not yet supported",
                    spec.iid
                ));
            }
            let collision = match spec.collision {
                SurfaceCollision::None => {
                    ambition_platformer2d_world::rooms::BreakableCollisionSpec::None
                }
                SurfaceCollision::Solid => {
                    ambition_platformer2d_world::rooms::BreakableCollisionSpec::Solid
                }
                SurfaceCollision::OneWayUp => {
                    ambition_platformer2d_world::rooms::BreakableCollisionSpec::OneWayUp
                }
                SurfaceCollision::BlinkSoft | SurfaceCollision::BlinkHard => {
                    return Err(format!(
                        "Surface {} cannot mix BlinkWall collision with breakability yet",
                        spec.iid
                    ));
                }
            };
            if matches!(breakable_kind, SurfaceBreakability::BreakOnStand)
                && !collision.blocks_movement()
            {
                return Err(format!(
                    "Surface {} BreakOnStand requires non-None collision",
                    spec.iid
                ));
            }
            let max_hp = spec.max_hp.max(1);
            let mut breakable = ambition_platformer2d_world::rooms::BreakableSpec::new(max_hp);
            breakable.collision = collision;
            breakable.trigger = match breakable_kind {
                SurfaceBreakability::BreakOnHit => {
                    ambition_platformer2d_world::rooms::BreakableTriggerSpec::OnHit
                }
                SurfaceBreakability::BreakOnStand => {
                    ambition_platformer2d_world::rooms::BreakableTriggerSpec::OnStand
                }
                SurfaceBreakability::BreakOnHitOrStand => {
                    ambition_platformer2d_world::rooms::BreakableTriggerSpec::Either
                }
                SurfaceBreakability::Indestructible => unreachable!(),
            };
            breakable.respawn = match spec.respawn {
                SurfaceRespawn::Never => ambition_entity_catalog::placements::HazardRespawn::Never,
                SurfaceRespawn::OnRoomReload => {
                    ambition_entity_catalog::placements::HazardRespawn::OnRoomReload
                }
                SurfaceRespawn::AfterSeconds(seconds) => {
                    ambition_entity_catalog::placements::HazardRespawn::AfterSeconds(seconds)
                }
            };
            breakable.pogo_refresh = pogo_orb_combo;
            breakables.push(ambition_platformer2d_world::rooms::Authored::new(
                spec.iid.clone(),
                spec.name.clone(),
                ae::aabb_from_min_size(spec.min, spec.size),
                breakable,
            ));
        }
    }

    Ok(SurfaceCompiled { blocks, breakables })
}

fn compile_static_surface_block(spec: &LdtkSurfaceSpec) -> Result<Option<ae::Block>, String> {
    let name = spec.name.clone();
    let min = spec.min;
    let size = spec.size;
    match (spec.collision, spec.contact) {
        (SurfaceCollision::None, SurfaceContact::None) => Ok(None),
        (SurfaceCollision::Solid, SurfaceContact::None) => {
            Ok(Some(ae::Block::solid(name, min, size)))
        }
        (SurfaceCollision::OneWayUp, SurfaceContact::None) => {
            Ok(Some(ae::Block::one_way(name, min, size)))
        }
        (SurfaceCollision::BlinkSoft, SurfaceContact::None) => Ok(Some(ae::Block::blink_wall(
            name,
            min,
            size,
            ae::BlinkWallTier::Soft,
        ))),
        (SurfaceCollision::BlinkHard, SurfaceContact::None) => Ok(Some(ae::Block::blink_wall(
            name,
            min,
            size,
            ae::BlinkWallTier::Hard,
        ))),
        // The reset volume becomes `BlockKind::Hazard` ("Reset surface. Hitting this
        // returns the player to spawn."). Damage does not use this path; it uses the
        // hazard placement from a `DamageVolume`. See [`SurfaceContact::ResetToSpawn`].
        (SurfaceCollision::None, SurfaceContact::ResetToSpawn) => {
            Ok(Some(ae::Block::hazard(name, min, size)))
        }
        (SurfaceCollision::None, SurfaceContact::PogoRefresh) => {
            let radius = size.x.min(size.y) * 0.5;
            Ok(Some(ae::Block::pogo_orb(name, min + size * 0.5, radius)))
        }
        (SurfaceCollision::None, SurfaceContact::Rebound { impulse }) => {
            Ok(Some(ae::Block::rebound(name, min, size, impulse)))
        }
        (collision, contact) => Err(format!(
            "Surface {} has unsupported collision/contact combination ({:?} + {:?})",
            spec.iid, collision, contact
        )),
    }
}
