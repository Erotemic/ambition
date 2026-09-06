//! The sheet-as-body-authority seam, exercised against the real baked
//! `solid_snake` manifest — the sheet that motivates it (a body whose
//! silhouette is a long serpent in one pose and a small box in another).

use ambition_platformer2d_core as ae;
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
    // Otherwise the sprite would visibly pulse as the state machine advanced, and the renderer
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
    let record = record_for_sheet_key(SNAKE).expect("baked snake record");
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
    let record = record_for_sheet_key(SNAKE).expect("baked snake record");
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

/// A crouching body stays crouched.
///
/// The stance is applied ONCE, on the tick the mode changes: the crouch
/// mechanics `continue` when the body is already in the target mode, so nothing
/// re-asserts the shorter box on later ticks. This pass, meanwhile, runs every
/// tick. Writing the pose's standing rectangle straight into `kin.size` was
/// therefore not "keeping the box in step with the art" — it was silently
/// undoing every stance the moment it stopped changing, leaving the body mode
/// saying `Crouching` and the collider standing at full height.
///
/// The pose says how big the body is; the MODE says what it is doing with it.
/// Both are facts, and the box is the composition of the two.
#[test]
fn a_stance_survives_the_per_tick_resync() {
    let mut app = bevy::prelude::App::new();
    let standing = geometry(CharacterAnim::Idle);
    let entity = app
        .world_mut()
        .spawn((
            SpritePosedBody::new(SNAKE, SCALE),
            ae::BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                size: standing.collision,
                facing: 1.0,
            },
            ae::BodyModeState {
                body_mode: ae::BodyMode::Crouching,
                ..Default::default()
            },
        ))
        .id();
    app.add_systems(bevy::prelude::Update, sync_sprite_posed_bodies);
    // Twice: once to settle, once to prove it is not a first-tick effect.
    app.update();
    app.update();

    let kin = app
        .world()
        .get::<ae::BodyKinematics>(entity)
        .copied()
        .expect("kinematics");
    let crouched = ae::BodyMode::Crouching.shape(standing.collision).size;
    assert_eq!(
        kin.size, crouched,
        "a crouching body was resynced back to its standing box ({:?}) — the \
         stance is applied once, on the mode change, and nothing re-asserts it",
        standing.collision,
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

/// Where a body's ART stands, for one stance. The two facts the renderer is
/// handed — `ActorRenderSize` and `ActorSpriteOffset` — rebuilt into the world
/// rectangle the sheet's body occupies once the quad is placed, the way
/// `sync_visuals` places it: the quad is `render`, drawn centred at
/// `pos + offset`.
fn drawn_body_rect(mode: ae::BodyMode, ground: f32) -> (ae::Vec2, ae::Vec2, f32) {
    let standing = geometry(CharacterAnim::Idle);
    let mut app = bevy::prelude::App::new();
    let entity = app
        .world_mut()
        .spawn((
            SpritePosedBody::new(SNAKE, SCALE),
            ae::BodyKinematics {
                pos: ae::Vec2::new(0.0, ground - standing.collision.y * 0.5),
                vel: ae::Vec2::ZERO,
                size: standing.collision,
                facing: 1.0,
            },
            ae::BodyBaseSize {
                base_size: standing.collision,
            },
            ae::BodyModeState {
                body_mode: mode,
                ..Default::default()
            },
        ))
        .id();
    app.add_systems(bevy::prelude::Update, sync_sprite_posed_bodies);
    // Twice: the stance is applied ONCE, on the tick the mode changes, while this
    // pass runs every tick. A placement that only survives the first tick is not
    // a placement.
    app.update();
    app.update();

    let kin = *app
        .world()
        .get::<ae::BodyKinematics>(entity)
        .expect("kinematics");
    let render = app
        .world()
        .get::<ActorRenderSize>(entity)
        .expect("the quad size is published")
        .0;
    let offset = app
        .world()
        .get::<ActorSpriteOffset>(entity)
        .expect("the quad placement is published")
        .0;
    // Rebuilt from the manifest rather than from the helper, so a sign flip in
    // either fails here instead of looking plausible on screen.
    let record = record_for_sheet_key(SNAKE).expect("baked snake record");
    let metrics = record.body_metrics.as_ref().expect("snake body metrics");
    let bbox = metrics
        .pose_body_bbox(CharacterAnim::Idle)
        .expect("pose bbox");
    let frame_origin = kin.pos + offset - render * 0.5;
    let art_min = frame_origin + ae::Vec2::new(bbox.x as f32, bbox.y as f32) * SCALE;
    let art_max = art_min + ae::Vec2::new(bbox.w as f32, bbox.h as f32) * SCALE;
    (art_min, art_max, kin.pos.y + kin.size.y * 0.5)
}

/// The ground a crouching body's ART stands on must be the ground its COLLIDER
/// stands on.
///
/// Two facts are published from one centre. `resize_feet_planted` holds the
/// +gravity face and slides `pos` toward the feet by half the stance shrink, so
/// a crouching body's centre is no longer the centre of the rectangle the SHEET
/// measured. The quad's placement is an offset from that centre — so an offset
/// derived from the sheet rectangle alone draws the art where a STANDING body's
/// centre would have put it, a quarter of the body's height into the floor.
///
/// Guards the OUTPUT — where the art lands — rather than the arithmetic that
/// gets it there.
#[test]
fn a_crouching_body_draws_its_art_on_the_ground_its_collider_stands_on() {
    let ground = 100.0_f32;
    for mode in [ae::BodyMode::Standing, ae::BodyMode::Crouching] {
        let (_, art_max, collider_foot) = drawn_body_rect(mode, ground);
        // The collision half: the stance composes with the pose, and the resize
        // holds the feet.
        assert!(
            (collider_foot - ground).abs() < 1e-3,
            "{mode:?}: the collider's feet left the ground \
             (foot {collider_foot}, ground {ground})"
        );
        // The placement half: so does the art.
        assert!(
            (art_max.y - ground).abs() < 1e-3,
            "{mode:?}: the drawn body's feet are {:.3} below the ground its \
             collider stands on (art foot {}, ground {ground}) — the art sinks \
             through the floor",
            art_max.y - ground,
            art_max.y,
        );
    }
}

/// A stance must not silently resize the drawn body.
///
/// The quad is the whole sheet frame in every pose, and the offset is a
/// TRANSLATION. A "fix" that scaled the art to the shorter box would put the
/// feet on the ground while squashing a body whose sheet authored real crouch
/// art — so the rectangle's extents are pinned alongside its foot line.
#[test]
fn a_stance_moves_the_art_without_resizing_it() {
    let ground = 100.0_f32;
    let (stand_min, stand_max, _) = drawn_body_rect(ae::BodyMode::Standing, ground);
    let (crouch_min, crouch_max, _) = drawn_body_rect(ae::BodyMode::Crouching, ground);
    assert!(
        ((stand_max - stand_min) - (crouch_max - crouch_min)).length() < 1e-3,
        "the drawn body changed size with the stance: standing {:?}, crouching {:?}",
        stand_max - stand_min,
        crouch_max - crouch_min
    );
}
