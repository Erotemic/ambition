//! Platformer navigation: where a body can stand in a room, and how it gets
//! from one place to the next, measured by the movement kernel it really runs.
//!
//! - [`envelope`]: how far and how high one body's jump and walk-off go.
//! - [`surfaces`]: the standing surfaces of a room for a body's size.
//! - [`graph`]: the legs between surfaces that the body can do, each one
//!   rolled out in the kernel, and the routes over them.
//!
//! The leg itself and the rule that follows it are in
//! `ambition_platformer2d_core::navigation`, so a brain and the graph use one
//! rule. See `docs/planning/engine/platformer-navigation-and-reachability.md`.

pub mod envelope;
pub mod graph;
pub mod surfaces;

pub use envelope::{ArcSample, EnvelopeProbe, TraversalEnvelope};
pub use graph::{NavGraph, NavLink};
pub use surfaces::{standing_surfaces, NavFrame, StandSurface};
