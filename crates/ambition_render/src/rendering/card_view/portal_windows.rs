use ambition_portal2d_presentation::{PortalClipMaterial, PortalConeMesh};
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::prelude::{
    Assets, ColorMaterial, Commands, Component, DetectChangesMut, Entity, Local, Mesh2d,
    Mesh3d, MeshMaterial2d, MeshMaterial3d, Query, Res, ResMut, Transform,
    Visibility, With, Without,
};

use super::paint_order::CardPaintDepth;
use super::world_mesh::CardMaterial;
use super::{CardRootDepths, CardScope, RetainedEntities};
use ambition_platformer2d_shared_tangle::camera_layers::CARD_VIEW_LAYER;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;

#[derive(Component)]
pub(super) struct CardWindow;

/// Place the existing portal picture on the play plane.
pub(super) fn realize_portal_windows(
    mut commands: Commands,
    windows: Query<
        (Entity, &Mesh2d, &MeshMaterial2d<ColorMaterial>, &Transform, &Visibility),
        With<PortalConeMesh>,
    >,
    mut cards: Query<
        (
            &mut Mesh3d,
            &MeshMaterial3d<CardMaterial<PortalClipMaterial>>,
            &mut Transform,
            &mut Visibility,
            &mut CardPaintDepth,
        ),
        (With<CardWindow>, Without<PortalConeMesh>),
    >,
    plane_materials: Res<Assets<ColorMaterial>>,
    mut materials: ResMut<Assets<CardMaterial<PortalClipMaterial>>>,
    mut products: Local<RetainedEntities<Entity>>,
    depths: Res<CardRootDepths>,
    rooms: LiveRooms,
) {
    products.reconcile(&mut commands, |source| windows.contains(*source));
    for (source, mesh, material, transform, visible) in &windows {
        let product = products
            .get(&source)
            .and_then(|product| cards.get_mut(*product).ok());
        let Some(material) = plane_materials.get(&material.0) else {
            if let Some((.., mut visibility, _)) = product {
                visibility.set_if_neq(Visibility::Hidden);
            }
            continue;
        };
        // A closed cone keeps an empty mesh. Keep its pipeline in use so the
        // first open frame can use it.
        let picture = PortalClipMaterial::window(material.color, material.texture.clone().unwrap_or_default());
        // The window stays at the seam. Its picture replaces the scene under
        // the wedge after the bodies and their portal pieces draw.
        let depth = depths.scenery(transform.translation.z, rooms.of(source)).on_play_plane();
        let place = depth.place(transform);
        if let Some((mut polygon, image, mut position, mut visibility, mut order)) = product {
            if polygon.0 != mesh.0 {
                polygon.0 = mesh.0.clone();
            }
            position.set_if_neq(place);
            visibility.set_if_neq(*visible);
            order.set_if_neq(depth.paint);
            if materials.get(&image.0).is_some_and(|material| material.0 != picture) {
                materials.insert(image.0.id(), CardMaterial(picture)).ok();
            }
        } else {
            let product = commands
                .spawn((
                    CardWindow,
                    depth.paint,
                    CardScope::Source(source),
                    Mesh3d(mesh.0.clone()),
                    MeshMaterial3d(materials.add(CardMaterial(picture))),
                    place,
                    *visible,
                    NoFrustumCulling,
                    RenderLayers::layer(CARD_VIEW_LAYER),
                ))
                .id();
            products.insert(source, product);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_portal2d_presentation::PORTAL_WINDOW_Z;
    use bevy::prelude::{App, Handle, Image, Mesh, Rectangle, Update};

    #[test]
    fn a_portal_card_retains_its_mesh_and_material_when_the_window_opens() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<Assets<CardMaterial<PortalClipMaterial>>>()
            .init_resource::<CardRootDepths>()
            .add_systems(Update, realize_portal_windows);
        let image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Rectangle::new(0.0, 0.0));
        let material = app
            .world_mut()
            .resource_mut::<Assets<ColorMaterial>>()
            .add(ColorMaterial::default());
        let source = app
            .world_mut()
            .spawn((
                PortalConeMesh,
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material.clone()),
                Transform::from_xyz(20.0, 10.0, PORTAL_WINDOW_Z),
                Visibility::Inherited,
            ))
            .id();
        app.update();
        let world = app.world_mut();
        let mut cards = world.query_filtered::<(Entity, &Mesh3d, &MeshMaterial3d<CardMaterial<PortalClipMaterial>>), With<CardWindow>>();
        let (card, polygon, photo) = cards.single(world).unwrap();
        assert_eq!(polygon.0, mesh);
        let photo = photo.0.clone();
        let plane = world.get::<Transform>(card).unwrap().translation.z;
        let paint_depth = world.get::<CardPaintDepth>(card).unwrap().0;
        assert_eq!(plane, super::super::PLAY_PLANE_Z);
        assert!(paint_depth > 0.0);
        assert_eq!(
            world
                .resource::<Assets<CardMaterial<PortalClipMaterial>>>()
                .get(&photo)
                .unwrap()
                .0.color_texture,
            Handle::default()
        );
        world
            .resource_mut::<Assets<Mesh>>()
            .insert(mesh.id(), Rectangle::new(10.0, 30.0).into())
            .unwrap();
        world
            .resource_mut::<Assets<ColorMaterial>>()
            .get_mut(&material)
            .unwrap()
            .texture = Some(image.clone());
        world.get_mut::<Transform>(source).unwrap().translation.y = 15.0;
        world.get_mut::<Transform>(source).unwrap().translation.z += 0.5;
        app.update();
        let world = app.world_mut();
        assert_eq!(world.get::<Mesh3d>(card).unwrap().0, mesh);
        assert_eq!(world.get::<MeshMaterial3d<CardMaterial<PortalClipMaterial>>>(card).unwrap().0, photo);
        assert_eq!(world.get::<Transform>(card).unwrap().translation.z, plane);
        assert!(world.get::<CardPaintDepth>(card).unwrap().0 > paint_depth);
        assert_eq!(
            world
                .resource::<Assets<CardMaterial<PortalClipMaterial>>>()
                .get(&photo)
                .unwrap()
                .0.color_texture,
            image
        );
        assert_eq!(
            app.world().get::<Transform>(card).unwrap().translation.y,
            15.0
        );
        app.world_mut().entity_mut(source).insert(Visibility::Hidden);
        app.update();
        assert_eq!(app.world().get::<Visibility>(card), Some(&Visibility::Hidden));
        app.world_mut().entity_mut(source).insert(Visibility::Inherited);
        app.update();
        assert_eq!(app.world().get::<Visibility>(card), Some(&Visibility::Inherited));
        app.world_mut().despawn(source);
        app.update();
        assert!(app.world().get::<CardWindow>(card).is_none());
    }
}
