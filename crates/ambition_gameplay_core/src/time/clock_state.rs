//! The live sim-clock scale resource.
//!
//! Facade: [`ClockState`] now lives in the reusable `ambition_time` crate
//! (Stage 18 T1b). Re-exported here so the historic
//! `crate::time::clock_state::ClockState` path keeps resolving; the
//! sandbox's feel-tuned smoother (`crate::time::time_control`) is its only
//! gameplay-mode writer.

pub use ambition_time::ClockState;
