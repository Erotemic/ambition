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
/// individually ordered Bevy systems / `SimSet`s once their behavior is
/// covered by tests. Until then, keep them as plain functions on a
/// shared `&mut SandboxRuntime` + `&mut FrameFeedback` so the borrow
/// graph stays linear.
///
/// Phase order (each phase comments its scope and what it should not own):
/// 1. `mode_gate_phase` — dialogue / pause / non-gameplay early returns.
/// 2. `input_timer_phase` — gameplay timer decay + double-tap detection.
/// 3. `reset_phase` — explicit reset input.
/// 4. `player_control_phase` — control-clock player update + pogo routing.
/// 5. `player_simulation_phase` — sim-clock player update + landing dust.
/// 6. `interaction_input_phase` — interact / double-tap-up + buffering.
/// 7. `feature_runtime_phase` — `runtime.features.update` + feedback.
/// 8. `damage_heal_dialogue_phase` — heals/damage/dialogue/feature reset.
/// 9. `room_transition_phase` — loading-zone transition + `load_room`.
/// 10. `attack_phase` — slash/pogo attack triggering.
/// 11. `cleanup_timers_phase` — flash/preset/slash animation timer decay.
/// 12. `flush_feedback` — drains `SfxMessage` / `VfxMessage` /
///     `DebrisBurstMessage` queues into the bundled writers.
pub fn sandbox_update(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<GameWorld>,
    mut room_set: ResMut<rooms::RoomSet>,
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    feel_tuning: Res<SandboxFeelTuning>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut runtime: ResMut<SandboxRuntime>,
    mut event_writers: SandboxEventWriters,
    control_frame: Res<ControlFrame>,
    user_settings: Res<crate::settings::UserSettings>,
    mut queues: SandboxQueues,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    game_assets: Option<Res<crate::game_assets::GameAssets>>,
) {
    let mut feedback = FrameFeedback::new();
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    let physics_settings = runtime.physics_settings;
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
    dev_tools::sync_live_ability_edits(&mut runtime, editable_abilities.as_engine(), tuning);

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

    if matches!(
        mode_gate_phase(mode.get(), &mut runtime, frame_dt),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    // Pause/resume toggling has moved to `pause_menu::pause_menu_toggle`,
    // which runs `.before(sandbox_update)`. The `start_pressed` flag is
    // still read here for compile-completeness; the pause logic itself
    // lives in the pause menu so it can drive a real overlay.
    let _ = controls.start_pressed;

    let door_double_tap_up = input_timer_phase(&mut controls, &mut runtime, feel, frame_dt);

    if matches!(
        reset_phase(
            &controls,
            &world.0,
            &mut runtime,
            &mut feedback,
            tuning,
            feel,
            &mut queues.reset_room_features,
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
            &mut runtime,
            &mut feedback,
            tuning,
            feel,
            frame_dt,
            &queues.feature_ecs_overlay,
            &mut queues.reset_room_features,
            &mut queues.pogo_bounces,
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
            &mut runtime,
            &mut feedback,
            tuning,
            feel,
            frame_dt,
            &queues.feature_ecs_overlay,
            &mut queues.reset_room_features,
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    interaction_input_phase(
        &mut controls,
        &mut runtime,
        feel,
        door_double_tap_up,
        frame_dt,
    );

    let mut feature_events = feature_runtime_phase(
        &controls,
        &world.0,
        &mut runtime,
        &mut feedback,
        feel,
        frame_dt,
        &queues.feature_ecs_overlay,
    );
    for ecs_feature_event in queues.feature_events.read() {
        handle_feature_events(
            &mut feedback.sfx,
            &mut feedback.vfx,
            &mut feedback.debris,
            &ecs_feature_event.0,
            runtime.player.pos,
        );
        feature_events.merge(ecs_feature_event.0.clone());
    }

    // Forward typed gameplay effects into Bevy's message stream. Domain
    // consumers run later in the same Update frame, before boss/quest
    // progression systems that consume the routed queues.
    crate::features::write_feature_effects(&mut queues.gameplay_effects, &feature_events);

    if matches!(
        damage_heal_dialogue_phase(
            &world.0,
            &mut runtime,
            &mut feedback,
            &feature_events,
            &mut queues.banner,
            &mut next_mode,
            tuning,
            feel,
            difficulty_multiplier,
            &queues.feature_ecs_overlay,
            &mut queues.reset_room_features,
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    if matches!(
        room_transition_phase(
            &mut commands,
            &controls,
            &mut world,
            &mut room_set,
            &mut runtime,
            &mut feedback,
            &room_visuals,
            tuning,
            feel,
            physics_settings,
            game_assets.as_deref(),
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    attack_phase(
        &controls,
        &world.0,
        &mut runtime,
        &mut feedback,
        tuning,
        feel,
        frame_dt,
        &queues.feature_ecs_overlay,
        &mut queues.damage_events,
        &mut queues.pogo_bounces,
    );

    cleanup_timers_phase(&mut runtime, frame_dt);

    flush_feedback(&mut feedback, &mut event_writers);
}
