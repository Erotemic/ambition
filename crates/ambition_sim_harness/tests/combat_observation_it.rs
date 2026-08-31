//! The observation, staged through a real world rather than a hand-built row.
//!
//! ⛔ THESE LIVE OUT HERE ON PURPOSE. They publish `DamageableVolumes` to stage a
//! body that is deliberately unhittable, and `check_absence_contracts.py` keeps
//! that type out of `combat_observation.rs` — the module must READ the runtime's
//! damageable rule through `CombatGeometryView`, never apply it. A fixture that
//! stages a world is not a second resolver, but the checker cannot tell the
//! difference and should not have to: the rule is about the module.

use ambition_platformer2d::engine_core as ae;
use ambition_sim_harness::combat_observation::{CombatObservation, ScenarioRoles};

/// The whole road, end to end: two seated bodies, the real read model, and
/// the artifact that comes out of it.
///
/// ⛔⛔ THE ARTIFACT MUST BE READABLE WITHOUT KNOWING A SEAT CONVENTION.
/// This scenario seats the SAME character twice on purpose, which is what
/// the recorder does — so an identity, a colour or a character id cannot say
/// which fighter the move belongs to, and only the role can.
#[test]
fn a_seated_scenario_serializes_roles_identities_and_both_geometries() {
    use ambition_platformer2d::actor::{BodyCombat, MatchSeat};
    use ambition_platformer2d::combat::components::{CenteredAabb, DamageableVolumes};
    use ambition_platformer2d::combat::strike::{
        HitSide, Hitbox, HitboxAnchor, HitboxHits, HitboxKnockback,
    };
    use ambition_platformer2d::platformer::sim_id::SimId;
    use ambition_platformer2d::sim_view::CombatGeometryView;
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<CombatGeometryView>();
    app.add_systems(
        Update,
        ambition_platformer2d::sim_view::rebuild_combat_geometry_view,
    );

    let seat = |app: &mut App, index: usize, x: f32, published: bool| {
        let centre = ae::Vec2::new(x, 100.0);
        let collision = ae::Aabb::new(centre, ae::Vec2::new(10.0, 20.0));
        let mut body = app.world_mut().spawn((
            MatchSeat(index),
            CenteredAabb::from_aabb(collision),
            BodyCombat::default(),
            SimId::placement(&format!("fighter#seat{index}")),
        ));
        if published {
            body.insert(DamageableVolumes::single(ae::Aabb::new(
                centre,
                ae::Vec2::new(8.0, 18.0),
            )));
        }
        body.id()
    };
    let subject = seat(&mut app, 0, 100.0, true);
    let target = seat(&mut app, 1, 130.0, true);

    // The subject's swing, which has already connected with the target.
    app.world_mut().spawn((
        Hitbox {
            owner: subject,
            source: HitSide::Player,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::new(20.0, 0.0),
            },
            half_extent: ae::Vec2::new(12.0, 6.0),
            shape: None,
            facing: 1.0,
            damage: 7,
            knockback: HitboxKnockback::FeelScale(1.0),
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
            strike_sfx: None,
            reaction: None,
        },
        HitboxHits {
            hit: std::iter::once(target).collect(),
        },
        SimId::from_snapshot("strike#1".to_string()),
    ));

    app.update();
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

    // ⛔⛔ AND THE CONTACT IS THE RUNTIME'S ANSWER, not an overlap test run
    // here: it comes from the resolver's own hit-once memory.
    let contacts = doc["contacts"].as_array().expect("contacts serialize");
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0]["victim"], "placement:fighter#seat1");
    assert_eq!(contacts[0]["victim_role"], "target");
    assert_eq!(contacts[0]["owner_role"], "subject_owned");
}

/// ⛔ A PUBLISHED-EMPTY DAMAGEABLE LIST PRODUCES NO HURTBOX, and says why.
#[test]
fn an_intangible_body_publishes_no_hurtbox_and_names_the_reason() {
    use ambition_platformer2d::actor::{BodyCombat, MatchSeat};
    use ambition_platformer2d::combat::components::{CenteredAabb, DamageableVolumes};
    use ambition_platformer2d::platformer::sim_id::SimId;
    use ambition_platformer2d::sim_view::CombatGeometryView;
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<CombatGeometryView>();
    app.add_systems(
        Update,
        ambition_platformer2d::sim_view::rebuild_combat_geometry_view,
    );
    let collision = ae::Aabb::new(ae::Vec2::new(10.0, 10.0), ae::Vec2::new(6.0, 12.0));
    let mut intangible = DamageableVolumes::default();
    intangible.clear();
    app.world_mut().spawn((
        MatchSeat(0),
        CenteredAabb::from_aabb(collision),
        BodyCombat::default(),
        intangible,
        SimId::placement("dodging#seat0"),
    ));
    // Nothing published at all: the coarse fallback, which is a DIFFERENT
    // fact from being deliberately unhittable.
    app.world_mut().spawn((
        MatchSeat(1),
        CenteredAabb::from_aabb(collision),
        BodyCombat::default(),
        SimId::placement("ordinary#seat1"),
    ));

    app.update();
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
