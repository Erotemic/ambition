//! CRUSHING C4, in the running game: a pinched body's response reaches the
//! ordinary death path, and a rewind replays it.
//!
//! The game's crush policy (`platformer_defaults.ron`) gives Scout
//! (`player_robot_v3`) a `Damage` response. A ceiling dropped in just above
//! Scout's feet pinches it against the floor: the step reports the pinch every
//! tick, the response deals its capped damage through the hit path, and at zero
//! health the body dies as any body does, by an `ActorDiedMessage` with the
//! world (`HitSource::Hazard`) as its cause. The unit arms
//! (`ambition_platformer2d_runtime::obstructions` tests) hand-build the
//! response; this one runs the game's own policy, hit path and death rules.
//!
//! The same scene under a GGRS sync-test session (rewinding 4 ticks each tick
//! and checking every resimulation) must give the same health on every tick
//! and the same tick of death.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{AmbitionSim as _, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::combat::death_rules::ActorDiedMessage;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::ecs::message::{MessageCursor, Messages};
use bevy::prelude::Entity;

use crate::common::base;

const ROOM: &str = "combat_calibration_lab";
/// How far above the feet the ceiling's underside is: well under Scout's height.
const GAP: f32 = 30.0;
/// Scout's health when the ceiling drops: a few capped blows, which the hit
/// path's invulnerability spaces about 45 ticks apart.
const HEALTH: i32 = 12;
const TICKS: usize = 400;

fn sim(rewinding: bool) -> Platformer2dSimHarness {
    let mut options = crate::common::sandbox_options()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(ROOM);
    if rewinding {
        options = options.with_sync_test_rollback_settings(4, 10);
    }
    Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds in the calibration lab")
}

fn scout(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .iter(world)
        .next()
        .expect("the session has a primary body")
}

/// What a run saw: Scout's health on each tick until it died, the tick its
/// death was reported (with its cause), and the refusal that stopped a
/// rewinding run, if one did.
#[derive(Debug, PartialEq)]
struct Run {
    health: Vec<i32>,
    died: Option<(usize, String)>,
    refusal: Option<String>,
}

fn drive(mut sim: Platformer2dSimHarness) -> Run {
    sim.step_n(base(), 20);
    let body = scout(&mut sim);
    let kin = *sim.world().get::<ae::BodyKinematics>(body).expect("Scout has kinematics");
    let half = kin.size() * 0.5;
    let feet = kin.pos.y + half.y;
    assert!(kin.size().y > GAP + 10.0, "premise: Scout ({:?}) fits under the ceiling", kin.size());
    sim.add_block(ae::Block::solid(
        "crush_ceiling",
        ae::Vec2::new(kin.pos.x - 60.0, feet - GAP - 40.0),
        ae::Vec2::new(120.0, 40.0),
    ));
    sim.world_mut().get_mut::<BodyHealth>(body).expect("Scout has health").health.current = HEALTH;
    sim.rebase_rollback_history().expect("the rollback history rebases over the scene");

    let mut deaths = MessageCursor::<ActorDiedMessage>::default();
    // Skip whatever was reported before the ceiling.
    deaths.read(sim.world().resource::<Messages<ActorDiedMessage>>()).count();
    let mut run = Run { health: Vec::new(), died: None, refusal: None };
    for tick in 0..TICKS {
        if let Err(refusal) = sim.try_step(base()) {
            run.refusal = Some(refusal);
            return run;
        }
        let died = deaths
            .read(sim.world().resource::<Messages<ActorDiedMessage>>())
            .find(|death| death.victim == body)
            .map(|death| format!("{:?}", death.cause.source));
        if let Some(cause) = died {
            run.died = Some((tick, cause));
            return run;
        }
        let health = sim.world().get::<BodyHealth>(body).expect("Scout has health").health.current;
        run.health.push(health);
    }
    run
}

/// ⭐ A pinched Scout loses health tick by tick and dies the ordinary death,
/// with the world as its cause; a rewinding session sees the same.
#[test]
fn a_pinched_body_dies_through_the_death_path_and_a_rewind_replays_it() {
    let fixed = drive(sim(false));
    assert_eq!(fixed.refusal, None, "a world with no rollback session refuses nothing");
    let first = fixed.health.first().copied().unwrap_or_default();
    assert!(
        fixed.health.windows(2).any(|w| w[1] < w[0]) || fixed.died.is_some(),
        "the pinch took no health: {:?}",
        &fixed.health[..fixed.health.len().min(20)]
    );
    let (tick, cause) = fixed.died.clone().unwrap_or_else(|| {
        panic!("Scout was pinched {TICKS} ticks and did not die; health {first} -> {:?}", fixed.health.last())
    });
    assert_eq!(cause, "Hazard", "a crush is the world's harm");
    assert!(tick > 1, "Scout died at once (tick {tick}): the response is capped per tick");

    let rewound = drive(sim(true));
    assert_eq!(rewound, fixed, "a rewind across the pinch gave another health or death");
}
