//! Enemy data + state for the actor simulation: the [`CharacterRoster`] of
//! archetype specs (loaded from `character_archetypes.ron`, installed via
//! [`install_enemy_roster`]), per-actor locomotion state ([`ActorSpawnState`],
//! [`ActorSurfaceState`]), surface-walker (PuppySlug) cling/wall predicates,
//! and composite-visual planning. The per-frame physics/AI tick lives in the
//! `integration` submodule; grounded enemies route through the shared
//! `integrate_normal_spine`, aerial ones through [`super::step_floating_body`].

use super::*;

mod integration;
pub use integration::ContactAttack;

/// Predicate matching any tile a surface-walker (PuppySlug) can
/// CLING TO — both solid blocks and one-way platforms count, mirroring
/// what step_kinematic treats as "ground" for grounded actors.
fn surface_solid_pred(b: &ae::Block) -> bool {
    matches!(
        b.kind,
        ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
    )
}

/// Predicate matching tiles a surface-walker treats as "walls in
/// the way" — strictly solid, NOT one-way. A one-way platform sitting
/// in the slug's path along a wall must not register as a concave
/// corner since the slug would never collide with its side anyway.
fn surface_wall_pred(b: &ae::Block) -> bool {
    matches!(
        b.kind,
        ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
    )
}

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
    /// Movement kit: this body can **blink** (short-range teleport). Authored
    /// per archetype; projects into BOTH the Smash brain's blink-evade emission
    /// (it *attempts* a blink on a perceived lunge) AND the body's
    /// [`crate::combat::CombatCapabilities::can_blink`] gate (the body *enforces*
    /// the capability + cooldown). One authored source, two projections —
    /// attempt vs enforce (invariants I2/I3/I7).
    #[serde(default)]
    pub smash_can_blink: bool,
    /// Movement kit: grounded-base **hybrid flyer** — prefers to fight grounded
    /// but takes to the air to cover a long traversal gap (brain preference;
    /// flight is free for now). Projects into BOTH `SmashCfg::can_fly` (attempt)
    /// and `CombatCapabilities::can_fly` (enforce).
    #[serde(default)]
    pub smash_can_fly: bool,
    /// Movement kit: this body can **reactive-block** — raise a shield to guard a
    /// perceived lunge it won't blink away from. Projects into BOTH the Smash
    /// brain's `can_shield` (it *attempts* a block: raises `shield_held` and
    /// stands its ground) AND the body's
    /// [`crate::combat::CombatCapabilities::can_shield`] gate (the body *enforces*
    /// the block — a guarded hit from the faced side is negated). One authored
    /// source, two projections — attempt vs enforce (invariants I2/I3/I7).
    #[serde(default)]
    pub smash_can_shield: bool,
    /// Movement kit: this body can **dash** — a short burst above walk speed when
    /// the brain commits a Dash (it dashes to close a gap; see `smash_dash_to_close`
    /// for the brain's *decision* to dash). Projects ONLY into the body's
    /// [`crate::combat::CombatCapabilities::can_dash`] enforce gate — the brain
    /// already attempts a dash via its Dash action, the body owns the burst.
    #[serde(default)]
    pub smash_can_dash: bool,
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

/// Test fixture for the lib's own unit tests: the AUTHORITATIVE roster lives in
/// `ambition_content`, and the lib's tests validate the generic spawn machinery
/// against that single source of truth by reading the same file at compile time
/// (`#[cfg(test)]` only — a production lib build embeds no enemy data; content
/// installs the roster via `install_enemy_roster`). The cross-crate include
/// keeps one roster file instead of a guarded duplicate.
#[cfg(test)]
static ENEMY_ARCHETYPE_REGISTRY: std::sync::LazyLock<
    std::collections::HashMap<String, CharacterArchetypeSpec>,
> = std::sync::LazyLock::new(|| {
    const ENEMY_ARCHETYPES_RON: &str =
        include_str!("../../../../../game/ambition_content/assets/data/character_archetypes.ron");
    ron::from_str(ENEMY_ARCHETYPES_RON).unwrap_or_else(|err| {
        panic!("ambition_content character_archetypes.ron failed to deserialize: {err}")
    })
});

/// The installed enemy roster: a brain-key → spec table plus the fallback
/// spec used for unknown brain keys and non-`Custom` brains. This is the
/// spawn path's only resolution surface and it is **roster-enum-free** — a
/// pure string lookup, so the named `CharacterArchetype` enum / RON / brain-name
/// table can be owned and installed by the content layer.
///
/// Held as an installable global (not a Bevy `Resource`) because spec
/// resolution is read from many non-system contexts — plain constructors
/// (`ActorClusterSeed::new`), presentation sprite-binding
/// (`presentation::rendering::world`), and asset resolution
/// (`assets::game_assets`) — where threading `Res<CharacterRoster>` would be a
/// pervasive, ugly ripple. The content layer installs the real table at
/// startup via [`install_enemy_roster`]; the lib ships an embedded default
/// (built from the bundled RON) so lib tests and the headless bin resolve
/// standalone.
#[derive(Clone, Debug)]
pub struct CharacterRoster {
    by_brain: std::collections::HashMap<String, CharacterArchetypeSpec>,
    fallback: CharacterArchetypeSpec,
}

impl CharacterRoster {
    /// Build a roster from a brain-key → spec table and the fallback spec
    /// (resolved for any unknown brain key, mirroring `from_brain`'s
    /// `Combatant` default).
    pub(crate) fn new(
        by_brain: std::collections::HashMap<String, CharacterArchetypeSpec>,
        fallback: CharacterArchetypeSpec,
    ) -> Self {
        Self { by_brain, fallback }
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

    /// Parse a brain-keyed roster RON document — the content layer's entry
    /// point: `install_enemy_roster(CharacterRoster::from_ron(MY_RON))`. Movement
    /// inheritance is resolved by `from_map`.
    pub fn from_ron(ron: &str) -> Self {
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

/// Test-only fallback roster, parsed from the lib's bundled fixture RON so
/// the lib's own unit tests resolve enemies standalone (without the content
/// plugin). In a real build the named roster is owned and installed by
/// `ambition_content`; the production binary embeds no enemy data here.
#[cfg(test)]
static EMBEDDED_ENEMY_ROSTER: std::sync::LazyLock<CharacterRoster> =
    std::sync::LazyLock::new(|| CharacterRoster::from_map(ENEMY_ARCHETYPE_REGISTRY.clone()));

/// Content-installed roster. Set once at plugin-build time; production
/// resolution REQUIRES it (there is no production embedded default).
///
/// §5 classification (restructuring-blueprint): **content registry** —
/// install-once seam, immutable after install, read from the pure
/// `spec_for_brain` helper (called deep in non-system spawn code). Deliberately
/// a process-global `OnceLock`, not a Bevy `Resource`: the spawn-path readers
/// have no `World` access and a resource would couple pure spec resolution to
/// the ECS. `install_enemy_roster` + the `cfg(test)` fixture ARE the
/// test-override mechanism.
static ENEMY_ROSTER_OVERRIDE: std::sync::OnceLock<CharacterRoster> = std::sync::OnceLock::new();

/// Install the authored enemy roster — the content layer calls this at
/// plugin-build time (before any spawn system runs). First install wins; later
/// calls are ignored, so a mid-run call can't clobber the live roster.
pub fn install_enemy_roster(roster: CharacterRoster) {
    let _ = ENEMY_ROSTER_OVERRIDE.set(roster);
}

#[cfg(test)]
fn roster_fallback() -> &'static CharacterRoster {
    &EMBEDDED_ENEMY_ROSTER
}

/// Production has no embedded enemy data: the content plugin must install the
/// roster at build time. Reaching here means `AmbitionContentPlugin` was not
/// mounted before the first enemy spawn.
#[cfg(not(test))]
fn roster_fallback() -> &'static CharacterRoster {
    panic!(
        "enemy roster not installed — AmbitionContentPlugin must call \
         install_enemy_roster() at build time before any enemy spawns"
    )
}

fn enemy_roster() -> &'static CharacterRoster {
    ENEMY_ROSTER_OVERRIDE.get().unwrap_or_else(roster_fallback)
}

/// Resolve the authored spec for a spawn `CharacterBrain` payload — a pure
/// string lookup against the installed [`CharacterRoster`]. The spawn path holds
/// the returned spec; the roster enum never appears here.
pub(crate) fn spec_for_brain(
    brain: &ambition_entity_catalog::placements::CharacterBrain,
) -> CharacterArchetypeSpec {
    enemy_roster().spec_for_brain(brain)
}

/// Resolve a spec by its spawn brain key against the lib's `#[cfg(test)]`
/// fixture roster — the test-side replacement for the deleted
/// `CharacterArchetype::X.spec()`. Tests are roster-string-keyed now: the named
/// enum is gone, so they reference enemies by the same `Custom("…")` key the
/// game authors.
#[cfg(test)]
pub(crate) fn test_spec(brain_key: &str) -> CharacterArchetypeSpec {
    spec_for_brain(
        &ambition_entity_catalog::placements::CharacterBrain::Custom(brain_key.to_string()),
    )
}

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
            smash_can_blink: self.smash_can_blink,
            smash_can_fly: self.smash_can_fly,
            smash_can_shield: self.smash_can_shield,
            provoke_forced_brute_min_aggro: self.provoke_forced_brute_min_aggro,
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

    /// Project the authored capability flags into the combat kit.
    pub(crate) fn combat_capabilities(&self) -> crate::combat::CombatCapabilities {
        crate::combat::CombatCapabilities {
            explodes_on_death: self.explodes_on_death,
            divides_on_death: self.divides_on_death,
            charge_crash_explodes: self.charge_crash_explodes,
            never_dies: self.never_dies,
            drops_held_item: self.held_item_spec(),
            can_blink: self.smash_can_blink,
            can_fly: self.smash_can_fly,
            can_shield: self.smash_can_shield,
            can_dash: self.smash_can_dash,
        }
    }
}
/// Whether a spawn payload is a sandbag (passive practice-target archetype).
/// The ONE surviving fragment of the deleted `enemy_visual_kind` derivation:
/// used at spawn to pick the static sandbag sprite (the rest of the
/// enemy/NPC/boss "kind" split was never a render type and collapsed into the
/// single `FeatureVisualKind::Actor`; live depiction is name-first + a
/// state-keyed fallback in `upgrade_actor_sprites`).
pub fn enemy_spawn_is_sandbag(
    payload: &ambition_entity_catalog::placements::CharacterBrain,
) -> bool {
    spec_for_brain(payload).is_sandbag
}

#[cfg(test)]
mod capability_tests;
#[cfg(test)]
mod enemy_archetype_data_tests;
#[cfg(test)]
mod movement_tuning_tests;
