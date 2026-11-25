//! A body behind the playable plane is DRAWN behind the bodies in it.
//!
//! Jon, 2026-09-26: GNU-ton's scholar was drawn behind the giant gnu he rides,
//! so he could not be seen in the fight. Every actor drew at one z and the tie
//! went to whichever sprite the renderer sorted last. The gnu already stands in
//! `DepthPlane::BEHIND` (no swing reaches it), and that is the signal the draw
//! order now reads.

use super::*;

const AT: ae::Vec2 = ae::Vec2::new(300.0, 500.0);

fn view(depth_plane: ae::DepthPlane) -> ambition_sim_view::FeatureView {
    view_at(AT, depth_plane)
}

/// A fighting actor's view row at `pos`, in `depth_plane`.
pub(super) fn view_at(pos: ae::Vec2, depth_plane: ae::DepthPlane) -> ambition_sim_view::FeatureView {
    ambition_sim_view::FeatureView {
        pos,
        size: ae::Vec2::new(60.0, 110.0),
        kind: FeatureVisualKind::Actor,
        visible: true,
        submerged: false,
        wire_anchor: None,
        grab_reach: None,
        line_anchor: None,
        limb_host: None,
        depth_plane,
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
        room: None,
        order: Default::default(),
    }
}

/// Draw the rider and the giant under it, at one spot, through `sync_visuals`;
/// return their z.
fn drawn_z(giant: ae::DepthPlane) -> (f32, f32) {
    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "depth plane test world",
            ae::Vec2::new(1600.0, 900.0),
            AT,
            Vec::new(),
        )),
    );
    let scoped = |depth| {
        let mut row = view(depth);
        row.room = Some(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION);
        row
    };
    app.insert_resource(FeatureViewIndex::from_rows([
        ("rider".to_string(), scoped(ae::DepthPlane::PLAYABLE)),
        ("giant".to_string(), scoped(giant)),
    ]));
    app.init_resource::<ambition_sim_view::PresentedFeaturePoses>();
    app.add_systems(Update, sync_visuals);
    let spawn = |app: &mut App, id: &str| {
        app.world_mut()
            .spawn((
                FeatureVisual { id: id.to_string() },
                Transform::default(),
                Sprite::default(),
                Visibility::Visible,
            ))
            .id()
    };
    let rider = spawn(&mut app, "rider");
    let giant_visual = spawn(&mut app, "giant");
    app.update();
    let z = |e: Entity| app.world().get::<Transform>(e).expect("a drawn feature").translation.z;
    (z(rider), z(giant_visual))
}

#[test]
fn a_body_behind_the_playable_plane_draws_behind_the_rider_on_it_and_over_the_limb_trail() {
    let (rider, giant) = drawn_z(ae::DepthPlane::BEHIND);
    assert!(giant < rider, "the gnu draws under the scholar on its shoulders: gnu {giant}, scholar {rider}");
    assert!(
        giant > crate::rendering::limb_trail::TRAIL_Z,
        "and over its fists' trail, which comes out from behind it: gnu {giant}, trail {}",
        crate::rendering::limb_trail::TRAIL_Z
    );
    // The control: in one plane only their occurrence tie orders them (two
    // order blocks of the actor layer), never as much as a plane does.
    let (rider, giant) = drawn_z(ae::DepthPlane::PLAYABLE);
    assert!(
        (giant - rider).abs() < crate::rendering::primitives::DEPTH_PLANE_Z_STEP,
        "the premise: the plane, and only the plane, separates them by a plane: gnu {giant}, scholar {rider}"
    );
}

/// Draw a rider and its mount at one spot in one plane, each with its
/// published order block; return their z. The rider's visual is spawned
/// first, so ordered by drawn entity it would take the lower tie.
fn drawn_rider_and_mount(rider: ambition_sim_view::OrderBlock, mount: ambition_sim_view::OrderBlock) -> (f32, f32) {
    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new("seat test world", ae::Vec2::new(1600.0, 900.0), AT, Vec::new())),
    );
    let row = |order| {
        let mut row = view(ae::DepthPlane::PLAYABLE);
        row.room = Some(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION);
        row.order = order;
        row
    };
    app.insert_resource(FeatureViewIndex::from_rows([
        ("rider".to_string(), row(rider)),
        ("mount".to_string(), row(mount)),
    ]));
    app.init_resource::<ambition_sim_view::PresentedFeaturePoses>();
    app.add_systems(Update, sync_visuals);
    let spawn = |app: &mut App, id: &str| {
        app.world_mut()
            .spawn((FeatureVisual { id: id.to_string() }, Transform::default(), Sprite::default(), Visibility::Visible))
            .id()
    };
    let rider_visual = spawn(&mut app, "rider");
    let mount_visual = spawn(&mut app, "mount");
    app.update();
    let z = |e: Entity| app.world().get::<Transform>(e).expect("a drawn feature").translation.z;
    (z(rider_visual), z(mount_visual))
}

/// PO2: an active rider is drawn as a seat of its mount's order block, just
/// over the mount, whichever simulation bodies they are (the tie wraps at
/// its span), and within the span the layer's overlays are drawn above.
#[test]
fn an_active_rider_draws_just_over_the_mount_it_sits() {
    use ambition_sim_view::OrderBlock;
    for mount_body in [0_u32, 3, 15, 16, 31, 1_000] {
        let (rider, mount) = drawn_rider_and_mount(
            OrderBlock { body: Some(mount_body), seat: 1 },
            OrderBlock { body: Some(mount_body), seat: 0 },
        );
        assert!(rider > mount, "mount body {mount_body}: the rider draws over its mount: rider {rider}, mount {mount}");
        assert!(rider - mount < 2.0 * crate::rendering::primitives::OCCURRENCE_SPAN,
            "and as its seat, not a plane away: rider {rider}, mount {mount}");
    }
    // The control: two bodies of their own blocks keep their own ties, so a
    // rider that is not seated can draw under the body beside it.
    let (alone, beside) = drawn_rider_and_mount(
        OrderBlock { body: Some(2), seat: 0 },
        OrderBlock { body: Some(9), seat: 0 },
    );
    assert!(alone < beside, "unseated, the lower block draws under: {alone} vs {beside}");
}
