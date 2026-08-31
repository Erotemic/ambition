//! Debug drawing for the Bevy sandbox backend.
//!
//! These overlays intentionally live in the Bevy adapter layer. The movement
//! engine exposes simulation state; this module decides how to visualize that
//! state for tuning and feel work.

#![allow(unused_imports)]
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use bevy::ecs::system::SystemParam;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use ambition_platformer2d::dev_tools::dev_tools::DeveloperTools;
use ambition_platformer2d::dev_tools::DeveloperRuntimeState;
use ambition_platformer2d::engine_core::config::world_to_bevy;
use ambition_platformer2d::engine_core::RoomGeometry;
#[cfg(feature = "input")]
use ambition_platformer2d::input::Platformer2dInputActionMonolith;
use ambition_platformer2d::input::{read_gameplay_control_frame, ControlFrame};
use ambition_platformer2d::platformer::schedule::GameMode;
use ambition_platformer2d::render::rendering::CameraViewState;
use ambition_platformer2d::world::rooms::{LoadingZone, LoadingZoneActivation, RoomSet};
#[cfg(feature = "input")]
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

mod gizmos;
mod prims;
pub use gizmos::*;
pub use prims::*;

// The engine-generic palette, primitives, and world layers live in the shared
// debug-viz module now (`DebugVizPlugin` gives them to any game); this richer
// overlay composes them with its game-specific layers below.
pub(crate) use ambition_platformer2d::render::rendering::debug_viz::{
    blue, cyan, draw_aabb, draw_aabb_styled, draw_arrow, draw_combat_geometry_view,
    draw_combat_volume, draw_hitbox_volume, draw_micro_grid, draw_moving_platform_debug,
    draw_rebound_vectors, draw_room_bounds, draw_surface_chains, draw_world_blocks,
    draw_world_grid, engine_delta_to_bevy, gray, green, magenta, orange, presentation_deltas, red,
    w2, white_dim, with_alpha, yellow,
};

/// Draw the per-frame [`DebugOverlayLabels`] buffer as TEXT GIZMOS.
///
/// ⛔⛔ THIS USED TO DESPAWN AND RESPAWN AN ENTITY PER LABEL, EVERY FRAME. The
/// labels are immediate-mode facts — "this box is the hurtbox" — drawn beside
/// gizmo lines that are already immediate-mode, and the only reason they were
/// retained `Text2d` entities is that Bevy had no way to draw text without one.
/// 0.19 does. What goes with the entities: the spawn churn, the despawn sweep,
/// the `DebugOverlayLabel` marker, the lifetime bookkeeping, and — the one that
/// mattered — the dependency of F1 world labels on the PRODUCT font stack.
/// A developer overlay should not be able to fail because a typeface has not
/// finished loading.
///
/// ⭐ THE SIZE CONSTANT SURVIVES UNCHANGED, and that was checked rather than
/// assumed: `Gizmos::text_2d` takes an `Isometry2d` in WORLD space, so its
/// `font_size` is world units exactly as `Text2d`'s was. Labels keep scaling
/// with camera zoom, which is the behaviour the boss-fight zoom was tuned at.
///
/// ⭐ The stroke font is ASCII-only, which is correct here — these strings are
/// `hurtbox`, `contact`, `collision`. Player-facing text stays on the real
/// typography stack.
///
/// Still drains the buffer every frame, so toggling the overlay off (no pushes)
/// clears the labels on the next frame exactly as before.
pub(crate) fn render_debug_overlay_labels(
    mut gizmos: Gizmos,
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    mut labels: ResMut<DebugOverlayLabels>,
) {
    for label in labels.0.drain(..) {
        let at = world_to_bevy(&world.0, label.world_pos, DEBUG_LABEL_Z);
        gizmos.text_2d(
            at.truncate(),
            &label.text,
            DEBUG_LABEL_FONT_PX,
            // Centred: the anchor the box-corner offsets in `label_box` were
            // chosen against, so the fan-out for overlapping boxes is preserved.
            Vec2::ZERO,
            label.color,
        );
    }
}

/// No-op stub for builds without the `input` feature. The full overlay
/// reads leafwing's `ActionState` to render combat/blink previews; without
/// leafwing in scope, gizmos for those would have no input source. Sim
/// gizmos that don't need input are also skipped to keep the chain
/// signature stable across feature combinations.
#[cfg(not(feature = "input"))]
pub(crate) fn draw_debug_overlay() {}

#[cfg(feature = "input")]
pub(crate) fn draw_debug_overlay(
    mut gizmos: Gizmos,
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    dev_state: Res<DeveloperRuntimeState>,
    platform_set: Res<ambition_platformer2d::world::collision::MovingPlatformSet>,
    // The ONE collision read-API, for the blink preview — the same composition
    // `step_motion` collides against. See `draw_player_debug`'s `blink_world`.
    collision: ambition_platformer2d::world::collision::CollisionWorld,
    developer_tools: Res<DeveloperTools>,
    room_set: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomSet>,
    ldtk_spine_index: Res<ambition_platformer2d::ldtk_map::LdtkRuntimeSpineIndex>,
    // Was `Res<CameraViewState>`, a process-global describing "the" gameplay view — which is
    // the one thing a debug overlay must not assume once a split layout draws two.
    camera_view: ambition_platformer2d::sim_view::PresentedViewState,
    mode: Res<State<GameMode>>,
    mut overlay_labels: ResMut<DebugOverlayLabels>,
    action_query: Query<
        &ActionState<Platformer2dInputActionMonolith>,
        With<ambition_platformer2d::input::InputParticipant>,
    >,
    mut player_q: Query<
        (
            Entity,
            ae::BodyClusterQueryData,
            // The movement policy: the overlay is a dev tool and draws the
            // policy's private internals (ledge anchor, blink aim) directly.
            &ae::MotionModel,
            Option<&ambition_platformer2d::characters::actor::BodyHealth>,
            &ambition_platformer2d::combat::BodyMelee,
            &ambition_platformer2d::characters::actor::WornCharacter,
            // The frame-clock position the sprite is drawn at. The overlay must
            // sample the SAME clock as the camera it is drawn through, or the box
            // shakes against a world that looks perfectly stable — see
            // `draw_player_debug`'s `draw_pos`.
            Option<&ambition_platformer2d::sim_view::PresentedPose>,
        ),
        // The primary player never carries `FeatureSimEntity` (player vs
        // feature-sim entities are mutually exclusive — see the kinematics
        // unification). Spell that disjointness out with `Without` so Bevy can
        // prove this `&mut BodyKinematics` (BodyClusterQueryData) query does
        // not conflict with the `bosses`/`actors` feature queries that read
        // `BodyKinematics` under `With<FeatureSimEntity>` (B0001).
        (
            ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
            Without<ambition_platformer2d::actor::FeatureSimEntity>,
        ),
    >,
    feature_q: FeatureDebugQueries,
    #[cfg(feature = "portal")] portals: Query<&ambition_platformer2d::portal::PlacedPortal>,
) {
    if !dev_state.debug_enabled() || !developer_tools.gizmos_enabled {
        return;
    }
    // Start each frame's label buffer fresh; `render_debug_overlay_labels`
    // drains it after this system runs.
    overlay_labels.0.clear();

    let world = &world.0;
    // Mirror the gameplay input gate used by the player tick. Raw Leafwing
    // action state still records button presses while paused so pause/menu
    // UI can respond, but debug combat/blink previews are gameplay-facing and
    // should not light up from those paused-mode inputs.
    let gameplay_active = mode.get().allows_gameplay();
    let actions = if gameplay_active {
        action_query.single().ok()
    } else {
        None
    };
    // World- and combat-level observability is independent of whether this host
    // happens to contain a legacy `PrimaryPlayerOnly` body. Smash deliberately
    // does not; that must not make every other debug layer disappear.
    if developer_tools.show_room_bounds {
        draw_room_bounds(&mut gizmos, world);
    }
    if developer_tools.show_world_blocks {
        draw_world_blocks(&mut gizmos, world, &developer_tools);
        draw_surface_chains(&mut gizmos, world);
    }
    if developer_tools.show_micro_grid {
        draw_micro_grid(&mut gizmos, world, 8.0, 16.0);
    }
    if developer_tools.hide_sprites {
        draw_world_grid(&mut gizmos, world);
    }
    if developer_tools.show_camera_frame {
        if let Some(camera_view) = camera_view.get() {
            draw_camera_frame(&mut gizmos, world, camera_view);
        }
    }
    if developer_tools.show_loading_zones {
        draw_loading_zones(&mut gizmos, world, room_set.active_loading_zones());
        draw_ldtk_runtime_spine(&mut gizmos, world, &ldtk_spine_index);
    }
    if developer_tools.show_rebound_vectors {
        draw_rebound_vectors(&mut gizmos, world);
    }
    if developer_tools.show_moving_platform {
        draw_moving_platform_debug(&mut gizmos, world, &platform_set.0);
    }
    draw_combat_geometry_view(
        &mut gizmos,
        world,
        &feature_q.combat_geometry,
        &developer_tools,
        &presentation_deltas(&feature_q.combat_geometry, &feature_q.body_deltas),
    );

    // The historical Ambition protagonist still has extra app-specific policy
    // diagnostics (blink internals, authored preview, health). Treat those as an
    // optional enrichment instead of the admission ticket for the whole overlay.
    if let Ok((
        _player_entity,
        mut cluster_item,
        motion_model,
        player_health,
        attack,
        worn_character,
        presented,
    )) = player_q.single_mut()
    {
        let clusters = cluster_item.as_clusters_mut();
        let player_draw_pos =
            presented.map_or(clusters.kinematics.pos, |presented| presented.presented());
        let player_gravity =
            ambition_platformer2d::world::gravity_dir_or_default(feature_q.gravity.as_deref());
        draw_player_debug(
            &mut gizmos,
            world,
            &feature_q.character_catalog,
            &feature_q.authored_attack_volumes,
            worn_character.id(),
            &clusters,
            player_draw_pos,
            motion_model,
            collision.solids().as_deref().unwrap_or(world),
            attack.swing.as_ref(),
            actions,
            gameplay_active,
            &developer_tools,
            player_gravity,
            &mut overlay_labels,
        );
        if developer_tools.show_health_bars {
            draw_health_bars(
                &mut gizmos,
                world,
                clusters.kinematics.aabb(),
                player_health,
            );
        }
    }

    if developer_tools.show_feature_hitboxes {
        draw_feature_debug(
            &mut gizmos,
            world,
            &feature_q,
            &developer_tools,
            &mut overlay_labels,
        );
        draw_projectile_debug(
            &mut gizmos,
            world,
            feature_q.live_projectiles.iter(),
            &developer_tools,
        );
        draw_held_projectiles(
            &mut gizmos,
            world,
            feature_q.held_projectiles.iter(),
            &developer_tools,
        );
        #[cfg(feature = "portal")]
        draw_portals(&mut gizmos, world, portals.iter());
    }
}
