//! Debug drawing for the Bevy sandbox backend.
//!
//! These overlays intentionally live in the Bevy adapter layer. The movement
//! engine exposes simulation state; this module decides how to visualize that
//! state for tuning and feel work.

use ambition_engine as ae;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::config::world_to_bevy;
use crate::dummies::DummyKind;
use crate::input::ControlFrame;
use crate::{slash_hitbox, GameWorld, SandboxRuntime};

fn cyan() -> Color { Color::srgba(0.30, 0.92, 1.00, 0.92) }
fn blue() -> Color { Color::srgba(0.30, 0.55, 1.00, 0.90) }
fn green() -> Color { Color::srgba(0.25, 1.00, 0.45, 0.90) }
fn yellow() -> Color { Color::srgba(1.00, 0.92, 0.22, 0.95) }
fn orange() -> Color { Color::srgba(1.00, 0.55, 0.16, 0.90) }
fn magenta() -> Color { Color::srgba(1.00, 0.32, 0.92, 0.88) }
fn red() -> Color { Color::srgba(1.00, 0.18, 0.22, 0.82) }
fn white_dim() -> Color { Color::srgba(0.90, 0.95, 1.00, 0.40) }
fn gray() -> Color { Color::srgba(0.62, 0.66, 0.75, 0.46) }

pub fn draw_debug_overlay(
    mut gizmos: Gizmos,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<GameWorld>,
    runtime: Res<SandboxRuntime>,
) {
    if !runtime.debug_enabled() {
        return;
    }

    let world = &world.0;
    draw_room_bounds(&mut gizmos, world);
    draw_rebound_vectors(&mut gizmos, world);
    draw_player_debug(&mut gizmos, world, &runtime, &keys);
    draw_dummy_debug(&mut gizmos, world, &runtime);
}

fn draw_room_bounds(gizmos: &mut Gizmos, world: &ae::World) {
    let room = ae::Aabb::from_min_size(ae::Vec2::ZERO, world.size);
    draw_aabb(gizmos, world, room, white_dim());
}

fn draw_player_debug(
    gizmos: &mut Gizmos,
    world: &ae::World,
    runtime: &SandboxRuntime,
    keys: &ButtonInput<KeyCode>,
) {
    let player = &runtime.player;
    let body = player.aabb();
    draw_aabb(gizmos, world, body, cyan());

    let center = w2(world, player.pos);

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
        let side_x = if player.wall_normal_x < 0.0 { body.left() } else { body.right() };
        let side = w2(world, ae::Vec2::new(side_x, player.pos.y));
        draw_arrow(gizmos, side, side + BVec2::new(player.wall_normal_x * 48.0, 0.0), green());
    }

    // Show the currently implied attack box while the attack key is held. This
    // brings back the old raw collision-box tuning view without requiring an
    // actual attack event every frame.
    let preset = runtime.preset();
    let controls = ControlFrame::read(keys, preset);
    let dedicated_pogo_held = preset.actions.dedicated_pogo.map(|key| keys.pressed(key)).unwrap_or(false);
    if keys.pressed(preset.actions.attack) || dedicated_pogo_held {
        let hitbox = slash_hitbox(player, controls.axis_y, dedicated_pogo_held || controls.pogo_pressed);
        draw_aabb(gizmos, world, hitbox, yellow());
    }

    // Small status ticks above the player: dash and air jump availability.
    let meter_y = body.top() - 18.0;
    let dash_color = if player.dash_available { yellow() } else { gray() };
    let dash_a = w2(world, ae::Vec2::new(player.pos.x - 24.0, meter_y));
    let dash_b = w2(world, ae::Vec2::new(player.pos.x - 4.0, meter_y));
    gizmos.line_2d(dash_a, dash_b, dash_color);
    for i in 0..2 {
        let x0 = player.pos.x + 6.0 + i as f32 * 11.0;
        let color = if i < player.air_jumps_available as usize { cyan() } else { gray() };
        let a = w2(world, ae::Vec2::new(x0, meter_y));
        let b = w2(world, ae::Vec2::new(x0 + 7.0, meter_y));
        gizmos.line_2d(a, b, color);
    }
}

fn draw_dummy_debug(gizmos: &mut Gizmos, world: &ae::World, runtime: &SandboxRuntime) {
    for dummy in &runtime.dummies {
        let color = match dummy.kind {
            DummyKind::InfiniteSandbag => orange(),
            DummyKind::FiniteRespawner => magenta(),
        };
        draw_aabb(gizmos, world, dummy.aabb(), color);

        if dummy.kind == DummyKind::FiniteRespawner {
            let ratio = if dummy.max_hp > 0 {
                (dummy.hp.max(0) as f32 / dummy.max_hp as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let y = dummy.pos.y - dummy.size.y * 0.5 - 16.0;
            let left = dummy.pos.x - dummy.size.x * 0.5;
            let right = dummy.pos.x + dummy.size.x * 0.5;
            let full_a = w2(world, ae::Vec2::new(left, y));
            let full_b = w2(world, ae::Vec2::new(right, y));
            gizmos.line_2d(full_a, full_b, gray());
            let hp_b = w2(world, ae::Vec2::new(left + (right - left) * ratio, y));
            gizmos.line_2d(full_a, hp_b, red());
        }
    }
}

fn draw_rebound_vectors(gizmos: &mut Gizmos, world: &ae::World) {
    for block in &world.blocks {
        let ae::BlockKind::Rebound { impulse } = block.kind else {
            continue;
        };
        draw_aabb(gizmos, world, block.aabb, orange());
        let start = w2(world, block.aabb.center);
        let direction = impulse.normalized_or(ae::Vec2::new(0.0, -1.0));
        let end = start + engine_delta_to_bevy(direction * 70.0);
        draw_arrow(gizmos, start, end, orange());
    }
}

fn draw_aabb(gizmos: &mut Gizmos, world: &ae::World, aabb: ae::Aabb, color: Color) {
    let min = aabb.min();
    let max = aabb.max();
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
