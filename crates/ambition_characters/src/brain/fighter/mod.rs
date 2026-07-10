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
//! - **FB2's L2** — [`options`], the option generator + utility scorer. Movement
//!   verbs from the body's capability mask; attacks from CM7's frame-data table,
//!   which is what lets the brain understand a character nobody wrote a table for.
//!
//! - **FB4a** — [`profile`], the nine-rung difficulty ladder as data, and the one
//!   humanity check that is now STRUCTURAL: [`crate::perception::Perceived`] has a
//!   private field, so a brain layer cannot name a live view. The delay buffer is
//!   the only read path because it is the only mint.
//!
//! Still owed: FB4's ladder self-play rig and APM enforcement (both need a brain
//! that emits inputs), the opponent model (FB5), and L3's forward rollouts (FB6,
//! on N3.1's snapshot seam).

pub mod options;
pub mod profile;
pub mod scenarios;
pub mod situation;

pub use options::{generate_options, AttackOption, MoveOption, OptionSet, UtilityWeights};
pub use profile::{FighterBrainLadder, FighterBrainProfile};
pub use scenarios::{suite, Scenario};
pub use situation::{classify, Situation};
