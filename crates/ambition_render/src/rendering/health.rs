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

/// One boss health bar: what it shows, over which view, in which rectangle.
/// The bar is rebuilt only when this changes.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BossHealthBarOverlayVisual(DrawnBossBar);

/// What a boss health bar shows, and where.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawnBossBar {
    name: String,
    hp_text: String,
    fill_percent: f32,
    /// The local view it is drawn over; `None` across the whole gameplay area.
    view: Option<Entity>,
    /// That view's rectangle (min, size), when it has one.
    rect: Option<(BVec2, BVec2)>,
}

/// The player-facing boss health bar, at the top of each local view, for
/// the boss being fought in that view's live room; with no local views, the
/// boss being fought across the whole gameplay area.
///
/// The debug `sync_health_overlays` system draws small bars above every
/// actor only when developer health bars are enabled.
///
/// A bar is rebuilt only when what it shows changes (the boss, its health,
/// its view, the fonts) or it is gone: a rebuilt UI is laid out and its text
/// measured again, and it was rebuilt each frame of a boss fight.
#[allow(clippy::too_many_arguments)]
pub fn sync_boss_health_bar_overlay(
    mut commands: Commands,
    overlays: Query<(Entity, &BossHealthBarOverlayVisual)>,
    boss_render: Res<BossRenderIndex>,
    feature_views: Res<FeatureViewIndex>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    primary: ambition_platformer2d_shared_tangle::body::PrimaryLiveRoom,
    views: Query<
        (Entity, &ambition_sim_view::ResolvedViewSubject, Option<&ambition_sim_view::ViewPlacement>),
        With<ambition_sim_view::LocalView>,
    >,
    presentation: Option<Res<ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation>>,
    ui_fonts: Option<Res<UiFonts>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    let session_scope = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    // The boss being fought in `room` (any room for `None`): alive and past
    // its intro. A successor that sleeps until its predecessor falls stands
    // in the room at full health the whole time, and a boss in its intro is
    // not fought yet.
    let fought_in = |room: Option<Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>>,
                     view: Option<Entity>,
                     rect: Option<(BVec2, BVec2)>| {
        boss_render.iter().find_map(|(id, ident)| {
            let row = feature_views.get(id)?;
            if room.is_some_and(|room| room.is_none() || row.room != room) {
                return None;
            }
            (row.alive && ident.fighting).then(|| {
                let health = Health {
                    current: row.hp_current,
                    max: row.hp_max,
                    invulnerable: Default::default(),
                };
                DrawnBossBar {
                    name: ident.name.clone(),
                    hp_text: format!("{} / {}", health.current.max(0), health.max.max(1)),
                    fill_percent: health.ratio().clamp(0.0, 1.0) * 100.0,
                    view,
                    rect,
                }
            })
        })
    };
    let mut wanted: Vec<DrawnBossBar> = Vec::new();
    if session_scope.is_some() {
        if views.is_empty() {
            wanted.extend(fought_in(None, None, None));
        } else {
            for (view, subject, placement) in &views {
                let here = subject.0.and_then(|body| rooms.room_of(body)).or_else(|| primary.get());
                let rect = presentation.as_deref().map(|presentation| {
                    let gameplay = presentation.gameplay_rect;
                    placement.copied().unwrap_or_default().carve(gameplay.min, gameplay.size())
                });
                wanted.extend(fought_in(Some(here), Some(view), rect));
            }
        }
    }
    let fonts_changed = ui_fonts.as_ref().is_some_and(|fonts| fonts.is_changed());
    let mut kept = vec![false; wanted.len()];
    for (entity, BossHealthBarOverlayVisual(drawn)) in &overlays {
        match wanted.iter().position(|want| want == drawn).filter(|at| !kept[*at] && !fonts_changed) {
            Some(at) => kept[at] = true,
            None => commands.entity(entity).despawn(),
        }
    }
    let Some(session_scope) = session_scope else {
        return;
    };
    for (bar, kept) in wanted.into_iter().zip(kept) {
        if !kept {
            spawn_boss_bar(&mut commands, session_scope, ui_fonts.as_deref(), bar);
        }
    }
}

fn spawn_boss_bar(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    ui_fonts: Option<&UiFonts>,
    bar: DrawnBossBar,
) {
    let (left, right, top, panel_width) = match bar.rect {
        Some((min, size)) => (Val::Px(min.x), Val::Auto, Val::Px(min.y + 18.0), (size.x - 32.0).clamp(160.0, 560.0)),
        None => (Val::Px(0.0), Val::Px(0.0), Val::Px(18.0), 560.0),
    };
    let width = bar.rect.map_or(Val::Auto, |(_, size)| Val::Px(size.x));
    let root = (
        Node {
            position_type: PositionType::Absolute,
            left,
            right,
            top,
            width,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::FlexStart,
            ..default()
        },
        ZIndex(34),
        Name::new("Boss Health Overlay Root"),
        BossHealthBarOverlayVisual(bar.clone()),
    );
    let fill_percent = bar.fill_percent;
    let hp_text = bar.hp_text.clone();
    let boss_name = bar.name.as_str();

    let font = |font_size: f32, weight: UiFontWeight| {
        ui_fonts
            .map(|fonts| fonts.text_font(font_size, weight))
            .unwrap_or(TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            })
    };

    commands
        .spawn_session_scoped(session_scope, root)
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(panel_width),
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

/// Run condition of [`sync_health_overlays`]: the developer health bars are
/// on, or a bar is still drawn and is taken away on this run.
pub fn health_bars_are_on_or_drawn(
    dev_state: Res<ambition_dev_tools::DeveloperRuntimeState>,
    developer_tools: Res<ambition_dev_tools::dev_tools::DeveloperTools>,
    overlays: Query<(), With<HealthOverlayVisual>>,
) -> bool {
    (dev_state.debug_enabled() && developer_tools.show_health_bars) || !overlays.is_empty()
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
            material: Default::default(),
            alive: true,
            hit_flash_secs: 0.0,
            parry_flash_secs: 0.0,
            hp_current: 1,
            hp_max: 2,
            training_dummy: false,
            armored: false,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
            sprite_offset: None,
            room: None,
        }
    }

    /// The boss bar is kept while the boss and its health stay, and rebuilt
    /// when its health changes: it was despawned and spawned each frame of a
    /// boss fight, and a new UI is laid out and measured again.
    #[test]
    fn the_boss_bar_is_kept_while_the_fight_stands_still() {
        let mut app = App::new();
        let mut active = ActiveSessionScope::default();
        active.begin();
        app.insert_resource(active);
        let boss = |hp: i32| {
            let mut view = crate_at(ae::Vec2::new(100.0, 100.0));
            view.hp_current = hp;
            view.hp_max = 10;
            ambition_sim_view::FeatureViewIndex::from_rows([("boss".to_string(), view)])
        };
        app.insert_resource(boss(10));
        app.insert_resource(ambition_sim_view::BossRenderIndex::from_rows([(
            "boss".to_string(),
            ambition_sim_view::BossRenderView {
                name: "Smirking Behemoth".to_string(),
                behavior_id: "behemoth".to_string(),
                title: None,
                stage: Default::default(),
                awake: true,
                fighting: true,
                finish: 0.0,
            },
        )]));
        app.add_systems(Update, sync_boss_health_bar_overlay);
        app.update();
        let roots = |app: &mut App| {
            app.world_mut()
                .query_filtered::<Entity, With<BossHealthBarOverlayVisual>>()
                .iter(app.world())
                .collect::<Vec<_>>()
        };
        let first = roots(&mut app);
        assert_eq!(first.len(), 1, "a boss fought, one bar");
        app.update();
        assert_eq!(roots(&mut app), first, "a fight standing still keeps its bar");
        app.insert_resource(boss(7));
        app.update();
        let hit = roots(&mut app);
        assert_eq!(hit.len(), 1, "still one bar");
        assert_ne!(hit, first, "a blow redraws it");
    }

    /// Each local view draws the bar of the boss fought in its own live room,
    /// over its own half: the bar was drawn once, top-centre across both
    /// views, over the second player's HUD (seen in the two-view captures).
    #[test]
    fn each_view_draws_the_bar_of_the_boss_fought_in_its_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::{spawn_live_room, InRoomInstance, LiveRoomInstance};
        use ambition_sim_view::{LocalView, LocalViewId, ResolvedViewSubject, ViewPlacement};
        let geometry = |name: &str| {
            ambition_platformer2d_core::RoomGeometry(ae::World::new(name, ae::Vec2::new(800.0, 600.0), ae::Vec2::new(16.0, 16.0), Vec::new()))
        };
        let mut app = App::new();
        let mut active = ActiveSessionScope::default();
        active.begin();
        app.insert_resource(active);
        let (first, second) = (LiveRoomInstance::ACTIVATION, LiveRoomInstance::from_ordinal(1));
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), geometry("hall"));
        spawn_live_room(app.world_mut(), second, geometry("arena"));
        let alice = app.world_mut().spawn(InRoomInstance(first)).id();
        let bob = app.world_mut().spawn(InRoomInstance(second)).id();
        let views = [(0, alice), (1, bob)].map(|(index, body)| {
            app.world_mut()
                .spawn((LocalView, LocalViewId(index as u8), ViewPlacement::column(index, 2), ResolvedViewSubject(Some(body))))
                .id()
        });
        let rows = |fought: [(&str, LiveRoomInstance, bool); 2]| {
            let features = ambition_sim_view::FeatureViewIndex::from_rows(fought.map(|(id, room, _)| {
                let mut view = crate_at(ae::Vec2::new(100.0, 100.0));
                view.hp_current = 5;
                view.hp_max = 10;
                view.room = Some(room);
                (id.to_string(), view)
            }));
            let bosses = ambition_sim_view::BossRenderIndex::from_rows(fought.map(|(id, _, fighting)| {
                (
                    id.to_string(),
                    ambition_sim_view::BossRenderView {
                        name: id.to_string(),
                        behavior_id: id.to_string(),
                        title: None,
                        stage: Default::default(),
                        awake: true,
                        fighting,
                        finish: 0.0,
                    },
                )
            }));
            (features, bosses)
        };
        app.add_systems(Update, sync_boss_health_bar_overlay);
        let shown = |app: &mut App| {
            let mut shown: Vec<(Option<Entity>, String)> = app
                .world_mut()
                .query::<&BossHealthBarOverlayVisual>()
                .iter(app.world())
                .map(|BossHealthBarOverlayVisual(bar)| (bar.view, bar.name.clone()))
                .collect();
            shown.sort();
            shown
        };

        let (features, bosses) = rows([("warden", first, false), ("trex", second, true)]);
        app.insert_resource(features);
        app.insert_resource(bosses);
        app.update();
        assert_eq!(shown(&mut app), vec![(Some(views[1]), "trex".to_string())], "only the second view's room has a boss fought");

        let (features, bosses) = rows([("warden", first, true), ("trex", second, true)]);
        app.insert_resource(features);
        app.insert_resource(bosses);
        app.update();
        let mut both = vec![(Some(views[0]), "warden".to_string()), (Some(views[1]), "trex".to_string())];
        both.sort();
        assert_eq!(shown(&mut app), both, "each view draws its own room's boss");
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
