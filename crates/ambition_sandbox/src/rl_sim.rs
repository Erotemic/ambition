//! Programmatic-input / rl_sim adapter for the sandbox simulation.
//!
//! The sandbox already separates simulation from presentation: the gameplay
//! systems read `Res<ControlFrame>`, and the visible binary's input pipeline
//! is the only thing that writes to it. Headless tests already drive the
//! sim by writing `ControlFrame` directly between `app.update()` calls.
//!
//! `SandboxSim` packages that stepping pattern into a small public API so
//! external drivers — RL agents, fuzz harnesses, scripted-replay tools,
//! Python bindings via PyO3 in the future — can build on top of one
//! shared seam instead of each rolling their own minimal-plugin App
//! boilerplate.
//!
//! Usage from Rust:
//!
//! ```no_run
//! use ambition_sandbox::rl_sim::{AgentAction, SandboxSim};
//!
//! let mut sim = SandboxSim::new().expect("sim builds");
//! let mut action = AgentAction::default();
//! action.move_x = 1.0;
//! action.jump = true;
//! let obs = sim.step(action);
//! println!("after one tick: pos {:?}, on_ground {}", obs.player_pos, obs.on_ground);
//! ```
//!
//! Action / observation shape matches the simulation's existing
//! `ControlFrame` and engine `Player` aggregate so the conversion is
//! lossless and the seam stays narrow. Adding a new action knob means
//! adding a `ControlFrame` field; adding a new observation field means
//! reading another piece of engine / ECS state out.

mod action;
mod observation;
mod options;
pub mod reward;
mod runtime;

#[cfg(test)]
mod tests;

pub use action::AgentAction;
pub use observation::AgentObservation;
pub use options::{SandboxSimOptions, TimestepMode};
pub use runtime::SandboxSim;
