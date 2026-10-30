//! Character-owned authored `smash_fighter` facet.
//!
//! This module owns what a character is when it fights, apart from its moves:
//! its fighter body, its weight, and the damage its moves deal on a
//! platform-fighter stage. The moves themselves, grab and throws included, are
//! a `moveset` file like every other fighter's; a second authoring road for a
//! move's frames or geometry would be a second place for them to drift.
//! Content-pack validation rejects unknown fields before runtime.

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
    /// The damage this fighter's moves deal on a platform-fighter stage, where
    /// damage is percent. Folded under [`FIGHTER_DAMAGE`]; a match applies it
    /// only when it plays in that scale.
    ///
    /// ⭐ A MOVE HAS ONE SHAPE AND ONE DAMAGE PER GAME. The moveset states the
    /// frames, the geometry, and the damage of the character's HOME game. A
    /// number that means something different on this stage is stated here, so
    /// each game balances its own damage and neither game rewrites the other.
    ///
    /// Keyed by move id. The list has one value for each volume of that move
    /// that deals damage, in authoring order (window by window). A move this
    /// map does not name keeps its moveset damage.
    #[serde(default)]
    pub move_damage: crate::move_damage::MoveDamage,
    /// The move each verb names on a platform-fighter stage, where that is
    /// not the move its moveset names. Folded under [`FIGHTER_DAMAGE`] with
    /// the damage; a match that plays in that scale applies both.
    ///
    /// The moveset's own `verbs` are the character's HOME game. A character
    /// whose home attack is not a platform fighter's (the robot at home
    /// swings as it walks; a fighter's jab roots, and a fighter has a dash
    /// attack) names here the moves a fighter's buttons play. Each move is
    /// still in the one moveset file: this map has ids, and no frame.
    #[serde(default)]
    pub verbs: crate::move_damage::VerbMoves,
    /// Each move's CLOCK on a platform-fighter stage (first strike, last
    /// strike, total, landing lag), where that differs from its moveset's.
    /// Folded under [`FIGHTER_DAMAGE`] with the damage and the verbs. The
    /// home game's moves keep their own pace; the fighter's commit. See
    /// `crate::move_damage::move_timing_over`.
    #[serde(default)]
    pub move_timing: crate::move_damage::MoveTimings,
    /// The factor each move's BASE knockback is scaled by on a platform-fighter
    /// stage (`1.25`: a quarter harder at 0%), where its home launch leaves the
    /// swing with no follow-up. Folded under [`FIGHTER_DAMAGE`] with the rest.
    /// See `crate::move_damage::move_knockback_over`.
    #[serde(default)]
    pub move_knockback: crate::move_damage::MoveKnockback,
}

/// The damage scale this capability folds a facet's `move_damage` under. A
/// composition that plays its moves in platform-fighter percent gives it to
/// the match as `MatchRules::move_damage`, as it gives `content_schema::fighter_body`
/// to the seat; the match and the kit resolver read only the scale.
pub const FIGHTER_DAMAGE: crate::move_damage::DamageScale =
    crate::move_damage::DamageScale("smash_fighter");

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
    /// How the press becomes a hop: `SquatDecided` is the platform fighter's
    /// (the squat decides full or short, the air does not).
    pub jump_law: Option<ambition_platformer2d_core::AxisJumpLaw>,
    /// Speed of the first step out of standing (px/s), the burst a direction
    /// change during the initial dash also gets. `0.0` is the run speed.
    pub initial_dash_speed: Option<f32>,
    /// See `MovementTuning::walk_speed`: the fighter's walk, a gait of its
    /// own under the dash threshold. `None` keeps the match's.
    #[serde(default)]
    pub walk_speed: Option<f32>,
    /// Ground deceleration with the stick let go (px/s²): TRACTION, how far a
    /// stopped run slides.
    pub ground_friction: Option<f32>,
    /// Acceleration toward the stick in the air (px/s²): air control.
    pub air_accel: Option<f32>,
    /// Top drift speed in the air (px/s). `0.0` is the run speed.
    pub max_air_speed: Option<f32>,
    /// Deceleration in the air with the stick let go (px/s²): AIR FRICTION.
    /// Low is a drift that carries until it is steered out of.
    pub air_stop_assist: Option<f32>,
    /// Fall speed cap while fast-falling (px/s).
    pub fast_fall_speed: Option<f32>,
    /// How fast a fast fall reaches that cap (px/s²).
    pub fast_fall_accel: Option<f32>,
    /// Ground roll speed (px/s); its distance is this times `dodge_roll_time`.
    pub dodge_roll_speed: Option<f32>,
    /// Ground roll duration (s): how long it travels and is protected.
    pub dodge_roll_time: Option<f32>,
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
            jump_law: self.jump_law.unwrap_or(base.jump_law),
            initial_dash_speed: self.initial_dash_speed.unwrap_or(base.initial_dash_speed),
            walk_speed: self.walk_speed.unwrap_or(base.walk_speed),
            ground_friction: self.ground_friction.unwrap_or(base.ground_friction),
            air_accel: self.air_accel.unwrap_or(base.air_accel),
            max_air_speed: self.max_air_speed.unwrap_or(base.max_air_speed),
            air_stop_assist: self.air_stop_assist.unwrap_or(base.air_stop_assist),
            fast_fall_speed: self.fast_fall_speed.unwrap_or(base.fast_fall_speed),
            fast_fall_accel: self.fast_fall_accel.unwrap_or(base.fast_fall_accel),
            dodge_roll_speed: self.dodge_roll_speed.unwrap_or(base.dodge_roll_speed),
            dodge_roll_time: self.dodge_roll_time.unwrap_or(base.dodge_roll_time),
            ..base
        }
    }

    /// Did this body state anything at all?
    fn states_nothing(&self) -> bool {
        *self == Self::default()
    }

    pub fn problems(&self, out: &mut Vec<String>) {
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
            ("air_accel", self.air_accel),
            ("fast_fall_speed", self.fast_fall_speed),
            ("fast_fall_accel", self.fast_fall_accel),
            ("dodge_roll_speed", self.dodge_roll_speed),
            ("dodge_roll_time", self.dodge_roll_time),
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
        // These four may be zero, each meaning something: the run speed, no
        // traction, the run speed again, no air friction.
        for (name, value) in [
            ("initial_dash_speed", self.initial_dash_speed),
            ("walk_speed", self.walk_speed),
            ("ground_friction", self.ground_friction),
            ("max_air_speed", self.max_air_speed),
            ("air_stop_assist", self.air_stop_assist),
        ] {
            let Some(value) = value else { continue };
            if !value.is_finite() || value < 0.0 {
                out.push(format!("`body.{name}` is {value}; it is finite and not negative"));
            }
        }
        if let Some(ambition_platformer2d_core::AxisJumpLaw::SquatDecided(law)) = self.jump_law {
            let frac = law.short_hop_speed_frac;
            if !(frac > 0.0 && frac <= 1.0) {
                out.push(format!(
                    "`body.jump_law`'s `short_hop_speed_frac` is {frac}; a short hop is a \
                     fraction of the full hop's launch, above 0 and at most 1"
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
        for (move_id, values) in &self.move_damage {
            // ⛔ AT LEAST 1: only a volume that deals damage has a slot here, and
            // a zero would turn that hit into a push.
            if values.is_empty() || values.iter().any(|value| *value < 1) {
                out.push(format!(
                    "`move_damage.{move_id}` is {values:?}; it needs one value of at least 1 \
                     for each volume of the move that deals damage"
                ));
            }
        }
        for (move_id, timing) in &self.move_timing {
            timing.problems(move_id, &mut out);
        }
        for (move_id, scale) in &self.move_knockback {
            if !(scale.is_finite() && *scale > 0.0) {
                out.push(format!("`move_knockback.{move_id}` is {scale}; it is a positive factor"));
            }
        }
        for (verb, move_id) in &self.verbs {
            if verb.trim().is_empty() || move_id.trim().is_empty() {
                out.push(format!("`verbs` has the entry `{verb}`: `{move_id}`; a verb and a move id are both names"));
            }
        }
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
