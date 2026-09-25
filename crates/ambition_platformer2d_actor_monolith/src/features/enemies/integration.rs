//! Actor physics/AI integration: the per-frame tick that drives actor
//! movement + attack geometry through the [`ActorMut`] ECS view. EVERY actor —
//! grounded, aerial, and the adhesive crawler — runs the one shared movement
//! kernel ([`ActorMutIntegrationExt::integrate_body`] → `ae::step_motion`, borrowing the
//! actor's `kin` + [`ActorBody`] clusters as one `BodyClustersMut` view). The
//! kernel picks the physics by the body's explicit `MotionModel`; the flight
//! limb vs grounded spine split rides `flight.fly_enabled` inside the
//! axis-swept policy. Attack AABBs are derived here; archetype tuning comes
//! from the [`super::CharacterRoster`].

use crate::actor_clusters::ActorMut;
use super::*;
use ambition_combat::events::{
    FeatureCombatTuning, HitEvent, HitKnockback, HitKnockbackMagnitude, HitMode, HitSource,
    HitTarget,
};

/// Minimum knockback strength a body-contact hit imparts on the struck body, even
/// when the archetype authored `contact_strength = 0`. Guarantees a body that
/// overlaps an enemy is pushed back OUT of its box rather than sitting inside it
/// taking a hit every i-frame window. Feel-tunable.
const BODY_CONTACT_MIN_KNOCKBACK: f32 = 0.6;

/// Simulation behavior layered over the spawn crate's mutable actor view.
///
/// `ActorMut` is owned by `ambition_platformer2d_actor_spawn` because that crate
/// owns the ECS cluster/query seam. The per-tick enemy/NPC integration remains
/// feature-simulation policy, so it is expressed as this local extension trait
/// rather than an inherent impl on a foreign type or a dependency back from the
/// spawn capability into the monolith.
pub(crate) trait ActorMutIntegrationExt {
    #[allow(clippy::too_many_arguments)]
    fn update(
        &mut self,
        world: &ae::World,
        tuning: FeatureCombatTuning,
        dt: f32,
        pose_owned_externally: bool,
        frame: ambition_characters::actor::control::ActorControlFrame,
        motion_model: &mut ambition_platformer2d_core::movement::MotionModel,
        motion_frame: ae::MotionFrame,
        playing_a_move: Option<&ambition_combat::moveset::MovePlayback>,
        feel: ambition_combat::feel::Platformer2dFeelTuningMonolith,
        authored_tuning: Option<ae::MovementTuning>,
        combat: &mut ambition_characters::actor::BodyCombat,
        tumbling: bool,
        out_of_play: bool,
        contact_field: ae::BodyContactField<'_>,
    ) -> (
        ambition_characters::actor::control::ActorControlFrame,
        ae::FrameEvents,
    );

    #[allow(clippy::too_many_arguments)]
    fn integrate_body(
        &mut self,
        world: &ae::World,
        frame: &ambition_characters::actor::control::ActorControlFrame,
        motion_model: &mut ambition_platformer2d_core::movement::MotionModel,
        dt: f32,
        motion_frame: ae::MotionFrame,
        playing_a_move: Option<&ambition_combat::moveset::MovePlayback>,
        feel: ambition_combat::feel::Platformer2dFeelTuningMonolith,
        authored_tuning: Option<ae::MovementTuning>,
        combat: &mut ambition_characters::actor::BodyCombat,
        tumbling: bool,
        out_of_play: bool,
        pose_owned_externally: bool,
        contact_field: ae::BodyContactField<'_>,
    ) -> ae::FrameEvents;

    fn aabb(&self) -> ae::Aabb;
    fn bark_anchor(&self) -> ae::Vec2;
    fn body_damage_aabb(&self) -> Option<ae::Aabb>;
    fn contact_attack(&self) -> Option<ContactAttack>;
}

impl<'a> ActorMutIntegrationExt for ActorMut<'a> {
    #[allow(clippy::too_many_arguments)]
    fn update(
        &mut self,
        world: &ae::World,
        tuning: FeatureCombatTuning,
        dt: f32,
        // Something else owns this body's pose. Named for the FACT rather than
        // for the saddle: see `PoseOwnedExternally`. ⛔ This is the RIDER's side
        // of the relationship — "somebody is carrying me" — and it is not the
        // mount's `MountSlot` question, which the charge-crash guard asks.
        pose_owned_externally: bool,
        frame: ambition_characters::actor::control::ActorControlFrame,
        motion_model: &mut ambition_platformer2d_core::movement::MotionModel,
        // The body's current acceleration/reference frame, resolved ONCE by the
        // environment (the driver) for this body tick. Input projection, the
        // active policy, and every frame-relative limb consume this same value.
        motion_frame: ae::MotionFrame,
        // The move PLAYING on this body, for the helpless derivation. ⛔⛔ THIS
        // WAS HARDCODED `false` with a comment claiming no actor authors a
        // recovery — and that stopped being true the day Smash's CPU fighters
        // took the common moveset resolver, which spends
        // `BodyJumpState::recovery_charges` for ANY body. A human was helpless
        // and a CPU was not, on the same rule.
        playing_a_move: Option<&ambition_combat::moveset::MovePlayback>,
        // Post-hit stagger (§A2 step 7): the body's own `BodyCombat`, applied to
        // the FINAL InputState by the SAME gate the player's input bridge uses.
        feel: ambition_combat::feel::Platformer2dFeelTuningMonolith,
        // The body's OWN feel, when its character authored one.
        //
        // without this the line below overwrote the axis params from the
        // SHARED dev tuning every tick, so a seated fighter's authored feel was
        // granted by seating and discarded by movement — the asymmetry the
        // grant site's own comment says it exists to prevent.
        authored_tuning: Option<ae::MovementTuning>,
        combat: &mut ambition_characters::actor::BodyCombat,
        // Is this body TUMBLING? Read from the PUBLISHED projection
        // (`BodyMotionFacts::tumbling`) by the driver, which holds it; the
        // maneuver state behind it is model-private (ADR 0024). The post-hit
        // gate needs it so a falling body's tech press is not deleted before
        // the kernel can read it.
        tumbling: bool,
        // Is this body's death window open (ADR 0033)?
        out_of_play: bool,
        // Inert for every body whose composition never granted the capability, which is every
        // body outside a smash match.
        contact_field: ae::BodyContactField<'_>,
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
                let spawn = self.spawn.pos;
                ae::reset_body_clusters(
                    motion_model,
                    &mut self.clusters_mut(),
                    spawn,
                    // An IN-PLACE revive moves the body nowhere, so nothing
                    // about it turned around. ⚠ The authored `SpawnFacing` is
                    // not available to restore instead: `SpawnBaseline` records
                    // pos, size and gravity scale and no facing, so a revive
                    // could not return one even if it wanted to. Measured
                    // 2026-09-21 over `mary_o.ldtk`: 30 authored entities face
                    // Left and 0 of them author a respawn interval, so no
                    // shipped content reaches this arm with a facing to lose —
                    // which is why the baseline is not being widened (and its
                    // rollback schema bumped) for it today.
                    ae::ResetFacing::Keep,
                    ae::DEFAULT_TUNING.air_jumps,
                );
            }
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


        // ONE integration arm for every actor: the kernel dispatches on the
        // body's explicit MotionModel (axis-swept, surface momentum, or the
        // adhesive crawler — the former hidden surface-walker path).
        let move_events = self.integrate_body(
            world,
            &frame,
            motion_model,
            dt,
            motion_frame,
            playing_a_move,
            feel,
            authored_tuning,
            combat,
            tumbling,
            out_of_play,
            // ⭐⭐ AND THIS IS WHERE `_is_mounted` FINALLY MEANS SOMETHING. The
            // parameter sat unused with a leading underscore, so a rider in a
            // saddle ran the whole movement pass and the saddle constraint
            // repaired the result afterwards — two authorities, and a snap
            // cannot undo a spent double-jump.
            //
            // ⛔ IT READS THE MARKER, NOT THE MOUNT. `PoseOwnedExternally` says
            // "somebody else owns this pose" without saying who, which is what
            // lets a lift or a grab reach the same road later without this
            // crate learning what a saddle is.
            pose_owned_externally,
            contact_field,
        );

        if frame.facing.abs() > 0.001 {
            self.kin.facing = frame.facing.signum();
        }

        // Facing is committed only from the control frame. Collision publishes
        // semantic contacts through body state; autonomous brains may turn on a
        // later tick, while human/fighter controllers retain their chosen facing.

        (frame, move_events)
    }

    /// Integration through the shared movement kernel
    /// (`ae::step_motion`) — the unification's core seam, for EVERY actor body.
    /// The actor's `kin` supplies the kinematics; its persistent [`ActorBody`]
    /// supplies the ancillary movement clusters. The brain's `ActorControlFrame`
    /// becomes the body's typed `InputState`, so an actor runs / jumps /
    /// coyote-grace-jumps / dashes / flies / crawls and collides through the
    /// EXACT code the human player uses — no parallel enemy integrator.
    ///
    /// Grounded bodies map `locomotion → run` + `jump_pressed → buffered jump`.
    /// Flying bodies (`flight.fly_enabled`) are steered by the brain's exact
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
        motion_model: &mut ambition_platformer2d_core::movement::MotionModel,
        dt: f32,
        motion_frame: ae::MotionFrame,
        // Threaded to `update`'s helpless derivation. See its parameter.
        playing_a_move: Option<&ambition_combat::moveset::MovePlayback>,
        feel: ambition_combat::feel::Platformer2dFeelTuningMonolith,
        // The body's OWN feel, when its character authored one.
        //
        // without this the line below overwrote the axis params from the
        // SHARED dev tuning every tick, so a seated fighter's authored feel was
        // granted by seating and discarded by movement — the asymmetry the
        // grant site's own comment says it exists to prevent.
        authored_tuning: Option<ae::MovementTuning>,
        combat: &mut ambition_characters::actor::BodyCombat,
        // See `update`'s own parameter: the published tumble fact, for the
        // tech exemption in the post-hit gate.
        tumbling: bool,
        // See `update`'s own parameter, and the call site below.
        out_of_play: bool,
        // Something else owns this body's pose — see `step_body`'s own note.
        pose_owned_externally: bool,
        contact_field: ae::BodyContactField<'_>,
    ) -> ae::FrameEvents {
        let flying = self.flight.fly_enabled;
        let mut tuning = self
            .config
            .tuning
            .movement
            .body_tuning(self.config.tuning.max_run_speed);
        // Flight tuning from the driver's chase speed: the body flies at its own
        // speed, steers responsively (matching the old floating accel), and does
        // NOT idle-bob like the player (hover speed 0) — an AI flyer holds station.
        let flight_speed = self.config.tuning.flight_speed(&self.policy.0);
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
        ambition_combat::attack_support::apply_post_hit_input_gates(
            &mut input,
            feel,
            combat,
            self.shield,
            tumbling,
            // THE SAME RULE THE HUMAN ROAD ASKS. An actor with no recovery never
            // satisfies it, so ordinary enemies are unaffected by construction —
            // which is what the hardcoded `false` was trying to say and got
            // wrong the moment a CPU fighter authored one.
            ambition_combat::moveset::body_is_helpless(
                self.jump,
                self.ground.on_ground,
                playing_a_move,
            ),
        );
        // What stays here is what legitimately differs: WHICH tuning this body moves under (its
        // character's authored feel, else its config's).
        let resolved_tuning = authored_tuning.unwrap_or(tuning);
        // A crawler's params are refreshed here as `step_body` refreshes the
        // axis params: its pace is its DRIVER's, asked of the live policy like
        // the flight speed above, so a policy a provocation installs paces the
        // body on its next step; its fall is the body's resolved tuning.
        if let ae::movement::MotionModel::AdhesiveCrawler(crawler) = motion_model {
            crawler.params = ae::CrawlerParams {
                crawl_speed: self.config.tuning.crawl_speed(&self.policy.0),
                max_fall_speed: resolved_tuning.max_fall_speed,
            };
        }
        let mut clusters = self.clusters_mut();
        let result = ambition_characters::actor::step_body(
            motion_model,
            &mut clusters,
            combat,
            resolved_tuning,
            // ⛔⛔ THIS WAS `false`, "a fact rather than an exemption": the
            // reasoning was that `open_death_interlude` queries
            // `With<PlayerEntity>`, so `OutOfPlay` could only ever reach a
            // participant's body — and an enemy dies by despawning or by its own
            // encounter rules, not by this window. It was true when it was
            // written and it stopped being true the day the stocks respawn beat
            // began opening a window of its own (D201): a Smash fighter is
            // integrated HERE, not on the player road, and it is not a
            // `PlayerEntity`. The measured symptom was the whole beat doing
            // nothing — the body coasted on the velocity that launched it and
            // answered the jump button while it waited to come back.
            //
            // ⭐ A COMMENT STATING A RULE IS A SPECIFICATION, and the second
            // opener of a state is what tests it. This is now READ rather than
            // asserted, so the next opener costs nothing. (The same function
            // already carries the same lesson about `playing_a_move`.)
            out_of_play,
            // ⭐ THE FACT, READ RATHER THAN INFERRED. `PoseOwnedExternally` is
            // stamped by `mount::board` and lives in `_core` precisely so the
            // domains that hold a body and the domains that read one need not
            // know about each other. This is its first consumer.
            pose_owned_externally,
            ae::MotionStepContext {
                world,
                input,
                frame: motion_frame,
                facing_intent: frame.facing,
                dt,
                contact: contact_field,
                pose_owned_externally: false,
                // ⭐ THE SAME PLAYBACK THE HELPLESS DERIVATION READS, asked a
                // second question. A recovery spent on the floor must not be
                // handed back by the grounded refresh while the move that spent
                // it is still running — see
                // `MotionStepContext::recovery_commitment_outstanding`.
                recovery_commitment_outstanding:
                    ambition_combat::moveset::recovery_commitment_outstanding(playing_a_move),
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

    fn aabb(&self) -> ae::Aabb {
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

    // `rotation_rad()` WAS HERE and nothing ever asked for it
    // . A body's presented
    // rotation is derived by the RENDER family from the same surface normal;
    // this was a second way to compute it, on the sim read-model, with no
    // reader — the shape `reference_a_comment_describes_intent` warns about,
    // where two derivations of one fact drift apart because only one is used.

    fn bark_anchor(&self) -> ae::Vec2 {
        self.kin.pos + ae::Vec2::new(0.0, -self.kin.size.y * 0.72 - 16.0)
    }

    // `begin_melee_attack` is deleted. A body's melee swing is a moveset
    // `"attack"` move: the brain's `melee_pressed` edge starts it via
    // `combat::moveset::trigger_moveset_moves` and `advance_move_playback` spawns
    // the active-window strike — one melee lifecycle for every body, paced by the
    // move's own duration rather than a separate recovery cooldown.

    fn body_damage_aabb(&self) -> Option<ae::Aabb> {
        if !self.config.tuning.body_contact_damage {
            return None;
        }
        Some(self.aabb())
    }

    /// Snapshot this actor's live body-contact attack (its damage box + the
    /// tuning/frame facts the victim pass needs), taken while the attacker's
    /// clusters are borrowed. The victim resolution runs AFTER the borrow ends
    /// (fable review §A4: contact damage targets any body, so the
    /// victim query aliases the attacker query and the two passes must split).
    fn contact_attack(&self) -> Option<ContactAttack> {
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
}

/// An actor's live body-contact attack, snapshotted by [`ActorMutIntegrationExt::contact_attack`]
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
                // An ordinary hit: it stuns.
                reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
                dir,
                magnitude: HitKnockbackMagnitude::FeelScale(self.strength),
                source_pos: self.source_pos,
                impact_pos: impact,
                launch_dir: None,
                follow: None,
            }),
            ignored_targets: Vec::new(),
                    attacker_move_instance: None,
        })
    }
}

#[cfg(test)]
pub(crate) trait SeedActorIntegrationTestExt:
    crate::actor_clusters::SeedActorMut
{
    /// One integration tick over a pre-spawn seed, with optional runtime inputs
    /// defaulted exactly as the historical actor-movement scratch harness did.
    #[allow(clippy::too_many_arguments)]
    fn update_for_test(
        &mut self,
        world: &ae::World,
        tuning: FeatureCombatTuning,
        dt: f32,
        pose_owned_externally: bool,
        frame: ambition_characters::actor::control::ActorControlFrame,
        motion_model: &mut ambition_platformer2d_core::movement::MotionModel,
        motion_frame: ae::MotionFrame,
    ) -> ambition_characters::actor::control::ActorControlFrame {
        self.as_actor_mut()
            .update(
                world,
                tuning,
                dt,
                pose_owned_externally,
                frame,
                motion_model,
                motion_frame,
                // No move playing on a scratch rig, so it is never helpless.
                None,
                ambition_combat::feel::Platformer2dFeelTuningMonolith::default(),
                None,
                &mut ambition_characters::actor::BodyCombat::default(),
                // Not tumbling — a scratch harness body is not in a floor game.
                false,
                // In play — a scratch rig has no death window open.
                false,
                // A single-body rig: nobody to be solid to.
                ae::BodyContactField::NONE,
            )
            .0
    }
}

#[cfg(test)]
impl SeedActorIntegrationTestExt for ambition_body_seed::ActorClusterSeed {}

#[cfg(test)]
mod dash_tests;
#[cfg(test)]
mod hitlag_tests;
#[cfg(test)]
mod lunge_tests;
