//! Actor physics/AI integration: the per-frame tick that drives actor
//! movement + attack geometry through the [`ActorMut`] ECS view. Grounded AND
//! aerial actors run the EXACT shared player movement pipeline
//! ([`ActorMut::integrate_body`] → `ae::update_body_with_tuning_clusters`,
//! borrowing the actor's `kin` + [`ActorBody`] clusters as one `BodyClustersMut`
//! view) — the pipeline picks the flight limb vs the grounded spine from
//! `flight.fly_enabled`; surface-walkers keep their glued crawl. Attack AABBs are
//! derived here; archetype tuning comes from the [`super::CharacterRoster`].

use super::super::ecs::actor_clusters::ActorMut;
use super::super::*;
use super::*;
use ambition_platformer_primitives::kinematic;

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
    brain: &ambition_characters::actor::CharacterBrain,
    tuning: &crate::combat::ActorTuning,
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
        ambition_characters::actor::CharacterBrain::Passive => 0.0,
        ambition_characters::actor::CharacterBrain::Guard { leash_radius } => *leash_radius,
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
                && !matches!(brain, ambition_characters::actor::CharacterBrain::Passive),
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
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
        _is_mounted: bool,
        frame: ambition_characters::actor::control::ActorControlFrame,
        // World gravity DIRECTION at the enemy (down/up/sideways) from
        // `GravityField`, so the enemy falls the way the player does under ANY
        // gravity — including left/right.
        gravity_dir: ae::Vec2,
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
            if self.config.tuning.revives_in_place && self.status.respawn_timer <= 0.0 {
                // `health.reset()` IS the revive — restoring HP makes `alive()` true.
                self.health.reset();
                self.kin.pos = self.config.spawn.pos;
                self.kin.vel = ae::Vec2::ZERO;
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

        let is_surface_walker = self.config.tuning.surface_walker;

        // Keep the published reference-frame normal LIVE for every body (fable
        // review 2026-07-02 §B2): a surface-walker's normal is its clung surface
        // (written by `step_surface_walker`); everyone else's is anti-gravity AT
        // THEIR POSITION. Before this, non-surface-walkers kept their spawn
        // constant `(0,-1)` forever, so every consumer that derived the body
        // frame from it (shield block side, slash knockback, ranged muzzle/aim)
        // silently stayed in down-gravity space while the movement obeyed the
        // real field.
        if !is_surface_walker {
            self.surface.surface_normal = -gravity_dir;
        }

        // Dash is no longer a bespoke actor mechanic: the body runs the SHARED
        // player dash limb (the real dash impulse + window), gated by the
        // `ActorBody` ability mask (`from_caps`, dash = `can_dash`) and driven by
        // the brain's `dash_pressed` through `to_input_state` — invariant I3, the
        // pipeline owns the burst. (blink / shield are still resolved below on the
        // capability path; folding them needs the aerial reconciliation too.)

        let move_events = if is_surface_walker {
            // Surface-walkers don't run the input pipeline, so the stagger gate
            // doesn't apply — their hit reaction is the cling-detach pop.
            self.step_surface_walker(world, nearest_neighbor, dt, gravity_dir);
            ae::FrameEvents::default()
        } else {
            // Grounded AND aerial bodies run the ONE shared movement pipeline; it
            // picks the flight limb vs the grounded spine internally from
            // `flight.fly_enabled` (set for aerial bodies at spawn / by the fly
            // toggle). The bespoke aerial integrator is gone. Its `FrameEvents`
            // (blink teleports, etc.) flow out to the driver.
            self.integrate_body(world, ai.intent, &frame, dt, gravity_dir, feel, stagger)
        };

        // Shield is the shared pipeline limb now (folded off the actor's own
        // `resolve_shield` call): `integrate_body`'s control phase resolved the
        // `shield_held` intent directly onto the body's `BodyShieldState`
        // (ability-gated by the mask, dash-blocked by the pipeline dash). The actor
        // DAMAGE path reads `shield.active` off that ONE component to negate a
        // guarded faced-side hit — no `status.shield_raised` mirror. (Surface-walkers
        // don't run the pipeline; their shield stays inactive, so they never guard.)

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
    #[allow(clippy::too_many_arguments)]
    fn integrate_body(
        &mut self,
        world: &ae::World,
        ai_intent: ambition_characters::actor::ai::CharacterAiIntent,
        frame: &ambition_characters::actor::control::ActorControlFrame,
        dt: f32,
        gravity_dir: ae::Vec2,
        feel: crate::time::feel::SandboxFeelTuning,
        stagger: (f32, f32),
    ) -> ae::FrameEvents {
        // Wall-stop detection on the gravity-PERPENDICULAR "side" axis the actor
        // walks along (so a patroller reverses when it stalls against a wall,
        // correctly under sideways gravity too).
        let perp = ae::Vec2::new(-gravity_dir.y, gravity_dir.x);
        let prev_side_speed = self.kin.vel.dot(perp);

        let flying = self.flight.fly_enabled;
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
        // Direct-velocity free-movers (bosses) take their commanded velocity verbatim
        // through the shared flight limb — byte-identical to the old SNAP float (AS4).
        tuning.flight_direct_velocity = self.config.tuning.flight_direct_velocity;

        let mut input = if flying {
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
        // Post-hit stagger on the FINAL InputState (§A2 step 7) — the SAME gate
        // the player's input bridge applies: recoil-lock is a hard zero (the
        // knockback carries the body, it can't steer back in), hitstun reduces
        // movement authority but preserves the attack verb. Applied after the
        // flight-axis override so a knocked flyer loses its steering too.
        let (hitstun_timer, recoil_lock_timer) = stagger;
        crate::combat::attack::apply_post_hit_input_gates(
            &mut input,
            feel,
            hitstun_timer,
            recoil_lock_timer,
        );
        // The cluster's ground/jump state persists between ticks (real components,
        // exactly like the player), so the pipeline reads coyote + jump gates from
        // it and writes back the contact directly — no `surface` round-trip. Borrow
        // `kin` + the 18 ancillary clusters as ONE `BodyClustersMut` view, the exact
        // aggregate the player builds.
        let mut clusters = self.clusters_mut();
        let events = ae::update_body_with_tuning_clusters(world, &mut clusters, input, dt, tuning);
        drop(clusters);
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

    fn step_surface_walker(
        &mut self,
        world: &ae::World,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
        gravity_dir: ae::Vec2,
    ) {
        if !self.ground.on_ground {
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
                self.ground.on_ground = true;
                return;
            }
            self.surface.surface_normal = n;
        }

        let original_pos = self.kin.pos;
        self.kin.pos += tangent * step_len;
        self.kin.vel = tangent * speed;

        if self.snap_pos_to_surface(world) {
            self.ground.on_ground = true;
            return;
        }

        let new_normal = tangent;
        let around_corner = original_pos + tangent * body_long + (-n) * body_long;
        self.kin.pos = around_corner;
        self.surface.surface_normal = new_normal;
        if self.snap_pos_to_surface(world) {
            self.kin.vel = ae::Vec2::ZERO;
            self.ground.on_ground = true;
            return;
        }

        self.kin.pos = original_pos;
        self.surface.surface_normal = -tangent;
        if self.snap_pos_to_surface(world) {
            self.kin.vel = ae::Vec2::ZERO;
            self.ground.on_ground = true;
            return;
        }

        self.surface.surface_normal = n;
        self.kin.pos = original_pos;
        self.ground.on_ground = false;
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
            on_ground: self.ground.on_ground,
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
        self.ground.on_ground = body.on_ground;
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

    pub fn reset_to_spawn(&mut self) {
        // Restore the authored spatial baseline. `tuning` / `brain_spec`
        // are projected once at spawn and never mutate at runtime (no
        // entity morphs its archetype in place), so they already hold the
        // baseline — there is nothing to re-project here.
        self.kin.size = self.config.spawn.size;
        self.kin.pos = self.config.spawn.pos;
        self.kin.vel = ae::Vec2::ZERO;
        // Fresh full-HP body → `alive()` is true; no separate liveness flag.
        *self.health = ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(
            self.config.tuning.max_health,
        ));
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
            }),
            ignored_targets: Vec::new(),
        })
    }
}

#[cfg(test)]
mod dash_tests {
    //! S3d: dash as a body-enforced capability. These drive the REAL grounded
    //! integration (`ActorMut::update` → the shared spine), so they prove the
    //! body owns the burst — a possessing human and an AI brain dash identically
    //! because both only set `dash_pressed` (invariants I2/I3).
    use super::*;
    use crate::features::ecs::actor_clusters::{ActorBody, ActorClusterSeed};
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::actor::CharacterBrain;

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
            CharacterBrain::Custom("cellular_automaton_fighter".into()),
            &[],
        );
        // Rest the body on the floor top (y = 100): center a half-height above it.
        let half_h = seed.kin.size.y * 0.5;
        seed.kin.pos = ae::Vec2::new(0.0, 100.0 - half_h);
        seed.kin.vel = ae::Vec2::ZERO;
        seed.kin.facing = 1.0;
        seed.surface.gravity_scale = 1.0;
        seed.caps.can_dash = can_dash;
        // The dash ability lives on the movement body's mask (derived from caps);
        // rebuild it after overriding the cap so the pipeline dash limb matches.
        seed.body = ActorBody::from_caps(&seed.caps, false);
        seed.body.0.ground.on_ground = true;
        let start_x = seed.kin.pos.x;
        let mut em = seed.as_actor_mut();
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
                crate::time::feel::SandboxFeelTuning::default(),
                (0.0, 0.0),
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

    /// B2 (fable review §B2): a non-surface-walker's published reference-frame
    /// normal must track LIVE gravity at its position, not stay pinned to the
    /// spawn constant `(0,-1)`. Consumers derive the body frame as
    /// `-surface_normal` (shield block side, slash knockback, ranged muzzle/aim);
    /// if it stayed screen-down, a body under sideways/inverted gravity would
    /// block/recoil/fire in the down-gravity frame while its movement obeyed the
    /// real field. Regression guard for the `!surface_walker` LIVE write.
    #[test]
    fn a_non_surface_walker_keeps_its_frame_normal_live_under_gravity() {
        let world = floored_world();
        for gravity in [
            ae::Vec2::new(0.0, 1.0),  // down (baseline)
            ae::Vec2::new(1.0, 0.0),  // right
            ae::Vec2::new(0.0, -1.0), // up
            ae::Vec2::new(-1.0, 0.0), // left
        ] {
            let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
            let mut seed = ActorClusterSeed::new(
                "grunt".to_string(),
                "Grunt".to_string(),
                aabb,
                CharacterBrain::Custom("cellular_automaton_fighter".into()),
                &[],
            );
            // A plain (non-clinging) fighter; make the invariant explicit.
            seed.config.tuning.surface_walker = false;
            // Spawn-pinned to screen-down — the exact stale state B2 fixes.
            seed.surface.surface_normal = ae::Vec2::new(0.0, -1.0);
            seed.kin.pos = ae::Vec2::new(0.0, 40.0);
            let mut em = seed.as_actor_mut();
            em.update(
                &world,
                ae::Vec2::new(2000.0, em.kin.pos.y),
                FeatureCombatTuning::default(),
                None,
                1.0 / 60.0,
                false,
                ActorControlFrame::neutral(),
                gravity,
                crate::time::feel::SandboxFeelTuning::default(),
                (0.0, 0.0),
            );
            let expected = -gravity;
            assert!(
                (em.surface.surface_normal - expected).length() < 1e-3,
                "gravity {gravity:?}: the frame normal must track live gravity; \
                 got {:?}, want {expected:?}",
                em.surface.surface_normal
            );
        }
    }

    /// Drive a grounded walker (locomotion full-right) for `ticks` steps under
    /// the given post-hit stagger `(hitstun_timer, recoil_lock_timer)`; return
    /// the ground covered along +x. The §A2 step 7 witness rig.
    fn walk_run_staggered(stagger: (f32, f32), ticks: u32) -> f32 {
        let world = floored_world();
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
        let mut seed = ActorClusterSeed::new(
            "staggered".to_string(),
            "Staggered".to_string(),
            aabb,
            CharacterBrain::Custom("cellular_automaton_fighter".into()),
            &[],
        );
        let half_h = seed.kin.size.y * 0.5;
        seed.kin.pos = ae::Vec2::new(0.0, 100.0 - half_h);
        seed.kin.vel = ae::Vec2::ZERO;
        seed.kin.facing = 1.0;
        seed.surface.gravity_scale = 1.0;
        seed.body.0.ground.on_ground = true;
        let start_x = seed.kin.pos.x;
        let mut em = seed.as_actor_mut();
        let mut frame = ActorControlFrame::neutral();
        frame.locomotion = ae::Vec2::new(1.0, 0.0);
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
                crate::time::feel::SandboxFeelTuning::default(),
                stagger,
            );
        }
        em.kin.pos.x - start_x
    }

    /// §A2 step 7: the post-hit stagger gates an actor's input through the SAME
    /// rule the player's input bridge applies — recoil-lock is a hard zero (no
    /// steering at all), hitstun leaves only reduced movement authority.
    #[test]
    fn a_staggered_body_loses_input_authority_like_the_player() {
        let free = walk_run_staggered((0.0, 0.0), 12);
        let recoil_locked = walk_run_staggered((0.0, 1.0), 12);
        let hitstunned = walk_run_staggered((1.0, 0.0), 12);
        assert!(
            free > 10.0,
            "sanity: an unstaggered walker covers real ground (got {free:.1}px)"
        );
        assert!(
            recoil_locked.abs() < 0.5,
            "a recoil-locked body has NO steering authority (moved {recoil_locked:.1}px)"
        );
        assert!(
            hitstunned < free * 0.8,
            "hitstun reduces movement authority (stunned {hitstunned:.1}px vs free {free:.1}px)"
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
            CharacterBrain::Custom("cellular_automaton_fighter".into()),
            &[],
        );
        let half_h = seed.kin.size.y * 0.5;
        seed.kin.pos = ae::Vec2::new(0.0, 100.0 - half_h);
        seed.surface.gravity_scale = 1.0;
        seed.caps.can_dash = false;
        seed.body = ActorBody::from_caps(&seed.caps, false);
        seed.body.0.ground.on_ground = true;
        let mut em = seed.as_actor_mut();
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
            crate::time::feel::SandboxFeelTuning::default(),
            (0.0, 0.0),
        );
        assert!(
            em.dash.timer <= 0.0,
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
            CharacterBrain::Custom("cellular_automaton_fighter".into()),
            &[],
        );
        seed.kin.pos = ae::Vec2::new(0.0, -200.0);
        seed.kin.vel = ae::Vec2::ZERO;
        seed.surface.gravity_scale = 0.0;
        // Aerial body: fly ability + fly_enabled from spawn.
        seed.body = ActorBody::from_caps(&seed.caps, true);
        let start = seed.kin.pos;
        let mut em = seed.as_actor_mut();
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
                crate::time::feel::SandboxFeelTuning::default(),
                (0.0, 0.0),
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
        assert!(!em.ground.on_ground, "a flying body is never grounded");
    }
}
