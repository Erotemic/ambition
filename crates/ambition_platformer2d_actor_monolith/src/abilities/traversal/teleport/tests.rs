//! ⛔⛔ THE ARMS STRADDLE THE RADIUS AND THE DIRECTION. A ledge assist tested
//! only against a ledge it should catch agrees with an implementation that snaps
//! to the nearest surface in the world, which would drag a fighter off the stage
//! and onto the wrong platform. Every "it helps" arm below is paired with a
//! "and it refuses" one.

use super::*;

/// The world's own gravity, for the arms written before the assist knew there
/// was more than one. ⚠ Every arm in this file used to be THIS and only this —
/// which is how the assist kept searching `+y` faces while the teleport aimed in
/// the resolved frame.
const DOWN: ae::Vec2 = ae::Vec2::new(0.0, 1.0);

/// A solid named by its centre and half-extents. `Block::solid` takes min +
/// size, so this is the conversion in one place rather than at seven call sites.
fn solid(name: &str, center: ae::Vec2, half: ae::Vec2) -> ae::Block {
    ae::Block::solid(name, center - half, half * 2.0)
}

fn world_with(blocks: Vec<ae::Block>) -> ae::World {
    ae::World::new(
        "teleport_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::ZERO,
        blocks,
    )
}

/// The fighter's half-extents throughout: a 32x64 body.
const HALF: ae::Vec2 = ae::Vec2::new(16.0, 32.0);

/// A platform whose top face is at y = 0, spanning x in [-100, 100].
fn stage() -> ae::World {
    world_with(vec![solid(
        "stage",
        ae::Vec2::new(0.0, 50.0),
        ae::Vec2::new(100.0, 50.0),
    )])
}

#[test]
fn an_arrival_just_under_a_ledge_is_placed_standing_on_it() {
    let world = stage();
    // The arrival's CENTRE 20px below the platform's top face — the miss a few
    // degrees of stick angle makes.
    let assisted = ledge_assisted_arrival(&world, ae::Vec2::new(0.0, 20.0), HALF, 40.0, DOWN);
    assert!(
        assisted.y < 20.0,
        "the arrival must be lifted onto the ledge, and it stayed at {}",
        assisted.y
    );
    assert!(
        (assisted.y - (0.0 - HALF.y)).abs() < 1e-3,
        "…standing exactly on the top face, and it is at {}",
        assisted.y
    );
}

/// ⛔ THE PAIRED ARM: the same miss, outside the authored radius, is left alone.
#[test]
fn an_arrival_far_below_a_ledge_is_left_where_it_landed() {
    let world = stage();
    let assisted = ledge_assisted_arrival(&world, ae::Vec2::new(0.0, 200.0), HALF, 40.0, DOWN);
    assert_eq!(
        assisted,
        ae::Vec2::new(0.0, 200.0),
        "a ledge 200px away is not a ledge this teleport was aimed at"
    );
}

/// ⛔ A DISABLED ASSIST IS DISABLED. `0.0` is what a teleport that is not a
/// recovery authors, and it must not quietly acquire the behaviour anyway.
#[test]
fn a_zero_radius_never_moves_an_arrival() {
    let world = stage();
    let at = ae::Vec2::new(0.0, 20.0);
    assert_eq!(ledge_assisted_arrival(&world, at, HALF, 0.0, DOWN), at);
}

/// ⛔⛔ IT ONLY EVER LIFTS. `+y` is gravity-down, so a ledge BELOW the arrival
/// has a LARGER y and is one the fighter CLEARED; dragging them down onto it
/// would end a recovery that had already worked. A naive "nearest surface" rule
/// does exactly that, and so did the first version of this function.
#[test]
fn a_ledge_the_fighter_already_cleared_never_drags_them_back_down() {
    // Two platforms: the one at y = 0 the fighter cleared, and nothing above.
    let world = stage();
    // Well ABOVE the platform's top face (smaller y) and unsupported — a
    // successful recovery, mid-air, with the surface within the radius.
    let cleared = ae::Vec2::new(0.0, -60.0);
    assert_eq!(
        ledge_assisted_arrival(&world, cleared, HALF, 200.0, DOWN),
        cleared,
        "a surface the fighter is already above is not a ledge to be pulled onto"
    );
}

/// ⛔ AN ARRIVAL THAT ALREADY HAD SUPPORT IS NOT MOVED, or the assist would
/// choose a different platform than the player did.
#[test]
fn an_arrival_already_standing_on_something_is_left_alone() {
    let world = stage();
    let standing = ae::Vec2::new(0.0, 0.0 - HALF.y);
    assert_eq!(
        ledge_assisted_arrival(&world, standing, HALF, 80.0, DOWN),
        standing
    );
}

/// ⛔ AND IT NEVER PLACES A BODY INSIDE GEOMETRY. A ledge with a wall sitting on
/// it has no room to stand, and the miss is better than the embed.
#[test]
fn a_ledge_with_no_room_to_stand_is_refused() {
    let world = world_with(vec![
        solid(
            "stage",
            ae::Vec2::new(0.0, 50.0),
            ae::Vec2::new(100.0, 50.0),
        ),
        // A wall occupying exactly where a body standing on that ledge would be.
        solid(
            "wall",
            ae::Vec2::new(0.0, -HALF.y),
            ae::Vec2::new(20.0, HALF.y),
        ),
    ]);
    let at = ae::Vec2::new(0.0, 20.0);
    assert_eq!(
        ledge_assisted_arrival(&world, at, HALF, 60.0, DOWN),
        at,
        "a placement that would embed the body is worse than the miss"
    );
}

/// ⛔ THE NEAREST QUALIFYING LEDGE WINS, so a teleport near two platforms lands
/// on the one it was aimed at rather than on whichever the scan reached first.
#[test]
fn the_nearest_qualifying_ledge_wins() {
    let world = world_with(vec![
        // ⛔ `Aabb::top()` IS THE MIN Y, because `+y` is gravity-down. A centre
        // of `c` with half-height `h` has its top face at `c - h`, and the first
        // version of this test placed both platforms 100px lower than it meant
        // to — then asserted the number it had meant, and failed against correct
        // code.
        //
        // Far above: top face at y = -200.
        solid(
            "far",
            ae::Vec2::new(0.0, -150.0),
            ae::Vec2::new(100.0, 50.0),
        ),
        // Near above: top face at y = -20.
        solid("near", ae::Vec2::new(0.0, 30.0), ae::Vec2::new(100.0, 50.0)),
    ]);
    let assisted = ledge_assisted_arrival(&world, ae::Vec2::new(0.0, 20.0), HALF, 400.0, DOWN);
    assert!(
        (assisted.y - (-20.0 - HALF.y)).abs() < 1e-3,
        "the NEAR ledge decides, and the arrival went to {}",
        assisted.y
    );
}

/// A body with every cluster the movement kernel reads, at `pos`, plus the two
/// non-cluster facts `apply_authored_teleports` queries beside them.
///
/// ⛔ SPELLED OUT rather than borrowed from a spawn helper: the actor spawners
/// live behind `pub(super)` in `features::ecs`, and a fixture that reached for
/// one would drag a character catalog into a test about a ledger entry.
fn spawn_teleporting_body(app: &mut bevy::prelude::App, pos: ae::Vec2) -> bevy::prelude::Entity {
    app.world_mut()
        .spawn((
            ae::BodyAbilities::default(),
            ae::BodyKinematics {
                pos,
                size: ae::Vec2::new(24.0, 48.0),
                ..Default::default()
            },
            ae::BodyBaseSize::default(),
            ae::BodyGroundState::default(),
            ae::BodyWallState::default(),
            ae::BodyJumpState::default(),
            ae::BodyDashState::default(),
            ae::BodyFlightState::default(),
            ae::BodyBlinkState::default(),
            ae::BodyLedgeState::default(),
        ))
        .insert((
            ae::BodyDodgeState::default(),
            ae::BodyShieldState::default(),
            ae::BodyModeState::default(),
            ae::BodyEnvironmentContact::default(),
            ae::BodyMana::default(),
            ae::BodyOffense::default(),
            ae::BodyActionBuffer::default(),
            ae::BodyLifetime::default(),
            ae::BodyComboTrace::default(),
        ))
        .insert((
            ae::movement::MotionModel::default(),
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
            ambition_characters::control::ActorControl::default(),
        ))
        .id()
}

/// The teleport records itself in the Class-B ledger, exactly once.
///
/// ⛔⛔ A BODY THAT MOVES DISCONTINUOUSLY WITHOUT AN ENTRY IS A BUG TO THE
/// INSTRUMENTS. The collision oracle uses that ledger for two things — exempting
/// a legal warp from its clipping probe, and catching two Class-B authorities
/// remapping one body in a single frame. `blink` has always recorded at its
/// `transit_body`; this road was added without it, so an
/// authored Smash teleport read as unexplained clipping and could collide with
/// another remap unseen.
///
/// ⭐ EXACTLY ONE, not "at least one": the count is the half that would catch a
/// record placed in a loop or duplicated across the two arrival paths.
#[test]
fn an_authored_teleport_records_one_scripted_remap() {
    use ambition_platformer2d_shared_tangle::class_b::{ClassBRemap, ClassBRemapLog};

    let mut app = bevy::prelude::App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(stage()),
    );
    app.init_resource::<ClassBRemapLog>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ActorActionMessage>();
    app.add_systems(bevy::prelude::Update, apply_authored_teleports);

    let body = spawn_teleporting_body(&mut app, ae::Vec2::new(200.0, 200.0));

    app.world_mut().write_message(ActorActionMessage {
        actor: body,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special(TELEPORT.to_string()),
            params: ambition_entity_catalog::ParamValue::from_typed(&TeleportParams {
                // AIMED, not an ambush: this arm is about the ledger, and
                // an empty stage has nobody to get behind.
                behind_nearest_foe: false,
                behind_gap: 0.0,
                distance: 250.0,
                // No assist: this arm is about the LEDGER, and a ledge catch
                // would move the arrival for a second reason.
                ledge_assist: 0.0,
                depart_vfx: "blink".to_string(),
                arrive_vfx: "blink".to_string(),
            })
            .expect("teleport params serialize"),
        },
    });
    app.update();

    let log = app.world().resource::<ClassBRemapLog>();
    let kinds: Vec<ClassBRemap> = log.kinds_for(body).collect();
    assert_eq!(
        kinds,
        vec![ClassBRemap::ScriptedTeleport],
        "the teleport moved the body and told the ledger {kinds:?} — a warp with \
         no entry reads to the collision oracle as unexplained clipping, and a \
         second Class-B author on the same frame becomes invisible",
    );
}

// ---------------------------------------------------------------------------
// The ambush chooser.
//
// ⛔⛔ EVERY RULE HERE STRADDLES ITS BOUNDARY. `reach` is a magnitude, so it
// needs an arm inside it and an arm outside; "nearest" needs two foes and an
// assertion about which; "foe" needs a body that is one and a body that is not.
// A chooser tested only on the case it was written for agrees with a chooser
// that snaps to whoever happens to be first in the query.
// ---------------------------------------------------------------------------

/// A body at `pos` with a `32x64` frame, on `team`, tracked as player `slot`.
fn body(id: u32, slot: u8, team: &str, pos: ae::Vec2) -> FoeCandidate {
    sized_body(id, slot, team, pos, HALF)
}

fn sized_body(id: u32, slot: u8, team: &str, pos: ae::Vec2, half: ae::Vec2) -> FoeCandidate {
    FoeCandidate {
        entity: Entity::from_raw_u32(id).expect("a valid test entity id"),
        pos,
        half,
        faction: Default::default(),
        team: Some(ambition_combat::targeting::MatchTeam::new(team)),
        driving: None,
        sim: Some(ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(slot)),
    }
}

/// Reach and gap wide enough that neither is what the arm under test is about.
const REACH: f32 = 400.0;
const GAP: f32 = 18.0;

#[test]
fn an_ambush_arrives_on_the_far_side_of_the_foe_facing_back_at_him() {
    let me = body(1, 0, "red", ae::Vec2::new(0.0, 0.0));
    let foe = body(2, 1, "blue", ae::Vec2::new(120.0, 0.0));
    let stage = vec![me, foe];
    let ambush = ambush_arrival(&stage[0], &stage, REACH, GAP, 1.0).expect("a foe in reach");
    // Past him, by his half-width plus hers plus the authored gap.
    assert_eq!(ambush.arrival.x, 120.0 + HALF.x + HALF.x + GAP);
    // ⛔ AND SHE LOOKS BACK. She travelled +x, so she must end up facing -x.
    assert_eq!(ambush.facing, -1.0);
}

/// ⛔ THE MIRRORED ARM. A foe on her LEFT puts the arrival further left, and a
/// sign error that reads as "behind" in one direction reads as "in front of" in
/// the other.
#[test]
fn a_foe_on_the_other_side_puts_the_arrival_on_the_other_side() {
    let me = body(1, 0, "red", ae::Vec2::new(0.0, 0.0));
    let foe = body(2, 1, "blue", ae::Vec2::new(-120.0, 0.0));
    let stage = vec![me, foe];
    let ambush = ambush_arrival(&stage[0], &stage, REACH, GAP, 1.0).expect("a foe in reach");
    assert_eq!(ambush.arrival.x, -120.0 - HALF.x - HALF.x - GAP);
    assert_eq!(ambush.facing, 1.0);
}

/// ⛔⛔ THE GAP IS BETWEEN EDGES, so a foe twice her width pushes the arrival
/// twice as far out — the same authored number reading the same behind bodies of
/// any size is the entire reason it is not a centre offset.
#[test]
fn a_wider_foe_pushes_the_arrival_further_out_by_exactly_its_edge() {
    let me = body(1, 0, "red", ae::Vec2::new(0.0, 0.0));
    let wide = sized_body(
        2,
        1,
        "blue",
        ae::Vec2::new(120.0, 0.0),
        ae::Vec2::new(48.0, 32.0),
    );
    let stage = vec![me, wide];
    let ambush = ambush_arrival(&stage[0], &stage, REACH, GAP, 1.0).expect("a foe in reach");
    assert_eq!(ambush.arrival.x, 120.0 + 48.0 + HALF.x + GAP);
}

/// ⛔⛔ FEET TO FEET, NOT CENTRE TO CENTRE. `+y` is gravity-down. A foe twice her
/// height standing on the same floor has his centre 32px higher than hers; a
/// centre-matched arrival would leave her floating at his waist.
#[test]
fn the_arrival_stands_where_the_foe_stands() {
    // Both on a floor at y = 0: her centre is 32 above it, his is 64 above it.
    let me = body(1, 0, "red", ae::Vec2::new(0.0, -32.0));
    let tall = sized_body(
        2,
        1,
        "blue",
        ae::Vec2::new(120.0, -64.0),
        ae::Vec2::new(16.0, 64.0),
    );
    let stage = vec![me, tall];
    let ambush = ambush_arrival(&stage[0], &stage, REACH, GAP, 1.0).expect("a foe in reach");
    assert_eq!(
        ambush.arrival.y, -32.0,
        "her feet belong on the floor he is standing on, not at his centre"
    );
}

#[test]
fn the_nearest_foe_is_the_one_ambushed() {
    let me = body(1, 0, "red", ae::Vec2::new(0.0, 0.0));
    let far = body(2, 1, "blue", ae::Vec2::new(300.0, 0.0));
    let near = body(3, 2, "blue", ae::Vec2::new(-90.0, 0.0));
    let stage = vec![me, far, near];
    let ambush = ambush_arrival(&stage[0], &stage, REACH, GAP, 1.0).expect("a foe in reach");
    assert!(
        ambush.arrival.x < -90.0,
        "the foe 90px away is nearer than the one 300px away, and the arrival \
         landed at {}",
        ambush.arrival.x
    );
}

/// ⛔⛔ `reach` IS A RANGE AND THE MOVE REFUSES OUTSIDE IT. The version this
/// replaced clamped the TRAVEL instead, which put her 320px along the line to a
/// foe 900px away — in front of him, mid-stage, with the move spent.
#[test]
fn a_foe_beyond_the_reach_is_not_a_target_at_all() {
    let me = body(1, 0, "red", ae::Vec2::new(0.0, 0.0));
    let distant = body(2, 1, "blue", ae::Vec2::new(900.0, 0.0));
    let stage = vec![me, distant];
    assert!(
        ambush_arrival(&stage[0], &stage, REACH, GAP, 1.0).is_none(),
        "a foe 900px away is out of a 400px reach, so the move must refuse \
         rather than deposit her partway"
    );
}

/// ⛔ THE PAIRED ARM: the same foe, one pixel INSIDE the reach, is taken.
#[test]
fn a_foe_just_inside_the_reach_is_taken() {
    let me = body(1, 0, "red", ae::Vec2::new(0.0, 0.0));
    let edge = body(2, 1, "blue", ae::Vec2::new(REACH - 1.0, 0.0));
    let stage = vec![me, edge];
    assert!(ambush_arrival(&stage[0], &stage, REACH, GAP, 1.0).is_some());
}

#[test]
fn an_empty_stage_has_nobody_to_get_behind() {
    let me = body(1, 0, "red", ae::Vec2::ZERO);
    let stage = vec![me];
    assert!(ambush_arrival(&stage[0], &stage, REACH, GAP, 1.0).is_none());
}

/// ⛔⛔ A TEAMMATE IS NOT A FOE, and this is the arm that separates "the nearest
/// body" from "the nearest foe". A chooser that took whoever was closest would
/// pass every other test in this file.
#[test]
fn a_teammate_standing_closer_is_never_the_target() {
    let me = body(1, 0, "red", ae::Vec2::new(0.0, 0.0));
    let ally = body(2, 1, "red", ae::Vec2::new(40.0, 0.0));
    let foe = body(3, 2, "blue", ae::Vec2::new(200.0, 0.0));
    let stage = vec![me, ally, foe];
    let ambush = ambush_arrival(&stage[0], &stage, REACH, GAP, 1.0).expect("a foe in reach");
    assert!(
        ambush.arrival.x > 200.0,
        "the ally 40px away must be skipped for the foe 200px away, and the \
         arrival landed at {}",
        ambush.arrival.x
    );
}

/// ⛔ AND SHE NEVER AMBUSHES HERSELF, which is the degenerate case of the above
/// — she is in her own candidate list by construction.
#[test]
fn the_teleporter_is_not_its_own_target() {
    let me = body(1, 0, "red", ae::Vec2::ZERO);
    let stage = vec![me];
    assert!(ambush_arrival(&stage[0], &stage, REACH, GAP, 1.0).is_none());
}

/// ⛔⛔ AN EXACT TIE RESOLVES BY `SimId`, NOT BY LIST ORDER. Two foes equidistant
/// on opposite sides is the state a rollback rewind can re-present with the
/// query in the other order; if the answer moved with it the match desyncs.
#[test]
fn an_exact_distance_tie_is_broken_by_sim_id_and_not_by_query_order() {
    let me = || body(1, 0, "red", ae::Vec2::ZERO);
    let left = || body(2, 9, "blue", ae::Vec2::new(-150.0, 0.0));
    let right = || body(3, 4, "blue", ae::Vec2::new(150.0, 0.0));

    let one = vec![me(), left(), right()];
    let other = vec![me(), right(), left()];
    let a = ambush_arrival(&one[0], &one, REACH, GAP, 1.0).expect("a foe in reach");
    let b = ambush_arrival(&other[0], &other, REACH, GAP, 1.0).expect("a foe in reach");
    assert_eq!(
        a.arrival, b.arrival,
        "the two foes are exactly equidistant, so reversing the candidate list \
         must not change who gets ambushed"
    );
    assert!(
        a.arrival.x > 150.0,
        "player slot 4 sorts before slot 9, so the RIGHT foe wins the tie and \
         the arrival landed at {}",
        a.arrival.x
    );
}

/// ⛔ DIRECTLY ABOVE HIM THERE IS NO "BEHIND", so the tiebreak is where she is
/// LOOKING — and it must actually depend on that, or the move silently always
/// picks +x for a foe on the same column.
#[test]
fn a_foe_on_the_same_column_is_passed_the_way_she_faces() {
    let me = body(1, 0, "red", ae::Vec2::new(0.0, -200.0));
    let below = body(2, 1, "blue", ae::Vec2::new(0.0, 0.0));
    let stage = vec![me, below];
    let facing_right = ambush_arrival(&stage[0], &stage, REACH, GAP, 1.0).expect("in reach");
    let facing_left = ambush_arrival(&stage[0], &stage, REACH, GAP, -1.0).expect("in reach");
    assert!(facing_right.arrival.x > 0.0);
    assert!(facing_left.arrival.x < 0.0);
}

/// The assist catches the ledge in WHATEVER frame the fighter is falling in.
///
/// ⛔⛔ IT SEARCHED THE WORLD'S `+y` FACES. The teleport itself has always aimed
/// in the resolved frame — `apply_authored_teleports` derives `gravity_dir` from
/// `ResolvedMotionFrame` and rotates both the aim and the no-stick "up" fallback
/// with it — while `ledge_assisted_arrival` defined a ledge as `Block::top()`,
/// compared world `y`, and landed at `top() - half.y`. Under flipped or sideways
/// gravity that is a SPLIT ABILITY: the teleport goes where the player pointed
/// and the recovery assist looks at the wrong face of the world
///.
///
/// ⭐ ONE FIXTURE, FOUR FRAMES, built by rotating the geometry rather than by
/// hand-writing four cases — a hand-written `-Y` arm is a second chance to make
/// the same sign error. Each frame asserts BOTH terms: the ledge on the
/// anti-gravity side is caught, and the one the fighter already cleared is
/// refused.
#[test]
fn the_ledge_assist_follows_gravity_into_every_frame() {
    // A platform 100 wide and 50 deep whose SUPPORT face is the origin plane,
    // expressed in a frame where `down` is the gravity direction.
    let frames = [
        ("down (+Y)", ae::Vec2::new(0.0, 1.0)),
        ("up (-Y)", ae::Vec2::new(0.0, -1.0)),
        ("right (+X)", ae::Vec2::new(1.0, 0.0)),
        ("left (-X)", ae::Vec2::new(-1.0, 0.0)),
    ];
    for (name, down) in frames {
        let across = ae::Vec2::new(-down.y, down.x);
        // Body half-extents in this frame: 16 across, 32 along gravity.
        let half = (across * 16.0 + down * 32.0).abs();
        // The slab sits BEHIND the origin plane along gravity, so its head face
        // (the one a falling body lands on) is at the origin.
        let block_centre = down * 50.0;
        let block_half = (across * 100.0 + down * 50.0).abs();
        let world = world_with(vec![solid("stage", block_centre, block_half)]);

        // SHORT of the lip: 20px further along gravity than the support face.
        let short = down * 20.0;
        let caught = ledge_assisted_arrival(&world, short, half, 40.0, down);
        assert!(
            caught.dot(down) < short.dot(down),
            "[{name}] a fighter hanging under the lip was not lifted onto it: \
             {short:?} -> {caught:?}"
        );
        assert!(
            (caught.dot(down) - (-32.0)).abs() < 0.01,
            "[{name}] the catch must stand the body ON the face (feet at 0, so \
             centre at -32 along gravity), and it landed at {}",
            caught.dot(down),
        );

        // CLEARED it: 200px toward anti-gravity, well past the face.
        let cleared = down * -200.0;
        assert_eq!(
            ledge_assisted_arrival(&world, cleared, half, 400.0, down),
            cleared,
            "[{name}] a surface the fighter had already cleared pulled them back \
             down onto it — the assist taking the stage away",
        );
    }
}
