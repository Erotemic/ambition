use ambition_platformer2d_shared_tangle::camera_layers::CARD_VIEW_LAYER;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use bevy::asset::{AssetId, RenderAssetUsages};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::RenderTarget;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::{
    Assets, Camera, ChildOf, Color, ColorToPacked, Commands, Component, DetectChanges, DetectChangesMut, Entity,
    GlobalTransform, Image, InheritedVisibility, Local, Mesh, Mesh3d, MeshMaterial3d, Name,
    Projection, Query, Ref, Res, ResMut, StandardMaterial, TextColor, Transform, UVec2, Vec2, Vec3,
    Visibility, With, Without,
};
use bevy::sprite::Anchor;
use bevy::text::{ComputedTextBlock, PositionedGlyph, TextBounds, TextLayoutInfo};

use super::paint_order::CardPaintDepth;
use super::{source_room, CardMeshes, CardRootDepths, CardScope, RetainedEntities, TextureQuad};
use crate::rendering::camera::coverage::ReplacedPlaneCamera;
use crate::rendering::primitives::WorldText;

#[derive(Component)]
pub(super) struct LabelLayoutCamera;

/// Supply Bevy's text layout with the plane view's visible sprites.
/// This camera has a projection and no render graph or window output.
pub(super) fn prepare_label_layout(
    mut commands: Commands,
    plane: Query<
        (Entity, &Camera, &Transform, &Projection, &RenderLayers),
        (With<ReplacedPlaneCamera>, Without<LabelLayoutCamera>),
    >,
    mut preparation: Query<
        (
            &mut Camera,
            &mut Transform,
            &mut Projection,
            &mut RenderTarget,
            &mut RenderLayers,
        ),
        (With<LabelLayoutCamera>, Without<ReplacedPlaneCamera>),
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct LabelRun {
    page: AssetId<Image>,
    section: usize,
}

impl LabelRun {
    fn of(glyph: &PositionedGlyph) -> Self {
        Self { page: glyph.atlas_info.texture, section: glyph.section_index }
    }
}

type LabelKey = (Entity, LabelRun);

#[cfg(feature = "portal_render")]
type LabelSource = bevy::prelude::Or<(
    With<WorldText>,
    With<ambition_portal2d_presentation::PortalVisual>,
    With<ambition_portal2d_presentation::PortalDisorientIndicator>,
)>;
#[cfg(not(feature = "portal_render"))]
type LabelSource = With<WorldText>;

/// Draw one text mesh per section and atlas page of each visible label.
pub(super) fn realize_card_labels(
    mut commands: Commands,
    labels: Query<
        (
            Entity,
            Ref<TextLayoutInfo>,
            Ref<TextBounds>,
            Ref<Anchor>,
            &GlobalTransform,
            &ComputedTextBlock,
            &Visibility,
            &InheritedVisibility,
        ),
        (LabelSource, Without<CardLabel>),
    >,
    colors: Query<&TextColor>,
    mut cards: Query<
        (
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Transform,
            &mut GlobalTransform,
            &mut CardPaintDepth,
        ),
        With<CardLabel>,
    >,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<CardMeshes>,
    depths: Res<CardRootDepths>,
    rooms: LiveRooms,
    parents: Query<&ChildOf>,
    mut retained: Local<RetainedEntities<LabelKey>>,
    mut runs: Local<Vec<LabelRun>>,
) {
    let shown = |visibility: &Visibility, inherited: &InheritedVisibility| match visibility {
        Visibility::Hidden => false,
        Visibility::Visible => true,
        Visibility::Inherited => inherited.get(),
    };
    retained.reconcile(&mut commands, |(source, run)| {
        labels
            .get(*source)
            .is_ok_and(|(_, layout, .., visibility, inherited)| {
                shown(visibility, inherited)
                    && layout
                        .glyphs
                        .iter()
                        .any(|glyph| LabelRun::of(glyph) == *run)
            })
    });
    for (source, layout, bounds, anchor, place, block, visibility, inherited) in &labels {
        if !shown(visibility, inherited) {
            continue;
        }
        runs.clear();
        for glyph in &layout.glyphs {
            let run = LabelRun::of(glyph);
            if !runs.contains(&run) {
                runs.push(run);
            }
        }
        let depth = depths.scenery(place.translation().z, source_room(source, &rooms, &parents));
        let place = depth.place(&place.compute_transform());
        for &run in &*runs {
            let Some(page_size) = images.get(run.page).map(|image| image.size().as_vec2()) else {
                continue;
            };
            let Some(texture) = images.get_strong_handle(run.page) else {
                continue;
            };
            let color = block.entities().get(run.section)
                .and_then(|section| colors.get(section.entity).ok())
                .map_or(Color::WHITE, |color| color.0);
            let material = cache.textured_material(&texture, color.to_srgba().to_u8_array(), &mut materials);
            let key = (source, run);
            if let Some(&entity) = retained.get(&key) {
                let Ok((mut mesh, mut tint, mut transform, mut global, mut paint)) = cards.get_mut(entity) else {
                    continue;
                };
                if layout.is_changed() || bounds.is_changed() || anchor.is_changed() {
                    mesh.0 = meshes.add(label_mesh(&layout, *bounds, *anchor, run, page_size));
                    commands.entity(entity).remove::<Aabb>();
                }
                if tint.0 != material {
                    tint.0 = material;
                }
                transform.set_if_neq(place);
                global.set_if_neq(GlobalTransform::from(place));
                paint.set_if_neq(depth.paint);
            } else {
                let card = commands
                    .spawn((
                        CardLabel,
                        CardScope::Source(source),
                        Mesh3d(meshes.add(label_mesh(&layout, *bounds, *anchor, run, page_size))),
                        MeshMaterial3d(material),
                        place,
                        GlobalTransform::from(place),
                        InheritedVisibility::VISIBLE,
                        depth.paint,
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
    run: LabelRun,
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
        .filter(|glyph| LabelRun::of(glyph) == run)
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

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use ambition_asset_manager::content_library::{ContentLibraries, ContentLibrary};
    use bevy::asset::AssetPlugin;
    use bevy::camera::{visibility::VisibleEntities, RenderTargetInfo};
    use bevy::prelude::{
        App, Assets, Camera, Color, ColorToPacked, Entity, Font, FontSize, Image, IntoScheduleConfigs, Mesh,
        MeshMaterial3d, MinimalPlugins, PostUpdate, Sprite, StandardMaterial, Text2d, TextColor,
        TextFont, TextSpan, UVec2, Visibility, World,
    };
    use bevy::text::{detect_text_needs_rerender, TextLayoutInfo, TextPlugin};

    use super::{realize_card_labels, CardLabel, CardMeshes, CardRootDepths, WorldText};

    #[test]
    fn span_color_changes_without_replacing_its_card() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TextPlugin))
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CardMeshes>()
            .init_resource::<CardRootDepths>()
            .add_systems(
                PostUpdate,
                (bevy::sprite::update_text2d_layout, realize_card_labels)
                    .chain()
                    .after(detect_text_needs_rerender),
            );
        let mut libraries = ContentLibraries::default();
        libraries.insert(ContentLibrary::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../game/ambition_content_library/library.json"
        )).unwrap()).unwrap();
        let font = libraries.files().unwrap()["content://ambition_shared/fonts/bundled/InterDisplay-Regular.otf"]
            .read().unwrap();
        let world = app.world_mut();
        let font = world.resource_mut::<Assets<Font>>().add(Font::from_bytes(font));
        let font = TextFont { font: font.into(), font_size: FontSize::Px(36.0), ..Default::default() };
        let mut camera = Camera::default();
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::splat(600),
            ..Default::default()
        });
        let mut visible = VisibleEntities::default();
        visible.push(Entity::PLACEHOLDER, TypeId::of::<Sprite>());
        world.spawn((camera, visible));
        let red = Color::srgb_u8(210, 70, 70);
        let root = world.spawn((Text2d::new("Press "), font.clone(), WorldText, Visibility::Visible)).id();
        let span = world.spawn((TextSpan::new("B"), font, TextColor(red), bevy::prelude::ChildOf(root))).id();
        for _ in 0..3 {
            app.update();
        }
        assert!(!app.world().get::<TextLayoutInfo>(root).unwrap().glyphs.is_empty());

        let card_with = |world: &mut World, color| {
            let mut cards = world.query_filtered::<(Entity, &MeshMaterial3d<StandardMaterial>), bevy::prelude::With<CardLabel>>();
            let materials = world.resource::<Assets<StandardMaterial>>();
            cards.iter(world)
                .find(|(_, material)| materials.get(&material.0).unwrap().base_color.to_srgba().to_u8_array() == color)
                .map(|(card, _)| card)
                .unwrap_or_else(|| panic!("no label card uses {color:?}"))
        };
        let white = [255; 4];
        let lettering = card_with(app.world_mut(), white);
        let button = card_with(app.world_mut(), red.to_srgba().to_u8_array());
        assert_ne!(lettering, button);

        app.world_mut().entity_mut(span).insert(TextColor(Color::NONE));
        app.update();
        assert_eq!(card_with(app.world_mut(), Color::NONE.to_srgba().to_u8_array()), button);
        assert_eq!(card_with(app.world_mut(), white), lettering);
    }
}
