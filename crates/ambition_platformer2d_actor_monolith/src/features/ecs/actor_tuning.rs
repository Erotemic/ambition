//! Resolved per-actor tuning carried on the runtime actor config.
//! Most fields are construction-time projections; `body_contact_damage` is mutable runtime state.
//! Combat-relevant facts are projected separately onto combat components.

use ambition_combat::BodyMovementTuning;
use ambition_entity_catalog::placements::RespawnPolicy;

/// Numeric and flag tuning resolved from body, brain policy, and placement.
/// Per-frame systems consume this projection instead of re-resolving authored content.
#[derive(Clone, Debug, PartialEq)]
pub struct ActorTuning {
    /// Resolved movement physics for this body. The spine reads
    /// gravity/run/jump/fall from here, not constants.
    pub movement: BodyMovementTuning,
    /// Patrol walking speed (px/s).
    pub patrol_speed: f32,
    /// Chase/steering speed (px/s).
    pub chase_speed: f32,
    /// Ground-run capability (px/s) — the fastest this body locomotes. Grounded
    /// brains emit a normalized throttle of it; the integrator scales velocity
    /// back as `locomotion * max_run_speed`, uniformly with the player.
    pub max_run_speed: f32,
    /// Contact-damage knockback strength.
    pub contact_strength: f32,
    /// Damage dealt by an attack / body contact.
    pub damage_amount: i32,
    /// Whether this placement currently drives hostile behavior; this is not a body identity trait.
    pub is_hostile: bool,
    /// SPAWN-TIME policy selector: this body crawls surfaces glued to
    /// the surface normal (the adhesive-crawler movement policy). Consumed
    /// once by [`Self::motion_model`]; runtime dispatch reads the body's
    /// explicit `MotionModel`, never this flag.
    pub surface_walker: bool,
    /// Surface-walker only: a hit knocks the actor off its surface (it
    /// falls with gravity for a moment, then re-attaches). `false` keeps
    /// it clinging when struck.
    pub cling_breaks_on_hit: bool,
    /// Authored respawn policy for this placed actor.
    pub respawn: RespawnPolicy,
    /// Knockback weight; larger values reduce launch growth. `1.0` is the reference body.
    pub weight: f32,
    /// Flies: no gravity, aerial slot class.
    pub is_aerial: bool,
    /// If true, flight consumes the brain's desired velocity directly instead of smoothing it.
    pub flight_direct_velocity: bool,
    /// Touching this actor's body hurts the player.
    pub body_contact_damage: bool,
    /// Deep-dream visual jitter seed; `None` = no dream pass.
    pub dream_seed: Option<f32>,
    /// Character-authored projectile presentation id; empty selects the generic visual.
    pub ranged_visual: String,
}

impl Default for ActorTuning {
    fn default() -> Self {
        Self {
            movement: BodyMovementTuning::default(),
            // `ActorTuning` keeps the DERIVED absolute speeds: this is the
            // body-space projection brains consume, not the authored row.
            patrol_speed: 0.0,
            chase_speed: 0.0,
            max_run_speed: 0.0,
            contact_strength: 0.0,
            damage_amount: 0,
            is_hostile: false,
            surface_walker: false,
            cling_breaks_on_hit: false,
            respawn: RespawnPolicy::default(),
            // Reference body: the default must not zero out the growth divisor.
            weight: 1.0,
            is_aerial: false,
            flight_direct_velocity: false,
            body_contact_damage: false,
            dream_seed: None,
            ranged_visual: String::new(),
        }
    }
}

impl ActorTuning {
    /// Where this body contests space when it fights — the one fact the
    /// crowding signal needs that positions do not carry.
    pub fn crowd_kind(&self) -> ambition_combat::crowd::CrowdKind {
        if self.is_aerial {
            ambition_combat::crowd::CrowdKind::Aerial
        } else {
            ambition_combat::crowd::CrowdKind::Ground
        }
    }

}

/// Universal actor-brain template re-export.
pub use ambition_characters::brain::CharacterBrainTemplate;

/// Reusable autonomous-controller profile re-export.
pub use ambition_characters::brain::BrainProfile;

impl ActorTuning {
    /// The explicit movement policy this archetype's bodies carry from spawn.
    ///
    /// Crawler archetypes (`surface_walker`) select the adhesive-crawler policy
    /// with their patrol speed as the crawl speed; everything else starts
    /// axis-swept with its authored body tuning (integration refreshes those
    /// parameters live each tick).
    pub fn motion_model(&self) -> ambition_platformer2d_core::movement::MotionModel {
        if self.surface_walker {
            ambition_platformer2d_core::movement::MotionModel::adhesive_crawler(
                ambition_platformer2d_core::CrawlerParams {
                    crawl_speed: self.patrol_speed,
                    max_fall_speed: self.movement.max_fall_speed,
                },
            )
        } else {
            ambition_platformer2d_core::movement::MotionModel::axis_swept(
                self.movement
                    .body_tuning(self.max_run_speed)
                    .axis_swept_params(),
            )
        }
    }
}

#[cfg(test)]
mod authority_split_tests {
    use super::*;

    /// Exhaustive destructuring forces every tuning field to declare its authority class.
    #[test]
    fn every_tuning_field_belongs_to_one_of_the_campaigns_authorities() {
        let ActorTuning {
            // Reusable character facts.
            movement: _,
            max_run_speed: _,
            contact_strength: _,
            damage_amount: _,
            surface_walker: _,
            cling_breaks_on_hit: _,
            weight: _,
            is_aerial: _,
            flight_direct_velocity: _,
            // Controller-policy projections resolved against this body.
            patrol_speed: _,
            chase_speed: _,
            // Placement/session facts for this instance.
            is_hostile: _,
            respawn: _,
            // Presentation facts.
            dream_seed: _,
            ranged_visual: _,
            // Mutable runtime state. `ActorConfig` is rollback registered, so this value rewinds.
            body_contact_damage: _,
        } = ActorTuning::default();

        // The exhaustive destructure is the structural assertion.
        assert!(
            true,
            "if this file failed to compile, a field was added to or removed \
             from `ActorTuning` — put it in one of the three columns above, or \
             establish that it belongs in none and delete it"
        );
    }
}
