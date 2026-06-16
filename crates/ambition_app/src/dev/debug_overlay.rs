//! Debug drawing for the Bevy sandbox backend.
//!
//! These overlays intentionally live in the Bevy adapter layer. The movement
//! engine exposes simulation state; this module decides how to visualize that
//! state for tuning and feel work.

#![allow(unused_imports)]
use ambition_sandbox::engine_core as ae;
use ambition_sandbox::engine_core::AabbExt;
use bevy::ecs::system::SystemParam;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

#[cfg(feature = "input")]
use ambition_render::rendering::PlayerVisual;
use ambition_render::rendering::{CameraViewState, SceneEntities};
use ambition_sandbox::config::world_to_bevy;
use ambition_sandbox::dev::dev_tools::DeveloperTools;
use ambition_sandbox::input::ControlFrame;
#[cfg(feature = "input")]
use ambition_sandbox::input::SandboxAction;
use ambition_sandbox::rooms::{LoadingZone, LoadingZoneActivation, RoomSet};
use ambition_sandbox::world::platforms;
use ambition_sandbox::{GameMode, GameWorld, SandboxDevState};
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

mod gizmos;
mod prims;
pub use gizmos::*;
pub use prims::*;

/// No-op stub for builds without the `input` feature. The full overlay
/// reads leafwing's `ActionState` to render combat/blink previews; without
/// leafwing in scope, gizmos for those would have no input source. Sim
/// gizmos that don't need input are also skipped to keep the chain
/// signature stable across feature combinations.
#[cfg(not(feature = "input"))]
pub fn draw_debug_overlay() {}

#[cfg(feature = "input")]
pub fn draw_debug_overlay(
    mut gizmos: Gizmos,
    world: Res<GameWorld>,
    dev_state: Res<SandboxDevState>,
    platform_set: Res<ambition_sandbox::MovingPlatformSet>,
    developer_tools: Res<DeveloperTools>,
    room_set: Res<RoomSet>,
    ldtk_spine_index: Res<ambition_sandbox::ldtk_world::LdtkRuntimeSpineIndex>,
    camera_view: Res<CameraViewState>,
    mode: Res<State<GameMode>>,
    entities: Res<SceneEntities>,
    // In-flight player projectiles are ECS entities now (Phase 3c-ii) —
    // draw each one's AABB from its kinematic body. `Without<PlayerEntity>`
    // spells out that a projectile is never the player, so Bevy can prove
    // this read of `BodyKinematics` is disjoint from the `&mut` player query
    // below (B0001).
    player_projectiles: Query<
        &ambition_sandbox::player::BodyKinematics,
        (
            With<ambition_sandbox::projectile::PlayerProjectile>,
            Without<ambition_sandbox::player::PlayerEntity>,
        ),
    >,
    // In-flight enemy projectiles are ECS entities now (Phase 3c-iii) — draw
    // each one's AABB from its kinematic body. Same `Without<PlayerEntity>`
    // disjointness as the player projectiles above.
    enemy_projectiles: Query<
        &ambition_sandbox::player::BodyKinematics,
        (
            With<ambition_sandbox::enemy_projectile::EnemyProjectile>,
            Without<ambition_sandbox::player::PlayerEntity>,
        ),
    >,
    action_query: Query<&ActionState<SandboxAction>, With<PlayerVisual>>,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            Option<&ambition_sandbox::player::PlayerHealth>,
            &ambition_sandbox::player::ActivePlayerAttack,
        ),
        // The primary player never carries `FeatureSimEntity` (player vs
        // feature-sim entities are mutually exclusive — see the kinematics
        // unification). Spell that disjointness out with `Without` so Bevy can
        // prove this `&mut BodyKinematics` (PlayerClusterQueryData) query does
        // not conflict with the `bosses`/`actors` feature queries that read
        // `BodyKinematics` under `With<FeatureSimEntity>` (B0001).
        (
            ambition_sandbox::player::PrimaryPlayerOnly,
            Without<ambition_sandbox::features::FeatureSimEntity>,
        ),
    >,
    feature_q: FeatureDebugQueries,
    #[cfg(feature = "portal")] portals: Query<&ambition_sandbox::portal::PlacedPortal>,
) {
    if !dev_state.debug_enabled() || !developer_tools.gizmos_enabled {
        return;
    }

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
    let player_gravity = feature_q
        .gravity
        .as_deref()
        .map_or(ae::Vec2::new(0.0, 1.0), |g| g.dir);
    draw_player_debug(
        &mut gizmos,
        world,
        &clusters,
        &platform_set.0,
        attack.0.as_ref(),
        actions,
        gameplay_active,
        &developer_tools,
        player_gravity,
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
        draw_feature_debug(&mut gizmos, world, &feature_q, &developer_tools);
        draw_projectile_debug(
            &mut gizmos,
            world,
            player_projectiles.iter(),
            enemy_projectiles.iter(),
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
