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
        left_pressed: false,
        right_pressed: false,
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
        interact_held: false,
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

#[cfg(feature = "portal")]
use ambition::portal::PlacedPortal;

/// Return all currently-live authored portal pairs, after any link resolution.
///
/// `portal_lab` now authors explicit `link` ids. After the app steps once,
/// linked portals are assigned generated `Indexed` channels, so tests should not
/// assume the old Purple/Yellow channels remain on the live `PlacedPortal`s.
#[cfg(feature = "portal")]
pub fn authored_portal_pairs(sim: &mut SandboxSim) -> Vec<(PlacedPortal, PlacedPortal)> {
    let mut q = sim.world_mut().query::<&PlacedPortal>();
    let world = sim.world();
    let mut portals: Vec<PlacedPortal> = q
        .iter(world)
        .filter(|p| !p.channel.is_gun_pair())
        .cloned()
        .collect();
    portals.sort_by(|a, b| {
        a.pos
            .x
            .total_cmp(&b.pos.x)
            .then(a.pos.y.total_cmp(&b.pos.y))
            .then(a.channel.name().cmp(&b.channel.name()))
    });

    let mut pairs = Vec::new();
    for entry in &portals {
        if let Some(exit) = portals
            .iter()
            .find(|candidate| candidate.channel == entry.channel.partner())
        {
            pairs.push((entry.clone(), exit.clone()));
        }
    }
    pairs
}

/// First live authored pair in deterministic left-to-right/top-to-bottom order.
#[cfg(feature = "portal")]
pub fn first_authored_portal_pair(sim: &mut SandboxSim) -> (PlacedPortal, PlacedPortal) {
    authored_portal_pairs(sim)
        .into_iter()
        .next()
        .expect("room has a linked authored portal pair")
}

/// First floor-to-floor authored pair, used by tests that must exercise a floor
/// carve instead of a wall/ceiling portal.
#[cfg(feature = "portal")]
pub fn first_floor_authored_portal_pair(sim: &mut SandboxSim) -> (PlacedPortal, PlacedPortal) {
    authored_portal_pairs(sim)
        .into_iter()
        .find(|(entry, exit)| entry.normal.y < -0.5 && exit.normal.y < -0.5)
        .expect("room has a linked floor-to-floor authored portal pair")
}
