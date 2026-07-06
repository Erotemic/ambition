//! ECS-native feature components.
//!
//! Gameplay feature families are represented as normal Bevy entities/components,
//! paired with typed messages for cross-system effects.

use super::*;

mod actors;
mod features;
// spawn BUNDLES moved to `features::ecs::actor_bundles` (E2): spawn
// machinery is features-side; the bundles reference combat components
// through the legal features → combat arrow.

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

/// Per-actor combat capabilities, derived from the actor's authored
/// archetype DATA at spawn (`character_archetypes.ron`) and attached as a
/// component so generic combat systems can branch on capabilities
/// instead of matching named archetype enums. The content layer
/// derives it; the kit only defines the vocabulary.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct CombatCapabilities {
    /// Detonates at the corpse on death (Enemy-faction blast), so a
    /// point-blank kill is punished.
    pub explodes_on_death: bool,
    /// Splits into offspring on death.
    pub divides_on_death: bool,
    /// A fast charge stopped dead by a wall destroys this actor.
    pub charge_crash_explodes: bool,
    /// Damage never kills (training dummy with an effectively
    /// infinite pool).
    pub never_dies: bool,
    /// Weapon dropped at the corpse as a wieldable `GroundItem` (the
    /// "steal the enemy's weapon" rule), resolved from authored data
    /// at spawn.
    pub drops_held_item: Option<ambition_characters::brain::HeldItemSpec>,
    /// Movement kit: this body can **blink** (short-range collision-clamped
    /// teleport). The body-side gate for the `blink` intent (invariant I3) — the
    /// controller (AI brain or possessing human) only *attempts* a blink; the
    /// body resolves it only when this is set and its blink cooldown is ready, so
    /// the player kit is a per-body capability, never gated on "is the player".
    pub can_blink: bool,
    /// Movement kit: this body can **fly** — toggle between grounded and free-
    /// mover (gravity-free) locomotion. The body-side gate for the `fly_toggle`
    /// intent (I3): the controller decides WHEN to switch modes (the brain
    /// prefers grounded and flies to traverse; a possessing human presses it),
    /// the body flips its own gravity mode.
    pub can_fly: bool,
    /// Movement/defense kit: this body can **reactive-block** with a shield. The
    /// body-side gate for the `shield_held` intent (I3): the controller decides
    /// WHEN to raise the guard (the brain shields a perceived lunge it won't blink;
    /// a possessing human holds the button), the body enforces the block — a
    /// guarded hit from the faced side is negated (the same frame-agnostic
    /// directional rule the player's shield uses, `shield_blocks_hit`). Never
    /// gated on "is the player".
    pub can_shield: bool,
    /// Movement kit: this body can **dash** — a short burst above its walk speed.
    /// The body-side gate for the `dash_pressed` intent (I3): the controller
    /// decides WHEN to dash (the brain dashes to close a gap; a possessing human
    /// presses it), the body owns the burst. For a grounded actor this enables the
    /// **shared movement pipeline's dash limb** (the same real dash impulse the
    /// player runs — `ActorBody::from_caps` flips on the `dash` ability), not a
    /// bespoke actor burst. A body WITHOUT this capability still moves at its walk
    /// speed on a Dash action (graceful fallback), it just doesn't dash.
    pub can_dash: bool,
}

/// Composable per-body movement knobs (gravity, run, jump, fall cap) — the
/// physics every body's spine runs on. Resolved hierarchically per archetype:
/// `BASELINE ← inherited archetype's resolved tuning ← this archetype's patch`
/// (see [`BodyMovementPatch`]). Today this feeds the actor integrator (replacing
/// the old hardcoded `ENEMY_*` constants); the roadmap's unification consumes it
/// as the per-body physics when actors run the shared player pipeline, so a heavy
/// brute can fall harder and a floaty wisp drift, all from data.
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
    /// The generic body baseline — the values every actor used to hardcode. An
    /// archetype that authors no movement overrides resolves to exactly this, so
    /// the data move is behavior-preserving until a row opts to differ.
    pub const BASELINE: Self = Self {
        gravity: 1450.0,
        max_fall_speed: 760.0,
        run_accel: 650.0,
        jump_speed: 520.0,
        double_jump_speed: 430.0,
    };

    /// Build the engine `MovementTuning` the grounded **spine** runs on for this
    /// body: the composed gravity/run/fall knobs over the bare default, with the
    /// body's run cap and gravity frame, frictionless (a grounded actor carries no
    /// friction limbs — friction lives in the rich pipeline this body adopts next).
    /// `gravity_scale` lets a partially-floating body damp its gravity. One movement
    /// source per body — the seam the unification's full pipeline also consumes.
    pub fn spine_tuning(
        &self,
        max_run_speed: f32,
        gravity_dir: ae::Vec2,
        gravity_scale: f32,
    ) -> ae::MovementTuning {
        ae::MovementTuning {
            gravity: self.gravity * gravity_scale,
            gravity_dir,
            run_accel: self.run_accel,
            air_accel: self.run_accel,
            ground_friction: 0.0,
            air_friction: 0.0,
            max_run_speed,
            max_fall_speed: self.max_fall_speed,
            ..ae::MovementTuning::default()
        }
    }

    /// Build the engine `MovementTuning` the **full player pipeline** runs on for
    /// this body. Extends [`Self::spine_tuning`] with the body's jump speeds (the
    /// rich pipeline owns jumping, where the bare spine left it to the caller), so
    /// a body routed through `update_body_*_with_clusters` jumps with its OWN
    /// authored impulse instead of the player default. Dash/blink/ledge distances
    /// stay at the engine default for now — gated off until the body's ability
    /// mask opts in.
    pub fn body_tuning(
        &self,
        max_run_speed: f32,
        gravity_dir: ae::Vec2,
        gravity_scale: f32,
    ) -> ae::MovementTuning {
        ae::MovementTuning {
            jump_speed: self.jump_speed,
            double_jump_speed: self.double_jump_speed,
            ..self.spine_tuning(max_run_speed, gravity_dir, gravity_scale)
        }
    }
}

impl Default for BodyMovementTuning {
    fn default() -> Self {
        Self::BASELINE
    }
}

/// A partial override layer authored on an archetype (RON). Every knob is
/// `Option`: `None` inherits (from the parent archetype or the baseline), `Some`
/// overrides. This is what makes the tuning COMPOSE — a row specifies only what
/// differs, and `inherits` lets one archetype extend another.
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
    /// Layer this patch onto a resolved base: each `Some` knob overrides, each
    /// `None` keeps the base. The single composition primitive the hierarchical
    /// resolver folds along the inheritance chain.
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

// `ActorTuning` moved to `features::ecs::actor_tuning` (E2): the actor
// archetype tuning block (it names projectile visual vocabulary); combat
// reads only the projected `CombatTuning` below.

/// Combat-owned per-body tuning read by the damage paths (E2 verdict b).
///
/// The CM1 knockback-scaling law needs the victim's `weight`, an actor fact
/// authored on `ActorTuning`. Combat may NOT import the sim-heart `ActorConfig`
/// to read it, so actor SPAWN projects the value onto this combat-owned carrier
/// (actors → combat, the legal arrow); the hitbox resolver reads `CombatTuning`
/// instead of reaching up into `ActorConfig`. Bodies without one (the player,
/// headless test bodies) fall back to the reference `1.0` — byte-parity with the
/// old `Option<&ActorConfig>` read.
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
    /// strike paths resolve (`None` = the player manifest root). Combat only
    /// forwards it to the installed authored-volume resolver.
    pub sprite_character_id: Option<String>,
}

impl Default for CombatTuning {
    fn default() -> Self {
        Self {
            weight: 1.0,
            attack_cooldown_mult: 1.0,
            sprite_character_id: None,
        }
    }
}

/// Which motion / AI state-machine template a brain instantiates.
// `CharacterBrainTemplate`/`CharacterBrainSpec` moved to
// `features::ecs::actor_tuning` (E2): actor archetype vocabulary.
// `RespawnPolicy` moved to `ambition_entity_catalog::placements`
// ([W-a]: the ADR-0022 authored schema half).

/// How a body's accumulated-damage meter relates to death (CM1). Smash's
/// percent and Ambition's HP are the SAME quantity read through two policies:
/// the meter itself is `BodyHealth` (`damage_taken()`); this enum only decides
/// whether reaching the pool max KILLS.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DeathPolicy {
    /// Dies when the meter reaches `max` — Ambition today. THE DEFAULT, so every
    /// existing archetype is unchanged.
    #[default]
    HpDepleted,
    /// The meter never kills on its own (`max` is a display normalizer only);
    /// death comes from the WORLD — the blast-zone / OOB / fell-out gate the
    /// engine already owns. This is smash percent, and it costs one enum.
    Unbounded,
}

impl DeathPolicy {
    /// Whether reaching the pool's max damage KILLS this body. `HpDepleted`
    /// (the default) does, so every existing kill path is byte-unchanged;
    /// `Unbounded` (smash percent) never dies from the meter — the blast-zone
    /// gate owns its death.
    pub fn kills_at_max(self) -> bool {
        matches!(self, DeathPolicy::HpDepleted)
    }
}
