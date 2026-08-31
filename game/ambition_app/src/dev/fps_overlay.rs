//! Lightweight FPS / frame-time overlay.
//!
//! Wraps Bevy's built-in [`FrameTimeDiagnosticsPlugin`] in a small
//! Bevy plugin that spawns a `Text` node and refreshes it with the
//! running FPS + rolling frame-time average. It sits under the
//! Menu/Back row where there is one, and in a corner where there is
//! not — see [`position_fps_overlay`].
//!
//! Visible by default on every platform — desktop, browser,
//! Android. Toggle via the Video settings page → "FPS Overlay" row
//! (persisted across sessions via `ambition_platformer2d::persistence::settings::persistence`), or
//! press `F6` for an in-session keyboard toggle that mutates the same
//! setting.
//!
//! ## Source of truth
//!
//! [`UserSettings::video::show_fps`] is the canonical flag and is what
//! lands on disk. [`FpsOverlayState`] is a runtime mirror so the
//! overlay systems don't have to query `UserSettings` every frame.
//! [`sync_fps_overlay_state_from_settings`] copies the value from
//! settings → state when the user changes it from the menu;
//! [`toggle_fps_overlay_from_hotkey`] writes back to settings so the keyboard
//! toggle persists too.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use ambition_platformer2d::persistence::settings::UserSettings;
use ambition_platformer2d::platformer::developer_hotkeys::DeveloperAction;
use ambition_platformer2d::presentation::gameplay_presentation::ResolvedGameplayPresentation;
use ambition_platformer2d::render::ui_fonts::{UiFontWeight, UiFonts};

const FPS_OVERLAY_REFRESH_SECONDS: f32 = 0.25;

/// Gap between the Menu/Back row and the counter tucked under it.
const FPS_OVERLAY_GAP: f32 = 6.0;

/// Inset from the corner when there is no on-screen control row to sit under.
const FPS_OVERLAY_MARGIN: f32 = 8.0;

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
/// - `toggle_fps_overlay_from_hotkey` + `update_fps_overlay_text` +
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
        app.add_message::<DeveloperAction>()
            .init_resource::<FpsOverlayState>()
            .add_systems(Startup, spawn_fps_overlay)
            .add_systems(
                Update,
                (
                    sync_fps_overlay_state_from_settings,
                    toggle_fps_overlay_from_hotkey,
                    update_fps_overlay_text,
                    update_fps_overlay_visibility,
                    position_fps_overlay,
                ),
            );
    }
}

/// Spawn the overlay `Text` node. Runs once at Startup. The text body is
/// updated each frame by [`update_fps_overlay_text`]; we spawn an empty `Text`
/// here so the node exists from frame zero (otherwise the first second is
/// blank).
///
/// The corner set here is only where it STARTS: [`position_fps_overlay`] moves
/// it under the Menu/Back row on the first frame a layout is published, and
/// leaves it here when none ever is.
fn spawn_fps_overlay(
    mut commands: Commands,
    state: Res<FpsOverlayState>,
    ui_fonts: Option<Res<UiFonts>>,
) {
    let font = ui_fonts
        .map(|fonts| fonts.text_font(12.0, UiFontWeight::Monospace))
        .unwrap_or(TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        });
    commands.spawn((
        Text::new(""),
        font,
        TextColor(Color::srgba(0.82, 0.95, 1.0, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(FPS_OVERLAY_MARGIN),
            bottom: Val::Px(FPS_OVERLAY_MARGIN),
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

/// Tuck the counter under the Menu/Back row instead of leaving it in a corner.
///
/// The bottom-right corner is where the action cluster goes on a touch device,
/// so the counter spent every phone session sitting underneath the buttons —
/// present, updating, and unreadable.
///
/// the row's rectangle is READ, not re-derived. `ResolvedGameplayPresentation`
/// says of itself that no camera, HUD, touch or pointer system should
/// independently recalculate margins, and a hardcoded top offset would be
/// exactly that: the row moves with the safe-area insets and with its own
/// resolved scale, so a constant would be right on one device and wrong on the
/// next. `ScreenRect` is logical pixels — the same space `Val::Px` is in, which
/// is why no scale factor appears here (`set_node_rect` in the touch overlay
/// assigns from these rects the same way).
///
/// With no row published — a keyboard session publishes no touch footprint —
/// there is nothing to sit under and the counter keeps its corner.
fn position_fps_overlay(
    presentation: Option<Res<ResolvedGameplayPresentation>>,
    mut overlays: Query<&mut Node, With<FpsOverlayText>>,
) {
    let menu_row = presentation
        .as_deref()
        .and_then(|presentation| presentation.controls.system_controls)
        .map(|placed| placed.rect);

    let (left, top, right, bottom) = match menu_row {
        Some(rect) => (
            Val::Px(rect.min.x),
            Val::Px(rect.max.y + FPS_OVERLAY_GAP),
            Val::Auto,
            Val::Auto,
        ),
        None => (
            Val::Auto,
            Val::Auto,
            Val::Px(FPS_OVERLAY_MARGIN),
            Val::Px(FPS_OVERLAY_MARGIN),
        ),
    };

    for mut node in &mut overlays {
        // Compared before writing: `Node` is change-detected and this runs every
        // frame, so assigning unconditionally would dirty UI layout forever over
        // a value that only moves on a resize or a rotation.
        if node.left != left || node.top != top || node.right != right || node.bottom != bottom {
            node.left = left;
            node.top = top;
            node.right = right;
            node.bottom = bottom;
        }
    }
}

/// The canonical developer action (F6 by default) toggles the FPS overlay by writing to
/// [`UserSettings::video::show_fps`]. The next
/// `sync_fps_overlay_state_from_settings` tick mirrors the change into
/// `FpsOverlayState`, and `ambition_platformer2d::persistence::settings::persistence` autosaves the
/// new value so the toggle survives a restart.
fn toggle_fps_overlay_from_hotkey(
    mut actions: MessageReader<DeveloperAction>,
    mut settings: ResMut<UserSettings>,
) {
    if actions
        .read()
        .any(|action| *action == DeveloperAction::ToggleFpsOverlay)
    {
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

/// Min/mean/max of a diagnostic's history window. The window size is
/// whatever `FrameTimeDiagnosticsPlugin` is configured with (Bevy's
/// default is ~120 samples = ~2 s at 60 Hz), so the stats reflect
/// recent gameplay rather than the entire session.
///
/// Returns `None` when the diagnostic has no samples yet — the
/// overlay falls back to showing dashes in that brief startup
/// window. Tested via `window_stats_from_iter` against a fixture
/// iterator so we don't have to construct a real `Diagnostic`.
fn window_stats_from_iter(values: impl IntoIterator<Item = f64>) -> Option<(f64, f64, f64)> {
    let mut count = 0_u32;
    let mut sum = 0.0_f64;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for v in values {
        count += 1;
        sum += v;
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    if count == 0 {
        None
    } else {
        Some((min, sum / f64::from(count), max))
    }
}

/// Pull min/mean/max out of a Bevy diagnostic's history window.
/// Thin wrapper over [`window_stats_from_iter`] that dereferences
/// the `&f64` items the diagnostic exposes.
fn window_stats(diagnostic: &bevy::diagnostic::Diagnostic) -> Option<(f64, f64, f64)> {
    window_stats_from_iter(diagnostic.values().copied())
}

/// Read `FPS` + `FRAME_TIME` from the diagnostics store and write the
/// overlay's two-line summary. Format:
///
/// ```text
/// FPS    60.0  min 58  max 62
/// frame  16.6  min 16.0  max 17.2 ms
/// ```
///
/// The middle column is the moving-window mean over the diagnostic
/// history (≈2 s by default); `min` / `max` show the worst- and
/// best-case sample in the same window so a single hitched frame
/// shows up as an outlier without polluting the mean.
fn update_fps_overlay_text(
    time: Res<Time>,
    state: Res<FpsOverlayState>,
    diagnostics: Res<DiagnosticsStore>,
    mut elapsed: Local<f32>,
    mut query: Query<&mut Text, With<FpsOverlayText>>,
) {
    if !state.visible {
        return;
    }
    *elapsed += time.delta_secs();
    if *elapsed < FPS_OVERLAY_REFRESH_SECONDS && !state.is_changed() {
        return;
    }
    *elapsed = 0.0;
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    let fps_line = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(window_stats)
        .map(|(min, mean, max)| format!("FPS    {mean:>5.1}  min {min:>3.0}  max {max:>3.0}"))
        .unwrap_or_else(|| "FPS    --     min  --   max  --".to_owned());
    let frame_line = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(window_stats)
        .map(|(min, mean, max)| format!("frame  {mean:>5.1}  min {min:>4.1} max {max:>4.1} ms"))
        .unwrap_or_else(|| "frame  --     min  --   max  --   ms".to_owned());
    let next = format!("{fps_line}\n{frame_line}");
    if text.as_str() != next.as_str() {
        text.0 = next;
    }
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
        let settings = UserSettings::default();
        assert!(
            settings.video.show_fps,
            "VideoSettings::show_fps default must be true so the overlay shows out of the box",
        );
    }

    /// Empty history → no stats.
    #[test]
    fn window_stats_returns_none_when_empty() {
        assert_eq!(window_stats_from_iter(Vec::<f64>::new()), None);
    }

    /// Min / mean / max across a small fixture window. Spot-checks
    /// the math without needing to construct a Bevy `Diagnostic`.
    #[test]
    fn window_stats_computes_min_mean_max() {
        let stats = window_stats_from_iter([60.0, 58.0, 62.0, 60.0]).unwrap();
        assert_eq!(stats.0, 58.0, "min");
        assert!((stats.1 - 60.0).abs() < 0.001, "mean ≈ 60");
        assert_eq!(stats.2, 62.0, "max");
    }

    #[test]
    fn window_stats_exposes_outliers_without_burying_mean() {
        // 99 nominal samples + 1 hitch.
        let mut values: Vec<f64> = vec![60.0; 99];
        values.push(15.0);
        let (min, mean, max) = window_stats_from_iter(values).unwrap();
        assert_eq!(min, 15.0, "outlier visible as min");
        assert_eq!(max, 60.0);
        // Mean stays close to the nominal — outlier only shifts it
        // by ~0.45 over 100 samples.
        assert!(
            (mean - 59.55).abs() < 0.01,
            "mean lightly dragged by hitch; got {}",
            mean,
        );
    }
}
