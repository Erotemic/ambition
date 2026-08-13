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

/// Every authored spawn brain key in the lib's fixture roster — the
/// string-keyed replacement for the deleted `CharacterArchetype` iteration
/// constants. `COMBAT_*` excludes the training-dummy + raw-mite rows that
/// don't run the standard combat AI loop (was `COMBAT_ALL`).
#[cfg(test)]
/// ⛔ **THIS LISTED FIVE KEYS AND MEASURED TWO ROWS** (trimmed 2026-08-12).
/// `test_spec` resolves against the SHIPPED file, and `puppy_slug` /
/// `cellular_automaton_fighter` were deleted from it when those creatures became
/// characters — so both fell through to `combatant` and every loop over this list
/// asserted the same row three times while reporting five subjects. Vacuous by
/// duplication, and green throughout.
///
/// ⇒ it names what the shipped file HAS. A test that needs a SHAPE the shipped
/// cast no longer carries names an engine-owned fixture row instead
/// (`fixture_spec`), which is the same rule the mounts and the sandbag follow.
pub(crate) const COMBAT_BRAIN_KEYS: &[&str] = &["combatant", "medium_striker"];

/// Every authored row in the fixture (combat + training dummies).
///
/// ⭐ **`exploding_mite` and `dividing_mite` left this list on 2026-08-11, and
/// leaving is the achievement.** Their rows are DELETED from
/// `character_archetypes.ron`: the two mites author their own health, run speed,
/// gait, contact damage, swipe, death blast and Smash policy on their character
/// DEFINITIONS, and their placements name them, so construction never resolves
/// an archetype for them.
///
/// ⭐ **AND SINCE 2026-08-12 THEIR BRAIN STRING DECIDES NOTHING AT ALL.** This
/// said the eight LDtk placements still carry `brain: "exploding_mite"` because
/// it is *"read for exactly one field, the placement's respawn policy, which has
/// nowhere else to live yet"*. It has somewhere to live — the shark riders proved
/// it the same day, authoring `respawn: OnRest` on their own placements — so all
/// eight mite placements author `OnRoomReenter` themselves now.
///
/// ⚠ **that value is what the deleted rows said AND what `combatant` answers**,
/// which is precisely why it needed authoring: since the rows went, the mites had
/// the right respawn policy BY COINCIDENCE, and would have silently inherited a
/// different one the day somebody retuned the fallback.
///
/// ⭐ **and the SHARK RIDERS left on 2026-08-11 too, with their respawn policy
/// going WITH them.** `pirate_shark_rider` and `pirate_heavy_shark_rider` are
/// `npc_pirate_raider` and `npc_pirate_heavy_iron_mary` now, and their seven
/// placements author `respawn: OnRest` themselves — so the last reason to read
/// their brain string is gone. That field is the one the mites' note above says
/// "has nowhere else to live"; it does now.
#[cfg(test)]
pub(crate) const ALL_BRAIN_KEYS: &[&str] = &["combatant", "medium_striker"];

/// The actor-crate projections of an authored archetype row.
///
/// ⛔ **an extension TRAIT because the orphan rule says so**, not as a style
/// choice: `ArchetypeSpec` is defined in `ambition_combat` now, so an
/// inherent `impl` for it can only be written there — and every one of these
/// returns an ACTOR-crate type (`ActorTuning`, `BrainProfile`). The data
/// moved; the projections into this crate's runtime shapes stayed with the
/// shapes.
pub(crate) trait ArchetypeSpecExt {
    fn brain_profile(&self) -> crate::features::ecs::actor_tuning::BrainProfile;
    fn movement_kit(&self) -> ae::AbilitySet;
    fn held_item_spec(&self) -> Option<ambition_characters::brain::HeldItemSpec>;
    fn melee_spec(&self) -> Option<ambition_characters::brain::MeleeActionSpec>;
    fn ranged_spec(&self) -> Option<ambition_characters::brain::RangedActionSpec>;
    fn move_style(&self) -> ambition_characters::brain::MoveStyleSpec;
    fn tuning(&self) -> crate::features::ecs::actor_tuning::ActorTuning;
    fn combat_capabilities(&self) -> crate::combat::CombatCapabilities;
}

impl ArchetypeSpecExt for ArchetypeSpec {
    /// Project this archetype's CONTROLLER half — the reusable autonomous
    /// policy, separated from the body the same row also describes.
    ///
    /// ⚠ **a projection is the migration, not the destination.** A profile
    /// reachable only by holding an archetype is still one authority; the
    /// endpoint is an authored profile a character NAMES, at which point these
    /// rows lose their controller fields and this method loses its subject.
    fn brain_profile(&self) -> crate::features::ecs::actor_tuning::BrainProfile {
        crate::features::ecs::actor_tuning::BrainProfile {
            template: self.brain_template,
            // ⭐ **the three CharacterAI knobs that used to live in
            // `ActorTuning`.** They read as body numbers and are not: a radius
            // at which a DRIVER notices, a range at which it commits, and
            // whether a walker reverses at a wall are all decisions about how
            // to play a body, and a human or scripted controller in the same
            // body must not inherit them.
            aggro_radius: self.aggro_radius,
            attack_range: self.attack_range,
            turns_at_walls: self.turns_at_walls,
            // ⭐ **and the FOURTH knob** (2026-08-13): how often the driver
            // commits again. Same argument as the three above — it was the last
            // controller fact left in `ActorTuning`.
            attack_cooldown_mult: self.attack_cooldown_mult,
            // The pacing the row has always authored as fractions of its own
            // `run_speed`, now carried by the authority that decides pace.
            patrol_effort: self.patrol_effort,
            chase_effort: self.chase_effort,
            // ⛔ **`hostile_by_default` does NOT come across, and that is the point.**
            // The row holds it because the archetype ontology fused body, driver
            // and social role; a controller policy answers how to play a body,
            // never who its enemies are. The archetype path keeps reading its own
            // field into `ActorTuning` below — this conversion is what a MIGRATED
            // character gets, and a migrated character's hostility comes from its
            // placement's `SpawnDisposition` (Jon's redirect §6).
            // The archetype's authored rung, or the middle one. A fighter
            // archetype that says nothing plays at 5 rather than refusing.
            fighter_level: self.fighter_level.unwrap_or(5),
            smash_hit_band: self.smash_hit_band.unwrap_or(
                crate::features::ecs::actor_tuning::BrainProfile::DEFAULT_SMASH_HIT_BAND,
            ),
            smash_heavy: self.smash_heavy,
            smash_dash_to_close: self.smash_dash_to_close,
            smash_duelist: self.smash_duelist,
            // ⛔ **`can_blink` / `can_fly` / `can_shield` do NOT come across.**
            // They were mirrored onto the profile as `smash_can_*` and deleted
            // 2026-08-11 (Jon's redirect §7): a capability copied onto a
            // controller policy makes the policy unreusable, because the copy
            // describes ONE body. The row still feeds the real port — the same
            // authored verbs become the body's movement `AbilitySet` through
            // `movement_kit` — and `smash_cfg_from_spec` now reads THAT.
            provoke_forced_brute_min_aggro: self.provoke_forced_brute_min_aggro,
        }
    }

    /// The character's authored **movement kit** as an [`ae::AbilitySet`] — the
    /// verbs this body HAS, in the one movement-capability vocabulary every body
    /// (player, enemy, boss) shares. This is the single authored source both
    /// ports read: the body unions it into its live `AbilitySet` at spawn
    /// (`ActorBody::from_kit`), and the Smash brain reads the same verbs to
    /// decide when to attempt them (`brain_profile`). Only the kit verbs are set;
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
            contact_strength: self.contact_strength,
            damage_amount: self.damage_amount,
            is_hostile: self.hostile_by_default,
            surface_walker: self.surface_walker,
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
            // ⛔ **an archetype row cannot say WHAT it splits into** (AC5.4).
            // The fact is now `divides_into: Option<CharacterId>` on the
            // character, because the offspring's identity is content and an
            // archetype row has no field for it — the engine used to hold the
            // name instead. No shipped row ever set `divides_on_death`, so this
            // is `None` rather than a lie, and the row's own dead field goes
            // with the ontology in AC6.
            divides_into: None,
            charge_crash_explodes: self.charge_crash_explodes,
            never_dies: self.never_dies,
            // An archetype that authors an intrinsic weapon drops one. WHICH
            // one is the body's live `HeldItem` at death, not this row —
            // identical today, and correct after a runtime weapon swap.
            drops_held_item: self.held_item_spec().is_some(),
        }
    }
}

/// **The one row a caller with an OPEN CASTING DECISION settles for.**
///
/// ⚠ it is an ordinary row now, not a reserved error value. Nothing resolves to
/// it by accident: the only road that reaches it is
/// [`CharacterRoster::generic_body_for_unresolved_brain`], and that road refuses
/// every identifier not on the list below.
pub(crate) const GENERIC_BODY_ROW: &str = "combatant";

/// **ONE IDENTIFIER WHOSE CASTING IS STILL OPEN**, declared by the provider that
/// owns the decision.
///
/// ⛔⛔ **the engine held this list until 2026-08-12, and a list is still a
/// global fallback — just a shorter one.** It read
/// `IDENTIFIERS_AWAITING_A_CASTING_DECISION = ["small_lurker", "large_brute",
/// "SmallSkitter"]`, three Ambition creature names compiled into a reusable
/// engine, and it moved the policy from "every unknown key silently gets
/// `combatant`" to "these three names silently get `combatant`" — better, and
/// the same shape (GPT 5.6 review, priority 3). A second game linking this
/// engine inherited Ambition's waiver for free, and a typo that happened to
/// match one of the three entered the temporary road as quietly as before.
///
/// ⭐ **so the waiver is CALLER INTENT now, and it travels on the fragment.** A
/// provider that has an unresolved content decision says so, names the row it
/// settles for while the decision stands, and carries the ledger reference that
/// will retire it. The engine knows none of the three names. Another provider
/// using the same string gets a construction error, because its own fragment
/// declared nothing.
///
/// ⚠ **it is not a shortcut for "make this spawn work".** Every field is
/// required, the reason is logged at every construction, and the fragment that
/// declares one is the file a reader opens to see what is still undecided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenCastingDecision {
    /// The authored identifier that resolves nothing yet.
    pub identifier: String,
    /// The brain-key row a body is built from while the decision stands. The
    /// PROVIDER's choice — the engine has no opinion about which of a game's
    /// rows is the neutral one.
    pub temporary_row: String,
    /// Why it is open and what will close it. Logged on every construction, so
    /// this is read by whoever is looking at the warning, not only by whoever
    /// opens this file.
    pub reason: String,
}

/// App-local hostile-archetype authority: a brain-key → spec table. This is the
/// spawn path's only resolution surface and it is **roster-enum-free** — a
/// pure string lookup, so the named `CharacterArchetype` enum / RON / brain-name
/// table can be owned and installed by the content layer.
///
/// ⛔ **it holds no fallback of any kind**, neither process-wide (D102) nor
/// per-provider (deleted 2026-08-12 — see `CharacterRosterRegistry::assemble`).
/// An identifier either names a row here, names a prepared character, or is a
/// construction error.
///
/// Providers assemble this resource transactionally inside each Bevy App.
/// Runtime systems receive `Res<CharacterRoster>` and pure construction helpers
/// receive `&CharacterRoster`; no process-global fallback participates in
/// production resolution.
#[derive(bevy::prelude::Resource, Clone, Debug)]
pub struct CharacterRoster {
    by_brain: std::collections::BTreeMap<String, ArchetypeSpec>,
    /// Identifier → (declaring provider, decision). Assembled from the
    /// fragments; see [`OpenCastingDecision`].
    open_casting: std::collections::BTreeMap<String, (String, OpenCastingDecision)>,
}

impl CharacterRoster {
    /// Build a roster from a brain-key → spec table.
    pub(crate) fn new(by_brain: std::collections::BTreeMap<String, ArchetypeSpec>) -> Self {
        Self {
            by_brain,
            open_casting: std::collections::BTreeMap::new(),
        }
    }

    fn with_open_casting(
        by_brain: std::collections::BTreeMap<String, ArchetypeSpec>,
        open_casting: std::collections::BTreeMap<String, (String, OpenCastingDecision)>,
    ) -> Self {
        Self {
            by_brain,
            open_casting,
        }
    }

    // ⛔⛔ **`sandbags_are_passive()` WAS HERE AND IS DELETED (2026-08-13,
    // campaign P2.19), because its SUBJECT left and it went quietly vacuous.**
    //
    // It read `all(|spec| !spec.is_sandbag || spec.melee.is_none())` and had one
    // caller: an `ambition_content` test over the SHIPPED roster. That roster is
    // down to `combatant` and `medium_striker`, neither of which sets
    // `is_sandbag`, so the `all` ran over zero matching rows and returned `true`
    // by having nothing to check — the D94 shape, a green guard whose subject
    // migrated out from under it.
    //
    // ⚠ **and the claim could not be migrated literally, which is the more
    // useful half.** Both sandbags are CHARACTERS now and both carry a real
    // `PunchWeak` through the `sandbag_punch` action set, on purpose: an
    // archetype row fused kit and policy, so the only way to say "never strikes
    // back" was to remove the fist. A character says it with its POLICY, and
    // `practice_target_characters_do_not_strike_back` asserts that instead —
    // aggro radius and attack range both zero. Asserting the old proxy would
    // have pushed content to strip a fist for a reason that was never the real
    // one.

    #[cfg(test)]
    pub(crate) fn contains_brain(&self, brain_id: &str) -> bool {
        self.by_brain.contains_key(brain_id)
    }

    /// Every brain key this roster answers to, sorted.
    ///
    /// ⚠ **the binding sweep needed it because the lookup used to be unable to
    /// fail** — an unknown key silently became the `combatant` fallback, so a
    /// provider that misspelled its own archetype got a generic enemy instead of
    /// an error, and resolving against this list was the only way to see it.
    /// Construction refuses an undeclared identifier itself now (D102), so this
    /// is a provider INSPECTING what it assembled rather than a sweep working
    /// around a lookup that lies.
    ///
    /// Sorted by construction — `by_brain` is a `BTreeMap` precisely so a roster
    /// read never depends on `RandomState` (ADR 0023).
    pub fn brain_keys(&self) -> Vec<String> {
        self.by_brain.keys().cloned().collect()
    }

    // ⛔⛔ **`brain_profile_for()` WAS HERE AND IS DELETED (2026-08-13, campaign
    // P2.18/P2.19) — the roster's LAST controller-policy surface.**
    //
    // Its own doc called it *"a projection, and therefore temporary… the endpoint
    // is a registry of authored `BrainProfile`s a character or a placement NAMES,
    // at which point this method's subject stops existing"*. That endpoint
    // arrived: Smash publishes `smash::duelist_l{n}` (D87), the versus stage
    // publishes `ambition_versus::versus_duelist`, and `seat_brain_profile` has
    // one arm. The compiler-backed census (`probe_dead_public_fns.py`, D105)
    // found it with zero call sites in the workspace or any excluded consumer.
    //
    // ⇒ `CharacterRoster` answers BODY questions now, and nothing else. Two of
    // the three fused authorities Jon's brief names are out of it — controller
    // policy here, placement policy as its fields migrate — and what is left is
    // the intrinsic body, which goes with the last two rows.

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

    /// The row for this brain key, or `None`.
    ///
    /// ⭐ **the honest question**, and the one production code asks. The
    /// variant that answered the same thing and then hid a miss behind the
    /// `combatant` row — which is how a typo'd key became a generic enemy
    /// instead of a build failure — is `#[cfg(test)]` now and named for what it
    /// does ([`Self::generic_body_for_a_test_fixture`]).
    pub(crate) fn try_spec_for_brain(
        &self,
        brain: &ambition_entity_catalog::placements::CharacterBrain,
    ) -> Option<ArchetypeSpec> {
        let ambition_entity_catalog::placements::CharacterBrain::Custom(name) = brain else {
            return None;
        };
        self.by_brain.get(name.as_str()).cloned()
    }

    /// **THE OLD SILENT FALLBACK, ALIVE ONLY FOR TESTS.**
    ///
    /// ⭐ this is `spec_for_brain` as it was — no waiver, no explanation — and
    /// `#[cfg(test)]` is the whole point of keeping it. Unit tests name shapes
    /// ("a rideable body", "a charge-crasher") against the engine's own fixture
    /// roster and do not care which row answers; production callers must say why
    /// they are settling. Hoisting the lookup out of `ActorClusterSeed::new_in`
    /// is what made that distinction expressible (D102).
    #[cfg(test)]
    pub(crate) fn spec_for_brain(
        &self,
        brain: &ambition_entity_catalog::placements::CharacterBrain,
    ) -> ArchetypeSpec {
        self.generic_body_for_a_test_fixture(brain)
    }

    /// **THE GENERIC BODY A COMPOSITION WITH NO CAST AT ALL SETTLES FOR.**
    ///
    /// ⚠ **this is a SECOND reason, and it must not be folded into the first.**
    /// `report_unprepared_character` already argues it for the character road
    /// and the argument is the same here: several hosts — the multi-game shell,
    /// the rollback door fixture, every unit test that composes one system —
    /// reach construction with a prepared registry holding ZERO characters. In
    /// those, EVERY identifier is "unknown", and refusing would blame the
    /// content for a COMPOSITION gap. A host that published a cast and still
    /// cannot name this creature is the defect;  a host that published none is
    /// simply not that host.
    ///
    /// ⇒ so the two waivers stay separately named: this one is about the HOST,
    /// [`Self::generic_body_for_unresolved_brain`] is about the IDENTIFIER, and
    /// only the second one shrinks as content decisions land.
    pub(crate) fn generic_body_for_a_composition_with_no_cast(&self) -> Option<ArchetypeSpec> {
        self.by_brain.get(GENERIC_BODY_ROW).cloned()
    }

    /// **THE GENERIC BODY A FIXTURE SETTLES FOR — TEST-ONLY, AND NAMED SO.**
    ///
    /// ⛔ production has no equivalent, deliberately: an identifier that names
    /// nothing is a construction ERROR out there (D102), and the whole reason
    /// this is a separate `#[cfg(test)]` method rather than a permissive branch
    /// of [`Self::generic_body_for_unresolved_brain`] is that the two callers
    /// want opposite things. A unit test naming "a rideable body" against the
    /// engine's fixture roster is not asserting on which row answered; a spawn
    /// road that cannot resolve what a level asked for is a defect.
    ///
    /// ⚠ **it is still the thing that hid five mount tests' subject** the day
    /// the shark became a character, so a test that CARES which body it got must
    /// assert on it rather than trusting this.
    #[cfg(test)]
    pub(crate) fn generic_body_for_a_test_fixture(
        &self,
        brain: &ambition_entity_catalog::placements::CharacterBrain,
    ) -> ArchetypeSpec {
        self.try_spec_for_brain(brain)
            .or_else(|| self.by_brain.get(GENERIC_BODY_ROW).cloned())
            .or_else(|| self.by_brain.values().next().cloned())
            .expect("a fixture roster with no rows at all cannot build any body")
    }

    /// **THE GENERIC BODY A CALLER SETTLES FOR, ASKED FOR BY NAME.**
    ///
    /// ⛔⛔ **this was `spec_for_brain`, and it lived INSIDE
    /// `ActorClusterSeed::new_in`** — so every construction that named an
    /// unresolvable brain key became a generic enemy without anybody choosing
    /// that, wearing whatever sprite the placement named, and every assertion
    /// about it passed. That happened three times in this campaign alone as rows
    /// migrated out: five engine mount tests became tests about `combatant` the
    /// day the shark became a character, and the respawn-policy tests went
    /// VACUOUS because `combatant` happens to author `OnRoomReenter` too.
    ///
    /// ⭐ **the lookup is hoisted to the call sites now** (ledger D102). A
    /// constructor cannot reach it, so a NEW spawn road cannot inherit the
    /// downgrade by accident: it either resolves a row through
    /// [`Self::try_spec_for_brain`] or it comes here and says why.
    ///
    /// ⚠ **TWO reasons meet here and neither is decoration.** `waiver` is the
    /// CALL SITE's — why this road settles for a body instead of refusing, which
    /// only the road knows. The other belongs to the PROVIDER that declared the
    /// identifier's casting still open ([`OpenCastingDecision`]), and it names
    /// the ledger row that will retire it. Both go into the warning, so a reader
    /// of the log gets the question rather than only the symptom.
    ///
    /// ⛔ **an identifier no provider declared resolves NOTHING**, which is the
    /// whole of D102's destination and the rule P0.1 established for an absent
    /// `CharacterId`. The engine holds no list of names; what stands between here
    /// and an empty `open_casting` map is content decisions, not engineering.
    pub(crate) fn generic_body_for_unresolved_brain(
        &self,
        brain: &ambition_entity_catalog::placements::CharacterBrain,
        waiver: &str,
    ) -> Option<ArchetypeSpec> {
        if let Some(spec) = self.try_spec_for_brain(brain) {
            return Some(spec);
        }
        let ambition_entity_catalog::placements::CharacterBrain::Custom(name) = brain else {
            // ⭐ **A BRAIN THAT NAMES NOTHING IS NOT A BRAIN THAT NAMED SOMETHING
            // WRONG.** `Passive` and its siblings state a POLICY, not an
            // identifier — a shipped `EnemySpawn` in the intro world does exactly
            // this — so there is no misspelling to catch and nothing to wait on:
            // the placement asked for a plain body and a plain body is a correct
            // answer. Refusing here was measured wrong the first time this rule
            // ran, and this is the distinction D102 is actually about.
            return self.by_brain.get(GENERIC_BODY_ROW).cloned();
        };
        let Some((provider, decision)) = self.open_casting.get(name.as_str()) else {
            return None;
        };
        let Some(spec) = self.by_brain.get(&decision.temporary_row).cloned() else {
            // A provider that waived an identifier onto a row it does not own is
            // a broken declaration, not a licence — refuse, and let the caller's
            // panic name both.
            return None;
        };
        bevy::prelude::warn!(
            target: "ambition_platformer2d_actor_monolith::enemies",
            "no archetype row and no prepared character for `{name}` — provider \
             `{provider}` declares its casting still OPEN, so this body is built \
             from that provider's `{row}` row while the decision stands. The \
             provider's reason: {reason}. The caller settled for it because: \
             {waiver}",
            name = name,
            provider = provider,
            row = decision.temporary_row,
            reason = decision.reason,
            waiver = waiver,
        );
        Some(spec)
    }

    /// Build a roster from a brain-keyed spec map. No key is reserved and no row
    /// is a fallback — an identifier that names none of these is a construction
    /// error unless its provider declared the casting open. This is the
    /// roster-enum-free construction path:
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
        Self::new(by_brain)
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
        hostile_by_default: false,
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
    by_brain: std::collections::BTreeMap<String, ArchetypeSpec>,
    /// Identifiers this provider has authored somewhere but not yet cast. See
    /// [`OpenCastingDecision`] and [`Self::with_open_casting_decision`].
    open_casting: Vec<OpenCastingDecision>,
    source_ron: String,
    // ⛔ **`source: Option<String>` WAS HERE**, "where the RON came from, for
    // diagnostics", citing a `from_ron_at` constructor that had no callers. So
    // it was `None` on every fragment ever built, and the compiler confirmed
    // nothing read it once the accessor went. The assembly error still carries a
    // `source` — it reads the BUILD parameter, not this field, which is why the
    // two looked like one feature.
}

impl CharacterRosterFragment {
    pub fn from_ron(
        provider_id: impl Into<String>,
        roster_ron: &str,
    ) -> Result<Self, CharacterRosterAssemblyError> {
        Self::build(None, provider_id, roster_ron)
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
        by_brain: std::collections::BTreeMap<String, ArchetypeSpec>,
        source_ron: impl Into<String>,
    ) -> Result<Self, CharacterRosterAssemblyError> {
        let provider_id = provider_id.into();
        if provider_id.trim().is_empty() {
            return Err(CharacterRosterAssemblyError::EmptyProviderId);
        }
        let fragment = Self {
            provider_id,
            by_brain,
            open_casting: Vec::new(),
            source_ron: source_ron.into(),
        };
        fragment.validate()?;
        Ok(fragment)
    }

    fn build(
        source: Option<String>,
        provider_id: impl Into<String>,
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
            by_brain,
            open_casting: Vec::new(),
            source_ron: roster_ron.to_string(),
        };
        fragment.validate()?;
        Ok(fragment)
    }

    // ⛔ **`fallback_brain_id` WAS HERE — the accessor first, then the whole
    // concept** (2026-08-12). The accessor had zero callers; the FIELD looked
    // load-bearing because validation, fragment dedup and assembly all read it.
    // They read it to produce a map production threw away. See `assemble`.

    // ⛔⛔ **THREE PUBLIC ACCESSORS WERE DELETED HERE (2026-08-12) "compiler-verified
    // to have no call site in the WORKSPACE" — and ONE OF THEM WAS NOT DEAD.**
    // `provider_id` and `source` were. `from_ron_at` is RESTORED below, because
    // the census's own words name its blind spot: *in the WORKSPACE*.
    // `fixtures/external_consumer` is `exclude`d in the root `Cargo.toml`, so a
    // `cargo check --workspace` cannot see it — and it is the only in-repo
    // consumer that links this engine from OUTSIDE, which is exactly the
    // population a public-API census is about. It called `from_ron_at` and had a
    // test asserting the located diagnostic (ledger D110).
    //
    // ⚠ **the technique was right and got refined twice, which is why this is
    // worth reading rather than deleting.** Marking every public fn
    // `#[deprecated]` and reading `cargo check` is ledger D105's answer to five
    // wrong grep censuses. Its first run used `-p`, which compiles this package
    // and its DEPENDENCIES and not its dependents, so every cross-crate caller
    // was invisible and it named five dead functions; `--workspace` named three.
    // This is the third refinement: `--workspace` still cannot see a consumer
    // the workspace excludes.
    //
    // ⚠ the `source` FIELD really was a provenance feature with no consumer and
    // stays deleted — the assembly ERROR carries its own `source`, read from the
    // BUILD parameter, and that is what makes a located diagnostic work. The two
    // looked like one feature, which is how a working one got taken with a dead
    // one.

    /// **Declare that one authored identifier's casting is still open.**
    ///
    /// The provider that owns the unresolved decision says so here, names the
    /// row of ITS OWN roster a body is built from meanwhile, and gives the
    /// reason a reader of the warning needs. Construction refuses every
    /// identifier that has not been through this call, so a typo cannot enter
    /// the temporary road and a second game linking this engine does not inherit
    /// another game's waiver.
    ///
    /// ⛔ **this is not the fallback coming back.** A fallback answers for
    /// everything the roster does not know; this answers for exactly the strings
    /// named here, loudly, with a ledger reference attached — and retiring one is
    /// deleting one call.
    ///
    /// ⚠ the row is validated at ASSEMBLY, not here, because a provider may
    /// declare the waiver beside a row another of its own fragments contributes.
    pub fn with_open_casting_decision(
        mut self,
        identifier: impl Into<String>,
        temporary_row: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        self.open_casting.push(OpenCastingDecision {
            identifier: identifier.into(),
            temporary_row: temporary_row.into(),
            reason: reason.into(),
        });
        self
    }

    /// **The same assembly, told WHERE its text came from.**
    ///
    /// A fragment built this way reports `source` in every diagnostic it raises,
    /// so an author who mistypes a roster reads which FILE to open instead of
    /// only which provider. That is the whole feature, and it is served by the
    /// build parameter rather than by any field on the fragment.
    pub fn from_ron_at(
        source: impl Into<String>,
        provider_id: impl Into<String>,
        roster_ron: &str,
    ) -> Result<Self, CharacterRosterAssemblyError> {
        Self::build(Some(source.into()), provider_id, roster_ron)
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
            if existing.source_ron == fragment.source_ron
                && existing.open_casting == fragment.open_casting
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
        }
        resolve_movement_inheritance(&mut by_brain, &owners)?;
        // The providers' OWN unresolved content decisions, keyed by identifier so
        // construction can ask one question. A declaration naming a row nobody
        // assembled is refused here rather than at the spawn that trips over it.
        let mut open_casting =
            std::collections::BTreeMap::<String, (String, OpenCastingDecision)>::new();
        for (provider_id, fragment) in &self.fragments {
            for decision in &fragment.open_casting {
                if !by_brain.contains_key(&decision.temporary_row) {
                    return Err(CharacterRosterAssemblyError::MissingTemporaryRow {
                        provider_id: provider_id.clone(),
                        identifier: decision.identifier.clone(),
                        brain_id: decision.temporary_row.clone(),
                    });
                }
                if let Some((first_provider, _)) = open_casting.get(&decision.identifier) {
                    return Err(CharacterRosterAssemblyError::DuplicateOpenCasting {
                        identifier: decision.identifier.clone(),
                        first_provider: first_provider.clone(),
                        second_provider: provider_id.clone(),
                    });
                }
                open_casting.insert(
                    decision.identifier.clone(),
                    (provider_id.clone(), decision.clone()),
                );
            }
        }
        // ⛔⛔ **A PER-PROVIDER FALLBACK USED TO BE ASSEMBLED HERE, AND IT WAS
        // CEREMONIAL** (deleted 2026-08-12, GPT 5.6 review priority 4).
        //
        // The process-wide default went first (D102): a single-provider host
        // promoted that provider's fallback row to the answer for every unknown
        // brain key in the App, while a multi-provider host quietly used the
        // inert engine one, so the same misspelling built two different bodies
        // depending on how many games were linked. What survived that deletion
        // was a per-provider map — and it was `#[cfg(test)]`. Production still
        // ACCEPTED a `fallback_brain_id`, VALIDATED it, could REJECT a provider
        // whose fallback named no row, and assembled the specs — then dropped
        // them on the floor, because the only reader was one test.
        //
        // ⚠ **and the one production caller that passed a fallback was passing
        // the rule that had already been deleted**: `ambition_content` said
        // `Some("combatant")`, whose entire remaining effect was to require a
        // `combatant` row — the same check `ambition_combat::content_schema`
        // dropped when unknown keys stopped downgrading there. A concept kept
        // alive by a test that asserts two of them coexist is a concept whose
        // subject no longer exists; the invariant worth keeping is the ABSENCE
        // of any default, and `provider_defaults_coexist...` still pins that.
        Ok(CharacterRoster::with_open_casting(by_brain, open_casting))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CharacterRosterAssemblyError {
    EmptyProviderId,
    /// A provider waived an identifier onto a row no fragment contributes.
    MissingTemporaryRow {
        provider_id: String,
        identifier: String,
        brain_id: String,
    },
    /// Two providers both claim the same identifier's casting is theirs to
    /// settle — which of their temporary bodies wins is not the engine's call.
    DuplicateOpenCasting {
        identifier: String,
        first_provider: String,
        second_provider: String,
    },
    EmptyBrainId {
        provider_id: String,
        brain_id: String,
    },
    DuplicateProvider {
        provider_id: String,
    },
    MalformedFragment {
        provider_id: String,
        source: Option<String>,
        message: String,
    },
    DuplicateBrain {
        brain_id: String,
        first_provider: String,
        second_provider: String,
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
            Self::DuplicateProvider { provider_id } => {
                write!(f, "character roster provider '{provider_id}' registered twice")
            }
            Self::MissingTemporaryRow {
                provider_id,
                identifier,
                brain_id,
            } => write!(
                f,
                "provider '{provider_id}' declares '{identifier}' still uncast and \
                 settles for row '{brain_id}', which no fragment contributes. A \
                 waiver onto a row nobody assembled would fail at the spawn that \
                 trips over it instead of here"
            ),
            Self::DuplicateOpenCasting {
                identifier,
                first_provider,
                second_provider,
            } => write!(
                f,
                "providers '{first_provider}' and '{second_provider}' both declare \
                 '{identifier}' still uncast. Whose temporary body a construction \
                 gets is not the engine's call to make"
            ),
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
            Self::DuplicateBrain {
                brain_id,
                first_provider,
                second_provider,
            } => write!(
                f,
                "character brain id '{brain_id}' is authored by both '{first_provider}' and '{second_provider}'"
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
    // ⚠ **THE ENGINE'S FIXTURE IS AMBITION'S SHIPPED CONTENT, and that coupling
    // bites as the character migration deletes rows.** A fixture naming a
    // migrated key does not fail — `spec_for_brain` answers `combatant` — so an
    // engine test about mounts quietly became a test about a generic enemy the
    // day the shark stopped being an archetype.
    //
    // ⇒ engine tests that need a SHAPE (a rideable body, a crawler, a flyer)
    // should name a row this crate owns. [`fixture_roster_with_mount`] is the
    // first; the rest follow as their subjects migrate.
    CharacterRoster::from_ron(include_str!(
        "../../../../../game/ambition_content/assets/data/character_archetypes.ron"
    ))
}

/// **A rideable fixture the ENGINE owns**, for tests about the mount machinery
/// rather than about any game's shark.
///
/// ⭐ added 2026-08-11, when `burning_flying_shark` became a character and five
/// engine tests about mounting silently started asserting things about the
/// `combatant` fallback. The shape is what those tests need: something with a
/// `mount_class`, something that can pilot it, and a mass difference so the
/// pair's centre of gravity is not ambiguous.
#[cfg(test)]
pub(crate) fn fixture_roster_with_mount() -> CharacterRoster {
    // The shipped fixture PLUS the engine's own rows: a mount test still needs
    // `pirate_raider`, `giant_gnu` and the rest, so this extends rather than
    // replaces. The extra rows are appended by splicing before the closing
    // brace — the file is a RON map and this is a test helper, not a parser.
    let shipped =
        include_str!("../../../../../game/ambition_content/assets/data/character_archetypes.ron");
    let close = shipped
        .rfind('}')
        .expect("the archetype fixture is a RON map");
    let extra = r#"
    // ⭐ **THE TWO PIRATES THE MOUNT AND HEAVY TESTS PLAN AGAINST, owned by the
    // FIXTURE.** Their shipped rows were deleted on 2026-08-11 (ledger D84): all
    // nine pirate characters state their own provoked policy now, and no
    // pirate-named placement in any world lacks a `character_id`, so nothing in
    // the game reaches them. What still needs them is a handful of tests about
    // riders and heavies — exactly the case the giant's note below describes,
    // and the same answer: a test's cast belongs to the test.
    // ⭐⭐ **THE DASHING, BLINKING, FLYING DUELIST — owned by the FIXTURE**
    // (2026-08-12). Its shipped row was deleted on 08-11 when the PCA became
    // `perfect_cellular_automaton` and authored its own body, and TWENTY engine
    // tests still name this key: the dash tests, the respawn-policy tests, the
    // brain-effect tests, the fighter harness. Every one of them has been
    // resolving the `combatant` fallback ever since and asserting about that.
    //
    // ⛔ **the respawn-policy tests were the worst of it — they went VACUOUS.**
    // `combatant` also authors `OnRoomReenter`, so "this archetype respawns on
    // room re-entry" kept passing while measuring a different archetype entirely.
    // A test that cannot tell its subject from the fallback is not a weaker test,
    // it is a different one.
    //
    // ⚠ the SHAPE is what those tests need — a Smash-brained duelist that can
    // dash, blink and fly, with a health pool nothing else has — so that is what
    // this row is. The signature move, the glider projectile and the tuned
    // efforts stayed with the CHARACTER, where they belong; copying them here
    // would be keeping the archetype alive under a test's name.
    "cellular_automaton_fighter": (
        respawn: OnRoomReenter,
        max_health: 60,
        run_speed: 168.0,
        patrol_effort: 0.5714,
        chase_effort: 1.0,
        aggro_radius: 540.0,
        attack_range: 150.0,
        contact_strength: 0.75,
        damage_amount: 1,
        is_aerial: Some(false),
        brain_template: Smash,
        melee: Some(Swipe((
            windup_s: 0.24,
            active_s: 0.08,
            recover_s: 0.30,
            damage: 1,
            reach_px: 30.0,
        ))),
        smash_dash_to_close: true,
        smash_duelist: true,
        can_blink: true,
        can_fly: true,
        move_style: Walk,
    ),
    // ⭐⭐ **THE MID-RANGE STRIKER — owned by the FIXTURE** (2026-08-12), and it
    // is the LAST shipped row but one. `goblin` is a character that authors its
    // own eleven-move repertoire; nothing in any world names `medium_striker` as
    // a spawn brain key, and `cargo check --all-targets` is clean without it.
    // What still needs the ROW is seven tests about the archetype MACHINERY: the
    // brain template comes from the row, a ranged Rock reaches the action set,
    // the smash hit band is data-authored, the derived-behaviour formulas match.
    //
    // ⚠ **all seven are about the SHAPE**, which is why the row moves here
    // rather than being deleted with its readers: something Smash-brained with a
    // melee AND a ranged verb AND an authored hit band is what they need, and
    // Ambition's goblin having been that once is incidental to every one of
    // them. The goblin's own numbers live on its character.
    "medium_striker": (
        respawn: OnRoomReenter,
        max_health: 5,
        run_speed: 170.0,
        patrol_effort: 0.6176,
        chase_effort: 1.0,
        aggro_radius: 460.0,
        attack_range: 150.0,
        contact_strength: 0.70,
        damage_amount: 1,
        brain_template: Smash,
        melee: Some(Swipe((
            windup_s: 0.28,
            active_s: 0.08,
            recover_s: 0.32,
            damage: 1,
            reach_px: 28.0,
        ))),
        // Goblins poke with a thrown rock at mid-range, then close for the
        // swing — the Smash brain's verb-selection-by-range (see
        // `brain::smash::maybe_substitute_ranged`). Modest speed so it is
        // readable/dodgeable; damage matches the swing. The 1.1s ranged
        // cadence keeps it from plinking. NOTE: no dedicated throw animation
        // yet — the rock just spawns from the goblin; sprite/feel is a handoff.
        ranged: Some((style: Rock, speed: 360.0, damage: 1)),
        smash_hit_band: Some(32.0),
        // Goblins dash to close a large gap (richer action set).
        smash_dash_to_close: true,
        move_style: Walk,
    ),

    // ⭐ **THE IMMORTAL PRACTICE DUMMY, owned by the FIXTURE** (2026-08-12). Its
    // shipped row was deleted with the `sandbag_infinite` migration: the combat
    // lab's two dummies name the `sandbag_infinite` CHARACTER now, which authors
    // `never_dies`, 9999 health, the StandStill policy and no contact damage.
    //
    // What still needs the ROW is a handful of tests about the archetype
    // machinery itself — "a never-dies row needs no revive timer", "contact
    // damage is opt-in", "the brain template comes from the row" — and those are
    // about the SHAPE, not about Ambition's dummy. Same answer as the pirates
    // and the shark above: a test's cast belongs to the test. The key is
    // unchanged so the tests read the same.
    "sandbag_infinite": (
        max_health: 9999,
        never_dies: true,
        body_contact_damage: false,
        run_speed: 155.0,
        patrol_effort: 0.6774,
        chase_effort: 1.0,
        aggro_radius: 0.0,
        attack_range: 150.0,
        contact_strength: 0.70,
        damage_amount: 1,
        is_sandbag: true,
        brain_template: StandStill,
        melee: None,
        move_style: Walk,
    ),
    "pirate_raider": (
        respawn: OnRoomReenter,
        max_health: 5,
        run_speed: 190.0,
        patrol_effort: 0.6842,
        chase_effort: 1.0,
        aggro_radius: 460.0,
        attack_range: 140.0,
        contact_strength: 0.85,
        damage_amount: 1,
        default_size: Some((44.0, 78.0)),
        // A cove raider can ride a "shark"-class mount (ADR 0020).
        pilotable_mount_classes: ["shark"],
        brain_template: Smash,
        melee: Some(Swipe((
            windup_s: 0.28,
            active_s: 0.08,
            recover_s: 0.32,
            damage: 1,
            reach_px: 28.0,
        ))),
        move_style: Walk,
    ),
    "pirate_heavy": (
        max_health: 10,
        // A heavy cove pirate can ride a "shark"-class mount (ADR 0020).
        pilotable_mount_classes: ["shark"],
        // Peaceful cove crew until provoked; then forced into an
        // aggressive MeleeBrute with a wide aggro radius.
        hostile_by_default: false,
        body_contact_damage: false,
        respawn: OnRest,
        provoke_forced_brute_min_aggro: Some(500.0),
        // attack_range is the stop-and-swing distance. PirateHeavy's
        // melee hitbox (attack_aabb_dir) reaches size.x*0.55+24+34 =
        // 97.6 px from her center, so against a ~14 px-half player she
        // can only connect at a center distance under ~112 px. The old
        // 150 made her stop ~40 px too far and swing into empty air.
        // 90 stops her inside her own reach (≈21 px hit margin) while
        // keeping ~40 px body clearance.
        attack_range: 90.0,
        run_speed: 130.0,
        patrol_effort: 0.5769,
        chase_effort: 1.0,
        aggro_radius: 0.0,
        contact_strength: 0.0,
        damage_amount: 2,
        default_size: Some((72.0, 110.0)),
        brain_template: MeleeBrute,
        melee: Some(Lunge((
            windup_s: 0.42,
            active_s: 0.12,
            recover_s: 0.46,
            damage: 2,
            reach_px: 38.0,
            step_px: 14.0,
        ))),
        move_style: WalkHeavy,
    ),
    "fixture_mount": (
        max_health: 6, run_speed: 260.0, patrol_effort: 0.42, chase_effort: 1.0,
        aggro_radius: 1200.0, attack_range: 200.0, contact_strength: 1.1,
        damage_amount: 2, is_aerial: Some(true), charge_crash_explodes: true,
        default_size: Some((126.0, 52.0)), mount_class: Some("shark"), mass: 6.0,
        brain_template: ChargeCrash, move_style: Float,
    ),
    // ⭐ **the limbed giant the construction tests plan against, owned by the
    // FIXTURE.** They used to name the shipped `giant_gnu`, so a content
    // migration that deleted that row turned 18 construction tests red about
    // nothing — the D73 campaign deletes shipped rows on purpose, and a
    // structural test that reads shipped content as its fixture fails for the
    // exact reason the campaign is succeeding.
    "fixture_giant": (
        max_health: 42, run_speed: 0.0, patrol_effort: 0.0, chase_effort: 0.0,
        aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
        damage_amount: 0, hostile_by_default: false, body_contact_damage: false,
        mount_class: Some("giant"), mass: 8.0, default_size: Some((220.0, 220.0)),
        brain_template: StandStill, move_style: WalkHeavy,
    ),
    // ⭐ **the ARMED rider the engine's weapon tests need**, owned by the
    // fixture. `pirate_shark_rider` and `pirate_heavy_shark_rider` were the only
    // shipped rows carrying a `held_item`, and they migrated on 2026-08-11 — so
    // five engine tests about "an archetype can resolve a weapon to drop" had no
    // subject left. The MECHANISM is still real and still worth pinning; what it
    // must not be pinned to is content the campaign deletes on purpose.
    // ⭐ **the in-place respawner the ADR-0022 tests need.** `sandbag_finite`
    // was the only shipped row authoring `InPlace(0.85)`, and it migrated on
    // 2026-08-11 — its respawn policy went to its three placements, which is
    // where a respawn policy belongs. The POLICY still has to be exercised.
    "fixture_in_place_respawner": (
        max_health: 6, respawn: InPlace(0.85), run_speed: 155.0,
        patrol_effort: 0.6774, chase_effort: 1.0, aggro_radius: 0.0,
        attack_range: 0.0, contact_strength: 0.0, damage_amount: 0,
        body_contact_damage: false, is_sandbag: true,
        brain_template: StandStill, move_style: Walk,
    ),
    "fixture_armed_rider": (
        max_health: 4, run_speed: 230.0, patrol_effort: 0.4783, chase_effort: 1.0,
        aggro_radius: 1200.0, attack_range: 1100.0, contact_strength: 1.10,
        damage_amount: 2, body_contact_damage: false,
        pilotable_mount_classes: ["shark"], held_item: Some("gun_sword"),
        ranged: Some((style: Bolt, speed: 500.0, damage: 2)),
        brain_template: Skirmisher, move_style: Walk,
    ),
    // The heavy half of the pair: the weapon tests compare a light rider with a
    // heavy one, and one fixture cannot be both.
    "fixture_armed_rider_heavy": (
        max_health: 6, run_speed: 215.0, patrol_effort: 0.5116, chase_effort: 1.0,
        aggro_radius: 1200.0, attack_range: 1100.0, contact_strength: 1.30,
        damage_amount: 3, body_contact_damage: false,
        pilotable_mount_classes: ["shark"], held_item: Some("gun_sword_heavy"),
        ranged: Some((style: Bolt, speed: 500.0, damage: 3)),
        brain_template: Skirmisher, move_style: WalkHeavy,
    ),
    "fixture_rider": (
        max_health: 3, run_speed: 150.0, patrol_effort: 0.6, chase_effort: 1.0,
        aggro_radius: 500.0, attack_range: 150.0, contact_strength: 0.6,
        damage_amount: 1, pilotable_mount_classes: ["shark"],
        brain_template: Skirmisher, move_style: Walk,
    ),
"#;
    CharacterRoster::from_ron(&format!(
        "{}{extra}{}",
        &shipped[..close],
        &shipped[close..]
    ))
}

/// A spec out of the roster THIS CRATE owns — for engine tests that need a
/// SHAPE (armed, rideable, limbed) rather than a shipped creature. See the note
/// on [`test_roster`].
#[cfg(test)]
pub(crate) fn fixture_spec(brain_key: &str) -> ArchetypeSpec {
    fixture_roster_with_mount().spec_for_brain(
        &ambition_entity_catalog::placements::CharacterBrain::Custom(brain_key.to_string()),
    )
}

/// Resolve a spec by its spawn brain key against the checked-in Ambition test
/// fixture. Production callers always receive an explicit App-local roster.
#[cfg(test)]
pub(crate) fn test_spec(brain_key: &str) -> ArchetypeSpec {
    // ⭐ **the ENGINE's roster, not the shipped file.** Every row an engine test
    // names has been migrating out of `character_archetypes.ron` for two days,
    // and `spec_for_brain` answers `combatant` for a key it cannot find — so a
    // test whose subject migrated does not fail, it quietly changes subject.
    // `fixture_roster_with_mount` is the shipped file PLUS the rows the engine
    // owns, so this resolves both and a migration stops being able to retarget
    // a test behind its back.
    fixture_roster_with_mount().spec_for_brain(
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
        let a = CharacterRosterFragment::from_ron("a", A).unwrap();
        let b = CharacterRosterFragment::from_ron("b", B).unwrap();
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
        isolated
            .register_character_roster_fragment(CharacterRosterFragment::from_ron("a", A).unwrap());
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
        app.register_character_roster_fragment(CharacterRosterFragment::from_ron("a", A).unwrap());
        let error = app
            .try_register_character_roster_fragment(
                CharacterRosterFragment::from_ron("b", A).unwrap(),
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
        app.register_character_roster_fragment(CharacterRosterFragment::from_ron("a", A).unwrap());
        let error = app
            .try_register_character_roster_fragment(
                CharacterRosterFragment::from_ron("b", CHILD).unwrap(),
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
        let fragment = CharacterRosterFragment::from_ron("p", BROKEN).unwrap();
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

    /// **A DECLARED IDENTIFIER GETS A BODY; A MISSPELLING AND ANOTHER
    /// PROVIDER'S NAME DO NOT.**
    ///
    /// ⭐ three clauses, one roster, because each of the last two is how the
    /// first would rot. A roster that answers every identifier passes clause one;
    /// so does an engine holding a hard-coded list of creature names, which is
    /// what this replaced — and clause three is the one that tells them apart.
    ///
    /// ⚠ the poison runs against the SAME roster in the SAME test, so a fixture
    /// that lost its rows cannot make a refusal look like a pass.
    #[test]
    fn a_declared_identifier_gets_a_body_and_nothing_else_does() {
        use ambition_entity_catalog::placements::CharacterBrain;
        let mut app = bevy::prelude::App::new();
        app.register_character_roster_fragment(
            CharacterRosterFragment::from_ron("a", A)
                .unwrap()
                .with_open_casting_decision(
                    "small_lurker",
                    "combatant",
                    "ledger D96 — what the cascade summons is undecided",
                ),
        );
        // A SECOND provider, with its own rows and NO declaration. It is the
        // control: whatever provider `a` waived must not be waived for it.
        app.register_character_roster_fragment(CharacterRosterFragment::from_ron("b", B).unwrap());
        let roster = app.world().resource::<CharacterRoster>();

        assert!(
            roster
                .generic_body_for_unresolved_brain(
                    &CharacterBrain::Custom("small_lurker".into()),
                    "this test"
                )
                .is_some(),
            "an identifier its own provider declared still uncast was refused a \
             body, which breaks the content that is waiting on the decision \
             rather than enforcing the rule"
        );

        // ⛔ one character off the declared name — the misspelling class the old
        // global fallback swallowed for three campaigns, and which an engine-side
        // allowlist would also have caught, so this alone proves nothing new.
        assert!(
            roster
                .generic_body_for_unresolved_brain(
                    &CharacterBrain::Custom("small_lurker_".into()),
                    "this test"
                )
                .is_none(),
            "a misspelled identifier was handed a body, so something is answering \
             for names nobody declared again"
        );

        // ⛔⛔ **THE CLAUSE AN ENGINE-OWNED LIST COULD NOT PASS.** A second game,
        // in its own App, declaring nothing. When the three live waivers were a
        // `const` in this crate, it inherited Ambition's open content decisions
        // for free — a silent generic for `small_lurker` because AMBITION had not
        // cast it.
        //
        // ⚠ **it registers roster `A`, and that is the whole fixture.** Written
        // with roster `B` this clause passed against a deliberately broken
        // engine, because `B` has no `combatant` row to borrow — the refusal came
        // from the roster being small, not from the waiver being local, and the
        // test asserted the right thing for the wrong reason. This host owns
        // exactly the row a leaked waiver would hand it.
        let mut alone = bevy::prelude::App::new();
        alone
            .register_character_roster_fragment(CharacterRosterFragment::from_ron("c", A).unwrap());
        assert!(
            alone
                .world()
                .resource::<CharacterRoster>()
                .generic_body_for_unresolved_brain(
                    &CharacterBrain::Custom("small_lurker".into()),
                    "this test"
                )
                .is_none(),
            "a provider that declared nothing was handed a body for another \
             provider's open content decision — the waiver is global again, and \
             a game that never heard of this creature now ships a generic one"
        );
    }

    /// **NO DEFAULT ANSWERS FOR AN UNKNOWN KEY, however many games are linked.**
    ///
    /// ⛔⛔ this test used to open by reading each provider's authored default
    /// back out, and that half is deleted with its subject (2026-08-12). A
    /// per-provider fallback map existed, was `#[cfg(test)]`, and had exactly one
    /// reader: these two lines. Production accepted the ids, validated them, and
    /// discarded the specs — so the test was pinning an invariant about a
    /// concept the engine no longer had, and passing green was how it stayed
    /// invisible.
    ///
    /// ⭐ what survives is the claim that MATTERS and always did: an unknown
    /// brain key resolves nothing, on both roads, with two providers linked —
    /// the configuration where a global default had to pick a winner and did it
    /// differently depending on how many games were in the binary.
    #[test]
    fn no_default_answers_for_an_unknown_key_with_two_providers_linked() {
        let mut app = bevy::prelude::App::new();
        app.register_character_roster_fragment(CharacterRosterFragment::from_ron("a", A).unwrap());
        app.register_character_roster_fragment(
            CharacterRosterFragment::from_ron("b", B_WITH_DEFAULT).unwrap(),
        );
        let roster = app.world().resource::<CharacterRoster>();
        // Both providers' own rows still resolve — the absence below is about
        // the fallback, not about the roster having stopped working.
        for (provider_row, health) in [("combatant", 2), ("beta", 7)] {
            assert_eq!(
                roster
                    .try_spec_for_brain(
                        &ambition_entity_catalog::placements::CharacterBrain::Custom(
                            provider_row.into()
                        )
                    )
                    .unwrap_or_else(|| panic!("`{provider_row}` is an authored row"))
                    .max_health,
                health
            );
        }
        let unknown = ambition_entity_catalog::placements::CharacterBrain::Custom("unknown".into());
        assert!(
            roster.try_spec_for_brain(&unknown).is_none(),
            "an unknown brain key resolved a spec, so some default is answering \
             for it again — with two providers linked, WHOSE?"
        );
        assert!(
            roster
                .generic_body_for_unresolved_brain(&unknown, "this test")
                .is_none(),
            "an identifier no provider declared uncast was handed a generic body \
             anyway, which is the global fallback back under another name"
        );
    }

    /// **A provider that declares no default keeps its rows and gets NO default.**
    ///
    /// ⚠ this used to assert "…and uses the explicit engine-generic default",
    /// which was the global fallback stated as a feature. The rows still
    /// resolve; the absence is the point.
    #[test]
    fn provider_without_a_default_keeps_its_rows_and_gets_no_default() {
        let mut app = bevy::prelude::App::new();
        app.register_character_roster_fragment(CharacterRosterFragment::from_ron("b", B).unwrap());
        let roster = app.world().resource::<CharacterRoster>();
        let beta = ambition_entity_catalog::placements::CharacterBrain::Custom("beta".into());
        let unknown = ambition_entity_catalog::placements::CharacterBrain::Custom("unknown".into());
        assert_eq!(
            roster
                .try_spec_for_brain(&beta)
                .expect("the provider's own row still resolves")
                .max_health,
            7
        );
        assert!(
            roster.try_spec_for_brain(&unknown).is_none(),
            "a provider that declared no default was given one"
        );
    }
}

#[cfg(test)]
mod capability_tests;
#[cfg(test)]
mod enemy_archetype_data_tests;
#[cfg(test)]
mod movement_tuning_tests;
