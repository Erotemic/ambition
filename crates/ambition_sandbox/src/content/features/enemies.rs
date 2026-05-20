use super::*;
use crate::enemy_projectile::EnemyProjectileSpawn;

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
    /// Set by [`step_kinematic`] each tick. Used by chase-drop-through
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
}

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
    }
}

impl EnemyArchetype {
    /// All combat-capable archetypes in a stable order. Useful for
    /// tests / tooling that want to iterate every variant; the
    /// sandbag training dummies are *not* in this list because they
    /// don't run the standard combat AI loop.
    pub const COMBAT_ALL: [Self; 10] = [
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
            Self::PirateOnShark => ae::AttackChoreography::AerialOrbitAndFire {
                altitude: 150.0,
                radius: 220.0,
                orbit_speed: 0.9,
                fire_interval: 1.4,
                projectile_speed: 380.0,
            },
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
        if self.archetype == EnemyArchetype::PirateOnShark {
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
            // If this was a fused pirate-on-shark, the shark died —
            // dismount the rider into a grounded pirate.
            if self.archetype == EnemyArchetype::PirateOnShark {
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

    /// Shark died: actor becomes a grounded pirate. Pirate inherits
    /// the rider hp pool (or starts at the grounded-pirate default
    /// if the rider had already died — which shouldn't happen since
    /// the actor would already be a BurningFlyingShark by then).
    fn dismount_shark(&mut self) -> EnemyDamageOutcome {
        let inherited_hp = self
            .rider_health
            .filter(|h| h.alive())
            .map(|h| h.current)
            .unwrap_or_else(|| EnemyArchetype::PirateRaider.max_health());
        self.archetype = EnemyArchetype::PirateRaider;
        self.choreography = self.archetype.choreography();
        self.choreography_state = ae::ChoreographyState::default();
        self.health = ae::Health::new(EnemyArchetype::PirateRaider.max_health());
        self.health.current = inherited_hp.min(self.health.max);
        self.rider_health = None;
        self.gravity_scale = 1.0;
        if let Some(default_size) = EnemyArchetype::PirateRaider.default_size() {
            self.size = default_size;
        }
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

    pub(super) fn update(
        &mut self,
        world: &ae::World,
        player: &ae::Player,
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

        let delta_to_player = player.pos - self.pos;
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
            player_pos: player.pos,
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
        let assigned_slot_pos = slot_pos.unwrap_or(player.pos);
        self.choreography_state.has_slot = slot_pos.is_some();
        let choreo_tick = ae::evaluate_choreography(
            self.choreography,
            &mut self.choreography_state,
            ae::ChoreographyInput {
                actor_pos: self.pos,
                target_pos: player.pos,
                assigned_slot_pos,
                dt,
                nearest_neighbor,
            },
        );

        // The engine CharacterAI output is now authoritative for the coarse
        // behavior decision. Sandbox code supplies archetype speeds and
        // collision, but no longer has a second, parallel set of
        // Guard/Custom/Patrol/attack-range branches.
        let is_aerial = self.gravity_scale <= 0.001;
        if let Some(motion) = &mut self.motion {
            if matches!(ai.intent, ae::CharacterAiIntent::Patrol) {
                let old = self.pos;
                self.pos = motion.advance(self.pos, dt);
                let delta = self.pos - old;
                self.vel = if dt > 0.0 { delta / dt } else { ae::Vec2::ZERO };
                self.facing = delta.x.signum_or(self.facing);
            } else {
                self.vel = ae::Vec2::ZERO;
            }
        } else if is_aerial {
            // Aerial flight: steer toward the choreography's target
            // position. No world collision (sky arenas are open) and
            // no gravity — the steering target *is* the path.
            // The choreography may override the convergence speed
            // (e.g. DiveStrike during the dive phase uses
            // `dive_speed` instead of `chase_speed`); fall back to
            // the archetype default otherwise.
            let steering_speed = choreo_tick
                .steering_speed_override
                .unwrap_or_else(|| self.archetype.chase_speed());
            let to_target = choreo_tick.steering_target - self.pos;
            let dist = to_target.length();
            let desired_vel = if dist > 1.0 {
                (to_target / dist) * steering_speed
            } else {
                ae::Vec2::ZERO
            };
            // Match accel to the steering speed so a faster dive
            // also ramps faster — otherwise an aerial actor at
            // 3x speed would still take 3x as long to converge.
            let accel = (steering_speed * 3.0).max(900.0) * dt;
            self.vel.x = approach(self.vel.x, desired_vel.x, accel);
            self.vel.y = approach(self.vel.y, desired_vel.y, accel);
            self.pos += self.vel * dt;
            self.on_ground = false;
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
                    // Steering target comes from the choreography
                    // (slot-aware) instead of the raw chase direction
                    // toward the player. Anti-clump behavior: enemies
                    // without a slot stand off; enemies with a slot
                    // approach the slot, not the player center.
                    let dx = choreo_tick.steering_target.x - self.pos.x;
                    let sign = if dx.abs() < 1.0 { 0.0 } else { dx.signum() };
                    let speed = choreo_tick
                        .steering_speed_override
                        .unwrap_or_else(|| self.archetype.chase_speed());
                    sign * speed
                }
            };
            self.vel.x = approach(self.vel.x, desired_x, 650.0 * dt);

            // Chase-drop-through: when actively chasing a player who is
            // meaningfully BELOW us, AND we're currently standing on something,
            // suppress the OneWay vertical block this tick so we follow the
            // player through the same platform.
            let drop_through = matches!(ai.intent, ae::CharacterAiIntent::Chase { .. })
                && self.on_ground
                && delta_to_player.y > 48.0;

            let mut body = ae::KinematicBody {
                pos: self.pos,
                vel: self.vel,
                size: self.size,
                on_ground: self.on_ground,
                facing: self.facing,
            };
            let prev_vel_x = body.vel.x;
            ae::step_kinematic(
                &mut body,
                world,
                ae::KinematicTuning {
                    gravity: ENEMY_GRAVITY * self.gravity_scale,
                    max_fall_speed: ENEMY_MAX_FALL,
                },
                ae::KinematicInputs { drop_through },
                dt,
            );
            self.pos = body.pos;
            self.vel = body.vel;
            self.on_ground = body.on_ground;

            if matches!(ai.intent, ae::CharacterAiIntent::Patrol)
                && prev_vel_x.abs() > 1.0
                && self.vel.x.abs() < 0.01
            {
                self.facing *= -1.0;
            }
        }

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

        // Translate the choreography's action request into either a
        // melee wind-up (legacy path) or a projectile spawn (new
        // ranged path).
        match choreo_tick.action {
            Some(ae::ChoreographyAction::Melee) => {
                if !self.archetype.is_sandbag() && self.attack_cooldown <= 0.0 {
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
            }
            Some(ae::ChoreographyAction::FireProjectile { dir, speed }) => {
                outputs.projectile_spawns.push(EnemyProjectileSpawn {
                    origin: self.pos + ae::Vec2::new(0.0, -8.0),
                    dir,
                    speed,
                    damage: self.archetype.damage_amount(),
                    max_lifetime: 2.4,
                    half_extent: ae::Vec2::new(10.0, 8.0),
                    owner_id: self.id.clone(),
                });
                // Brief telegraph for the HUD so the volley reads as a "shot".
                self.ai_mode = ae::CharacterAiMode::Attack;
            }
            None => {}
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
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
        if self.archetype.is_sandbag() || self.archetype == EnemyArchetype::PirateOnShark {
            None
        } else {
            Some(self.aabb())
        }
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
