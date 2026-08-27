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
