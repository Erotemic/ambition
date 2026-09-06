//! WHAT A BODY CAN BE MADE TO PRESS — the vocabulary a `BrainSnapshot` carries
//! and the fighter brain scores.
//!
//! ⭐⭐ IT LIVED IN `brain/fighter/options` BECAUSE THE FIGHTER IS THE ONLY
//! SCORER, and that made a generic thing look like a fighter thing:
//! `BrainSnapshot::attack_kit` — a field EVERY brain carries — was typed
//! `Vec<fighter::options::AttackCandidate>`, which is the first of the five edges
//! `docs/planning/queue.md` D166 prices the fighter carve at.
//!
//! ⛔ AND IT WAS NEVER FIGHTER VOCABULARY. `AttackBinding`'s own doc says so:
//! *"the ordinary gesture vocabulary, not a fighter-only bypass: a verb plus a
//! direction is exactly what a human's stick and button produce"*. The kit is
//! FILLED by the actors-side snapshot builder from the body's live `ActorMoveset`
//! — body-derived truth arriving through the world-in port — and the brain is
//! merely told it.
//!
//! ⚠ `AttackVerb` IS WIRE VOCABULARY (`Brain`'s rollback codec writes its tag as
//! a literal `0/1/2/3`), and moving it changes nothing on the wire: the encoder
//! names the VALUES, not the type's path. Moved 2026-08-28.

use ambition_entity_catalog::{AttackDir, MoveFrameData};

/// How a chosen attack is actually PRESSED.
///
/// L2 scored every move in the kit, L3 refined the choice, `RefinedChoice::move_id` named a
/// concrete move — and the emission set `melee_pressed = true` with a neutral axis, so
/// `trigger_moveset_moves` resolved whatever the DEFAULT gesture maps to. The brain decided whether
/// to attack and never which attack.
///
/// It is the ordinary gesture vocabulary, not a fighter-only bypass: a verb plus
/// a direction is exactly what a human's stick and button produce, and what
/// `move_for_directional_verb` consumes. The POSTURE is deliberately absent — the
/// body's real grounded state decides it at press time, and a brain that could
/// claim a posture it does not have would be reaching past the no-cheat contract
/// to pick a move its body cannot reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackBinding {
    pub verb: AttackVerb,
    pub direction: AttackDir,
}

/// The three press KINDS a moveset distinguishes. Not the move — the button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttackVerb {
    /// The plain attack button (`"attack"` and its directional variants).
    #[default]
    Basic,
    /// Attack with a smash/strong hint (`"smash_*"`, falling back to `attack_*`).
    Smash,
    /// The special button (`"special"` and its directional variants).
    Special,
    /// The GRAB button (`"grab"`).
    ///
    /// no directional variants, and a CENTRED stick. A grab is a button,
    /// not a stick gesture — a deflection beside it would arm a flick the next
    /// ordinary attack would inherit as an accidental smash.
    Grab,
}

/// One attack the caller's kit offers. The caller resolves these from the body's
/// moveset; L2 never queries anything.
///
/// The caller enumerates BINDINGS and asks the moveset what each one reaches, so
/// a candidate is a move the body can actually be made to perform — a move with
/// no binding (a buff, a summon, an on-hit technique) never enters the kit, and
/// a scored choice is executable by construction.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackCandidate {
    pub move_id: String,
    pub frames: MoveFrameData,
    pub binding: AttackBinding,
    /// Whether the BODY can begin this move on this tick — see
    /// [`ActionLegality`]. Supplied by the caller, which is the only layer that
    /// can see the running `MovePlayback`.
    pub legality: ActionLegality,
}

/// CAN this action begin right now? — a question about the BODY's state,
/// kept deliberately separate from *how useful would it be*, which is the
/// scorer's ([`Features`]) subject.
///
/// it is a FILTER, never a weight. A cheap move that cannot be started is
/// not a slightly worse option than one that can — it is not an option. Pricing
/// it low would leave it winning whenever the kit is bad, which is exactly how
/// the "an attack that cannot REACH is not an option" filter came to exist one
/// class over.
///
/// the third state is deliberately absent and named here so it is not
/// invented twice. `BodyActionBuffer` IS fed now, so a press that cannot
/// execute this tick but would be consumed on the first actionable frame is a
/// real option — `BufferableSoon` — and issuing it is what a person pressing
/// into the tail of endlag is doing. What is still missing is the fact it needs:
/// the brain cannot see the buffer's remaining window against the body's
/// remaining lock, and without that comparison "legal eventually" still must not
/// read as "press now".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActionLegality {
    /// Nothing owns the body, or the running move's cancel windows admit this
    /// one under its hit-state condition.
    #[default]
    Now,
    /// Another move owns the body and its cancel windows do not admit this one.
    /// The press would be discarded, so the brain does not spend it.
    BlockedByPlayback,
}
