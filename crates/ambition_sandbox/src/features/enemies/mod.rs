use super::*;

mod integration;

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

/// An actor's locomotion contact + vertical-control state, maintained
/// by the kinematic integration each tick.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorSurfaceState {
    /// Set by [`crate::kinematic::step_kinematic`](crate::engine_core::step_kinematic)
    /// each tick. Used by chase-drop-through (enemy must be standing on
    /// something before it tries to fall through it) and by jump AI.
    pub on_ground: bool,
    /// Outward-pointing unit normal of the surface the actor is
    /// currently clinging to. Used by surface-walking archetypes
    /// (`PuppySlug`) to crawl floors, walls, and ceilings; every other
    /// archetype pins this at `(0, -1)` (floor) and ignores it. Engine
    /// y grows downward, so floor → (0, -1), right wall → (-1, 0),
    /// ceiling → (0, 1), left wall → (1, 0).
    pub surface_normal: ae::Vec2,
    /// 0.0 = ignores gravity (flying); 1.0 = full gravity.
    pub gravity_scale: f32,
    /// Mid-air jumps the actor has left until next landing. Reset to
    /// `MAX_ENEMY_AIR_JUMPS` when `on_ground` transitions false → true
    /// in the integration step. Decremented when `frame.jump_pressed`
    /// fires while airborne AND a jump remains; the grounded-jump path
    /// doesn't touch this counter.
    pub air_jumps_remaining: u8,
}

// `EnemyRespawnPolicy` moved to the combat kit (generic death/respawn
// vocabulary); re-exported so `crate::features::EnemyRespawnPolicy`
// paths keep working.
pub use crate::mechanics::combat::EnemyRespawnPolicy;

/// Flag-id suffix used by `_dead_until_rest` flags. Constant so the
/// kill hook, save sync, and `clear_dead_until_rest_flags` all
/// agree on the spelling.
pub const ENEMY_DEAD_UNTIL_REST_SUFFIX: &str = "_dead_until_rest";

/// Authored mount+rider visual fan-out for a composite spawn (see
/// [`EnemyArchetypeSpec::composite_visual`]). `mount_brain` / `rider_brain`
/// are spawn brain keys into the roster; the names are display fallbacks for
/// the spawned visuals.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct CompositeVisualSpec {
    pub mount_brain: String,
    pub mount_name: String,
    pub rider_brain: String,
    pub rider_fallback_name: String,
    /// When true, the rider's display name comes from the authored
    /// spawn name minus its " on Shark" suffix (named heavy variants);
    /// otherwise the fallback name is always used.
    #[serde(default)]
    pub rider_name_from_spawn: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub(crate) struct EnemyArchetypeSpec {
    pub max_health: i32,
    #[serde(default)]
    pub rider_max_health: Option<i32>,
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
    /// On death, respawn in place after this many seconds.
    #[serde(default)]
    pub respawn_in_place_seconds: Option<f32>,
    /// Deep-dream visual jitter seed (psychedelic shader pass);
    /// `None` = the archetype doesn't participate.
    #[serde(default)]
    pub dream_seed: Option<f32>,
    /// When set, this spawn renders as a mount + rider visual pair
    /// (the sim fans it into two entities; presentation mirrors that).
    #[serde(default)]
    pub composite_visual: Option<CompositeVisualSpec>,
    #[serde(default, with = "vec2_option")]
    pub default_size: Option<ae::Vec2>,
    /// Brain template the spawn site instantiates for this archetype.
    /// MeleeBrute reads the archetype's tunings (chase_speed,
    /// aggro_radius, attack_range) for its cfg; Wanderer + StandStill
    /// ignore them.
    pub brain_template: EnemyBrainTemplate,
    /// Concrete melee action this archetype's `ActionSet` carries.
    /// `None` = no melee capability (peaceful patrollers, ranged-only
    /// actors).
    #[serde(default)]
    pub melee: Option<crate::brain::MeleeActionSpec>,
    /// Concrete ranged action this archetype's `ActionSet` carries.
    /// `None` = no ranged capability.
    #[serde(default)]
    pub ranged: Option<crate::brain::RangedActionSpec>,
    /// Optional held-item id, resolved against the held-item registry
    /// (`crate::brain::held_item_by_id`). The item's abilities overlay the
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
    /// Defeated body takes a Rest to reappear (heavier mini-boss-tier
    /// presences) instead of refreshing on every room re-entry.
    #[serde(default)]
    pub respawn_on_rest: bool,
    /// Locomotion style for the actor's `ActionSet.move_style`.
    pub move_style: crate::brain::MoveStyleSpec,
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
    use crate::engine_core as ae;
    use serde::Deserialize;

    pub fn deserialize<'de, D>(de: D) -> Result<Option<ae::Vec2>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: Option<(f32, f32)> = Option::deserialize(de)?;
        Ok(raw.map(|(x, y)| ae::Vec2::new(x, y)))
    }
}

/// Brain template choice keyed off `EnemyArchetype`. The definition is
/// generic kit vocabulary — re-exported here so the archetype spec row
/// (`brain_template`) and the spawn-site projection keep their existing
/// path. See [`crate::mechanics::combat::EnemyBrainTemplate`].
pub(super) use crate::mechanics::combat::EnemyBrainTemplate;

/// Serde default for [`EnemyArchetypeSpec::attack_cooldown_mult`]: the
/// multiplicative identity (most archetypes use the shared cooldown).
fn default_attack_cooldown_mult() -> f32 {
    1.0
}

fn default_mass() -> f32 {
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
    std::collections::HashMap<String, EnemyArchetypeSpec>,
> = std::sync::LazyLock::new(|| {
    const ENEMY_ARCHETYPES_RON: &str =
        include_str!("../../../../ambition_content/assets/data/enemy_archetypes.ron");
    ron::from_str(ENEMY_ARCHETYPES_RON).unwrap_or_else(|err| {
        panic!("ambition_content enemy_archetypes.ron failed to deserialize: {err}")
    })
});

/// The installed enemy roster: a brain-key → spec table plus the fallback
/// spec used for unknown brain keys and non-`Custom` brains. This is the
/// spawn path's only resolution surface and it is **roster-enum-free** — a
/// pure string lookup, so the named `EnemyArchetype` enum / RON / brain-name
/// table can be owned and installed by the content layer.
///
/// Held as an installable global (not a Bevy `Resource`) because spec
/// resolution is read from many non-system contexts — plain constructors
/// (`EnemyClusterSeed::new`), presentation sprite-binding
/// (`presentation::rendering::world`), and asset resolution
/// (`assets::game_assets`) — where threading `Res<EnemyRoster>` would be a
/// pervasive, ugly ripple. The content layer installs the real table at
/// startup via [`install_enemy_roster`]; the lib ships an embedded default
/// (built from the bundled RON) so lib tests and the headless bin resolve
/// standalone.
#[derive(Clone, Debug)]
pub struct EnemyRoster {
    by_brain: std::collections::HashMap<String, EnemyArchetypeSpec>,
    fallback: EnemyArchetypeSpec,
}

impl EnemyRoster {
    /// Build a roster from a brain-key → spec table and the fallback spec
    /// (resolved for any unknown brain key, mirroring `from_brain`'s
    /// `Combatant` default).
    pub(crate) fn new(
        by_brain: std::collections::HashMap<String, EnemyArchetypeSpec>,
        fallback: EnemyArchetypeSpec,
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

    /// Resolve the authored spec for a spawn `EnemyBrain` payload by its
    /// `Custom("…")` brain key, falling back to the roster's default for an
    /// unknown key or a non-`Custom` brain.
    pub(crate) fn spec_for_brain(&self, brain: &crate::actor::EnemyBrain) -> EnemyArchetypeSpec {
        let key = match brain {
            crate::actor::EnemyBrain::Custom(name) => name.as_str(),
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
    /// the map keys ARE the spawn brain keys, so no `EnemyArchetype` is named.
    pub(crate) fn from_map(
        by_brain: std::collections::HashMap<String, EnemyArchetypeSpec>,
    ) -> Self {
        let fallback = by_brain
            .get("combatant")
            .cloned()
            .expect("enemy roster must define a \"combatant\" fallback row");
        Self::new(by_brain, fallback)
    }

    /// Parse a brain-keyed roster RON document — the content layer's entry
    /// point: `install_enemy_roster(EnemyRoster::from_ron(MY_RON))`.
    pub fn from_ron(ron: &str) -> Self {
        let by_brain: std::collections::HashMap<String, EnemyArchetypeSpec> = ron::from_str(ron)
            .unwrap_or_else(|err| panic!("enemy roster RON failed to deserialize: {err}"));
        Self::from_map(by_brain)
    }
}

/// Test-only fallback roster, parsed from the lib's bundled fixture RON so
/// the lib's own unit tests resolve enemies standalone (without the content
/// plugin). In a real build the named roster is owned and installed by
/// `ambition_content`; the production binary embeds no enemy data here.
#[cfg(test)]
static EMBEDDED_ENEMY_ROSTER: std::sync::LazyLock<EnemyRoster> =
    std::sync::LazyLock::new(|| EnemyRoster::from_map(ENEMY_ARCHETYPE_REGISTRY.clone()));

/// Content-installed roster. Set once at plugin-build time; production
/// resolution REQUIRES it (there is no production embedded default).
static ENEMY_ROSTER_OVERRIDE: std::sync::OnceLock<EnemyRoster> = std::sync::OnceLock::new();

/// Install the authored enemy roster — the content layer calls this at
/// plugin-build time (before any spawn system runs). First install wins; later
/// calls are ignored, so a mid-run call can't clobber the live roster.
pub fn install_enemy_roster(roster: EnemyRoster) {
    let _ = ENEMY_ROSTER_OVERRIDE.set(roster);
}

#[cfg(test)]
fn roster_fallback() -> &'static EnemyRoster {
    &EMBEDDED_ENEMY_ROSTER
}

/// Production has no embedded enemy data: the content plugin must install the
/// roster at build time. Reaching here means `AmbitionContentPlugin` was not
/// mounted before the first enemy spawn.
#[cfg(not(test))]
fn roster_fallback() -> &'static EnemyRoster {
    panic!(
        "enemy roster not installed — AmbitionContentPlugin must call \
         install_enemy_roster() at build time before any enemy spawns"
    )
}

fn enemy_roster() -> &'static EnemyRoster {
    ENEMY_ROSTER_OVERRIDE.get().unwrap_or_else(roster_fallback)
}

/// Resolve the authored spec for a spawn `EnemyBrain` payload — a pure
/// string lookup against the installed [`EnemyRoster`]. The spawn path holds
/// the returned spec; the roster enum never appears here.
pub(crate) fn spec_for_brain(brain: &crate::actor::EnemyBrain) -> EnemyArchetypeSpec {
    enemy_roster().spec_for_brain(brain)
}

/// Resolve a spec by its spawn brain key against the lib's `#[cfg(test)]`
/// fixture roster — the test-side replacement for the deleted
/// `EnemyArchetype::X.spec()`. Tests are roster-string-keyed now: the named
/// enum is gone, so they reference enemies by the same `Custom("…")` key the
/// game authors.
#[cfg(test)]
pub(crate) fn test_spec(brain_key: &str) -> EnemyArchetypeSpec {
    spec_for_brain(&crate::actor::EnemyBrain::Custom(brain_key.to_string()))
}

/// Every authored spawn brain key in the lib's fixture roster — the
/// string-keyed replacement for the deleted `EnemyArchetype` iteration
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
    "pirate_on_shark",
    "puppy_slug",
    "pirate_heavy",
    "pirate_heavy_on_shark",
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
    "pirate_on_shark",
    "pirate_heavy",
    "pirate_heavy_on_shark",
    "puppy_slug",
    "exploding_mite",
    "dividing_mite",
    "ranged_skirmisher",
];

impl EnemyArchetypeSpec {
    /// Project the generic brain-construction inputs (kit vocabulary) the
    /// runtime brain rebuilds reconstruct without naming the roster.
    pub(super) fn brain_spec(&self) -> crate::mechanics::combat::EnemyBrainSpec {
        crate::mechanics::combat::EnemyBrainSpec {
            template: self.brain_template,
            smash_hit_band: self.smash_hit_band.unwrap_or(36.0),
            smash_heavy: self.smash_heavy,
            smash_dash_to_close: self.smash_dash_to_close,
            provoke_forced_brute_min_aggro: self.provoke_forced_brute_min_aggro,
        }
    }

    /// Authored held item resolved against the held-item registry.
    pub(super) fn held_item_spec(&self) -> Option<crate::brain::HeldItemSpec> {
        self.held_item
            .as_deref()
            .and_then(crate::brain::held_item_by_id)
    }

    /// Concrete melee/ranged/locomotion the actor's `ActionSet` carries
    /// at spawn. Thin field accessors so the spawn path can read the spec
    /// without naming the roster enum.
    pub(super) fn melee_spec(&self) -> Option<crate::brain::MeleeActionSpec> {
        self.melee.clone()
    }
    pub(super) fn ranged_spec(&self) -> Option<crate::brain::RangedActionSpec> {
        self.ranged.clone()
    }
    pub(super) fn move_style(&self) -> crate::brain::MoveStyleSpec {
        self.move_style
    }
    /// True when this spawn renders / fans out as a mount + rider pair.
    pub(super) fn is_composite(&self) -> bool {
        self.composite_visual.is_some()
    }

    /// Default respawn cadence: heavier presences take a Rest, the rest
    /// refresh on every room re-entry.
    pub(super) fn respawn_policy(&self) -> EnemyRespawnPolicy {
        if self.respawn_on_rest {
            EnemyRespawnPolicy::OnRest
        } else {
            EnemyRespawnPolicy::OnRoomReenter
        }
    }

    /// Project the per-frame runtime tuning carried on `EnemyConfig.tuning`.
    pub(crate) fn tuning(&self) -> crate::mechanics::combat::EnemyTuning {
        crate::mechanics::combat::EnemyTuning {
            max_health: self.max_health,
            patrol_speed: self.patrol_speed,
            chase_speed: self.chase_speed,
            aggro_radius: self.aggro_radius,
            attack_range: self.attack_range,
            contact_strength: self.contact_strength,
            damage_amount: self.damage_amount,
            attack_cooldown_mult: self.attack_cooldown_mult,
            attacks_player: self.attacks_player,
            surface_walker: self.surface_walker,
            cling_breaks_on_hit: self.cling_breaks_on_hit,
            // Self-revive loop = the authored respawn-in-place timer exists.
            revives_in_place: self.respawn_in_place_seconds.is_some(),
            is_aerial: self.is_aerial,
            is_sandbag: self.is_sandbag,
            body_contact_damage: self.body_contact_damage,
            dream_seed: self.dream_seed,
        }
    }

    /// Project the authored capability flags into the combat kit.
    pub(crate) fn combat_capabilities(&self) -> crate::mechanics::combat::CombatCapabilities {
        crate::mechanics::combat::CombatCapabilities {
            explodes_on_death: self.explodes_on_death,
            divides_on_death: self.divides_on_death,
            charge_crash_explodes: self.charge_crash_explodes,
            never_dies: self.never_dies,
            respawn_in_place_seconds: self.respawn_in_place_seconds,
            respawn_policy: self.respawn_policy(),
            drops_held_item: self.held_item_spec(),
        }
    }
}
/// Per-spawn VISUAL plan for an enemy payload, derived from authored
/// archetype data. Presentation consumes this instead of the
/// archetype enum (Stage 20 / B3): the named knowledge stays on this
/// side of the named/generic boundary, as data.
#[derive(Clone, Debug)]
pub struct CompositeVisualPlan {
    pub rider_name_from_spawn: bool,
    pub mount_name: String,
    pub mount_brain: crate::actor::EnemyBrain,
    pub rider_brain: crate::actor::EnemyBrain,
    pub rider_fallback_name: String,
    /// Rider's standalone body size (the visual renders at half while
    /// mounted, mirroring the sim's `MountedSize`).
    pub rider_standalone_size: ae::Vec2,
    pub mount_size: ae::Vec2,
}

/// Visual kind for an enemy spawn payload (training dummies render as
/// sandbags; everything else as a standard enemy).
pub fn enemy_visual_kind(payload: &crate::actor::EnemyBrain) -> FeatureVisualKind {
    if spec_for_brain(payload).is_sandbag {
        FeatureVisualKind::TrainingDummy
    } else {
        FeatureVisualKind::Enemy
    }
}

/// The mount+rider visual fan-out plan for a composite spawn payload,
/// or `None` for ordinary single-entity spawns. Backed by the
/// `composite_visual` rows in `enemy_archetypes.ron`.
pub fn composite_visual_plan(payload: &crate::actor::EnemyBrain) -> Option<CompositeVisualPlan> {
    let spec = spec_for_brain(payload);
    let composite = spec.composite_visual.as_ref()?;
    let mount_brain = crate::actor::EnemyBrain::Custom(composite.mount_brain.clone());
    let rider_brain = crate::actor::EnemyBrain::Custom(composite.rider_brain.clone());
    let rider_standalone_size = spec_for_brain(&rider_brain)
        .default_size
        .unwrap_or(ae::Vec2::new(44.0, 78.0));
    let mount_size = spec_for_brain(&mount_brain)
        .default_size
        .unwrap_or(ae::Vec2::new(126.0, 52.0));
    Some(CompositeVisualPlan {
        rider_name_from_spawn: composite.rider_name_from_spawn,
        mount_name: composite.mount_name.clone(),
        mount_brain,
        rider_brain,
        rider_fallback_name: composite.rider_fallback_name.clone(),
        rider_standalone_size,
        mount_size,
    })
}

#[cfg(test)]
mod enemy_archetype_data_tests {
    use super::integration::enemy_attack_aabb_dir;
    use super::*;

    /// The installable [`EnemyRoster`] holder resolves a known brain key to
    /// its spec and falls back for an unknown / non-`Custom` brain, and the
    /// lib's embedded default reproduces `from_brain` exactly (the
    /// replay-identity guarantee for the resolution inversion). Built
    /// locally so it doesn't touch the process-global override.
    #[test]
    fn enemy_roster_resolves_brain_keys_with_fallback() {
        use crate::actor::EnemyBrain;
        let mut by_brain = std::collections::HashMap::new();
        by_brain.insert("pirate_heavy".to_string(), test_spec("pirate_heavy"));
        let roster = EnemyRoster::new(by_brain, test_spec("combatant"));
        // Known key → its spec (PirateHeavy is peaceful by default).
        assert!(
            !roster
                .spec_for_brain(&EnemyBrain::Custom("pirate_heavy".into()))
                .attacks_player
        );
        // Unknown key + non-Custom → fallback (Combatant is hostile).
        assert!(
            roster
                .spec_for_brain(&EnemyBrain::Custom("does_not_exist".into()))
                .attacks_player
        );
    }

    /// The fixture roster must carry a row for every authored spawn brain key
    /// (a missing row would resolve to the `combatant` fallback rather than
    /// the intended enemy).
    #[test]
    fn ron_carries_every_known_brain_key() {
        for key in ALL_BRAIN_KEYS {
            assert!(
                ENEMY_ARCHETYPE_REGISTRY.contains_key(*key),
                "enemy_archetypes.ron missing row for brain key '{key}'",
            );
        }
    }

    /// Spot-check the legacy pre-data values for two divergent
    /// archetypes so a regen of the RON without re-tuning catches
    /// accidental drift on the rows the player notices first.
    #[test]
    fn legacy_baseline_pins() {
        use crate::brain::MeleeActionSpec;
        let combatant = test_spec("combatant");
        assert_eq!(combatant.max_health, 4);
        assert!((combatant.chase_speed - 155.0).abs() < f32::EPSILON);
        assert!((combatant.aggro_radius - 460.0).abs() < f32::EPSILON);
        assert!(
            matches!(combatant.melee, Some(MeleeActionSpec::Swipe(_))),
            "Combatant melee should be Swipe; got {:?}",
            combatant.melee
        );
        let slug = test_spec("puppy_slug");
        assert_eq!(slug.max_health, 2);
        assert!((slug.patrol_speed - 55.0).abs() < f32::EPSILON);
        assert_eq!(slug.aggro_radius, 0.0);
        assert_eq!(slug.brain_template, EnemyBrainTemplate::Wanderer);
        assert!(slug.melee.is_none());
        assert!(slug.ranged.is_none());
    }

    /// The two gun-sword archetypes reference their weapon by id in the
    /// RON; guard that the id resolves against the held-item registry
    /// (a typo would silently drop the weapon, leaving them unarmed) and
    /// that the resolved Bolt damage matches the authored per-archetype
    /// scaling.
    #[test]
    fn gun_sword_archetypes_resolve_held_item_by_id() {
        use crate::brain::RangedActionSpec;
        let on_shark = test_spec("pirate_on_shark")
            .held_item_spec()
            .expect("PirateOnShark should resolve a held item");
        assert_eq!(on_shark.id, "gun_sword");
        assert!(matches!(
            on_shark.ranged,
            Some(RangedActionSpec::Bolt { damage: 2, .. })
        ));
        let heavy = test_spec("pirate_heavy_on_shark")
            .held_item_spec()
            .expect("PirateHeavyOnShark should resolve a held item");
        assert_eq!(heavy.id, "gun_sword_heavy");
        assert!(matches!(
            heavy.ranged,
            Some(RangedActionSpec::Bolt { damage: 3, .. })
        ));
    }

    /// The Smash melee hit band is now authored per-archetype in the RON
    /// (CharacterAI migration #194). Guard the values that drove the old
    /// `smash_cfg_for_archetype` match arms so a RON re-tune can't silently
    /// resize the goblin/brute hit bands, and confirm the 36px-default
    /// archetypes correctly omit the field (fall through to the builder
    /// fallback).
    #[test]
    fn smash_hit_band_is_data_authored() {
        assert_eq!(
            crate::features::enemies::test_spec("medium_striker").smash_hit_band,
            Some(32.0)
        );
        assert_eq!(
            crate::features::enemies::test_spec("small_skitter").smash_hit_band,
            Some(32.0)
        );
        assert_eq!(
            crate::features::enemies::test_spec("small_lurker").smash_hit_band,
            Some(32.0)
        );
        assert_eq!(
            crate::features::enemies::test_spec("large_brute").smash_hit_band,
            Some(48.0)
        );
        assert_eq!(
            crate::features::enemies::test_spec("large_colossus").smash_hit_band,
            Some(48.0)
        );
        // 36px-default Smash archetypes omit the field on purpose.
        assert_eq!(
            crate::features::enemies::test_spec("combatant").smash_hit_band,
            None
        );
        assert_eq!(
            crate::features::enemies::test_spec("gradient_seeker").smash_hit_band,
            None
        );
        assert_eq!(
            crate::features::enemies::test_spec("pirate_raider").smash_hit_band,
            None
        );
    }

    #[test]
    fn body_contact_damage_is_explicitly_opted_in() {
        assert!(crate::features::enemies::test_spec("combatant").body_contact_damage);
        assert!(crate::features::enemies::test_spec("puppy_slug").body_contact_damage);
        assert!(!crate::features::enemies::test_spec("pirate_heavy").body_contact_damage);
        assert!(!crate::features::enemies::test_spec("pirate_on_shark").body_contact_damage);
        assert!(!crate::features::enemies::test_spec("sandbag_finite").body_contact_damage);
    }

    /// Regression for the cove bug "an aggressive PirateHeavy never gets
    /// close enough to land a hit." `attack_range` is the
    /// stop-and-swing distance read by `evaluate_character_ai_output`;
    /// her horizontal melee hitbox (`attack_aabb_dir`) only reaches
    /// `size.x*0.55 + 24 + 34` px from her center. If `attack_range`
    /// exceeds that far edge she halts out of reach and swings into
    /// empty air. Pin that `attack_range` stays inside the swing reach
    /// so the strike can actually overlap a player standing at the
    /// stop distance.
    #[test]
    fn pirate_heavy_stops_within_her_melee_reach() {
        let spec = test_spec("pirate_heavy");
        let authored_aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(36.0, 55.0));
        let pos = authored_aabb.center();
        let size = spec
            .default_size
            .unwrap_or_else(|| authored_aabb.half_size() * 2.0);
        let hitbox = enemy_attack_aabb_dir(pos, size, 1.0, ae::Vec2::new(1.0, 0.0));
        let reach_edge = hitbox.center().x + hitbox.half_size().x - pos.x;
        let attack_range = spec.tuning().attack_range;
        assert!(
            attack_range <= reach_edge,
            "PirateHeavy attack_range {attack_range} must stay within her swing far \
             edge {reach_edge} so she stops inside her own reach instead of whiffing",
        );
    }
}

#[cfg(test)]
mod capability_tests {
    use super::{test_spec, ALL_BRAIN_KEYS};

    /// Pin the authored capability rows in `enemy_archetypes.ron` to the
    /// behavior the actor layer used to hardcode by archetype identity
    /// (Stage 20: the named checks became data-driven capabilities).
    #[test]
    fn archetype_capabilities_match_the_legacy_identity_checks() {
        let mite = crate::features::enemies::test_spec("exploding_mite").combat_capabilities();
        assert!(mite.explodes_on_death && !mite.divides_on_death);

        let blob = crate::features::enemies::test_spec("dividing_mite").combat_capabilities();
        assert!(blob.divides_on_death && !blob.explodes_on_death);

        let shark =
            crate::features::enemies::test_spec("burning_flying_shark").combat_capabilities();
        assert!(shark.charge_crash_explodes);

        let infinite =
            crate::features::enemies::test_spec("sandbag_infinite").combat_capabilities();
        assert!(infinite.never_dies && infinite.respawn_in_place_seconds.is_none());

        let finite = crate::features::enemies::test_spec("sandbag_finite").combat_capabilities();
        assert!(!finite.never_dies);
        assert_eq!(finite.respawn_in_place_seconds, Some(0.85));

        // A plain combatant has no special capabilities.
        let base = crate::features::enemies::test_spec("combatant").combat_capabilities();
        assert_eq!(base, Default::default());
    }

    /// The Stochastic Parrot's DUAL nature, proven from the authored data:
    ///   - the friendly cove bird is a catalog character (`stochastic_parrot`,
    ///     peaceful) — its sprite binds by `character_id`;
    ///   - the aggressive sky raiders are the `sky_parrot` enemy archetype —
    ///     hostile + aerial, reusing the Shark dive brain;
    ///   - both wear the SAME parrot sprite. The aggressive form binds by
    ///     DISPLAY NAME, so this pins that the enemy's authored spawn name
    ///     ("Stochastic Parrot", set on the sky `EnemySpawn`s) exactly equals
    ///     the catalog `display_name` — the fragile string join in P2 of the
    ///     content-authoring pain-points journal. If someone renames either
    ///     side, the sky parrots silently lose their sprite; this test screams.
    #[test]
    fn stochastic_parrot_is_friendly_in_the_cove_and_hostile_in_the_sky() {
        use super::EnemyBrainTemplate;

        // Aggressive sky form.
        let sky = test_spec("sky_parrot");
        assert!(sky.attacks_player, "sky_parrot is hostile by default");
        assert!(sky.is_aerial, "sky_parrot flies (aerial, no gravity)");
        assert!(sky.melee.is_some(), "sky_parrot has a dive/peck melee");
        assert_eq!(
            sky.brain_template,
            EnemyBrainTemplate::Shark,
            "sky_parrot reuses the aerial Shark dive brain",
        );

        // Friendly cove form: a catalog character with a peaceful default.
        let display = crate::character_roster::display_name_for_character_id("stochastic_parrot");
        assert_eq!(
            display,
            Some("Stochastic Parrot"),
            "the catalog display_name MUST equal the sky EnemySpawn name, or the \
             aggressive parrot loses its sprite (P2 name-join)",
        );
        // Both forms wear the same parrot sheet (the friendly form binds it by
        // character_id; the sheet must actually resolve).
        assert!(
            crate::character_sprites::sheet_for_character_id("stochastic_parrot").is_some(),
            "the parrot catalog row must resolve a sprite sheet",
        );
    }

    /// Parity net for the Session-6/7 data migration: the four behaviors
    /// that used to be hardcoded `match self { … }` arms on the enum are now
    /// authored RON fields (`attacks_player`, `body_contact_damage`,
    /// `respawn_on_rest`, the smash/provoke flags). Re-encode the OLD
    /// identity formulas here as the oracle and assert every archetype's
    /// RON row reproduces them — replay only exercises the archetypes in the
    /// fixture, so this guards the exotic rows (sandbags, mites, composites)
    /// against a silent mis-migration.
    #[test]
    fn ron_derived_behaviors_match_the_legacy_identity_formulas() {
        use super::EnemyRespawnPolicy;
        for &key in ALL_BRAIN_KEYS {
            let spec = test_spec(key);
            let attacks = !matches!(key, "puppy_slug" | "pirate_heavy");
            assert_eq!(spec.attacks_player, attacks, "{key} attacks_player");

            let body = !matches!(
                key,
                "sandbag_infinite" | "sandbag_finite" | "pirate_on_shark" | "pirate_heavy_on_shark"
            ) && (attacks || key == "puppy_slug");
            assert_eq!(spec.body_contact_damage, body, "{key} body_contact");

            let policy = if matches!(
                key,
                "large_brute"
                    | "large_colossus"
                    | "pirate_heavy"
                    | "pirate_on_shark"
                    | "pirate_heavy_on_shark"
            ) {
                EnemyRespawnPolicy::OnRest
            } else {
                EnemyRespawnPolicy::OnRoomReenter
            };
            assert_eq!(spec.respawn_policy(), policy, "{key} respawn_policy");

            let bs = spec.brain_spec();
            assert_eq!(
                bs.smash_heavy,
                matches!(key, "large_brute" | "large_colossus"),
                "{key} smash_heavy"
            );
            assert_eq!(
                bs.smash_dash_to_close,
                key == "medium_striker",
                "{key} smash_dash_to_close"
            );
            assert_eq!(
                bs.provoke_forced_brute_min_aggro,
                if key == "pirate_heavy" {
                    Some(500.0)
                } else {
                    None
                },
                "{key} provoke_forced_brute_min_aggro"
            );
        }
    }
}
