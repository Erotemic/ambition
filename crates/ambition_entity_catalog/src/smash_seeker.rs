//! Authored payload for the seeker: a shot that locks one foe and follows it.
//! The ruleset owns the flight (`ambition_demo_smash_modules::seeker`); this
//! module only defines the move payload and its validation.

use serde::{Deserialize, Serialize};

use crate::ParamValue;

/// The authored effect key.
pub const SEEKER: &str = "smash.seeker";

/// Authored parameters of one seeker shot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeekerParams {
    /// How fast it travels, in world px per second. Constant: it turns and
    /// never speeds up, so a foe who moves across its path outruns its turn.
    pub speed: f32,
    /// Maximum turn rate, in degrees per second.
    pub turn_rate_deg: f32,
    /// How far ahead of the caster it looks for the foe it locks, in world
    /// px. Only a foe on the side the caster faces is locked.
    pub lock_range: f32,
    /// Seconds before it fades on its own.
    pub lifetime_s: f32,
    /// Effect row drawn along its path.
    pub trail_vfx: String,
    /// Seconds between trail marks.
    pub trail_every_s: f32,
    /// Damage to the rival it reaches.
    pub damage: i32,
    /// Half-extent of its contact box.
    pub radius: f32,
    /// Hitbox feel multiplier, not launch speed.
    pub knockback: f32,
    /// Where it appears, body-local (`+x` toward facing, `+y` gravity-down).
    pub offset: (f32, f32),
    /// How long its hit stays live where it reaches somebody, in seconds.
    pub hit_s: f32,
}

impl SeekerParams {
    /// Every semantic problem of this payload.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let positive = |name: &str, value: f32, out: &mut Vec<String>| {
            if !(value.is_finite() && value > 0.0) {
                out.push(format!("`{name}` is {value}; it must be a finite number above 0"));
            }
        };
        positive("speed", self.speed, &mut out);
        positive("turn_rate_deg", self.turn_rate_deg, &mut out);
        positive("lock_range", self.lock_range, &mut out);
        positive("lifetime_s", self.lifetime_s, &mut out);
        positive("trail_every_s", self.trail_every_s, &mut out);
        positive("radius", self.radius, &mut out);
        positive("hit_s", self.hit_s, &mut out);
        if self.trail_vfx.trim().is_empty() {
            out.push("`trail_vfx` is empty, so a shot that turns is drawn nowhere".to_string());
        }
        if self.damage <= 0 {
            out.push(format!("`damage` is {}, so the shot that reaches its foe does nothing", self.damage));
        }
        if !(self.knockback.is_finite() && (0.0..=8.0).contains(&self.knockback)) {
            out.push(format!("`knockback` is {}; it is a feel multiplier from 0 to 8", self.knockback));
        }
        out
    }
}

/// Admission's check of a seeker's params: they hydrate, and
/// [`SeekerParams::problems`] finds nothing.
pub fn check_seeker_params(params: &ParamValue) -> Result<(), String> {
    let typed: SeekerParams = params.hydrate().map_err(|error| error.to_string())?;
    let problems = typed.problems();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> SeekerParams {
        SeekerParams {
            speed: 420.0,
            turn_rate_deg: 140.0,
            lock_range: 520.0,
            lifetime_s: 1.6,
            trail_vfx: "spark_trail".into(),
            trail_every_s: 0.06,
            damage: 6,
            radius: 10.0,
            knockback: 0.9,
            offset: (24.0, -22.0),
            hit_s: 0.1,
        }
    }

    #[test]
    fn admission_refuses_a_seeker_that_cannot_turn_lock_or_hurt() {
        let check = |p: SeekerParams| check_seeker_params(&ParamValue::from_typed(&p).unwrap());
        assert_eq!(check(params()), Ok(()));
        assert!(check(SeekerParams { turn_rate_deg: 0.0, ..params() }).is_err());
        assert!(check(SeekerParams { lock_range: f32::NAN, ..params() }).is_err());
        assert!(check(SeekerParams { damage: 0, ..params() }).is_err());
        assert!(check(SeekerParams { trail_vfx: " ".into(), ..params() }).is_err());
    }
}
