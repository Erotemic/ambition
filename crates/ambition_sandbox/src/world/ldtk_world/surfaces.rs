use ambition_engine as ae;

use super::fields::{field_f32, field_i32, field_string};
use super::project::LdtkEntityInstance;

/// Collision behavior contributed by an LDtk-authored `Surface`.
///
/// `Surface` is the authoring-time primitive: designers place a single
/// rectangular entity and tweak its `collision`, `breakability`, `contact`,
/// and `respawn` fields rather than swapping between a zoo of one-purpose
/// entities. The compile step translates this into typed engine
/// `Block`/`Breakable`/contact data.
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
    /// Damage / hazard reset (legacy `HazardBlock`).
    Damage { amount: i32 },
    /// Refreshes pogo / movement resources (legacy `PogoOrb`).
    PogoRefresh,
    /// Applies a fixed impulse on contact (legacy `ReboundPad`).
    Rebound { impulse: ae::Vec2 },
}

/// When a destroyed `Surface` returns.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SurfaceRespawn {
    #[default]
    Never,
    OnRoomReload,
    AfterSeconds(f32),
}

/// Typed intermediate representation for a single LDtk `Surface` (or legacy
/// alias such as `Solid`, `OneWayPlatform`, `BlinkWall`, `HazardBlock`,
/// `PogoOrb`, `ReboundPad`, `Breakable`).
///
/// This is the authoring-side data parsed straight out of LDtk JSON. The
/// compile step (`compile_surface`) lowers it into engine-native runtime
/// pieces (`ae::Block`, `ae::RoomObject`) so collision/contact systems never
/// have to reparse strings or JSON.
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
    /// Build an indestructible solid wall with no contact behavior. Convenient
    /// for tests and migration shims.
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
    pub breakables: Vec<crate::rooms::Authored<crate::interaction::Breakable>>,
}

/// LDtk identifiers that lower into the typed runtime "surface" conversion
/// pipeline.
///
/// The LDtk editor keeps these visually/semantically distinct so designers
/// pick the right primitive (Solid, OneWayPlatform, BlinkWall, HazardBlock,
/// PogoOrb, ReboundPad, Breakable). Internally the parser collapses them to
/// the same typed `LdtkSurfaceSpec` so collision/contact/breakability code
/// has a single conversion path. There is intentionally no canonical
/// generic `Surface` authoring entity; the editor stays differentiated.
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
/// Identifier-based dispatch:
/// - `Surface`: parse fields directly (the canonical authoring path).
/// - `Solid`/`OneWayPlatform`/`BlinkWall`/`HazardBlock`/`PogoOrb`/`ReboundPad`/`Breakable`:
///   legacy aliases — fields are remapped onto the Surface model so the same
///   compile path produces the same runtime data the old per-identifier
///   branches did.
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
            spec.contact = SurfaceContact::Damage {
                amount: field_i32(entity, "damage").unwrap_or(1),
            };
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
            // Constrained breakable: `collision` must be Solid or OneWayUp
            // (the LDtk enum has no None option), so the historically
            // incoherent OnStand+None combo is unrepresentable in the
            // editor — no degrade path needed.
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
            // Pogo-orb-with-health. No body collision; while intact the
            // collision world gets a `BlockKind::PogoOrb` block emitted
            // by `world_with_sandbox_solids`, and successful pogo bounces
            // damage the orb until it breaks.
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

/// Parse the `Breakable.respawn` field plus its companion `respawn_seconds`.
///
/// Accepted forms:
/// - `"Never"` (default), `"OnRoomReload"`
/// - `"AfterSeconds"` paired with a positive `respawn_seconds` float field
/// - legacy inline `"AfterSeconds:<n>"` shorthand (still accepted for older
///   instances saved before `respawn_seconds` was added)
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
/// Combinations supported today:
///
/// - `Indestructible` + collision (or static contact) → a single `ae::Block`.
/// - Any breakable collision/`None` contact → a `RoomObjectKind::Breakable`,
///   whose engine `BreakableCollision` mirrors the authored `SurfaceCollision`.
///
/// Combinations that are not yet wired (e.g. breakable + damage contact, or
/// breakable + blink wall) return descriptive errors so authors hit a clear
/// validation message rather than silent gameplay drift.
pub fn compile_surface(spec: &LdtkSurfaceSpec) -> Result<SurfaceCompiled, String> {
    if spec.size.x <= 0.0 || spec.size.y <= 0.0 {
        return Err(format!(
            "Surface {} has non-positive size {}x{}",
            spec.iid, spec.size.x, spec.size.y
        ));
    }

    let mut blocks = Vec::new();
    let mut breakables: Vec<crate::rooms::Authored<crate::interaction::Breakable>> = Vec::new();

    match spec.breakability {
        SurfaceBreakability::Indestructible => {
            if let Some(block) = compile_static_surface_block(spec)? {
                blocks.push(block);
            }
        }
        breakable_kind => {
            // Allow exactly one breakable+contact combo: BreakablePogoOrb,
            // which is BreakOnHit with collision=None and PogoRefresh contact.
            // The runtime emits a `BlockKind::PogoOrb` block in
            // `world_with_sandbox_solids` while the orb is intact, and the
            // sandbox damages the orb on each pogo bounce. Other
            // breakable+contact combos remain unsupported.
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
                SurfaceCollision::None => crate::interaction::BreakableCollision::None,
                SurfaceCollision::Solid => crate::interaction::BreakableCollision::Solid,
                SurfaceCollision::OneWayUp => crate::interaction::BreakableCollision::OneWayUp,
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
            let mut breakable = crate::interaction::Breakable::new(spec.iid.clone(), max_hp);
            breakable.collision = collision;
            breakable.trigger = match breakable_kind {
                SurfaceBreakability::BreakOnHit => crate::interaction::BreakableTrigger::OnHit,
                SurfaceBreakability::BreakOnStand => crate::interaction::BreakableTrigger::OnStand,
                SurfaceBreakability::BreakOnHitOrStand => crate::interaction::BreakableTrigger::Either,
                SurfaceBreakability::Indestructible => unreachable!(),
            };
            breakable.respawn = match spec.respawn {
                SurfaceRespawn::Never => ae::RespawnPolicy::Never,
                SurfaceRespawn::OnRoomReload => ae::RespawnPolicy::OnRoomReload,
                SurfaceRespawn::AfterSeconds(seconds) => ae::RespawnPolicy::AfterSeconds(seconds),
            };
            breakable.pogo_refresh = pogo_orb_combo;
            breakables.push(crate::rooms::Authored::new(
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
        // Damage contact maps to the legacy hazard reset block; per-amount
        // damage tuning today flows through `RoomObjectKind::DamageVolume`,
        // so for now Surface damage parity stays at the BlockKind::Hazard
        // level. TODO: emit a `DamageVolume` object when amount != 1.
        (SurfaceCollision::None, SurfaceContact::Damage { .. }) => {
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
