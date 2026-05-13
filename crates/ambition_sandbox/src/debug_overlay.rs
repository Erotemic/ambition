//! Debug drawing for the Bevy sandbox backend.
//!
//! These overlays intentionally live in the Bevy adapter layer. The movement
//! engine exposes simulation state; this module decides how to visualize that
//! state for tuning and feel work.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::config::world_to_bevy;
use crate::dev_tools::DeveloperTools;
use crate::input::ControlFrame;
#[cfg(feature = "input")]
use crate::input::SandboxAction;
use crate::platforms;
#[cfg(feature = "input")]
use crate::rendering::PlayerVisual;
use crate::rendering::{CameraViewState, SceneEntities};
use crate::rooms::{LoadingZone, LoadingZoneActivation, RoomSet};
use crate::{GameMode, GameWorld, SandboxRuntime};
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
    runtime: Res<SandboxRuntime>,
    developer_tools: Res<DeveloperTools>,
    room_set: Res<RoomSet>,
    ldtk_spine_index: Res<crate::ldtk_world::LdtkRuntimeSpineIndex>,
    camera_view: Res<CameraViewState>,
    mode: Res<State<GameMode>>,
    entities: Res<SceneEntities>,
    action_query: Query<&ActionState<SandboxAction>, With<PlayerVisual>>,
) {
    if !runtime.debug_enabled() || !developer_tools.gizmos_enabled {
        return;
    }

    let world = &world.0;
    // Mirror the gameplay input gate used by sandbox_update. Raw Leafwing
    // action state still records button presses while paused so pause/menu
    // UI can respond, but debug combat/blink previews are gameplay-facing and
    // should not light up from those paused-mode inputs.
    let gameplay_active = mode.get().allows_gameplay();
    let actions = if gameplay_active {
        action_query.get(entities.player).ok()
    } else {
        None
    };
    if developer_tools.show_room_bounds {
        draw_room_bounds(&mut gizmos, world);
    }
    if developer_tools.show_world_blocks {
        draw_world_blocks(&mut gizmos, world);
    }
    if developer_tools.show_micro_grid {
        draw_micro_grid(&mut gizmos, world, 8.0, 16.0);
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
        draw_moving_platform_debug(&mut gizmos, world, &runtime);
    }
    draw_player_debug(
        &mut gizmos,
        world,
        &runtime,
        actions,
        gameplay_active,
        &developer_tools,
    );
    if developer_tools.show_health_bars {
        draw_health_bars(&mut gizmos, world, &runtime);
    }
    if developer_tools.show_feature_hitboxes {
        draw_feature_combat_debug(&mut gizmos, world, &runtime);
    }
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

fn draw_world_blocks(gizmos: &mut Gizmos, world: &ae::World) {
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
        draw_aabb(gizmos, world, block.aabb, color);
    }
}

fn draw_loading_zones(gizmos: &mut Gizmos, world: &ae::World, zones: &[LoadingZone]) {
    for zone in zones {
        let color = match zone.activation {
            LoadingZoneActivation::EdgeExit => cyan(),
            LoadingZoneActivation::Door => yellow(),
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
fn draw_player_debug(
    gizmos: &mut Gizmos,
    world: &ae::World,
    runtime: &SandboxRuntime,
    actions: Option<&ActionState<SandboxAction>>,
    gameplay_active: bool,
    developer_tools: &DeveloperTools,
) {
    let player = &runtime.player;
    let body = player.aabb();
    if developer_tools.show_player_hitbox {
        draw_aabb(gizmos, world, body, cyan());
    }

    let center = w2(world, player.pos);

    if developer_tools.show_player_vectors {
        // Velocity is the most important feel-tuning vector. The scalar is visual
        // only; it keeps endgame speeds readable inside the 1600x900 sandbox.
        let velocity_delta = engine_delta_to_bevy(player.vel * 0.18);
        draw_arrow(gizmos, center, center + velocity_delta, blue());

        // Facing/control intent vector. This helps diagnose attack orientation and
        // whether the current preset feels natural in the hands.
        let facing_end = center + BVec2::new(player.facing * 58.0, 0.0);
        draw_arrow(gizmos, center, facing_end, green());

        // Contact hints: upward ground normal and lateral wall normal.
        if player.on_ground {
            let feet = w2(world, ae::Vec2::new(player.pos.x, body.bottom()));
            draw_arrow(gizmos, feet, feet + BVec2::new(0.0, 44.0), green());
        }
        if player.on_wall {
            let side_x = if player.wall_normal_x < 0.0 {
                body.left()
            } else {
                body.right()
            };
            let side = w2(world, ae::Vec2::new(side_x, player.pos.y));
            draw_arrow(
                gizmos,
                side,
                side + BVec2::new(player.wall_normal_x * 48.0, 0.0),
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
        if let Some(attack_state) = runtime.player_attack.as_ref() {
            let hitbox = ae::attack_hitbox(player, attack_state.spec);
            let color = match attack_state.phase() {
                Some(ae::AttackPhase::Startup) => yellow(),
                Some(ae::AttackPhase::Active) => red(),
                Some(ae::AttackPhase::Recovery) => gray(),
                None => gray(),
            };
            draw_aabb(gizmos, world, hitbox, color);
        } else if attack_held || dedicated_pogo_held {
            let intent = ae::resolve_attack_intent(
                player,
                controls.axis_x,
                controls.axis_y,
                dedicated_pogo_held || controls.pogo_pressed,
            );
            let hitbox = ae::attack_hitbox(player, ae::attack_spec(player, intent));
            draw_aabb(gizmos, world, hitbox, yellow());
        }
    }

    // Ledge grab / climb debug. Reuse the combat preview toggle because this
    // is a high-tempo traversal affordance that should be visible during feel
    // tuning without adding another F3 row.
    if developer_tools.show_combat_preview {
        if let Some(ledge) = runtime.ledge_grab.as_ref() {
            let anchor_box = ae::Aabb::new(ledge.contact.anchor, ae::Vec2::splat(5.0));
            let target_box = ae::Aabb::new(ledge.contact.climb_target, player.size * 0.35);
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

    // Blink aim preview. A quick tap blinks a short distance; once the hold
    // crosses the threshold, the engine sets `blink_aiming` and the sandbox
    // enters bullet-time while previewing the longer precision destination.
    if gameplay_active
        && developer_tools.show_blink_preview
        && (controls.blink_held || player.blink_aiming)
    {
        // Use the same temporary collision world that drives player movement.
        // Otherwise the preview can claim a blink is clear while release-time
        // resolution stops on sandbox-only geometry such as the moving platform.
        let blink_world = platforms::world_with_moving_platforms(world, &runtime.moving_platforms);
        let (desired, target) = if player.blink_aiming {
            let desired = player.pos + player.blink_aim_offset;
            let target = ae::blink_destination_to_point(&blink_world, player, desired);
            (desired, target)
        } else {
            let aim = ae::Vec2::new(controls.axis_x, controls.axis_y)
                .normalize_or(ae::Vec2::new(player.facing, 0.0));
            let desired = player.pos + aim * ae::BLINK_DISTANCE;
            let target = ae::blink_destination(&blink_world, player, aim, ae::BLINK_DISTANCE);
            (desired, target)
        };
        let target_center = w2(world, target);
        draw_arrow(gizmos, center, target_center, magenta());
        draw_aabb(
            gizmos,
            world,
            ae::Aabb::new(target, player.size * 0.5),
            magenta(),
        );
        // Raw desired cursor: useful when a hard wall blocks the actual blink.
        // If the raw cursor and safe destination diverge, the blocked segment
        // becomes obvious rather than feeling like the cursor is buggy.
        if (desired - target).length_squared() > 4.0 {
            draw_aabb(
                gizmos,
                world,
                ae::Aabb::new(desired, player.size * 0.35),
                red(),
            );
            gizmos.line_2d(w2(world, desired), target_center, red());
        }
    }

    // Small status ticks above the player: dash and air jump availability.
    let meter_y = body.top() - 18.0;
    let dash_slots = player.abilities.dash_charge_count().max(1) as usize;
    for i in 0..dash_slots {
        let x0 = player.pos.x - 28.0 + i as f32 * 12.0;
        let color = if i < player.dash_charges_available as usize {
            yellow()
        } else {
            gray()
        };
        let a = w2(world, ae::Vec2::new(x0, meter_y));
        let b = w2(world, ae::Vec2::new(x0 + 8.0, meter_y));
        gizmos.line_2d(a, b, color);
    }
    let air_jump_slots = player.abilities.air_jump_count(ae::AIR_JUMPS).max(1) as usize;
    for i in 0..air_jump_slots {
        let x0 = player.pos.x + 6.0 + i as f32 * 11.0;
        let color = if i < player.air_jumps_available as usize {
            cyan()
        } else {
            gray()
        };
        let a = w2(world, ae::Vec2::new(x0, meter_y));
        let b = w2(world, ae::Vec2::new(x0 + 7.0, meter_y));
        gizmos.line_2d(a, b, color);
    }
}

fn draw_moving_platform_debug(gizmos: &mut Gizmos, world: &ae::World, runtime: &SandboxRuntime) {
    for platform in &runtime.moving_platforms {
        let aabb = platform.aabb();
        draw_aabb(gizmos, world, aabb, blue());
        let center = w2(world, aabb.center());
        draw_arrow(gizmos, center, center + BVec2::new(44.0, 0.0), blue());
    }
}

fn draw_health_bars(gizmos: &mut Gizmos, world: &ae::World, runtime: &SandboxRuntime) {
    draw_health_bar(
        gizmos,
        world,
        runtime.player.aabb(),
        runtime.player_health.ratio(),
        cyan(),
    );

    for enemy in &runtime.features.enemies {
        if enemy.alive {
            let color = if enemy.archetype.is_sandbag() {
                orange()
            } else {
                red()
            };
            draw_health_bar(gizmos, world, enemy.aabb(), enemy.health.ratio(), color);
        }
    }
    for boss in &runtime.features.bosses {
        if boss.alive {
            draw_health_bar(gizmos, world, boss.aabb(), boss.health.ratio(), magenta());
        }
    }
    for breakable in &runtime.features.breakables {
        if !breakable.broken() {
            draw_health_bar(
                gizmos,
                world,
                breakable.aabb(),
                breakable.breakable.health.ratio(),
                orange(),
            );
        }
    }
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

fn draw_feature_combat_debug(gizmos: &mut Gizmos, world: &ae::World, runtime: &SandboxRuntime) {
    // Sandbox runtime feature volumes — drawn so that visual drift, bad
    // authored sizes, transparent sprite regions, and attack-reach bugs
    // are all visible under the same F1/F3 debug workflow.
    for hazard in &runtime.features.hazards {
        if hazard.active() {
            draw_aabb(gizmos, world, hazard.aabb(), red());
        }
    }

    for enemy in &runtime.features.enemies {
        if !enemy.alive {
            continue;
        }
        if let Some(body_damage) = enemy.body_damage_aabb() {
            // Always-on hostile body contact volume. This is separate from
            // the player-attack hurtbox used to damage the enemy.
            draw_aabb(gizmos, world, body_damage, red());
        } else {
            draw_aabb(gizmos, world, enemy.aabb(), orange());
        }
        if enemy.attack_windup_timer > 0.0 {
            draw_aabb(gizmos, world, enemy.attack_telegraph_aabb(), orange());
        }
        if enemy.attack_timer > 0.0 {
            draw_aabb(gizmos, world, enemy.attack_aabb(), yellow());
        }
    }

    for boss in &runtime.features.bosses {
        if !boss.alive {
            continue;
        }
        draw_aabb(gizmos, world, boss.body_damage_aabb(), magenta());
        for volume in boss.attack_telegraph_volumes() {
            draw_aabb(gizmos, world, volume, orange());
        }
        for volume in boss.attack_volumes() {
            draw_aabb(gizmos, world, volume, yellow());
        }
    }

    // Breakables — color-keyed by collision behavior so the player can see
    // at a glance whether a tile actually blocks movement. Broken ones are
    // skipped (no live volume).
    for breakable in &runtime.features.breakables {
        if breakable.broken() {
            continue;
        }
        let color = if breakable.breakable.pogo_refresh {
            green()
        } else {
            match breakable.breakable.collision {
                ae::BreakableCollision::Solid => gray(),
                ae::BreakableCollision::OneWayUp => blue(),
                ae::BreakableCollision::None => yellow(),
            }
        };
        draw_aabb(gizmos, world, breakable.aabb(), color);
    }

    // Chests — visible whether opened or not so players can spot interactable
    // bounds even when the open sprite is mostly transparent.
    for chest in &runtime.features.chests {
        let color = if chest.opened { gray() } else { yellow() };
        draw_aabb(gizmos, world, chest.aabb(), color);
    }

    // Pickups — show only while still pickable.
    for pickup in &runtime.features.pickups {
        if !pickup.visible {
            continue;
        }
        draw_aabb(gizmos, world, pickup.aabb(), green());
    }

    // NPCs — interactable but non-combat; use cyan so they don't read as
    // hostile.
    for npc in &runtime.features.npcs {
        draw_aabb(gizmos, world, npc.aabb(), cyan());
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
