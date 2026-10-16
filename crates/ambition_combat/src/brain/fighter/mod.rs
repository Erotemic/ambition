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

/// The shipped ladder (`game/ambition_content/assets/data/fighter_brain_ladder.ron`),
/// for the tests that make a claim about the rungs a player meets. The
/// engine's default is one profile at every level, so a rung-order claim made
/// of it would be vacuous.
#[cfg(test)]
pub(crate) fn shipped_ladder() -> &'static ambition_characters::brain::fighter::FighterBrainLadder {
    static LADDER: std::sync::OnceLock<ambition_characters::brain::fighter::FighterBrainLadder> =
        std::sync::OnceLock::new();
    LADDER.get_or_init(|| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../game/ambition_content/assets/data/fighter_brain_ladder.ron");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        ambition_characters::brain::fighter::FighterBrainLadder::from_ron(&text)
            .unwrap_or_else(|err| panic!("{}: {err}", path.display()))
    })
}

/// One rung of [`shipped_ladder`].
#[cfg(test)]
pub(crate) fn shipped_rung(level: u8) -> ambition_characters::brain::fighter::FighterBrainProfile {
    *shipped_ladder().level(level).expect("the shipped ladder has nine rungs")
}
