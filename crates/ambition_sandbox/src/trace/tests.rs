use super::*;
use ae::{Block, World};

fn dummy_world() -> World {
    let blocks = vec![Block::solid(
        "floor",
        ae::Vec2::new(0.0, 100.0),
        ae::Vec2::new(200.0, 20.0),
    )];
    World::new(
        "test",
        ae::Vec2::new(200.0, 200.0),
        ae::Vec2::new(50.0, 50.0),
        blocks,
    )
}

fn dummy_player(at: ae::Vec2) -> ae::Player {
    ae::Player::new(at)
}

fn dummy_moving_platform() -> crate::platforms::MovingPlatformState {
    crate::platforms::MovingPlatformState::from_authored(
        ae::Vec2::new(96.0, 80.0),
        ae::Vec2::new(80.0, 12.0),
        48.0,
        30.0,
    )
}

#[test]
fn ring_buffer_caps_at_capacity() {
    let mut buf = GameplayTraceBuffer::with_capacity(4, 4);
    for i in 0..10 {
        buf.push_frame(GameplayTraceFrame {
            seq: i,
            tick: i,
            real_dt: 0.016,
            sim_dt: 0.016,
            time_scale: 1.0,
            game_mode: "Playing".into(),
            active_area: "test".into(),
            world_size: TracePoint::default(),
            world_spawn: TracePoint::default(),
            player: PlayerTraceState {
                pos: TracePoint::default(),
                vel: TracePoint::default(),
                size: TracePoint::default(),
                aabb: TraceAabb::default(),
                facing: 1.0,
                on_ground: false,
                on_wall: false,
                wall_clinging: false,
                wall_climbing: false,
                fast_falling: false,
                fly_enabled: false,
                dash_charges_available: 0,
                air_jumps_available: 0,
                blink_aiming: false,
                blink_grace_timer: 0.0,
                locomotion: "Airborne".into(),
                body_mode: "Standing".into(),
                last_safe_pos: TracePoint::default(),
                time_alive: 0.0,
                resets: 0,
            },
            controls: ControlFrameTrace::default(),
            nearby_collision: Vec::new(),
            moving_platforms: Vec::new(),
        });
    }
    assert_eq!(buf.frame_count(), 4, "wraparound should cap at capacity");
    // Earliest preserved frame should be the 6th pushed (index 6).
    let first = buf.frames.front().expect("non-empty");
    assert_eq!(first.tick, 6);
    let last = buf.frames.back().expect("non-empty");
    assert_eq!(last.tick, 9);
}

#[test]
fn detect_oob_inside_world_returns_none() {
    let world = dummy_world();
    let player = dummy_player(ae::Vec2::new(50.0, 50.0));
    assert!(detect_oob(&player, &world, OOB_MARGIN).is_none());
}

#[test]
fn detect_oob_outside_envelope_x() {
    let world = dummy_world();
    // Place player far to the right of world envelope + margin.
    let player = dummy_player(ae::Vec2::new(2000.0, 50.0));
    match detect_oob(&player, &world, OOB_MARGIN) {
        Some(OobReason::OutsideWorldEnvelope { axis }) => assert_eq!(axis, 'x'),
        other => panic!("expected OutsideWorldEnvelope x, got {other:?}"),
    }
}

#[test]
fn detect_oob_outside_envelope_y() {
    let world = dummy_world();
    let player = dummy_player(ae::Vec2::new(50.0, -2000.0));
    match detect_oob(&player, &world, OOB_MARGIN) {
        Some(OobReason::OutsideWorldEnvelope { axis }) => assert_eq!(axis, 'y'),
        other => panic!("expected OutsideWorldEnvelope y, got {other:?}"),
    }
}

#[test]
fn detect_oob_inside_solid() {
    let world = dummy_world();
    // Floor is at (0,100)-(200,120). Place player center in floor.
    let player = dummy_player(ae::Vec2::new(100.0, 110.0));
    match detect_oob(&player, &world, OOB_MARGIN) {
        Some(OobReason::InsideSolid { block_name }) => assert_eq!(block_name, "floor"),
        other => panic!("expected InsideSolid, got {other:?}"),
    }
}

#[test]
fn detect_oob_position_non_finite() {
    let world = dummy_world();
    let mut player = dummy_player(ae::Vec2::new(50.0, 50.0));
    player.pos = ae::Vec2::new(f32::NAN, 0.0);
    assert!(matches!(
        detect_oob(&player, &world, OOB_MARGIN),
        Some(OobReason::PositionNonFinite)
    ));
}

#[test]
fn detect_oob_absurd_velocity() {
    let world = dummy_world();
    let mut player = dummy_player(ae::Vec2::new(50.0, 50.0));
    player.vel = ae::Vec2::new(1.0e6, 0.0);
    assert!(matches!(
        detect_oob(&player, &world, OOB_MARGIN),
        Some(OobReason::AbsurdVelocity { .. })
    ));
}

#[test]
fn dump_paths_does_not_panic_and_is_unique_per_label() {
    let dir = Path::new("/tmp/nope");
    let (a_json, a_md) = dump_paths(dir, "label_a");
    let (b_json, b_md) = dump_paths(dir, "label_b");
    assert!(a_json.to_string_lossy().ends_with("label_a.json"));
    assert!(a_md.to_string_lossy().ends_with("label_a.md"));
    assert_ne!(a_json, b_json);
    assert_ne!(a_md, b_md);
}

#[test]
fn timestamp_label_changes_with_time() {
    // Construct two SystemTimes one second apart.
    let a = UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    let b = UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_001);
    assert_ne!(timestamp_label(a), timestamp_label(b));
}

#[test]
fn record_frame_with_oob_pushes_event_and_requests_dump() {
    let mut buf = GameplayTraceBuffer::with_capacity(8, 8);
    let world = dummy_world();
    let mut player = dummy_player(ae::Vec2::new(50.0, 50.0));
    player.pos = ae::Vec2::new(2000.0, 50.0); // outside envelope.x
    let frame = build_frame(
        &SandboxRuntime {
            player: player.clone(),
            player_health: ae::Health::new(5),
            debug: false,
            slowmo: false,
            presets: crate::input::KeyboardPreset::presets().to_vec(),
            preset_index: 0,
            preset_flash: 0.0,
            last_safe_player_pos: ae::Vec2::ZERO,
            time_scale: 1.0,
            moving_platforms: vec![dummy_moving_platform()],
            dialogue: crate::dialog::DialogState::default(),
            physics_settings: crate::physics::PhysicsSandboxSettings::default(),
            room_transition_cooldown: 0.0,
            player_attack: None,
            blink_in_timer: 0.0,
            blink_in_duration: 0.0,
            blink_camera_from: ae::Vec2::ZERO,
            blink_camera_to: ae::Vec2::ZERO,
            camera_snap_timer: 0.0,
        },
        &world,
        ControlFrame::default(),
        0.016,
        0.016,
        "Playing",
        "test",
        0,
        0,
        "Airborne",
        "Standing",
    );
    let oob = detect_oob(&player, &world, OOB_MARGIN);
    record_frame(&mut buf, frame, oob.as_ref());
    assert_eq!(buf.frame_count(), 1);
    assert_eq!(buf.event_count(), 1, "OOB event should be pushed");
    assert!(matches!(buf.dump_request, Some(DumpReason::OobAuto { .. })));
}

#[test]
fn write_dump_writes_two_files() {
    let mut buf = GameplayTraceBuffer::with_capacity(4, 4);
    let world = dummy_world();
    let player = dummy_player(ae::Vec2::new(50.0, 50.0));
    let frame = build_frame(
        &SandboxRuntime {
            player: player.clone(),
            player_health: ae::Health::new(5),
            debug: false,
            slowmo: false,
            presets: crate::input::KeyboardPreset::presets().to_vec(),
            preset_index: 0,
            preset_flash: 0.0,
            last_safe_player_pos: ae::Vec2::ZERO,
            time_scale: 1.0,
            moving_platforms: vec![dummy_moving_platform()],
            dialogue: crate::dialog::DialogState::default(),
            physics_settings: crate::physics::PhysicsSandboxSettings::default(),
            room_transition_cooldown: 0.0,
            player_attack: None,
            blink_in_timer: 0.0,
            blink_in_duration: 0.0,
            blink_camera_from: ae::Vec2::ZERO,
            blink_camera_to: ae::Vec2::ZERO,
            camera_snap_timer: 0.0,
        },
        &world,
        ControlFrame::default(),
        0.016,
        0.016,
        "Playing",
        "test",
        0,
        0,
        "Airborne",
        "Standing",
    );
    record_frame(&mut buf, frame, None);
    let dir = std::env::temp_dir().join("ambition_trace_test_dump");
    let _ = std::fs::remove_dir_all(&dir);
    let json_path = write_dump(&buf, &DumpReason::Manual, &dir).expect("write dump");
    assert!(json_path.exists());
    let md_path = json_path.with_extension("md");
    assert!(md_path.exists());
    let json_body = std::fs::read_to_string(&json_path).unwrap();
    assert!(json_body.contains("\"schema_version\": 1"));
    assert!(json_body.contains("\"dump_reason\""));
}

/// P1 — `timestamp_label` calls in quick succession (same nanosecond
/// or not) must produce distinct strings, because the atomic
/// sequence counter is appended.
#[test]
fn timestamp_label_unique_in_tight_loop() {
    let now = SystemTime::now();
    // Use a fixed `ts` so the seconds/nanoseconds segments do not
    // change between calls; the only differentiator left is the
    // atomic sequence.
    let labels: Vec<String> = (0..32).map(|_| timestamp_label(now)).collect();
    let unique: std::collections::HashSet<&String> = labels.iter().collect();
    assert_eq!(
        unique.len(),
        labels.len(),
        "all dump labels in a tight loop must be unique; got {labels:?}"
    );
}

/// P1 — `timestamp_label_with_seq` lets tests pin a sequence value
/// for stable expectations. Two distinct sequences must produce
/// different strings even when `ts` is identical.
#[test]
fn timestamp_label_with_seq_is_stable_per_seq() {
    let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_777_902_031);
    let a = timestamp_label_with_seq(now, 0);
    let b = timestamp_label_with_seq(now, 1);
    assert_ne!(a, b);
    // Same inputs produce same output.
    assert_eq!(a, timestamp_label_with_seq(now, 0));
}

fn make_runtime(_world: &ae::World, player: ae::Player) -> SandboxRuntime {
    SandboxRuntime {
        player,
        player_health: ae::Health::new(5),
        debug: false,
        slowmo: false,
        presets: crate::input::KeyboardPreset::presets().to_vec(),
        preset_index: 0,
        preset_flash: 0.0,
        last_safe_player_pos: ae::Vec2::ZERO,
        time_scale: 1.0,
        moving_platforms: vec![dummy_moving_platform()],
        dialogue: crate::dialog::DialogState::default(),
        physics_settings: crate::physics::PhysicsSandboxSettings::default(),
        room_transition_cooldown: 0.0,
        player_attack: None,
        blink_in_timer: 0.0,
        blink_in_duration: 0.0,
        blink_camera_from: ae::Vec2::ZERO,
        blink_camera_to: ae::Vec2::ZERO,
        camera_snap_timer: 0.0,
    }
}

/// P2 — pressing a button that wasn't pressed last frame should
/// emit an `InputEdge` event. We seed the buffer with an initial
/// snapshot, then call `synthesize_events_from_diff` directly so
/// the test doesn't need a full Bevy App.
#[test]
fn synthesizes_input_edge_event_on_button_press() {
    let mut buf = GameplayTraceBuffer::with_capacity(16, 16);
    let world = dummy_world();
    let runtime = make_runtime(&world, dummy_player(ae::Vec2::new(50.0, 50.0)));
    // Seed previous snapshot with no buttons pressed.
    update_previous_snapshot(
        &mut buf,
        &runtime,
        ControlFrame::default(),
        "test",
        ae::LocomotionState::Grounded,
        ae::BodyMode::Standing,
    );
    // Player starts pressing Jump this frame.
    let mut controls = ControlFrame::default();
    controls.jump_pressed = true;
    synthesize_events_from_diff(
        &mut buf,
        &runtime,
        controls,
        0.016,
        "test",
        ae::LocomotionState::Grounded,
        ae::BodyMode::Standing,
    );
    let edges: Vec<_> = buf
        .events()
        .filter_map(|e| match e {
            GameplayTraceEvent::InputEdge { action, .. } => Some(action.clone()),
            _ => None,
        })
        .collect();
    assert!(
        edges.iter().any(|a| a == "Jump"),
        "expected Jump InputEdge; got {edges:?}"
    );
}

/// P2 — an unexplained position delta (much larger than the velocity
/// budget) should produce a `CollisionCorrection` event so the
/// trace surfaces teleports of the kind that landed in
/// `debug_traces/ambition_trace_1777902031_*.json`.
#[test]
fn synthesizes_collision_correction_on_unexplained_teleport() {
    let mut buf = GameplayTraceBuffer::with_capacity(16, 16);
    let world = dummy_world();
    let runtime_prev = make_runtime(&world, dummy_player(ae::Vec2::new(62.0, 1564.0)));
    update_previous_snapshot(
        &mut buf,
        &runtime_prev,
        ControlFrame::default(),
        "square_arena",
        ae::LocomotionState::WallCling,
        ae::BodyMode::Standing,
    );
    // Now jump to a wildly different position with no plausible
    // velocity to explain it. Same active area + same `resets` so
    // the teleport detector isn't suppressed by Reset/RoomTransition.
    let mut player2 = dummy_player(ae::Vec2::new(62.0, -23.0));
    player2.vel = ae::Vec2::ZERO;
    let runtime_cur = make_runtime(&world, player2);
    synthesize_events_from_diff(
        &mut buf,
        &runtime_cur,
        ControlFrame::default(),
        0.0069,
        "square_arena",
        ae::LocomotionState::Grounded,
        ae::BodyMode::Standing,
    );
    let teleports: Vec<_> = buf
        .events()
        .filter(|e| matches!(e, GameplayTraceEvent::CollisionCorrection { .. }))
        .collect();
    assert_eq!(
        teleports.len(),
        1,
        "expected one CollisionCorrection event for the teleport; got {teleports:?}"
    );
}

/// P2 — incrementing `player.resets` should emit a `Reset` event
/// AND suppress the teleport detector (the player position can
/// legitimately jump to spawn on reset).
#[test]
fn reset_emits_event_and_suppresses_teleport_event() {
    let mut buf = GameplayTraceBuffer::with_capacity(16, 16);
    let world = dummy_world();
    let runtime_prev = make_runtime(&world, dummy_player(ae::Vec2::new(50.0, 50.0)));
    update_previous_snapshot(
        &mut buf,
        &runtime_prev,
        ControlFrame::default(),
        "test",
        ae::LocomotionState::Grounded,
        ae::BodyMode::Standing,
    );
    let mut player2 = dummy_player(ae::Vec2::new(150.0, 150.0));
    player2.resets = runtime_prev.player.resets + 1;
    let runtime_cur = make_runtime(&world, player2);
    synthesize_events_from_diff(
        &mut buf,
        &runtime_cur,
        ControlFrame::default(),
        0.016,
        "test",
        ae::LocomotionState::Grounded,
        ae::BodyMode::Standing,
    );
    let resets: Vec<_> = buf
        .events()
        .filter(|e| matches!(e, GameplayTraceEvent::Reset { .. }))
        .collect();
    assert_eq!(resets.len(), 1, "expected one Reset event");
    let teleports: Vec<_> = buf
        .events()
        .filter(|e| matches!(e, GameplayTraceEvent::CollisionCorrection { .. }))
        .collect();
    assert!(
        teleports.is_empty(),
        "Reset should suppress the teleport detector"
    );
}

/// P3 — frame snapshots include a populated `moving_platforms` slot
/// with the active sandbox platform.
#[test]
fn frame_includes_moving_platform_state() {
    let world = dummy_world();
    let player = dummy_player(ae::Vec2::new(50.0, 50.0));
    let runtime = make_runtime(&world, player);
    let frame = build_frame(
        &runtime,
        &world,
        ControlFrame::default(),
        0.016,
        0.016,
        "Playing",
        "test",
        0,
        0,
        "Grounded",
        "Standing",
    );
    assert_eq!(
        frame.moving_platforms.len(),
        1,
        "expected one moving-platform entry per frame"
    );
    let platform = &frame.moving_platforms[0];
    assert!(platform.size.x > 0.0);
    assert!(platform.size.y > 0.0);
    assert!(platform.player_distance > 0.0);
}

/// P4 — `BodyMode::from_player` reads `player.body_mode` (the
/// authoritative field). Default is `Standing`; setting the field
/// changes what the recorder/HUD see.
#[test]
fn body_mode_reads_authoritative_field() {
    let mut player = dummy_player(ae::Vec2::ZERO);
    assert_eq!(ae::BodyMode::from_player(&player), ae::BodyMode::Standing);
    player.body_mode = ae::BodyMode::MorphBall;
    assert_eq!(ae::BodyMode::from_player(&player), ae::BodyMode::MorphBall);
}
