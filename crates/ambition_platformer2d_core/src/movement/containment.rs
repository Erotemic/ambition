//! Does this movement policy stay inside a room?
//!
//! A motion model and a level are each correct alone and can be broken together. That is not a
//! hypothesis: the surface-momentum solver had no horizontal collision on its riding arm for
//! the whole life of the project, and nothing noticed, because the only level it was ever
//! played in was a hand-authored chain course with nothing to run into.
//!
//! The instrument is embarrassingly cheap: put a body in a box, hold a
//! direction, check it is still in the box. What makes it worth having as a
//! library function rather than one test is that the interesting axis is the
//! POLICY, and the set of policies grows with the content — every character a
//! provider adds picks one, and a game built on this engine picks its own. A
//! game embedding the engine should be able to run this over its own cast.
//!
//! ## What this deliberately does not do
//!
//! It does not judge FEEL. How far the body gets, whether it accelerates
//! plausibly, whether the stop is soft or hard — none of that is here. This
//! answers one question that has exactly one right answer, so it can be run
//! over every policy without anyone tuning an expectation per character.

use crate::movement::kernel::{step_motion, MotionStepContext};
use crate::movement::{InputState, MotionModelSpec};
use crate::{Aabb, LocalAxes, MotionFrame, Vec2, World};

/// How to drive the body during a containment probe.
#[derive(Clone, Copy, Debug)]
pub struct ContainmentProbe {
    /// Held movement axes, in the body's local frame. `(1.0, 0.0)` is "hold
    /// right for the whole probe", which is the case that finds walls.
    pub axes: LocalAxes,
    pub steps: usize,
    pub dt: f32,
    /// The body's collision size. `None` uses the engine default, which is what
    /// a probe of a POLICY wants; a probe of a CHARACTER passes its authored
    /// size, because a wide body reaches a wall before a narrow one does and a
    /// tall one can clip a ceiling the default never touches.
    pub body_size: Option<Vec2>,
}

impl ContainmentProbe {
    /// Hold one direction long enough to cross any plausible room several
    /// times over — a probe that stops before the body reaches the wall
    /// passes for the wrong reason.
    pub fn holding(axes: LocalAxes) -> Self {
        Self {
            axes,
            steps: 900,
            dt: 1.0 / 60.0,
            body_size: None,
        }
    }

    /// Probe a specific body size — a CHARACTER rather than a policy.
    pub fn with_body_size(mut self, size: Vec2) -> Self {
        self.body_size = Some(size);
        self
    }
}

/// Where the body ended up, and whether it ever left `bounds`.
#[derive(Clone, Copy, Debug)]
pub struct ContainmentOutcome {
    pub final_pos: Vec2,
    /// The furthest any part of the body's BOX got outside `bounds`, in pixels.
    /// Zero means it never left.
    ///
    /// The box, not the centre. A centre-point test passes a body that is
    /// visibly half outside the room, which is the thing a player would call
    /// "he went through the wall" — and the bigger the character, the more of it
    /// can be outside before the test notices.
    pub max_escape_px: f32,
    /// True if the body was in contact with something at the end. A body that
    /// stopped because it hit a wall and one that stopped because it fell out
    /// of the world and is still falling are both "not moving right"; this
    /// tells them apart.
    pub grounded: bool,
}

impl ContainmentOutcome {
    pub fn contained(&self) -> bool {
        self.max_escape_px <= 0.0
    }
}

/// Run one body, under one movement policy, against one world.
///
/// `bounds` is the region the body is expected to stay inside — normally the
/// room's interior. The probe reports the worst excursion rather than a bool so
/// a failure message can say HOW far out it got, which is the difference
/// between "clipped a corner by a pixel" and "left the world".
pub fn probe_containment(
    world: &World,
    spec: MotionModelSpec,
    spawn: Vec2,
    bounds: Aabb,
    probe: ContainmentProbe,
) -> ContainmentOutcome {
    let mut scratch = crate::body_clusters::BodyClusterScratch::new_with_abilities(
        spawn,
        crate::AbilitySet::sandbox_all(),
    );
    if let Some(size) = probe.body_size {
        scratch.kinematics.size = size;
    }
    // The scratch OWNS its motion model, exactly like a real body does — so the
    // probe drives the same component the simulation drives, not a copy beside it.
    scratch.parts().0.apply_spec(spec);
    let frame = MotionFrame::from_acceleration(
        crate::movement::DEFAULT_GRAVITY_DIR * crate::movement::GRAVITY,
    )
    .expect("the default gravity is non-zero");

    let mut max_escape: f32 = 0.0;
    for _ in 0..probe.steps {
        let (model, mut clusters) = scratch.parts();
        step_motion(
            model,
            &mut clusters,
            MotionStepContext {
                world,
                input: InputState {
                    axes: probe.axes,
                    ..Default::default()
                },
                frame,
                facing_intent: probe.axes.x,
                dt: probe.dt,
                // Containment asks the kernel where a body FITS in its room; other
                // bodies are not part of that question.
                contact: crate::movement::body_contact::BodyContactField::NONE,
                pose_owned_externally: false,
                recovery_commitment_outstanding: false,
            },
        );
        let pos = scratch.kinematics.pos;
        let half = scratch.kinematics.size * 0.5;
        let escape = (bounds.min.x - (pos.x - half.x))
            .max((pos.x + half.x) - bounds.max.x)
            .max(bounds.min.y - (pos.y - half.y))
            .max((pos.y + half.y) - bounds.max.y);
        max_escape = max_escape.max(escape);
    }

    ContainmentOutcome {
        final_pos: scratch.kinematics.pos,
        max_escape_px: max_escape.max(0.0),
        grounded: scratch.ground.on_ground,
    }
}

/// A plain rectangular room: floor, ceiling, and a wall at each end.
///
/// The most boring level that can exist, and the one every policy has to
/// survive before it is usable in a level anybody authors.
pub fn walled_box(size: Vec2, wall_px: f32) -> World {
    World::new(
        "containment box",
        size,
        Vec2::new(size.x * 0.5, size.y - wall_px - 24.0),
        vec![
            crate::world::Block::solid(
                "box_floor",
                Vec2::new(0.0, size.y - wall_px),
                Vec2::new(size.x, wall_px),
            ),
            crate::world::Block::solid("box_ceiling", Vec2::ZERO, Vec2::new(size.x, wall_px)),
            crate::world::Block::solid("box_wall_left", Vec2::ZERO, Vec2::new(wall_px, size.y)),
            crate::world::Block::solid(
                "box_wall_right",
                Vec2::new(size.x - wall_px, 0.0),
                Vec2::new(wall_px, size.y),
            ),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The escape measure is the BOX, not the centre.
    ///
    /// A centre-point test passes a body that is visibly half outside the room —
    /// and the bigger the character, the more of it can be out before the test
    /// notices. This pins the difference directly: a body wider than the room's
    /// clear span reports an escape while its centre never leaves.
    #[test]
    fn a_body_wider_than_the_room_is_reported_even_with_its_centre_inside() {
        let size = Vec2::new(240.0, 240.0);
        let world = walled_box(size, 16.0);
        let bounds = Aabb {
            min: Vec2::ZERO,
            max: size,
        };
        let outcome = probe_containment(
            &world,
            MotionModelSpec::AxisSwept(Default::default()),
            world.spawn,
            bounds,
            ContainmentProbe::holding(LocalAxes::new(0.0, 0.0))
                // Wider and taller than the room itself: whatever the solver
                // does, part of this body is outside.
                .with_body_size(Vec2::new(400.0, 400.0)),
        );
        assert!(
            outcome.max_escape_px > 0.0,
            "a body larger than the room reported full containment, so the \
             measure is still the centre point"
        );
        assert!(
            outcome.final_pos.x > bounds.min.x && outcome.final_pos.x < bounds.max.x,
            "this test is only meaningful while the CENTRE stays inside — \
             otherwise a centre test would have caught it too"
        );
    }

    /// Both shipped policies stay in the box. This is the engine-side half of
    /// L6; the app-side half runs it over every REGISTERED character, which is
    /// where a provider's new cast member enters the population.
    #[test]
    fn every_movement_policy_stays_inside_a_plain_room() {
        let size = Vec2::new(960.0, 540.0);
        let world = walled_box(size, 16.0);
        let bounds = Aabb {
            min: Vec2::ZERO,
            max: size,
        };
        for (label, spec) in [
            ("axis-swept", MotionModelSpec::AxisSwept(Default::default())),
            (
                "surface-momentum",
                MotionModelSpec::SurfaceMomentum(Default::default()),
            ),
            (
                "adhesive-crawler",
                MotionModelSpec::AdhesiveCrawler(Default::default()),
            ),
        ] {
            let outcome = probe_containment(
                &world,
                spec,
                world.spawn,
                bounds,
                ContainmentProbe::holding(LocalAxes::new(1.0, 0.0)),
            );
            assert!(
                outcome.contained(),
                "the {label} policy left a plain walled room by {:.1}px (ended at \
                 {:?}). A movement policy that cannot be contained by four solid \
                 blocks cannot be used in any level the LDtk importer produces.",
                outcome.max_escape_px,
                outcome.final_pos
            );
        }
    }
}
