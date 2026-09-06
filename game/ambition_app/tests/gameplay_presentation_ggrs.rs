//! The GGRS half of the presentation/camera handoff.
//!
//! Under rollback the simulation advances inside `GgrsSchedule`, which
//! `bevy_ggrs` drives from `PreUpdate`. Ordering edges declared between the
//! `Update`-side presentation cluster and a system living in `GgrsSchedule`
//! are inert — Bevy ordering is schedule-local — so the physical
//! `Camera.viewport` could describe this frame's layout while the camera
//! snapshot still described the previous one.
//!
//! Rollback makes it worse than a stale read. No camera state is
//! rollback-registered, so a camera resolve inside `GgrsSchedule` re-integrates
//! `CameraEaseState` once per RESIMULATED frame: a 7-frame rollback advanced
//! camera easing seven extra times, and the amount depended on network
//! conditions. Camera observation belongs on the render clock, and this test
//! pins that it is there.
//!
//! This is the real flagship composition — the same `ambition_sim_composition`
//! the RL/trace binaries use, on a live `SyncTestSession` that genuinely rewinds
//! and resimulates, plus the visible host's presentation cluster.

#![cfg(feature = "rl_sim")]

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};

use ambition_app::rl_sim::ambition_sim_composition;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::camera_layers::MainCamera;
use ambition_platformer2d::platformer::gameplay_presentation::ResolvedGameplayPresentation;
use ambition_platformer2d::sim_view::camera_snapshot::{
    CameraScreenFraming, CameraViewport, ResolvedCameraSnapshot,
};
use ambition_sim_harness::{
    AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};

const DISPLAY: ae::Vec2 = ae::Vec2::new(2400.0, 1080.0);
const RESIZED: ae::Vec2 = ae::Vec2::new(1600.0, 1200.0);

fn window_at(display: ae::Vec2) -> Window {
    let mut resolution = WindowResolution::new(display.x as u32, display.y as u32);
    resolution.set_scale_factor(1.0);
    resolution.set(display.x, display.y);
    Window {
        resolution,
        ..default()
    }
}

/// The flagship simulation on a real sync-test rollback session, composed with
/// the VISIBLE host so the presentation cluster is actually installed.
fn ggrs_flagship() -> Platformer2dSimHarness {
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_sim_composition(app, options)?;
            // The visible host is what installs HostGameplayPresentationPlugin
            // and the camera cluster. Without it this would test nothing.
            app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
            app.world_mut().spawn((window_at(DISPLAY), PrimaryWindow));
            app.world_mut().spawn((Camera::default(), MainCamera));
            Ok(())
        },
    )
    .expect("the flagship GGRS sync-test harness builds with the visible host")
}

fn resize(sim: &mut Platformer2dSimHarness, display: ae::Vec2) {
    let world = sim.world_mut();
    let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
    let mut window = windows.single_mut(world).expect("a primary window");
    window.resolution.set(display.x, display.y);
}

/// See the Mary-O/Sanic siblings for the reasoning: `visible_view` divided by
/// `orthographic_scale` recovers the viewport the snapshot was actually
/// resolved against, so comparing it to the live [`CameraViewport`] detects a
/// one-frame-old mixture directly.
#[track_caller]
fn assert_one_coherent_layout(sim: &mut Platformer2dSimHarness, display: ae::Vec2, label: &str) {
    let presentation = sim
        .world()
        .resource::<ResolvedGameplayPresentation>()
        .clone();
    // The camera's observer facts belong to a local VIEW, not to the process.
    let view = ambition_platformer2d::sim_view::the_only_view(sim.world_mut());
    let view = sim.world().entity(view);
    let viewport = view
        .get::<CameraViewport>()
        .expect("the view's viewport")
        .px;
    let framing = *view
        .get::<CameraScreenFraming>()
        .expect("the view's screen framing");
    let snapshot = view
        .get::<ResolvedCameraSnapshot>()
        .expect("the view's resolved snapshot")
        .frame()
        // See the sanic/mary-o handoff tests: an unframed view is a broken
        // fixture here, not a tolerated case.
        .expect("the view has been framed by the time presentation hands off")
        .snapshot
        .clone();

    assert_eq!(
        presentation.display_rect.size(),
        display,
        "{label}: the layout should describe the current display",
    );
    assert_eq!(
        viewport,
        presentation.gameplay_rect.size(),
        "{label}: CameraViewport must be this frame's gameplay rect",
    );
    let implied = viewport * snapshot.orthographic_scale;
    assert!(
        (snapshot.visible_view - implied).length() < 0.05,
        "{label}: the camera snapshot was resolved against a DIFFERENT viewport \
         (visible_view {:?} implies {:?}, current viewport is {viewport:?})",
        snapshot.visible_view,
        snapshot.visible_view / snapshot.orthographic_scale.max(f32::EPSILON),
    );
    assert_eq!(
        framing.active,
        presentation.soft_framing.is_some(),
        "{label}: published framing must agree with the resolved profile",
    );

    let world = sim.world_mut();
    let mut cameras = world.query_filtered::<&Camera, With<MainCamera>>();
    let physical = cameras
        .iter(world)
        .filter_map(|camera| camera.viewport.clone())
        .next();
    if let Some(rect) = physical {
        assert_eq!(
            rect.physical_size,
            presentation.gameplay_rect.size().as_uvec2(),
            "{label}: the physical camera viewport must match the same gameplay rect",
        );
    }
}

/// The flagship's declared profile stays coherent across a live resize while
/// GGRS is genuinely rewinding and resimulating underneath.
#[test]
fn ggrs_flagship_keeps_one_layout_across_a_resize() {
    let mut sim = ggrs_flagship();
    for frame in 0..24 {
        sim.step(AgentAction {
            move_x: if frame % 8 < 4 { 1.0 } else { -1.0 },
            ..AgentAction::default()
        });
    }
    sim.rollback_health()
        .expect("the sync-test session is rewinding and resimulating for real");

    assert_one_coherent_layout(&mut sim, DISPLAY, "settled");

    // ONE step after the resize: a stale snapshot survives exactly one frame.
    resize(&mut sim, RESIZED);
    sim.step(AgentAction::default());
    assert_one_coherent_layout(&mut sim, RESIZED, "one step after resize");
}

/// Rollback must not advance presentation easing.
///
/// The camera resolve is the only writer of `CameraEaseState`, and NOTHING
/// rolls that state back. A resolve living in `GgrsSchedule` therefore
/// integrates it once per RESIMULATED frame, so camera smoothing becomes a
/// function of network conditions rather than of elapsed time.
///
/// This is asserted structurally, on purpose. The behavioural half — that the facts a visible frame
/// is built from agree — is covered by the coherence test above, which does fail when the resolve
/// is moved back. Schedule membership IS the invariant here, so that is what is checked.
#[test]
fn the_camera_resolve_is_not_inside_the_rollback_schedule() {
    use ambition_platformer2d::rollback::GgrsSchedule;
    use ambition_platformer2d::sim_view::camera_snapshot::CameraObservationSet;
    use bevy::ecs::schedule::Schedules;

    use bevy::ecs::schedule::ScheduleLabel as _;

    let sim = ggrs_flagship();
    let schedules = sim.world().resource::<Schedules>();
    let set = CameraObservationSet.intern();
    let members = |label: bevy::ecs::schedule::InternedScheduleLabel| {
        schedules
            .get(label)
            .map(|schedule| schedule.graph())
            // `SetNotFound` means the set has no node in this schedule at all,
            // which is exactly "no systems" rather than a failure.
            .map_or(0, |graph| {
                graph.systems_in_set(set).map_or(0, |systems| systems.len())
            })
    };

    assert_eq!(
        members(GgrsSchedule.intern()),
        0,
        "the camera observation resolve must not run inside the rollback \
         schedule: nothing rolls CameraEaseState back, so every resimulated \
         frame would integrate presentation easing again",
    );

    // counting the SET was a proxy, and it broke the first time the set
    // legitimately grew (the camera-frame setting's applier joined it, which
    // has to be in this set to inherit the not-in-rollback guarantee). The
    // condition was never "the set has one member" — it is that the ONE writer of
    // `CameraEaseState` runs once per rendered frame. So name it.
    let schedules = sim.world().resource::<Schedules>();
    let update = schedules.get(Update).expect("Update exists in the host");
    let graph = update.graph();
    let in_set = graph
        .systems_in_set(set)
        .expect("the plugin registers CameraObservationSet in Update");
    // names come off the EXECUTABLE, not the graph: once a schedule is built
    // its `SystemNode`s are drained into the executor, so `graph.systems.get`
    // answers `None` for every key and a name-based filter silently matches
    // nothing — a check that cannot fail in the direction that matters.
    let names: Vec<String> = update
        .systems()
        .expect("the host has run, so Update is initialized")
        .filter(|(key, _)| in_set.contains(key))
        .map(|(_, system)| system.name().to_string())
        .collect();
    assert_eq!(
        names.len(),
        in_set.len(),
        "every system in the set must be resolvable to a name, or the filter \
         below proves nothing"
    );
    let resolves = names
        .iter()
        .filter(|name| name.contains("resolve_camera_observation"))
        .count();
    assert_eq!(
        resolves, 1,
        "`resolve_camera_observation` must appear exactly once per rendered \
         frame in Update — it is the only writer of CameraEaseState, so a second \
         copy double-integrates the easing. Other systems may share the set; \
         only this one is rationed. The set holds: {names:?}",
    );
}
