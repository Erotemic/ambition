//! Lightweight FPS / frame-time overlay.
//!
//! Wraps Bevy's built-in [`FrameTimeDiagnosticsPlugin`] in a small
//! Bevy plugin that spawns a `Text` node in the bottom-right corner
//! and refreshes it with the running FPS + rolling frame-time average.
//!
//! **Toggle:** press `F3` to show / hide.
//!
//! **Default visibility:** ON in browser builds (useful for diagnosing
//! the jumpy-animation symptom Jon hit during the GPU-training-job
//! window), OFF on desktop (the in-engine devtools provide richer
//! data).
//!
//! Designed to be flag-friendly: callers that want to hide it without
//! the keypress can mutate [`FpsOverlayState::visible`] from any
//! system, or skip [`FpsOverlayPlugin`] entirely.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::ui_fonts::{UiFontWeight, UiFonts};

/// Per-overlay state. Inserted as a `Resource` by
/// [`FpsOverlayPlugin`]. Mutate `visible` from any system to
/// programmatically show/hide; the next `update_fps_overlay_visibility`
/// tick reflects the change.
#[derive(Resource, Clone, Copy, Debug)]
pub struct FpsOverlayState {
    pub visible: bool,
}

impl Default for FpsOverlayState {
    fn default() -> Self {
        // ON by default on web (debug aid), OFF on desktop (avoid
        // chrome over the gameplay area until a user asks for it via
        // F3).
        Self {
            visible: cfg!(target_arch = "wasm32"),
        }
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

/// F3 toggles `FpsOverlayState::visible`. Cheap; the visibility flip
/// is consumed by `update_fps_overlay_visibility` next tick.
fn toggle_fps_overlay_on_f3(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<FpsOverlayState>,
) {
    if keys.just_pressed(KeyCode::F3) {
        state.visible = !state.visible;
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
    fn default_state_visible_on_wasm_hidden_on_desktop() {
        let state = FpsOverlayState::default();
        assert_eq!(state.visible, cfg!(target_arch = "wasm32"));
    }
}
