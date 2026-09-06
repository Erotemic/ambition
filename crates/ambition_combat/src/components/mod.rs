//! ECS-native feature components.
//!
//! Gameplay feature families are represented as normal Bevy entities/components,
//! paired with typed messages for cross-system effects.

use super::*;

mod actors;
mod features;

pub use actors::*;
pub use features::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_aabb_round_trips_center_and_size() {
        let feature =
            CenteredAabb::from_center_size(ae::Vec2::new(10.0, 20.0), ae::Vec2::new(8.0, 6.0));

        assert_eq!(feature.center, ae::Vec2::new(10.0, 20.0));
        assert_eq!(feature.half_size, ae::Vec2::new(4.0, 3.0));
        assert_eq!(feature.size(), ae::Vec2::new(8.0, 6.0));
        assert_eq!(
            feature.aabb(),
            ae::Aabb::new(ae::Vec2::new(10.0, 20.0), ae::Vec2::new(4.0, 3.0))
        );
    }

    #[test]
    fn actor_faction_player_is_player_side_others_are_not() {
        assert!(ActorFaction::Player.is_player_side());
        assert!(!ActorFaction::Enemy.is_player_side());
        assert!(!ActorFaction::Npc.is_player_side());
        assert!(!ActorFaction::Boss.is_player_side());
        assert!(!ActorFaction::Neutral.is_player_side());
    }

    #[test]
    fn actor_faction_enemy_and_boss_are_hostile_side() {
        assert!(ActorFaction::Enemy.is_hostile_side());
        assert!(ActorFaction::Boss.is_hostile_side());
        assert!(!ActorFaction::Player.is_hostile_side());
        assert!(!ActorFaction::Npc.is_hostile_side());
        assert!(!ActorFaction::Neutral.is_hostile_side());
    }

    #[test]
    fn actor_faction_default_is_player() {
        assert_eq!(ActorFaction::default(), ActorFaction::Player);
    }

    #[test]
    fn pogo_policy_defaults_to_damageable() {
        assert_eq!(PogoPolicy::default(), PogoPolicy::FromDamageable);
    }
}

/// Runtime combat consequences derived from authored character death traits.
///
/// Movement verbs are not mirrored here; body movement capability remains authoritative in
/// [`ae::AbilitySet`].
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct CombatCapabilities {
    /// Detonates at the corpse on death (Enemy-faction blast), so a
    /// point-blank kill is punished.
    pub explodes_on_death: bool,
    pub divides_into: Option<String>,
    /// A fast charge stopped dead by a wall destroys this actor.
    pub charge_crash_explodes: bool,
    /// Damage never kills (training dummy with an effectively
    /// infinite pool).
    pub never_dies: bool,
    /// Whether the corpse leaves what the body was HOLDING as a wieldable
    /// `GroundItem` — the "steal the enemy's weapon" rule.
    ///
    /// a policy, not an item. The item comes from the body's live
    /// [`crate::held_items::HeldItem`] at death, so a body that changed weapons
    /// drops the one it actually has.
    pub drops_held_item: bool,
}

impl From<&ambition_characters::actor::CharacterDeathTraits> for CombatCapabilities {
    /// Lower authored death traits into the combat-owned runtime component.
    /// Explicit field mapping keeps ownership changes compile-visible.
    fn from(traits: &ambition_characters::actor::CharacterDeathTraits) -> Self {
        let ambition_characters::actor::CharacterDeathTraits {
            explodes_on_death,
            divides_into,
            charge_crash_explodes,
            never_dies,
            drops_held_item,
        } = traits;
        Self {
            explodes_on_death: *explodes_on_death,
            divides_into: divides_into.clone(),
            charge_crash_explodes: *charge_crash_explodes,
            never_dies: *never_dies,
            drops_held_item: *drops_held_item,
        }
    }
}

/// Composable per-body movement tuning resolved from baseline, inheritance, and
/// [`BodyMovementPatch`] overrides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyMovementTuning {
    /// Downward acceleration along the local gravity axis (px/s²).
    pub gravity: f32,
    /// Terminal fall speed cap (px/s).
    pub max_fall_speed: f32,
    /// Ground/air run acceleration toward the locomotion target (px/s²).
    pub run_accel: f32,
    /// Launch speed of a grounded jump, opposite gravity (px/s).
    pub jump_speed: f32,
    /// Launch speed of a mid-air (double) jump (px/s).
    pub double_jump_speed: f32,
}

impl BodyMovementTuning {
    pub const BASELINE: Self = Self {
        gravity: 1450.0,
        max_fall_speed: 760.0,
        run_accel: 650.0,
        jump_speed: 520.0,
        double_jump_speed: 430.0,
    };

    /// Build shared engine movement tuning for the body's gravity/run/fall parameters.
    pub fn spine_tuning(&self, max_run_speed: f32) -> ae::MovementTuning {
        ae::MovementTuning {
            gravity: self.gravity,
            run_accel: self.run_accel,
            air_accel: self.run_accel,
            ground_friction: 0.0,
            air_friction: 0.0,
            max_run_speed,
            max_fall_speed: self.max_fall_speed,
            ..ae::MovementTuning::default()
        }
    }

    /// Extend [`Self::spine_tuning`] with this body's authored jump impulses.
    pub fn body_tuning(&self, max_run_speed: f32) -> ae::MovementTuning {
        ae::MovementTuning {
            jump_speed: self.jump_speed,
            double_jump_speed: self.double_jump_speed,
            ..self.spine_tuning(max_run_speed)
        }
    }
}

impl Default for BodyMovementTuning {
    fn default() -> Self {
        Self::BASELINE
    }
}

/// Partial authored movement override; `None` inherits and `Some` replaces the base value.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct BodyMovementPatch {
    pub gravity: Option<f32>,
    pub max_fall_speed: Option<f32>,
    pub run_accel: Option<f32>,
    pub jump_speed: Option<f32>,
    pub double_jump_speed: Option<f32>,
}

impl BodyMovementPatch {
    /// Layer this patch onto an already resolved base tuning.
    pub fn apply_onto(&self, base: BodyMovementTuning) -> BodyMovementTuning {
        BodyMovementTuning {
            gravity: self.gravity.unwrap_or(base.gravity),
            max_fall_speed: self.max_fall_speed.unwrap_or(base.max_fall_speed),
            run_accel: self.run_accel.unwrap_or(base.run_accel),
            jump_speed: self.jump_speed.unwrap_or(base.jump_speed),
            double_jump_speed: self.double_jump_speed.unwrap_or(base.double_jump_speed),
        }
    }
}


/// Combat-owned projection of authored per-body tuning used by damage/hit resolution.
/// Bodies without it use the reference defaults.
#[derive(Component, Clone, Debug)]
pub struct CombatTuning {
    /// Knockback weight (CM1): heavier bodies launch less under the same growth
    /// term. `1.0` is the reference body.
    pub weight: f32,
    /// Per-actor scale on the baseline enemy attack cooldown
    /// (`ENEMY_ATTACK_COOLDOWN * attack_cooldown_mult` paces the brain's next
    /// swing). The player carries no cooldown floor (`1.0` is inert for it —
    /// bodies without the component skip the floor entirely).
    pub attack_cooldown_mult: f32,
    /// Sprite-catalog id whose AUTHORED per-animation attack polygons the
    /// strike paths resolve. Controllable bodies use `WornCharacter`; `None`
    /// remains only for content-free fixtures. Combat forwards the stable id to
    /// the App-local authored-volume resolver.
    pub sprite_character_id: Option<String>,
    /// Victim-owned hurt sound and spray/debris response.
    pub hurt_feedback: ambition_vfx::HurtFeedback,
}

impl Default for CombatTuning {
    fn default() -> Self {
        Self {
            weight: 1.0,
            attack_cooldown_mult: 1.0,
            sprite_character_id: None,
            hurt_feedback: ambition_vfx::HurtFeedback::ENEMY,
        }
    }
}

/// TODO(compat-remove): migrate combat callers to `ambition_characters::actor::DeathPolicy`,
/// then delete this re-export.
pub use ambition_characters::actor::DeathPolicy;
