//! Platformer navigation: what a body can traverse, measured by the movement
//! kernel it really runs. See `docs/planning/engine/platformer-navigation-and-reachability.md`.

pub mod envelope;

pub use envelope::{ArcSample, EnvelopeProbe, TraversalEnvelope};
