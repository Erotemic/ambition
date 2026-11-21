//! Camera policies run on the real Mary-o host and observe its native movement.
use ambition_platformer2d::{
    engine_core as ae,
    extension::ExtensionGeneration,
    input::ControlFrame,
    platformer::markers::PrimaryPlayer,
    sim_view::{
        camera_policy::CameraPolicyState,
        camera_snapshot::{CameraSnapshot2d, ResolvedCameraSnapshot},
    },
};
use bevy::prelude::*;

fn boot(files: Vec<std::path::PathBuf>) -> (App, Entity) {
    boot_room(files, "mary_o_1_2")
}

fn boot_room(files: Vec<std::path::PathBuf>, room: &str) -> (App, Entity) {
    let mut app = ambition_demo_mary_o_app::build_demo_app_loading(files);
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(room.into()));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..90 {
        app.update();
    }
    let body = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryPlayer>>()
        .single(app.world())
        .unwrap();
    (app, body)
}
fn pin(app: &mut App, body: Entity, at: Vec2) {
    let world = app.world_mut();
    let mut q = world.query::<(
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
    )>();
    let (mut item, mut model) = q.get_mut(world, body).unwrap();
    ae::movement::transit_body(
        &mut model,
        &mut item.as_clusters_mut(),
        at,
        ae::movement::TransitVelocity::Zero,
    );
}
fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}
fn camera(app: &mut App) -> CameraSnapshot2d {
    let w = app.world_mut();
    for policy in w.query::<&CameraPolicyState>().iter(w) {
        assert!(policy.fault.is_none(), "{:?}", policy.fault);
    }
    w.query::<&ResolvedCameraSnapshot>()
        .iter(w)
        .find_map(|view| view.0.as_ref().map(|frame| frame.snapshot.clone()))
        .expect("camera frame")
}
fn ride_up(app: &mut App, body: Entity) {
    let w = app.world_mut();
    let platforms = w
        .query::<&ambition_platformer2d::world::collision::MovingPlatformSet>()
        .single(w)
        .unwrap();
    let platform = platforms
        .0
        .iter()
        .filter(|p| p.id.contains("lift_up") && p.aabb().min.y > 300.0 && p.aabb().min.y < 600.0)
        .min_by(|a, b| a.aabb().min.y.total_cmp(&b.aabb().min.y))
        .unwrap()
        .aabb();
    let half = app
        .world()
        .get::<ae::BodyKinematics>(body)
        .unwrap()
        .size()
        .y
        * 0.5;
    pin(
        app,
        body,
        Vec2::new(
            (platform.min.x + platform.max.x) * 0.5,
            platform.min.y - half,
        ),
    );
}
fn roof_entry(app: &mut App, body: Entity) {
    ride_up(app, body);
    let mut saw_airborne_upper = false;
    let mut saw_roof_landing = false;
    for tick in 0..150 {
        step(
            app,
            ControlFrame {
                axis_x: if (23..68).contains(&tick) { 1.0 } else { 0.0 },
                right_pressed: (23..68).contains(&tick),
                jump_pressed: tick == 23,
                jump_held: (23..38).contains(&tick),
                ..Default::default()
            },
        );
        let kin = app.world().get::<ae::BodyKinematics>(body).unwrap();
        let ground = app
            .world()
            .get::<ae::BodyGroundState>(body)
            .unwrap()
            .on_ground;
        let feet = kin.pos.y + kin.size().y * 0.5;
        let on_roof = kin.pos.x > 4524.0 && (feet - 256.0).abs() < 0.01;
        let above = feet <= 256.0;
        let snapshot = camera(app);
        if !ground && above {
            assert_eq!(
                snapshot.active_camera_zone.as_deref(),
                Some("mary_o_1_2_upper")
            );
            assert!((snapshot.target_world.y - 240.0).abs() < 0.001);
            saw_airborne_upper = true;
        }
        if ground && on_roof {
            assert!(saw_airborne_upper, "the upper view starts before landing");
            assert!(
                (snapshot.target_world.y - 240.0).abs() < 0.001,
                "landing does not change the composition goal"
            );
            saw_roof_landing = true;
        }
    }
    assert!(saw_airborne_upper && saw_roof_landing);
}
#[test]
fn entering_the_roof_in_air_and_jumping_on_it_are_distinct_camera_events() {
    let (mut app, body) = boot(vec![]);
    roof_entry(&mut app, body);
    let standing = camera(&mut app).center_world.y;
    for tick in 0..90 {
        step(
            &mut app,
            ControlFrame {
                jump_pressed: tick == 0,
                jump_held: tick < 15,
                ..Default::default()
            },
        );
        assert!(
            (camera(&mut app).center_world.y - standing).abs() < 0.01,
            "a roof hop retains its composition"
        );
    }
}
#[test]
fn a_rising_platform_drives_a_frame_clock_transition() {
    let (mut app, body) = boot(vec![]);
    ride_up(&mut app, body);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 120.0),
    ));
    let mut previous_y = f32::NAN;
    let mut sampled_between_ticks = false;
    for _ in 0..260 {
        app.update();
        let y = app.world().get::<ae::BodyKinematics>(body).unwrap().pos.y;
        let snap = camera(&mut app);
        let w = app.world_mut();
        let ease = w
            .query::<&ambition_platformer2d::camera_presentation::CameraEaseState>()
            .single(w)
            .unwrap();
        if snap.active_camera_zone.as_deref() == Some("mary_o_1_2_upper")
            && ease.transition_active
            && previous_y == y
        {
            assert!(ease.transition_elapsed > 0.0);
            sampled_between_ticks = true;
        }
        previous_y = y;
    }
    assert!(
        sampled_between_ticks,
        "the transition advances between simulation ticks"
    );
}

#[test]
fn an_ordinary_room_keeps_grounded_follow_during_a_jump() {
    let (mut app, body) = boot_room(vec![], "mary_o_1_1");
    let before = camera(&mut app);
    let mut airborne = 0;
    for frame in 0..60 {
        step(
            &mut app,
            ControlFrame {
                jump_pressed: frame == 0,
                jump_held: frame < 12,
                ..Default::default()
            },
        );
        let supported = app
            .world()
            .get::<ae::BodyGroundState>(body)
            .unwrap()
            .on_ground;
        let snapshot = camera(&mut app);
        if !supported {
            airborne += 1;
            assert!((snapshot.target_world.y - before.target_world.y).abs() < 0.01);
        }
    }
    assert!(airborne > 0);
}

struct ScriptFile(std::path::PathBuf);
impl ScriptFile {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ambition-camera-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ambition.camera.rhai");
        std::fs::write(
            &path,
            include_str!("../../../crates/ambition_world_port/examples/ambition.camera.rhai"),
        )
        .unwrap();
        Self(path)
    }
}
impl Drop for ScriptFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        if let Some(dir) = self.0.parent() {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

#[test]
fn rhai_camera_reload_changes_the_view_without_changing_gameplay_generation() {
    let file = ScriptFile::new();
    let (mut app, body) = boot(vec![file.0.clone()]);
    let generation = app.world().resource::<ExtensionGeneration>().0.clone();
    roof_entry(&mut app, body);
    let before = camera(&mut app).target_world.y;
    let changed = include_str!("../../../crates/ambition_world_port/examples/ambition.camera.rhai")
        .replace("response_hz: 7.0", "response_hz: 7.0, target_y: 280.0");
    std::fs::write(&file.0, &changed).unwrap();
    for _ in 0..150 {
        app.update();
    }
    assert_eq!(app.world().resource::<ExtensionGeneration>().0, generation);
    assert!(
        app.world().get_entity(body).is_ok(),
        "a camera edit retains the live body"
    );
    assert!((camera(&mut app).target_world.y - 280.0).abs() < 0.001);
    assert!(
        (camera(&mut app).center_world.y - before).abs() > 20.0,
        "the loaded edit changes the visible frame"
    );
    std::fs::write(
        &file.0,
        changed.replace("target_y: 280.0", "target_y: 280.0, zoom: -1.0"),
    )
    .unwrap();
    for _ in 0..150 {
        app.update();
    }
    let last_records = {
        let w = app.world_mut();
        let state = w.query::<&CameraPolicyState>().single(w).unwrap();
        assert!(state.fault.is_some());
        assert_eq!(state.last_intent.as_ref().unwrap().target_y, Some(280.0));
        state.records.clone()
    };
    for _ in 0..30 {
        app.update();
    }
    {
        let w = app.world_mut();
        let state = w.query::<&CameraPolicyState>().single(w).unwrap();
        assert_eq!(
            state.records, last_records,
            "invalid output does not commit view state"
        );
    }
    std::fs::write(&file.0, &changed).unwrap();
    for _ in 0..150 {
        app.update();
    }
    camera(&mut app);
    use ambition_platformer2d::platformer::schedule::GameMode;
    app.world_mut()
        .resource_mut::<NextState<GameMode>>()
        .set(GameMode::Paused);
    app.update();
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::time::WorldTime>()
            .sim_dt(),
        0.0
    );
    let elapsed = |app: &mut App| {
        let w = app.world_mut();
        let state = w.query::<&CameraPolicyState>().single(w).unwrap();
        state.records.records.records()[0]
            .record
            .get(ambition_platformer2d::extension::sdk::FieldRef(1))
            .unwrap()
            .as_f32()
            .unwrap()
    };
    let paused = elapsed(&mut app);
    for _ in 0..30 {
        app.update();
    }
    assert!(
        elapsed(&mut app) > paused,
        "presentation time advances while gameplay is paused"
    );
}

#[test]
#[ignore = "build camera_policy for wasm32-unknown-unknown, then run this acceptance case"]
fn the_independent_rust_wasm_camera_policy_runs_on_the_native_host() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32-unknown-unknown/release/examples/camera_policy.wasm");
    assert!(path.is_file(),"build: cargo build -p ambition_world_port --example camera_policy --release --target wasm32-unknown-unknown");
    let (mut app, body) = boot(vec![path]);
    roof_entry(&mut app, body);
}

#[test]
fn two_views_keep_separate_script_memory_and_retire_independently() {
    use ambition_platformer2d::sim_view::local_view::{LocalView, LocalViewId, ViewSubject};
    let file = ScriptFile::new();
    let (mut app, body) = boot(vec![file.0.clone()]);
    let first = app
        .world_mut()
        .query_filtered::<Entity, With<LocalView>>()
        .single(app.world())
        .unwrap();
    let second = app
        .world_mut()
        .spawn((
            LocalView,
            LocalViewId(1),
            ViewSubject(body),
            ambition_platformer2d::sim_view::camera_snapshot::local_view_facts(),
        ))
        .id();
    for _ in 0..3 {
        app.update();
    }
    let frames = |app: &App, view| {
        app.world()
            .get::<CameraPolicyState>(view)
            .unwrap()
            .records
            .records
            .records()[0]
            .record
            .get(ambition_platformer2d::extension::sdk::FieldRef(0))
            .unwrap()
            .as_u32()
            .unwrap()
    };
    assert_eq!(frames(&app, second), 3);
    assert!(frames(&app, first) > frames(&app, second));
    let before = frames(&app, first);
    assert!(app
        .world()
        .get::<ResolvedCameraSnapshot>(second)
        .unwrap()
        .0
        .is_some());
    app.world_mut().despawn(second);
    app.update();
    assert_eq!(frames(&app, first), before + 1);
}
