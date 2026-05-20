//! `compile_identifier` round-trips for the typed surface family:
//! Solid / OneWay / Pogo / Rebound / BlinkWall / Breakable…

use ambition_engine as ae;
use serde_json::Value;

use super::super::surfaces::*;
use super::super::*;

#[test]
fn one_way_platform_compiles_to_one_way_block() {
    let compiled = super::compile_identifier("OneWayPlatform", [96, 16], &[]);
    assert_eq!(compiled.blocks.len(), 1);
    assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::OneWay));
}

#[test]
fn solid_compiles_to_solid_block() {
    let compiled = super::compile_identifier("Solid", [128, 32], &[]);
    assert_eq!(compiled.breakables.len(), 0);
    assert_eq!(compiled.blocks.len(), 1);
    assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::Solid));
}

#[test]
fn hazard_block_compiles_to_hazard_block() {
    let compiled = super::compile_identifier("HazardBlock", [64, 16], &[]);
    assert_eq!(compiled.blocks.len(), 1);
    assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::Hazard));
}

#[test]
fn pogo_orb_compiles_to_pogo_orb_block() {
    let compiled = super::compile_identifier("PogoOrb", [32, 32], &[]);
    assert_eq!(compiled.blocks.len(), 1);
    assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::PogoOrb));
}

#[test]
fn rebound_pad_compiles_to_rebound_block() {
    let compiled = super::compile_identifier(
        "ReboundPad",
        [32, 16],
        &[
            ("impulseX", Value::Number(serde_json::Number::from(0))),
            ("impulseY", Value::Number(serde_json::Number::from(-600))),
        ],
    );
    assert_eq!(compiled.blocks.len(), 1);
    assert!(matches!(
        compiled.blocks[0].kind,
        ae::BlockKind::Rebound { .. }
    ));
}

#[test]
fn blink_wall_uses_tier_field() {
    let soft = super::compile_identifier(
        "BlinkWall",
        [32, 32],
        &[("tier", Value::String("Soft".into()))],
    );
    let hard = super::compile_identifier(
        "BlinkWall",
        [32, 32],
        &[("tier", Value::String("Hard".into()))],
    );
    assert!(matches!(
        soft.blocks[0].kind,
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Soft
        }
    ));
    assert!(matches!(
        hard.blocks[0].kind,
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Hard
        }
    ));
}

#[test]
fn rebound_pad_requires_impulse_fields() {
    let entity = super::make_entity("ReboundPad", [16, 16], &[]);
    let err = parse_surface_spec(
        &entity,
        ae::Vec2::ZERO,
        ae::Vec2::new(16.0, 16.0),
        "rp".into(),
    )
    .expect_err("missing impulses");
    assert!(err.contains("missing impulseX"), "{err}");
}

/// `BreakablePlatform` with `collision=Solid` lowers to a Breakable
/// runtime object with hard collision while intact.
#[test]
fn breakable_platform_solid_compiles_with_solid_collision() {
    let compiled = super::compile_identifier(
        "BreakablePlatform",
        [48, 48],
        &[
            ("collision", Value::String("Solid".into())),
            ("trigger", Value::String("OnHit".into())),
            ("max_hp", Value::Number(serde_json::Number::from(2))),
        ],
    );
    assert!(compiled.blocks.is_empty());
    assert_eq!(compiled.breakables.len(), 1);
    let breakable = &compiled.breakables[0].payload;
    assert_eq!(breakable.collision, ae::BreakableCollision::Solid);
    assert_eq!(breakable.trigger, ae::BreakableTrigger::OnHit);
    assert_eq!(breakable.health.max, 2);
    assert!(!breakable.pogo_refresh);
}

/// `BreakablePlatform` with `collision=OneWayUp` lowers to a Breakable
/// runtime object that lands as a one-way platform.
#[test]
fn breakable_platform_one_way_up_compiles() {
    let compiled = super::compile_identifier(
        "BreakablePlatform",
        [80, 16],
        &[
            ("collision", Value::String("OneWayUp".into())),
            ("trigger", Value::String("OnStand".into())),
        ],
    );
    assert_eq!(compiled.breakables.len(), 1);
    let breakable = &compiled.breakables[0].payload;
    assert_eq!(breakable.collision, ae::BreakableCollision::OneWayUp);
    assert_eq!(breakable.trigger, ae::BreakableTrigger::OnStand);
}

/// `BreakablePlatform` rejects unknown collision values. The LDtk enum
/// has only Solid|OneWayUp, so the previous OnStand+None combo is
/// unrepresentable in the editor and we don't even need a degrade path.
#[test]
fn breakable_platform_rejects_unknown_collision() {
    let entity = super::make_entity(
        "BreakablePlatform",
        [32, 32],
        &[("collision", Value::String("None".into()))],
    );
    let err = parse_surface_spec(
        &entity,
        ae::Vec2::ZERO,
        ae::Vec2::new(32.0, 32.0),
        "p".into(),
    )
    .expect_err("None is not a valid BreakablePlatform collision");
    assert!(err.contains("BreakablePlatform"), "{err}");
}

/// Engine compile path stays strict: a hand-crafted incoherent combo
/// (BreakOnStand with collision=None) is still rejected, even though
/// the LDtk adapter can no longer produce one for BreakablePlatform.
#[test]
fn engine_compile_still_rejects_on_stand_without_collision() {
    let bad_spec = LdtkSurfaceSpec {
        iid: "test".into(),
        name: "test".into(),
        min: ae::Vec2::ZERO,
        size: ae::Vec2::new(32.0, 32.0),
        collision: SurfaceCollision::None,
        breakability: SurfaceBreakability::BreakOnStand,
        contact: SurfaceContact::None,
        respawn: SurfaceRespawn::Never,
        max_hp: 3,
    };
    let err = compile_surface(&bad_spec).expect_err("BreakOnStand requires collision");
    assert!(
        err.contains("BreakOnStand requires non-None collision"),
        "{err}"
    );
}

/// `respawn = AfterSeconds` requires a positive `respawn_seconds` field.
#[test]
fn breakable_platform_after_seconds_requires_positive_respawn_seconds() {
    let missing_field = super::make_entity(
        "BreakablePlatform",
        [32, 32],
        &[
            ("collision", Value::String("Solid".into())),
            ("trigger", Value::String("OnHit".into())),
            ("respawn", Value::String("AfterSeconds".into())),
        ],
    );
    let err = parse_surface_spec(
        &missing_field,
        ae::Vec2::ZERO,
        ae::Vec2::new(32.0, 32.0),
        "p".into(),
    )
    .expect_err("AfterSeconds without respawn_seconds is rejected");
    assert!(err.contains("respawn_seconds"), "{err}");

    let zero_seconds = super::make_entity(
        "BreakablePlatform",
        [32, 32],
        &[
            ("collision", Value::String("Solid".into())),
            ("trigger", Value::String("OnHit".into())),
            ("respawn", Value::String("AfterSeconds".into())),
            (
                "respawn_seconds",
                Value::Number(serde_json::Number::from(0)),
            ),
        ],
    );
    let err = parse_surface_spec(
        &zero_seconds,
        ae::Vec2::ZERO,
        ae::Vec2::new(32.0, 32.0),
        "p".into(),
    )
    .expect_err("respawn_seconds must be positive");
    assert!(err.contains("positive"), "{err}");
}

/// `BreakablePogoOrb` lowers to a Breakable with the `pogo_refresh`
/// flag set, so the gameplay loop emits a PogoOrb collision-world
/// block while intact and routes pogo bounces back as damage.
#[test]
fn breakable_pogo_orb_compiles_with_pogo_flag() {
    let compiled = super::compile_identifier(
        "BreakablePogoOrb",
        [36, 36],
        &[("max_hp", Value::Number(serde_json::Number::from(4)))],
    );
    assert!(compiled.blocks.is_empty());
    assert_eq!(compiled.breakables.len(), 1);
    let breakable = &compiled.breakables[0].payload;
    assert!(breakable.pogo_refresh);
    assert_eq!(breakable.collision, ae::BreakableCollision::None);
    assert_eq!(breakable.trigger, ae::BreakableTrigger::OnHit);
    assert_eq!(breakable.health.max, 4);
}

#[test]
fn no_surface_authoring_primitive_is_registered() {
    // The LDtk editor stays differentiated; there should be no canonical
    // generic Surface entity registered or routed through the parser.
    assert!(
        !known_entity("Surface"),
        "Surface must not be a registered LDtk entity"
    );
    assert!(
        !is_surface_like_identifier("Surface"),
        "Surface must not route through the typed surface conversion path"
    );
    // The legacy generic `Breakable` is gone; only the narrow types
    // remain.
    assert!(!known_entity("Breakable"), "legacy Breakable was removed");
    assert!(
        !is_surface_like_identifier("Breakable"),
        "legacy Breakable parser branch was removed"
    );
    // Differentiated identifiers DO still route through the typed
    // conversion path.
    for id in [
        "Solid",
        "OneWayPlatform",
        "BlinkWall",
        "HazardBlock",
        "PogoOrb",
        "ReboundPad",
        "BreakablePlatform",
        "BreakablePogoOrb",
    ] {
        assert!(is_surface_like_identifier(id), "{id}");
    }
    for id in ["PlayerStart", "LoadingZone", "DebugLabel", "NpcSpawn"] {
        assert!(!is_surface_like_identifier(id), "{id}");
    }
}
