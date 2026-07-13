//! Actor physics/AI integration: the per-frame tick that drives actor
//! movement + attack geometry through the [`ActorMut`] ECS view. EVERY actor —
//! grounded, aerial, and the adhesive crawler — runs the one shared movement
//! kernel ([`ActorMut::integrate_body`] → `ae::step_motion`, borrowing the
//! actor's `kin` + [`ActorBody`] clusters as one `BodyClustersMut` view). The
//! kernel picks the physics by the body's explicit `MotionModel`; the flight
//! limb vs grounded spine split rides `flight.fly_enabled` inside the
//! axis-swept policy. Attack AABBs are derived here; archetype tuning comes
//! from the [`super::CharacterRoster`].

use super::super::ecs::actor_clusters::ActorMut;
use super::super::*;
use super::*;

/// Minimum knockback strength a body-contact hit imparts on the struck body, even
/// when the archetype authored `contact_strength = 0`. Guarantees a body that
/// overlaps an enemy is pushed back OUT of its box rather than sitting inside it
/// taking a hit every i-frame window. Feel-tunable.
const BODY_CONTACT_MIN_KNOCKBACK: f32 = 0.6;

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
    brain: &ambition_entity_catalog::placements::CharacterBrain,
    tuning: &crate::features::ecs::actor_tuning::ActorTuning,
    attack: &crate::features::BodyMelee,
    alive: bool,
) -> ambition_characters::actor::ai::CharacterAiOutput {
    let recover_remaining =
        if attack.on_cooldown() && !attack.is_winding_up() && !attack.is_active() {
            attack.cooldown.min(0.30)
        } else {
            0.0
        };
    let effective_aggro_radius = match brain {
        ambition_entity_catalog::placements::CharacterBrain::Passive => 0.0,
        ambition_entity_catalog::placements::CharacterBrain::Guard { leash_radius } => {
            *leash_radius
        }
        _ => tuning.aggro_radius,
    };
    ambition_characters::actor::ai::evaluate_character_ai_output(
        ambition_characters::actor::ai::CharacterAiSnapshot {
            actor_pos: pos,
            player_pos: target_pos,
            aggro_radius: effective_aggro_radius,
            attack_range: tuning.attack_range,
            attack_windup_remaining: attack.windup_remaining(),
            attack_active_remaining: attack.active_remaining(),
            attack_recover_remaining: recover_remaining,
            stun_remaining: 0.0,
            alive,
            patrol_enabled: !tuning.is_sandbag
                && !matches!(
                    brain,
                    ambition_entity_catalog::placements::CharacterBrain::Passive
                ),
        },
    )
}

impl<'a> ActorMut<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: FeatureCombatTuning,
        dt: f32,
        _is_mounted: bool,
        frame: ambition_characters::actor::control::ActorControlFrame,
        motion_model: &mut crate::features::MotionModel,
        // The body's current acceleration/reference frame, resolved ONCE by the
        // environment (the driver) for this body tick. Input projection, the
        // active policy, and every frame-relative limb consume this same value.
        motion_frame: ae::MotionFrame,
        // Post-hit stagger inputs (§A2 step 7): the body's live hitstun /
        // recoil-lock timers (from its `BodyCombat`) + the feel tuning, applied
        // to the FINAL InputState by the SAME gate the player's input bridge
        // uses. (hitstun_timer, recoil_lock_timer).
        feel: crate::time::feel::SandboxFeelTuning,
        stagger: (f32, f32),
    ) -> (
        ambition_characters::actor::control::ActorControlFrame,
        ae::FrameEvents,
    ) {
        // Reaction timers (hit_flash, post-hit i-frame) live on the body's
        // `BodyCombat` now — decremented + the respawn blink applied in the actor
        // driver, where that component is in scope.
        if !self.health.alive() {
            self.status.respawn_timer = (self.status.respawn_timer - dt).max(0.0);
            if matches!(
                self.config.tuning.respawn,
                ambition_entity_catalog::placements::RespawnPolicy::InPlace(_)
            ) && self.status.respawn_timer <= 0.0
            {
                // `health.reset()` IS the revive — restoring HP makes `alive()` true.
                self.health.reset();
                // Respawn is a discrete transit: arrive at rest with departure
                // contacts and any attachment reconciled (ADR 0024 authority).
                let spawn = self.config.spawn.pos;
                ae::movement::transit_body(
                    motion_model,
                    &mut self.clusters_mut(),
                    spawn,
                    ae::movement::TransitVelocity::Zero,
                );
            }
            self.status.ai_mode = ambition_characters::actor::ai::CharacterAiMode::Dead;
            return (
                ambition_characters::actor::control::ActorControlFrame::neutral(),
                ae::FrameEvents::default(),
            );
        }

        // Melee is NOT advanced here anymore. The body-generic `advance_body_melee`
        // phase (Combat set) ticks EVERY body's `BodyMelee` swing + cooldown floors
        // and spawns the active-edge strike, so movement integration owns movement
        // only. The AI reads `self.attack` as of the previous frame's advance — a
        // consistent one-frame view, no double-tick.
        let _ = tuning.enemy_attack_active;

        let ai = evaluate_enemy_ai_output(
            self.kin.pos,
            target_pos,
            &self.config.brain,
            &self.config.tuning,
            self.attack,
            self.health.alive(),
        );
        self.status.ai_mode = ai.mode;

        // ONE integration arm for every actor: the kernel dispatches on the
        // body's explicit MotionModel (axis-swept, surface momentum, or the
        // adhesive crawler — the former hidden surface-walker path).
        let move_events = self.integrate_body(
            world,
            ai.intent,
            &frame,
            motion_model,
            dt,
            motion_frame,
            feel,
            stagger,
        );

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

    /// Integration through the **shared movement kernel**
    /// (`ae::step_motion`) — the unification's core seam, for EVERY actor body.
    /// The actor's `kin` supplies the kinematics; its persistent [`ActorBody`]
    /// supplies the ancillary movement clusters. The brain's `ActorControlFrame`
    /// becomes the body's typed `InputState`, so an actor runs / jumps /
    /// coyote-grace-jumps / dashes / **flies** / crawls and collides through the
    /// EXACT code the human player uses — no parallel enemy integrator.
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
    #[allow(clippy::too_many_arguments)]
    fn integrate_body(
        &mut self,
        world: &ae::World,
        ai_intent: ambition_characters::actor::ai::CharacterAiIntent,
        frame: &ambition_characters::actor::control::ActorControlFrame,
        motion_model: &mut crate::features::MotionModel,
        dt: f32,
        motion_frame: ae::MotionFrame,
        feel: crate::time::feel::SandboxFeelTuning,
        stagger: (f32, f32),
    ) -> ae::FrameEvents {
        // Wall-stop detection on the frame-PERPENDICULAR "side" axis the actor
        // walks along (so a patroller reverses when it stalls against a wall,
        // correctly under sideways gravity too).
        let perp = motion_frame.side();
        let prev_side_speed = self.kin.vel.dot(perp);

        let flying = self.flight.fly_enabled;
        let mut tuning = self
            .config
            .tuning
            .movement
            .body_tuning(self.config.tuning.max_run_speed);
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
        // Direct-velocity free-movers (bosses) take their commanded velocity verbatim
        // through the shared flight limb — byte-identical to the old SNAP float (AS4).
        tuning.flight_direct_velocity = self.config.tuning.flight_direct_velocity;

        let mut input = if flying {
            // `velocity_target` (world px/s) → flight stick intent: project onto the
            // body frame the flight limb integrates in, normalise by the terminal so
            // a full-speed command maps to a full-deflection stick.
            let vt = frame.velocity_target;
            let mut i = frame.to_input_state();
            let local_target = motion_frame.to_local(vt);
            i.axes = ae::LocalAxes::new(
                (local_target.x / flight_speed).clamp(-1.0, 1.0),
                (local_target.y / flight_speed).clamp(-1.0, 1.0),
            );
            i
        } else {
            frame.to_input_state()
        };
        // Post-hit stagger on the FINAL InputState (§A2 step 7) — the SAME gate
        // the player's input bridge applies: recoil-lock is a hard zero (the
        // knockback carries the body, it can't steer back in), hitstun reduces
        // movement authority but preserves the attack verb. Applied after the
        // flight-axis override so a knocked flyer loses its steering too.
        let (hitstun_timer, recoil_lock_timer) = stagger;
        crate::features::ecs::attack::apply_post_hit_input_gates(
            &mut input,
            feel,
            hitstun_timer,
            recoil_lock_timer,
        );
        // Live authored tuning refreshes only the active policy's parameters —
        // the frame is environmental and cannot ride along.
        if let crate::features::MotionModel::AxisSwept(axis) = motion_model {
            axis.params = tuning.axis_swept_params();
        }
        let mut clusters = self.clusters_mut();
        let result = ae::step_motion(
            motion_model,
            &mut clusters,
            ae::MotionStepContext {
                world,
                input,
                frame: motion_frame,
                facing_intent: frame.facing,
                dt,
            },
        );
        drop(clusters);
        // Publish the body's support/orientation fact from the ONE kernel
        // result: a crawler's clung surface, a supported body's contact normal,
        // anti-down otherwise. This keeps the read-model live for every body
        // (§B2) without any policy-specific branch.
        self.surface.surface_normal = result.surface_normal;
        let events = result.events;
        // Two actor policies applied on the ONE ground/jump authority: a flying body
        // is never grounded (the collision sweep can still find support under a
        // hovering flyer), and a grounded body refreshes its air jumps each tick
        // (more forgiving than the player's jump-only refresh — an actor tuning).
        if flying {
            self.ground.on_ground = false;
        }
        if self.ground.on_ground {
            self.jump.air_jumps_available = MAX_ENEMY_AIR_JUMPS;
        }

        if let Some(motion) = &mut self.motion.0 {
            let _ = motion.advance(self.kin.pos, dt);
        }
        // Patrol stall → reverse (a wall-stopped patroller turns around).
        if matches!(
            ai_intent,
            ambition_characters::actor::ai::CharacterAiIntent::Patrol
        ) && prev_side_speed.abs() > 1.0
            && self.kin.vel.dot(perp).abs() < 0.01
        {
            self.kin.facing *= -1.0;
        }
        events
    }

    // ---- Consumer-facing geometry / combat helpers (ports of the
    // matching the cluster component accessors.

    pub fn aabb(&self) -> ae::Aabb {
        // Orientation follows the published support normal — a crawler clung to
        // a wall and a body under sideways gravity both lie ALONG the surface,
        // so the footprint swaps its extents (frame-derived, policy-free).
        let size = if self.surface.surface_normal.x.abs() > 0.5 {
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

    // `begin_melee_attack` is deleted. A body's melee swing is now begun by the
    // body-generic `combat::attack::start_body_melee` phase (which resolves the
    // swing through the SAME `attack_spec_from_view` pipeline the player uses and
    // arms the recovery cooldown from the body's authored
    // `ENEMY_ATTACK_COOLDOWN * attack_cooldown_mult`), and advanced/spawned by
    // `advance_body_melee` — one melee lifecycle for every body.

    pub fn body_damage_aabb(&self) -> Option<ae::Aabb> {
        if !self.config.tuning.body_contact_damage {
            return None;
        }
        Some(self.aabb())
    }

    /// Snapshot this actor's live body-contact attack (its damage box + the
    /// tuning/frame facts the victim pass needs), taken while the attacker's
    /// clusters are borrowed. The victim resolution runs AFTER the borrow ends
    /// (fable review 2026-07-02 §A4: contact damage targets any body, so the
    /// victim query aliases the attacker query and the two passes must split).
    pub fn contact_attack(&self) -> Option<ContactAttack> {
        let body_damage = self.body_damage_aabb()?;
        // The attacker's live reference frame (§B2 keeps `surface_normal`
        // current for every body): knockback separates along ITS side axis,
        // not screen-X.
        let down = -self
            .surface
            .surface_normal
            .normalize_or(ae::Vec2::new(0.0, -1.0));
        Some(ContactAttack {
            volume: body_damage,
            damage: self.config.tuning.damage_amount,
            // Body contact ALWAYS imparts a separating push: a body that runs into
            // an enemy is shoved out of its box, so it doesn't sit inside taking
            // a hit every i-frame window. Most archetypes author `contact_strength
            // = 0` (it tuned the OLD knockback-scaling, not "no knockback"), which
            // read as "you stick to the enemy" — the floor fixes that. Feel-tunable.
            strength: self
                .config
                .tuning
                .contact_strength
                .max(BODY_CONTACT_MIN_KNOCKBACK),
            source_pos: self.kin.pos,
            facing: self.kin.facing,
            frame_side: ae::AccelerationFrame::new(down).side,
        })
    }

    pub fn reset_to_spawn(&mut self, motion_model: &mut crate::features::MotionModel) {
        // Restore the authored spatial baseline. `tuning` / `brain_spec`
        // are projected once at spawn and never mutate at runtime (no
        // entity morphs its archetype in place), so they already hold the
        // baseline — there is nothing to re-project here.
        self.kin.size = self.config.spawn.size;
        // Respawn is a discrete transit (ADR 0024 authority): arrive at rest,
        // departure contacts and any attachment reconciled.
        let spawn = self.config.spawn.pos;
        ae::movement::transit_body(
            motion_model,
            &mut self.clusters_mut(),
            spawn,
            ae::movement::TransitVelocity::Zero,
        );
        // Fresh full-HP body → `alive()` is true; no separate liveness flag.
        *self.health = ambition_characters::actor::BodyHealth::new(
            ambition_characters::actor::Health::new(self.config.tuning.max_health),
        );
        *self.attack = BodyMelee::default();
        self.status.respawn_timer = 0.0;
        self.status.ai_mode = ambition_characters::actor::ai::CharacterAiMode::Idle;
        self.kin.facing = -1.0;
        *self.surface = ActorSurfaceState {
            surface_normal: ae::Vec2::new(0.0, -1.0),
            gravity_scale: if self.config.tuning.is_aerial {
                0.0
            } else {
                1.0
            },
        };
        // Ground/jump authority is the shared cluster now — reset it too.
        self.ground.on_ground = false;
        self.jump.air_jumps_available = MAX_ENEMY_AIR_JUMPS;
    }
}

/// An actor's live body-contact attack, snapshotted by [`ActorMut::contact_attack`]
/// so the victim pass can resolve player AND actor victims after the attacker
/// borrow ends. One event builder for every victim kind — the `HitTarget` stamp
/// is the only difference.
pub struct ContactAttack {
    pub volume: ae::Aabb,
    pub damage: i32,
    pub strength: f32,
    pub source_pos: ae::Vec2,
    pub facing: f32,
    /// The attacker's local side axis, for the frame-correct separating push.
    pub frame_side: ae::Vec2,
}

impl ContactAttack {
    pub fn hit_event(
        &self,
        attacker: bevy::prelude::Entity,
        target: bevy::prelude::Entity,
        target_body: ae::Aabb,
        target_is_player: bool,
    ) -> Option<HitEvent> {
        if !self.volume.strict_intersects(target_body) {
            return None;
        }
        let impact = midpoint(target_body.center(), self.volume.center());
        let dir =
            ((target_body.center() - self.source_pos).dot(self.frame_side)).signum_or(self.facing);
        Some(HitEvent {
            volume: self.volume.into(),
            damage: self.damage,
            source: HitSource::EnemyBody,
            attacker: Some(attacker),
            target: if target_is_player {
                HitTarget::Player(target)
            } else {
                HitTarget::Actor(target)
            },
            mode: HitMode::Knockback,
            knockback: Some(HitKnockback {
                dir,
                strength: self.strength,
                source_pos: self.source_pos,
                impact_pos: impact,
                launch_dir: None,
            }),
            ignored_targets: Vec::new(),
        })
    }
}

#[cfg(test)]
mod dash_tests;
