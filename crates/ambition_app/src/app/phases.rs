#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::player_tick::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use ambition_gameplay_core::schedule::*;

/// How a ledge-grabbing player should react to the moving platform that carries
/// them this frame: ride along with it, or be knocked off because the carry
/// would shove them into a wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LedgePlatformCarry {
    Carry,
    KnockOff,
}

/// Decide [`LedgePlatformCarry`] for a ledge-grabbing player about to be carried
/// by `delta`. `world` is the base collision world, which **excludes** the
/// moving platform (it's composited in separately), so a solid overlap here is a
/// genuine *other* wall — meaning the platform would push the player through it
/// (#126 "ledge grab on a moving platform into a wall pushes you through").
/// Pure, so the wall decision is unit-testable without the full phase context.
pub(super) fn ledge_platform_carry(
    world: &ae::World,
    player_aabb: ae::Aabb,
    delta: ae::Vec2,
) -> LedgePlatformCarry {
    use ambition_gameplay_core::engine_core::AabbExt;
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

/// Phase 4 — control-clock half of the two-clock player update.
///
/// Owns: hitstun-filtered control snapshot, real-time `frame_dt`
/// `update_player_control_with_clusters` call, pogo-bounce → feature-event
/// routing, `handle_player_events` for the control-clock pass.
///
/// Should not own: gravity/platform/AI ticks (those run on `sim_dt` in
/// `player_simulation_phase`). New responsive-input mechanics that need
/// real time (jump buffers, blink aim, dash chains) belong here. Returns
/// `Return` if the engine asked for a sandbox reset.
pub(super) fn player_control_phase(
    // Vestigial since the engine `pogo_hits` path was removed (orb damage now
    // flows from the sandbox attack pogo). Kept on the signature for now.
    _player_entity: bevy::prelude::Entity,
    actor_control: ambition_gameplay_core::actor::control::ActorControlFrame,
    world: &ae::World,
    clusters: &mut ae::PlayerClustersMut<'_>,
    sim_state: &mut ambition_gameplay_core::SandboxSimState,
    clock: &mut ambition_gameplay_core::time::clock_state::ClockState,
    safety: &mut ambition_gameplay_core::player::PlayerSafetyState,
    moving_platforms: &[ambition_gameplay_core::world::platforms::MovingPlatformState],
    attack: &mut Option<ambition_gameplay_core::PlayerAttackState>,
    sfx_writer: &mut MessageWriter<SfxMessage>,
    vfx_writer: &mut MessageWriter<VfxMessage>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    reset_room_features: &mut MessageWriter<features::ResetRoomFeaturesEvent>,
    _hit_events: &mut MessageWriter<features::HitEvent>,
    anim: &mut ambition_gameplay_core::player::PlayerAnimState,
    combat: &mut ambition_gameplay_core::player::PlayerCombatState,
    interaction: &mut ambition_gameplay_core::player::PlayerInteractionState,
    blink_cam: &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
    // True only for the camera/HUD-owning primary player. A brain-driven clone (or
    // any other player-bodied entity) runs the SAME per-entity movement core but must
    // NOT trigger the world-global sandbox reset — that is the primary's concern.
    is_primary: bool,
) -> PhaseOutcome {
    // Two-clock update:
    // - control_dt is real time for responsive inputs and precision-blink aim;
    // - sim_dt is scaled game time for gravity, platforms, enemies, particles.
    //
    // Per the actor/brain migration, `ActorControl` is the single
    // source of truth for player input. The player brain translates
    // every `ControlFrame` verb the simulation needs (movement, jump,
    // dash, attack, interact, shield, pogo, blink, fly_toggle,
    // fast_fall, projectile-charge, aim) so this phase never reads
    // raw input. Hitstun gate applies inside the helper.
    let input =
        engine_input_from_actor_control(actor_control, feel, combat.hitstun_timer, frame_dt);
    let control_world =
        features::world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);
    let control_events =
        ae::update_player_control_with_clusters(&control_world, clusters, input, frame_dt, tuning);
    if control_events.reset && is_primary {
        reset_sandbox(
            world,
            sfx_writer,
            vfx_writer,
            clusters,
            sim_state,
            clock,
            safety,
            attack,
            anim,
            combat,
            interaction,
            blink_cam,
            tuning,
            feel,
        );
        reset_room_features.write(features::ResetRoomFeaturesEvent {
            reason: features::RoomResetReason::PlayerDeath,
        });
        return PhaseOutcome::Return;
    }
    // (Breakable pogo-orb damage now flows solely from the sandbox attack pogo
    // (`advance_attack` → `PogoBounce` HitEvent); the engine `pogo_hits` path was
    // a redundant duplicate and was removed.)
    handle_player_events(
        sfx_writer,
        vfx_writer,
        clusters,
        combat,
        blink_cam,
        anim,
        control_events,
        None,
    );
    PhaseOutcome::Continue
}

/// Phase 5 — sim-clock half of the two-clock player update.
///
/// Owns: scaled `sim_dt`, moving-platform tick + ride-along,
/// sandbox-side solid rebuild, `update_player_simulation_with_clusters`,
/// landing-dust feedback through `handle_player_events`.
///
/// Should not own: feature-runtime ticks or interact-buffering. New
/// game-time-affected motion (gravity tweaks, platform AI, knockback
/// resolution) belongs here. Returns `Return` if simulation asked for a
/// sandbox reset.
///
/// Time-scale authority moved out of this phase in ADR 0010 step 4
/// — see `ambition_gameplay_core::time::time_control::{emit_player_time_intent_system,
/// apply_clock_scale_requests, smooth_sim_clock_toward_target_system}`.
/// This phase observes the smoothed `sim_state.time_scale` set by
/// the PlayerInput pipeline.
pub(super) fn player_simulation_phase(
    actor_control: ambition_gameplay_core::actor::control::ActorControlFrame,
    world: &ae::World,
    clusters: &mut ae::PlayerClustersMut<'_>,
    dev_state: &ambition_gameplay_core::SandboxDevState,
    sim_state: &mut ambition_gameplay_core::SandboxSimState,
    clock: &mut ambition_gameplay_core::time::clock_state::ClockState,
    safety: &mut ambition_gameplay_core::player::PlayerSafetyState,
    moving_platforms: &[ambition_gameplay_core::world::platforms::MovingPlatformState],
    attack: &mut Option<ambition_gameplay_core::PlayerAttackState>,
    sfx_writer: &mut MessageWriter<SfxMessage>,
    vfx_writer: &mut MessageWriter<VfxMessage>,
    shake: &mut ambition_gameplay_core::time::camera_ease::CameraShakeState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    reset_room_features: &mut MessageWriter<features::ResetRoomFeaturesEvent>,
    anim: &mut ambition_gameplay_core::player::PlayerAnimState,
    combat: &mut ambition_gameplay_core::player::PlayerCombatState,
    interaction: &mut ambition_gameplay_core::player::PlayerInteractionState,
    blink_cam: &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
    // True only for the camera/HUD-owning primary. Non-primary player bodies (the
    // clone) run the same per-entity sim but must not shake the camera or reset the
    // world — those are primary-only.
    is_primary: bool,
) -> PhaseOutcome {
    let input =
        engine_input_from_actor_control(actor_control, feel, combat.hitstop_timer, frame_dt);

    // sim_state.time_scale was set this frame by the time-control
    // pipeline in SandboxSet::PlayerInput (emit → apply → smooth).
    // The local `dev_state` reference + `feel` parameter are kept so
    // tests + tuning hooks still compile, even though the smoothing
    // is no longer driven from here.
    let _ = dev_state; // intentional: the dev slowmo intent is consumed by the time-control pipeline.
    let sim_dt = sandbox_dt(combat.hitstop_timer, clock.time_scale, frame_dt);

    let player_aabb_pre = clusters.kinematics.aabb();
    let player_size_pre = clusters.kinematics.size;
    let on_ground_pre = clusters.ground.on_ground;
    let active_ledge_platform = clusters.ledge.grab.and_then(|grab| {
        moving_platforms.iter().position(|platform| {
            platform.matches_ledge_contact_in_frame(
                grab.contact,
                player_size_pre,
                tuning.gravity_dir,
            )
        })
    });
    // Standing-on-platform RIDING is no longer here — it is EMERGENT in the movement
    // sweep (`integrate_velocity_clusters` carries any grounded body by the supporting
    // solid's velocity, the same rule `step_kinematic` applies to enemies), so the
    // player rides like every other body, with no player-specific ride code. What
    // stays is the LEDGE-platform carry: hanging off a moving platform's edge is
    // player-specific (only the player ledge-grabs) AND the body isn't grounded
    // then, so the sweep carry can't apply.
    let _ = on_ground_pre;
    if let Some(platform_delta) =
        active_ledge_platform.map(|idx| moving_platforms[idx].last_delta())
    {
        // Ledge grabs can latch to the temporary moving-platform collision block.
        match ledge_platform_carry(world, player_aabb_pre, platform_delta) {
            // #126: the platform is about to carry the hanging player INTO a wall.
            // Don't ride into it (that clips through) — knock off the ledge and fall.
            LedgePlatformCarry::KnockOff => {
                clusters.ledge.knock_off_on_hit();
            }
            // Carry both the player and the stored ledge contact so hang / climb /
            // roll interpolation stays platform-relative after the platform moves.
            LedgePlatformCarry::Carry => {
                clusters.kinematics.pos += platform_delta;
                if let Some(grab) = clusters.ledge.grab.as_mut() {
                    grab.contact.anchor += platform_delta;
                    grab.contact.climb_target += platform_delta;
                }
            }
        }
    }
    let collision_world =
        features::world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);

    let was_grounded = clusters.ground.on_ground;
    let pre_sim_vy = clusters.kinematics.vel.y;
    let sim_events = ae::update_player_simulation_with_clusters(
        &collision_world,
        clusters,
        input,
        sim_dt,
        tuning,
    );
    // Hard-fall screen shake: pure trigger function in
    // `time::camera_ease`. Avoids tiny hops, saturates above
    // terminal velocity via the `kick()` cap.
    let shake_amplitude = ambition_gameplay_core::time::camera_ease::hard_fall_shake_amplitude(
        was_grounded,
        clusters.ground.on_ground,
        pre_sim_vy,
    );
    if is_primary && shake_amplitude > 0.0 {
        shake.kick(shake_amplitude);
        sfx_writer.write(SfxMessage::Play {
            id: ambition_sfx::ids::PLAYER_LAND,
            pos: clusters.kinematics.pos,
        });
    }
    if sim_events.reset && is_primary {
        reset_sandbox(
            world,
            sfx_writer,
            vfx_writer,
            clusters,
            sim_state,
            clock,
            safety,
            attack,
            anim,
            combat,
            interaction,
            blink_cam,
            tuning,
            feel,
        );
        reset_room_features.write(features::ResetRoomFeaturesEvent {
            reason: features::RoomResetReason::PlayerDeath,
        });
        return PhaseOutcome::Return;
    }
    handle_player_events(
        sfx_writer,
        vfx_writer,
        clusters,
        combat,
        blink_cam,
        anim,
        sim_events,
        Some(was_grounded),
    );
    PhaseOutcome::Continue
}

#[cfg(test)]
mod ledge_carry_tests {
    use super::{ledge_platform_carry, LedgePlatformCarry};
    use ambition_gameplay_core::engine_core as ae;

    fn world_with_right_wall() -> ae::World {
        // A solid wall occupying x[100,120], full height; open space to its left.
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

    // A ledge-grabbing player hugging the left of the wall (right edge at x=92).
    fn player() -> ae::Aabb {
        ae::Aabb::new(ae::Vec2::new(80.0, 50.0), ae::Vec2::new(12.0, 20.0))
    }

    #[test]
    fn carry_into_a_wall_knocks_the_player_off() {
        // A rightward platform delta would push the player's right edge (92) past
        // the wall face (100) and into it — #126: knock off, don't clip through.
        assert_eq!(
            ledge_platform_carry(&world_with_right_wall(), player(), ae::Vec2::new(30.0, 0.0)),
            LedgePlatformCarry::KnockOff,
        );
    }

    #[test]
    fn carry_away_from_walls_rides_normally() {
        // Leftward (away) — and a small rightward nudge that stays clear — both
        // ride along with the platform.
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
