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
    ambition_sim_view::FeatureView {
        pos: AT,
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

/// Draw the rider and the giant under it, at one spot, through `sync_visuals`;
/// return their z.
fn drawn_z(giant: ae::DepthPlane) -> (f32, f32) {
    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "depth plane test world",
            ae::Vec2::new(1600.0, 900.0),
            AT,
            Vec::new(),
        )),
    );
    app.insert_resource(FeatureViewIndex::from_rows([
        ("rider".to_string(), view(ae::DepthPlane::PLAYABLE)),
        ("giant".to_string(), view(giant)),
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
    // The control: in one plane nothing orders them (both at the actor layer).
    let (rider, giant) = drawn_z(ae::DepthPlane::PLAYABLE);
    assert_eq!(giant, rider, "the premise: the plane, and only the plane, separates them");
}
