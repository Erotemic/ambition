//! Procedural visual effects for the sandbox.
//!
//! Particles are CPU-side Bevy sprite entities for now. Keeping this behind a
//! compact module gives us a later migration seam to GPU particles or Hanabi.

use ambition_engine as ae;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
use std::f32::consts::TAU;

use crate::config::{rgba, world_to_bevy, WORLD_Z_FX};

#[derive(Component)]
pub struct ParticleVisual {
    kind: ParticleKind,
    pos: ae::Vec2,
    vel: ae::Vec2,
    age: f32,
    lifetime: f32,
    radius: f32,
    rgba: [f32; 4],
    gravity: f32,
    drag: f32,
}

#[derive(Component)]
pub struct ImpactVisual {
    pos: ae::Vec2,
    age: f32,
    duration: f32,
    radius: f32,
}

#[derive(Component)]
pub struct SlashPreviewVisual {
    age: f32,
    duration: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleKind {
    Spark,
    Dust,
    Shard,
}

pub fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<crate::GameWorld>,
    mut query: Query<(Entity, &mut ParticleVisual, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut p, mut transform, mut sprite) in &mut query {
        p.age += dt;
        if p.age >= p.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        p.vel.y += p.gravity * dt;
        let drag = (1.0 - p.drag * dt).clamp(0.0, 1.0);
        p.vel *= drag;
        let velocity = p.vel;
        p.pos += velocity * dt;
        let t = (p.age / p.lifetime).clamp(0.0, 1.0);
        let alpha = p.rgba[3] * (1.0 - t);
        let size = match p.kind {
            ParticleKind::Spark => p.radius * (1.0 - 0.35 * t),
            ParticleKind::Dust => p.radius * (1.0 + 0.70 * t),
            ParticleKind::Shard => p.radius * (1.0 - 0.15 * t),
        };
        transform.translation = world_to_bevy(&world.0, p.pos, WORLD_Z_FX);
        sprite.custom_size = Some(BVec2::splat(size.max(0.5)));
        sprite.color = rgba(p.rgba[0], p.rgba[1], p.rgba[2], alpha);
    }
}

pub fn update_impacts(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<crate::GameWorld>,
    mut query: Query<(Entity, &mut ImpactVisual, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut fx, mut transform, mut sprite) in &mut query {
        fx.age += dt;
        if fx.age >= fx.duration {
            commands.entity(entity).despawn();
            continue;
        }
        let t = (fx.age / fx.duration).clamp(0.0, 1.0);
        let radius = fx.radius + 46.0 * t;
        let alpha = 0.82 * (1.0 - t);
        transform.translation = world_to_bevy(&world.0, fx.pos, WORLD_Z_FX + 1.0);
        sprite.custom_size = Some(BVec2::splat(radius));
        sprite.color = Color::srgba(1.0, 1.0, 0.35, alpha);
    }
}

pub fn update_slash_previews(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SlashPreviewVisual, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut preview, mut sprite) in &mut query {
        preview.age += dt;
        if preview.age >= preview.duration {
            commands.entity(entity).despawn();
            continue;
        }
        let alpha = 0.80 * (1.0 - preview.age / preview.duration);
        sprite.color = Color::srgba(1.0, 1.0, 0.35, alpha);
    }
}

pub fn spawn_slash_preview(commands: &mut Commands, world: &ae::World, hitbox: ae::Aabb) {
    let size = hitbox.half * 2.0;
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 1.0, 0.35, 0.80), BVec2::new(size.x, size.y)),
        Transform::from_translation(world_to_bevy(world, hitbox.center, WORLD_Z_FX + 2.0)),
        SlashPreviewVisual { age: 0.0, duration: 0.10 },
    ));
}

pub fn spawn_impact(commands: &mut Commands, world: &ae::World, pos: ae::Vec2) {
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 1.0, 0.35, 0.82), BVec2::splat(12.0)),
        Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX + 1.0)),
        ImpactVisual {
            pos,
            age: 0.0,
            duration: 0.24,
            radius: 12.0,
        },
    ));
}

pub fn spawn_reset_effects(commands: &mut Commands, world: &ae::World, from: ae::Vec2, to: ae::Vec2) {
    // Reset is a teleport-like state transition. Showing both endpoints avoids
    // the ambiguity where a burst at spawn can look like a coordinate bug when
    // the player reset from somewhere else.
    if (from - to).length() > 8.0 {
        spawn_burst(
            commands,
            world,
            from,
            10,
            180.0,
            [0.32, 0.48, 0.70, 0.52],
            ParticleKind::Dust,
        );
    }
    spawn_burst(
        commands,
        world,
        to,
        24,
        280.0,
        [0.55, 0.85, 1.0, 0.90],
        ParticleKind::Spark,
    );
    spawn_impact(commands, world, to);
}

pub fn spawn_burst(
    commands: &mut Commands,
    world: &ae::World,
    pos: ae::Vec2,
    count: usize,
    speed: f32,
    color_rgba: [f32; 4],
    kind: ParticleKind,
) {
    let count = count.max(1);
    for i in 0..count {
        let t = i as f32 / count as f32;
        let wobble = ((i * 37 + 17) as f32).sin() * 0.22;
        let angle = TAU * t + wobble;
        let strength = speed * (0.45 + 0.55 * ((i * 13 + 5) % 11) as f32 / 10.0);
        let vel = ae::Vec2::new(angle.cos() * strength, angle.sin() * strength);
        let radius = 2.0 + 2.5 * ((i * 5 + 1) % 7) as f32 / 6.0;
        let lifetime = 0.22 + 0.16 * ((i * 7 + 3) % 9) as f32 / 8.0;
        commands.spawn((
            Sprite::from_color(rgba(color_rgba[0], color_rgba[1], color_rgba[2], color_rgba[3]), BVec2::splat(radius)),
            Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX)),
            ParticleVisual {
                kind,
                pos,
                vel,
                age: 0.0,
                lifetime,
                radius,
                rgba: color_rgba,
                gravity: match kind {
                    ParticleKind::Spark => 300.0,
                    ParticleKind::Dust => 120.0,
                    ParticleKind::Shard => 650.0,
                },
                drag: match kind {
                    ParticleKind::Spark => 3.4,
                    ParticleKind::Dust => 4.7,
                    ParticleKind::Shard => 1.8,
                },
            },
        ));
    }
}

pub fn spawn_dust(commands: &mut Commands, world: &ae::World, pos: ae::Vec2, facing: f32) {
    for i in 0..6 {
        let lateral = -facing * (75.0 + i as f32 * 18.0);
        let upward = -35.0 - i as f32 * 8.0;
        let radius = 3.5 + i as f32 * 0.35;
        commands.spawn((
            Sprite::from_color(Color::srgba(0.58, 0.62, 0.72, 0.75), BVec2::splat(radius)),
            Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX)),
            ParticleVisual {
                kind: ParticleKind::Dust,
                pos,
                vel: ae::Vec2::new(lateral, upward),
                age: 0.0,
                lifetime: 0.28 + 0.03 * i as f32,
                radius,
                rgba: [0.58, 0.62, 0.72, 0.75],
                gravity: 80.0,
                drag: 4.4,
            },
        ));
    }
}

pub fn spawn_blink_effects(commands: &mut Commands, world: &ae::World, from: ae::Vec2, to: ae::Vec2, precision: bool) {
    let exit_color = if precision { [0.40, 0.34, 1.00, 0.78] } else { [0.24, 0.74, 1.00, 0.68] };
    let entry_color = if precision { [0.92, 0.42, 1.00, 0.92] } else { [0.42, 1.00, 0.92, 0.90] };
    spawn_burst(commands, world, from, if precision { 18 } else { 12 }, 250.0, exit_color, ParticleKind::Spark);
    spawn_burst(commands, world, to, if precision { 28 } else { 18 }, 360.0, entry_color, ParticleKind::Spark);
    spawn_impact(commands, world, to);
}
