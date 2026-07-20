//! Runtime oracles for the host presentation cluster.
//!
//! These pin acceptance oracles 3, 5 and 9 from
//! `docs/planning/triage/gameplay-presentation-profiles.md` against the real
//! systems, with a synthetic primary window instead of a winit surface.

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};

use ambition_engine_core as ae;
use ambition_platformer_primitives::camera_layers::MainCamera;
use ambition_platformer_primitives::gameplay_presentation::{
    profiles, ActiveGameplayPresentationProfiles, GameplayPresentationProfile,
    GameplayPresentationProfiles, PresentationEnvironment, ResolvedGameplayPresentation,
    ScreenAnchor, ScreenOccluder, ScreenOcclusionPurpose, SoftFramingProfile,
};
use ambition_sim_view::camera_snapshot::{CameraScreenFraming, CameraViewport};

use super::*;

/// Build an app running just this cluster against a synthetic window.
fn host_app(
    display: ae::Vec2,
    scale_factor: f32,
    profiles: GameplayPresentationProfiles,
    environment: PresentationEnvironment,
) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(HostGameplayPresentationPlugin);
    app.insert_resource(ActiveGameplayPresentationProfiles(profiles));
    app.insert_resource(environment);

    let mut resolution = WindowResolution::new(display.x as u32, display.y as u32);
    resolution.set_scale_factor(scale_factor);
    resolution.set(display.x, display.y);
    app.world_mut().spawn((
        Window {
            resolution,
            ..default()
        },
        PrimaryWindow,
    ));
    app.world_mut().spawn((Camera::default(), MainCamera));
    app
}

fn resolved(app: &App) -> &ResolvedGameplayPresentation {
    app.world().resource::<ResolvedGameplayPresentation>()
}

fn main_camera_viewport(app: &mut App) -> Option<Viewport> {
    app.world_mut()
        .query_filtered::<&Camera, With<MainCamera>>()
        .single(app.world())
        .expect("one main camera")
        .viewport
        .clone()
}

/// Oracle 3 — the camera observer reports the GAMEPLAY viewport dimensions,
/// not blindly the window dimensions. This is the single fact the whole
/// fixed-aspect slice rests on: every downstream visible-world and clamp
/// calculation reads `CameraViewport`.
#[test]
fn fixed_aspect_publishes_the_gameplay_viewport_not_the_window() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::Desktop,
    );
    app.update();

    let gameplay = resolved(&app).gameplay_rect;
    assert_eq!(gameplay.size(), ae::Vec2::new(1440.0, 1080.0));
    assert_eq!(app.world().resource::<CameraViewport>().px, gameplay.size());
    assert_ne!(
        app.world().resource::<CameraViewport>().px,
        ae::Vec2::new(2400.0, 1080.0),
        "publishing the window here would silently stretch the 4:3 view",
    );
}

/// Full bleed publishes the whole window, which is exactly the pre-existing
/// behavior — a game that declares nothing must not move.
#[test]
fn full_bleed_publishes_the_whole_window() {
    let mut app = host_app(
        ae::Vec2::new(1920.0, 1080.0),
        1.0,
        GameplayPresentationProfiles::default(),
        PresentationEnvironment::Desktop,
    );
    app.update();

    assert_eq!(
        app.world().resource::<CameraViewport>().px,
        ae::Vec2::new(1920.0, 1080.0),
    );
    assert!(
        main_camera_viewport(&mut app).is_none(),
        "full bleed must leave the camera viewport cleared, not set it to the window",
    );
}

/// Oracle 5's mechanism — the gameplay rectangle becomes a real physical
/// `Camera::viewport`, converted through the window scale factor exactly once.
#[test]
fn fixed_aspect_applies_a_physical_camera_viewport() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        2.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::Desktop,
    );
    app.update();

    let gameplay = resolved(&app).gameplay_rect;
    let viewport = main_camera_viewport(&mut app).expect("fixed aspect sets a viewport");
    assert_eq!(
        viewport.physical_position,
        (gameplay.min * 2.0).round().as_uvec2(),
    );
    assert_eq!(
        viewport.physical_size,
        (gameplay.size() * 2.0).round().as_uvec2(),
    );
    // Sanity: the physical rect is inside the physical window.
    assert!(viewport.physical_position.x + viewport.physical_size.x <= 4800);
}

/// The front HUD camera is untouched, so full-screen menus, load screens and
/// dialogue keep the whole display (oracle 11).
#[test]
fn only_the_main_camera_receives_a_viewport() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::Desktop,
    );
    let hud = app.world_mut().spawn(Camera::default()).id();
    app.update();

    assert!(app
        .world()
        .entity(hud)
        .get::<Camera>()
        .expect("hud camera")
        .viewport
        .is_none());
}

/// Selection is by declared ENVIRONMENT, never by game name, and a touch
/// profile really does differ from its desktop sibling.
#[test]
fn the_environment_selects_the_declared_profile() {
    let display = ae::Vec2::new(900.0, 900.0);
    let mut desktop = host_app(
        display,
        1.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::Desktop,
    );
    let mut touch = host_app(
        display,
        1.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::TouchPrimary,
    );
    desktop.update();
    touch.update();

    assert_eq!(
        resolved(&desktop).gameplay_rect.size(),
        resolved(&touch).gameplay_rect.size(),
        "the same aspect is requested in both environments",
    );
    assert!(
        resolved(&touch).gameplay_rect.min.y < resolved(&desktop).gameplay_rect.min.y,
        "touch-primary pins the rectangle toward the top",
    );
}

fn stick_occluder() -> ScreenOccluder {
    ScreenOccluder::new(
        ScreenOcclusionPurpose::VirtualMovementStick,
        ScreenAnchor::BottomLeft,
        ae::Vec2::splat(24.0),
        ae::Vec2::splat(600.0),
    )
}

fn occlusion_aware() -> GameplayPresentationProfiles {
    GameplayPresentationProfiles::uniform(
        GameplayPresentationProfile::full_bleed()
            .with_occlusion_aware_framing(SoftFramingProfile::platformer()),
    )
}

/// A control that is not on screen must not reserve space for itself.
#[test]
fn hidden_occluders_do_not_reserve_space() {
    let display = ae::Vec2::new(2400.0, 1080.0);
    let mut visible = host_app(
        display,
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    visible.world_mut().spawn(stick_occluder());
    visible.update();

    let mut hidden = host_app(
        display,
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    hidden
        .world_mut()
        .spawn((stick_occluder(), InheritedVisibility::HIDDEN));
    hidden.update();

    let mut none = host_app(
        display,
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    none.update();

    assert_eq!(
        resolved(&hidden).subject_safe_rect,
        resolved(&none).subject_safe_rect,
        "a hidden control must not shrink framing",
    );
    assert_ne!(
        resolved(&visible).subject_safe_rect,
        resolved(&none).subject_safe_rect,
        "a visible control must shrink framing",
    );
}

/// Normal framing publishes an INACTIVE screen-framing fact, so the camera
/// resolver takes its ordinary centering path untouched (oracle 9's mechanism
/// on the desktop side).
#[test]
fn normal_framing_publishes_an_inactive_fact() {
    let mut app = host_app(
        ae::Vec2::new(1920.0, 1080.0),
        1.0,
        profiles::adaptive_platformer(),
        PresentationEnvironment::Desktop,
    );
    app.update();

    let framing = app.world().resource::<CameraScreenFraming>();
    assert!(!framing.active);
    assert_eq!(*framing, CameraScreenFraming::default());
}

/// Soft framing publishes the resolved region and its tuning.
#[test]
fn soft_framing_publishes_the_resolved_region() {
    let mut app = host_app(
        ae::Vec2::new(1920.0, 1080.0),
        1.0,
        profiles::high_speed_full_bleed(),
        PresentationEnvironment::Desktop,
    );
    app.update();

    let expected = SoftFramingProfile::high_speed();
    let framing = app.world().resource::<CameraScreenFraming>();
    assert!(framing.active);
    assert_eq!(framing.subject_safe_region, expected.safe_region);
    assert_eq!(framing.look_ahead_seconds, expected.look_ahead_seconds);
    assert_eq!(framing.subject_padding_px, expected.subject_padding_px);
}

/// Hysteresis: occupancy appearing mid-session must ease the region rather
/// than step it, or the camera twitches every time a contextual button shows.
#[test]
fn a_control_appearing_eases_the_region_instead_of_stepping_it() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    app.update();
    let settled = app.world().resource::<CameraScreenFraming>().subject_safe_region;

    app.world_mut().spawn(stick_occluder());
    app.update();
    let after_one_frame = app.world().resource::<CameraScreenFraming>().subject_safe_region;
    let target = resolved(&app).subject_safe_region;

    assert_ne!(target, settled, "the fixture must actually change the region");
    assert_ne!(
        after_one_frame, target,
        "the published region must not jump to the new target in one frame",
    );
    assert!(
        after_one_frame.min.x > settled.min.x && after_one_frame.min.x < target.min.x,
        "the published region should be easing between {settled:?} and {target:?}, got {after_one_frame:?}",
    );
}
