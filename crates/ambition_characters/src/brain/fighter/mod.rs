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
//! - **FB3's L1** — [`situation`](self::situation), the tactical-state classifier. A pure function
//!   of the view; and [`scenarios`](self::scenarios), the fixture suite it is asserted against and
//!   that FB4's ladder rig will score survival % and damage ratio over.
//!
//! - **FB2's L2** — [`options`](self::options), the option generator + utility scorer. Movement
//!   verbs from the body's capability mask; attacks from CM7's frame-data table,
//!   which is what lets the brain understand a character nobody wrote a table for.
//!
//! - **FB4a** — [`profile`], the nine-rung difficulty ladder as data, and the one
//!   humanity check that is now STRUCTURAL: [`crate::perception::Perceived`] has a
//!   private field, so a brain layer cannot name a live view. The delay buffer is
//!   the only read path because it is the only mint.
//!
//! - **FB5** — [`habit`](self::habit), the opponent model. Bounded (`Situation × Choice`),
//!   inspectable, decayed, and deterministic. `read_weight = 0` on levels 1–3
//!   means the model, however confident, contributes nothing.
//!
//! - **FB6's L3** — [`rollout`](self::rollout), forward rollouts on a SHADOW MODEL
//!   (fighter-brain.md §12): a pure imagination built only from a `Perceived`,
//!   stepped under an exact `rollout_k × (1 + rollout_depth)` budget against a
//!   deterministic predicted opponent, striking with the REAL hit-response
//!   kernel (`ambition_platformer2d_core::hit_response` — the same one
//!   `damage_apply` resolves authoritative hits with).
//!
//! - **the recovery lens** — [`recovery`](self::recovery), the one place a REAL movement-kernel
//!   step enters a decision. The shadow is an approximation stacked on an
//!   approximation about the only question that costs a stock (*"is this fall
//!   survivable"*), so that question alone is handed to
//!   `ambition_platformer2d_core::movement::recovery`, which drives the body's
//!   own kernel over its own kit. the reusable layer reports whether support
//!   was regained; [`rollout::refine_by_rollout`] decides that this means "do not
//!   take this line".
//!
//! Still owed: FB6e's `l3_earns_its_depth` ladder gate.

/// The `fighter_brain_ladder` schema this capability owns. Behind `content_pack`:
/// a game that never validates its content must not link a compiler.
#[cfg(feature = "content_pack")]
pub mod content_schema;
pub mod decision;
pub mod evaluation;
pub mod habit;
pub mod options;
pub mod profile;
pub mod recovery;
pub mod rollout;
pub mod scenarios;
pub mod situation;

pub use decision::{tick_fighter, ApmLedger, FighterCfg, FighterState};
pub use habit::{Choice, HabitModel};
pub use options::{generate_options, AttackOption, MoveOption, OptionSet, UtilityWeights};
pub use profile::{
    profile_for_level, AuthoredFighterLadder, FighterBrainLadder, FighterBrainProfile,
};
pub use recovery::{BodyKit, RecoveryLens, RecoveryQuery};
pub use rollout::{
    refine_by_rollout, shadow_step, RefinedChoice, ShadowEvent, ShadowIntent, ShadowState,
    ShadowTuning,
};
pub use scenarios::{suite, Scenario};
pub use situation::{classify, Situation};
