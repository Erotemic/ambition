//! The actor-road tests matter most. `PlayerVisual` is on only the one
//! exploration player, not on match fighters, so tests that spawn only a
//! `PlayerVisual` cannot catch a rope missing from a versus match.

use super::*;

fn wire_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
            "wire test world",
            ambition_platformer2d_core::Vec2::new(1600.0, 900.0),
            ambition_platformer2d_core::Vec2::new(300.0, 500.0),
            Vec::new(),
        )),
    );
    app.add_systems(Startup, build_flyline_sprite);
    app.add_systems(Update, sync_flyline_visuals);
    app
}

const AT: ambition_platformer2d_core::Vec2 = ambition_platformer2d_core::Vec2::new(300.0, 500.0);
const ANCHOR: ambition_platformer2d_core::Vec2 =
    ambition_platformer2d_core::Vec2::new(300.0, 500.0 - 720.0);

fn actor_view(
    wire_anchor: Option<ambition_platformer2d_core::Vec2>,
) -> ambition_sim_view::FeatureView {
    ambition_sim_view::FeatureView {
        pos: AT,
        size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
        kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
        visible: true,
        submerged: false,
        wire_anchor,
        grab_reach: None,
        line_anchor: None,
        flash: false,
        breakable_state: None,
        chest_opened: false,
        fighting: true,
        switch_on: false,
        rotation_rad: 0.0,
        alive: true,
        hit_flash_secs: 0.0,
        parry_flash_secs: 0.0,
        hp_current: 40,
        hp_max: 40,
        training_dummy: false,
        hit_strength: 0.0,
        unhittable: false,
        defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
        sprite_offset: None,
    }
}

fn a_fighter(app: &mut App, wire_anchor: Option<ambition_platformer2d_core::Vec2>) -> Entity {
    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        "fighter".to_string(),
        actor_view(wire_anchor),
    )]));
    app.world_mut()
        .spawn(crate::rendering::FeatureVisual {
            id: "fighter".to_string(),
        })
        .id()
}

fn wires(app: &mut App) -> Vec<(Entity, Entity, Transform, Option<bevy::math::Vec2>)> {
    app.world_mut()
        .query::<(Entity, &FlylineVisual, &Transform, &Sprite)>()
        .iter(app.world())
        .map(|(e, owner, t, s)| (e, owner.body, *t, s.custom_size))
        .collect()
}

/// A match fighter has no `PlayerVisual`, so a visual gated on that marker
/// never draws in a versus match, where the up-B is played.
#[test]
fn a_match_fighter_on_a_wire_gets_one_though_it_carries_no_player_visual() {
    let mut app = wire_app();
    let body = a_fighter(&mut app, Some(ANCHOR));
    app.update();
    let found = wires(&mut app);
    assert_eq!(
        found.len(),
        1,
        "a fighter on a wire got no wire, so she is flying on nothing — the \
         gate is back on `PlayerVisual`"
    );
    assert_eq!(found[0].1, body, "the wire names the fighter it holds up");
}

/// The rope is removed when the wire lets go, so none hangs over an empty
/// stage.
#[test]
fn the_wire_goes_when_the_rope_lets_go() {
    let mut app = wire_app();
    a_fighter(&mut app, Some(ANCHOR));
    app.update();
    assert_eq!(wires(&mut app).len(), 1, "she is on it, so a rope hangs");

    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        "fighter".to_string(),
        actor_view(None),
    )]));
    app.update();
    assert!(
        wires(&mut app).is_empty(),
        "the wire let go and the rope stayed in the air"
    );
}

/// A fighter not on a wire gets no rope. Without this test, "a wire appears"
/// passes on a system that draws one for everybody.
#[test]
fn a_fighter_on_no_wire_gets_no_rope() {
    let mut app = wire_app();
    a_fighter(&mut app, None);
    app.update();
    assert!(wires(&mut app).is_empty());
}

/// The rope reaches from the anchor to the body. The length check matters: a
/// sprite placed correctly but left at its authored size is a 32px stub.
#[test]
fn the_rope_spans_the_whole_distance_from_the_sky_to_the_body() {
    let mut app = wire_app();
    a_fighter(&mut app, Some(ANCHOR));
    app.update();
    let found = wires(&mut app);
    let size = found[0].3.expect("the rope is sized to its span");
    assert!(
        (size.y - 720.0).abs() < 1.0,
        "the rope is {}px long against a 720px span",
        size.y
    );
    assert!(size.x < 8.0, "a {}px-wide rope is a pillar", size.x);
}

/// The rope follows the swing. A rope drawn straight down while the body
/// hangs to one side means presentation reads a length, not a pair of points.
#[test]
fn the_rope_leans_with_the_body_it_is_holding() {
    let mut app = wire_app();
    // Swung out to the right: the anchor is above and left of her.
    let swung = ambition_platformer2d_core::Vec2::new(AT.x - 200.0, AT.y - 690.0);
    a_fighter(&mut app, Some(swung));
    app.update();
    let hung = wires(&mut app)[0].2.rotation;

    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        "fighter".to_string(),
        actor_view(Some(ANCHOR)),
    )]));
    app.update();
    let plumb = wires(&mut app)[0].2.rotation;

    assert!(
        plumb.angle_between(hung) > 0.1,
        "the rope is at the same angle swung out as it is hanging plumb, so it \
         is not drawn between two points"
    );
}

/// The player road still works, so a session with an exploration player does
/// not break.
#[test]
fn the_exploration_player_gets_a_rope_too() {
    let mut app = wire_app();
    let body = app
        .world_mut()
        .spawn((
            PlayerVisual,
            ambition_sim_view::BodyPoseView {
                pos: AT,
                size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
                wire_anchor: Some(ANCHOR),
                ..Default::default()
            },
        ))
        .id();
    app.update();
    let found = wires(&mut app);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1, body);
}
