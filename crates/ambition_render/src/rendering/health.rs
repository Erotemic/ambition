//! Optional debug health-bar overlay rendered above every actor with
//! a `Health` resource. Toggled via
//! `DeveloperTools::show_health_bars`.

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::HealthOverlayVisual;
use crate::ui_fonts::{UiFontWeight, UiFonts};
use ambition_engine_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_characters::actor::{BodyCombat, BodyHealth};
use ambition_gameplay_core::features::{
    ActorDisposition, BossClusterRef, BossConfig, BreakableFeature, CenteredAabb, FeatureName,
};

#[derive(Component)]
pub struct BossHealthBarOverlayVisual;

/// Always-on top-center boss health overlay.
///
/// The debug `sync_health_overlays` system draws small bars above every
/// actor only when developer health bars are enabled. This system is the
/// player-facing boss UI: if a live boss exists in the active room, show
/// the boss name and HP fraction in the top-center HUD overlay.
pub fn sync_boss_health_bar_overlay(
    mut commands: Commands,
    overlays: Query<Entity, With<BossHealthBarOverlayVisual>>,
    bosses: Query<(BossClusterRef, &BodyHealth)>,
    ui_fonts: Option<Res<UiFonts>>,
) {
    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }

    let Some((health, boss_name)) = bosses.iter().find_map(|(item, health)| {
        let boss = item.as_boss_ref();
        if health.alive() {
            Some((health.health, boss.config.name.clone()))
        } else {
            None
        }
    }) else {
        return;
    };

    let ratio = health.ratio().clamp(0.0, 1.0);
    let fill_percent = ratio * 100.0;
    let hp_text = format!("{} / {}", health.current.max(0), health.max.max(1));
    let boss_name = boss_name.as_str();

    let font = |font_size: f32, weight: UiFontWeight| {
        ui_fonts
            .as_deref()
            .map(|fonts| fonts.text_font(font_size, weight))
            .unwrap_or(TextFont {
                font_size,
                ..default()
            })
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(18.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                ..default()
            },
            ZIndex(34),
            Name::new("Boss Health Overlay Root"),
            BossHealthBarOverlayVisual,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(560.0),
                    min_height: Val::Px(58.0),
                    padding: UiRect::axes(Val::Px(18.0), Val::Px(8.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(18.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.025, 0.018, 0.030, 0.82)),
                BorderColor::all(Color::srgba(0.88, 0.64, 0.95, 0.86)),
                Name::new(format!("Boss Health Panel: {boss_name}")),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new(boss_name.to_string()),
                    font(19.0, UiFontWeight::Semibold),
                    TextColor(Color::srgba(0.98, 0.91, 1.00, 1.0)),
                ));
                panel
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(16.0),
                            border: UiRect::all(Val::Px(2.0)),
                            border_radius: BorderRadius::all(Val::Px(9.0)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.05, 0.03, 0.06, 0.96)),
                        BorderColor::all(Color::srgba(0.18, 0.11, 0.20, 1.0)),
                        Name::new(format!("Boss Health Track: {boss_name}")),
                    ))
                    .with_children(|track| {
                        track.spawn((
                            Node {
                                width: Val::Percent(fill_percent),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.86, 0.10, 0.24, 0.96)),
                            Name::new(format!("Boss Health Fill: {boss_name}")),
                        ));
                    });
                panel.spawn((
                    Text::new(hp_text),
                    font(12.0, UiFontWeight::Regular),
                    TextColor(Color::srgba(0.90, 0.84, 0.95, 0.92)),
                ));
            });
        });
}

pub fn sync_health_overlays(
    mut commands: Commands,
    world: Res<ambition_gameplay_core::RoomGeometry>,
    dev_state: Res<ambition_gameplay_core::SandboxDevState>,
    developer_tools: Res<ambition_gameplay_core::dev::dev_tools::DeveloperTools>,
    overlays: Query<Entity, With<HealthOverlayVisual>>,
    player: Query<
        (
            &ambition_platformer_primitives::body::BodyKinematics,
            &ambition_characters::actor::BodyHealth,
        ),
        ambition_gameplay_core::actor::PrimaryPlayerOnly,
    >,
    ecs_breakables: Query<(&FeatureName, &CenteredAabb, &BreakableFeature)>,
    ecs_actors: Query<
        (
            &FeatureName,
            &CenteredAabb,
            &ActorDisposition,
            &BodyHealth,
            &BodyCombat,
        ),
        Without<BossConfig>,
    >,
    ecs_bosses: Query<(&FeatureName, BossClusterRef, &BodyHealth)>,
) {
    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }

    if !dev_state.debug_enabled() || !developer_tools.show_health_bars {
        return;
    }

    if let Ok((body, health)) = player.single() {
        spawn_health_overlay(
            &mut commands,
            &world.0,
            "player",
            body.aabb(),
            health.health,
            Color::srgba(0.30, 0.92, 1.00, 0.96),
        );
    }

    for (name, aabb, disposition, health, combat) in &ecs_actors {
        if disposition.is_hostile() && combat.alive {
            let color = if combat.training_dummy {
                Color::srgba(1.00, 0.66, 0.24, 0.96)
            } else {
                Color::srgba(1.00, 0.20, 0.22, 0.96)
            };
            spawn_health_overlay(
                &mut commands,
                &world.0,
                name.0.as_str(),
                aabb.aabb(),
                health.health,
                color,
            );
        }
    }
    for (name, item, health) in &ecs_bosses {
        let boss = item.as_boss_ref();
        if health.alive() {
            spawn_health_overlay(
                &mut commands,
                &world.0,
                name.0.as_str(),
                boss.aabb(),
                health.health,
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
    health: ambition_characters::actor::Health,
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
