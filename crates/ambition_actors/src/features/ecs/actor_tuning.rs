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
    /// Distance (px) at which the actor notices the player.
    pub aggro_radius: f32,
    /// Distance (px) at which the actor commits to an attack.
    pub attack_range: f32,
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
            patrol_speed: 0.0,
            chase_speed: 0.0,
            max_run_speed: 0.0,
            aggro_radius: 0.0,
            attack_range: 0.0,
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
}

/// Generic kit vocabulary: the brain module is the universal-actor
/// abstraction and shouldn't know named enemies, and the runtime brain
/// rebuild (provoke-to-hostile, dismount) must reconstruct a brain from
/// projected data without naming the content archetype enum. Authored
/// per archetype in `character_archetypes.ron` and projected onto
/// [`CharacterBrainSpec`] at spawn.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub enum CharacterBrainTemplate {
    /// No motion / no AI — the actor only reacts to events (sandbag's
    /// PunchWeak counter, dialogue-only NPCs that become hostile).
    StandStill,
    /// Surface-walking idle wanderer.
    Wanderer,
    /// Approach-then-strike melee policy. Variety comes from the
    /// per-actor chase_speed / attack_range / aggro_radius in
    /// [`ActorTuning`].
    MeleeBrute,
    /// Strafe-and-fire ranged policy. Maintains a standoff distance and
    /// emits `frame.fire` on a fixed cooldown.
    Skirmisher,
    /// Hold position + long-range fire. Like `Skirmisher` but does not
    /// strafe — stationary turret-like enemies.
    Sniper,
    /// Charge-and-crash motion policy: dive at the target, then recover.
    ChargeCrash,
    /// Smash-brawl pipeline: observe → mode → action → difficulty →
    /// emit. See `ambition_characters::brain::smash`.
    Smash,
    /// Lively flyer: an aerial dive-bomber when hostile (stalk → dive →
    /// recover). Shares its code with the peaceful catalog `Aerial` bird via
    /// `StateMachineCfg::Aerial` — hostility is just `aggressiveness > 0`.
    Aerial,
}

/// The generic brain-construction inputs projected from an actor's
/// authored archetype at spawn, carried on the enemy config component so
/// the runtime brain rebuilds (provoke-to-hostile, dismount) can
/// reconstruct the brain WITHOUT naming the content archetype enum. The
/// numeric inputs (aggro/chase/attack/attacks_player) live in
/// [`ActorTuning`]; this carries the structural choices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterBrainSpec {
    /// Which motion / AI policy template the brain instantiates.
    pub template: CharacterBrainTemplate,
    /// Smash-template hit band (px) — the radius the brain closes to
    /// before emitting MeleeAttack. Authored per archetype; legacy
    /// fallback is 36 px.
    pub smash_hit_band: f32,
    /// Smash-template heavy base: longer reach + slower chase
    /// (`SmashCfg::BRUTE_DEFAULT`) vs the lighter striker default.
    pub smash_heavy: bool,
    /// Smash-template dash-to-close: a richer action set that dashes to
    /// close a large gap (goblins).
    pub smash_dash_to_close: bool,
    /// Smash-template **duelist neutral game** (`SmashCfg::DUELIST_DEFAULT` base):
    /// footsies (weave in/out of poke range), neutral hops, and a real retreat /
    /// spacing rhythm, instead of the grunt's close-and-camp. The per-flag kit
    /// (`smash_can_blink/_shield/_dash/_fly`) still layers on top. This is what
    /// makes a "platform fighter" (the PCA, the player-robot) move and space
    /// rather than mash at point-blank.
    pub smash_duelist: bool,
    /// Movement kit: the Smash brain blink-evades a perceived lunge. Projected
    /// into `SmashCfg::can_blink` (the controller's *attempt*); the body's
    /// `CombatCapabilities::can_blink` is the matching *enforce* gate.
    pub smash_can_blink: bool,
    /// Movement kit: grounded-base hybrid that flies to cover a long traversal
    /// gap. Projected into `SmashCfg::can_fly` (attempt); the body's
    /// `CombatCapabilities::can_fly` is the matching *enforce* gate.
    pub smash_can_fly: bool,
    /// Movement/defense kit: the Smash brain reactive-blocks a perceived lunge it
    /// won't blink. Projected into `SmashCfg::can_shield` (the controller's
    /// *attempt*); the body's `CombatCapabilities::can_shield` is the matching
    /// *enforce* gate.
    pub smash_can_shield: bool,
    /// When provoked from peaceful, force an aggressive MeleeBrute brain
    /// with at least this aggro radius (cove PirateHeavy crew).
    /// `None` = use the template's default aggressive brain.
    pub provoke_forced_brute_min_aggro: Option<f32>,
}

impl CharacterBrainSpec {
    /// Default melee smash hit-band (px) when an archetype authors none. Single
    /// source of truth shared with `CharacterArchetypeSpec::brain_spec`.
    pub const DEFAULT_SMASH_HIT_BAND: f32 = 36.0;
}

impl Default for CharacterBrainSpec {
    fn default() -> Self {
        Self {
            template: CharacterBrainTemplate::MeleeBrute,
            smash_hit_band: Self::DEFAULT_SMASH_HIT_BAND,
            smash_heavy: false,
            smash_dash_to_close: false,
            smash_duelist: false,
            smash_can_blink: false,
            smash_can_fly: false,
            smash_can_shield: false,
            provoke_forced_brute_min_aggro: None,
        }
    }
}

impl ActorTuning {
    /// The explicit movement policy this archetype's bodies carry from spawn.
    ///
    /// Crawler archetypes (`surface_walker`) select the adhesive-crawler policy
    /// with their patrol speed as the crawl speed; everything else starts
    /// axis-swept with its authored body tuning (integration refreshes those
    /// parameters live each tick).
    pub fn motion_model(&self) -> crate::features::MotionModel {
        if self.surface_walker {
            crate::features::MotionModel::adhesive_crawler(ambition_engine_core::CrawlerParams {
                crawl_speed: self.patrol_speed,
                max_fall_speed: self.movement.max_fall_speed,
            })
        } else {
            crate::features::MotionModel::axis_swept(
                self.movement
                    .body_tuning(self.max_run_speed)
                    .axis_swept_params(),
            )
        }
    }
}
