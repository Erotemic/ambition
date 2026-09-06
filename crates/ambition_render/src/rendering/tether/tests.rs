//! ⛔⛔ THE ACTOR-ROAD ARM IS THE LOAD-BEARING ONE, for the reason the flyline's
//! tests state: the trapdoor's visual was declared done twice while the move was
//! visibly broken, and both times every test spawned a `PlayerVisual` — which is
//! inserted in exactly ONE place in the engine and is not what a match fighter
//! carries. A tether that draws only on the player road is a tether that never
//! appears in a versus match, which is the whole reason it exists.

use super::*;

const AT: ambition_platformer2d_core::Vec2 = ambition_platformer2d_core::Vec2::new(300.0, 500.0);
const REACH: ambition_platformer2d_core::Vec2 =
    ambition_platformer2d_core::Vec2::new(300.0 + 150.0, 500.0);

fn tether_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
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

fn actor_view(grab_reach: Option<ambition_platformer2d_core::Vec2>) -> ambition_sim_view::FeatureView {
    ambition_sim_view::FeatureView {
        pos: AT,
        size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
        kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
        visible: true,
        submerged: false,
        wire_anchor: None,
        grab_reach,
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

/// A MATCH FIGHTER'S tether draws. This is the arm the trapdoor did not have.
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

/// And it is retired the moment the grab stops reaching.
///
/// ⛔ A LINE THAT OUTLIVES ITS GRAB IS WORSE THAN NONE: it shows a threat that
/// is not there, and a player who respects it is being lied to.
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
