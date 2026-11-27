use std::collections::HashSet;

use ambition_platformer2d_shared_tangle::camera_layers::{MainCamera, PrivateCameraLayer};
use ambition_sim_view::{LocalView, LocalViewId, PresentsView, ViewsOnHand};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{CameraMainTextureUsages, ClearColorConfig, CompositingSpace, MsaaWriteback, RenderTarget, Viewport};
use bevy::prelude::{
    Camera, Camera2d, Commands, Component, Entity, Local, Query, Res, UVec2, With, Without,
};
use bevy::render::view::Msaa;

use super::{CardViewCamera, CardViewSettings, PresentedViewFrames};
use crate::rendering::actors::rigged::AppearanceSampleCamera;

#[derive(Component, PartialEq, Eq)]
pub(super) enum CardScope {
    Source(Entity),
    Camera(Entity),
}

#[derive(Component, PartialEq, Eq)]
pub(super) struct PlaneCameraForCard(Entity);

/// Use the gameplay viewport, target, and stack settings for the card view.
/// Keep the plane camera for systems that calculate placement in 2D.
pub(super) fn route_card_outputs(
    mut commands: Commands,
    frames: Res<PresentedViewFrames>,
    settings: Res<CardViewSettings>,
    views: Query<Entity, With<LocalView>>,
    mut plane_cameras: Query<
        (
            Entity,
            &mut Camera,
            &RenderTarget,
            Option<&PresentsView>,
            &CameraMainTextureUsages,
            Option<&CompositingSpace>,
            Option<&Msaa>,
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
            Option<&Msaa>,
            Option<&PlaneCameraForCard>,
        ),
        With<CardViewCamera>,
    >,
    mut hidden: Local<HashSet<Entity>>,
    mut selected: Local<HashSet<Entity>>,
) {
    selected.clear();
    let on_hand = ViewsOnHand::survey(views.iter());
    for (rig, view, mut card, mut target, mut usages, space, msaa, plane_link) in &mut card_cameras {
        let source = plane_cameras
            .iter_mut()
            .find(|(id, camera, target, link, ..)| {
                matches!(target, RenderTarget::Window(_) | RenderTarget::Image(_))
                    && (camera.is_active || hidden.contains(id))
                    && on_hand.presented_by(link.copied()) == Some(view.0)
            });
        let Some((entity, mut plane, plane_target, _, plane_usages, plane_space, plane_msaa)) =
            source.filter(|_| frames.of(view.0).is_some())
        else {
            if card.is_active {
                card.is_active = false;
            }
            if plane_link.is_some() {
                commands.entity(rig).remove::<PlaneCameraForCard>();
            }
            continue;
        };
        let replacing = settings.mode.replaces_plane();
        let (active, order, viewport, clear_color, writeback) = if replacing {
            (true, plane.order, plane.viewport.clone(), plane.clear_color, plane.msaa_writeback)
        } else {
            let viewport = inset_viewport(&plane, settings.viewport_fraction);
            (viewport.is_some(), plane.order.saturating_add(1).max(6), viewport, ClearColorConfig::None, MsaaWriteback::Auto)
        };
        if card.is_active != active || card.order != order || !same_viewport(&card.viewport, &viewport)
            || !same_clear_color(card.clear_color, clear_color) || card.msaa_writeback != writeback
        {
            card.is_active = active;
            card.order = order;
            card.viewport = viewport;
            card.clear_color = clear_color;
            card.msaa_writeback = writeback;
        }
        if target.normalize(None) != plane_target.normalize(None) {
            *target = plane_target.clone();
        }
        if usages.0 != plane_usages.0 {
            usages.0 = plane_usages.0;
        }
        if let Some(samples) = plane_msaa.filter(|samples| Some(*samples) != msaa) {
            commands.entity(rig).insert(*samples);
        }
        if space != plane_space {
            if let Some(space) = plane_space {
                commands.entity(rig).insert(*space);
            } else {
                commands.entity(rig).remove::<CompositingSpace>();
            }
        }
        if plane_link.is_none_or(|previous| previous.0 != entity) {
            commands.entity(rig).insert(PlaneCameraForCard(entity));
        }
        if replacing {
            if plane.is_active {
                plane.is_active = false;
            }
            if !hidden.contains(&entity) {
                commands.entity(entity).insert(AppearanceSampleCamera);
            }
            selected.insert(entity);
        }
    }

    for entity in hidden.difference(&selected) {
        if let Ok((_, mut camera, ..)) = plane_cameras.get_mut(*entity) {
            camera.is_active = true;
            commands.entity(*entity).remove::<AppearanceSampleCamera>();
        }
    }
    std::mem::swap(&mut *hidden, &mut *selected);
}

fn same_viewport(a: &Option<Viewport>, b: &Option<Viewport>) -> bool {
    let rect = |viewport: &Option<Viewport>| {
        viewport.as_ref().map(|v| (v.physical_position, v.physical_size, v.depth.clone()))
    };
    rect(a) == rect(b)
}

fn same_clear_color(a: ClearColorConfig, b: ClearColorConfig) -> bool {
    use ClearColorConfig::{Custom, Default, None};
    match (a, b) {
        (Custom(a), Custom(b)) => a == b,
        (Default, Default) | (None, None) => true,
        _ => false,
    }
}

fn inset_viewport(plane: &Camera, fraction: f32) -> Option<Viewport> {
    let size = plane.physical_viewport_size()?;
    if size.x < 64 || size.y < 64 {
        return None;
    }
    let origin = plane.viewport.as_ref().map_or(UVec2::ZERO, |v| v.physical_position);
    let w = ((size.x as f32 * fraction) as u32).clamp(32, size.x);
    let h = ((w as f32 * size.y as f32 / size.x as f32) as u32).clamp(32, size.y);
    let inset = UVec2::new(w, h);
    Some(Viewport { physical_position: origin + size - inset, physical_size: inset, ..Default::default() })
}

/// Use the plane camera's isolation mask to select cards in its view.
pub(super) fn isolate_cards(
    views: Query<(Entity, &LocalViewId), With<LocalView>>,
    mut card_cameras: Query<
        (Entity, &PresentsView, &PlaneCameraForCard, &Camera, &mut RenderLayers),
        (With<CardViewCamera>, Without<CardScope>),
    >,
    sources: Query<Option<&RenderLayers>, (Without<CardScope>, Without<CardViewCamera>)>,
    mut cards: Query<(&CardScope, &mut RenderLayers), Without<CardViewCamera>>,
) {
    let world = RenderLayers::default();
    let mut active = Vec::new();
    for (entity, view, plane, camera, mut layers) in &mut card_cameras {
        if !camera.is_active {
            continue;
        }
        let Ok((_, id)) = views.get(view.0) else {
            continue;
        };
        let Ok(mask) = sources.get(plane.0) else {
            continue;
        };
        let slot = PrivateCameraLayer::CardOutput.at(usize::from(id.0));
        let desired = RenderLayers::none().with(slot);
        if *layers != desired {
            *layers = desired;
        }
        active.push((entity, mask.unwrap_or(&world), slot));
    }

    active.sort_unstable_by_key(|(_, _, slot)| *slot);
    for (scope, mut layers) in &mut cards {
        let memberships = || {
            active.iter().filter_map(|&(camera, mask, slot)| {
                let shown = match scope {
                    CardScope::Source(entity) => sources.get(*entity)
                        .is_ok_and(|layers| mask.intersects(layers.unwrap_or(&world))),
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
    fn a_comparison_inset_uses_its_plane_target_and_default_visibility() {
        let mut app = App::new();
        app.insert_resource(CardViewSettings::for_game(CardViewMode::Inset))
            .init_resource::<Assets<Image>>()
            .add_systems(PostUpdate, (route_card_outputs, isolate_cards).chain());
        let world = app.world_mut();
        let image = world.resource_mut::<Assets<Image>>().add(Image::default());
        let view = world.spawn((LocalView, LocalViewId::FIRST)).id();
        world.insert_resource(PresentedViewFrames::of_views(vec![(
            view,
            PresentedViewFrame {
                room: LiveRoomInstance::ACTIVATION,
                plane_center: Vec2::ZERO,
                roll_radians: 0.0,
                orthographic_scale: 1.0,
            },
        )]));
        let plane = world.spawn((
            Camera2d,
            Camera {
                viewport: Some(Viewport {
                    physical_position: UVec2::new(200, 100),
                    physical_size: UVec2::new(800, 400),
                    ..Default::default()
                }),
                ..Default::default()
            },
            MainCamera,
            PresentsView(view),
            RenderTarget::Image(image.clone().into()),
            Msaa::Off,
        )).id();
        let inset = world.spawn((
            CardViewCamera,
            Camera::default(),
            PresentsView(view),
            RenderLayers::none(),
            Msaa::Sample4,
        )).id();
        let world_sprite = world.spawn_empty().id();
        let private_sprite = world.spawn(RenderLayers::none().with(123)).id();
        let visible_card = world.spawn((CardScope::Source(world_sprite), RenderLayers::none())).id();
        let private_card = world.spawn((CardScope::Source(private_sprite), RenderLayers::none())).id();

        world.run_schedule(PostUpdate);

        assert!(world.get::<Camera>(plane).unwrap().is_active);
        assert_eq!(world.get::<RenderTarget>(inset).unwrap().as_image(), Some(&image));
        assert_eq!(world.get::<Msaa>(inset), Some(&Msaa::Off));
        let camera = world.get::<Camera>(inset).unwrap();
        assert!(camera.is_active);
        let viewport = camera.viewport.as_ref().unwrap();
        assert_eq!(viewport.physical_size, UVec2::new(384, 192));
        assert_eq!(viewport.physical_position, UVec2::new(616, 308));
        let mask = world.get::<RenderLayers>(inset).unwrap();
        assert!(mask.intersects(world.get::<RenderLayers>(visible_card).unwrap()));
        assert!(!mask.intersects(world.get::<RenderLayers>(private_card).unwrap()));
    }

    #[test]
    fn two_card_outputs_keep_their_viewports_and_release_the_plane_on_switch() {
        let mut app = App::new();
        app.insert_resource(CardViewSettings::gameplay())
            .init_resource::<CardMeshes>()
            .init_resource::<Assets<Image>>()
            .add_systems(
                PostUpdate,
                (retire_card_outputs, route_card_outputs, isolate_cards).chain(),
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
