use std::collections::HashSet;

use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_sim_view::{LocalView, LocalViewId, PresentsView, ViewsOnHand};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{CameraMainTextureUsages, CompositingSpace, RenderTarget};
use bevy::prelude::{
    Camera, Camera2d, Commands, Component, DetectChanges, Entity, Local, Query, Ref, Res, With, Without,
};

use super::{CardViewCamera, CardViewSettings, PresentedViewFrames};
use crate::rendering::actors::rigged::AppearanceSampleCamera;

// The existing private view layers end below 2048 for every LocalViewId.
const FIRST_CARD_LAYER: usize = 2048;

#[derive(Component, PartialEq, Eq)]
pub(super) enum CardScope {
    Source(Entity),
    Camera(Entity),
}

/// Use the gameplay viewport, target, and stack settings for the card view.
/// Keep the plane camera for systems that calculate placement in 2D.
pub(super) fn select_gameplay_cameras(
    mut commands: Commands,
    frames: Res<PresentedViewFrames>,
    settings: Res<CardViewSettings>,
    views: Query<Entity, With<LocalView>>,
    mut plane_cameras: Query<
        (
            Entity,
            &mut Camera,
            Ref<RenderTarget>,
            Option<&PresentsView>,
            &CameraMainTextureUsages,
            Option<&CompositingSpace>,
        ),
        (With<MainCamera>, With<Camera2d>, Without<CardViewCamera>),
    >,
    mut card_cameras: Query<
        (
            Entity,
            &PresentsView,
            &mut Camera,
            &mut RenderTarget,
            &mut CameraMainTextureUsages,
            Option<&CompositingSpace>,
        ),
        With<CardViewCamera>,
    >,
    mut hidden: Local<HashSet<Entity>>,
    mut selected: Local<HashSet<Entity>>,
) {
    selected.clear();
    let on_hand = ViewsOnHand::survey(views.iter());
    for (rig, view, mut card, mut target, mut usages, space) in card_cameras
        .iter_mut()
        .filter(|_| settings.mode.replaces_plane())
    {
        let source = plane_cameras
            .iter_mut()
            .find(|(id, camera, target, link, ..)| {
                matches!(&**target, RenderTarget::Window(_) | RenderTarget::Image(_))
                    && (camera.is_active || hidden.contains(id))
                    && on_hand.presented_by(link.copied()) == Some(view.0)
            });
        let Some((entity, mut plane, plane_target, _, plane_usages, plane_space)) =
            source.filter(|_| frames.of(view.0).is_some())
        else {
            if card.is_active {
                card.is_active = false;
            }
            continue;
        };
        let taking_over = !card.is_active || !hidden.contains(&entity);
        if plane.is_changed() || taking_over {
            card.is_active = true;
            card.order = plane.order;
            card.viewport.clone_from(&plane.viewport);
            card.clear_color = plane.clear_color;
            card.msaa_writeback = plane.msaa_writeback;
        }
        if plane_target.is_changed() || taking_over {
            *target = (*plane_target).clone();
        }
        if usages.0 != plane_usages.0 {
            usages.0 = plane_usages.0;
        }
        if space != plane_space {
            if let Some(space) = plane_space {
                commands.entity(rig).insert(*space);
            } else {
                commands.entity(rig).remove::<CompositingSpace>();
            }
        }
        if plane.is_active {
            plane.is_active = false;
        }
        if !hidden.contains(&entity) {
            commands.entity(entity).insert(AppearanceSampleCamera);
        }
        selected.insert(entity);
    }

    for entity in hidden.difference(&selected) {
        if let Ok((_, mut camera, ..)) = plane_cameras.get_mut(*entity) {
            camera.is_active = true;
            commands.entity(*entity).remove::<AppearanceSampleCamera>();
        }
    }
    std::mem::swap(&mut *hidden, &mut *selected);
}

/// Use the plane camera's isolation mask to select cards in its view.
pub(super) fn isolate_cards(
    frames: Res<PresentedViewFrames>,
    views: Query<(Entity, &LocalViewId), With<LocalView>>,
    plane_cameras: Query<
        (Option<&PresentsView>, &RenderLayers),
        (
            With<MainCamera>,
            With<Camera2d>,
            Without<CardViewCamera>,
            Without<CardScope>,
        ),
    >,
    mut card_cameras: Query<
        (Entity, &PresentsView, &Camera, &mut RenderLayers),
        (With<CardViewCamera>, Without<CardScope>),
    >,
    sources: Query<&RenderLayers, (Without<CardScope>, Without<CardViewCamera>)>,
    mut cards: Query<(&CardScope, &mut RenderLayers), Without<CardViewCamera>>,
) {
    let on_hand = ViewsOnHand::survey(views.iter().map(|(view, _)| view));
    let mut active = Vec::new();
    for (entity, view, camera, mut layers) in &mut card_cameras {
        if !camera.is_active {
            continue;
        }
        let Some((_, (_, id))) = frames.of(view.0).zip(views.get(view.0).ok()) else {
            continue;
        };
        let mask = plane_cameras.iter().find_map(|(link, mask)| {
            (on_hand.presented_by(link.copied()) == Some(view.0)).then_some(mask)
        });
        let slot = FIRST_CARD_LAYER + usize::from(id.0);
        let desired = RenderLayers::none().with(slot);
        if *layers != desired {
            *layers = desired;
        }
        active.push((entity, mask, slot));
    }

    active.sort_unstable_by_key(|(_, _, slot)| *slot);
    let world = RenderLayers::default();
    for (scope, mut layers) in &mut cards {
        let memberships = || {
            active.iter().filter_map(|&(camera, mask, slot)| {
                let shown = match scope {
                    CardScope::Source(entity) => mask
                        .is_none_or(|mask| mask.intersects(sources.get(*entity).unwrap_or(&world))),
                    CardScope::Camera(owner) => *owner == camera,
                };
                shown.then_some(slot)
            })
        };
        if !layers.iter().eq(memberships()) {
            *layers = memberships().collect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
    use crate::rendering::camera::PresentedViewFrame;
    use crate::rendering::card_view::{retire_card_outputs, CardMeshes, CardViewMode};
    use bevy::camera::Viewport;
    use bevy::prelude::{App, Assets, Image, IntoScheduleConfigs, PostUpdate, UVec2, Vec2};

    #[test]
    fn two_card_outputs_keep_their_viewports_and_release_the_plane_on_switch() {
        let mut app = App::new();
        app.insert_resource(CardViewSettings::gameplay())
            .init_resource::<CardMeshes>()
            .init_resource::<Assets<Image>>()
            .add_systems(
                PostUpdate,
                (retire_card_outputs, select_gameplay_cameras, isolate_cards).chain(),
            );
        let world = app.world_mut();
        let image = world.resource_mut::<Assets<Image>>().add(Image::default());
        let rooms = [
            LiveRoomInstance::ACTIVATION,
            LiveRoomInstance::ACTIVATION.next(),
        ];
        let mut frames = Vec::new();
        let mut outputs = Vec::new();
        for (index, room) in rooms.into_iter().enumerate() {
            let view = world.spawn((LocalView, LocalViewId(index as u8))).id();
            let mask = RenderLayers::none().with(256 + index);
            let origin = UVec2::new(index as u32 * 600, 0);
            let plane = world
                .spawn((
                    MainCamera,
                    Camera2d,
                    Camera {
                        order: index as isize,
                        viewport: Some(Viewport {
                            physical_position: origin,
                            physical_size: UVec2::new(600, 600),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    PresentsView(view),
                    mask.clone(),
                ))
                .id();
            if index == 1 {
                world.entity_mut(plane).insert(RenderTarget::Image(image.clone().into()));
            }
            let card = world
                .spawn((
                    CardViewCamera,
                    Camera::default(),
                    PresentsView(view),
                    RenderLayers::none(),
                ))
                .id();
            let source = world.spawn(mask).id();
            let model = world
                .spawn((CardScope::Source(source), RenderLayers::none()))
                .id();
            frames.push((
                view,
                PresentedViewFrame {
                    room,
                    plane_center: Vec2::ZERO,
                    roll_radians: 0.0,
                    orthographic_scale: 1.0,
                },
            ));
            outputs.push((plane, card, model, origin));
        }
        world.insert_resource(PresentedViewFrames::of_views(frames));
        world.run_schedule(PostUpdate);
        for (index, &(plane, card, model, origin)) in outputs.iter().enumerate() {
            assert!(!world.get::<Camera>(plane).unwrap().is_active);
            assert!(world.get::<AppearanceSampleCamera>(plane).is_some());
            let camera = world.get::<Camera>(card).unwrap();
            assert!(camera.is_active);
            assert_eq!(camera.viewport.as_ref().unwrap().physical_position, origin);
            let target = world.get::<RenderTarget>(card).unwrap();
            if index == 1 {
                assert_eq!(target.as_image(), Some(&image));
            } else {
                assert!(matches!(target, RenderTarget::Window(_)));
            }
            let layers = world.get::<RenderLayers>(model).unwrap();
            assert!(layers.intersects(world.get::<RenderLayers>(card).unwrap()));
            assert!(!layers.intersects(world.get::<RenderLayers>(outputs[1 - index].1).unwrap()));
        }

        world.resource_mut::<CardViewSettings>().mode = CardViewMode::Plane;
        world.run_schedule(PostUpdate);
        for (plane, card, model, _) in outputs {
            assert!(world.get::<Camera>(plane).unwrap().is_active);
            assert!(world.get::<AppearanceSampleCamera>(plane).is_none());
            assert!(world.get::<Camera>(card).is_none());
            assert!(world.get::<CardScope>(model).is_none());
        }
    }
}
