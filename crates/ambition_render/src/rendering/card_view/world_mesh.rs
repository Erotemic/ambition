use std::collections::{HashMap, HashSet};

use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers, VisibilitySystems};
use bevy::ecs::system::{SystemParam, SystemParamItem};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, AsBindGroupError, BindGroupLayout, BindGroupLayoutEntry, BlendState, RenderPipelineDescriptor,
    SpecializedMeshPipelineError, UnpreparedBindGroup,
};
use bevy::render::renderer::RenderDevice;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, MeshMaterial2d};
use bevy::transform::TransformSystems;

use super::{CARD_BODY_STEP, TERRAIN_DEPTH, scenery_depth};
use super::{CardRootDepths, CardScope, CardViewMode, CardViewSettings, RetainedEntities};
use crate::folded_mesh::{AddedLightMaterial, FoldedMesh};
use crate::rendering::terrain_skin::TerrainSkinMesh;

#[derive(Component)]
pub(super) struct CardWorldMesh;

#[derive(SystemSet, Debug, Clone, Eq, PartialEq, Hash)]
pub(super) struct CardWorldMeshSet;

pub(super) fn install(app: &mut App) {
    app.configure_sets(
        PostUpdate,
        CardWorldMeshSet
            .after(TransformSystems::Propagate)
            .after(VisibilitySystems::VisibilityPropagate)
            .before(VisibilitySystems::CheckVisibility),
    );
    app.add_systems(
        PostUpdate,
        realize_folded_meshes
            .in_set(CardWorldMeshSet)
            .run_if(cards_are_visible),
    );
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        install_card_mesh_material::<ambition_sprite_fx::SpriteFxMaterial>(app);
        install_card_mesh_material::<crate::rendering::darkness::DarknessMaterial>(app);
        install_card_mesh_material::<crate::bubble_shield_fx::BubbleShieldFxMaterial>(app);
        install_card_mesh_material::<crate::rendering::daylight::DaylightMaterial>(app);
        install_card_mesh_material::<crate::rendering::daylight::LampGlowMaterial>(app);
        #[cfg(feature = "portal_render")]
        install_card_mesh_material::<ambition_portal2d_presentation::PortalGlowMaterial>(app);
    }
}

/// A planar material with a fragment shader for the card output.
pub trait CardMaterialSource: Material2d + PartialEq {
    /// A blend state other than the material's alpha mode.
    fn card_blend() -> Option<BlendState> {
        None
    }
}

impl CardMaterialSource for ambition_sprite_fx::SpriteFxMaterial {}
impl CardMaterialSource for crate::rendering::darkness::DarknessMaterial {}
impl CardMaterialSource for crate::bubble_shield_fx::BubbleShieldFxMaterial {}
#[cfg(feature = "portal_render")]
impl CardMaterialSource for ambition_portal2d_presentation::PortalGlowMaterial {}

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
struct CardMaterial<M: Material2d>(#[dependency] M);

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
) {
    if !cards_are_visible(settings) {
        by_material.clear();
        return;
    }
    let Some(source_materials) = source_materials else {
        return;
    };
    products
        .retained
        .reconcile(&mut products.commands, |entity| {
            sources
                .get(*entity)
                .is_ok_and(|(_, _, material, _, _, visible, _)| {
                    visible.get() && source_materials.contains(&material.0)
                })
        });
    used.clear();
    for (source, mesh, material, local, global, visible, terrain) in &sources {
        if !visible.get() {
            continue;
        }
        let Some(draw) = source_materials.get(&material.0) else {
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
        let place = if terrain {
            super::sprite_card_place(
                &global.compute_transform(),
                TERRAIN_DEPTH + local.translation.z,
            )
        } else {
            effect_place(source, global, &rooms, &depths)
        };
        products.show(source, &mesh.0, card, place, false);
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
