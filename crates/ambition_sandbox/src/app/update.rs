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
use super::phases::*;
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

/// Bevy gameplay system that drives the sandbox simulation.
///
/// This is intentionally a thin orchestrator around named `*_phase`
/// helpers — the function body should make the gameplay frame order
/// readable in one screen so future agents can find the right phase by
/// grep without reading the whole loop.
///
/// The next likely refactor is promoting these phase helpers into
/// individually ordered Bevy systems, one at a time, once their behavior
/// is covered by tests. Until then, keep them as plain functions sharing
/// `&mut PlayerMovementAuthority` and `&mut FrameFeedback` so the borrow
/// graph stays linear.
///
/// Phase order (each phase comments its scope and what it should not own):
/// 0. Gameplay-suspended modes (pause / dialogue / room transition /
///    cutscene) are filtered out by `run_if(gameplay_allowed)`; the
///    presentation-side `apply_suspended_time_scale_system` in
///    `sim_systems` zeros `time_scale` for those modes instead.
/// 1. `input_timer_system` (extracted to `sim_systems`) — gameplay timer
///    decay + double-tap detection. Runs before `sandbox_update`.
/// 2. `reset_phase` — explicit reset input.
/// 3. `player_control_phase` — control-clock player update + pogo routing.
/// 4. `player_simulation_phase` — sim-clock player update + landing dust.
/// 5. `interaction_input_phase` — interact / double-tap-up + buffering.
/// 6. Collect ECS feature events and any damage/heals for this frame.
/// 7. `damage_heal_dialogue_phase` — heals/damage/dialogue/feature reset.
/// 8. `room_transition_phase` — loading-zone transition request emission.
///    `apply_room_transition_system` runs after `sandbox_update` and
///    consumes the request.
/// 9. `attack_phase` — slash/pogo attack triggering.
/// 10. `cleanup_timers_system` (extracted to `sim_systems`) — flash /
///     preset / slash / blink animation timer decay. Runs after
///     `sandbox_update` every frame unconditionally.
/// 11. `flush_feedback` — drains `SfxMessage` / `VfxMessage` /
///     `DebrisBurstMessage` queues into the bundled writers.
pub fn sandbox_update(
    time: Res<Time>,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut event_writers: SandboxEventWriters,
    control_frame: Res<ControlFrame>,
    user_settings: Res<crate::settings::UserSettings>,
    mut queues: SandboxQueues,
    mut player_q: Query<
        (
            &mut crate::player::PlayerMovementAuthority,
            &mut crate::player::PlayerAnimState,
            &mut crate::player::PlayerCombatState,
            &mut crate::player::PlayerInteractionState,
            &mut crate::player::PlayerBlinkCameraState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let mut feedback = FrameFeedback::new();
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    // Compose difficulty + assist + the fine-grained menu multiplier
    // into one scalar that `handle_player_damage_events` consults.
    // Assist mode halves incoming damage on top of difficulty so a
    // user who needs the extra help can stack the two.
    let assist_factor = match user_settings.gameplay.assist {
        crate::settings::AssistMode::Off => 1.0,
        crate::settings::AssistMode::On => 0.5,
    };
    let difficulty_multiplier = user_settings.gameplay.difficulty.damage_taken_multiplier()
        * user_settings.gameplay.player_damage_multiplier
        * assist_factor;

    // Acquire ECS player components for this frame.
    let Ok((mut authority, mut anim, mut combat, mut interaction, mut blink_cam)) = player_q.single_mut() else {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    };
    let player = &mut authority.player;
    // Note: `sync_live_player_dev_edits_system` (in sim_systems) runs
    // unconditionally before sandbox_update so dev-tool ability /
    // tuning edits land even while the sim is paused.

    // sandbox_update no longer queries leafwing directly. Input arrives
    // through `Res<ControlFrame>` — visible builds derive it from
    // ActionState in `populate_control_frame_from_actions` (runs
    // `.before(sandbox_update)`); headless / RL drivers can write the
    // resource directly. Debug hotkeys live in their own presentation-side
    // system, also `.before(sandbox_update)`. Local mutable copy because
    // `interaction_input_phase` rewrites `controls.interact_pressed` via
    // the input buffer (runtime state, not raw input).
    let mut controls = *control_frame;
    let frame_dt = time.delta_secs();

    // Pause/resume toggling has moved to `pause_menu::pause_menu_toggle`,
    // which runs `.before(SandboxSet::CoreSimulation)`. The `start_pressed`
    // flag is still read here for compile-completeness; the pause logic
    // lives in the pause menu so it can drive a real overlay.
    let _ = controls.start_pressed;

    // `input_timer_system` ran earlier in the CoreSimulation chain:
    // it ticked gameplay timers, detected double-tap gestures, and stored
    // the door-double-tap-up result in the ECS component. Read and clear
    // it here so interaction_input_phase can consume the value exactly
    // once per frame.
    let door_double_tap_up = interaction.double_tap_up_pending;
    interaction.double_tap_up_pending = false;

    if matches!(
        reset_phase(
            &controls,
            &world.0,
            player,
            &mut queues.sim_state,
            &mut queues.current_attack.0,
            &mut feedback,
            tuning,
            feel,
            &mut queues.reset_room_features,
            &mut *anim,
            &mut *combat,
            &mut *interaction,
            &mut *blink_cam,
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    if matches!(
        player_control_phase(
            controls,
            &world.0,
            player,
            &mut queues.sim_state,
            &queues.moving_platforms.0,
            &mut queues.current_attack.0,
            &mut feedback,
            tuning,
            feel,
            frame_dt,
            &queues.feature_ecs_overlay,
            &mut queues.reset_room_features,
            &mut queues.pogo_bounces,
            &mut *anim,
            &mut *combat,
            &mut *interaction,
            &mut *blink_cam,
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    if matches!(
        player_simulation_phase(
            controls,
            &world.0,
            player,
            &queues.dev_state,
            &mut queues.sim_state,
            &mut queues.moving_platforms.0,
            &mut queues.current_attack.0,
            &mut feedback,
            tuning,
            feel,
            frame_dt,
            &queues.feature_ecs_overlay,
            &mut queues.reset_room_features,
            &mut *anim,
            &mut *combat,
            &mut *interaction,
            &mut *blink_cam,
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    interaction_input_phase(
        &mut controls,
        &mut *interaction,
        &*combat,
        feel,
        door_double_tap_up,
        frame_dt,
    );

    let player_damage_events: Vec<features::PlayerDamageEvent> =
        queues.player_damage_events.read().copied().collect();

    if let Ok(mut health) = queues.player_health.single_mut() {
        damage_heal_dialogue_phase(
            &world.0,
            player,
            &mut queues.sim_state,
            &queues.moving_platforms.0,
            &mut feedback,
            Some(&mut *health),
            &player_damage_events,
            &mut queues.banner,
            tuning,
            feel,
            difficulty_multiplier,
            &queues.feature_ecs_overlay,
            &mut *anim,
            &mut *combat,
        );
    } else {
        damage_heal_dialogue_phase(
            &world.0,
            player,
            &mut queues.sim_state,
            &queues.moving_platforms.0,
            &mut feedback,
            None,
            &player_damage_events,
            &mut queues.banner,
            tuning,
            feel,
            difficulty_multiplier,
            &queues.feature_ecs_overlay,
            &mut *anim,
            &mut *combat,
        );
    }

    if matches!(
        room_transition_phase(
            &controls,
            &room_set,
            player,
            &queues.sim_state,
            &mut queues.transition_requests,
            &mut *interaction,
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    attack_phase(
        &controls,
        &world.0,
        &queues.moving_platforms.0,
        player,
        &mut queues.current_attack.0,
        &mut feedback,
        tuning,
        feel,
        frame_dt,
        &queues.feature_ecs_overlay,
        &mut queues.damage_events,
        &mut queues.pogo_bounces,
        &mut *anim,
        &mut *combat,
    );

    // cleanup_timers_system runs after write_player_ecs_components in the
    // CoreSimulation chain every frame unconditionally (it lives outside
    // sandbox_update so paused/dialogue modes still wind down flash and
    // landing-pose timers).

    flush_feedback(&mut feedback, &mut event_writers);
}
