//! Optional debug health-bar overlay rendered above every actor with
//! a `Health` resource. Toggled via
//! `DeveloperTools::show_health_bars`.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::HealthOverlayVisual;
use crate::config::{world_to_bevy, WORLD_Z_PLAYER};
use crate::features::{ActorRuntime, BossFeature, BreakableFeature, FeatureAabb, FeatureName};

pub fn sync_health_overlays(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    developer_tools: Res<crate::dev_tools::DeveloperTools>,
    overlays: Query<Entity, With<HealthOverlayVisual>>,
    ecs_breakables: Query<(&FeatureName, &FeatureAabb, &BreakableFeature)>,
    ecs_actors: Query<(&FeatureName, &FeatureAabb, &ActorRuntime)>,
    ecs_bosses: Query<(&FeatureName, &BossFeature)>,
) {
    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }

    if !runtime.debug_enabled() || !developer_tools.show_health_bars {
        return;
    }

    spawn_health_overlay(
        &mut commands,
        &world.0,
        "player",
        runtime.player.aabb(),
        runtime.player_health,
        Color::srgba(0.30, 0.92, 1.00, 0.96),
    );

    for (name, aabb, actor) in &ecs_actors {
        if let ActorRuntime::Hostile(enemy) = actor {
            if enemy.alive {
                let color = if enemy.archetype.is_sandbag() {
                    Color::srgba(1.00, 0.66, 0.24, 0.96)
                } else {
                    Color::srgba(1.00, 0.20, 0.22, 0.96)
                };
                spawn_health_overlay(
                    &mut commands,
                    &world.0,
                    name.0.as_str(),
                    aabb.aabb(),
                    enemy.health,
                    color,
                );
            }
        }
    }
    for (name, boss) in &ecs_bosses {
        let boss = &boss.boss;
        if boss.alive {
            spawn_health_overlay(
                &mut commands,
                &world.0,
                name.0.as_str(),
                boss.aabb(),
                boss.health,
                Color::srgba(1.00, 0.32, 0.92, 0.96),
            );
        }
    }
    for (name, aabb, breakable) in &ecs_breakables {
        if !breakable.broken() {
            spawn_health_overlay(
                &mut commands,
                &world.0,
                name.0.as_str(),
                aabb.aabb(),
                breakable.breakable.health,
                Color::srgba(1.00, 0.72, 0.24, 0.96),
            );
        }
    }
}

fn spawn_health_overlay(
    commands: &mut Commands,
    world: &ae::World,
    name: &str,
    aabb: ae::Aabb,
    health: ae::Health,
    fill_color: Color,
) {
    let width = aabb.width().max(56.0);
    let height = 7.0;
    let y = aabb.top() - 26.0;
    let center_x = aabb.center().x;
    let left = center_x - width * 0.5;
    let ratio = health.ratio().clamp(0.0, 1.0);
    let fill_w = width * ratio;
    let text = format!("{}/{}", health.current.max(0), health.max);

    commands.spawn((
        Sprite::from_color(
            Color::srgba(0.02, 0.03, 0.05, 0.86),
            BVec2::new(width + 5.0, height + 5.0),
        ),
        Transform::from_translation(world_to_bevy(
            world,
            ae::Vec2::new(center_x, y),
            WORLD_Z_PLAYER + 12.0,
        )),
        Name::new(format!("Health bar bg: {name}")),
        HealthOverlayVisual,
    ));
    if fill_w > 0.5 {
        commands.spawn((
            Sprite::from_color(fill_color, BVec2::new(fill_w, height)),
            Transform::from_translation(world_to_bevy(
                world,
                ae::Vec2::new(left + fill_w * 0.5, y),
                WORLD_Z_PLAYER + 13.0,
            )),
            Name::new(format!("Health bar fill: {name}")),
            HealthOverlayVisual,
        ));
    }
    commands.spawn((
        Text2d::new(text),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgba(0.96, 0.98, 1.0, 0.98)),
        Transform::from_translation(world_to_bevy(
            world,
            ae::Vec2::new(center_x, y - 13.0),
            WORLD_Z_PLAYER + 14.0,
        )),
        Name::new(format!("Health label: {name}")),
        HealthOverlayVisual,
    ));
}
