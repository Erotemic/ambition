//! The Flying Spaghetti Monster wears its wounds as sauce.
//!
//! Its sheet ships a companion `sauce` layer (`BossSheetSpec::layers`): the
//! same rows and frames, each cell a REVEAL MAP — where marinara sits on that
//! frame and at what damage it appears. This draws that layer's cell over the
//! god's sprite, cell for cell ([`BossDrawnCell`]), revealed up to the god's
//! damage fraction: no sauce at the start of the fight, a splatter by its end.
//!
//! Keyed on the LAYER, not the boss: any boss whose sheet ships a `sauce`
//! layer bleeds this way. Presentation only; the damage it reads is the one
//! the health bar reads (`FeatureView::hp_current` / `hp_max`).

use bevy::{
    asset::embedded_asset,
    image::TextureAtlasLayout,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite::Anchor,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use ambition_platformer2d_shared_tangle::lifecycle::{SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt};
use ambition_render::rendering::{ActorOverlaySet, FeatureVisual, RoomVisual};
use ambition_sprite_sheet::boss::{BossAnimator, BossDrawnCell};

/// The companion layer this draws.
pub const SAUCE_LAYER: &str = "sauce";

/// In front of the god's sprite, behind the renderer's hit-flash overlay (1.5)
/// so a hit still flashes over the sauce.
const SAUCE_Z_BIAS: f32 = 0.8;

pub fn install(app: &mut App) {
    // A headless app has no asset/render stack, and no sauce.
    if app.world().get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>().is_none() {
        return;
    }
    embedded_asset!(app, "shaders/fsm_sauce.wgsl");
    app.add_plugins(Material2dPlugin::<SauceMaterial>::default());
    app.add_systems(
        Update,
        (attach_sauce_overlays, sync_sauce_overlays, cleanup_sauce_overlays)
            .chain()
            .in_set(ActorOverlaySet),
    );
}

/// `uv_rect`: the layer cell in its page; `control.x`: damage fraction,
/// `control.y`: x-flip.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct SauceMaterial {
    #[uniform(0)]
    pub uv_rect: Vec4,
    #[uniform(1)]
    pub control: Vec4,
    #[texture(2)]
    #[sampler(3)]
    pub sauce_texture: Handle<Image>,
}

impl Material2d for SauceMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_content/presentation/shaders/fsm_sauce.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// On the boss sprite: its sauce sibling.
#[derive(Component, Debug, Clone, Copy)]
pub struct SauceSource {
    overlay: Entity,
}

/// On the sibling quad: whose sauce it is.
#[derive(Component, Debug, Clone, Copy)]
pub struct SauceOverlay {
    source: Entity,
}

/// A sauce sibling for every boss sprite whose sheet ships a sauce layer.
pub fn attach_sauce_overlays(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SauceMaterial>>,
    candidates: Query<(Entity, &FeatureVisual, &Transform, &BossAnimator, Option<&SessionScopedEntity>), (With<BossDrawnCell>, Without<SauceSource>)>,
) {
    for (source, visual, transform, animator, session_owner) in &candidates {
        let Some(layer) = animator.layers.iter().find(|l| l.name == SAUCE_LAYER) else {
            continue;
        };
        let material = materials.add(SauceMaterial {
            uv_rect: Vec4::ZERO,
            control: Vec4::ZERO,
            sauce_texture: layer.pages[0].texture.clone(),
        });
        let overlay = commands
            .spawn_session_scoped(
                SessionSpawnScope::new(session_owner.map(|owner| owner.0)),
                (
                    Mesh2d(meshes.add(Rectangle::default())),
                    MeshMaterial2d(material),
                    *transform,
                    // Hidden until its first sync has placed it on a cell.
                    Visibility::Hidden,
                    SauceOverlay { source },
                    RoomVisual,
                    Name::new(format!("Sauce overlay: {}", visual.id)),
                ),
            )
            .id();
        commands.entity(source).insert(SauceSource { overlay });
    }
}

/// Draw the sauce layer's cell for the cell the god is drawn at, revealed to
/// its damage.
pub fn sync_sauce_overlays(
    layouts: Res<Assets<TextureAtlasLayout>>,
    feature_views: Res<ambition_sim_view::FeatureViewIndex>,
    sources: Query<
        (&FeatureVisual, &Transform, &Sprite, Option<&Anchor>, &BossAnimator, &BossDrawnCell, &SauceSource, Option<&Visibility>),
        Without<SauceOverlay>,
    >,
    mut overlays: Query<(&mut Transform, &mut Visibility, &MeshMaterial2d<SauceMaterial>), With<SauceOverlay>>,
    mut materials: ResMut<Assets<SauceMaterial>>,
) {
    for (visual, transform, sprite, anchor, animator, drawn, source, source_visibility) in &sources {
        let Ok((mut overlay_transform, mut overlay_visibility, material)) = overlays.get_mut(source.overlay) else {
            continue;
        };
        let damage = feature_views
            .get(&visual.id)
            .filter(|view| view.hp_max > 0)
            .map_or(0.0, |view| 1.0 - view.hp_current.max(0) as f32 / view.hp_max as f32);
        let hidden = matches!(source_visibility, Some(v) if *v == Visibility::Hidden);
        let cell = animator.layer_cell(SAUCE_LAYER, drawn.row, drawn.frame);
        let (Some((layer, cell)), false, true) = (cell, hidden, damage > 0.0) else {
            *overlay_visibility = Visibility::Hidden;
            continue;
        };
        let Some(page) = layer.pages.get(cell.page as usize) else {
            *overlay_visibility = Visibility::Hidden;
            continue;
        };
        let Some(layout) = layouts.get(&page.layout) else {
            continue;
        };
        let Some(rect) = layout.textures.get(cell.index) else {
            continue;
        };
        let size = Vec2::new(layout.size.x.max(1) as f32, layout.size.y.max(1) as f32);
        // The layer cell's own trim; an untrimmed layer covers the sprite's quad.
        let (render_size, mut cell_anchor) = match cell.render {
            Some((render_size, anchor)) => (render_size, anchor),
            None => (sprite.custom_size.unwrap_or(Vec2::ONE), anchor.map_or(Vec2::ZERO, |a| a.0)),
        };
        if sprite.flip_x && cell.render.is_some() {
            cell_anchor.x = -cell_anchor.x;
        }
        *overlay_transform = super::deep_dream::sibling_quad_transform(transform, Some(cell_anchor), render_size, SAUCE_Z_BIAS);
        *overlay_visibility = Visibility::Visible;
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.uv_rect = Vec4::new(
                rect.min.x as f32 / size.x,
                rect.min.y as f32 / size.y,
                rect.max.x as f32 / size.x,
                rect.max.y as f32 / size.y,
            );
            material.control = Vec4::new(damage, if sprite.flip_x { 1.0 } else { 0.0 }, 0.0, 0.0);
            if material.sauce_texture != page.texture {
                material.sauce_texture = page.texture.clone();
            }
        }
    }
}

/// A sauce sibling whose god is gone goes with it.
pub fn cleanup_sauce_overlays(mut commands: Commands, sources: Query<(), With<SauceSource>>, overlays: Query<(Entity, &SauceOverlay)>) {
    for (overlay, of) in &overlays {
        if sources.get(of.source).is_err() {
            commands.entity(overlay).despawn();
        }
    }
}
