//! Optional debug health-bar overlay above every actor with a `Health`
//! resource. Toggled by `DeveloperTools::show_health_bars`.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{FeatureVisual, WorldSprite, WorldText};
use super::retained_entities::RetainedEntities;
use crate::ui_fonts::{UiFontWeight, UiFonts};
use ambition_characters::actor::Health;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, LiveRoomOf, SessionScopeId, SessionSpawnScope,
    SpawnSessionScopedExt,
};
use ambition_sim_view::{ActorRenderIndex, BossRenderIndex, FeatureViewIndex};
use bevy::ecs::system::SystemParam;
use std::collections::HashSet;

/// One boss health bar: what it shows, over which view, in which rectangle.
/// The bar is rebuilt only when this changes.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BossHealthBarOverlayVisual(DrawnBossBar);

#[derive(Component)]
#[require(WorldSprite)]
pub struct HealthOverlayVisual {
    fill: Entity,
    label: Entity,
    hp: [i32; 2],
}

type HealthBarKey = (Entity, LiveRoomInstance, Option<SessionScopeId>);

#[derive(SystemParam)]
pub struct HealthBarProducts<'w, 's> {
    commands: Commands<'w, 's>,
    retained: Local<'s, RetainedEntities<HealthBarKey>>,
    shown: Local<'s, HashSet<HealthBarKey>>,
    bars: Query<'w, 's, (
        &'static mut HealthOverlayVisual,
        &'static mut Transform,
        &'static mut Sprite,
    )>,
    fills: Query<'w, 's, (
        &'static mut Transform,
        &'static mut Sprite,
        &'static mut Visibility,
    ), Without<HealthOverlayVisual>>,
    labels: Query<'w, 's, &'static mut Text2d>,
}

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
    /// Its place in the view's stack: two bosses fought at once (GNU-ton
    /// and GNU2) each have a bar, one under the other.
    slot: usize,
}

/// A boss panel's height, in UI pixels: its padding, the name line, the
/// track, the HP line and its border, with room to spare. Fixed, so a stack
/// of bars is laid out from a height the panel cannot exceed.
const BAR_PANEL_HEIGHT: f32 = 92.0;
/// How far each bar of a stack is under the one before it: a panel and a gap.
const BAR_STACK_STEP: f32 = BAR_PANEL_HEIGHT + 8.0;

/// The player-facing boss health bars, at the top of each local view, one for
/// each boss being fought in that view's live room (in the order of their
/// ids, so a stack keeps its order); with no local views, the bosses being
/// fought across the whole gameplay area.
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
    // The bosses being fought in `room` (any room for `None`): alive and
    // past their intro. A successor that sleeps until its predecessor falls
    // stands in the room at full health the whole time, and a boss in its
    // intro is not fought yet.
    let fought_in = |room: Option<Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>>,
                     view: Option<Entity>,
                     rect: Option<(BVec2, BVec2)>| {
        let mut fought: Vec<(&str, DrawnBossBar)> = boss_render.iter().filter_map(|(id, ident)| {
            let row = feature_views.get_in(id, ident.room)?;
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
                    slot: 0,
                }
            }).map(|bar| (id, bar))
        }).collect();
        fought.sort_unstable_by(|a, b| a.0.cmp(b.0));
        fought.into_iter().enumerate().map(|(slot, (_, bar))| DrawnBossBar { slot, ..bar }).collect::<Vec<_>>()
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
    let below = bar.slot as f32 * BAR_STACK_STEP;
    let (left, right, top, panel_width) = match bar.rect {
        Some((min, size)) => (Val::Px(min.x), Val::Auto, Val::Px(min.y + 18.0 + below), (size.x - 32.0).clamp(160.0, 560.0)),
        None => (Val::Px(0.0), Val::Px(0.0), Val::Px(18.0 + below), 560.0),
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
                    height: Val::Px(BAR_PANEL_HEIGHT),
                    overflow: Overflow::clip(),
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
    mut bars: HealthBarProducts,
    rooms: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    sprites: Query<(Entity, &FeatureVisual)>,
    dev_state: Res<ambition_dev_tools::DeveloperRuntimeState>,
    active_session: Option<Res<ActiveSessionScope>>,
    developer_tools: Res<ambition_dev_tools::dev_tools::DeveloperTools>,
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
    let scope = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    let Some(scope) = scope.filter(|_| dev_state.debug_enabled() && developer_tools.show_health_bars)
    else {
        bars.finish();
        return;
    };
    let placed = |entity| {
        let room = rooms.room_of(entity)?;
        Some(((entity, room, scope.id()), &rooms.in_room(room)?.0))
    };
    if let Some((pose, presented, (key, world))) = player
        .single()
        .ok()
        .and_then(|(body, pose, presented)| Some((pose, presented, placed(body)?)))
    {
        bars.show(
            key,
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

    for (entity, sprite) in &sprites {
        let Some((key, world)) = placed(entity) else {
            continue;
        };
        let id = sprite.id.as_str();
        let Some(view) = feature_views.get_in(id, Some(key.1)).filter(|view| view.alive) else {
            continue;
        };
        let bounds = ae::Aabb::new(view.pos, view.size * 0.5);
        let (name, bounds, color) = match view.kind {
            FeatureVisualKind::Actor => {
                if let Some(frame) = boss_frames.get_in(id, Some(key.1)) {
                    let label = boss_render.get_in(id, Some(key.1)).map_or(id, |boss| boss.name.as_str());
                    (label, frame.aabb, Color::srgba(1.00, 0.32, 0.92, 0.96))
                } else if view.fighting {
                    let label = actor_render.get(id).map_or(id, |actor| actor.name.as_str());
                    let color = if view.training_dummy {
                        Color::srgba(1.00, 0.66, 0.24, 0.96)
                    } else {
                        Color::srgba(1.00, 0.20, 0.22, 0.96)
                    };
                    (label, bounds, color)
                } else {
                    continue;
                }
            }
            FeatureVisualKind::Breakable => (id, bounds, Color::srgba(1.00, 0.72, 0.24, 0.96)),
            _ => continue,
        };
        bars.show(
            key,
            scope,
            world,
            name,
            bounds,
            Health {
                current: view.hp_current,
                max: view.hp_max,
                invulnerable: Default::default(),
            },
            color,
        );
    }
    bars.finish();
}

impl HealthBarProducts<'_, '_> {
    #[allow(clippy::too_many_arguments)]
    fn show(
        &mut self,
        key: HealthBarKey,
        scope: SessionSpawnScope,
        world: &ae::World,
        name: &str,
        bounds: ae::Aabb,
        health: Health,
        fill_color: Color,
    ) {
        if !self.shown.insert(key) {
            return;
        }
        let width = bounds.width().max(56.0);
        let height = 7.0;
        let at = ae::Vec2::new(bounds.center().x, bounds.top() - 26.0);
        let position = world_to_bevy(world, at, WORLD_Z_PLAYER + 12.0);
        let fill_width = width * health.ratio().clamp(0.0, 1.0);
        let background_size = BVec2::new(width + 5.0, height + 5.0);
        let fill_size = BVec2::new(fill_width, height);
        let fill_offset = Vec3::new((fill_width - width) * 0.5, 0.0, 1.0);
        let fill_visibility = if fill_width > 0.5 { Visibility::Inherited } else { Visibility::Hidden };
        let hp = [health.current.max(0), health.max];
        let text = || format!("{}/{}", hp[0], hp[1]);

        if let Some(&root) = self.retained.get(&key) {
            if let Ok((mut bar, mut transform, mut background)) = self.bars.get_mut(root) {
                if let (Ok((mut offset, mut fill, mut visible)), Ok(mut label)) =
                    (self.fills.get_mut(bar.fill), self.labels.get_mut(bar.label))
                {
                    transform.set_if_neq(Transform::from_translation(position));
                    offset.set_if_neq(Transform::from_translation(fill_offset));
                    visible.set_if_neq(fill_visibility);
                    if background.custom_size != Some(background_size) {
                        background.custom_size = Some(background_size);
                    }
                    if fill.custom_size != Some(fill_size) || fill.color != fill_color {
                        fill.custom_size = Some(fill_size);
                        fill.color = fill_color;
                    }
                    if bar.hp != hp {
                        label.0 = text();
                        bar.hp = hp;
                    }
                    return;
                }
                self.commands.entity(root).try_despawn();
            }
        }

        let scope = scope.in_room(Some(key.1));
        let root = self.commands.spawn_session_scoped(scope, (
            Sprite::from_color(Color::srgba(0.02, 0.03, 0.05, 0.86), background_size),
            Transform::from_translation(position),
            Name::new(format!("Health bar bg: {name}")),
        )).id();
        let fill = self.commands.spawn_session_scoped(scope, (
            WorldSprite,
            Sprite::from_color(fill_color, fill_size),
            Transform::from_translation(fill_offset),
            fill_visibility,
            ChildOf(root),
            Name::new(format!("Health bar fill: {name}")),
        )).id();
        let label = self.commands.spawn_session_scoped(scope, (
            WorldText,
            Text2d::new(text()),
            TextFont { font_size: FontSize::Px(11.0), ..default() },
            TextColor(Color::srgba(0.96, 0.98, 1.0, 0.98)),
            Transform::from_xyz(0.0, 13.0, 2.0),
            ChildOf(root),
            Name::new(format!("Health label: {name}")),
        )).id();
        self.commands.entity(root).insert(HealthOverlayVisual { fill, label, hp });
        self.retained.insert(key, root);
    }

    fn finish(&mut self) {
        self.retained.reconcile(&mut self.commands, |key| self.shown.contains(key));
        self.shown.clear();
    }
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
            order: Default::default(),
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
                room: None,
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
            let bosses = ambition_sim_view::BossRenderIndex::from_rows(fought.map(|(id, room, fighting)| {
                (
                    id.to_string(),
                    ambition_sim_view::BossRenderView {
                        room: Some(room),
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

    #[test]
    fn health_bars_keep_their_parts_in_rooms_with_the_same_feature_id() {
        let world_of = |size| ae::World::new("bar room", size, ae::Vec2::splat(40.0), Vec::new());
        let (big, small) = (ae::Vec2::new(800.0, 600.0), ae::Vec2::new(400.0, 300.0));
        let (first, second) = (LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next());
        let mut app = App::new();
        insert_live_room_component(app.world_mut(), ae::RoomGeometry(world_of(big)));
        spawn_live_room(app.world_mut(), second, ae::RoomGeometry(world_of(small)));
        let view = |room, x, hp| {
            let mut view = crate_at(ae::Vec2::new(x, 200.0));
            view.room = Some(room);
            view.hp_current = hp;
            ("crate".to_string(), view)
        };
        app.insert_resource(FeatureViewIndex::from_rows([
            view(first, 100.0, 1),
            view(second, 100.0, 2),
        ]));
        for room in [first, second] {
            app.world_mut().spawn((FeatureVisual { id: "crate".into() }, InRoomInstance(room)));
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

        let products = |world: &mut World| {
            world.query::<(Entity, &InRoomInstance, &HealthOverlayVisual, &Transform)>()
                .iter(world)
                .map(|(root, room, parts, place)| {
                    (room.0, ([root, parts.fill, parts.label], place.translation.truncate()))
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        app.update();
        let before = products(app.world_mut());
        assert_eq!(before.len(), 2);
        assert_eq!(before[&first].1, ae::Vec2::new(-300.0, 136.0));
        assert_eq!(before[&second].1, ae::Vec2::new(-100.0, -14.0));
        assert_eq!(app.world().get::<Text2d>(before[&first].0[2]).unwrap().0, "1/2");
        assert_eq!(app.world().get::<Text2d>(before[&second].0[2]).unwrap().0, "2/2");

        app.insert_resource(FeatureViewIndex::from_rows([view(second, 130.0, 1)]));
        app.update();
        let after = products(app.world_mut());
        assert_eq!(after.len(), 1);
        assert!(before[&first].0.iter().all(|&part| app.world().get_entity(part).is_err()));
        let ([root, fill, label], place) = after[&second];
        assert_eq!([root, fill, label], before[&second].0);
        assert_eq!(place, ae::Vec2::new(-70.0, -14.0));
        assert_eq!(app.world().get::<Sprite>(fill).unwrap().custom_size, Some(Vec2::new(28.0, 7.0)));
        assert_eq!(app.world().get::<Text2d>(label).unwrap().0, "1/2");

        app.world_mut().resource_mut::<ambition_dev_tools::dev_tools::DeveloperTools>().show_health_bars = false;
        app.update();
        assert!(products(app.world_mut()).is_empty());
        assert!([root, fill, label].iter().all(|&part| app.world().get_entity(part).is_err()));
    }
}
