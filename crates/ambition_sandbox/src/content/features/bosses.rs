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
    fn target(&self, boss: &BossRuntime, player: &ae::Player) -> ae::Vec2 {
        let anchor_to_player = player.pos - boss.spawn;
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
    /// - Intro      : single show-of-force slam (no rest after) to set tone
    /// - Phase 1    : ~9s — slam → rest → sweep → rest → slam → long rest
    /// - Transition : ~3s pure rest (player gets a breath)
    /// - Phase 2    : ~12s — adds head-descent windows where the head is
    ///                exposed and vulnerable, framed by long rests so the
    ///                player can punish during the descent and then reset
    /// - Enrage     : ~8s — shockwave + double slam, shorter rests
    pub fn gnu_ton() -> Self {
        Self {
            id: "gnu_ton".into(),
            // Large combat size covers the full body + hand extension range.
            // The player can be damaged by the hands (far sides) or the descending head.
            combat_size: Some(ae::Vec2::new(580.0, 320.0)),
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
                        BossPatternStep::Rest { duration: 1.2 },
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
            // GNU-ton's entity transform sits at the *scholar* (top of the
            // sprite) per `feet_anchor_y: 0.32` in `boss_encounter/sprites.rs`.
            // The visible giant body extends downward from there, so hand /
            // shockwave hitboxes need to anchor below the scholar to land
            // on the giant's body rather than around the scholar's head.
            // The combat box is 580x320 — drop the origin ~95px so hands
            // appear at the giant's mid-torso height.
            attack_origin_offset: ae::Vec2::new(0.0, 95.0),
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
}

impl BossRuntime {
    pub(super) fn new(object: &ae::RoomObject, brain: ae::BossBrain) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            spawn: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            health: ae::Health::new(18),
            behavior: BossBehaviorProfile::for_authored_boss(&object.name),
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
        }
    }

    pub(super) fn update(
        &mut self,
        world: &ae::World,
        player: &ae::Player,
        tuning: FeatureCombatTuning,
        dt: f32,
    ) {
        if !self.alive {
            return;
        }
        self.pattern_timer += dt;
        self.movement_timer += dt;
        let target = self.behavior.movement.target(self, player);
        self.move_toward_target(world, target, dt);
        self.hit_flash = (self.hit_flash - dt).max(0.0);

        match &self.behavior.attack_pattern {
            BossAttackPattern::Cycle => self.update_cycle_attacks(tuning, dt),
            BossAttackPattern::Scripted { .. } => self.update_scripted_attacks(dt),
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

    pub(super) fn move_toward_target(&mut self, world: &ae::World, target: ae::Vec2, dt: f32) {
        let move_size = self.combat_size();
        let half = move_size * 0.5;
        let margin = 8.0;
        let max_x = (world.size.x - half.x - margin).max(half.x + margin);
        let max_y = (world.size.y - half.y - margin).max(half.y + margin);
        let clamped_target = ae::Vec2::new(
            target.x.clamp(half.x + margin, max_x),
            target.y.clamp(half.y + margin, max_y),
        );
        let delta = clamped_target - self.pos;
        let max_step = self.behavior.movement.speed() * dt.max(0.0);
        let step = if delta.length() > max_step && max_step > 0.0 {
            delta.normalize_or_zero() * max_step
        } else {
            delta
        };

        let try_x = ae::Vec2::new(self.pos.x + step.x, self.pos.y);
        if boss_space_is_free(world, try_x, move_size) {
            self.pos.x = try_x.x;
        }
        let try_y = ae::Vec2::new(self.pos.x, self.pos.y + step.y);
        if boss_space_is_free(world, try_y, move_size) {
            self.pos.y = try_y.y;
        }
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
    /// world-space hitbox volumes. Centered on `self.pos + attack_origin_offset`
    /// so body-centered giants (GNU-ton) can anchor "hand"-class hitboxes
    /// to the giant's torso rather than to the scholar perched on top.
    pub(super) fn volumes_for(&self, attack: &BossAttackProfile) -> Vec<ae::Aabb> {
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
            // extending from near the top down to the floor.
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
        }
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
                });
            }
        }
        let body_damage = self.body_damage_aabb();
        if body_damage.strict_intersects(player_body) {
            return Some(PlayerDamageEvent {
                mode: PlayerDamageMode::Knockback,
                source: PlayerDamageSource::BossBody,
                source_pos: self.pos,
                impact_pos: midpoint(player_body.center(), body_damage.center()),
                knockback_dir: (player_body.center().x - self.pos.x).signum_or(1.0),
                strength: 1.0,
                amount: self.behavior.body_damage.max(1),
            });
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
        let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(580.0, 320.0));
        let pos = ae::Vec2::new(500.0, 400.0);
        let aabb = ae::Aabb::new(pos, combat_size * 0.5);
        let object = ae::RoomObject::new(
            "boss_gnu_ton",
            "GNU-ton",
            aabb,
            ae::RoomObjectKind::BossSpawn(ae::BossBrain::Dormant),
        );
        let mut runtime = BossRuntime::new(&object, ae::BossBrain::Dormant);
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
            boss.update(&world, &player, FeatureCombatTuning::default(), dt);
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
    fn gnu_ton_hand_slam_anchors_below_scholar() {
        // GNU-ton's transform sits at the scholar atop the giant. With
        // `attack_origin_offset` applied, hand-slam hitboxes should be
        // *below* the scholar (positive Y in engine space = downward)
        // — i.e. on the giant's body, not floating at the scholar's
        // shoulders. Reads as a visual sanity check on the alignment
        // fix the user reported.
        let boss = gnu_ton_runtime();
        let scholar_y = boss.pos.y;
        let slam = boss.volumes_for(&BossAttackProfile::GnuHandSlam);
        for vol in &slam {
            assert!(
                vol.center().y > scholar_y,
                "GnuHandSlam volume center.y ({}) should be below the scholar \
                 (pos.y={}) once attack_origin_offset is applied",
                vol.center().y,
                scholar_y
            );
        }
    }
}
