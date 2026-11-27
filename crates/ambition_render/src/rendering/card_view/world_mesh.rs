use std::collections::{HashMap, HashSet};

use ambition_platformer2d_core::config::WORLD_Z_BLOCK;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRooms, PresentationOf};
#[cfg(feature = "portal_render")]
use ambition_portal2d_presentation::PortalCarryOrder;
use bevy::asset::AssetEventSystems;
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers, VisibilitySystems};
use bevy::ecs::system::{SystemParam, SystemParamItem};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::erased_render_asset::prepare_erased_assets;
use bevy::render::render_asset::prepare_assets;
use bevy::render::render_resource::{
    AsBindGroup, AsBindGroupError, BindGroupLayout, BindGroupLayoutEntry, BlendState, RenderPipelineDescriptor,
    SpecializedMeshPipelineError, UnpreparedBindGroup,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::GpuImage;
use bevy::render::{Render, RenderApp};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, ColorMaterial, Material2d, MeshMaterial2d};
use bevy::transform::TransformSystems;

use super::{CARD_BODY_STEP, EARTH_BEHIND_SKIN, TERRAIN_DEPTH, scenery_depth, sprite_material};
use super::{CardBodyRepresentation, CardRootDepths, CardScope, CardViewMode, CardViewSettings, RetainedEntities};
use crate::folded_mesh::{AddedLightMaterial, FoldedMesh};
use crate::rendering::chain_skin::ChainEarth;
use crate::rendering::terrain_skin::TerrainSkinMesh;
use crate::rendering::unauthored_volumes::UnauthoredVolumeVisual;

#[derive(Component)]
pub(super) struct CardWorldMesh;

#[derive(SystemSet, Debug, Clone, Eq, PartialEq, Hash)]
pub(super) struct CardWorldMeshSet;

#[derive(SystemSet, Debug, Clone, Eq, PartialEq, Hash)]
struct CardTextureBindings;

pub(super) fn install(app: &mut App) {
    app.configure_sets(
        PostUpdate,
        CardWorldMeshSet
            .after(TransformSystems::Propagate)
            .after(VisibilitySystems::VisibilityPropagate)
            .before(AssetEventSystems)
            .before(VisibilitySystems::CheckVisibility),
    );
    app.add_systems(
        PostUpdate,
        (
            realize_color_meshes.run_if(cards_are_visible_or_were),
            realize_folded_meshes.run_if(cards_are_visible),
        )
            .in_set(CardWorldMeshSet),
    );
    if app.get_sub_app(RenderApp).is_some() {
        prepare_card_material_after_image::<StandardMaterial>(app);
        install_card_mesh_material::<ambition_sprite_fx::SpriteFxMaterial>(app);
        install_card_mesh_material::<crate::rendering::darkness::DarknessMaterial>(app);
        install_card_mesh_material::<crate::bubble_shield_fx::BubbleShieldFxMaterial>(app);
        install_card_mesh_material::<crate::gate_portal_fx::GatePortalFxMaterial>(app);
        install_card_mesh_material::<crate::rendering::daylight::DaylightMaterial>(app);
        install_card_mesh_material::<crate::rendering::daylight::LampGlowMaterial>(app);
        #[cfg(not(target_os = "android"))]
        {
            install_card_mesh_material::<crate::rendering::actor_glow::ActorGlowMaterial>(app);
            install_card_mesh_material::<crate::rendering::hit_flash::HitFlashMaterial>(app);
        }
        #[cfg(feature = "portal_render")]
        {
            install_card_mesh_material::<ambition_portal2d_presentation::PortalGlowMaterial>(app);
            install_card_mesh_material::<ambition_portal2d_presentation::PortalClipMaterial>(app);
        }
    }
}

/// A planar material with a fragment shader for the card output.
pub trait CardMaterialSource: Material2d + PartialEq {
    /// A blend state other than the material's alpha mode.
    fn card_blend() -> Option<BlendState> {
        None
    }

    /// Check whether this material needs a card for the current subject.
    /// A material such as a terrain skin can have no body as its subject.
    fn draws_on_card(&self, _body: Option<CardBodyRepresentation>) -> bool {
        true
    }
}

impl CardMaterialSource for ambition_sprite_fx::SpriteFxMaterial {}
impl CardMaterialSource for crate::rendering::darkness::DarknessMaterial {}
impl CardMaterialSource for crate::bubble_shield_fx::BubbleShieldFxMaterial {}
impl CardMaterialSource for crate::gate_portal_fx::GatePortalFxMaterial {}
#[cfg(feature = "portal_render")]
impl CardMaterialSource for ambition_portal2d_presentation::PortalGlowMaterial {}
#[cfg(feature = "portal_render")]
impl CardMaterialSource for ambition_portal2d_presentation::PortalClipMaterial {}

/// Draw a planar material with the same bindings in the card view. Its fragment
/// shader must accept the 3D vertex output when `AMBITION_CARD_MESH` is set.
pub fn install_card_mesh_material<M: CardMaterialSource>(app: &mut App)
where
    M::Data: PartialEq + Eq + std::hash::Hash + Clone,
{
    app.add_plugins(MaterialPlugin::<CardMaterial<M>>::default())
        .add_systems(
            PostUpdate,
            realize_shaded_meshes::<M>
                .in_set(CardWorldMeshSet)
                .run_if(cards_are_visible_or_were),
        );
    prepare_card_material_after_image::<CardMaterial<M>>(app);
}

fn prepare_card_material_after_image<M: Material>(app: &mut App)
where
    M::Data: PartialEq + Eq + std::hash::Hash + Clone,
{
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        // Order the two Bevy systems through an empty set. A capture image and
        // its material can arrive in the same frame.
        render.configure_sets(
            Render,
            CardTextureBindings
                .after(prepare_assets::<GpuImage>)
                .before(prepare_erased_assets::<MeshMaterial3d<M>>),
        );
    }
}

fn cards_are_visible(settings: Option<Res<CardViewSettings>>) -> bool {
    settings.is_some_and(|settings| settings.mode != CardViewMode::Plane)
}

fn cards_are_visible_or_were(
    settings: Option<Res<CardViewSettings>>,
    mut previous: Local<bool>,
) -> bool {
    let active = cards_are_visible(settings);
    active | std::mem::replace(&mut *previous, active)
}

// The 2D and 3D material plugins each initialize their own asset storage.
#[derive(Asset, TypePath, Clone)]
pub(super) struct CardMaterial<M: Material2d>(#[dependency] pub(super) M);

impl<M: Material2d> AsBindGroup for CardMaterial<M> {
    type Data = M::Data;
    type Param = M::Param;

    fn label() -> &'static str {
        M::label()
    }

    fn bind_group_data(&self) -> Self::Data {
        self.0.bind_group_data()
    }

    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        device: &RenderDevice,
        param: &mut SystemParamItem<'_, '_, Self::Param>,
        _force_no_bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        self.0.unprepared_bind_group(layout, device, param, true)
    }

    fn bind_group_layout_entries(
        device: &RenderDevice,
        _force_no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry> {
        M::bind_group_layout_entries(device, true)
    }
}

impl<M: CardMaterialSource> Material for CardMaterial<M> {
    fn fragment_shader() -> ShaderRef {
        M::fragment_shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        match self.0.alpha_mode() {
            AlphaMode2d::Opaque => AlphaMode::Opaque,
            AlphaMode2d::Mask(cutoff) => AlphaMode::Mask(cutoff),
            AlphaMode2d::Blend => AlphaMode::Blend,
        }
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = &mut descriptor.fragment {
            fragment.shader_defs.push("AMBITION_CARD_MESH".into());
            if let Some(blend) = M::card_blend() {
                for target in fragment.targets.iter_mut().flatten() {
                    target.blend = Some(blend);
                }
            }
        }
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[derive(SystemParam)]
struct MeshProducts<'w, 's, M: Material> {
    commands: Commands<'w, 's>,
    retained: Local<'s, RetainedEntities<Entity>>,
    meshes: Query<
        'w,
        's,
        (
            &'static mut Mesh3d,
            &'static mut MeshMaterial3d<M>,
            &'static mut Transform,
            &'static mut GlobalTransform,
        ),
        With<CardWorldMesh>,
    >,
}

impl<M: Material> MeshProducts<'_, '_, M> {
    fn show(
        &mut self,
        source: Entity,
        mesh: &Handle<Mesh>,
        material: &Handle<M>,
        place: Transform,
        skip_culling: bool,
    ) {
        let global = GlobalTransform::from(place);
        if let Some((mut drawn_mesh, mut drawn_material, mut transform, mut drawn_global)) = self
            .retained
            .get(&source)
            .and_then(|entity| self.meshes.get_mut(*entity).ok())
        {
            if drawn_mesh.0 != *mesh {
                drawn_mesh.0 = mesh.clone();
            }
            if drawn_material.0 != *material {
                drawn_material.0 = material.clone();
            }
            transform.set_if_neq(place);
            drawn_global.set_if_neq(global);
            return;
        }
        let mut entity = self
            .commands
            .spawn((
                CardWorldMesh,
                CardScope::Source(source),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                place,
                global,
                InheritedVisibility::VISIBLE,
                RenderLayers::none(),
                Name::new("Card world mesh"),
            ));
        if skip_culling {
            entity.insert(NoFrustumCulling);
        }
        self.retained.insert(source, entity.id());
    }
}

fn effect_place(
    source: Entity,
    global: &GlobalTransform,
    rooms: &LiveRooms,
    depths: &CardRootDepths,
) -> Transform {
    let place = global.compute_transform();
    let front = depths
        .fronts
        .get(&rooms.of(source))
        .copied()
        .unwrap_or(CARD_BODY_STEP);
    super::sprite_card_place(&place, scenery_depth(place.translation.z, front))
}

fn realize_color_meshes(
    sources: Query<
        (
            Entity,
            &Mesh2d,
            &MeshMaterial2d<ColorMaterial>,
            &Transform,
            &GlobalTransform,
            &InheritedVisibility,
            Has<ChainEarth>,
            Has<TerrainSkinMesh>,
            Has<NoFrustumCulling>,
        ),
        (
            Or<(With<ChainEarth>, With<TerrainSkinMesh>, With<UnauthoredVolumeVisual>)>,
            Without<CardWorldMesh>,
        ),
    >,
    colors: Option<Res<Assets<ColorMaterial>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut by_material: Local<HashMap<AssetId<ColorMaterial>, Handle<StandardMaterial>>>,
    mut used: Local<HashSet<AssetId<ColorMaterial>>>,
    mut products: MeshProducts<StandardMaterial>,
    settings: Option<Res<CardViewSettings>>,
    rooms: LiveRooms,
    depths: Res<CardRootDepths>,
) {
    if !cards_are_visible(settings) {
        by_material.clear();
        return;
    }
    let (Some(colors), Some(mut materials)) = (colors, materials) else {
        return;
    };
    products.retained.reconcile(&mut products.commands, |entity| {
        sources.get(*entity).is_ok_and(|(_, _, material, _, _, visible, ..)| {
            visible.get() && colors.contains(material.0.id())
        })
    });
    used.clear();
    for (source, mesh, material, local, global, visible, earth, terrain, skip_culling) in &sources {
        let Some(color) = colors.get(&material.0).filter(|_| visible.get()) else {
            continue;
        };
        let id = material.0.id();
        let alpha_mode = match color.alpha_mode {
            AlphaMode2d::Opaque => AlphaMode::Opaque,
            AlphaMode2d::Mask(cutoff) => AlphaMode::Mask(cutoff),
            AlphaMode2d::Blend => AlphaMode::Blend,
        };
        let make = || StandardMaterial {
            uv_transform: color.uv_transform,
            alpha_mode,
            ..sprite_material(color.texture.clone(), color.color)
        };
        let card = by_material.entry(id).or_insert_with(|| materials.add(make()));
        if used.insert(id)
            && materials.get(&*card).is_some_and(|material| {
                material.base_color != color.color
                    || material.base_color_texture != color.texture
                    || material.uv_transform != color.uv_transform
                    || material.alpha_mode != alpha_mode
            })
        {
            if let Some(mut material) = materials.get_mut(&*card) {
                *material = make();
            }
        }
        let place = if terrain {
            super::sprite_card_place(&global.compute_transform(), TERRAIN_DEPTH + local.translation.z)
        } else if earth && local.translation.z < WORLD_Z_BLOCK {
            super::sprite_card_place(&global.compute_transform(), TERRAIN_DEPTH - EARTH_BEHIND_SKIN)
        } else {
            effect_place(source, global, &rooms, &depths)
        };
        products.show(source, &mesh.0, card, place, skip_culling);
    }
    by_material.retain(|id, _| used.contains(id));
}

fn realize_shaded_meshes<M: CardMaterialSource>(
    sources: Query<
        (
            Entity,
            &Mesh2d,
            &MeshMaterial2d<M>,
            &Transform,
            &GlobalTransform,
            &InheritedVisibility,
            Has<TerrainSkinMesh>,
            Has<NoFrustumCulling>,
            Option<&PresentationOf>,
        ),
        Without<CardWorldMesh>,
    >,
    source_materials: Option<Res<Assets<M>>>,
    mut card_materials: ResMut<Assets<CardMaterial<M>>>,
    mut by_material: Local<HashMap<AssetId<M>, Handle<CardMaterial<M>>>>,
    mut used: Local<HashSet<AssetId<M>>>,
    mut products: MeshProducts<CardMaterial<M>>,
    settings: Option<Res<CardViewSettings>>,
    rooms: LiveRooms,
    depths: Res<CardRootDepths>,
    #[cfg(feature = "portal_render")] carry_orders: Query<&PortalCarryOrder>,
) {
    if !cards_are_visible(settings) {
        by_material.clear();
        return;
    }
    let Some(source_materials) = source_materials else {
        return;
    };
    let in_view = |material: &MeshMaterial2d<M>, subject: Option<&PresentationOf>| {
        source_materials.get(&material.0)
            .filter(|draw| draw.draws_on_card(depths.representation_of(subject)))
    };
    products
        .retained
        .reconcile(&mut products.commands, |entity| {
            sources
                .get(*entity)
                .is_ok_and(|(_, _, material, _, _, visible, _, _, subject)| {
                    visible.get() && in_view(material, subject).is_some()
                })
        });
    used.clear();
    for (source, mesh, material, local, global, visible, terrain, skip_culling, subject) in &sources {
        if !visible.get() {
            continue;
        }
        let Some(draw) = in_view(material, subject) else {
            continue;
        };
        let id = material.0.id();
        let card = by_material
            .entry(id)
            .or_insert_with(|| card_materials.add(CardMaterial(draw.clone())));
        if used.insert(id)
            && card_materials
                .get(&*card)
                .is_some_and(|current| current.0 != *draw)
        {
            if let Some(mut current) = card_materials.get_mut(&*card) {
                current.0 = draw.clone();
            }
        }
        let body_depth = depths.effect_depth(subject, global.translation().z);
        #[cfg(feature = "portal_render")]
        let body_depth = match carry_orders.get(source) {
            Ok(PortalCarryOrder::Entry(root)) => depths.roots.get(root).map(|depth| depth.plane_z),
            Ok(PortalCarryOrder::Exit) => None,
            Err(_) => body_depth,
        };
        let place = if terrain {
            super::sprite_card_place(
                &global.compute_transform(),
                TERRAIN_DEPTH + local.translation.z,
            )
        } else if let Some(depth) = body_depth {
            super::sprite_card_place(&global.compute_transform(), depth)
        } else {
            effect_place(source, global, &rooms, &depths)
        };
        products.show(source, &mesh.0, card, place, skip_culling);
    }
    by_material.retain(|id, _| used.contains(id));
}

fn realize_folded_meshes(
    sources: Query<
        (
            Entity,
            &Mesh2d,
            &GlobalTransform,
            &InheritedVisibility,
            Has<MeshMaterial2d<AddedLightMaterial>>,
        ),
        (With<FoldedMesh>, Without<CardWorldMesh>),
    >,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut prepared: Local<[Option<Handle<StandardMaterial>>; 2]>,
    mut products: MeshProducts<StandardMaterial>,
    rooms: LiveRooms,
    depths: Res<CardRootDepths>,
) {
    let Some(materials) = materials.as_deref_mut() else {
        return;
    };
    products
        .retained
        .reconcile(&mut products.commands, |entity| {
            sources
                .get(*entity)
                .is_ok_and(|(_, _, _, visible, _)| visible.get())
        });
    for (source, mesh, global, visible, additive) in &sources {
        if !visible.get() {
            continue;
        }
        let material = prepared[usize::from(additive)].get_or_insert_with(|| {
            materials.add(StandardMaterial {
                unlit: true,
                cull_mode: None,
                alpha_mode: if additive {
                    AlphaMode::Add
                } else {
                    AlphaMode::Blend
                },
                ..default()
            })
        });
        products.show(
            source,
            &mesh.0,
            material,
            effect_place(source, global, &rooms, &depths),
            true,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::hit_flash::{HitFlashMaterial, FLASH_OVERLAY_Z_BIAS};
    use super::super::{card_body_depth, CardBodyDepth};

    #[test]
    fn terrain_cards_draw_untextured_earth_and_keep_skin_layer_order() {
        use ambition_platformer2d_core as ae;
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CardRootDepths>()
            .insert_resource(CardViewSettings::for_game(CardViewMode::SideOn))
            .add_systems(Update, (
                crate::rendering::chain_skin::build_chain_earth_meshes, realize_color_meshes,
            ).chain());
        let world = app.world_mut();
        let ground = ae::World::new("hill", ae::Vec2::new(80.0, 48.0), ae::Vec2::ZERO, Vec::new());
        let hill = ChainEarth::under(&ground, &[ae::Vec2::new(0.0, 20.0), ae::Vec2::new(80.0, 28.0)], None);
        let earth_z = WORLD_Z_BLOCK - 0.5;
        let earth = world.spawn((
            hill.clone(), Transform::from_xyz(0.0, 0.0, earth_z),
            GlobalTransform::from_translation(Vec3::Z * earth_z), InheritedVisibility::VISIBLE,
        )).id();
        let near_z = ae::config::WORLD_Z_PLAYER + 0.3;
        let near_track = world.spawn((
            hill, Transform::from_xyz(0.0, 0.0, near_z),
            GlobalTransform::from_translation(Vec3::Z * near_z), InheritedVisibility::VISIBLE,
        )).id();
        let mesh = world.resource_mut::<Assets<Mesh>>().add(Rectangle::new(32.0, 16.0));
        let skin = world.resource_mut::<Assets<ColorMaterial>>().add(ColorMaterial {
            texture: Some(Handle::default()),
            ..default()
        });
        let terrain = |z: f32| (
            Mesh2d(mesh.clone()),
            MeshMaterial2d(skin.clone()),
            Transform::from_xyz(0.0, 0.0, z),
            GlobalTransform::from_translation(Vec3::new(40.0, -12.0, 7.0 + z)),
            TerrainSkinMesh {
                theme: ambition_sprite_sheet::game_assets::ParallaxTheme::new("cave"),
                centre: Vec2::ZERO,
            },
        );
        let trim = world.spawn((terrain(0.03), InheritedVisibility::VISIBLE)).id();
        let fill = world.spawn((terrain(0.005), InheritedVisibility::VISIBLE)).id();
        world.spawn((terrain(0.01), InheritedVisibility::HIDDEN));

        let shown = |world: &mut World| {
            world.query_filtered::<(&CardScope, &Mesh3d, &Transform), With<CardWorldMesh>>()
                .iter(world)
                .filter_map(|(scope, mesh, place)| match scope {
                    CardScope::Source(source) => Some((*source, (mesh.0.clone(), place.translation))),
                    CardScope::Camera(_) => None,
                })
                .collect::<HashMap<_, _>>()
        };
        app.update();
        let drawn = shown(app.world_mut());
        assert_eq!(drawn.len(), 4);
        let earth_mesh = &app.world().get::<Mesh2d>(earth).unwrap().0;
        assert_eq!(&drawn[&earth].0, earth_mesh);
        assert_eq!(drawn[&earth].1, Vec3::Z * (TERRAIN_DEPTH - EARTH_BEHIND_SKIN));
        assert_eq!(drawn[&fill], (mesh.clone(), Vec3::new(40.0, -12.0, TERRAIN_DEPTH + 0.005)));
        assert_eq!(drawn[&trim], (mesh, Vec3::new(40.0, -12.0, TERRAIN_DEPTH + 0.03)));
        assert!(drawn[&near_track].1.z > card_body_depth(false, 0));

        app.world_mut().despawn(trim);
        app.update();
        assert!(!shown(app.world_mut()).contains_key(&trim));
    }

    #[cfg(feature = "portal_render")]
    #[test]
    fn a_portal_entry_piece_keeps_its_sources_draw_depth() {
        use ambition_portal2d_presentation::{PORTAL_EXIT_COPY_Z, PortalClipMaterial};

        let mut app = App::new();
        app.init_resource::<Assets<PortalClipMaterial>>()
            .init_resource::<Assets<CardMaterial<PortalClipMaterial>>>()
            .init_resource::<CardRootDepths>()
            .insert_resource(CardViewSettings::for_game(CardViewMode::SideOn))
            .add_systems(Update, realize_shaded_meshes::<PortalClipMaterial>);
        let world = app.world_mut();
        let root = world.spawn_empty().id();
        let depths = &mut *world.resource_mut::<CardRootDepths>();
        depths.fronts.insert(None, 7.5);
        depths.roots.insert(root, CardBodyDepth {
            plane_z: 2.5,
            source_z: 20.0,
            order_scale: 1.0,
            front_part: 0.0,
            representation: CardBodyRepresentation::Sprite,
        });
        let material = world.resource_mut::<Assets<PortalClipMaterial>>().add(
            PortalClipMaterial::new(Vec4::new(0.0, 0.0, 1.0, 1.0), Vec4::ZERO, Vec4::ONE, Handle::default())
        );
        let piece = world.spawn((
            PortalCarryOrder::Entry(root),
            Mesh2d::default(),
            MeshMaterial2d(material),
            GlobalTransform::from_translation(Vec3::new(42.0, 13.0, 20.0)),
            InheritedVisibility::VISIBLE,
        )).id();
        let card = |world: &mut World| {
            let (card, place) = world.query_filtered::<(Entity, &Transform), With<CardWorldMesh>>()
                .single(world)
                .expect("the piece has one card");
            (card, place.translation)
        };

        app.update();
        let (entry_card, entry_place) = card(app.world_mut());
        assert_eq!(entry_place, Vec3::new(42.0, 13.0, 2.5));

        app.world_mut().entity_mut(piece).insert((
            PortalCarryOrder::Exit,
            GlobalTransform::from_translation(Vec3::new(-36.0, 13.0, PORTAL_EXIT_COPY_Z)),
        ));
        app.update();
        let (exit_card, exit_place) = card(app.world_mut());
        assert_eq!(entry_card, exit_card);
        assert_eq!(exit_place, Vec3::new(-36.0, 13.0, scenery_depth(PORTAL_EXIT_COPY_Z, 7.5)));
    }

    #[test]
    fn a_flash_draws_on_a_whole_body_until_the_cue_ends() {
        let mut app = App::new();
        app.init_resource::<Assets<HitFlashMaterial>>()
            .init_resource::<Assets<CardMaterial<HitFlashMaterial>>>()
            .init_resource::<CardRootDepths>()
            .insert_resource(CardViewSettings::for_game(CardViewMode::SideOn))
            .add_systems(Update, realize_shaded_meshes::<HitFlashMaterial>);
        let world = app.world_mut();
        let material = world.resource_mut::<Assets<HitFlashMaterial>>().add(HitFlashMaterial {
            uv_rect: Vec4::new(0.0, 0.0, 1.0, 1.0),
            control: Vec4::new(0.8, 0.0, 0.7, 1.0),
            color_texture: Handle::default(),
            tint: Vec4::ONE,
        });
        let mut sources = Vec::new();
        for (index, representation) in [
            CardBodyRepresentation::Sprite,
            CardBodyRepresentation::RiggedFrame,
            CardBodyRepresentation::RiggedParts,
        ].into_iter().enumerate() {
            let actor = world.spawn_empty().id();
            world.resource_mut::<CardRootDepths>().roots.insert(actor, CardBodyDepth {
                plane_z: (index as f32 + 1.0) * 2.5,
                source_z: 20.0,
                order_scale: 1.0,
                front_part: 0.0,
                representation,
            });
            sources.push(world.spawn((
                PresentationOf(actor),
                Mesh2d::default(),
                MeshMaterial2d(material.clone()),
                GlobalTransform::from_translation(Vec3::Z * (20.0 + FLASH_OVERLAY_Z_BIAS)),
                InheritedVisibility::VISIBLE,
            )).id());
        }
        let flash_cards = |world: &mut World| {
            world.query_filtered::<(&CardScope, &Transform), With<CardWorldMesh>>()
                .iter(world)
                .filter_map(|(scope, place)| match scope {
                    CardScope::Source(source) => Some((*source, place.translation.z)),
                    CardScope::Camera(_) => None,
                })
                .collect::<Vec<_>>()
        };
        app.update();
        let mut shown = flash_cards(app.world_mut());
        shown.sort_by(|a, b| a.1.total_cmp(&b.1));
        assert_eq!(shown.len(), 2);
        assert_eq!([shown[0].0, shown[1].0], sources[..2]);
        assert!((2.5..5.0).contains(&shown[0].1) && shown[0].1 > 2.5);
        assert!((5.0..7.5).contains(&shown[1].1) && shown[1].1 > 5.0);

        app.world_mut().resource_mut::<Assets<HitFlashMaterial>>().get_mut(&material).unwrap().control.x = 0.0;
        app.update();
        assert!(flash_cards(app.world_mut()).is_empty());
    }
}
