//! **What a death MEANS for the run** (ADR 0033).
//!
//! The engine publishes the FACT that a participant died and then does nothing.
//! What happens next is stated here, once, by the game.
//!
//! ⚠ **not [`DeathPolicy`](ambition_characters::actor::DeathPolicy)**, which
//! answers a different question — *does a full damage meter kill this body* —
//! and lives beside `BodyHealth` because it travels with the health component.
//! This answers *what happens after it does*.
//!
//! # Why the level reset is not a death consequence
//!
//! Jon, on co-op: *"Say we make maryo 2 player like NSMB, the level reset would
//! only happen if a player dies and all other players are also dead."*
//!
//! Hanging the reset off the individual death is player-centric, and in a
//! single-player game the mistake is INVISIBLE — with a roster of one, "this
//! participant died" and "no participant remains" are the same event. So the
//! reset asks a question about the ROSTER ([`LevelReset`]) and single player
//! falls out as the one-element case, by the same value co-op uses.
//!
//! # What is deliberately not here yet
//!
//! There is no per-participant FATE axis (bubble-revive, respawn at a
//! checkpoint, wait for a teammate). Nothing in the workspace has two
//! participants in one level yet, and the axis is unanswerable until something
//! does. Jon: *"we could build a new on death policy if we ever want something
//! more elaborate."* Grow this then; do not pre-shape it now.

use bevy::prelude::*;

/// **The game's death rules.** Stated once, beside its other rules.
///
/// Absent means the engine defaults below, which are the conservative answer for
/// a composition that has not thought about it: hold for nothing, reset nothing.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct DeathRules {
    /// Seconds a participant's death holds before its consequence runs — the
    /// window content may fill with presentation.
    ///
    /// ⛔ **this does NOT freeze the world**, and must never grow into that. In
    /// NSMB the other player is still playing while this one's death animation
    /// runs, so an interlude that held the world would be wrong the moment a
    /// second participant existed. It is strictly THIS participant's window. A
    /// game that wants the world to hold still claims a time scale, which is a
    /// separate statement anything can make.
    pub interlude: f32,
    /// When the level goes back to how it started.
    pub level_reset: LevelReset,
}

impl Default for DeathRules {
    fn default() -> Self {
        Self {
            interlude: 0.0,
            level_reset: LevelReset::Never,
        }
    }
}

impl DeathRules {
    /// The classic single-player answer, and the one most games want: hold for
    /// the death beat, then put the level back.
    ///
    /// Named as a constructor because "very very easy to opt in" is a design
    /// requirement (Jon), not a nicety — a game should not have to know the
    /// roster vocabulary to get the behaviour every platformer has.
    pub fn replay_level_after(interlude: f32) -> Self {
        Self {
            interlude,
            level_reset: LevelReset::WhenNoParticipantRemains,
        }
    }
}

/// **When does the LEVEL go back?** A question about the roster, never about one
/// death.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LevelReset {
    /// Never on a death. A versus round, an arena, a multiplayer level that
    /// outlives its participants — the ruleset acts on the death itself and the
    /// level is none of death's business.
    #[default]
    Never,
    /// When the participant who died was the last one standing.
    ///
    /// ⭐ **NSMB and single-player Mary-O are this same value.** Co-op resets
    /// when a player dies and every other player is already dead; a roster of one
    /// meets that condition on the first death.
    WhenNoParticipantRemains,
}

/// **This participant's attempt is over, and the world must not act on the
/// body.**
///
/// While it is held: no control frame, no hurtbox, nothing teleports the body,
/// nothing heals it, nothing resets its anim, and **the world's reset gates skip
/// it**.
///
/// ⛔ **that last clause is the whole defect this component exists to close.**
/// The blast-zone gate is a POSITION TEST that re-fires every tick a body is
/// past the margin. The ACTOR path has always known this — its gate is written
/// `em.health.alive() && …`, with a comment saying so — and the PLAYER path
/// never got the same guard. Measured 2026-08-09: one fall into a Mary-O pit
/// re-flagged the reset on 192 of the 192 frames of the death beat, and in the
/// hosted app each of those was a full room-feature reset while the death music
/// played.
///
/// ⭐ **and it makes "she dies where she died" free.** The pose pin, the anim
/// re-arm, the spent-life latch and the scripted control/immunity grants in
/// Mary-O's death beat all existed to claw the body back from a respawn that had
/// already happened. Nothing moves her now, so there is nothing to pin.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutOfPlay;

/// **The open window between a participant's death and its consequence.**
///
/// Rides the BODY rather than the level, because it is that participant's
/// window: in co-op one player's death beat plays while the other is still
/// running the level.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct DeathInterlude {
    /// Seconds left before the consequence runs. The window closes at zero.
    pub remaining: f32,
    /// This window still owes the game its consequence. Armed when the window
    /// opens, spent once when it closes.
    ///
    /// ⛔ **the debt is why the window is not simply REMOVED when it closes**,
    /// and dropping it cost a rollback desync inside an hour. "Has the
    /// consequence already run for this death" has to be answerable from
    /// SNAPSHOT STATE: the level reset it triggers is a message written late in
    /// one frame and consumed early in the next, so a rewind across that
    /// boundary re-simulates a frame whose cause has already been deleted — the
    /// request is lost and the two branches diverge.
    ///
    /// As a debt the window carries, it rides the same snapshot as the rest of
    /// the window, and spending it is idempotent because it is a STATE rather
    /// than an observation of the previous frame. (Mary-O's deleted beat carried
    /// exactly this field, for exactly this reason, and its doc said so. It was
    /// right; only its owner was wrong.)
    pub consequence_pending: bool,
}

impl DeathInterlude {
    /// Is the window still open? While it is, the death row plays and the
    /// consequence has not run.
    pub fn open(&self) -> bool {
        self.remaining > 0.0
    }
}
