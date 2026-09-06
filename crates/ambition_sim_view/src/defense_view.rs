//! Presentation-facing semantic causes of body untouchability.
//!
//! Hit eligibility remains owned by `ambition_combat::util::body_vulnerable`.
//! This module preserves WHY that gate is closed so presentation policy can
//! select cues without re-deriving combat rules or growing exception booleans.

use ambition_characters::actor::{BodyCombat, Invulnerability};
use ambition_platformer2d_core::{BodyMotionFacts, BodyShieldState};
pub use ambition_platformer2d_shared_tangle::gameplay_presentation::DefenseCueCauses;

/// Project canonical simulation state into presentation-facing defense causes.
///
/// This does not answer whether the body is hittable. Callers publish that
/// separately from the canonical damage gate. New gameplay invulnerability
/// reasons must be mapped here deliberately before presentation can opt into
/// them; unknown gameplay reasons therefore never acquire a visual effect by
/// accident.
pub fn defense_cue_causes(
    invulnerable: Invulnerability,
    motion: Option<&BodyMotionFacts>,
    shield: &BodyShieldState,
    combat: &BodyCombat,
    respawn_grace: bool,
) -> DefenseCueCauses {
    let mut causes = DefenseCueCauses::NONE;

    if invulnerable.holds(Invulnerability::TRANSFORMING) {
        causes = causes.union(DefenseCueCauses::TRANSFORMING);
    }
    if invulnerable.holds(Invulnerability::EMPOWERED) {
        causes = causes.union(DefenseCueCauses::EMPOWERED);
    }
    if invulnerable.holds(Invulnerability::SCRIPTED) {
        causes = causes.union(DefenseCueCauses::SCRIPTED);
    }
    if invulnerable.holds(Invulnerability::MOVE) {
        causes = causes.union(DefenseCueCauses::MOVE_IFRAME);
    }

    if let Some(motion) = motion {
        if motion.dodge_rolling || motion.air_dodging {
            causes = causes.union(DefenseCueCauses::DODGE);
        }
        if motion.ledge_intangible {
            causes = causes.union(DefenseCueCauses::LEDGE);
        }
        if motion.getup_invulnerable {
            causes = causes.union(DefenseCueCauses::GETUP);
        }
    }
    if shield.parrying() {
        causes = causes.union(DefenseCueCauses::PARRY);
    }
    if !combat.vulnerable() {
        causes = causes.union(DefenseCueCauses::DAMAGE_IFRAME);
    }
    if respawn_grace {
        causes = causes.union(DefenseCueCauses::RESPAWN);
    }
    causes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_and_maneuver_causes_compose_in_one_snapshot() {
        let mut invulnerable = Invulnerability::none();
        invulnerable.set(Invulnerability::EMPOWERED, true);
        invulnerable.set(Invulnerability::MOVE, true);
        let motion = BodyMotionFacts {
            air_dodging: true,
            ledge_intangible: true,
            ..BodyMotionFacts::default()
        };
        let shield = BodyShieldState::default();
        let combat = BodyCombat::default();

        let causes = defense_cue_causes(invulnerable, Some(&motion), &shield, &combat, false);
        assert!(causes.intersects(DefenseCueCauses::EMPOWERED));
        assert!(causes.intersects(DefenseCueCauses::MOVE_IFRAME));
        assert!(causes.intersects(DefenseCueCauses::DODGE));
        assert!(causes.intersects(DefenseCueCauses::LEDGE));
    }

    #[test]
    fn respawn_protection_has_its_own_presentation_cause() {
        let causes = defense_cue_causes(
            Invulnerability::none(),
            None,
            &BodyShieldState::default(),
            &BodyCombat::default(),
            true,
        );
        assert!(causes.intersects(DefenseCueCauses::RESPAWN));
        assert!(!causes.intersects(DefenseCueCauses::EMPOWERED));
    }

    #[test]
    fn parry_and_post_hit_iframe_are_preserved_as_distinct_causes() {
        let mut shield = BodyShieldState::default();
        shield.active = true;
        shield.parry_window_timer = 0.1;
        let combat = BodyCombat {
            damage_invuln_timer: 0.2,
            ..BodyCombat::default()
        };

        let causes = defense_cue_causes(Invulnerability::none(), None, &shield, &combat, false);
        assert!(causes.intersects(DefenseCueCauses::PARRY));
        assert!(causes.intersects(DefenseCueCauses::DAMAGE_IFRAME));
    }
}
