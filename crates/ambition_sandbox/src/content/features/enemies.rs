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

#[derive(Clone, Debug)]
pub struct EnemyRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub spawn: ae::Vec2,
    pub size: ae::Vec2,
    pub vel: ae::Vec2,
    pub health: ae::Health,
    pub brain: ae::EnemyBrain,
    pub archetype: EnemyArchetype,
    /// Authored spawn archetype, captured at construction. `archetype`
    /// can mutate at runtime (PirateOnShark dismounts into
    /// PirateRaider or BurningFlyingShark on rider/shark death), so
    /// `spawn_archetype` is the "what the level author wrote" record
    /// that `reset_to_spawn` restores. Identical to `archetype` for
    /// every non-morphing actor.
    pub spawn_archetype: EnemyArchetype,
    /// Authored spawn size (in case `archetype` mutates to one with
    /// a different `default_size`, like PirateOnShark → PirateRaider).
    pub spawn_size: ae::Vec2,
    pub motion: Option<PathMotion>,
    pub alive: bool,
    pub facing: f32,
    pub attack_windup_timer: f32,
    pub attack_timer: f32,
    pub attack_cooldown: f32,
    pub respawn_timer: f32,
    pub hit_flash: f32,
    /// Last-evaluated `CharacterAiMode`. Updated by `update`. Read by
    /// HUD / rendering / debug overlay so they can branch on a single
    /// vocabulary instead of inferring it from the timer fields.
    pub ai_mode: ae::CharacterAiMode,
    /// Set by [`ae::step_kinematic`](ambition_engine::step_kinematic)
    /// each tick. Used by chase-drop-through
    /// (enemy must be standing on something before it tries to fall
    /// through it) and by future jump AI.
    pub on_ground: bool,
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
    /// 0.0 = ignores gravity (flying); 1.0 = full gravity. Set by the
    /// archetype (`BurningFlyingShark` / `PirateOnShark` are 0.0).
    pub gravity_scale: f32,
    /// Authored attack choreography. `MeleeContact` for legacy
    /// enemies; the pirate-sky archetypes use volley/orbit/dive.
    pub choreography: ae::AttackChoreography,
    /// Persistent per-tick state for the choreography evaluator.
    pub choreography_state: ae::ChoreographyState,
    /// Optional separate "rider" health — used by the fused
    /// `PirateOnShark` actor where the pirate on top can be killed
    /// independently of the shark. `None` for everyone else.
    pub rider_health: Option<ae::Health>,
    /// Outward-pointing unit normal of the surface the actor is
    /// currently clinging to. Used by surface-walking archetypes
    /// (`PuppySlug`) to crawl floors, walls, and ceilings; all
    /// other archetypes pin this at `(0, -1)` (floor) and ignore
    /// it. Engine y grows downward, so floor → (0, -1), right wall
    /// → (-1, 0), ceiling → (0, 1), left wall → (1, 0).
    pub surface_normal: ae::Vec2,
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
    /// Fused pirate-on-shark actor. Two health pools, aerial
    /// orbit-and-fire choreography. Dismounts into `PirateRaider`
    /// when the shark dies and into `BurningFlyingShark` when the
    /// rider dies.
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
    /// `PirateOnShark` clone (two health pools, aerial orbit-and-
    /// fire choreography) but the rider sprite resolves to one of
    /// the heavy-variant sheets instead of `Pirate Raider`. On
    /// shark-death dismount, the rider drops to a ground
    /// `PirateHeavy` (heavier and slower than a `PirateRaider`).
    PirateHeavyOnShark,
}

/// Maps `ae::EnemyBrain::Custom("...")` strings to archetype variants.
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
#[derive(Clone, Copy, Debug, serde::Deserialize)]
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
    /// Locomotion style for the actor's `ActionSet.move_style`.
    pub move_style: crate::brain::MoveStyleSpec,
}

/// Glue: `Option<ae::Vec2>` deserializes from a `(x, y)` tuple in RON
/// or an explicit `None`. `bevy_math::Vec2` doesn't implement
/// `Deserialize` directly under the features the sandbox compiles
/// with, so route through a tuple shim.
mod vec2_option {
    use ambition_engine as ae;
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
    *ENEMY_ARCHETYPE_REGISTRY.get(key).unwrap_or_else(|| {
        panic!("enemy archetype {arch:?} (RON key '{key}') missing from enemy_archetypes.ron")
    })
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
    const ENEMY_ARCHETYPES_RON: &str =
        include_str!("../../../assets/data/enemy_archetypes.ron");
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

    pub(super) fn from_brain(brain: &ae::EnemyBrain) -> Self {
        let ae::EnemyBrain::Custom(name) = brain else {
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

    /// Locomotion style for this archetype's `ActionSet.move_style`.
    pub(super) fn move_style(self) -> crate::brain::MoveStyleSpec {
        self.spec().move_style
    }

    /// Slot kind this archetype requests from the combat slot board.
    /// Used by the per-frame slot allocator.
    pub(super) fn slot_kind(self) -> ae::SlotKind {
        if self.is_aerial() {
            ae::SlotKind::Aerial
        } else {
            ae::SlotKind::Melee
        }
    }

    /// Authored attack choreography for this archetype. Kept as a
    /// dedicated method (not table data) because each ranged
    /// variant carries its own non-Copy parameter bag — putting them
    /// in the spec table would force every row to spell out a
    /// `MeleeContact` literal.
    pub(super) fn choreography(self) -> ae::AttackChoreography {
        match self {
            Self::PirateOnShark | Self::PirateHeavyOnShark => {
                ae::AttackChoreography::AerialOrbitAndFire {
                    altitude: 150.0,
                    radius: 220.0,
                    orbit_speed: 0.85,
                    fire_interval: 1.5,
                    projectile_speed: 360.0,
                }
            }
            Self::BurningFlyingShark => ae::AttackChoreography::DiveStrike {
                hover_altitude: 140.0,
                hover_rest: 0.55,
                dive_speed: 360.0,
                recover_height: 100.0,
            },
            // Default: legacy melee-contact behavior.
            _ => ae::AttackChoreography::MeleeContact,
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
    pub(super) fn default_size(self) -> Option<ae::Vec2> {
        self.spec().default_size
    }

    /// True when this archetype is hostile by default — initiates
    /// attacks, deals contact damage, and tracks the player. False
    /// for "peaceful patrol" archetypes (PuppySlug, PirateHeavy)
    /// that exist as ambient threats / cove crew rather than active
    /// combatants. The gate is applied uniformly in the EFFECTS
    /// stage (no melee windups) and in `body_damage_aabb` (no
    /// touch damage), so a peaceful patroller can pace around the
    /// player without harming them.
    pub fn attacks_player(self) -> bool {
        use EnemyArchetype::*;
        !matches!(self, PuppySlug | PirateHeavy)
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

/// Per-tick outputs the caller (`update_ecs_actors`) flushes into
/// world resources. Empty today — projectile spawns moved to the
/// EFFECTS-stage consumer `spawn_enemy_projectiles_from_brain_actions`
/// per the actor/brain migration. Kept as a placeholder so future
/// runtime-internal side effects (telegraph SFX requests, area-of-
/// effect hazards) can land without churning the `update` signature.
#[derive(Default)]
pub struct EnemyTickOutputs;

/// Outcome of [`EnemyRuntime::apply_damage_at`]. Callers branch on
/// this to know whether to play a hit SFX, despawn the actor, or
/// rebind sprite/visual state because the archetype morphed
/// (pirate-on-shark dismounting into pirate or shark).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyDamageOutcome {
    /// Hit didn't land (dead enemy, missed hitbox).
    NoOp,
    Damaged {
        killed: bool,
        /// True when the actor's archetype changed in place (fused
        /// dismount). Sprite/anim systems should refresh their
        /// per-archetype state.
        archetype_changed: bool,
    },
}

impl EnemyRuntime {
    pub(super) fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: ae::EnemyBrain,
        paths: &[(String, ae::KinematicPath)],
    ) -> Self {
        let archetype = EnemyArchetype::from_brain(&brain);
        let motion = match &brain {
            ae::EnemyBrain::Patrol {
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
            spawn: pos,
            size,
            vel: ae::Vec2::ZERO,
            health: ae::Health::new(archetype.max_health()),
            brain,
            archetype,
            spawn_archetype: archetype,
            spawn_size: size,
            motion,
            alive: true,
            facing: -1.0,
            attack_windup_timer: 0.0,
            attack_timer: 0.0,
            attack_cooldown: 0.2,
            respawn_timer: 0.0,
            hit_flash: 0.0,
            ai_mode: ae::CharacterAiMode::Idle,
            on_ground: false,
            sprite_override_npc_name: None,
            gravity_scale: if archetype.is_aerial() { 0.0 } else { 1.0 },
            choreography: archetype.choreography(),
            choreography_state: ae::ChoreographyState::default(),
            rider_health: archetype.rider_max_health().map(ae::Health::new),
            surface_normal: ae::Vec2::new(0.0, -1.0),
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
        let archetype = self.spawn_archetype;
        self.archetype = archetype;
        self.size = self.spawn_size;
        self.pos = self.spawn;
        self.vel = ae::Vec2::ZERO;
        self.alive = true;
        self.health = ae::Health::new(archetype.max_health());
        self.rider_health = archetype.rider_max_health().map(ae::Health::new);
        self.gravity_scale = if archetype.is_aerial() { 0.0 } else { 1.0 };
        self.choreography = archetype.choreography();
        self.choreography_state = ae::ChoreographyState::default();
        self.attack_windup_timer = 0.0;
        self.attack_timer = 0.0;
        self.attack_cooldown = 0.2;
        self.respawn_timer = 0.0;
        self.hit_flash = 0.0;
        self.ai_mode = ae::CharacterAiMode::Idle;
        self.on_ground = false;
        self.facing = -1.0;
        // Surface walkers reset to floor — even if the slug died on
        // a wall or ceiling, respawn pins it on whatever the platform
        // its `spawn` sits above is. Same-room reset path's
        // collision-shape rewrite in `EnemyTickOutputs` reads the
        // unrotated `size`, which is correct again at floor stance.
        self.surface_normal = ae::Vec2::new(0.0, -1.0);
    }

    /// True when this actor still has a rider (live pirate on top).
    /// Used by the renderer to decide whether to composite a pirate
    /// sprite over the shark.
    pub fn has_live_rider(&self) -> bool {
        self.rider_health.map(|h| h.alive()).unwrap_or(false)
    }

    /// AABB covering the upper half of the actor — the "rider"
    /// hitbox on a fused pirate-on-shark. Player hits that overlap
    /// this region damage the rider's HP pool, not the shark's.
    pub fn rider_aabb(&self) -> Option<ae::Aabb> {
        self.rider_health?;
        // The pirate sits on top of the shark. The shark sprite is
        // ~96 tall; the pirate occupies the top ~52 px (its sprite is
        // 128 tall but visually compressed when riding).
        let rider_height = 52.0;
        let half_h = rider_height * 0.5;
        let center = ae::Vec2::new(self.pos.x, self.pos.y - (self.size.y * 0.5) - half_h + 8.0);
        Some(ae::Aabb::new(
            center,
            ae::Vec2::new(self.size.x * 0.4, half_h),
        ))
    }

    /// Route an incoming player attack hit to either the rider or
    /// the body (shark) on a fused pirate-on-shark. Returns the
    /// archetype the actor *now* is — different from the pre-hit
    /// archetype if a death triggered a dismount morph. Used by the
    /// caller to decide whether to swap sprite / brain bindings.
    pub fn apply_damage_at(&mut self, hit_volume: ae::Aabb, damage: i32) -> EnemyDamageOutcome {
        if !self.alive {
            return EnemyDamageOutcome::NoOp;
        }
        // Fused pirate-on-shark: route by overlap with the rider hitbox.
        if matches!(
            self.archetype,
            EnemyArchetype::PirateOnShark | EnemyArchetype::PirateHeavyOnShark
        ) {
            if let (Some(rider_aabb), Some(rider)) = (self.rider_aabb(), self.rider_health.as_mut())
            {
                if rider_aabb.strict_intersects(hit_volume) && rider.alive() {
                    let killed = rider.damage(damage);
                    self.hit_flash = 0.18;
                    if killed {
                        return self.dismount_rider();
                    }
                    return EnemyDamageOutcome::Damaged {
                        killed: false,
                        archetype_changed: false,
                    };
                }
            }
        }
        // Default path: damage the body health pool.
        let killed = self.health.damage(damage);
        self.hit_flash = 0.18;
        if killed {
            // If this was a fused pirate-on-shark variant, the shark
            // died — dismount the rider into a grounded pirate (heavy
            // bruiser if the rider was a heavy, raider otherwise).
            if matches!(
                self.archetype,
                EnemyArchetype::PirateOnShark | EnemyArchetype::PirateHeavyOnShark
            ) {
                return self.dismount_shark();
            }
            self.alive = false;
            self.respawn_timer = 0.0;
            return EnemyDamageOutcome::Damaged {
                killed: true,
                archetype_changed: false,
            };
        }
        EnemyDamageOutcome::Damaged {
            killed: false,
            archetype_changed: false,
        }
    }

    /// Rider died: actor becomes a riderless burning shark. Shark hp
    /// pool is preserved; choreography swaps to dive-strike.
    fn dismount_rider(&mut self) -> EnemyDamageOutcome {
        self.archetype = EnemyArchetype::BurningFlyingShark;
        self.choreography = self.archetype.choreography();
        self.choreography_state = ae::ChoreographyState::default();
        self.rider_health = None;
        EnemyDamageOutcome::Damaged {
            killed: false,
            archetype_changed: true,
        }
    }

    /// Shark died: actor becomes a grounded pirate. The grounded
    /// form depends on which fused variant the actor was —
    /// `PirateOnShark` → `PirateRaider`, `PirateHeavyOnShark` →
    /// `PirateHeavy`. The rider keeps its (potentially partial) HP
    /// pool, capped by the new archetype's max.
    ///
    /// Renaming the runtime is what makes the sprite layer pick up
    /// the right sheet on the next `upgrade_enemy_sprites` pass.
    /// Without this, the actor walks around as a pirate-sized
    /// hitbox but still rendered as a shark. For `PirateHeavy` the
    /// rider name was already a heavy variant (e.g. "Broadside Bess
    /// on Shark"); we strip the " on Shark" suffix so the sprite
    /// resolver finds the same heavy sheet on the ground.
    fn dismount_shark(&mut self) -> EnemyDamageOutcome {
        let dismount_target = match self.archetype {
            EnemyArchetype::PirateHeavyOnShark => EnemyArchetype::PirateHeavy,
            _ => EnemyArchetype::PirateRaider,
        };
        let inherited_hp = self
            .rider_health
            .filter(|h| h.alive())
            .map(|h| h.current)
            .unwrap_or_else(|| dismount_target.max_health());
        self.archetype = dismount_target;
        self.choreography = self.archetype.choreography();
        self.choreography_state = ae::ChoreographyState::default();
        self.health = ae::Health::new(dismount_target.max_health());
        self.health.current = inherited_hp.min(self.health.max);
        self.rider_health = None;
        self.gravity_scale = 1.0;
        if let Some(default_size) = dismount_target.default_size() {
            self.size = default_size;
        }
        self.name = match dismount_target {
            EnemyArchetype::PirateHeavy => {
                // Strip the " on Shark" suffix authored on the
                // EnemySpawn so the heavy's ground sheet (Broadside
                // Bess / Iron Mary / Salt Annet) resolves cleanly.
                self.name
                    .strip_suffix(" on Shark")
                    .map(str::to_owned)
                    .unwrap_or_else(|| String::from("Broadside Bess"))
            }
            _ => String::from("Pirate Raider"),
        };
        EnemyDamageOutcome::Damaged {
            killed: false,
            archetype_changed: true,
        }
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
    /// Tick the enemy's AI + integration. Returns the per-tick
    /// `ActorControlFrame` the runtime's legacy choreography
    /// computed; the caller writes it into the entity's
    /// `ActorControl` component so `emit_brain_action_messages`
    /// + EFFECTS-stage consumers see the intent. Per the
    /// actor/brain migration mandate, this is the seam the brain
    /// will eventually take over from — for now the legacy AI is
    /// the authority for fire/melee intent on hostile actors.
    /// Tick the enemy: decrement timers, run the legacy AI +
    /// choreography (still drives `ai_mode` + `choreography_state`
    /// for HUD / animation), and integrate one kinematic step.
    ///
    /// When `override_frame` is `Some`, the integration uses THAT
    /// frame's `desired_vel` / `drop_through` / facing instead of the
    /// legacy-AI's. This is the seam the Smash brain uses to take
    /// movement authority: actors.rs runs the brain to build a
    /// frame, then calls `update` with that frame as the override.
    /// The legacy choreography still ticks (so animation state
    /// stays sensible) but its movement intent is shelved. `None` =
    /// pre-brain behavior (legacy AI drives integration).
    pub(super) fn update(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: FeatureCombatTuning,
        slot_pos: Option<ae::Vec2>,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
        override_frame: Option<ae::ActorControlFrame>,
    ) -> ae::ActorControlFrame {
        // `EnemyTickOutputs` is gone — projectile spawns flow through
        // the EFFECTS-stage consumer per the actor/brain migration.
        // The struct is preserved for future runtime-internal side
        // effects (telegraph SFX requests, area-of-effect hazards).
        let _ = EnemyTickOutputs;
        // Seed is derived from the actor id and cached on the
        // choreography state. Done lazily here (rather than in
        // `new`) so reset_to_spawn — which `Default`s the state —
        // re-establishes the seed automatically on the next tick.
        if self.choreography_state.seed == 0 {
            self.choreography_state.seed = ae::seed_from_id(&self.id);
        }
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        if !self.alive {
            self.respawn_timer = (self.respawn_timer - dt).max(0.0);
            if self.archetype == EnemyArchetype::FiniteSandbag && self.respawn_timer <= 0.0 {
                self.alive = true;
                self.health.reset();
                self.pos = self.spawn;
                self.vel = ae::Vec2::ZERO;
                self.hit_flash = 0.24;
            }
            self.ai_mode = ae::CharacterAiMode::Dead;
            return ae::ActorControlFrame::neutral();
        }

        let was_winding_up = self.attack_windup_timer > 0.0;
        self.attack_windup_timer = (self.attack_windup_timer - dt).max(0.0);
        self.attack_timer = (self.attack_timer - dt).max(0.0);
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        if was_winding_up && self.attack_windup_timer <= 0.0 {
            self.attack_timer = tuning.enemy_attack_active.max(0.01);
        }

        let recover_remaining = if self.attack_cooldown > 0.0
            && self.attack_windup_timer <= 0.0
            && self.attack_timer <= 0.0
        {
            self.attack_cooldown.min(0.30)
        } else {
            0.0
        };
        let effective_aggro_radius = match &self.brain {
            ae::EnemyBrain::Passive => 0.0,
            ae::EnemyBrain::Guard { leash_radius } => *leash_radius,
            _ => self.archetype.aggro_radius(),
        };
        let ai = ae::evaluate_character_ai_output(ae::CharacterAiSnapshot {
            actor_pos: self.pos,
            player_pos: target_pos,
            aggro_radius: effective_aggro_radius,
            attack_range: self.archetype.attack_range(),
            attack_windup_remaining: self.attack_windup_timer,
            attack_active_remaining: self.attack_timer,
            attack_recover_remaining: recover_remaining,
            stun_remaining: 0.0,
            alive: self.alive,
            patrol_enabled: !self.archetype.is_sandbag()
                && !matches!(self.brain, ae::EnemyBrain::Passive),
        });
        self.ai_mode = ai.mode;

        // Run the authored attack choreography. It produces a
        // steering target (where the actor would *like* to be) plus
        // an optional attack action (melee swing / fire projectile).
        // The choreography is consulted regardless of `ai.intent` —
        // it does not bypass the AI mode, just refines spatial
        // targeting and attack flavor.
        let assigned_slot_pos = slot_pos.unwrap_or(target_pos);
        self.choreography_state.has_slot = slot_pos.is_some();
        let choreo_tick = ae::evaluate_choreography(
            self.choreography,
            &mut self.choreography_state,
            ae::ChoreographyInput {
                actor_pos: self.pos,
                target_pos,
                assigned_slot_pos,
                dt,
                nearest_neighbor,
            },
        );

        // BRAIN STAGE — read AI mode + choreography output, plus the
        // archetype's patrol/chase speeds and (when present) the
        // KinematicPath the enemy is bound to, and pack the whole
        // "what does this actor want this tick" decision into a
        // single `ActorControlFrame`. The integration stage below
        // only reads the frame, never the underlying brain — so a
        // future RL-policy or scripted brain that fills the same
        // frame plugs in without touching collision logic.
        let is_aerial = self.gravity_scale <= 0.001;
        let is_surface_walker = self.archetype == EnemyArchetype::PuppySlug;

        let legacy_frame = self.build_control_frame(&ai, &choreo_tick, target_pos, is_aerial, dt);
        // When the caller supplied an override (Smash brain has
        // authority), use it for integration. Otherwise fall back to
        // the legacy AI frame the choreography just produced.
        let frame = override_frame.unwrap_or(legacy_frame);

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
                ENEMY_GRAVITY * self.gravity_scale
            };
            let mut body = ae::KinematicBody {
                pos: self.pos,
                vel: self.vel,
                size: self.size,
                on_ground: self.on_ground,
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
            }
            ae::step_kinematic(
                &mut body,
                world,
                ae::KinematicTuning {
                    gravity,
                    max_fall_speed: max_fall,
                },
                ae::KinematicInputs {
                    drop_through: frame.drop_through,
                },
                dt,
            );
            self.pos = body.pos;
            self.vel = body.vel;
            self.on_ground = if is_aerial { false } else { body.on_ground };

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
                && matches!(ai.intent, ae::CharacterAiIntent::Patrol)
                && prev_vel_x.abs() > 1.0
                && self.vel.x.abs() < 0.01
            {
                self.facing *= -1.0;
            }
        }

        // Facing: AI/choreography facing always wins over derived
        // facing; brain-frame facing (when set) wins over both. This
        // ordering matches the pre-refactor behaviour.
        //
        // Non-aggressive archetypes (PuppySlug + peaceful patrollers
        // like PirateHeavy in the cove — anyone with
        // `attacks_player() == false`) opt OUT of every player-aware
        // facing override. The melee choreography's
        // `face_x = face_toward(self, player)` runs unconditionally
        // — leaving it enabled here means a peaceful patroller's
        // facing flips toward the player every tick, and since
        // `desired_x = facing * patrol_speed`, the patroller ends
        // up *walking toward* the player rather than pacing in
        // place. Gating on `attacks_player()` keeps cove crew
        // visually "patrolling on their own beat" instead of
        // shadowing the player. This is the stop-gap until the
        // universal-brain refactor lands (see
        // `docs/planning/universal-brain-interface.md`).
        if self.archetype.attacks_player() {
            match ai.intent {
                ae::CharacterAiIntent::Chase { direction_x }
                | ae::CharacterAiIntent::Attack { direction_x }
                    if direction_x.abs() > 0.001 =>
                {
                    self.facing = direction_x.signum();
                }
                _ => {}
            }
            if choreo_tick.face_x.abs() > 0.001 {
                self.facing = choreo_tick.face_x;
            }
            if frame.facing.abs() > 0.001 {
                self.facing = frame.facing.signum();
            }
        }

        // EFFECTS STAGE — translate the frame's attack intents into
        // wind-up timers / projectile spawns. Archetypes that don't
        // attack by default (`PuppySlug`, `PirateHeavy`) skip this
        // entirely so the `MeleeContact` choreography's reflexive
        // "swing when player is close" can't leak through into a
        // real hit-volume. The aerial PirateHeavyOnShark variant
        // is a separate archetype with `attacks_player() == true`,
        // so it still fires projectiles + recoil.
        // Melee windup/cooldown start moved to the EFFECTS consumer
        // `start_enemy_melee_from_brain_actions`. The legacy gate
        // (`frame.melee_pressed && !is_sandbag && attacks_player &&
        // cooldown<=0`) is now expressed as: the resolver only emits
        // `ActorActionMessage::Melee` when the archetype's ActionSet
        // has a Melee spec (peaceful archetypes get no melee in
        // their ActionSet); the consumer adds the cooldown check.
        // Sandbags are also gated by their action_kind = PunchWeak —
        // their attack_windup runs through the same consumer.
        // `update` keeps `ai_mode = Attack` when `frame.fire.is_some()`
        // since that HUD signal is integration-side state.
        if frame.fire.is_some() {
            self.ai_mode = ae::CharacterAiMode::Attack;
        }
        frame
    }

    /// Pack the per-tick AI + choreography decision into a flat
    /// `ActorControlFrame`. This is the brain-to-sim seam — a
    /// future RL policy that wants to control an enemy fills the
    /// SAME frame and the integration code in `update` is
    /// unchanged.
    fn build_control_frame(
        &mut self,
        ai: &ae::CharacterAiOutput,
        choreo_tick: &ae::ChoreographyTick,
        target_pos: ae::Vec2,
        is_aerial: bool,
        dt: f32,
    ) -> ae::ActorControlFrame {
        let mut frame = ae::ActorControlFrame::neutral();

        // Drop-through: chasing a player meaningfully below the actor
        // while currently grounded. Lets enemies follow through
        // one-way platforms the player just used. Aerial bodies have
        // no on_ground so this is naturally a no-op for them.
        let delta_y = target_pos.y - self.pos.y;
        frame.drop_through = !is_aerial
            && matches!(ai.intent, ae::CharacterAiIntent::Chase { .. })
            && self.on_ground
            && delta_y > 48.0;

        // Desired velocity. Aerial actors fly in 2D toward the
        // choreography's steering target; grounded actors get an
        // x-axis intent that the integration stage ramps toward.
        // KinematicPath patrol overrides the x intent with the
        // path's lookahead so patrols actually walk their curve.
        if is_aerial {
            let steering_speed = choreo_tick
                .steering_speed_override
                .unwrap_or_else(|| self.archetype.chase_speed());
            let to_target = choreo_tick.steering_target - self.pos;
            let dist = to_target.length();
            frame.desired_vel = if dist > 1.0 {
                (to_target / dist) * steering_speed
            } else {
                ae::Vec2::ZERO
            };
        } else if let Some(motion) = self.motion.as_ref() {
            // Path patrols only walk their curve when the AI is in
            // Patrol intent. Hold/Attack must keep the actor pinned
            // (e.g. during a telegraph against an in-range player).
            // Lookahead-by-dt asks the path where it wants to be
            // next tick without mutating the cursor; `update`
            // advances the cursor separately so the path remains
            // the source of truth even when collision blocks the
            // body.
            if matches!(ai.intent, ae::CharacterAiIntent::Patrol) {
                let target_pos = motion.lookahead(self.pos, dt);
                let dx = target_pos.x - self.pos.x;
                let desired_x = if dt > 0.0 { dx / dt } else { 0.0 };
                frame.desired_vel = ae::Vec2::new(desired_x, 0.0);
                frame.facing = dx.signum_or(self.facing);
            }
        } else {
            let desired_x = match ai.intent {
                ae::CharacterAiIntent::Hold | ae::CharacterAiIntent::Attack { .. } => {
                    if ai.committed() {
                        self.vel.x * 0.4
                    } else {
                        0.0
                    }
                }
                ae::CharacterAiIntent::Patrol => self.facing * self.archetype.patrol_speed(),
                ae::CharacterAiIntent::Chase { .. } => {
                    let dx = choreo_tick.steering_target.x - self.pos.x;
                    let sign = if dx.abs() < 1.0 { 0.0 } else { dx.signum() };
                    let speed = choreo_tick
                        .steering_speed_override
                        .unwrap_or_else(|| self.archetype.chase_speed());
                    sign * speed
                }
            };
            frame.desired_vel = ae::Vec2::new(desired_x, 0.0);
        }

        // Attack intents from the choreography are forwarded onto
        // the frame; the simulation half handles cooldown gating.
        match choreo_tick.action {
            Some(ae::ChoreographyAction::Melee) => {
                frame.melee_pressed = true;
            }
            Some(ae::ChoreographyAction::FireProjectile { dir, speed }) => {
                frame.fire = Some(ae::ActorFireRequest { dir, speed });
            }
            None => {}
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
        let size =
            if self.archetype == EnemyArchetype::PuppySlug && self.surface_normal.x.abs() > 0.5 {
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
        f32::atan2(-self.surface_normal.x, -self.surface_normal.y)
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
        if !self.on_ground {
            self.fall_until_landed(world, dt);
            return;
        }

        let n = self.surface_normal;
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
            self.surface_normal = -tangent;
            if self.snap_pos_to_surface(world) {
                self.vel = ae::Vec2::ZERO;
                self.on_ground = true;
                return;
            }
            self.surface_normal = n;
        }

        // ---- 2. Step along tangent --------------------------------
        let original_pos = self.pos;
        self.pos += tangent * step_len;
        self.vel = tangent * speed;

        // ---- 3. Snap to current surface ---------------------------
        if self.snap_pos_to_surface(world) {
            self.on_ground = true;
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
        self.surface_normal = new_normal;
        if self.snap_pos_to_surface(world) {
            self.vel = ae::Vec2::ZERO;
            self.on_ground = true;
            return;
        }

        // ---- 5. Concave wrap retry (back) -------------------------
        // Very rare — the slug ran straight into a concave corner
        // that the pre-step probe missed because the geometry sat
        // just outside the probe envelope. Same rotation as step 1,
        // applied from the un-stepped position.
        self.pos = original_pos;
        self.surface_normal = -tangent;
        if self.snap_pos_to_surface(world) {
            self.vel = ae::Vec2::ZERO;
            self.on_ground = true;
            return;
        }

        // ---- 6. Fall -----------------------------------------------
        // No surface in any direction — drop. The air-path early
        // return at the top of this function keeps gravity
        // accumulating each tick once we land here.
        self.surface_normal = n;
        self.pos = original_pos;
        self.on_ground = false;
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
        let n = self.surface_normal;
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
        let mut body = ae::KinematicBody {
            pos: self.pos,
            vel: self.vel,
            size: self.size,
            on_ground: self.on_ground,
            facing: self.facing,
        };
        ae::step_kinematic(
            &mut body,
            world,
            ae::KinematicTuning {
                gravity: ENEMY_GRAVITY,
                max_fall_speed: ENEMY_MAX_FALL,
            },
            ae::KinematicInputs {
                drop_through: false,
            },
            dt,
        );
        self.pos = body.pos;
        self.vel = body.vel;
        self.on_ground = body.on_ground;
        if body.on_ground {
            // Re-pin to a floor surface — the slug forgets it was
            // ever on a wall once it lands.
            self.surface_normal = ae::Vec2::new(0.0, -1.0);
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

    /// Begin a melee attack windup + cooldown. Called by the EFFECTS
    /// consumer `start_enemy_melee_from_brain_actions` in response
    /// to an `ActorActionMessage::Melee`. Returns `true` if the
    /// attack actually started (the cooldown gate passed). Sandbag
    /// archetypes deliberately accept this — their PunchWeak counter
    /// is the legitimate use case.
    pub fn begin_melee_attack(&mut self, tuning: FeatureCombatTuning) -> bool {
        if self.attack_cooldown > 0.0 || !self.alive {
            return false;
        }
        self.attack_windup_timer = tuning.enemy_attack_windup.max(0.01);
        self.attack_cooldown = ENEMY_ATTACK_COOLDOWN
            * if self.archetype == EnemyArchetype::SmallSkitter {
                0.75
            } else if self.archetype == EnemyArchetype::LargeBrute {
                1.35
            } else {
                1.0
            };
        self.ai_mode = ae::CharacterAiMode::Telegraph;
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
        if self.archetype.is_sandbag()
            || matches!(
                self.archetype,
                EnemyArchetype::PirateOnShark | EnemyArchetype::PirateHeavyOnShark
            )
        {
            return None;
        }
        // PuppySlug: contact damage stays on (its only damage
        // source). PirateHeavy: contact damage off (cove crew).
        if !self.archetype.attacks_player() && self.archetype != EnemyArchetype::PuppySlug {
            return None;
        }
        Some(self.aabb())
    }

    /// Polled body-contact damage check. The attack-swing arm moved
    /// to the `Hitbox` entity lifecycle (see
    /// `content/features/ecs/hitbox.rs`); body contact is "you ran
    /// into the enemy" — a per-tick test against the integration
    /// state, not a discrete strike — and keeps its polled shape.
    /// Per the actor/brain follow-up plan
    /// (`dev/journals/actor-brain-migration-followups-plan.md`,
    /// Task A "What stays in `EnemyRuntime`").
    pub(super) fn body_contact_damage(&self, player_body: ae::Aabb) -> Option<PlayerDamageEvent> {
        let body_damage = self.body_damage_aabb()?;
        if !body_damage.strict_intersects(player_body) {
            return None;
        }
        Some(PlayerDamageEvent {
            mode: PlayerDamageMode::Knockback,
            source: PlayerDamageSource::EnemyBody,
            source_pos: self.pos,
            impact_pos: midpoint(player_body.center(), body_damage.center()),
            knockback_dir: (player_body.center().x - self.pos.x).signum_or(self.facing),
            strength: self.archetype.contact_strength(),
            amount: self.archetype.damage_amount(),
            // Hostile body contact targets primary today; per-target
            // routing arrives with OVERNIGHT-TODO #17.6.
            target: None,
        })
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
}
