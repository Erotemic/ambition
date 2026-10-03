//! GATE-PER-ACTOR (Q54) in a live match: a fighter's brain reads the floors its
//! own body meets.
//!
//! A gate solid is a floor for one body and open for another. The body steps
//! against the walls without its open gates. Its brain must ask its floor
//! queries (`ground_below`, `supporting_floor`, `floor_below`) of the same
//! walls, or it plans on a floor its body falls through.
//!
//! The geometry is read from the live stage.

use bevy::prelude::*;

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;

const FLOOR: &str = "gate:void_floor";

/// The gate floor a test puts in the void, and the body it is open for.
#[derive(Resource)]
struct VoidFloor {
    floor: Option<ae::Aabb>,
    open_for: Option<Entity>,
}

/// Put the gate floor into the room's overlay every frame, after the overlay
/// rebuild.
fn floor_the_void(
    void: Res<VoidFloor>,
    mut overlays: Query<&mut ambition_platformer2d::world::FeatureEcsWorldOverlay>,
) {
    let Some(floor) = void.floor else {
        return;
    };
    for mut overlay in &mut overlays {
        overlay
            .gate_solids
            .push(ae::Block::solid(FLOOR, floor.min, floor.max - floor.min));
        if let Some(body) = void.open_for {
            overlay.gate_passes.push(ambition_platformer2d::world::GatePass {
                block: FLOOR.to_string(),
                bodies: vec![body],
            });
        }
    }
}

/// A match past its countdown. Seat 0 is the human seat with no controller,
/// so it stands still; seat 1 is a CPU.
fn a_live_stage() -> App {
    let characters = [
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
    ];
    let mut app = ambition_demo_smash_app::build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let roster = ambition_demo_smash::smash_roster(characters);
    let countdown = roster.rules.opening_countdown_ticks;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }
    app
}

/// The gate a test puts under the CPU.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Gate {
    /// No gate floor at all.
    None,
    /// A gate floor that is closed for the CPU.
    Closed,
    /// A gate floor that is open for the CPU.
    OpenForIt,
}

/// The path of the CPU put in the air over the void left of the main
/// platform, with a gate floor below it or not.
fn the_cpu_over_the_void(gate: Gate) -> Vec<Vec2> {
    let mut app = a_live_stage();
    let lip = {
        let mut rooms = app.world_mut().query::<&ae::RoomGeometry>();
        let world = &rooms
            .iter(app.world())
            .next()
            .expect("a live stage has room geometry")
            .0;
        world
            .blocks
            .iter()
            .max_by(|a, b| {
                a.aabb
                    .width()
                    .partial_cmp(&b.aabb.width())
                    .expect("stage geometry is finite")
            })
            .expect("a stage has at least one solid")
            .aabb
            .min
    };
    assert!(lip.x > 40.0, "the stage has no void left of its platform (lip {lip:?})");
    let cpu = {
        let mut seats = app
            .world_mut()
            .query::<(Entity, &ambition_platformer2d::actor::MatchSeat)>();
        seats
            .iter(app.world())
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("seat 1 is seated")
    };
    // +Y is down. The floor is in the void, above the platform's top.
    let floor = ae::Aabb {
        min: Vec2::new(8.0, lip.y - 60.0),
        max: Vec2::new(lip.x - 8.0, lip.y - 28.0),
    };
    app.insert_resource(VoidFloor {
        floor: (gate != Gate::None).then_some(floor),
        open_for: (gate == Gate::OpenForIt).then_some(cpu),
    });
    use ambition_platformer2d::sim::{
        FeatureWorldOverlayContributions, Platformer2dSimulationPhaseMonolith, SimScheduleExt,
    };
    let sim = app.sim_schedule();
    app.add_systems(
        sim,
        floor_the_void
            .in_set(FeatureWorldOverlayContributions)
            .in_set(Platformer2dSimulationPhaseMonolith::WorldPrep),
    );
    {
        let mut kin = app
            .world_mut()
            .get_mut::<ae::BodyKinematics>(cpu)
            .expect("a seated fighter has kinematics");
        kin.pos = Vec2::new(lip.x * 0.5, lip.y - 200.0);
        kin.vel = Vec2::ZERO;
    }
    (0..60)
        .map(|_| {
            app.update();
            app.world()
                .get::<ae::BodyKinematics>(cpu)
                .map_or(Vec2::splat(f32::NAN), |kin| kin.pos)
        })
        .collect()
}

fn first_difference(a: &[Vec2], b: &[Vec2]) -> Option<usize> {
    a.iter().zip(b).position(|(a, b)| a != b)
}

/// A CPU over the void with a gate floor below it. With the floor closed for
/// it, it plays differently from a CPU over the bare void, before its body
/// reaches the floor. With the floor open for it, its body falls through, so
/// its brain must see the bare void too: it must play exactly as over the
/// bare void.
#[test]
fn a_fighter_over_a_floor_open_for_it_plays_as_over_the_void() {
    let bare = the_cpu_over_the_void(Gate::None);
    let closed = the_cpu_over_the_void(Gate::Closed);
    assert!(
        first_difference(&bare, &closed).is_some(),
        "control: a CPU over a closed gate floor played exactly as over the bare \
         void, so this fixture cannot see what the brain reads"
    );

    let open = the_cpu_over_the_void(Gate::OpenForIt);
    assert_eq!(
        first_difference(&bare, &open),
        None,
        "a CPU over a gate floor open for it did not play as over the bare void: \
         its brain read a floor its body does not meet"
    );
}
