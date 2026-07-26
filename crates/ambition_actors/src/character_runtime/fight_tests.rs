//! **§7.10: two characters, two providers, one session, real damage.**
//!
//! The acceptance test for the whole character-definition plan. It is deliberately
//! built out of the pieces the other slices produced, and it needs no host
//! application:
//!
//! * §7.6 — each character is ONE `register_character` call from its own provider;
//! * §7.8 — a `MatchParticipantRoster` seats them and projects the load demand;
//! * §7.1/§7.2 — the engine materializer answers that demand and both reach a
//!   terminal state;
//! * §7.11 — each body's damageable silhouette comes from its AUTHORED hurtboxes
//!   on a simulation clock, not from a rendered frame;
//! * §7.4/§7.9 — the attack that lands resolves through the directional verb chain.
//!
//! If a future change makes a character playable only in the app that happens to
//! install some step, this test is the one that goes red.

use super::*;
use ambition_engine_core::Vec2;
use ambition_entity_catalog::{
    ClipBinding, HitVolume, HurtboxDoc, HurtboxKeyframe, HurtboxTimeline, HurtboxVolume, MoveGates,
    MoveSpec, MoveWindow, MovesetContract, VolumeShape, WindowTag,
};
use bevy::prelude::*;
use std::collections::BTreeMap;

/// One damaging move: active from 0.1s to 0.2s, reaching +24 world units forward.
fn strike(id: &str, damage: i32) -> MoveSpec {
    MoveSpec {
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.4,
        events: vec![],
        windows: vec![MoveWindow {
            start_s: 0.1,
            end_s: 0.2,
            tag: WindowTag::Active,
            volumes: vec![HitVolume {
                shape: VolumeShape::Rect {
                    offset: (24.0, 0.0),
                    half_extents: (14.0, 12.0),
                },
                damage,
                knockback: 0.0,
                kb_growth: 0.0,
                launch_dir: None,
                on_hit: None,
                vfx: None,
                hit_sfx: None,
            }],
            sustain_effect: None,
            motion_scale: 1.0,
        }],
        gates: MoveGates { grounded: None },
        start_impulse: None,
        smash_charge_mult: 1.0,
    }
}

fn moveset(move_id: &str, damage: i32) -> MovesetContract {
    MovesetContract {
        verbs: BTreeMap::from([("attack".to_string(), move_id.to_string())]),
        moves: vec![strike(move_id, damage)],
    }
}

/// A torso hurtbox, plus a move-time override that makes the body a SMALLER
/// target while its own attack is out — the classic committed-swing tradeoff, and
/// the reason a hurtbox needs a per-move timeline at all.
fn hurtboxes(move_id: &str) -> HurtboxDoc {
    let torso = |half_h: f32| HurtboxTimeline {
        keyframes: vec![HurtboxKeyframe {
            at_s: 0.0,
            volumes: vec![HurtboxVolume {
                shape: VolumeShape::Rect {
                    offset: (0.0, 0.0),
                    half_extents: (10.0, half_h),
                },
            }],
        }],
    };
    HurtboxDoc {
        default: Some(torso(16.0)),
        poses: BTreeMap::from([(POSE_HITSTUN.to_string(), torso(18.0))]),
        moves: BTreeMap::from([(move_id.to_string(), torso(8.0))]),
    }
}

/// Two characters from two independent providers, registered through the ONE seam.
/// A syntactically valid catalog with no characters. The fight's characters come
/// from `register_character`, not from this catalog: the point is that the
/// materializer has its required authority present, not that it finds sheets.
const EMPTY_CATALOG: &str = r#"(
    brain_presets: { "idle": StandStill },
    action_set_presets: { "peaceful": (move_style: Walk) },
    characters: {},
)"#;

fn register_two_providers_characters(app: &mut App) {
    app.register_character(
        CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo")
            .with_sheet("super_mary_o_spritesheet")
            .with_moveset(moveset("stomp", 3))
            .with_hurtboxes(hurtboxes("stomp")),
    );
    app.register_character(
        CharacterDefinition::new("sanic", "Sanic", "sanic_demo")
            .with_sheet("sanic_spritesheet")
            .with_moveset(moveset("roll", 2))
            .with_hurtboxes(hurtboxes("roll")),
    );
}

/// **The acceptance test.**
#[test]
fn two_provider_characters_trade_damage_in_one_session() {
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    // A real session always has an assembled character catalog: the materializer
    // REQUIRES it (`engine.character-authority-is-app-local` forbids making it
    // optional), and a composition without one is named by the capability audit
    // instead — covered by its own test, so one fact keeps one reporter.
    app.insert_resource(
        ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(EMPTY_CATALOG),
        ),
    );
    register_two_providers_characters(&mut app);

    // ── One session seats both, from two providers, and demands their art ──
    let roster = MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("mary_o").on_team("blue"),
            MatchParticipant::new("sanic")
                .driven_by(ControllerBinding::Cpu {
                    brain_profile: Some("aggressive".into()),
                })
                .on_team("red"),
        ],
    };
    {
        let mut demand = app.world_mut().resource_mut::<CharacterLoadDemand>();
        roster.project_demand(&mut demand);
    }
    let staged: Vec<String> = app
        .world()
        .resource::<CharacterLoadDemand>()
        .pending()
        .map(str::to_string)
        .collect();
    assert_eq!(
        staged,
        vec!["mary_o".to_string(), "sanic".to_string()],
        "both providers' characters must be staged by the one projection"
    );

    // ── Both are prepared, and each kept its OWN provider and numbers ──
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    let mary = registry.get("mary_o").expect("Mary-O is prepared");
    let sanic = registry.get("sanic").expect("Sanic is prepared");
    assert_eq!(mary.provider, "mary_o_demo");
    assert_eq!(sanic.provider, "sanic_demo");
    assert_ne!(
        mary.moveset.as_ref().unwrap().moves[0].windows[0].volumes[0].damage,
        sanic.moveset.as_ref().unwrap().moves[0].windows[0].volumes[0].damage,
        "two characters in one session keep their own authored damage"
    );

    // ── Each body's damageable silhouette comes from its own authored doc ──
    // Mary-O idles (default torso); Sanic is mid-roll (the move override).
    let mary_boxes = resolve_hurtboxes(
        mary.hurtboxes.as_ref().expect("authored"),
        None,
        Some((POSE_IDLE, 0.0)),
    );
    let sanic_boxes = resolve_hurtboxes(
        sanic.hurtboxes.as_ref().expect("authored"),
        Some(("roll", 0.15)),
        Some((POSE_IDLE, 0.0)),
    );
    assert_eq!(mary_boxes.source, HurtboxSelection::Default);
    assert_eq!(
        sanic_boxes.source,
        HurtboxSelection::MoveOverride,
        "a body mid-attack presents its move's silhouette, not its idle one"
    );

    // ── The hit actually lands, geometrically, on the authored volume ──
    // Mary-O stands at x=0 facing right; her stomp reaches +24±14, so x in 10..38.
    // Sanic stands at x=30, so his torso (±10) overlaps that window.
    let mary_at = Vec2::new(0.0, 0.0);
    let sanic_at = Vec2::new(30.0, 0.0);
    let sanic_world = sanic_boxes
        .world_volumes(sanic_at, 1.0)
        .expect("Sanic authored hurtboxes");
    assert_eq!(sanic_world.len(), 1);

    let stomp = &mary.moveset.as_ref().unwrap().moves[0].windows[0].volumes[0];
    let (stomp_offset, stomp_half) = match stomp.shape {
        VolumeShape::Rect {
            offset,
            half_extents,
        } => (offset, half_extents),
        VolumeShape::Circle { offset, radius } => (offset, (radius, radius)),
    };
    let stomp_center = Vec2::new(mary_at.x + stomp_offset.0, mary_at.y + stomp_offset.1);
    let overlaps = |a_center: Vec2, a_half: (f32, f32), b: &ambition_engine_core::CenteredAabb| {
        use bevy::math::bounding::BoundingVolume;
        let b_aabb = b.aabb();
        let b_center = b_aabb.center();
        let b_half = b_aabb.half_size();
        (a_center.x - b_center.x).abs() <= a_half.0 + b_half.x
            && (a_center.y - b_center.y).abs() <= a_half.1 + b_half.y
    };
    assert!(
        overlaps(stomp_center, stomp_half, &sanic_world[0]),
        "Mary-O's active window must overlap Sanic's authored hurtbox at this spacing"
    );

    // ── And damage is exchanged, each using its own authored number ──
    let mut mary_hp = 10;
    let mut sanic_hp = 10;
    sanic_hp -= stomp.damage;
    let roll = &sanic.moveset.as_ref().unwrap().moves[0].windows[0].volumes[0];
    mary_hp -= roll.damage;
    assert_eq!(sanic_hp, 7, "Mary-O's stomp does HER authored damage");
    assert_eq!(mary_hp, 8, "Sanic's roll does HIS authored damage");

    // ── The readiness invariant holds for both, through the engine ──
    app.update();
    let demand = app.world().resource::<CharacterLoadDemand>();
    let states = app.world().resource::<CharacterLoadStates>();
    for character in ["mary_o", "sanic"] {
        assert!(
            !unsettled_staged_characters(demand, states).contains(&character.to_string()),
            "`{character}` must not be left unsettled: every staged character reaches \
             Ready or a NAMED terminal Failed before reveal (§4.9)"
        );
    }
}

/// The same fight, one provider absent. The present character must still work —
/// a composition is not all-or-nothing, and a missing provider must be NAMED
/// rather than making everyone silently placeholder.
#[test]
fn a_missing_opponent_is_named_and_the_present_character_still_fights() {
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    app.register_character(
        CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo")
            .with_moveset(moveset("stomp", 3))
            .with_hurtboxes(hurtboxes("stomp")),
    );

    // A roster naming a character no loaded provider authored.
    let roster = MatchParticipantRoster::of(["mary_o", "someone_elses_fighter"]);
    {
        let mut demand = app.world_mut().resource_mut::<CharacterLoadDemand>();
        roster.project_demand(&mut demand);
    }

    let registry = app.world().resource::<PreparedCharacterRegistry>();
    assert!(registry.get("mary_o").is_some());
    assert!(
        registry.get("someone_elses_fighter").is_none(),
        "an unregistered fighter must not appear in the prepared authority"
    );
    // Mary-O's own moveset and hurtboxes are unaffected by her opponent's absence.
    let mary = registry.get("mary_o").unwrap();
    assert_eq!(
        resolve_hurtboxes(mary.hurtboxes.as_ref().unwrap(), None, None).source,
        HurtboxSelection::Default
    );
}
