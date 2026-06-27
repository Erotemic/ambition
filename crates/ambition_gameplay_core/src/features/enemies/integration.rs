//! Actor physics/AI integration: the per-frame tick that drives actor
//! movement + attack geometry through the [`ActorMut`] ECS view. Grounded AND
//! aerial actors run the EXACT shared player movement pipeline
//! ([`ActorMut::integrate_body`] → `ae::update_body_with_tuning_clusters`,
//! borrowing the actor's `kin` + [`ActorBody`] clusters as one `PlayerClustersMut`
//! view) — the pipeline picks the flight limb vs the grounded spine from
//! `flight.fly_enabled`; surface-walkers keep their glued crawl. Attack AABBs are
//! derived here; archetype tuning comes from the [`super::EnemyRoster`].

use super::super::ecs::actor_clusters::ActorMut;
use super::super::*;
use super::*;
use ambition_platformer_primitives::kinematic;

/// Enemy physics/AI integration, operating directly on the authoritative
/// ECS components through the [`ActorMut`] view.
pub(crate) fn enemy_attack_aabb_dir(
    pos: ae::Vec2,
    size: ae::Vec2,
    facing: f32,
    axis_local: ae::Vec2,
    gravity_dir: ae::Vec2,
) -> ae::Aabb {
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let axis = if axis_local.length_squared() > 0.01 {
        axis_local.normalize_or_zero()
    } else {
        ae::Vec2::new(facing, 0.0)
    };
    let horizontal = axis.x.abs() >= axis.y.abs();
    let (center_local, half_local) = if horizontal {
        let side = if axis.x.abs() > 0.1 {
            axis.x.signum()
        } else {
            facing
        };
        (
            ae::Vec2::new(side * (size.x * 0.55 + 24.0), -4.0),
            ae::Vec2::new(34.0, 28.0),
        )
    } else if axis.y < 0.0 {
        let half = ae::Vec2::new(16.0, 36.0);
        (ae::Vec2::new(0.0, -(size.y * 0.5 + half.y + 4.0)), half)
    } else {
        let half = ae::Vec2::new(36.0, 20.0);
        (ae::Vec2::new(0.0, size.y * 0.5 + half.y - 2.0), half)
    };
    ae::Aabb::new(
        pos + frame.to_world(center_local),
        frame.to_world_half(half_local),
    )
}

fn evaluate_enemy_ai_output(
    pos: ae::Vec2,
    target_pos: ae::Vec2,
    brain: &ambition_characters::actor::EnemyBrain,
    tuning: &crate::combat::ActorTuning,
    attack: &crate::features::ActorAttackState,
    alive: bool,
) -> ambition_characters::actor::ai::CharacterAiOutput {
    let recover_remaining =
        if attack.cooldown > 0.0 && attack.windup_timer <= 0.0 && attack.active_timer <= 0.0 {
            attack.cooldown.min(0.30)
        } else {
            0.0
        };
    let effective_aggro_radius = match brain {
        ambition_characters::actor::EnemyBrain::Passive => 0.0,
        ambition_characters::actor::EnemyBrain::Guard { leash_radius } => *leash_radius,
        _ => tuning.aggro_radius,
    };
    ambition_characters::actor::ai::evaluate_character_ai_output(ambition_characters::actor::ai::CharacterAiSnapshot {
        actor_pos: pos,
        player_pos: target_pos,
        aggro_radius: effective_aggro_radius,
        attack_range: tuning.attack_range,
        attack_windup_remaining: attack.windup_timer,
        attack_active_remaining: attack.active_timer,
        attack_recover_remaining: recover_remaining,
        stun_remaining: 0.0,
        alive,
        patrol_enabled: !tuning.is_sandbag && !matches!(brain, ambition_characters::actor::EnemyBrain::Passive),
    })
}

impl<'a> ActorMut<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: FeatureCombatTuning,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
        _is_mounted: bool,
        frame: ambition_characters::actor::control::ActorControlFrame,
        // World gravity DIRECTION at the enemy (down/up/sideways) from
        // `GravityField`, so the enemy falls the way the player does under ANY
        // gravity — including left/right.
        gravity_dir: ae::Vec2,
    ) -> (
        ambition_characters::actor::control::ActorControlFrame,
        ae::FrameEvents,
    ) {
        self.status.hit_flash = (self.status.hit_flash - dt).max(0.0);
        if !self.status.alive {
            self.status.respawn_timer = (self.status.respawn_timer - dt).max(0.0);
            if self.config.tuning.revives_in_place && self.status.respawn_timer <= 0.0 {
                self.status.alive = true;
                self.health.reset();
                self.kin.pos = self.config.spawn.pos;
                self.kin.vel = ae::Vec2::ZERO;
                self.status.hit_flash = 0.24;
            }
            self.status.ai_mode = ambition_characters::actor::ai::CharacterAiMode::Dead;
            return (
                ambition_characters::actor::control::ActorControlFrame::neutral(),
                ae::FrameEvents::default(),
            );
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

        let is_surface_walker = self.config.tuning.surface_walker;

        // Dash is no longer a bespoke actor mechanic: the body runs the SHARED
        // player dash limb (the real dash impulse + window), gated by the
        // `ActorBody` ability mask (`from_caps`, dash = `can_dash`) and driven by
        // the brain's `dash_pressed` through `to_input_state` — invariant I3, the
        // pipeline owns the burst. (blink / shield are still resolved below on the
        // capability path; folding them needs the aerial reconciliation too.)

        let move_events = if is_surface_walker {
            self.step_surface_walker(world, nearest_neighbor, dt, gravity_dir);
            ae::FrameEvents::default()
        } else {
            // Grounded AND aerial bodies run the ONE shared movement pipeline; it
            // picks the flight limb vs the grounded spine internally from
            // `flight.fly_enabled` (set for aerial bodies at spawn / by the fly
            // toggle). The bespoke aerial integrator is gone. Its `FrameEvents`
            // (blink teleports, etc.) flow out to the driver.
            self.integrate_body(world, ai.intent, &frame, dt, gravity_dir)
        };

        // Shield is the shared pipeline limb now (folded off the actor's own
        // `resolve_shield` call): `integrate_body`'s control phase resolved the
        // `shield_held` intent on `body.shield` (ability-gated by the mask,
        // dash-blocked by the pipeline dash). Bridge it back onto `status.shield_raised`
        // — the bool the actor DAMAGE path reads to negate a guarded faced-side hit.
        // (Surface-walkers don't run the pipeline; they never raise a guard.)
        self.status.shield_raised = self.body.0.shield.active;

        // Face the brain's committed direction whenever it commits one. Hostile
        // chasers AND peaceful patrollers/flyers both set `frame.facing`; a
        // standstill/idle brain leaves it ~0 so facing is preserved. (Previously
        // gated on `attacks_player`, which left peaceful actors facing-frozen.)
        if frame.facing.abs() > 0.001 {
            self.kin.facing = frame.facing.signum();
        }

        if frame.fire.is_some() {
            self.status.ai_mode = ambition_characters::actor::ai::CharacterAiMode::Attack;
        }
        (frame, move_events)
    }

    /// Integration through the **shared player movement pipeline**
    /// (`ae::update_body_with_tuning_clusters`) — the unification's core seam, for
    /// BOTH grounded and aerial bodies. The actor's `kin` supplies the kinematics;
    /// its persistent [`ActorBody`] supplies the 18 ancillary movement clusters.
    /// The brain's `ActorControlFrame` becomes the body's `InputState`, so an actor
    /// runs / jumps / coyote-grace-jumps / dashes / **flies** and collides through
    /// the EXACT code the human player uses — no parallel enemy integrator.
    ///
    /// **Grounded** bodies map `locomotion → run` + `jump_pressed → buffered jump`.
    /// **Flying** bodies (`flight.fly_enabled`) are steered by the brain's exact
    /// `velocity_target` (the free-mover command): it is projected into the body
    /// frame and normalised by the flight terminal so the shared flight limb steers
    /// toward it at the body's own flight speed — the `velocity_target`→intent
    /// bridge that lets aerial actors share the pipeline.
    ///
    /// The pipeline owns hazard/out-of-bounds as a *flag* (it never teleports an
    /// actor to the player spawn); the actor's damage / OOB systems own that. The
    /// pipeline `FrameEvents` are RETURNED so the driver can react to body events
    /// it cares about (e.g. emit the blink sfx/vfx from `events.blinks`).
    fn integrate_body(
        &mut self,
        world: &ae::World,
        ai_intent: ambition_characters::actor::ai::CharacterAiIntent,
        frame: &ambition_characters::actor::control::ActorControlFrame,
        dt: f32,
        gravity_dir: ae::Vec2,
    ) -> ae::FrameEvents {
        // Wall-stop detection on the gravity-PERPENDICULAR "side" axis the actor
        // walks along (so a patroller reverses when it stalls against a wall,
        // correctly under sideways gravity too).
        let perp = ae::Vec2::new(-gravity_dir.y, gravity_dir.x);
        let prev_side_speed = self.kin.vel.dot(perp);

        let flying = self.body.0.flight.fly_enabled;
        let mut tuning = self.config.tuning.movement.body_tuning(
            self.config.tuning.max_run_speed,
            gravity_dir,
            self.surface.gravity_scale,
        );
        // Flight tuning from the actor's chase speed: the body flies at its own
        // speed, steers responsively (matching the old floating accel), and does
        // NOT idle-bob like the player (hover speed 0) — an AI flyer holds station.
        let flight_speed = self
            .config
            .tuning
            .chase_speed
            .max(self.config.tuning.max_run_speed)
            .max(1.0);
        tuning.flight_terminal_speed = flight_speed;
        tuning.flight_accel = (flight_speed * 3.0).max(900.0);
        tuning.flight_drag = (flight_speed * 3.0).max(900.0);
        tuning.flight_hover_speed = 0.0;
        tuning.flight_hover_hz = 0.0;

        let input = if flying {
            // `velocity_target` (world px/s) → flight stick intent: project onto the
            // body frame the flight limb integrates in, normalise by the terminal so
            // a full-speed command maps to a full-deflection stick.
            let fref = ae::AccelerationFrame::new(gravity_dir);
            let vt = frame.velocity_target;
            let mut i = frame.to_input_state();
            i.axis_x = (vt.dot(fref.side) / flight_speed).clamp(-1.0, 1.0);
            i.axis_y = (vt.dot(fref.down) / flight_speed).clamp(-1.0, 1.0);
            i
        } else {
            frame.to_input_state()
        };
        let on_ground = self.surface.on_ground;
        let air_jumps = self.surface.air_jumps_remaining;

        // Borrow the actor's persistent movement clusters + the shared kinematics
        // as ONE `PlayerClustersMut` view (kin = the single kinematic source; no
        // duplication). Seed the pipeline's ground/jump state from the actor's
        // surface truth so coyote + jump gates start correct.
        let body = &mut self.body.0;
        body.ground.on_ground = on_ground;
        body.jump.air_jumps_available = air_jumps;
        let mut clusters = ae::PlayerClustersMut {
            kinematics: self.kin,
            abilities: &body.abilities,
            base_size: &mut body.base_size,
            ground: &mut body.ground,
            wall: &mut body.wall,
            jump: &mut body.jump,
            dash: &mut body.dash,
            flight: &mut body.flight,
            blink: &mut body.blink,
            ledge: &mut body.ledge,
            dodge: &mut body.dodge,
            shield: &mut body.shield,
            body_mode: &mut body.body_mode,
            env_contact: &mut body.env_contact,
            mana: &mut body.mana,
            offense: &mut body.offense,
            action_buffer: &mut body.action_buffer,
            lifetime: &mut body.lifetime,
            combo_trace: &mut body.combo_trace,
        };
        let events = ae::update_body_with_tuning_clusters(world, &mut clusters, input, dt, tuning);
        // Reflect the pipeline's ground contact back onto the actor surface (the
        // surface state the rest of the actor systems + rendering read). A flying
        // body is never grounded.
        self.surface.on_ground = !flying && clusters.ground.on_ground;
        self.surface.air_jumps_remaining = clusters.jump.air_jumps_available;
        if self.surface.on_ground {
            self.surface.air_jumps_remaining = MAX_ENEMY_AIR_JUMPS;
        }

        if let Some(motion) = &mut self.motion.0 {
            let _ = motion.advance(self.kin.pos, dt);
        }
        // Patrol stall → reverse (a wall-stopped patroller turns around).
        if matches!(ai_intent, ambition_characters::actor::ai::CharacterAiIntent::Patrol)
            && prev_side_speed.abs() > 1.0
            && self.kin.vel.dot(perp).abs() < 0.01
        {
            self.kin.facing *= -1.0;
        }
        events
    }

    fn step_surface_walker(
        &mut self,
        world: &ae::World,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
        gravity_dir: ae::Vec2,
    ) {
        if !self.surface.on_ground {
            self.fall_until_landed(world, dt, gravity_dir);
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
        self.fall_until_landed(world, dt, gravity_dir);
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

    fn fall_until_landed(&mut self, world: &ae::World, dt: f32, gravity_dir: ae::Vec2) {
        let mut body = kinematic::KinematicBody {
            pos: self.kin.pos,
            vel: self.kin.vel,
            size: self.kin.size,
            on_ground: self.surface.on_ground,
            facing: self.kin.facing,
        };
        kinematic::step_kinematic(
            &mut body,
            world,
            kinematic::KinematicTuning {
                gravity: self.config.tuning.movement.gravity,
                max_fall_speed: self.config.tuning.movement.max_fall_speed,
                // Detached surface-walkers fall toward the active acceleration frame,
                // then reattach with their surface normal opposite local down.
                gravity_dir,
            },
            kinematic::KinematicInputs {
                drop_through: false,
            },
            dt,
        );
        self.kin.pos = body.pos;
        self.kin.vel = body.vel;
        self.surface.on_ground = body.on_ground;
        if body.on_ground {
            self.surface.surface_normal = -gravity_dir.normalize_or(ae::Vec2::new(0.0, 1.0));
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
        self.attack_aabb_dir(ae::Vec2::new(self.kin.facing, 0.0))
    }

    pub fn attack_telegraph_aabb(&self) -> ae::Aabb {
        self.attack_aabb()
    }

    pub fn attack_aabb_dir(&self, axis: ae::Vec2) -> ae::Aabb {
        let gravity_dir = -self
            .surface
            .surface_normal
            .normalize_or(ae::Vec2::new(0.0, -1.0));
        enemy_attack_aabb_dir(
            self.kin.pos,
            self.kin.size,
            self.kin.facing,
            axis,
            gravity_dir,
        )
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
        self.status.ai_mode = ambition_characters::actor::ai::CharacterAiMode::Telegraph;
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
        attacker: bevy::prelude::Entity,
        player_entity: bevy::prelude::Entity,
        player_body: ae::Aabb,
    ) -> Option<HitEvent> {
        let body_damage = self.body_damage_aabb()?;
        if !body_damage.strict_intersects(player_body) {
            return None;
        }
        let impact = midpoint(player_body.center(), body_damage.center());
        Some(HitEvent {
            volume: body_damage.into(),
            damage: self.config.tuning.damage_amount,
            source: HitSource::EnemyBody,
            attacker: Some(attacker),
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
        *self.health = crate::actor::BodyHealth::new(ambition_characters::actor::Health::new(self.config.tuning.max_health));
        *self.attack = ActorAttackState::default();
        self.status.respawn_timer = 0.0;
        self.status.hit_flash = 0.0;
        self.status.ai_mode = ambition_characters::actor::ai::CharacterAiMode::Idle;
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

#[cfg(test)]
mod dash_tests {
    //! S3d: dash as a body-enforced capability. These drive the REAL grounded
    //! integration (`ActorMut::update` → the shared spine), so they prove the
    //! body owns the burst — a possessing human and an AI brain dash identically
    //! because both only set `dash_pressed` (invariants I2/I3).
    use super::*;
    use crate::features::ecs::actor_clusters::{ActorBody, ActorClusterSeed, ActorMut};
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::actor::EnemyBrain;

    /// A wide solid floor; bodies rest on its top face at y = 100.
    fn floored_world() -> ae::World {
        ae::World::new(
            "dash_test",
            ae::Vec2::new(4000.0, 800.0),
            ae::Vec2::ZERO,
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(-2000.0, 100.0),
                ae::Vec2::new(4000.0, 80.0),
            )],
        )
    }

    /// Drop a grounded body (dash-capable iff `can_dash`) and drive a full-right
    /// dash for `ticks` steps; return how far it traveled along +x.
    fn dash_run(can_dash: bool, ticks: u32) -> f32 {
        let world = floored_world();
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
        let mut seed = ActorClusterSeed::new(
            "dasher".to_string(),
            "Dasher".to_string(),
            aabb,
            EnemyBrain::Custom("cellular_automaton_fighter".into()),
            &[],
        );
        // Rest the body on the floor top (y = 100): center a half-height above it.
        let half_h = seed.kin.size.y * 0.5;
        seed.kin.pos = ae::Vec2::new(0.0, 100.0 - half_h);
        seed.kin.vel = ae::Vec2::ZERO;
        seed.kin.facing = 1.0;
        seed.surface.on_ground = true;
        seed.surface.gravity_scale = 1.0;
        seed.caps.can_dash = can_dash;
        // The dash ability lives on the movement body's mask (derived from caps);
        // rebuild it after overriding the cap so the pipeline dash limb matches.
        seed.body = ActorBody::from_caps(&seed.caps, false);
        let start_x = seed.kin.pos.x;
        let mut em = ActorMut {
            kin: &mut seed.kin,
            status: &mut seed.status,
            health: &mut seed.health,
            surface: &mut seed.surface,
            attack: &mut seed.attack,
            config: &mut seed.config,
            motion: &mut seed.motion,
            body: &mut seed.body,
            caps: &seed.caps,
        };
        let mut frame = ActorControlFrame::neutral();
        frame.locomotion = ae::Vec2::new(1.0, 0.0);
        frame.dash_pressed = true;
        frame.facing = 1.0;
        let dt = 1.0 / 60.0;
        for _ in 0..ticks {
            em.update(
                &world,
                ae::Vec2::new(2000.0, em.kin.pos.y),
                FeatureCombatTuning::default(),
                None,
                dt,
                false,
                frame,
                ae::Vec2::new(0.0, 1.0),
            );
        }
        em.kin.pos.x - start_x
    }

    #[test]
    fn a_dash_capable_body_covers_more_ground_than_a_walker_over_the_window() {
        // ~the dash window (DASH_TIME_S = 0.18 s ≈ 11 ticks), plus a tick of slack.
        let dashed = dash_run(true, 12);
        let walked = dash_run(false, 12);
        assert!(
            dashed > walked * 1.3,
            "the dash burst should cover meaningfully more ground than a top-speed \
             walk over the same window: dashed={dashed:.1}px walked={walked:.1}px"
        );
    }

    #[test]
    fn an_uncapable_body_does_not_burst_and_just_walks() {
        // Sanity: with the capability off, `dash_pressed` never opens a window —
        // the body's attack state stays dash-inert (the body enforces the kit).
        let world = floored_world();
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
        let mut seed = ActorClusterSeed::new(
            "walker".to_string(),
            "Walker".to_string(),
            aabb,
            EnemyBrain::Custom("cellular_automaton_fighter".into()),
            &[],
        );
        let half_h = seed.kin.size.y * 0.5;
        seed.kin.pos = ae::Vec2::new(0.0, 100.0 - half_h);
        seed.surface.on_ground = true;
        seed.surface.gravity_scale = 1.0;
        seed.caps.can_dash = false;
        seed.body = ActorBody::from_caps(&seed.caps, false);
        let mut em = ActorMut {
            kin: &mut seed.kin,
            status: &mut seed.status,
            health: &mut seed.health,
            surface: &mut seed.surface,
            attack: &mut seed.attack,
            config: &mut seed.config,
            motion: &mut seed.motion,
            body: &mut seed.body,
            caps: &seed.caps,
        };
        let mut frame = ActorControlFrame::neutral();
        frame.locomotion = ae::Vec2::new(1.0, 0.0);
        frame.dash_pressed = true;
        em.update(
            &world,
            ae::Vec2::new(2000.0, em.kin.pos.y),
            FeatureCombatTuning::default(),
            None,
            1.0 / 60.0,
            false,
            frame,
            ae::Vec2::new(0.0, 1.0),
        );
        assert!(
            em.body.0.dash.timer <= 0.0,
            "a body without the dash capability must not open a dash window"
        );
    }

    /// Witness for the aerial reconciliation: an aerial body (fly_enabled) is
    /// steered by the brain's world-space `velocity_target` THROUGH the shared
    /// pipeline's flight limb (the `velocity_target`→stick-intent bridge). It flies
    /// toward the command and holds altitude (gravity-free flight, no idle bob).
    #[test]
    fn an_aerial_body_steers_toward_its_velocity_target_through_the_flight_limb() {
        let world = floored_world();
        // Hover in open air well above the floor (floor top is y = 100).
        let aabb = ae::Aabb::new(ae::Vec2::new(0.0, -200.0), ae::Vec2::new(24.0, 24.0));
        let mut seed = ActorClusterSeed::new(
            "flyer".to_string(),
            "Flyer".to_string(),
            aabb,
            EnemyBrain::Custom("cellular_automaton_fighter".into()),
            &[],
        );
        seed.kin.pos = ae::Vec2::new(0.0, -200.0);
        seed.kin.vel = ae::Vec2::ZERO;
        seed.surface.gravity_scale = 0.0;
        seed.surface.on_ground = false;
        // Aerial body: fly ability + fly_enabled from spawn.
        seed.body = ActorBody::from_caps(&seed.caps, true);
        let start = seed.kin.pos;
        let mut em = ActorMut {
            kin: &mut seed.kin,
            status: &mut seed.status,
            health: &mut seed.health,
            surface: &mut seed.surface,
            attack: &mut seed.attack,
            config: &mut seed.config,
            motion: &mut seed.motion,
            body: &mut seed.body,
            caps: &seed.caps,
        };
        let mut frame = ActorControlFrame::neutral();
        // Command a pure +x world velocity (the free-mover modality).
        frame.velocity_target = ae::Vec2::new(300.0, 0.0);
        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            em.update(
                &world,
                ae::Vec2::new(2000.0, em.kin.pos.y),
                FeatureCombatTuning::default(),
                None,
                dt,
                false,
                frame,
                ae::Vec2::new(0.0, 1.0),
            );
        }
        assert!(
            em.kin.pos.x - start.x > 100.0,
            "an aerial body should fly toward its +x velocity_target through the \
             shared flight limb; moved {:.1}px",
            em.kin.pos.x - start.x
        );
        assert!(
            (em.kin.pos.y - start.y).abs() < 50.0,
            "gravity-free flight holds altitude (no fall, no idle hover bob); \
             drifted {:.1}px on y",
            em.kin.pos.y - start.y
        );
        assert!(
            !em.surface.on_ground,
            "a flying body is never grounded"
        );
    }
}
