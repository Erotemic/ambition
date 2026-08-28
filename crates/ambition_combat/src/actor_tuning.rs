//! PER-ACTOR RUNTIME TUNING, THE BRAIN-CONSTRUCTION INPUTS, AND THE CONFIG THAT
//! CARRIES THEM.
//!
//! ⭐⭐ MOVED OUT OF THE ACTOR MONOLITH 2026-08-27 (D33). `ActorConfig` is an
//! actor's authored IDENTITY — its id, its name, the kit its archetype was
//! projected into, the brain it was authored with, the sprite it resolves to. It
//! sat in a 95k-line crate because that is where the spawner happened to be.
//!
//! ⛔ AND THE DESTINATION IS DECIDED BY VOCABULARY, NOT BY TASTE. The obvious
//! home was `ambition_characters` — "what a character IS" — and it is the WRONG
//! one: `ActorTuning.movement` is an `crate::BodyMovementTuning`, and
//! combat DEPENDS on characters, so the floor crate can never name it. The type
//! follows what it is built from.
//!
//! ⛔⛔ AND IT WAS THE LAST REAL THING TYING `ambition_sim_view` TO THE MONOLITH.
//! Of the 24 names the view crate imported from there, twenty were re-exports of
//! other crates; this was one of the four that were not.

use crate::components::BodyMovementTuning;
use ambition_entity_catalog::placements::RespawnPolicy;
use bevy::prelude::Component;

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
    /// THE SPEED THIS BODY FLIES AT — the one number two callers were computing
    /// separately.
    ///
    /// ⭐ A flying body's throttle is its CHASE speed, not its run speed: the
    /// flight limb sets `flight_terminal_speed` from this, and a stick deflection
    /// is a commanded velocity divided by that terminal. So a producer that
    /// scaled a human's command by `max_run_speed` while the integrator
    /// normalised by this one handed a fully deflected stick
    /// `max_run_speed / flight_speed` of the available deflection — a possessed
    /// flyer could not reach its own top speed (D117).
    ///
    /// ⛔ ONE OWNER, because the two sites were the same expression twice and a
    /// value with two homes cannot be attributed when it disagrees with itself.
    /// `BrainSnapshot::max_run_speed`'s own doc already said what this is for:
    /// *"the throttle scale the caller wants this body's locomotion intent
    /// expressed against … a boss's flight speed for a body that flies."*
    pub fn flight_speed(&self) -> f32 {
        self.chase_speed.max(self.max_run_speed).max(1.0)
    }

    /// Where this body contests space when it fights — the one fact the
    /// crowding signal needs that positions do not carry.
    pub fn crowd_kind(&self) -> crate::crowd::CrowdKind {
        if self.is_aerial {
            crate::crowd::CrowdKind::Aerial
        } else {
            crate::crowd::CrowdKind::Ground
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
mod flight_speed_tests {
    use super::*;

    /// A FLYING BODY'S THROTTLE IS ITS CHASE SPEED, and the arms straddle the
    /// comparison rather than sitting on one side of it.
    ///
    /// ⭐⭐ THE DEFECT THIS PINS (D117): the flight limb sets
    /// `flight_terminal_speed` from this, and a stick deflection is a commanded
    /// velocity divided by that terminal — so a producer scaling a human's
    /// command by `max_run_speed` while the integrator normalised by this handed
    /// a fully deflected stick `max_run_speed / flight_speed` of the deflection.
    /// A possessed flyer could not reach its own top speed.
    ///
    /// ⛔ THE `chase > run` ARM IS THE ONE THAT DISCRIMINATES. Every shipped
    /// body has `chase <= run` (only two catalog rows author `chase_speed` at
    /// all, and no flyer among them), so an assertion taken from the live cast
    /// agrees with the OLD behaviour and with the new one — the defect is
    /// latent because the content cannot currently express it.
    #[test]
    fn a_flying_bodys_throttle_is_whichever_speed_is_larger() {
        let mut tuning = ActorTuning {
            chase_speed: 900.0,
            max_run_speed: 300.0,
            ..Default::default()
        };
        assert_eq!(
            tuning.flight_speed(),
            900.0,
            "a body that chases faster than it runs flies at its CHASE speed; \
             answering 300 is the deflection defect"
        );

        // The ordinary shape, and the reason the defect hid: every shipped body
        // is on this side of the comparison.
        tuning.chase_speed = 100.0;
        assert_eq!(
            tuning.flight_speed(),
            300.0,
            "a body that runs faster than it chases still flies at the larger"
        );

        // ⛔ AND THE FLOOR IS NOT DECORATION: a zero throttle is a division the
        // integrator performs, and `0` there is every stick reading NaN.
        tuning.chase_speed = 0.0;
        tuning.max_run_speed = 0.0;
        assert_eq!(tuning.flight_speed(), 1.0);
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

/// Authored configuration + identity for an actor (any disposition). Archetype-
/// free by construction: the named roster enum is resolved at spawn and projected
/// into generic kit data (`tuning` + `brain_profile` + the `CombatCapabilities`
/// component), so neither the per-frame integration nor the runtime brain
/// rebuilds (provoke, dismount) call back into the content roster. `spawn` records
/// the authored baseline `reset_to_spawn` restores.
#[derive(Component, Clone, Debug)]
pub struct ActorConfig {
    pub id: String,
    pub name: String,
    /// Per-frame runtime tuning snapshot (kit vocabulary), projected
    /// from the archetype's authored spec at spawn.
    pub tuning: ActorTuning,
    /// Generic brain-construction inputs (kit vocabulary), projected
    /// from the archetype at spawn so the runtime brain rebuilds
    /// reconstruct a brain without naming the roster enum.
    pub brain_profile: BrainProfile,
    pub brain: ambition_entity_catalog::placements::CharacterBrain,
    /// LDtk display name of the original NPC when this enemy was spawned
    /// by migrating a hostile NPC (keeps its own sprite sheet). `None`
    /// uses the default enemy sprite.
    pub sprite_override_npc_name: Option<String>,
    /// Sprite-catalog identity: the catalog `character_id` this actor's sprite
    /// resolves to. `Some` for catalog characters (player, named NPCs/enemies,
    /// content actors); `None` for a body that renders from a kind-default
    /// sheet. Lets gameplay resolve any actor's `SheetRecord` / per-animation
    /// hit/hurt metrics — the same sprite-metadata path the player and bosses
    /// use — without reaching into the presentation registry. See
    /// [`CombatGeometry`].
    ///
    /// NOT the body's gameplay character authority, and `WornCharacter` OUTRANKS it (AC7.1). It
    /// is not: every seam that resolves a character asks `WornCharacter` first and falls back to a
    /// sprite id only for a body that wears nothing — see `presentation.rs`'s `worn …
    /// .or_else(tuning .sprite_character_id)`. That precedence is what lets a body SWAP its
    /// character at runtime (Sanic's transformation) and take its new repertoire and volumes with
    /// it while this field stays put.
    pub sprite_character_id: Option<String>,
    /// Does this body's autonomous driver share one deterministic cognitive
    /// stream with its twins? Resolved from the character at construction — see
    /// [`ambition_characters::actor::CharacterDefinition::preserves_mirror_symmetry`].
    ///
    /// it lives HERE, on the config, because three roads build this body's
    /// brain and they must not disagree: a match seat, a room spawn, and a
    /// rewind/live restore all go through
    /// [`enemy_default_brain`](ambition_characters::features::ecs::enemy_default_brain), and
    /// the note on `PreparedCharacterDefinition::autonomous_profile` says why
    /// that matters — *"spawn, rewind and live restore all make the same call,
    /// which is why they cannot disagree"*. A trait the seat road looked up in a
    /// registry the restore road cannot reach would let a rewound Emmy think
    /// different thoughts from the one that was standing there a frame ago.
    ///
    /// `ActorConfig` is registered `rollback_component_clone`, so this rewinds
    /// with the rest of the config and costs no wire format.
    pub preserves_mirror_symmetry: bool,
}
