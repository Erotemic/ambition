//! Contact attacks, published envelopes and motion records use one physical body pose.

use super::*;
use ambition_body_seed::clusters::SeedActorMut;
use ambition_body_seed::{ActorBody, ActorClusterSeed};
use ambition_characters::actor::control::ActorControlFrame;
use ambition_entity_catalog::placements::CharacterBrain;

const BODY: ae::Vec2 = ae::Vec2::new(48.0, 22.0);

/// What one airborne step in gravity toward `down` gives.
struct Stepped {
    /// The half extents of the contact attack's volume.
    contact_half: ae::Vec2,
    contact_centre: ae::Vec2,
    /// The footprint, as `update_actor_body` publishes it.
    footprint: ae::CenteredAabb,
    /// The box the movement step moved.
    step: ae::Aabb,
    normal: ae::Vec2,
    pos: ae::Vec2,
}

fn step_in_gravity(sprite_content: &ambition_sprite_sheet::SpriteContent, down: ae::Vec2) -> Stepped {
    let material = ae::frame::BodyMaterial::new(1.0, bevy::math::Mat2::from_cols(
        ae::Vec2::new(down.y, -down.x), down,
    )).unwrap();
    step_with_material(sprite_content, down, material)
}

fn step_with_material(
    sprite_content: &ambition_sprite_sheet::SpriteContent,
    down: ae::Vec2,
    material: ae::frame::BodyMaterial,
) -> Stepped {
    let world = ae::World::new("body_box_test", ae::Vec2::new(4000.0, 4000.0), ae::Vec2::ZERO, Vec::new());
    let mut seed = ActorClusterSeed::new(&sprite_content,
        "slug".to_string(),
        "Slug".to_string(),
        ae::Aabb::new(ae::Vec2::new(2000.0, 2000.0), BODY * 0.5),
        CharacterBrain::Passive,
        &[],
    );
    assert_eq!(seed.kin.size(), BODY, "premise: the body is not square");
    seed.kin.material = material;
    seed.kin.vel = ae::Vec2::ZERO;
    seed.surface.gravity_scale = 1.0;
    seed.body = ActorBody::from_abilities(ae::AbilitySet::classic_actor(), false, seed.kin.size());
    seed.config.tuning.body_contact_damage = true;

    let mut model = ambition_platformer2d_core::movement::MotionModel::default();
    let mut combat = ambition_characters::actor::BodyCombat::default();
    let mut sample = ae::SweepSample::default();
    let mut em = seed.as_actor_mut();
    em.sweep = Some(&mut sample);
    em.update(
        &world,
        FeatureCombatTuning::default(),
        1.0 / 60.0,
        false,
        ActorControlFrame::neutral(),
        &mut model,
        ae::MotionFrame::from_direction(down, ae::GRAVITY),
        None,
        ambition_combat::feel::Platformer2dFeelTuningMonolith::default(),
        None,
        &mut combat,
        false,
        false,
        ae::BodyContactField::NONE,
        None,
    );
    let attack = super::ContactAttack::from_body(em.kin, em.surface, &em.config.tuning).expect("premise: the body has a contact attack");
    let mut footprint = ae::CenteredAabb::from_center_size(em.kin.pos, em.kin.size());
    ambition_combat::body_geometry::publish_body_footprint(
        &mut footprint,
        em.kin,
        em.kin.canonical_size,
    );
    let normal = em.surface.surface_normal;
    let pos = em.kin.pos;
    Stepped {
        contact_half: (attack.volume.max - attack.volume.min) * 0.5,
        contact_centre: (attack.volume.max + attack.volume.min) * 0.5,
        footprint,
        step: sample.end_aabb(),
        normal,
        pos,
    }
}

#[track_caller]
fn the_three_boxes_are_one(sprite_content: &ambition_sprite_sheet::SpriteContent, down: ae::Vec2, expected_half: ae::Vec2) {
    let down = down.normalize();
    let stepped = step_in_gravity(&sprite_content, down);
    assert!(
        (stepped.normal + down).length() < 1.0e-4,
        "premise: an airborne body publishes the normal opposite its gravity: {:?}",
        stepped.normal
    );
    assert!(
        (stepped.footprint.half_size - expected_half).abs().max_element() < 0.05,
        "premise: the footprint of a 48x22 body with DOWN {down:?} has the half {expected_half:?}: {:?}",
        stepped.footprint.half_size
    );
    let step_half = (stepped.step.max - stepped.step.min) * 0.5;
    assert!(
        (step_half - stepped.footprint.half_size).abs().max_element() < 1.0e-3,
        "premise: the box the step moved is the footprint: {step_half:?} and {:?}",
        stepped.footprint.half_size
    );
    assert!((stepped.contact_centre - stepped.pos).length() < 1.0e-3);
    assert!(
        (stepped.contact_half - stepped.footprint.half_size).abs().max_element() < 1.0e-3,
        "with DOWN {down:?} the contact attack has the half {:?} and the footprint has {:?}: a body has one box",
        stepped.contact_half,
        stepped.footprint.half_size
    );
}

/// 0.6 to the side is more than the 30 degrees of the old swap: it gave the
/// box of a body on a wall, 22 by 48.
#[test]
fn a_body_in_steep_diagonal_gravity_attacks_with_its_footprint() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    the_three_boxes_are_one(&sprite_content, ae::Vec2::new(0.6, 0.8), ae::Vec2::new(25.8, 23.2));
}

/// 0.3 to the side is less: the old swap gave the level box, 48 by 22.
#[test]
fn a_body_in_shallow_diagonal_gravity_attacks_with_its_footprint() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let down = ae::Vec2::new(0.3, 0.954).normalize();
    let expected = ae::Vec2::new(
        down.y * BODY.x * 0.5 + down.x * BODY.y * 0.5,
        down.x * BODY.x * 0.5 + down.y * BODY.y * 0.5,
    );
    assert!((expected - ae::Vec2::new(26.2, 17.7)).abs().max_element() < 0.1, "{expected:?}");
    the_three_boxes_are_one(&sprite_content, down, expected);
}

/// The control: for a cardinal DOWN the swap and the rule were equal.
#[test]
fn a_body_in_cardinal_gravity_attacks_with_its_footprint() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    the_three_boxes_are_one(&sprite_content, ae::Vec2::new(1.0, 0.0), ae::Vec2::new(11.0, 24.0));
    the_three_boxes_are_one(&sprite_content, ae::Vec2::new(0.0, 1.0), ae::Vec2::new(24.0, 11.0));
}

/// A gravity change does not replace the physical pose during admitted righting.
#[test]
fn contact_motion_and_published_envelopes_agree_during_righting() {
    let sprites = ambition_sprite_sheet::test_support::shared_sprites();
    let stepped = step_with_material(&sprites, ae::Vec2::X, ae::frame::BodyMaterial::IDENTITY);
    let half = (stepped.step.max - stepped.step.min) * 0.5;
    assert!((half - stepped.footprint.half_size).length() < 0.001);
    assert!((stepped.contact_half - stepped.footprint.half_size).length() < 0.001);
    assert!(half.y > BODY.y * 0.5 && half.y < BODY.x * 0.5);
}
