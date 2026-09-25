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
    /// The chase half is the driver's `policy` against this body's top speed,
    /// asked here rather than stored: a stored product kept the construction
    /// policy's effort after a provocation swapped the policy.
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
    pub fn flight_speed(&self, policy: &BrainProfile) -> f32 {
        policy
            .chase_speed(self.max_run_speed)
            .max(self.max_run_speed)
            .max(1.0)
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
    /// with the constructing `policy`'s patrol speed as the crawl speed;
    /// everything else starts axis-swept with its authored body tuning
    /// (integration refreshes those parameters live each tick).
    pub fn motion_model(
        &self,
        policy: &BrainProfile,
    ) -> ambition_platformer2d_core::movement::MotionModel {
        if self.surface_walker {
            ambition_platformer2d_core::movement::MotionModel::adhesive_crawler(
                ambition_platformer2d_core::CrawlerParams {
                    crawl_speed: policy.patrol_speed(self.max_run_speed),
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
    /// profile authors `chase_effort <= 1`, so an assertion taken from the live cast
    /// agrees with the OLD behaviour and with the new one — the defect is
    /// latent because the content cannot currently express it.
    #[test]
    fn a_flying_bodys_throttle_is_whichever_speed_is_larger() {
        let mut tuning = ActorTuning {
            max_run_speed: 300.0,
            ..Default::default()
        };
        let chasing = |effort: f32| BrainProfile {
            chase_effort: effort,
            ..Default::default()
        };
        assert_eq!(
            tuning.flight_speed(&chasing(3.0)),
            900.0,
            "a body that chases faster than it runs flies at its CHASE speed; \
             answering 300 is the deflection defect"
        );

        // The ordinary shape, and the reason the defect hid: every shipped body
        // is on this side of the comparison.
        assert_eq!(
            tuning.flight_speed(&chasing(1.0 / 3.0)),
            300.0,
            "a body that runs faster than it chases still flies at the larger"
        );

        // ⛔ AND THE FLOOR IS NOT DECORATION: a zero throttle is a division the
        // integrator performs, and `0` there is every stick reading NaN.
        tuning.max_run_speed = 0.0;
        assert_eq!(tuning.flight_speed(&chasing(1.0)), 1.0);
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
            // (The controller-policy projections `patrol_speed`/`chase_speed`
            // are gone: a driver's speeds are its `ActorPolicy` against
            // `max_run_speed`, computed where the driver is lowered.)
            // Placement/session facts for this instance.
            is_hostile: _,
            respawn: _,
            // Presentation facts.
            dream_seed: _,
            ranged_visual: _,
            // The character's contact hazard, which a summoner may decline at
            // construction. Not runtime state: a live withdrawal is its own
            // component, and nothing writes a tuning field after spawn (the
            // cluster view borrows `ActorConfig` read-only). What does change at
            // runtime, the driver's policy, is `ActorPolicy`.
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

/// This body's contact threat is withdrawn this tick, whatever its authored
/// `body_contact_damage` says.
///
/// Not a second authority: the system that owns the body's canonical state
/// re-derives it every tick BEFORE the contact pass reads it (a Mary-O snake
/// from its `SnakeShell` phase), so it cannot disagree with that state at a read. That keeps [`ActorTuning`] what it
/// is — construction input — instead of a row a content state machine rewrites
/// every tick. Absent means nothing has withdrawn the threat.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContactThreatWithdrawn(pub bool);

/// The policy this body's AUTONOMOUS driver plays it by: template, radii,
/// normalized effort, swing pace.
///
/// Split out of [`ActorConfig`] because it is the one thing about an actor that
/// changes after construction: a provocation installs the provoked policy and
/// a brain command installs another, and the brain rebuilds read it. Keeping it
/// in the config made the whole construction record a runtime-written row.
/// Required by [`ActorConfig`], so every actor has one on every road, including
/// a rollback restore.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct ActorPolicy(pub BrainProfile);

/// Authored configuration for an actor (any disposition), written once at
/// construction. Archetype-free: the named roster enum is resolved at spawn and
/// projected into generic kit data (`tuning` + the `CombatCapabilities`
/// component), so neither the per-frame integration nor the runtime brain
/// rebuilds (provoke, dismount) call back into the content roster. The policy
/// the driver plays by changes at runtime and is its own component,
/// [`ActorPolicy`].
///
/// WHO the body is lives on [`crate::components::ActorIdentity`], not here.
#[derive(Component, Clone, Debug)]
#[require(ActorPolicy)]
pub struct ActorConfig {
    /// Per-frame runtime tuning snapshot (kit vocabulary), projected
    /// from the archetype's authored spec at spawn.
    pub tuning: ActorTuning,
    /// The placement's AUTHORED brain key (`Custom("snake")`, `Guard`, …), a
    /// content label read by the tag and sprite passes. Written at construction
    /// and never changed: the live mind is `Brain`'s, not this field's.
    pub brain: ambition_entity_catalog::placements::CharacterBrain,
    /// Does this body's autonomous driver share one deterministic cognitive
    /// stream with its twins? Resolved from the character at construction — see
    /// [`ambition_characters::actor::definition::CharacterDefinition::preserves_mirror_symmetry`].
    ///
    /// it lives HERE, on the config, because three roads build this body's
    /// brain and they must not disagree: a match seat, a room spawn, and a
    /// rewind/live restore all go through
    /// `enemy_default_brain` — ⚠ deliberately NOT a link: it is `pub(crate)` in
    /// `ambition_platformer2d_actor_monolith::features::ecs::brain_builders`, so
    /// no doc link from here can ever resolve to it. Naming the crate and module
    /// in prose is the most a reader can be given — and
    /// the
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
