//! The LDtk entity-converter REGISTRY (ADR 0009): the engine's standard
//! vocabulary and content-installed converters enter conversion through the
//! same seam. These tests pin the registry's three contracts: the standard
//! table mirrors the marker-registration list exactly, an installed content
//! converter passes validation + converts like a built-in, and an unknown
//! identifier still fails loudly.

use serde_json::Value;

use ambition_engine_core as ae;

use super::super::conversion::converter_for;
use super::super::project::*;
use super::super::{
    install_ldtk_entity_converters, LdtkEntityConverter, LdtkEntityCtx, RuntimeEntityEmission,
    AMBITION_LDTK_ENTITY_IDENTIFIERS,
};

/// The standard converter table and `AMBITION_LDTK_ENTITY_IDENTIFIERS` (the
/// bevy_ecs_ldtk marker-registration list) must not drift: every identifier
/// the runtime spine registers converts, and the converter table introduces
/// no identifier the spine doesn't know.
#[test]
fn standard_converters_mirror_the_marker_identifier_list() {
    for identifier in AMBITION_LDTK_ENTITY_IDENTIFIERS {
        assert!(
            converter_for(identifier).is_some(),
            "marker-registered identifier '{identifier}' has no standard converter"
        );
    }
}

fn synthetic_level(entities: Vec<LdtkEntityInstance>) -> LdtkProject {
    let mut instances = vec![super::make_entity_at(
        "PlayerStart",
        [32, 400],
        [16, 32],
        &[],
    )];
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

/// A game-registered converter: a `TestContentTotem` entity emits a decorative
/// prop, exactly as a content crate would extend the vocabulary.
fn convert_test_content_totem(ctx: &LdtkEntityCtx<'_>) -> Result<RuntimeEntityEmission, String> {
    Ok(RuntimeEntityEmission {
        props: vec![crate::rooms::PropSpec {
            id: ctx.entity.iid.clone(),
            name: ctx.name.clone(),
            kind: "test_totem".to_string(),
            pos: ctx.min + ctx.size * 0.5,
            size: ctx.size,
        }],
        ..Default::default()
    })
}

/// Install-once + convert: a content identifier flows through validation and
/// `to_room_set` exactly like the engine vocabulary. (The install is
/// process-global — the identifier is test-namespaced so no other fixture can
/// collide with it.)
#[test]
fn installed_content_converter_validates_and_converts() {
    install_ldtk_entity_converters([(
        "TestContentTotem".to_string(),
        convert_test_content_totem as LdtkEntityConverter,
    )]);

    let project = synthetic_level(vec![super::make_entity_at(
        "TestContentTotem",
        [128, 320],
        [32, 48],
        &[],
    )]);
    assert!(
        project.validate().is_ok(),
        "installed identifier should pass validation: {:?}",
        project.validate().errors
    );
    let room_set = project
        .to_room_set()
        .expect("synthetic project with a content entity should compose");
    let room = &room_set.rooms[0];
    assert_eq!(room.props.len(), 1, "the content converter should emit");
    assert_eq!(room.props[0].kind, "test_totem");
    assert_eq!(room.props[0].pos, ae::Vec2::new(144.0, 344.0));
}

/// The `SurfaceChain` standard converter (demo plan S3): a points-field
/// entity lands in `World::chains` — the momentum-locomotion geometry flows
/// LDtk → emission → RoomSpec like every other family.
#[test]
fn surface_chain_entity_converts_into_world_chains() {
    let project = synthetic_level(vec![super::make_entity_at(
        "SurfaceChain",
        [0, 0],
        [16, 16],
        &[("points", Value::String("0,400; 300,400; 500,300".into()))],
    )]);
    assert!(
        project.validate().is_ok(),
        "SurfaceChain should validate: {:?}",
        project.validate().errors
    );
    let room_set = project.to_room_set().expect("chain project composes");
    let chains = &room_set.rooms[0].world.chains;
    assert_eq!(chains.len(), 1, "one authored chain");
    assert_eq!(chains[0].points.len(), 3);
    assert_eq!(chains[0].points[0], ae::Vec2::new(0.0, 400.0));
    assert!(!chains[0].closed);
    // Winding convention: authored left→right, the floor normal points up.
    assert_eq!(chains[0].normal(0), ae::Vec2::new(0.0, -1.0));
}

/// Bad chain geometry fails at CONVERSION (loudly), never reaching the sim —
/// the spatial-model validation tier.
#[test]
fn degenerate_surface_chain_fails_conversion() {
    let project = synthetic_level(vec![super::make_entity_at(
        "SurfaceChain",
        [0, 0],
        [16, 16],
        &[("points", Value::String("0,400".into()))],
    )]);
    assert!(
        project.to_room_set().is_err(),
        "a one-point chain must fail conversion"
    );
}

/// An identifier NOBODY registered still fails validation loudly — the
/// registry widens the vocabulary, it does not make the loader tolerant.
#[test]
fn unregistered_identifier_still_fails_validation() {
    let project = synthetic_level(vec![super::make_entity_at(
        "TotallyUnknownEntity",
        [128, 320],
        [32, 48],
        &[],
    )]);
    let report = project.validate();
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("TotallyUnknownEntity")),
        "unknown identifier should fail validation, got: {:?}",
        report.errors
    );
}
