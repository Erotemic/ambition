use bevy::camera::visibility::RenderLayers;
use bevy::ecs::{query::QueryData, system::SystemParam};
use bevy::prelude::*;

#[cfg(feature = "card_view")]
use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
#[cfg(feature = "card_view")]
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
#[cfg(feature = "card_view")]
use crate::rendering::card_view::{CardRootDepths, PlaneCameraForCard};
use crate::rendering::GameplayOutputCamera;

#[derive(QueryData)]
pub(crate) struct FieldSite {
    entity: Entity,
    at: &'static GlobalTransform,
    visibility: Option<&'static InheritedVisibility>,
    layers: Option<&'static RenderLayers>,
}

#[derive(SystemParam)]
pub(crate) struct FieldViews<'w, 's> {
    cameras: Query<'w, 's, (
        Entity, &'static Camera, &'static GlobalTransform, Option<&'static RenderLayers>,
    ), GameplayOutputCamera>,
    #[cfg(feature = "card_view")]
    links: Query<'w, 's, &'static PlaneCameraForCard>,
    #[cfg(feature = "card_view")]
    plane_layers: Query<'w, 's, Option<&'static RenderLayers>, With<MainCamera>>,
    #[cfg(feature = "card_view")]
    depths: Option<Res<'w, CardRootDepths>>,
    #[cfg(feature = "card_view")]
    rooms: LiveRooms<'w, 's>,
}

pub(crate) struct FieldView<'a> {
    pub camera: &'a Camera,
    at: &'a GlobalTransform,
    rect: Rect,
    layers: Option<&'a RenderLayers>,
    #[cfg(feature = "card_view")]
    card_output: bool,
}

impl FieldView<'_> {
    pub fn aspect(&self) -> f32 {
        self.rect.width() / self.rect.height()
    }
}

impl FieldViews<'_, '_> {
    pub fn iter(&self) -> impl Iterator<Item = (Entity, Option<FieldView<'_>>)> {
        self.cameras.iter().map(|(entity, camera, at, layers)| {
            let view = || {
                if !camera.is_active {
                    return None;
                }
                let rect = camera.logical_viewport_rect()?;
                if rect.width() <= 0.0 || rect.height() <= 0.0 {
                    return None;
                }
                // A card has a private output layer. The plane camera owns
                // the room mask and can be inactive while the card draws.
                #[cfg(feature = "card_view")]
                let (layers, card_output) = match self.links.get(entity) {
                    Ok(plane) => (self.plane_layers.get(plane.0).ok()?, true),
                    Err(_) if self.plane_layers.contains(entity) => (layers, false),
                    Err(_) => return None,
                };
                Some(FieldView {
                    camera, at, rect, layers,
                    #[cfg(feature = "card_view")]
                    card_output,
                })
            };
            (entity, view())
        })
    }

    /// Project the field from its visible site to the output's own viewport.
    pub fn project(&self, view: &FieldView, site: FieldSiteItem, radius: f32, strength: f32) -> Option<Vec4> {
        if site.visibility.is_some_and(|visible| !visible.get()) || !drawn_through(view.layers, site.layers) {
            return None;
        }
        let middle = site.at.translation();
        #[cfg(feature = "card_view")]
        let middle = if let Some(depths) = self.depths.as_deref().filter(|_| view.card_output) {
            middle.truncate().extend(depths.field_depth(site.entity, middle.z, self.rooms.of(site.entity)))
        } else {
            middle
        };
        let center = view.camera.world_to_ndc(view.at, middle)?;
        if !(0.0..=1.0).contains(&center.z) {
            return None;
        }
        // A wide field can extend past a depth plane. Its edge still gives
        // the projected radius of the field.
        let edge = view.camera.world_to_ndc(view.at, middle + Vec3::X * radius)?;
        let size = view.rect.size();
        let half = size * 0.5;
        let at = (center.truncate() * Vec2::new(1.0, -1.0) + Vec2::ONE) * half;
        let radius_px = ((edge.truncate() - center.truncate()) * half).length();
        source_in_view(size, at, radius_px, strength)
    }
}

/// A field of another room cannot change the pixels of this view.
fn drawn_through(camera: Option<&RenderLayers>, site: Option<&RenderLayers>) -> bool {
    let world = RenderLayers::default();
    camera.unwrap_or(&world).intersects(site.unwrap_or(&world))
}

/// Pack a source in viewport coordinates. A field outside the viewport has no source.
pub(crate) fn source_in_view(view_px: Vec2, middle_px: Vec2, radius_px: f32, strength: f32) -> Option<Vec4> {
    if view_px.x <= 0.0 || view_px.y <= 0.0 || !radius_px.is_finite() || radius_px <= 0.0 || strength <= 0.0 {
        return None;
    }
    let outside = middle_px.x < -radius_px
        || middle_px.y < -radius_px
        || middle_px.x > view_px.x + radius_px
        || middle_px.y > view_px.y + radius_px;
    if outside {
        return None;
    }
    Some(Vec4::new(
        middle_px.x / view_px.x,
        middle_px.y / view_px.y,
        radius_px / view_px.y,
        strength.clamp(0.0, 1.0),
    ))
}

/// Pack the sources nearest to the middle of the viewport.
pub(crate) fn nearest<const N: usize>(mut seen: Vec<Vec4>) -> ([Vec4; N], usize) {
    let from_middle = |source: &Vec4| (source.truncate().truncate() - Vec2::splat(0.5)).length_squared();
    seen.sort_by(|a, b| from_middle(a).total_cmp(&from_middle(b)));
    seen.truncate(N);
    let mut packed = [Vec4::ZERO; N];
    packed[..seen.len()].copy_from_slice(&seen);
    (packed, seen.len())
}
