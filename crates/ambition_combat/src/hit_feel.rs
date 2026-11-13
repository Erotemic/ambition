//! How a landed hit answers in one game's rooms: how long both bodies freeze
//! and how far a strike pushes back the body that landed it (HIT-FEEL).
//!
//! Declared with [`DeclareRulesExt`](ambition_platformer2d_world::scoped_rules::DeclareRulesExt)
//! apart from [`CombatRules`](crate::rules::CombatRules), as
//! [`crate::landing_lag`] is: a game that declares no combat ruleset (Ambition)
//! still states how its hits feel. Its hitlag law is folded into
//! [`ResolvedCombatTuning::hitlag`](crate::rules::ResolvedCombatTuning::hitlag)
//! per live room; the recoil is answered by [`recoil_attackers_on_connects`].
//!
//! The engine's own hitlag law is a share of one reference freeze calibrated
//! on a platform fighter's damage (11 for an ordinary hit), so a game whose
//! ordinary hit deals 1 gets the floor of that law on every hit.

use bevy::prelude::*;

/// A game's hit feel for its own rooms.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitFeelRules {
    /// The freeze a connect costs both bodies. Absent: the ruleset's or the
    /// engine's law.
    #[serde(default)]
    pub hitlag: Option<ambition_platformer2d_core::hit_response::HitlagLaw>,
    /// Speed (px/s) a melee connect pushes an airborne attacker back from
    /// what it struck. 0: none.
    #[serde(default)]
    pub attacker_recoil: f32,
    /// How hard every hit throws a fresh body, as a multiplier on its launch
    /// base: [`CombatRules::base_knockback_scale`](crate::rules::CombatRules::base_knockback_scale)
    /// for a game that declares no combat ruleset. Absent: the ruleset's.
    #[serde(default)]
    pub knockback_scale: Option<f32>,
}

impl HitFeelRules {
    /// Each value the runtime cannot use, named by its field.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if !(self.attacker_recoil.is_finite() && self.attacker_recoil >= 0.0) {
            problems.push(format!("`hit_feel.attacker_recoil` is {}; it is a speed, finite and not negative", self.attacker_recoil));
        }
        if let Some(scale) = self.knockback_scale {
            if !(scale.is_finite() && scale >= 0.0) {
                problems.push(format!("`hit_feel.knockback_scale` is {scale}; it is a multiplier, finite and not negative"));
            }
        }
        if let Some(ambition_platformer2d_core::hit_response::HitlagLaw::Affine { base_s, per_damage_s, max_s }) = self.hitlag {
            if ![base_s, per_damage_s, max_s].iter().all(|s| s.is_finite() && *s >= 0.0) {
                problems.push("`hit_feel.hitlag` seconds are finite and not negative".into());
            }
        }
        problems
    }
}

/// How long a recoil's speed is held against the attacker's own drift,
/// after the connect's freeze.
const RECOIL_HOLD_S: f32 = 0.12;

/// Push each airborne attacker back from what its melee strike connected
/// with, by its room's [`HitFeelRules::attacker_recoil`]: its run, handed to
/// the motion gateway as a knockback's is (a body's own drift would overwrite
/// a speed written into its kinematics) and held for the freeze and
/// [`RECOIL_HOLD_S`].
///
/// ⛔ NOT ON THE GROUND. A grounded body's run law sets its speed toward its
/// stick every tick and brakes to a stop without one, so a pushed speed is
/// gone on the next tick (measured: 110 px/s through the freeze, 0 on the
/// first tick after it). Holding it there would take the run law away, which
/// is the recoil lock, and that reads as hitstun to every brain and breaks a
/// conversation: the wrong fact about the body that landed the hit.
pub fn recoil_attackers_on_connects(
    mut hits: MessageReader<crate::hitbox::ResolvedBodyHit>,
    rules: ambition_platformer2d_world::governing_rules::RulesOf<HitFeelRules>,
    mut bodies: Query<(
        &mut ambition_platformer2d_core::BodyKinematics,
        &ambition_platformer2d_core::BodyGroundState,
        Option<&mut ambition_platformer2d_core::BodyFlightState>,
    )>,
) {
    for hit in hits.read() {
        let Some(attacker) = hit.attacker.filter(|attacker| *attacker != hit.victim) else { continue };
        if !matches!(hit.source, crate::HitSource::Melee) || hit.damage <= 0 {
            continue;
        }
        let Some(speed) = rules.of(attacker).map(|rules| rules.attacker_recoil).filter(|speed| *speed > 0.0) else { continue };
        let Ok(victim) = bodies.get(hit.victim).map(|(kin, ..)| kin.pos) else { continue };
        let Ok((mut kin, ground, flight)) = bodies.get_mut(attacker) else { continue };
        if ground.on_ground {
            continue;
        }
        let run = if victim.x >= kin.pos.x { -speed } else { speed };
        match flight {
            Some(mut flight) => {
                flight.stage_launch(ambition_platformer2d_core::Vec2::new(run, kin.vel.y), true);
                flight.carried_run = run;
                flight.carried_hold = flight.carried_hold.max(hit.hitlag_seconds + RECOIL_HOLD_S);
            }
            None => kin.vel.x = run,
        }
    }
}
