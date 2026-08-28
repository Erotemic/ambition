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
// The marker that stands in for her.
// ---------------------------------------------------------------------------

fn marker_app(facts: Vec<ambition_sim_view::SubmergedMarkerFact>) -> App {
    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
            "t",
            ambition_platformer2d_core::Vec2::new(640.0, 480.0),
            ambition_platformer2d_core::Vec2::ZERO,
            Vec::new(),
        )),
    );
    app.insert_resource(ambition_sim_view::SubmergedMarkersView(facts));
    app.world_mut().spawn((
        SubmergedMarkerVisual,
        Sprite::default(),
        Transform::default(),
        Visibility::Hidden,
    ));
    app.add_systems(Update, sync_submerged_markers);
    app
}

fn fact(x: f32) -> ambition_sim_view::SubmergedMarkerFact {
    ambition_sim_view::SubmergedMarkerFact {
        pos: ambition_platformer2d_core::Vec2::new(x, 100.0),
        size: ambition_platformer2d_core::Vec2::new(32.0, 64.0),
        gravity_dir: ambition_platformer2d_core::Vec2::new(0.0, 1.0),
    }
}

fn marker(app: &mut App) -> (Visibility, f32) {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Visibility, &Transform), With<SubmergedMarkerVisual>>();
    let (vis, transform) = q.iter(app.world()).next().expect("one marker");
    (*vis, transform.translation.x)
}

/// ⭐⭐ THE INSTRUCTION. Jon, 2026-08-27: *"you should just see an unopened trap
/// door move around to indicate where she is."* A hidden body with nothing drawn
/// in its place is a fighter who has simply vanished.
#[test]
fn a_submerged_body_gets_a_hatch_where_it_is() {
    let mut app = marker_app(vec![fact(120.0)]);
    app.update();
    assert_eq!(marker(&mut app).0, Visibility::Visible);
}

/// ⛔⛔ AND IT MOVES WITH HER, which is the entire reason it is drawn: it is how
/// the player knows where they are steering. A marker pinned where she went
/// UNDER would be a lie the moment the mode did its job.
#[test]
fn the_hatch_follows_her_under_the_stage() {
    let mut app = marker_app(vec![fact(120.0)]);
    app.update();
    let first = marker(&mut app).1;
    app.world_mut()
        .resource_mut::<ambition_sim_view::SubmergedMarkersView>()
        .0 = vec![fact(300.0)];
    app.update();
    let second = marker(&mut app).1;
    assert!(
        (second - first).abs() > 100.0,
        "the hatch stayed at {first} while she travelled to 300"
    );
}

/// ⛔ AND IT GOES AWAY WHEN SHE SURFACES, or the stage collects a hatch per
/// press.
#[test]
fn the_hatch_is_put_away_when_she_comes_up() {
    let mut app = marker_app(vec![fact(120.0)]);
    app.update();
    assert_eq!(marker(&mut app).0, Visibility::Visible);
    app.world_mut()
        .resource_mut::<ambition_sim_view::SubmergedMarkersView>()
        .0 = Vec::new();
    app.update();
    assert_eq!(marker(&mut app).0, Visibility::Hidden);
}

/// ⛔ THE HATCH IS DRAWN BEHIND THE BODIES STILL ON THE STAGE. She is under the
/// floor; a marker over the fighter standing on top of her reads as an object in
/// the air rather than a hatch in the ground.
#[test]
fn the_hatch_sits_behind_the_fighters_above_it() {
    assert!(MARKER_Z < ambition_platformer2d_core::config::WORLD_Z_PLAYER);
}
