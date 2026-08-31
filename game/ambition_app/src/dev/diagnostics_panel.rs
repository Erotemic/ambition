//! The F1 numeric panel, on Bevy's diagnostics overlay.
//!
//! ⭐ ONE MEASUREMENT, MANY CONSUMERS. Nothing here counts anything. Every value
//! shown is a `DiagnosticPath` published by the subsystem that already knew the
//! fact — `ambition_dev_tools::runtime_census` for the ECS populations,
//! `ambition_render::runtime_census` for the camera populations, Bevy itself for
//! frame timing and render-pass spans — so the panel, the periodic `[census]`
//! log and any future report all read the SAME number. A panel that computed
//! its own would be a second answer with the same name.
//!
//! ⛔ A MISSING PATH RENDERS AS `Missing`, AND THAT IS THE FEATURE. Bevy's
//! overlay says so rather than showing a zero, which is exactly the distinction
//! this repository insists on elsewhere: a zero from an instrument that never
//! reports that category is not a measurement. Two paths here are legitimately
//! absent on some platforms and MUST read as missing rather than as nothing
//! happening — see [`AmbitionDiagnosticsPanelPlugin`].

use bevy::dev_tools::diagnostics_overlay::{DiagnosticsOverlay, DiagnosticsOverlayPlugin};
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

// ⭐ THROUGH THE RE-EXPORTS, like every other app-side reader. `ambition_app`
// does not name `ambition_dev_tools` or `ambition_render` as its own
// dependencies; it reaches them as `ambition_platformer2d::{dev_tools, render}`,
// which is the seam that decides what the app is allowed to see.
use ambition_platformer2d::dev_tools::runtime_census::{BODIES, RESOURCE_ENTITIES, SCENE_ENTITIES};
use ambition_platformer2d::dev_tools::DeveloperRuntimeState;
use ambition_platformer2d::render::runtime_census::{CAMERAS, OFFSCREEN_TARGETS, WORLD_DRAWS};

/// Marks the windows this module owns, so F1 can retire exactly them.
#[derive(Component)]
struct AmbitionDiagnosticsWindow;

/// F1's numeric surface.
///
/// ⭐ SYSTEM INFORMATION IS DESKTOP-ONLY, AND DELIBERATELY SO. Bevy's
/// `SystemInformationDiagnosticsPlugin` rides `bevy/sysinfo_plugin`, which
/// `default_platform` carries and which Ambition's `android_platform` and
/// `web_platform` feature sets EXCLUDE on purpose. So CPU and memory appear on
/// desktop and read `Missing` elsewhere — which is the honest rendering of "this
/// platform does not report it", and is why no attempt is made to synthesize a
/// substitute.
pub struct AmbitionDiagnosticsPanelPlugin;

impl Plugin for AmbitionDiagnosticsPanelPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        #[cfg(feature = "desktop_platform")]
        if !app.is_plugin_added::<bevy::diagnostic::SystemInformationDiagnosticsPlugin>() {
            app.add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin);
        }
        app.add_plugins(DiagnosticsOverlayPlugin)
            .add_plugins(ambition_platformer2d::dev_tools::runtime_census::EcsDiagnosticsPlugin)
            .add_plugins(
                ambition_platformer2d::render::runtime_census::RenderDiagnosticsPublishPlugin,
            )
            .add_systems(Update, follow_the_debug_toggle);
    }
}

/// Show the panel while F1's debug mode is on, and take it away when it is off.
///
/// ⛔ KEYED TO `DeveloperRuntimeState.debug`, WHICH IS NOW A HOST FACT. Until
/// this campaign's A2 that flag could not be toggled without a live session, so
/// a panel bound to it would have been unreachable from the launcher — the exact
/// place a "why is the title screen slow" question gets asked.
fn follow_the_debug_toggle(
    mut commands: Commands,
    dev_state: Res<DeveloperRuntimeState>,
    windows: Query<Entity, With<AmbitionDiagnosticsWindow>>,
) {
    if !dev_state.is_changed() {
        return;
    }
    let showing = !windows.is_empty();
    if dev_state.debug == showing {
        return;
    }
    if dev_state.debug {
        commands.spawn((
            AmbitionDiagnosticsWindow,
            DiagnosticsOverlay::new(
                "Frame",
                vec![
                    FrameTimeDiagnosticsPlugin::FPS.into(),
                    FrameTimeDiagnosticsPlugin::FRAME_TIME.into(),
                ],
            ),
        ));
        commands.spawn((
            AmbitionDiagnosticsWindow,
            // ⭐ TWO ENTITY NUMBERS, NAMED. One number called "entities" would
            // carry Bevy 0.19's resources-are-entities ambiguity into every note
            // taken from this panel.
            DiagnosticsOverlay::new(
                "Ambition",
                vec![
                    SCENE_ENTITIES.into(),
                    RESOURCE_ENTITIES.into(),
                    BODIES.into(),
                    CAMERAS.into(),
                    WORLD_DRAWS.into(),
                    OFFSCREEN_TARGETS.into(),
                ],
            ),
        ));
        #[cfg(feature = "desktop_platform")]
        commands.spawn((
            AmbitionDiagnosticsWindow,
            DiagnosticsOverlay::new(
                "Host",
                vec![
                    bevy::diagnostic::SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE.into(),
                    bevy::diagnostic::SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE.into(),
                ],
            ),
        ));
    } else {
        for window in &windows {
            commands.entity(window).despawn();
        }
    }
}
