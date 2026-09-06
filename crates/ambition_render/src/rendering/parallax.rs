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
    /// Panel extent as a multiple of the LONGER side of the drawing view's
    /// viewport. We avoid tile repetition by keeping each layer as a single
    /// large panel and shifting it within the budget the overhang buys.
    pub panel_scale: f32,
    /// Screen-space room-relative travel budget, derived — see the type doc.
    /// Zero until the first sync, which is also what a panel nobody can draw
    /// keeps.
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
/// it is declared as this panel's RESTING mask, not only set. The per-view
/// isolation pass is the single writer of `RenderLayers` on anything keyed by
/// `PresentedForView`, and while isolating it replaces the mask outright — so the
/// layer a panel returns to when a session collapses back to one view cannot be
/// derived from what is left on the entity. Stating it here is what keeps a
/// collapsed session's backdrop off layer 0, where the portal capture cameras
/// would draw it from the wrong eye.
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
        // no size here, and that is the point. The panel's extent is a
        // function of the viewport it is drawn into, and this call site has no
        // view in scope — the room spawns visuals, it does not know who is
        // watching. `sync_parallax_layers` sizes it against the owning view's
        // own rectangle on the first frame it can resolve one.
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

/// The two session-world reads are OPTIONAL, and that is a fact about who
/// runs this now. While it was registered by `game/ambition_app` alone, its
/// `Single` params were always satisfied — that host's session root carries both.
/// Installing it engine-side (S12) ran it in compositions whose root carries
/// neither, and a `Single` that matches nothing is a system-param VALIDATION
/// PANIC, not a skip: eight tests across `ambition_platformer2d_host` and both consumer
/// fixtures died on *"Resource does not exist"* with the system name compiled
/// out. A world with no room geometry has no parallax to refresh, which is an
/// ordinary state and not an error.
///
/// the despawn sweep takes ROOTS AND COPIES alike — every entity carrying
/// [`ParallaxLayerVisual`] that is not a portal capture copy — because the whole
/// backdrop is being rebuilt and a copy of a despawned root belongs to nobody.
/// [`mirror_parallax_layers_per_view`] rebuilds the per-view set from the fresh
/// roots.
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
/// What this session's theme loads have already been tried, and which of them
/// produced NO ART AT ALL.
///
/// ⛔⛔ IT IS A RESOURCE BECAUSE PRESENTATION HAS TO READ IT. This was a
/// `Local<Vec<ParallaxTheme>>` inside the loader, so "the loader has stopped
/// trying" was a fact only the loader could see. `sync_session_room_visuals`
/// therefore had no way to tell "the theme has not arrived YET" from "the theme
/// resolved to nothing and never will", and its no-art branch — which its own
/// comment describes as settled — was unreachable, so it re-asked a dead
/// question every frame for the life of the session.
///
/// ⚠ REACHABLE IN SHIPPED PROFILES, not hypothetical. `WebStatic` /
/// `BundledStatic` attempt an optional image only when it has an authored
/// embedded candidate, and the generated parallax manifest authors logical
/// entries without one — so on those profiles the load yields zero handles and
/// there is nothing more to wait for.
#[derive(bevy::prelude::Resource, Default, Debug)]
pub struct ParallaxThemeAttempts {
    // ⚠ `pub(crate)` for the CONSUMER'S TESTS, not for its systems.
    // `platformer_presentation` has to be able to stage "the loader tried and
    // found nothing" without standing up an asset server and a catalog to
    // produce it for real. Reading stays behind `attempted_without_art`.
    pub(crate) attempted: Vec<ParallaxTheme>,
    /// Attempted, and the asset profile produced no layer at all.
    pub(crate) without_art: Vec<ParallaxTheme>,
}

impl ParallaxThemeAttempts {
    /// Has this theme been tried and come back with nothing?
    ///
    /// ⚠ NOT "is it missing". A theme nobody has attempted yet is also missing,
    /// and that one is worth waiting for.
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
    // A rebuilt `GameAssets` (a fresh bind) starts with no themes at all, so the
    // memo has to start again with it — otherwise a theme this system already
    // "attempted" would never be loaded into the new set.
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
    // ⭐ THE OUTCOME, NOT THE ATTEMPT. Asked AFTER the load, because "we tried"
    // and "nothing came" are different facts and only the second one lets
    // presentation stop waiting. A profile that refuses every candidate leaves
    // zero handles here, and no later frame will add one — the memo above has
    // already closed this theme.
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
        // No observation seam in this composition, so nothing presents and there
        // is nothing to mirror. `ambition_sim_view::ViewsOnHand` calls the
        // no-views case quiet for exactly this reason.
        return;
    };

    // Retract BEFORE spawning, so a view that went away takes its whole set with
    // it rather than being counted as still-mirrored below.
    let live: std::collections::HashSet<Entity> = ordered.iter().map(|(_, view)| *view).collect();
    let mut mirrored: std::collections::HashSet<(Entity, Entity)> =
        std::collections::HashSet::new();
    for (entity, copy, key) in &copies {
        let root_is_gone = roots.get(copy.root).is_err();
        // `key.0 == root_view` is the re-key case: the view this copy served has
        // become the root's own view, so the root already draws it and the copy
        // is a duplicate.
        if root_is_gone || !live.contains(&key.0) || key.0 == root_view {
            // A room/content teardown can retire this presentation copy in the
            // same frame after this query observed it. Retraction is already
            // the desired outcome, so a vanished target is success rather than
            // an error worth escalating through Bevy's deferred-command
            // handler.
            commands.entity(entity).try_despawn();
            continue;
        }
        mirrored.insert((copy.root, key.0));
    }

    for (root, sprite, layer, bound, key) in &roots {
        if key.map(|key| key.0) != Some(root_view) {
            // Room replacement (including the developer LDtk hot-reload
            // transaction) can queue destruction of this root after the query
            // has yielded it but before Commands flush. This tag is pure
            // presentation ownership: if the root no longer exists there is
            // nothing left to present, so decline the stale write instead of
            // panicking the host.
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
                    // Hidden until it is placed. A panel is the size of a screen,
                    // so one frame of it sitting at the world origin is not a
                    // subtle artefact — and it is exactly the picture this whole
                    // change exists to stop drawing.
                    Visibility::Hidden,
                    copied_layer,
                    MirroredParallaxLayer { root },
                    ambition_sim_view::PresentedForView(*view),
                    parallax_resting_layers(),
                    ProjectionRestingLayers(parallax_resting_layers()),
                    RoomVisual,
                    // no `Name`. `entity.name` is registered for rollback
                    // and the coverage contract sweeps any entity carrying a type
                    // the rollback knows about, so labelling these would enlist a
                    // whole view's presentation set in the sim sweep.
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
/// That is the silent-wrong fallback `awaiting-maintainer-decision.md` §11 names, one family over
/// from the two it names explicitly: not a focus invented at `Vec2::ZERO`, but a POSITION left at
/// it.
///
/// each camera resolves its own view through `PresentsView`, by the same
/// `ambition_sim_view::ViewsOnHand` rule the follow camera, the physical viewport
/// applier and the draw-side lookup share — and each panel resolves its view
/// through `PresentedForView`, the other end of that seam.
///
/// a panel whose view or camera cannot be resolved DECLINES TO DRAW. It is
/// hidden and its transform is left exactly where it was, rather than being
/// synced against somebody else's camera or abandoned at the origin: a backdrop
/// that is absent is an obvious defect, and a backdrop plastered over the world
/// origin looks like a level-authoring mistake in a far corner of the map.
///
/// and the viewport is the view's, not `WINDOW_W`/`WINDOW_H`. Panel extent
/// and travel budget are re-derived here from
/// [`ambition_sim_view::camera_snapshot::CameraViewport`] every frame, so a
/// letterboxed gameplay rectangle and a split-screen half are described by the
/// same arithmetic the full window was.
#[allow(clippy::type_complexity)]
pub fn sync_parallax_layers(
    // the viewport is OPTIONAL here, and that is load-bearing. Requiring
    // `&CameraViewport` would make a view that lacks one invisible to this query
    // — and `ViewsOnHand::survey` would then count ONE view where the session has
    // two, so an unkeyed panel would be handed the complete view instead of being
    // refused. A survey that cannot see every view cannot refuse for the right
    // reason. (`CameraObservationPlugin` spawns the component with the view, and
    // `the_plugin_spawns_one_complete_view_at_build_time` pins that; this is the
    // belt for the frame where somebody composes a view by hand.)
    views: Query<
        (
            Entity,
            Option<&ambition_sim_view::camera_snapshot::CameraViewport>,
        ),
        With<ambition_sim_view::LocalView>,
    >,
    // `With<MainCamera>`: ignore the #31 cube overlay Camera3d AND the portal
    // view-cone capture `Camera2d`s — a capture rig is a lens inside the
    // simulation, not an observer of it, and it gets its own parallax copies
    // through `sync_portal_capture_parallax_layers`.
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
    // two cameras naming ONE view is not ambiguous: `camera_follow` hands both
    // of them the same view's framing, so `or_insert` records the same position
    // whichever the archetype yields first. Two cameras naming two views is the
    // split case and produces two rows.
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
            // Nobody draws this panel — no view claims it, or the view that does
            // has no camera. Declining is the honest answer; see the system doc.
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        if *visibility == Visibility::Hidden {
            *visibility = Visibility::Inherited;
        }

        // Compare before writing: `Sprite` and `ParallaxLayerVisual` are only
        // touched when the view's rectangle actually changed, so a settled panel
        // reports no change tick per frame.
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
    // the ROOT set only (`Without<MirroredParallaxLayer>`). Every live view
    // now owns a panel per layer, and a rig that copied all of them would stack N
    // identical skies in one capture. Portal camera continuity is still one
    // process-global host view (`PortalCameraContinuityState`/`HostView`), so the
    // root set is the honest source for it — and giving each rig the view its
    // portal is actually seen through is that seam's own job, not this one.
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
    // Dividing one by the other therefore gave −0.5 ..= +0.5, and the clamp turned the entire
    // negative half into a flat 0 — so across the whole LEFT half of every room the backdrop
    // sat pinned at maximum travel and did not move, and only the right half parallaxed, over
    // half the intended range.
    //
    // a unit error the clamp HID. Nothing ever read out of range or crashed; the wrong half
    // simply stopped animating, which reads as "this background is far away" rather than as a
    // defect.
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

    /// Trusts packaging rather than the filesystem — `AndroidBundle` is the
    /// profile whose `should_attempt_resolved_load` is unconditionally true, so
    /// this test asks "does the engine REQUEST the theme's art" without also
    /// asking whether a generated PNG happens to exist next to the test binary.
    fn packaged_catalog() -> Platformer2dAssetCatalog {
        let manifest = ambition_sprite_sheet::game_assets::sandbox_image_manifest("sprites");
        Platformer2dAssetCatalog::new(
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
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "second_biome",
            vec![room],
            Vec::new(),
        )
    }

    /// The theme the ACTIVE room asks for is loaded by whoever presents it.
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
        // A test that adds the system by hand cannot go red on the thing that was wrong.
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

    /// The layers MOVE with the camera, in every composition.
    ///
    /// `sync_parallax_layers` was app-local too — the same class one step
    /// further along. A composition that got its backdrop spawned still left it
    /// at the world origin forever, so it slid out of frame as the camera walked
    /// away and the one thing a parallax layer is for never happened. Nothing
    /// about that reads as a missing system: the art is correct, in the wrong
    /// place, and only when you walk.
    ///
    /// the fixture now spawns a LOCAL VIEW as well as a camera, because
    /// the sync resolves a camera through the view it presents. That is the same
    /// requirement `layout_world_labels` and `sync_actor_nameplates` already
    /// impose — a per-view draw system needs an observation seam to draw for —
    /// and every composed host has one: `CameraObservationPlugin` spawns the view
    /// at plugin BUILD time.
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

/// TWO VIEWS, TWO BACKDROPS — each from its own camera and its own viewport.
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
        // The panel is placed AT the camera plus a parallax offset, so the
        // offset alone is what this pass decides.
        t.translation.x - camera_x
    }

    /// The backdrop must travel across the WHOLE room, not the right half.
    ///
    /// `camera_xy` is the camera's centred Bevy transform (`camera.rs` builds it as
    /// `center_world.x - size.x * 0.5`), so it runs −size/2 ..= +size/2 while `world_size` is
    /// the full span.
    ///
    /// the assertion is on the SPAN and the MIDPOINT together. Either alone
    /// passes for a broken mapping: a half-range still has two distinct ends, and
    /// a centred midpoint says nothing about how far it reaches.
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

    /// The poison, stated as its own claim: a camera in the room's left half must MOVE the
    /// backdrop.
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

    /// A 2000-wide room, so the horizontal fraction the offset is derived from is
    /// arithmetic anyone can check by hand rather than a number copied from a run.
    const WORLD_SIZE: Vec2 = Vec2::new(2000.0, 480.0);

    /// Bevy-space camera x, and Bevy space is CENTRED: `camera.rs` builds the
    /// transform as `center_world.x - size.x * 0.5`, so a 2000-wide room runs
    /// −1000 ..= +1000. `tx = −500/2000 + 0.5 = 0.25`, so `centered.x = −0.5` and
    /// the panel is pushed RIGHT by half its travel budget — a fraction strictly
    /// inside the clamp at both ends, so no expectation below is secretly an
    /// assertion about the clamp.
    ///
    /// Fixing the mapping is what exposed it — the constants and the code agreed with each
    /// other and with nothing else.
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

    /// `panel_scale = 2.0` and `factor = 1.0` make every number below exact: the
    /// panel is twice the LONGER side of its viewport, and the whole travel budget
    /// is spent (nothing is scaled down by a fractional factor on the way out).
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

    /// A PANEL IS SIZED AND OFFSET BY ITS OWN VIEW'S VIEWPORT, NOT BY A
    /// WINDOW GLOBAL.
    ///
    /// the two views are given DIFFERENT viewports and the SAME camera
    /// position, so the only thing that can produce two different answers is the
    /// viewport itself. Neither viewport is 1600x900, so a build that still read
    /// the window constants would agree with neither.
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
        // The SAME camera position for both: any difference in the result can
        // only have come from the viewport.
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

    /// EACH PANEL FOLLOWS THE CAMERA THAT DRAWS IT.
    ///
    /// Both views are given the SAME viewport here, so the only thing that can
    /// separate the two answers is which camera each panel's view is presented by.
    /// The near camera pushes its panel right (`500 + 400/2 = 700`); the far one
    /// pulls its panel left (`1500 - 400/2 = 1300`).
    ///
    /// the falsifier is inside the test. The second run swaps only which
    /// view each camera presents — same spawn order, same entities, same
    /// viewports, same panels — and the two backdrops must swap with them. An
    /// implementation that takes the first camera the archetype yields (which is
    /// what `.single()` degraded into the moment a second camera existed, when it
    /// did not simply refuse and leave every panel at the origin) passes the first
    /// run and fails this one.
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

    /// AN UNRESOLVABLE PANEL DRAWS NOTHING — IT DOES NOT DRAW AT THE WORLD
    /// ORIGIN.
    ///
    /// Two things make a panel unresolvable, and under an adaptive split layout
    /// both are ordinary rather than exotic:
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
        // Non-vacuity: a panel that CAN be resolved is placed in the same run, so
        // "the system hid everything" cannot pass this test.
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

    /// A SECOND VIEW GETS ITS OWN PANEL SET, AND THE FIRST ONE'S IS THE
    /// ENTITY THE ROOM ALREADY SPAWNED.
    ///
    /// The single-view case must stay exactly one entity per layer: a mirror that
    /// demoted the room's panel to an un-drawn template would make every shipped
    /// one-view game allocate two sprites per layer to draw one.
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
