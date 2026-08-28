//! Boss encounter-phase projection, brain tick, and body integration systems.
//!
//! Phase projection precedes brain decisions; move playback remains attack execution authority.

mod sync;
mod tick;

pub use sync::*;
pub use tick::*;

#[cfg(test)]
mod tests;
