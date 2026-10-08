//! E5 step 6 — THE DEMO GATE, executable: a demo-shaped app assembles from the engine group + the
//! host group + a tiny fixture content plugin and ticks without panicking.
//!
//! The fixture provides exactly what the ENGINE deliberately does not own:
//! the installed WORLD (which rooms exist is the game's choice).

use bevy::prelude::*;

use ambition_platformer2d_core as ae;


mod support;
use support::FixtureContentPlugin;

/// A shell with no encounter content boots. Worlds that contain encounters but
/// provide no encounter authority still fail.
/// PROBE: what does an UNPINNED headless app actually step?
///
/// `demo_shell_boots_and_ticks` below pins the frame to the tick and counts,
/// because *"without the count, silence and success look the same"*. This is the
/// other half of that sentence, measured rather than assumed: the same
/// composition with `TimeUpdateStrategy` left ALONE, counting fixed steps over
/// the same number of `update()` calls.
///
/// Census 2026-09-16: 24 files call `add_headless_foundation`, and NINE of them
/// call `update()` without pinning the timestep. Whether that matters depends
/// entirely on this number, which nobody had.
#[test]
#[ignore = "PROBE, print-only: how many fixed steps an UNPINNED headless app takes"]
fn probe_how_many_fixed_steps_an_unpinned_headless_app_takes() {
    #[derive(Resource, Default)]
    struct Steps(u32);
    let mut app = App::new();
    ambition_platformer2d_runtime::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d_runtime::PlatformerEnginePlugins::default());
    app.add_plugins(ambition_platformer2d_host::PlatformerHostPlugins);
    app.add_plugins(FixtureContentPlugin);
    app.init_resource::<Steps>();
    app.add_systems(FixedUpdate, |mut s: ResMut<Steps>| s.0 += 1);

    let timestep = app
        .world()
        .resource::<bevy::time::Time<bevy::time::Fixed>>()
        .timestep();
    let mut seen = Vec::new();
    for n in 1..=10 {
        app.update();
        seen.push((n, app.world().resource::<Steps>().0));
    }
    println!(
        "[unpinned] timestep {:?}; fixed steps after each of 10 update() calls: {:?}",
        timestep, seen
    );
    println!(
        "[unpinned] TOTAL fixed steps over 10 updates: {}",
        app.world().resource::<Steps>().0
    );
    println!(
        "[unpinned] an arm asserting about simulation state after this many \
         update() calls is asserting over whatever this number turned out to be \
         on the box that ran it"
    );
}

#[test]
fn demo_shell_boots_and_ticks() {
    let mut app = App::new();
    ambition_platformer2d_runtime::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d_runtime::PlatformerEnginePlugins::default());
    app.add_plugins(ambition_platformer2d_host::PlatformerHostPlugins);
    app.add_plugins(FixtureContentPlugin);

    // `add_headless_foundation` brings `MinimalPlugins`, which leaves
    // `TimeUpdateStrategy::Automatic`. Then the fixed schedule runs zero or more
    // times, related to the elapsed wall time, and a quick arm steps nothing.
    // Pin the frame to the tick, then show that the fixed schedule did run:
    // without the count, silence and success look the same.
    #[derive(Resource, Default)]
    struct FixedStepsTaken(u32);
    let timestep = app.world().resource::<bevy::time::Time<bevy::time::Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app.init_resource::<FixedStepsTaken>();
    app.add_systems(FixedUpdate, |mut taken: ResMut<FixedStepsTaken>| taken.0 += 1);

    // First update runs Startup; a couple more prove the sim loop holds.
    app.update();
    app.update();
    app.update();

    assert!(
        app.world().resource::<FixedStepsTaken>().0 > 0,
        "the shell booted but the fixed schedule never ran, so this arm shows \
         only that the demo assembly BUILDS"
    );
}

// The exit check has two halves. Here: the demo assembly boots in `FixedUpdate`
// and the sim graph does not SPLIT across two schedules. In
// `game/ambition_app/tests/{player,actor}_phase_split.rs`: the rl_sim
// schedule-shape suites pass with the label threaded BOTH ways.
// ─────────────────────────────────────────────────────────────────────────────

use ambition_platformer2d_runtime::SimTick;
use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase;
use bevy::ecs::schedule::Schedules;
use bevy::time::{Fixed, Time, TimeUpdateStrategy};

/// Every sim phase. `PresentationVisualSync` is deliberately absent: it is the
/// one presentation-side label in `Platformer2dSimulationPhase`, and render joins it in `Update`.
const SIM_PHASES: &[Platformer2dSimulationPhase] = &[
    Platformer2dSimulationPhase::CoreSimulation,
    Platformer2dSimulationPhase::WorldPrep,
    Platformer2dSimulationPhase::PlayerInput,
    Platformer2dSimulationPhase::PlayerSimulation,
    Platformer2dSimulationPhase::RoomTransition,
    Platformer2dSimulationPhase::Combat,
    Platformer2dSimulationPhase::PresentationSync,
    Platformer2dSimulationPhase::FeatureCollection,
    Platformer2dSimulationPhase::FeatureInteraction,
    Platformer2dSimulationPhase::LdtkRuntimeSpine,
    Platformer2dSimulationPhase::EncounterSimulation,
    Platformer2dSimulationPhase::Cutscene,
    Platformer2dSimulationPhase::GameplayEffects,
    Platformer2dSimulationPhase::Progression,
    Platformer2dSimulationPhase::ResetProcessing,
    Platformer2dSimulationPhase::FeatureViewSync,
    Platformer2dSimulationPhase::Trace,
];

fn systems_in(
    app: &App,
    schedule: impl bevy::ecs::schedule::ScheduleLabel,
    set: Platformer2dSimulationPhase,
) -> usize {
    let schedules = app.world().resource::<Schedules>();
    let Some(graph) = schedules.get(schedule).map(|s| s.graph()) else {
        return 0;
    };
    // `SetNotFound` means the set has no node in this schedule at all — which is exactly "no
    // systems", not a failure. A node with zero members reads the same.
    graph.systems_in_set(set.intern()).map_or(0, |s| s.len())
}

/// Build the shell and run Bevy's Startup frame.
///
/// Every frame after it advances exactly one tick, because the frame dt is pinned to the tick
/// dt (identical `Duration`s, hence integer nanoseconds, hence no accumulator drift ever).
fn fixed_tick_shell() -> App {
    let mut app = App::new();
    ambition_platformer2d_runtime::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d_runtime::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d_host::PlatformerHostPlugins);
    app.add_plugins(FixtureContentPlugin);
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(timestep));
    app.update(); // Startup; zero ticks.
    app
}

#[test]
fn fixed_tick_demo_shell_boots_and_ticks() {
    let mut app = fixed_tick_shell();
    assert_eq!(
        app.world().resource::<SimTick>().get(),
        0,
        "Startup alone must not advance the timeline"
    );

    // The first step is tick 1: tick 0 names the moment before it (Q128).
    for expected in 1..=6 {
        app.update();
        assert_eq!(
            app.world().resource::<SimTick>().get(),
            expected,
            "one frame at exactly the tick dt must expend exactly one tick"
        );
    }
}

/// The graph must not split. A content or engine plugin that hardcoded `Update` instead of
/// asking `app.sim_schedule()` would land its systems here, where they would silently stop
/// ordering against the rest of the sim.
#[test]
fn fixed_tick_leaves_no_sim_system_in_update() {
    let mut app = fixed_tick_shell();
    app.update(); // one real tick, so BOTH schedule graphs are initialized

    let mut stranded = Vec::new();
    for &phase in SIM_PHASES {
        let n = systems_in(&app, Update, phase);
        if n > 0 {
            stranded.push(format!("{phase:?} ({n} system(s))"));
        }
    }
    assert!(
        stranded.is_empty(),
        "sim systems stranded in `Update` under fixed tick: {}. \
         They were registered with a literal `Update` instead of \
         `app.sim_schedule()`, so they no longer order against the sim.",
        stranded.join(", "),
    );
}

/// ...and the phases really are populated on the other side.
#[test]
fn fixed_tick_puts_the_sim_phases_in_fixed_update() {
    let mut app = fixed_tick_shell();
    app.update(); // one real tick, so the FixedUpdate graph is initialized

    for phase in [
        Platformer2dSimulationPhase::PlayerInput,
        Platformer2dSimulationPhase::WorldPrep,
        Platformer2dSimulationPhase::Combat,
        Platformer2dSimulationPhase::FeatureViewSync,
    ] {
        assert!(
            systems_in(&app, FixedUpdate, phase) > 0,
            "{phase:?} must carry systems in FixedUpdate under fixed tick"
        );
    }
}

/// Frame-stepped is the default and is unchanged: the sim lives in `Update`,
/// and `FixedUpdate` carries nothing of ours.
#[test]
fn frame_stepped_shell_keeps_the_sim_in_update() {
    let mut app = App::new();
    ambition_platformer2d_runtime::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d_runtime::PlatformerEnginePlugins::default());
    app.add_plugins(ambition_platformer2d_host::PlatformerHostPlugins);
    app.add_plugins(FixtureContentPlugin);
    app.update();

    assert!(systems_in(&app, Update, Platformer2dSimulationPhase::WorldPrep) > 0);
    assert_eq!(
        systems_in(
            &app,
            FixedUpdate,
            Platformer2dSimulationPhase::WorldPrep
        ),
        0
    );
    // The timeline advances in both modes. The first step is tick 1 (Q128).
    assert_eq!(app.world().resource::<SimTick>().get(), 1);
    app.update();
    assert_eq!(app.world().resource::<SimTick>().get(), 2);
}

/// Choosing the mode after a sim plugin has already committed systems is the
/// one way to get a split graph. It must be loud, not silent.
#[test]
#[should_panic(expected = "sim schedule already sealed")]
fn changing_the_sim_schedule_after_a_sim_plugin_panics() {
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;
    let mut app = App::new();
    ambition_platformer2d_runtime::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d_runtime::PlatformerEnginePlugins::default());
    app.set_sim_schedule(FixedUpdate);
}

// ─────────────────────────────────────────────────────────────────────────────
// Gameplay presentation profiles — the chain, through the REAL composition.
//
// The per-crate tests each prove one link with the neighbouring links faked.
// This proves the links are actually CONNECTED in the assembled host: the
// declared profile reaches the sim's camera observation input, the physical
// camera viewport, and the painted surround, in one frame, under the real
// system ordering. A missing `.before()` is invisible to every unit test and
// shows up here.
// ─────────────────────────────────────────────────────────────────────────────

use ambition_platformer2d_shared_tangle::camera_layers::{FrontHudCamera, MainCamera};
use ambition_platformer2d_shared_tangle::gameplay_presentation::{
    profiles, ActiveGameplayPresentationProfiles, PresentationEnvironment,
    ResolvedGameplayPresentation,
};
use ambition_sim_view::camera_snapshot::CameraViewport;

/// The one local view's viewport. The camera's observer facts belong to a VIEW,
/// so a fixture asking "what is the viewport" has to say whose.
fn view_viewport(app: &mut App) -> CameraViewport {
    let view = ambition_sim_view::the_only_view(app.world_mut());
    *app.world().entity(view).get::<CameraViewport>().unwrap()
}
use bevy::window::{PrimaryWindow, WindowResolution};

/// A 20:9 phone-shaped display, which pillarboxes a 4:3 gameplay rectangle.
const DISPLAY: ae::Vec2 = ae::Vec2::new(2400.0, 1080.0);

fn presentation_shell(profiles: ActiveGameplayPresentationProfiles) -> App {
    let mut app = App::new();
    ambition_platformer2d_runtime::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d_runtime::PlatformerEnginePlugins::default());
    app.add_plugins(ambition_platformer2d_host::PlatformerHostPlugins);
    app.add_plugins(FixtureContentPlugin);
    app.insert_resource(profiles);
    app.insert_resource(PresentationEnvironment::Desktop);

    let mut resolution = WindowResolution::new(DISPLAY.x as u32, DISPLAY.y as u32);
    resolution.set_scale_factor(1.0);
    resolution.set(DISPLAY.x, DISPLAY.y);
    app.world_mut().spawn((
        Window {
            resolution,
            ..default()
        },
        PrimaryWindow,
    ));
    // The camera rig a visible host installs.
    app.world_mut().spawn((Camera::default(), MainCamera));
    app.world_mut().spawn((Camera::default(), FrontHudCamera));

    app.update();
    app.update();
    app
}

#[test]
fn a_fixed_aspect_profile_reaches_the_camera_and_the_surround() {
    let mut app = presentation_shell(ActiveGameplayPresentationProfiles(
        profiles::fixed_four_by_three(),
    ));

    let gameplay = app
        .world()
        .resource::<ResolvedGameplayPresentation>()
        .gameplay_rect;
    assert_eq!(
        gameplay.size(),
        ae::Vec2::new(1440.0, 1080.0),
        "4:3 inside a 20:9 display",
    );

    // 1. The sim's observation input is the gameplay rect, not the window.
    assert_eq!(view_viewport(&mut app).px, gameplay.size());

    // 2. The main camera carries the physical viewport; the HUD camera does not.
    let main = app
        .world_mut()
        .query_filtered::<&Camera, With<MainCamera>>()
        .single(app.world())
        .expect("one main camera")
        .viewport
        .clone()
        .expect("the main camera is viewport-clipped");
    assert_eq!(main.physical_size, gameplay.size().as_uvec2());
    assert!(
        app.world_mut()
            .query_filtered::<&Camera, With<FrontHudCamera>>()
            .single(app.world())
            .expect("one hud camera")
            .viewport
            .is_none(),
        "the HUD camera must stay full-screen, or menus letterbox too",
    );

    // 3. Something paints the pillarboxes. Nothing else clears them.
    let painted: f32 = app
        .world_mut()
        .query::<&Node>()
        .iter(app.world())
        .filter_map(|node| match (node.width, node.height) {
            (Val::Px(w), Val::Px(h)) if w > 0.5 && h > 0.5 => Some(w * h),
            _ => None,
        })
        .sum();
    let unpainted = DISPLAY.x * DISPLAY.y - gameplay.width() * gameplay.height();
    assert!(
        (painted - unpainted).abs() < 1.0,
        "surround painted {painted}px² of {unpainted}px² of uncleared display",
    );
}

/// The default declaration — what a provider that says nothing gets — leaves
/// the assembled host exactly as it was before this subsystem existed.
#[test]
fn an_undeclared_profile_leaves_the_host_full_bleed() {
    let mut app = presentation_shell(ActiveGameplayPresentationProfiles::default());

    assert_eq!(view_viewport(&mut app).px, DISPLAY);
    assert!(
        app.world_mut()
            .query_filtered::<&Camera, With<MainCamera>>()
            .single(app.world())
            .expect("one main camera")
            .viewport
            .is_none(),
        "full bleed must not set a viewport at all",
    );
    assert_eq!(
        app.world_mut().query::<&Node>().iter(app.world()).count(),
        0,
        "full bleed owes the display no surround",
    );
}
