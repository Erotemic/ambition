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
    profile: &crate::features::ecs::actor_tuning::BrainProfile,
    attack: &crate::features::BodyMelee,
    alive: bool,
) -> ambition_characters::actor::ai::CharacterAiOutput {
    let recover_remaining =
        if attack.on_cooldown() && !attack.is_winding_up() && !attack.is_active() {
            attack.cooldown.min(0.30)
        } else {
            0.0
        };
    // ⛔⛔ **A `Passive => 0.0` ARM STOOD HERE, AND IT WAS A SECOND ANSWER TO A
    // QUESTION THE PROFILE ALREADY OWNS** (campaign P2.20, 2026-08-13).
    //
    // How far a body notices from is `BrainProfile::aggro_radius`. The arm made
    // the integrator-facing `CharacterBrain` read-model a co-authority over the
    // same fact — and that read-model is a SILHOUETTE, written as `Passive` for
    // anything that is not a patrol brain, including a boss whose real mind is a
    // `BossPattern`. Two authorities over one number is what this campaign
    // exists to end.
    //
    // ⚠ **inert by measurement, not by argument**: every production site that
    // writes `Passive` writes `BrainProfile::default()` beside it, and that
    // default's `aggro_radius` is `0.0` — the peaceful NPC seed
    // (`actor_clusters`), the boss config (`spawn_actors`) and the reconcile
    // projection all do. So the arm was restating what the profile already said.
    //
    // ⚠⚠ **AND THIS WHOLE FUNCTION IS A READ-MODEL, which is worth stating
    // because the first version of this note got the consequence wrong.** Its
    // output goes to `ActorStatus::ai_mode` and nowhere else, and `ai_mode`'s only
    // readers are `ActorIntent` — whose own doc says it exists "so rendering and
    // HUD systems can branch on actor state" — and the rollback snapshot.
    // `is_dangerous()` has no gameplay caller. What a body actually chases is
    // decided by its BRAIN from the same `BrainProfile`, not here.
    //
    // ⇒ so the arm did not make a provoked body fail to notice anybody; it made
    // the HUD say `Idle` about a body its brain was chasing with. That is still a
    // defect and still the same defect — two authorities over one number, and the
    // presentation copy free to disagree with the policy — but it is a
    // presentation bug, not a combat one.
    //
    // ⭐ **and that is the good news for P2.20**: the read-model provocation
    // writes (`CharacterBrain::Custom("combatant")`) is not load-bearing for
    // gameplay either, so replacing it costs no creature its behaviour.
    //
    // ⚠ `Guard` stays, and is not the same shape: `leash_radius` is a PLACEMENT
    // fact — this guard, at this post — not a property of the policy driving it.
    let effective_aggro_radius = match brain {
        ambition_entity_catalog::placements::CharacterBrain::Guard { leash_radius } => {
            *leash_radius
        }
        _ => profile.aggro_radius,
    };
    ambition_characters::actor::ai::evaluate_character_ai_output(
        ambition_characters::actor::ai::CharacterAiSnapshot {
            actor_pos: pos,
            player_pos: target_pos,
            aggro_radius: effective_aggro_radius,
            attack_range: profile.attack_range,
            attack_windup_remaining: attack.windup_remaining(),
            attack_active_remaining: attack.active_remaining(),
            attack_recover_remaining: recover_remaining,
            stun_remaining: 0.0,
            alive,
            // **Does this driver wander when it has nothing to chase?** The
            // field's own doc names the fact — "has a path or a NON-ZERO PATROL
            // SPEED" — and `BrainProfile::patrol_effort` is that speed, as a
            // fraction of the body's top speed (§4.7).
            //
            // ⛔ it read `!matches!(brain, Passive)` until 2026-08-13: the same
            // silhouette co-authority the aggro radius above just lost, one flag
            // over. A body whose policy authors a real patrol effort read as
            // `Idle` because its integrator read-model said `Passive`, which is
            // every peaceful NPC in the Hall — they wander, and the HUD said they
            // were standing still.
            //
            // ⚠ **safe to change because the mode is a READ-MODEL** (see the
            // block above): no gameplay branches on it, so this corrects what the
            // presentation layer reports rather than what any creature does.
            // `is_sandbag` stays and is not the same shape — a practice target
            // holds still because of what its BODY is, not what its policy wants.
            patrol_enabled: !tuning.is_sandbag && profile.patrol_effort > 0.0,
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
        feel: crate::time::feel::Platformer2dFeelTuningMonolith,
        // **The body's OWN feel, when its character authored one.**
        //
        // ⚠ without this the line below overwrote the axis params from the
        // SHARED dev tuning every tick, so a seated fighter's authored feel was
        // granted by seating and discarded by movement — the asymmetry the
        // grant site's own comment says it exists to prevent.
        authored_tuning: Option<ae::MovementTuning>,
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
                // A revive is a RESTART. Same reasoning as the respawn below:
                // `transit_body` keeps maneuver state on purpose, which is right
                // for a blink and wrong for coming back from the dead, and it
                // does not announce `ae::BodyRestarted` to any provider.
                let spawn = self.config.spawn.pos;
                ae::reset_body_clusters(
                    motion_model,
                    &mut self.clusters_mut(),
                    spawn,
                    ae::DEFAULT_TUNING.air_jumps,
                );
            }
            self.status.ai_mode = ambition_characters::actor::ai::CharacterAiMode::Dead;
            return (
                ambition_characters::actor::control::ActorControlFrame::neutral(),
                ae::FrameEvents::default(),
            );
        }

        // Melee is NOT advanced here. A body's swing is a moveset `"attack"` move
        // (Combat set): `advance_move_playback` ticks it on the owner's proper time
        // and spawns the active-window strike, so movement integration owns movement
        // only. The AI reads `self.attack` (the projected `BodyMelee` read-model) as
        // of the previous frame's advance — a consistent one-frame view.
        let _ = tuning.enemy_attack_active;

        let ai = evaluate_enemy_ai_output(
            self.kin.pos,
            target_pos,
            &self.config.brain,
            &self.config.tuning,
            &self.config.brain_profile,
            self.attack,
            self.health.alive(),
        );
        self.status.ai_mode = ai.mode;

        // ONE integration arm for every actor: the kernel dispatches on the
        // body's explicit MotionModel (axis-swept, surface momentum, or the
        // adhesive crawler — the former hidden surface-walker path).
        let move_events = self.integrate_body(
            world,
            &frame,
            motion_model,
            dt,
            motion_frame,
            feel,
            authored_tuning,
            stagger,
        );

        // Face the brain's committed direction whenever it commits one. Hostile
        // chasers AND peaceful patrollers/flyers both set `frame.facing`; a
        // standstill/idle brain leaves it ~0 so facing is preserved. (Previously
        // gated on `attacks_player`, which left peaceful actors facing-frozen.)
        if frame.facing.abs() > 0.001 {
            self.kin.facing = frame.facing.signum();
        }

        // Facing is committed only from the control frame. Collision publishes
        // semantic contacts through body state; autonomous brains may turn on a
        // later tick, while human/fighter controllers retain their chosen facing.

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
        frame: &ambition_characters::actor::control::ActorControlFrame,
        motion_model: &mut crate::features::MotionModel,
        dt: f32,
        motion_frame: ae::MotionFrame,
        feel: crate::time::feel::Platformer2dFeelTuningMonolith,
        // **The body's OWN feel, when its character authored one.**
        //
        // ⚠ without this the line below overwrote the axis params from the
        // SHARED dev tuning every tick, so a seated fighter's authored feel was
        // granted by seating and discarded by movement — the asymmetry the
        // grant site's own comment says it exists to prevent.
        authored_tuning: Option<ae::MovementTuning>,
        stagger: (f32, f32),
    ) -> ae::FrameEvents {
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
            let local_target = motion_frame.to_local(vt.vec());
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
            axis.params = authored_tuning
                .map(|authored| authored.axis_swept_params())
                .unwrap_or_else(|| tuning.axis_swept_params());
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
        let mut events = result.events;
        // Two actor policies applied on the ONE ground/jump authority: a flying body
        // is never grounded (the collision sweep can still find support under a
        // hovering flyer), and a grounded body refreshes its air jumps each tick
        // (more forgiving than the player's jump-only refresh — an actor tuning).
        if flying {
            self.ground.on_ground = false;
            // Overriding the support FACT means overriding its TRANSITION too:
            // with the flag forced false, the kernel's sweep re-finds support
            // under a floor-skimming flyer every tick and reports a fresh
            // `Landed` — one land SFX + dust puff per tick (the boss-room
            // "insane sfx" torrent: 'player.land' at 38-63 plays/s from the
            // clockwork warden). A flying body emits no ground transitions.
            events.ground_contact = ae::GroundContactTransition::Unchanged;
        }
        if self.ground.on_ground {
            self.jump.air_jumps_available = MAX_ENEMY_AIR_JUMPS;
        }

        if let Some(motion) = &mut self.motion.0 {
            let _ = motion.advance(self.kin.pos, dt);
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

    // ⛔ **`rotation_rad()` WAS HERE and nothing ever asked for it**
    // (compiler-verified across the workspace, 2026-08-12). A body's presented
    // rotation is derived by the RENDER family from the same surface normal;
    // this was a second way to compute it, on the sim read-model, with no
    // reader — the shape `reference_a_comment_describes_intent` warns about,
    // where two derivations of one fact drift apart because only one is used.

    pub fn bark_anchor(&self) -> ae::Vec2 {
        self.kin.pos + ae::Vec2::new(0.0, -self.kin.size.y * 0.72 - 16.0)
    }

    pub fn attack_aabb(&self) -> ae::Aabb {
        self.attack_aabb_dir(ae::Vec2::new(self.kin.facing, 0.0))
    }

    // ⛔⛔ **`attack_telegraph_aabb()` WAS HERE, AND IT WAS WORSE THAN DEAD.** It
    // returned `self.attack_aabb()` verbatim — a differently-NAMED accessor for
    // the identical box. A reader reaching for a "telegraph" box is looking for
    // the windup's warning volume, which is normally LARGER and earlier than the
    // hitbox; this would have handed them the hitbox and looked right. No caller
    // ever did, which is the only reason it never mattered.

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

    // `begin_melee_attack` is deleted. A body's melee swing is a moveset
    // `"attack"` move: the brain's `melee_pressed` edge starts it via
    // `combat::moveset::trigger_moveset_moves` and `advance_move_playback` spawns
    // the active-window strike — one melee lifecycle for every body, paced by the
    // move's own duration rather than a separate recovery cooldown.

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

    /// Restore this actor to its authored spawn state.
    ///
    /// **Liveness is decided by the actor's own [`RespawnPolicy`], not by the
    /// reset.** A room reset is a room-scoped return, so it revives a dead actor
    /// only when its policy says a room-scoped return is what it does
    /// (`OnRoomReenter`, or `InPlace` which revives on its own timer anyway).
    /// A `DeadStaysDead` / `OnRest` corpse stays dead and only has its spatial
    /// baseline restored.
    ///
    /// Before this gate existed, reset full-healed EVERY actor unconditionally.
    /// `sync_ecs_actors_with_save` (Progression) re-zeroed the HP a moment later,
    /// so the end-of-frame state looked right — but the actor was ALIVE for the
    /// remainder of that frame: drawable, targetable, and able to act. Worse, it
    /// meant "who decides a dead actor comes back" had two answers, and the reset
    /// path was the one that never consulted the policy the kill hook and
    /// save-sync both read.
    pub fn reset_to_spawn(&mut self, motion_model: &mut crate::features::MotionModel) {
        // Restore the authored spatial baseline. `tuning` / `brain_profile`
        // are projected once at spawn and never mutate at runtime (no
        // entity morphs its archetype in place), so they already hold the
        // baseline — there is nothing to re-project here.
        let was_dead = !self.health.alive();
        let revives_on_room_reset = matches!(
            self.config.tuning.respawn,
            ambition_entity_catalog::placements::RespawnPolicy::OnRoomReenter
                | ambition_entity_catalog::placements::RespawnPolicy::InPlace(_)
        );
        let stays_dead = was_dead && !revives_on_room_reset;
        // A respawn is a RESTART, not a transit.
        //
        // ⚠ this was `transit_body` under the comment "respawn is a discrete
        // transit (ADR 0024 authority)". Right about the POSE — a body arriving
        // somewhere must reconcile departure contacts and attachment — and
        // silent about everything else: `transit_body` documents that maneuver
        // state (coyote, buffers, dash timers) is deliberately KEPT, which is
        // true of a blink and false of coming back from the dead. It also does
        // not raise `restart_pending`, so `ae::BodyRestarted` never fired for an
        // enemy respawn and no provider heard about it.
        //
        // `reset_body_clusters` transits internally, so the ADR 0024 property
        // that comment was protecting is not lost by saying the stronger thing.
        let spawn = self.config.spawn.pos;
        ae::reset_body_clusters(
            motion_model,
            &mut self.clusters_mut(),
            spawn,
            ae::DEFAULT_TUNING.air_jumps,
        );
        // Fresh full-HP body → `alive()` is true; no separate liveness flag.
        // Skipped entirely for a corpse whose policy forbids a room-scoped
        // return, so it is never briefly alive (see the doc comment).
        if !stays_dead {
            *self.health = ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(self.config.tuning.max_health),
            );
        }
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
    ) -> Option<HitEvent> {
        if !self.volume.strict_intersects(target_body) {
            return None;
        }
        let impact = midpoint(target_body.center(), self.volume.center());
        let dir =
            ((target_body.center() - self.source_pos).dot(self.frame_side)).signum_or(self.facing);
        Some(HitEvent {
            strike_sfx: None,
            volume: self.volume.into(),
            damage: self.damage,
            source: HitSource::Contact,
            attacker: Some(attacker),
            target: HitTarget::Body(target),
            mode: HitMode::Knockback,
            knockback: Some(HitKnockback {
                dir,
                magnitude: HitKnockbackMagnitude::FeelScale(self.strength),
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
#[cfg(test)]
mod respawn_policy_tests;

#[cfg(test)]
mod aggro_authority_tests {
    use super::evaluate_enemy_ai_output;
    use ambition_characters::actor::ai::CharacterAiMode;
    use ambition_entity_catalog::placements::CharacterBrain;

    fn look(brain: CharacterBrain, aggro_radius: f32) -> CharacterAiMode {
        let tuning = crate::features::ecs::actor_tuning::ActorTuning::default();
        let profile = crate::features::ecs::actor_tuning::BrainProfile {
            aggro_radius,
            attack_range: 8.0,
            ..Default::default()
        };
        evaluate_enemy_ai_output(
            ambition_platformer2d_core::Vec2::new(0.0, 0.0),
            // Well inside a 200px notice radius and well outside an 8px reach, so
            // the answer is Chase or it is not noticing at all.
            ambition_platformer2d_core::Vec2::new(100.0, 0.0),
            &brain,
            &tuning,
            &profile,
            &crate::features::BodyMelee::default(),
            true,
        )
        .mode
    }

    /// **HOW FAR A BODY NOTICES FROM IS ITS PROFILE'S, AND ONLY ITS PROFILE'S.**
    ///
    /// ⛔ the first two rows are the deleted `Passive => 0.0` arm's whole
    /// subject: a body whose read-model says `Passive` now notices exactly what
    /// its POLICY says it notices, which for every production body that carries
    /// that read-model is still nothing — `BrainProfile::default()` authors
    /// `0.0` and is what the peaceful seed, the boss config and the reconcile
    /// projection all pair it with.
    ///
    /// ⭐ the third row is why this matters to P2.20: a hostile policy is heard
    /// through a `Passive` read-model. That is what lets provocation stop writing
    /// `CharacterBrain::Custom("combatant")` to be noticed — the archetype name
    /// was standing in for "this body is hostile now".
    #[test]
    fn the_notice_radius_comes_from_the_policy_not_from_the_read_model() {
        // ⚠ **`!= Chase`, not a named idle mode.** What the body does INSTEAD of
        // chasing is `patrol_enabled`'s answer, and that flag is still read off
        // the read-model — a second co-authority, and the next step of this row.
        // Asserting `Patrol` here would quietly pin the coupling this test is
        // about removing.
        assert_ne!(
            look(CharacterBrain::Passive, 0.0),
            CharacterAiMode::Chase,
            "a body whose policy authors no notice radius chased anyway"
        );
        assert_ne!(
            look(CharacterBrain::Custom("anything".into()), 0.0),
            CharacterAiMode::Chase,
            "the read-model, not the policy, decided this body notices — a \
             non-`Passive` silhouette is being read as hostility again"
        );
        assert_eq!(
            look(CharacterBrain::Passive, 200.0),
            CharacterAiMode::Chase,
            "a body carrying a hostile policy reads as not-chasing because its \
             integrator read-model still says `Passive`, so the HUD disagrees \
             with the brain about what this creature is doing"
        );
    }

    /// `Guard` is NOT the same shape and does not follow the policy: its
    /// `leash_radius` is a placement fact — this guard, at this post — so it
    /// overrides. The poison is the same profile read through a different brain.
    #[test]
    fn a_guards_leash_is_the_placements_answer_and_still_overrides() {
        assert_ne!(
            look(CharacterBrain::Guard { leash_radius: 0.0 }, 200.0),
            CharacterAiMode::Chase,
            "a guard posted with a zero leash chased on its policy's radius, so \
             the placement's answer is no longer overriding"
        );
        assert_eq!(
            look(
                CharacterBrain::Guard {
                    leash_radius: 200.0
                },
                0.0
            ),
            CharacterAiMode::Chase,
            "a guard with a real leash did not notice, so the override is gone \
             in the other direction too"
        );
    }
}
