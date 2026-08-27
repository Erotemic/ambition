//! Deterministic fighter brain built on delayed perception.
//!
//! The brain reads only [`crate::perception::WorldView`] through
//! [`crate::perception::DelayedPerception`] and acts through `ActorControl`. Tactical
//! classification, option scoring, difficulty profiles, opponent habits, recovery
//! probing, and bounded forward rollouts are split into dedicated submodules. Rollouts
//! use shadow state; authoritative movement remains in the engine kernel.

pub mod charge;
/// The `fighter_brain_ladder` schema this capability owns. Behind `content_pack`:
/// a game that never validates its content must not link a compiler.
#[cfg(feature = "content_pack")]
pub mod content_schema;
pub mod data;
pub mod decision;
pub mod evaluation;
pub mod habit;
pub mod options;
pub mod profile;
pub mod recovery;
pub mod reeling;
pub mod rollout;
pub mod scenarios;
pub mod situation;

pub use charge::{charge_ticks_for, hold_ticks};
pub use data::{ApmLedger, FighterCfg, FighterState, FoeSample, PendingAttack};
pub use decision::tick_fighter;
pub use habit::{Choice, HabitModel};
pub use options::{generate_options, AttackOption, MoveOption, OptionSet, UtilityWeights};
pub use profile::{
    profile_for_level, AuthoredFighterLadder, FighterBrainLadder, FighterBrainProfile,
};
pub use recovery::{BodyKit, RecoveryLens, RecoveryQuery};
pub use reeling::survival_stick;
pub use rollout::{
    refine_by_rollout, shadow_step, RefinedChoice, ShadowEvent, ShadowIntent, ShadowState,
    ShadowTuning,
};
pub use scenarios::{suite, Scenario};
pub use situation::{classify, Situation};
