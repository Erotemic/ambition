//! **The fixed-tick half of the presentation/camera handoff, soft-framing side.**
//!
//! Bevy ordering constraints are SCHEDULE-LOCAL. The gameplay-presentation
//! cluster runs in `Update`; the camera observation resolve used to run in
//! `app.sim_schedule()`, which for this demo is `FixedUpdate`. The
//! `.before`/`.after` edges between them were therefore inert here — they
//! compiled, they read as guarantees, and they constrained nothing.
//!
//! The symptom is not a crash. It is a one-frame MIXTURE: the physical
//! `Camera.viewport` describes this frame's layout while the camera snapshot
//! still describes last frame's, so the world is framed for a viewport it is
//! no longer being drawn into.
//!
//! This is the real Sanic composition — the same `build_windowed_demo_app` the
//! demo binary uses, on `PlatformerEnginePlugins::fixed_tick()`, with the real
//! provider, route, session and `high_speed_full_bleed` profile.
//!
//! Sanic is the complementary case to Mary-O: FULL BLEED with soft framing
//! ACTIVE, so it exercises the `CameraScreenFraming` publication and the
//! cleared-viewport path rather than the pillarboxed one. A handoff fix that
//! only worked for fixed-aspect profiles would pass Mary-O and fail here.

#![cfg(feature = "visible")]

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};

use ambition::engine_core as ae;
use ambition::platformer::camera_layers::MainCamera;
use ambition::platformer::gameplay_presentation::ResolvedGameplayPresentation;
use ambition::sim_view::camera_snapshot::{
    CameraScreenFraming, CameraViewport, ResolvedCameraSnapshot,
};

use ambition_demo_sanic_app::{build_windowed_demo_app, RenderMode};

/// A 20:9 phone-shaped display: wide enough that a 4:3 gameplay rectangle is
/// visibly pillarboxed, so a stale viewport is unmistakable.
const DISPLAY: ae::Vec2 = ae::Vec2::new(2400.0, 1080.0);
/// A different aspect, to force a genuinely new layout mid-session.
const RESIZED: ae::Vec2 = ae::Vec2::new(1600.0, 1200.0);

fn sanic_app(display: ae::Vec2) -> App {
    let mut app = build_windowed_demo_app(RenderMode::Headless);
    // `RenderMode::Headless` builds with `primary_window: None`. The presentation
    // host reads the window as plain ECS data, so a synthetic one is a faithful
    // stand-in for a winit surface and lets this run with no display.
    app.world_mut().spawn((window_at(display), PrimaryWindow));
    app
}

fn window_at(display: ae::Vec2) -> Window {
    let mut resolution = WindowResolution::new(display.x as u32, display.y as u32);
    resolution.set_scale_factor(1.0);
    resolution.set(display.x, display.y);
    Window {
        resolution,
        ..default()
    }
}

fn settle(app: &mut App) {
    for _ in 0..12 {
        app.update();
    }
}

fn resize(app: &mut App, display: ae::Vec2) {
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    let mut window = windows
        .single_mut(app.world_mut())
        .expect("a primary window");
    window.resolution.set(display.x, display.y);
}

/// Every fact a visible frame is composed from must describe ONE layout.
///
/// The load-bearing assertion is the second: `visible_view` is
/// `viewport_px * orthographic_scale` *of the viewport the snapshot was
/// resolved against*. Checking that identity against the CURRENT
/// [`CameraViewport`] is therefore a direct test for staleness — a snapshot
/// resolved from last frame's viewport fails it the instant the display
/// changes size.
#[track_caller]
fn assert_one_coherent_layout(app: &mut App, display: ae::Vec2, label: &str) {
    let presentation = app
        .world()
        .resource::<ResolvedGameplayPresentation>()
        .clone();
    let viewport = app.world().resource::<CameraViewport>().px;
    let framing = *app.world().resource::<CameraScreenFraming>();
    let snapshot = app
        .world()
        .resource::<ResolvedCameraSnapshot>()
        .snapshot
        .clone();

    assert_eq!(
        presentation.display_rect.size(),
        display,
        "{label}: the layout should describe the current display",
    );

    let gameplay = presentation.gameplay_rect.size();
    assert_eq!(
        viewport, gameplay,
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

    let physical = app
        .world_mut()
        .query_filtered::<&Camera, With<MainCamera>>()
        .iter(app.world())
        .filter_map(|camera| camera.viewport.clone())
        .next();
    match physical {
        Some(viewport_rect) => assert_eq!(
            viewport_rect.physical_size,
            gameplay.as_uvec2(),
            "{label}: the physical camera viewport must match the same gameplay rect",
        ),
        None => assert_eq!(
            gameplay,
            presentation.display_rect.size(),
            "{label}: only a full-bleed layout may leave the viewport cleared",
        ),
    }
}

fn player_exists(app: &mut App) -> bool {
    app.world_mut()
        .query::<&ambition::platformer::body::BodyKinematics>()
        .iter(app.world())
        .next()
        .is_some()
}

/// Sanic's declared full-bleed soft-framing profile reaches every consumer
/// coherently on a fixed-tick host, both at rest and across a live resize.
#[test]
fn fixed_tick_sanic_keeps_one_layout_across_a_resize() {
    let mut app = sanic_app(DISPLAY);
    settle(&mut app);
    assert!(
        player_exists(&mut app),
        "the session must be live, or the camera resolve early-returns and \
         this test would pass vacuously",
    );

    assert_one_coherent_layout(&mut app, DISPLAY, "settled");

    // Full bleed: the gameplay rect IS the display, and soft framing is on.
    // Both are what distinguishes this fixture from the Mary-O one.
    let presentation = app
        .world()
        .resource::<ResolvedGameplayPresentation>()
        .clone();
    assert_eq!(presentation.gameplay_rect.size(), DISPLAY);
    assert!(
        presentation.soft_framing.is_some(),
        "Sanic declares velocity-aware soft framing on every platform",
    );
    assert!(app.world().resource::<CameraScreenFraming>().active);

    resize(&mut app, RESIZED);
    app.update();
    assert_one_coherent_layout(&mut app, RESIZED, "one update after resize");
    assert_eq!(
        app.world()
            .resource::<ResolvedGameplayPresentation>()
            .gameplay_rect
            .size(),
        RESIZED,
        "full bleed follows the display exactly",
    );
}

/// The sim really is on the other clock here — otherwise this test would be a
/// second render-frame test wearing a fixed-tick label.
#[test]
fn the_sanic_demo_really_runs_the_sim_in_fixed_update() {
    use ambition::platformer::schedule::SimScheduleExt as _;
    let app = sanic_app(DISPLAY);
    assert!(
        app.sim_is_fixed_tick(),
        "build_windowed_demo_app must compose PlatformerEnginePlugins::fixed_tick()",
    );
    assert!(
        !app.sim_is(Update),
        "a fixed-tick host must NOT share Update with the presentation cluster",
    );
}
