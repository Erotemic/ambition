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

#[derive(Clone, Debug)]
pub struct EnemyRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub vel: ae::Vec2,
    pub health: crate::actor::Health,
    pub brain: crate::actor::EnemyBrain,
    pub archetype: EnemyArchetype,
    /// Authored spawn baseline (pos / archetype / size) that
    /// `reset_to_spawn` restores. See [`ActorSpawnState`].
    pub spawn: ActorSpawnState,
    pub motion: Option<PathMotion>,
    pub alive: bool,
    pub facing: f32,
    /// Melee attack timing + aim (windup / active / cooldown / axis),
    /// grouped into one coherent unit. See [`ActorAttackState`].
    pub attack: ActorAttackState,
    pub respawn_timer: f32,
    pub hit_flash: f32,
    /// Last-evaluated `CharacterAiMode`. Updated by `update`. Read by
    /// HUD / rendering / debug overlay so they can branch on a single
    /// vocabulary instead of inferring it from the timer fields.
    pub ai_mode: crate::character_ai::CharacterAiMode,
    /// When this enemy was spawned by migrating a hostile NPC, the
    /// LDtk display name of the original NPC. The sprite resolver
    /// passes this through `npc_asset_for_name` so faction NPCs that
    /// turn hostile keep their own sheet (and their own slash / hit
    /// rows) instead of being re-skinned as a goblin. Only the Kernel
    /// Guide NPC has the dedicated "transforms into a goblin" beat —
    /// every other hostile NPC stays themselves and uses their own
    /// attack animations. `None` means "use the default `Enemy` sprite
    /// (currently `goblin_spritesheet`)".
    pub sprite_override_npc_name: Option<String>,
    /// Locomotion contact + vertical-control state (ground / surface
    /// normal / gravity scale / air jumps). See [`ActorSurfaceState`].
    pub surface: ActorSurfaceState,
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
    /// Optional authored held item. Its abilities overlay the archetype action
    /// set at spawn / state transitions so weapons, not ad-hoc Rust branches,
    /// own whether an actor can melee or fire.
    #[serde(default)]
    pub held_item: Option<crate::brain::HeldItemSpec>,
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
        self.spec().held_item
    }

    /// Locomotion style for this archetype's `ActionSet.move_style`.
    pub(super) fn move_style(self) -> crate::brain::MoveStyleSpec {
        self.spec().move_style
    }

    /// Slot kind this archetype requests from the combat slot board.
    /// Used by the per-frame slot allocator.
    pub(super) fn slot_kind(self) -> crate::combat_slots::SlotKind {
        if self.is_aerial() {
            crate::combat_slots::SlotKind::Aerial
        } else {
            crate::combat_slots::SlotKind::Melee
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
            | PuppySlug | PirateRaider | BurningFlyingShark | InfiniteSandbag | FiniteSandbag => {
                EnemyRespawnPolicy::OnRoomReenter
            }
            LargeBrute | LargeColossus | PirateHeavy | PirateOnShark | PirateHeavyOnShark => {
                EnemyRespawnPolicy::OnRest
            }
        }
    }
}

impl EnemyRuntime {
    pub(super) fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: crate::actor::EnemyBrain,
        paths: &[(String, crate::actor::KinematicPath)],
    ) -> Self {
        let archetype = EnemyArchetype::from_brain(&brain);
        let motion = match &brain {
            crate::actor::EnemyBrain::Patrol {
                path_id: Some(path_id),
            } if !archetype.is_sandbag() => paths
                .iter()
                .find(|(p_id, _)| p_id == path_id)
                .map(|(_, path)| PathMotion::new(path.clone())),
            _ => None,
        };
        let pos = motion
            .as_ref()
            .and_then(PathMotion::start_pos)
            .unwrap_or_else(|| aabb.center());
        let size = archetype
            .default_size()
            .unwrap_or_else(|| aabb.half_size() * 2.0);
        Self {
            id: id.into(),
            name: name.into(),
            pos,
            size,
            vel: ae::Vec2::ZERO,
            health: crate::actor::Health::new(archetype.max_health()),
            brain,
            archetype,
            spawn: ActorSpawnState {
                pos,
                archetype,
                size,
            },
            motion,
            alive: true,
            facing: -1.0,
            attack: ActorAttackState::default(),
            respawn_timer: 0.0,
            hit_flash: 0.0,
            ai_mode: crate::character_ai::CharacterAiMode::Idle,
            sprite_override_npc_name: None,
            surface: ActorSurfaceState {
                on_ground: false,
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale: if archetype.is_aerial() { 0.0 } else { 1.0 },
                air_jumps_remaining: MAX_ENEMY_AIR_JUMPS,
            },
        }
    }

    /// Restore the actor to its authored spawn state. Used by the
    /// same-room reset path so a PirateOnShark that morphed into
    /// PirateRaider or BurningFlyingShark mid-fight returns as the
    /// original fused actor; non-morphing enemies are reset to a
    /// clean baseline too (health full, timers zeroed, pos back at
    /// spawn). Callers must follow with an `aabb.half_size = size
    /// * 0.5` write on the ECS `FeatureAabb` component so the
    /// collision shape matches when the archetype changes its
    /// `default_size`.
    pub fn reset_to_spawn(&mut self) {
        let archetype = self.spawn.archetype;
        self.archetype = archetype;
        self.size = self.spawn.size;
        self.pos = self.spawn.pos;
        self.vel = ae::Vec2::ZERO;
        self.alive = true;
        self.health = crate::actor::Health::new(archetype.max_health());
        self.attack = ActorAttackState::default();
        self.respawn_timer = 0.0;
        self.hit_flash = 0.0;
        self.ai_mode = crate::character_ai::CharacterAiMode::Idle;
        self.facing = -1.0;
        // Surface walkers reset to floor — even if the slug died on
        // a wall or ceiling, respawn pins it on whatever platform
        // its `spawn` sits above is. The same-room reset path rewrites
        // the ECS `FeatureAabb` from `size`, so the unrotated floor
        // stance is correct again.
        self.surface = ActorSurfaceState {
            on_ground: false,
            surface_normal: ae::Vec2::new(0.0, -1.0),
            gravity_scale: if archetype.is_aerial() { 0.0 } else { 1.0 },
            air_jumps_remaining: MAX_ENEMY_AIR_JUMPS,
        };
    }

    /// World-space anchor for a combat-banter speech bubble. Sits
    /// just above the enemy's sprite top so the bubble doesn't
    /// occlude their silhouette. Mirrors `NpcRuntime::bark_anchor`.
    pub fn bark_anchor(&self) -> ae::Vec2 {
        self.pos + ae::Vec2::new(0.0, -self.size.y * 0.72 - 16.0)
    }

    /// `target_pos` is the per-frame "who is this enemy looking at"
    /// position, populated from the entity's `ActorTarget` component
    /// by `select_actor_targets` (OVERNIGHT-TODO #17.8). In a
    /// single-player build it's always the player's `pos`; a future
    /// co-op build varies it per-enemy without changing this
    /// function's shape.
    /// Tick the enemy: decrement attack-lifecycle timers, refresh
    /// the `ai_mode` HUD signal from `evaluate_character_ai_output`,
    /// and integrate one kinematic step from the brain-emitted
    /// `frame`. Returns the same `frame` so the caller can write
    /// it into the entity's `ActorControl` component for
    /// `emit_brain_action_messages` + the EFFECTS-stage consumers.
    ///
    /// `frame` is the brain's authoritative intent for this tick.
    /// Every brain-attached actor (Smash + every other state-machine
    /// variant) supplies one via actors.rs's per-actor brain tick;
    /// debug actors without a Brain pass `ActorControlFrame::neutral()`
    /// and stand still. The integration stage reads only the frame —
    /// the legacy `build_control_frame` path was deleted in the
    /// brain-authority GC pass.
    ///
    /// Still owned here (not the brain): the windup → active → recover
    /// attack lifecycle timers, the per-tick `ai_mode` HUD signal
    /// (computed by `evaluate_character_ai_output` and mirrored onto
    /// `ActorIntent`), and the kinematic integration step itself
    /// (gravity, ground contact, surface-walker crawl).
    pub(super) fn update(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: FeatureCombatTuning,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
        _is_mounted: bool,
        frame: crate::actor_control::ActorControlFrame,
    ) -> crate::actor_control::ActorControlFrame {
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        if !self.alive {
            self.respawn_timer = (self.respawn_timer - dt).max(0.0);
            if self.archetype == EnemyArchetype::FiniteSandbag && self.respawn_timer <= 0.0 {
                self.alive = true;
                self.health.reset();
                self.pos = self.spawn.pos;
                self.vel = ae::Vec2::ZERO;
                self.hit_flash = 0.24;
            }
            self.ai_mode = crate::character_ai::CharacterAiMode::Dead;
            return crate::actor_control::ActorControlFrame::neutral();
        }

        self.attack.tick(dt, tuning.enemy_attack_active);

        // `ai_mode` is the HUD / animation signal mirrored onto
        // `ActorIntent` by actors.rs. Compute it from the same
        // `evaluate_character_ai_output` table the legacy path used
        // so animation states (Telegraph, Recover, Patrol, etc.)
        // stay consistent; the *intent* fields from `ai` are no
        // longer consulted (the brain frame replaces them).
        let recover_remaining = if self.attack.cooldown > 0.0
            && self.attack.windup_timer <= 0.0
            && self.attack.active_timer <= 0.0
        {
            self.attack.cooldown.min(0.30)
        } else {
            0.0
        };
        let effective_aggro_radius = match &self.brain {
            crate::actor::EnemyBrain::Passive => 0.0,
            crate::actor::EnemyBrain::Guard { leash_radius } => *leash_radius,
            _ => self.archetype.aggro_radius(),
        };
        let ai = crate::character_ai::evaluate_character_ai_output(
            crate::character_ai::CharacterAiSnapshot {
                actor_pos: self.pos,
                player_pos: target_pos,
                aggro_radius: effective_aggro_radius,
                attack_range: self.archetype.attack_range(),
                attack_windup_remaining: self.attack.windup_timer,
                attack_active_remaining: self.attack.active_timer,
                attack_recover_remaining: recover_remaining,
                stun_remaining: 0.0,
                alive: self.alive,
                patrol_enabled: !self.archetype.is_sandbag()
                    && !matches!(self.brain, crate::actor::EnemyBrain::Passive),
            },
        );
        self.ai_mode = ai.mode;

        let is_aerial = self.surface.gravity_scale <= 0.001;
        let is_surface_walker = self.archetype == EnemyArchetype::PuppySlug;

        if is_surface_walker {
            // Surface-walker integration: the slug crawls along any
            // surface (floor / wall / ceiling), wrapping around
            // convex corners and turning into concave ones. Bypasses
            // `step_kinematic` entirely — that primitive bakes in
            // gravity-down + axis-aligned solid sweeps, which is
            // wrong on rotated surfaces. The custom step has its own
            // ledge / wall / fall handling. `nearest_neighbor` lets
            // two slugs that bump into each other reverse instead of
            // overlapping.
            self.step_surface_walker(world, nearest_neighbor, dt);
        } else {
            // INTEGRATION STAGE — every actor (aerial, grounded, patrol)
            // goes through `step_kinematic` against the live `world`.
            // This is the seam the player uses too: brain produces
            // desired velocity, the kinematic primitive resolves
            // collision. The previous codebase wrote `self.pos += ...`
            // directly for aerial + patrol movement, which meant flying
            // sharks could clip through walls and KinematicPath patrols
            // ignored solids entirely.
            let max_fall = ENEMY_MAX_FALL;
            let gravity = if is_aerial {
                0.0
            } else {
                ENEMY_GRAVITY * self.surface.gravity_scale
            };
            let mut body = crate::kinematic::KinematicBody {
                pos: self.pos,
                vel: self.vel,
                size: self.size,
                on_ground: self.surface.on_ground,
                facing: self.facing,
            };
            let prev_vel_x = body.vel.x;
            if is_aerial {
                // Aerial bodies own both vx and vy via the brain. We
                // approach the desired velocity (not snap to it) so a
                // dive-strike speed override accelerates believably
                // rather than teleporting velocity.
                let target_speed = frame.desired_vel.length();
                let archetype_chase = self.archetype.chase_speed();
                let accel = (target_speed.max(archetype_chase) * 3.0).max(900.0) * dt;
                body.vel.x = approach(body.vel.x, frame.desired_vel.x, accel);
                body.vel.y = approach(body.vel.y, frame.desired_vel.y, accel);
            } else {
                // Grounded bodies own vx via the brain; gravity owns vy.
                // Match the previous ground-acceleration constant
                // (650 px/s²·dt) so chase/patrol feel doesn't shift.
                body.vel.x = approach(body.vel.x, frame.desired_vel.x, 650.0 * dt);
                // Jump impulse:
                //   - Grounded: full `ENEMY_JUMP_SPEED` impulse.
                //   - Airborne with `air_jumps_remaining > 0`:
                //     smaller `ENEMY_DOUBLE_JUMP_SPEED` impulse,
                //     decrement counter. Matches the player's
                //     "double-jump is a smaller boost" feel.
                // Engine y grows downward → negative vy = upward.
                if frame.jump_pressed {
                    if body.on_ground {
                        body.vel.y = -ENEMY_JUMP_SPEED;
                        body.on_ground = false;
                    } else if self.surface.air_jumps_remaining > 0 {
                        body.vel.y = -ENEMY_DOUBLE_JUMP_SPEED;
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
                },
                crate::kinematic::KinematicInputs {
                    drop_through: frame.drop_through,
                },
                dt,
            );
            self.pos = body.pos;
            self.vel = body.vel;
            self.surface.on_ground = if is_aerial { false } else { body.on_ground };
            // Refresh the air-jump counter whenever we're standing
            // on a surface. Cheaper than landing-edge detection and
            // self-corrects if a brain happens to spam jump_pressed
            // mid-fall before the consume gate fires.
            if self.surface.on_ground {
                self.surface.air_jumps_remaining = MAX_ENEMY_AIR_JUMPS;
            }

            // KinematicPath patrols: the brain reads the path's "would
            // be" position to derive the desired_vel above. If
            // step_kinematic clipped the actor against a wall, the path
            // moved on without us — that's fine; the next tick the
            // brain re-derives velocity from the new path target and
            // catches up. Path itself is still advanced (single source
            // of truth for the patrol curve).
            if let Some(motion) = &mut self.motion {
                // Advance the path's internal cursor by `dt` regardless
                // of whether the body kept up. The brain reads from
                // `motion.advance` next tick to recompute desired_vel.
                let _ = motion.advance(self.pos, dt);
            }

            // Patrol turn-around: if the body's x velocity was non-zero
            // before the sweep but the sweep zeroed it (wall block),
            // flip facing so the next tick walks back. Aerial actors
            // skip this — they bend around obstacles by re-steering.
            if !is_aerial
                && matches!(ai.intent, crate::character_ai::CharacterAiIntent::Patrol)
                && prev_vel_x.abs() > 1.0
                && self.vel.x.abs() < 0.01
            {
                self.facing *= -1.0;
            }
        }

        // Facing: brain-frame facing is now authoritative. The legacy
        // ai.intent / fallback face_x paths were removed in the
        // brain-authority GC pass — every brain emits a facing
        // intent based on its own snapshot, and that intent wins.
        // Peaceful archetypes (`attacks_player() == false`) still
        // opt out so a passive patroller's brain frame can't make
        // it shadow the player.
        if self.archetype.attacks_player() && frame.facing.abs() > 0.001 {
            self.facing = frame.facing.signum();
        }

        // EFFECTS STAGE — translate the frame's attack intents into
        // wind-up timers / projectile spawns. Melee windup/cooldown start
        // moved to the EFFECTS consumer `start_enemy_melee_from_brain_actions`.
        // The old `attacks_player()` gate is intentionally not used for melee
        // capability anymore: PirateHeavy is peaceful by default through brain
        // aggressiveness, but an explicit-hostile path may install/provoke the
        // same archetype with a concrete ActionSet so it can swing.
        // `update` keeps `ai_mode = Attack` when `frame.fire.is_some()` since
        // that HUD signal is integration-side state.
        if frame.fire.is_some() {
            self.ai_mode = crate::character_ai::CharacterAiMode::Attack;
        }
        frame
    }

    /// Body AABB used for collision + damage routing.
    ///
    /// Surface-walking archetypes (`PuppySlug`) swap width × height
    /// when the surface normal is horizontal (slug clinging to a
    /// wall) so the hit-detection envelope stays aligned with the
    /// rendered, rotated sprite. All other archetypes report the
    /// authored AABB unchanged.
    pub fn aabb(&self) -> ae::Aabb {
        let size = if self.archetype == EnemyArchetype::PuppySlug
            && self.surface.surface_normal.x.abs() > 0.5
        {
            ae::Vec2::new(self.size.y, self.size.x)
        } else {
            self.size
        };
        ae::Aabb::new(self.pos, size * 0.5)
    }

    /// Bevy-frame Z rotation (radians) implied by `surface_normal`.
    /// Engine y grows downward but Bevy y grows upward; the formula
    /// `atan2(-n.x, -n.y)` accounts for that flip while keeping the
    /// sprite's authored "up" axis aligned with the surface's
    /// outward normal.
    /// - Floor (n=(0,-1))  → 0
    /// - Right wall (n=(-1,0)) → +π/2 (CCW in Bevy)
    /// - Ceiling (n=(0,1))   → ±π
    /// - Left wall  (n=(1,0))  → -π/2
    pub fn rotation_rad(&self) -> f32 {
        f32::atan2(
            -self.surface.surface_normal.x,
            -self.surface.surface_normal.y,
        )
    }

    /// Surface-walking integration for `PuppySlug`. The slug's
    /// invariant is "body is `body_thick` away from a solid in
    /// `-surface_normal` direction" — every tick re-establishes
    /// it. Routine per tick:
    ///
    /// 1. **Pre-step wall check.** Probe just past the leading edge
    ///    at body thickness. If solid → rotate CW (so the wall
    ///    becomes the new surface) and snap to it. Don't step.
    /// 2. **Step along tangent.** Move the slug forward.
    /// 3. **Snap to current surface.** Cast in `-normal` for a
    ///    solid within reach; reposition `pos` so the body just
    ///    touches it. If success, done.
    /// 4. **Try CCW rotation.** The slug walked off a ledge; the
    ///    surface ahead-and-down is the new surface. Snap.
    /// 5. **Try CW rotation.** Rare — overshot a concave corner
    ///    that pre-step missed. Snap.
    /// 6. **Fall.** No surface in any cardinal direction. Hand off
    ///    to a one-tick gravity step; on landing, re-pin to floor.
    ///
    /// Compared to the previous design (which fired discrete
    /// "convex"/"concave" rules and applied hand-tuned post-rotation
    /// offsets), this one delegates corner geometry to a single
    /// `snap_pos_to_surface` helper that finds the actual solid
    /// edge. That eliminates the "stuck at the lip of a ledge"
    /// failure where the hand-tuned offset didn't land on the new
    /// cliff face, and the "crawls between two stacked blocks"
    /// failure where the slug penetrated the wall without
    /// re-contacting it.
    fn step_surface_walker(
        &mut self,
        world: &ae::World,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
    ) {
        // ---- AIR PATH ---------------------------------------------
        // If the slug isn't already pinned to a surface, gravity
        // wins. Crawling logic only runs while `on_ground` is true.
        // Without this gate, the surface-clinging steps below kept
        // resetting `vel` to `tangent * speed` each tick, wiping out
        // the gravity accumulator from `fall_until_landed` — the
        // visible bug was the slug "gliding" slowly downward after
        // walking off something instead of falling. By short-circuiting
        // into the regular kinematic fall, the slug accelerates the
        // same way any other actor does until it touches a surface;
        // `fall_until_landed` re-pins the slug to a floor on landing
        // so crawling resumes next tick.
        if !self.surface.on_ground {
            self.fall_until_landed(world, dt);
            return;
        }

        let n = self.surface.surface_normal;
        let speed = self.archetype.patrol_speed();
        let step_len = speed * dt;
        // tangent_base = 90° math-CCW rotation of normal; multiply by
        // facing to pick a direction along the surface.
        let tangent = ae::Vec2::new(-n.y * self.facing, n.x * self.facing);
        let body_long = self.size.x * 0.5;
        let body_thick = self.size.y * 0.5;

        // ---- 0. Slug-vs-slug collision ----------------------------
        // If another actor is within body extent in front of us
        // (along tangent), reverse facing rather than stepping into
        // them. Both slugs do this on the same tick, so they bounce
        // apart instead of stacking. The `(body_long + 6)` along
        // and `(body_thick + 4)` perpendicular envelope is sized to
        // match the visual silhouette so two slugs visibly making
        // contact reverse; passing each other on parallel surfaces
        // (one floor, one ceiling) is unaffected because the
        // perpendicular check rejects.
        if let Some(neighbor_pos) = nearest_neighbor {
            let delta = neighbor_pos - self.pos;
            let along = delta.x * tangent.x + delta.y * tangent.y;
            let perp = delta.x * n.x + delta.y * n.y;
            if along > 0.0 && along < body_long + 6.0 && perp.abs() < body_thick + 4.0 {
                self.facing = -self.facing;
                self.vel = ae::Vec2::ZERO;
                return;
            }
        }

        // ---- 1. Pre-step wall check -------------------------------
        if self.wall_ahead(world, tangent, body_long, body_thick) {
            // Concave corner: the wall in `+tangent` becomes the new
            // floor. Its surface normal points back at the slug, i.e.
            // `-tangent`. Using a hardcoded CW rotation here used to
            // give the wrong direction for slugs facing -x (a slug
            // crawling left into a left wall rotated onto a phantom
            // wall on its right, snap_pos_to_surface failed because
            // there's no surface there, and the slug oscillated
            // between vertical and horizontal stuck in the corner).
            // `-tangent` works for both facings without branching.
            self.surface.surface_normal = -tangent;
            if self.snap_pos_to_surface(world) {
                self.vel = ae::Vec2::ZERO;
                self.surface.on_ground = true;
                return;
            }
            self.surface.surface_normal = n;
        }

        // ---- 2. Step along tangent --------------------------------
        let original_pos = self.pos;
        self.pos += tangent * step_len;
        self.vel = tangent * speed;

        // ---- 3. Snap to current surface ---------------------------
        if self.snap_pos_to_surface(world) {
            self.surface.on_ground = true;
            return;
        }

        // ---- 4. Convex wrap (forward) -----------------------------
        // Slug walked off the surface in the tangent direction; the
        // cliff face past the corner is the new surface. Its normal
        // points the direction the slug was moving — i.e. `tangent`.
        // Same facing-direction story as step 1: the original
        // hardcoded CCW rotation only worked for facing=+1.
        let new_normal = tangent;
        let around_corner = original_pos + tangent * body_long + (-n) * body_long;
        self.pos = around_corner;
        self.surface.surface_normal = new_normal;
        if self.snap_pos_to_surface(world) {
            self.vel = ae::Vec2::ZERO;
            self.surface.on_ground = true;
            return;
        }

        // ---- 5. Concave wrap retry (back) -------------------------
        // Very rare — the slug ran straight into a concave corner
        // that the pre-step probe missed because the geometry sat
        // just outside the probe envelope. Same rotation as step 1,
        // applied from the un-stepped position.
        self.pos = original_pos;
        self.surface.surface_normal = -tangent;
        if self.snap_pos_to_surface(world) {
            self.vel = ae::Vec2::ZERO;
            self.surface.on_ground = true;
            return;
        }

        // ---- 6. Fall -----------------------------------------------
        // No surface in any direction — drop. The air-path early
        // return at the top of this function keeps gravity
        // accumulating each tick once we land here.
        self.surface.surface_normal = n;
        self.pos = original_pos;
        self.surface.on_ground = false;
        self.fall_until_landed(world, dt);
    }

    /// True when the slug's next step in the tangent direction
    /// would punch its body into a wall. Probe is a thin slice
    /// perpendicular to tangent, sized just shy of body thickness
    /// across the normal axis so floor bumps under the body don't
    /// trigger it.
    fn wall_ahead(
        &self,
        world: &ae::World,
        tangent: ae::Vec2,
        body_long: f32,
        body_thick: f32,
    ) -> bool {
        let probe_center = self.pos + tangent * (body_long + 3.0);
        // Probe half-extents: slim along tangent (the direction we
        // care about) and ~body_thickness across normal (so the
        // probe matches the body cross-section, not the floor).
        let half = if tangent.x.abs() > 0.5 {
            ae::Vec2::new(2.0, body_thick * 0.7)
        } else {
            ae::Vec2::new(body_thick * 0.7, 2.0)
        };
        let probe = ae::Aabb::new(probe_center, half);
        world.body_overlaps_any(probe, surface_wall_pred)
    }

    /// Slide pos along the normal axis so the slug's body just
    /// touches a solid in `-surface_normal` direction. Casts a
    /// thin AABB outward from `self.pos` step-by-pixel until a
    /// solid block is hit; then shifts pos so the body's
    /// `body_thick` half-extent rests on the contact edge.
    ///
    /// Returns `false` if no solid is within reach (the slug is in
    /// open space; caller should fall or try another orientation).
    fn snap_pos_to_surface(&mut self, world: &ae::World) -> bool {
        let n = self.surface.surface_normal;
        let body_thick = self.size.y * 0.5;
        let body_long = self.size.x * 0.5;
        let down = -n;
        // Search outward up to a body_long beyond the contact line
        // — covers a full body-rotation around a corner (the slug
        // can wrap onto a wall whose face is up to body_long away
        // from where pos currently sits, post-corner-shift).
        let max_d = (body_thick + body_long + 4.0) as i32;
        // Probe is slim along the cast axis, wide across the
        // perpendicular so it matches the body's "foot print" on
        // the surface and tolerates 1-px floating-point jitter.
        let half = if n.x.abs() > 0.5 {
            ae::Vec2::new(0.75, body_long * 0.35)
        } else {
            ae::Vec2::new(body_long * 0.35, 0.75)
        };
        for i in 0..=max_d {
            let d = i as f32;
            let probe = ae::Aabb::new(self.pos + down * d, half);
            if world.body_overlaps_any(probe, surface_solid_pred) {
                // The solid edge is approximately `d - 0.5` from
                // pos along `down` (the 1-px probe spans
                // `[d - 0.5, d + 0.5]`). Shift pos along +normal so
                // the body's `body_thick`-deep underside touches the
                // edge exactly.
                self.pos += n * (body_thick - (d - 0.5));
                return true;
            }
        }
        false
    }

    /// Gravity-fall step when the slug has lost contact with any
    /// surface. Uses the standard `step_kinematic` path with
    /// world-frame gravity; on landing, re-orients the slug onto a
    /// floor (normal = (0, -1)).
    fn fall_until_landed(&mut self, world: &ae::World, dt: f32) {
        let mut body = crate::kinematic::KinematicBody {
            pos: self.pos,
            vel: self.vel,
            size: self.size,
            on_ground: self.surface.on_ground,
            facing: self.facing,
        };
        crate::kinematic::step_kinematic(
            &mut body,
            world,
            crate::kinematic::KinematicTuning {
                gravity: ENEMY_GRAVITY,
                max_fall_speed: ENEMY_MAX_FALL,
            },
            crate::kinematic::KinematicInputs {
                drop_through: false,
            },
            dt,
        );
        self.pos = body.pos;
        self.vel = body.vel;
        self.surface.on_ground = body.on_ground;
        if body.on_ground {
            // Re-pin to a floor surface — the slug forgets it was
            // ever on a wall once it lands.
            self.surface.surface_normal = ae::Vec2::new(0.0, -1.0);
        }
    }

    pub fn visual_kind(&self) -> FeatureVisualKind {
        if self.archetype.is_sandbag() {
            FeatureVisualKind::Sandbag
        } else {
            FeatureVisualKind::Enemy
        }
    }

    pub fn attack_aabb(&self) -> ae::Aabb {
        ae::Aabb::new(
            self.pos + ae::Vec2::new(self.facing * (self.size.x * 0.55 + 24.0), -4.0),
            ae::Vec2::new(34.0, 28.0),
        )
    }

    /// Directional hitbox geometry. `axis` is normalized (or
    /// expected to be small-magnitude); whichever component
    /// dominates picks the swing shape:
    ///   - `(±1, 0)` → forward / back swing — wide on x, mid on y
    ///     (legacy `attack_aabb` shape).
    ///   - `(0, -1)` → up-tilt: a TALL narrow column above the
    ///     head — Smash-style uppercut / juggle hit. Reach
    ///     stretches up far enough to grab a player at a hop's
    ///     apex.
    ///   - `(0, +1)` → down-air / stomp: wide and short below the
    ///     feet — covers a horizontal slice the falling actor
    ///     drops onto.
    /// Used at the windup → active edge in `update_ecs_actors` so
    /// the strike's `Hitbox` entity reflects the direction the
    /// brain committed to when it called `begin_melee_attack`.
    pub fn attack_aabb_dir(&self, axis: ae::Vec2) -> ae::Aabb {
        let horizontal = axis.x.abs() >= axis.y.abs();
        if horizontal {
            // Forward / back swing — pick the side from the axis sign
            // (falls back to facing if axis.x is near zero).
            let side = if axis.x.abs() > 0.1 {
                axis.x.signum()
            } else {
                self.facing
            };
            let center = self.pos + ae::Vec2::new(side * (self.size.x * 0.55 + 24.0), -4.0);
            return ae::Aabb::new(center, ae::Vec2::new(34.0, 28.0));
        }
        // Vertical attack — engine y grows downward, so `axis.y < 0`
        // = up-tilt, `axis.y > 0` = down-air.
        if axis.y < 0.0 {
            // Up-tilt: TALL + narrow column above the actor's head.
            // half_extent (16, 36) spans from ~22 px above the head
            // up to ~94 px above the head — long enough to catch a
            // player jumping straight up or hanging in a hop's apex.
            let half = ae::Vec2::new(16.0, 36.0);
            let center = self.pos + ae::Vec2::new(0.0, -(self.size.y * 0.5 + half.y + 4.0));
            return ae::Aabb::new(center, half);
        }
        // Down-air: wide stomp below the feet.
        let half = ae::Vec2::new(36.0, 20.0);
        let center = self.pos + ae::Vec2::new(0.0, self.size.y * 0.5 + half.y - 2.0);
        ae::Aabb::new(center, half)
    }

    /// Begin a melee attack windup + cooldown. Called by the EFFECTS
    /// consumer `start_enemy_melee_from_brain_actions` in response
    /// to an `ActorActionMessage::Melee`. Returns `true` if the
    /// attack actually started (the cooldown gate passed). Sandbag
    /// archetypes deliberately accept this — their PunchWeak counter
    /// is the legitimate use case.
    ///
    /// `attack_axis` is the swing direction the brain emitted on the
    /// originating frame (forward / up / down / back). It is stored
    /// on the runtime so the windup → active edge spawns the hitbox
    /// in the same direction the brain committed to, even though the
    /// edge fires many frames after the brain's decision. A zero
    /// vector defaults to a forward swing along the actor's facing.
    pub fn begin_melee_attack(
        &mut self,
        tuning: FeatureCombatTuning,
        attack_axis: ae::Vec2,
    ) -> bool {
        if self.attack.cooldown > 0.0 || !self.alive {
            return false;
        }
        self.attack.windup_timer = tuning.enemy_attack_windup.max(0.01);
        self.attack.cooldown = ENEMY_ATTACK_COOLDOWN
            * if self.archetype == EnemyArchetype::SmallSkitter {
                0.75
            } else if self.archetype == EnemyArchetype::LargeBrute {
                1.35
            } else {
                1.0
            };
        self.ai_mode = crate::character_ai::CharacterAiMode::Telegraph;
        self.attack.pending_axis = if attack_axis.length_squared() > 0.01 {
            attack_axis.normalize_or_zero()
        } else {
            ae::Vec2::new(self.facing, 0.0)
        };
        true
    }

    pub fn attack_telegraph_aabb(&self) -> ae::Aabb {
        self.attack_aabb()
    }

    /// Always-on body contact damage volume for normal enemies.
    ///
    /// Sandbags intentionally opt out: they are hit-confirm / tuning targets,
    /// not hostile actors. Their body AABB remains their player-attack hurtbox.
    /// `PirateOnShark` also opts out — body contact would punish the
    /// player simply for being below an orbiting shark; its damage
    /// comes through projectile volleys.
    pub fn body_damage_aabb(&self) -> Option<ae::Aabb> {
        // Sandbags are training dummies, not threats. The fused
        // `*OnShark` actors damage through projectile volleys, not
        // body contact (orbiting sharks would otherwise punish the
        // player just for standing under one). Peaceful patrollers
        // (PuppySlug & PirateHeavy by default — see
        // `EnemyArchetype::attacks_player`) opt out of touch damage
        // entirely so the player can walk past or through them
        // without taking hits. PuppySlug is an intentional
        // exception to that rule — its whole gimmick is "you take
        // damage only on physical contact," so we re-enable its
        // body hitbox below.
        if !self.archetype.body_contact_damage_enabled() {
            return None;
        }
        Some(self.aabb())
    }

    /// Polled body-contact damage check. The attack-swing arm moved
    /// to the `Hitbox` entity lifecycle (see
    /// `content/features/ecs/hitbox.rs`); body contact is "you ran
    /// into the enemy" — a per-tick test against the integration
    /// state, not a discrete strike — and keeps its polled shape.
    ///
    /// `player_entity` is the player whose body the caller is
    /// checking against; it's stamped on the returned `HitEvent`'s
    /// `target` so the player-side reader lands the hit on that
    /// specific player rather than falling back to primary. The
    /// caller (`update_ecs_actors`) iterates every player and calls
    /// this once per (enemy, player) pair.
    pub(super) fn body_contact_damage(
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
            damage: self.archetype.damage_amount(),
            source: HitSource::EnemyBody,
            attacker: None,
            // Pre-resolved victim: the caller already picked the
            // overlapping player. Multi-player ready.
            target: HitTarget::Player(player_entity),
            mode: HitMode::Knockback,
            knockback: Some(HitKnockback {
                dir: (player_body.center().x - self.pos.x).signum_or(self.facing),
                strength: self.archetype.contact_strength(),
                source_pos: self.pos,
                impact_pos: impact,
            }),
            ignored_targets: Vec::new(),
        })
    }
}

/// Cluster-native enemy integration. This is the EnemyRuntime::update
/// physics/AI port, operating directly on the authoritative ECS
/// components through the [`EnemyMut`] view (player cluster pattern).
/// Field map: self.kin.* (pos/vel/size/facing), self.status.* (alive/
/// respawn_timer/hit_flash/ai_mode/health), self.config.* (archetype/
/// brain/spawn), self.attack.* / self.surface.* unchanged, self.motion.0.
impl<'a> EnemyMut<'a> {
    pub fn update(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: FeatureCombatTuning,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
        _is_mounted: bool,
        frame: crate::actor_control::ActorControlFrame,
    ) -> crate::actor_control::ActorControlFrame {
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
            self.status.ai_mode = crate::character_ai::CharacterAiMode::Dead;
            return crate::actor_control::ActorControlFrame::neutral();
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
        let ai = crate::character_ai::evaluate_character_ai_output(
            crate::character_ai::CharacterAiSnapshot {
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
            },
        );
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
                    if body.on_ground {
                        body.vel.y = -ENEMY_JUMP_SPEED;
                        body.on_ground = false;
                    } else if self.surface.air_jumps_remaining > 0 {
                        body.vel.y = -ENEMY_DOUBLE_JUMP_SPEED;
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
                && matches!(ai.intent, crate::character_ai::CharacterAiIntent::Patrol)
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
            self.status.ai_mode = crate::character_ai::CharacterAiMode::Attack;
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
        self.status.ai_mode = crate::character_ai::CharacterAiMode::Telegraph;
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
        self.status.ai_mode = crate::character_ai::CharacterAiMode::Idle;
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
        let mut enemy = EnemyRuntime::new(
            "pirate_heavy_reach_probe",
            "Broadside Bess",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(36.0, 55.0)),
            crate::actor::EnemyBrain::Custom("pirate_heavy".into()),
            &[],
        );
        enemy.facing = 1.0;
        assert_eq!(enemy.archetype, EnemyArchetype::PirateHeavy);
        let hitbox = enemy.attack_aabb_dir(ae::Vec2::new(1.0, 0.0));
        let reach_edge = hitbox.center().x + hitbox.half_size().x - enemy.pos.x;
        let attack_range = enemy.archetype.attack_range();
        assert!(
            attack_range <= reach_edge,
            "PirateHeavy attack_range {attack_range} must stay within her swing far \
             edge {reach_edge} so she stops inside her own reach instead of whiffing",
        );
    }
}
