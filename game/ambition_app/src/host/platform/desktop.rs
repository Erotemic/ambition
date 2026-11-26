//! Desktop-specific platform setup.
//!
//! Mirror of [`super::android`] for desktop builds: the cursor hides while the
//! mouse rests (a controller player on a Steam Deck or a couch never sees it),
//! and shows again on the first motion, press or wheel.

use std::time::Duration;

use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorMoved, CursorOptions, Monitor, MonitorSelection, OnMonitor, PrimaryWindow, WindowMode};

use ambition_platformer2d_shared_tangle::gameplay_presentation::PresentationEnvironment;

/// How long the mouse rests before the cursor hides.
const CURSOR_REST: Duration = Duration::from_millis(2500);

/// Bevy plugin for desktop-only setup.
pub struct DesktopPlatformPlugin;

impl Plugin for DesktopPlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (fit_initial_window_to_monitor, hide_the_resting_cursor));
        app.add_systems(PostUpdate, log_window_size);
    }
}

/// Show the primary window's cursor from a mouse motion, press or wheel until
/// the mouse has rested [`CURSOR_REST`]; hidden before the mouse first moves.
/// Writes only on a change of visibility.
fn hide_the_resting_cursor(
    time: Res<Time<Real>>,
    mut moved: MessageReader<CursorMoved>,
    mut pressed: MessageReader<MouseButtonInput>,
    mut wheeled: MessageReader<MouseWheel>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut last_used: Local<Option<Duration>>,
) {
    let used = moved.read().count() + pressed.read().count() + wheeled.read().count() > 0;
    let now = time.elapsed();
    if used {
        *last_used = Some(now);
    }
    let visible = last_used.is_some_and(|at| now.saturating_sub(at) < CURSOR_REST);
    for mut cursor in &mut cursors {
        if cursor.visible != visible {
            cursor.visible = visible;
        }
    }
}

/// Fit the initial window to its display, then leave user resizes alone.
fn fit_initial_window_to_monitor(
    monitors: Query<&Monitor>,
    mut windows: Query<(&mut Window, &OnMonitor), With<PrimaryWindow>>,
    environment: Option<Res<PresentationEnvironment>>,
    mut fitted: Local<bool>,
) {
    if *fitted { return; }
    let Ok((mut window, on_monitor)) = windows.single_mut() else { return; };
    let Ok(monitor) = monitors.get(on_monitor.0) else { return; };
    if window.mode == WindowMode::Windowed {
        let gaming_mode = std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|desktop| {
            desktop.split(':').any(|name| name.eq_ignore_ascii_case("gamescope"))
        });
        if gaming_mode && environment.as_deref() == Some(&PresentationEnvironment::Handheld) {
            window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
        } else {
            let available = UVec2::new(monitor.physical_width, monitor.physical_height);
            let requested = window.physical_size();
            let size = requested.min(available);
            if size != requested {
                window.resolution.set_physical_resolution(size.x, size.y);
            }
        }
    }
    *fitted = true;
}

fn log_window_size(
    windows: Query<&Window, (With<PrimaryWindow>, Changed<Window>)>,
    mut last: Local<Option<(UVec2, f32, WindowMode)>>,
) {
    let Ok(window) = windows.single() else { return; };
    let size = (window.physical_size(), window.scale_factor(), window.mode);
    if *last != Some(size) {
        *last = Some(size);
        info!(
            "Window: {:?}, {} physical px, {} logical px, DPI {}",
            window.mode, window.physical_size(), window.size(), window.scale_factor(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_initial_window_fits_the_monitor_and_later_resizes_remain_available() {
        let mut app = App::new();
        app.add_systems(Update, fit_initial_window_to_monitor);
        let entity = app.world_mut().spawn((Window {
            resolution: bevy::window::WindowResolution::new(1600, 900)
                .with_scale_factor_override(1.0),
            ..default()
        }, PrimaryWindow)).id();
        app.update();
        let monitor = app.world_mut().spawn(Monitor {
            name: None, physical_width: 1280, physical_height: 800,
            physical_position: IVec2::ZERO, refresh_rate_millihertz: None,
            scale_factor: 1.25, video_modes: vec![],
        }).id();
        app.world_mut().entity_mut(entity).insert(OnMonitor(monitor));
        app.update();
        assert_eq!(app.world().get::<Window>(entity).unwrap().physical_size(), UVec2::new(1280, 800));
        app.world_mut().get_mut::<Window>(entity).unwrap().resolution.set(1440.0, 900.0);
        app.update();
        assert_eq!(app.world().get::<Window>(entity).unwrap().physical_size(), UVec2::new(1440, 900));
    }
}
