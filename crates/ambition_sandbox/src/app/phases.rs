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
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::update::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;

/// Phase 3 — explicit reset input.
///
/// Owns: routing the `reset_pressed` button through `reset_sandbox`. New
/// "the player asked for a reset / restart" branches belong here; engine
/// or feature-driven resets stay in `player_control_phase`,
/// `player_simulation_phase`, or `damage_heal_dialogue_phase`.
pub(super) fn reset_phase(
    controls: &ControlFrame,
    world: &ae::World,
    player: &mut ae::Player,
    sim_state: &mut crate::SandboxSimState,
    attack: &mut Option<crate::PlayerAttackState>,
    feedback: &mut FrameFeedback,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    reset_room_features: &mut MessageWriter<features::ResetRoomFeaturesEvent>,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
    interaction: &mut crate::player::PlayerInteractionState,
    blink_cam: &mut crate::player::PlayerBlinkCameraState,
) -> PhaseOutcome {
    if controls.reset_pressed {
        reset_sandbox(
            world,
            &mut feedback.sfx,
            &mut feedback.vfx,
            player,
            sim_state,
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
    PhaseOutcome::Continue
}

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
    controls: ControlFrame,
    world: &ae::World,
    player: &mut ae::Player,
    sim_state: &mut crate::SandboxSimState,
    moving_platforms: &[crate::platforms::MovingPlatformState],
    attack: &mut Option<crate::PlayerAttackState>,
    feedback: &mut FrameFeedback,
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
    let filtered = controls_for_hitstun(controls, feel, combat.hitstun_timer);
    let input = filtered.engine_input(frame_dt);
    let control_world =
        features::world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);
    let control_events = ae::update_player_control_with_tuning(
        &control_world,
        player,
        input,
        frame_dt,
        tuning,
    );
    if control_events.reset {
        reset_sandbox(
            world,
            &mut feedback.sfx,
            &mut feedback.vfx,
            player,
            sim_state,
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
        &mut feedback.sfx,
        &mut feedback.vfx,
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
/// Owns: `update_time_scale` (hitstop / bullet-time / slowmo ramp),
/// scaled `sim_dt`, moving-platform tick + ride-along, sandbox-side
/// solid rebuild, `update_player_simulation_with_tuning`, landing-dust
/// feedback through `handle_player_events`.
///
/// Should not own: feature-runtime ticks or interact-buffering. New
/// game-time-affected motion (gravity tweaks, platform AI, knockback
/// resolution) belongs here. Returns `Return` if simulation asked for a
/// sandbox reset.
pub(super) fn player_simulation_phase(
    controls: ControlFrame,
    world: &ae::World,
    player: &mut ae::Player,
    dev_state: &crate::SandboxDevState,
    sim_state: &mut crate::SandboxSimState,
    moving_platforms: &mut Vec<crate::platforms::MovingPlatformState>,
    attack: &mut Option<crate::PlayerAttackState>,
    feedback: &mut FrameFeedback,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    reset_room_features: &mut MessageWriter<features::ResetRoomFeaturesEvent>,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
    interaction: &mut crate::player::PlayerInteractionState,
    blink_cam: &mut crate::player::PlayerBlinkCameraState,
) -> PhaseOutcome {
    let filtered = controls_for_hitstun(controls, feel, combat.hitstun_timer);
    let input = filtered.engine_input(frame_dt);

    crate::update_time_scale(dev_state.slowmo, sim_state, player, combat.hitstop_timer, frame_dt, feel);
    let sim_dt = sandbox_dt(combat.hitstop_timer, sim_state.time_scale, frame_dt);

    let mut riding_platform = None;
    for (index, platform) in moving_platforms.iter_mut().enumerate() {
        let delta = platform.update(sim_dt);
        if riding_platform.is_none() && platform.is_riding(player) {
            riding_platform = Some((index, delta, platform.pos, platform.direction()));
        }
    }
    let riding_now = riding_platform.is_some();
    let was_riding_platform = player.was_riding_platform;
    if riding_now != was_riding_platform {
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
    player.was_riding_platform = riding_now;
    if let Some((_, platform_delta, _, _)) = riding_platform {
        player.pos += platform_delta;
    }
    let collision_world =
        features::world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);

    let was_grounded = player.on_ground;
    let sim_events = ae::update_player_simulation_with_tuning(
        &collision_world,
        player,
        input,
        sim_dt,
        tuning,
    );
    if sim_events.reset {
        reset_sandbox(
            world,
            &mut feedback.sfx,
            &mut feedback.vfx,
            player,
            sim_state,
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
        &mut feedback.sfx,
        &mut feedback.vfx,
        player,
        combat,
        blink_cam,
        sim_events,
        Some(was_grounded),
    );
    PhaseOutcome::Continue
}

/// Phase 8 — apply heals/damage, dialogue start, feature-driven reset.
///
/// Owns: `handle_player_damage_events`, `remember_safe_player_position`
/// when the player wasn't damaged this frame.
///
/// Should not own: the feature tick itself or attack / room-transition routing.
pub(super) fn damage_heal_dialogue_phase(
    world: &ae::World,
    player: &mut ae::Player,
    sim_state: &mut crate::SandboxSimState,
    moving_platforms: &[crate::platforms::MovingPlatformState],
    feedback: &mut FrameFeedback,
    player_health: Option<&mut crate::player::PlayerHealth>,
    player_damage_events: &[features::PlayerDamageEvent],
    banner: &mut features::GameplayBanner,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    difficulty_multiplier: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
) {
    let feature_damaged_player = !player_damage_events.is_empty();
    handle_player_damage_events(
        world,
        &mut feedback.sfx,
        &mut feedback.vfx,
        &mut feedback.died,
        player,
        sim_state,
        banner,
        player_health,
        player_damage_events,
        tuning,
        feel,
        difficulty_multiplier,
        anim,
        combat,
    );
    let safe_world = features::world_with_sandbox_solids(
        world,
        moving_platforms,
        feature_ecs_overlay,
    );
    let ctx = crate::SafePositionContext {
        damaged_this_frame: feature_damaged_player,
        in_hitstun: combat.hitstun_timer > 0.0,
        feature_requested_reset: false,
        blink_grace_active: player.blink_grace_timer > 0.0,
        room_transitioning: sim_state.room_transition_cooldown > 0.0,
    };
    crate::remember_safe_player_position(sim_state, player, &safe_world, ctx);
}

/// Phase 10 — slash / pogo attack triggering.
///
/// Owns: hitstun gate, attack/pogo button check, dispatching to
/// `start_attack` / `advance_attack` (which emit sfx/vfx/debris and run
/// feature-side hit application during active frames).
///
/// Should not own: damage application semantics — those live in
/// `advance_attack` and the engine. New attack archetypes should add
/// branches here only when the trigger condition differs.
pub(super) fn attack_phase(
    controls: &ControlFrame,
    world: &ae::World,
    moving_platforms: &[crate::platforms::MovingPlatformState],
    player: &mut ae::Player,
    attack: &mut Option<crate::PlayerAttackState>,
    feedback: &mut FrameFeedback,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    damage_events: &mut MessageWriter<features::DamageEvent>,
    pogo_bounces: &mut MessageWriter<features::PogoBounceEvent>,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
) {
    if combat.hitstun_timer <= 0.0 && (controls.attack_pressed || controls.pogo_pressed) {
        start_attack(&mut feedback.sfx, &mut feedback.vfx, player, attack, anim, *controls);
    }
    advance_attack(
        &mut feedback.sfx,
        &mut feedback.vfx,
        world,
        moving_platforms,
        player,
        attack,
        anim,
        combat,
        tuning,
        feel,
        frame_dt,
        feature_ecs_overlay,
        damage_events,
        pogo_bounces,
    );
}

