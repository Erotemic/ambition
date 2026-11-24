//! Desktop-specific platform setup.
//!
//! Mirror of [`super::android`] for desktop builds: the cursor hides while the
//! mouse rests (a controller player on a Steam Deck or a couch never sees it),
//! and shows again on the first motion, press or wheel.

use std::time::Duration;

use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorMoved, CursorOptions, Monitor, OnMonitor, PrimaryWindow, WindowMode};

/// How long the mouse rests before the cursor hides.
const CURSOR_REST: Duration = Duration::from_millis(2500);

/// Bevy plugin for desktop-only setup.
pub struct DesktopPlatformPlugin;

impl Plugin for DesktopPlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (fit_initial_window_to_monitor, hide_the_resting_cursor));
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
    mut fitted: Local<bool>,
) {
    if *fitted { return; }
    let Ok((mut window, on_monitor)) = windows.single_mut() else { return; };
    let Ok(monitor) = monitors.get(on_monitor.0) else { return; };
    if window.mode == WindowMode::Windowed {
        let available = Vec2::new(monitor.physical_width as f32, monitor.physical_height as f32)
            / monitor.scale_factor as f32;
        let requested = Vec2::new(window.width(), window.height());
        let size = requested.min(available);
        if size != requested {
            window.resolution.set(size.x, size.y);
            info!("Initial window fitted to display: {requested} -> {size}");
        }
    }
    *fitted = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_initial_window_fits_the_monitor_and_later_resizes_remain_available() {
        let mut app = App::new();
        app.add_systems(Update, fit_initial_window_to_monitor);
        let entity = app.world_mut().spawn((Window {
            resolution: (1600, 900).into(), ..default()
        }, PrimaryWindow)).id();
        app.update();
        let monitor = app.world_mut().spawn(Monitor {
            name: None, physical_width: 1280, physical_height: 800,
            physical_position: IVec2::ZERO, refresh_rate_millihertz: None,
            scale_factor: 1.0, video_modes: vec![],
        }).id();
        app.world_mut().entity_mut(entity).insert(OnMonitor(monitor));
        app.update();
        assert_eq!(app.world().get::<Window>(entity).unwrap().physical_size(), UVec2::new(1280, 800));
        app.world_mut().get_mut::<Window>(entity).unwrap().resolution.set(1440.0, 900.0);
        app.update();
        assert_eq!(app.world().get::<Window>(entity).unwrap().physical_size(), UVec2::new(1440, 900));
    }
}
