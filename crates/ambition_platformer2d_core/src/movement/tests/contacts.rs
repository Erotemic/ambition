
use super::*;
use crate::collision_semantics::ContactSource;
use crate::world::BlockKind;
use crate::AbilitySet;

#[test]
fn grounded_player_step_reports_a_feet_contact() {
    let world = test_world();
    let mut scratch = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        world.spawn,
        AbilitySet::sandbox_all(),
    );
    // Settle onto the floor, then read one grounded frame.
    let mut last = FrameEvents::default();
    for _ in 0..30 {
        last = step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(scratch.ground.on_ground, "settled on the floor");
    let feet = last
        .contacts
        .iter()
        .find(|c| (c.normal - Vec2::new(0.0, -1.0)).length() < 1e-3)
        .unwrap_or_else(|| panic!("a feet contact with an up normal, got {:?}", last.contacts));
    let ContactSource::Block { kind, id } = &feet.source else {
        panic!("a feet contact against a block, got {:?}", feet.source);
    };
    assert_eq!(*kind, BlockKind::Solid);
    assert!(
        world.blocks.iter().any(|b| &b.id == id),
        "the contact carries a real world block's geometry id"
    );
    // The floor is static: no frame motion on the contact.
    assert_eq!(feet.surface_velocity, Vec2::ZERO);
    // The contact point sits on the floor's support face.
    assert!((feet.point.y - (world.size.y - 48.0)).abs() < 1.0);
}

#[test]
fn running_into_a_wall_reports_a_side_contact_with_the_surface_normal() {
    let world = test_world();
    let mut scratch = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        world.spawn,
        AbilitySet::sandbox_all(),
    );
    let mut saw_wall_contact = false;
    for _ in 0..240 {
        let events = step_scratch(
            &world,
            &mut scratch,
            InputState {
                axes: crate::LocalAxes::new(-1.0, 0.0),
                ..Default::default()
            },
        );
        // Left wall's outward (rightward) normal, pointing back at the body.
        if events
            .contacts
            .iter()
            .any(|c| (c.normal - Vec2::new(1.0, 0.0)).length() < 1e-3)
        {
            saw_wall_contact = true;
            break;
        }
    }
    assert!(
        saw_wall_contact,
        "running left eventually reports the left wall's side contact"
    );
}

/// **A rising head into a hidden block is a CONTACT, not just a stop.**
///
/// **it was only a stop.** `BlockKind::BonkOnly` exists to be solid against a
/// rising head and air to everything else, and the swept resolution truncated
/// the motion correctly — but then took a `BonkOnly`-only arm of an `if / else
/// if` chain whose comment claimed it "falls through to the ordinary face
/// resolution below". It does not; the head-contact arm was skipped. So the body
/// stopped dead under an invisible block and `FrameEvents.contacts` was empty,
/// which is indistinguishable from never having touched it. Mary-O's hidden
/// block in 1-2 could not be bonked at all.
///
/// **both terms are OBSERVED**: her head is asserted to have actually reached
/// the block's underside before the contact is demanded, so a jump that fell
/// short accuses the probe rather than the engine.
#[test]
fn rising_into_a_bonk_only_block_reports_a_head_contact() {
    use crate::collision_semantics::ContactKind;
    use crate::{ActionEdges, Edge, MovementAction};

    fn jump() -> InputState {
        InputState {
            movement: ActionEdges::EMPTY.with(
                MovementAction::Jump,
                Edge {
                    pressed: true,
                    held: true,
                    released: false,
                },
            ),
            ..Default::default()
        }
    }

    // Where her head sits when standing, measured rather than assumed — the
    // block is then placed relative to the BODY, so retuning her size or the
    // floor cannot silently move this test's subject out of reach.
    let base = test_world();
    let mut settle = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        base.spawn,
        AbilitySet::sandbox_all(),
    );
    for _ in 0..30 {
        step_scratch(&base, &mut settle, InputState::default());
    }
    let standing_head = settle.kinematics.pos.y - settle.kinematics.size.y * 0.5;

    // A hidden block a comfortable hop above her head, spanning her column.
    let underside = standing_head - 40.0;
    let mut world = test_world();
    world.blocks.push(crate::world::Block {
        kind: BlockKind::BonkOnly,
        ..crate::world::Block::solid(
            "hidden",
            Vec2::new(base.spawn.x - 48.0, underside - 32.0),
            Vec2::new(96.0, 32.0),
        )
    });

    let mut scratch = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        world.spawn,
        AbilitySet::sandbox_all(),
    );
    for _ in 0..30 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(
        scratch.ground.on_ground,
        "she has to be standing before the jump means anything"
    );

    let mut head_kind = None;
    let mut stood_on_it = false;
    let mut peak = f32::INFINITY;
    for _ in 0..90 {
        let events = step_scratch(&world, &mut scratch, jump());
        peak = peak.min(scratch.kinematics.pos.y - scratch.kinematics.size.y * 0.5);
        for contact in &events.contacts {
            let ContactSource::Block { kind, .. } = &contact.source else {
                continue;
            };
            if *kind != BlockKind::BonkOnly {
                continue;
            }
            match contact.kind {
                ContactKind::Head => head_kind = Some(contact.normal),
                // A hidden block must not become standing support when its head contact is fixed.
                ContactKind::Support => stood_on_it = true,
                _ => {}
            }
        }
        if head_kind.is_some() {
            break;
        }
    }

    assert!(
        peak <= underside + 1.0,
        "the jump never reached the block, so this proves nothing about the \
         contact: her head peaked at y={peak} and the underside is at \
         y={underside} (y grows downward)",
    );
    assert!(
        head_kind.is_some(),
        "her head was stopped by the hidden block and no Head contact was \
         reported — the block is solid to the sweep and invisible to every \
         consumer of `FrameEvents.contacts`",
    );
    assert!(
        !stood_on_it,
        "the hidden block reported a SUPPORT contact — it became a floor, which \
         is the thing `BonkOnly` exists to prevent",
    );
}

/// **A BODY HELD AGAINST GEOMETRY MUST NOT RE-LAND EVERY TICK.**
///
/// an sfx flurry that is loud and off-putting… I think the pirate sky issue is
/// collision into the ceiling causing the sfx"* — and, on the grab: *"I don't
/// think it's the grab with the noise, I think it causes the noise via world
/// interaction."* Two reports, one shape: a body **held against geometry** by
/// something other than its own motion — a flyer pressed into a ceiling, a
/// captive whose position is forced by its captor.
///
/// this asserts a CONTACT-STATE property, not an audio one, because that is
/// where the fault would be: the emitter is correct to voice a landing, so the
/// only wrong thing available is a landing that did not happen.
#[test]
fn a_body_pressed_into_the_ceiling_does_not_re_land_every_tick() {
    let world = test_world();
    let mut scratch = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        Vec2::new(210.0, 200.0),
        AbilitySet::sandbox_all(),
    );
    // Fly, and hold "up" into the ceiling for two seconds of ticks.
    scratch.flight.fly_enabled = true;
    let up = InputState {
        axes: crate::LocalAxes::new(0.0, -1.0),
        ..Default::default()
    };

    let mut landings = 0usize;
    let mut head_contacts = 0usize;
    for _ in 0..120 {
        let events = step_scratch(&world, &mut scratch, up);
        if matches!(
            events.ground_contact,
            crate::movement::GroundContactTransition::Landed { .. }
        ) {
            landings += 1;
        }
        if events
            .contacts
            .iter()
            .any(|c| c.kind == crate::collision_semantics::ContactKind::Head)
        {
            head_contacts += 1;
        }
    }

    // **the zero floor.** A run that never reached the ceiling would report
    // zero landings and pass while measuring nothing at all.
    assert!(
        head_contacts > 0,
        "the body never touched the ceiling in 120 ticks, so this measured nothing"
    );
    assert_eq!(
        landings, 0,
        "a body pressed into a CEILING registered {landings} landing(s) in 120 \
         ticks — every one of them writes an `SfxMessage::Land`, and nothing \
         downstream caps or dedupes voices"
    );
}

/// **A BODY WHOSE POSITION IS WRITTEN BY ANOTHER BODY MUST NOT RE-LAND EVERY
/// TICK EITHER.**
///
/// What the pirate sky and the grab have in common is a body being *carried*:
/// `pirate_sky_lookout`'s riders are mounted on sharks and a captive's pose is written by its
/// captor, so in both cases something other than the body's own motion decides where it is, every
/// tick.
///
/// The fixture is that forcing, reduced to its essential: re-place the body on
/// the floor line every tick and step it normally, which is exactly what a
/// carrier does to a carried body.
#[test]
fn a_body_whose_pose_is_written_each_tick_does_not_re_land_each_tick() {
    let world = test_world();
    let mut scratch = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        world.spawn,
        AbilitySet::sandbox_all(),
    );
    // Settle first, so the baseline is a body that is genuinely grounded.
    for _ in 0..30 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(scratch.ground.on_ground, "the fixture starts grounded");
    let carried_at = scratch.kinematics.pos;

    let mut landings = 0usize;
    for _ in 0..120 {
        // THE CARRY: another body decides where this one is, every tick.
        scratch.kinematics.pos = carried_at;
        let events = step_scratch(&world, &mut scratch, InputState::default());
        if matches!(
            events.ground_contact,
            crate::movement::GroundContactTransition::Landed { .. }
        ) {
            landings += 1;
        }
    }
    assert_eq!(
        landings, 0,
        "a carried body re-landed {landings} time(s) in 120 ticks — each one \
         writes an `SfxMessage::Land`, and with four riders in a room that is a \
         hundred voices a second of one cue"
    );
}
