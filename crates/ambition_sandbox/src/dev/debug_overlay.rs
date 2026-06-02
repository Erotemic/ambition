//! Debug drawing for the Bevy sandbox backend.
//!
//! These overlays intentionally live in the Bevy adapter layer. The movement
//! engine exposes simulation state; this module decides how to visualize that
//! state for tuning and feel work.

use crate::engine_core as ae;
use crate::engine_core::AabbExt;
use bevy::ecs::system::SystemParam;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::config::world_to_bevy;
use crate::dev::dev_tools::DeveloperTools;
use crate::input::ControlFrame;
#[cfg(feature = "input")]
use crate::input::SandboxAction;
#[cfg(feature = "input")]
use crate::presentation::rendering::PlayerVisual;
use crate::presentation::rendering::{CameraViewState, SceneEntities};
use crate::rooms::{LoadingZone, LoadingZoneActivation, RoomSet};
use crate::world::platforms;
use crate::{GameMode, GameWorld, SandboxDevState};
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

fn cyan() -> Color {
    Color::srgba(0.30, 0.92, 1.00, 0.92)
}
fn blue() -> Color {
    Color::srgba(0.30, 0.55, 1.00, 0.90)
}
fn green() -> Color {
    Color::srgba(0.25, 1.00, 0.45, 0.90)
}
fn yellow() -> Color {
    Color::srgba(1.00, 0.92, 0.22, 0.95)
}
fn orange() -> Color {
    Color::srgba(1.00, 0.55, 0.16, 0.90)
}
fn magenta() -> Color {
    Color::srgba(1.00, 0.32, 0.92, 0.88)
}
fn red() -> Color {
    Color::srgba(1.00, 0.18, 0.22, 0.82)
}
fn white_dim() -> Color {
    Color::srgba(0.90, 0.95, 1.00, 0.40)
}
fn gray() -> Color {
    Color::srgba(0.62, 0.66, 0.75, 0.46)
}

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
    platform_set: Res<crate::MovingPlatformSet>,
    developer_tools: Res<DeveloperTools>,
    room_set: Res<RoomSet>,
    ldtk_spine_index: Res<crate::ldtk_world::LdtkRuntimeSpineIndex>,
    camera_view: Res<CameraViewState>,
    mode: Res<State<GameMode>>,
    entities: Res<SceneEntities>,
    // Per-player projectile state lives on player entities now —
    // iterate every player to render their bodies.
    player_projectiles: Query<
        &crate::projectile::PlayerProjectileState,
        With<crate::player::PlayerEntity>,
    >,
    enemy_projectiles: Res<crate::enemy_projectile::EnemyProjectileState>,
    action_query: Query<&ActionState<SandboxAction>, With<PlayerVisual>>,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            Option<&crate::player::PlayerHealth>,
            &crate::player::ActivePlayerAttack,
        ),
        crate::player::PrimaryPlayerOnly,
    >,
    feature_q: FeatureDebugQueries,
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
    draw_player_debug(
        &mut gizmos,
        world,
        &clusters,
        &platform_set.0,
        attack.0.as_ref(),
        actions,
        gameplay_active,
        &developer_tools,
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
            &enemy_projectiles,
            &developer_tools,
        );
    }
}

#[cfg(feature = "input")]
#[derive(SystemParam)]
pub struct FeatureDebugQueries<'w, 's> {
    pub bosses: Query<
        'w,
        's,
        (
            &'static crate::features::BossFeature,
            &'static crate::brain::BossAttackState,
            Option<&'static crate::features::BossAnimationFrameSample>,
        ),
        With<crate::features::FeatureSimEntity>,
    >,
    pub actors: Query<
        'w,
        's,
        (
            &'static crate::features::ActorRuntime,
            &'static crate::features::FeatureAabb,
            Option<&'static crate::features::ActorKinematics>,
            Option<&'static crate::features::ActorAttackState>,
            Option<&'static crate::features::ActorSurfaceState>,
        ),
        With<crate::features::FeatureSimEntity>,
    >,
    pub breakables: Query<
        'w,
        's,
        &'static crate::features::FeatureAabb,
        (
            With<crate::features::FeatureSimEntity>,
            With<crate::features::BreakableFeature>,
        ),
    >,
    pub chests: Query<
        'w,
        's,
        &'static crate::features::FeatureAabb,
        (
            With<crate::features::FeatureSimEntity>,
            With<crate::features::ChestFeature>,
        ),
    >,
    pub hazards: Query<
        'w,
        's,
        &'static crate::features::HazardFeature,
        With<crate::features::FeatureSimEntity>,
    >,
    /// All live `Hitbox` entities (melee swings, World-anchored
    /// hazards like the Gradient Sentinel's MinimaTrap pit /
    /// SaddlePoint arms / GradientLane column). Drawn so the debug
    /// view answers "what just hit me?" — without this pass the
    /// World-anchored boss specials are invisible even though they
    /// deal damage.
    pub hitboxes: Query<'w, 's, &'static crate::features::Hitbox>,
    /// FeatureAabb lookup for resolving `FollowOwner` hitboxes to
    /// their current world-space rectangle. World-anchored
    /// hitboxes don't need this — their AABB is fixed at spawn.
    pub hitbox_owners: Query<'w, 's, &'static crate::features::FeatureAabb>,
}

fn draw_room_bounds(gizmos: &mut Gizmos, world: &ae::World) {
    let room = ae::aabb_from_min_size(ae::Vec2::ZERO, world.size);
    draw_aabb(gizmos, world, room, white_dim());
}

fn draw_micro_grid(gizmos: &mut Gizmos, world: &ae::World, minor: f32, major: f32) {
    if minor <= 0.0 || major <= 0.0 {
        return;
    }
    let minor_color = Color::srgba(0.45, 0.55, 0.70, 0.13);
    let major_color = Color::srgba(0.70, 0.80, 1.00, 0.23);
    let cols = (world.size.x / minor).ceil() as i32;
    let rows = (world.size.y / minor).ceil() as i32;
    for i in 0..=cols {
        let x = (i as f32 * minor).min(world.size.x);
        let is_major = (x / major).fract().abs() < 0.01;
        let color = if is_major { major_color } else { minor_color };
        gizmos.line_2d(
            w2(world, ae::Vec2::new(x, 0.0)),
            w2(world, ae::Vec2::new(x, world.size.y)),
            color,
        );
    }
    for i in 0..=rows {
        let y = (i as f32 * minor).min(world.size.y);
        let is_major = (y / major).fract().abs() < 0.01;
        let color = if is_major { major_color } else { minor_color };
        gizmos.line_2d(
            w2(world, ae::Vec2::new(0.0, y)),
            w2(world, ae::Vec2::new(world.size.x, y)),
            color,
        );
    }
}

fn draw_camera_frame(gizmos: &mut Gizmos, world: &ae::World, view: &CameraViewState) {
    let requested = ae::Aabb::new(view.target_world, view.requested_view * 0.5);
    let visible = ae::Aabb::new(view.center_world, view.visible_view * 0.5);
    draw_aabb(gizmos, world, visible, Color::srgba(0.20, 0.95, 1.00, 0.22));
    draw_aabb(
        gizmos,
        world,
        requested,
        Color::srgba(1.00, 0.95, 0.20, 0.22),
    );
}

fn draw_world_blocks(gizmos: &mut Gizmos, world: &ae::World, developer_tools: &DeveloperTools) {
    for block in &world.blocks {
        let color = match block.kind {
            ae::BlockKind::Solid => gray(),
            ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Soft,
            } => magenta(),
            ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Hard,
            } => red(),
            ae::BlockKind::OneWay => blue(),
            ae::BlockKind::Hazard => red(),
            ae::BlockKind::PogoOrb => green(),
            ae::BlockKind::Rebound { .. } => orange(),
        };
        draw_aabb_styled(gizmos, world, block.aabb, color, developer_tools);
    }
}

/// Lightweight coarse grid drawn straight through gizmos. Used when
/// `hide_sprites` strips the authored sprite grid so the player still
/// has a spatial reference. Spacing matches `crate::config::GRID_STEP`
/// (the same step the sprite grid spawned in `spawn_grid` uses).
fn draw_world_grid(gizmos: &mut Gizmos, world: &ae::World) {
    let step = crate::config::GRID_STEP;
    if step <= 0.0 {
        return;
    }
    let color = Color::srgba(0.45, 0.55, 0.70, 0.32);
    let cols = (world.size.x / step).ceil() as i32;
    let rows = (world.size.y / step).ceil() as i32;
    for i in 0..=cols {
        let x = (i as f32 * step).min(world.size.x);
        gizmos.line_2d(
            w2(world, ae::Vec2::new(x, 0.0)),
            w2(world, ae::Vec2::new(x, world.size.y)),
            color,
        );
    }
    for j in 0..=rows {
        let y = (j as f32 * step).min(world.size.y);
        gizmos.line_2d(
            w2(world, ae::Vec2::new(0.0, y)),
            w2(world, ae::Vec2::new(world.size.x, y)),
            color,
        );
    }
}

fn draw_loading_zones(gizmos: &mut Gizmos, world: &ae::World, zones: &[LoadingZone]) {
    for zone in zones {
        let color = match zone.activation {
            LoadingZoneActivation::EdgeExit => cyan(),
            LoadingZoneActivation::Door => yellow(),
            // `Walk` zones — mid-room walk-through portals.
            // Distinct green so they don't read as either an edge
            // exit (cyan) or an interact door (yellow).
            LoadingZoneActivation::Walk => Color::srgba(0.40, 1.00, 0.55, 0.85),
        };
        draw_aabb(gizmos, world, zone.aabb, color);
    }
}

fn draw_ldtk_runtime_spine(
    gizmos: &mut Gizmos,
    world: &ae::World,
    spine_index: &crate::ldtk_world::LdtkRuntimeSpineIndex,
) {
    for entity in &spine_index.entities {
        let color = match entity.role {
            crate::ldtk_world::LdtkRuntimeRole::PlayerStart => green(),
            crate::ldtk_world::LdtkRuntimeRole::LoadingZone => Color::srgba(1.0, 1.0, 1.0, 0.70),
            crate::ldtk_world::LdtkRuntimeRole::DebugLabel => magenta(),
            crate::ldtk_world::LdtkRuntimeRole::CameraZone => blue(),
            // Solid runtime rects are drawn by the dedicated Solid index pass
            // so they can be color-keyed against the JSON-derived collision
            // blocks during the Step 2 raw-vs-runtime overlay work.
            crate::ldtk_world::LdtkRuntimeRole::Solid => continue,
            // OneWayPlatform / DamageVolume have their own dedicated runtime
            // indices and overlay passes; skip them in the generic spine
            // overlay so colors don't double-stamp.
            crate::ldtk_world::LdtkRuntimeRole::OneWayPlatform => continue,
            crate::ldtk_world::LdtkRuntimeRole::DamageVolume => continue,
            crate::ldtk_world::LdtkRuntimeRole::Other => continue,
        };
        draw_aabb(gizmos, world, entity.aabb(), color);
    }
}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
fn draw_player_debug(
    gizmos: &mut Gizmos,
    world: &ae::World,
    clusters: &ae::PlayerClustersMut<'_>,
    moving_platforms: &[crate::world::platforms::MovingPlatformState],
    attack: Option<&crate::PlayerAttackState>,
    actions: Option<&ActionState<SandboxAction>>,
    gameplay_active: bool,
    developer_tools: &DeveloperTools,
) {
    let pos = clusters.kinematics.pos;
    let vel = clusters.kinematics.vel;
    let size = clusters.kinematics.size;
    let facing = clusters.kinematics.facing;
    let on_ground = clusters.ground.on_ground;
    let on_wall = clusters.wall.on_wall;
    let wall_normal_x = clusters.wall.wall_normal_x;
    let body = clusters.kinematics.aabb();
    if developer_tools.show_player_hitbox {
        draw_aabb_styled(gizmos, world, body, cyan(), developer_tools);
    }

    let center = w2(world, pos);

    if developer_tools.show_player_vectors {
        let velocity_delta = engine_delta_to_bevy(vel * 0.18);
        draw_arrow(gizmos, center, center + velocity_delta, blue());

        let facing_end = center + BVec2::new(facing * 58.0, 0.0);
        draw_arrow(gizmos, center, facing_end, green());

        if on_ground {
            let feet = w2(world, ae::Vec2::new(pos.x, body.bottom()));
            draw_arrow(gizmos, feet, feet + BVec2::new(0.0, 44.0), green());
        }
        if on_wall {
            let side_x = if wall_normal_x < 0.0 {
                body.left()
            } else {
                body.right()
            };
            let side = w2(world, ae::Vec2::new(side_x, pos.y));
            draw_arrow(
                gizmos,
                side,
                side + BVec2::new(wall_normal_x * 48.0, 0.0),
                green(),
            );
        }
    }

    // Combat preview: active attacks show their real phase hitbox. When no
    // swing is active, holding the button previews the resolved directional
    // intent from the live input axes. Colors mirror the attack lifecycle:
    // startup = yellow, active = red, recovery = gray.
    let controls = actions.map(ControlFrame::read_gameplay).unwrap_or_default();
    let attack_held = actions
        .map(|actions| actions.pressed(&SandboxAction::Attack))
        .unwrap_or(false);
    let dedicated_pogo_held = actions
        .map(|actions| actions.pressed(&SandboxAction::Pogo))
        .unwrap_or(false);
    if gameplay_active && developer_tools.show_combat_preview {
        let view = crate::combat::AttackView {
            pos,
            size,
            facing,
            on_ground,
            wall_clinging: clusters.wall.wall_clinging,
            dash_timer: clusters.dash.timer,
            abilities_directional_primary: clusters.abilities.abilities.directional_primary,
        };
        if let Some(attack_state) = attack {
            let hitbox = crate::combat::attack_hitbox_from_view(&view, attack_state.spec);
            let color = match attack_state.phase() {
                Some(crate::combat::AttackPhase::Startup) => yellow(),
                Some(crate::combat::AttackPhase::Active) => red(),
                Some(crate::combat::AttackPhase::Recovery) => gray(),
                None => gray(),
            };
            draw_aabb(gizmos, world, hitbox, color);
        } else if attack_held || dedicated_pogo_held {
            let intent = crate::combat::resolve_attack_intent_from_view(
                &view,
                controls.axis_x,
                controls.axis_y,
                dedicated_pogo_held || controls.pogo_pressed,
            );
            let hitbox = crate::combat::attack_hitbox_from_view(
                &view,
                crate::combat::attack_spec_from_view(&view, intent),
            );
            draw_aabb(gizmos, world, hitbox, yellow());
        }
    }

    // Ledge grab / climb debug.
    if developer_tools.show_combat_preview {
        if let Some(ledge) = clusters.ledge.grab.as_ref() {
            let anchor_box = ae::Aabb::new(ledge.contact.anchor, ae::Vec2::splat(5.0));
            let target_box = ae::Aabb::new(ledge.contact.climb_target, size * 0.35);
            draw_aabb(gizmos, world, anchor_box, cyan());
            draw_aabb(
                gizmos,
                world,
                target_box,
                if ledge.climbing { green() } else { yellow() },
            );
            draw_arrow(
                gizmos,
                w2(world, ledge.contact.anchor),
                w2(world, ledge.contact.climb_target),
                if ledge.climbing { green() } else { yellow() },
            );
        }
    }

    // Blink aim preview.
    if gameplay_active
        && developer_tools.show_blink_preview
        && (controls.blink_held || clusters.blink.aiming)
    {
        let blink_world = platforms::world_with_moving_platforms(world, moving_platforms);
        let (desired, target) = if clusters.blink.aiming {
            let desired = pos + clusters.blink.aim_offset;
            let target = ae::blink_destination_to_point_clusters(
                &blink_world,
                clusters.kinematics,
                clusters.abilities,
                desired,
            );
            (desired, target)
        } else {
            let aim = ae::Vec2::new(controls.axis_x, controls.axis_y)
                .normalize_or(ae::Vec2::new(facing, 0.0));
            let desired = pos + aim * ae::BLINK_DISTANCE;
            let target = ae::blink_destination_clusters(
                &blink_world,
                clusters.kinematics,
                clusters.abilities,
                aim,
                ae::BLINK_DISTANCE,
            );
            (desired, target)
        };
        let target_center = w2(world, target);
        draw_arrow(gizmos, center, target_center, magenta());
        draw_aabb(gizmos, world, ae::Aabb::new(target, size * 0.5), magenta());
        if (desired - target).length_squared() > 4.0 {
            draw_aabb(gizmos, world, ae::Aabb::new(desired, size * 0.35), red());
            gizmos.line_2d(w2(world, desired), target_center, red());
        }
    }

    // Small status ticks above the player: dash and air jump availability.
    let meter_y = body.top() - 18.0;
    let abilities = &clusters.abilities.abilities;
    let dash_slots = abilities.dash_charge_count().max(1) as usize;
    for i in 0..dash_slots {
        let x0 = pos.x - 28.0 + i as f32 * 12.0;
        let color = if i < clusters.dash.charges_available as usize {
            yellow()
        } else {
            gray()
        };
        let a = w2(world, ae::Vec2::new(x0, meter_y));
        let b = w2(world, ae::Vec2::new(x0 + 8.0, meter_y));
        gizmos.line_2d(a, b, color);
    }
    let air_jump_slots = abilities.air_jump_count(ae::AIR_JUMPS).max(1) as usize;
    for i in 0..air_jump_slots {
        let x0 = pos.x + 6.0 + i as f32 * 11.0;
        let color = if i < clusters.jump.air_jumps_available as usize {
            cyan()
        } else {
            gray()
        };
        let a = w2(world, ae::Vec2::new(x0, meter_y));
        let b = w2(world, ae::Vec2::new(x0 + 7.0, meter_y));
        gizmos.line_2d(a, b, color);
    }
}

fn draw_moving_platform_debug(
    gizmos: &mut Gizmos,
    world: &ae::World,
    moving_platforms: &[crate::world::platforms::MovingPlatformState],
) {
    for platform in moving_platforms {
        let aabb = platform.aabb();
        draw_aabb(gizmos, world, aabb, blue());
        let center = w2(world, aabb.center());
        draw_arrow(gizmos, center, center + BVec2::new(44.0, 0.0), blue());
    }
}

fn draw_health_bars(
    gizmos: &mut Gizmos,
    world: &ae::World,
    player_aabb: ae::Aabb,
    player_health: Option<&crate::player::PlayerHealth>,
) {
    let ratio = player_health.map_or(1.0, |h| h.health.ratio());
    draw_health_bar(gizmos, world, player_aabb, ratio, cyan());
    // Enemy / boss / breakable health bars are now drawn by
    // `sync_health_overlays` (the Bevy sprite overlay system), which reads
    // ECS `ActorRuntime`, `BossFeature`, and `BreakableFeature` components.
}

fn draw_health_bar(
    gizmos: &mut Gizmos,
    world: &ae::World,
    aabb: ae::Aabb,
    ratio: f32,
    fill: Color,
) {
    let width = (aabb.half_size().x * 2.0).max(28.0);
    let y = aabb.top() - 14.0;
    let left = aabb.center().x - width * 0.5;
    let right = aabb.center().x + width * 0.5;
    let fill_right = left + width * ratio.clamp(0.0, 1.0);
    gizmos.line_2d(
        w2(world, ae::Vec2::new(left, y)),
        w2(world, ae::Vec2::new(right, y)),
        gray(),
    );
    gizmos.line_2d(
        w2(world, ae::Vec2::new(left, y)),
        w2(world, ae::Vec2::new(fill_right, y)),
        fill,
    );
}

/// Draw debug rectangles for every gameplay feature (NPCs, enemies, bosses,
/// breakables, chests, hazards). Also overlays boss attack telegraph + active
/// volumes when an attack is firing. This is the "solid box" view the player
/// expects when `Hide Sprites` is also on — sprites disappear and the boxes
/// reveal exactly where each entity lives.
fn draw_feature_debug(
    gizmos: &mut Gizmos,
    world: &ae::World,
    feature_q: &FeatureDebugQueries,
    developer_tools: &DeveloperTools,
) {
    // Colors per role — strong enough to read against most backgrounds.
    let npc_color = Color::srgba(0.30, 1.00, 0.45, 0.85); // green
    let enemy_color = Color::srgba(1.00, 0.32, 0.32, 0.88); // red
    let boss_color = Color::srgba(1.00, 0.60, 0.10, 0.88); // orange
    let breakable_color = Color::srgba(0.55, 0.80, 1.00, 0.80); // light blue
    let chest_color = Color::srgba(1.00, 0.85, 0.25, 0.85); // gold
    let hazard_color = Color::srgba(1.00, 0.32, 0.92, 0.80); // magenta
    let telegraph_color = Color::srgba(1.00, 0.95, 0.20, 0.60); // yellow
    let active_color = Color::srgba(1.00, 0.12, 0.12, 0.95); // bright red

    for (actor, aabb, kin, attack, _surface) in feature_q.actors.iter() {
        let color = match actor {
            crate::features::ActorRuntime::Npc(_) => npc_color,
            crate::features::ActorRuntime::Enemy => enemy_color,
        };
        draw_aabb_styled(gizmos, world, aabb.aabb(), color, developer_tools);
        // Hostile actors (and turned-hostile NPCs like the Kernel Guide)
        // own an attack volume that becomes active during a swing — draw
        // it whenever windup or strike timer is live so the player can
        // see exactly where the hit will land. Telegraph wins when both
        // are zero so a frame on the edge still reads as "incoming".
        if matches!(actor, crate::features::ActorRuntime::Enemy) {
            if let (Some(kin), Some(attack)) = (kin, attack) {
                // Forward-swing hitbox geometry (matches
                // EnemyMut::attack_aabb): offset by facing.
                let center = kin.pos
                    + crate::engine_core::Vec2::new(kin.facing * (kin.size.x * 0.55 + 24.0), -4.0);
                let attack_box = crate::engine_core::Aabb::new(
                    center,
                    crate::engine_core::Vec2::new(34.0, 28.0),
                );
                if attack.is_active() {
                    draw_aabb_styled(gizmos, world, attack_box, active_color, developer_tools);
                } else if attack.is_winding_up() {
                    draw_aabb_styled(gizmos, world, attack_box, telegraph_color, developer_tools);
                }
            }
        }
    }
    // Boss debug colors — each color answers a distinct question
    // the player might ask while reading the overlay:
    //
    // - **orange** (`boss_color`, `boss.aabb()`): the combat-collision
    //   envelope. The boss uses this for kinematic step / world-bounds
    //   clamp. Does NOT, by itself, deal damage.
    // - **cyan** (`hurtbox_color`, `damageable_volumes`): where the
    //   *player's* attacks register hits on the boss. With the
    //   sprite-metadata-driven derivation, this can be one rect
    //   (single-piece boss) or many (multi-part body — head + body
    //   + arms).
    // - **magenta** (`body_contact_color`, `body_damage_aabb`): the
    //   boss's body-contact damage zone. Touching this when
    //   `BossBehaviorProfile::body_damage > 0` hurts the player
    //   (e.g. clockwork_warden has body_damage=1). Drawn separately
    //   so the player can answer "why did I get hit by just touching
    //   the boss?" without source-diving.
    // - **yellow** (`telegraph_color`, `telegraph_volumes`):
    //   attack windup volumes (e.g. FloorSlam telegraph).
    // - **red** (`active_color`, `active_attack_volumes`): live
    //   strike volumes. These are also the source of `boss_attack_damage`.
    //
    // Special attack profiles (MinimaTrap, SaddlePoint, GradientLane,
    // OverfitVolley, GradientCascade) route damage through World-
    // anchored `Hitbox` entities, drawn by the later
    // `feature_q.hitboxes` pass with faction colors.
    let hurtbox_color = cyan();
    let body_contact_color = Color::srgba(0.95, 0.30, 0.95, 0.85); // magenta
    for (bf, attack_state, animation_frame) in feature_q.bosses.iter() {
        let boss = &bf.boss;
        if !boss.alive {
            continue;
        }
        let ctx = crate::features::BossVolumeContext::from_runtime(boss, attack_state)
            .with_animation_frame(animation_frame);
        draw_aabb_styled(gizmos, world, boss.aabb(), boss_color, developer_tools);
        // Body-contact damage zone — drawn ONLY when the boss
        // actually deals contact damage so a `body_damage = 0`
        // boss (like GNU-ton) doesn't show a misleading magenta
        // outline.
        if boss.behavior.body_damage > 0 {
            // Use `boss.aabb()` directly — that already factors in
            // `combat_offset` so the magenta box lines up with the
            // visible body (and matches the pogo zone, which uses
            // the same call).
            draw_aabb_styled(
                gizmos,
                world,
                boss.aabb(),
                body_contact_color,
                developer_tools,
            );
        }
        for hurtbox in crate::features::damageable_volumes(&ctx) {
            draw_aabb_styled(gizmos, world, hurtbox, hurtbox_color, developer_tools);
        }
        for vol in crate::features::telegraph_volumes(&ctx) {
            draw_aabb_styled(gizmos, world, vol, telegraph_color, developer_tools);
        }
        for vol in crate::features::active_attack_volumes(&ctx) {
            draw_aabb_styled(gizmos, world, vol, active_color, developer_tools);
        }
    }
    for aabb in feature_q.breakables.iter() {
        draw_aabb_styled(gizmos, world, aabb.aabb(), breakable_color, developer_tools);
    }
    for aabb in feature_q.chests.iter() {
        draw_aabb_styled(gizmos, world, aabb.aabb(), chest_color, developer_tools);
    }
    for hf in feature_q.hazards.iter() {
        draw_aabb_styled(
            gizmos,
            world,
            hf.hazard.aabb(),
            hazard_color,
            developer_tools,
        );
    }

    // Live Hitbox entities — melee swings (FollowOwner) + World-
    // anchored boss specials (MinimaTrap pit, SaddlePoint arms,
    // GradientLane column). Without this pass, the World-anchored
    // hitboxes are invisible in debug mode even though they deal
    // damage. Faction-color-coded so you can read which side it
    // belongs to at a glance.
    let player_hitbox_color = Color::srgba(0.35, 0.85, 1.00, 0.90); // light blue
    let enemy_hitbox_color = Color::srgba(1.00, 0.18, 0.18, 0.90); // bright red
    let boss_hitbox_color = Color::srgba(1.00, 0.55, 0.10, 0.90); // bright orange
    let npc_hitbox_color = Color::srgba(0.60, 1.00, 0.45, 0.85); // light green
    for hitbox in feature_q.hitboxes.iter() {
        let owner_pos = match feature_q.hitbox_owners.get(hitbox.owner) {
            Ok(aabb) => aabb.center,
            // Owner despawned or never had a FeatureAabb — for
            // World-anchored hitboxes this doesn't matter (the
            // anchor carries the center). For FollowOwner with a
            // dead owner the draw position is ambiguous; fall back
            // to ZERO and the rect will appear at the origin.
            Err(_) => ae::Vec2::ZERO,
        };
        let aabb = hitbox.world_aabb(owner_pos);
        let color = match hitbox.source {
            crate::features::ActorFaction::Player => player_hitbox_color,
            crate::features::ActorFaction::Enemy => enemy_hitbox_color,
            crate::features::ActorFaction::Boss => boss_hitbox_color,
            crate::features::ActorFaction::Npc | crate::features::ActorFaction::Neutral => {
                npc_hitbox_color
            }
        };
        draw_aabb_styled(gizmos, world, aabb, color, developer_tools);
    }
}

/// Draw in-flight player and enemy projectile AABBs so they remain
/// visible when `hide_sprites` strips the textured projectile ring.
/// Player projectiles use a warm orange (matches charge tint); enemy
/// projectiles use red so the faction is immediately readable.
fn draw_projectile_debug<'a>(
    gizmos: &mut Gizmos,
    world: &ae::World,
    player_states: impl IntoIterator<Item = &'a crate::projectile::PlayerProjectileState>,
    enemy_state: &crate::enemy_projectile::EnemyProjectileState,
    developer_tools: &DeveloperTools,
) {
    let player_color = Color::srgba(1.00, 0.74, 0.30, 0.92);
    let enemy_color = Color::srgba(1.00, 0.32, 0.32, 0.92);
    for state in player_states {
        for proj in &state.bodies {
            draw_aabb_styled(
                gizmos,
                world,
                proj.body.aabb(),
                player_color,
                developer_tools,
            );
        }
    }
    for proj in &enemy_state.bodies {
        draw_aabb_styled(
            gizmos,
            world,
            proj.body.aabb(),
            enemy_color,
            developer_tools,
        );
    }
}

fn draw_rebound_vectors(gizmos: &mut Gizmos, world: &ae::World) {
    for block in &world.blocks {
        let ae::BlockKind::Rebound { impulse } = block.kind else {
            continue;
        };
        draw_aabb(gizmos, world, block.aabb, orange());
        let start = w2(world, block.aabb.center());
        let direction = impulse.normalize_or(ae::Vec2::new(0.0, -1.0));
        let end = start + engine_delta_to_bevy(direction * 70.0);
        draw_arrow(gizmos, start, end, orange());
    }
}

fn draw_aabb(gizmos: &mut Gizmos, world: &ae::World, aabb: ae::Aabb, color: Color) {
    let min = aabb.min;
    let max = aabb.max;
    let tl = w2(world, ae::Vec2::new(min.x, min.y));
    let tr = w2(world, ae::Vec2::new(max.x, min.y));
    let br = w2(world, ae::Vec2::new(max.x, max.y));
    let bl = w2(world, ae::Vec2::new(min.x, max.y));
    gizmos.line_2d(tl, tr, color);
    gizmos.line_2d(tr, br, color);
    gizmos.line_2d(br, bl, color);
    gizmos.line_2d(bl, tl, color);
}

/// Outline + optional translucent fill. Fills are controlled directly by the
/// current debug view mode instead of being coupled to sprite hiding; choose
/// Collision/Combat/Triggers when the filled volume view is useful.
fn draw_aabb_styled(
    gizmos: &mut Gizmos,
    world: &ae::World,
    aabb: ae::Aabb,
    color: Color,
    developer_tools: &DeveloperTools,
) {
    draw_aabb(gizmos, world, aabb, color);
    if !developer_tools.fill_debug_boxes {
        return;
    }
    let size = aabb.half_size() * 2.0;
    let center = w2(world, aabb.center());
    let fill = with_alpha(color, 0.22);
    // Bevy gizmos' `rect_2d` draws the outline by default. We want a
    // filled appearance, so draw a stack of horizontal lines spaced
    // 2px apart — works on every Bevy gizmo backend without needing a
    // separate mesh path. The cost is bounded (each AABB is small in
    // pixel terms and we only call this when the toggle is on).
    let step = 2.0;
    let half_h = (size.y * 0.5).max(0.5);
    let mut y = -half_h;
    while y < half_h {
        let a = BVec2::new(center.x - size.x * 0.5, center.y + y);
        let b = BVec2::new(center.x + size.x * 0.5, center.y + y);
        gizmos.line_2d(a, b, fill);
        y += step;
    }
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    let srgba = color.to_srgba();
    Color::srgba(srgba.red, srgba.green, srgba.blue, alpha.clamp(0.0, 1.0))
}

fn draw_arrow(gizmos: &mut Gizmos, start: BVec2, end: BVec2, color: Color) {
    gizmos.line_2d(start, end, color);
    let delta = end - start;
    let len = delta.length();
    if len <= 1.0 {
        return;
    }
    let dir = delta / len;
    let side = BVec2::new(-dir.y, dir.x);
    let head = 9.0_f32.min(len * 0.28);
    gizmos.line_2d(end, end - dir * head + side * head * 0.55, color);
    gizmos.line_2d(end, end - dir * head - side * head * 0.55, color);
}

fn w2(world: &ae::World, p: ae::Vec2) -> BVec2 {
    world_to_bevy(world, p, 0.0).truncate()
}

fn engine_delta_to_bevy(delta: ae::Vec2) -> BVec2 {
    BVec2::new(delta.x, -delta.y)
}
