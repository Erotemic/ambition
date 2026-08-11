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

/// The enemy-ARCHETYPE authoring vocabulary.
///
/// ⚠ **DEFINED in `ambition_combat::archetype_spec`** since 2026-08-03. Every
/// field is combat/movement tuning and the two types it could not name from
/// lower down (`BodyMovementPatch`, `BodyMovementTuning`) are defined there. It
/// moved so the content compiler could own `character_archetypes.ron` without
/// linking this crate. The roster ASSEMBLY below — inheritance folding,
/// provider-local parents, transactional publication — is genuinely coupled to
/// this crate and stayed.
pub use ambition_combat::archetype_spec::ArchetypeSpec;

/// Glue: `Option<ae::Vec2>` deserializes from a `(x, y)` tuple in RON
/// or an explicit `None`. `bevy_math::Vec2` doesn't implement
/// `Deserialize` directly under the features the sandbox compiles
/// with, so route through a tuple shim.

/// Brain template choice keyed off `CharacterArchetype`. The definition is
/// generic kit vocabulary — re-exported here so the archetype spec row
/// (`brain_template`) and the spawn-site projection keep their existing
/// path. See [`crate::features::ecs::actor_tuning::CharacterBrainTemplate`].
///
/// ⚠ **`cfg(test)`: the production callers this re-export was made for are
/// gone.** Its own doc still claims the archetype spec row and the spawn-site
/// projection come through here; they do not — the only remaining users are this
/// module's `enemy_archetype_data_tests` and `capability_tests`. Left in place
/// rather than deleted because those tests read better naming it through the
/// family they are about, but gated so it stops costing the suite's
/// `no warnings (cargo check --all-targets)` job a warning on every run.
#[cfg(test)]
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

/// The actor-crate projections of an authored archetype row.
///
/// ⛔ **an extension TRAIT because the orphan rule says so**, not as a style
/// choice: `ArchetypeSpec` is defined in `ambition_combat` now, so an
/// inherent `impl` for it can only be written there — and every one of these
/// returns an ACTOR-crate type (`ActorTuning`, `CharacterBrainSpec`). The data
/// moved; the projections into this crate's runtime shapes stayed with the
/// shapes.
pub(crate) trait ArchetypeSpecExt {
    fn brain_spec(&self) -> crate::features::ecs::actor_tuning::CharacterBrainSpec;
    fn movement_kit(&self) -> ae::AbilitySet;
    fn held_item_spec(&self) -> Option<ambition_characters::brain::HeldItemSpec>;
    fn melee_spec(&self) -> Option<ambition_characters::brain::MeleeActionSpec>;
    fn ranged_spec(&self) -> Option<ambition_characters::brain::RangedActionSpec>;
    fn move_style(&self) -> ambition_characters::brain::MoveStyleSpec;
    fn tuning(&self) -> crate::features::ecs::actor_tuning::ActorTuning;
    fn combat_capabilities(&self) -> crate::combat::CombatCapabilities;
}

impl ArchetypeSpecExt for ArchetypeSpec {
    /// Project the generic brain-construction inputs (kit vocabulary) the
    /// runtime brain rebuilds reconstruct without naming the roster.
    fn brain_spec(&self) -> crate::features::ecs::actor_tuning::CharacterBrainSpec {
        crate::features::ecs::actor_tuning::CharacterBrainSpec {
            template: self.brain_template,
            // The archetype's authored rung, or the middle one. A fighter
            // archetype that says nothing plays at 5 rather than refusing.
            fighter_level: self.fighter_level.unwrap_or(5),
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
    fn movement_kit(&self) -> ae::AbilitySet {
        ae::AbilitySet {
            blink: self.can_blink,
            fly: self.can_fly,
            // ⛔ **paired with `fly`, and forgetting it is SILENT.** `NONE`
            // leaves `fly_toggle` false, which means PERMANENT flight — and
            // permanent flight is latched when the cluster is built, so a body
            // that gains the capability afterwards never flies at all. An
            // authored enemy that "can fly" means the ordinary toggled kind.
            fly_toggle: self.can_fly,
            shield: self.can_shield,
            dash: self.can_dash,
            ..ae::AbilitySet::NONE
        }
    }

    /// Authored held item resolved against the held-item registry.
    fn held_item_spec(&self) -> Option<ambition_characters::brain::HeldItemSpec> {
        self.held_item
            .as_deref()
            .and_then(ambition_characters::brain::held_item_by_id)
    }

    /// Concrete melee/ranged/locomotion the actor's `ActionSet` carries
    /// at spawn. Thin field accessors so the spawn path can read the spec
    /// without naming the roster enum.
    fn melee_spec(&self) -> Option<ambition_characters::brain::MeleeActionSpec> {
        self.melee.clone()
    }
    fn ranged_spec(&self) -> Option<ambition_characters::brain::RangedActionSpec> {
        self.ranged.clone()
    }
    fn move_style(&self) -> ambition_characters::brain::MoveStyleSpec {
        self.move_style
    }

    /// Project the per-frame runtime tuning carried on `ActorConfig.tuning`.
    fn tuning(&self) -> crate::features::ecs::actor_tuning::ActorTuning {
        crate::features::ecs::actor_tuning::ActorTuning {
            // Resolved at roster-build time from the archetype hierarchy
            // (BASELINE <- inherits-chain <- this row's `movement` patch).
            movement: self.movement_resolved,
            max_health: self.max_health,
            // The absolute speeds brains consume are DERIVED here: the body owns
            // the number, the author owns the fraction (§4.7, queue C1). Every
            // consumer downstream is unchanged — it still reads px/s — but no
            // authored row states one any more.
            patrol_speed: crate::character_runtime::NormalizedEffort::new(self.patrol_effort)
                .applied_to(self.run_speed),
            chase_speed: crate::character_runtime::NormalizedEffort::new(self.chase_effort)
                .applied_to(self.run_speed),
            max_run_speed: self.run_speed,
            aggro_radius: self.aggro_radius,
            attack_range: self.attack_range,
            contact_strength: self.contact_strength,
            damage_amount: self.damage_amount,
            attack_cooldown_mult: self.attack_cooldown_mult,
            attacks_player: self.attacks_player,
            surface_walker: self.surface_walker,
            turns_at_walls: self.turns_at_walls,
            cling_breaks_on_hit: self.cling_breaks_on_hit,
            // The ONE authored respawn policy (ADR 0022) — the kill hook and
            // the in-place revive tick both match on it.
            respawn: self.respawn,
            weight: self.weight,
            death_policy: self.death_policy,
            // `ActorTuning` carries a decided bool, so silence resolves here.
            is_aerial: self.is_aerial.unwrap_or(false),
            // Archetype flyers use smoothed accel flight; direct-velocity is a boss
            // opt-in (its brain commands exact velocities). See AS4.
            flight_direct_velocity: false,
            is_sandbag: self.is_sandbag,
            body_contact_damage: self.body_contact_damage,
            dream_seed: self.dream_seed,
            ranged_visual: self.ranged_visual.clone(),
        }
    }

    /// Project the authored combat-CONSEQUENCE flags (death behaviors + weapon
    /// drop) into the combat kit. Movement capability is NOT here — it lives on
    /// the body's `AbilitySet` (see [`Self::movement_kit`]).
    fn combat_capabilities(&self) -> crate::combat::CombatCapabilities {
        crate::combat::CombatCapabilities {
            explodes_on_death: self.explodes_on_death,
            divides_on_death: self.divides_on_death,
            charge_crash_explodes: self.charge_crash_explodes,
            never_dies: self.never_dies,
            // An archetype that authors an intrinsic weapon drops one. WHICH
            // one is the body's live `HeldItem` at death, not this row —
            // identical today, and correct after a runtime weapon swap.
            drops_held_item: self.held_item_spec().is_some(),
        }
    }
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
    by_brain: std::collections::BTreeMap<String, ArchetypeSpec>,
    fallback: ArchetypeSpec,
    #[cfg(test)]
    provider_fallbacks: std::collections::BTreeMap<String, ArchetypeSpec>,
}

impl CharacterRoster {
    /// Build a roster from a brain-key → spec table and the fallback spec
    /// (resolved for any unknown brain key, mirroring `from_brain`'s
    /// `Combatant` default).
    pub(crate) fn new(
        by_brain: std::collections::BTreeMap<String, ArchetypeSpec>,
        fallback: ArchetypeSpec,
    ) -> Self {
        Self {
            by_brain,
            fallback,
            #[cfg(test)]
            provider_fallbacks: std::collections::BTreeMap::new(),
        }
    }

    fn with_provider_fallbacks(
        by_brain: std::collections::BTreeMap<String, ArchetypeSpec>,
        fallback: ArchetypeSpec,
        provider_fallbacks: std::collections::BTreeMap<String, ArchetypeSpec>,
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
    pub(crate) fn fallback_for_provider(&self, provider_id: &str) -> Option<&ArchetypeSpec> {
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
    /// Every brain key this roster answers to, sorted.
    ///
    /// The binding sweep needs it because [`Self::spec_for_brain`] cannot fail:
    /// an unknown key silently becomes the `combatant` fallback, so a provider
    /// that misspells its own archetype gets a generic enemy instead of an error.
    /// Resolving against this list is how that stops being invisible.
    ///
    /// Sorted by construction — `by_brain` is a `BTreeMap` precisely so a roster
    /// read never depends on `RandomState` (ADR 0023).
    pub fn brain_keys(&self) -> Vec<String> {
        self.by_brain.keys().cloned().collect()
    }

    /// Whether this roster actually has an archetype for `key`.
    ///
    /// The question [`Self::spec_for_brain`] does not ask before falling back.
    /// Exposed so a caller that can REFUSE — match seating does — asks it first
    /// rather than receiving a generic enemy and reporting success.
    pub fn has_brain_key(&self, key: &str) -> bool {
        self.by_brain.contains_key(key)
    }

    /// **The archetype for `key`, or `None` — no fallback.**
    ///
    /// [`Self::spec_for_brain`] is `pub(crate)` on purpose: it answers every key
    /// by falling back to `combatant`, so a caller outside this crate could not
    /// tell a registered archetype from a misspelled one. That is the right
    /// behaviour at a SPAWN site (a generic enemy beats a crash) and the wrong
    /// answer to every other question.
    ///
    /// This is the accessor for INSPECTING a roster — a provider checking its own
    /// rows assembled the way it wrote them, a tool listing what a composition
    /// carries — and it is the same reasoning that exposed `has_brain_key`: a
    /// caller that can refuse should be able to.
    pub fn archetype_for(&self, key: &str) -> Option<&ArchetypeSpec> {
        self.by_brain.get(key)
    }

    pub(crate) fn spec_for_brain(
        &self,
        brain: &ambition_entity_catalog::placements::CharacterBrain,
    ) -> ArchetypeSpec {
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
        mut by_brain: std::collections::BTreeMap<String, ArchetypeSpec>,
    ) -> Self {
        // Resolve each archetype's movement tuning by folding its patch along the
        // inheritance chain. Done HERE — the single chokepoint every roster passes
        // through — because inheritance needs sibling specs the per-row `tuning()`
        // builder can't see.
        let owners = by_brain
            .keys()
            .map(|brain_id| (brain_id.clone(), "<local>".to_owned()))
            .collect();
        resolve_movement_inheritance(&mut by_brain, &owners)
            .expect("internal character roster movement inheritance must be valid");
        let fallback = by_brain
            .get("combatant")
            .cloned()
            .expect("enemy roster must define a \"combatant\" fallback row");
        Self::new(by_brain, fallback)
    }

    /// Internal parser used by the engine-generic empty default and test
    /// fixtures (crate-visible so sibling modules can build purpose-shaped
    /// rosters). Provider code uses the fallible
    /// [`CharacterRosterFragment::from_ron`].
    pub(crate) fn from_ron(ron: &str) -> Self {
        let by_brain: std::collections::BTreeMap<String, ArchetypeSpec> = ron::from_str(ron)
            .unwrap_or_else(|err| panic!("enemy roster RON failed to deserialize: {err}"));
        Self::from_map(by_brain)
    }
}

/// Fold every archetype's authored movement patch along its inheritance chain and
/// store the resolved [`crate::combat::BodyMovementTuning`] back on each spec.
///
/// Inheritance is deliberately provider-local. An unqualified parent id must be
/// owned by the same provider as the child; unknown parents, cross-provider
/// parents, and cycles are publication errors rather than silent baseline
/// fallbacks. The candidate registry is assembled transactionally, so callers
/// keep the previous prepared roster when this returns an error.
fn resolve_movement_inheritance(
    specs: &mut std::collections::BTreeMap<String, ArchetypeSpec>,
    owners: &std::collections::BTreeMap<String, String>,
) -> Result<(), CharacterRosterAssemblyError> {
    // Snapshot the authored (patch, parent) so resolution reads immutable data
    // while we write resolved values back into the same map.
    let raw: std::collections::BTreeMap<
        String,
        (crate::combat::BodyMovementPatch, Option<String>),
    > = specs
        .iter()
        .map(|(k, s)| (k.clone(), (s.movement, s.inherits.clone())))
        .collect();
    let resolved: std::collections::BTreeMap<String, crate::combat::BodyMovementTuning> = raw
        .keys()
        .map(|k| {
            resolve_movement_for(&raw, owners, k, &mut vec![k.clone()])
                .map(|tuning| (k.clone(), tuning))
        })
        .collect::<Result<_, _>>()?;
    for (k, spec) in specs.iter_mut() {
        if let Some(tuning) = resolved.get(k) {
            spec.movement_resolved = *tuning;
        }
    }
    Ok(())
}

/// Recursively resolve one archetype's movement tuning. `chain` contains the
/// active DFS path so a cycle can report the complete loop.
fn resolve_movement_for(
    raw: &std::collections::BTreeMap<String, (crate::combat::BodyMovementPatch, Option<String>)>,
    owners: &std::collections::BTreeMap<String, String>,
    id: &str,
    chain: &mut Vec<String>,
) -> Result<crate::combat::BodyMovementTuning, CharacterRosterAssemblyError> {
    let (patch, parent) = raw
        .get(id)
        .expect("movement resolver only visits ids from the roster snapshot");
    let provider_id = owners
        .get(id)
        .expect("every assembled brain id has a provider owner");
    let base = match parent {
        None => crate::combat::BodyMovementTuning::BASELINE,
        Some(parent_id) => {
            let Some(parent_provider) = owners.get(parent_id) else {
                // The did-you-mean list is what the author could legally have
                // written instead, so it excludes the child itself: offering
                // `child` as a candidate parent for `child` suggests a cycle as
                // the fix for an unresolved reference.
                let available = owners
                    .iter()
                    .filter_map(|(candidate, owner)| {
                        (owner == provider_id && candidate != id).then_some(candidate.clone())
                    })
                    .collect();
                return Err(CharacterRosterAssemblyError::UnknownMovementParent {
                    provider_id: provider_id.clone(),
                    brain_id: id.to_owned(),
                    parent_id: parent_id.clone(),
                    available,
                });
            };
            if parent_provider != provider_id {
                return Err(
                    CharacterRosterAssemblyError::CrossProviderMovementInheritance {
                        provider_id: provider_id.clone(),
                        brain_id: id.to_owned(),
                        parent_id: parent_id.clone(),
                        parent_provider: parent_provider.clone(),
                    },
                );
            }
            if let Some(cycle_start) = chain.iter().position(|seen| seen == parent_id) {
                let mut cycle = chain[cycle_start..].to_vec();
                cycle.push(parent_id.clone());
                return Err(CharacterRosterAssemblyError::MovementInheritanceCycle {
                    provider_id: provider_id.clone(),
                    chain: cycle,
                });
            }
            chain.push(parent_id.clone());
            let resolved = resolve_movement_for(raw, owners, parent_id, chain)?;
            chain.pop();
            resolved
        }
    };
    Ok(patch.apply_onto(base))
}

/// Engine-generic fallback used by Apps that intentionally register no hostile
/// archetype content. It is inert and exists only so the reusable engine can run
/// menu/demo worlds without installing Ambition's authored enemy table.
const CONTENT_FREE_ROSTER_RON: &str = r#"{
    "combatant": (
        max_health: 1,
        run_speed: 0.0,
        patrol_effort: 0.0,
        chase_effort: 0.0,
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
    by_brain: std::collections::BTreeMap<String, ArchetypeSpec>,
    source_ron: String,
    /// WHERE the RON came from, for diagnostics. See
    /// [`Self::from_ron_at`]; `None` means "built from a literal", not "unknown".
    source: Option<String>,
}

impl CharacterRosterFragment {
    pub fn from_ron(
        provider_id: impl Into<String>,
        fallback_brain_id: Option<impl Into<String>>,
        roster_ron: &str,
    ) -> Result<Self, CharacterRosterAssemblyError> {
        Self::build(None, provider_id, fallback_brain_id, roster_ron)
    }

    /// The same assembly, with the roster ALREADY PARSED.
    ///
    /// ⛔ **this is what lets the content pack be the load path for
    /// `character_archetypes.ron`.** [`Self::from_ron`] re-parses bytes the
    /// compiler has already read and judged — two readers of one file, which is
    /// the split the content pack exists to close.
    ///
    /// ⚠ `source_ron` is kept for the fragment's own reporting, so a caller that
    /// has the text passes it; the PARSE is what must not happen twice.
    pub fn from_prepared_specs(
        provider_id: impl Into<String>,
        fallback_brain_id: Option<impl Into<String>>,
        by_brain: std::collections::BTreeMap<String, ArchetypeSpec>,
        source_ron: impl Into<String>,
    ) -> Result<Self, CharacterRosterAssemblyError> {
        let provider_id = provider_id.into();
        if provider_id.trim().is_empty() {
            return Err(CharacterRosterAssemblyError::EmptyProviderId);
        }
        let fragment = Self {
            provider_id,
            fallback_brain_id: fallback_brain_id.map(Into::into),
            by_brain,
            source_ron: source_ron.into(),
            source: None,
        };
        fragment.validate()?;
        Ok(fragment)
    }

    /// The same, plus WHERE the text came from — the roster twin of
    /// `CharacterCatalogFragment::from_ron_at`. An authoring error in a roster
    /// could name the provider and the brain id but never the file, because the
    /// API took an anonymous `&str` (GPT 5.6, 2026-07-28).
    pub fn from_ron_at(
        source: impl Into<String>,
        provider_id: impl Into<String>,
        fallback_brain_id: Option<impl Into<String>>,
        roster_ron: &str,
    ) -> Result<Self, CharacterRosterAssemblyError> {
        Self::build(
            Some(source.into()),
            provider_id,
            fallback_brain_id,
            roster_ron,
        )
    }

    fn build(
        source: Option<String>,
        provider_id: impl Into<String>,
        fallback_brain_id: Option<impl Into<String>>,
        roster_ron: &str,
    ) -> Result<Self, CharacterRosterAssemblyError> {
        let provider_id = provider_id.into();
        if provider_id.trim().is_empty() {
            return Err(CharacterRosterAssemblyError::EmptyProviderId);
        }
        let by_brain =
            ron::from_str::<std::collections::BTreeMap<String, ArchetypeSpec>>(roster_ron)
                .map_err(|error| CharacterRosterAssemblyError::MalformedFragment {
                    provider_id: provider_id.clone(),
                    source: source.clone(),
                    message: error.to_string(),
                })?;
        let fragment = Self {
            provider_id,
            fallback_brain_id: fallback_brain_id.map(Into::into),
            by_brain,
            source_ron: roster_ron.to_string(),
            source,
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

    /// Where this fragment's RON came from, when the provider said
    /// ([`Self::from_ron_at`]). Read by hosts that report authoring failures
    /// after assembly, when the fragment itself is the only thing left holding
    /// the answer.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
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
        let mut by_brain = std::collections::BTreeMap::new();
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
        resolve_movement_inheritance(&mut by_brain, &owners)?;
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
        source: Option<String>,
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
    UnknownMovementParent {
        provider_id: String,
        brain_id: String,
        parent_id: String,
        available: Vec<String>,
    },
    CrossProviderMovementInheritance {
        provider_id: String,
        brain_id: String,
        parent_id: String,
        parent_provider: String,
    },
    MovementInheritanceCycle {
        provider_id: String,
        chain: Vec<String>,
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
                source,
                message,
            } => write!(
                f,
                "character roster fragment '{provider_id}'{} is malformed RON: {message}",
                source
                    .as_deref()
                    .map(|source| format!(" ({source})"))
                    .unwrap_or_default()
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
            Self::UnknownMovementParent {
                provider_id,
                brain_id,
                parent_id,
                available,
            } => write!(
                f,
                "character roster fragment '{provider_id}' brain '{brain_id}' inherits unknown movement parent '{parent_id}' (available provider-local brains: {})",
                available.join(", ")
            ),
            Self::CrossProviderMovementInheritance {
                provider_id,
                brain_id,
                parent_id,
                parent_provider,
            } => write!(
                f,
                "character roster fragment '{provider_id}' brain '{brain_id}' cannot inherit movement from '{parent_id}' owned by provider '{parent_provider}'; unqualified inheritance is provider-local"
            ),
            Self::MovementInheritanceCycle { provider_id, chain } => write!(
                f,
                "movement inheritance cycle in provider '{provider_id}': {}",
                chain.join(" -> ")
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
pub(crate) fn test_spec(brain_key: &str) -> ArchetypeSpec {
    test_roster().spec_for_brain(
        &ambition_entity_catalog::placements::CharacterBrain::Custom(brain_key.to_string()),
    )
}

#[cfg(test)]
mod app_local_roster_tests {
    use super::*;

    const A: &str = r#"{
        "combatant": (
            max_health: 2, run_speed: 0.0, patrol_effort: 0.0, chase_effort: 0.0,
            aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
            damage_amount: 0, brain_template: StandStill, move_style: Walk,
        ),
    }"#;
    const B: &str = r#"{
        "beta": (
            max_health: 7, run_speed: 0.0, patrol_effort: 0.0, chase_effort: 0.0,
            aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
            damage_amount: 0, brain_template: StandStill, move_style: Walk,
        ),
    }"#;
    const B_WITH_DEFAULT: &str = r#"{
        "beta": (
            max_health: 7, run_speed: 0.0, patrol_effort: 0.0, chase_effort: 0.0,
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
    fn cross_provider_movement_inheritance_is_rejected_transactionally() {
        const CHILD: &str = r#"{
            "child": (
                inherits: Some("combatant"),
                max_health: 7, run_speed: 0.0, patrol_effort: 0.0, chase_effort: 0.0,
                aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
                damage_amount: 0, brain_template: StandStill, move_style: Walk,
            ),
        }"#;
        let mut app = bevy::prelude::App::new();
        app.register_character_roster_fragment(
            CharacterRosterFragment::from_ron("a", Some("combatant"), A).unwrap(),
        );
        let error = app
            .try_register_character_roster_fragment(
                CharacterRosterFragment::from_ron("b", None::<String>, CHILD).unwrap(),
            )
            .err()
            .expect("unqualified inheritance must stay provider-local");
        assert_eq!(
            error,
            CharacterRosterAssemblyError::CrossProviderMovementInheritance {
                provider_id: "b".to_string(),
                brain_id: "child".to_string(),
                parent_id: "combatant".to_string(),
                parent_provider: "a".to_string(),
            }
        );
        assert_eq!(
            app.world()
                .resource::<CharacterRosterRegistry>()
                .providers()
                .collect::<Vec<_>>(),
            vec!["a"],
            "the last known-good registry remains active"
        );
    }

    #[test]
    fn unknown_movement_parent_is_rejected_with_local_candidates() {
        const BROKEN: &str = r#"{
            "combatant": (
                max_health: 2, run_speed: 0.0, patrol_effort: 0.0, chase_effort: 0.0,
                aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
                damage_amount: 0, brain_template: StandStill, move_style: Walk,
            ),
            "child": (
                inherits: Some("missing"),
                max_health: 7, run_speed: 0.0, patrol_effort: 0.0, chase_effort: 0.0,
                aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
                damage_amount: 0, brain_template: StandStill, move_style: Walk,
            ),
        }"#;
        let fragment = CharacterRosterFragment::from_ron("p", Some("combatant"), BROKEN).unwrap();
        let mut registry = CharacterRosterRegistry::default();
        registry.register(fragment).unwrap();
        let error = registry
            .assemble()
            .err()
            .expect("unknown parent must reject the candidate roster");
        assert_eq!(
            error,
            CharacterRosterAssemblyError::UnknownMovementParent {
                provider_id: "p".to_string(),
                brain_id: "child".to_string(),
                parent_id: "missing".to_string(),
                // Not "child": a did-you-mean must not propose self-inheritance.
                available: vec!["combatant".to_string()],
            }
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
