//! Desktop-specific platform setup.
//!
//! Mirror of [`super::android`] for desktop builds: the cursor hides while the
//! mouse rests (a controller player on a Steam Deck or a couch never sees it),
//! and shows again on the first motion, press or wheel.

use std::time::Duration;

use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorMoved, CursorOptions, PrimaryWindow};

/// How long the mouse rests before the cursor hides.
const CURSOR_REST: Duration = Duration::from_millis(2500);

/// Bevy plugin for desktop-only setup.
pub struct DesktopPlatformPlugin;

impl Plugin for DesktopPlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, hide_the_resting_cursor);
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
