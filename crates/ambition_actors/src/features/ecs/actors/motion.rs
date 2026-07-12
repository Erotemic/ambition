//! Movement-policy vocabulary re-exported by the actors facade.
//!
//! Physics selection, parameters, private solver state, frame handling, and
//! dispatch all live in `ambition_engine_core::movement`.  The actors layer owns
//! ECS assembly and controller translation only; it intentionally exposes no
//! model-specific stepping helper.

use ambition_engine_core as ae;

pub use ae::movement::SurfaceMomentumMotion as MomentumMotion;
pub use ae::movement::{AxisSweptMotion, MotionModel};
