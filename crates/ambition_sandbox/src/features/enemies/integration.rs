//! Enemy physics/AI integration: the per-frame tick that drives enemy
//! movement + attack geometry through the [`EnemyMut`] ECS view. Grounded
//! enemies run the shared grounded movement spine (`integrate_normal_spine`
//! + `step_kinematic` sweep); aerial enemies and the shark/rider composite
//! go through [`super::super::step_floating_body`]. Attack AABBs are derived
//! here; archetype tuning comes from the [`super::EnemyRoster`].

use super::super::ecs::enemy_clusters::EnemyMut;
use super::super::*;
use super::*;

/// Enemy physics/AI integration, operating directly on the authoritative
/// ECS components through the [`EnemyMut`] view.
pub(crate) fn enemy_attack_aabb_dir(
    pos: ae::Vec2,
    size: ae::Vec2,
    facing: f32,
    axis: ae::Vec2,
) -> ae::Aabb {
    let horizontal = axis.x.abs() >= axis.y.abs();
    if horizontal {
        let side = if axis.x.abs() > 0.1 {
            axis.x.signum()
        } else {
            facing
        };
        let center = pos + ae::Vec2::new(side * (size.x * 0.55 + 24.0), -4.0);
        return ae::Aabb::new(center, ae::Vec2::new(34.0, 28.0));
    }
    if axis.y < 0.0 {
        let half = ae::Vec2::new(16.0, 36.0);
        let center = pos + ae::Vec2::new(0.0, -(size.y * 0.5 + half.y + 4.0));
        return ae::Aabb::new(center, half);
    }
    let half = ae::Vec2::new(36.0, 20.0);
    let center = pos + ae::Vec2::new(0.0, size.y * 0.5 + half.y - 2.0);
    ae::Aabb::new(center, half)
}

fn evaluate_enemy_ai_output(
    pos: ae::Vec2,
    target_pos: ae::Vec2,
    brain: &crate::actor::EnemyBrain,
    tuning: &crate::mechanics::combat::EnemyTuning,
    attack: &crate::features::ActorAttackState,
    alive: bool,
) -> crate::actor::ai::CharacterAiOutput {
    let recover_remaining =
        if attack.cooldown > 0.0 && attack.windup_timer <= 0.0 && attack.active_timer <= 0.0 {
            attack.cooldown.min(0.30)
        } else {
            0.0
        };
    let effective_aggro_radius = match brain {
        crate::actor::EnemyBrain::Passive => 0.0,
        crate::actor::EnemyBrain::Guard { leash_radius } => *leash_radius,
        _ => tuning.aggro_radius,
    };
    crate::actor::ai::evaluate_character_ai_output(crate::actor::ai::CharacterAiSnapshot {
        actor_pos: pos,
        player_pos: target_pos,
        aggro_radius: effective_aggro_radius,
        attack_range: tuning.attack_range,
        attack_windup_remaining: attack.windup_timer,
        attack_active_remaining: attack.active_timer,
        attack_recover_remaining: recover_remaining,
        stun_remaining: 0.0,
        alive,
        patrol_enabled: !tuning.is_sandbag && !matches!(brain, crate::actor::EnemyBrain::Passive),
    })
}

#[allow(clippy::too_many_arguments)]
fn integrate_standard_enemy_body(
    world: &ae::World,
    kin: &mut super::super::ecs::enemy_clusters::BodyKinematics,
    surface: &mut ActorSurfaceState,
    motion: &mut super::super::ecs::enemy_clusters::ActorMotionPath,
    tuning: &crate::mechanics::combat::EnemyTuning,
    ai_intent: crate::actor::ai::CharacterAiIntent,
    is_aerial: bool,
    frame: &crate::actor::control::ActorControlFrame,
    dt: f32,
    gravity_dir: ae::Vec2,
) {
    let mut body = crate::kinematic::KinematicBody {
        pos: kin.pos,
        vel: kin.vel,
        size: kin.size,
        on_ground: surface.on_ground,
        facing: kin.facing,
    };
    // Wall-stop detection runs on the gravity-PERPENDICULAR "side" axis the enemy
    // actually walks along (the spine projects run onto it). Under vertical gravity
    // `perp = (-1, 0)` so this is `±vel.x` — byte-identical to the old `vel.x` read;
    // under sideways gravity it correctly watches `vel.y`, so a patroller still
    // reverses when it stalls against a wall.
    let perp = ae::Vec2::new(-gravity_dir.y, gravity_dir.x);
    let prev_side_speed = body.vel.dot(perp);
    if is_aerial {
        let target_speed = frame.desired_vel.length();
        let archetype_chase = tuning.chase_speed;
        let accel = (target_speed.max(archetype_chase) * 3.0).max(900.0) * dt;
        // Aerial enemies are floating free-movers (shared with NPC flyers + bosses).
        super::super::step_floating_body(
            &mut body,
            world,
            frame.desired_vel,
            Some(accel),
            ENEMY_MAX_FALL,
            dt,
        );
    } else {
        // Grounded walkers run the SHARED player physics spine: gravity + run +
        // fall-cap, gravity-direction-relative. The spine projects `axis_x` onto
        // the gravity-perpendicular "side" axis (so a wall-standing enemy walks
        // ALONG the wall) and applies gravity along `gravity_dir`. We map the AI's
        // velocity-valued `desired_vel.x` onto the spine's `axis_x * max_run_speed`
        // model by setting `max_run_speed = |desired|` and `axis_x = sign(desired)`,
        // with run/air accel = ENEMY_RUN_ACCEL and friction = 0, so this is
        // byte-identical under vertical gravity to the old hand-rolled run.
        let desired = frame.desired_vel.x;
        let axis_x = if desired.abs() > 1e-3 {
            desired.signum()
        } else {
            0.0
        };
        let spine_tuning = ae::MovementTuning {
            gravity: ENEMY_GRAVITY * surface.gravity_scale,
            gravity_dir,
            run_accel: ENEMY_RUN_ACCEL,
            air_accel: ENEMY_RUN_ACCEL,
            ground_friction: 0.0,
            air_friction: 0.0,
            max_run_speed: desired.abs(),
            max_fall_speed: ENEMY_MAX_FALL,
            ..ae::MovementTuning::default()
        };
        // A grounded enemy carries no player ability components: the spine's
        // fast-fall / glide / water / blink gates are all off (pay-for-use).
        let mut fast_falling = false;
        let mut gliding = false;
        ae::integrate_normal_spine(
            &mut body.vel,
            &mut fast_falling,
            &mut gliding,
            ae::NormalSpineCtx::bare(body.on_ground),
            ae::InputState {
                axis_x,
                ..Default::default()
            },
            dt,
            spine_tuning,
        );
        if frame.jump_pressed {
            // Jump opposes gravity (2D): keep the perpendicular component, set the
            // gravity-axis component to -jump_speed. Vertical-identical.
            let g = gravity_dir;
            let jump_off = |vel: ae::Vec2, speed: f32| vel - vel.dot(g) * g - speed * g;
            if body.on_ground {
                body.vel = jump_off(body.vel, ENEMY_JUMP_SPEED);
                body.on_ground = false;
            } else if surface.air_jumps_remaining > 0 {
                body.vel = jump_off(body.vel, ENEMY_DOUBLE_JUMP_SPEED);
                surface.air_jumps_remaining -= 1;
            }
        }
        // Grounded sweep: the spine already applied gravity along `gravity_dir`,
        // so this is pure collision resolution (the same intent/sweep split the
        // player uses). The aerial branch did its own sweep via step_floating_body.
        crate::kinematic::step_kinematic(
            &mut body,
            world,
            crate::kinematic::KinematicTuning {
                gravity: 0.0,
                max_fall_speed: ENEMY_MAX_FALL,
                gravity_dir,
            },
            crate::kinematic::KinematicInputs {
                drop_through: frame.drop_through,
            },
            dt,
        );
    }
    kin.pos = body.pos;
    kin.vel = body.vel;
    surface.on_ground = if is_aerial { false } else { body.on_ground };
    if surface.on_ground {
        surface.air_jumps_remaining = MAX_ENEMY_AIR_JUMPS;
    }

    if let Some(motion) = &mut motion.0 {
        let _ = motion.advance(kin.pos, dt);
    }

    if !is_aerial
        && matches!(ai_intent, crate::actor::ai::CharacterAiIntent::Patrol)
        && prev_side_speed.abs() > 1.0
        && kin.vel.dot(perp).abs() < 0.01
    {
        kin.facing *= -1.0;
    }
}

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
        // World gravity DIRECTION at the enemy (down/up/sideways) from
        // `GravityField`, so the enemy falls the way the player does under ANY
        // gravity — including left/right.
        gravity_dir: ae::Vec2,
    ) -> crate::actor::control::ActorControlFrame {
        self.status.hit_flash = (self.status.hit_flash - dt).max(0.0);
        if !self.status.alive {
            self.status.respawn_timer = (self.status.respawn_timer - dt).max(0.0);
            if self.config.tuning.revives_in_place && self.status.respawn_timer <= 0.0 {
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

        let ai = evaluate_enemy_ai_output(
            self.kin.pos,
            target_pos,
            &self.config.brain,
            &self.config.tuning,
            self.attack,
            self.status.alive,
        );
        self.status.ai_mode = ai.mode;

        let is_aerial = self.surface.gravity_scale <= 0.001;
        let is_surface_walker = self.config.tuning.surface_walker;

        if is_surface_walker {
            self.step_surface_walker(world, nearest_neighbor, dt);
        } else {
            integrate_standard_enemy_body(
                world,
                self.kin,
                self.surface,
                self.motion,
                &self.config.tuning,
                ai.intent,
                is_aerial,
                &frame,
                dt,
                gravity_dir,
            );
        }

        if self.config.tuning.attacks_player && frame.facing.abs() > 0.001 {
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

        // Emergent riding for a surface-walker: it is GLUED to its surface (it crawls
        // floors, walls, ceilings), so a MOVING surface carries it by the FULL
        // velocity — both axes, not just the gravity-perpendicular component a
        // gravity-resting body gets. Probe toward the surface it's clinging to.
        {
            let toward_surface = -self.surface.surface_normal;
            let probe = ae::Aabb::new(self.kin.pos + toward_surface * 2.0, self.kin.size * 0.5);
            if let Some(block) = world.first_overlapping_block(probe, surface_solid_pred) {
                self.kin.pos += block.velocity;
            }
        }

        let n = self.surface.surface_normal;
        let speed = self.config.tuning.patrol_speed;
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
                gravity_dir: ae::Vec2::new(0.0, 1.0),
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
    // matching the cluster component accessors.

    pub fn aabb(&self) -> ae::Aabb {
        let size = if self.config.tuning.surface_walker && self.surface.surface_normal.x.abs() > 0.5
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
        if self.config.tuning.is_sandbag {
            FeatureVisualKind::TrainingDummy
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
        enemy_attack_aabb_dir(self.kin.pos, self.kin.size, self.kin.facing, axis)
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
        self.attack.cooldown = ENEMY_ATTACK_COOLDOWN * self.config.tuning.attack_cooldown_mult;
        self.status.ai_mode = crate::actor::ai::CharacterAiMode::Telegraph;
        self.attack.pending_axis = if attack_axis.length_squared() > 0.01 {
            attack_axis.normalize_or_zero()
        } else {
            ae::Vec2::new(self.kin.facing, 0.0)
        };
        true
    }

    pub fn body_damage_aabb(&self) -> Option<ae::Aabb> {
        if !self.config.tuning.body_contact_damage {
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
            damage: self.config.tuning.damage_amount,
            source: HitSource::EnemyBody,
            attacker: None,
            target: HitTarget::Player(player_entity),
            mode: HitMode::Knockback,
            knockback: Some(HitKnockback {
                dir: (player_body.center().x - self.kin.pos.x).signum_or(self.kin.facing),
                strength: self.config.tuning.contact_strength,
                source_pos: self.kin.pos,
                impact_pos: impact,
            }),
            ignored_targets: Vec::new(),
        })
    }

    pub fn reset_to_spawn(&mut self) {
        // Restore the authored spatial baseline. `tuning` / `brain_spec`
        // are projected once at spawn and never mutate at runtime (no
        // entity morphs its archetype in place), so they already hold the
        // baseline — there is nothing to re-project here.
        self.kin.size = self.config.spawn.size;
        self.kin.pos = self.config.spawn.pos;
        self.kin.vel = ae::Vec2::ZERO;
        self.status.alive = true;
        self.status.health = crate::actor::Health::new(self.config.tuning.max_health);
        *self.attack = ActorAttackState::default();
        self.status.respawn_timer = 0.0;
        self.status.hit_flash = 0.0;
        self.status.ai_mode = crate::actor::ai::CharacterAiMode::Idle;
        self.kin.facing = -1.0;
        *self.surface = ActorSurfaceState {
            on_ground: false,
            surface_normal: ae::Vec2::new(0.0, -1.0),
            gravity_scale: if self.config.tuning.is_aerial {
                0.0
            } else {
                1.0
            },
            air_jumps_remaining: MAX_ENEMY_AIR_JUMPS,
        };
    }
}
