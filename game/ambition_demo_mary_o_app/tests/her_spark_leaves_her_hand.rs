//! Her spark is born at the hand that throws it.
//!
//! Jon, 2026-10-09: "improve her fireball animation / posing. She should
//! shoot it from her hand."
//!
//! Measured before (2026-10-09), two defects:
//!
//! - The spark stated no muzzle, so it took `Muzzle::BodyOrigin`: it was born
//!   8 px above the centre of her body, inside her dress.
//! - Her fire sheet named its throw row `fireball`. The engine has no row of
//!   that name (`CharacterAnim::from_name`), so the sheet spec dropped it and
//!   she showed no throw at all. The row is `shoot` now.
//!
//! ⚠ The hand these tests expect does not come from the muzzle. It comes from
//! her published body rig (`mary_o_v2_fire_body_rig.ron`, solved by the sprite
//! publisher from the same poses as her frames): the near hand in the middle
//! frame of `shoot`. The shipped demo does not admit body rigs, so the spark
//! states its muzzle as numbers (`powerups::SPARK_MUZZLE`). A throw row that is
//! posed again and leaves the numbers behind fails here.

use bevy::math::Vec2;
use bevy::prelude::*;

use ambition_demo_mary_o::powerups::cinder_beacon;
use ambition_demo_mary_o::test_course::TEST_COURSE_ROOM_ID;
use ambition_platformer2d::characters::actor::Landmark;
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::combat::body_landmarks::{rig_landmark, rig_point_in_world, LandmarkPose};
use ambition_platformer2d::combat::body_rig::BodyRig;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::projectiles::ProjectileSpawnRequest;

/// The row a firing body shows, and the part of it the shot leaves the hand
/// in: the rule of the engine's own hand muzzle (`SHOOT_CLIPS`,
/// `SHOOT_RELEASE_PHASE` in `projectile::systems`).
const SHOOT_ROW: &str = "shoot";
const RELEASE_PHASE: f32 = 0.5;

/// How far the spark may be born from the published hand, in px. The muzzle
/// is stated to four decimals of her height.
const TOLERANCE: f32 = 0.5;

const DOWN: Vec2 = Vec2::new(0.0, 1.0);

/// Her published sprites are not in git: each checkout publishes its own. A
/// checkout that pulled the `shoot` row and did not publish reads the old
/// sheet, and these tests fail on it.
const PUBLISH_HER_SPRITES: &str = "If this checkout did not publish her sprites after the pull, run \
    `scripts/regen/sprites.sh --target mary_o_v2 --target mary_o_v2_tall --target mary_o_v2_fire` \
    and build again";

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

/// `app` in the flat fixture course, with her seated, settled and wearing the
/// cinder beacon.
fn in_her_fire_form(mut app: App) -> (App, Entity) {
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(TEST_COURSE_ROOM_ID.to_string()));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    let mut body = None;
    for _ in 0..600 {
        app.update();
        let world = app.world_mut();
        let mut players = world.query_filtered::<Entity, With<PrimaryPlayer>>();
        if let Some(found) = players.iter(world).next() {
            body = Some(found);
            break;
        }
    }
    let body = body.expect("Mary-O never took a seat in her own demo");
    for _ in 0..30 {
        step(&mut app, ControlFrame::default());
    }
    app.world_mut().entity_mut(body).insert(WornEquipment::new(vec![cinder_beacon()]));
    for _ in 0..30 {
        step(&mut app, ControlFrame::default());
    }
    (app, body)
}

/// Her near hand in the release frame of `shoot`, in rig space (from her
/// feet, +x the way she faces, +y down, world units), from her published rig.
fn published_throwing_hand() -> Vec2 {
    let (app, body) = in_her_fire_form(ambition_demo_mary_o_app::build_demo_app_with_body_rigs());
    let rig = app.world().get::<BodyRig>(body).expect(
        "Mary-O has no body rig where rigs are admitted; publish it with \
         `scripts/regen/sprites.sh --target mary_o_v2_fire`",
    );
    assert!(
        rig.0.clip(SHOOT_ROW).is_some(),
        "the rig of her fire form has no `{SHOOT_ROW}` clip: {:?}. {PUBLISH_HER_SPRITES}",
        rig.0.clip_names().collect::<Vec<_>>()
    );
    let row = LandmarkPose::Clip {
        chain: &[SHOOT_ROW],
        phase: RELEASE_PHASE,
    };
    rig_landmark(Landmark::HandNear, row, rig.0.as_ref(), None).expect("her rig has a near hand")
}

/// One spark of the shipped demo: her body on the tick she threw it, where the
/// spark was born and its half extent.
struct Throw {
    body: BodyKinematics,
    born: Vec2,
    half: Vec2,
}

fn requests(app: &App, owner: Entity) -> Vec<(Vec2, Vec2)> {
    app.world()
        .resource::<Messages<ProjectileSpawnRequest>>()
        .iter_current_update_messages()
        .filter(|request| request.owner == owner)
        .map(|request| {
            let kin = &request.projectile.body.kin;
            (kin.pos, kin.size * 0.5)
        })
        .collect()
}

/// She turns to `facing`, stands, and presses the spark button one time.
fn throw_facing(app: &mut App, body: Entity, facing: f32) -> Throw {
    for _ in 0..12 {
        step(
            app,
            ControlFrame {
                axis_x: facing,
                aim_x: facing,
                left_pressed: facing < 0.0,
                right_pressed: facing > 0.0,
                ..ControlFrame::default()
            },
        );
    }
    for _ in 0..90 {
        step(app, ControlFrame::default());
    }
    let before = app.world().get::<BodyKinematics>(body).expect("a live body").clone();
    assert_eq!(before.facing.signum(), facing, "control: she faces the way she was turned");
    assert!(requests(app, body).is_empty(), "control: no spark before the press");
    let mut born = Vec::new();
    step(
        app,
        ControlFrame {
            modifier_held: true,
            modifier_pressed: true,
            ..ControlFrame::default()
        },
    );
    born.extend(requests(app, body));
    let mut waited = 0;
    while born.is_empty() && waited < 30 {
        step(
            app,
            ControlFrame {
                modifier_held: true,
                ..ControlFrame::default()
            },
        );
        born.extend(requests(app, body));
        waited += 1;
    }
    assert_eq!(born.len(), 1, "one press asks for one spark ({waited} ticks after the press): {born:?}");
    let after = app.world().get::<BodyKinematics>(body).expect("a live body").clone();
    assert!(
        (after.pos - before.pos).length() < 0.01,
        "control: she stood still through the press ({:?} -> {:?})",
        before.pos,
        after.pos
    );
    // The throw arms the pose a firing body shows. `her_throw_is_drawn` holds
    // what that pose draws.
    let shows = app
        .world()
        .get::<ambition_platformer2d::characters::actor::BodyAnimFacts>(body)
        .map(|anim| anim.shoot_anim_timer);
    assert!(
        shows.is_some_and(|timer| timer > 0.0),
        "the throw did not arm her shoot pose (timer {shows:?})"
    );
    Throw {
        body: after,
        born: born[0].0,
        half: born[0].1,
    }
}

/// The spark's rear edge is at her throwing hand, on each side she can face.
#[test]
fn her_spark_is_born_at_her_throwing_hand() {
    let hand = published_throwing_hand();
    let (mut app, body) = in_her_fire_form(ambition_demo_mary_o_app::build_demo_app());
    assert!(
        app.world().get::<BodyRig>(body).is_none(),
        "premise: the shipped demo admits no body rig, so the spark states its muzzle"
    );
    for facing in [1.0_f32, -1.0] {
        let throw = throw_facing(&mut app, body, facing);
        let hand_world = rig_point_in_world(hand, &throw.body, DOWN);
        // The hand is not where a shot with no muzzle is born: the arms below
        // cannot pass on `Muzzle::BodyOrigin`.
        let no_muzzle = throw.body.pos + Vec2::new(0.0, -8.0);
        assert!(
            (hand_world.x - throw.body.pos.x) * facing > throw.body.size.x * 0.5,
            "premise: her throwing hand reaches past her box (hand {hand_world:?}, body {:?} of {:?})",
            throw.body.pos,
            throw.body.size
        );
        assert!((hand_world - no_muzzle).length() > 8.0, "premise: the hand is not the body origin");
        let rear = throw.born - Vec2::new(facing * throw.half.x, 0.0);
        assert!(
            (rear - hand_world).length() <= TOLERANCE,
            "facing {facing}: the spark's rear edge was born at {rear:?} and her throwing hand is at \
             {hand_world:?} ({:?} apart; the body origin muzzle is {no_muzzle:?}). In her rig the hand is \
             {hand:?} from her feet and she is {:?}",
            rear - hand_world,
            throw.body.size
        );
    }
}

/// Her fire sheet has the row the engine shows on a firing body.
#[test]
fn her_fire_sheet_has_the_row_a_firing_body_shows() {
    use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

    let sheets = ambition_platformer2d::sprite_sheet::shared_baked_sheet_registry();
    let fire = sheets.get("mary_o_v2_fire").expect("her fire sheet is published");
    let rows: Vec<&str> = fire.rows.iter().map(|row| row.animation.as_str()).collect();
    assert!(
        rows.contains(&SHOOT_ROW),
        "her fire sheet has no `{SHOOT_ROW}` row: {rows:?}. {PUBLISH_HER_SPRITES}"
    );
    for row in &rows {
        assert!(
            CharacterAnim::from_name(row).is_some(),
            "her fire sheet has a row `{row}` that the engine has no name for, so it is never shown"
        );
    }
    assert_eq!(CharacterAnim::from_name(SHOOT_ROW), Some(CharacterAnim::Shoot));
    // Control: the name the row had is not a row, so the loop above refuses it.
    assert_eq!(CharacterAnim::from_name("fireball"), None);
}

/// A throw is drawn from her `shoot` row, and the row is not her idle pose.
///
/// The drawn composition (`visible`). The sim arms the pose with a timer on
/// her body (`her_spark_is_born_at_her_throwing_hand` holds that a throw sets
/// it), so this arm sets the timer and reads what her animator draws.
#[cfg(feature = "visible")]
#[test]
fn her_throw_is_drawn() {
    use ambition_demo_mary_o_app::{build_windowed_demo_app_entering, RenderMode};
    use ambition_platformer2d::characters::actor::BodyAnimFacts;
    use ambition_platformer2d::sprite_sheet::character::{CharacterAnim, CharacterAnimator};

    let mut app = build_windowed_demo_app_entering(
        RenderMode::Headless,
        ambition_demo_mary_o::MARY_O_GAMEPLAY_ROUTE,
        ambition_demo_mary_o::LEVEL_1_1_ROOM_ID,
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    for _ in 0..240 {
        app.update();
    }
    let body = {
        let world = app.world_mut();
        let mut players = world.query_filtered::<Entity, With<PrimaryPlayer>>();
        players.iter(world).next().expect("a player")
    };
    app.world_mut().entity_mut(body).insert(WornEquipment::new(vec![cinder_beacon()]));
    // An enemy of 1-1 walks to where she stands, and a hit takes the form
    // (measured: she was small again 180 frames after she wore the beacon).
    app.world_mut()
        .get_mut::<ambition_platformer2d::characters::actor::BodyHealth>(body)
        .expect("her health")
        .health
        .invulnerable
        .set(ambition_platformer2d::characters::actor::Invulnerability::SCRIPTED, true);
    let shown = |app: &App| {
        let animator = app.world().get::<CharacterAnimator>(body).expect("her animator");
        (animator.current, animator.drawn_row())
    };
    // Past the transform beat of the new form.
    let mut beat = 0;
    while beat < 240 && (beat < 2 || shown(&app).0 == CharacterAnim::Transform) {
        app.update();
        beat += 1;
    }
    let animator = app.world().get::<CharacterAnimator>(body).expect("her animator");
    assert!(
        animator.spec.maps(CharacterAnim::Shoot),
        "premise: she draws from the sheet of her fire form ({:?})",
        animator.spec.mapped_anims().collect::<Vec<_>>()
    );
    let (standing, standing_row) = shown(&app);
    assert_ne!(standing, CharacterAnim::Shoot, "premise: she does not show a throw while she stands");

    app.world_mut().get_mut::<BodyAnimFacts>(body).expect("her pose facts").shoot_anim_timer = 0.18;
    let mut thrown = None;
    for _ in 0..4 {
        app.update();
        let (anim, row) = shown(&app);
        if anim == CharacterAnim::Shoot {
            thrown = Some(row);
            break;
        }
    }
    let thrown_row = thrown.expect("her animator did not show `Shoot` in 4 frames of an armed throw");
    assert_ne!(
        thrown_row, standing_row,
        "her throw is drawn from the row she stands in ({standing:?}): her sheet has no throw row"
    );
    for _ in 0..40 {
        app.update();
    }
    assert_ne!(shown(&app).0, CharacterAnim::Shoot, "the throw ended and she still shows it");
}

/// A spark she throws while she touches a wall does not come out of the far
/// side of the wall.
///
/// The hand of her throw is in front of her body box, so against a wall the
/// spark is born inside the wall. The left wall of the fixture course is one
/// tile (32 px) thick, from x = -32 to x = 0.
///
/// Measured 2026-10-09: the spark born in the wall is gone on its first tick
/// and is never seen in flight.
#[test]
fn a_spark_thrown_against_a_wall_does_not_pass_the_wall() {
    use ambition_demo_mary_o::powerups::SPARK_VISUAL;
    use ambition_platformer2d::projectiles::ProjectileVisualId;

    /// One press, then 60 ticks: where the spark was born, and each place a
    /// spark was seen in flight.
    fn throw_and_watch(app: &mut App, body: Entity) -> (Vec2, Vec2, Vec<Vec2>) {
        let mut born = Vec::new();
        let mut seen: Vec<Vec2> = Vec::new();
        for tick in 0..60 {
            step(
                app,
                ControlFrame {
                    modifier_held: true,
                    modifier_pressed: tick == 0,
                    ..ControlFrame::default()
                },
            );
            born.extend(requests(app, body));
            let world = app.world_mut();
            let mut sparks = world.query::<(&ProjectileVisualId, &BodyKinematics)>();
            seen.extend(sparks.iter(world).filter(|(visual, _)| visual.0 == SPARK_VISUAL).map(|(_, kin)| kin.pos));
        }
        assert_eq!(born.len(), 1, "control: she threw one spark: {born:?}");
        (born[0].0, born[0].1, seen)
    }

    let (mut app, body) = in_her_fire_form(ambition_demo_mary_o_app::build_demo_app());
    // Control: a spark thrown in the open is seen in flight, so an empty list
    // at the wall is a spark that is gone and not a watch that sees nothing.
    let (_, _, in_the_open) = throw_and_watch(&mut app, body);
    assert!(!in_the_open.is_empty(), "control: a spark thrown in the open was never seen in flight");

    // Walk into the left wall.
    let mut at_wall = false;
    for _ in 0..600 {
        step(
            &mut app,
            ControlFrame {
                axis_x: -1.0,
                aim_x: -1.0,
                left_pressed: true,
                ..ControlFrame::default()
            },
        );
        let kin = app.world().get::<BodyKinematics>(body).expect("a live body");
        if kin.pos.x - kin.size.x * 0.5 <= 0.05 {
            at_wall = true;
            break;
        }
    }
    assert!(at_wall, "control: she reached the left wall");
    for _ in 0..60 {
        step(&mut app, ControlFrame::default());
    }
    let kin = app.world().get::<BodyKinematics>(body).expect("a live body").clone();
    assert!(
        kin.facing < 0.0 && kin.pos.x - kin.size.x * 0.5 <= 0.05,
        "control: she faces the wall she touches"
    );

    let (born, half, seen) = throw_and_watch(&mut app, body);
    assert!(born.x < 0.0, "premise: the spark is born inside the wall ({born:?})");
    let far_side = -32.0;
    let beyond: Vec<&Vec2> = seen.iter().filter(|pos| pos.x + half.x < far_side).collect();
    assert!(
        beyond.is_empty(),
        "a spark thrown against the wall was seen past its far side: {beyond:?} (all places: {seen:?})"
    );
}
