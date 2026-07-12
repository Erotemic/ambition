//! Room-scoped generated background/parallax spawning and camera-relative motion.
//!
//! Crate choice note: this stays local instead of pulling in a parallax plugin.
//! The sandbox already owns camera follow / room transitions, and generated
//! background assets are optional. A few small components keep the current
//! fallback-friendly loading behavior without forcing the room renderer through
//! an external API.

use ambition_engine_core as ae;
use bevy::camera::visibility::RenderLayers;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
#[cfg(feature = "portal_render")]
use std::collections::HashSet;

use super::primitives::RoomVisual;
use ambition_engine_core::config::{WINDOW_H, WINDOW_W};
use ambition_persistence::settings::ParallaxBudget;
use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sprite_sheet::game_assets::{GameAssets, ParallaxLayerAsset, ParallaxTheme};
use ambition_world::rooms::RoomMetadata;

#[derive(Component, Clone, Copy, Debug)]
pub struct ParallaxLayerVisual {
    /// 0.0 is screen locked; 1.0 tracks gameplay/world motion.
    pub factor: Vec2,
    pub z: f32,
    /// Screen-space room-relative travel budget. We avoid tile repetition by
    /// keeping each layer as a single large panel and shifting it within this
    /// budget based on camera position inside the room.
    pub travel: Vec2,
    pub world_size: Vec2,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundParallaxLayer {
    theme: ParallaxTheme,
    asset: ParallaxLayerAsset,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortalCaptureParallaxLayerVisual {
    rig: Entity,
    source: Entity,
}

#[derive(Clone, Copy)]
struct RuntimeParallaxLayerSpec {
    asset: ParallaxLayerAsset,
    factor: f32,
    z: f32,
    panel_scale: f32,
}

const RUNTIME_PARALLAX_LAYERS: &[RuntimeParallaxLayerSpec] = &[
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::Sky,
        factor: 0.10,
        z: -18.0,
        panel_scale: 1.20,
    },
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::FarBackplate,
        factor: 0.20,
        z: -17.0,
        panel_scale: 1.34,
    },
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::NearBackground,
        factor: 0.42,
        z: -16.0,
        panel_scale: 1.52,
    },
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::ForegroundAtmosphere,
        factor: 0.60,
        z: -15.0,
        panel_scale: 1.72,
    },
];

pub fn spawn_parallax_layers(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    metadata: &RoomMetadata,
    assets: Option<&GameAssets>,
    quality: Option<&ParallaxBudget>,
) {
    let Some(assets) = assets else {
        return;
    };
    if assets.parallax_layers.is_empty() {
        return;
    }
    if quality.is_some_and(|q| !q.enabled) {
        return;
    }
    let theme = ParallaxTheme::from_room_metadata(metadata);
    let viewport = BVec2::new(WINDOW_W as f32, WINDOW_H as f32);
    let panel_base = viewport.x.max(viewport.y);
    let max_layers = quality.and_then(|q| q.max_layers).unwrap_or(usize::MAX);
    for spec in RUNTIME_PARALLAX_LAYERS.iter().take(max_layers) {
        let Some(image) = assets.parallax_layers.get(theme, spec.asset) else {
            continue;
        };
        let panel_extent = panel_base * spec.panel_scale;
        let panel_size = BVec2::splat(panel_extent);
        let travel = ((panel_size - viewport) * 0.5).max(BVec2::ZERO);
        let mut sprite = Sprite::from_image(image.clone());
        sprite.custom_size = Some(panel_size);
        commands.spawn_session_scoped(
            session_scope,
            (
                sprite,
                Transform::from_translation(Vec3::new(0.0, 0.0, spec.z)),
                ParallaxLayerVisual {
                    factor: Vec2::splat(spec.factor),
                    z: spec.z,
                    travel: Vec2::new(travel.x, travel.y),
                    world_size: Vec2::new(world.size.x.max(1.0), world.size.y.max(1.0)),
                },
                BoundParallaxLayer {
                    theme,
                    asset: spec.asset,
                },
                RenderLayers::layer(
                    ambition_platformer_primitives::camera_layers::PARALLAX_BACKGROUND_LAYER,
                ),
                RoomVisual,
                Name::new(format!(
                    "Background parallax layer: {} {}",
                    theme.key(),
                    spec.asset.key()
                )),
            ),
        );
    }
}

pub fn refresh_parallax_layers_on_quality_change(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    world: Res<ambition_engine_core::RoomGeometry>,
    room_set: Res<ambition_world::rooms::RoomSet>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    layers: Query<
        Entity,
        (
            With<ParallaxLayerVisual>,
            Without<PortalCaptureParallaxLayerVisual>,
        ),
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    let assets_changed = assets.is_changed();
    let quality_changed = quality.as_ref().is_some_and(|q| q.is_changed());
    if !assets_changed && !quality_changed {
        return;
    }
    for entity in &layers {
        commands.entity(entity).despawn();
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    spawn_parallax_layers(
        &mut commands,
        session_scope,
        &world.0,
        &room_set.active_spec().metadata,
        Some(assets.as_ref()),
        quality.as_deref().map(|q| &q.budget.parallax),
    );
}

pub fn sync_parallax_layers(
    // `With<MainCamera>`: ignore the #31 cube overlay Camera3d AND the portal
    // view-cone capture `Camera2d`s, so `.single()` still resolves the one main
    // game camera (a broad `With<Camera2d>` now matches the captures too).
    camera: Query<
        &Transform,
        (
            With<ambition_platformer_primitives::camera_layers::MainCamera>,
            Without<ParallaxLayerVisual>,
        ),
    >,
    mut layers: Query<
        (&mut Transform, &ParallaxLayerVisual),
        (Without<Camera>, Without<PortalCaptureParallaxLayerVisual>),
    >,
) {
    let Ok(camera_transform) = camera.single() else {
        return;
    };
    let camera_xy = camera_transform.translation.truncate();
    for (mut transform, layer) in &mut layers {
        sync_parallax_transform_to_camera(&mut transform, layer, camera_xy);
    }
}

#[cfg(feature = "portal_render")]
pub fn sync_portal_capture_parallax_layers(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    sources: Query<
        (Entity, &Sprite, &ParallaxLayerVisual),
        Without<PortalCaptureParallaxLayerVisual>,
    >,
    rigs: Query<
        (Entity, &ambition_portal_presentation::PortalViewRig),
        Without<PortalCaptureParallaxLayerVisual>,
    >,
    mut copies: Query<(
        Entity,
        &PortalCaptureParallaxLayerVisual,
        &mut Sprite,
        &mut Transform,
        &mut RenderLayers,
    )>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let mut live: HashSet<(Entity, Entity)> = HashSet::new();
    for (entity, copy, mut sprite, mut transform, mut render_layers) in &mut copies {
        let Ok((_, source_sprite, source_layer)) = sources.get(copy.source) else {
            commands.entity(entity).despawn();
            continue;
        };
        let Ok((_, rig)) = rigs.get(copy.rig) else {
            commands.entity(entity).despawn();
            continue;
        };
        live.insert((copy.rig, copy.source));
        *sprite = source_sprite.clone();
        *render_layers = RenderLayers::none().with(rig.parallax_layer());
        // Anchor parallax at the MAPPED HOST CAMERA viewpoint (the position a
        // viewer looking through the window sees from), not the capture
        // camera's own framing center — a tight cone-rect frame would
        // otherwise evaluate the background at the wrong viewpoint.
        sync_parallax_transform_to_camera(&mut transform, source_layer, rig.parallax_anchor());
    }

    for (rig_entity, rig) in &rigs {
        for (source_entity, source_sprite, source_layer) in &sources {
            if live.contains(&(rig_entity, source_entity)) {
                continue;
            }
            let mut transform = Transform::default();
            sync_parallax_transform_to_camera(&mut transform, source_layer, rig.parallax_anchor());
            commands.spawn_session_scoped(
                session_scope,
                (
                    source_sprite.clone(),
                    transform,
                    *source_layer,
                    PortalCaptureParallaxLayerVisual {
                        rig: rig_entity,
                        source: source_entity,
                    },
                    RenderLayers::none().with(rig.parallax_layer()),
                    RoomVisual,
                    Name::new(format!(
                        "Portal capture parallax layer {} ({})",
                        rig.parallax_layer(),
                        rig.channel().name()
                    )),
                ),
            );
        }
    }
}

fn sync_parallax_transform_to_camera(
    transform: &mut Transform,
    layer: &ParallaxLayerVisual,
    camera_xy: Vec2,
) {
    let tx = if layer.world_size.x > 1.0 {
        (camera_xy.x / layer.world_size.x).clamp(0.0, 1.0)
    } else {
        0.5
    };
    let ty = if layer.world_size.y > 1.0 {
        (camera_xy.y / layer.world_size.y).clamp(0.0, 1.0)
    } else {
        0.5
    };
    let centered = Vec2::new(tx * 2.0 - 1.0, ty * 2.0 - 1.0);
    let offset = Vec2::new(
        -centered.x * layer.travel.x * layer.factor.x,
        -centered.y * layer.travel.y * layer.factor.y,
    );
    transform.translation.x = camera_xy.x + offset.x;
    transform.translation.y = camera_xy.y + offset.y;
    transform.translation.z = layer.z;
}

#[cfg(all(test, feature = "portal_render"))]
mod tests {
    use super::*;

    #[test]
    fn portal_capture_parallax_system_params_are_disjoint() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, sync_portal_capture_parallax_layers);

        app.update();
    }

    #[test]
    fn portal_capture_parallax_layers_use_dynamic_masks() {
        let private_layer = 32 + 255;
        let copy_layers = RenderLayers::none().with(private_layer);
        let capture_layers = RenderLayers::layer(0).with(private_layer);

        assert!(copy_layers.intersects(&capture_layers));
        assert!(!copy_layers.intersects(&RenderLayers::layer(0)));
    }
}
