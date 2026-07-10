//! **The advanced fighter brain** (`docs/planning/engine/fighter-brain.md`).
//!
//! A level-9 CPU that does not cheat: it reads only [`crate::perception::WorldView`],
//! it acts only through `ActorControl`, and its skill comes from prediction and
//! option quality rather than privileged state or frame-perfect reflexes.
//!
//! Three layers over the existing brain seam. What exists today:
//!
//! - **FB1** — the view audit and [`crate::perception::DelayedPerception`], the
//!   reaction-latency buffer that makes the no-cheat contract structural.
//! - **FB3's L1** — [`situation`], the tactical-state classifier. A pure function
//!   of the view; and [`scenarios`], the fixture suite it is asserted against and
//!   that FB4's ladder rig will score survival % and damage ratio over.
//!
//! Still owed: L2's option generator + utility scorer (FB2, needs CM7's frame-data
//! table), the difficulty profiles and humanity checks (FB4 — which is also what
//! finally FORCES every brain through the delay buffer), the opponent model (FB5),
//! and L3's forward rollouts (FB6, on N3.1's snapshot seam).

pub mod scenarios;
pub mod situation;

pub use scenarios::{suite, Scenario};
pub use situation::{classify, Situation};
