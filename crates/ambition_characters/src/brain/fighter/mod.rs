//! THE FIGHTER BRAIN'S SHAPE — and only its shape, since 2026-08-27.
//!
//! ⛔⛔ THE THINKING LEFT (D168). The decision tick, the option scoring, the
//! shadow rollout, the recovery probe, the reeling response, the charge maths,
//! the scenario suite and the content schema are `ambition_platformer2d::combat::brain::fighter`
//! now: a floor crate owns what a character IS, and the layer above owns how it
//! THINKS.
//!
//! ⭐ WHAT COULD NOT FOLLOW is everything here. `Brain`'s snapshot encoder is
//! bound to this crate by the orphan rule and `ambition_combat` depends on this
//! crate, so a type the encoder reads can never move up — and `BrainSnapshot`
//! pins more on top of that: `attack_kit` is a `Vec<AttackCandidate>` BY VALUE,
//! which is why the whole option vocabulary stayed while its scoring went.
//!
//! ⚠ `habit`, `options`, `profile` and `situation` stayed WHOLE rather than being
//! split again. Each is majority-shape with a little behaviour ON that shape —
//! `HabitModel` learning, `classify`, `profile_for_level` — and splitting them
//! would buy a boundary nobody is asking for.

pub mod data;
pub mod habit;
pub mod options;
pub mod profile;
pub mod situation;

pub use data::{ApmLedger, FighterCfg, FighterState, FoeSample, PendingAttack, ShadowTuning};
pub use habit::{Choice, HabitModel};
pub use options::{generate_options, AttackOption, MoveOption, OptionSet, UtilityWeights};
pub use profile::{
    profile_for_level, AuthoredFighterLadder, FighterBrainLadder, FighterBrainProfile,
};
pub use situation::{classify, Situation};
