//! Programmatic harness for driving a platformer simulation headlessly.
//!
//! The harness owns stepping/input injection and accepts a caller-supplied composition closure,
//! so tests, RL agents, and fuzz drivers can run game simulation without linking `ambition_app`.
//!
//! ⛔ `use ambition_sim_harness::…`, NOT `use crate::…`. A doctest compiles as
//! its own crate against this one from the OUTSIDE, so `crate::` names the
//! doctest and resolves nothing. This block said `crate::` and had been failing
//! to compile for as long as anybody had run it — which nobody had, because the
//! suite ran no doctests at all until 2026-08-27.
//!
//! ```no_run
//! use ambition_sim_harness::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions};
//! # fn compose(_: &mut bevy::prelude::App, _: &Platformer2dSimHarnessOptions) -> Result<(), String> { Ok(()) }
//! let mut sim = Platformer2dSimHarness::build(Platformer2dSimHarnessOptions::default(), compose).expect("sim builds");
//! let mut action = AgentAction::default();
//! action.move_x = 1.0;
//! action.jump = true;
//! let obs = sim.step(action);
//! println!("after one tick: pos {:?}, on_ground {}", obs.player_pos, obs.on_ground);
//! ```

pub mod action;
#[cfg(feature = "capture")]
pub mod capture;
// The recording half of the move exercise: what combat DID on a tick, in the
// one vocabulary every recorder writes it down in. Beside `move_exercise` for
// the same reason it is — two tools drive one move, and neither may describe it
// in words of its own.
pub mod combat_observation;
pub mod move_exercise;
pub mod observation;
pub mod options;
pub mod random_policy;
pub mod reward;
pub mod runtime;

pub use action::AgentAction;
pub use combat_observation::{
    CombatObservation, ObservedBody, ResolvedRoles, ScenarioRole, ScenarioRoles,
    OBSERVATION_SCHEMA,
};
#[cfg(feature = "capture")]
pub use capture::{AdapterPreference, CaptureError, CapturedFrame, DeterministicCaptureSession};
pub use observation::{AgentObservation, EnemyObs, PickupObs};
pub use options::{RollbackMode, Platformer2dSimHarnessOptions, TimestepMode};
pub use random_policy::{Lcg, RandomWalkPolicy, RandomWalkTuning};
pub use runtime::Platformer2dSimHarness;
