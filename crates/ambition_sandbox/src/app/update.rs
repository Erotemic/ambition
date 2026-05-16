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
/// Phase order (each phase comments its scope and what it should not own).
/// Phases marked "(extracted)" are real Bevy systems in `sim_systems.rs`
/// or `world_flow.rs`; they no longer run inline here.
///
/// Pre-tick (CoreSimulation, before sandbox_update):
/// - `sync_live_player_dev_edits_system` (extracted) — F3 inspector
///   ability/tuning edits, runs every frame including paused.
/// - `input_timer_system` (extracted) — gameplay timer decay +
///   double-tap detection, gated by `gameplay_allowed`.
/// - `interaction_input_system` (extracted) — fold raw Interact +
///   double-tap-up signal, gate by hit-stun, update
///   `PlayerInteractionState`'s buffer.
/// - `apply_suspended_time_scale_system` (extracted) — zeros
///   `SandboxSimState::time_scale` when gameplay is suspended;
///   complement of `sandbox_update`'s `gameplay_allowed` gate.
///
/// Inside sandbox_update (gated by `run_if(gameplay_allowed)`):
/// 1. `reset_phase` — explicit reset input.
/// 2. `player_control_phase` — control-clock player update + pogo routing.
/// 3. `player_simulation_phase` — sim-clock player update + landing dust.
/// 4. Collect ECS feature events and any damage/heals for this frame.
/// 5. `damage_heal_dialogue_phase` — heals/damage/dialogue/feature reset.
/// 6. `attack_phase` — slash/pogo attack triggering.
/// 7. `flush_feedback` — drains `SfxMessage` / `VfxMessage` /
///    `DebrisBurstMessage` queues into the bundled writers (single
///    drain at the bottom of the `'frame` labeled block).
///
/// Post-tick (CoreSimulation, after sandbox_update):
/// - `detect_room_transition_system` (extracted) — loading-zone overlap
///   detection; emits `RoomTransitionRequested`.
/// - `apply_room_transition_system` (extracted) — consumes the message
///   and runs `load_room`.
/// - `cleanup_timers_system` (extracted) — flash / preset / slash /
///   blink animation timer decay, runs every frame unconditionally.
pub fn sandbox_update(
    time: Res<Time>,
    world: Res<GameWorld>,
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

    // Each phase appends to `feedback` (sfx/vfx/debris/died); the labeled
    // block below lets any phase short-circuit the tick via `break` while
    // keeping the single `flush_feedback` drain at the bottom. This also
    // guarantees feedback is drained on the "no player entity yet" path,
    // since that's modeled as `break` here.
    'frame: {
        // Acquire ECS player components for this frame.
        let Ok((mut authority, mut anim, mut combat, mut interaction, mut blink_cam)) = player_q.single_mut() else {
            break 'frame;
        };
        let player = &mut authority.player;
        // Note: `sync_live_player_dev_edits_system` (in sim_systems) runs
        // unconditionally before sandbox_update so dev-tool ability /
        // tuning edits land even while the sim is paused.

        // sandbox_update no longer queries leafwing directly. Input arrives
        // through `Res<ControlFrame>` — visible builds derive it from
        // ActionState in `populate_control_frame_from_actions` (runs
        // `.before(sandbox_update)`); headless / RL drivers can write the
        // resource directly. Debug hotkeys live in their own presentation-
        // side system, also `.before(sandbox_update)`. The local copy is
        // read-only for the rest of this function; `interaction_input_system`
        // already wrote the buffered interact result into
        // `PlayerInteractionState` before sandbox_update started, so
        // `controls.interact_pressed` is just the raw frame input.
        let controls = *control_frame;
        let frame_dt = time.delta_secs();

        // Pause/resume toggling has moved to `pause_menu::pause_menu_toggle`,
        // which runs `.before(SandboxSet::CoreSimulation)`. The
        // `start_pressed` flag is still read here for compile-completeness;
        // the pause logic lives in the pause menu so it can drive a real
        // overlay.
        let _ = controls.start_pressed;

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
            break 'frame;
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
            break 'frame;
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
            break 'frame;
        }

        // interaction_input_phase has moved to `interaction_input_system`
        // (sim_systems), which runs after input_timer_system and before
        // sandbox_update. It updates `PlayerInteractionState`'s buffer in
        // place; downstream code reads `interaction.buffered()` directly.

        let player_damage_events: Vec<features::PlayerDamageEvent> =
            queues.player_damage_events.read().copied().collect();

        let player_health = queues.player_health.single_mut().ok();
        damage_heal_dialogue_phase(
            &world.0,
            player,
            &mut queues.sim_state,
            &queues.moving_platforms.0,
            &mut feedback,
            player_health.map(|h| h.into_inner()),
            &player_damage_events,
            &mut queues.banner,
            tuning,
            feel,
            difficulty_multiplier,
            &queues.feature_ecs_overlay,
            &mut *anim,
            &mut *combat,
        );

        // room_transition_phase has moved to `detect_room_transition_system`
        // (sim_systems), which runs after sandbox_update and emits a
        // `RoomTransitionRequested` message consumed by
        // `apply_room_transition_system`. attack_phase no longer skips on
        // a transition frame (small semantic change; player position is
        // still deterministic per replay_fixture_regression).

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

        // cleanup_timers_system runs after write_player_ecs_components in
        // the CoreSimulation chain every frame unconditionally (it lives
        // outside sandbox_update so paused/dialogue modes still wind down
        // flash and landing-pose timers).
    }

    flush_feedback(&mut feedback, &mut event_writers);
}
