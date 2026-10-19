use super::*;
use ambition_platformer2d_core as ae;
use ambition_sim_view::{ActorAnimFrame, ActorAnimIndex, FeatureView, FeatureViewIndex, PassageLookView};

const LOOK: PassageLookView = PassageLookView { scale: 0.5, alpha: 0.4, shade: 0.5 };

fn actor_view(height: f32) -> FeatureView {
    FeatureView {
        pos: ae::Vec2::ZERO,
        size: ae::Vec2::new(20.0, height),
        kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
        visible: true,
        submerged: false,
        wire_anchor: None,
        grab_reach: None,
        line_anchor: None,
        limb_host: None,
        depth_plane: ae::DepthPlane::PLAYABLE,
        flash: false,
        breakable_state: None,
        chest_opened: false,
        fighting: false,
        switch_on: false,
        rotation_rad: 0.0,
        alive: true,
        hit_flash_secs: 0.0,
        parry_flash_secs: 0.0,
        hp_current: 1,
        hp_max: 1,
        training_dummy: false,
        hit_strength: 0.0,
        unhittable: false,
        defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
        sprite_offset: None,
        room: None,
    }
}

fn frame(passage_look: Option<PassageLookView>) -> ActorAnimFrame {
    ActorAnimFrame {
        anim: ambition_sprite_sheet::character::CharacterAnim::Idle,
        conversation_held: false,
        pos: ae::Vec2::ZERO,
        facing: 1.0,
        gravity_dir: ae::DEFAULT_GRAVITY_DIR,
        clip: None,
        move_charge: None,
        passage_look,
    }
}

fn drawn(app: &App, body: Entity) -> (Vec3, f32, Srgba) {
    let transform = app.world().get::<Transform>(body).expect("a transform");
    let sprite = app.world().get::<Sprite>(body).expect("a sprite");
    (transform.scale, transform.translation.y, sprite.color.to_srgba())
}

/// Review finding #3 (2026-10-19). A body drawn on the actor road (a
/// possessed body, an NPC) in a passage is drawn at its passage's size,
/// opacity and shade, about its feet; when the passage is over it is drawn at
/// its own size again. Measured before: the look was applied to player bodies
/// only, so a possessed body walked through a door at full size. Poison:
/// filter the actor query out.
#[test]
fn a_body_drawn_as_a_feature_takes_its_passage_look() {
    let mut app = App::new();
    app.add_systems(Update, apply_passage_looks);
    app.insert_resource(ActorAnimIndex::from_rows([("npc".to_string(), frame(Some(LOOK)))]));
    app.insert_resource(FeatureViewIndex::from_rows([("npc".to_string(), actor_view(40.0))]));
    let body = app
        .world_mut()
        .spawn((FeatureVisual { id: "npc".to_string() }, Transform::from_xyz(0.0, 100.0, 0.0), Sprite::default()))
        .id();
    app.update();
    let (scale, y, color) = drawn(&app, body);
    assert_eq!(scale, Vec3::new(0.5, 0.5, 1.0), "the body is drawn at its passage's size");
    assert!((y - 90.0).abs() < 1e-4, "about its feet: half of the 40 px body, halved, is 10 px down; y is {y}");
    assert!((color.alpha - 0.4).abs() < 1e-4 && (color.red - 0.5).abs() < 1e-4, "opacity and shade: {color:?}");

    // The passage is over. `sync_visuals` places the body again each frame.
    app.insert_resource(ActorAnimIndex::from_rows([("npc".to_string(), frame(None))]));
    app.world_mut().get_mut::<Transform>(body).expect("a transform").translation.y = 100.0;
    app.update();
    let (scale, y, _) = drawn(&app, body);
    assert_eq!((scale, y), (Vec3::ONE, 100.0), "after the passage the body is drawn at its own size");
    assert!(app.world().get::<PassageScaled>(body).is_none());
}

/// Control: a player body still takes the look beside its pose.
#[test]
fn a_player_body_takes_its_passage_look() {
    let mut app = App::new();
    app.add_systems(Update, apply_passage_looks);
    let pose = ambition_sim_view::BodyPoseView { size: ae::Vec2::new(20.0, 40.0), ..Default::default() };
    let body = app
        .world_mut()
        .spawn((PlayerVisual, pose, LOOK, Transform::from_xyz(0.0, 100.0, 0.0), Sprite::default()))
        .id();
    app.update();
    let (scale, y, color) = drawn(&app, body);
    assert_eq!(scale, Vec3::new(0.5, 0.5, 1.0));
    assert!((y - 90.0).abs() < 1e-4 && (color.alpha - 0.4).abs() < 1e-4, "y {y}, {color:?}");
}
