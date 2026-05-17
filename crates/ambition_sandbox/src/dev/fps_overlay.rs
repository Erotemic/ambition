//! Lightweight FPS / frame-time overlay.
//!
//! Wraps Bevy's built-in [`FrameTimeDiagnosticsPlugin`] in a small
//! Bevy plugin that spawns a `Text` node in the bottom-right corner
//! and refreshes it with the running FPS + rolling frame-time average.
//!
//! **Visible by default on every platform** — desktop, browser,
//! Android. Toggle via the **Video settings page → "FPS Overlay"** row
//! (persisted across sessions via `crate::settings::persistence`), or
//! press `F3` for an in-session keyboard toggle that mutates the same
//! setting.
//!
//! ## Source of truth
//!
//! [`UserSettings::video::show_fps`] is the canonical flag and is what
//! lands on disk. [`FpsOverlayState`] is a runtime mirror so the
//! overlay systems don't have to query `UserSettings` every frame.
//! [`sync_fps_overlay_state_from_settings`] copies the value from
//! settings → state when the user changes it from the menu;
//! [`toggle_fps_overlay_on_f3`] writes back to settings so the keyboard
//! toggle persists too.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::settings::UserSettings;
use crate::ui_fonts::{UiFontWeight, UiFonts};

/// Runtime mirror of [`UserSettings::video::show_fps`]. Updated by
/// [`sync_fps_overlay_state_from_settings`] when the persisted flag
/// changes; the overlay systems read this resource instead of querying
/// `UserSettings` directly each frame.
#[derive(Resource, Clone, Copy, Debug)]
pub struct FpsOverlayState {
    pub visible: bool,
}

impl Default for FpsOverlayState {
    fn default() -> Self {
        // Visible everywhere by default. The Video settings row
        // overrides this once `UserSettings` is loaded; this default
        // is for the brief window between resource init and the first
        // settings sync.
        Self { visible: true }
    }
}

/// Tag on the overlay `Text` entity so `update_fps_overlay_text` can
/// `query_mut` exactly it.
#[derive(Component)]
struct FpsOverlayText;

/// Bevy plugin for the FPS overlay. Adds:
/// - `FrameTimeDiagnosticsPlugin::default()` (registers the FPS +
///   FRAME_TIME diagnostics if not already present),
/// - the [`FpsOverlayState`] resource,
/// - `spawn_fps_overlay` (Startup),
/// - `toggle_fps_overlay_on_f3` + `update_fps_overlay_text` +
///   `update_fps_overlay_visibility` (Update).
pub struct FpsOverlayPlugin;

impl Plugin for FpsOverlayPlugin {
    fn build(&self, app: &mut App) {
        // `FrameTimeDiagnosticsPlugin` is safe to add twice — Bevy
        // dedupes via plugin name. Insert defensively in case the
        // consumer registered diagnostics elsewhere.
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        app.init_resource::<FpsOverlayState>()
            .add_systems(Startup, spawn_fps_overlay)
            .add_systems(
                Update,
                (
                    sync_fps_overlay_state_from_settings,
                    toggle_fps_overlay_on_f3,
                    update_fps_overlay_text,
                    update_fps_overlay_visibility,
                ),
            );
    }
}

/// Spawn the overlay `Text` node in the bottom-right corner. Runs
/// once at Startup. The text body is updated each frame by
/// [`update_fps_overlay_text`]; we spawn an empty `Text` here so the
/// node exists from frame zero (otherwise the first second is
/// blank).
fn spawn_fps_overlay(
    mut commands: Commands,
    state: Res<FpsOverlayState>,
    ui_fonts: Option<Res<UiFonts>>,
) {
    let font = ui_fonts
        .map(|fonts| fonts.text_font(12.0, UiFontWeight::Monospace))
        .unwrap_or(TextFont {
            font_size: 12.0,
            ..default()
        });
    commands.spawn((
        Text::new(""),
        font,
        TextColor(Color::srgba(0.82, 0.95, 1.0, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(8.0),
            bottom: Val::Px(8.0),
            ..default()
        },
        // Initial visibility matches the resource default.
        if state.visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        },
        Name::new("FPS Overlay"),
        FpsOverlayText,
    ));
}

/// F3 toggles the FPS overlay by writing to
/// [`UserSettings::video::show_fps`]. The next
/// `sync_fps_overlay_state_from_settings` tick mirrors the change into
/// `FpsOverlayState`, and `crate::settings::persistence` autosaves the
/// new value so the toggle survives a restart.
fn toggle_fps_overlay_on_f3(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<UserSettings>,
) {
    if keys.just_pressed(KeyCode::F3) {
        settings.video.show_fps = !settings.video.show_fps;
    }
}

/// Mirror `UserSettings::video::show_fps` into `FpsOverlayState`. Runs
/// every Update; the cost is `Res::is_changed` change-detection on
/// `UserSettings` + a single boolean write.
fn sync_fps_overlay_state_from_settings(
    settings: Res<UserSettings>,
    mut state: ResMut<FpsOverlayState>,
) {
    if settings.is_changed() && state.visible != settings.video.show_fps {
        state.visible = settings.video.show_fps;
    }
}

/// Read `FPS` + `FRAME_TIME` from the diagnostics store and write a
/// single short line into the overlay text. Format:
///
/// ```text
/// FPS 60  |  frame 16.6ms
/// ```
///
/// Uses the smoothed (`smoothed()`) value when available; falls back
/// to the latest instantaneous value if smoothing hasn't built up
/// history yet.
fn update_fps_overlay_text(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<FpsOverlayText>>,
) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed().or_else(|| d.value()))
        .unwrap_or(0.0);
    let frame_time_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed().or_else(|| d.value()))
        .unwrap_or(0.0);
    text.0 = format!("FPS {fps:>3.0}  |  frame {frame_time_ms:>5.1}ms");
}

/// Sync the overlay entity's `Visibility` with `FpsOverlayState`.
/// Runs every frame; the cost is a single query + write.
fn update_fps_overlay_visibility(
    state: Res<FpsOverlayState>,
    mut query: Query<&mut Visibility, With<FpsOverlayText>>,
) {
    if !state.is_changed() {
        return;
    }
    let Ok(mut vis) = query.single_mut() else {
        return;
    };
    *vis = if state.visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_visible_on_every_platform() {
        // The FPS overlay is opt-out, not opt-in. The Video settings
        // page lets the user hide it; the default is `true` so the
        // counter shows up the moment the user runs the game without
        // touching settings.
        assert!(FpsOverlayState::default().visible);
    }

    #[test]
    fn default_video_settings_show_fps_is_true() {
        let settings = crate::settings::UserSettings::default();
        assert!(
            settings.video.show_fps,
            "VideoSettings::show_fps default must be true so the overlay shows out of the box",
        );
    }
}
