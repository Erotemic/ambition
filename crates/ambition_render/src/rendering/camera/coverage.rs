use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_sim_view::{LocalView, LocalViewId, PresentsView, ViewsOnHand};
use bevy::camera::{visibility::RenderLayers, CameraProjection, RenderTarget};
use bevy::ecs::system::SystemParam;
#[cfg(feature = "portal_render")]
use bevy::prelude::Mat2;
use bevy::prelude::{
    Camera, Camera2d, Component, Entity, GlobalTransform, Has, OrthographicProjection, Projection,
    Query, Rect, Res, Transform, UVec2, Vec2, Vec3, With,
};

use super::PresentedViewFrame;
use super::PresentedViewFrames;
#[cfg(feature = "card_view")]
use crate::rendering::card_view::{
    card_camera_projection, card_camera_transform, CardViewMode, CardViewSettings,
};

/// The plane output that supplies layout data while a card camera draws.
#[derive(Component)]
pub(crate) struct ReplacedPlaneCamera;

pub(in crate::rendering) struct PlayPlane([Vec2; 4]);

impl PlayPlane {
    /// Intersect the four orthographic view rays with the XY plane.
    pub fn intersect(camera: &GlobalTransform, area: Rect) -> Option<Self> {
        let back = camera.affine().transform_vector3(Vec3::Z);
        if back.z.abs() < f32::EPSILON {
            return None;
        }
        let corners = [
            area.min,
            area.max,
            Vec2::new(area.min.x, area.max.y),
            Vec2::new(area.max.x, area.min.y),
        ];
        Some(Self(corners.map(|corner| {
            let point = camera.transform_point(corner.extend(0.0));
            (point - back * (point.z / back.z)).truncate()
        })))
    }

    pub fn bounds(&self, right: Vec2) -> Rect {
        let [a, b, c, d] = self
            .0
            .map(|point| Vec2::new(point.dot(right), point.dot(right.perp())));
        Rect {
            min: a.min(b).min(c).min(d),
            max: a.max(b).max(c).max(d),
        }
    }
}

pub(in crate::rendering) struct OutputFootprint {
    plane: PlayPlane,
    #[cfg(feature = "portal_render")]
    pixels: UVec2,
}

impl OutputFootprint {
    pub fn bounds(&self, right: Vec2) -> Rect {
        self.plane.bounds(right)
    }

    /// Get the pixel density along two axes in the play plane.
    #[cfg(feature = "portal_render")]
    pub fn texels_per_world(&self, right: Vec2) -> Option<Vec2> {
        let [base, _, top, east] = self.plane.0;
        let pixels = self.pixels.as_vec2();
        let world_per_pixel = Mat2::from_cols((east - base) / pixels.x, (top - base) / pixels.y);
        let area = world_per_pixel.determinant();
        if area == 0.0 || !area.is_finite() {
            return None;
        }
        let pixels_per_world = world_per_pixel.inverse();
        let density = Vec2::new(
            (pixels_per_world * right).length(),
            (pixels_per_world * right.perp()).length(),
        );
        density.is_finite().then_some(density)
    }
}

pub(in crate::rendering) struct GameplayCoverage<'a> {
    pub source: Entity,
    pub observer: Entity,
    pub frame: &'a PresentedViewFrame,
    pub layers: Option<&'a RenderLayers>,
    pub plane: Option<OutputFootprint>,
    pub cards: Option<OutputFootprint>,
}

/// The play plane shown by each gameplay output. The card output uses the
/// same scene mask as its plane source and has its own footprint.
#[derive(SystemParam)]
pub struct GameplayOutputs<'w, 's> {
    frames: Option<Res<'w, PresentedViewFrames>>,
    views: Query<'w, 's, (Entity, &'static LocalViewId), With<LocalView>>,
    cameras: Query<
        'w,
        's,
        (
            Entity,
            &'static Camera,
            &'static RenderTarget,
            &'static Projection,
            &'static Transform,
            Option<&'static PresentsView>,
            Option<&'static RenderLayers>,
            Has<ReplacedPlaneCamera>,
        ),
        (With<MainCamera>, With<Camera2d>),
    >,
    #[cfg(feature = "card_view")]
    cards: Option<Res<'w, CardViewSettings>>,
}

impl GameplayOutputs<'_, '_> {
    pub(in crate::rendering) fn coverage(&self) -> impl Iterator<Item = GameplayCoverage<'_>> {
        let on_hand = ViewsOnHand::survey(self.views.iter().map(|(view, _)| view));
        self.cameras.iter().filter_map(
            move |(source, camera, target, projection, place, link, layers, replaced)| {
                if !(camera.is_active || replaced)
                    || !matches!(target, RenderTarget::Window(_) | RenderTarget::Image(_))
                {
                    return None;
                }
                let observer = on_hand.presented_by(link.copied())?;
                let (_, _view_id) = self.views.get(observer).ok()?;
                let frame = self.frames.as_deref()?.of(observer)?;
                let Projection::Orthographic(projection) = projection else {
                    return None;
                };
                let pixels = camera.physical_viewport_size()?;
                let factor = camera.target_scaling_factor()?;
                let plane = || footprint(projection.clone(), pixels, factor, *place);
                #[cfg(not(feature = "card_view"))]
                let (plane, cards) = (plane(), None);
                #[cfg(feature = "card_view")]
                let (plane, cards) = match self.cards.as_deref() {
                    Some(settings) if settings.mode.replaces_plane() => {
                        (None, card_footprint(settings, frame, pixels, factor))
                    }
                    Some(settings)
                        if settings.mode == CardViewMode::Inset
                            && *_view_id == LocalViewId::FIRST =>
                    {
                        let inset = settings.inset_size(pixels).and_then(|size| {
                            card_footprint(settings, frame, size, factor)
                        });
                        (plane(), inset)
                    }
                    _ => (plane(), None),
                };
                Some(GameplayCoverage {
                    source,
                    observer,
                    frame,
                    layers,
                    plane,
                    cards,
                })
            },
        )
    }

    /// The world rectangle covered by all outputs of each observer.
    pub(in crate::rendering) fn view_bounds(&self) -> std::collections::BTreeMap<Entity, (&PresentedViewFrame, Rect)> {
        let mut views = std::collections::BTreeMap::<Entity, (&PresentedViewFrame, Rect)>::new();
        for output in self.coverage() {
            for footprint in output.plane.iter().chain(output.cards.iter()) {
                let bounds = footprint.bounds(Vec2::X);
                views.entry(output.observer)
                    .and_modify(|(_, old)| *old = old.union(bounds))
                    .or_insert((output.frame, bounds));
            }
        }
        views
    }
}

fn footprint(
    mut projection: OrthographicProjection,
    pixels: UVec2,
    scale_factor: f32,
    place: Transform,
) -> Option<OutputFootprint> {
    if pixels.min_element() == 0 || !scale_factor.is_finite() || scale_factor <= 0.0 {
        return None;
    }
    let viewport = pixels.as_vec2() / scale_factor;
    projection.update(viewport.x, viewport.y);
    let plane = PlayPlane::intersect(&GlobalTransform::from(place), projection.area)?;
    Some(OutputFootprint {
        plane,
        #[cfg(feature = "portal_render")]
        pixels,
    })
}

#[cfg(feature = "card_view")]
fn card_footprint(
    settings: &CardViewSettings,
    frame: &PresentedViewFrame,
    pixels: UVec2,
    scale_factor: f32,
) -> Option<OutputFootprint> {
    let projection = OrthographicProjection {
        scale: settings.projection_scale(frame.orthographic_scale),
        ..card_camera_projection()
    };
    footprint(
        projection,
        pixels,
        scale_factor,
        card_camera_transform(frame, settings),
    )
}

#[cfg(all(test, feature = "card_view", feature = "portal_render"))]
mod tests {
    use super::{card_footprint, footprint, CardViewMode, CardViewSettings, PresentedViewFrame};
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
    use bevy::prelude::{OrthographicProjection, Transform, UVec2, Vec2};

    #[test]
    fn zoom_and_tilt_change_the_inset_pixel_density_independently_of_its_bounds() {
        let frame = PresentedViewFrame {
            room: LiveRoomInstance::ACTIVATION,
            plane_center: Vec2::ZERO,
            roll_radians: 0.0,
            orthographic_scale: 1.0,
        };
        let settings = CardViewSettings {
            viewport_fraction: 0.5,
            zoom: 6.0,
            yaw_radians: 60_f32.to_radians(),
            pitch_radians: 0.0,
            ..CardViewSettings::for_game(CardViewMode::Inset)
        };
        let pixels = UVec2::new(1280, 800);
        let plane = footprint(
            OrthographicProjection::default_2d(),
            pixels,
            2.0,
            Transform::IDENTITY,
        )
        .unwrap();
        let inset = card_footprint(&settings, &frame, settings.inset_size(pixels).unwrap(), 2.0)
            .unwrap();

        assert!(plane.bounds(Vec2::X).contains(inset.bounds(Vec2::X).max));
        assert_eq!(plane.texels_per_world(Vec2::X), Some(Vec2::splat(2.0)));
        let inset_density = inset.texels_per_world(Vec2::X).unwrap();
        assert!(inset_density.abs_diff_eq(Vec2::new(3.0, 6.0), 1e-4), "{inset_density:?}");
    }
}
