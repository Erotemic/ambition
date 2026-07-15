//! Enemy data + state for the actor simulation: the [`CharacterRoster`] of
//! archetype specs assembled as an App-local resource from provider fragments,
//! per-actor locomotion state ([`ActorSpawnState`],
//! [`ActorSurfaceState`]), and composite-visual planning. The per-frame
//! physics/AI tick lives in the `integration` submodule; every actor —
//! grounded, aerial, and the adhesive crawler — integrates through the one
//! shared movement kernel (`ae::step_motion`).

use super::*;

mod integration;
pub use integration::ContactAttack;

/// The authored spawn baseline an actor reverts to on a same-room reset
/// (`reset_to_spawn`): position and body size. No entity morphs its
/// archetype in place — a composite (PirateOnShark) is spawned as two
/// SEPARATE standalone entities (`spawn_mounts`) and dismount swaps the
/// rider's brain/action-set, never its archetype — so there is nothing
/// to record here but the spatial baseline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorSpawnState {
    /// World position the actor spawned at.
    pub pos: ae::Vec2,
    /// Authored body size.
    pub size: ae::Vec2,
}

/// An actor's surface-cling state for the glued surface-walker crawl.
///
/// Ground contact (`on_ground`) and air-jump budget now live on the shared
/// movement clusters — [`crate::actor::BodyGroundState::on_ground`] and
/// [`crate::actor::BodyJumpState::air_jumps_available`] — the SAME components the
/// player carries, so there is one ground/jump authority for every body (the
/// grounded/aerial pipeline writes them directly; the surface-walker crawl writes
/// `ground.on_ground` too). This component keeps only the surface-walker's cling
/// geometry, which the shared clusters don't model.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorSurfaceState {
    /// Outward-pointing unit normal of the surface the actor is
    /// currently clinging to. Used by surface-walking archetypes
    /// (`PuppySlug`) to crawl floors, walls, and ceilings; every other
    /// archetype pins this at `(0, -1)` (floor) and ignores it. Engine
    /// y grows downward, so floor → (0, -1), right wall → (-1, 0),
    /// ceiling → (0, 1), left wall → (1, 0).
    pub surface_normal: ae::Vec2,
    /// 0.0 = ignores gravity (flying); 1.0 = full gravity.
    pub gravity_scale: f32,
}

// `RespawnPolicy` moved to the combat kit (generic death/respawn
// vocabulary); re-exported so `crate::features::RespawnPolicy`
// paths keep working.
pub use ambition_entity_catalog::placements::RespawnPolicy;

/// Flag-id suffix used by `_dead_until_rest` flags. Constant so the
/// kill hook, save sync, and `clear_dead_until_rest_flags` all
/// agree on the spelling.
pub const ENEMY_DEAD_UNTIL_REST_SUFFIX: &str = "_dead_until_rest";

#[derive(Clone, Debug, serde::Deserialize)]
pub(crate) struct CharacterArchetypeSpec {
    /// Optional parent archetype id to inherit movement tuning from. The resolver
    /// folds `BASELINE ← parent (resolved) ← this row's `movement` patch`, so an
    /// archetype can extend another and override only what differs. `None` =
    /// inherit straight from the generic baseline.
    #[serde(default)]
    pub inherits: Option<String>,
    /// Authored movement overrides (a partial patch; every knob optional). Layered
    /// onto the resolved parent/baseline at roster-build time.
    #[serde(default)]
    pub movement: crate::combat::BodyMovementPatch,
    /// Resolved movement physics — filled by the roster's inheritance pass, NOT
    /// authored. Defaults to the baseline so a spec used outside the roster still
    /// has sane physics.
    #[serde(skip)]
    pub movement_resolved: crate::combat::BodyMovementTuning,
    pub max_health: i32,
    pub patrol_speed: f32,
    pub chase_speed: f32,
    pub aggro_radius: f32,
    pub attack_range: f32,
    pub contact_strength: f32,
    pub damage_amount: i32,
    /// Multiplier on the shared attack cooldown (fast skirmishers
    /// < 1.0, lumbering heavies > 1.0).
    #[serde(default = "default_attack_cooldown_mult")]
    pub attack_cooldown_mult: f32,
    /// Physical mass, used to weight the mount+rider center of gravity (a heavy
    /// shark vs a light rider) so the pair rotates as a unit around the COG under
    /// a gravity flip. Defaults to 1.0 so existing archetypes need no RON change;
    /// heavy mounts author a larger value.
    #[serde(default = "default_mass")]
    pub mass: f32,
    /// Walks surfaces hugging the surface normal (wall/ceiling
    /// crawler with ledge-aware patrol).
    #[serde(default)]
    pub surface_walker: bool,
    /// Surface-walker only: a hit knocks the actor off its surface — it
    /// loses cling and falls with gravity for a moment before re-attaching.
    /// Authored `false` for crawlers that hold on when struck.
    #[serde(default)]
    pub cling_breaks_on_hit: bool,
    #[serde(default)]
    pub is_aerial: bool,
    #[serde(default)]
    pub is_sandbag: bool,
    /// Detonates at the corpse on death (see `CombatCapabilities`).
    #[serde(default)]
    pub explodes_on_death: bool,
    /// Splits into offspring on death.
    #[serde(default)]
    pub divides_on_death: bool,
    /// A fast charge stopped dead by a wall destroys this actor.
    #[serde(default)]
    pub charge_crash_explodes: bool,
    /// Damage never kills (infinite training dummy).
    #[serde(default)]
    pub never_dies: bool,
    /// When this defeated actor reappears (ADR 0022). DEFAULT =
    /// `DeadStaysDead` — respawning is an authored opt-in: trash mobs
    /// author `OnRoomReenter`, mini-boss presences `OnRest`, training
    /// sandbags `InPlace(secs)`.
    #[serde(default)]
    pub respawn: ambition_entity_catalog::placements::RespawnPolicy,
    /// Knockback weight (CM1): heavier bodies launch less under the growth term.
    /// Default `1.0` (the reference body) keeps every un-authored archetype at
    /// today's flat knockback.
    #[serde(default = "default_weight")]
    pub weight: f32,
    /// Damage-meter death policy (CM1). DEFAULT `HpDepleted` (dies at pool max)
    /// leaves Ambition unchanged; a smash-style fighter authors `Unbounded`
    /// (death from the blast-zone, not the meter).
    #[serde(default)]
    pub death_policy: crate::combat::DeathPolicy,
    /// Deep-dream visual jitter seed (psychedelic shader pass);
    /// `None` = the archetype doesn't participate.
    #[serde(default)]
    pub dream_seed: Option<f32>,
    /// This archetype can be ridden (ADR 0020): the content-defined mount
    /// class a rider must be allowed to pilot. `None` = not a mount.
    #[serde(default)]
    pub mount_class: Option<String>,
    /// Mount classes a *rider* of this archetype may pilot (ADR 0020).
    /// Empty = this archetype cannot mount anything. A shark-rider carries
    /// `["shark"]`; it cannot board a `"mech"`-class mount.
    #[serde(default)]
    pub pilotable_mount_classes: Vec<String>,
    /// Damage this *mount* splashes onto its rider when it dies (ADR 0020).
    /// `None` = the rider drops unharmed (a `MountDeathImpact::Dismount`);
    /// `Some(n)` = the rider takes `n` damage (a mech exploding).
    #[serde(default)]
    pub mount_death_splash: Option<i32>,
    #[serde(default, with = "vec2_option")]
    pub default_size: Option<ae::Vec2>,
    /// Brain template the spawn site instantiates for this archetype.
    /// MeleeBrute reads the archetype's tunings (chase_speed,
    /// aggro_radius, attack_range) for its cfg; Wanderer + StandStill
    /// ignore them.
    pub brain_template: CharacterBrainTemplate,
    /// Concrete melee action this archetype's `ActionSet` carries.
    /// `None` = no melee capability (peaceful patrollers, ranged-only
    /// actors).
    #[serde(default)]
    pub melee: Option<ambition_characters::brain::MeleeActionSpec>,
    /// Concrete ranged action this archetype's `ActionSet` carries.
    /// `None` = no ranged capability.
    #[serde(default)]
    pub ranged: Option<ambition_characters::brain::RangedActionSpec>,
    /// Optional held-item id, resolved against the held-item registry
    /// (`ambition_characters::brain::held_item_by_id`). The item's abilities overlay the
    /// archetype action set at spawn / state transitions so weapons, not
    /// ad-hoc Rust branches, own whether an actor can melee or fire.
    #[serde(default)]
    pub held_item: Option<String>,
    /// Smash-brain melee hit band (the `attack_range`/engage sizing the
    /// `Smash` template uses, distinct from the CharacterAI stop-distance
    /// `attack_range` above). `None` for non-Smash archetypes; the Smash
    /// config builder falls back to a 36px default. Moving this out of the
    /// `smash_cfg_for_archetype` match arms (CharacterAI migration, #194)
    /// so a new Smash enemy is a data row, not a code edit.
    #[serde(default)]
    pub smash_hit_band: Option<f32>,
    /// Smash-template heavy base: longer reach + slower chase
    /// (`SmashCfg::BRUTE_DEFAULT`) vs the lighter striker default. Inert
    /// unless `brain_template` is `Smash`.
    #[serde(default)]
    pub smash_heavy: bool,
    /// Smash-template dash-to-close: a richer action set that dashes to
    /// close a large gap (goblins). Inert unless `brain_template` is `Smash`.
    #[serde(default)]
    pub smash_dash_to_close: bool,
    /// Smash-template **duelist neutral game**: footsies (weave in/out of poke
    /// range), neutral hops, and a real spacing/retreat rhythm instead of the
    /// grunt's close-and-camp (`SmashCfg::DUELIST_DEFAULT` base). Set for the
    /// "platform fighter" archetypes (the PCA, the player-robot) so they MOVE
    /// and space rather than mash at point-blank. Inert unless `brain_template`
    /// is `Smash`; the per-flag kit (blink/shield/dash/fly) still layers on top.
    #[serde(default)]
    pub smash_duelist: bool,
    // --- Movement kit ---
    //
    // The verbs THIS body has, independent of which brain drives it — the
    // character IS its movement kit. Each authored verb feeds ONE authored
    // [`ae::AbilitySet`] (`movement_kit`) that both ports read: the body
    // unions it into its `AbilitySet` at spawn (`ActorBody::from_kit`, enforce)
    // and the Smash brain reads the same verbs to decide when to attempt them
    // (`brain_spec`, attempt). No `smash_` prefix: these are body capabilities,
    // not Smash-template tuning (cf. `smash_heavy`/`smash_duelist`, which ARE).
    /// Movement kit: this body can **blink** (short-range teleport).
    #[serde(default)]
    pub can_blink: bool,
    /// Movement kit: grounded-base **hybrid flyer** — prefers to fight grounded
    /// but takes to the air to cover a long traversal gap. (`is_aerial` bodies
    /// fly unconditionally; this is the grounded-base opt-in.)
    #[serde(default)]
    pub can_fly: bool,
    /// Movement kit: this body can **reactive-block** — raise a shield to guard a
    /// perceived lunge it won't blink away from.
    #[serde(default)]
    pub can_shield: bool,
    /// Movement kit: this body can **dash** — a short burst above walk speed
    /// (see `smash_dash_to_close` for the Smash brain's *decision* to dash; the
    /// brain always attempts a dash via its Dash action, this lets the body
    /// turn it into a real burst).
    #[serde(default)]
    pub can_dash: bool,
    /// When provoked from peaceful, force an aggressive MeleeBrute brain
    /// with at least this aggro radius (cove PirateHeavy crew). `None` =
    /// use the template's default aggressive brain.
    #[serde(default)]
    pub provoke_forced_brute_min_aggro: Option<f32>,
    /// Hostile by default: actively tracks the player and publishes contact
    /// damage. Peaceful patrollers (cove crew, ambient wildlife) set false
    /// and stay dormant until a system explicitly provokes them.
    #[serde(default = "default_true")]
    pub attacks_player: bool,
    /// Body touch hurts the player. Training dummies and the composite shark
    /// (whose rider is the threat) opt out; the peaceful cove crew also
    /// stay non-damaging until provoked.
    #[serde(default = "default_true")]
    pub body_contact_damage: bool,
    /// Visual identity of this archetype's ranged projectile. Authored so the
    /// render layer selects shot art by KIND (e.g. `Glider` for the Perfect
    /// Cell-ular Automaton) instead of sniffing the owner-id string. Defaults
    /// to the generic `EnemyDefault` (orange shot); archetypes with a distinct
    /// projectile look name it explicitly.
    #[serde(default)]
    pub ranged_visual: crate::projectile::ProjectileVisualKind,
    /// Data-driven signature MOVE repertoire — the Smash-model moveset this
    /// character carries (windows / hit volumes / timed effects, authored on the
    /// owner's proper-time clock). Attached at spawn as an `ActorMoveset`; a control
    /// verb edge (`special`/`attack`) triggers the matching move through the shared
    /// moveset runtime (`combat::moveset`). This is how a character's expressive,
    /// boss-grade moves are designed AS DATA (the engine-for-2D-platformers vision) —
    /// the PCA is the first consumer (fable review 2026-07-02 §A1, Path B). `None`
    /// for characters whose combat is only the flat `melee`/`ranged` `ActionSet`.
    #[serde(default)]
    pub signature_move: Option<ambition_entity_catalog::MovesetContract>,
    /// Locomotion style for the actor's `ActionSet.move_style`.
    pub move_style: ambition_characters::brain::MoveStyleSpec,
}

/// Serde default for the `bool` spec fields that are true for the common
/// case (`attacks_player`, `body_contact_damage`).
fn default_true() -> bool {
    true
}

/// Glue: `Option<ae::Vec2>` deserializes from a `(x, y)` tuple in RON
/// or an explicit `None`. `bevy_math::Vec2` doesn't implement
/// `Deserialize` directly under the features the sandbox compiles
/// with, so route through a tuple shim.
mod vec2_option {
    use ambition_engine_core as ae;
    use serde::Deserialize;

    pub fn deserialize<'de, D>(de: D) -> Result<Option<ae::Vec2>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: Option<(f32, f32)> = Option::deserialize(de)?;
        Ok(raw.map(|(x, y)| ae::Vec2::new(x, y)))
    }
}

/// Brain template choice keyed off `CharacterArchetype`. The definition is
/// generic kit vocabulary — re-exported here so the archetype spec row
/// (`brain_template`) and the spawn-site projection keep their existing
/// path. See [`crate::features::ecs::actor_tuning::CharacterBrainTemplate`].
pub(super) use crate::features::ecs::actor_tuning::CharacterBrainTemplate;

/// Every authored spawn brain key in the lib's fixture roster — the
/// string-keyed replacement for the deleted `CharacterArchetype` iteration
/// constants. `COMBAT_*` excludes the training-dummy + raw-mite rows that
/// don't run the standard combat AI loop (was `COMBAT_ALL`).
#[cfg(test)]
pub(crate) const COMBAT_BRAIN_KEYS: &[&str] = &[
    "combatant",
    "small_skitter",
    "small_lurker",
    "medium_striker",
    "large_brute",
    "large_colossus",
    "gradient_seeker",
    "pirate_raider",
    "burning_flying_shark",
    "pirate_shark_rider",
    "puppy_slug",
    "pirate_heavy",
    "pirate_heavy_shark_rider",
    "cellular_automaton_fighter",
];

/// Every authored row in the fixture (combat + training dummies + raw mites).
#[cfg(test)]
pub(crate) const ALL_BRAIN_KEYS: &[&str] = &[
    "combatant",
    "small_skitter",
    "small_lurker",
    "medium_striker",
    "large_brute",
    "large_colossus",
    "gradient_seeker",
    "sandbag_infinite",
    "sandbag_finite",
    "pirate_raider",
    "burning_flying_shark",
    "pirate_shark_rider",
    "pirate_heavy",
    "pirate_heavy_shark_rider",
    "puppy_slug",
    "exploding_mite",
    "dividing_mite",
    "ranged_skirmisher",
];

impl CharacterArchetypeSpec {
    /// Project the generic brain-construction inputs (kit vocabulary) the
    /// runtime brain rebuilds reconstruct without naming the roster.
    pub(super) fn brain_spec(&self) -> crate::features::ecs::actor_tuning::CharacterBrainSpec {
        crate::features::ecs::actor_tuning::CharacterBrainSpec {
            template: self.brain_template,
            smash_hit_band: self.smash_hit_band.unwrap_or(
                crate::features::ecs::actor_tuning::CharacterBrainSpec::DEFAULT_SMASH_HIT_BAND,
            ),
            smash_heavy: self.smash_heavy,
            smash_dash_to_close: self.smash_dash_to_close,
            smash_duelist: self.smash_duelist,
            smash_can_blink: self.can_blink,
            smash_can_fly: self.can_fly,
            smash_can_shield: self.can_shield,
            provoke_forced_brute_min_aggro: self.provoke_forced_brute_min_aggro,
        }
    }

    /// The character's authored **movement kit** as an [`ae::AbilitySet`] — the
    /// verbs this body HAS, in the one movement-capability vocabulary every body
    /// (player, enemy, boss) shares. This is the single authored source both
    /// ports read: the body unions it into its live `AbilitySet` at spawn
    /// (`ActorBody::from_kit`), and the Smash brain reads the same verbs to
    /// decide when to attempt them (`brain_spec`). Only the kit verbs are set;
    /// locomotion (run/jump) and the `attack` verb are layered on by the body
    /// seed, and `is_aerial` flight is forced there too.
    pub(crate) fn movement_kit(&self) -> ae::AbilitySet {
        ae::AbilitySet {
            blink: self.can_blink,
            fly: self.can_fly,
            shield: self.can_shield,
            dash: self.can_dash,
            ..ae::AbilitySet::NONE
        }
    }

    /// Authored held item resolved against the held-item registry.
    pub(super) fn held_item_spec(&self) -> Option<ambition_characters::brain::HeldItemSpec> {
        self.held_item
            .as_deref()
            .and_then(ambition_characters::brain::held_item_by_id)
    }

    /// Concrete melee/ranged/locomotion the actor's `ActionSet` carries
    /// at spawn. Thin field accessors so the spawn path can read the spec
    /// without naming the roster enum.
    pub(super) fn melee_spec(&self) -> Option<ambition_characters::brain::MeleeActionSpec> {
        self.melee.clone()
    }
    pub(super) fn ranged_spec(&self) -> Option<ambition_characters::brain::RangedActionSpec> {
        self.ranged.clone()
    }
    pub(super) fn move_style(&self) -> ambition_characters::brain::MoveStyleSpec {
        self.move_style
    }

    /// Project the per-frame runtime tuning carried on `ActorConfig.tuning`.
    pub(crate) fn tuning(&self) -> crate::features::ecs::actor_tuning::ActorTuning {
        crate::features::ecs::actor_tuning::ActorTuning {
            // Resolved at roster-build time from the archetype hierarchy
            // (BASELINE <- inherits-chain <- this row's `movement` patch).
            movement: self.movement_resolved,
            max_health: self.max_health,
            patrol_speed: self.patrol_speed,
            chase_speed: self.chase_speed,
            // Ground-run capability = the fastest this body locomotes; the brain
            // expresses patrol/chase (with jitter) as a throttle of it.
            max_run_speed: self.patrol_speed.max(self.chase_speed),
            aggro_radius: self.aggro_radius,
            attack_range: self.attack_range,
            contact_strength: self.contact_strength,
            damage_amount: self.damage_amount,
            attack_cooldown_mult: self.attack_cooldown_mult,
            attacks_player: self.attacks_player,
            surface_walker: self.surface_walker,
            cling_breaks_on_hit: self.cling_breaks_on_hit,
            // The ONE authored respawn policy (ADR 0022) — the kill hook and
            // the in-place revive tick both match on it.
            respawn: self.respawn,
            weight: self.weight,
            death_policy: self.death_policy,
            is_aerial: self.is_aerial,
            // Archetype flyers use smoothed accel flight; direct-velocity is a boss
            // opt-in (its brain commands exact velocities). See AS4.
            flight_direct_velocity: false,
            is_sandbag: self.is_sandbag,
            body_contact_damage: self.body_contact_damage,
            dream_seed: self.dream_seed,
            ranged_visual: self.ranged_visual,
        }
    }

    /// Project the authored combat-CONSEQUENCE flags (death behaviors + weapon
    /// drop) into the combat kit. Movement capability is NOT here — it lives on
    /// the body's `AbilitySet` (see [`Self::movement_kit`]).
    pub(crate) fn combat_capabilities(&self) -> crate::combat::CombatCapabilities {
        crate::combat::CombatCapabilities {
            explodes_on_death: self.explodes_on_death,
            divides_on_death: self.divides_on_death,
            charge_crash_explodes: self.charge_crash_explodes,
            never_dies: self.never_dies,
            drops_held_item: self.held_item_spec(),
        }
    }
}

/// Serde default for [`CharacterArchetypeSpec::attack_cooldown_mult`]: the
/// multiplicative identity (most archetypes use the shared cooldown).
fn default_attack_cooldown_mult() -> f32 {
    1.0
}

fn default_mass() -> f32 {
    1.0
}

/// Serde default for [`CharacterArchetypeSpec::weight`] (CM1): the reference
/// body, so knockback growth divides by 1.0 for every un-authored archetype.
fn default_weight() -> f32 {
    1.0
}

/// App-local hostile-archetype authority: a brain-key → spec table plus the
/// fallback used for unknown brain keys and non-`Custom` brains. This is the
/// spawn path's only resolution surface and it is **roster-enum-free** — a
/// pure string lookup, so the named `CharacterArchetype` enum / RON / brain-name
/// table can be owned and installed by the content layer.
///
/// Providers assemble this resource transactionally inside each Bevy App.
/// Runtime systems receive `Res<CharacterRoster>` and pure construction helpers
/// receive `&CharacterRoster`; no process-global fallback participates in
/// production resolution.
#[derive(bevy::prelude::Resource, Clone, Debug)]
pub struct CharacterRoster {
    by_brain: std::collections::HashMap<String, CharacterArchetypeSpec>,
    fallback: CharacterArchetypeSpec,
    #[cfg(test)]
    provider_fallbacks: std::collections::BTreeMap<String, CharacterArchetypeSpec>,
}

impl CharacterRoster {
    /// Build a roster from a brain-key → spec table and the fallback spec
    /// (resolved for any unknown brain key, mirroring `from_brain`'s
    /// `Combatant` default).
    pub(crate) fn new(
        by_brain: std::collections::HashMap<String, CharacterArchetypeSpec>,
        fallback: CharacterArchetypeSpec,
    ) -> Self {
        Self {
            by_brain,
            fallback,
            #[cfg(test)]
            provider_fallbacks: std::collections::BTreeMap::new(),
        }
    }

    fn with_provider_fallbacks(
        by_brain: std::collections::HashMap<String, CharacterArchetypeSpec>,
        fallback: CharacterArchetypeSpec,
        provider_fallbacks: std::collections::BTreeMap<String, CharacterArchetypeSpec>,
    ) -> Self {
        #[cfg(not(test))]
        let _ = &provider_fallbacks;
        Self {
            by_brain,
            fallback,
            #[cfg(test)]
            provider_fallbacks,
        }
    }

    /// Resolve one provider's authored default without making it the default
    /// for every other game linked into the App.
    #[cfg(test)]
    pub(crate) fn fallback_for_provider(
        &self,
        provider_id: &str,
    ) -> Option<&CharacterArchetypeSpec> {
        self.provider_fallbacks.get(provider_id)
    }

    /// Invariant: a practice-target ("sandbag" / `is_sandbag`) archetype is
    /// PASSIVE — it carries no melee attack and never strikes back. Pins the
    /// authored roster against accidentally giving a dummy a counter-attack.
    pub fn sandbags_are_passive(&self) -> bool {
        self.by_brain
            .values()
            .chain(std::iter::once(&self.fallback))
            .all(|spec| !spec.is_sandbag || spec.melee.is_none())
    }

    #[cfg(test)]
    pub(crate) fn contains_brain(&self, brain_id: &str) -> bool {
        self.by_brain.contains_key(brain_id)
    }

    /// Resolve the authored spec for a spawn `CharacterBrain` payload by its
    /// `Custom("…")` brain key, falling back to the roster's default for an
    /// unknown key or a non-`Custom` brain.
    pub(crate) fn spec_for_brain(
        &self,
        brain: &ambition_entity_catalog::placements::CharacterBrain,
    ) -> CharacterArchetypeSpec {
        let key = match brain {
            ambition_entity_catalog::placements::CharacterBrain::Custom(name) => name.as_str(),
            _ => "",
        };
        self.by_brain
            .get(key)
            .cloned()
            .unwrap_or_else(|| self.fallback.clone())
    }

    /// Build a roster from a brain-keyed spec map. The reserved `"combatant"`
    /// row is the fallback for unknown brain keys (mirroring the legacy
    /// `from_brain` default). This is the roster-enum-free construction path:
    /// the map keys ARE the spawn brain keys, so no `CharacterArchetype` is named.
    pub(crate) fn from_map(
        mut by_brain: std::collections::HashMap<String, CharacterArchetypeSpec>,
    ) -> Self {
        // Resolve each archetype's movement tuning by folding its patch along the
        // inheritance chain. Done HERE — the single chokepoint every roster passes
        // through — because inheritance needs sibling specs the per-row `tuning()`
        // builder can't see.
        resolve_movement_inheritance(&mut by_brain);
        let fallback = by_brain
            .get("combatant")
            .cloned()
            .expect("enemy roster must define a \"combatant\" fallback row");
        Self::new(by_brain, fallback)
    }

    /// Internal parser used by the engine-generic empty default and test fixture.
    /// Provider code uses the fallible [`CharacterRosterFragment::from_ron`].
    fn from_ron(ron: &str) -> Self {
        let by_brain: std::collections::HashMap<String, CharacterArchetypeSpec> =
            ron::from_str(ron)
                .unwrap_or_else(|err| panic!("enemy roster RON failed to deserialize: {err}"));
        Self::from_map(by_brain)
    }
}

/// Fold every archetype's authored movement patch along its inheritance chain and
/// store the resolved [`crate::combat::BodyMovementTuning`] back on each spec.
/// `BASELINE ← parent (resolved) ← this row's patch`; a missing parent or a cycle
/// falls back to the baseline rather than panicking (a malformed `inherits` is a
/// data smell, not a crash).
fn resolve_movement_inheritance(
    specs: &mut std::collections::HashMap<String, CharacterArchetypeSpec>,
) {
    // Snapshot the authored (patch, parent) so resolution reads immutable data
    // while we write resolved values back into the same map.
    let raw: std::collections::HashMap<String, (crate::combat::BodyMovementPatch, Option<String>)> =
        specs
            .iter()
            .map(|(k, s)| (k.clone(), (s.movement, s.inherits.clone())))
            .collect();
    let resolved: std::collections::HashMap<String, crate::combat::BodyMovementTuning> = raw
        .keys()
        .map(|k| {
            (
                k.clone(),
                resolve_movement_for(&raw, k, &mut vec![k.clone()]),
            )
        })
        .collect();
    // AMBITION_REVIEW(determinism): hash-order iteration is safe here. Each step
    // writes only its OWN key's `movement_resolved` from an already-resolved
    // lookup, so the pass is commutative — the map's contents after it are
    // identical for every visit order, and nothing observes the order itself.
    for (k, spec) in specs.iter_mut() {
        if let Some(tuning) = resolved.get(k) {
            spec.movement_resolved = *tuning;
        }
    }
}

/// Recursively resolve one archetype's movement tuning. `seen` carries the chain
/// so a cycle (or self-reference) stops at the baseline instead of recursing
/// forever.
fn resolve_movement_for(
    raw: &std::collections::HashMap<String, (crate::combat::BodyMovementPatch, Option<String>)>,
    id: &str,
    seen: &mut Vec<String>,
) -> crate::combat::BodyMovementTuning {
    let Some((patch, parent)) = raw.get(id) else {
        return crate::combat::BodyMovementTuning::BASELINE;
    };
    let base = match parent {
        Some(parent_id) if !seen.iter().any(|s| s == parent_id) => {
            seen.push(parent_id.clone());
            resolve_movement_for(raw, parent_id, seen)
        }
        // No parent, or a cycle/unknown parent → start from the generic baseline.
        _ => crate::combat::BodyMovementTuning::BASELINE,
    };
    patch.apply_onto(base)
}

/// Engine-generic fallback used by Apps that intentionally register no hostile
/// archetype content. It is inert and exists only so the reusable engine can run
/// menu/demo worlds without installing Ambition's authored enemy table.
const CONTENT_FREE_ROSTER_RON: &str = r#"{
    "combatant": (
        max_health: 1,
        patrol_speed: 0.0,
        chase_speed: 0.0,
        aggro_radius: 0.0,
        attack_range: 0.0,
        contact_strength: 0.0,
        damage_amount: 0,
        brain_template: StandStill,
        move_style: Walk,
        attacks_player: false,
        body_contact_damage: false,
    ),
}"#;

impl Default for CharacterRoster {
    fn default() -> Self {
        Self::from_ron(CONTENT_FREE_ROSTER_RON)
    }
}

/// One provider's immutable hostile-archetype definitions.
#[derive(Clone, Debug)]
pub struct CharacterRosterFragment {
    provider_id: String,
    fallback_brain_id: Option<String>,
    by_brain: std::collections::BTreeMap<String, CharacterArchetypeSpec>,
    source_ron: String,
}

impl CharacterRosterFragment {
    pub fn from_ron(
        provider_id: impl Into<String>,
        fallback_brain_id: Option<impl Into<String>>,
        roster_ron: &str,
    ) -> Result<Self, CharacterRosterAssemblyError> {
        let provider_id = provider_id.into();
        if provider_id.trim().is_empty() {
            return Err(CharacterRosterAssemblyError::EmptyProviderId);
        }
        let by_brain =
            ron::from_str::<std::collections::BTreeMap<String, CharacterArchetypeSpec>>(roster_ron)
                .map_err(|error| CharacterRosterAssemblyError::MalformedFragment {
                    provider_id: provider_id.clone(),
                    message: error.to_string(),
                })?;
        let fragment = Self {
            provider_id,
            fallback_brain_id: fallback_brain_id.map(Into::into),
            by_brain,
            source_ron: roster_ron.to_string(),
        };
        fragment.validate()?;
        Ok(fragment)
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn fallback_brain_id(&self) -> Option<&str> {
        self.fallback_brain_id.as_deref()
    }

    fn validate(&self) -> Result<(), CharacterRosterAssemblyError> {
        if self.provider_id.trim().is_empty() {
            return Err(CharacterRosterAssemblyError::EmptyProviderId);
        }
        if let Some(brain_id) = self
            .by_brain
            .keys()
            .find(|brain_id| brain_id.trim().is_empty())
        {
            return Err(CharacterRosterAssemblyError::EmptyBrainId {
                provider_id: self.provider_id.clone(),
                brain_id: brain_id.clone(),
            });
        }
        if let Some(fallback) = self.fallback_brain_id.as_deref() {
            if fallback.trim().is_empty() {
                return Err(CharacterRosterAssemblyError::EmptyFallbackBrainId {
                    provider_id: self.provider_id.clone(),
                });
            }
            if !self.by_brain.contains_key(fallback) {
                return Err(CharacterRosterAssemblyError::MissingFallbackBrain {
                    provider_id: self.provider_id.clone(),
                    brain_id: fallback.to_string(),
                });
            }
        }
        Ok(())
    }
}

/// All hostile-archetype fragments linked into one Bevy App.
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct CharacterRosterRegistry {
    fragments: std::collections::BTreeMap<String, CharacterRosterFragment>,
}

impl CharacterRosterRegistry {
    pub fn providers(&self) -> impl Iterator<Item = &str> {
        self.fragments.keys().map(String::as_str)
    }

    pub fn register(
        &mut self,
        fragment: CharacterRosterFragment,
    ) -> Result<(), CharacterRosterAssemblyError> {
        fragment.validate()?;
        if let Some(existing) = self.fragments.get(&fragment.provider_id) {
            if existing.fallback_brain_id == fragment.fallback_brain_id
                && existing.source_ron == fragment.source_ron
            {
                return Ok(());
            }
            return Err(CharacterRosterAssemblyError::DuplicateProvider {
                provider_id: fragment.provider_id,
            });
        }
        self.fragments
            .insert(fragment.provider_id.clone(), fragment);
        Ok(())
    }

    pub fn assemble(&self) -> Result<CharacterRoster, CharacterRosterAssemblyError> {
        let mut by_brain = std::collections::HashMap::new();
        let mut owners = std::collections::BTreeMap::<String, String>::new();
        let mut provider_fallback_ids = std::collections::BTreeMap::<String, String>::new();
        for (provider_id, fragment) in &self.fragments {
            for (brain_id, spec) in &fragment.by_brain {
                if let Some(first_provider) = owners.get(brain_id) {
                    return Err(CharacterRosterAssemblyError::DuplicateBrain {
                        brain_id: brain_id.clone(),
                        first_provider: first_provider.clone(),
                        second_provider: provider_id.clone(),
                    });
                }
                owners.insert(brain_id.clone(), provider_id.clone());
                by_brain.insert(brain_id.clone(), spec.clone());
            }
            if let Some(brain_id) = fragment.fallback_brain_id.as_ref() {
                provider_fallback_ids.insert(provider_id.clone(), brain_id.clone());
            }
        }
        resolve_movement_inheritance(&mut by_brain);
        let mut provider_fallbacks = std::collections::BTreeMap::new();
        for (provider_id, fallback_brain) in provider_fallback_ids {
            let spec = by_brain.get(&fallback_brain).cloned().ok_or_else(|| {
                CharacterRosterAssemblyError::MissingAssembledFallback {
                    brain_id: fallback_brain.clone(),
                }
            })?;
            provider_fallbacks.insert(provider_id, spec);
        }
        // Preserve the historical single-game fallback without allowing two
        // linked providers to fight over one process-wide default. A host with
        // multiple provider defaults must select one through session authority;
        // until then, unknown/non-Custom brains use the inert engine fallback.
        let fallback = if provider_fallbacks.len() == 1 {
            provider_fallbacks
                .values()
                .next()
                .expect("length checked")
                .clone()
        } else {
            CharacterRoster::default().fallback
        };
        Ok(CharacterRoster::with_provider_fallbacks(
            by_brain,
            fallback,
            provider_fallbacks,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CharacterRosterAssemblyError {
    EmptyProviderId,
    EmptyBrainId {
        provider_id: String,
        brain_id: String,
    },
    EmptyFallbackBrainId {
        provider_id: String,
    },
    DuplicateProvider {
        provider_id: String,
    },
    MalformedFragment {
        provider_id: String,
        message: String,
    },
    MissingFallbackBrain {
        provider_id: String,
        brain_id: String,
    },
    DuplicateBrain {
        brain_id: String,
        first_provider: String,
        second_provider: String,
    },
    MissingAssembledFallback {
        brain_id: String,
    },
}

impl std::fmt::Display for CharacterRosterAssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyProviderId => write!(f, "character roster provider id must not be empty"),
            Self::EmptyBrainId {
                provider_id,
                brain_id,
            } => write!(
                f,
                "character roster fragment '{provider_id}' contains empty brain id '{brain_id}'"
            ),
            Self::EmptyFallbackBrainId { provider_id } => write!(
                f,
                "character roster fragment '{provider_id}' names an empty fallback brain id"
            ),
            Self::DuplicateProvider { provider_id } => {
                write!(f, "character roster provider '{provider_id}' registered twice")
            }
            Self::MalformedFragment {
                provider_id,
                message,
            } => write!(
                f,
                "character roster fragment '{provider_id}' is malformed RON: {message}"
            ),
            Self::MissingFallbackBrain {
                provider_id,
                brain_id,
            } => write!(
                f,
                "character roster fragment '{provider_id}' names missing fallback brain '{brain_id}'"
            ),
            Self::DuplicateBrain {
                brain_id,
                first_provider,
                second_provider,
            } => write!(
                f,
                "character brain id '{brain_id}' is authored by both '{first_provider}' and '{second_provider}'"
            ),
            Self::MissingAssembledFallback { brain_id } => write!(
                f,
                "assembled character roster is missing fallback brain '{brain_id}'"
            ),
        }
    }
}

impl std::error::Error for CharacterRosterAssemblyError {}

/// Bevy build-time registration seam for provider-owned hostile archetypes.
pub trait CharacterRosterAppExt {
    fn try_register_character_roster_fragment(
        &mut self,
        fragment: CharacterRosterFragment,
    ) -> Result<&mut Self, CharacterRosterAssemblyError>;

    fn register_character_roster_fragment(
        &mut self,
        fragment: CharacterRosterFragment,
    ) -> &mut Self {
        self.try_register_character_roster_fragment(fragment)
            .unwrap_or_else(|error| panic!("{error}"))
    }
}

impl CharacterRosterAppExt for bevy::prelude::App {
    fn try_register_character_roster_fragment(
        &mut self,
        fragment: CharacterRosterFragment,
    ) -> Result<&mut Self, CharacterRosterAssemblyError> {
        let (registry, roster) = {
            let mut candidate = self
                .world()
                .get_resource::<CharacterRosterRegistry>()
                .cloned()
                .unwrap_or_default();
            candidate.register(fragment)?;
            let roster = candidate.assemble()?;
            (candidate, roster)
        };
        self.insert_resource(registry).insert_resource(roster);
        Ok(self)
    }
}

#[cfg(test)]
pub(crate) fn test_roster() -> CharacterRoster {
    CharacterRoster::from_ron(include_str!(
        "../../../../../game/ambition_content/assets/data/character_archetypes.ron"
    ))
}

/// Resolve a spec by its spawn brain key against the checked-in Ambition test
/// fixture. Production callers always receive an explicit App-local roster.
#[cfg(test)]
pub(crate) fn test_spec(brain_key: &str) -> CharacterArchetypeSpec {
    test_roster().spec_for_brain(
        &ambition_entity_catalog::placements::CharacterBrain::Custom(brain_key.to_string()),
    )
}

#[cfg(test)]
mod app_local_roster_tests {
    use super::*;

    const A: &str = r#"{
        "combatant": (
            max_health: 2, patrol_speed: 0.0, chase_speed: 0.0,
            aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
            damage_amount: 0, brain_template: StandStill, move_style: Walk,
        ),
    }"#;
    const B: &str = r#"{
        "beta": (
            max_health: 7, patrol_speed: 0.0, chase_speed: 0.0,
            aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
            damage_amount: 0, brain_template: StandStill, move_style: Walk,
        ),
    }"#;
    const B_WITH_DEFAULT: &str = r#"{
        "beta": (
            max_health: 7, patrol_speed: 0.0, chase_speed: 0.0,
            aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
            damage_amount: 0, brain_template: StandStill, move_style: Walk,
        ),
    }"#;

    #[test]
    fn provider_order_is_deterministic_and_separate_apps_are_isolated() {
        let a = CharacterRosterFragment::from_ron("a", Some("combatant"), A).unwrap();
        let b = CharacterRosterFragment::from_ron("b", None::<String>, B).unwrap();
        let mut first = bevy::prelude::App::new();
        first.register_character_roster_fragment(a.clone());
        first.register_character_roster_fragment(b.clone());
        let mut second = bevy::prelude::App::new();
        second.register_character_roster_fragment(b);
        second.register_character_roster_fragment(a);
        let brain = ambition_entity_catalog::placements::CharacterBrain::Custom("beta".into());
        assert_eq!(
            first
                .world()
                .resource::<CharacterRoster>()
                .spec_for_brain(&brain)
                .max_health,
            7
        );
        assert_eq!(
            second
                .world()
                .resource::<CharacterRoster>()
                .spec_for_brain(&brain)
                .max_health,
            7
        );

        let mut isolated = bevy::prelude::App::new();
        isolated.register_character_roster_fragment(
            CharacterRosterFragment::from_ron("a", Some("combatant"), A).unwrap(),
        );
        assert_eq!(
            isolated
                .world()
                .resource::<CharacterRoster>()
                .spec_for_brain(&brain)
                .max_health,
            2,
            "the second App must not observe provider b"
        );
    }

    #[test]
    fn failed_registration_preserves_the_previous_roster() {
        let mut app = bevy::prelude::App::new();
        app.register_character_roster_fragment(
            CharacterRosterFragment::from_ron("a", Some("combatant"), A).unwrap(),
        );
        let error = app
            .try_register_character_roster_fragment(
                CharacterRosterFragment::from_ron("b", None::<String>, A).unwrap(),
            )
            .err()
            .expect("duplicate brain id should fail");
        assert!(matches!(
            error,
            CharacterRosterAssemblyError::DuplicateBrain { .. }
        ));
        let brain = ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into());
        assert_eq!(
            app.world()
                .resource::<CharacterRoster>()
                .spec_for_brain(&brain)
                .max_health,
            2
        );
        assert_eq!(
            app.world()
                .resource::<CharacterRosterRegistry>()
                .providers()
                .collect::<Vec<_>>(),
            vec!["a"]
        );
    }

    #[test]
    fn provider_defaults_coexist_without_becoming_a_cross_game_global() {
        let mut app = bevy::prelude::App::new();
        app.register_character_roster_fragment(
            CharacterRosterFragment::from_ron("a", Some("combatant"), A).unwrap(),
        );
        app.register_character_roster_fragment(
            CharacterRosterFragment::from_ron("b", Some("beta"), B_WITH_DEFAULT).unwrap(),
        );
        let roster = app.world().resource::<CharacterRoster>();
        assert_eq!(roster.fallback_for_provider("a").unwrap().max_health, 2);
        assert_eq!(roster.fallback_for_provider("b").unwrap().max_health, 7);
        let unknown = ambition_entity_catalog::placements::CharacterBrain::Custom("unknown".into());
        assert_eq!(
            roster.spec_for_brain(&unknown).max_health,
            1,
            "without active-provider selection, an ambiguous default must not leak across games"
        );
    }

    #[test]
    fn provider_without_fallback_keeps_its_rows_and_uses_generic_default() {
        let mut app = bevy::prelude::App::new();
        app.register_character_roster_fragment(
            CharacterRosterFragment::from_ron("b", None::<String>, B).unwrap(),
        );
        let roster = app.world().resource::<CharacterRoster>();
        let beta = ambition_entity_catalog::placements::CharacterBrain::Custom("beta".into());
        let unknown = ambition_entity_catalog::placements::CharacterBrain::Custom("unknown".into());
        assert_eq!(roster.spec_for_brain(&beta).max_health, 7);
        assert_eq!(
            roster.spec_for_brain(&unknown).max_health,
            1,
            "an App with no provider fallback uses the explicit engine-generic default"
        );
    }
}

/// Whether a spawn payload is a sandbag (passive practice-target archetype).
/// The ONE surviving fragment of the deleted `enemy_visual_kind` derivation:
/// used at spawn to pick the static sandbag sprite (the rest of the
/// enemy/NPC/boss "kind" split was never a render type and collapsed into the
/// single `FeatureVisualKind::Actor`; live depiction is name-first + a
/// state-keyed fallback in `upgrade_actor_sprites`).
pub fn enemy_spawn_is_sandbag(
    roster: &CharacterRoster,
    payload: &ambition_entity_catalog::placements::CharacterBrain,
) -> bool {
    roster.spec_for_brain(payload).is_sandbag
}

#[cfg(test)]
mod capability_tests;
#[cfg(test)]
mod enemy_archetype_data_tests;
#[cfg(test)]
mod movement_tuning_tests;
