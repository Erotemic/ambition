use super::*;

fn pose(submerged: bool) -> ambition_sim_view::BodyPoseView {
    ambition_sim_view::BodyPoseView {
        submerged,
        ..Default::default()
    }
}

fn run(submerged: bool, start: Visibility) -> Visibility {
    let mut app = App::new();
    let body = app
        .world_mut()
        .spawn((PlayerVisual, pose(submerged), start))
        .id();
    app.add_systems(Update, sync_submerged_visibility);
    app.update();
    *app.world().entity(body).get::<Visibility>().expect("visibility")
}

#[test]
fn a_submerged_body_is_hidden() {
    assert_eq!(run(true, Visibility::Inherited), Visibility::Hidden);
}

/// Paired arm: a body that is not submerged is shown again, or she never
/// comes out of the trapdoor.
#[test]
fn a_body_that_surfaced_is_handed_back() {
    assert_eq!(run(false, Visibility::Hidden), Visibility::Inherited);
}

/// It is restored as `Inherited`, not `Visible`. A death overlay and a room
/// fade hide bodies through the parent; `Visible` would leave a surfacing
/// fighter as the only thing on screen.
#[test]
fn the_restore_never_forces_a_body_visible_over_its_parent() {
    assert_ne!(run(false, Visibility::Hidden), Visibility::Visible);
}

/// A body nobody hides is left alone, so this system never turns another
/// system's `Visible` into `Inherited`.
#[test]
fn a_visible_body_is_not_touched() {
    assert_eq!(run(false, Visibility::Visible), Visibility::Visible);
}

// ---------------------------------------------------------------------------
// The door
// ---------------------------------------------------------------------------

fn door_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
            "door test world",
            ambition_platformer2d_core::Vec2::new(1600.0, 900.0),
            ambition_platformer2d_core::Vec2::new(300.0, 500.0),
            Vec::new(),
        )),
    );
    app.add_systems(Startup, build_trapdoor_sprite);
    app.add_systems(Update, sync_trapdoor_visuals);
    app
}

fn body_pose(submerged: bool) -> ambition_sim_view::BodyPoseView {
    ambition_sim_view::BodyPoseView {
        submerged,
        pos: ambition_platformer2d_core::Vec2::new(300.0, 500.0),
        size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
        ..Default::default()
    }
}

fn doors(app: &mut App) -> Vec<(Entity, Entity, Vec3)> {
    app.world_mut()
        .query::<(Entity, &TrapdoorVisual, &Transform)>()
        .iter(app.world())
        .map(|(e, owner, t)| (e, owner.body, t.translation))
        .collect()
}

/// A submerged body is replaced by a trapdoor on the ground, so the stage
/// shows where she is.
///
/// The door is at her feet. She does not move along gravity while submerged,
/// so the feet line is the surface; the centre would float the door half a
/// body up.
#[test]
fn a_submerged_body_is_given_a_door_on_the_floor_it_went_through() {
    let mut app = door_app();
    let body = app
        .world_mut()
        .spawn((PlayerVisual, body_pose(true)))
        .id();
    app.update();
    let found = doors(&mut app);
    assert_eq!(found.len(), 1, "one submerged body, one door");
    assert_eq!(found[0].1, body, "the door names the body it belongs to");
    let feet = ambition_platformer2d_core::config::world_to_bevy(
        &ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
            ambition_platformer2d_core::RoomGeometry,
        >(app.world())
        .expect("room")
        .0,
        ambition_platformer2d_core::Vec2::new(300.0, 500.0 + 24.0),
        ambition_platformer2d_core::config::WORLD_Z_PLAYER + 0.05,
    );
    assert!(
        (found[0].2 - feet).length() < 1e-3,
        "the door drew at {:?}, wanted her feet at {feet:?}",
        found[0].2,
    );
}

/// Paired arm: surfacing removes the door. Without it a door stays on stage
/// forever.
#[test]
fn the_door_goes_when_she_surfaces() {
    let mut app = door_app();
    let body = app
        .world_mut()
        .spawn((PlayerVisual, body_pose(true)))
        .id();
    app.update();
    assert_eq!(doors(&mut app).len(), 1, "the premise: she is under the stage");
    app.world_mut()
        .get_mut::<ambition_sim_view::BodyPoseView>(body)
        .expect("pose")
        .submerged = false;
    app.update();
    assert!(doors(&mut app).is_empty(), "she surfaced and the door stayed");
}

/// One door per body: a versus match has four fighters, and any of them may
/// use this move.
#[test]
fn two_submerged_fighters_get_two_doors() {
    let mut app = door_app();
    let a = app.world_mut().spawn((PlayerVisual, body_pose(true))).id();
    let mut second = body_pose(true);
    second.pos.x = 900.0;
    let b = app.world_mut().spawn((PlayerVisual, second)).id();
    app.update();
    let found = doors(&mut app);
    assert_eq!(found.len(), 2, "two submerged bodies, two doors");
    let owners: Vec<Entity> = found.iter().map(|(_, owner, _)| *owner).collect();
    assert!(owners.contains(&a) && owners.contains(&b));
}

// ---------------------------------------------------------------------------
// The actor road
// ---------------------------------------------------------------------------

/// The tests above spawn a `PlayerVisual`, which only the session's single
/// exploration player has (`session/setup.rs`). A Smash match fighter is an
/// actor: a `FeatureVisual` whose facts come from `FeatureViewIndex`. These
/// tests cover that road, for both the hide and the door.
fn actor_view(submerged: bool) -> ambition_sim_view::FeatureView {
    ambition_sim_view::FeatureView {
        pos: ambition_platformer2d_core::Vec2::new(300.0, 500.0),
        size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
        kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
        // Not one fact: a dead hostile is invisible too, and a trapdoor must
        // not open over a corpse.
        visible: !submerged,
        submerged,
        wire_anchor: None,
        grab_reach: None,
        line_anchor: None,
        limb_host: None,
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

fn a_fighter(app: &mut App, submerged: bool) -> Entity {
    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        "fighter".to_string(),
        actor_view(submerged),
    )]));
    app.world_mut()
        .spawn(crate::rendering::FeatureVisual {
            id: "fighter".to_string(),
        })
        .id()
}

#[test]
fn a_submerged_match_fighter_gets_a_door_though_it_carries_no_player_visual() {
    let mut app = door_app();
    let body = a_fighter(&mut app, true);
    app.update();
    let found = doors(&mut app);
    assert_eq!(
        found.len(),
        1,
        "an actor under the stage got no door, so nothing on stage says where \
         she is — the gate is back on `PlayerVisual`"
    );
    assert_eq!(found[0].1, body, "the door names the fighter it belongs to");
}

/// It comes back up: otherwise a door stays over an empty stage for the rest
/// of the match.
#[test]
fn the_match_fighters_door_goes_when_she_surfaces() {
    let mut app = door_app();
    a_fighter(&mut app, true);
    app.update();
    assert_eq!(doors(&mut app).len(), 1, "she is under, so a door stands");

    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        "fighter".to_string(),
        actor_view(false),
    )]));
    app.update();
    assert!(
        doors(&mut app).is_empty(),
        "she surfaced and the boards stayed open"
    );
}
