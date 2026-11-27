use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_sim_view::{LocalView, LocalViewId, PresentsView, ViewsOnHand};
use bevy::camera::{visibility::RenderLayers, CameraProjection, RenderTarget};
use bevy::ecs::system::SystemParam;
#[cfg(feature = "portal_render")]
use bevy::prelude::UVec2;
use bevy::prelude::{
    Camera, Camera2d, Component, Entity, GlobalTransform, Has, OrthographicProjection, Projection,
    Query, Rect, Res, Transform, Vec2, Vec3, With,
};

#[cfg(any(feature = "card_view", feature = "portal_render"))]
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

pub(in crate::rendering) struct GameplayCoverage<'a> {
    pub source: Entity,
    #[cfg(feature = "portal_render")]
    pub observer: Entity,
    #[cfg(feature = "portal_render")]
    pub frame: &'a PresentedViewFrame,
    #[cfg(feature = "portal_render")]
    pub pixels: UVec2,
    pub layers: Option<&'a RenderLayers>,
    pub plane: Option<PlayPlane>,
    pub cards: Option<PlayPlane>,
}

/// The play plane shown by each gameplay output. The card output uses the
/// same scene mask as its plane source and has its own footprint.
#[derive(SystemParam)]
pub(in crate::rendering) struct GameplayOutputs<'w, 's> {
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
    pub fn coverage(&self) -> impl Iterator<Item = GameplayCoverage<'_>> {
        let on_hand = ViewsOnHand::survey(self.views.iter().map(|(view, _)| view));
        self.cameras.iter().filter_map(
            move |(source, camera, target, projection, place, link, layers, replaced)| {
                if !(camera.is_active || replaced)
                    || !matches!(target, RenderTarget::Window(_) | RenderTarget::Image(_))
                {
                    return None;
                }
                let observer = on_hand.presented_by(link.copied())?;
                let frame = self.frames.as_deref()?.of(observer)?;
                #[cfg(not(any(feature = "card_view", feature = "portal_render")))]
                let _ = frame;
                let Projection::Orthographic(projection) = projection else {
                    return None;
                };
                let pixels = camera.physical_viewport_size()?;
                let factor = camera.target_scaling_factor()?;
                let viewport = pixels.as_vec2() / factor;
                let plane = || footprint(projection.clone(), viewport, *place);
                #[cfg(not(feature = "card_view"))]
                let (plane, cards) = (plane(), None);
                #[cfg(feature = "card_view")]
                let (plane, cards) = match self.cards.as_deref() {
                    Some(settings) if settings.mode.replaces_plane() => {
                        (None, card_footprint(settings, frame, viewport))
                    }
                    Some(settings)
                        if settings.mode == CardViewMode::Inset
                            && *self.views.get(observer).ok()?.1 == LocalViewId::FIRST =>
                    {
                        let inset = settings.inset_size(pixels).and_then(|size| {
                            card_footprint(settings, frame, size.as_vec2() / factor)
                        });
                        (plane(), inset)
                    }
                    _ => (plane(), None),
                };
                Some(GameplayCoverage {
                    source,
                    #[cfg(feature = "portal_render")]
                    observer,
                    #[cfg(feature = "portal_render")]
                    frame,
                    #[cfg(feature = "portal_render")]
                    pixels,
                    layers,
                    plane,
                    cards,
                })
            },
        )
    }
}

fn footprint(
    mut projection: OrthographicProjection,
    viewport: Vec2,
    place: Transform,
) -> Option<PlayPlane> {
    projection.update(viewport.x, viewport.y);
    PlayPlane::intersect(&GlobalTransform::from(place), projection.area)
}

#[cfg(feature = "card_view")]
fn card_footprint(
    settings: &CardViewSettings,
    frame: &PresentedViewFrame,
    viewport: Vec2,
) -> Option<PlayPlane> {
    let projection = OrthographicProjection {
        scale: settings.projection_scale(frame.orthographic_scale),
        ..card_camera_projection()
    };
    footprint(projection, viewport, card_camera_transform(frame, settings))
}
