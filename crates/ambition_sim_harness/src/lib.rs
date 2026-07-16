//! `ambition_sim_harness` — a programmatic harness for driving the platformer
//! simulation headlessly.
//!
//! The sandbox separates simulation from presentation: gameplay systems read
//! `Res<ControlFrame>`, and drivers write it between `app.update()` calls. This
//! crate packages that stepping pattern into a small public API — [`SandboxSim`]
//! ([`SandboxSim::build`], [`SandboxSim::step`], [`SandboxSim::reset_episode`],
//! [`AgentAction`], [`AgentObservation`], the example [`reward`] shaping, and the
//! [`random_policy`] fuzz driver) — so RL agents, fuzz harnesses, scripted-replay
//! tools, and future Python bindings build on ONE shared seam.
//!
//! **It composes below the product shell.** The harness builds the engine
//! foundation (`add_headless_foundation`) and the sim-schedule choice, then hands
//! the App to a caller-supplied `compose` closure that installs *that game's*
//! content + sim plugins. So a demo/test runs a sim through the harness WITHOUT
//! linking `ambition_app`: it passes its own composition. `ambition_app` passes
//! Ambition's (see its `rl_sim::AmbitionSim` constructors).
//!
//! ```no_run
//! use ambition_sim_harness::{AgentAction, SandboxSim, SandboxSimOptions};
//! # fn compose(_: &mut bevy::prelude::App, _: &SandboxSimOptions) -> Result<(), String> { Ok(()) }
//! let mut sim = SandboxSim::build(SandboxSimOptions::default(), compose).expect("sim builds");
//! let mut action = AgentAction::default();
//! action.move_x = 1.0;
//! action.jump = true;
//! let obs = sim.step(action);
//! println!("after one tick: pos {:?}, on_ground {}", obs.player_pos, obs.on_ground);
//! ```

pub mod action;
pub mod observation;
pub mod options;
pub mod random_policy;
pub mod reward;
pub mod runtime;

pub use action::AgentAction;
pub use observation::{AgentObservation, EnemyObs, PickupObs};
pub use options::{SandboxSimOptions, TimestepMode};
pub use random_policy::{Lcg, RandomWalkPolicy, RandomWalkTuning};
pub use runtime::SandboxSim;
