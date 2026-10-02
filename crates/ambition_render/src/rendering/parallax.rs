//! Room-scoped generated parallax with one panel set per local view.
//!
//! Each live room has its own panel set, stamped with the room (view half V4c),
//! in its own room's theme. A panel's transform and travel depend on its
//! observer's viewport, so each view that frames the room draws its own copy,
//! keyed by [`ambition_sim_view::PresentedForView`]. Each set is re-derived
//! from its owning view's camera viewport every frame.

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

/// Marks that one live room's parallax is settled: its layers are spawned, or
/// the tier wants none, or its theme resolved to no art. It is stamped with
/// its room and is a [`RoomVisual`], so it retires with the room; a refresh
/// takes it, so the room is presented again.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PresentedRoomParallax;

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

/// Take every panel and every room's parallax marker when the art or the
/// quality changes, so [`present_live_room_parallax`] presents each live room
/// again with the new art and budget.
///
/// The sweep takes a live room's roots and copies alike (every stamped
/// [`ParallaxLayerVisual`] that is not a portal capture copy): a copy of a
/// despawned root has no owner. A panel no room owns (one a host placed
/// itself) is not the reconciler's, so it is not taken.
pub fn refresh_parallax_layers_on_quality_change(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    layers: Query<
        Entity,
        (
            With<ParallaxLayerVisual>,
            With<ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
            Without<PortalCaptureParallaxLayerVisual>,
        ),
    >,
    presented: Query<Entity, With<PresentedRoomParallax>>,
) {
    let Some(assets) = assets else {
        return;
    };
    let assets_changed = assets.is_changed();
    let quality_changed = quality.as_ref().is_some_and(|q| q.is_changed());
    if !assets_changed && !quality_changed {
        return;
    }
    for entity in layers.iter().chain(presented.iter()) {
        commands.entity(entity).try_despawn();
    }
}

/// Give each live room its parallax, stamped with that room (view half V4c).
///
/// One panel set showed the sole live room, so while two rooms were live it
/// showed one room's backdrop in both rooms' views, or none. Now each live
/// room has its own set in its own theme, and the mirror draws it only in the
/// views that frame that room.
///
/// The room and its backdrop become available at different times: the theme
/// may load later than the room ([`ensure_active_room_parallax_theme`]), and
/// the room's static visuals do not wait for it. So a room whose theme is not
/// here yet is asked again on the next frame. A room is settled, and gets a
/// [`PresentedRoomParallax`] marker, when its layers are spawned, when the
/// tier wants no parallax, or when its theme was tried and has no art.
#[allow(clippy::too_many_arguments)]
pub fn present_live_room_parallax(
    mut commands: Commands,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    presented: Query<
        &ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance,
        With<PresentedRoomParallax>,
    >,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    // What the theme loader tried and found empty: "not yet" is not "never".
    attempts: Option<Res<ParallaxThemeAttempts>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let wants_parallax = quality
        .as_deref()
        .map(|q| q.budget.parallax.enabled)
        .unwrap_or(true);
    for (room, definition) in rooms.live_rooms() {
        if presented.iter().any(|stamp| stamp.0 == room) {
            continue;
        }
        let spec = rooms.rooms().spec(definition);
        if wants_parallax {
            let theme = ParallaxTheme::from_room_metadata(&spec.metadata);
            let loaded = assets.as_deref().is_some_and(|assets| {
                ParallaxLayerAsset::ALL
                    .iter()
                    .any(|layer| assets.parallax_layers.get(theme, *layer).is_some())
            });
            let nothing_is_coming = attempts
                .as_deref()
                .is_some_and(|attempts| attempts.attempted_without_art(theme));
            if !loaded && !nothing_is_coming {
                continue;
            }
        }
        let scope = session_scope.in_room(Some(room));
        spawn_parallax_layers(
            &mut commands,
            scope,
            &spec.world,
            &spec.metadata,
            assets.as_deref(),
            quality.as_deref().map(|q| &q.budget.parallax),
        );
        commands.spawn_session_scoped(
            scope,
            (PresentedRoomParallax, RoomVisual, Name::new("presented room parallax")),
        );
    }
}

/// The themes whose load produced no art.
///
/// Only this is remembered. Whether a theme is loaded is read from
/// [`GameAssets`] each time, because the residency policy evicts themes: a
/// memo of "already tried" outlived an eviction and kept a theme unloaded.
///
/// This is a resource so that presentation can read it:
/// `sync_session_room_visuals` must tell "not arrived yet" from "resolved to
/// nothing". This occurs in shipped profiles: `WebStatic` / `BundledStatic`
/// load an optional image only with an embedded candidate, and the generated
/// parallax manifest has none, so the load yields zero handles.
#[derive(bevy::prelude::Resource, Default, Debug)]
pub struct ParallaxThemeAttempts {
    /// Attempted, and the asset profile produced no layer at all. `pub(crate)`
    /// so `platformer_presentation` tests can stage "tried and found nothing"
    /// without an asset server. Read it through `attempted_without_art`.
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

/// Load each live room's parallax theme when it is not resident.
///
/// Loading mutates [`GameAssets`], which makes
/// [`refresh_parallax_layers_on_quality_change`] take the layers and
/// [`present_live_room_parallax`] present them again. A theme that produced no
/// art is not retried, so it does not invalidate the layers every frame.
pub fn ensure_active_room_parallax_theme(
    assets: Option<ResMut<GameAssets>>,
    catalog: Option<Res<ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog>>,
    asset_server: Option<Res<AssetServer>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    rooms: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    attempts: Option<ResMut<ParallaxThemeAttempts>>,
) {
    let (Some(mut assets), Some(catalog), Some(asset_server), Some(rooms), Some(mut attempts)) =
        (assets, catalog, asset_server, rooms, attempts)
    else {
        return;
    };
    // A rebuilt `GameAssets` may use another profile, so ask again.
    if assets.is_added() {
        attempts.without_art.clear();
    }
    for (_, definition) in rooms.live_rooms() {
        let metadata = rooms.rooms().spec(definition).metadata.clone();
        let theme = ParallaxTheme::from_room_metadata(&metadata);
        if attempts.without_art.contains(&theme) {
            continue;
        }
        // Resident. Do not touch `GameAssets` mutably: a mutable deref marks it
        // changed, and the refresh system would respawn every layer.
        if ParallaxLayerAsset::ALL
            .iter()
            .any(|layer| assets.parallax_layers.get(theme, *layer).is_some())
        {
            continue;
        }
        ambition_sprite_sheet::game_assets::ensure_parallax_layers_for_room(
            &mut assets,
            &catalog,
            &asset_server,
            &metadata,
            quality.as_deref().map(|q| &q.budget),
        );
        // Record the outcome, not the attempt. Only "nothing came" lets
        // presentation stop waiting. A profile that refuses every candidate
        // leaves zero handles, and no later frame adds one.
        if !ParallaxLayerAsset::ALL
            .iter()
            .any(|layer| assets.parallax_layers.get(theme, *layer).is_some())
        {
            attempts.without_art.push(theme);
        }
    }
}

/// Maintain one parallax panel set per local view, for the room it frames.
///
/// A room's panels are drawn by the views that frame that room
/// (`ResolvedCameraSnapshot::frame`), in `LocalViewId` order: the lowest claims
/// each root and the others receive copies. A room no view frames keeps its
/// roots claimed by the lowest view, and [`sync_parallax_layers`] hides them.
/// An unstamped root (a host that spawns its own) is drawn by every view.
/// A copy whose view no longer frames its root's room, or whose root is gone,
/// is despawned. Never clear `PresentedForView` on a still-rendered sprite,
/// because it would become an unscoped draw.
#[allow(clippy::type_complexity)]
pub fn mirror_parallax_layers_per_view(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    views: Query<
        (
            Entity,
            &ambition_sim_view::LocalViewId,
            Option<&ambition_sim_view::camera_snapshot::ResolvedCameraSnapshot>,
        ),
        With<ambition_sim_view::LocalView>,
    >,
    roots: Query<
        (
            Entity,
            &Sprite,
            &ParallaxLayerVisual,
            Option<&BoundParallaxLayer>,
            Option<&ambition_sim_view::PresentedForView>,
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
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

    let mut ordered: Vec<(
        ambition_sim_view::LocalViewId,
        Entity,
        Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    )> = views
        .iter()
        .map(|(view, id, resolved)| {
            (*id, view, resolved.and_then(|resolved| resolved.frame()).map(|frame| frame.room))
        })
        .collect();
    ordered.sort_by_key(|(id, ..)| *id);
    let Some((_, lowest, _)) = ordered.first().copied() else {
        // No observation seam, so nothing presents (see
        // `ambition_sim_view::ViewsOnHand`).
        return;
    };
    // The views that draw a root of room `stamp`, the claimer first.
    let drawers = |stamp: Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>| -> Vec<Entity> {
        let Some(stamp) = stamp else {
            return ordered.iter().map(|(_, view, _)| *view).collect();
        };
        let framing: Vec<Entity> = ordered
            .iter()
            .filter(|(_, _, frame)| *frame == Some(stamp.0))
            .map(|(_, view, _)| *view)
            .collect();
        if framing.is_empty() {
            vec![lowest]
        } else {
            framing
        }
    };

    // Retract before spawning, so a removed view takes its whole set with it.
    let mut mirrored: std::collections::HashSet<(Entity, Entity)> =
        std::collections::HashSet::new();
    for (entity, copy, key) in &copies {
        let keep = roots
            .get(copy.root)
            .ok()
            .is_some_and(|(.., stamp)| drawers(stamp).iter().skip(1).any(|view| *view == key.0));
        if !keep {
            // A teardown can retire this copy in the same frame. Retraction is the
            // wanted outcome, so a missing target is success.
            commands.entity(entity).try_despawn();
            continue;
        }
        mirrored.insert((copy.root, key.0));
    }

    for (root, sprite, layer, bound, key, stamp) in &roots {
        let drawers = drawers(stamp);
        let claimer = drawers[0];
        if key.map(|key| key.0) != Some(claimer) {
            // Room replacement (including LDtk hot-reload) can queue this root's
            // destruction before Commands flush. The tag is presentation-only, so skip
            // the stale write instead of panicking.
            commands
                .entity(root)
                .try_insert(ambition_sim_view::PresentedForView(claimer));
        }
        for view in drawers.iter().skip(1) {
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
            // The copy is its root's room's, and retires with it.
            if let Some(stamp) = stamp {
                copy.insert(*stamp);
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
            Option<&ambition_sim_view::camera_snapshot::ResolvedCameraSnapshot>,
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
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        ),
        (Without<Camera>, Without<PortalCaptureParallaxLayerVisual>),
    >,
) {
    let on_hand = ambition_sim_view::ViewsOnHand::survey(views.iter().map(|(view, ..)| view));

    // Where each view's camera stands, and how big that view's rectangle is.
    //
    // Two cameras on one view get the same framing from `camera_follow`, so
    // `or_insert` is order-independent. Two views give two rows.
    let mut drawn_by: std::collections::HashMap<
        Entity,
        (Vec2, Vec2, Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>),
    > = std::collections::HashMap::new();
    for (camera_transform, link) in &cameras {
        let Some(view) = on_hand.presented_by(link.copied()) else {
            continue;
        };
        let Ok((_, viewport, resolved)) = views.get(view) else {
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
            .or_insert((
                camera_transform.translation.truncate(),
                viewport.px,
                resolved.and_then(|resolved| resolved.frame()).map(|frame| frame.room),
            ));
    }

    for (mut transform, mut sprite, mut visibility, mut layer, key, stamp) in &mut layers {
        let resolved = on_hand
            .drawn_for(key.copied())
            .and_then(|view| drawn_by.get(&view).copied())
            // A room's panel is drawn only by a view that frames that room: a view
            // of another live room declines it (V4c).
            .filter(|(_, _, frame)| match (stamp, frame) {
                (Some(stamp), Some(frame)) => stamp.0 == *frame,
                _ => true,
            });
        let Some((camera_xy, viewport_px, _)) = resolved else {
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
        let rooms = room_set_in("cave");
        // The live room root names which room of the set it is (OW1 cut 5e).
        let definition = rooms.activation_definition();
        app.world_mut().spawn((SessionRoot(SessionScopeId(1)), rooms));
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(SessionScopeId(1)),
            definition,
        ));
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
        let rooms = room_set_in("cave");
        // The live room root names which room of the set it is (OW1 cut 5e).
        let definition = rooms.activation_definition();
        app.world_mut().spawn((SessionRoot(SessionScopeId(1)), rooms));
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(SessionScopeId(1)),
            definition,
        ));
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
        let rooms = room_set_in("cave");
        // The live room root names which room of the set it is (OW1 cut 5e).
        let definition = rooms.activation_definition();
        app.world_mut().spawn((SessionRoot(SessionScopeId(1)), rooms));
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(SessionScopeId(1)),
            definition,
        ));
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

    fn framing(world: &mut World, view: Entity, room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance) {
        world.entity_mut(view).insert(ambition_sim_view::camera_snapshot::ResolvedCameraSnapshot(Some(
            ambition_sim_view::camera_snapshot::ResolvedCameraFrame {
                snapshot: Default::default(),
                follow_world: Default::default(),
                room,
            },
        )));
    }

    /// The views each panel is drawn for, and whether it shows: `(room, view, shown)`.
    fn drawn(world: &mut World) -> Vec<(u32, Entity, bool)> {
        let mut panels = world.query::<(
            &ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance,
            &PresentedForView,
            &Visibility,
        )>();
        let mut rows: Vec<_> = panels
            .iter(world)
            .map(|(stamp, key, visibility)| (stamp.0.ordinal(), key.0, *visibility != Visibility::Hidden))
            .collect();
        rows.sort();
        rows
    }

    /// Two players in two live rooms: each view draws its own room's sky and
    /// not the other's (V4c).
    ///
    /// Each live room presents its own panel, stamped with its room. View A
    /// frames the first room and view B the second, so each panel has one
    /// drawer and nothing is copied. The control is the same two panels with
    /// both views in the first room: the first room's panel is drawn by both
    /// (one copy), and the second room's panel, which no view frames, is
    /// hidden.
    #[test]
    fn each_view_draws_only_the_sky_of_the_room_it_frames() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
        let first = LiveRoomInstance::ACTIVATION;
        let second = first.next();
        let mut world = World::new();
        let a = spawn_view(&mut world, 0, viewport(800.0, 400.0));
        let b = spawn_view(&mut world, 1, viewport(800.0, 400.0));
        spawn_camera(&mut world, a, CAMERA_X);
        spawn_camera(&mut world, b, FAR_CAMERA_X);
        for room in [first, second] {
            let panel = spawn_panel(&mut world, None);
            world.entity_mut(panel).insert(InRoomInstance(room));
        }
        let settle = |world: &mut World| {
            for _ in 0..2 {
                world
                    .run_system_once(mirror_parallax_layers_per_view)
                    .expect("the mirror reads only components the fixture spawns");
                world
                    .run_system_once(sync_parallax_layers)
                    .expect("the sync reads only components the fixture spawns");
            }
        };

        // Control: both views in the first room.
        framing(&mut world, a, first);
        framing(&mut world, b, first);
        settle(&mut world);
        let mut expected = vec![(0, a, true), (0, b, true), (1, a, false)];
        expected.sort();
        assert_eq!(
            drawn(&mut world),
            expected,
            "control: two views of one room both draw its sky, and the sky of a \
             room nobody frames is hidden"
        );

        // Bob goes into the second room.
        framing(&mut world, b, second);
        settle(&mut world);
        let mut expected = vec![(0, a, true), (1, b, true)];
        expected.sort();
        assert_eq!(
            drawn(&mut world),
            expected,
            "each view must draw the sky of the room it frames, and only that one"
        );
    }
}

#[cfg(test)]
mod theme_residency_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;

    fn hub_room_set() -> ambition_platformer2d_world::rooms::RoomSet {
        let mut room = ambition_platformer2d_world::rooms::RoomSpec::new(
            "hub_room",
            ambition_platformer2d_core::World::new(
                "hub_room",
                ambition_platformer2d_core::Vec2::new(640.0, 480.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        room.metadata.visual_profile.parallax_theme = Some("hub".to_string());
        ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
            "hub_room",
            vec![room],
            Vec::new(),
        )
    }

    fn hub_is_resident(app: &App) -> bool {
        app.world()
            .resource::<GameAssets>()
            .parallax_layers
            .resident_themes()
            .contains(&ParallaxTheme::Hub)
    }

    /// A theme the residency policy evicted loads again while its room is active.
    ///
    /// Reset Sandbox rebuilds the room without a door transition. A memo of
    /// themes "already attempted" outlived the eviction, so the hub's backdrop
    /// never came back and the black grid showed through.
    #[test]
    fn an_evicted_theme_loads_again_while_its_room_is_active() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::image::Image>();
        app.insert_resource(
            ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog::new(
                ambition_asset_manager::AmbitionAssetCatalog::new(
                    ambition_sprite_sheet::game_assets::sandbox_image_manifest("sprites"),
                ),
                ambition_asset_manager::AssetProfile::AndroidBundle,
            ),
        );
        app.init_resource::<GameAssets>();
        app.init_resource::<ParallaxThemeAttempts>();
        let mut active = ActiveSessionScope::default();
        let scope = active.begin();
        app.insert_resource(active);
        let rooms = hub_room_set();
        // The live room root names which room of the set it is (OW1 cut 5e).
        let definition = rooms.activation_definition();
        app.world_mut().spawn((SessionRoot(scope), rooms));
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(scope),
            definition,
        ));
        app.add_systems(bevy::prelude::Update, ensure_active_room_parallax_theme);

        app.update();
        assert!(hub_is_resident(&app), "the active room's theme loads");

        app.world_mut()
            .resource_mut::<GameAssets>()
            .parallax_layers
            .retain_themes(|_| false);
        assert!(!hub_is_resident(&app), "the eviction applied");
        app.update();
        assert!(
            hub_is_resident(&app),
            "an evicted theme must load again while its room is active"
        );
    }
}
