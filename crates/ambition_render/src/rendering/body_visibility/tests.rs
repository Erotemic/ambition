use super::*;

#[derive(Clone, Copy, Debug, Default)]
struct Pose {
    morph_ball: bool,
    submerged: bool,
    out_of_the_world: bool,
}

fn view(pose: Pose) -> ambition_sim_view::BodyPoseView {
    ambition_sim_view::BodyPoseView {
        morph_ball: pose.morph_ball,
        submerged: pose.submerged,
        out_of_the_world: pose.out_of_the_world,
        ..Default::default()
    }
}

/// The body sprite after `frames` frames of the one visibility writer.
fn after(pose: Pose, start: Visibility, frames: usize) -> Visibility {
    let mut app = App::new();
    let body = app.world_mut().spawn((PlayerVisual, view(pose), start)).id();
    app.add_systems(Update, sync_body_visibility);
    for _ in 0..frames {
        app.update();
    }
    *app.world().entity(body).get::<Visibility>().expect("visibility")
}

/// A morphed body stays hidden on every frame (the submerged rule used to show
/// it again). Control: the same body standing is not hidden. The shipped
/// witness is `a_morphed_body_draws_only_its_ball` in the app tests.
#[test]
fn a_morphed_body_stays_hidden_frame_after_frame() {
    let morphed = Pose { morph_ball: true, ..Pose::default() };
    for frames in [1, 2, 10] {
        assert_eq!(after(morphed, Visibility::Inherited, frames), Visibility::Hidden, "after {frames} frame(s)");
    }
    assert_eq!(after(Pose::default(), Visibility::Inherited, 10), Visibility::Inherited, "control: standing");
}

/// Each pose that takes the body away hides it, alone and together.
#[test]
fn each_pose_that_replaces_the_body_hides_it() {
    for pose in [
        Pose { morph_ball: true, ..Pose::default() },
        Pose { submerged: true, ..Pose::default() },
        Pose { out_of_the_world: true, ..Pose::default() },
        Pose { morph_ball: true, submerged: true, out_of_the_world: false },
        Pose { morph_ball: true, submerged: false, out_of_the_world: true },
    ] {
        assert_eq!(after(pose, Visibility::Inherited, 2), Visibility::Hidden, "{pose:?}");
    }
}

/// Leaving the pose hands the body back as `Inherited`, never `Visible`: a
/// death overlay and a room fade hide bodies through the parent.
#[test]
fn a_body_that_leaves_the_pose_is_handed_back_as_inherited() {
    assert_eq!(after(Pose::default(), Visibility::Hidden, 1), Visibility::Inherited);
}

/// A body nobody hides is left alone, so another owner's `Visible` stays.
#[test]
fn a_visible_body_is_not_touched() {
    assert_eq!(after(Pose::default(), Visibility::Visible, 2), Visibility::Visible);
}
