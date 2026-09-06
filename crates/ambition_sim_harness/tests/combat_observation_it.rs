//! The artifact `CombatObservation` writes, from a view and a world of identities.
//!
//! ⛔ IT BUILDS THE VIEW BY HAND, ON PURPOSE. Whether `CombatGeometryView`
//! resolves the runtime's three-way damageable rule correctly is
//! `ambition_sim_view`'s invariant and is tested there, against real
//! `DamageableVolumes`. What this crate owns is the SERIALIZATION and the
//! identity/role join — so staging combat components here would test the other
//! crate's job through this one, and would need authority types the observation
//! surface deliberately does not expose.

use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::observation::{
    Aabb, CombatBodyGeometryView, CombatGeometryView, CombatStrikeGeometryView, CombatVolume,
    HurtboxSource, SimId, Vec2,
};
use ambition_sim_harness::combat_observation::{CombatObservation, ScenarioRoles};
use bevy::prelude::*;

fn body(entity: Entity, at: Vec2, hurt: bool) -> CombatBodyGeometryView {
    let collision = Aabb::new(at, Vec2::new(10.0, 20.0));
    CombatBodyGeometryView {
        body: entity,
        collision,
        hurtboxes: if hurt {
            vec![CombatVolume::aabb(Aabb::new(at, Vec2::new(8.0, 18.0)))]
        } else {
            Vec::new()
        },
        hurtbox_source: if hurt {
            HurtboxSource::Published
        } else {
            HurtboxSource::Intangible
        },
        damage_taken: 0,
        facing: 1.0,
        hitstun_s: 0.0,
        hitlag_s: 0.0,
        landing_lag_s: 0.0,
        jump_squat_s: 0.0,
        velocity: Vec2::ZERO,
        grounded: true,
        on_wall: false,
        wall_normal_x: 0.0,
        move_state: None,
    }
}

/// Two seated fighters, a strike, and the artifact that comes out.
///
/// ⛔⛔ THE ARTIFACT MUST BE READABLE WITHOUT KNOWING A SEAT CONVENTION. This
/// scenario seats the SAME character twice on purpose, which is what the
/// recorder does — so an identity, a colour or a character id cannot say which
/// fighter the move belongs to, and only the role can.
#[test]
fn a_seated_scenario_serializes_roles_identities_and_both_geometries() {
    let mut app = App::new();
    let seat = |app: &mut App, index: usize| {
        app.world_mut()
            .spawn((
                MatchSeat(index),
                SimId::placement(&format!("fighter#seat{index}")),
            ))
            .id()
    };
    let subject = seat(&mut app, 0);
    let target = seat(&mut app, 1);
    let strike = app
        .world_mut()
        .spawn(SimId::from_snapshot("strike#1".to_string()))
        .id();

    app.insert_resource(CombatGeometryView {
        bodies: vec![
            body(subject, Vec2::new(100.0, 100.0), true),
            body(target, Vec2::new(130.0, 100.0), true),
        ],
        strikes: vec![CombatStrikeGeometryView {
            volume: CombatVolume::aabb(Aabb::new(Vec2::new(120.0, 100.0), Vec2::new(12.0, 6.0))),
            strike,
            owner: subject,
            damage: 7,
            anchored_to_body: true,
            // The runtime's own hit-once memory: this strike HAS connected.
            hit: vec![target],
        }],
    });

    let scenario = ScenarioRoles::from_seats(app.world_mut(), 0, 1);
    assert_eq!(scenario.subject(), Some(subject));
    assert_eq!(scenario.target(), Some(target));
    let roles = scenario.resolve(app.world_mut());
    let doc = CombatObservation::capture(app.world_mut(), &roles).to_json();

    let bodies = doc["bodies"].as_array().expect("bodies serialize");
    assert_eq!(bodies.len(), 2);
    let by_role = |role: &str| {
        bodies
            .iter()
            .find(|b| b["role"] == role)
            .unwrap_or_else(|| panic!("no body with role {role}"))
    };
    assert_eq!(by_role("subject")["id"], "placement:fighter#seat0");
    assert_eq!(by_role("target")["id"], "placement:fighter#seat1");
    // BOTH halves of the interaction are in the artifact.
    assert_eq!(by_role("target")["hurtboxes"].as_array().map(Vec::len), Some(1));
    assert_eq!(by_role("target")["hurtbox_source"], "published");

    let strikes = doc["strikes"].as_array().expect("strikes serialize");
    assert_eq!(strikes.len(), 1);
    // ⛔ A SWING IS ITS OWNER'S SIDE, not its owner: `subject_owned`, never
    // `subject`.
    assert_eq!(strikes[0]["role"], "subject_owned");
    assert_eq!(strikes[0]["subject_owned"], true);
    assert_eq!(strikes[0]["damage"], 7);
    assert_eq!(strikes[0]["owner_id"], "placement:fighter#seat0");

    // ⛔⛔ AND THE CONTACT IS THE RUNTIME'S ANSWER, not an overlap test run here.
    let contacts = doc["contacts"].as_array().expect("contacts serialize");
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0]["victim"], "placement:fighter#seat1");
    assert_eq!(contacts[0]["victim_role"], "target");
    assert_eq!(contacts[0]["owner_role"], "subject_owned");
}

/// ⛔ AN EMPTY HURTBOX LIST IS A DECISION, AND THE ARTIFACT SAYS WHICH ONE.
///
/// A body nothing published for falls back to its coarse box; a body mid-dodge
/// publishes nothing ON PURPOSE. Both reach a reader as "no volumes", and an
/// inspector that could not tell them apart would report a bug for a rule.
#[test]
fn the_artifact_distinguishes_intangible_from_a_coarse_fallback() {
    let mut app = App::new();
    let dodging = app
        .world_mut()
        .spawn((MatchSeat(0), SimId::placement("dodging#seat0")))
        .id();
    let ordinary = app
        .world_mut()
        .spawn((MatchSeat(1), SimId::placement("ordinary#seat1")))
        .id();

    let mut fallback = body(ordinary, Vec2::new(130.0, 100.0), true);
    fallback.hurtbox_source = HurtboxSource::BodyFallback;
    app.insert_resource(CombatGeometryView {
        bodies: vec![body(dodging, Vec2::new(100.0, 100.0), false), fallback],
        strikes: Vec::new(),
    });

    let scenario = ScenarioRoles::from_seats(app.world_mut(), 0, 1);
    let roles = scenario.resolve(app.world_mut());
    let doc = CombatObservation::capture(app.world_mut(), &roles).to_json();
    let bodies = doc["bodies"].as_array().expect("bodies serialize");
    let row = |role: &str| bodies.iter().find(|b| b["role"] == role).expect("role present");

    assert_eq!(row("subject")["hurtboxes"].as_array().map(Vec::len), Some(0));
    assert_eq!(row("subject")["hurtbox_source"], "intangible");
    assert_eq!(row("target")["hurtboxes"].as_array().map(Vec::len), Some(1));
    assert_eq!(row("target")["hurtbox_source"], "body_fallback");
}
