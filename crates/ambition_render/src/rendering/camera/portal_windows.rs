use ambition_platformer2d_shared_tangle::{camera_layers::MainCamera, lifecycle::LiveRoomOf};
use ambition_portal2d_presentation::{PortalObserverViews, PortalWindowView};
use ambition_sim_view::{LocalView, LocalViewId, PresentsView, ViewsOnHand};
use bevy::camera::{CameraProjection, RenderTarget};
use bevy::prelude::{
    Camera, Camera2d, Entity, Has, OrthographicProjection, Projection, Query, Rect, Res, ResMut,
    Transform, Vec2, With,
};

use super::PresentedViewFrames;
use crate::rendering::actors::rigged::AppearanceSampleCamera;
#[cfg(feature = "card_view")]
use crate::rendering::card_view::{
    card_camera_projection, card_camera_transform, CardViewMode, CardViewSettings,
};

/// Publish the play-plane coverage after the view's final pose and zoom.
pub(in crate::rendering) fn publish_portal_window_views(
    mut windows: ResMut<PortalObserverViews>,
    frames: Res<PresentedViewFrames>,
    views: Query<(Entity, &LocalViewId), With<LocalView>>,
    cameras: Query<
        (
            &Camera,
            &RenderTarget,
            &Projection,
            &Transform,
            Option<&PresentsView>,
            Has<AppearanceSampleCamera>,
        ),
        (With<MainCamera>, With<Camera2d>),
    >,
    rooms: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    #[cfg(feature = "card_view")] cards: Option<Res<CardViewSettings>>,
) {
    let on_hand = ViewsOnHand::survey(views.iter().map(|(view, _)| view));
    windows.set_windows(cameras.iter().filter_map(
        |(camera, target, projection, transform, link, samples)| {
            if !(camera.is_active || samples)
                || !matches!(target, RenderTarget::Window(_) | RenderTarget::Image(_))
            {
                return None;
            }
            let observer = on_hand.presented_by(link.copied())?;
            let frame = frames.of(observer)?;
            let room = rooms.in_room(frame.room)?;
            let Projection::Orthographic(plane) = projection else {
                return None;
            };
            let physical = camera.physical_viewport_size()?;
            let output_pixels = physical.as_vec2();
            let render_scale = camera.target_scaling_factor()?;
            let right = Vec2::from_angle(frame.roll_radians);
            let plane_coverage = || {
                plane_bounds(
                    plane.clone(),
                    output_pixels / render_scale,
                    transform,
                    right,
                )
            };
            #[cfg(not(feature = "card_view"))]
            let coverage = plane_coverage()?;
            #[cfg(feature = "card_view")]
            let coverage = match cards.as_deref() {
                Some(cards)
                    if matches!(cards.mode, CardViewMode::SideOn | CardViewMode::Gameplay) =>
                {
                    card_bounds(cards, frame, output_pixels / render_scale, right)?
                }
                Some(cards)
                    if cards.mode == CardViewMode::Inset
                        && *views.get(observer).ok()?.1 == LocalViewId::FIRST =>
                {
                    let plane = plane_coverage()?;
                    cards
                        .inset_size(physical)
                        .and_then(|inset| {
                            card_bounds(cards, frame, inset.as_vec2() / render_scale, right)
                        })
                        .map_or(plane, |card| plane.union(card))
                }
                _ => plane_coverage()?,
            };
            let size = coverage.size();
            let center = right * coverage.center().x + right.perp() * coverage.center().y;
            let flip_y = Vec2::new(1.0, -1.0);
            Some((
                observer,
                PortalWindowView {
                    center_world: room.0.size * 0.5 + center * flip_y,
                    visible_view: size,
                    axis: right * flip_y,
                    output_pixels,
                },
            ))
        },
    ));
}

#[cfg(feature = "card_view")]
fn card_bounds(
    cards: &CardViewSettings,
    frame: &super::PresentedViewFrame,
    viewport: Vec2,
    right: Vec2,
) -> Option<Rect> {
    let projection = OrthographicProjection {
        scale: cards.projection_scale(frame.orthographic_scale),
        ..card_camera_projection()
    };
    plane_bounds(
        projection,
        viewport,
        &card_camera_transform(frame, cards),
        right,
    )
}

/// Enclose the camera's viewport on the XY plane in the view's roll axes.
/// Tilt can make the projected viewport a parallelogram.
fn plane_bounds(
    mut projection: OrthographicProjection,
    viewport: Vec2,
    camera: &Transform,
    right: Vec2,
) -> Option<Rect> {
    let back = camera.local_z();
    if back.z.abs() < f32::EPSILON {
        return None;
    }
    projection.update(viewport.x, viewport.y);
    let area = projection.area;
    let origin = camera.translation;
    let on_plane = |corner: Vec2| {
        let point = origin + camera.rotation * corner.extend(0.0);
        let hit = (point - back.as_vec3() * (point.z / back.z)).truncate();
        Vec2::new(hit.dot(right), hit.dot(right.perp()))
    };
    let [a, b, c, d] = [
        area.min,
        area.max,
        Vec2::new(area.min.x, area.max.y),
        Vec2::new(area.max.x, area.min.y),
    ]
    .map(on_plane);
    Some(Rect {
        min: a.min(b).min(c).min(d),
        max: a.max(b).max(c).max(d),
    })
}

#[cfg(all(test, feature = "card_view"))]
mod tests {
    use super::card_bounds;
    use crate::rendering::camera::PresentedViewFrame;
    use crate::rendering::card_view::{CardViewMode, CardViewSettings};
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
    use bevy::prelude::Vec2;

    #[test]
    fn tilted_output_covers_visible_ground_outside_the_side_on_view() {
        let frame = PresentedViewFrame {
            room: LiveRoomInstance::ACTIVATION,
            plane_center: Vec2::new(150.0, 425.0),
            roll_radians: std::f32::consts::FRAC_PI_2,
            orthographic_scale: 0.75,
        };
        let right = Vec2::from_angle(frame.roll_radians);
        let viewport = Vec2::new(400.0, 800.0);
        let mut settings = CardViewSettings::for_game(CardViewMode::SideOn);
        let side = card_bounds(&settings, &frame, viewport, right).unwrap();
        let center = Vec2::new(
            frame.plane_center.dot(right),
            frame.plane_center.dot(right.perp()),
        );
        assert!(side.center().abs_diff_eq(center, 0.001));
        assert!(side
            .size()
            .abs_diff_eq(viewport * frame.orthographic_scale, 0.001));
        settings.mode = CardViewMode::Gameplay;
        settings.yaw_radians = 30.0_f32.to_radians();
        settings.pitch_radians = 0.0;
        let tilted = card_bounds(&settings, &frame, viewport, right).unwrap();
        let sample = center + Vec2::Y * 335.0;
        assert!(!side.contains(sample));
        assert!(tilted.contains(sample));
    }
}
