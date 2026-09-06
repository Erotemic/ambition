//! THE FIGHTER BRAIN'S THINKING — the decision tick and everything it runs.
//!
//! ⭐⭐ D168: a floor crate owns what a character IS; the layer above owns how it
//! THINKS. `ambition_characters` keeps the fighter's SHAPE — `FighterCfg`,
//! `FighterState`, the habit model, the option vocabulary, the profile ladder and
//! the situation enum — and the decision, the option scoring's engine, the shadow
//! rollout, the recovery probe, the reeling response, the charge maths, the
//! scenario suite and the content schema live here.
//!
//! ⛔ WHAT COULD NOT COME, and it is not a judgement: every type the `Brain`
//! snapshot encoder reads, plus every type `BrainSnapshot` names BY VALUE, is
//! pinned to `ambition_characters` by the orphan rule — that crate owns `Brain`
//! and this one depends on it. `BrainSnapshot.attack_kit` is a
//! `Vec<AttackCandidate>`, which is why the whole option VOCABULARY stayed while
//! its scoring came.
//!
//! ⚠ `habit`, `options`, `profile` and `situation` stayed WHOLE rather than being
//! split again. Each is majority-shape with a little behaviour ON that shape —
//! `HabitModel` learning, `classify`, `profile_for_level` — and splitting them
//! would buy a boundary nobody is asking for. Say so rather than leaving the next
//! reader to wonder why the line is where it is.

pub mod charge;
/// The `fighter_brain_ladder` schema this capability owns. Behind `content_pack`:
/// a game that never validates its content must not link a compiler.
#[cfg(feature = "content_pack")]
pub mod content_schema;
pub mod decision;
pub mod evaluation;
pub mod recovery;
pub mod reeling;
pub mod rollout;
pub mod scenarios;

pub use decision::tick_fighter;
