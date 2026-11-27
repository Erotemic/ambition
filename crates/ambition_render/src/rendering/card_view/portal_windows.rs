use ambition_portal2d_presentation::{PortalConeMesh, PORTAL_WINDOW_Z};
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::prelude::{
    AlphaMode, Assets, ColorMaterial, Commands, Component, DetectChangesMut, Entity, Local, Mesh2d,
    Mesh3d, MeshMaterial2d, MeshMaterial3d, Query, Res, ResMut, StandardMaterial, Transform,
    Visibility, With, Without,
};

use super::{CardScope, RetainedEntities, PORTAL_DEPTH};
use ambition_platformer2d_shared_tangle::camera_layers::CARD_VIEW_LAYER;

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
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
            &mut Visibility,
        ),
        (With<CardWindow>, Without<PortalConeMesh>),
    >,
    plane_materials: Res<Assets<ColorMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut products: Local<RetainedEntities<Entity>>,
) {
    products.reconcile(&mut commands, |source| windows.contains(*source));
    for (source, mesh, material, transform, visible) in &windows {
        let product = products
            .get(&source)
            .and_then(|product| cards.get_mut(*product).ok());
        let Some(material) = plane_materials.get(&material.0) else {
            if let Some((.., mut visibility)) = product {
                visibility.set_if_neq(Visibility::Hidden);
            }
            continue;
        };
        // A closed cone keeps an empty mesh. Keep its pipeline in use so the
        // first open frame can use it.
        let color = material.color;
        let texture = &material.texture;
        let mut place = *transform;
        place.translation.z =
            PORTAL_DEPTH - 0.01 + (transform.translation.z - PORTAL_WINDOW_Z) * 0.001;
        if let Some((mut polygon, image, mut position, mut visibility)) = product {
            if polygon.0 != mesh.0 {
                polygon.0 = mesh.0.clone();
            }
            position.set_if_neq(place);
            visibility.set_if_neq(*visible);
            if materials.get(&image.0).is_some_and(|material| {
                material.base_color != color
                    || material.base_color_texture != *texture
            }) {
                if let Some(mut material) = materials.get_mut(&image.0) {
                    material.base_color = color;
                    material.base_color_texture = texture.clone();
                }
            }
        } else {
            let product = commands
                .spawn((
                    CardWindow,
                    CardScope::Source(source),
                    Mesh3d(mesh.0.clone()),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: color,
                        base_color_texture: texture.clone(),
                        unlit: true,
                        cull_mode: None,
                        alpha_mode: AlphaMode::Opaque,
                        ..Default::default()
                    })),
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
    use bevy::prelude::{App, Image, Mesh, Rectangle, Update};

    #[test]
    fn a_portal_card_retains_its_mesh_and_material_when_the_window_opens() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<Assets<StandardMaterial>>()
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
        let mut cards = world.query_filtered::<(Entity, &Mesh3d, &MeshMaterial3d<StandardMaterial>), With<CardWindow>>();
        let (card, polygon, photo) = cards.single(world).unwrap();
        assert_eq!(polygon.0, mesh);
        let photo = photo.0.clone();
        assert_eq!(
            world
                .resource::<Assets<StandardMaterial>>()
                .get(&photo)
                .unwrap()
                .base_color_texture,
            None
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
        app.update();
        let world = app.world_mut();
        assert_eq!(world.get::<Mesh3d>(card).unwrap().0, mesh);
        assert_eq!(world.get::<MeshMaterial3d<StandardMaterial>>(card).unwrap().0, photo);
        assert_eq!(
            world
                .resource::<Assets<StandardMaterial>>()
                .get(&photo)
                .unwrap()
                .base_color_texture
                .as_ref(),
            Some(&image)
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
