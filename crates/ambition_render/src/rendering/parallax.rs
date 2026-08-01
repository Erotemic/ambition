//! Room-scoped generated background/parallax spawning and camera-relative motion.
//!
//! Crate choice note: this stays local instead of pulling in a parallax plugin.
//! The sandbox already owns camera follow / room transitions, and generated
//! background assets are optional. A few small components keep the current
//! fallback-friendly loading behavior without forcing the room renderer through
//! an external API.

use ambition_platformer2d_core as ae;
use bevy::camera::visibility::RenderLayers;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
#[cfg(feature = "portal_render")]
use std::collections::HashSet;

use super::primitives::RoomVisual;
use ambition_platformer2d_core::config::{WINDOW_H, WINDOW_W};
use ambition_persistence::settings::ParallaxBudget;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sprite_sheet::game_assets::{GameAssets, ParallaxLayerAsset, ParallaxTheme};
use ambition_platformer2d_world::rooms::RoomMetadata;

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
                    ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER,
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

/// ⚠ **The two session-world reads are OPTIONAL, and that is a fact about who
/// runs this now.** While it was registered by `game/ambition_app` alone, its
/// `Single` params were always satisfied — that host's session root carries both.
/// Installing it engine-side (S12) ran it in compositions whose root carries
/// neither, and a `Single` that matches nothing is a system-param VALIDATION
/// PANIC, not a skip: eight tests across `ambition_platformer2d_host` and both consumer
/// fixtures died on *"Resource does not exist"* with the system name compiled
/// out. A world with no room geometry has no parallax to refresh, which is an
/// ordinary state and not an error.
pub fn refresh_parallax_layers_on_quality_change(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    world: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_core::RoomGeometry,
        >,
    >,
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ambition_platformer2d_world::rooms::RoomSet>,
    >,
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
    let (Some(assets), Some(world), Some(room_set)) = (assets, world, room_set) else {
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

/// **Load the ACTIVE room's parallax theme, in whatever composition is
/// running.**
///
/// ⚠ **This was app-local, and it is the recurring class.** The lazy load lived
/// in `game/ambition_app`'s room-transition machinery
/// (`build_room_asset_manifest`), so the shipped host got a backdrop in every
/// biome and every other composition — the demos, the external consumer, any
/// game built through `PlatformerApp` — got the STARTUP room's theme and nothing
/// else. And it failed the quiet way: [`spawn_parallax_layers`] skips a layer
/// whose handle is absent, so a room in a second biome simply has no background
/// and says nothing. Same shape as the world-label placement pass (AE1), which
/// is why the plugin that spawns the layers now owns the load as well.
///
/// It is a lazy load rather than a preload for the reason the original comment
/// gives: startup pays for the first room's zone art only, and a theme is loaded
/// the first time a room asks for it.
///
/// The load MUTATES [`GameAssets`], which is exactly the signal
/// [`refresh_parallax_layers_on_quality_change`] watches — so the layers that
/// were skipped respawn on the next frame with no extra wiring. `attempted`
/// keeps a theme whose art is genuinely absent from re-deriving its paths (and
/// re-touching `GameAssets`) every frame, which would respawn every layer in
/// the world forever.
pub fn ensure_active_room_parallax_theme(
    assets: Option<ResMut<GameAssets>>,
    catalog: Option<Res<ambition_asset_manager::sandbox_assets::SandboxAssetCatalog>>,
    asset_server: Option<Res<AssetServer>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ambition_platformer2d_world::rooms::RoomSet>,
    >,
    mut attempted: Local<Vec<ParallaxTheme>>,
) {
    let (Some(mut assets), Some(catalog), Some(asset_server), Some(room_set)) =
        (assets, catalog, asset_server, room_set)
    else {
        return;
    };
    // A rebuilt `GameAssets` (a fresh bind) starts with no themes at all, so the
    // memo has to start again with it — otherwise a theme this system already
    // "attempted" would never be loaded into the new set.
    if assets.is_added() {
        attempted.clear();
    }
    let metadata = room_set.active_spec().metadata.clone();
    let theme = ParallaxTheme::from_room_metadata(&metadata);
    if attempted.contains(&theme) {
        return;
    }
    attempted.push(theme);
    // Already present — the startup bind loads the first room's theme, and the
    // Ambition host's own transition path may have loaded others. Returning
    // without touching `GameAssets` matters: a mutable deref alone marks it
    // changed, and the refresh system would despawn and respawn every layer in
    // the world to arrive at the same picture.
    if ParallaxLayerAsset::ALL
        .iter()
        .any(|layer| assets.parallax_layers.get(theme, *layer).is_some())
    {
        return;
    }
    ambition_sprite_sheet::game_assets::ensure_parallax_layers_for_room(
        &mut assets,
        &catalog,
        &asset_server,
        &metadata,
        quality.as_deref().map(|q| &q.budget),
    );
}

pub fn sync_parallax_layers(
    // `With<MainCamera>`: ignore the #31 cube overlay Camera3d AND the portal
    // view-cone capture `Camera2d`s, so `.single()` still resolves the one main
    // game camera (a broad `With<Camera2d>` now matches the captures too).
    camera: Query<
        &Transform,
        (
            With<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>,
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
        (Entity, &ambition_portal2d_presentation::PortalViewRig),
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

/// **A second biome gets its own backdrop, in a composition that is not the
/// shipped app.**
#[cfg(test)]
mod theme_load_tests {
    use super::*;
    use ambition_asset_manager::profile::AssetProfile;
    use ambition_asset_manager::sandbox_assets::SandboxAssetCatalog;
    use ambition_platformer2d_shared_tangle::lifecycle::{SessionRoot, SessionScopeId};

    /// Trusts packaging rather than the filesystem — `AndroidBundle` is the
    /// profile whose `should_attempt_resolved_load` is unconditionally true, so
    /// this test asks "does the engine REQUEST the theme's art" without also
    /// asking whether a generated PNG happens to exist next to the test binary.
    fn packaged_catalog() -> SandboxAssetCatalog {
        let manifest = ambition_sprite_sheet::game_assets::sandbox_image_manifest("sprites");
        SandboxAssetCatalog::new(
            ambition_asset_manager::AmbitionAssetCatalog::new(manifest),
            AssetProfile::AndroidBundle,
        )
    }

    /// One room, in a biome that is deliberately NOT the engine's default.
    fn room_set_in(theme_key: &str) -> ambition_platformer2d_world::rooms::RoomSet {
        let mut room = ambition_platformer2d_world::rooms::RoomSpec::new(
            "second_biome",
            ambition_platformer2d_core::World::new(
                "second_biome",
                ambition_platformer2d_core::Vec2::new(640.0, 480.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        room.metadata.visual_profile.parallax_theme = Some(theme_key.to_string());
        ambition_platformer2d_world::rooms::RoomSet::from_parts("second_biome", vec![room], Vec::new())
    }

    /// **The theme the ACTIVE room asks for is loaded by whoever presents it.**
    ///
    /// This lived in `game/ambition_app`'s room-transition machinery, so the
    /// shipped host had a backdrop in every biome and every other composition —
    /// the demos, the external consumer, anything built through `PlatformerApp`
    /// — drew the startup room's theme and nothing else. Silently:
    /// [`spawn_parallax_layers`] skips a layer whose handle is absent, so the
    /// second biome simply had no sky.
    #[test]
    fn a_room_in_a_second_biome_loads_its_own_parallax_theme() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        app.insert_resource(GameAssets::default());
        app.insert_resource(packaged_catalog());
        app.world_mut()
            .spawn((SessionRoot(SessionScopeId(1)), room_set_in("cave")));
        // ⚠ the PLUGIN, not the system: the defect was never that the load did
        // not work, it was that only one composition installed it. A test that
        // adds the system by hand cannot go red on the thing that was wrong.
        app.add_plugins(crate::platformer_presentation::SessionRoomVisualsPlugin);

        // Non-vacuity: nothing has this theme before the frame runs, so the
        // assertion below is about the system and not about a default.
        assert!(app
            .world()
            .resource::<GameAssets>()
            .parallax_layers
            .get(ParallaxTheme::Cave, ParallaxLayerAsset::ALL[0])
            .is_none());

        app.update();

        assert!(
            app.world()
                .resource::<GameAssets>()
                .parallax_layers
                .get(ParallaxTheme::Cave, ParallaxLayerAsset::ALL[0])
                .is_some(),
            "the active room asked for the `cave` theme and nothing loaded it, \
             so every one of its layers would be skipped and the room would \
             draw no background at all"
        );
    }

    /// **The layers MOVE with the camera, in every composition.**
    ///
    /// `sync_parallax_layers` was app-local too — the same class one step
    /// further along. A composition that got its backdrop spawned still left it
    /// at the world origin forever, so it slid out of frame as the camera walked
    /// away and the one thing a parallax layer is for never happened. Nothing
    /// about that reads as a missing system: the art is correct, in the wrong
    /// place, and only when you walk.
    #[test]
    fn the_backdrop_follows_the_camera_in_a_composition_that_is_not_the_app() {
        use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        app.insert_resource(GameAssets::default());
        app.insert_resource(packaged_catalog());
        app.world_mut()
            .spawn((SessionRoot(SessionScopeId(1)), room_set_in("cave")));
        app.add_plugins(crate::platformer_presentation::SessionRoomVisualsPlugin);

        // A camera well away from the origin, and one layer sitting at it.
        app.world_mut()
            .spawn((MainCamera, Transform::from_xyz(900.0, 0.0, 0.0)));
        let layer = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.0, 0.0, -18.0),
                ParallaxLayerVisual {
                    factor: Vec2::splat(0.5),
                    z: -18.0,
                    travel: Vec2::new(120.0, 0.0),
                    world_size: Vec2::new(2000.0, 480.0),
                },
            ))
            .id();

        app.update();

        let moved = app.world().get::<Transform>(layer).unwrap().translation;
        assert!(
            moved.x != 0.0,
            "the layer never moved: the backdrop is pinned to the world origin \
             while the camera stands at x=900, which is what a composition \
             without `sync_parallax_layers` draws"
        );
    }

    /// **And it does not touch `GameAssets` once the theme is in.**
    ///
    /// A mutable deref alone marks the resource changed, and
    /// [`refresh_parallax_layers_on_quality_change`] answers that by despawning
    /// and respawning every parallax layer in the world. A load that ran every
    /// frame would rebuild the backdrop every frame — the fix that is worse than
    /// the bug.
    #[test]
    fn a_theme_already_loaded_is_not_reloaded_every_frame() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        app.insert_resource(GameAssets::default());
        app.insert_resource(packaged_catalog());
        app.world_mut()
            .spawn((SessionRoot(SessionScopeId(1)), room_set_in("cave")));
        app.add_plugins(crate::platformer_presentation::SessionRoomVisualsPlugin);

        app.update();
        // Two more frames: the theme is present, so neither may report a change.
        app.update();
        let changed = app.world().resource_ref::<GameAssets>().is_changed();
        assert!(
            !changed,
            "the theme load ran again on a frame where nothing was missing"
        );
    }
}
