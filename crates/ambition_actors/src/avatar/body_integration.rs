//! Home/player body movement, decomposed so it joins the SAME scheduled body
//! integration phase as actors.
//!
//! The home body is NOT a separate gameplay species: [`integrate_home_body`] is
//! the per-body movement core the unified `integrate_sim_bodies` phase calls for
//! every `PlayerEntity`, right beside the actor bodies it integrates in the same
//! system. It runs the LITERAL same engine entry an actor uses
//! (`ae::update_body_with_tuning_clusters`) over the body's `BodyClustersMut`
//! view. The ONLY home-specific work here is:
//!
//! - the pre-sim ledge-platform carry ([`ledge_platform_carry`]) — only the home
//!   body ledge-grabs;
//! - the two-clock precision-blink affordance carried by `InputState::control_dt`
//!   (an INPUT affordance, not a simulation structure);
//! - flagging a body reset ([`PlayerBodyFrameOutput::reset`]) for the separate
//!   home reset POLICY and PRESENTATION phases to consume.
//!
//! It performs NO sandbox reset, NO room reset, and NO presentation — those are
//! home-policy / home-view phases that read the [`PlayerBodyFrameOutput`] hand-off
//! this phase writes.

use bevy::prelude::*;

use ambition_engine_core as ae;

use crate::features::ecs::attack::engine_input_from_actor_control;
use crate::features::FeatureEcsWorldOverlay;
use crate::time::feel::SandboxFeelTuning;
use crate::world::platforms::MovingPlatformState;
use ambition_characters::actor::BodyCombat;
use ambition_world::collision::world_with_sandbox_solids;

/// Movement→(reset/presentation) hand-off for a home/player body, written by the
/// unified body integration phase (`integrate_sim_bodies` → [`integrate_home_body`])
/// and read by the two home-policy phases: the home reset POLICY (sandbox reset on
/// `reset`) and the home PRESENTATION phase (screen shake / landing SFX / per-op
/// anim/SFX/VFX). Body-generic in SHAPE — it carries only integration facts (this
/// frame's `FrameEvents` + the landing inputs + a reset flag), never any player
/// presentation state — so movement stays a pure integrate-and-report phase.
/// A required component of every player body.
#[derive(Component, Default)]
pub struct PlayerBodyFrameOutput {
    /// The movement tick's events (jump/dash/blink ops, blink endpoints, …).
    pub events: ae::FrameEvents,
    /// Grounded state ENTERING the movement tick (for the hard-fall shake edge).
    pub was_grounded: bool,
    /// Fall speed entering the tick — the velocity component ALONG gravity
    /// (hard-fall shake magnitude; frame-agnostic, fable review 2026-07-02 §B
    /// minor: the raw `vel.y` form misfired under sideways gravity).
    pub pre_sim_fall_speed: f32,
    /// The integration flagged a body reset this frame (drown / hazard /
    /// out-of-bounds / death). The body was already teleported to spawn by this
    /// phase; the home reset POLICY consumes this to run the full sandbox reset for
    /// the primary, and the PRESENTATION phase skips the frame.
    pub reset: bool,
}

/// How a ledge-grabbing player should react to the moving platform that carries
/// them this frame: ride along with it, or be knocked off because the carry
/// would shove them into a wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgePlatformCarry {
    Carry,
    KnockOff,
}

/// Decide [`LedgePlatformCarry`] for a ledge-grabbing player about to be carried
/// by `delta`. `world` is the base collision world, which **excludes** the
/// moving platform (it's composited in separately), so a solid overlap here is a
/// genuine *other* wall — meaning the platform would push the player through it
/// (#126 "ledge grab on a moving platform into a wall pushes you through").
/// Pure, so the wall decision is unit-testable without the full phase context.
pub fn ledge_platform_carry(
    world: &ae::World,
    player_aabb: ae::Aabb,
    delta: ae::Vec2,
) -> LedgePlatformCarry {
    use ambition_engine_core::AabbExt;
    let carried = player_aabb.translated(delta);
    let into_wall = world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, ae::BlockKind::Solid) && carried.strict_intersects(b.aabb));
    if into_wall {
        LedgePlatformCarry::KnockOff
    } else {
        LedgePlatformCarry::Carry
    }
}

/// The per-body home movement core — control phase **and** simulation phase in ONE
/// combined engine call, `ae::update_body_with_tuning_clusters`: the LITERAL same
/// engine entry a brain-driven actor uses (`ActorMut::integrate_body`). Called by
/// the unified `integrate_sim_bodies` phase for every `PlayerEntity`, so the home
/// body and every actor integrate through one function inside one scheduled system.
///
/// THE TWO-CLOCK SPLIT IS AN INPUT AFFORDANCE, NOT A SIMULATION STRUCTURE.
/// Precision-blink bullet-time keeps the player's aim responsive while the world
/// slows. It is carried entirely by `InputState::control_dt`: the human sets
/// `control_dt = real frame_dt` (so the engine runs the control phase at real time
/// and the simulation phase at `sim_dt`), while a brain leaves `control_dt = 0`.
///
/// `ActorControl` is the single source of truth for input — the brain translates
/// every verb the simulation consumes. The hitstun gate applies inside
/// `engine_input_from_actor_control`.
///
/// On a flagged reset (drown / hazard / out-of-bounds / death) the body teleports
/// to spawn (engine-level body reset, the same on every body) and `frame_out.reset`
/// is set. The SANDBOX reset + ROOM reset are HOME POLICY, run by a separate phase
/// that reads this flag — this function never performs them.
#[allow(clippy::too_many_arguments)]
pub fn integrate_home_body(
    actor_control: ambition_characters::actor::control::ActorControlFrame,
    world: &ae::World,
    clusters: &mut ae::BodyClustersMut<'_>,
    combat: &BodyCombat,
    hurtbox: &mut ae::CenteredAabb,
    frame_out: &mut PlayerBodyFrameOutput,
    moving_platforms: &[MovingPlatformState],
    // The body's motion IDENTITY (demo plan AJ11/Q16 — "Sanic is BOTH"):
    // `None`/`AxisSwept` = the axis-swept path below, byte-identical;
    // `SurfaceMomentum` dispatches the HOME body to the surface-follower
    // solver — the same policy branch `integrate_actor_body` carries, so a
    // worn momentum character rides whether it is the start character or a
    // possessed actor.
    motion_model: Option<&mut crate::features::MotionModel>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    scaled_dt: f32,
    feature_ecs_overlay: &FeatureEcsWorldOverlay,
) {
    // ONE input frame. `control_dt = frame_dt` (real time) IS the precision-blink
    // affordance: the combined entry below runs the control phase at this rate and
    // the simulation phase at the scaled `sim_dt`. The hitstun gate applies inside
    // the helper.
    let input = engine_input_from_actor_control(
        actor_control,
        feel,
        combat.hitstun_timer,
        combat.recoil_lock_timer,
        frame_dt,
    );
    // Per-body sim dt: frozen during this body's hitstop, otherwise the scaled
    // gameplay dt (bullet-time / pause already folded into `scaled_dt`).
    let sim_dt = if combat.hitstop_timer > 0.0 {
        0.0
    } else {
        scaled_dt
    };

    // ── SurfaceMomentum dispatch (Q16): the home body's movement identity ──
    // Uses the GATED input (hitstun/recoil authority-reduction stays uniform
    // with every other body) and skips the AABB-path machinery below (ledge
    // carry, jump buffer, dash/blink — capabilities absent on a momentum
    // body v1).
    if let Some(crate::features::MotionModel::SurfaceMomentum(m)) = motion_model {
        integrate_home_momentum(
            input,
            actor_control.facing,
            world,
            clusters,
            hurtbox,
            frame_out,
            moving_platforms,
            m,
            tuning,
            sim_dt,
            feature_ecs_overlay,
        );
        return;
    }

    // Pre-sim LEDGE-platform carry. Platforms are advanced once (by
    // `advance_moving_platforms`) ahead of this whole phase, so we read this frame's
    // delta. Standing-on-platform RIDING is EMERGENT in the movement sweep (the same
    // rule enemies ride by), so there is no player-specific ride code. What stays
    // home-specific is the LEDGE carry: hanging off a moving platform's edge (only
    // the home body ledge-grabs) leaves the body un-grounded, so the sweep carry
    // can't apply.
    let player_aabb_pre = clusters.kinematics.aabb();
    let player_size_pre = clusters.kinematics.size;
    let active_ledge_platform = clusters.ledge.grab.and_then(|grab| {
        moving_platforms.iter().position(|platform| {
            platform.matches_ledge_contact_in_frame(
                grab.contact,
                player_size_pre,
                tuning.gravity_dir,
            )
        })
    });
    if let Some(platform_delta) =
        active_ledge_platform.map(|idx| moving_platforms[idx].last_delta())
    {
        match ledge_platform_carry(world, player_aabb_pre, platform_delta) {
            // #126: the platform is about to carry the hanging player INTO a wall.
            LedgePlatformCarry::KnockOff => {
                clusters.ledge.knock_off_on_hit();
            }
            LedgePlatformCarry::Carry => {
                clusters.kinematics.pos += platform_delta;
                if let Some(grab) = clusters.ledge.grab.as_mut() {
                    grab.contact.anchor += platform_delta;
                    grab.contact.climb_target += platform_delta;
                }
            }
        }
    }

    let collision_world = world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);
    let was_grounded = clusters.ground.on_ground;
    let pre_sim_fall_speed = clusters.kinematics.vel.dot(tuning.gravity_dir);

    // THE single combined body tick: control phase (at `input.control_dt`) then
    // simulation phase (at `sim_dt`). The EXACT engine entry an actor body uses.
    let events =
        ae::update_body_with_tuning_clusters(&collision_world, clusters, input, sim_dt, tuning);
    // Engine-level body reset (teleport to spawn) — the same reset every body does
    // on a hazard flag; NOT the sandbox/room reset (that is home policy, elsewhere).
    if events.reset {
        ae::reset_body_clusters(clusters, world.spawn);
    }

    *frame_out = PlayerBodyFrameOutput {
        was_grounded,
        pre_sim_fall_speed,
        reset: events.reset,
        events,
    };

    // Publish the body's combat footprint ORIENTED to its gravity frame — the
    // IDENTICAL single-source-of-truth publish every actor performs in
    // `integrate_actor_body` (§A6). Every hurtbox consumer (enemy hitboxes,
    // hazards, boss volumes, contact damage, enemy projectiles) reads THIS
    // component instead of rebuilding the box per-site.
    use ambition_engine_core::AabbExt;
    let body = crate::features::collision_aabb(&crate::features::SimpleActorGeometry {
        pos: clusters.kinematics.pos,
        size: clusters.kinematics.size,
        facing: clusters.kinematics.facing,
        frame_down: tuning.gravity_dir,
    });
    hurtbox.center = body.center();
    hurtbox.half_size = body.half_size();
}

/// The home body's surface-momentum frame (the Q16 branch): drive the R9.1
/// pure core over the SAME composited collision view, then apply the SAME
/// hazard/out-of-bounds gate the engine sim phase applies to axis-swept
/// bodies — Sanic dies in pits. Publishes the hurtbox oriented to the ridden
/// surface (`frame_down = -surface_normal`, gravity when airborne — the §B2
/// rule); sprite tilt-on-slope is a presentation follow-up (BLIND).
#[allow(clippy::too_many_arguments)]
fn integrate_home_momentum(
    input: ae::InputState,
    facing_intent: f32,
    world: &ae::World,
    clusters: &mut ae::BodyClustersMut<'_>,
    hurtbox: &mut ae::CenteredAabb,
    frame_out: &mut PlayerBodyFrameOutput,
    moving_platforms: &[MovingPlatformState],
    m: &mut crate::features::MomentumMotion,
    tuning: ae::MovementTuning,
    sim_dt: f32,
    feature_ecs_overlay: &FeatureEcsWorldOverlay,
) {
    use ambition_engine_core::AabbExt;

    let collision_world = world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);
    let was_grounded = clusters.ground.on_ground;
    let pre_sim_fall_speed = clusters.kinematics.vel.dot(tuning.gravity_dir);

    let mut on_ground = clusters.ground.on_ground;
    // Recomputed per step by the momentum core (ride contact, else gravity);
    // the home body has no persistent `ActorSurfaceState` — the frame publish
    // below is this step's truth.
    let mut surface_normal = -tuning.gravity_dir;
    let mut events = ae::FrameEvents::default();
    // §3.1 sample capture around the momentum step (this path skips the
    // shared pipeline's kernel write). The hazard respawn below overwrites
    // it with a zero-length record via `reset_body_clusters` — a respawn is
    // a teleport, never path.
    let sweep_entry = (clusters.kinematics.pos, clusters.kinematics.vel);
    events.contacts = crate::features::step_momentum_body(
        clusters.kinematics,
        &mut on_ground,
        &mut surface_normal,
        m,
        &collision_world,
        tuning.gravity_dir * tuning.gravity,
        input.axis_x,
        input.jump_pressed,
        facing_intent,
        sim_dt,
    );
    if let Some(sweep) = clusters.sweep.as_deref_mut() {
        *sweep = ae::SweepSample {
            prev: sweep_entry.0,
            curr: clusters.kinematics.pos,
            vel: sweep_entry.1,
            half: clusters.kinematics.size * 0.5,
        };
    }
    clusters.ground.on_ground = on_ground;

    // Hazard / out-of-bounds parity with the axis-swept sim phase: the SAME
    // hazard predicate over the SAME composited view + the gravity-relative
    // "fell 200px past the world AABB" rule. On trigger: engine-level body
    // reset to spawn, and the follower state returns to Airborne (never
    // respawn "riding" a chain the body is no longer on).
    let pos = clusters.kinematics.pos;
    let clamped = ae::Vec2::new(
        pos.x.clamp(0.0, world.size.x),
        pos.y.clamp(0.0, world.size.y),
    );
    let fell_out = (pos - clamped).dot(tuning.gravity_dir) > 200.0;
    if ae::movement::touching_hazard_aabb(&collision_world, clusters.kinematics.aabb()) || fell_out
    {
        events.hazard = true;
        events.reset = true;
        ae::reset_body_clusters(clusters, world.spawn);
        m.state = ambition_engine_core::surface::SurfaceMotion::Airborne;
    }

    *frame_out = PlayerBodyFrameOutput {
        was_grounded,
        pre_sim_fall_speed,
        reset: events.reset,
        events,
    };

    let body = crate::features::collision_aabb(&crate::features::SimpleActorGeometry {
        pos: clusters.kinematics.pos,
        size: clusters.kinematics.size,
        facing: clusters.kinematics.facing,
        frame_down: -surface_normal,
    });
    hurtbox.center = body.center();
    hurtbox.half_size = body.half_size();
}

/// Advance the world's moving platforms ONCE per frame, ahead of every body
/// integration (home + actors), so every body rides this frame's platform
/// positions. Peeled out of the per-entity body loop so it can't multiply. Uses
/// the PRIMARY player's hitstop so platforms freeze during the player's hitstop.
pub fn advance_moving_platforms(
    world_time: Res<ambition_time::WorldTime>,
    mut platforms: ResMut<ambition_world::collision::MovingPlatformSet>,
    primary_combat: Query<&BodyCombat, crate::actor::PrimaryPlayerOnly>,
) {
    let Ok(combat) = primary_combat.single() else {
        return;
    };
    let sim_dt = if combat.hitstop_timer > 0.0 {
        0.0
    } else {
        world_time.scaled_dt
    };
    for platform in platforms.0.iter_mut() {
        platform.update(sim_dt);
    }
}

#[cfg(test)]
mod home_momentum_tests {
    use super::*;
    use crate::features::{MomentumMotion, MotionModel};
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_engine_core as ae;

    const DT: f32 = 1.0 / 60.0;

    fn chain_world() -> ae::World {
        ae::World::new(
            "home-momentum",
            ae::Vec2::new(3000.0, 1200.0),
            ae::Vec2::new(200.0, 500.0),
            Vec::new(),
        )
        .with_chains(vec![ae::SurfaceChain::open(
            "floor",
            vec![ae::Vec2::new(0.0, 600.0), ae::Vec2::new(1500.0, 600.0)],
        )])
    }

    struct Rig {
        scratch: ae::BodyClusterScratch,
        model: MotionModel,
        hurtbox: ae::CenteredAabb,
        frame_out: PlayerBodyFrameOutput,
        world: ae::World,
        overlay: FeatureEcsWorldOverlay,
    }

    fn rig(world: ae::World) -> Rig {
        Rig {
            scratch: crate::avatar::primary_player_scratch(
                world.spawn,
                ae::AbilitySet::sandbox_all(),
            ),
            model: MotionModel::SurfaceMomentum(MomentumMotion::new(
                ae::surface::MomentumParams::default(),
            )),
            hurtbox: ae::CenteredAabb::new(world.spawn, ae::Vec2::splat(10.0)),
            frame_out: PlayerBodyFrameOutput::default(),
            world,
            overlay: FeatureEcsWorldOverlay::default(),
        }
    }

    fn step(r: &mut Rig, frame: ActorControlFrame) {
        let mut clusters = r.scratch.as_mut();
        integrate_home_body(
            frame,
            &r.world,
            &mut clusters,
            &BodyCombat::default(),
            &mut r.hurtbox,
            &mut r.frame_out,
            &[],
            Some(&mut r.model),
            ae::DEFAULT_TUNING,
            SandboxFeelTuning::default(),
            DT,
            DT,
            &r.overlay,
        );
    }

    #[test]
    fn worn_momentum_home_body_rides_runs_and_jumps() {
        let mut r = rig(chain_world());
        // Fall onto the chain, then run right.
        let mut run = ActorControlFrame::neutral();
        run.locomotion.x = 1.0;
        run.facing = 1.0;
        // Sample mid-run: kept running, the body (correctly) launches off the
        // chain's open end around x=1500 and falls out — not this test's
        // subject.
        let mut mid_run = false;
        for _ in 0..240 {
            step(&mut r, run);
            if r.scratch.ground.on_ground && r.scratch.kinematics.pos.x > 500.0 {
                mid_run = true;
                break;
            }
        }
        assert!(mid_run, "rode the chain and advanced past x=500");
        // The hurtbox publish followed the body.
        assert!((r.hurtbox.center - r.scratch.kinematics.pos).length() < 40.0);
        // The frame reports ride contacts (the contact vocabulary reaches the
        // home body's FrameEvents).
        assert!(
            r.frame_out.events.contacts.iter().any(|c| matches!(
                c.source,
                ae::collision_semantics::ContactSource::Chain { .. }
            )),
            "ride contact published"
        );
        // Jump: the GATED input path maps jump_pressed through.
        let mut jump = run;
        jump.jump_pressed = true;
        step(&mut r, jump);
        assert!(!r.scratch.ground.on_ground, "left the surface");
        assert!(
            r.scratch.kinematics.vel.y < -400.0,
            "launched along +normal: {:?}",
            r.scratch.kinematics.vel
        );
    }

    #[test]
    fn momentum_home_body_rides_ordinary_block_floors() {
        // THE Sanic-in-a-normal-room regression (Jon, 2026-07-05): every
        // sandbox room floors with AABB `Block`s, not authored chains. A
        // worn momentum body must land, run, and jump on plain solids —
        // blocks are surfaces (`Block::boundary_chain`), not just obstacles.
        let world = ae::World::new(
            "home-momentum-blocks",
            ae::Vec2::new(3000.0, 1200.0),
            ae::Vec2::new(200.0, 500.0),
            vec![ae::world::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 600.0),
                ae::Vec2::new(2800.0, 100.0),
            )],
        );
        let mut r = rig(world);
        let mut run = ActorControlFrame::neutral();
        run.locomotion.x = 1.0;
        run.facing = 1.0;
        let mut mid_run = false;
        for _ in 0..240 {
            step(&mut r, run);
            if r.scratch.ground.on_ground && r.scratch.kinematics.pos.x > 500.0 {
                mid_run = true;
                break;
            }
        }
        assert!(mid_run, "rode the block floor and advanced past x=500");
        assert!(
            r.frame_out.events.contacts.iter().any(|c| matches!(
                c.source,
                ae::collision_semantics::ContactSource::Block { .. }
            )),
            "block ride contact published"
        );
        let mut jump = run;
        jump.jump_pressed = true;
        step(&mut r, jump);
        assert!(!r.scratch.ground.on_ground, "left the floor");
        assert!(
            r.scratch.kinematics.vel.y < -400.0,
            "jumped off a block floor: {:?}",
            r.scratch.kinematics.vel
        );
    }

    #[test]
    fn momentum_home_body_dies_in_pits_and_respawns_airborne() {
        // The chain ends mid-world; running off it drops the body past the
        // world bottom — the Q16 hazard/OOB parity gate must fire.
        let mut r = rig(chain_world());
        let mut run = ActorControlFrame::neutral();
        run.locomotion.x = 1.0;
        let mut saw_reset = false;
        for _ in 0..1800 {
            step(&mut r, run);
            if r.frame_out.reset {
                saw_reset = true;
                break;
            }
        }
        assert!(saw_reset, "fell out and the reset flagged");
        assert_eq!(
            r.scratch.kinematics.pos, r.world.spawn,
            "engine-level body reset to spawn"
        );
        assert!(
            matches!(
                r.model,
                MotionModel::SurfaceMomentum(MomentumMotion {
                    state: ae::surface::SurfaceMotion::Airborne,
                    ..
                })
            ),
            "respawns airborne, never 'riding' a chain it left"
        );
    }
}

#[cfg(test)]
mod ledge_carry_tests {
    use super::{ledge_platform_carry, LedgePlatformCarry};
    use ambition_engine_core as ae;

    fn world_with_right_wall() -> ae::World {
        ae::World::new(
            "ledge_carry_test",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::new(20.0, 50.0),
            vec![ae::Block::solid(
                "wall",
                ae::Vec2::new(100.0, 0.0),
                ae::Vec2::new(20.0, 400.0),
            )],
        )
    }

    fn player() -> ae::Aabb {
        ae::Aabb::new(ae::Vec2::new(80.0, 50.0), ae::Vec2::new(12.0, 20.0))
    }

    #[test]
    fn carry_into_a_wall_knocks_the_player_off() {
        assert_eq!(
            ledge_platform_carry(&world_with_right_wall(), player(), ae::Vec2::new(30.0, 0.0)),
            LedgePlatformCarry::KnockOff,
        );
    }

    #[test]
    fn carry_away_from_walls_rides_normally() {
        let world = world_with_right_wall();
        assert_eq!(
            ledge_platform_carry(&world, player(), ae::Vec2::new(-30.0, 0.0)),
            LedgePlatformCarry::Carry,
        );
        assert_eq!(
            ledge_platform_carry(&world, player(), ae::Vec2::new(5.0, 0.0)),
            LedgePlatformCarry::Carry,
        );
    }
}
