//! OOB/anomaly detection + frame-snapshot construction for the trace recorder.
//!
//! Pure functions that classify a player frame: out-of-bounds reasons
//! (`detect_oob_*`, delegating envelope/inside-solid checks to
//! `ae::classify_player_safety`), teleport/collision-correction synthesis from
//! frame diffs, and `build_frame`/`record_frame` snapshot assembly.

use super::*;

/// Inspect the current player state against the active world and produce
/// the *first* OOB reason found, if any. Order matters: NaN/inf should
/// be reported before "outside envelope" because both can be true.
///
/// The world envelope / inside-solid check is delegated to
/// `ae::classify_player_safety` so the trace recorder and
/// `crate::remember_safe_player_position` use the same definition.
/// The recorder layers the trace-only "absurd velocity" rule on top.
pub fn detect_oob_scratch(
    scratch: &ae::BodyClusterScratch,
    world: &ae::World,
    margin: f32,
) -> Option<OobReason> {
    detect_oob_from_kinematics(
        scratch.kinematics.pos,
        scratch.kinematics.vel,
        scratch.kinematics.aabb(),
        world,
        margin,
    )
}

/// Produce the first OOB reason the player kinematics + world
/// geometry imply (if any). Takes pos / vel / AABB directly so the
/// live trace recorder can call it from cluster components.
pub fn detect_oob_from_kinematics(
    pos: ae::Vec2,
    vel: ae::Vec2,
    aabb: ae::Aabb,
    world: &ae::World,
    margin: f32,
) -> Option<OobReason> {
    let speed = vel.length();
    if speed.is_finite() && speed > ABSURD_VELOCITY_MAGNITUDE {
        return Some(OobReason::AbsurdVelocity { magnitude: speed });
    }
    match ae::classify_safety_from_kinematics(pos, vel, aabb, world, margin, |b| {
        matches!(b.kind, ae::BlockKind::Solid)
    }) {
        ae::PlayerSafetyVerdict::Safe => None,
        ae::PlayerSafetyVerdict::PositionNonFinite => Some(OobReason::PositionNonFinite),
        ae::PlayerSafetyVerdict::VelocityNonFinite => Some(OobReason::VelocityNonFinite),
        ae::PlayerSafetyVerdict::OutsideWorldEnvelope { axis } => {
            Some(OobReason::OutsideWorldEnvelope { axis })
        }
        ae::PlayerSafetyVerdict::InsideSolid => {
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

/// Collect blocks whose AABB is within `radius` px of `body` (player
/// AABB) — used by `CollisionCorrection` event enrichment so the trace
/// shows exactly which wall/edge a snap aligned with. Distance is
/// box-to-box rather than centre-to-centre so an edge-touching wall
/// (distance = 0) sorts first.
fn nearby_collision_around(
    world: &ae::World,
    body: ae::Aabb,
    radius: f32,
) -> Vec<CollisionTraceShape> {
    use ambition_engine_core::AabbExt;
    let mut hits: Vec<CollisionTraceShape> = world
        .blocks
        .iter()
        .map(|block| {
            let bx = block.aabb;
            let dx = (body.left() - bx.right())
                .max(bx.left() - body.right())
                .max(0.0);
            let dy = (body.top() - bx.bottom())
                .max(bx.top() - body.bottom())
                .max(0.0);
            let distance = (dx * dx + dy * dy).sqrt();
            CollisionTraceShape {
                kind: format!("{:?}", block.kind),
                name: block.name.clone(),
                aabb: bx.into(),
                distance,
            }
        })
        .filter(|shape| shape.distance < radius)
        .collect();
    hits.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(MAX_NEARBY_COLLISION);
    hits
}

/// Diff important player flags against the previous frame and emit
/// `"<field>: <prev>→<curr>"` strings for any that flipped this tick.
/// Used by the `CollisionCorrection` event so a teleport trace shows
/// state transitions that coincide with the snap — most notably
/// `ledge_grabbing: false→true` (the canonical "I just grabbed a
/// ledge and snapped to its anchor" attribution).
fn collect_state_flips(
    prev: &PreviousFrameSnapshot,
    clusters: &ae::BodyClustersMut<'_>,
) -> Vec<String> {
    let mut flips = Vec::new();
    let cur_ledge = clusters.ledge.grab.is_some();
    if cur_ledge != prev.ledge_grabbing {
        flips.push(format!(
            "ledge_grabbing: {} → {}",
            prev.ledge_grabbing, cur_ledge
        ));
    }
    if clusters.flight.fly_enabled != prev.fly_enabled {
        flips.push(format!(
            "fly_enabled: {} → {}",
            prev.fly_enabled, clusters.flight.fly_enabled
        ));
    }
    if clusters.wall.on_wall != prev.on_wall {
        flips.push(format!(
            "on_wall: {} → {}",
            prev.on_wall, clusters.wall.on_wall
        ));
    }
    if (clusters.wall.wall_normal_x - prev.wall_normal_x).abs() > 1.0e-3 {
        flips.push(format!(
            "wall_normal_x: {:+.0} → {:+.0}",
            prev.wall_normal_x, clusters.wall.wall_normal_x
        ));
    }
    if clusters.ground.on_ground != prev.on_ground {
        flips.push(format!(
            "on_ground: {} → {}",
            prev.on_ground, clusters.ground.on_ground
        ));
    }
    flips
}

fn nearby_collision(world: &ae::World, player_pos: ae::Vec2) -> Vec<CollisionTraceShape> {
    let center = player_pos;
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
/// can call it once per player tick.
#[allow(clippy::too_many_arguments)]
pub fn build_frame(
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
    seq: u64,
    tick: u64,
    moving_platforms: &[crate::world::platforms::MovingPlatformState],
    locomotion: &str,
    body_mode: &str,
) -> GameplayTraceFrame {
    GameplayTraceFrame {
        seq,
        tick,
        real_dt,
        sim_dt,
        time_scale: clock.time_scale,
        game_mode: game_mode.into(),
        active_area: active_area.into(),
        world_size: world.size.into(),
        world_spawn: world.spawn.into(),
        player: PlayerTraceState {
            pos: clusters.kinematics.pos.into(),
            vel: clusters.kinematics.vel.into(),
            size: clusters.kinematics.size.into(),
            aabb: clusters.kinematics.aabb().into(),
            facing: clusters.kinematics.facing,
            on_ground: clusters.ground.on_ground,
            on_wall: clusters.wall.on_wall,
            wall_clinging: clusters.wall.wall_clinging,
            wall_climbing: clusters.wall.wall_climbing,
            fast_falling: clusters.flight.fast_falling,
            fly_enabled: clusters.flight.fly_enabled,
            dash_charges_available: clusters.dash.charges_available,
            air_jumps_available: clusters.jump.air_jumps_available,
            blink_aiming: clusters.blink.aiming,
            blink_grace_timer: clusters.blink.grace_timer,
            locomotion: locomotion.into(),
            body_mode: body_mode.into(),
            last_safe_pos: safety.last_safe_pos.into(),
            time_alive: clusters.lifetime.time_alive,
            resets: clusters.lifetime.resets,
            wall_normal_x: clusters.wall.wall_normal_x,
            ledge_grabbing: clusters.ledge.grab.is_some(),
            attacking: combat.attacking,
            hitstun_timer: combat.hitstun_timer,
            damage_invuln_timer: combat.damage_invuln_timer,
            attack_ability_enabled: clusters.abilities.abilities.attack,
        },
        controls: controls.into(),
        nearby_collision: nearby_collision(world, clusters.kinematics.pos),
        moving_platforms: build_moving_platform_states(clusters, moving_platforms),
    }
}

/// Snapshot all active moving platforms into trace shapes.
fn build_moving_platform_states(
    clusters: &ae::BodyClustersMut<'_>,
    moving_platforms: &[crate::world::platforms::MovingPlatformState],
) -> Vec<MovingPlatformTraceState> {
    let player_pos = clusters.kinematics.pos;
    let player_aabb = clusters.kinematics.aabb();
    let on_ground = clusters.ground.on_ground;
    moving_platforms
        .iter()
        .map(|p| {
            let aabb = p.aabb();
            let player_distance = (player_pos - p.pos).length();
            MovingPlatformTraceState {
                pos: p.pos.into(),
                size: p.size.into(),
                aabb: aabb.into(),
                direction: p.direction(),
                player_riding: p.is_riding(player_aabb, on_ground),
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
    clusters: &ae::BodyClustersMut<'_>,
    hp_current: i32,
    controls: ControlFrame,
    real_dt: f32,
    active_area: &str,
    locomotion: ae::LocomotionState,
    body_mode: ae::BodyMode,
    world: &ae::World,
) {
    let Some(prev) = buffer.previous.clone() else {
        return;
    };
    let tick = buffer.tick;
    let cur_pos = clusters.kinematics.pos;
    let cur_vel = clusters.kinematics.vel;

    // An intentional teleport (e.g. a portal jump) is not an anomaly — don't let
    // the position-delta check auto-dump while the portal suppression window is
    // open. The window is decremented once per frame in `record_frame`.
    let mut suppressed_teleport = buffer.teleport_suppress_ticks > 0;

    if prev.active_area != active_area {
        buffer.push_event(GameplayTraceEvent::RoomTransition {
            tick,
            from: prev.active_area.clone(),
            to: active_area.into(),
        });
        suppressed_teleport = true;
    }

    if clusters.lifetime.resets > prev.resets {
        buffer.push_event(GameplayTraceEvent::Reset { tick });
        suppressed_teleport = true;
    }

    // A blink is an intentional same-room teleport (its grace timer starts the
    // frame it fires), so the resulting big position delta is expected -- don't
    // let the velocity-budget check auto-dump on every blink.
    if prev.blink_grace_timer <= 0.0 && clusters.blink.grace_timer > 0.0 {
        suppressed_teleport = true;
    }

    // Position-delta vs velocity-budget check. Catches teleports that
    // aren't covered by Reset / RoomTransition.
    let dpos = cur_pos - prev.pos;
    let dlen = dpos.length();
    let max_speed = prev.vel.length().max(cur_vel.length());
    let budget = max_speed * real_dt.max(0.0) + TELEPORT_DETECTION_SLACK_PX;
    if !suppressed_teleport && dlen > budget && dlen > TELEPORT_DETECTION_SLACK_PX {
        let nearby_after = nearby_collision_around(world, clusters.kinematics.aabb(), 64.0);
        let state_flips = collect_state_flips(&prev, clusters);
        let reason = format!("unexplained delta {dlen:.1}px (vel-budget {budget:.1}px)");
        buffer.push_event(GameplayTraceEvent::CollisionCorrection {
            tick,
            before: prev.pos.into(),
            after: cur_pos.into(),
            reason: reason.clone(),
            nearby_after,
            state_flips,
        });
        // Auto-dump the ring buffer NOW, while the pre-teleport frames are
        // still in it. The OOB auto-dump misses teleports that land inside
        // `OOB_MARGIN` (the lock-wall snap to y=-23 is only ~46px OOB), so
        // by the time a human dumps manually the ring holds only the stuck
        // aftermath. `request_dump` no-ops if a dump is already pending, so
        // a ping-pong yields one dump per flush cycle, not per frame.
        // The CollisionCorrection event above is always recorded; only the
        // auto-DUMP waits for warm-up context (spawn-settling shouldn't dump).
        if buffer.has_min_context() {
            buffer.request_dump(DumpReason::TeleportAuto { reason });
        }
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

    if clusters.dash.charges_available < prev.dash_charges_available {
        buffer.push_event(GameplayTraceEvent::Dash { tick });
    }
    if clusters.jump.air_jumps_available < prev.air_jumps_available {
        buffer.push_event(GameplayTraceEvent::DoubleJump { tick });
    } else if !prev.on_ground && cur_vel.y < prev.vel.y - 50.0 && controls.jump_pressed {
        buffer.push_event(GameplayTraceEvent::Jump { tick });
    } else if prev.on_ground
        && !clusters.ground.on_ground
        && controls.jump_pressed
        && cur_vel.y < 0.0
    {
        buffer.push_event(GameplayTraceEvent::Jump { tick });
    }

    if !prev.blink_aiming && clusters.blink.aiming {
        buffer.push_event(GameplayTraceEvent::Blink {
            tick,
            from: prev.pos.into(),
            to: cur_pos.into(),
            precision: false,
        });
    }
    if prev.blink_grace_timer <= 0.0 && clusters.blink.grace_timer > 0.0 {
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
        // Suppress the OOB auto-dump during a portal-transit window: a crossing
        // lands the player at the exit before the exit-side carve opens, so it
        // momentarily reads as inside-solid — that is not a stuck-body anomaly.
        if buffer.auto_dump_armed
            && buffer.dump_request.is_none()
            && buffer.teleport_suppress_ticks == 0
            && buffer.has_min_context()
        {
            buffer.dump_request = Some(DumpReason::OobAuto { reason: label });
            buffer.auto_dump_armed = false;
        }
    } else if !buffer.auto_dump_armed {
        // Player returned to a healthy state; rearm so a future OOB
        // re-fires.
        buffer.auto_dump_armed = true;
    }
    // Tick down the portal-transit suppression window once per recorded frame.
    buffer.teleport_suppress_ticks = buffer.teleport_suppress_ticks.saturating_sub(1);
    buffer.push_frame(frame);
}

/// Replace the diff snapshot with the just-recorded frame's state.
/// Caller drives this after `record_simulation_frame` so the next
/// tick's `synthesize_events_from_diff` sees an up-to-date baseline.
pub(crate) fn update_previous_snapshot(
    buffer: &mut GameplayTraceBuffer,
    clusters: &ae::BodyClustersMut<'_>,
    hp_current: i32,
    controls: ControlFrame,
    active_area: &str,
    locomotion: ae::LocomotionState,
    body_mode: ae::BodyMode,
) {
    buffer.previous = Some(PreviousFrameSnapshot {
        pos: clusters.kinematics.pos,
        vel: clusters.kinematics.vel,
        on_ground: clusters.ground.on_ground,
        fly_enabled: clusters.flight.fly_enabled,
        blink_aiming: clusters.blink.aiming,
        blink_grace_timer: clusters.blink.grace_timer,
        fast_falling: clusters.flight.fast_falling,
        dash_charges_available: clusters.dash.charges_available,
        air_jumps_available: clusters.jump.air_jumps_available,
        resets: clusters.lifetime.resets,
        hp_current,
        locomotion,
        body_mode,
        active_area: active_area.into(),
        controls,
        ledge_grabbing: clusters.ledge.grab.is_some(),
        wall_normal_x: clusters.wall.wall_normal_x,
        on_wall: clusters.wall.on_wall,
    });
}
