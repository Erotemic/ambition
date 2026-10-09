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
