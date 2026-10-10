use super::*;
use ae::movement::GRAVITY;
use ae::navigation::LegInput;
use ae::{AbilitySet, Block};

const DT: f32 = 1.0 / 60.0;
const FLOOR: f32 = 2000.0;

fn normal_frame() -> MotionFrame {
    MotionFrame::from_acceleration(Vec2::new(0.0, GRAVITY)).expect("a non-zero gravity")
}

/// A body that walks and jumps: no air jump, no dash, no wall verb.
fn walker() -> BodyClusterScratch {
    let abilities = AbilitySet { move_horizontal: true, jump: true, ..AbilitySet::NONE };
    BodyClusterScratch::new_with_abilities(Vec2::ZERO, abilities)
}

/// A room with one of each relation, sized from the body's own envelope:
///
/// - `floor`: where the body starts;
/// - `perch`: a one-way surface above the floor, in reach;
/// - `ledge`: across a gap from the floor, at its height;
/// - `cellar`: below the far end of the ledge, too deep to jump out of;
/// - `shelf`: above the floor, out of reach. The control.
fn room(apex: f32, level_gap: f32) -> World {
    let ledge = 1500.0 + level_gap * 0.5;
    World::new(
        "navigation fixture",
        Vec2::new(4000.0, 3000.0),
        Vec2::ZERO,
        vec![
            Block::solid("floor", Vec2::new(0.0, FLOOR), Vec2::new(1500.0, 64.0)),
            Block::one_way("perch", Vec2::new(400.0, FLOOR - apex * 0.6), Vec2::new(300.0, 16.0)),
            Block::solid("ledge", Vec2::new(ledge, FLOOR), Vec2::new(600.0, 64.0)),
            Block::solid("cellar", Vec2::new(ledge + 600.0, FLOOR + apex * 2.0), Vec2::new(800.0, 64.0)),
            Block::one_way("shelf", Vec2::new(900.0, FLOOR - apex * 2.5), Vec2::new(300.0, 16.0)),
        ],
    )
}

struct Fixture {
    world: World,
    body: BodyClusterScratch,
    graph: NavGraph,
}

impl Fixture {
    fn new() -> Self {
        let body = walker();
        let envelope = TraversalEnvelope::measure(&body, normal_frame(), EnvelopeProbe::default()).expect("measured");
        let level_gap = envelope.landing_lead(0.0).expect("a level jump lands");
        let world = room(envelope.apex_rise(), level_gap);
        let graph = NavGraph::build(&world, &body, normal_frame()).expect("an upright walker has a graph");
        Self { world, body, graph }
    }

    fn surface(&self, name: &str) -> usize {
        let block = self.world.blocks.iter().find(|block| block.name == name).expect("a block of the fixture");
        let top = self.graph.frame.point((block.aabb.min.x + block.aabb.max.x) * 0.5, block.aabb.min.y);
        self.graph.surface_at(top).unwrap_or_else(|| panic!("`{name}` is no standing surface"))
    }

    fn middle(&self, name: &str) -> Vec2 {
        let surface = &self.graph.surfaces[self.surface(name)];
        self.graph.frame.point((surface.left + surface.right) * 0.5, surface.top)
    }

    fn linked(&self, from: &str, to: &str) -> Option<NavLegKind> {
        let (from, to) = (self.surface(from), self.surface(to));
        self.graph.links.iter().find(|link| link.from == from && link.to == to).map(|link| link.leg.kind)
    }

    /// Put the body at `from` and let it follow the graph's advice to `goal`,
    /// as a brain does: ask for the next leg when it has none, follow it with
    /// `follow_leg`, at a walk when the leg does not ask for full speed.
    fn travel(&self, from: Vec2, goal: Vec2) -> Result<usize, String> {
        let mut body = self.body.clone();
        let half = self.graph.half;
        {
            let (model, mut clusters) = body.parts();
            ae::movement::transit_body(model, &mut clusters, from - Vec2::Y * half.y, ae::movement::TransitVelocity::Zero);
        }
        let mut leg: Option<(NavLeg, LegPhase)> = None;
        for index in 0..4000 {
            let feet = body.kinematics.pos + Vec2::Y * half.y;
            let mut input = LegInput::default();
            if leg.is_none() && body.ground.on_ground {
                match self.graph.next(feet, goal) {
                    NavNext::Arrived => return Ok(index),
                    NavNext::Leg(next) => leg = Some((next, LegPhase::Approach)),
                    NavNext::Unreachable => return Err(format!("unreachable from {feet:?}")),
                    NavNext::Unknown => {}
                }
            }
            if let Some((current, phase)) = leg {
                let facts = LegFacts {
                    feet,
                    vel: body.kinematics.vel,
                    on_ground: body.ground.on_ground,
                    side: Vec2::X,
                    down: Vec2::Y,
                    dt: DT,
                };
                let (step_input, progress) = follow_leg(&current, phase, &facts);
                input = step_input;
                if !input.full_speed {
                    input.axis *= 0.45;
                }
                leg = match progress {
                    LegProgress::Going(next) => Some((current, next)),
                    LegProgress::Arrived | LegProgress::Failed => None,
                };
            }
            step(&mut body, &self.world, normal_frame(), DT, input);
        }
        Err(format!("still going at {:?}", body.kinematics.pos))
    }
}

#[test]
fn the_graph_links_what_the_body_can_do_and_nothing_else() {
    let fixture = Fixture::new();
    assert_eq!(fixture.linked("floor", "perch"), Some(NavLegKind::Hop));
    assert_eq!(fixture.linked("perch", "floor"), Some(NavLegKind::Drop));
    assert_eq!(fixture.linked("floor", "ledge"), Some(NavLegKind::Hop));
    assert_eq!(fixture.linked("ledge", "floor"), Some(NavLegKind::Hop));
    assert_eq!(fixture.linked("ledge", "cellar"), Some(NavLegKind::Drop));
    // Too deep to jump out of, and too high to jump to.
    assert_eq!(fixture.linked("cellar", "ledge"), None);
    assert_eq!(fixture.linked("floor", "shelf"), None);
    assert_eq!(fixture.linked("perch", "shelf"), None);
    let reachable = fixture.graph.reachable_from(fixture.surface("floor"));
    assert!(reachable.contains(&fixture.surface("cellar")) && !reachable.contains(&fixture.surface("shelf")));
    assert_eq!(fixture.graph.reachable_from(fixture.surface("cellar")), vec![fixture.surface("cellar")]);
}

/// ⭐ THE GUARD. A body that follows the advice arrives, in the real kernel,
/// at each surface the graph says it can reach; and the graph says so of no
/// surface the body cannot reach.
#[test]
fn a_body_that_follows_the_advice_arrives() {
    let fixture = Fixture::new();
    let start = fixture.middle("floor") - Vec2::X * 500.0;
    for goal in ["perch", "ledge", "cellar"] {
        fixture.travel(start, fixture.middle(goal)).unwrap_or_else(|why| panic!("floor to {goal}: {why}"));
    }
    // Two legs back: down from the perch, then over the gap.
    fixture.travel(fixture.middle("perch"), fixture.middle("ledge")).expect("perch to ledge");
    assert_eq!(fixture.graph.next(start, fixture.middle("shelf")), NavNext::Unreachable);
    assert_eq!(fixture.graph.next(fixture.middle("cellar"), fixture.middle("floor")), NavNext::Unreachable);
}

#[test]
fn a_wall_takes_its_stretch_out_of_a_surface() {
    let size = walker().kinematics.size;
    let frame = NavFrame { side: Vec2::X, down: Vec2::Y };
    let floor = Block::solid("floor", Vec2::new(0.0, FLOOR), Vec2::new(2000.0, 64.0));
    let open = World::new("open", Vec2::new(2000.0, 3000.0), Vec2::ZERO, vec![floor.clone()]);
    assert_eq!(standing_surfaces(&open, frame, size).len(), 1);
    let wall = Block::solid("wall", Vec2::new(900.0, FLOOR - 300.0), Vec2::new(100.0, 300.0));
    let walled = World::new("walled", Vec2::new(2000.0, 3000.0), Vec2::ZERO, vec![floor, wall]);
    let surfaces = standing_surfaces(&walled, frame, size);
    // The floor on each side of the wall, and the top of the wall.
    assert_eq!(surfaces.len(), 3, "{surfaces:#?}");
    let on_the_floor: Vec<_> = surfaces.iter().filter(|surface| surface.top == FLOOR).collect();
    assert!(on_the_floor[0].right < 900.0 - size.x * 0.5 + 0.01 && on_the_floor[1].left > 1000.0 + size.x * 0.5 - 0.01);
}

#[test]
fn the_waypoints_are_reachable_and_the_same_for_one_seed() {
    let fixture = Fixture::new();
    let feet = fixture.middle("floor");
    let (points, count) = fixture.graph.waypoints(feet, 7);
    assert_eq!((points, count), fixture.graph.waypoints(feet, 7));
    // Floor, perch, ledge and cellar; the shelf is not one of them.
    assert_eq!(count, 4);
    let shelf = fixture.surface("shelf");
    assert!(points[..count as usize].iter().all(|point| fixture.graph.surface_at(*point).is_some_and(|at| at != shelf)));
    assert_ne!(points, fixture.graph.waypoints(feet, 8).0);
}

#[test]
fn a_place_beside_a_point_is_on_the_surface_under_it_when_that_is_in_reach() {
    let fixture = Fixture::new();
    let feet = fixture.middle("floor");
    // A body that stands on the perch: its centre is above the perch's top.
    let on_the_perch = fixture.middle("perch") - Vec2::Y * 40.0;
    let place = fixture.graph.place_beside(feet, on_the_perch, 60.0, 160.0).expect("the perch is in reach");
    assert_eq!(fixture.graph.surface_at(place), Some(fixture.surface("perch")));
    assert!((place.x - on_the_perch.x).abs() > 30.0, "beside the point, not on it: {place:?}");
    // Over the shelf, which is out of reach: no place.
    let on_the_shelf = fixture.middle("shelf") - Vec2::Y * 40.0;
    assert_eq!(fixture.graph.place_beside(feet, on_the_shelf, 60.0, 160.0), None);
}

/// Reach is a fact about the body's real abilities. The same room, the same
/// size and speed, and no jump: each place that needs a jump is out of reach,
/// and the drops are still there.
#[test]
fn a_body_with_no_jump_cannot_reach_what_needs_one() {
    let fixture = Fixture::new();
    let mut grounded = walker();
    grounded.abilities.abilities.jump = false;
    let graph = NavGraph::build(&fixture.world, &grounded, normal_frame()).expect("a walker has a graph");
    let floor = fixture.middle("floor");
    for needs_a_jump in ["perch", "ledge", "cellar"] {
        assert_eq!(graph.next(floor, fixture.middle(needs_a_jump)), NavNext::Unreachable, "{needs_a_jump}");
    }
    // Each leg it has goes down. (A leg can still be named a hop: the press
    // does nothing, the body walks off the end, and it arrives.)
    assert!(!graph.links.is_empty());
    assert!(graph.links.iter().all(|link| link.leg.land.y > link.leg.start.y + 1.0), "{:#?}", graph.links);
    // The control: down is still a route, and with the jump the perch is one.
    assert!(matches!(graph.next(fixture.middle("ledge"), fixture.middle("cellar")), NavNext::Leg(_)));
    assert!(matches!(fixture.graph.next(floor, fixture.middle("perch")), NavNext::Leg(_)));
}

/// ⭐ A LEG THAT RUNS INTO A WALL FAILS WHEN IT STOPS, NOT AT THE STEP LIMIT.
///
/// The floor ends at a wall the body cannot jump, and a lower floor lies past
/// it. The walk-off proposal off that end pushes into the wall. Measured on the
/// shipped rooms, rollouts like it were most of the build's failed steps,
/// because each ran to `MAX_LEG_STEPS`.
#[test]
fn a_leg_into_a_wall_fails_when_the_body_stops() {
    let world = World::new(
        "a wall at the end of the floor",
        Vec2::new(4000.0, 3000.0),
        Vec2::ZERO,
        vec![
            Block::solid("floor", Vec2::new(0.0, FLOOR), Vec2::new(1500.0, 64.0)),
            // To the top of the room: its own top is too high for any leg.
            Block::solid("wall", Vec2::new(1500.0, 0.0), Vec2::new(32.0, FLOOR + 64.0)),
            Block::solid("below", Vec2::new(1532.0, FLOOR + 100.0), Vec2::new(1000.0, 64.0)),
        ],
    );
    let graph = NavGraph::build(&world, &walker(), normal_frame()).expect("a graph");
    assert!(graph.links.is_empty(), "a leg crossed a wall it cannot jump: {:?}", graph.links);
    assert!(graph.cost.rollouts > 0, "no leg was rolled out, so nothing below is measured");
    assert!(
        graph.cost.steps < MAX_LEG_STEPS,
        "{} kernel steps over {} rollouts: a rollout stood at the wall until the step limit",
        graph.cost.steps,
        graph.cost.rollouts
    );
}

/// ⭐ AN AIR JUMP REACHES WHAT ONE JUMP DOES NOT. A ledge across a short gap,
/// higher than the body's jump and lower than its double jump: the body with
/// an air jump has a double hop to it, and arrives in the kernel by the
/// follower's rule. A low step on the other side stays a hop: a double hop
/// is proposed only where one jump does not reach. The control is the same
/// body with no air jump: the ledge is out of reach.
#[test]
fn a_body_with_an_air_jump_reaches_what_one_jump_does_not() {
    let mut jumper = walker();
    jumper.abilities.abilities.double_jump = true;
    let envelope = TraversalEnvelope::measure(&jumper, normal_frame(), EnvelopeProbe::default()).expect("measured");
    let (apex, air_apex) = (envelope.apex_rise(), envelope.air_jump_apex_rise());
    assert!(air_apex > apex + 16.0, "premise: the air jump lifts the body higher: {apex} then {air_apex}");
    let high = (apex + air_apex) * 0.5;
    let world = World::new(
        "a ledge only an air jump reaches",
        Vec2::new(4000.0, 3000.0),
        Vec2::ZERO,
        vec![
            Block::solid("floor", Vec2::new(600.0, FLOOR), Vec2::new(1000.0, 64.0)),
            Block::solid("ledge", Vec2::new(1640.0, FLOOR - high), Vec2::new(600.0, 64.0 + high)),
            Block::solid("step", Vec2::new(0.0, FLOOR - apex * 0.5), Vec2::new(560.0, 64.0 + apex * 0.5)),
        ],
    );
    let fixture = Fixture {
        graph: NavGraph::build(&world, &jumper, normal_frame()).expect("an upright jumper has a graph"),
        world,
        body: jumper,
    };
    assert_eq!(fixture.linked("floor", "ledge"), Some(NavLegKind::DoubleHop));
    assert_eq!(fixture.linked("floor", "step"), Some(NavLegKind::Hop));
    fixture
        .travel(fixture.middle("floor"), fixture.middle("ledge"))
        .unwrap_or_else(|why| panic!("floor to ledge by a double hop: {why}"));

    let one_jump = NavGraph::build(&fixture.world, &walker(), normal_frame()).expect("a walker has a graph");
    assert_eq!(
        one_jump.next(fixture.middle("floor"), fixture.middle("ledge")),
        NavNext::Unreachable,
        "control: with no air jump the ledge is out of reach"
    );
}

/// ⭐ A BODY THAT CAN CLIMB GETS UP A WALL NO JUMP CLEARS. A pillar beside the
/// floor, three jumps tall: the body that clings, climbs and takes ledges has
/// a wall climb to its top, and arrives in the kernel by the follower's rule
/// (it jumps to the face, holds into it and up, takes the ledge and pulls
/// itself up). The control is the same body with no climb: the top is out of
/// reach.
#[test]
fn a_body_that_climbs_gets_up_a_wall_no_jump_clears() {
    let climber = || {
        let mut body = walker();
        let verbs = &mut body.abilities.abilities;
        verbs.wall_cling = true;
        verbs.wall_climb = true;
        verbs.ledge_grab = true;
        body
    };
    let apex = TraversalEnvelope::measure(&climber(), normal_frame(), EnvelopeProbe::default())
        .expect("measured")
        .apex_rise();
    let tall = apex * 3.0;
    let world = World::new(
        "a pillar no jump clears",
        Vec2::new(4000.0, 3000.0),
        Vec2::ZERO,
        vec![
            Block::solid("floor", Vec2::new(0.0, FLOOR), Vec2::new(1600.0, 64.0)),
            Block::solid("pillar", Vec2::new(1200.0, FLOOR - tall), Vec2::new(160.0, tall)),
        ],
    );
    let fixture = Fixture {
        graph: NavGraph::build(&world, &climber(), normal_frame()).expect("an upright climber has a graph"),
        world,
        body: climber(),
    };
    assert_eq!(fixture.linked("floor", "pillar"), Some(NavLegKind::WallClimb));
    fixture
        .travel(fixture.middle("floor") - Vec2::X * 300.0, fixture.middle("pillar"))
        .unwrap_or_else(|why| panic!("floor to the pillar's top by a wall climb: {why}"));

    let mut no_climb = climber();
    no_climb.abilities.abilities.wall_climb = false;
    let graph = NavGraph::build(&fixture.world, &no_climb, normal_frame()).expect("a walker has a graph");
    assert_eq!(
        graph.next(fixture.middle("floor") - Vec2::X * 300.0, fixture.middle("pillar")),
        NavNext::Unreachable,
        "control: with no climb the pillar's top is out of reach"
    );
}

/// ⭐ A LEG THROUGH A HAZARD IS NO LEG. A gap the body hops, with a hazard
/// hung in the air over it: the kernel resets a body that touches it, so the
/// rollout fails and there is no link. Found on the walked route: legs of
/// `intro_escape_shaft` flew through its hazards, and the room reset the
/// player. The control is the same room with no hazard: the hop is there.
#[test]
fn a_leg_through_a_hazard_is_no_leg() {
    let body = walker();
    let envelope = TraversalEnvelope::measure(&body, normal_frame(), EnvelopeProbe::default()).expect("measured");
    let gap = envelope.landing_lead(0.0).expect("a level jump lands") * 0.6;
    let blocks = |hazard: bool| {
        let mut blocks = vec![
            Block::solid("floor", Vec2::new(0.0, FLOOR), Vec2::new(1000.0, 64.0)),
            Block::solid("ledge", Vec2::new(1000.0 + gap, FLOOR), Vec2::new(1000.0, 64.0)),
        ];
        if hazard {
            // Across the whole gap, at the height the arc crosses it.
            let height = envelope.apex_rise() * 0.5;
            blocks.push(Block::hazard("thorns", Vec2::new(1000.0, FLOOR - height - 8.0), Vec2::new(gap, 16.0)));
        }
        blocks
    };
    let linked = |hazard: bool| {
        let world = World::new("a gap", Vec2::new(4000.0, 3000.0), Vec2::ZERO, blocks(hazard));
        let graph = NavGraph::build(&world, &body, normal_frame()).expect("a walker has a graph");
        let fixture = Fixture { graph, world, body: body.clone() };
        fixture.linked("floor", "ledge")
    };
    assert_eq!(linked(false), Some(NavLegKind::Hop), "control: with no hazard the hop is there");
    assert_eq!(linked(true), None, "a hop through the hazard is a link");
}

/// A body that a zone takes to another room is not on the far side of that
/// zone: a leg whose body enters an exit is no leg. The same gap with no exit
/// in its air is the control.
#[test]
fn a_leg_through_an_exit_is_no_leg_for_a_body_the_exit_takes() {
    let body = walker();
    let envelope = TraversalEnvelope::measure(&body, normal_frame(), EnvelopeProbe::default()).expect("measured");
    let gap = envelope.landing_lead(0.0).expect("a level jump lands") * 0.6;
    let world = World::new(
        "a gap",
        Vec2::new(4000.0, 3000.0),
        Vec2::ZERO,
        vec![
            Block::solid("floor", Vec2::new(0.0, FLOOR), Vec2::new(1000.0, 64.0)),
            Block::solid("ledge", Vec2::new(1000.0 + gap, FLOOR), Vec2::new(1000.0, 64.0)),
        ],
    );
    // Across the whole gap, from the arc's height down into the pit.
    let height = envelope.apex_rise() * 0.5;
    let exit = ae::Aabb { min: Vec2::new(1000.0, FLOOR - height), max: Vec2::new(1000.0 + gap, FLOOR + 400.0) };
    let linked = |exits: &[ae::Aabb]| {
        let graph = NavGraph::build_avoiding(&world, exits, &body, normal_frame()).expect("a walker has a graph");
        let fixture = Fixture { graph, world: world.clone(), body: body.clone() };
        fixture.linked("floor", "ledge")
    };
    assert_eq!(linked(&[]), Some(NavLegKind::Hop), "control: with no exit the hop is there");
    assert_eq!(linked(&[exit]), None, "a hop through the exit is a link");
}
