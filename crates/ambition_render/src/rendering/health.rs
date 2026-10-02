//! Optional debug health-bar overlay above every actor with a `Health`
//! resource. Toggled by `DeveloperTools::show_health_bars`.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::HealthOverlayVisual;
use crate::ui_fonts::{UiFontWeight, UiFonts};
use ambition_characters::actor::Health;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::{ActorRenderIndex, BossRenderIndex, FeatureViewIndex};

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
    boss_render: Res<BossRenderIndex>,
    feature_views: Res<FeatureViewIndex>,
    ui_fonts: Option<Res<UiFonts>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }

    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };

    let Some((health, boss_name)) = boss_render.iter().find_map(|(id, ident)| {
        let view = feature_views.get(id)?;
        if view.alive {
            Some((
                Health {
                    current: view.hp_current,
                    max: view.hp_max,
                    invulnerable: Default::default(),
                },
                ident.name.clone(),
            ))
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
                font_size: FontSize::Px(font_size),
                ..default()
            })
    };

    commands
        .spawn_session_scoped(
            session_scope,
            (
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
            ),
        )
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
    // Each bar is drawn in the live room of the body it measures, by that
    // room's geometry: the player's own room, or the room a feature's sprite
    // is stamped with (V2b).
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    sprites: Query<(Entity, &super::primitives::FeatureVisual)>,
    dev_state: Res<ambition_dev_tools::DeveloperRuntimeState>,
    active_session: Option<Res<ActiveSessionScope>>,
    developer_tools: Res<ambition_dev_tools::dev_tools::DeveloperTools>,
    overlays: Query<Entity, With<HealthOverlayVisual>>,
    player: Query<
        (
            Entity,
            &ambition_sim_view::BodyPoseView,
            Option<&ambition_sim_view::PresentedPose>,
        ),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    feature_views: Res<FeatureViewIndex>,
    actor_render: Res<ActorRenderIndex>,
    boss_render: Res<BossRenderIndex>,
    boss_frames: Res<ambition_sim_view::BossFrameIndex>,
) {
    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }

    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };

    if !dev_state.debug_enabled() || !developer_tools.show_health_bars {
        return;
    }

    let room_by_id: std::collections::HashMap<&str, ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance> =
        sprites
            .iter()
            .filter_map(|(entity, sprite)| Some((sprite.id.as_str(), rooms.room_of(entity)?)))
            .collect();
    // A body whose live room cannot be told gets no bar.
    let placed = |room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>| {
        room.and_then(|room| Some((session_scope.in_room(Some(room)), &rooms.in_room(room)?.0)))
    };

    if let Some((pose, presented, (scope, world))) = player
        .single()
        .ok()
        .and_then(|(body, pose, presented)| Some((pose, presented, placed(rooms.room_of(body))?)))
    {
        spawn_health_overlay(
            &mut commands,
            scope,
            world,
            "player_robot_v3",
            ae::Aabb::new(
                ambition_sim_view::presented_pose::draw_pos(pose, presented),
                pose.size * 0.5,
            ),
            Health {
                current: pose.hp_current,
                max: pose.hp_max,
                invulnerable: Default::default(),
            },
            Color::srgba(0.30, 0.92, 1.00, 0.96),
        );
    }

    for (id, view) in feature_views.iter() {
        let Some((scope, world)) = placed(room_by_id.get(id).copied()) else {
            continue;
        };
        let hp = Health {
            current: view.hp_current,
            max: view.hp_max,
            invulnerable: Default::default(),
        };
        match view.kind {
            FeatureVisualKind::Actor => {
                // Bosses share the Actor view kind; their bar anchors to the
                // combat AABB (`BossFrameIndex`) and draws pink.
                if let Some(frame) = boss_frames.get(id) {
                    if view.alive {
                        let label = boss_render.get(id).map(|b| b.name.as_str()).unwrap_or(id);
                        spawn_health_overlay(
                            &mut commands,
                            scope,
                            world,
                            label,
                            frame.aabb,
                            hp,
                            Color::srgba(1.00, 0.32, 0.92, 0.96),
                        );
                    }
                } else if view.fighting && view.alive {
                    let color = if view.training_dummy {
                        Color::srgba(1.00, 0.66, 0.24, 0.96)
                    } else {
                        Color::srgba(1.00, 0.20, 0.22, 0.96)
                    };
                    let label = actor_render.get(id).map(|a| a.name.as_str()).unwrap_or(id);
                    spawn_health_overlay(
                        &mut commands,
                        scope,
                        world,
                        label,
                        ae::Aabb::new(view.pos, view.size * 0.5),
                        hp,
                        color,
                    );
                }
            }
            FeatureVisualKind::Breakable => {
                if view.alive {
                    spawn_health_overlay(
                        &mut commands,
                        scope,
                        world,
                        id,
                        ae::Aabb::new(view.pos, view.size * 0.5),
                        hp,
                        Color::srgba(1.00, 0.72, 0.24, 0.96),
                    );
                }
            }
            _ => {}
        }
    }
}

fn spawn_health_overlay(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    name: &str,
    aabb: ae::Aabb,
    health: Health,
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

    commands.spawn_session_scoped(
        session_scope,
        (
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
        ),
    );
    if fill_w > 0.5 {
        commands.spawn_session_scoped(
            session_scope,
            (
                Sprite::from_color(fill_color, BVec2::new(fill_w, height)),
                Transform::from_translation(world_to_bevy(
                    world,
                    ae::Vec2::new(left + fill_w * 0.5, y),
                    WORLD_Z_PLAYER + 13.0,
                )),
                Name::new(format!("Health bar fill: {name}")),
                HealthOverlayVisual,
            ),
        );
    }
    commands.spawn_session_scoped(
        session_scope,
        (
            Text2d::new(text),
            TextFont {
                font_size: FontSize::Px(11.0),
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
        ),
    );
}

#[cfg(test)]
mod room_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_live_room_component, spawn_live_room, InRoomInstance, LiveRoomInstance,
    };

    fn crate_at(at: ae::Vec2) -> ambition_sim_view::FeatureView {
        ambition_sim_view::FeatureView {
            pos: at,
            size: ae::Vec2::new(20.0, 20.0),
            kind: FeatureVisualKind::Breakable,
            visible: true,
            submerged: false,
            wire_anchor: None,
            grab_reach: None,
            line_anchor: None,
            limb_host: None,
            depth_plane: ae::DepthPlane::PLAYABLE,
            flash: false,
            breakable_state: None,
            chest_opened: false,
            fighting: false,
            switch_on: false,
            rotation_rad: 0.0,
            alive: true,
            hit_flash_secs: 0.0,
            parry_flash_secs: 0.0,
            hp_current: 1,
            hp_max: 2,
            training_dummy: false,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
            sprite_offset: None,
        }
    }

    /// Each health bar is drawn in the live room of the body it measures
    /// (view half, cut V2k): the room the body's sprite is stamped with.
    /// Two live rooms of different sizes, a breakable in each at one
    /// simulation position: each bar has its body's room and that room's
    /// position. A body with no sprite, so no room, gets no bar.
    #[test]
    fn each_health_bar_is_drawn_in_its_body_s_own_live_room() {
        let world_of = |size: ae::Vec2| ae::World::new("bar room", size, ae::Vec2::new(40.0, 40.0), Vec::new());
        let (big, small) = (ae::Vec2::new(800.0, 600.0), ae::Vec2::new(400.0, 300.0));
        let mut app = App::new();
        insert_live_room_component(app.world_mut(), ambition_platformer2d_core::RoomGeometry(world_of(big)));
        let second = LiveRoomInstance::ACTIVATION.next();
        spawn_live_room(app.world_mut(), second, ambition_platformer2d_core::RoomGeometry(world_of(small)));
        let at = ae::Vec2::new(100.0, 200.0);
        app.insert_resource(FeatureViewIndex::from_rows([
            ("crate_home".to_string(), crate_at(at)),
            ("crate_away".to_string(), crate_at(at)),
            ("crate_nowhere".to_string(), crate_at(at)),
        ]));
        for (id, room) in [("crate_home", LiveRoomInstance::ACTIVATION), ("crate_away", second)] {
            app.world_mut()
                .spawn((super::super::primitives::FeatureVisual { id: id.to_string() }, InRoomInstance(room)));
        }
        let mut dev = ambition_dev_tools::DeveloperRuntimeState::default();
        dev.debug = true;
        app.insert_resource(dev);
        let mut tools = ambition_dev_tools::dev_tools::DeveloperTools::default();
        tools.show_health_bars = true;
        app.insert_resource(tools);
        app.init_resource::<ActorRenderIndex>();
        app.init_resource::<BossRenderIndex>();
        app.init_resource::<ambition_sim_view::BossFrameIndex>();
        app.add_systems(Update, sync_health_overlays);
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<(&Name, &Transform, Option<&InRoomInstance>)>();
        let mut bars: Vec<(String, Option<u32>, (i32, i32))> = q
            .iter(world)
            .filter(|(name, ..)| name.as_str().starts_with("Health bar bg: "))
            .map(|(name, transform, stamp)| {
                (
                    name.as_str().trim_start_matches("Health bar bg: ").to_string(),
                    stamp.map(|stamp| stamp.0.ordinal()),
                    (transform.translation.x as i32, transform.translation.y as i32),
                )
            })
            .collect();
        bars.sort();
        // The bar sits 26 px above the box top: y = 200 - 10 - 26 = 164.
        let flipped = |size: ae::Vec2| ((100.0 - size.x * 0.5) as i32, (size.y * 0.5 - 164.0) as i32);
        assert_eq!(
            bars,
            vec![
                ("crate_away".to_string(), Some(second.ordinal()), flipped(small)),
                ("crate_home".to_string(), Some(LiveRoomInstance::ACTIVATION.ordinal()), flipped(big)),
            ],
            "(body, room, position) of each bar: each must be placed by its body's live room and stamped with it"
        );
    }
}
