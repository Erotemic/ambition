//! A pose that names her new form while carrying the old one's body SAYS so.
//!
//! `sync_grown_form` swaps her identity late in a tick and the tall form's
//! prepared body is granted at the head of the next, so for one published pose
//! she is `mary_o_tall` at the small form's standing box and offset. A
//! presentation that finalizes from that snapshot keeps the small form's
//! offset for as long as the binding lives (`BodyPoseView::geometry`).

use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::sim_view::{BodyPoseView, PoseGeometry};
use bevy::prelude::*;

#[test]
fn the_pose_on_the_swap_tick_is_pending_and_settles_on_the_tall_body() {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    let player = (0..600)
        .find_map(|_| {
            app.update();
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, (With<PrimaryPlayer>, With<BodyPoseView>)>();
            q.iter(app.world()).next()
        })
        .expect("the demo never published a pose for a playable body");
    for _ in 0..30 {
        app.update();
    }
    let read = |app: &App| {
        let world = app.world();
        let worn = world.get::<WornCharacter>(player).expect("worn").id().to_string();
        let pose = world.get::<BodyPoseView>(player).expect("pose");
        (worn, pose.geometry, pose.base_size, pose.authored_offset)
    };
    let (small_id, small_geometry, small_base, small_offset) = read(&app);
    assert_eq!(small_geometry, PoseGeometry::Settled, "she starts settled as {small_id}");

    app.world_mut()
        .entity_mut(player)
        .insert(WornEquipment::new(vec![
            ambition_demo_mary_o::powerups::star_wand(),
        ]));
    app.update();
    let (swap_id, swap_geometry, swap_base, swap_offset) = read(&app);
    // ⛔ PREMISE: the swap tick is the incomplete snapshot this guards — a new
    // identity over the old body. Without it the arm below proves nothing.
    assert_ne!(swap_id, small_id, "the wand did not change her form this tick");
    assert_eq!(
        (swap_base, swap_offset),
        (small_base, small_offset),
        "the pose already carries the new form's body on the swap tick, so \
         there is no window for this test to witness"
    );
    assert_eq!(
        swap_geometry,
        PoseGeometry::Pending,
        "{swap_id} was published as settled while carrying the small form's \
         body ({swap_base:?}, offset {swap_offset:?})"
    );

    app.update();
    let (settled_id, settled_geometry, settled_base, settled_offset) = read(&app);
    assert_eq!(settled_id, swap_id);
    assert_eq!(settled_geometry, PoseGeometry::Settled);
    assert!(
        settled_base.y > small_base.y && settled_offset != small_offset,
        "the settled pose is not the tall body: base {settled_base:?}, offset {settled_offset:?}"
    );
}
