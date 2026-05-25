use super::*;

// Boss policy vocabulary (`BossMovementProfile`, `BossPatternStep`,
// `BossPattern`, `BossAttackPattern`, `BossAttackProfile`,
// `step_duration`) moved to `crate::brain::boss_pattern` per the
// "move boss policy out of BossRuntime" migration. Re-exported here
// because `BossBehaviorProfile` and the volumes / construction code
// below still reference them by their old `content::features::bosses`
// path — those references stay legal via the re-export while call
// sites migrate to the brain-module path at their leisure.
pub use crate::brain::boss_pattern::{
    BossAttackPattern, BossAttackProfile, BossMovementProfile, BossPattern, BossPatternStep,
};

// `BossTickOutputs` (previously: `projectile_spawns: Vec<…>`) was
// deleted with Task B of the actor/brain follow-up plan. Apple-rain
// spawning moved to `spawn_gnu_apple_rain_from_special_messages` (an
// EFFECTS-stage consumer driven by `ActorActionMessage::Special`).
// Future boss specials follow the same pattern — one consumer per
// `SpecialActionSpec` variant — instead of accumulating side-channel
// `Vec`s the caller flushes.

/// Encounter id of the gnu_ton boss — derived from
/// `encounter_id_from_name("GNU-ton")`. Centralized so the boss
/// ActionSet wiring (which binds the boss's special slot to
/// `SpecialActionSpec::GnuAppleRain`) can string-match without
/// re-deriving the slug.
pub const GNU_TON_ENCOUNTER_ID: &str = "gnu_ton";

/// Apple-rain tuning consumed by the spawn-time `ActionSet` wiring
/// (spawn.rs binds these into `SpecialActionSpec::GnuAppleRain`).
/// The visual / collision constants (gravity, lifetime, half_extent,
/// spawn-height) live next to the EFFECTS consumer in
/// `content/features/ecs/brain_effects.rs` — the consumer is the
/// only thing that reads them, so they're local there instead of
/// a cross-module knob set.
pub const APPLE_RAIN_INTERVAL: f32 = 0.35;
pub const APPLE_RAIN_SPAWN_SPEED: f32 = 35.0;
pub const APPLE_RAIN_DAMAGE: i32 = 1;
/// Stable id prefix used by the visuals layer to switch the
/// flat-red-rectangle bullet shape to the apple sprite (red body +
/// green leaf + brown stem). Keep in sync with
/// `enemy_projectile::visuals::is_apple_owner`.
pub const GNU_TON_APPLE_OWNER_PREFIX: &str = "gnu_ton_apple";

const GNU_TON_COLLISION_SCALE: f32 = 4.5;
const GNU_TON_FRAME_HEIGHT: f32 = 576.0;
// Design-space anchor sits at the shoulder ridge (REST_BODY_Y 60 - 62 = -2)
// in the regenerated 768×576 sprite. Must stay in lockstep with
// `boss_encounter::sprites::GNU_TON_SHEET::feet_anchor_y`.
const GNU_TON_ANCHOR_Y: f32 = -2.0;

fn gnu_ton_sprite_scale(collision_size: ae::Vec2) -> f32 {
    collision_size.x.max(collision_size.y).max(8.0) * GNU_TON_COLLISION_SCALE / GNU_TON_FRAME_HEIGHT
}

/// Live sandbox-side behavior tuning for a boss. This is deliberately separate
/// from `ae::BossEncounterSpec`: the engine spec owns phase progression and HP
/// thresholds, while this profile owns sandbox movement, contact size, damage,
/// and hitbox shapes.
#[derive(Clone, Debug, PartialEq)]
pub struct BossBehaviorProfile {
    pub id: String,
    pub combat_size: Option<ae::Vec2>,
    pub movement: BossMovementProfile,
    pub attacks: Vec<BossAttackProfile>,
    pub attack_cooldown: f32,
    pub attack_windup: f32,
    pub attack_active: f32,
    pub attack_damage: i32,
    pub body_damage: i32,
    /// How attack hitboxes are selected. `Cycle` (default for legacy bosses)
    /// rotates through `attacks` using the flat durations above. `Scripted`
    /// runs an authored phase-keyed timeline of telegraph / strike / rest
    /// beats and ignores `attacks` / `attack_cooldown` / `attack_windup` /
    /// `attack_active`.
    pub attack_pattern: BossAttackPattern,
    /// World-space anchor offset (in pixels) from the boss center where
    /// "hand"-class attacks should originate. For body-centered giants
    /// (GNU-ton) the entity transform sits at the scholar on the shoulder,
    /// not the giant's body — without this offset, hand hitboxes would
    /// hover near the scholar instead of where the giant's arms are. Y is
    /// world-space positive-down; leave at `Vec2::ZERO` for ordinary bosses.
    pub attack_origin_offset: ae::Vec2,
}

impl BossBehaviorProfile {
    pub fn clockwork_warden() -> Self {
        Self {
            id: "clockwork_warden".into(),
            combat_size: None,
            movement: BossMovementProfile::AnchorSway {
                x_radius: 130.0,
                y_bob: 18.0,
                x_frequency: 0.72,
                y_frequency: 1.10,
                chase_scale: 0.18,
                chase_limit: 70.0,
                speed: 220.0,
            },
            attacks: vec![
                BossAttackProfile::FloorSlam,
                BossAttackProfile::SideSweep,
                BossAttackProfile::FullBodyPulse,
            ],
            attack_cooldown: BOSS_ATTACK_COOLDOWN,
            attack_windup: 0.52,
            attack_active: 0.32,
            attack_damage: 2,
            body_damage: 1,
            attack_pattern: BossAttackPattern::Cycle,
            attack_origin_offset: ae::Vec2::ZERO,
        }
    }

    pub fn mockingbird() -> Self {
        Self {
            id: "mockingbird".into(),
            combat_size: Some(ae::Vec2::new(500.0, 185.0)),
            movement: BossMovementProfile::AirSwoop {
                x_radius: 250.0,
                y_radius: 62.0,
                x_frequency: 0.56,
                y_frequency: 1.35,
                chase_scale: 0.08,
                chase_limit: 95.0,
                speed: 320.0,
            },
            attacks: vec![
                BossAttackProfile::WingSweep,
                BossAttackProfile::DiveLane,
                BossAttackProfile::Broadside,
            ],
            attack_cooldown: 1.05,
            attack_windup: 0.44,
            attack_active: 0.28,
            attack_damage: 2,
            body_damage: 1,
            attack_pattern: BossAttackPattern::Cycle,
            attack_origin_offset: ae::Vec2::ZERO,
        }
    }

    /// GNU-ton: stationary giant with wide-ranging hand attacks.
    ///
    /// Unique pacing among bosses: scripted timeline with explicit *rest*
    /// beats between strikes so the player can read each windup, react,
    /// and learn the sequence. The other bosses (clockwork_warden,
    /// mockingbird) keep the fast `Cycle` rhythm — the contrast itself is
    /// the design intent. GNU-ton should feel like a slow, deliberate
    /// monolith; the other bosses feel like dueling opponents.
    ///
    /// Phase pacing (longer than other bosses by design):
    /// - Intro: single show-of-force slam (no rest after) to set tone
    /// - Phase 1: ~9s — slam → rest → sweep → rest → slam → long rest
    /// - Transition: ~3s pure rest (player gets a breath)
    /// - Phase 2: ~12s — adds head-descent windows where the head is
    ///   exposed and vulnerable, framed by long rests so the player
    ///   can punish during the descent and then reset
    /// - Enrage: ~8s — shockwave + double slam, shorter rests
    pub fn gnu_ton() -> Self {
        Self {
            id: "gnu_ton".into(),
            // The sprite is huge, but the boss entity itself is anchored to
            // the shoulder ridge under the scholar. GNU-ton's damaging and
            // vulnerable regions are generated from named sprite parts, so
            // this combat size is only the movement/placeholder envelope.
            combat_size: Some(ae::Vec2::new(220.0, 220.0)),
            movement: BossMovementProfile::StationaryGiant {
                sway_amplitude: 6.0,
                sway_frequency: 0.28,
                speed: 40.0,
            },
            // Legacy `attacks` is unused for Scripted bosses — keep it for
            // diagnostics so `boss inspect` style tooling can still list
            // the attack vocabulary.
            attacks: vec![
                BossAttackProfile::GnuHandSlam,
                BossAttackProfile::GnuHandSweep,
                BossAttackProfile::GnuHeadDescent,
                BossAttackProfile::GnuShockwave,
                BossAttackProfile::GnuAppleRain,
            ],
            attack_cooldown: 0.0,
            attack_windup: 0.0,
            attack_active: 0.0,
            attack_damage: 2,
            body_damage: 0, // no contact damage from the offscreen body
            attack_pattern: BossAttackPattern::Scripted {
                intro: BossPattern {
                    steps: vec![
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 1.6,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.55,
                        },
                        BossPatternStep::Rest { duration: 1.4 },
                    ],
                },
                phase1: BossPattern {
                    steps: vec![
                        // Hand slam from above, long telegraph so the player
                        // sees the arms rise.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 1.6,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.55,
                        },
                        BossPatternStep::Rest { duration: 1.2 },
                        // Side sweep — a totally different motion / hitbox shape.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 1.4,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 0.50,
                        },
                        BossPatternStep::Rest { duration: 1.0 },
                        // Apple rain: the scholar gestures up and apples
                        // fall around the player. Strike window is long
                        // enough to drop ~4 apples at the chosen interval.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuAppleRain,
                            duration: 1.0,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuAppleRain,
                            duration: 2.2,
                        },
                        BossPatternStep::Rest { duration: 1.0 },
                        // Repeat the slam to reward memorizers.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 1.6,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.55,
                        },
                        // Long breather closes the cycle.
                        BossPatternStep::Rest { duration: 1.8 },
                    ],
                },
                transition: BossPattern {
                    steps: vec![BossPatternStep::Rest { duration: 3.0 }],
                },
                phase2: BossPattern {
                    steps: vec![
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 1.4,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.55,
                        },
                        BossPatternStep::Rest { duration: 1.0 },
                        // Long head-descent: head is the vulnerable target;
                        // duration matches the score so the music's
                        // "harpsichord exposure" beat lands in this window.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHeadDescent,
                            duration: 1.8,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHeadDescent,
                            duration: 1.4,
                        },
                        BossPatternStep::Rest { duration: 1.4 },
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 1.4,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 0.50,
                        },
                        BossPatternStep::Rest { duration: 2.0 },
                    ],
                },
                enrage: BossPattern {
                    steps: vec![
                        // Faster pace, no head exposure: it's punishing.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.90,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.45,
                        },
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 0.90,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 0.45,
                        },
                        BossPatternStep::Rest { duration: 0.6 },
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuShockwave,
                            duration: 1.10,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuShockwave,
                            duration: 0.70,
                        },
                        BossPatternStep::Rest { duration: 1.2 },
                    ],
                },
            },
            attack_origin_offset: ae::Vec2::ZERO,
        }
    }

    pub fn generic(id: impl Into<String>) -> Self {
        let mut profile = Self::clockwork_warden();
        profile.id = id.into();
        profile
    }

    pub fn for_authored_boss(id_or_name: &str) -> Self {
        let key = crate::boss_encounter::encounter_id_from_name(id_or_name);
        match key.as_str() {
            "mockingbird" => Self::mockingbird(),
            "clockwork_warden" | "gradient_sentinel" => Self::clockwork_warden(),
            "gnu_ton" => Self::gnu_ton(),
            other => Self::generic(other),
        }
    }
}

// `step_duration` moved to `crate::brain::boss_pattern`.

#[derive(Clone, Debug)]
pub struct BossRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub spawn: ae::Vec2,
    pub size: ae::Vec2,
    pub health: ae::Health,
    pub brain: ae::BossBrain,
    pub behavior: BossBehaviorProfile,
    pub alive: bool,
    pub pattern_timer: f32,
    pub movement_timer: f32,
    pub attack_windup_timer: f32,
    pub attack_timer: f32,
    pub attack_cooldown: f32,
    pub hit_flash: f32,
    /// Active encounter phase. Forwarded by `update_ecs_bosses` from
    /// `BossEncounterRegistry`. `Dormant` until the encounter wakes
    /// up. The brain reads this via `BossPatternContext`; pattern
    /// selection happens in the brain, not here.
    pub encounter_phase: ae::BossEncounterPhase,
    /// **Mirror of [`BossAttackState::active_profile`].** Written by
    /// `tick_boss_brains_system` after each brain tick so legacy
    /// readers (`attack_volumes`, `player_damage`, debug overlay) can
    /// keep reading off `BossRuntime`. Do not set this from inside
    /// `BossRuntime` — the brain owns the policy decision.
    pub active_strike_profile: Option<BossAttackProfile>,
    /// **Mirror of [`BossAttackState::telegraph_profile`].** Same
    /// "brain-written, runtime-readable" contract as
    /// `active_strike_profile`.
    pub telegraph_profile: Option<BossAttackProfile>,
}

impl BossRuntime {
    pub(crate) fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: ae::BossBrain,
    ) -> Self {
        let name = name.into();
        Self {
            id: id.into(),
            pos: aabb.center(),
            spawn: aabb.center(),
            size: aabb.half_size() * 2.0,
            health: ae::Health::new(18),
            behavior: BossBehaviorProfile::for_authored_boss(&name),
            name,
            brain,
            alive: true,
            pattern_timer: 0.0,
            movement_timer: 0.0,
            attack_windup_timer: 0.0,
            attack_timer: 0.0,
            attack_cooldown: 0.35,
            hit_flash: 0.0,
            encounter_phase: ae::BossEncounterPhase::Dormant,
            active_strike_profile: None,
            telegraph_profile: None,
        }
    }

    /// `target_pos` is populated from the boss entity's `ActorTarget`
    /// component by `select_actor_targets` (OVERNIGHT-TODO #17.8).
    /// The boss movement profile reads it for anchor-sway / air-swoop
    /// chase math; scripted patterns (`StationaryGiant`) ignore it.
    /// Integrate the boss's body using the brain-emitted `desired_vel`
    /// from `ActorControl`. **Integration only** — the brain
    /// (`tick_boss_brains_system` → `boss_pattern::tick_boss_pattern`)
    /// owns the policy decision and writes `ActorControl` upstream;
    /// this method only translates that desired velocity into a
    /// collision-resolved position change.
    ///
    /// `BossRuntime::update` (the old policy + integration combo)
    /// was deleted by the "move boss policy out of BossRuntime"
    /// migration. The runtime no longer ticks the scripted cursor
    /// or chooses Telegraph/Strike/Rest; it just integrates the
    /// velocity the brain produced.
    pub fn integrate_body(&mut self, world: &ae::World, desired_vel: ae::Vec2, dt: f32) {
        if !self.alive || dt <= 0.0 {
            return;
        }
        // Bosses float (gravity = 0, max_fall_speed = 0). Multi-part
        // bosses like GNU-ton expose a `combat_size` distinct from
        // the sprite `size`; that's the size we collide against.
        let mut body = ae::KinematicBody {
            pos: self.pos,
            vel: desired_vel,
            size: self.combat_size(),
            on_ground: false,
            facing: 1.0,
        };
        ae::step_kinematic(
            &mut body,
            world,
            ae::KinematicTuning {
                gravity: 0.0,
                max_fall_speed: 0.0,
            },
            ae::KinematicInputs {
                drop_through: false,
            },
            dt,
        );
        self.pos = body.pos;
        self.hit_flash = (self.hit_flash - dt).max(0.0);
    }

    /// Advance the free-running clocks the legacy
    /// `cycle_pattern_volumes` path reads (`pattern_timer`). The
    /// brain owns its own `movement_timer` inside `BossPatternState`;
    /// this clock is the runtime's tiny share that cycle-mode volume
    /// rendering still consults until Cycle volumes also migrate.
    pub fn tick_runtime_clocks(&mut self, dt: f32) {
        if !self.alive {
            return;
        }
        self.pattern_timer += dt;
        self.movement_timer += dt;
    }

    // `tick_apple_rain` was deleted with Task B of the actor/brain
    // follow-up plan. The spawn loop, golden-ratio x distribution,
    // and self-aabb dodge live in
    // `content/features/ecs/brain_effects.rs::spawn_gnu_apple_rain_from_special_messages`.
    // Per-boss accumulator state moved to the
    // `AppleRainSpawnState` component on the boss entity.

    // The old per-tick `update_scripted_attacks` / `update_cycle_attacks`
    // methods (cursor advancement, telegraph/strike/rest profile
    // choice, attack_timer/windup mirror) all moved to
    // `brain/boss_pattern.rs::tick_boss_pattern`. The boss tick
    // system (`content/features/ecs/bosses.rs::tick_boss_brains_system`)
    // mirrors the brain's chosen profile + remaining time into
    // `active_strike_profile` / `telegraph_profile` /
    // `attack_timer` / `attack_windup_timer` so legacy volume /
    // damage readers (`attack_volumes`, `attack_telegraph_volumes`,
    // `player_damage`) keep working without per-call changes.

    pub fn is_mockingbird(&self) -> bool {
        self.behavior.id == "mockingbird" || self.name.eq_ignore_ascii_case("mockingbird")
    }

    pub fn is_gnu_ton(&self) -> bool {
        self.behavior.id == "gnu_ton"
            || self.name.eq_ignore_ascii_case("gnu_ton")
            || self.name.eq_ignore_ascii_case("gnu-ton")
    }

    pub fn render_size(&self) -> ae::Vec2 {
        self.size
    }

    /// World-space anchor for a combat-banter speech bubble. For GNU-ton the
    /// scholar sits on the right shoulder — offset slightly right and not as
    /// high as the body top so the bubble appears near the character, not
    /// floating above the beast's head.
    pub fn bark_anchor(&self) -> ae::Vec2 {
        if self.is_gnu_ton() {
            let half_h = self.combat_size().y * 0.5;
            ae::Vec2::new(self.pos.x + 38.0, self.pos.y - half_h * 0.55 - 18.0)
        } else {
            let half_h = self.combat_size().y * 0.5;
            ae::Vec2::new(self.pos.x, self.pos.y - half_h - 20.0)
        }
    }

    pub fn apply_behavior_profile(&mut self, behavior: BossBehaviorProfile) {
        self.behavior = behavior;
    }

    pub fn combat_size(&self) -> ae::Vec2 {
        self.behavior.combat_size.unwrap_or(self.size)
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.combat_size() * 0.5)
    }

    pub fn attack_volumes(&self) -> Vec<ae::Aabb> {
        if self.attack_timer <= 0.0 {
            return Vec::new();
        }
        match &self.behavior.attack_pattern {
            BossAttackPattern::Cycle => self.cycle_pattern_volumes(),
            BossAttackPattern::Scripted { .. } => self
                .active_strike_profile
                .as_ref()
                .map(|profile| self.volumes_for(profile))
                .unwrap_or_default(),
        }
    }

    pub fn attack_telegraph_volumes(&self) -> Vec<ae::Aabb> {
        if self.attack_windup_timer <= 0.0 {
            return Vec::new();
        }
        match &self.behavior.attack_pattern {
            BossAttackPattern::Cycle => self.cycle_pattern_volumes(),
            BossAttackPattern::Scripted { .. } => self
                .telegraph_profile
                .as_ref()
                .map(|profile| self.volumes_for(profile))
                .unwrap_or_default(),
        }
    }

    pub fn body_damage_aabb(&self) -> ae::Aabb {
        self.aabb()
    }

    pub fn damageable_aabbs(&self) -> Vec<ae::Aabb> {
        if !self.is_gnu_ton() {
            return vec![self.aabb()];
        }
        // GNU-ton's head is the only vulnerable target, but it is ALWAYS
        // hittable — the head_descent windows just move it down to player
        // level so the player doesn't have to climb. Previously this
        // returned an empty list outside the GnuHeadDescent strike, which
        // made the boss permanently invulnerable in Phase1 (no descent
        // beat exists in that phase) and therefore unkillable, since HP
        // never dropped enough to unlock Phase2. Repro: spawn the boss
        // and watch it sit at full HP forever in Phase1.
        let head_design_y = if matches!(
            self.active_strike_profile,
            Some(BossAttackProfile::GnuHeadDescent)
        ) || matches!(
            self.telegraph_profile,
            Some(BossAttackProfile::GnuHeadDescent)
        ) {
            // Held-low position during the descent telegraph and strike.
            // Matches the generator's `_draw_head_down` target y=30.
            30.0
        } else {
            // Rest position high above the shoulder. Hard to reach but
            // not impossible — the player can climb the perches and jump.
            // Matches the generator's REST_HEAD_Y.
            -75.0
        };
        vec![self.gnu_ton_part_aabb(ae::Vec2::new(0.0, head_design_y), ae::Vec2::new(92.0, 74.0))]
    }

    /// Cycle-mode dispatch — picks the next attack profile from the flat
    /// `attacks` list using `pattern_timer / attack_cooldown` and renders
    /// its volumes via `volumes_for`. Used only when `attack_pattern` is
    /// `BossAttackPattern::Cycle`.
    pub(super) fn cycle_pattern_volumes(&self) -> Vec<ae::Aabb> {
        let attack_count = self.behavior.attacks.len().max(1);
        let phase = ((self.pattern_timer / self.behavior.attack_cooldown.max(0.05)) as usize)
            % attack_count;
        let attack = self
            .behavior
            .attacks
            .get(phase)
            .cloned()
            .unwrap_or(BossAttackProfile::FullBodyPulse);
        self.volumes_for(&attack)
    }

    /// Pure-data dispatch: given a specific attack profile, produce its
    /// world-space hitbox volumes. Ordinary bosses use
    /// `self.pos + attack_origin_offset`; GNU-ton overrides this path with
    /// part-specific boxes derived from the generated sprite design coordinates.
    pub(super) fn volumes_for(&self, attack: &BossAttackProfile) -> Vec<ae::Aabb> {
        if self.is_gnu_ton() {
            // Design-space anchors match the regenerated 768×576 GNU-ton
            // sprite: hands rest at x=±235, slam strike peaks at y=195
            // (below the leg hooves at design +175), shockwave fires at
            // floor level. Group B will replace these constants with
            // values pulled from `gnu_ton_boss_parts.json`.
            match attack {
                BossAttackProfile::GnuHandSlam => {
                    return vec![
                        self.gnu_ton_part_aabb(
                            ae::Vec2::new(-235.0, 195.0),
                            ae::Vec2::new(78.0, 60.0),
                        ),
                        self.gnu_ton_part_aabb(
                            ae::Vec2::new(235.0, 195.0),
                            ae::Vec2::new(78.0, 60.0),
                        ),
                    ];
                }
                BossAttackProfile::GnuHandSweep => {
                    return vec![
                        self.gnu_ton_part_aabb(
                            ae::Vec2::new(-185.0, 20.0),
                            ae::Vec2::new(140.0, 60.0),
                        ),
                        self.gnu_ton_part_aabb(
                            ae::Vec2::new(185.0, 20.0),
                            ae::Vec2::new(140.0, 60.0),
                        ),
                    ];
                }
                BossAttackProfile::GnuHeadDescent => {
                    return vec![
                        self.gnu_ton_part_aabb(ae::Vec2::new(0.0, 30.0), ae::Vec2::new(92.0, 74.0))
                    ];
                }
                BossAttackProfile::GnuShockwave => {
                    return vec![self
                        .gnu_ton_part_aabb(ae::Vec2::new(0.0, 195.0), ae::Vec2::new(300.0, 18.0))];
                }
                // Apple rain damage routes through the spawned projectile
                // bodies, not a stationary AABB on the boss. Returning
                // empty here keeps `player_damage` from double-counting
                // contact-on-boss while the apples are in flight, and
                // the debug overlay correctly draws no active strike
                // volume (the apples themselves are the threat).
                BossAttackProfile::GnuAppleRain => {
                    return Vec::new();
                }
                _ => {}
            }
        }
        let size = self.combat_size();
        let origin = self.pos + self.behavior.attack_origin_offset;
        match attack {
            BossAttackProfile::FloorSlam => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.5 + 22.0),
                ae::Vec2::new(size.x * 0.75, 18.0),
            )],
            BossAttackProfile::SideSweep => vec![
                ae::Aabb::new(
                    origin + ae::Vec2::new(-size.x * 0.50, 0.0),
                    ae::Vec2::new(size.x * 0.25, size.y * 0.72),
                ),
                ae::Aabb::new(
                    origin + ae::Vec2::new(size.x * 0.50, 0.0),
                    ae::Vec2::new(size.x * 0.25, size.y * 0.72),
                ),
            ],
            BossAttackProfile::FullBodyPulse => vec![ae::Aabb::new(origin, size * 0.70)],
            BossAttackProfile::WingSweep => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.08),
                ae::Vec2::new(size.x * 0.56, size.y * 0.42),
            )],
            BossAttackProfile::DiveLane => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.42),
                ae::Vec2::new(size.x * 0.22, size.y * 0.72),
            )],
            BossAttackProfile::Broadside => vec![
                ae::Aabb::new(
                    origin + ae::Vec2::new(-size.x * 0.34, 0.0),
                    ae::Vec2::new(size.x * 0.18, size.y * 0.84),
                ),
                ae::Aabb::new(
                    origin + ae::Vec2::new(size.x * 0.34, 0.0),
                    ae::Vec2::new(size.x * 0.18, size.y * 0.84),
                ),
            ],
            // GNU-ton: two giant hands slam down from the top of the arena.
            // Hitboxes appear at the far left and right of the combat zone,
            // extending from near the top down to the floor. (Unused for
            // gnu_ton bosses — they take the part-anchored branch above.)
            BossAttackProfile::GnuHandSlam => vec![
                ae::Aabb::new(
                    origin + ae::Vec2::new(-size.x * 0.40, size.y * 0.25),
                    ae::Vec2::new(size.x * 0.14, size.y * 0.60),
                ),
                ae::Aabb::new(
                    origin + ae::Vec2::new(size.x * 0.40, size.y * 0.25),
                    ae::Vec2::new(size.x * 0.14, size.y * 0.60),
                ),
            ],
            // GNU-ton: hands sweep from the far sides inward.
            // A wide horizontal hitbox covers most of the arena width at mid-height.
            BossAttackProfile::GnuHandSweep => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.15),
                ae::Vec2::new(size.x * 0.85, size.y * 0.28),
            )],
            // GNU-ton: the GNU head descends into player space.
            // Contact with the center-top region is dangerous; this is also
            // the window where the head becomes the vulnerable target.
            BossAttackProfile::GnuHeadDescent => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.05),
                ae::Vec2::new(size.x * 0.32, size.y * 0.38),
            )],
            // GNU-ton: shockwave when both hands meet in the center.
            // Floor-level shockwave spanning the full arena width.
            BossAttackProfile::GnuShockwave => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.48),
                ae::Vec2::new(size.x * 0.90, size.y * 0.08),
            )],
            // Apple rain damages via spawned projectiles, not a static
            // AABB on the boss. Empty here mirrors the gnu_ton branch
            // above so ordinary bosses that ever inherit the profile
            // (none today) behave the same.
            BossAttackProfile::GnuAppleRain => Vec::new(),
        }
    }

    fn gnu_ton_part_aabb(&self, design_center: ae::Vec2, design_half_size: ae::Vec2) -> ae::Aabb {
        let scale = gnu_ton_sprite_scale(self.size);
        let center = self.pos
            + ae::Vec2::new(
                design_center.x * scale,
                (design_center.y - GNU_TON_ANCHOR_Y) * scale,
            );
        ae::Aabb::new(center, design_half_size * scale)
    }

    pub(super) fn player_damage(&self, player_body: ae::Aabb) -> Option<PlayerDamageEvent> {
        if self.attack_timer > 0.0 {
            if let Some(volume) = self
                .attack_volumes()
                .into_iter()
                .find(|volume| volume.strict_intersects(player_body))
            {
                return Some(PlayerDamageEvent {
                    mode: PlayerDamageMode::Knockback,
                    source: PlayerDamageSource::BossAttack,
                    source_pos: self.pos,
                    impact_pos: midpoint(player_body.center(), volume.center()),
                    knockback_dir: (player_body.center().x - self.pos.x).signum_or(1.0),
                    strength: 1.25,
                    amount: self.behavior.attack_damage.max(1),
                    // Boss AI targets primary player (PrimaryPlayerOnly
                    // at the call site) — leave routing on the legacy
                    // primary-receives path until #17.8 lands per-target
                    // AI.
                    target: None,
                });
            }
        }
        let body_damage_amount = self.behavior.body_damage;
        if body_damage_amount > 0 {
            let body_damage = self.body_damage_aabb();
            if body_damage.strict_intersects(player_body) {
                return Some(PlayerDamageEvent {
                    mode: PlayerDamageMode::Knockback,
                    source: PlayerDamageSource::BossBody,
                    source_pos: self.pos,
                    impact_pos: midpoint(player_body.center(), body_damage.center()),
                    knockback_dir: (player_body.center().x - self.pos.x).signum_or(1.0),
                    strength: 1.0,
                    amount: body_damage_amount,
                    // Same as the attack arm: boss body contact routes
                    // to primary via the call-site filter.
                    target: None,
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod scripted_pattern_tests {
    use super::*;
    use ambition_engine as ae;

    fn gnu_ton_runtime() -> BossRuntime {
        let behavior = BossBehaviorProfile::gnu_ton();
        let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(220.0, 220.0));
        let pos = ae::Vec2::new(500.0, 400.0);
        let aabb = ae::Aabb::new(pos, combat_size * 0.5);
        let mut runtime = BossRuntime::new("boss_gnu_ton", "GNU-ton", aabb, ae::BossBrain::Dormant);
        runtime.behavior = behavior;
        runtime.encounter_phase = ae::BossEncounterPhase::Phase1;
        runtime
    }

    #[test]
    fn gnu_ton_pattern_includes_explicit_rest_beats_in_every_phase() {
        let BossAttackPattern::Scripted {
            phase1,
            transition,
            phase2,
            enrage,
            ..
        } = BossBehaviorProfile::gnu_ton().attack_pattern
        else {
            panic!("gnu_ton must use a Scripted attack pattern");
        };
        for (label, pattern) in [
            ("phase1", &phase1),
            ("transition", &transition),
            ("phase2", &phase2),
            ("enrage", &enrage),
        ] {
            let has_rest = pattern
                .steps
                .iter()
                .any(|step| matches!(step, BossPatternStep::Rest { .. }));
            assert!(
                has_rest,
                "{label} pattern must include at least one Rest beat so the \
                 player has breathing room — got steps {:?}",
                pattern.steps
            );
        }
    }

    #[test]
    fn gnu_ton_phase1_is_materially_longer_than_other_bosses() {
        let gnu_phase1 = match BossBehaviorProfile::gnu_ton().attack_pattern {
            BossAttackPattern::Scripted { phase1, .. } => phase1.total_duration(),
            _ => unreachable!(),
        };
        let warden = BossBehaviorProfile::clockwork_warden();
        let warden_cycle = warden.attack_windup + warden.attack_active + warden.attack_cooldown;
        assert!(
            gnu_phase1 > warden_cycle * 3.0,
            "gnu_ton phase1 ({gnu_phase1}s) should be much slower than the \
             clockwork warden cycle ({warden_cycle}s) — design intent is a \
             deliberate, memorizable rhythm"
        );
    }

    // `gnu_ton_scripted_advance_cycles_telegraph_strike_rest` deleted:
    // the cursor-through-steps invariant moved to
    // `brain::boss_pattern::tests::{boss_pattern_telegraph_step_updates_telegraph_profile_state,
    // boss_pattern_strike_step_emits_melee_intent,
    // boss_pattern_resets_cursor_on_phase_change}`. The runtime no
    // longer ticks the cursor (the brain does), so polling
    // `boss.update(...)` and reading `boss.telegraph_profile` is no
    // longer a meaningful exercise — those mirror fields are written
    // by the boss tick system, not advanced by the runtime.

    #[test]
    fn gnu_ton_hand_slam_anchors_to_drawn_hands() {
        // GNU-ton's transform sits on the shoulder ridge. Hand-slam
        // hitboxes should land *below* the shoulder (positive y) and on
        // opposite sides of it (one to the left, one to the right), no
        // matter how the sprite is resized. Earlier revisions pinned
        // these to absolute world-pixel thresholds (>400, >300) tuned to
        // a 384-tall frame; bumping the source PNG to 768×576 silently
        // broke the test even though the visual / hitbox correspondence
        // stayed correct. Stick to invariants instead of magic numbers.
        let boss = gnu_ton_runtime();
        let slam = boss.volumes_for(&BossAttackProfile::GnuHandSlam);
        assert_eq!(slam.len(), 2);
        let (left, right) = if slam[0].center().x < slam[1].center().x {
            (&slam[0], &slam[1])
        } else {
            (&slam[1], &slam[0])
        };
        assert!(left.center().x < boss.pos.x, "{slam:?}");
        assert!(right.center().x > boss.pos.x, "{slam:?}");
        assert!(left.center().y > boss.pos.y, "{slam:?}");
        assert!(right.center().y > boss.pos.y, "{slam:?}");
    }

    #[test]
    fn gnu_ton_body_contact_does_not_damage_player() {
        // `body_damage: 0` on the gnu_ton behavior is the authored
        // statement "no contact damage from the offscreen body". A prior
        // revision still dealt 1 damage because `player_damage` used
        // `body_damage.max(1)` after the intersect test. Guard the whole
        // block on `body_damage > 0` instead. Concrete repro: the player
        // hitbox identical to the boss body AABB must produce no event.
        let boss = gnu_ton_runtime();
        let player_body = boss.body_damage_aabb();
        assert!(
            boss.player_damage(player_body).is_none(),
            "gnu_ton must not deal contact damage when body_damage = 0"
        );
    }

    // `gnu_ton_scripted_patterns_skip_non_attacking_phases` deleted:
    // the "Dormant / Stagger / Death emit neutral intent + clear
    // attack-state mirror" invariant moved to
    // `brain::boss_pattern::tests::boss_pattern_brain_emits_neutral_in_non_attacking_phase`.
    // The runtime no longer chooses the pattern step, so polling
    // `boss.update(...)` and reading the mirror fields is no longer
    // the right exercise — the brain owns the gate.

    // The `gnu_ton_apple_rain_strike_emits_falling_apple_spawns`,
    // `gnu_ton_apple_rain_spawns_avoid_self_aabb`,
    // `gnu_ton_apple_rain_spawns_cover_full_arena_width`, and
    // `gnu_ton_apple_rain_resets_accumulator_when_strike_ends` tests
    // were deleted with Task B of the actor/brain follow-up plan.
    // They tested `BossRuntime::tick_apple_rain` directly, which no
    // longer exists. The same invariants (downward gravity, owner
    // prefix, self-aabb dodge, full-width coverage, reset-on-leave)
    // are now exercised in
    // `content/features/ecs/brain_effects.rs::tests` against the
    // EFFECTS consumer `spawn_gnu_apple_rain_from_special_messages`.

    #[test]
    fn gnu_ton_apple_rain_volumes_are_empty_so_contact_does_not_double_count() {
        // The strike's damage path goes through enemy projectiles, not
        // a stationary boss AABB. `volumes_for(GnuAppleRain)` must
        // return an empty list so the regular contact-damage check in
        // `player_damage` doesn't ALSO hit the player at the boss's
        // position while apples are in flight.
        let boss = gnu_ton_runtime();
        assert!(
            boss.volumes_for(&BossAttackProfile::GnuAppleRain)
                .is_empty(),
            "apple-rain volumes must be empty — damage routes through projectiles"
        );
    }

    #[test]
    fn gnu_ton_head_is_always_damageable_but_descent_brings_it_lower() {
        // The head is always a valid hit target — the older "only
        // damageable during head_descent strike" rule made the boss
        // permanently invulnerable in Phase1 (no descent beat) and
        // therefore unkillable. Now the head is always hittable; the
        // descent window just moves it down to player level so the
        // player doesn't have to climb. Both states must produce
        // exactly one head AABB.
        let mut boss = gnu_ton_runtime();
        let rest_head = boss.damageable_aabbs();
        assert_eq!(
            rest_head.len(),
            1,
            "head must always be a damageable target"
        );
        let rest_y = rest_head[0].center().y;
        // Rest head sits ABOVE the shoulder anchor (player must climb).
        assert!(
            rest_y < boss.pos.y,
            "rest head should be above the shoulder anchor, got y={rest_y} vs pos.y={}",
            boss.pos.y
        );

        boss.active_strike_profile = Some(BossAttackProfile::GnuHeadDescent);
        let descent_head = boss.damageable_aabbs();
        assert_eq!(descent_head.len(), 1);
        let descent_y = descent_head[0].center().y;
        // Descended head sits BELOW the shoulder anchor (at player level).
        assert!(
            descent_y > boss.pos.y,
            "descent head should be below the shoulder anchor"
        );
        // And materially lower than the rest position — that's the
        // whole point of the vulnerability window.
        assert!(
            descent_y > rest_y + 50.0,
            "descent must drop the head meaningfully (got rest_y={rest_y}, descent_y={descent_y})"
        );
    }

    /// Bosses used to write `self.pos` via a bespoke per-axis sweep
    /// against `boss_space_is_free`. With the brain→sim seam they
    /// run through the SAME `step_kinematic` primitive every other
    /// actor uses — so a wall placed in the chase path blocks them
    /// at the wall instead of relying on a parallel-but-different
    /// collision code path. This guards against future regressions
    /// where someone reintroduces a position-space write.
    #[test]
    fn boss_motion_respects_world_collision_against_a_wall() {
        let combat_size = ae::Vec2::new(80.0, 80.0);
        let spawn = ae::Vec2::new(200.0, 400.0);
        let aabb = ae::Aabb::new(spawn, combat_size * 0.5);
        let mut boss = BossRuntime::new(
            "test_warden",
            "Clockwork Warden",
            aabb,
            ae::BossBrain::Dormant,
        );
        boss.behavior = BossBehaviorProfile::clockwork_warden();
        boss.encounter_phase = ae::BossEncounterPhase::Phase1;
        // World: a wall at x=400 blocks any rightward chase past it.
        let world = ae::World::new(
            String::from("boss_collision_test"),
            ae::Vec2::new(1200.0, 800.0),
            ae::Vec2::new(100.0, 100.0),
            vec![
                ae::Block::solid(
                    String::from("floor"),
                    ae::Vec2::new(0.0, 760.0),
                    ae::Vec2::new(1200.0, 40.0),
                ),
                ae::Block::solid(
                    String::from("wall"),
                    ae::Vec2::new(400.0, 200.0),
                    ae::Vec2::new(40.0, 500.0),
                ),
            ],
        );
        // Place the player far to the right of the wall so the
        // AnchorSway profile pulls the boss as far right as its
        // chase_limit allows.
        let player_pos = ae::Vec2::new(1000.0, 400.0);
        // Build the brain cfg + state directly — the runtime no
        // longer ticks scripted attacks, so we drive
        // `tick_boss_pattern` ourselves and hand the resulting
        // `desired_vel` to `integrate_body`. This mirrors what
        // `tick_boss_brains_system` + `update_ecs_bosses` do in the
        // real schedule.
        use crate::brain::{
            tick_boss_pattern, BossPatternCfg, BossPatternContext, BossPatternState,
            BossAttackState,
        };
        let mut cfg = BossPatternCfg::neutral_test();
        cfg.aggressiveness = 1.0;
        cfg.pattern = boss.behavior.attack_pattern.clone();
        cfg.movement = boss.behavior.movement.clone();
        cfg.spawn = boss.spawn;
        cfg.combat_size = boss.combat_size();
        cfg.cycle_attack_windup = boss.behavior.attack_windup.max(0.01);
        cfg.cycle_attack_active = boss
            .behavior
            .attack_active
            .max(FeatureCombatTuning::default().boss_attack_active)
            .max(0.01);
        cfg.cycle_attack_cooldown = boss.behavior.attack_cooldown.max(0.05);
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let dt = 1.0 / 60.0;
        for _ in 0..600 {
            boss.tick_runtime_clocks(dt);
            let mut frame = ae::ActorControlFrame::neutral();
            tick_boss_pattern(
                &cfg,
                &mut state,
                &BossPatternContext {
                    encounter_phase: boss.encounter_phase,
                    actor_pos: boss.pos,
                    target_pos: player_pos,
                    world_size: world.size,
                    dt,
                },
                &mut frame,
                &mut attack_state,
            );
            boss.integrate_body(&world, frame.desired_vel, dt);
        }
        let boss_right_edge = boss.pos.x + boss.combat_size().x * 0.5;
        let wall_left_edge = 400.0;
        assert!(
            boss_right_edge <= wall_left_edge + 0.5,
            "boss clipped into wall at pos {:?} (right edge {}); wall left edge {}",
            boss.pos,
            boss_right_edge,
            wall_left_edge,
        );
    }
}
