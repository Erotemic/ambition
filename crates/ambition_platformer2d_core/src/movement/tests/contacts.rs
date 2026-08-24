
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

/// A rising head into a `BonkOnly` block must both stop and emit a head contact.
/// The fixture separately proves that the body reached the block underside so a
/// short jump cannot satisfy the contact assertion vacuously.
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

/// A BODY HELD AGAINST GEOMETRY MUST NOT RE-LAND EVERY TICK.
///
/// an sfx flurry that is loud and off-putting… I think the pirate sky issue is
/// collision into the ceiling causing the sfx"* — and, on the grab: *"I don't
/// think it's the grab with the noise, I think it causes the noise via world
/// interaction."* Two reports, one shape: a body held against geometry by
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

    // the zero floor. A run that never reached the ceiling would report
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

/// A BODY WHOSE POSITION IS WRITTEN BY ANOTHER BODY MUST NOT RE-LAND EVERY
/// TICK EITHER.
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

/// ⛔⛔ **A CARRIER THAT CLEARS THE GROUND FLAG MUST CLEAR THE BASELINE WITH IT.**
///
/// Jon, 2026-08-22: *"Grabbing a character still pushes them into the ground
/// causing sfx to play repeatedly."* Measured: **120 landings in 120 ticks.**
///
/// ⭐⭐ **the two carry tests above miss it by one variable, and that is the
/// lesson.** Both re-place the body every tick and both pass, because neither
/// touches `on_ground` — and the real carriers do. The captive hold
/// and the mount's saddle pin each wrote `on_ground = false` beside the pose,
/// which does not say *"this body is being carried"*, it says **"this body was
/// AIRBORNE last tick"**. The kernel then samples support at the forced pose,
/// reads `false -> true`, and reports a LANDING: an `SfxMessage::Land` and a
/// dust puff at the feet, every tick, for the length of the grab.
///
/// `BodyGroundState::invalidate()` is the call that already meant this — *"after
/// a discrete pose change … the next movement step establishes a new baseline"*
/// — and a carry is a discrete pose change happening every tick. A carried body
/// has no contact HISTORY to transition from, which is a different claim from
/// having been in the air.
#[test]
fn a_carrier_that_clears_the_ground_flag_does_not_re_land_the_body_each_tick() {
    let world = test_world();
    let mut scratch = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        world.spawn,
        AbilitySet::sandbox_all(),
    );
    for _ in 0..30 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(scratch.ground.on_ground, "the fixture starts grounded");
    let carried_at = scratch.kinematics.pos;

    let mut landings = 0usize;
    let mut grounded_samples = 0usize;
    for _ in 0..120 {
        // THE CARRY, both halves of it, exactly as a captor and a mount write it.
        scratch.kinematics.pos = carried_at;
        scratch.ground.invalidate();
        let events = step_scratch(&world, &mut scratch, InputState::default());
        if matches!(
            events.ground_contact,
            crate::movement::GroundContactTransition::Landed { .. }
        ) {
            landings += 1;
        }
        if scratch.ground.on_ground {
            grounded_samples += 1;
        }
    }

    //  the zero floor. The hold is ON the floor line, so every tick DOES
    // sample support — a run that never touched geometry would report zero
    // landings while measuring nothing, which is how the two tests above missed
    // this in the first place.
    assert!(
        grounded_samples > 0,
        "the carried body never touched the floor, so this measured nothing"
    );
    assert_eq!(
        landings, 0,
        "a carried body re-landed {landings} time(s) in 120 ticks — with \
         `on_ground = false` instead of `invalidate()` this is 120, one \
         `SfxMessage::Land` and one dust puff per tick for the whole grab"
    );
}

/// ⭐ A CRASH IS NOT A LANDING, and the two facts that say so have to arrive in
/// the SAME bundle.
///
/// A splat built on `MovementOp::Knockdown` scaled by the landing impact reads
/// zero forever: `tick_knockdown` runs in the CONTROL phase, which precedes
/// integration, so it sees `on_ground` only the tick after touchdown and is
/// never in the same `FrameEvents` as the impact speed that measured the fall.
/// `Landed` carries both, which is what makes the pair readable at all.
#[test]
fn a_body_that_falls_out_of_a_launch_lands_involuntarily_and_says_how_hard() {
    let world = test_world();
    let mut scratch = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        world.spawn,
        AbilitySet::sandbox_all(),
    );
    // Airborne, thrown, and still tumbling on the way down.
    scratch.kinematics.pos = Vec2::new(300.0, 400.0);
    scratch.ground.on_ground = false;
    scratch.ground.contact_initialized = true;
    // The state a hard launch leaves behind: still tumbling, and it is the
    // LANDING that ends it.
    scratch.axis_mut().tumble_until_landing = true;
    scratch.axis_mut().tumble_timer = 0.25;

    let mut landing = None;
    for _ in 0..240 {
        let events = step_scratch(&world, &mut scratch, InputState::default());
        if let crate::GroundContactTransition::Landed {
            impact_speed,
            involuntary,
        } = events.ground_contact
        {
            landing = Some((impact_speed, involuntary));
            break;
        }
    }
    let (impact_speed, involuntary) = landing.expect("the thrown body never reached the floor");
    assert!(
        involuntary,
        "a body still falling out of a launch reported a landing it chose"
    );
    assert!(
        impact_speed > 0.0,
        "the crash carried no impact speed, so a splat scaled by it draws \
         nothing — this is the phase trap the fact exists to close"
    );
}

/// The ordinary case, so the flag is not simply always true: a body that jumped
/// and came down chose to be there.
#[test]
fn an_ordinary_landing_is_not_involuntary() {
    let world = test_world();
    let mut scratch = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        world.spawn,
        AbilitySet::sandbox_all(),
    );
    scratch.kinematics.pos = Vec2::new(300.0, 400.0);
    scratch.ground.on_ground = false;
    scratch.ground.contact_initialized = true;

    for _ in 0..240 {
        let events = step_scratch(&world, &mut scratch, InputState::default());
        if let crate::GroundContactTransition::Landed { involuntary, .. } = events.ground_contact {
            assert!(!involuntary, "a plain fall reported itself as a crash");
            return;
        }
    }
    panic!("the falling body never reached the floor");
}

/// ⭐ THE WALL HIT, measured. Nothing downstream could recover this: the step
/// zeroes the body's velocity along the contact axis as it resolves, so a body
/// already stopped against a wall looks the same however hard it arrived.
#[test]
fn a_side_contact_carries_the_speed_the_body_arrived_at() {
    let world = test_world();
    let mut scratch = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        world.spawn,
        AbilitySet::sandbox_all(),
    );
    // Just clear of the right wall, driven into it hard.
    scratch.kinematics.pos = Vec2::new(1500.0, world.size.y - 95.0);
    scratch.ground.contact_initialized = true;
    scratch.kinematics.vel = Vec2::new(1200.0, 0.0);

    let mut hit = None;
    for _ in 0..120 {
        let events = step_scratch(&world, &mut scratch, InputState::default());
        if let Some(side) = events
            .contacts
            .iter()
            .find(|c| c.kind == crate::collision_semantics::ContactKind::Side)
        {
            hit = Some(side.impact_speed);
            break;
        }
        // Keep driving: the step's own friction is not the subject.
        scratch.kinematics.vel.x = 1200.0;
    }
    let impact = hit.expect("the body never reached the wall");
    assert!(
        impact > 100.0,
        "the wall contact reported {impact} px/s of approach for a body \
         travelling at 1200 — the speed is being read after the step destroyed it"
    );
}

/// ⭐ A CRASH INTO A WALL IS NOT A COMMUTE INTO ONE, and the contact has to say
/// so on its own.
///
/// A body thrown into a wall and one that dashed into it arrive with the same
/// normal, the same point and possibly the same speed. `Landed` already carries
/// this distinction for the floor; this is the same fact on the other surface,
/// and without it a crash cue has nothing to gate on but speed.
#[test]
fn a_wall_contact_says_whether_the_body_chose_to_arrive() {
    let world = test_world();

    // DROVE into it: a body under its own power.
    let mut driven = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        Vec2::new(1500.0, world.size.y - 95.0),
        AbilitySet::sandbox_all(),
    );
    driven.ground.contact_initialized = true;
    let mut chosen = None;
    for _ in 0..120 {
        driven.kinematics.vel.x = 1200.0;
        let events = step_scratch(&world, &mut driven, InputState::default());
        if let Some(side) = events
            .contacts
            .iter()
            .find(|c| c.kind == crate::collision_semantics::ContactKind::Side)
        {
            chosen = Some(side.involuntary);
            break;
        }
    }
    assert_eq!(
        chosen,
        Some(false),
        "a body that drove into a wall reported a crash"
    );

    // THROWN into it: the same wall, the same speed, still tumbling.
    let mut thrown = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        Vec2::new(1500.0, world.size.y - 300.0),
        AbilitySet::sandbox_all(),
    );
    thrown.ground.contact_initialized = true;
    thrown.ground.on_ground = false;
    thrown.axis_mut().tumble_until_landing = true;
    thrown.axis_mut().tumble_timer = 0.5;
    let mut crashed = None;
    for _ in 0..120 {
        thrown.kinematics.vel.x = 1200.0;
        let events = step_scratch(&world, &mut thrown, InputState::default());
        if let Some(side) = events
            .contacts
            .iter()
            .find(|c| c.kind == crate::collision_semantics::ContactKind::Side)
        {
            crashed = Some(side.involuntary);
            break;
        }
    }
    assert_eq!(
        crashed,
        Some(true),
        "a body still falling out of a launch hit the wall and reported a \
         commute, so a splat has nothing to gate on but speed"
    );
}
