//! Character-owned authored `smash_fighter` facet.
//!
//! This module owns what a character is when it fights, apart from its moves:
//! its fighter body and its weight. The moves, grab and throws included, are a
//! `moveset` file like every other fighter's; a second authoring road for the
//! grab would be a second place for its numbers to drift. Content-pack
//! validation rejects unknown fields before runtime.

use serde::{Deserialize, Serialize};

/// The `smash_fighter` schema this capability owns. Behind `content_pack`: a
/// game that never validates its content must not link a compiler.
#[cfg(feature = "content_pack")]
pub mod content_schema;

/// The capability that owns platform-fighter authoring.
///
/// A composition that installs the schema installs this capability, so "who owns a fighter's
/// authored values" has one answer that a tool can print.
pub const SMASH_FIGHTER_CAPABILITY: &str = "smash_fighter";

/// The authored FILE kind: one character's platform-fighter facet.
pub const SMASH_FIGHTER_SCHEMA: &str = "smash_fighter";

/// One character's platform-fighter values.
///
/// One file, one character — not a book keyed by id. A character package owns
/// its own file, so adding a fighter never edits another fighter's, and a merge
/// conflict names a character instead of a line number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmashFighterFacet {
    /// The character id these values belong to — the same string the game
    /// registers the character under.
    pub character: String,
    /// The BODY this character plays on when it is being a FIGHTER.
    ///
    /// ⭐⭐ A CHARACTER'S CATALOG ROW IS ITS FEEL EVERYWHERE IT APPEARS — a hub,
    /// a room, a stage — so a character that walks around a hub and also fights
    /// cannot state two gravities there. It states the second one HERE, and a
    /// composition hands it to the seat as `MatchParticipant::body`.
    ///
    /// ⛔ NOT a match-wide number. `MatchBody`'s own doc refuses a mode-owned
    /// gravity in advance, and it is right to: per-fighter gravity, fall speed
    /// and jump arc are what make a heavy heavy.
    ///
    /// `None` keeps whatever body the character already had, which is every
    /// facet authored before this field existed.
    #[serde(default)]
    pub body: Option<FighterBodyAuthoring>,
    /// How hard this fighter is to LAUNCH — the divisor in the launch law
    /// (`ambition_entity_catalog::launch::launch_speed`).
    ///
    /// ⭐ A CHARACTER OWNS ITS OWN WEIGHT. This lived in the Smash demo as a
    /// `match definition.id` table until 2026-08-31: a game-owned map from
    /// character id to an ordinary `Vitals` field the engine already owns, which
    /// is the exact shape `character-authoring-package.md` names as a falsifier.
    /// A heavy is heavy because its own package says so.
    ///
    /// Above 1.0 resists a launch, below 1.0 flies further; 1.0 is the reference
    /// body. `None` keeps whatever weight the character already had, which is
    /// every fighter that has not thought about it.
    #[serde(default)]
    pub knockback_weight: Option<f32>,
}

/// A fighter's body, as a PATCH over the body it would otherwise have.
///
/// ⭐ EVERY FIELD IS OPTIONAL BECAUSE A FIGHTER STATES ITS DIFFERENCES. A heavy
/// authors a gravity and a fall speed and says nothing about its jump; the
/// alternative — a full body per fighter — makes every author restate the shared
/// numbers and makes a later change to them unreachable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FighterBodyAuthoring {
    /// Downward acceleration along the local gravity axis (px/s²).
    pub gravity: Option<f32>,
    /// Terminal fall speed cap (px/s) — the edgeguard knob.
    pub max_fall_speed: Option<f32>,
    /// Ground/air acceleration toward the locomotion target (px/s²). This is the
    /// one that reads as WEIGHT: low values make a fighter build speed slowly
    /// and slide when it reverses.
    pub run_accel: Option<f32>,
    /// Top ground speed (px/s) — the gait.
    pub max_run_speed: Option<f32>,
    /// Grounded jump launch speed (px/s). Apex is `v²/(2·gravity)`.
    pub jump_speed: Option<f32>,
    /// Mid-air jump launch speed (px/s).
    pub double_jump_speed: Option<f32>,
    /// Mid-air jump COUNT. Needs the `AirJump` grant to have any effect — the
    /// grant lights the capability, this is the number of them.
    pub air_jumps: Option<u8>,
}

impl FighterBodyAuthoring {
    /// Layer what this fighter states onto the body it would otherwise have.
    pub fn over(
        &self,
        base: ambition_platformer2d_core::MovementTuning,
    ) -> ambition_platformer2d_core::MovementTuning {
        ambition_platformer2d_core::MovementTuning {
            gravity: self.gravity.unwrap_or(base.gravity),
            max_fall_speed: self.max_fall_speed.unwrap_or(base.max_fall_speed),
            run_accel: self.run_accel.unwrap_or(base.run_accel),
            max_run_speed: self.max_run_speed.unwrap_or(base.max_run_speed),
            jump_speed: self.jump_speed.unwrap_or(base.jump_speed),
            double_jump_speed: self.double_jump_speed.unwrap_or(base.double_jump_speed),
            air_jumps: self.air_jumps.unwrap_or(base.air_jumps),
            ..base
        }
    }

    /// Did this body state anything at all?
    fn states_nothing(&self) -> bool {
        *self == Self::default()
    }

    fn problems(&self, out: &mut Vec<String>) {
        if self.states_nothing() {
            out.push(
                "`body` is present and states no number, so it declares a fighter body and                  means nothing. Author at least one field or remove it"
                    .to_string(),
            );
        }
        for (name, value) in [
            ("gravity", self.gravity),
            ("max_fall_speed", self.max_fall_speed),
            ("run_accel", self.run_accel),
            ("max_run_speed", self.max_run_speed),
            ("jump_speed", self.jump_speed),
            ("double_jump_speed", self.double_jump_speed),
        ] {
            let Some(value) = value else { continue };
            // ⛔ POSITIVE, not merely finite: every one of these is a MAGNITUDE
            // the kernel scales by, and a zero or negative gait, gravity or jump
            // is a body that cannot move rather than a slow one.
            if !value.is_finite() || value <= 0.0 {
                out.push(format!(
                    "`body.{name}` is {value}, and every number here is a magnitude the                      movement kernel scales by — zero or negative is a body that cannot                      move rather than a slow one"
                ));
            }
        }
    }
}

/// Every character's facet in one pack, keyed by character id.
pub type SmashFighterBook = std::collections::BTreeMap<String, SmashFighterFacet>;

impl SmashFighterFacet {
    /// Everything this facet can SAY that the runtime cannot USE.
    ///
    /// The list is deliberately not a taste filter. Every entry names a value
    /// whose consequence is that some authored thing never happens at all —
    /// a reach that cannot overlap, an impact past the end of its own move, a
    /// release the captive never reaches — because that is the class of fault
    /// that is invisible in a playtest and looks like the mechanic being bad.
    ///
    /// what is NOT here: balance. A 400-knockback pummel and a two-second grab
    /// are both usable and both terrible, and refusing them would make this
    /// module the designer.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.character.trim().is_empty() {
            out.push("`character` is empty, so nothing can look this facet up".to_string());
        }
        if let Some(body) = &self.body {
            body.problems(&mut out);
        }
        // ⛔ POSITIVE, not merely finite, for the same reason the body's
        // magnitudes are: the launch law DIVIDES by this. Zero is a division
        // by zero and a negative weight launches a fighter toward the attacker.
        if let Some(weight) = self.knockback_weight {
            if !weight.is_finite() || weight <= 0.0 {
                out.push(format!(
                    "`knockback_weight` is {weight}, and the knockback term DIVIDES by it — \
                     zero or negative is not a heavy fighter, it is a broken launch"
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
