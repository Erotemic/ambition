//! The hazard gate tests the box the movement step used.
//!
//! A body that is not square has two boxes when its support is not the floor:
//! the level box (`BodyKinematics::aabb`) and the box the step moved (a body
//! on a wall, or a body in sideways gravity, lies along its support). The
//! hazard gate tested the level box at the end of the step and swept the level
//! half along the path, so a slug on a wall was hit by a hazard beside it and
//! was not hit by a hazard under its own end.
//!
//! Each case here uses a 48x22 body and a thin hazard that the two boxes
//! answer in opposite ways. Each case asks the end overlap and the swept path
//! separately, and each steps a real body through [`step_motion`].

use super::super::*;
use crate::body_clusters::{BodyClusterScratch, SweepSample};
use crate::world::Block;
use crate::{AbilitySet, Aabb, MotionFrame, Vec2, World};

pub(super) const BODY: Vec2 = Vec2::new(48.0, 22.0);
/// The box of the body when it lies along a vertical support.
pub(super) const STEP_HALF: Vec2 = Vec2::new(11.0, 24.0);
const DT: f32 = 1.0 / 60.0;
/// The right face of the wall the slug is attached to.
const WALL_FACE_X: f32 = 48.0;

/// A room with one wall on its left side and nothing else.
pub(super) fn room(hazard: Option<Aabb>) -> World {
    let size = Vec2::new(4000.0, 2400.0);
    let mut blocks = vec![Block::solid("left wall", Vec2::ZERO, Vec2::new(WALL_FACE_X, size.y))];
    if let Some(hazard) = hazard {
        blocks.push(Block::hazard("thin hazard", hazard.min, hazard.max - hazard.min));
    }
    World {
        name: "hazard footprint".to_string(),
        size,
        spawn: size * 0.5,
        blocks,
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
        chains: Vec::new(),
        edges: Default::default(),
    }
}

pub(super) fn normal_gravity() -> MotionFrame {
    MotionFrame::from_acceleration(Vec2::new(0.0, GRAVITY)).expect("gravity is not zero")
}

/// Gravity toward +x: a falling body lies along a vertical support.
pub(super) fn sideways_gravity() -> MotionFrame {
    MotionFrame::from_acceleration(Vec2::new(GRAVITY, 0.0)).expect("gravity is not zero")
}

/// What one step of a body gave.
pub(super) struct Stepped {
    pub(super) reset: Option<ResetCause>,
    pub(super) sample: SweepSample,
    pub(super) attached: bool,
    pub(super) pos: Vec2,
    pub(super) vel: Vec2,
    /// The water and the ladder the step cached (the axis arm caches them).
    pub(super) in_water: bool,
    pub(super) on_climbable: bool,
    /// An axis body still hangs on a ledge.
    pub(super) hanging: bool,
}

/// A body, and the step it takes.
#[derive(Clone, Copy)]
pub(super) struct Body {
    pub(super) spec: MotionModelSpec,
    /// The normal of the wall a crawler starts attached to.
    pub(super) attached_to: Option<Vec2>,
    /// The ledge an axis body starts the step hanging on.
    pub(super) hang: Option<crate::LedgeContact>,
    pub(super) abilities: AbilitySet,
    pub(super) pos: Vec2,
    pub(super) vel: Vec2,
    pub(super) frame: MotionFrame,
}

impl Body {
    /// A slug attached to the left wall, which crawls up it at `crawl_speed`.
    fn slug_on_the_wall(crawl_speed: f32) -> Self {
        Self {
            spec: MotionModelSpec::AdhesiveCrawler(CrawlerParams {
                crawl_speed,
                ..CrawlerParams::default()
            }),
            attached_to: Some(Vec2::new(1.0, 0.0)),
            hang: None,
            abilities: AbilitySet::default(),
            pos: Vec2::new(WALL_FACE_X + STEP_HALF.x, 1000.0),
            vel: Vec2::ZERO,
            frame: normal_gravity(),
        }
    }

    /// A body in sideways gravity with no surface near it.
    pub(super) fn in_sideways_gravity(spec: MotionModelSpec, vel: Vec2) -> Self {
        Self {
            spec,
            attached_to: None,
            hang: None,
            abilities: AbilitySet::default(),
            pos: Vec2::new(1000.0, 1200.0),
            vel,
            frame: sideways_gravity(),
        }
    }

    /// One step. `sampled` gives the body a [`SweepSample`], as an entity has;
    /// a body with none is tested at its end only.
    pub(super) fn step(self, world: &World, sampled: bool) -> Stepped {
        let mut scratch = BodyClusterScratch::new_with_abilities(self.pos, self.abilities);
        scratch.kinematics.size = BODY;
        scratch.base_size.base_size = BODY;
        scratch.kinematics.vel = self.vel;
        scratch.kinematics.facing = -1.0;
        let mut sample = SweepSample::default();
        let (model, mut clusters) = scratch.parts();
        switch_motion_model(model, self.spec);
        if let (MotionModel::AdhesiveCrawler(crawler), Some(normal)) = (&mut *model, self.attached_to) {
            crawler.state = CrawlerState::attached(normal);
        }
        if let (MotionModel::AxisSwept(axis), Some(contact)) = (&mut *model, self.hang) {
            axis.state.ledge_grab = Some(crate::LedgeGrabState::hanging(contact));
        }
        if sampled {
            clusters.sweep = Some(&mut sample);
        }
        let result = step_motion(
            model,
            &mut clusters,
            MotionStepContext {
                world,
                input: InputState::default(),
                frame: self.frame,
                // A crawler that faces -1 on this wall crawls toward -y.
                facing_intent: -1.0,
                dt: DT,
                contact: body_contact::BodyContactField::NONE,
                pose_owned_externally: false,
                recovery_commitment_outstanding: false,
            },
        );
        let attached = matches!(&*model, MotionModel::AdhesiveCrawler(crawler) if crawler.state.is_attached());
        let hanging = matches!(&*model, MotionModel::AxisSwept(axis) if axis.state.ledge_grab.is_some());
        Stepped {
            reset: result.events.reset,
            sample,
            attached,
            pos: scratch.kinematics.pos,
            vel: scratch.kinematics.vel,
            in_water: scratch.env_contact.water.is_some(),
            on_climbable: scratch.env_contact.climbable.is_some(),
            hanging,
        }
    }
}

fn overlaps(a: Aabb, b: Aabb) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

/// A thin region, as an offset box from `centre`.
pub(super) fn hazard_at(centre: Vec2, min: Vec2, max: Vec2) -> Aabb {
    Aabb {
        min: centre + min,
        max: centre + max,
    }
}

/// One case: a body, a hazard, and the answer of the box the step used.
struct Case {
    name: &'static str,
    body: Body,
    hazard: Aabb,
    /// The hazard is between the two ends of the path, and touches neither.
    swept: bool,
    hit: bool,
}

/// Run a case. The premise comes from the room with no hazard: a hazard block
/// stops no body, so the body moves the same in the two rooms.
#[track_caller]
fn judge(case: Case) {
    let Case { name, body, hazard, swept, hit } = case;
    let clear = body.step(&room(None), true);
    assert_eq!(clear.reset, None, "{name}: premise, a room with no hazard resets nothing");
    assert_eq!(
        clear.attached,
        body.attached_to.is_some(),
        "{name}: premise, the step did not change the attachment",
    );
    // The two boxes disagree about this hazard, at one end or on the path.
    let ends = |half: Vec2| {
        overlaps(Aabb::new(clear.sample.prev, half), hazard) || overlaps(Aabb::new(clear.sample.curr, half), hazard)
    };
    let on_path =
        |half: Vec2| crate::cast::aabb_path_contacts(clear.sample.curr, half, clear.sample.delta(), hazard);
    let level_half = BODY * 0.5;
    if swept {
        assert!(clear.sample.delta().length() > 90.0, "{name}: premise, a long step: {:?}", clear.sample);
        assert!(!ends(STEP_HALF) && !ends(level_half), "{name}: premise, no end of the path touches the hazard");
    }
    assert_eq!(on_path(STEP_HALF), hit, "{name}: premise, the answer of the box the step used");
    assert_eq!(on_path(level_half), !hit, "{name}: premise, the level box gives the opposite answer");

    let stepped = body.step(&room(Some(hazard)), true);
    assert_eq!(
        stepped.reset,
        hit.then_some(ResetCause::Hazard),
        "{name}: the hazard gate must answer for the box the step used",
    );
    // With a sample, the path test covers a hit at the end, and so hides an
    // end test of the wrong box. A body with no sample has the end test only.
    if !swept {
        assert_eq!(
            body.step(&room(Some(hazard)), false).reset,
            hit.then_some(ResetCause::Hazard),
            "{name}: with no sample, the end test must answer for the box the step used",
        );
    }
}

/// The slug's centre while it is attached to the wall.
const ON_WALL: Vec2 = Vec2::new(WALL_FACE_X + STEP_HALF.x, 1000.0);

/// A slug on a wall: its box is 22 wide and 48 tall.
#[test]
fn a_slug_on_a_wall_is_hit_at_its_end_by_a_hazard_its_own_box_touches() {
    judge(Case {
        name: "under the end of the slug",
        body: Body::slug_on_the_wall(40.0),
        hazard: hazard_at(ON_WALL, Vec2::new(-7.0, 16.0), Vec2::new(5.0, 20.0)),
        swept: false,
        hit: true,
    });
}

#[test]
fn a_slug_on_a_wall_is_not_hit_at_its_end_by_a_hazard_beside_it() {
    judge(Case {
        name: "beside the slug",
        body: Body::slug_on_the_wall(40.0),
        hazard: hazard_at(ON_WALL, Vec2::new(15.0, -3.0), Vec2::new(21.0, 3.0)),
        swept: false,
        hit: false,
    });
}

/// 6000 px/s is 100 px in a tick: the hazard is between the two ends.
#[test]
fn a_slug_that_crawls_past_a_hazard_beside_its_path_is_not_hit() {
    judge(Case {
        name: "beside the path of the slug",
        body: Body::slug_on_the_wall(6000.0),
        hazard: hazard_at(ON_WALL, Vec2::new(15.0, -52.0), Vec2::new(21.0, -48.0)),
        swept: true,
        hit: false,
    });
}

/// The rule is the frame's, not the crawler's: each policy in sideways gravity
/// moves a box 22 wide and 48 tall, and its path is 48 tall.
fn policies_that_fall() -> [(&'static str, MotionModelSpec); 2] {
    [
        ("axis-swept", MotionModelSpec::AxisSwept(AxisSweptParams::default())),
        ("detached crawler", MotionModelSpec::AdhesiveCrawler(CrawlerParams::default())),
    ]
}

#[test]
fn a_body_in_sideways_gravity_is_hit_by_a_hazard_its_path_crosses() {
    for (name, spec) in policies_that_fall() {
        let body = Body::in_sideways_gravity(spec, Vec2::new(9000.0, 0.0));
        judge(Case {
            name,
            body,
            hazard: hazard_at(body.pos, Vec2::new(70.0, 16.0), Vec2::new(74.0, 20.0)),
            swept: true,
            hit: true,
        });
    }
}

#[test]
fn a_body_in_sideways_gravity_is_hit_at_its_end_by_a_hazard_its_own_box_touches() {
    for (name, spec) in policies_that_fall() {
        let body = Body::in_sideways_gravity(spec, Vec2::ZERO);
        judge(Case {
            name,
            body,
            hazard: hazard_at(body.pos, Vec2::new(-4.0, 16.0), Vec2::new(4.0, 20.0)),
            swept: false,
            hit: true,
        });
    }
}

#[test]
fn a_body_in_sideways_gravity_is_not_hit_at_its_end_by_a_hazard_beside_it() {
    for (name, spec) in policies_that_fall() {
        let body = Body::in_sideways_gravity(spec, Vec2::ZERO);
        judge(Case {
            name,
            body,
            hazard: hazard_at(body.pos, Vec2::new(15.0, -3.0), Vec2::new(20.0, 3.0)),
            swept: false,
            hit: false,
        });
    }
}

/// The record a later reader sweeps (`SweepSample::touches`, `end_aabb`) holds
/// the same box.
#[test]
fn the_sweep_sample_records_the_box_the_step_used() {
    let on_wall = Body::slug_on_the_wall(40.0).step(&room(None), true);
    assert!(on_wall.attached, "premise: the slug stays on the wall");
    assert_eq!(on_wall.sample.half, STEP_HALF, "a slug on a wall");
    for (name, spec) in policies_that_fall() {
        let fell = Body::in_sideways_gravity(spec, Vec2::ZERO).step(&room(None), true);
        assert_eq!(fell.sample.half, STEP_HALF, "{name} in sideways gravity");
    }
    // In normal gravity with no wall, the step box is the level box.
    let mut level = Body::in_sideways_gravity(MotionModelSpec::AxisSwept(AxisSweptParams::default()), Vec2::ZERO);
    level.frame = normal_gravity();
    assert_eq!(level.step(&room(None), true).sample.half, BODY * 0.5, "a body in normal gravity");
}
