#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::input_systems::*;
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

/// Phase 4 — control-clock half of the two-clock player update.
///
/// Owns: hitstun-filtered control snapshot, real-time `frame_dt`
/// `update_player_control_with_tuning` call, pogo-bounce → feature-event
/// routing, `handle_player_events` for the control-clock pass.
///
/// Should not own: gravity/platform/AI ticks (those run on `sim_dt` in
/// `player_simulation_phase`). New responsive-input mechanics that need
/// real time (jump buffers, blink aim, dash chains) belong here. Returns
/// `Return` if the engine asked for a sandbox reset.
pub(super) fn player_control_phase(
    actor_control: ae::ActorControlFrame,
    world: &ae::World,
    player: &mut ae::Player,
    sim_state: &mut crate::SandboxSimState,
    safety: &mut crate::player::PlayerSafetyState,
    moving_platforms: &[crate::world::platforms::MovingPlatformState],
    attack: &mut Option<crate::PlayerAttackState>,
    sfx_writer: &mut MessageWriter<SfxMessage>,
    vfx_writer: &mut MessageWriter<VfxMessage>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    reset_room_features: &mut MessageWriter<features::ResetRoomFeaturesEvent>,
    pogo_bounces: &mut MessageWriter<features::PogoBounceEvent>,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
    interaction: &mut crate::player::PlayerInteractionState,
    blink_cam: &mut crate::player::PlayerBlinkCameraState,
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
        ae::update_player_control_with_tuning(&control_world, player, input, frame_dt, tuning);
    if control_events.reset {
        reset_sandbox(
            world,
            sfx_writer,
            vfx_writer,
            player,
            sim_state,
            safety,
            attack,
            anim,
            combat,
            interaction,
            blink_cam,
            tuning,
            feel,
        );
        reset_room_features.write(features::ResetRoomFeaturesEvent);
        return PhaseOutcome::Return;
    }
    // Damage breakable pogo orbs the player just bounced off. The
    // engine reports orb AABBs; the sandbox matches them against
    // breakables flagged `pogo_refresh` and routes hit/break events
    // through the standard feature pipeline.
    for &orb_aabb in &control_events.pogo_hits {
        pogo_bounces.write(features::PogoBounceEvent::new(orb_aabb, 1));
    }
    handle_player_events(
        sfx_writer,
        vfx_writer,
        player,
        combat,
        blink_cam,
        control_events,
        None,
    );
    PhaseOutcome::Continue
}

/// Phase 5 — sim-clock half of the two-clock player update.
///
/// Owns: scaled `sim_dt`, moving-platform tick + ride-along,
/// sandbox-side solid rebuild, `update_player_simulation_with_tuning`,
/// landing-dust feedback through `handle_player_events`.
///
/// Should not own: feature-runtime ticks or interact-buffering. New
/// game-time-affected motion (gravity tweaks, platform AI, knockback
/// resolution) belongs here. Returns `Return` if simulation asked for a
/// sandbox reset.
///
/// Time-scale authority moved out of this phase in ADR 0010 step 4
/// — see `crate::time::time_control::{emit_player_time_intent_system,
/// apply_clock_scale_requests, smooth_sim_clock_toward_target_system}`.
/// This phase observes the smoothed `sim_state.time_scale` set by
/// the PlayerInput pipeline.
pub(super) fn player_simulation_phase(
    actor_control: ae::ActorControlFrame,
    world: &ae::World,
    player: &mut ae::Player,
    dev_state: &crate::SandboxDevState,
    sim_state: &mut crate::SandboxSimState,
    safety: &mut crate::player::PlayerSafetyState,
    moving_platforms: &mut [crate::world::platforms::MovingPlatformState],
    attack: &mut Option<crate::PlayerAttackState>,
    sfx_writer: &mut MessageWriter<SfxMessage>,
    vfx_writer: &mut MessageWriter<VfxMessage>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    reset_room_features: &mut MessageWriter<features::ResetRoomFeaturesEvent>,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
    interaction: &mut crate::player::PlayerInteractionState,
    blink_cam: &mut crate::player::PlayerBlinkCameraState,
    ride: &mut crate::player::PlayerPlatformRideState,
) -> PhaseOutcome {
    let input =
        engine_input_from_actor_control(actor_control, feel, combat.hitstop_timer, frame_dt);

    // sim_state.time_scale was set this frame by the time-control
    // pipeline in SandboxSet::PlayerInput (emit → apply → smooth).
    // The local `dev_state` reference + `feel` parameter are kept so
    // tests + tuning hooks still compile, even though the smoothing
    // is no longer driven from here.
    let _ = dev_state; // intentional: the dev slowmo intent is consumed by the time-control pipeline.
    let sim_dt = sandbox_dt(combat.hitstop_timer, sim_state.time_scale, frame_dt);

    let mut riding_platform = None;
    for (index, platform) in moving_platforms.iter_mut().enumerate() {
        let delta = platform.update(sim_dt);
        if riding_platform.is_none() && platform.is_riding(player) {
            riding_platform = Some((index, delta, platform.pos, platform.direction()));
        }
    }
    let riding_now = riding_platform.is_some();
    if riding_now != ride.was_riding {
        // Diagnostic: log riding-state transitions. Useful for chasing the
        // "intermittent glitchy platform behavior" repro (TODO S). With
        // multiple authored platforms, the first current rider is reported.
        if let Some((platform_index, _, platform_pos, platform_dir)) = riding_platform {
            debug!(
                target: "ambition::platform",
                riding = true,
                platform_index,
                player_pos = ?player.pos,
                player_vel = ?player.vel,
                on_ground = player.on_ground,
                platform_pos = ?platform_pos,
                platform_dir,
                "moving-platform riding transition"
            );
        } else {
            debug!(
                target: "ambition::platform",
                riding = false,
                player_pos = ?player.pos,
                player_vel = ?player.vel,
                on_ground = player.on_ground,
                "moving-platform riding transition"
            );
        }
    }
    ride.was_riding = riding_now;
    if let Some((_, platform_delta, _, _)) = riding_platform {
        player.pos += platform_delta;
    }
    let collision_world =
        features::world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);

    let was_grounded = player.on_ground;
    let sim_events =
        ae::update_player_simulation_with_tuning(&collision_world, player, input, sim_dt, tuning);
    if sim_events.reset {
        reset_sandbox(
            world,
            sfx_writer,
            vfx_writer,
            player,
            sim_state,
            safety,
            attack,
            anim,
            combat,
            interaction,
            blink_cam,
            tuning,
            feel,
        );
        reset_room_features.write(features::ResetRoomFeaturesEvent);
        return PhaseOutcome::Return;
    }
    handle_player_events(
        sfx_writer,
        vfx_writer,
        player,
        combat,
        blink_cam,
        sim_events,
        Some(was_grounded),
    );
    PhaseOutcome::Continue
}
