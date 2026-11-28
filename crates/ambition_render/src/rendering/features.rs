//! Per-frame discovery system that spawns Bevy `FeatureVisual` entities for
//! live breakables and dynamically introduced features. Other authored visuals
//! are handled by [`super::world::spawn_room_visuals`] at room load.
//!
//! A pure consumer of the sim-built [`ambition_sim_view::DynamicFeatureViews`]
//! rows. The sim resolves identity, geometry, and placeholder-sprite facts;
//! this pass only spawns the missing visuals.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::actor_entrance::ActorEntranceFade;
use super::primitives::{feature_color, feature_z, FeatureVisual, RoomVisual};
use ambition_platformer2d_core::config::world_to_bevy;
use ambition_platformer2d_shared_tangle::gameplay_presentation::ActorEntrance;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::{DynamicFeatureViews, FeatureSheet};
use ambition_sprite_sheet::game_assets::{entity_sprite_or_color, GameAssets};

/// Spawn missing visuals for the live feature discovery feed.
/// Authored and runtime breakables share this path. Other authored features
/// receive their initial visuals from `spawn_room_visuals`.
///
/// `sync_visuals` reads the matching `FeatureView` and
/// `upgrade_actor_sprites` swaps in the character spritesheet on the
/// same frame; chests pick up their sprite via `state_aware_entity_sprite`.
pub fn spawn_dynamic_feature_visuals(
    mut commands: Commands,
    // Each visual is placed by the geometry of its body's own live room, and
    // stamped with it, so the room's retirement and render band take it.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    assets: Option<Res<GameAssets>>,
    active_session: Option<Res<ActiveSessionScope>>,
    existing: Query<(Entity, &FeatureVisual, Has<UnclaimedBodyPlaceholder>, Option<&super::primitives::PropVisual>)>,
    dynamic: Res<DynamicFeatureViews>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    // A diagnostic placeholder does not count as a claim.
    let mut known = std::collections::HashSet::new();
    let mut placeholders = std::collections::HashMap::new();
    let mut looks = std::collections::HashMap::new();
    for (entity, visual, is_placeholder, prop) in &existing {
        let key = (rooms.room_of(entity), visual.id.as_str());
        if is_placeholder {
            placeholders.insert(key, entity);
        } else {
            known.insert(key);
            looks.insert(key, (entity, prop.map(|prop| prop.kind.as_str())));
        }
    }
    let assets_ref = assets.as_deref();
    for fact in &dynamic.0 {
        let key = (fact.room, fact.id.as_str());
        if known.contains(&key) {
            let requested = match &fact.sheet {Some(FeatureSheet::Structure(look)) => Some(look.as_str()), _ => None};
            let replace = fact.visual_kind == ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Breakable
                && looks.get(&key).is_some_and(|(_, current)| *current != requested);
            if !replace { continue; }
            commands.entity(looks.remove(&key).unwrap().0).try_despawn();
            known.remove(&key);
        }
        // A body whose live room cannot be told is not drawn: there is no
        // geometry to place it by.
        let Some(room) = fact.room else {
            continue;
        };
        let Some(world) = rooms.in_room(room) else {
            continue;
        };
        let scope = session_scope.in_room(Some(room));
        // The real visual arrives: despawn the stand-in in the same flush, so the
        // frame never shows both.
        if let Some(placeholder) = placeholders.get(&key) {
            commands.entity(*placeholder).try_despawn();
        }
        known.insert(key);
        if let Some(FeatureSheet::Structure(look)) = &fact.sheet {
            let entity = super::world::spawn_feature_look(&mut commands, scope, &world.0,
                &fact.id, &fact.label, ambition_platformer2d_core::Aabb::new(fact.pos, fact.size * 0.5),
                fact.visual_kind, look, fact.sprite_key.unwrap_or(ambition_sprite_sheet::game_assets::EntitySprite::BreakableIntact),
                assets_ref);
            commands.entity(entity).insert(DynamicFeatureVisual);
            continue;
        }
        let render = BVec2::new(fact.size.x, fact.size.y);
        let fallback = feature_color(fact.visual_kind, fact.fighting, false);
        // A drop can name an animated sheet (a spinning ring). Bind it like the
        // room-load pass binds an authored pickup, so a dropped ring looks the
        // same as one placed in the level.
        let animated = match &fact.sheet {
            Some(FeatureSheet::Animated(kind)) => assets_ref.and_then(|assets| assets.characters.prop_asset_for_kind(kind)),
            _ => None,
        };
        let transform = Transform::from_translation(world_to_bevy(
            &world.0,
            fact.pos,
            feature_z(fact.visual_kind),
        ));
        let name = Name::new(format!("{}: {}", fact.family, fact.label));
        let visual = FeatureVisual {
            id: fact.id.clone(),
        };
        let (sprite, animation) = if let Some(asset) = animated {
            let (sprite, anchor, animator) =
                ambition_sprite_sheet::character::build_character_presentation(
                    asset,
                    render,
                    bevy::sprite::Anchor::CENTER,
                );
            (sprite, Some((anchor, animator)))
        } else {
            let sprite = match assets_ref {
                Some(a) => entity_sprite_or_color(a, fact.sprite_key, render, fallback),
                None => Sprite::from_color(fallback, render),
            };
            (sprite, None)
        };
        let mut entity = commands.spawn_session_scoped(
            scope,
            (sprite, transform, name, visual, RoomVisual, DynamicFeatureVisual),
        );
        if let Some(animation) = animation {
            entity.insert(animation);
        }
        if let Some(entrance) = fact.entrance {
            match entrance {
                ActorEntrance::FadeIn => entity.insert(ActorEntranceFade::default()),
            };
        }
    }
}

/// Marks a visual that this module spawned and must clean up.
///
/// The cleanup below despawns only entities with this marker. So it cannot
/// mistake a static room-load visual for a dead one on a frame when the view
/// index is not built yet.
#[derive(Component)]
pub struct DynamicFeatureVisual;

/// Despawn the visual of a dynamic feature that the sim no longer has.
///
/// For example, a dropped ring expires. Without this, its hidden sprite stays
/// for the life of the room, one entity per drop.
///
/// A feature is gone only when it is absent from both read-models.
/// `DynamicFeatureViews` is a discovery feed with per-family conditions (a mob
/// that turns peaceful leaves it but still exists). `FeatureViewIndex` holds
/// every live feature.
pub fn despawn_dead_dynamic_feature_visuals(
    mut commands: Commands,
    dynamic: Res<DynamicFeatureViews>,
    features: Res<ambition_sim_view::FeatureViewIndex>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    visuals: Query<(Entity, &FeatureVisual), With<DynamicFeatureVisual>>,
) {
    if visuals.is_empty() {
        return;
    }
    let discovered: std::collections::HashSet<_> =
        dynamic.0.iter().map(|fact| (fact.room, fact.id.as_str())).collect();
    for (entity, visual) in &visuals {
        let id = visual.id.as_str();
        let room = rooms.of(entity);
        if !discovered.contains(&(room, id)) && features.get_in(id, room).is_none() {
            commands.entity(entity).despawn();
        }
    }
}

/// Publish the set of feature views not yet claimed by a renderer and draw a
/// marked fallback rectangle for them. Family-specific renderers run first. A
/// fallback is not itself a claim and is retired as soon as a real visual
/// appears. Publishing remains valid without a session world; only fallback
/// drawing requires world geometry.
pub fn draw_unclaimed_feature_views(
    mut commands: Commands,
    // Each stand-in is placed by the geometry of its feature's own live room
    // and stamped with it, as `spawn_dynamic_feature_visuals` places a visual.
    // This is not a `Single`, so the census below is published with no room
    // live, and with two.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    views: Res<ambition_sim_view::FeatureViewIndex>,
    existing: Query<(Entity, &FeatureVisual, Has<UnclaimedBodyPlaceholder>)>,
    mut unsettled: ResMut<UnclaimedFeatureViews>,
    // Consecutive unclaimed frames per id. See
    // `UNCLAIMED_STAND_IN_GRACE_FRAMES`.
    mut unclaimed_streak: Local<std::collections::HashMap<(Option<LiveRoomInstance>, String), u32>>,
) {
    // Split drawn ids into real visuals and this system's own stand-ins.
    // "Claimed" and "standing in for a claim" are different answers.
    let mut known = std::collections::HashSet::new();
    let mut stand_ins = Vec::new();
    for (entity, visual, is_stand_in) in &existing {
        if is_stand_in {
            stand_ins.push((entity, rooms.room_of(entity), visual.id.as_str()));
        } else {
            known.insert((rooms.room_of(entity), visual.id.as_str()));
        }
    }
    // The real visual arrived: despawn the stand-in. Do this before the
    // session-scope guard, because despawn needs no scope, and a stale
    // placeholder would hold the cover open.
    for (entity, room, id) in &stand_ins {
        if known.contains(&(*room, *id)) {
            commands.entity(*entity).try_despawn();
        }
    }

    // Census, for the room-transition cover. Stand-ins are not subtracted: a
    // view that wears one is still a view that nothing drew.
    let mut unclaimed_now: Vec<(&str, &ambition_sim_view::FeatureView)> = views
        .iter()
        .filter(|(id, view)| {
            // Zero-sized views have no body (for example trigger state).
            // Nothing will draw them.
            !known.contains(&(view.room, *id)) && view.size.x > 0.0 && view.size.y > 0.0
        })
        .collect();
    // `FeatureViewIndex::iter` is hash-ordered. Sort so reports and spawn
    // order are stable.
    unclaimed_now.sort_unstable_by_key(|(id, view)| (*id, view.room));
    unsettled.entries.clear();
    unsettled
        .entries
        .extend(unclaimed_now.iter().map(|(id, view)| (id.to_string(), view.room)));

    // A claimed or removed id restarts its grace. This also keeps the map
    // from growing forever.
    unclaimed_streak.retain(|(room, id), _| views.get_in(id, *room)
        .is_some_and(|view| view.size.x > 0.0 && view.size.y > 0.0)
        && !known.contains(&(*room, id.as_str())));

    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let already_standing: std::collections::HashSet<_> =
        stand_ins.iter().map(|(_, room, id)| (*room, *id)).collect();
    for (id, view) in unclaimed_now {
        if already_standing.contains(&(view.room, id)) {
            continue;
        }
        // Diagnosis, which is allowed to be late. Drawing the stand-in at once
        // gave only false positives on the happy path. Guarded by
        // `a_permanently_unclaimed_view_still_gets_its_stand_in`.
        let streak = unclaimed_streak.entry((view.room, id.to_string())).or_insert(0);
        *streak = streak.saturating_add(1);
        if *streak < UNCLAIMED_STAND_IN_GRACE_FRAMES {
            continue;
        }
        // A feature whose live room cannot be told is not drawn: there is no
        // geometry to place it by.
        let Some((room, world)) = view.room.and_then(|room| Some((room, rooms.in_room(room)?))) else {
            continue;
        };
        bevy::log::warn!(
            target: "ambition_platformer2d::render",
            "no render family claimed `{id}` ({:?}) for {} consecutive frames; \
             drawing the unclaimed-body placeholder. Some spawn path is missing \
             its family marker.",
            view.kind,
            *streak,
        );
        commands.spawn_session_scoped(
            session_scope.in_room(Some(room)),
            (
                Sprite::from_color(UNCLAIMED_BODY_COLOR, BVec2::new(view.size.x, view.size.y)),
                Transform::from_translation(world_to_bevy(
                    &world.0,
                    view.pos,
                    feature_z(view.kind),
                )),
                Name::new(format!("UNCLAIMED body placeholder: {id}")),
                FeatureVisual { id: id.to_string() },
                RoomVisual,
                DynamicFeatureVisual,
                UnclaimedBodyPlaceholder,
            ),
        );
    }
}

/// Presentation is dormant, so nothing is waiting on it. Publish an empty
/// census explicitly. This is the inverse of the live publisher.
pub fn forget_unclaimed_feature_views_while_dormant(mut unsettled: ResMut<UnclaimedFeatureViews>) {
    if !unsettled.is_empty() {
        unsettled.entries.clear();
    }
}

/// Marks the fallback stand-in, not a render family's visual. See
/// [`draw_unclaimed_feature_views`].
#[derive(Component)]
pub struct UnclaimedBodyPlaceholder;

/// The published feature views that nothing has drawn.
///
/// [`draw_unclaimed_feature_views`] republishes it every frame: every
/// [`ambition_sim_view::FeatureViewIndex`] row with a body (`size > 0`) that no
/// render family claims with a real [`FeatureVisual`].
/// ## This is not the count of magenta boxes on screen
///
/// | role | question | wants to fire |
/// |---|---|---|
/// | the stand-in ([`UnclaimedBodyPlaceholder`]) | *did somebody forget a family marker?* | late — only once a view has stayed unclaimed long enough to be a real orphan |
/// | this resource | *is the new room finished drawing?* | immediately — the instant a view is unclaimed, so the cover keeps waiting |
///
/// A view that wears a stand-in is still counted here. A stand-in is not art.
///
/// A reader must run after the publisher in the same schedule.
#[derive(Resource, Default, Debug)]
pub struct UnclaimedFeatureViews {
    /// Sort by feature ID, then by room instance.
    entries: Vec<(String, Option<LiveRoomInstance>)>,
}

impl UnclaimedFeatureViews {
    /// How many published views are still undrawn.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is every published view drawn by some render family?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The ID and live room of each undrawn feature.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&str, Option<LiveRoomInstance>)> {
        self.entries.iter().map(|(id, room)| (id.as_str(), *room))
    }
}

/// Consecutive frames a view must stay unclaimed before the fallback draws its
/// magenta stand-in and warns. Small on purpose. Raise it if measurements
/// show a long tail; the cover does not depend on it.
const UNCLAIMED_STAND_IN_GRACE_FRAMES: u32 = 5;

/// Magenta, because nobody ships magenta on purpose.
const UNCLAIMED_BODY_COLOR: Color = Color::srgba(1.0, 0.0, 0.85, 0.85);

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;
    use ambition_sim_view::DynamicFeatureFact;

    fn app_with_a_room() -> App {
        let mut app = App::new();
        app.init_resource::<DynamicFeatureViews>();
        // Required. Without it, `ResMut` param validation skips the system and
        // the test passes without running anything.
        app.init_resource::<UnclaimedFeatureViews>();
        // A real session, so the spawn path uses its scoped arm.
        let mut active = ActiveSessionScope::default();
        let scope = active.begin();
        app.insert_resource(active);
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(scope),
            ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
                "probe",
                ambition_platformer2d_core::Vec2::new(320.0, 180.0),
                ambition_platformer2d_core::Vec2::new(40.0, 40.0),
                Vec::new(),
            )),
        ));
        app.world_mut().spawn((
            SessionRoot(scope),
        ));
        app
    }

    fn a_fact(id: &str) -> DynamicFeatureFact {
        DynamicFeatureFact {
            id: id.to_string(),
            label: id.to_string(),
            family: "probe",
            pos: ambition_platformer2d_core::Vec2::new(10.0, 10.0),
            size: ambition_platformer2d_core::Vec2::new(16.0, 24.0),
            visual_kind:
                ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
            fighting: false,
            sprite_key: None,
            sheet: None,
            entrance: None,
            room: Some(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION),
        }
    }

    #[test]
    fn a_runtime_actor_fades_once_when_its_visual_appears() {
        use super::super::actor_entrance::present_actor_entrances;
        use ambition_sprite_sheet::character::rigged::ComposedBodyDemand;
        use std::time::Duration;

        let mut app = app_with_a_room();
        app.init_resource::<Time>()
            .init_resource::<ambition_time::ClockState>()
            .init_resource::<ComposedBodyDemand>()
            .add_systems(
                Update,
                (
                    spawn_dynamic_feature_visuals,
                    |mut sprites: Query<&mut Sprite, With<DynamicFeatureVisual>>,
                     mut demand: ResMut<ComposedBodyDemand>| {
                        demand.clear();
                        // The actor animator sets the base opacity each frame.
                        for mut sprite in &mut sprites {
                            sprite.color.set_alpha(0.8);
                        }
                    },
                    present_actor_entrances,
                )
                    .chain(),
            );
        app.world_mut().resource_mut::<DynamicFeatureViews>().0 = vec![
            DynamicFeatureFact {
                entrance: Some(ActorEntrance::FadeIn),
                ..a_fact("arrival")
            },
            a_fact("immediate"),
        ];

        let advance = |app: &mut App, millis| {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_millis(millis));
            app.update();
        };
        advance(&mut app, 16);
        let world = app.world_mut();
        let mut visuals = world.query::<(Entity, &FeatureVisual)>();
        let mut entity_for = |id| {
            visuals
                .iter(world)
                .find(|(_, visual)| visual.id == id)
                .unwrap()
                .0
        };
        let arrival = entity_for("arrival");
        let immediate = entity_for("immediate");
        let alpha = |app: &App, entity| app.world().get::<Sprite>(entity).unwrap().color.alpha();
        let composed = |app: &App, entity| {
            app.world().resource::<ComposedBodyDemand>().is_declared(entity)
        };

        let first = alpha(&app, arrival);
        assert!(first > 0.0 && first < 0.8 && composed(&app, arrival));
        assert_eq!(alpha(&app, immediate), 0.8);
        assert!(!composed(&app, immediate));

        advance(&mut app, 80);
        let next = alpha(&app, arrival);
        assert!(next > first && next < 0.8 && composed(&app, arrival));
        advance(&mut app, 250);
        advance(&mut app, 16);
        assert_eq!(alpha(&app, arrival), 0.8);
        assert!(!composed(&app, arrival));
    }

    fn a_view() -> ambition_sim_view::FeatureView {
        ambition_sim_view::FeatureView {
            pos: ambition_platformer2d_core::Vec2::new(10.0, 10.0),
            size: ambition_platformer2d_core::Vec2::new(16.0, 24.0),
            kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Breakable,
            visible: true,
            submerged: false,
            wire_anchor: None,
            grab_reach: None,
            line_anchor: None,
            limb_host: None,
            depth_plane: Default::default(),
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
            hp_max: 1,
            training_dummy: false,
            armored: false,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
            sprite_offset: None,
            room: Some(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION),
            order: Default::default(),
        }
    }

    /// OW1: with two live rooms, each unclaimed view gets its stand-in, placed
    /// by the geometry of its own live room and stamped with it. The rooms
    /// differ in height, so one position lands at two places. Before, the
    /// stand-in read the sole live room and none was drawn while two rooms
    /// were live. The control is the one-room app, which draws its one
    /// stand-in.
    #[test]
    fn each_unclaimed_view_stands_in_its_own_live_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
        fn stand_ins(app: &mut App) -> Vec<(String, Option<LiveRoomInstance>, f32)> {
            let world = app.world_mut();
            let mut query = world.query_filtered::<
                (&FeatureVisual, Option<&InRoomInstance>, &Transform),
                With<UnclaimedBodyPlaceholder>,
            >();
            let mut rows: Vec<_> = query
                .iter(world)
                .map(|(visual, room, at)| (visual.id.clone(), room.map(|room| room.0), at.translation.y))
                .collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            rows
        }
        let second = LiveRoomInstance::ACTIVATION.next();
        let in_second = ambition_sim_view::FeatureView { room: Some(second), ..a_view() };

        let mut one = app_with_a_room();
        one.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "in_first".to_string(),
            a_view(),
        )]));
        one.add_systems(Update, draw_unclaimed_feature_views);
        for _ in 0..UNCLAIMED_STAND_IN_GRACE_FRAMES {
            one.update();
        }
        assert_eq!(stand_ins(&mut one).len(), 1, "control: one live room");

        let mut two = app_with_a_room();
        ambition_platformer2d_shared_tangle::lifecycle::spawn_live_room(
            two.world_mut(),
            second,
            ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
                "probe, taller",
                ambition_platformer2d_core::Vec2::new(320.0, 900.0),
                ambition_platformer2d_core::Vec2::new(40.0, 40.0),
                Vec::new(),
            )),
        );
        two.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([
            ("in_first".to_string(), a_view()),
            ("in_second".to_string(), in_second),
        ]));
        two.add_systems(Update, draw_unclaimed_feature_views);
        for _ in 0..UNCLAIMED_STAND_IN_GRACE_FRAMES {
            two.update();
        }
        let rows = stand_ins(&mut two);
        let placed = |height: f32| {
            let room = ambition_platformer2d_core::World::new(
                "placement",
                ambition_platformer2d_core::Vec2::new(320.0, height),
                ambition_platformer2d_core::Vec2::new(40.0, 40.0),
                Vec::new(),
            );
            world_to_bevy(&room, a_view().pos, 0.0).y
        };
        assert_eq!(
            rows,
            vec![
                ("in_first".to_string(), Some(LiveRoomInstance::ACTIVATION), placed(180.0)),
                ("in_second".to_string(), Some(second), placed(900.0)),
            ],
            "two live rooms: (id, stamp, placed y) of each stand-in"
        );
    }

    /// A diagnosis must not outlive the bug.
    ///
    /// The family spawner skips any id that already has a `FeatureVisual`, so a
    /// stand-in that stays would block the real family forever.
    #[test]
    fn the_real_visual_replaces_the_unclaimed_stand_in() {
        let mut app = app_with_a_room();
        // The fallback got there first.
        app.world_mut().spawn((
            FeatureVisual {
                id: "late_arrival".into(),
            },
            DynamicFeatureVisual,
            UnclaimedBodyPlaceholder,
        ));
        app.world_mut()
            .resource_mut::<DynamicFeatureViews>()
            .0
            .push(a_fact("late_arrival"));

        app.add_systems(Update, spawn_dynamic_feature_visuals);
        app.update();

        let world = app.world_mut();
        let mut visuals = world.query::<(&FeatureVisual, Has<UnclaimedBodyPlaceholder>)>();
        let rows: Vec<bool> = visuals
            .iter(world)
            .filter(|(v, _)| v.id == "late_arrival")
            .map(|(_, placeholder)| placeholder)
            .collect();
        assert_eq!(
            rows,
            vec![false],
            "expected exactly one visual for the id and for it to be the REAL \
             one; got {rows:?} (true = the magenta stand-in). A stand-in that \
             survives the family's own spawn is a permanent misdiagnosis, and \
             two visuals for one id is a double draw."
        );
    }

    /// An authored feature's placeholder must despawn when its real visual
    /// arrives. Otherwise it keeps the room-transition cover active.
    #[test]
    fn the_stand_in_for_an_authored_feature_is_retired_when_its_visual_arrives() {
        let mut app = app_with_a_room();
        // An authored feature: in the view index, not in the dynamic feed.
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "authored_brick".to_string(),
            a_view(),
        )]));
        // The fallback drew its stand-in before the room visuals were up.
        app.world_mut().spawn((
            FeatureVisual {
                id: "authored_brick".into(),
            },
            DynamicFeatureVisual,
            UnclaimedBodyPlaceholder,
        ));
        // Then the room-load spawner draws the real one.
        app.world_mut().spawn((
            FeatureVisual {
                id: "authored_brick".into(),
            },
            RoomVisual,
        ));

        app.add_systems(Update, draw_unclaimed_feature_views);
        app.update();

        let world = app.world_mut();
        let mut placeholders = world.query::<(&FeatureVisual, &UnclaimedBodyPlaceholder)>();
        assert_eq!(
            placeholders
                .iter(world)
                .filter(|(v, _)| v.id == "authored_brick")
                .count(),
            0,
            "the real visual has arrived, so the stand-in must go. While it \
             stays, the room-transition cover holds the screen black to its \
             8-second deadline."
        );
    }

    /// Limit on the grace period: the fallback must still draw a stand-in for a
    /// view that nothing will ever claim.
    #[test]
    fn a_permanently_unclaimed_view_still_gets_its_stand_in() {
        let mut app = app_with_a_room();
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "nobody_drew_me".to_string(),
            a_view(),
        )]));

        app.add_systems(Update, draw_unclaimed_feature_views);
        for _ in 0..UNCLAIMED_STAND_IN_GRACE_FRAMES {
            app.update();
        }

        let world = app.world_mut();
        let mut placeholders = world.query::<(&FeatureVisual, &UnclaimedBodyPlaceholder)>();
        assert_eq!(
            placeholders
                .iter(world)
                .filter(|(v, _)| v.id == "nobody_drew_me")
                .count(),
            1,
            "a view no family will ever claim is still a bug, and still gets its \
             marked box. The grace period is a delay on the DIAGNOSIS, never an \
             amnesty for it."
        );
    }

    /// The split in one test. On the first unclaimed frame the census must
    /// already report the view, and no magenta box may exist yet: a one-flush
    /// ordering gap is not a diagnosis.
    #[test]
    fn a_view_is_censused_as_unsettled_before_any_stand_in_is_drawn() {
        let mut app = app_with_a_room();
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "not_drawn_yet".to_string(),
            a_view(),
        )]));

        app.add_systems(Update, draw_unclaimed_feature_views);
        app.update();

        assert_eq!(
            app.world()
                .resource::<UnclaimedFeatureViews>()
                .iter()
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec!["not_drawn_yet"],
            "the census must report an undrawn view on the FIRST frame it is \
             undrawn. This is what the room-transition cover waits on; anything \
             later and it retires over art that has not arrived."
        );

        let world = app.world_mut();
        let mut placeholders = world.query_filtered::<(), With<UnclaimedBodyPlaceholder>>();
        assert_eq!(
            placeholders.iter(world).count(),
            0,
            "…and NO magenta box yet. One frame unclaimed is a `Commands` flush, \
             not a missing family marker — 190 warnings against 0 cover give-ups \
             in one healthy session is what drawing it immediately produced."
        );
    }

    /// A view claimed two flushes after publish keeps the room unsettled on the
    /// frames between. A placeholder-based count would read zero on frames 1
    /// and 2 and retire the cover too early.
    #[test]
    fn a_view_claimed_two_flushes_later_is_unsettled_until_it_is_drawn() {
        let mut app = app_with_a_room();
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "late_family".to_string(),
            a_view(),
        )]));
        app.add_systems(Update, draw_unclaimed_feature_views);

        let mut census: Vec<usize> = Vec::new();
        app.update();
        census.push(app.world().resource::<UnclaimedFeatureViews>().len());
        app.update();
        census.push(app.world().resource::<UnclaimedFeatureViews>().len());

        // The family finally draws it.
        app.world_mut().spawn((
            FeatureVisual {
                id: "late_family".into(),
            },
            RoomVisual,
        ));
        app.update();
        census.push(app.world().resource::<UnclaimedFeatureViews>().len());

        assert_eq!(
            census,
            vec![1, 1, 0],
            "the census must stay non-zero for every frame the view is undrawn \
             and drop to zero on the frame it is drawn. A zero on frame 1 or 2 is \
             a cover retiring into the gap it exists to hide."
        );
    }

    /// A stand-in is not art. A view that wears one must still hold the cover.
    /// The draw loop subtracts stand-ins to avoid a second box; the census must
    /// not.
    #[test]
    fn a_view_wearing_a_stand_in_is_still_unsettled() {
        let mut app = app_with_a_room();
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "wearing_magenta".to_string(),
            a_view(),
        )]));
        // The fallback already stood in for it on an earlier frame.
        app.world_mut().spawn((
            FeatureVisual {
                id: "wearing_magenta".into(),
            },
            DynamicFeatureVisual,
            UnclaimedBodyPlaceholder,
        ));

        app.add_systems(Update, draw_unclaimed_feature_views);
        app.update();

        assert_eq!(
            app.world()
                .resource::<UnclaimedFeatureViews>()
                .iter()
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec!["wearing_magenta"],
            "a view standing under the diagnostic rectangle is UNDRAWN. If this \
             ever reports empty, the room-transition cover retires over a magenta \
             box — which is the flash, exactly."
        );

        let world = app.world_mut();
        let mut placeholders = world.query_filtered::<(), With<UnclaimedBodyPlaceholder>>();
        assert_eq!(
            placeholders.iter(world).count(),
            1,
            "…and it must not be given a SECOND box. The draw loop skips ids that \
             already have one; only the census ignores that."
        );
    }

    /// Presentation is dormant, so the census must be empty.
    ///
    /// The publisher is session-gated. A `Resource` keeps its last value when
    /// nobody writes it (unlike session-scoped entities), and a stale non-zero
    /// census holds the transition cover black until its deadline.
    #[test]
    fn a_dormant_presentation_publishes_an_empty_census() {
        let mut app = App::new();
        app.init_resource::<UnclaimedFeatureViews>();
        // What the last live frame left behind.
        app.world_mut()
            .resource_mut::<UnclaimedFeatureViews>()
            .entries
            .push(("left_over_from_the_last_session".to_string(), None));
        assert_eq!(
            app.world().resource::<UnclaimedFeatureViews>().len(),
            1,
            "the fixture has to actually leave something in the census, or the \
             clear below proves nothing"
        );

        app.add_systems(Update, forget_unclaimed_feature_views_while_dormant);
        app.update();
        assert!(
            app.world().resource::<UnclaimedFeatureViews>().is_empty(),
            "a dormant presentation is not waiting for anything to be drawn"
        );
    }

    /// A family that already drew an id is not asked to draw it again.
    #[test]
    fn a_real_visual_is_not_respawned() {
        let mut app = app_with_a_room();
        app.world_mut().spawn((
            FeatureVisual {
                id: "already_drawn".into(),
            },
            DynamicFeatureVisual,
        ));
        app.world_mut()
            .resource_mut::<DynamicFeatureViews>()
            .0
            .push(a_fact("already_drawn"));

        app.add_systems(Update, spawn_dynamic_feature_visuals);
        app.update();

        let world = app.world_mut();
        let mut visuals = world.query::<&FeatureVisual>();
        assert_eq!(
            visuals
                .iter(world)
                .filter(|v| v.id == "already_drawn")
                .count(),
            1
        );
    }

    #[test]
    fn breakable_looks_converge_without_duplicate_or_stale_visuals() {
        use super::super::primitives::PropVisual;
        use ambition_platformer2d_world::rooms::PropDraw;
        use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
        let mut app = app_with_a_room();
        let mut fact = a_fact("created_block");
        fact.visual_kind = FeatureVisualKind::Breakable;
        fact.sheet = Some(FeatureSheet::Structure("first_look".into()));
        app.world_mut().resource_mut::<DynamicFeatureViews>().0 = vec![fact.clone(), fact];
        app.add_systems(Update, spawn_dynamic_feature_visuals);
        app.update();
        let mut visuals = app.world_mut().query::<(Entity, &FeatureVisual, Option<&PropVisual>, &Sprite)>();
        let rows: Vec<_> = visuals.iter(app.world()).collect();
        assert_eq!(rows.len(), 1);
        let original = rows[0].0;
        assert_eq!(rows[0].2.unwrap().kind, "first_look");
        assert_eq!(rows[0].2.unwrap().draw, PropDraw::Structure);
        assert!(rows[0].3.custom_size.is_some());
        for fact in &mut app.world_mut().resource_mut::<DynamicFeatureViews>().0 {
            fact.sheet = Some(FeatureSheet::Structure("second_look".into()));
        }
        app.update();
        assert!(app.world().get_entity(original).is_err());
        let rows: Vec<_> = visuals.iter(app.world()).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].2.unwrap().kind, "second_look");
        for fact in &mut app.world_mut().resource_mut::<DynamicFeatureViews>().0 { fact.sheet = None; }
        app.update();
        let rows: Vec<_> = visuals.iter(app.world()).collect();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].2.is_none());
    }

    #[test]
    fn equal_feature_ids_in_distinct_rooms_get_distinct_visuals() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
        let mut app = app_with_a_room();
        let world = app.world_mut();
        let geometry = world.query::<&ambition_platformer2d_core::RoomGeometry>().single(world).unwrap().clone();
        let second = LiveRoomInstance::from_ordinal(27);
        world.spawn((RoomInstanceRoot, second, geometry));
        let a = a_fact("same_identity");
        let mut b = a.clone();b.room = Some(second);
        world.resource_mut::<DynamicFeatureViews>().0 = vec![a, b];
        app.init_resource::<ambition_sim_view::FeatureViewIndex>();
        app.add_systems(Update, (spawn_dynamic_feature_visuals, despawn_dead_dynamic_feature_visuals).chain());
        app.update();app.update();
        let world = app.world_mut();
        let mut rooms: Vec<_> = world.query::<(&FeatureVisual, &InRoomInstance)>().iter(world)
            .filter(|(visual, _)| visual.id == "same_identity").map(|(_, room)| room.0).collect();
        rooms.sort();
        assert_eq!(rooms, vec![LiveRoomInstance::ACTIVATION, second]);
        app.world_mut().resource_mut::<DynamicFeatureViews>().0.retain(|row| row.room == Some(second));
        app.update();
        let world = app.world_mut();
        let rooms: Vec<_> = world.query::<(&FeatureVisual, &InRoomInstance)>().iter(world)
            .filter(|(visual, _)| visual.id == "same_identity").map(|(_, room)| room.0).collect();
        assert_eq!(rooms, vec![second]);
    }

}
