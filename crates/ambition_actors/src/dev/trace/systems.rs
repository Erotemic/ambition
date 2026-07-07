//! Trace recorder Bevy systems + the headless entry point.
//!
//! `record_simulation_frame` is the sim-side core (callable from the headless
//! driver); `record_frame_system` wires it into the live app, `handle_trace_hotkey`
//! arms a manual dump, and `flush_pending_dump` writes the ring buffer to disk.

use super::*;

/// SystemParam-friendly bundle: gives the player tick everything it
/// needs to record one frame and (if requested) write a dump.
#[allow(clippy::too_many_arguments)]
pub fn record_simulation_frame(
    buffer: &mut GameplayTraceBuffer,
    clusters: &ae::BodyClustersMut<'_>,
    combat: &ambition_characters::actor::BodyCombat,
    clock: &ambition_time::ClockState,
    safety: &crate::player::PlayerSafetyState,
    world: &ae::World,
    controls: ControlFrame,
    real_dt: f32,
    sim_dt: f32,
    game_mode: &str,
    active_area: &str,
    moving_platforms: &[crate::world::platforms::MovingPlatformState],
    locomotion: &str,
    body_mode: &str,
) {
    let oob = detect_oob_from_kinematics(
        clusters.kinematics.pos,
        clusters.kinematics.vel,
        clusters.kinematics.aabb(),
        world,
        OOB_MARGIN,
    );
    let frame = build_frame(
        clusters,
        combat,
        clock,
        safety,
        world,
        controls,
        real_dt,
        sim_dt,
        game_mode,
        active_area,
        buffer.sequence,
        buffer.tick,
        moving_platforms,
        locomotion,
        body_mode,
    );
    record_frame(buffer, frame, oob.as_ref());
}

/// Bevy system: drains pending dump requests, writes JSON+MD if any.
/// Ordered after the player tick so manual F8 presses recorded earlier
/// in the frame still see the latest snapshot.
///
/// Wasm (`target_arch = "wasm32"`): drains + clears the dump request
/// (so the buffer doesn't accumulate stale requests) but skips
/// `write_dump`. The dump path uses `std::fs` and `SystemTime::now()`,
/// neither of which is supported under `wasm32-unknown-unknown` —
/// `SystemTime::now()` panics with "time not implemented on this
/// platform" exactly like `Instant::now()`. Reports a single warning
/// per drop so the user knows F8 was received but ignored.
pub fn flush_pending_dump(mut buffer: ResMut<GameplayTraceBuffer>) {
    let Some(_reason) = buffer.dump_request.take() else {
        return;
    };
    #[cfg(target_arch = "wasm32")]
    {
        let msg = "trace dump skipped: file IO + SystemTime::now() not supported on wasm32";
        buffer.last_dump_status = Some(msg.to_string());
        bevy::log::warn!(target: "ambition::trace", "{msg}");
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let dir = default_dump_dir();
        match write_dump(&buffer, &_reason, &dir) {
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
/// reading the resources the player tick already consumes. We keep this
/// outside the phase pipeline so the recorder stays out of the player tick's
/// 16-system-param budget. Synthesizes per-frame events by diffing
/// against the previous tick's snapshot (input edges, locomotion
/// changes, dash/jump/blink heuristics, room transitions, resets,
/// damage, and unexplained position deltas).
///
/// The trace's collision view (`nearby_collision`, `detect_oob`'s
/// inside-solid check) uses the same `world_with_sandbox_solids` view
/// that the player tick feeds to the engine. Without that, the trace
/// would miss feature-runtime solids the player can collide with —
/// which is exactly what happened in the May 2026 wall-cling teleport
/// trace, where `nearby_collision` was empty even though the player
/// was clinging to a wall.
pub fn record_frame_system(
    mut buffer: ResMut<GameplayTraceBuffer>,
    clock: Res<ambition_time::ClockState>,
    platform_set: Res<crate::MovingPlatformSet>,
    world: Res<RoomGeometry>,
    time: Res<Time>,
    rooms: Option<Res<crate::rooms::RoomSet>>,
    mode: Res<State<crate::game_mode::GameMode>>,
    feature_ecs_overlay: Res<crate::features::FeatureEcsWorldOverlay>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            Option<&ambition_characters::actor::BodyHealth>,
            &crate::player::PlayerSafetyState,
            &crate::player::PlayerInputFrame,
            &ambition_characters::actor::BodyCombat,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
    #[cfg(feature = "portal")] mut teleported: MessageReader<ambition_portal::BodyTeleported>,
) {
    // A portal jump is an intentional teleport — open a short suppression window
    // so neither the position-delta snap nor the exit-side inside-solid check
    // auto-dumps a trace for a normal crossing. Only the primary player emits
    // `BodyTeleported`. The window (not a one-frame flag) covers the transfer plus
    // the exit-side settle; it counts down in `record_frame`. With portal compiled
    // out there are no teleports to expect.
    #[cfg(feature = "portal")]
    if teleported.read().next().is_some() {
        buffer.teleport_suppress_ticks = super::PORTAL_TELEPORT_SUPPRESS_FRAMES;
    }
    let Ok((mut cluster_item, player_health, safety, input, combat)) = player_q.single_mut() else {
        return;
    };
    // Trace recording is read-only. Walks the cluster components
    // directly through `BodyClustersMut`.
    let clusters = cluster_item.as_clusters_mut();
    let control_frame = input.frame;
    let real_dt = time.delta_secs();
    let sim_dt = real_dt * clock.time_scale;
    let active_area = rooms
        .as_ref()
        .map(|r| r.active_spec().id.clone())
        .unwrap_or_else(|| "<unknown>".into());
    let mode_label = format!("{:?}", mode.get());
    let hp_current = player_health.map_or(0, |h| h.health.current);
    let locomotion_state = ae::LocomotionState::from_clusters(
        clusters.ground,
        clusters.wall,
        clusters.flight,
        clusters.dash,
        clusters.blink,
        clusters.ledge,
    );
    let body_mode_state = ae::BodyMode::from_clusters(clusters.body_mode);
    let locomotion = locomotion_state.label().to_string();
    let body_mode = body_mode_state.label().to_string();

    let augmented_world =
        crate::features::world_with_sandbox_solids(&world.0, &platform_set.0, &feature_ecs_overlay);

    synthesize_events_from_diff(
        &mut buffer,
        &clusters,
        hp_current,
        control_frame,
        real_dt,
        &active_area,
        locomotion_state,
        body_mode_state,
        &augmented_world,
    );

    record_simulation_frame(
        &mut buffer,
        &clusters,
        combat,
        &clock,
        safety,
        &augmented_world,
        control_frame,
        real_dt,
        sim_dt,
        &mode_label,
        &active_area,
        &platform_set.0,
        &locomotion,
        &body_mode,
    );

    update_previous_snapshot(
        &mut buffer,
        &clusters,
        hp_current,
        control_frame,
        &active_area,
        locomotion_state,
        body_mode_state,
    );
}
