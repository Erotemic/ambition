use super::*;

/// SystemParam-friendly bundle: gives `sandbox_update` everything it
/// needs to record one frame and (if requested) write a dump.
#[allow(clippy::too_many_arguments)]
pub fn record_simulation_frame(
    buffer: &mut GameplayTraceBuffer,
    runtime: &SandboxRuntime,
    world: &ae::World,
    controls: ControlFrame,
    real_dt: f32,
    sim_dt: f32,
    game_mode: &str,
    active_area: &str,
    locomotion: &str,
    body_mode: &str,
) {
    let oob = detect_oob(&runtime.player, world, OOB_MARGIN);
    let frame = build_frame(
        runtime,
        world,
        controls,
        real_dt,
        sim_dt,
        game_mode,
        active_area,
        buffer.sequence,
        buffer.tick,
        locomotion,
        body_mode,
    );
    record_frame(buffer, frame, oob.as_ref());
}

/// Bevy system: drains pending dump requests, writes JSON+MD if any.
/// Ordered after `sandbox_update` so manual F8 presses recorded earlier
/// in the frame still see the latest snapshot.
pub fn flush_pending_dump(mut buffer: ResMut<GameplayTraceBuffer>) {
    let Some(reason) = buffer.dump_request.take() else {
        return;
    };
    let dir = default_dump_dir();
    match write_dump(&buffer, &reason, &dir) {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            buffer.last_dump_path = Some(path_str.clone());
            buffer.last_dump_status = Some(format!("OK: {path_str}"));
            eprintln!("ambition trace dumped: {path_str}");
        }
        Err(err) => {
            buffer.last_dump_status = Some(format!("error: {err}"));
            eprintln!("ambition trace dump failed: {err}");
        }
    }
}

/// Presentation-side hotkey reader: F8 sets a manual dump request.
/// Lives in `trace.rs` rather than `app.rs` so the lookup is grep-able
/// near the rest of the recorder code.
pub fn handle_trace_hotkey(
    keys: Res<ButtonInput<KeyCode>>,
    mut buffer: ResMut<GameplayTraceBuffer>,
) {
    if keys.just_pressed(KeyCode::F8) {
        buffer.request_dump(DumpReason::Manual);
    }
}

/// Bevy system: when in scope, writes one trace frame per Update tick by
/// reading the resources `sandbox_update` already consumes. We keep this
/// outside the phase pipeline so the recorder stays out of `sandbox_update`'s
/// 16-system-param budget. Synthesizes per-frame events by diffing
/// against the previous tick's snapshot (input edges, locomotion
/// changes, dash/jump/blink heuristics, room transitions, resets,
/// damage, and unexplained position deltas).
///
/// The trace's collision view (`nearby_collision`, `detect_oob`'s
/// inside-solid check) uses the same `world_with_sandbox_solids` view
/// that `sandbox_update` feeds to the engine. Without that, the trace
/// would miss feature-runtime solids the player can collide with —
/// which is exactly what happened in the May 2026 wall-cling teleport
/// trace, where `nearby_collision` was empty even though the player
/// was clinging to a wall.
pub fn record_frame_system(
    mut buffer: ResMut<GameplayTraceBuffer>,
    runtime: Res<SandboxRuntime>,
    world: Res<GameWorld>,
    control_frame: Res<ControlFrame>,
    time: Res<Time>,
    rooms: Option<Res<crate::rooms::RoomSet>>,
    mode: Res<State<crate::game_mode::GameMode>>,
) {
    let real_dt = time.delta_secs();
    let sim_dt = real_dt * runtime.time_scale;
    let active_area = rooms
        .as_ref()
        .map(|r| r.active_spec().id.clone())
        .unwrap_or_else(|| "<unknown>".into());
    let mode_label = format!("{:?}", mode.get());
    let locomotion_state = ae::LocomotionState::from_player(&runtime.player);
    let body_mode_state = ae::BodyMode::from_player(&runtime.player);
    let locomotion = locomotion_state.label().to_string();
    let body_mode = body_mode_state.label().to_string();

    let augmented_world = crate::features::world_with_sandbox_solids(
        &world.0,
        &runtime.moving_platforms,
        &runtime.features,
    );

    // Synthesize events from the diff before pushing the frame so the
    // event tick aligns with the frame the user will see in the dump.
    synthesize_events_from_diff(
        &mut buffer,
        &runtime,
        *control_frame,
        real_dt,
        &active_area,
        locomotion_state,
        body_mode_state,
    );

    record_simulation_frame(
        &mut buffer,
        &runtime,
        &augmented_world,
        *control_frame,
        real_dt,
        sim_dt,
        &mode_label,
        &active_area,
        &locomotion,
        &body_mode,
    );

    // Update the diff snapshot AFTER recording so the next tick's
    // `synthesize_events_from_diff` can compare against this frame's
    // state. Setting it after `record_simulation_frame` also means a
    // panic / early return upstream leaves the previous snapshot in
    // place rather than corrupting the timeline.
    update_previous_snapshot(
        &mut buffer,
        &runtime,
        *control_frame,
        &active_area,
        locomotion_state,
        body_mode_state,
    );
}
