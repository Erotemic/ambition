//! Bubble Shield END-TO-END through the real headless sim: the canonical player's
//! Special slot deploys the ONE shield on the SAME tick the button is pressed.
//!
//! The unit tests in `ambition_actors` prove the two halves separately — that
//! pressing Special starts the folded `bubble_shield` move, and that
//! `sustain_bubble_shield` forces `shield_held` while that move plays. Neither
//! exercises the INTEGRATED production path, and the folded move alone would raise
//! the guard one tick late (`trigger_moveset_moves` runs in `Combat`, after the
//! `PlayerInput` seam that reads input). This drives a real `AgentAction` through
//! the whole schedule and pins that `BodyShieldState.active` flips UP on the press
//! tick, then stays up while the move plays:
//!
//!   AgentAction{special} -> ControlFrame -> ActorControl.special_pressed
//!     -> gate_worn_player_control (keeps the `Move("special")` press)
//!     -> sustain_bubble_shield (press-tick + duration => shield_held)
//!     -> resolve_shield (kernel) -> BodyShieldState.active
//!
//! The `trigger_moveset_moves` Combat step also starts the folded move, so the
//! read-model reports which move raised the guard — proving it is the special, not
//! a bare held-guard.

#![cfg(feature = "rl_sim")]

use ambition::actors::actor::PrimaryPlayerOnly;
use ambition::combat::moveset::MovePlayback;
use ambition::engine_core::body_clusters::BodyShieldState;
use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use bevy::prelude::{Entity, World};

fn primary_player(world: &mut World) -> Entity {
    let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
    q.single(world)
        .expect("the sandbox spawns a primary player")
}

fn shield_active(sim: &mut SandboxSim, player: Entity) -> bool {
    sim.world_mut()
        .get::<BodyShieldState>(player)
        .copied()
        .expect("the player body carries a shield cluster")
        .active
}

#[test]
fn pressing_special_raises_the_bubble_shield_on_the_press_tick() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    // Settle the initial spawn (deferred cluster inserts applied) with a neutral
    // tick, then the guard is down and no special move is playing.
    sim.step(AgentAction::default());
    let player = primary_player(sim.world_mut());
    assert!(
        !shield_active(&mut sim, player),
        "the bubble shield is down before Special is pressed"
    );
    assert!(
        sim.world_mut().get::<MovePlayback>(player).is_none(),
        "no move is playing before the press"
    );

    // Press Special through the REAL input/brain path.
    sim.step(AgentAction {
        special: true,
        ..AgentAction::default()
    });

    // SAME tick as the press: the guard is up (no one-tick lag), and it is the
    // folded `bubble_shield` special move that raised it.
    assert!(
        shield_active(&mut sim, player),
        "pressing Special raises the bubble shield the SAME tick it is pressed"
    );
    let playback = sim
        .world_mut()
        .get::<MovePlayback>(player)
        .expect("the folded special move started this tick");
    assert_eq!(
        playback.spec.id, "bubble_shield",
        "the started move IS the folded bubble_shield special, not a bare guard"
    );

    // A following tick with NO input keeps the guard up for the move's duration —
    // `sustain_bubble_shield`'s duration branch, now that `special_pressed` is gone.
    sim.step(AgentAction::default());
    assert!(
        shield_active(&mut sim, player),
        "the guard stays up while the bubble_shield move plays"
    );
}
