//! Debug drawing for the Bevy sandbox backend.
//!
//! These overlays intentionally live in the Bevy adapter layer. The movement
//! engine exposes simulation state; this module decides how to visualize that
//! state for tuning and feel work.

#![allow(unused_imports)]
use ambition::engine_core as ae;
use ambition::engine_core::AabbExt;
use bevy::ecs::system::SystemParam;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use ambition::actors::rooms::{LoadingZone, LoadingZoneActivation, RoomSet};
use ambition::actors::world::platforms;
use ambition::dev_tools::dev_tools::DeveloperTools;
use ambition::dev_tools::SandboxDevState;
use ambition::engine_core::config::world_to_bevy;
use ambition::engine_core::RoomGeometry;
#[cfg(feature = "input")]
use ambition::input::SandboxAction;
use ambition::input::{read_gameplay_control_frame, ControlFrame};
use ambition::platformer::schedule::GameMode;
#[cfg(feature = "input")]
use ambition::render::rendering::PlayerVisual;
use ambition::render::rendering::{CameraViewState, SceneEntities};
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

mod gizmos;
mod prims;
pub use gizmos::*;
pub use prims::*;

/// Marker for the pooled `Text2d` entities that render debug-box labels.
#[derive(Component)]
pub struct DebugOverlayLabel;

/// Materialize the per-frame [`DebugOverlayLabels`] buffer (filled by the
/// overlay draw calls) as world-space `Text2d`. Despawns last frame's labels
/// and respawns this frame's — debug-only and a handful of labels, so the spawn
/// churn is negligible and avoids pool bookkeeping. Empties the buffer every
/// frame, so toggling the overlay off (no pushes) clears the labels next frame.
pub(crate) fn render_debug_overlay_labels(
    mut commands: Commands,
    world: Res<RoomGeometry>,
    mut labels: ResMut<DebugOverlayLabels>,
    existing: Query<Entity, With<DebugOverlayLabel>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for label in labels.0.drain(..) {
        commands.spawn((
            Text2d::new(label.text),
            TextFont {
                font_size: DEBUG_LABEL_FONT_PX,
                ..default()
            },
            TextColor(label.color),
            Transform::from_translation(world_to_bevy(&world.0, label.world_pos, DEBUG_LABEL_Z)),
            DebugOverlayLabel,
            Name::new("Debug box label"),
        ));
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
    world: Res<RoomGeometry>,
    dev_state: Res<SandboxDevState>,
    platform_set: Res<ambition::actors::MovingPlatformSet>,
    developer_tools: Res<DeveloperTools>,
    room_set: Res<RoomSet>,
    ldtk_spine_index: Res<ambition::actors::ldtk_world::LdtkRuntimeSpineIndex>,
    camera_view: Res<CameraViewState>,
    mode: Res<State<GameMode>>,
    entities: Res<SceneEntities>,
    // Per-frame buffer of debug-box labels; filled below, rendered as Text2d by
    // `render_debug_overlay_labels`. (In-flight projectile queries moved into
    // `FeatureDebugQueries` to keep this system under Bevy's 16-param ceiling.)
    mut overlay_labels: ResMut<DebugOverlayLabels>,
    action_query: Query<&ActionState<SandboxAction>, With<PlayerVisual>>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            Option<&ambition::characters::actor::BodyHealth>,
            &ambition::actors::player::BodyMelee,
        ),
        // The primary player never carries `FeatureSimEntity` (player vs
        // feature-sim entities are mutually exclusive — see the kinematics
        // unification). Spell that disjointness out with `Without` so Bevy can
        // prove this `&mut BodyKinematics` (BodyClusterQueryData) query does
        // not conflict with the `bosses`/`actors` feature queries that read
        // `BodyKinematics` under `With<FeatureSimEntity>` (B0001).
        (
            ambition::actors::actor::PrimaryPlayerOnly,
            Without<ambition::actors::features::FeatureSimEntity>,
        ),
    >,
    feature_q: FeatureDebugQueries,
    #[cfg(feature = "portal")] portals: Query<&ambition::portal::PlacedPortal>,
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
        action_query.get(entities.player).ok()
    } else {
        None
    };
    let Ok((mut cluster_item, player_health, attack)) = player_q.single_mut() else {
        return;
    };
    // Both debug-overlay helpers (`draw_player_debug`,
    // `draw_health_bars`) take cluster refs directly.
    let clusters = cluster_item.as_clusters_mut();
    if developer_tools.show_room_bounds {
        draw_room_bounds(&mut gizmos, world);
    }
    if developer_tools.show_world_blocks {
        draw_world_blocks(&mut gizmos, world, &developer_tools);
        // Momentum ride-surfaces live alongside the blocks (S3b): show the
        // SurfaceChains + their normals/tangents under the same toggle.
        draw_surface_chains(&mut gizmos, world);
    }
    if developer_tools.show_micro_grid {
        draw_micro_grid(&mut gizmos, world, 8.0, 16.0);
    }
    // With sprites hidden the world is a black void; draw the coarse
    // world grid (matches the sprite-grid GRID_STEP) so the player
    // keeps a spatial reference once the tile / parallax sprites
    // disappear. Uses the regular grid, not the micro-grid.
    if developer_tools.hide_sprites {
        draw_world_grid(&mut gizmos, world);
    }
    if developer_tools.show_camera_frame {
        draw_camera_frame(&mut gizmos, world, &camera_view);
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
    let player_gravity =
        ambition::actors::physics::gravity_dir_or_default(feature_q.gravity.as_deref());
    draw_player_debug(
        &mut gizmos,
        world,
        &clusters,
        &platform_set.0,
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
    if developer_tools.show_feature_hitboxes {
        draw_feature_debug(
            &mut gizmos,
            world,
            &feature_q,
            Some((entities.player, clusters.kinematics.pos)),
            &developer_tools,
            &mut overlay_labels,
        );
        draw_projectile_debug(
            &mut gizmos,
            world,
            feature_q.player_projectiles.iter(),
            feature_q.enemy_projectiles.iter(),
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
