//! The actor ARCHETYPE tuning vocabulary (moved out of the combat kit at
//! E2): per-actor numeric/flag tuning + the generic brain-construction
//! inputs, authored per archetype (`character_archetypes.ron`) and projected
//! onto the enemy config component at spawn. Combat reads none of this —
//! spawn projects the combat-relevant facts onto `CombatTuning` (the legal
//! actors → combat arrow).

use crate::combat::{BodyMovementTuning, DeathPolicy};
use ambition_entity_catalog::placements::RespawnPolicy;

/// Per-actor numeric/flag tuning the RUNTIME combat loops read each
/// frame, derived from the actor's authored archetype DATA at spawn
/// (like [`CombatCapabilities`], but plain tuning rather than special
/// behaviors). Carried as a field on the enemy config component so
/// the per-frame systems never call back into a named archetype enum.
///
/// `Clone` (not `Copy`): the open `ranged_visual` id is an owned `String`.
#[derive(Clone, Debug, PartialEq)]
pub struct ActorTuning {
    /// Resolved movement physics for this body (composed from the archetype
    /// hierarchy). The spine reads gravity/run/jump/fall from here, not constants.
    pub movement: BodyMovementTuning,
    /// Full health pool at spawn / respawn-reset.
    pub max_health: i32,
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
    /// Multiplier on the shared attack cooldown (fast skirmishers
    /// < 1.0, lumbering heavies > 1.0).
    pub attack_cooldown_mult: f32,
    /// Hostile by default: actively tracks the player and publishes
    /// contact damage. Peaceful patrollers are false.
    pub attacks_player: bool,
    /// SPAWN-TIME policy selector: this archetype crawls surfaces glued to
    /// the surface normal (the adhesive-crawler movement policy). Consumed
    /// once by [`Self::motion_model`]; runtime dispatch reads the body's
    /// explicit `MotionModel`, never this flag.
    pub surface_walker: bool,
    /// Surface-walker only: a hit knocks the actor off its surface (it
    /// falls with gravity for a moment, then re-attaches). `false` keeps
    /// it clinging when struck.
    pub cling_breaks_on_hit: bool,
    /// When this defeated actor reappears (ADR 0022) — the ONE authored
    /// respawn policy. `InPlace(secs)` self-revives where it stood
    /// (finite training dummies); the flag-writing arms are consumed by
    /// the kill hook; DEFAULT: dead stays dead.
    pub respawn: RespawnPolicy,
    /// Knockback weight (CM1): heavier bodies launch less under the same growth
    /// term (`kb_growth * damage_taken / weight`). `1.0` is the reference body;
    /// the default keeps every un-authored archetype at the reference.
    pub weight: f32,
    /// How this body's damage meter relates to death (CM1). `HpDepleted`
    /// (default) dies at pool max; `Unbounded` is smash percent — death comes
    /// from the blast-zone/OOB gate, not the meter.
    pub death_policy: DeathPolicy,
    /// Flies: no gravity, aerial slot class.
    pub is_aerial: bool,
    /// Direct-velocity free-mover: the brain commands an EXACT velocity each tick
    /// (a boss pattern's `desired_vel`), so the shared flight limb takes it verbatim
    /// (no accel ramp / drag / deadzone) — byte-identical to the old bespoke SNAP
    /// float. Threaded into the engine `MovementTuning.flight_direct_velocity`
    /// (archetype swap AS4). Ordinary flyers (parrot) leave this false for smoothed
    /// flight.
    pub flight_direct_velocity: bool,
    /// Training-dummy family: excluded from slot pressure and save
    /// persistence.
    pub is_sandbag: bool,
    /// Touching this actor's body hurts the player.
    pub body_contact_damage: bool,
    /// Deep-dream visual jitter seed; `None` = no dream pass.
    pub dream_seed: Option<f32>,
    /// Open visual id of this actor's ranged projectile, authored on the
    /// archetype. The ranged-fire effects consumer stamps it onto the spawned
    /// shot so the render layer resolves art by id through the content-owned
    /// catalog (e.g. the PCA's `"glider"`) rather than by reading the owner-id
    /// string. The empty string is the generic orange shot.
    pub ranged_visual: String,
}

impl Default for ActorTuning {
    fn default() -> Self {
        Self {
            movement: BodyMovementTuning::default(),
            max_health: 0,
            // `ActorTuning` keeps the DERIVED absolute speeds: this is the
            // body-space projection brains consume, not the authored row.
            patrol_speed: 0.0,
            chase_speed: 0.0,
            max_run_speed: 0.0,
            contact_strength: 0.0,
            damage_amount: 0,
            // Multiplicative identity — a defaulted tuning must not
            // zero out the shared attack cooldown.
            attack_cooldown_mult: 1.0,
            attacks_player: false,
            surface_walker: false,
            cling_breaks_on_hit: false,
            respawn: RespawnPolicy::default(),
            // Reference body: the default tuning must not zero out the growth
            // divisor, and every un-authored archetype dies at pool max.
            weight: 1.0,
            death_policy: DeathPolicy::default(),
            is_aerial: false,
            flight_direct_velocity: false,
            is_sandbag: false,
            body_contact_damage: false,
            dream_seed: None,
            ranged_visual: String::new(),
        }
    }
}

impl ActorTuning {
    /// Slot class this actor requests from the combat slot board.
    pub fn slot_kind(&self) -> crate::combat::slots::SlotKind {
        if self.is_aerial {
            crate::combat::slots::SlotKind::Aerial
        } else {
            crate::combat::slots::SlotKind::Melee
        }
    }

    /// Take on `archetype`'s combat tuning while keeping the fields this
    /// actor's PLACEMENT owns.
    ///
    /// Respawn policy is placement-scoped, not archetype-scoped (ADR 0022): a
    /// named NPC is a person and its death is permanent, even while it fights
    /// with the combat numbers of a borrowed mob archetype whose own policy is
    /// `OnRoomReenter`. Assigning a projected archetype tuning WHOLESALE drags
    /// that mob policy onto the person — the kill hook then writes no death
    /// flag, `sync_ecs_actors_with_save` has nothing to read, and the NPC is
    /// rebuilt alive by the next room construction. That was a live bug.
    ///
    /// Every archetype projection goes through here, so the invariant holds by
    /// construction rather than by each caller remembering it.
    #[must_use]
    pub fn adopting_archetype(&self, archetype: ActorTuning) -> ActorTuning {
        ActorTuning {
            respawn: self.respawn,
            ..archetype
        }
    }
}

/// Generic kit vocabulary for an archetype's brain.
///
/// ⚠ **DEFINED in `ambition_characters::brain`** since 2026-08-03. Its own doc
/// already said the brain module is the universal-actor abstraction; it was
/// merely LOCATED here. It moved so `character_archetypes.ron`'s authored
/// vocabulary could leave this crate, which is what let the content compiler
/// own the enemy-roster family without linking a renderer.
pub use ambition_characters::brain::CharacterBrainTemplate;

/// **The reusable autonomous-controller profile** — DEFINED in
/// `ambition_characters::brain::profile`, and it is not this crate's type.
///
/// ⭐ **it replaced `BrainProfile` outright** (campaign 2026-08-11, the
/// controller authority). That struct was the same idea one crate too high and
/// one concept too narrow: a *projection* of an archetype, reachable only by
/// having an archetype, so two characters that fight alike could not share a
/// policy without sharing a body. [`BrainProfile`] is authorable, reusable, and
/// carries the three CharacterAI knobs that used to sit in [`ActorTuning`]
/// (`aggro_radius`, `attack_range`, `turns_at_walls`) — those are decisions a
/// DRIVER makes, not facts a body states.
pub use ambition_characters::brain::BrainProfile;

impl ActorTuning {
    /// The explicit movement policy this archetype's bodies carry from spawn.
    ///
    /// Crawler archetypes (`surface_walker`) select the adhesive-crawler policy
    /// with their patrol speed as the crawl speed; everything else starts
    /// axis-swept with its authored body tuning (integration refreshes those
    /// parameters live each tick).
    pub fn motion_model(&self) -> crate::features::MotionModel {
        if self.surface_walker {
            crate::features::MotionModel::adhesive_crawler(
                ambition_platformer2d_core::CrawlerParams {
                    crawl_speed: self.patrol_speed,
                    max_fall_speed: self.movement.max_fall_speed,
                },
            )
        } else {
            crate::features::MotionModel::axis_swept(
                self.movement
                    .body_tuning(self.max_run_speed)
                    .axis_swept_params(),
            )
        }
    }
}
