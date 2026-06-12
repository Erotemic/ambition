#![allow(dead_code)]

//! Shared fixtures for `ambition_app` integration tests.
//!
//! Keep this intentionally small: integration tests should still read like
//! end-to-end scripts, but the neutral `AgentAction` and fixed-60Hz sim setup are
//! common enough that copying them into every test obscures the scenario logic.

use ambition_app::rl_sim::TimestepMode;
use ambition_app::{AgentAction, SandboxSim, SandboxSimOptions};

/// A fully-neutral action; build scenario inputs with struct update:
/// `AgentAction { move_x: 1.0, ..base() }`.
pub fn base() -> AgentAction {
    AgentAction {
        move_x: 0.0,
        move_y: 0.0,
        up_pressed: false,
        down_pressed: false,
        jump: false,
        jump_held: false,
        jump_released: false,
        dash: false,
        attack: false,
        blink: false,
        blink_held: false,
        blink_released: false,
        pogo: false,
        interact: false,
        projectile: false,
        projectile_held: false,
        projectile_released: false,
        fly_toggle: false,
        reset: false,
        start: false,
        aim_x: 0.0,
        aim_y: 0.0,
    }
}

/// Hold full right for tests that only need a simple locomotion input.
pub fn hold_right() -> AgentAction {
    AgentAction {
        move_x: 1.0,
        ..base()
    }
}

/// Fixed-60Hz options in the default start room.
pub fn fixed_60hz_options() -> SandboxSimOptions {
    SandboxSimOptions::default().with_timestep(TimestepMode::fixed_60hz())
}

/// Fixed-60Hz options for a named start room.
pub fn fixed_60hz_room_options(room: &str) -> SandboxSimOptions {
    fixed_60hz_options().with_start_room(room)
}

/// Fixed-60Hz simulation in the default start room.
pub fn fixed_60hz_sim() -> SandboxSim {
    SandboxSim::new_with_options(fixed_60hz_options()).expect("SandboxSim::new")
}

/// Fixed-60Hz simulation for a named start room.
pub fn fixed_60hz_room_sim(room: &str) -> SandboxSim {
    SandboxSim::new_with_options(fixed_60hz_room_options(room)).expect("SandboxSim::new")
}
