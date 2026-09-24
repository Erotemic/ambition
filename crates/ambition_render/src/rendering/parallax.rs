//! Room-scoped generated parallax with one panel set per local view.
//!
//! A panel's transform and travel depend on its observer's viewport, so additional
//! views receive mirrored sets keyed by [`ambition_sim_view::PresentedForView`].
//! Each set is re-derived from its owning view's camera viewport every frame.

use ambition_platformer2d_core as ae;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
#[cfg(feature = "portal_render")]
use std::collections::HashSet;

use super::primitives::RoomVisual;
use super::view_isolation::ProjectionRestingLayers;
use ambition_persistence::settings::ParallaxBudget;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::rooms::RoomMetadata;
use ambition_sprite_sheet::game_assets::{GameAssets, ParallaxLayerAsset, ParallaxTheme};

/// Camera-relative background panel.
///
/// Layer parameters are fixed at spawn; [`Self::travel`] is derived each frame
/// from the owning view's viewport.
#[derive(Component, Clone, Copy, Debug)]
pub struct ParallaxLayerVisual {
    /// 0.0 is screen locked; 1.0 tracks gameplay/world motion.
    pub factor: Vec2,
    pub z: f32,
    /// Panel extent as a multiple of the longer side of the drawing view's
    /// viewport. Each layer is one large panel that shifts inside its overhang,
    /// so no tiles repeat.
    pub panel_scale: f32,
    /// Screen-space room-relative travel budget, derived each frame. Zero until
    /// the first sync, and zero for a panel that no view draws.
    pub travel: Vec2,
    pub world_size: Vec2,
}

impl ParallaxLayerVisual {
    /// The square panel this layer wants in a viewport of `viewport_px`.
    pub fn panel_size(&self, viewport_px: Vec2) -> Vec2 {
        Vec2::splat(viewport_px.x.max(viewport_px.y) * self.panel_scale)
    }

    /// How far the panel may slide inside that viewport before its edge shows:
    /// half the overhang, per axis, never negative.
    pub fn travel_in(&self, viewport_px: Vec2) -> Vec2 {
        ((self.panel_size(viewport_px) - viewport_px) * 0.5).max(Vec2::ZERO)
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundParallaxLayer {
    theme: ParallaxTheme,
    asset: ParallaxLayerAsset,
}

/// Mirrored parallax panel linked to its room-owned root for lifecycle and
/// root/copy query separation.
#[derive(Component, Clone, Copy, Debug)]
pub struct MirroredParallaxLayer {
    pub root: Entity,
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

/// The layers every main camera renders the room's backdrop on.
///
/// This is the panel's resting mask. The per-view isolation pass replaces
/// `RenderLayers` on everything keyed by `PresentedForView`, so the layer to
/// return to after a collapse to one view cannot be read back from the entity.
/// This keeps a collapsed session's backdrop off layer 0, where the portal
/// capture cameras would draw it from the wrong eye.
fn parallax_resting_layers() -> RenderLayers {
    RenderLayers::layer(
        ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER,
    )
}

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
    let max_layers = quality.and_then(|q| q.max_layers).unwrap_or(usize::MAX);
    for spec in RUNTIME_PARALLAX_LAYERS.iter().take(max_layers) {
        let Some(image) = assets.parallax_layers.get(theme, spec.asset) else {
            continue;
        };
        // No size here: the extent depends on the drawing viewport, and no view is in
        // scope. `sync_parallax_layers` sizes it against the owning view.
        let mut sprite = Sprite::from_image(image.clone());
        sprite.custom_size = None;
        commands.spawn_session_scoped(
            session_scope,
            (
                sprite,
                Transform::from_translation(Vec3::new(0.0, 0.0, spec.z)),
                Visibility::Inherited,
                ParallaxLayerVisual {
                    factor: Vec2::splat(spec.factor),
                    z: spec.z,
                    panel_scale: spec.panel_scale,
                    travel: Vec2::ZERO,
                    world_size: Vec2::new(world.size.x.max(1.0), world.size.y.max(1.0)),
                },
                BoundParallaxLayer {
                    theme,
                    asset: spec.asset,
                },
                parallax_resting_layers(),
                ProjectionRestingLayers(parallax_resting_layers()),
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

/// The two session-world reads are optional. Some compositions have no room
/// geometry on the root, and a `Single` that matches nothing panics at param
/// validation. A world with no room geometry has no parallax to refresh.
///
/// The despawn sweep takes roots and copies alike (every [`ParallaxLayerVisual`]
/// that is not a portal capture copy): the backdrop is rebuilt, and a copy of a
/// despawned root has no owner. [`mirror_parallax_layers_per_view`] rebuilds the
/// per-view set from the new roots.
pub fn refresh_parallax_layers_on_quality_change(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    world: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_core::RoomGeometry,
        >,
    >,
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
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

/// Lazily load the active room's parallax theme.
///
/// Loading mutates [`GameAssets`], which causes skipped layers to be rebuilt by
/// [`refresh_parallax_layers_on_quality_change`]. `attempted` prevents missing
/// themes from being retried every frame and repeatedly invalidating layers.
/// The theme loads this session has tried, and which of them produced no art.
///
/// This is a resource so that presentation can read it:
/// `sync_session_room_visuals` must tell "not arrived yet" from "resolved to
/// nothing". This occurs in shipped profiles: `WebStatic` / `BundledStatic`
/// load an optional image only with an embedded candidate, and the generated
/// parallax manifest has none, so the load yields zero handles.
#[derive(bevy::prelude::Resource, Default, Debug)]
pub struct ParallaxThemeAttempts {
    // `pub(crate)` so `platformer_presentation` tests can stage "tried and found
    // nothing" without an asset server. Read it through `attempted_without_art`.
    pub(crate) attempted: Vec<ParallaxTheme>,
    /// Attempted, and the asset profile produced no layer at all.
    pub(crate) without_art: Vec<ParallaxTheme>,
}

impl ParallaxThemeAttempts {
    /// Has this theme been tried and come back with nothing?
    ///
    /// Not "is it missing": a theme not yet attempted is also missing, and is
    /// worth waiting for.
    pub fn attempted_without_art(&self, theme: ParallaxTheme) -> bool {
        self.without_art.contains(&theme)
    }
}

pub fn ensure_active_room_parallax_theme(
    assets: Option<ResMut<GameAssets>>,
    catalog: Option<Res<ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog>>,
    asset_server: Option<Res<AssetServer>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    attempts: Option<ResMut<ParallaxThemeAttempts>>,
) {
    let (Some(mut assets), Some(catalog), Some(asset_server), Some(room_set), Some(mut attempts)) =
        (assets, catalog, asset_server, room_set, attempts)
    else {
        return;
    };
    // A rebuilt `GameAssets` has no themes, so the memo restarts with it.
    // Otherwise a theme already "attempted" would never load into the new set.
    if assets.is_added() {
        attempts.attempted.clear();
        attempts.without_art.clear();
    }
    let metadata = room_set.active_spec().metadata.clone();
    let theme = ParallaxTheme::from_room_metadata(&metadata);
    if attempts.attempted.contains(&theme) {
        return;
    }
    attempts.attempted.push(theme);
    // Already present. Return without touching `GameAssets`: a mutable deref
    // marks it changed, and the refresh system would respawn every layer.
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
    // Record the outcome, not the attempt. Only "nothing came" lets presentation
    // stop waiting. A profile that refuses every candidate leaves zero handles,
    // and no later frame adds one.
    if !ParallaxLayerAsset::ALL
        .iter()
        .any(|layer| assets.parallax_layers.get(theme, *layer).is_some())
    {
        attempts.without_art.push(theme);
    }
}

/// Maintain one parallax panel set per live local view.
///
/// The lowest `LocalViewId` deterministically claims each room-spawned root;
/// additional views receive copies. Removed views despawn their copies, while a
/// root is re-keyed to the lowest survivor. Never clear `PresentedForView` on a
/// still-rendered sprite, because it would become an unscoped draw.
#[allow(clippy::type_complexity)]
pub fn mirror_parallax_layers_per_view(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    views: Query<(Entity, &ambition_sim_view::LocalViewId), With<ambition_sim_view::LocalView>>,
    roots: Query<
        (
            Entity,
            &Sprite,
            &ParallaxLayerVisual,
            Option<&BoundParallaxLayer>,
            Option<&ambition_sim_view::PresentedForView>,
        ),
        (
            Without<MirroredParallaxLayer>,
            Without<PortalCaptureParallaxLayerVisual>,
        ),
    >,
    copies: Query<
        (
            Entity,
            &MirroredParallaxLayer,
            &ambition_sim_view::PresentedForView,
        ),
        With<ParallaxLayerVisual>,
    >,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };

    let mut ordered: Vec<(ambition_sim_view::LocalViewId, Entity)> =
        views.iter().map(|(view, id)| (*id, view)).collect();
    ordered.sort_by_key(|(id, _)| *id);
    let Some((_, root_view)) = ordered.first().copied() else {
        // No observation seam, so nothing presents (see
        // `ambition_sim_view::ViewsOnHand`).
        return;
    };

    // Retract before spawning, so a removed view takes its whole set with it.
    let live: std::collections::HashSet<Entity> = ordered.iter().map(|(_, view)| *view).collect();
    let mut mirrored: std::collections::HashSet<(Entity, Entity)> =
        std::collections::HashSet::new();
    for (entity, copy, key) in &copies {
        let root_is_gone = roots.get(copy.root).is_err();
        // `key.0 == root_view` is the re-key case: the root now draws this view, so
        // the copy is a duplicate.
        if root_is_gone || !live.contains(&key.0) || key.0 == root_view {
            // A teardown can retire this copy in the same frame. Retraction is the
            // wanted outcome, so a missing target is success.
            commands.entity(entity).try_despawn();
            continue;
        }
        mirrored.insert((copy.root, key.0));
    }

    for (root, sprite, layer, bound, key) in &roots {
        if key.map(|key| key.0) != Some(root_view) {
            // Room replacement (including LDtk hot-reload) can queue this root's
            // destruction before Commands flush. The tag is presentation-only, so skip
            // the stale write instead of panicking.
            commands
                .entity(root)
                .try_insert(ambition_sim_view::PresentedForView(root_view));
        }
        for (_, view) in ordered.iter().skip(1) {
            if mirrored.contains(&(root, *view)) {
                continue;
            }
            let mut copied_sprite = sprite.clone();
            // `sync_parallax_layers` sizes it against its own view on the frame it appears.
            copied_sprite.custom_size = None;
            let mut copied_layer = *layer;
            copied_layer.travel = Vec2::ZERO;
            let mut copy = commands.spawn_session_scoped(
                session_scope,
                (
                    copied_sprite,
                    Transform::from_translation(Vec3::new(0.0, 0.0, layer.z)),
                    // Hidden until placed, so a screen-sized panel never draws at the world
                    // origin for one frame.
                    Visibility::Hidden,
                    copied_layer,
                    MirroredParallaxLayer { root },
                    ambition_sim_view::PresentedForView(*view),
                    parallax_resting_layers(),
                    ProjectionRestingLayers(parallax_resting_layers()),
                    RoomVisual,
                    // No `Name`: `entity.name` is registered for rollback, and the coverage
                    // contract would then sweep the whole per-view presentation set.
                ),
            );
            if let Some(bound) = bound {
                copy.insert(*bound);
            }
        }
    }
}

/// Each panel follows the camera that draws it, inside that camera's own
/// viewport.
///
/// Each camera resolves its view through `PresentsView` (the
/// `ambition_sim_view::ViewsOnHand` rule that the follow camera, viewport
/// applier and draw lookup share). Each panel resolves its view through
/// `PresentedForView`.
///
/// A panel whose view or camera cannot be resolved is hidden, and its
/// transform is not changed. It does not use another camera or fall back to
/// the origin: a missing backdrop is an obvious defect, a backdrop at the
/// origin looks like an authoring mistake.
///
/// Extent and travel come from the view's
/// [`ambition_sim_view::camera_snapshot::CameraViewport`] each frame, not from
/// `WINDOW_W`/`WINDOW_H`, so letterboxed and split-screen rectangles use the
/// same arithmetic.
#[allow(clippy::type_complexity)]
pub fn sync_parallax_layers(
    // The viewport is optional. If it were required, a view without one would be
    // invisible to this query, `ViewsOnHand::survey` would count too few views,
    // and an unkeyed panel would get a view instead of a refusal.
    // (`the_plugin_spawns_one_complete_view_at_build_time` pins the normal case.)
    views: Query<
        (
            Entity,
            Option<&ambition_sim_view::camera_snapshot::CameraViewport>,
        ),
        With<ambition_sim_view::LocalView>,
    >,
    // `With<MainCamera>` excludes the #31 cube overlay and the portal capture
    // cameras. Capture rigs get copies through
    // `sync_portal_capture_parallax_layers`.
    cameras: Query<
        (&Transform, Option<&ambition_sim_view::PresentsView>),
        (
            With<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>,
            Without<ParallaxLayerVisual>,
        ),
    >,
    mut layers: Query<
        (
            &mut Transform,
            &mut Sprite,
            &mut Visibility,
            &mut ParallaxLayerVisual,
            Option<&ambition_sim_view::PresentedForView>,
        ),
        (Without<Camera>, Without<PortalCaptureParallaxLayerVisual>),
    >,
) {
    let on_hand = ambition_sim_view::ViewsOnHand::survey(views.iter().map(|(view, _)| view));

    // Where each view's camera stands, and how big that view's rectangle is.
    //
    // Two cameras on one view get the same framing from `camera_follow`, so
    // `or_insert` is order-independent. Two views give two rows.
    let mut drawn_by: std::collections::HashMap<Entity, (Vec2, Vec2)> =
        std::collections::HashMap::new();
    for (camera_transform, link) in &cameras {
        let Some(view) = on_hand.presented_by(link.copied()) else {
            continue;
        };
        let Ok((_, viewport)) = views.get(view) else {
            bevy::log::error_once!("a camera presents view {view:?}, which is not a local view");
            continue;
        };
        let Some(viewport) = viewport else {
            bevy::log::error_once!(
                "local view {view:?} carries no `CameraViewport`, so a backdrop \
                 drawn for it has no rectangle to be sized against; it declines to \
                 draw rather than borrowing the design window's"
            );
            continue;
        };
        drawn_by
            .entry(view)
            .or_insert((camera_transform.translation.truncate(), viewport.px));
    }

    for (mut transform, mut sprite, mut visibility, mut layer, key) in &mut layers {
        let resolved = on_hand
            .drawn_for(key.copied())
            .and_then(|view| drawn_by.get(&view).copied());
        let Some((camera_xy, viewport_px)) = resolved else {
            // No view claims this panel, or its view has no camera. Decline (see the
            // system doc).
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        if *visibility == Visibility::Hidden {
            *visibility = Visibility::Inherited;
        }

        // Write only when the viewport changed, so a settled panel makes no change
        // ticks.
        let panel_size = layer.panel_size(viewport_px);
        if sprite.custom_size != Some(panel_size) {
            sprite.custom_size = Some(panel_size);
        }
        let travel = layer.travel_in(viewport_px);
        if layer.travel != travel {
            layer.travel = travel;
        }
        sync_parallax_transform_to_camera(&mut transform, &layer, camera_xy);
    }
}

#[cfg(feature = "portal_render")]
pub fn sync_portal_capture_parallax_layers(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    // Copy the root set only (`Without<MirroredParallaxLayer>`); copying every
    // view's panels would stack N skies in one capture. Portal camera continuity
    // is one global host view (`PortalCameraContinuityState`/`HostView`), so the
    // root set is its source.
    sources: Query<
        (Entity, &Sprite, &ParallaxLayerVisual),
        (
            Without<PortalCaptureParallaxLayerVisual>,
            Without<MirroredParallaxLayer>,
        ),
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
        // Anchor at the mapped host camera viewpoint, not the capture camera's frame
        // center. A tight cone-rect frame would give the wrong viewpoint.
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
    // `camera_xy` is centred (−size/2 ..= +size/2), so add 0.5 after dividing by
    // the full `world_size`. Without it the clamp flattens the left half of the
    // room to 0 and the backdrop stops moving there.
    let tx = if layer.world_size.x > 1.0 {
        (camera_xy.x / layer.world_size.x + 0.5).clamp(0.0, 1.0)
    } else {
        0.5
    };
    let ty = if layer.world_size.y > 1.0 {
        (camera_xy.y / layer.world_size.y + 0.5).clamp(0.0, 1.0)
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

/// A second biome gets its own backdrop, in a composition that is not the
/// shipped app.
#[cfg(test)]
mod theme_load_tests {
    use super::*;
    use ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog;
    use ambition_asset_manager::profile::AssetProfile;
    use ambition_platformer2d_shared_tangle::lifecycle::{SessionRoot, SessionScopeId};

    /// `AndroidBundle` always attempts a resolved load, so this test checks that
    /// the engine requests the art, not that a PNG exists on disk.
    fn packaged_catalog() -> Platformer2dAssetCatalog {
        let manifest = ambition_sprite_sheet::game_assets::sandbox_image_manifest("sprites");
        Platformer2dAssetCatalog::new(
            ambition_asset_manager::AmbitionAssetCatalog::new(manifest),
            AssetProfile::AndroidBundle,
        )
    }

    /// One room, in a biome that is not the engine's default.
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
        ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
            "second_biome",
            vec![room],
            Vec::new(),
        )
    }

    /// The active room's theme is loaded by whoever presents it, in every
    /// composition. [`spawn_parallax_layers`] skips a layer with no handle, so a
    /// missing load gives a biome no sky, silently.
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
        // Use the real plugin: a test that adds the system by hand cannot catch a missing registration.
        app.add_plugins(crate::platformer_presentation::SessionRoomVisualsPlugin);

        // Non-vacuity: nothing has this theme before the frame runs.
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

    /// The layers move with the camera, in every composition. Otherwise a panel
    /// stays at the world origin and slides out of frame as the camera moves.
    ///
    /// The fixture spawns a local view as well as a camera, because the sync
    /// resolves a camera through its view (as `layout_world_labels` and
    /// `sync_actor_nameplates` do). `CameraObservationPlugin` spawns that view at
    /// plugin build time.
    #[test]
    fn the_backdrop_follows_the_camera_in_a_composition_that_is_not_the_app() {
        use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
        use ambition_sim_view::camera_snapshot::CameraViewport;
        use ambition_sim_view::{LocalView, LocalViewId, PresentsView};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        app.insert_resource(GameAssets::default());
        app.insert_resource(packaged_catalog());
        app.world_mut()
            .spawn((SessionRoot(SessionScopeId(1)), room_set_in("cave")));
        app.add_plugins(crate::platformer_presentation::SessionRoomVisualsPlugin);

        let view = app
            .world_mut()
            .spawn((LocalView, LocalViewId(0), CameraViewport::default()))
            .id();
        // A camera well away from the origin, and one layer sitting at it.
        app.world_mut().spawn((
            MainCamera,
            PresentsView(view),
            Transform::from_xyz(900.0, 0.0, 0.0),
        ));
        let layer = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.0, 0.0, -18.0),
                Visibility::Inherited,
                Sprite::default(),
                ParallaxLayerVisual {
                    factor: Vec2::splat(0.5),
                    z: -18.0,
                    panel_scale: 1.2,
                    travel: Vec2::ZERO,
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

    /// And it does not touch `GameAssets` once the theme is in.
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

/// Two views, two backdrops: each from its own camera and viewport.
#[cfg(test)]
mod parallax_travel_tests {
    use super::*;

    fn layer(world_w: f32) -> ParallaxLayerVisual {
        ParallaxLayerVisual {
            factor: Vec2::splat(1.0),
            z: 0.0,
            panel_scale: 1.0,
            travel: Vec2::new(100.0, 0.0),
            world_size: Vec2::new(world_w, 480.0),
        }
    }

    fn offset_at(camera_x: f32, world_w: f32) -> f32 {
        let mut t = Transform::default();
        sync_parallax_transform_to_camera(&mut t, &layer(world_w), Vec2::new(camera_x, 0.0));
        // The panel is at the camera plus a parallax offset, so only the offset is tested.
        t.translation.x - camera_x
    }

    /// The backdrop must travel across the whole room, not the right half.
    ///
    /// `camera_xy` is the centred camera transform (`camera.rs`:
    /// `center_world.x - size.x * 0.5`), so it runs −size/2 ..= +size/2 while
    /// `world_size` is the full span. Assert span and midpoint together: either
    /// alone passes for a broken mapping.
    #[test]
    fn the_backdrop_travels_the_full_width_of_the_room() {
        let w = 2000.0;
        let left = offset_at(-w / 2.0, w);
        let mid = offset_at(0.0, w);
        let right = offset_at(w / 2.0, w);

        assert!(
            (left - 100.0).abs() < 1e-3,
            "at the LEFT edge the backdrop should sit at one extreme of its \
             travel, got {left}"
        );
        assert!(
            (right + 100.0).abs() < 1e-3,
            "at the RIGHT edge it should sit at the other, got {right}"
        );
        assert!(
            mid.abs() < 1e-3,
            "at the room's centre the backdrop should be centred, got {mid}"
        );
    }

    /// A camera in the room's left half must move the backdrop.
    #[test]
    fn a_camera_in_the_left_half_still_moves_the_backdrop() {
        let w = 2000.0;
        let quarter = offset_at(-w / 4.0, w);
        let edge = offset_at(-w / 2.0, w);

        assert!(
            (quarter - edge).abs() > 1.0,
            "a quarter into the room reads the same as the far edge ({quarter} vs \
             {edge}) — the left half is pinned again"
        );
        assert!(
            quarter < edge,
            "travel should decrease monotonically from the left edge inward"
        );
    }
}

#[cfg(test)]
mod two_views_one_backdrop_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
    use ambition_sim_view::camera_snapshot::CameraViewport;
    use ambition_sim_view::{LocalView, LocalViewId, PresentedForView, PresentsView};
    use bevy::ecs::system::RunSystemOnce as _;

    /// A 2000-wide room, so the fraction is easy to check by hand.
    const WORLD_SIZE: Vec2 = Vec2::new(2000.0, 480.0);

    /// Bevy-space camera x. Bevy space is centred, so a 2000-wide room runs
    /// −1000 ..= +1000. `tx = −500/2000 + 0.5 = 0.25`, so `centered.x = −0.5` and
    /// the panel moves right by half its travel budget. The value is inside the
    /// clamp at both ends, so no expectation tests the clamp.
    const CAMERA_X: f32 = -500.0;

    /// The far camera: `tx = 500/2000 + 0.5 = 0.75`, `centered.x = +0.5`, so its
    /// panel is pulled LEFT by half its budget — the mirror image of `CAMERA_X`.
    const FAR_CAMERA_X: f32 = 500.0;

    fn viewport(w: f32, h: f32) -> CameraViewport {
        CameraViewport {
            px: Vec2::new(w, h),
            origin_px: Vec2::ZERO,
        }
    }

    fn spawn_view(world: &mut World, id: u8, viewport: CameraViewport) -> Entity {
        world.spawn((LocalView, LocalViewId(id), viewport)).id()
    }

    fn spawn_camera(world: &mut World, view: Entity, x: f32) {
        world.spawn((
            MainCamera,
            PresentsView(view),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }

    /// `panel_scale = 2.0` and `factor = 1.0` make the numbers exact: the panel is
    /// twice the longer viewport side, and the whole travel budget is used.
    fn spawn_panel(world: &mut World, view: Option<Entity>) -> Entity {
        let mut panel = world.spawn((
            Sprite::default(),
            Transform::from_xyz(0.0, 0.0, -18.0),
            Visibility::Inherited,
            ParallaxLayerVisual {
                factor: Vec2::splat(1.0),
                z: -18.0,
                panel_scale: 2.0,
                travel: Vec2::ZERO,
                world_size: WORLD_SIZE,
            },
        ));
        if let Some(view) = view {
            panel.insert(PresentedForView(view));
        }
        panel.id()
    }

    fn panel_x(world: &World, panel: Entity) -> f32 {
        world
            .entity(panel)
            .get::<Transform>()
            .expect("a panel keeps its transform")
            .translation
            .x
    }

    /// A panel is sized and offset by its own view's viewport, not a window global.
    ///
    /// The views have different viewports and the same camera position, so only
    /// the viewport can make the results differ. Neither is 1600x900, so a build
    /// that reads the window constants fails both.
    ///
    /// - view A: 800x400 → panel `2*800 = 1600`, `travel.x = (1600-800)/2 = 400`
    /// - view B: 400x400 → panel `2*400 = 800`,  `travel.x = (800-400)/2 = 200`
    ///
    /// At `CAMERA_X` the offset is `+travel.x/2`, so A draws at `-500+200 = -300`
    /// and B at `500+100 = 600`.
    #[test]
    fn each_view_sizes_its_panel_from_its_own_viewport() {
        let mut world = World::new();
        let wide = spawn_view(&mut world, 0, viewport(800.0, 400.0));
        let narrow = spawn_view(&mut world, 1, viewport(400.0, 400.0));
        // Same camera position for both, so any difference comes from the viewport.
        spawn_camera(&mut world, wide, CAMERA_X);
        spawn_camera(&mut world, narrow, CAMERA_X);
        let wide_panel = spawn_panel(&mut world, Some(wide));
        let narrow_panel = spawn_panel(&mut world, Some(narrow));

        world
            .run_system_once(sync_parallax_layers)
            .expect("the sync reads only components the fixture spawns");

        let size = |panel: Entity| world.entity(panel).get::<Sprite>().unwrap().custom_size;
        assert_eq!(
            size(wide_panel),
            Some(Vec2::splat(1600.0)),
            "the wide view's panel must be twice ITS OWN longer side (800), not \
             twice a window constant"
        );
        assert_eq!(
            size(narrow_panel),
            Some(Vec2::splat(800.0)),
            "and the narrow view's twice its own (400) — one window-derived size \
             would have given both the same panel"
        );

        let travel = |panel: Entity| {
            world
                .entity(panel)
                .get::<ParallaxLayerVisual>()
                .unwrap()
                .travel
        };
        assert_eq!(travel(wide_panel).x, 400.0);
        assert_eq!(travel(narrow_panel).x, 200.0);

        assert_eq!(
            panel_x(&world, wide_panel),
            -300.0,
            "the wide view's backdrop is offset by ITS travel budget"
        );
        assert_eq!(
            panel_x(&world, narrow_panel),
            -400.0,
            "and the narrow view's by its own"
        );
    }

    /// Each panel follows the camera that draws it.
    ///
    /// Both views have the same viewport, so only the presenting camera separates
    /// the results. The near camera pushes its panel right (`500 + 400/2 = 700`);
    /// the far one pulls it left (`1500 - 400/2 = 1300`).
    ///
    /// The second run swaps only which view each camera presents, and the two
    /// backdrops must swap too. A "first camera the archetype yields" build passes
    /// the first run and fails the second.
    #[test]
    fn each_panel_follows_the_camera_of_the_view_it_belongs_to() {
        for first_presents_lower in [true, false] {
            let mut world = World::new();
            let lower = spawn_view(&mut world, 0, viewport(800.0, 400.0));
            let upper = spawn_view(&mut world, 1, viewport(800.0, 400.0));
            let presented = if first_presents_lower {
                [lower, upper]
            } else {
                [upper, lower]
            };
            spawn_camera(&mut world, presented[0], CAMERA_X);
            spawn_camera(&mut world, presented[1], FAR_CAMERA_X);
            let panels = presented.map(|view| spawn_panel(&mut world, Some(view)));

            world
                .run_system_once(sync_parallax_layers)
                .expect("the sync reads only components the fixture spawns");

            assert_eq!(
                panel_x(&world, panels[0]),
                -300.0,
                "the panel of the view the NEAR camera presents must be placed \
                 against that camera"
            );
            assert_eq!(
                panel_x(&world, panels[1]),
                300.0,
                "and the FAR camera's view's panel against the far camera — one \
                 shared panel synced to `the` main camera cannot hold both numbers"
            );
        }
    }

    /// An unresolvable panel draws nothing. It does not draw at the world origin.
    ///
    /// A panel is unresolvable in two ordinary cases (under an adaptive split layout):
    ///
    /// - it names no view while several exist, so `ViewsOnHand` refuses to guess;
    /// - it names a view that no camera presents this frame.
    #[test]
    fn a_panel_with_no_resolvable_camera_is_hidden_rather_than_drawn_at_the_origin() {
        let mut world = World::new();
        let lower = spawn_view(&mut world, 0, viewport(800.0, 400.0));
        let upper = spawn_view(&mut world, 1, viewport(800.0, 400.0));
        spawn_camera(&mut world, lower, CAMERA_X);

        let unkeyed = spawn_panel(&mut world, None);
        let orphan = spawn_panel(&mut world, Some(upper));
        // Non-vacuity: a resolvable panel is placed in the same run, so "hide
        // everything" fails.
        let drawn = spawn_panel(&mut world, Some(lower));

        world
            .run_system_once(sync_parallax_layers)
            .expect("the sync reads only components the fixture spawns");

        for (panel, why) in [
            (
                unkeyed,
                "a panel naming no view while two exist must decline: picking one \
                 is the arbitrary process-global this seam exists to delete",
            ),
            (
                orphan,
                "a panel whose view no camera presents has nobody to follow, and a \
                 full-screen sky left at the world origin looks like a level bug",
            ),
        ] {
            assert_eq!(
                *world.entity(panel).get::<Visibility>().unwrap(),
                Visibility::Hidden,
                "{why}"
            );
            assert_eq!(
                world.entity(panel).get::<Sprite>().unwrap().custom_size,
                None,
                "an undrawn panel must not be sized against somebody else's \
                 viewport either"
            );
            assert_eq!(
                panel_x(&world, panel),
                0.0,
                "and it is left where it was rather than dragged to another \
                 view's camera"
            );
        }

        assert_eq!(
            *world.entity(drawn).get::<Visibility>().unwrap(),
            Visibility::Inherited,
            "the resolvable panel must still be drawn, or the assertions above \
             are satisfied by a system that hides everything"
        );
        assert_eq!(
            panel_x(&world, drawn),
            -300.0,
            "and it must have been placed against its own camera"
        );
    }

    /// A second view gets its own panel set; the first view keeps the entity the
    /// room spawned.
    ///
    /// One view must stay one entity per layer, so a one-view game does not
    /// allocate two sprites per layer.
    #[test]
    fn the_mirror_claims_the_root_and_copies_it_once_per_extra_view() {
        let mut world = World::new();
        let lower = spawn_view(&mut world, 0, viewport(800.0, 400.0));
        let root = spawn_panel(&mut world, None);

        world
            .run_system_once(mirror_parallax_layers_per_view)
            .expect("the mirror reads only components the fixture spawns");

        assert_eq!(
            world
                .entity(root)
                .get::<PresentedForView>()
                .map(|key| key.0),
            Some(lower),
            "one view claims the room's own panel rather than being handed a copy"
        );
        let mut panels = world.query_filtered::<Entity, With<ParallaxLayerVisual>>();
        assert_eq!(
            panels.iter(&world).count(),
            1,
            "a single-view composition must draw the panel it already had and \
             allocate nothing else"
        );

        // A second view appears.
        let upper = spawn_view(&mut world, 1, viewport(800.0, 400.0));
        world
            .run_system_once(mirror_parallax_layers_per_view)
            .expect("the mirror reads only components the fixture spawns");

        let mut keyed = world.query::<(&ParallaxLayerVisual, &PresentedForView)>();
        let mut owners: Vec<Entity> = keyed.iter(&world).map(|(_, key)| key.0).collect();
        owners.sort();
        let mut expected = vec![lower, upper];
        expected.sort();
        assert_eq!(
            owners, expected,
            "two views must own one panel each — no view without a backdrop, and \
             no backdrop belonging to nobody"
        );

        // And the second view goes away again.
        world.entity_mut(upper).despawn();
        world
            .run_system_once(mirror_parallax_layers_per_view)
            .expect("the mirror reads only components the fixture spawns");
        let mut panels = world.query_filtered::<Entity, With<ParallaxLayerVisual>>();
        let survivors: Vec<Entity> = panels.iter(&world).collect();
        assert_eq!(
            survivors,
            vec![root],
            "a retired view's copy is DESPAWNED, not left keyed to a view that is \
             gone — an orphan copy still draws while falling out of every query \
             that selects by view"
        );
    }
}
