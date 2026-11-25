//! The actor-road tests matter most. `PlayerVisual` is on only the one
//! exploration player, not on match fighters, so a tether drawn only on the
//! player road never appears in a versus match.

use super::*;
use ambition_platformer2d_shared_tangle::{lifecycle::PlayerVisual, sim_id::SimId};

const AT: ambition_platformer2d_core::Vec2 = ambition_platformer2d_core::Vec2::new(300.0, 500.0);
const REACH: ambition_platformer2d_core::Vec2 =
    ambition_platformer2d_core::Vec2::new(300.0 + 150.0, 500.0);

fn tether_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
            "tether test world",
            ambition_platformer2d_core::Vec2::new(1600.0, 900.0),
            ambition_platformer2d_core::Vec2::new(300.0, 500.0),
            Vec::new(),
        )),
    );
    app.add_systems(Startup, crate::rendering::flyline::build_flyline_sprite);
    app.add_systems(Update, sync_tether_visuals);
    app
}

fn actor_view(
    grab_reach: Option<ambition_platformer2d_core::Vec2>,
) -> ambition_sim_view::FeatureView {
    ambition_sim_view::FeatureView {
        pos: AT,
        size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
        kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
        visible: true,
        submerged: false,
        wire_anchor: None,
        grab_reach,
        line_anchor: None,
        limb_host: None,
        depth_plane: Default::default(),
        flash: false,
        breakable_state: None,
        chest_opened: false,
        fighting: true,
        switch_on: false,
        material: Default::default(),
        alive: true,
        hit_flash_secs: 0.0,
        parry_flash_secs: 0.0,
        hp_current: 40,
        hp_max: 40,
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

fn a_fighter(app: &mut App, grab_reach: Option<ambition_platformer2d_core::Vec2>) -> Entity {
    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        "fighter".to_string(),
        actor_view(grab_reach),
    )]));
    app.world_mut()
        .spawn(crate::rendering::FeatureVisual {
            id: "fighter".to_string(),
        })
        .id()
}

fn lines(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query::<(Entity, &TetherVisual)>()
        .iter(app.world())
        .map(|(e, _)| e)
        .collect()
}

/// A match fighter's tether draws.
///
/// A ledge reel also draws its line, and it is not a grab. The ruleset
/// publishes a generic `BodyLineAnchor`, the read models project it as
/// `line_anchor`, and this file draws `grab_reach.or(line_anchor)`. Nothing
/// here knows what a tether is.
#[test]
fn a_fighter_reeled_to_a_ledge_gets_a_line_without_a_grab() {
    let mut app = tether_app();
    // No grab anywhere — only the latched anchor.
    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        "fighter".to_string(),
        ambition_sim_view::FeatureView {
            line_anchor: Some(ambition_platformer2d_core::Vec2::new(400.0, 120.0)),
            limb_host: None,
            depth_plane: Default::default(),
            ..actor_view(None)
        },
    )]));
    app.world_mut().spawn(crate::rendering::FeatureVisual {
        id: "fighter".to_string(),
    });
    app.update();
    assert_eq!(
        lines(&mut app).len(),
        1,
        "a body reeling to a latched ledge drew no line, so the reel is visible \
         only through the movement it causes"
    );
}

/// The control: without it, the test above passes on a system that draws a
/// line for every body, lined to anything or not.
#[test]
fn a_fighter_lined_to_nothing_gets_no_line() {
    let mut app = tether_app();
    a_fighter(&mut app, None);
    app.update();
    assert!(
        lines(&mut app).is_empty(),
        "a body with neither a grab nor an anchor was given a line"
    );
}

#[test]
fn a_match_fighter_reaching_gets_a_line() {
    let mut app = tether_app();
    a_fighter(&mut app, Some(REACH));
    app.update();
    assert_eq!(
        lines(&mut app).len(),
        1,
        "a fighter on the ACTOR road reached and drew no line — which is the \
         trapdoor's defect, and the reason this file exists"
    );
}

/// The line is removed when the grab stops reaching.
///
/// A line that outlives its grab shows a threat that is not there.
#[test]
fn the_line_is_retired_when_the_grab_ends() {
    let mut app = tether_app();
    a_fighter(&mut app, Some(REACH));
    app.update();
    assert_eq!(lines(&mut app).len(), 1);
    a_fighter(&mut app, None);
    app.update();
    assert!(
        lines(&mut app).is_empty(),
        "the tether line outlived the grab that drew it"
    );
}

/// A fighter reaching for nothing has no line at all.
#[test]
fn a_fighter_not_reaching_draws_nothing() {
    let mut app = tether_app();
    a_fighter(&mut app, None);
    app.update();
    assert!(
        lines(&mut app).is_empty(),
        "a line was drawn for a fighter with no live grab"
    );
}

#[test]
fn body_attachments_follow_room_identity_and_retire_with_their_owners() {
    use crate::rendering::{
        flyline::{sync_flyline_visuals, FlylineVisual},
        submerged::{build_trapdoor_sprite, sync_trapdoor_visuals, TrapdoorSprite, TrapdoorVisual},
    };
    use ambition_platformer2d_shared_tangle::lifecycle::{
        InRoomInstance, LiveRoomInstance, RoomInstanceRoot,
    };
    let mut app = tether_app();
    app.add_systems(Startup, build_trapdoor_sprite);
    app.add_systems(Update, (sync_flyline_visuals, sync_trapdoor_visuals));
    let first = LiveRoomInstance::ACTIVATION;
    let second = first.next();
    let geometry = ambition_platformer2d_core::World::new(
        "second",
        ambition_platformer2d_core::Vec2::new(800.0, 600.0),
        AT,
        vec![],
    );
    let second_root = app
        .world_mut()
        .spawn((
            RoomInstanceRoot,
            second,
            ambition_platformer2d_core::RoomGeometry(geometry.clone()),
        ))
        .id();
    let player = app
        .world_mut()
        .spawn((
            PlayerVisual,
            SimId::player_slot(0),
            InRoomInstance(first),
            ambition_sim_view::BodyPoseView {
                pos: AT,
                size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
                grab_reach: Some(REACH),
                wire_anchor: Some(AT),
                submerged: true,
                ..Default::default()
            },
        ))
        .id();
    let mut rows = Vec::new();
    let mut fighters = Vec::new();
    for (room, x) in [(first, 200.0), (second, 600.0)] {
        let mut view = actor_view(Some(REACH));
        view.room = Some(room);
        view.pos.x = x;
        view.wire_anchor = Some(AT);
        view.submerged = true;
        rows.push(("same_placement".to_owned(), view));
        fighters.push(
            app.world_mut()
                .spawn((
                    InRoomInstance(room),
                    crate::rendering::FeatureVisual {
                        id: "same_placement".into(),
                    },
                ))
                .id(),
        );
    }
    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows(rows));
    app.update();
    app.update();
    assert!(app.world().contains_resource::<TrapdoorSprite>());
    assert_eq!(lines(&mut app).len(), 3);
    assert_eq!(
        app.world_mut()
            .query::<&FlylineVisual>()
            .iter(app.world())
            .count(),
        3
    );
    assert_eq!(
        app.world_mut()
            .query::<&TrapdoorVisual>()
            .iter(app.world())
            .count(),
        3
    );
    let line_for = |app: &mut App, owner| {
        app.world_mut()
            .query::<(&TetherVisual, &Transform)>()
            .iter(app.world())
            .find(|(line, _)| line.body == owner)
            .map(|(_, t)| *t)
            .unwrap()
    };
    let mut expected = Transform::default();
    let mut art = Sprite::default();
    // Under its owner's drawn root, as the system draws it.
    let wire_z = |app: &App, owner: Entity| {
        app.world().get::<Transform>(owner).map_or(ambition_platformer2d_core::config::WORLD_Z_PLAYER, |t| t.translation.z)
            - crate::rendering::flyline::WIRE_UNDER_BODY
    };
    crate::rendering::flyline::place_wire(
        &geometry,
        &mut expected,
        &mut art,
        Vec2::new(REACH.x, REACH.y),
        Vec2::new(600.0, AT.y),
        wire_z(&app, fighters[1]),
    );
    assert_eq!(line_for(&mut app, fighters[1]), expected);

    app.world_mut()
        .entity_mut(player)
        .insert(InRoomInstance(second));
    app.update();
    crate::rendering::flyline::place_wire(
        &geometry,
        &mut expected,
        &mut art,
        Vec2::new(REACH.x, REACH.y),
        Vec2::new(AT.x, AT.y),
        wire_z(&app, player),
    );
    assert_eq!(line_for(&mut app, player), expected);
    assert_eq!(lines(&mut app).len(), 3);

    let retained_line = app
        .world_mut()
        .query::<(Entity, &TetherVisual)>()
        .iter(app.world())
        .find(|(_, line)| line.body == fighters[0])
        .unwrap()
        .0;
    app.world_mut().despawn(fighters[0]);
    fighters[0] = app
        .world_mut()
        .spawn((
            InRoomInstance(first),
            crate::rendering::FeatureVisual {
                id: "same_placement".into(),
            },
        ))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<TetherVisual>(retained_line).unwrap().body,
        fighters[0]
    );
    assert_eq!(
        app.world()
            .get::<ambition_platformer2d_shared_tangle::lifecycle::PresentationOf>(retained_line)
            .unwrap()
            .0,
        fighters[0]
    );
    assert_eq!(lines(&mut app).len(), 3);
    app.world_mut().despawn(fighters[0]);
    app.update();
    assert_eq!(lines(&mut app).len(), 2);
    app.world_mut().despawn(player);
    app.world_mut().despawn(fighters[1]);
    app.world_mut().despawn(second_root);
    app.update();
    assert!(lines(&mut app).is_empty());
    assert_eq!(
        app.world_mut()
            .query::<&FlylineVisual>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&TrapdoorVisual>()
            .iter(app.world())
            .count(),
        0
    );
}
