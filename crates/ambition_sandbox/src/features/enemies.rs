use super::*;

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
}

impl EnemyArchetype {
    /// All combat-capable archetypes in a stable order. Useful for
    /// tests / tooling that want to iterate every variant; the
    /// sandbag training dummies are *not* in this list because they
    /// don't run the standard combat AI loop.
    pub const COMBAT_ALL: [Self; 7] = [
        Self::Combatant,
        Self::SmallSkitter,
        Self::SmallLurker,
        Self::MediumStriker,
        Self::LargeBrute,
        Self::LargeColossus,
        Self::AggressiveSeeker,
    ];

    pub(super) fn from_brain(brain: &ae::EnemyBrain) -> Self {
        match brain {
            ae::EnemyBrain::Custom(name) if name == "small_skitter" => Self::SmallSkitter,
            ae::EnemyBrain::Custom(name) if name == "small_lurker" => Self::SmallLurker,
            ae::EnemyBrain::Custom(name) if name == "medium_striker" => Self::MediumStriker,
            ae::EnemyBrain::Custom(name) if name == "large_brute" => Self::LargeBrute,
            ae::EnemyBrain::Custom(name) if name == "large_colossus" => Self::LargeColossus,
            ae::EnemyBrain::Custom(name) if name == "gradient_seeker" => Self::AggressiveSeeker,
            ae::EnemyBrain::Custom(name) if name == "sandbag_infinite" => Self::InfiniteSandbag,
            ae::EnemyBrain::Custom(name) if name == "sandbag_finite" => Self::FiniteSandbag,
            _ => Self::Combatant,
        }
    }

    pub(crate) fn is_sandbag(self) -> bool {
        matches!(self, Self::InfiniteSandbag | Self::FiniteSandbag)
    }

    pub(super) fn max_health(self) -> i32 {
        match self {
            Self::SmallSkitter => 2,
            Self::SmallLurker => 2,
            Self::Combatant | Self::AggressiveSeeker => 4,
            Self::MediumStriker => 5,
            Self::LargeBrute => 9,
            Self::LargeColossus => 14,
            Self::InfiniteSandbag => 9999,
            Self::FiniteSandbag => 6,
        }
    }

    pub(super) fn patrol_speed(self) -> f32 {
        match self {
            Self::SmallSkitter => 150.0,
            Self::SmallLurker => 60.0, // sluggish — that's the point
            Self::LargeBrute => 72.0,
            Self::LargeColossus => 40.0, // barely moves; almost stationary
            Self::AggressiveSeeker => 130.0,
            _ => ENEMY_PATROL_SPEED,
        }
    }

    pub(super) fn chase_speed(self) -> f32 {
        match self {
            Self::SmallSkitter => 210.0,
            Self::SmallLurker => 90.0,
            Self::LargeBrute => 118.0,
            Self::LargeColossus => 80.0, // never sprints
            Self::AggressiveSeeker => 225.0,
            Self::MediumStriker => 170.0,
            _ => ENEMY_CHASE_SPEED,
        }
    }

    pub(super) fn aggro_radius(self) -> f32 {
        match self {
            Self::SmallSkitter => 320.0,
            Self::SmallLurker => 96.0, // tight — player can walk past
            Self::MediumStriker | Self::Combatant => 460.0,
            Self::LargeBrute => 380.0,
            Self::LargeColossus => 200.0, // narrow threat envelope
            Self::AggressiveSeeker => 900.0,
            Self::InfiniteSandbag | Self::FiniteSandbag => 0.0,
        }
    }

    pub(super) fn attack_range(self) -> f32 {
        match self {
            Self::SmallSkitter => 105.0,
            Self::SmallLurker => 90.0,
            Self::LargeBrute => 205.0,
            Self::LargeColossus => 240.0, // big arms reach further
            _ => ENEMY_ATTACK_RANGE,
        }
    }

    pub(super) fn contact_strength(self) -> f32 {
        match self {
            Self::SmallSkitter => 0.55,
            Self::SmallLurker => 0.45,
            Self::LargeBrute => 1.25,
            Self::LargeColossus => 1.50, // hits the hardest of any non-boss
            Self::AggressiveSeeker => 0.80,
            _ => 0.70,
        }
    }

    pub(super) fn damage_amount(self) -> i32 {
        match self {
            Self::LargeBrute => 2,
            Self::LargeColossus => 3,
            _ => 1,
        }
    }
}

impl EnemyRuntime {
    pub(super) fn new(
        object: &ae::RoomObject,
        brain: ae::EnemyBrain,
        paths: &[(String, ae::KinematicPath)],
    ) -> Self {
        let archetype = EnemyArchetype::from_brain(&brain);
        let motion = match &brain {
            ae::EnemyBrain::Patrol {
                path_id: Some(path_id),
            } if !archetype.is_sandbag() => paths
                .iter()
                .find(|(id, _)| id == path_id)
                .map(|(_, path)| PathMotion::new(path.clone())),
            _ => None,
        };
        let pos = motion
            .as_ref()
            .and_then(PathMotion::start_pos)
            .unwrap_or_else(|| object.aabb.center());
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos,
            spawn: pos,
            size: object.aabb.half_size() * 2.0,
            vel: ae::Vec2::ZERO,
            health: ae::Health::new(archetype.max_health()),
            brain,
            archetype,
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
        }
    }

    pub(super) fn update(
        &mut self,
        world: &ae::World,
        player: &ae::Player,
        tuning: FeatureCombatTuning,
        dt: f32,
    ) {
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

        // The engine CharacterAI output is now authoritative for the coarse
        // behavior decision. Sandbox code supplies archetype speeds and
        // collision, but no longer has a second, parallel set of
        // Guard/Custom/Patrol/attack-range branches.
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
                ae::CharacterAiIntent::Chase { direction_x } => {
                    direction_x * self.archetype.chase_speed()
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
                    gravity: ENEMY_GRAVITY,
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
            | ae::CharacterAiIntent::Attack { direction_x } => {
                if direction_x.abs() > 0.001 {
                    self.facing = direction_x.signum();
                }
            }
            _ => {}
        }

        if matches!(ai.intent, ae::CharacterAiIntent::Attack { .. })
            && !self.archetype.is_sandbag()
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
    pub fn body_damage_aabb(&self) -> Option<ae::Aabb> {
        if self.archetype.is_sandbag() {
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
                });
            }
        }
        None
    }
}
