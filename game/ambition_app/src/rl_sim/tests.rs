use super::*;
use ambition::input::ControlFrame;

#[test]
fn sim_constructs_and_returns_initial_observation() {
    let mut sim = SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sim builds");
    let obs = sim.observation();
    assert_eq!(obs.tick, 0, "fresh sim is at tick 0");
    assert!(obs.alive(), "spawned player is alive");
    assert!(obs.hp_max > 0, "max hp populated from data");
    assert!(!obs.active_room.is_empty(), "active room id populated");
}

#[test]
fn idle_step_advances_tick_without_panicking() {
    let mut sim = SandboxSim::new().expect("sim builds");
    let obs = sim.step(AgentAction::default());
    assert_eq!(obs.tick, 1);
}

#[test]
fn step_n_holds_action_across_frames() {
    let mut sim = SandboxSim::new().expect("sim builds");
    let obs = sim.step_n(AgentAction::default(), 30);
    assert_eq!(obs.tick, 30);
    // 30 idle frames should not have killed the player.
    assert!(obs.alive(), "30 idle frames don't kill the player");
}

#[test]
fn step_with_reward_returns_finite_positive_reward_while_surviving() {
    let mut sim = SandboxSim::new().expect("sim builds");
    let mut total = 0.0_f32;
    for _ in 0..30 {
        let (obs, reward) = sim.step_with_reward(AgentAction::default());
        assert!(reward.is_finite(), "reward must be finite, got {reward}");
        assert!(obs.alive(), "30 idle frames don't kill the player");
        total += reward;
    }
    // No deaths, so the survival + health terms keep the cumulative
    // reward positive across a calm idle episode.
    assert!(
        total > 0.0,
        "an alive idle episode should accumulate positive shaped reward, got {total}"
    );
}

#[test]
fn move_action_translates_to_horizontal_velocity() {
    let mut sim = SandboxSim::new().expect("sim builds");
    // 10 frames of "walk right". Velocity should pick up positive x;
    // exact magnitude depends on movement tuning so we only assert
    // the sign.
    let obs = sim.step_n(AgentAction::move_x(1.0), 10);
    assert!(
        obs.player_vel.0 > 0.0,
        "after 10 frames of walk-right, vel.x should be positive (got {})",
        obs.player_vel.0
    );
}

#[test]
fn agent_action_to_control_frame_preserves_axes() {
    let action = AgentAction {
        move_x: 0.7,
        move_y: -0.3,
        jump: true,
        ..AgentAction::default()
    };
    let frame: ControlFrame = action.into();
    assert!((frame.axis_x - 0.7).abs() < f32::EPSILON);
    assert!((frame.axis_y + 0.3).abs() < f32::EPSILON);
    assert!(frame.jump_pressed);
    assert!(!frame.jump_held);
}

#[test]
fn fixed_timestep_produces_deterministic_trajectory() {
    // Two sims, same fixed timestep, same action sequence: their
    // player positions must match exactly at every step. This is
    // the foundation for replay debugging and RL training.
    let actions = [
        AgentAction::move_x(1.0),
        AgentAction::jump(),
        AgentAction::move_x(1.0),
        AgentAction::move_x(1.0),
        AgentAction::default(),
        AgentAction::move_x(-1.0),
        AgentAction::move_x(-1.0),
        AgentAction {
            dash: true,
            move_x: -1.0,
            ..AgentAction::default()
        },
        AgentAction::default(),
        AgentAction::default(),
    ];

    let mut sim_a = SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).unwrap();
    let mut sim_b = SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).unwrap();
    for (i, action) in actions.iter().enumerate() {
        let a = sim_a.step(*action);
        let b = sim_b.step(*action);
        assert_eq!(
            a.player_pos, b.player_pos,
            "tick {i}: positions diverged ({:?} vs {:?})",
            a.player_pos, b.player_pos
        );
        assert_eq!(
            a.player_vel, b.player_vel,
            "tick {i}: velocities diverged ({:?} vs {:?})",
            a.player_vel, b.player_vel
        );
        assert_eq!(a.hp, b.hp, "tick {i}: HP diverged ({} vs {})", a.hp, b.hp);
    }
}

#[test]
fn timestep_setter_round_trips() {
    let mut sim = SandboxSim::new().unwrap();
    assert!(matches!(sim.timestep(), TimestepMode::WallClock));
    sim.set_timestep(TimestepMode::fixed_144hz());
    assert!(matches!(
        sim.timestep(),
        TimestepMode::Fixed { dt } if (dt - 1.0 / 144.0).abs() < 1e-6
    ));
}

#[test]
fn observation_hp_fraction_handles_default() {
    let obs = AgentObservation {
        tick: 0,
        player_pos: (0.0, 0.0),
        player_vel: (0.0, 0.0),
        player_size: (16.0, 32.0),
        on_ground: false,
        on_wall: false,
        wall_clinging: false,
        wall_climbing: false,
        facing: 1.0,
        fast_falling: false,
        fly_enabled: false,
        gliding: false,
        dash_charges: 0,
        air_jumps: 0,
        blink_aiming: false,
        hp: 10,
        hp_max: 20,
        mana: 0,
        mana_max: 100,
        time_alive: 0.0,
        resets: 0,
        body_mode: "Standing".to_string(),
        active_room: "test".to_string(),
        world_size: (256.0, 256.0),
        world_spawn: (0.0, 0.0),
        last_safe_pos: (0.0, 0.0),
        recently_damaged: false,
        in_hitstun: false,
        invincible: false,
        in_water: false,
        water_kind: None,
        water_submersion: 0.0,
        on_climbable: false,
        climbable_kind: None,
        gravity_dir: (0.0, 1.0),
        enemies: vec![],
        pickups: vec![],
    };
    assert!((obs.hp_fraction() - 0.5).abs() < f32::EPSILON);
    assert!(obs.alive());
}

#[test]
fn sim_can_start_in_a_specific_room_via_options() {
    // Build a sim explicitly starting in goblin_encounter. The default
    // start room is central_hub_complex, so a successful override
    // should change `active_room`.
    let mut default_sim = SandboxSim::new().expect("default builds");
    let default_room = default_sim.observation().active_room.clone();

    let mut goblin_encounter_sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("goblin_encounter"),
    )
    .expect("goblin_encounter override builds");
    let active = goblin_encounter_sim.observation().active_room.clone();
    assert_ne!(
        active, default_room,
        "start_room override should change the active room from the default"
    );
    assert_eq!(
        active, "goblin_encounter",
        "expected to start in goblin_encounter"
    );
}

#[test]
fn unknown_start_room_does_not_panic_or_error() {
    // Per app.rs's resolution: an unknown start room id prints a
    // warning and falls back to the LDtk-authored start. The sim
    // still constructs cleanly.
    let mut sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("definitely_not_a_real_room"),
    )
    .expect("unknown start room should not error");
    assert!(!sim.observation().active_room.is_empty());
}

#[test]
fn observation_reports_no_water_no_climbable_in_default_spawn() {
    let mut sim = SandboxSim::new().expect("sim builds");
    let obs = sim.observation();
    // central_hub_complex spawn has neither water nor climbables.
    assert!(!obs.in_water, "default spawn should not be in water");
    assert_eq!(obs.water_submersion, 0.0);
    assert!(obs.water_kind.is_none());
    assert!(!obs.on_climbable);
    assert!(obs.climbable_kind.is_none());
}
