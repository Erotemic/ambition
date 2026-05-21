use super::*;

/// Movement family for a live boss actor. Encounter phases decide *when* a boss
/// is active; this profile decides how the authored actor moves while active.
#[derive(Clone, Debug, PartialEq)]
pub enum BossMovementProfile {
    /// Existing grounded/hovering sentinel feel: stay near the authored spawn,
    /// sway horizontally, and chase the player a little without abandoning the
    /// arena anchor.
    AnchorSway {
        x_radius: f32,
        y_bob: f32,
        x_frequency: f32,
        y_frequency: f32,
        chase_scale: f32,
        chase_limit: f32,
        speed: f32,
    },
    /// Wide airborne arcs for ship/bird-like bosses. Keeps a stable home anchor
    /// but spends more of the fight sweeping across it.
    AirSwoop {
        x_radius: f32,
        y_radius: f32,
        x_frequency: f32,
        y_frequency: f32,
        chase_scale: f32,
        chase_limit: f32,
        speed: f32,
    },
    /// Stationary giant: the entity barely moves — only a slow breath-like
    /// sway. The hands and head do the attacking via hitbox volumes computed
    /// relative to spawn; the entity itself stays nearly fixed so the large
    /// background body sprite reads as immovable.
    StationaryGiant {
        sway_amplitude: f32,
        sway_frequency: f32,
        speed: f32,
    },
}

impl BossMovementProfile {
    fn target(&self, boss: &BossRuntime, target_pos: ae::Vec2) -> ae::Vec2 {
        let anchor_to_player = target_pos - boss.spawn;
        match *self {
            Self::AnchorSway {
                x_radius,
                y_bob,
                x_frequency,
                y_frequency,
                chase_scale,
                chase_limit,
                ..
            } => {
                let chase = (anchor_to_player.x * chase_scale).clamp(-chase_limit, chase_limit);
                ae::Vec2::new(
                    boss.spawn.x + (boss.movement_timer * x_frequency).sin() * x_radius + chase,
                    boss.spawn.y - (boss.movement_timer * y_frequency).sin().abs() * y_bob,
                )
            }
            Self::AirSwoop {
                x_radius,
                y_radius,
                x_frequency,
                y_frequency,
                chase_scale,
                chase_limit,
                ..
            } => {
                let chase = (anchor_to_player.x * chase_scale).clamp(-chase_limit, chase_limit);
                ae::Vec2::new(
                    boss.spawn.x + (boss.movement_timer * x_frequency).sin() * x_radius + chase,
                    boss.spawn.y + (boss.movement_timer * y_frequency).sin() * y_radius
                        - y_radius * 0.35,
                )
            }
            Self::StationaryGiant {
                sway_amplitude,
                sway_frequency,
                ..
            } => {
                // Minimal sway around spawn — the GNU-ton body stays nearly fixed.
                let _ = anchor_to_player; // giant ignores player for movement
                ae::Vec2::new(
                    boss.spawn.x + (boss.movement_timer * sway_frequency).sin() * sway_amplitude,
                    boss.spawn.y,
                )
            }
        }
    }

    fn speed(&self) -> f32 {
        match *self {
            Self::AnchorSway { speed, .. }
            | Self::AirSwoop { speed, .. }
            | Self::StationaryGiant { speed, .. } => speed,
        }
    }
}

/// One beat in a scripted boss attack timeline. Patterns built from these
/// steps give each boss a memorizable rhythm — explicit rest beats let the
/// player read the telegraph, react, and then learn the sequence over time.
/// Bosses without a scripted pattern fall back to the older
/// `attack_cooldown`-driven cycle through `BossBehaviorProfile::attacks`.
#[derive(Clone, Debug, PartialEq)]
pub enum BossPatternStep {
    /// Boss is winding up: telegraph volumes draw, no damage yet.
    Telegraph {
        profile: BossAttackProfile,
        duration: f32,
    },
    /// Hitbox is live: active volumes draw, contact damages the player.
    Strike {
        profile: BossAttackProfile,
        duration: f32,
    },
    /// No volume. Pure breathing room so the player can reposition or punish.
    Rest { duration: f32 },
}

/// A full attack script for one boss phase. Loops when it reaches the end.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BossPattern {
    pub steps: Vec<BossPatternStep>,
}

impl BossPattern {
    pub fn total_duration(&self) -> f32 {
        self.steps
            .iter()
            .map(|step| match step {
                BossPatternStep::Telegraph { duration, .. }
                | BossPatternStep::Strike { duration, .. }
                | BossPatternStep::Rest { duration } => *duration,
            })
            .sum()
    }
}

/// How a boss decides which attack hitbox is active each frame.
#[derive(Clone, Debug, PartialEq)]
pub enum BossAttackPattern {
    /// Legacy cycle: rotate through `BossBehaviorProfile::attacks` using the
    /// flat windup / active / cooldown durations on the profile. Cheap, but
    /// every attack uses the same rhythm.
    Cycle,
    /// Scripted timeline keyed off `BossEncounterPhase`. Each phase carries
    /// its own ordered list of telegraph / strike / rest beats. Missing
    /// phases fall back to `phase1`.
    Scripted {
        intro: BossPattern,
        phase1: BossPattern,
        transition: BossPattern,
        phase2: BossPattern,
        enrage: BossPattern,
    },
}

impl BossAttackPattern {
    pub fn pattern_for(&self, phase: ae::BossEncounterPhase) -> Option<&BossPattern> {
        match self {
            BossAttackPattern::Cycle => None,
            BossAttackPattern::Scripted {
                intro,
                phase1,
                transition,
                phase2,
                enrage,
            } => match phase {
                ae::BossEncounterPhase::Intro => Some(intro),
                ae::BossEncounterPhase::Phase1 => Some(phase1),
                ae::BossEncounterPhase::Transition => Some(transition),
                ae::BossEncounterPhase::Phase2 => Some(phase2),
                ae::BossEncounterPhase::Enrage => Some(enrage),
                // Dormant / Stagger / Death don't run patterns; the caller
                // already skips attacks in those phases.
                _ => Some(phase1),
            },
        }
    }
}

/// Attack hitbox vocabulary used by `BossRuntime`.
#[derive(Clone, Debug, PartialEq)]
pub enum BossAttackProfile {
    FloorSlam,
    SideSweep,
    FullBodyPulse,
    WingSweep,
    DiveLane,
    Broadside,
    // GNU-ton specific: giant hands slam from above
    GnuHandSlam,
    // GNU-ton specific: hands sweep in from the far sides
    GnuHandSweep,
    // GNU-ton specific: the head descends into player space (vulnerability + hazard)
    GnuHeadDescent,
    // GNU-ton specific: shockwave from both hands meeting in the center
    GnuShockwave,
    // GNU-ton specific: apples fall from the ceiling around the player.
    // Direct contact damage comes from spawned enemy projectiles
    // (gravity > 0), not a single AABB, so `volumes_for` returns empty
    // for this profile and damage routes through the projectile path.
    GnuAppleRain,
}

/// Outputs produced by `BossRuntime::update` that need to be flushed
/// into world resources by the calling system. Today this is just
/// enemy-projectile spawn requests (GNU-ton's apple rain); future
/// attacks that emit hazards or SFX can plug in here without further
/// signature churn.
#[derive(Default)]
pub struct BossTickOutputs {
    pub projectile_spawns: Vec<crate::enemy_projectile::EnemyProjectileSpawn>,
}

/// Apple-rain tuning. One spawn every `APPLE_RAIN_INTERVAL` seconds
/// inside an active GnuAppleRain strike; gravity is gentle enough that
/// even a casual sidestep clears the falling arc, but the cadence is
/// fast enough that standing still under the giant is unsafe.
/// Apples spawn anywhere along the arena width (not just over the
/// player) so the entire floor becomes a no-stand zone for the
/// duration of the strike — the player must keep moving.
const APPLE_RAIN_INTERVAL: f32 = 0.35;
const APPLE_RAIN_GRAVITY: f32 = 540.0;
const APPLE_RAIN_SPAWN_SPEED: f32 = 35.0;
const APPLE_RAIN_LIFETIME: f32 = 6.0;
/// Apples are large enough to clearly read as the dominant arena
/// threat (14×16 half-extent = 28×32 collision box, sprite renders
/// ~5% larger). Earlier 9×10 was too easy to overlook visually.
const APPLE_RAIN_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(14.0, 16.0);
const APPLE_RAIN_DAMAGE: i32 = 1;
const APPLE_RAIN_SPAWN_HEIGHT_ABOVE_PLAYER: f32 = 320.0;
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

fn step_duration(step: &BossPatternStep) -> f32 {
    match step {
        BossPatternStep::Telegraph { duration, .. }
        | BossPatternStep::Strike { duration, .. }
        | BossPatternStep::Rest { duration } => *duration,
    }
}

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
    /// `BossEncounterRegistry` so scripted patterns can pick the right
    /// phase timeline. `Dormant` until the encounter wakes up.
    pub encounter_phase: ae::BossEncounterPhase,
    /// Cursor into the active scripted pattern. Cycle-mode bosses leave
    /// this at 0. Resets to 0 on phase change.
    pub scripted_step_index: usize,
    /// Seconds spent in the current scripted step. Reset on step advance.
    pub scripted_step_elapsed: f32,
    /// Active strike's attack profile (set while the runtime is inside a
    /// `Strike` step). `None` outside Strike.
    pub active_strike_profile: Option<BossAttackProfile>,
    /// Telegraphed attack profile (set while inside a `Telegraph` step).
    pub telegraph_profile: Option<BossAttackProfile>,
    /// Accumulates dt while a `GnuAppleRain` strike is active; emits
    /// one apple every `APPLE_RAIN_INTERVAL` seconds. Reset to 0 when
    /// the strike ends so the next rain window starts from a clean
    /// cadence rather than an off-beat carry-over.
    pub apple_spawn_accum: f32,
    /// Monotonic counter for apple spawns this encounter — used as
    /// a deterministic seed for x-jitter so two consecutive apples
    /// land at different offsets without dragging in an RNG.
    pub apple_spawn_index: u32,
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
            scripted_step_index: 0,
            scripted_step_elapsed: 0.0,
            active_strike_profile: None,
            telegraph_profile: None,
            apple_spawn_accum: 0.0,
            apple_spawn_index: 0,
        }
    }

    /// `target_pos` is populated from the boss entity's `ActorTarget`
    /// component by `select_actor_targets` (OVERNIGHT-TODO #17.8).
    /// The boss movement profile reads it for anchor-sway / air-swoop
    /// chase math; scripted patterns (`StationaryGiant`) ignore it.
    ///
    /// `outputs` accumulates spawn requests that the calling system
    /// is expected to flush into the relevant world resources
    /// (e.g. `EnemyProjectileState` for apple-rain apples).
    pub(super) fn update(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: FeatureCombatTuning,
        outputs: &mut BossTickOutputs,
        dt: f32,
    ) {
        if !self.alive {
            return;
        }
        self.pattern_timer += dt;
        self.movement_timer += dt;

        // BRAIN STAGE — scripted boss pattern + movement profile pack
        // their per-tick decision into an `ActorControlFrame`. Same
        // contract enemies use; same one a future player-controlled
        // boss or RL-driven boss would fill in. The brain owns the
        // sway/dodge math and the clamp to world bounds; the
        // simulation half decides what's actually possible against
        // collision.
        let frame = self.build_control_frame(world, target_pos, dt);

        // INTEGRATION STAGE — gravity=0 (bosses float; ground state
        // is irrelevant for current bosses), max_fall_speed=0. The
        // single `step_kinematic` call replaces the bespoke per-axis
        // `boss_space_is_free` sweep `move_toward_target` used to do
        // by hand. Multi-part bosses like GNU-ton expose a
        // `combat_size` distinct from the sprite `size`; that's the
        // size we collide against, matching the previous behavior.
        let mut body = ae::KinematicBody {
            pos: self.pos,
            vel: frame.desired_vel,
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
                drop_through: frame.drop_through,
            },
            dt,
        );
        self.pos = body.pos;
        self.hit_flash = (self.hit_flash - dt).max(0.0);

        // EFFECTS STAGE — attack pattern state machine and apple-rain
        // tick. These are time-driven self-state mutations (not
        // per-tick brain intents), so they live outside the frame and
        // run after integration.
        match &self.behavior.attack_pattern {
            BossAttackPattern::Cycle => self.update_cycle_attacks(tuning, dt),
            BossAttackPattern::Scripted { .. } => self.update_scripted_attacks(dt),
        }

        if matches!(
            self.active_strike_profile,
            Some(BossAttackProfile::GnuAppleRain)
        ) {
            self.tick_apple_rain(world, target_pos, outputs, dt);
        } else {
            // Drop the accumulator outside the strike so the next
            // rain window starts on a clean beat instead of dumping
            // a burst at t=0 from leftover dt.
            self.apple_spawn_accum = 0.0;
        }
    }

    /// Pack the boss's per-tick movement decision into a flat
    /// `ActorControlFrame`. This is the brain-to-sim seam for bosses
    /// — a future RL policy or remote-player driver fills the SAME
    /// frame and the integration code in `update` is unchanged.
    ///
    /// Currently emits only `desired_vel`. Bosses don't melee-press
    /// or fire individual projectiles through this seam yet —
    /// scripted attack patterns + apple-rain tick handle those via
    /// internal state, not per-tick intents. When bosses gain
    /// brain-driven attack timing those signals will land on the
    /// same frame.
    fn build_control_frame(
        &self,
        world: &ae::World,
        target_pos: ae::Vec2,
        dt: f32,
    ) -> ae::ActorControlFrame {
        let mut frame = ae::ActorControlFrame::neutral();
        if dt <= 0.0 {
            return frame;
        }

        // Where the movement profile would like to be this tick,
        // before the apple-rain dodge layer.
        let mut target = self.behavior.movement.target(self, target_pos);
        // While the giant is raining apples on the arena, add a
        // visible horizontal dodge so it reads as "stepping aside to
        // avoid its own experiment" instead of standing perfectly
        // still under a deathstorm of fruit. Layered over (not
        // replacing) the baseline StationaryGiant sway so the
        // silhouette already moving for `sway_amplitude` keeps its
        // phase while the amplitude swells.
        if matches!(
            self.active_strike_profile,
            Some(BossAttackProfile::GnuAppleRain)
        ) {
            let dodge_amp = 70.0;
            let dodge_freq = 1.6;
            target.x += (self.movement_timer * dodge_freq).sin() * dodge_amp;
        }

        // Soft world-bounds clamp: keep the brain from asking to walk
        // off the map. Real collision is still enforced by
        // `step_kinematic` against world blocks; this clamp matches
        // the previous `move_toward_target` margin semantics.
        let half = self.combat_size() * 0.5;
        let margin = 8.0;
        let max_x = (world.size.x - half.x - margin).max(half.x + margin);
        let max_y = (world.size.y - half.y - margin).max(half.y + margin);
        let clamped_target = ae::Vec2::new(
            target.x.clamp(half.x + margin, max_x),
            target.y.clamp(half.y + margin, max_y),
        );

        let delta = clamped_target - self.pos;
        let speed = self.behavior.movement.speed();
        let max_step = speed * dt;
        frame.desired_vel = if delta.length() > max_step && max_step > 0.0 {
            delta.normalize_or_zero() * speed
        } else {
            delta / dt
        };
        frame
    }

    /// Emit one falling-apple spawn per `APPLE_RAIN_INTERVAL` while
    /// the GnuAppleRain strike is active. Spawn x is distributed
    /// pseudo-uniformly across the FULL arena width via a low-discrepancy
    /// sequence (golden-ratio rotation of `apple_spawn_index`), so the
    /// whole arena becomes a no-stand zone — not just the column over
    /// the player. The boss itself avoids dropping apples directly on
    /// its own body so it doesn't self-flagellate during the strike;
    /// see `apple_spawn_avoids_self_aabb`.
    fn tick_apple_rain(
        &mut self,
        world: &ae::World,
        _target_pos: ae::Vec2,
        outputs: &mut BossTickOutputs,
        dt: f32,
    ) {
        self.apple_spawn_accum += dt;
        let margin = APPLE_RAIN_HALF_EXTENT.x + 8.0;
        let max_x = (world.size.x - margin).max(margin);
        let spawnable_width = (max_x - margin).max(0.0);
        let self_aabb = self.aabb();
        while self.apple_spawn_accum >= APPLE_RAIN_INTERVAL {
            self.apple_spawn_accum -= APPLE_RAIN_INTERVAL;
            // Golden-ratio low-discrepancy x — covers the whole arena
            // evenly across consecutive spawns without RNG, and reads as
            // "random" to the player. The Hammersley/Halton family
            // would also work; phi gives the right "no two adjacent
            // apples land on top of each other" property cheaply.
            const PHI_FRAC: f32 = 0.618_033_99;
            let i = self.apple_spawn_index;
            let frac = ((i as f32) * PHI_FRAC).fract();
            let mut spawn_x = margin + frac * spawnable_width;
            // Slide the x out from under the boss body so an apple
            // doesn't immediately hit GNU-ton on the head. Pick the
            // nearer of the boss's left/right edges so the cleaver
            // motion is minimal.
            let self_left = self_aabb.min.x - APPLE_RAIN_HALF_EXTENT.x;
            let self_right = self_aabb.max.x + APPLE_RAIN_HALF_EXTENT.x;
            if spawn_x > self_left && spawn_x < self_right {
                spawn_x = if spawn_x - self_left < self_right - spawn_x {
                    self_left
                } else {
                    self_right
                };
                spawn_x = spawn_x.clamp(margin, max_x);
            }
            let spawn_y = (self.pos.y - APPLE_RAIN_SPAWN_HEIGHT_ABOVE_PLAYER)
                .max(APPLE_RAIN_HALF_EXTENT.y + 8.0);
            outputs
                .projectile_spawns
                .push(crate::enemy_projectile::EnemyProjectileSpawn {
                    origin: ae::Vec2::new(spawn_x, spawn_y),
                    // Downward initial velocity so the apple commits to
                    // its lane immediately instead of hanging at zero
                    // until gravity catches up.
                    dir: ae::Vec2::new(0.0, 1.0),
                    speed: APPLE_RAIN_SPAWN_SPEED,
                    damage: APPLE_RAIN_DAMAGE,
                    max_lifetime: APPLE_RAIN_LIFETIME,
                    half_extent: APPLE_RAIN_HALF_EXTENT,
                    owner_id: format!("{}:{}", GNU_TON_APPLE_OWNER_PREFIX, self.id),
                    gravity: APPLE_RAIN_GRAVITY,
                });
            self.apple_spawn_index = self.apple_spawn_index.wrapping_add(1);
        }
    }

    fn update_cycle_attacks(&mut self, tuning: FeatureCombatTuning, dt: f32) {
        let was_winding_up = self.attack_windup_timer > 0.0;
        self.attack_windup_timer = (self.attack_windup_timer - dt).max(0.0);
        self.attack_timer = (self.attack_timer - dt).max(0.0);
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        if was_winding_up && self.attack_windup_timer <= 0.0 {
            self.attack_timer = self
                .behavior
                .attack_active
                .max(tuning.boss_attack_active)
                .max(0.01);
        }
        if self.attack_cooldown <= 0.0
            && self.attack_windup_timer <= 0.0
            && self.attack_timer <= 0.0
        {
            self.attack_windup_timer = self.behavior.attack_windup.max(0.01);
            self.attack_cooldown = self.behavior.attack_cooldown.max(0.05);
        }
    }

    fn update_scripted_attacks(&mut self, dt: f32) {
        // Scripted timelines must only fire during attacking phases —
        // Dormant / Stagger / Death / pre-intro outros are explicit
        // breathing room. Without this gate, `pattern_for(Dormant)`
        // falls back to `phase1` and the boss keeps telegraphing/
        // striking through what should be a stagger window.
        if !self.encounter_phase.is_attacking() {
            self.active_strike_profile = None;
            self.telegraph_profile = None;
            self.attack_timer = 0.0;
            self.attack_windup_timer = 0.0;
            return;
        }
        // Clone the active pattern's steps so we can mutate the cursor
        // without aliasing the immutable behavior borrow. Scripts are
        // small (~10 steps) so the per-frame clone cost is negligible.
        let phase = self.encounter_phase;
        let steps: Vec<BossPatternStep> = match self.behavior.attack_pattern.pattern_for(phase) {
            Some(pattern) if !pattern.steps.is_empty() => pattern.steps.clone(),
            _ => {
                self.active_strike_profile = None;
                self.telegraph_profile = None;
                self.attack_timer = 0.0;
                self.attack_windup_timer = 0.0;
                return;
            }
        };

        self.scripted_step_elapsed += dt;
        // Wrap the cursor if a phase transition shrunk the script under
        // our feet, then advance through any completed steps this frame.
        if self.scripted_step_index >= steps.len() {
            self.scripted_step_index = 0;
            self.scripted_step_elapsed = 0.0;
        }
        loop {
            let current = &steps[self.scripted_step_index];
            let duration = step_duration(current).max(0.01);
            if self.scripted_step_elapsed < duration {
                break;
            }
            self.scripted_step_elapsed -= duration;
            self.scripted_step_index = (self.scripted_step_index + 1) % steps.len();
        }

        // Drive the legacy `attack_windup_timer` / `attack_timer` mirror
        // and the live profile slots from the active step. This keeps
        // existing consumers (`attack_volumes()`, `attack_telegraph_volumes()`,
        // `player_damage()`) working without per-call match arms.
        let current = &steps[self.scripted_step_index];
        let remaining = (step_duration(current) - self.scripted_step_elapsed).max(0.0);
        match current {
            BossPatternStep::Telegraph { profile, .. } => {
                self.telegraph_profile = Some(profile.clone());
                self.active_strike_profile = None;
                self.attack_windup_timer = remaining;
                self.attack_timer = 0.0;
            }
            BossPatternStep::Strike { profile, .. } => {
                self.telegraph_profile = None;
                self.active_strike_profile = Some(profile.clone());
                self.attack_windup_timer = 0.0;
                self.attack_timer = remaining;
            }
            BossPatternStep::Rest { .. } => {
                self.telegraph_profile = None;
                self.active_strike_profile = None;
                self.attack_windup_timer = 0.0;
                self.attack_timer = 0.0;
            }
        }
        self.attack_cooldown = 0.0;
    }

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

    #[test]
    fn gnu_ton_scripted_advance_cycles_telegraph_strike_rest() {
        let mut boss = gnu_ton_runtime();
        let world = ae::World::new(
            "test_arena",
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        );
        let player = ae::Player::new_with_abilities(ae::Vec2::ZERO, ae::AbilitySet::default());
        let mut observed: Vec<&'static str> = Vec::new();
        let dt = 0.05;
        let mut ticks = 0;
        let mut last: &'static str = "";
        while observed.len() < 6 && ticks < 4_000 {
            boss.update(
                &world,
                player.pos,
                FeatureCombatTuning::default(),
                &mut BossTickOutputs::default(),
                dt,
            );
            let now = if boss.telegraph_profile.is_some() {
                "telegraph"
            } else if boss.active_strike_profile.is_some() {
                "strike"
            } else {
                "rest"
            };
            if now != last {
                observed.push(now);
                last = now;
            }
            ticks += 1;
        }
        // Phase 1 always begins on a Telegraph; we should see at least
        // one telegraph -> strike transition AND one rest beat before
        // looping. This catches regressions where the scripted runtime
        // gets stuck inside one step type.
        assert!(observed.contains(&"telegraph"), "{observed:?}");
        assert!(observed.contains(&"strike"), "{observed:?}");
        assert!(observed.contains(&"rest"), "{observed:?}");
    }

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

    #[test]
    fn gnu_ton_scripted_patterns_skip_non_attacking_phases() {
        // `BossAttackPattern::pattern_for(Dormant)` falls back to
        // `phase1` (so phase enum drift doesn't crash), but the runtime
        // must not actually *run* that pattern during Dormant / Stagger /
        // Death — those are explicit breathing/punish windows. Without
        // the `is_attacking()` gate the boss would keep telegraphing
        // through a stagger window. Repro: tick the runtime in Dormant
        // for the duration of a full pattern and assert no strike/
        // telegraph profile is ever assigned.
        let mut boss = gnu_ton_runtime();
        boss.encounter_phase = ae::BossEncounterPhase::Dormant;
        let world = ae::World::new(
            "test_arena",
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        );
        let player = ae::Player::new_with_abilities(ae::Vec2::ZERO, ae::AbilitySet::default());
        for _ in 0..200 {
            boss.update(
                &world,
                player.pos,
                FeatureCombatTuning::default(),
                &mut BossTickOutputs::default(),
                0.05,
            );
            assert!(boss.active_strike_profile.is_none());
            assert!(boss.telegraph_profile.is_none());
            assert_eq!(boss.attack_timer, 0.0);
            assert_eq!(boss.attack_windup_timer, 0.0);
        }
    }

    #[test]
    fn gnu_ton_apple_rain_strike_emits_falling_apple_spawns() {
        // Concrete contract: while the GnuAppleRain strike is active,
        // the boss must accumulate enemy-projectile spawn requests in
        // `outputs.projectile_spawns` at the documented cadence. Each
        // spawn must point downward, carry positive gravity, and be
        // tagged with the apple owner prefix so the visuals layer
        // picks the apple sprite instead of the bullet rectangle.
        let mut boss = gnu_ton_runtime();
        boss.active_strike_profile = Some(BossAttackProfile::GnuAppleRain);
        let world = ae::World::new(
            "test_arena",
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        );
        let player_pos = ae::Vec2::new(500.0, 400.0);
        let mut outputs = BossTickOutputs::default();
        // Hand-tick the apple-rain branch with a step a hair past
        // 3 intervals (epsilon avoids float-precision shaving the
        // last spawn off when interval * 3 doesn't quite land at the
        // boundary due to f32 rounding).
        boss.tick_apple_rain(
            &world,
            player_pos,
            &mut outputs,
            APPLE_RAIN_INTERVAL * 3.0 + 1.0e-3,
        );
        assert!(
            outputs.projectile_spawns.len() >= 3,
            "expected >=3 apple spawns over 3 intervals, got {}",
            outputs.projectile_spawns.len()
        );
        for spawn in &outputs.projectile_spawns {
            assert!(spawn.gravity > 0.0, "apples must fall under gravity");
            assert!(spawn.dir.y > 0.0, "apples must start moving down");
            assert!(
                spawn.owner_id.starts_with(GNU_TON_APPLE_OWNER_PREFIX),
                "apples must use the apple owner prefix so the visuals \
                 layer can swap in the apple sprite; got {}",
                spawn.owner_id
            );
            // Spawn height is anchored to the boss now (not the
            // player target), but in this fixture the two share y so
            // the original-spirit assertion still holds: apples fall
            // from above the player onto the player.
            assert!(
                spawn.origin.y < player_pos.y,
                "apples must spawn above the player"
            );
        }
    }

    #[test]
    fn gnu_ton_apple_rain_spawns_avoid_self_aabb() {
        // The boss must not drop apples on its own head — the strike
        // is choreography, not friendly fire. After ticking for many
        // intervals, no spawn x should fall inside the boss AABB
        // (expanded by the apple half-extent for the collision check).
        let mut boss = gnu_ton_runtime();
        boss.active_strike_profile = Some(BossAttackProfile::GnuAppleRain);
        let world = ae::World::new(
            "test_arena",
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        );
        let player_pos = ae::Vec2::new(500.0, 400.0);
        let mut outputs = BossTickOutputs::default();
        // Long enough to walk through a full lap of the
        // golden-ratio sequence so the avoidance path actually
        // exercises the boss-overlap case.
        boss.tick_apple_rain(
            &world,
            player_pos,
            &mut outputs,
            APPLE_RAIN_INTERVAL * 30.0 + 1.0e-3,
        );
        let self_aabb = boss.aabb();
        let pad = APPLE_RAIN_HALF_EXTENT.x;
        for spawn in &outputs.projectile_spawns {
            assert!(
                spawn.origin.x <= self_aabb.min.x - pad + 1e-3
                    || spawn.origin.x >= self_aabb.max.x + pad - 1e-3,
                "apple at x={} fell inside the boss aabb [{},{}] +/- {}",
                spawn.origin.x,
                self_aabb.min.x,
                self_aabb.max.x,
                pad
            );
        }
    }

    #[test]
    fn gnu_ton_apple_rain_spawns_cover_full_arena_width() {
        // The whole arena should become a no-stand zone, not just the
        // column directly over the player. Across many spawns we
        // should see apples both to the LEFT and to the RIGHT of the
        // boss center — otherwise the player just stands on the
        // opposite side of the boss and waits the strike out.
        let mut boss = gnu_ton_runtime();
        boss.active_strike_profile = Some(BossAttackProfile::GnuAppleRain);
        let world = ae::World::new(
            "test_arena",
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        );
        let player_pos = ae::Vec2::new(500.0, 400.0);
        let mut outputs = BossTickOutputs::default();
        boss.tick_apple_rain(
            &world,
            player_pos,
            &mut outputs,
            APPLE_RAIN_INTERVAL * 20.0 + 1.0e-3,
        );
        let any_left = outputs
            .projectile_spawns
            .iter()
            .any(|s| s.origin.x < boss.pos.x - 100.0);
        let any_right = outputs
            .projectile_spawns
            .iter()
            .any(|s| s.origin.x > boss.pos.x + 100.0);
        assert!(any_left && any_right, "{:?}", outputs.projectile_spawns);
    }

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
    fn gnu_ton_apple_rain_resets_accumulator_when_strike_ends() {
        // Leaving the strike must zero `apple_spawn_accum` so the next
        // rain window starts on a clean beat instead of dumping a burst
        // at t=0 from leftover dt.
        let mut boss = gnu_ton_runtime();
        boss.active_strike_profile = Some(BossAttackProfile::GnuAppleRain);
        let world = ae::World::new(
            "test_arena",
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        );
        let player = ae::Player::new_with_abilities(ae::Vec2::ZERO, ae::AbilitySet::default());
        let mut outputs = BossTickOutputs::default();
        // Tick mid-interval (so accum sits non-zero), then transition
        // out of the strike and tick again — the non-rain branch should
        // reset the accumulator without emitting spawns.
        boss.tick_apple_rain(&world, player.pos, &mut outputs, APPLE_RAIN_INTERVAL * 0.5);
        assert!(boss.apple_spawn_accum > 0.0);
        boss.active_strike_profile = None;
        boss.update(
            &world,
            player.pos,
            FeatureCombatTuning::default(),
            &mut outputs,
            0.05,
        );
        assert_eq!(
            boss.apple_spawn_accum, 0.0,
            "accumulator must reset when the rain strike ends"
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
        let mut outputs = BossTickOutputs::default();
        for _ in 0..600 {
            boss.update(
                &world,
                player_pos,
                FeatureCombatTuning::default(),
                &mut outputs,
                1.0 / 60.0,
            );
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
