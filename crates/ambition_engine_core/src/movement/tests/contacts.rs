//! The contact vocabulary through the PLAYER sweep (fable review 2026-07-05
//! AJ10 / R8.1): `FrameEvents.contacts` reports what the body touched, with
//! surface-outward normals, without changing resolution.

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
    assert_eq!(
        feet.source,
        ContactSource::Block {
            kind: BlockKind::Solid
        }
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
                axis_x: -1.0,
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
