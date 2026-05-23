use super::*;
use crate::enemy_projectile::EnemyProjectileSpawn;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug)]
pub(super) struct EnemyArchetypeSpec {
    pub max_health: i32,
    pub rider_max_health: Option<i32>,
    pub patrol_speed: f32,
    pub chase_speed: f32,
    pub aggro_radius: f32,
    pub attack_range: f32,
    pub contact_strength: f32,
    pub damage_amount: i32,
    pub is_aerial: bool,
    pub is_sandbag: bool,
    pub default_size: Option<ae::Vec2>,
}

/// Table indexed by [`EnemyArchetype`]; built via a single `match`
/// over the enum so the compiler still flags every variant if we add
/// one. Aerial archetypes inherit `is_aerial: true` which feeds both
/// `gravity_scale` (zero gravity) and the combat slot kind
/// (`SlotKind::Aerial`).
const fn archetype_spec(arch: EnemyArchetype) -> EnemyArchetypeSpec {
    use EnemyArchetype::*;
    match arch {
        Combatant => EnemyArchetypeSpec {
            max_health: 4,
            rider_max_health: None,
            patrol_speed: ENEMY_PATROL_SPEED,
            chase_speed: ENEMY_CHASE_SPEED,
            aggro_radius: 460.0,
            attack_range: ENEMY_ATTACK_RANGE,
            contact_strength: 0.70,
            damage_amount: 1,
            is_aerial: false,
            is_sandbag: false,
            default_size: None,
        },
        SmallSkitter => EnemyArchetypeSpec {
            max_health: 2,
            rider_max_health: None,
            patrol_speed: 150.0,
            chase_speed: 210.0,
            aggro_radius: 320.0,
            attack_range: 105.0,
            contact_strength: 0.55,
            damage_amount: 1,
            is_aerial: false,
            is_sandbag: false,
            default_size: None,
        },
        SmallLurker => EnemyArchetypeSpec {
            max_health: 2,
            rider_max_health: None,
            patrol_speed: 60.0, // sluggish — that's the point
            chase_speed: 90.0,
            aggro_radius: 96.0, // tight — player can walk past
            attack_range: 90.0,
            contact_strength: 0.45,
            damage_amount: 1,
            is_aerial: false,
            is_sandbag: false,
            default_size: None,
        },
        MediumStriker => EnemyArchetypeSpec {
            max_health: 5,
            rider_max_health: None,
            patrol_speed: ENEMY_PATROL_SPEED,
            chase_speed: 170.0,
            aggro_radius: 460.0,
            attack_range: ENEMY_ATTACK_RANGE,
            contact_strength: 0.70,
            damage_amount: 1,
            is_aerial: false,
            is_sandbag: false,
            default_size: None,
        },
        LargeBrute => EnemyArchetypeSpec {
            max_health: 9,
            rider_max_health: None,
            patrol_speed: 72.0,
            chase_speed: 118.0,
            aggro_radius: 380.0,
            attack_range: 205.0,
            contact_strength: 1.25,
            damage_amount: 2,
            is_aerial: false,
            is_sandbag: false,
            default_size: None,
        },
        LargeColossus => EnemyArchetypeSpec {
            max_health: 14,
            rider_max_health: None,
            patrol_speed: 40.0, // barely moves; almost stationary
            chase_speed: 80.0,  // never sprints
            aggro_radius: 200.0, // narrow threat envelope
            attack_range: 240.0, // big arms reach further
            contact_strength: 1.50, // hits the hardest of any non-boss
            damage_amount: 3,
            is_aerial: false,
            is_sandbag: false,
            default_size: None,
        },
        AggressiveSeeker => EnemyArchetypeSpec {
            max_health: 4,
            rider_max_health: None,
            patrol_speed: 130.0,
            chase_speed: 225.0,
            aggro_radius: 900.0,
            attack_range: ENEMY_ATTACK_RANGE,
            contact_strength: 0.80,
            damage_amount: 1,
            is_aerial: false,
            is_sandbag: false,
            default_size: None,
        },
        InfiniteSandbag => EnemyArchetypeSpec {
            max_health: 9999,
            rider_max_health: None,
            patrol_speed: ENEMY_PATROL_SPEED,
            chase_speed: ENEMY_CHASE_SPEED,
            aggro_radius: 0.0,
            attack_range: ENEMY_ATTACK_RANGE,
            contact_strength: 0.70,
            damage_amount: 1,
            is_aerial: false,
            is_sandbag: true,
            default_size: None,
        },
        FiniteSandbag => EnemyArchetypeSpec {
            max_health: 6,
            rider_max_health: None,
            patrol_speed: ENEMY_PATROL_SPEED,
            chase_speed: ENEMY_CHASE_SPEED,
            aggro_radius: 0.0,
            attack_range: ENEMY_ATTACK_RANGE,
            contact_strength: 0.70,
            damage_amount: 1,
            is_aerial: false,
            is_sandbag: true,
            default_size: None,
        },
        PirateRaider => EnemyArchetypeSpec {
            max_health: 5,
            rider_max_health: None,
            patrol_speed: 130.0,
            chase_speed: 190.0,
            aggro_radius: 460.0,
            attack_range: 140.0,
            contact_strength: 0.85,
            damage_amount: 1,
            is_aerial: false,
            is_sandbag: false,
            default_size: Some(ae::Vec2::new(44.0, 78.0)),
        },
        BurningFlyingShark => EnemyArchetypeSpec {
            // Shark hp (the body pool); no rider on this dismounted form.
            max_health: 6,
            rider_max_health: None,
            patrol_speed: 110.0,
            // Aerial fly speed — used as the steering convergence rate
            // toward the choreography's engage position.
            chase_speed: 260.0,
            // Aerial archetypes spot the player from across the arena.
            aggro_radius: 1200.0,
            // For ranged actors `attack_range` is the AI "I am willing
            // to attack" gate; choreography decides the actual engage.
            attack_range: 200.0,
            contact_strength: 1.10,
            damage_amount: 2,
            is_aerial: true,
            is_sandbag: false,
            default_size: Some(ae::Vec2::new(108.0, 96.0)),
        },
        PirateOnShark => EnemyArchetypeSpec {
            // Shark hp (the body pool). Rider has its own pool — see
            // `rider_max_health`.
            max_health: 6,
            rider_max_health: Some(4),
            patrol_speed: 110.0,
            chase_speed: 230.0,
            aggro_radius: 1200.0,
            attack_range: 1100.0,
            contact_strength: 1.10,
            damage_amount: 2,
            is_aerial: true,
            is_sandbag: false,
            default_size: Some(ae::Vec2::new(108.0, 96.0)),
        },
        PirateHeavy => EnemyArchetypeSpec {
            // Cove crew member. Peaceful BY DEFAULT (aggro 0,
            // attack range 0) — the heavy paces back and forth at
            // patrol speed but never aggros, never swings, never
            // deals contact damage. Player can walk past safely.
            // Provoking-into-hostility is future work; right now
            // the archetype reads as "tanky bruiser silhouette,
            // crew disposition" until/unless we add a turn-hostile
            // event hook. The aerial PirateHeavyOnShark variant is
            // a separate archetype and keeps full aggression.
            max_health: 10,
            rider_max_health: None,
            patrol_speed: 75.0,
            chase_speed: 130.0,
            aggro_radius: 0.0,
            attack_range: 0.0,
            contact_strength: 0.0,
            damage_amount: 2,
            is_aerial: false,
            is_sandbag: false,
            // Sized to the variants' rendered silhouette
            // (broadside_bess: 172×138, iron_mary: 176×143,
            // salt_annet: 169×134). Collider uses the bess midpoint
            // so all three variants share a hitbox; tuning by
            // variant can come later if combat asks for it.
            default_size: Some(ae::Vec2::new(72.0, 110.0)),
        },
        PirateHeavyOnShark => EnemyArchetypeSpec {
            // Aerial fused actor. Same envelope as PirateOnShark
            // but with a chunkier rider — heavier melee on the body
            // contact, bigger rider HP pool, slightly slower orbit
            // because the heavy weight drags the shark.
            max_health: 7,
            rider_max_health: Some(6),
            patrol_speed: 110.0,
            chase_speed: 215.0,
            aggro_radius: 1200.0,
            attack_range: 1100.0,
            contact_strength: 1.30,
            damage_amount: 3,
            is_aerial: true,
            is_sandbag: false,
            default_size: Some(ae::Vec2::new(108.0, 96.0)),
        },
        PuppySlug => EnemyArchetypeSpec {
            // Crawlid-style grunt: 1 HP, slow, body-contact damage,
            // never aggros (aggro_radius = 0 → AI stays in Patrol).
            // chase_speed is unused but kept non-zero so any future
            // promotion to "panic-on-hit chase" reads sensibly.
            max_health: 1,
            rider_max_health: None,
            patrol_speed: 55.0,
            chase_speed: 80.0,
            aggro_radius: 0.0,
            attack_range: 0.0,
            contact_strength: 0.55,
            damage_amount: 1,
            is_aerial: false,
            is_sandbag: false,
            // Match the rendered sheet's body proportions (puppy_slug
            // sprite is 128×95 with body bbox ~120×31). The collider
            // hugs the dorsal-ridge silhouette.
            default_size: Some(ae::Vec2::new(48.0, 22.0)),
        },
    }
}

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
            Combatant
            | SmallSkitter
            | SmallLurker
            | MediumStriker
            | AggressiveSeeker
            | PuppySlug
            | PirateRaider
            | BurningFlyingShark
            | InfiniteSandbag
            | FiniteSandbag => EnemyRespawnPolicy::OnRoomReenter,
            LargeBrute
            | LargeColossus
            | PirateHeavy
            | PirateOnShark
            | PirateHeavyOnShark => EnemyRespawnPolicy::OnRest,
        }
    }
}

/// Per-tick outputs the caller (`update_ecs_actors`) flushes into
/// world resources. Today this is just enemy-fired projectile spawn
/// requests; future patterns (telegraph SFX events, area-of-effect
/// hazards) will land here too without further signature churn.
#[derive(Default)]
pub struct EnemyTickOutputs {
    pub projectile_spawns: Vec<EnemyProjectileSpawn>,
}

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
    pub(super) fn update(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: FeatureCombatTuning,
        slot_pos: Option<ae::Vec2>,
        nearest_neighbor: Option<ae::Vec2>,
        outputs: &mut EnemyTickOutputs,
        dt: f32,
    ) {
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
            return;
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

        let frame = self.build_control_frame(&ai, &choreo_tick, target_pos, is_aerial, dt);

        if is_surface_walker {
            // Surface-walker integration: the slug crawls along any
            // surface (floor / wall / ceiling), wrapping around
            // convex corners and turning into concave ones. Bypasses
            // `step_kinematic` entirely — that primitive bakes in
            // gravity-down + axis-aligned solid sweeps, which is
            // wrong on rotated surfaces. The custom step has its own
            // ledge / wall / fall handling.
            self.step_surface_walker(world, dt);
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
        if frame.melee_pressed
            && !self.archetype.is_sandbag()
            && self.archetype.attacks_player()
            && self.attack_cooldown <= 0.0
        {
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
        }
        if let Some(fire) = frame.fire {
            // PirateOnShark fires from the rider's hand (where the
            // visible gun-sword's muzzle sits) so the projectile
            // looks like it's leaving the barrel. The `lasersword:`
            // prefix on `owner_id` routes the projectile to the
            // lasersword visual in `enemy_projectile/visuals.rs`.
            let (origin, owner_id) = if matches!(
                self.archetype,
                EnemyArchetype::PirateOnShark | EnemyArchetype::PirateHeavyOnShark
            ) {
                let hand = crate::presentation::rendering::rider_hand_world_pos(
                    self.pos,
                    self.facing,
                );
                let muzzle_origin = hand + fire.dir.normalize_or_zero() * 18.0;
                (muzzle_origin, format!("lasersword:{}", self.id))
            } else {
                (self.pos + ae::Vec2::new(0.0, -8.0), self.id.clone())
            };
            outputs.projectile_spawns.push(EnemyProjectileSpawn {
                origin,
                dir: fire.dir,
                speed: fire.speed,
                damage: self.archetype.damage_amount(),
                max_lifetime: 2.4,
                half_extent: ae::Vec2::new(10.0, 8.0),
                owner_id,
                gravity: 0.0,
            });
            // Recoil kick: push the firing actor backward along the
            // negative fire direction. For the PirateOnShark this
            // shoves both rider and shark (one fused actor) so the
            // discharge reads as a noticeable knock-back.
            let recoil_strength = if matches!(
                self.archetype,
                EnemyArchetype::PirateOnShark | EnemyArchetype::PirateHeavyOnShark
            ) {
                ENEMY_FIRE_RECOIL_PIRATE
            } else {
                ENEMY_FIRE_RECOIL_DEFAULT
            };
            let kick = fire.dir.normalize_or_zero() * -recoil_strength;
            self.vel += kick;
            // Brief telegraph for the HUD so the volley reads as a "shot".
            self.ai_mode = ae::CharacterAiMode::Attack;
        }
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
        let size = if self.archetype == EnemyArchetype::PuppySlug
            && self.surface_normal.x.abs() > 0.5
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
    fn step_surface_walker(&mut self, world: &ae::World, dt: f32) {
        let n = self.surface_normal;
        let speed = self.archetype.patrol_speed();
        let step_len = speed * dt;
        // tangent_base = 90° math-CCW rotation of normal; multiply by
        // facing to pick a direction along the surface.
        let tangent =
            ae::Vec2::new(-n.y * self.facing, n.x * self.facing);
        let body_long = self.size.x * 0.5;
        let body_thick = self.size.y * 0.5;

        // ---- 1. Pre-step wall check -------------------------------
        if self.wall_ahead(world, tangent, body_long, body_thick) {
            // Rotate CW (math): (x, y) → (y, -x). Picks the
            // orientation where the obstructing surface becomes the
            // new "down".
            self.surface_normal = ae::Vec2::new(n.y, -n.x);
            if self.snap_pos_to_surface(world) {
                self.vel = ae::Vec2::ZERO;
                self.on_ground = true;
                return;
            }
            // Snap failed (no actual surface in the CW direction —
            // edge case, e.g. a single-tile-wide bump). Revert and
            // try the step anyway.
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

        // ---- 4. Try CCW rotation (convex wrap) --------------------
        // Slug walked off the surface in the direction of motion;
        // the wall just past the corner is the new surface. Before
        // snapping, slide pos a half-body along the OLD tangent +
        // OLD -normal direction — that puts the slug center "around
        // the corner" so the snap cast in the new -normal direction
        // can actually hit the new wall face.
        let ccw_normal = ae::Vec2::new(-n.y, n.x);
        let around_corner = original_pos
            + tangent * body_long
            + (-n) * body_long;
        self.pos = around_corner;
        self.surface_normal = ccw_normal;
        if self.snap_pos_to_surface(world) {
            self.vel = ae::Vec2::ZERO;
            self.on_ground = true;
            return;
        }

        // ---- 5. Try CW rotation (back-wrap) -----------------------
        // Very rare — the slug ran straight into a concave corner
        // that the pre-step probe missed because the geometry sat
        // just outside the probe envelope. Try the opposite
        // rotation from the same pre-step pos.
        self.pos = original_pos;
        self.surface_normal = ae::Vec2::new(n.y, -n.x);
        if self.snap_pos_to_surface(world) {
            self.vel = ae::Vec2::ZERO;
            self.on_ground = true;
            return;
        }

        // ---- 6. Fall -----------------------------------------------
        self.surface_normal = n;
        self.pos = original_pos;
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
            ae::KinematicInputs { drop_through: false },
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
        if !self.archetype.attacks_player()
            && self.archetype != EnemyArchetype::PuppySlug
        {
            return None;
        }
        Some(self.aabb())
    }

    pub(super) fn player_damage(&self, player_body: ae::Aabb) -> Option<PlayerDamageEvent> {
        if self.attack_timer > 0.0 && self.attack_aabb().strict_intersects(player_body) {
            return Some(PlayerDamageEvent {
                mode: PlayerDamageMode::Knockback,
                source: PlayerDamageSource::EnemyAttack,
                source_pos: self.pos,
                impact_pos: midpoint(player_body.center(), self.attack_aabb().center()),
                knockback_dir: (player_body.center().x - self.pos.x).signum_or(self.facing),
                strength: 1.0,
                amount: 1,
                // Enemy AI targets primary at the call site
                // (`PrimaryPlayerOnly` in update_ecs_actors); leave on
                // the legacy primary-receives path until #17.8 lands
                // per-target enemy AI.
                target: None,
            });
        }
        if let Some(body_damage) = self.body_damage_aabb() {
            if body_damage.strict_intersects(player_body) {
                return Some(PlayerDamageEvent {
                    mode: PlayerDamageMode::Knockback,
                    source: PlayerDamageSource::EnemyBody,
                    source_pos: self.pos,
                    impact_pos: midpoint(player_body.center(), body_damage.center()),
                    knockback_dir: (player_body.center().x - self.pos.x).signum_or(self.facing),
                    strength: self.archetype.contact_strength(),
                    amount: self.archetype.damage_amount(),
                    // Same as the attack arm: enemy body contact is
                    // resolved against the primary player at the call
                    // site filter.
                    target: None,
                });
            }
        }
        None
    }
}
