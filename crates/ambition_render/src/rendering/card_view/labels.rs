use ambition_platformer2d_shared_tangle::camera_layers::CARD_VIEW_LAYER;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use bevy::asset::{AssetId, RenderAssetUsages};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::RenderTarget;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::{
    Assets, Camera, ColorToPacked, Commands, Component, DetectChanges, DetectChangesMut, Entity,
    GlobalTransform, Image, InheritedVisibility, Local, Mesh, Mesh3d, MeshMaterial3d, Name,
    Projection, Query, Ref, Res, ResMut, StandardMaterial, TextColor, Transform, UVec2, Vec2, Vec3,
    Visibility, With, Without,
};
use bevy::sprite::Anchor;
use bevy::text::{TextBounds, TextLayoutInfo};

use super::{CardMeshes, CardRootDepths, CardScope, RetainedEntities, TextureQuad, CARD_BODY_STEP};
use crate::rendering::actors::rigged::AppearanceSampleCamera;
use crate::rendering::label_layout::WorldLabel;

#[derive(Component)]
pub(super) struct LabelLayoutCamera;

/// Supply Bevy's text layout with the plane view's visible sprites.
/// This camera has a projection and no render graph or window output.
pub(super) fn prepare_label_layout(
    mut commands: Commands,
    plane: Query<
        (Entity, &Camera, &Transform, &Projection, &RenderLayers),
        (With<AppearanceSampleCamera>, Without<LabelLayoutCamera>),
    >,
    mut preparation: Query<
        (
            &mut Camera,
            &mut Transform,
            &mut Projection,
            &mut RenderTarget,
            &mut RenderLayers,
        ),
        (With<LabelLayoutCamera>, Without<AppearanceSampleCamera>),
    >,
    mut retained: Local<RetainedEntities<Entity>>,
) {
    retained.reconcile(&mut commands, |source| plane.contains(*source));
    for (source, plane, place, projection, layers) in &plane {
        let size = plane.physical_target_size().unwrap_or(UVec2::ONE);
        if let Some((mut camera, mut transform, mut lens, mut target, mut mask)) = retained
            .get(&source)
            .and_then(|camera| preparation.get_mut(*camera).ok())
        {
            let rect = |camera: &Camera| {
                camera
                    .viewport
                    .as_ref()
                    .map(|viewport| (viewport.physical_position, viewport.physical_size))
            };
            if rect(&camera) != rect(plane) {
                camera.viewport.clone_from(&plane.viewport);
            }
            if !matches!(&*target, RenderTarget::None { size: current } if *current == size) {
                *target = RenderTarget::None { size };
            }
            transform.set_if_neq(*place);
            *lens = projection.clone();
            mask.set_if_neq(layers.clone());
        } else {
            let camera = commands
                .spawn((
                    LabelLayoutCamera,
                    Camera {
                        viewport: plane.viewport.clone(),
                        ..Default::default()
                    },
                    *place,
                    projection.clone(),
                    RenderTarget::None { size },
                    layers.clone(),
                    Name::new("World label layout view"),
                ))
                .id();
            retained.insert(source, camera);
        }
    }
}

#[derive(Component)]
pub(super) struct CardLabel;

type LabelKey = (Entity, AssetId<Image>);

/// Draw one text mesh per atlas page of each visible world label.
pub(super) fn realize_card_labels(
    mut commands: Commands,
    labels: Query<
        (
            Entity,
            Ref<TextLayoutInfo>,
            Ref<TextBounds>,
            Ref<Anchor>,
            &GlobalTransform,
            &TextColor,
            &Visibility,
            &InheritedVisibility,
        ),
        With<WorldLabel>,
    >,
    mut cards: Query<
        (
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        With<CardLabel>,
    >,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<CardMeshes>,
    depths: Res<CardRootDepths>,
    rooms: LiveRooms,
    mut retained: Local<RetainedEntities<LabelKey>>,
    mut pages: Local<Vec<AssetId<Image>>>,
) {
    let shown = |visibility: &Visibility, inherited: &InheritedVisibility| match visibility {
        Visibility::Hidden => false,
        Visibility::Visible => true,
        Visibility::Inherited => inherited.get(),
    };
    retained.reconcile(&mut commands, |(source, atlas)| {
        labels
            .get(*source)
            .is_ok_and(|(_, layout, .., visibility, inherited)| {
                shown(visibility, inherited)
                    && layout
                        .glyphs
                        .iter()
                        .any(|glyph| glyph.atlas_info.texture == *atlas)
            })
    });
    for (source, layout, bounds, anchor, place, color, visibility, inherited) in &labels {
        if !shown(visibility, inherited) {
            continue;
        }
        pages.clear();
        for glyph in &layout.glyphs {
            let page = glyph.atlas_info.texture;
            if !pages.contains(&page) {
                pages.push(page);
            }
        }
        let z = depths
            .fronts
            .get(&rooms.of(source))
            .copied()
            .unwrap_or(CARD_BODY_STEP)
            + 0.5;
        let mut place = place.compute_transform();
        place.translation.z = z;
        let color = color.0.to_srgba().to_u8_array();
        for &page in &*pages {
            let Some(page_size) = images.get(page).map(|image| image.size().as_vec2()) else {
                continue;
            };
            let Some(texture) = images.get_strong_handle(page) else {
                continue;
            };
            let material = cache.textured_material(&texture, color, &mut materials);
            let key = (source, page);
            if let Some(&entity) = retained.get(&key) {
                let Ok((mut mesh, mut tint, mut transform)) = cards.get_mut(entity) else {
                    continue;
                };
                if layout.is_changed() || bounds.is_changed() || anchor.is_changed() {
                    mesh.0 = meshes.add(label_mesh(&layout, *bounds, *anchor, page, page_size));
                    commands.entity(entity).remove::<Aabb>();
                }
                if tint.0 != material {
                    tint.0 = material;
                }
                transform.set_if_neq(place);
            } else {
                let card = commands
                    .spawn((
                        CardLabel,
                        CardScope::Source(source),
                        Mesh3d(meshes.add(label_mesh(&layout, *bounds, *anchor, page, page_size))),
                        MeshMaterial3d(material),
                        place,
                        RenderLayers::layer(CARD_VIEW_LAYER),
                    ))
                    .id();
                retained.insert(key, card);
            }
        }
    }
}

fn label_mesh(
    layout: &TextLayoutInfo,
    bounds: TextBounds,
    anchor: Anchor,
    page: AssetId<Image>,
    page_size: Vec2,
) -> Mesh {
    let inv = layout.scale_factor.recip();
    let box_size = Vec2::new(
        bounds.width.unwrap_or(layout.size.x),
        bounds.height.unwrap_or(layout.size.y),
    );
    let top_left = (Anchor::TOP_LEFT.0 - anchor.0) * box_size;
    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for glyph in layout
        .glyphs
        .iter()
        .filter(|glyph| glyph.atlas_info.texture == page)
    {
        let rect = glyph.atlas_info.rect;
        let (positions, texcoords) = TextureQuad {
            rect,
            page: page_size,
            size: rect.size() * inv,
            anchor: Vec2::ZERO,
            flip: [false, false],
        }
        .vertices();
        let offset = top_left + glyph.position * Vec2::new(inv, -inv);
        let start = vertices.len() as u32;
        vertices.extend(positions.map(|[x, y, z]| [x + offset.x, y + offset.y, z]));
        uvs.extend(texcoords);
        indices.extend([0, 1, 2, 0, 2, 3].map(|index| start + index));
    }
    let normals = vec![Vec3::Z.to_array(); vertices.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}
