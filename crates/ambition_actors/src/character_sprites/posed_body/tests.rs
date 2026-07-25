//! The sheet-as-body-authority seam, exercised against the real baked
//! `solid_snake` manifest — the sheet that motivates it (a body whose
//! silhouette is a long serpent in one pose and a small box in another).

use ambition_engine_core as ae;
use ambition_sprite_sheet::character::CharacterAnim;

use super::*;

const SNAKE: &str = "solid_snake";
/// Same scale Mary-O authors, so the numbers here are the shipped ones.
const SCALE: f32 = 0.5;

fn geometry(anim: CharacterAnim) -> PosedBodyGeometry {
    posed_body_geometry(SNAKE, anim, SCALE)
        .expect("the baked solid_snake sheet publishes per-animation body metrics")
}

#[test]
fn withdrawing_into_the_shell_shrinks_the_body() {
    let walking = geometry(CharacterAnim::Idle);
    let boxed = geometry(CharacterAnim::ShellIdle);
    // The whole point: a stomped snake is a SMALLER THING to run into, kick,
    // and stand on. If these ever coincide the sheet stopped publishing per-pose
    // metrics and every box silently collapsed onto the idle bbox.
    assert!(
        boxed.collision.x < walking.collision.x * 0.6,
        "a boxed snake should be far narrower than a sprawled one; \
         boxed {:?} vs walking {:?}",
        boxed.collision,
        walking.collision
    );
    assert!(
        boxed.collision.y < walking.collision.y,
        "a boxed snake should be shorter than a sprawled one; \
         boxed {:?} vs walking {:?}",
        boxed.collision,
        walking.collision
    );
}

#[test]
fn the_sprite_quad_is_the_same_frame_in_every_pose() {
    // Only the BOX moves between poses; the drawn frame is fixed. Otherwise the
    // sprite would visibly pulse as the state machine advanced, and the renderer
    // would rebind its atlas on every pose change.
    let quads: Vec<_> = [
        CharacterAnim::Idle,
        CharacterAnim::Retreat,
        CharacterAnim::ShellIdle,
        CharacterAnim::Peek,
        CharacterAnim::Emerge,
    ]
    .into_iter()
    .map(|anim| geometry(anim).render)
    .collect();
    assert!(
        quads.windows(2).all(|w| w[0] == w[1]),
        "every pose must draw the same quad; got {quads:?}"
    );
}

#[test]
fn the_quad_placement_puts_the_art_on_the_box() {
    // The offset's contract: shifting the frame by it makes the pose's authored
    // rectangle land exactly on the collision box, which is centred on the body.
    // Recompute that from the manifest independently of the helper so a sign
    // flip in either direction fails here rather than looking plausible on
    // screen.
    let record = record_for_target(SNAKE).expect("baked snake record");
    let metrics = record.body_metrics.as_ref().expect("snake body metrics");
    for anim in [CharacterAnim::Idle, CharacterAnim::ShellIdle] {
        let bbox = metrics.pose_body_bbox(anim).expect("pose bbox");
        let geometry = geometry(anim);
        // Frame top-left, in world units, once the quad is placed.
        let frame_origin = geometry.sprite_offset - geometry.render * 0.5;
        let bbox_origin = frame_origin + ae::Vec2::new(bbox.x as f32, bbox.y as f32) * SCALE;
        let box_origin = -geometry.collision * 0.5;
        assert!(
            (bbox_origin - box_origin).length() < 1e-3,
            "{anim:?}: the art's rectangle must land on the collision box; \
             art at {bbox_origin:?}, box at {box_origin:?}"
        );
    }
}

#[test]
fn a_sheet_that_publishes_nothing_yields_nothing() {
    // The caller keeps its authored box. Inflating every collision box to the
    // whole frame would be a silent, game-wide geometry change.
    assert!(posed_body_geometry("no_such_sheet_target", CharacterAnim::Idle, SCALE).is_none());
}

#[test]
fn an_unpublished_pose_falls_back_to_the_static_body_box() {
    // `Jump` is not a row the snake sheet carries. It must resolve to the sheet's
    // overall body bbox rather than vanishing — a body has geometry in every
    // pose, published or not.
    let jump = posed_body_geometry(SNAKE, CharacterAnim::Jump, SCALE).expect("fallback geometry");
    let record = record_for_target(SNAKE).expect("baked snake record");
    let bbox = record
        .body_metrics
        .as_ref()
        .and_then(|m| m.body_pixel_bbox)
        .expect("static body bbox");
    assert_eq!(
        jump.collision,
        ae::Vec2::new(bbox.w as f32, bbox.h as f32) * SCALE
    );
}

/// A resize must hold the body's FEET, not its centre — the difference between
/// a snake that withdraws in place and one that drops half a box into the floor
/// (or is shoved out of it, which this project does not do).
#[test]
fn the_resize_holds_the_feet() {
    let mut app = bevy::prelude::App::new();
    let walking = geometry(CharacterAnim::Idle);
    let boxed = geometry(CharacterAnim::ShellIdle);
    let ground_y = 100.0;
    let entity = app
        .world_mut()
        .spawn((
            SpritePosedBody::new(SNAKE, SCALE),
            ActorAnimOverride(CharacterAnim::ShellIdle),
            ae::BodyKinematics {
                pos: ae::Vec2::new(0.0, ground_y - walking.collision.y * 0.5),
                vel: ae::Vec2::ZERO,
                size: walking.collision,
                facing: 1.0,
            },
        ))
        .id();
    app.add_systems(bevy::prelude::Update, sync_sprite_posed_bodies);
    app.update();

    let kin = app
        .world()
        .get::<ae::BodyKinematics>(entity)
        .copied()
        .expect("kinematics");
    assert_eq!(kin.size, boxed.collision, "the box follows the pinned pose");
    assert!(
        (kin.pos.y + kin.size.y * 0.5 - ground_y).abs() < 1e-3,
        "the +gravity face must not move: feet at {}, ground at {ground_y}",
        kin.pos.y + kin.size.y * 0.5
    );
    assert!(
        kin.pos.x.abs() < 1e-3,
        "a horizontal resize is centred, not feet-anchored"
    );
}

#[test]
fn the_pose_pin_drives_the_geometry_the_renderer_is_told_about() {
    let mut app = bevy::prelude::App::new();
    let entity = app
        .world_mut()
        .spawn((
            SpritePosedBody::new(SNAKE, SCALE),
            ae::BodyKinematics::default(),
        ))
        .id();
    app.add_systems(bevy::prelude::Update, sync_sprite_posed_bodies);
    app.update();
    let unpinned = app
        .world()
        .get::<ActorSpriteOffset>(entity)
        .copied()
        .expect("an opted-in body publishes its quad placement");

    app.world_mut()
        .entity_mut(entity)
        .insert(ActorAnimOverride(CharacterAnim::ShellIdle));
    app.update();
    let pinned = app
        .world()
        .get::<ActorSpriteOffset>(entity)
        .copied()
        .expect("placement survives the pose change");
    assert_ne!(
        unpinned.0, pinned.0,
        "the quad must MOVE when the pose does — a fixed offset would draw the \
         withdrawn box wherever the sprawled snake's art happened to sit"
    );
    assert_eq!(
        app.world().get::<ActorRenderSize>(entity).map(|r| r.0),
        Some(geometry(CharacterAnim::ShellIdle).render),
        "the quad size is published too, so the renderer never re-derives it"
    );
}
