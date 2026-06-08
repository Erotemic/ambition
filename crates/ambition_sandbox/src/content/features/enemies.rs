use super::ecs::enemy_clusters::EnemyMut;
use super::*;

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

/// The authored spawn baseline an actor reverts to on a same-room
/// reset. Grouped out of `EnemyRuntime`'s flat `spawn` /
/// `spawn_archetype` / `spawn_size` fields. `archetype` and `size`
/// can mutate at runtime (PirateOnShark dismounts into PirateRaider
/// with a different `default_size`), so this records the level
/// author's original so [`EnemyRuntime::reset_to_spawn`] can rebuild
/// the fused actor. Identical to the live `archetype`/`size` for every
/// non-morphing actor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorSpawnState {
    /// World position the actor spawned at.
    pub pos: ae::Vec2,
    /// Authored archetype — the "what the level author wrote" record.
    pub archetype: EnemyArchetype,
    /// Authored body size.
    pub size: ae::Vec2,
}

/// An actor's locomotion contact + vertical-control state, grouped out
/// of `EnemyRuntime`'s flat `on_ground` / `surface_normal` /
/// `gravity_scale` / `air_jumps_remaining` fields. Maintained by the
/// kinematic integration each tick.
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
    /// 0.0 = ignores gravity (flying); 1.0 = full gravity. Set by the
    /// archetype (`BurningFlyingShark` / `PirateOnShark` are 0.0).
    pub gravity_scale: f32,
    /// Mid-air jumps the actor has left until next landing. Reset to
    /// `MAX_ENEMY_AIR_JUMPS` when `on_ground` transitions false → true
    /// in the integration step. Decremented when `frame.jump_pressed`
    /// fires while airborne AND a jump remains; the grounded-jump path
    /// doesn't touch this counter.
    pub air_jumps_remaining: u8,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyRespawnPolicy {
    /// Fresh every time the player enters the room. Default for
    /// trash grunts (skitters, lurkers, raiders, puppy slugs).
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

/// Flag-id suffix used by `_dead_until_rest` flags. Constant so the
/// kill hook, save sync, and `clear_dead_until_rest_flags` all
/// agree on the spelling.
pub const ENEMY_DEAD_UNTIL_REST_SUFFIX: &str = "_dead_until_rest";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
pub enum EnemyArchetype {
    Combatant,
    SmallSkitter,
    MediumStriker,
    LargeBrute,
    AggressiveSeeker,
    InfiniteSandbag,
    FiniteSandbag,
    /// Small + low aggression: slow patrol, tiny aggro radius, low
    /// damage. Fits scenery-flavored encounters (rats, fungi) where
    /// the player can ignore them but a careless approach still
    /// punishes.
    SmallLurker,
    /// Large + low aggression: bigger HP / damage than `LargeBrute`
    /// but with a much narrower aggro radius. Boss-room
    /// "stationary heavy" archetype — the player has to step
    /// inside its threat envelope deliberately.
    LargeColossus,
    /// Grounded pirate (dismounted form of `PirateOnShark`). Melee
    /// striker with a cutlass — same kit as a `MediumStriker` but
    /// with the pirate sprite.
    PirateRaider,
    /// Riderless burning flying shark. Aerial dive-strike pattern.
    BurningFlyingShark,
    /// Authored pirate-on-shark composite. The ECS spawn path fans
    /// this out into separate mount and rider actor entities. The
    /// rider dismounts into `PirateRaider` when the shark dies, and
    /// the mount continues as `BurningFlyingShark` when the rider dies.
    PirateOnShark,
    /// Deep-dream "puppy slug" — small ground-walker (Crawlid
    /// analogue from Hollow Knight). Always patrols, no chase, no
    /// attack windup. Body-contact damages on touch. The brain
    /// reverses facing at walls (the standard patrol-blocked path)
    /// AND at ledges (custom probe in `update`) so it never falls
    /// off platforms — even though `aggro_radius = 0` keeps it
    /// completely ignorant of the player.
    PuppySlug,
    /// Pirate heavy bruiser. Slow, tanky, big-cleaver swing. Three
    /// authored sprite variants (Broadside Bess, Iron Mary, Salt
    /// Annet) all map to this archetype — variants differ by
    /// EnemySpawn display name, not by tuning.
    PirateHeavy,
    /// Pirate heavy riding a burning flying shark. Mechanically a
    /// `PirateOnShark` composite, but the rider sprite resolves to
    /// one of the heavy-variant sheets instead of `Pirate Raider`.
    /// On shark-death dismount, the rider drops to a ground
    /// `PirateHeavy` (heavier and slower than a `PirateRaider`).
    PirateHeavyOnShark,
    /// Volatile kamikaze mite — low HP, fast aggressive rush, and it
    /// **detonates on death** in a sizable Enemy-faction blast
    /// (`damage.rs::spawn_death_explosion`). The threat is the blast,
    /// not the body: meleeing it point-blank eats the explosion, so the
    /// read is "kill it at range or sidestep the corpse." Thematically
    /// the Exploding Gradient boss's runaway spawn.
    ExplodingMite,
    /// Replicating blob — slow and a bit tanky, and on death it **splits
    /// into two fast `SmallSkitter` offspring** (one level deep — the
    /// children don't re-split). The read is the inverse of the mite's:
    /// a deliberate priority target you whittle down, then clean up the
    /// two quick children before they swarm. Thematically an overfit
    /// model memorizing (replicating) its data points.
    DividingMite,
    /// Ranged skirmisher — the roster's only true **kiter**. It holds at
    /// long range and peppers you with arrows, and backs off early when
    /// you close (a large `too_close_distance` in
    /// `smash_cfg_for_archetype`), so the read is "chase it down or use
    /// your own ranged/AOE" rather than the melee rushers' "block and
    /// punish." Thematically an outlier that snipes from the margins.
    RangedSkirmisher,
}

/// Maps `crate::actor::EnemyBrain::Custom("...")` strings to archetype variants.
/// `from_brain` walks this table, falling back to `Combatant` for any
/// unknown brain string or a non-`Custom` variant.
const BRAIN_NAME_TO_ARCHETYPE: &[(&str, EnemyArchetype)] = &[
    ("small_skitter", EnemyArchetype::SmallSkitter),
    ("small_lurker", EnemyArchetype::SmallLurker),
    ("medium_striker", EnemyArchetype::MediumStriker),
    ("large_brute", EnemyArchetype::LargeBrute),
    ("large_colossus", EnemyArchetype::LargeColossus),
    ("gradient_seeker", EnemyArchetype::AggressiveSeeker),
    ("sandbag_infinite", EnemyArchetype::InfiniteSandbag),
    ("sandbag_finite", EnemyArchetype::FiniteSandbag),
    ("pirate_raider", EnemyArchetype::PirateRaider),
    ("burning_flying_shark", EnemyArchetype::BurningFlyingShark),
    ("pirate_on_shark", EnemyArchetype::PirateOnShark),
    ("puppy_slug", EnemyArchetype::PuppySlug),
    ("pirate_heavy", EnemyArchetype::PirateHeavy),
    ("pirate_heavy_on_shark", EnemyArchetype::PirateHeavyOnShark),
    ("exploding_mite", EnemyArchetype::ExplodingMite),
    ("dividing_mite", EnemyArchetype::DividingMite),
    ("ranged_skirmisher", EnemyArchetype::RangedSkirmisher),
];

/// Authored tuning row for one [`EnemyArchetype`]. Every archetype is
/// fully specified in [`ARCHETYPE_SPECS`]; the small accessor methods
/// on [`EnemyArchetype`] (`max_health`, `patrol_speed`, ...) all read
/// from this row.
///
/// Adding a new archetype is one new entry in the table plus one new
/// `Custom("…")` arm in [`EnemyArchetype::from_brain`] — no more
/// hunting through ten parallel `match` blocks.
///
/// Behavior fields (`brain_template`, `attack`, `move_style`) collapse
/// what used to be three independent per-archetype matches
/// (`enemy_default_brain`, `enemy_default_action_set`, and the
/// peaceful-vs-hostile move-style branches) into one source of truth.
/// Any new archetype now defines its full behavior shape in a single
/// `archetype_spec` arm.
/// Tuning row for one enemy archetype. The Rust enum
/// [`EnemyArchetype`] stays as the closed set of "known" archetypes
/// the codebase ships, but every field here is authored in
/// `assets/data/enemy_archetypes.ron` so a designer can tweak the
/// chase / aggro / damage numbers (the things that decide whether a
/// fight feels fair) without a Rust patch.
///
/// The `melee` / `ranged` fields carry the FULL attack spec — phase
/// timings, damage, reach — so a designer who wants to make the
/// Brute's lunge slower has one place to look. `damage_amount` is
/// the BODY-CONTACT damage; attack damage lives in the spec.
#[derive(Clone, Debug, serde::Deserialize)]
pub(super) struct EnemyArchetypeSpec {
    pub max_health: i32,
    #[serde(default)]
    pub rider_max_health: Option<i32>,
    pub patrol_speed: f32,
    pub chase_speed: f32,
    pub aggro_radius: f32,
    pub attack_range: f32,
    pub contact_strength: f32,
    pub damage_amount: i32,
    #[serde(default)]
    pub is_aerial: bool,
    #[serde(default)]
    pub is_sandbag: bool,
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
    /// Locomotion style for the actor's `ActionSet.move_style`.
    pub move_style: crate::brain::MoveStyleSpec,
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

/// Brain template choice keyed off `EnemyArchetype`. Sandbox-side
/// enum because the brain module is the universal-actor abstraction
/// and shouldn't know about enemies.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub(super) enum EnemyBrainTemplate {
    /// No motion / no AI — the actor only reacts to events
    /// (sandbag's PunchWeak counter, dialogue-only NPCs that become
    /// hostile).
    StandStill,
    /// Surface-walking idle wanderer. Used by the puppy slug.
    Wanderer,
    /// Approach-then-strike melee policy. The default for almost
    /// every hostile archetype — variety comes from per-archetype
    /// chase_speed / attack_range / aggro_radius pulled into the
    /// cfg.
    MeleeBrute,
    /// Strafe-and-fire ranged policy. Maintains a standoff distance
    /// from the target and emits `frame.fire` on a fixed cooldown.
    /// Use for archetypes that should harass with projectiles —
    /// shark-riders are the canonical case (aerial body + Bolt
    /// ranged capability). Pairs with a `ranged: Some(...)` row in
    /// the archetype data; without it the resolver swallows the fire
    /// intent.
    Skirmisher,
    /// Hold position + long-range fire. Like `Skirmisher` but does
    /// not strafe — used by stationary turret-like enemies.
    Sniper,
    /// Dedicated shark motion policy. Drives the riderless burning
    /// shark's charge-and-crash behavior without changing the
    /// mounted shark+pirate composite, which keeps its own mount
    /// brain.
    Shark,
    /// Smash-brawl pipeline: observe → mode → action → difficulty
    /// → emit. See `crate::brain::smash`. Use for humanoid melee
    /// archetypes that should approach, swing, and step back with
    /// crowding awareness.
    Smash,
}

/// Per-archetype tuning rows live in `assets/data/enemy_archetypes.ron`
/// (loaded once at startup via the `LazyLock` below). Designers edit
/// that file to tune fights — no Rust patch needed. The enum stays
/// as the closed "known archetypes" set so the compiler still flags
/// missing variants in `match` arms across the codebase.
fn archetype_spec(arch: EnemyArchetype) -> EnemyArchetypeSpec {
    let key = archetype_data_key(arch);
    ENEMY_ARCHETYPE_REGISTRY
        .get(key)
        .unwrap_or_else(|| {
            panic!("enemy archetype {arch:?} (RON key '{key}') missing from enemy_archetypes.ron")
        })
        .clone()
}

/// Stable RON key for an `EnemyArchetype` variant. Matches the
/// Rust variant name exactly so adding a variant is a one-line
/// change on both sides. Kept as an explicit match (not Debug /
/// derive) so the contract with the data file is searchable.
fn archetype_data_key(arch: EnemyArchetype) -> &'static str {
    use EnemyArchetype::*;
    match arch {
        Combatant => "Combatant",
        SmallSkitter => "SmallSkitter",
        SmallLurker => "SmallLurker",
        MediumStriker => "MediumStriker",
        LargeBrute => "LargeBrute",
        LargeColossus => "LargeColossus",
        AggressiveSeeker => "AggressiveSeeker",
        InfiniteSandbag => "InfiniteSandbag",
        FiniteSandbag => "FiniteSandbag",
        PirateRaider => "PirateRaider",
        BurningFlyingShark => "BurningFlyingShark",
        PirateOnShark => "PirateOnShark",
        PirateHeavy => "PirateHeavy",
        PirateHeavyOnShark => "PirateHeavyOnShark",
        PuppySlug => "PuppySlug",
        ExplodingMite => "ExplodingMite",
        DividingMite => "DividingMite",
        RangedSkirmisher => "RangedSkirmisher",
    }
}

/// Parsed contents of `assets/data/enemy_archetypes.ron`. `LazyLock`
/// (not a Bevy `Resource`) so `archetype_spec()` can stay a plain
/// function callable from non-system contexts (e.g.
/// `BossBehaviorProfile` constructors). Hot reload is a future-work
/// item — for now the data is read once when first accessed.
///
/// Keyed by `String` rather than `EnemyArchetype` to dodge serde's
/// enum-key-in-HashMap quirks; the variant ↔ string round-trip lives
/// in `archetype_data_key`.
static ENEMY_ARCHETYPE_REGISTRY: std::sync::LazyLock<
    std::collections::HashMap<String, EnemyArchetypeSpec>,
> = std::sync::LazyLock::new(|| {
    const ENEMY_ARCHETYPES_RON: &str = include_str!("../../../assets/data/enemy_archetypes.ron");
    ron::from_str(ENEMY_ARCHETYPES_RON).unwrap_or_else(|err| {
        panic!(
            "assets/data/enemy_archetypes.ron failed to deserialize as HashMap<String, \
             EnemyArchetypeSpec>: {err}"
        )
    })
});

impl EnemyArchetype {
    /// All combat-capable archetypes in a stable order. Useful for
    /// tests / tooling that want to iterate every variant; the
    /// sandbag training dummies are *not* in this list because they
    /// don't run the standard combat AI loop.
    pub const COMBAT_ALL: [Self; 13] = [
        Self::Combatant,
        Self::SmallSkitter,
        Self::SmallLurker,
        Self::MediumStriker,
        Self::LargeBrute,
        Self::LargeColossus,
        Self::AggressiveSeeker,
        Self::PirateRaider,
        Self::BurningFlyingShark,
        Self::PirateOnShark,
        Self::PuppySlug,
        Self::PirateHeavy,
        Self::PirateHeavyOnShark,
    ];

    pub fn from_brain(brain: &crate::actor::EnemyBrain) -> Self {
        let crate::actor::EnemyBrain::Custom(name) = brain else {
            return Self::Combatant;
        };
        BRAIN_NAME_TO_ARCHETYPE
            .iter()
            .find(|(key, _)| *key == name.as_str())
            .map(|(_, archetype)| *archetype)
            .unwrap_or(Self::Combatant)
    }

    /// Tuning row for this archetype.
    #[inline]
    pub(super) fn spec(self) -> EnemyArchetypeSpec {
        archetype_spec(self)
    }

    /// True for archetypes that ignore gravity. Drives the
    /// `gravity_scale` field on `EnemyRuntime`.
    pub(super) fn is_aerial(self) -> bool {
        self.spec().is_aerial
    }

    /// Brain template the spawn site instantiates for this archetype.
    /// See [`EnemyBrainTemplate`].
    pub(super) fn brain_template(self) -> EnemyBrainTemplate {
        self.spec().brain_template
    }

    /// Concrete melee spec this archetype's `ActionSet` carries at
    /// spawn. `None` = no melee capability (peaceful patrollers).
    pub(super) fn melee_spec(self) -> Option<crate::brain::MeleeActionSpec> {
        self.spec().melee
    }

    /// Concrete ranged spec this archetype's `ActionSet` carries at
    /// spawn. `None` = no ranged capability.
    pub(super) fn ranged_spec(self) -> Option<crate::brain::RangedActionSpec> {
        self.spec().ranged
    }

    /// Authored held item, if any. This is separate from the actor's default
    /// hostility: a peaceful NPC can carry a weapon that becomes active only
    /// if another system provokes them.
    pub(super) fn held_item_spec(self) -> Option<crate::brain::HeldItemSpec> {
        self.spec()
            .held_item
            .as_deref()
            .and_then(crate::brain::held_item_by_id)
    }

    /// Locomotion style for this archetype's `ActionSet.move_style`.
    pub(super) fn move_style(self) -> crate::brain::MoveStyleSpec {
        self.spec().move_style
    }

    /// Authored Smash-brain melee hit band, if any. `None` falls back to
    /// the Smash config builder's default (see `smash_cfg_for_archetype`).
    pub(super) fn smash_hit_band(self) -> Option<f32> {
        self.spec().smash_hit_band
    }

    /// Slot kind this archetype requests from the combat slot board.
    /// Used by the per-frame slot allocator.
    pub(super) fn slot_kind(self) -> crate::combat::slots::SlotKind {
        if self.is_aerial() {
            crate::combat::slots::SlotKind::Aerial
        } else {
            crate::combat::slots::SlotKind::Melee
        }
    }

    pub(crate) fn is_sandbag(self) -> bool {
        self.spec().is_sandbag
    }

    pub(super) fn max_health(self) -> i32 {
        self.spec().max_health
    }

    /// Extra HP pool for actors that have a "rider" on top — today
    /// only `PirateOnShark`. `None` for every other archetype.
    pub(super) fn rider_max_health(self) -> Option<i32> {
        self.spec().rider_max_health
    }

    pub(super) fn patrol_speed(self) -> f32 {
        self.spec().patrol_speed
    }

    pub(super) fn chase_speed(self) -> f32 {
        self.spec().chase_speed
    }

    pub(super) fn aggro_radius(self) -> f32 {
        self.spec().aggro_radius
    }

    pub(super) fn attack_range(self) -> f32 {
        self.spec().attack_range
    }

    pub(super) fn contact_strength(self) -> f32 {
        self.spec().contact_strength
    }

    pub(super) fn damage_amount(self) -> i32 {
        self.spec().damage_amount
    }

    /// Body size (px) for actors of this archetype. Aerial actors
    /// are larger because the shark sprite is 192×128.
    pub fn default_size(self) -> Option<ae::Vec2> {
        self.spec().default_size
    }

    /// True when this archetype is hostile by default — actively tracks
    /// the player and publishes contact damage. False for "peaceful patrol"
    /// archetypes (PuppySlug, PirateHeavy) that exist as ambient threats /
    /// cove crew rather than active combatants. A peaceful-by-default row may
    /// still carry dormant attack data so a separate explicit-hostile path can
    /// provoke it without changing its authored identity.
    pub fn attacks_player(self) -> bool {
        use EnemyArchetype::*;
        !matches!(self, PuppySlug | PirateHeavy)
    }

    /// True when the archetype should publish a body-contact hazard
    /// on touch. Sandbags and composite shark riders opt out; the
    /// peaceful cove crew do not emit touch damage unless a future
    /// mode explicitly opts them back in.
    pub fn body_contact_damage_enabled(self) -> bool {
        use EnemyArchetype::*;
        !matches!(
            self,
            InfiniteSandbag | FiniteSandbag | PirateOnShark | PirateHeavyOnShark
        ) && (self.attacks_player() || self == PuppySlug)
    }

    /// Default respawn cadence for this archetype. Grunts refresh
    /// every visit (OnRoomReenter); heavier mini-boss-tier presences
    /// (Heavies, Brutes, Colossi, sharks-with-rider) take a Rest to
    /// come back. Sandbags handle their own respawn loop in `update`
    /// and report OnRoomReenter as a stable default that the kill
    /// hook ignores anyway.
    pub fn respawn_policy(self) -> EnemyRespawnPolicy {
        use EnemyArchetype::*;
        match self {
            Combatant | SmallSkitter | SmallLurker | MediumStriker | AggressiveSeeker
            | PuppySlug | PirateRaider | BurningFlyingShark | InfiniteSandbag | FiniteSandbag
            | ExplodingMite | DividingMite | RangedSkirmisher => EnemyRespawnPolicy::OnRoomReenter,
            LargeBrute | LargeColossus | PirateHeavy | PirateOnShark | PirateHeavyOnShark => {
                EnemyRespawnPolicy::OnRest
            }
        }
    }
}

/// Cluster-native enemy integration. This is the EnemyRuntime::update
/// physics/AI port, operating directly on the authoritative ECS
/// components through the [`EnemyMut`] view (player cluster pattern).
/// Field map: self.kin.* (pos/vel/size/facing), self.status.* (alive/
/// respawn_timer/hit_flash/ai_mode/health), self.config.* (archetype/
/// brain/spawn), self.attack.* / self.surface.* unchanged, self.motion.0.
impl<'a> EnemyMut<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: FeatureCombatTuning,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
        _is_mounted: bool,
        frame: crate::actor::control::ActorControlFrame,
        // Sign of world gravity (+1 down / -1 up) from `GravityField`, so the
        // enemy falls + jumps the way the player does when gravity flips.
        gravity_sign: f32,
    ) -> crate::actor::control::ActorControlFrame {
        self.status.hit_flash = (self.status.hit_flash - dt).max(0.0);
        if !self.status.alive {
            self.status.respawn_timer = (self.status.respawn_timer - dt).max(0.0);
            if self.config.archetype == EnemyArchetype::FiniteSandbag
                && self.status.respawn_timer <= 0.0
            {
                self.status.alive = true;
                self.status.health.reset();
                self.kin.pos = self.config.spawn.pos;
                self.kin.vel = ae::Vec2::ZERO;
                self.status.hit_flash = 0.24;
            }
            self.status.ai_mode = crate::actor::ai::CharacterAiMode::Dead;
            return crate::actor::control::ActorControlFrame::neutral();
        }

        self.attack.tick(dt, tuning.enemy_attack_active);

        let recover_remaining = if self.attack.cooldown > 0.0
            && self.attack.windup_timer <= 0.0
            && self.attack.active_timer <= 0.0
        {
            self.attack.cooldown.min(0.30)
        } else {
            0.0
        };
        let effective_aggro_radius = match &self.config.brain {
            crate::actor::EnemyBrain::Passive => 0.0,
            crate::actor::EnemyBrain::Guard { leash_radius } => *leash_radius,
            _ => self.config.archetype.aggro_radius(),
        };
        let ai =
            crate::actor::ai::evaluate_character_ai_output(crate::actor::ai::CharacterAiSnapshot {
                actor_pos: self.kin.pos,
                player_pos: target_pos,
                aggro_radius: effective_aggro_radius,
                attack_range: self.config.archetype.attack_range(),
                attack_windup_remaining: self.attack.windup_timer,
                attack_active_remaining: self.attack.active_timer,
                attack_recover_remaining: recover_remaining,
                stun_remaining: 0.0,
                alive: self.status.alive,
                patrol_enabled: !self.config.archetype.is_sandbag()
                    && !matches!(self.config.brain, crate::actor::EnemyBrain::Passive),
            });
        self.status.ai_mode = ai.mode;

        let is_aerial = self.surface.gravity_scale <= 0.001;
        let is_surface_walker = self.config.archetype == EnemyArchetype::PuppySlug;

        if is_surface_walker {
            self.step_surface_walker(world, nearest_neighbor, dt);
        } else {
            let max_fall = ENEMY_MAX_FALL;
            let gravity = if is_aerial {
                0.0
            } else {
                ENEMY_GRAVITY * self.surface.gravity_scale
            };
            let mut body = crate::kinematic::KinematicBody {
                pos: self.kin.pos,
                vel: self.kin.vel,
                size: self.kin.size,
                on_ground: self.surface.on_ground,
                facing: self.kin.facing,
            };
            let prev_vel_x = body.vel.x;
            if is_aerial {
                let target_speed = frame.desired_vel.length();
                let archetype_chase = self.config.archetype.chase_speed();
                let accel = (target_speed.max(archetype_chase) * 3.0).max(900.0) * dt;
                body.vel.x = approach(body.vel.x, frame.desired_vel.x, accel);
                body.vel.y = approach(body.vel.y, frame.desired_vel.y, accel);
            } else {
                body.vel.x = approach(body.vel.x, frame.desired_vel.x, 650.0 * dt);
                if frame.jump_pressed {
                    // Jumps oppose gravity, so they flip with it.
                    if body.on_ground {
                        body.vel.y = -ENEMY_JUMP_SPEED * gravity_sign;
                        body.on_ground = false;
                    } else if self.surface.air_jumps_remaining > 0 {
                        body.vel.y = -ENEMY_DOUBLE_JUMP_SPEED * gravity_sign;
                        self.surface.air_jumps_remaining -= 1;
                    }
                }
            }
            crate::kinematic::step_kinematic(
                &mut body,
                world,
                crate::kinematic::KinematicTuning {
                    gravity,
                    max_fall_speed: max_fall,
                    // Falls the same way the player does when gravity flips.
                    gravity_sign,
                },
                crate::kinematic::KinematicInputs {
                    drop_through: frame.drop_through,
                },
                dt,
            );
            self.kin.pos = body.pos;
            self.kin.vel = body.vel;
            self.surface.on_ground = if is_aerial { false } else { body.on_ground };
            if self.surface.on_ground {
                self.surface.air_jumps_remaining = MAX_ENEMY_AIR_JUMPS;
            }

            if let Some(motion) = &mut self.motion.0 {
                let _ = motion.advance(self.kin.pos, dt);
            }

            if !is_aerial
                && matches!(ai.intent, crate::actor::ai::CharacterAiIntent::Patrol)
                && prev_vel_x.abs() > 1.0
                && self.kin.vel.x.abs() < 0.01
            {
                self.kin.facing *= -1.0;
            }
        }

        if self.config.archetype.attacks_player() && frame.facing.abs() > 0.001 {
            self.kin.facing = frame.facing.signum();
        }

        if frame.fire.is_some() {
            self.status.ai_mode = crate::actor::ai::CharacterAiMode::Attack;
        }
        frame
    }

    fn step_surface_walker(
        &mut self,
        world: &ae::World,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
    ) {
        if !self.surface.on_ground {
            self.fall_until_landed(world, dt);
            return;
        }

        let n = self.surface.surface_normal;
        let speed = self.config.archetype.patrol_speed();
        let step_len = speed * dt;
        let tangent = ae::Vec2::new(-n.y * self.kin.facing, n.x * self.kin.facing);
        let body_long = self.kin.size.x * 0.5;
        let body_thick = self.kin.size.y * 0.5;

        if let Some(neighbor_pos) = nearest_neighbor {
            let delta = neighbor_pos - self.kin.pos;
            let along = delta.x * tangent.x + delta.y * tangent.y;
            let perp = delta.x * n.x + delta.y * n.y;
            if along > 0.0 && along < body_long + 6.0 && perp.abs() < body_thick + 4.0 {
                self.kin.facing = -self.kin.facing;
                self.kin.vel = ae::Vec2::ZERO;
                return;
            }
        }

        if self.wall_ahead(world, tangent, body_long, body_thick) {
            self.surface.surface_normal = -tangent;
            if self.snap_pos_to_surface(world) {
                self.kin.vel = ae::Vec2::ZERO;
                self.surface.on_ground = true;
                return;
            }
            self.surface.surface_normal = n;
        }

        let original_pos = self.kin.pos;
        self.kin.pos += tangent * step_len;
        self.kin.vel = tangent * speed;

        if self.snap_pos_to_surface(world) {
            self.surface.on_ground = true;
            return;
        }

        let new_normal = tangent;
        let around_corner = original_pos + tangent * body_long + (-n) * body_long;
        self.kin.pos = around_corner;
        self.surface.surface_normal = new_normal;
        if self.snap_pos_to_surface(world) {
            self.kin.vel = ae::Vec2::ZERO;
            self.surface.on_ground = true;
            return;
        }

        self.kin.pos = original_pos;
        self.surface.surface_normal = -tangent;
        if self.snap_pos_to_surface(world) {
            self.kin.vel = ae::Vec2::ZERO;
            self.surface.on_ground = true;
            return;
        }

        self.surface.surface_normal = n;
        self.kin.pos = original_pos;
        self.surface.on_ground = false;
        self.fall_until_landed(world, dt);
    }

    fn wall_ahead(
        &self,
        world: &ae::World,
        tangent: ae::Vec2,
        body_long: f32,
        body_thick: f32,
    ) -> bool {
        let probe_center = self.kin.pos + tangent * (body_long + 3.0);
        let half = if tangent.x.abs() > 0.5 {
            ae::Vec2::new(2.0, body_thick * 0.7)
        } else {
            ae::Vec2::new(body_thick * 0.7, 2.0)
        };
        let probe = ae::Aabb::new(probe_center, half);
        world.body_overlaps_any(probe, surface_wall_pred)
    }

    fn snap_pos_to_surface(&mut self, world: &ae::World) -> bool {
        let n = self.surface.surface_normal;
        let body_thick = self.kin.size.y * 0.5;
        let body_long = self.kin.size.x * 0.5;
        let down = -n;
        let max_d = (body_thick + body_long + 4.0) as i32;
        let half = if n.x.abs() > 0.5 {
            ae::Vec2::new(0.75, body_long * 0.35)
        } else {
            ae::Vec2::new(body_long * 0.35, 0.75)
        };
        for i in 0..=max_d {
            let d = i as f32;
            let probe = ae::Aabb::new(self.kin.pos + down * d, half);
            if world.body_overlaps_any(probe, surface_solid_pred) {
                self.kin.pos += n * (body_thick - (d - 0.5));
                return true;
            }
        }
        false
    }

    fn fall_until_landed(&mut self, world: &ae::World, dt: f32) {
        let mut body = crate::kinematic::KinematicBody {
            pos: self.kin.pos,
            vel: self.kin.vel,
            size: self.kin.size,
            on_ground: self.surface.on_ground,
            facing: self.kin.facing,
        };
        crate::kinematic::step_kinematic(
            &mut body,
            world,
            crate::kinematic::KinematicTuning {
                gravity: ENEMY_GRAVITY,
                max_fall_speed: ENEMY_MAX_FALL,
                // Spawn-time snap-to-ground assumes normal gravity.
                gravity_sign: 1.0,
            },
            crate::kinematic::KinematicInputs {
                drop_through: false,
            },
            dt,
        );
        self.kin.pos = body.pos;
        self.kin.vel = body.vel;
        self.surface.on_ground = body.on_ground;
        if body.on_ground {
            self.surface.surface_normal = ae::Vec2::new(0.0, -1.0);
        }
    }

    // ---- Consumer-facing geometry / combat helpers (ports of the
    // matching EnemyRuntime methods, reading the cluster components).

    pub fn aabb(&self) -> ae::Aabb {
        let size = if self.config.archetype == EnemyArchetype::PuppySlug
            && self.surface.surface_normal.x.abs() > 0.5
        {
            ae::Vec2::new(self.kin.size.y, self.kin.size.x)
        } else {
            self.kin.size
        };
        ae::Aabb::new(self.kin.pos, size * 0.5)
    }

    pub fn rotation_rad(&self) -> f32 {
        f32::atan2(
            -self.surface.surface_normal.x,
            -self.surface.surface_normal.y,
        )
    }

    pub fn visual_kind(&self) -> FeatureVisualKind {
        if self.config.archetype.is_sandbag() {
            FeatureVisualKind::Sandbag
        } else {
            FeatureVisualKind::Enemy
        }
    }

    pub fn bark_anchor(&self) -> ae::Vec2 {
        self.kin.pos + ae::Vec2::new(0.0, -self.kin.size.y * 0.72 - 16.0)
    }

    pub fn attack_aabb(&self) -> ae::Aabb {
        ae::Aabb::new(
            self.kin.pos + ae::Vec2::new(self.kin.facing * (self.kin.size.x * 0.55 + 24.0), -4.0),
            ae::Vec2::new(34.0, 28.0),
        )
    }

    pub fn attack_telegraph_aabb(&self) -> ae::Aabb {
        self.attack_aabb()
    }

    pub fn attack_aabb_dir(&self, axis: ae::Vec2) -> ae::Aabb {
        let horizontal = axis.x.abs() >= axis.y.abs();
        if horizontal {
            let side = if axis.x.abs() > 0.1 {
                axis.x.signum()
            } else {
                self.kin.facing
            };
            let center = self.kin.pos + ae::Vec2::new(side * (self.kin.size.x * 0.55 + 24.0), -4.0);
            return ae::Aabb::new(center, ae::Vec2::new(34.0, 28.0));
        }
        if axis.y < 0.0 {
            let half = ae::Vec2::new(16.0, 36.0);
            let center = self.kin.pos + ae::Vec2::new(0.0, -(self.kin.size.y * 0.5 + half.y + 4.0));
            return ae::Aabb::new(center, half);
        }
        let half = ae::Vec2::new(36.0, 20.0);
        let center = self.kin.pos + ae::Vec2::new(0.0, self.kin.size.y * 0.5 + half.y - 2.0);
        ae::Aabb::new(center, half)
    }

    pub fn begin_melee_attack(
        &mut self,
        tuning: FeatureCombatTuning,
        attack_axis: ae::Vec2,
    ) -> bool {
        if self.attack.cooldown > 0.0 || !self.status.alive {
            return false;
        }
        self.attack.windup_timer = tuning.enemy_attack_windup.max(0.01);
        self.attack.cooldown = ENEMY_ATTACK_COOLDOWN
            * if self.config.archetype == EnemyArchetype::SmallSkitter {
                0.75
            } else if self.config.archetype == EnemyArchetype::LargeBrute {
                1.35
            } else {
                1.0
            };
        self.status.ai_mode = crate::actor::ai::CharacterAiMode::Telegraph;
        self.attack.pending_axis = if attack_axis.length_squared() > 0.01 {
            attack_axis.normalize_or_zero()
        } else {
            ae::Vec2::new(self.kin.facing, 0.0)
        };
        true
    }

    pub fn body_damage_aabb(&self) -> Option<ae::Aabb> {
        if !self.config.archetype.body_contact_damage_enabled() {
            return None;
        }
        Some(self.aabb())
    }

    pub fn body_contact_damage(
        &self,
        player_entity: bevy::prelude::Entity,
        player_body: ae::Aabb,
    ) -> Option<HitEvent> {
        let body_damage = self.body_damage_aabb()?;
        if !body_damage.strict_intersects(player_body) {
            return None;
        }
        let impact = midpoint(player_body.center(), body_damage.center());
        Some(HitEvent {
            volume: body_damage,
            damage: self.config.archetype.damage_amount(),
            source: HitSource::EnemyBody,
            attacker: None,
            target: HitTarget::Player(player_entity),
            mode: HitMode::Knockback,
            knockback: Some(HitKnockback {
                dir: (player_body.center().x - self.kin.pos.x).signum_or(self.kin.facing),
                strength: self.config.archetype.contact_strength(),
                source_pos: self.kin.pos,
                impact_pos: impact,
            }),
            ignored_targets: Vec::new(),
        })
    }

    pub fn reset_to_spawn(&mut self) {
        let archetype = self.config.spawn.archetype;
        self.config.archetype = archetype;
        self.kin.size = self.config.spawn.size;
        self.kin.pos = self.config.spawn.pos;
        self.kin.vel = ae::Vec2::ZERO;
        self.status.alive = true;
        self.status.health = crate::actor::Health::new(archetype.max_health());
        *self.attack = ActorAttackState::default();
        self.status.respawn_timer = 0.0;
        self.status.hit_flash = 0.0;
        self.status.ai_mode = crate::actor::ai::CharacterAiMode::Idle;
        self.kin.facing = -1.0;
        *self.surface = ActorSurfaceState {
            on_ground: false,
            surface_normal: ae::Vec2::new(0.0, -1.0),
            gravity_scale: if archetype.is_aerial() { 0.0 } else { 1.0 },
            air_jumps_remaining: MAX_ENEMY_AIR_JUMPS,
        };
    }
}

#[cfg(test)]
mod enemy_archetype_data_tests {
    use super::*;

    /// `assets/data/enemy_archetypes.ron` must carry a row for every
    /// `EnemyArchetype` variant the codebase knows about — otherwise
    /// `archetype_spec()` would panic at the first spawn of the
    /// missing archetype. Pin every enum variant the engine ships.
    #[test]
    fn ron_carries_every_known_archetype() {
        use EnemyArchetype::*;
        for arch in [
            Combatant,
            SmallSkitter,
            SmallLurker,
            MediumStriker,
            LargeBrute,
            LargeColossus,
            AggressiveSeeker,
            InfiniteSandbag,
            FiniteSandbag,
            PirateRaider,
            BurningFlyingShark,
            PirateOnShark,
            PirateHeavy,
            PirateHeavyOnShark,
            PuppySlug,
        ] {
            let key = archetype_data_key(arch);
            assert!(
                ENEMY_ARCHETYPE_REGISTRY.contains_key(key),
                "enemy_archetypes.ron missing row for {arch:?} (key '{key}')",
            );
        }
    }

    /// Spot-check the legacy pre-data values for two divergent
    /// archetypes so a regen of the RON without re-tuning catches
    /// accidental drift on the rows the player notices first.
    #[test]
    fn legacy_baseline_pins() {
        use crate::brain::MeleeActionSpec;
        let combatant = archetype_spec(EnemyArchetype::Combatant);
        assert_eq!(combatant.max_health, 4);
        assert!((combatant.chase_speed - 155.0).abs() < f32::EPSILON);
        assert!((combatant.aggro_radius - 460.0).abs() < f32::EPSILON);
        assert!(
            matches!(combatant.melee, Some(MeleeActionSpec::Swipe(_))),
            "Combatant melee should be Swipe; got {:?}",
            combatant.melee
        );
        let slug = archetype_spec(EnemyArchetype::PuppySlug);
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
        let on_shark = EnemyArchetype::PirateOnShark
            .held_item_spec()
            .expect("PirateOnShark should resolve a held item");
        assert_eq!(on_shark.id, "gun_sword");
        assert!(matches!(
            on_shark.ranged,
            Some(RangedActionSpec::Bolt { damage: 2, .. })
        ));
        let heavy = EnemyArchetype::PirateHeavyOnShark
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
        assert_eq!(EnemyArchetype::MediumStriker.smash_hit_band(), Some(32.0));
        assert_eq!(EnemyArchetype::SmallSkitter.smash_hit_band(), Some(32.0));
        assert_eq!(EnemyArchetype::SmallLurker.smash_hit_band(), Some(32.0));
        assert_eq!(EnemyArchetype::LargeBrute.smash_hit_band(), Some(48.0));
        assert_eq!(EnemyArchetype::LargeColossus.smash_hit_band(), Some(48.0));
        // 36px-default Smash archetypes omit the field on purpose.
        assert_eq!(EnemyArchetype::Combatant.smash_hit_band(), None);
        assert_eq!(EnemyArchetype::AggressiveSeeker.smash_hit_band(), None);
        assert_eq!(EnemyArchetype::PirateRaider.smash_hit_band(), None);
    }

    #[test]
    fn body_contact_damage_is_explicitly_opted_in() {
        assert!(EnemyArchetype::Combatant.body_contact_damage_enabled());
        assert!(EnemyArchetype::PuppySlug.body_contact_damage_enabled());
        assert!(!EnemyArchetype::PirateHeavy.body_contact_damage_enabled());
        assert!(!EnemyArchetype::PirateOnShark.body_contact_damage_enabled());
        assert!(!EnemyArchetype::FiniteSandbag.body_contact_damage_enabled());
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
        let mut enemy = crate::content::features::ecs::enemy_clusters::EnemyClusterScratch::new(
            "pirate_heavy_reach_probe",
            "Broadside Bess",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(36.0, 55.0)),
            crate::actor::EnemyBrain::Custom("pirate_heavy".into()),
            &[],
        );
        enemy.kin.facing = 1.0;
        assert_eq!(enemy.config.archetype, EnemyArchetype::PirateHeavy);
        let hitbox = enemy.as_mut().attack_aabb_dir(ae::Vec2::new(1.0, 0.0));
        let reach_edge = hitbox.center().x + hitbox.half_size().x - enemy.kin.pos.x;
        let attack_range = enemy.config.archetype.attack_range();
        assert!(
            attack_range <= reach_edge,
            "PirateHeavy attack_range {attack_range} must stay within her swing far \
             edge {reach_edge} so she stops inside her own reach instead of whiffing",
        );
    }
}
