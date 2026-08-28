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

/// ⛔ THE PAIRED ARM: a body that is NOT submerged is handed back, or she never
/// comes out of the trapdoor.
#[test]
fn a_body_that_surfaced_is_handed_back() {
    assert_eq!(run(false, Visibility::Hidden), Visibility::Inherited);
}

/// ⛔⛔ AND IT IS HANDED BACK AS `Inherited`, NOT `Visible`. A death overlay and
/// a room fade both hide bodies through the parent; a hard `Visible` would make
/// a fighter who surfaced mid-fade the one thing still on screen.
#[test]
fn the_restore_never_forces_a_body_visible_over_its_parent() {
    assert_ne!(run(false, Visibility::Hidden), Visibility::Visible);
}

/// ⛔ AND A BODY NOBODY IS HIDING IS LEFT ALONE, so this system cannot be the
/// reason something else's `Visible` became `Inherited`.
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

/// ⭐⭐ THE OTHER HALF OF HIDING HER. Jon, 2026-08-28: *"There should be a
/// trapdoor sprite she is replaced with on the ground."* Hiding the body left
/// nothing at all on stage, which makes a move whose cost is being readable
/// free.
///
/// ⛔ THE DOOR IS AT HER FEET. She never moves along gravity while submerged, so
/// the feet line is the surface she is under; drawing at her centre floats it
/// half a body above the boards.
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

/// ⛔ THE PAIRED ARM, and it is the one whose absence leaves a door on the stage
/// forever: coming up takes the boards with her.
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

/// ⛔⛔ ONE DOOR PER BODY. `morph_ball.rs` next door is a singleton and its own
/// comments record what that cost — a versus match has four fighters, and any of
/// them may be holding this move.
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
