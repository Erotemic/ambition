use super::*;

/// Inspect the current player state against the active world and produce
/// the *first* OOB reason found, if any. Order matters: NaN/inf should
/// be reported before "outside envelope" because both can be true.
///
/// The world envelope / inside-solid check is delegated to
/// `ae::classify_player_safety` so the trace recorder and
/// `SandboxRuntime::remember_safe_player_position` use the same
/// definition. The recorder layers the trace-only "absurd velocity"
/// rule on top.
pub fn detect_oob(player: &ae::Player, world: &ae::World, margin: f32) -> Option<OobReason> {
    let speed = player.vel.length();
    if speed.is_finite() && speed > ABSURD_VELOCITY_MAGNITUDE {
        return Some(OobReason::AbsurdVelocity { magnitude: speed });
    }
    match ae::classify_player_safety(player, world, margin, |b| {
        matches!(b.kind, ae::BlockKind::Solid)
    }) {
        ae::PlayerSafetyVerdict::Safe => None,
        ae::PlayerSafetyVerdict::PositionNonFinite => Some(OobReason::PositionNonFinite),
        ae::PlayerSafetyVerdict::VelocityNonFinite => Some(OobReason::VelocityNonFinite),
        ae::PlayerSafetyVerdict::OutsideWorldEnvelope { axis } => {
            Some(OobReason::OutsideWorldEnvelope { axis })
        }
        ae::PlayerSafetyVerdict::InsideSolid => {
            // Find which block we're inside so the dump names it. The
            // shared classifier doesn't return the block reference (it
            // takes a predicate closure to stay engine-side); a small
            // second walk here is fine for the "we're already in trouble"
            // path.
            let aabb = player.aabb();
            let block_name = world
                .blocks
                .iter()
                .find(|b| matches!(b.kind, ae::BlockKind::Solid) && aabb.strict_intersects(b.aabb))
                .map(|b| b.name.clone())
                .unwrap_or_else(|| "<unknown>".into());
            Some(OobReason::InsideSolid { block_name })
        }
    }
}

fn nearby_collision(world: &ae::World, player: &ae::Player) -> Vec<CollisionTraceShape> {
    let center = player.pos;
    let mut hits: Vec<CollisionTraceShape> = world
        .blocks
        .iter()
        .map(|block| {
            let bcenter = block.aabb.center();
            let distance = (bcenter - center).length();
            CollisionTraceShape {
                kind: format!("{:?}", block.kind),
                name: block.name.clone(),
                aabb: block.aabb.into(),
                distance,
            }
        })
        .filter(|shape| shape.distance < NEARBY_COLLISION_RADIUS)
        .collect();
    hits.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(MAX_NEARBY_COLLISION);
    hits
}

/// Build a `GameplayTraceFrame` from current sim resources. This lives
/// next to `record_frame_in_simulation` so the sandbox phase pipeline
/// can call it once per `sandbox_update` tick.
#[allow(clippy::too_many_arguments)]
pub fn build_frame(
    runtime: &SandboxRuntime,
    sim_state: &crate::SandboxSimState,
    world: &ae::World,
    controls: ControlFrame,
    real_dt: f32,
    sim_dt: f32,
    game_mode: &str,
    active_area: &str,
    seq: u64,
    tick: u64,
    moving_platforms: &[crate::platforms::MovingPlatformState],
    locomotion: &str,
    body_mode: &str,
) -> GameplayTraceFrame {
    let player = &runtime.player;
    GameplayTraceFrame {
        seq,
        tick,
        real_dt,
        sim_dt,
        time_scale: sim_state.time_scale,
        game_mode: game_mode.into(),
        active_area: active_area.into(),
        world_size: world.size.into(),
        world_spawn: world.spawn.into(),
        player: PlayerTraceState {
            pos: player.pos.into(),
            vel: player.vel.into(),
            size: player.size.into(),
            aabb: player.aabb().into(),
            facing: player.facing,
            on_ground: player.on_ground,
            on_wall: player.on_wall,
            wall_clinging: player.wall_clinging,
            wall_climbing: player.wall_climbing,
            fast_falling: player.fast_falling,
            fly_enabled: player.fly_enabled,
            dash_charges_available: player.dash_charges_available,
            air_jumps_available: player.air_jumps_available,
            blink_aiming: player.blink_aiming,
            blink_grace_timer: player.blink_grace_timer,
            locomotion: locomotion.into(),
            body_mode: body_mode.into(),
            last_safe_pos: sim_state.last_safe_player_pos.into(),
            time_alive: player.time_alive,
            resets: player.resets,
        },
        controls: controls.into(),
        nearby_collision: nearby_collision(world, player),
        moving_platforms: build_moving_platform_states(&runtime.player, moving_platforms),
    }
}

/// Snapshot all active moving platforms into trace shapes.
fn build_moving_platform_states(
    player: &ae::Player,
    moving_platforms: &[crate::platforms::MovingPlatformState],
) -> Vec<MovingPlatformTraceState> {
    moving_platforms
        .iter()
        .map(|p| {
            let aabb = p.aabb();
            let player_distance = (player.pos - p.pos).length();
            MovingPlatformTraceState {
                pos: p.pos.into(),
                size: p.size.into(),
                aabb: aabb.into(),
                direction: p.direction(),
                player_riding: p.is_riding(player),
                player_distance,
            }
        })
        .collect()
}

/// Diff the current player+control state against the previous snapshot
/// and synthesize gameplay events. The buffer is the single owner of
/// trace state so this stays alongside `record_frame`.
///
/// Events emitted, in order:
///
/// 1. `RoomTransition` (if `active_area` changed),
/// 2. `Reset` (if `player.resets` increased),
/// 3. `CollisionCorrection` for unexplained position deltas — i.e.
///    deltas larger than what the recent velocity could produce. This
///    catches teleports that aren't covered by `Reset` /
///    `RoomTransition`.
/// 4. `LocomotionChanged`,
/// 5. `Dash`, `DoubleJump`, `Jump` (heuristics from charge / vel deltas),
/// 6. `Blink` start / fail,
/// 7. `Damage` (HP delta),
/// 8. `InputEdge` for newly-pressed buttons.
///
/// The recorder is intentionally a passive observer. Sandbox phases
/// can still push richer events directly via `buffer.push_event` if
/// they have non-state-derivable info (e.g. "pogo missed because
/// target was a non-pogo block"), but the diff gives us a useful
/// timeline without touching every phase helper.
pub(crate) fn synthesize_events_from_diff(
    buffer: &mut GameplayTraceBuffer,
    runtime: &SandboxRuntime,
    hp_current: i32,
    controls: ControlFrame,
    real_dt: f32,
    active_area: &str,
    locomotion: ae::LocomotionState,
    body_mode: ae::BodyMode,
) {
    let Some(prev) = buffer.previous.clone() else {
        return;
    };
    let tick = buffer.tick;
    let player = &runtime.player;
    let cur_pos = player.pos;
    let cur_vel = player.vel;

    let mut suppressed_teleport = false;

    if prev.active_area != active_area {
        buffer.push_event(GameplayTraceEvent::RoomTransition {
            tick,
            from: prev.active_area.clone(),
            to: active_area.into(),
        });
        suppressed_teleport = true;
    }

    if player.resets > prev.resets {
        buffer.push_event(GameplayTraceEvent::Reset { tick });
        suppressed_teleport = true;
    }

    // Position-delta vs velocity-budget check. Catches teleports that
    // aren't covered by Reset / RoomTransition. This is the OOB-debug
    // smoking-gun event: a 1500-px jump in one tick will surface here.
    let dpos = cur_pos - prev.pos;
    let dlen = dpos.length();
    let max_speed = prev.vel.length().max(cur_vel.length());
    let budget = max_speed * real_dt.max(0.0) + TELEPORT_DETECTION_SLACK_PX;
    if !suppressed_teleport && dlen > budget && dlen > TELEPORT_DETECTION_SLACK_PX {
        buffer.push_event(GameplayTraceEvent::CollisionCorrection {
            tick,
            before: prev.pos.into(),
            after: cur_pos.into(),
            reason: format!(
                "unexplained delta {:.1}px (vel-budget {:.1}px)",
                dlen, budget
            ),
        });
    }

    if prev.locomotion != locomotion {
        buffer.push_event(GameplayTraceEvent::PlayerModeChanged {
            tick,
            from: prev.locomotion.label().into(),
            to: locomotion.label().into(),
        });
    }
    if prev.body_mode != body_mode {
        buffer.push_event(GameplayTraceEvent::PlayerModeChanged {
            tick,
            from: format!("body:{}", prev.body_mode.label()),
            to: format!("body:{}", body_mode.label()),
        });
    }

    if player.dash_charges_available < prev.dash_charges_available {
        buffer.push_event(GameplayTraceEvent::Dash { tick });
    }
    if player.air_jumps_available < prev.air_jumps_available {
        buffer.push_event(GameplayTraceEvent::DoubleJump { tick });
    } else if !prev.on_ground && cur_vel.y < prev.vel.y - 50.0 && controls.jump_pressed {
        // Jump-edge heuristic: y velocity went meaningfully more
        // negative (Ambition's screen-space +y is down so upward
        // jumps make vel.y decrease) on a frame where the player
        // pressed jump, while the player was airborne.
        buffer.push_event(GameplayTraceEvent::Jump { tick });
    } else if prev.on_ground && !player.on_ground && controls.jump_pressed && cur_vel.y < 0.0 {
        buffer.push_event(GameplayTraceEvent::Jump { tick });
    }

    if !prev.blink_aiming && player.blink_aiming {
        buffer.push_event(GameplayTraceEvent::Blink {
            tick,
            from: prev.pos.into(),
            to: cur_pos.into(),
            precision: false,
        });
    }
    // Blink-fired heuristic: blink_grace_timer just became positive,
    // which the engine sets after a successful blink commit.
    if prev.blink_grace_timer <= 0.0 && player.blink_grace_timer > 0.0 {
        buffer.push_event(GameplayTraceEvent::Blink {
            tick,
            from: prev.pos.into(),
            to: cur_pos.into(),
            precision: true,
        });
    }

    if hp_current < prev.hp_current {
        let amount = (prev.hp_current - hp_current).max(0);
        buffer.push_event(GameplayTraceEvent::Damage {
            tick,
            source: "feature".into(),
            amount,
        });
        if hp_current <= 0 {
            buffer.push_event(GameplayTraceEvent::Death { tick });
        }
    }

    if controls.attack_pressed && !prev.controls.attack_pressed {
        buffer.push_event(GameplayTraceEvent::Attack {
            tick,
            kind: "slash".into(),
        });
    }
    if controls.pogo_pressed && !prev.controls.pogo_pressed {
        buffer.push_event(GameplayTraceEvent::Attack {
            tick,
            kind: "pogo".into(),
        });
    }

    // Input edges for the bool fields the player can newly press this
    // frame. We compare against the previous frame's `controls` so a
    // genuine press → release → press in one tick still records the
    // press (the previous frame's value was already false).
    let pairs: &[(&str, bool, bool)] = &[
        ("Jump", controls.jump_pressed, prev.controls.jump_pressed),
        ("Dash", controls.dash_pressed, prev.controls.dash_pressed),
        ("Blink", controls.blink_pressed, prev.controls.blink_pressed),
        ("Up", controls.up_pressed, prev.controls.up_pressed),
        ("Down", controls.down_pressed, prev.controls.down_pressed),
        (
            "Attack",
            controls.attack_pressed,
            prev.controls.attack_pressed,
        ),
        ("Pogo", controls.pogo_pressed, prev.controls.pogo_pressed),
        (
            "Interact",
            controls.interact_pressed,
            prev.controls.interact_pressed,
        ),
        ("Reset", controls.reset_pressed, prev.controls.reset_pressed),
        ("Start", controls.start_pressed, prev.controls.start_pressed),
        (
            "FlyToggle",
            controls.fly_toggle_pressed,
            prev.controls.fly_toggle_pressed,
        ),
        (
            "FastFall",
            controls.fast_fall_pressed,
            prev.controls.fast_fall_pressed,
        ),
    ];
    for (label, cur, prev_v) in pairs {
        if *cur && !*prev_v {
            buffer.push_event(GameplayTraceEvent::InputEdge {
                tick,
                action: (*label).into(),
            });
        }
    }
}

/// Push the constructed frame into the buffer and (if not already armed)
/// auto-request an OOB dump.
pub fn record_frame(
    buffer: &mut GameplayTraceBuffer,
    frame: GameplayTraceFrame,
    oob: Option<&OobReason>,
) {
    if let Some(reason) = oob {
        let label = reason.short_label();
        buffer.push_event(GameplayTraceEvent::OobDetected {
            tick: buffer.tick,
            reason: label.clone(),
            pos: frame.player.pos,
        });
        if buffer.auto_dump_armed && buffer.dump_request.is_none() {
            buffer.dump_request = Some(DumpReason::OobAuto { reason: label });
            buffer.auto_dump_armed = false;
        }
    } else if !buffer.auto_dump_armed {
        // Player returned to a healthy state; rearm so a future OOB
        // re-fires.
        buffer.auto_dump_armed = true;
    }
    buffer.push_frame(frame);
}

/// Replace the diff snapshot with the just-recorded frame's state.
/// Caller drives this after `record_simulation_frame` so the next
/// tick's `synthesize_events_from_diff` sees an up-to-date baseline.
pub(crate) fn update_previous_snapshot(
    buffer: &mut GameplayTraceBuffer,
    runtime: &SandboxRuntime,
    hp_current: i32,
    controls: ControlFrame,
    active_area: &str,
    locomotion: ae::LocomotionState,
    body_mode: ae::BodyMode,
) {
    let player = &runtime.player;
    buffer.previous = Some(PreviousFrameSnapshot {
        pos: player.pos,
        vel: player.vel,
        on_ground: player.on_ground,
        fly_enabled: player.fly_enabled,
        blink_aiming: player.blink_aiming,
        blink_grace_timer: player.blink_grace_timer,
        fast_falling: player.fast_falling,
        dash_charges_available: player.dash_charges_available,
        air_jumps_available: player.air_jumps_available,
        resets: player.resets,
        hp_current,
        locomotion,
        body_mode,
        active_area: active_area.into(),
        controls,
    });
}
