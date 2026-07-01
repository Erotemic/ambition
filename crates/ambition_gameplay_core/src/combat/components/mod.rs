//! ECS-native feature components.
//!
//! Gameplay feature families are represented as normal Bevy entities/components,
//! paired with typed messages for cross-system effects.

use super::*;

mod actors;
mod features;
mod spawn;

pub use actors::*;
pub use features::*;
pub use spawn::*;

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
    /// On death, respawns in place after this many seconds instead of
    /// counting as defeated.
    pub respawn_in_place_seconds: Option<f32>,
    /// When a real (non-encounter) kill should clear: the death flag
    /// vocabulary the save mirror consumes.
    pub respawn_policy: EnemyRespawnPolicy,
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

/// Per-actor numeric/flag tuning the RUNTIME combat loops read each
/// frame, derived from the actor's authored archetype DATA at spawn
/// (like [`CombatCapabilities`], but plain tuning rather than special
/// behaviors). Carried as a field on the enemy config component so
/// the per-frame systems never call back into a named archetype enum.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Walks surfaces hugging the surface normal: body axes swap on
    /// vertical surfaces and patrol probes ledges instead of walking
    /// off them.
    pub surface_walker: bool,
    /// Surface-walker only: a hit knocks the actor off its surface (it
    /// falls with gravity for a moment, then re-attaches). `false` keeps
    /// it clinging when struck.
    pub cling_breaks_on_hit: bool,
    /// Self-revives in place after its respawn timer instead of
    /// counting as defeated (finite training dummies).
    pub revives_in_place: bool,
    /// Flies: no gravity, aerial slot class.
    pub is_aerial: bool,
    /// Training-dummy family: excluded from slot pressure and save
    /// persistence.
    pub is_sandbag: bool,
    /// Touching this actor's body hurts the player.
    pub body_contact_damage: bool,
    /// Deep-dream visual jitter seed; `None` = no dream pass.
    pub dream_seed: Option<f32>,
    /// Visual identity of this actor's ranged projectile, authored on the
    /// archetype. The ranged-fire effects consumer stamps it onto the spawned
    /// shot so the render layer picks art by KIND (e.g. the PCA's Conway
    /// glider) rather than by reading the owner-id string. `EnemyDefault` is
    /// the generic orange shot.
    pub ranged_visual: crate::projectile::ProjectileVisualKind,
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
            revives_in_place: false,
            is_aerial: false,
            is_sandbag: false,
            body_contact_damage: false,
            dream_seed: None,
            ranged_visual: crate::projectile::ProjectileVisualKind::EnemyDefault,
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

/// Which motion / AI state-machine template a brain instantiates.
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
    /// Dedicated shark motion policy (charge-and-crash).
    Shark,
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

/// Authored rule for when a defeated enemy should reappear. Picked
/// per-archetype today; a future EnemySpawn LDtk field can override
/// it on a single spawn without touching the archetype default.
///
/// The kill hook in `damage.rs` writes one of two persistent flags
/// (or none) depending on this policy; the room-load `save_sync`
/// reads either flag back into `alive = false`. A "rest" event
/// clears just the `_dead_until_rest` flags, so OnRest enemies come
/// back at the next rest but OnRoomReenter ones come back on the
/// next room load.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EnemyRespawnPolicy {
    /// Fresh every time the player enters the room. Default for
    /// trash grunts (skitters, lurkers, raiders, puppy slugs).
    #[default]
    OnRoomReenter,
    /// Stays dead until the player rests at a save point. Default
    /// for mini-boss-tier presences (brutes, colossi, pirate
    /// heavies, sharks-with-riders).
    OnRest,
    /// Permanent kill — only an explicit save reset brings them
    /// back. Reserved for scripted one-off encounters that aren't
    /// `encounter:*` ids (which have their own state machine).
    Never,
}
