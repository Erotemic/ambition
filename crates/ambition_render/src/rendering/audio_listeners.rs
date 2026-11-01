//! Project local observers into world-space audio listening positions.

use ambition_sfx::SfxListeners;
use ambition_sim_view::{camera_snapshot::ResolvedCameraSnapshot, LocalView};
use bevy::prelude::*;

/// Listen at each view's followed body, independent of camera zoom and easing.
pub fn publish_audio_listeners(
    views: Query<&ResolvedCameraSnapshot, With<LocalView>>,
    mut listeners: ResMut<SfxListeners>,
) {
    listeners.0.clear();
    for frame in views.iter().filter_map(ResolvedCameraSnapshot::frame) {
        if !listeners.0.contains(&frame.follow_world) {
            listeners.0.push(frame.follow_world);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_sim_view::camera_snapshot::{CameraSnapshot2d, ResolvedCameraFrame};
    #[test]
    fn each_local_view_hears_its_subject_and_removed_views_leave_no_listener() {
        let mut app = App::new();
        app.init_resource::<SfxListeners>()
            .add_systems(Update, publish_audio_listeners);
        let mut views = Vec::new();
        for position in [Vec2::ZERO, Vec2::new(500.0, 300.0)] {
            views.push(app.world_mut().spawn((LocalView, ResolvedCameraSnapshot(Some(ResolvedCameraFrame {
                snapshot: CameraSnapshot2d::default(), follow_world: position,
                room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION,
            })))).id());
        }
        app.update();
        assert_eq!(app.world().resource::<SfxListeners>().0.len(), 2);
        assert!(app
            .world()
            .resource::<SfxListeners>()
            .0
            .contains(&Vec2::new(500.0, 300.0)));
        for view in views {
            app.world_mut().despawn(view);
        }
        app.update();
        assert!(app.world().resource::<SfxListeners>().0.is_empty());
    }
}
