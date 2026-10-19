use super::*;
use ambition_characters::actor::{GuardBreak, PartBox};

const DOWN: ae::Vec2 = ae::Vec2::new(0.0, 1.0);

/// A shield 20 wide and 80 tall, its top at y = 20, held by a body facing
/// `facing`.
fn shield(faces: GuardFaces, facing: f32, open_above: bool) -> GuardPiece {
    GuardPiece {
        guard: 0,
        volume: ae::Aabb {
            min: ae::Vec2::new(10.0, 20.0),
            max: ae::Vec2::new(30.0, 100.0),
        },
        faces,
        facing,
        open_above,
    }
}

/// A front guard covers the side its body faces and not the other; a strike
/// from above its top goes over it unless it is closed above. A guard all
/// round covers both sides.
#[test]
fn a_guard_covers_its_front_and_not_its_back_or_over_its_top() {
    let front = shield(GuardFaces::Front, 1.0, true);
    let ahead = ae::Vec2::new(60.0, 60.0);
    let behind = ae::Vec2::new(-20.0, 60.0);
    let above = ae::Vec2::new(60.0, 0.0);
    assert!(front.covers(ahead, DOWN), "a strike from the front is turned");
    assert!(!front.covers(behind, DOWN), "a strike from behind gets past");
    assert!(!front.covers(above, DOWN), "a strike from above its top goes over it");
    assert!(shield(GuardFaces::Front, 1.0, false).covers(above, DOWN), "a guard closed above turns it");
    // Facing the other way, its front is the other side.
    assert!(shield(GuardFaces::Front, -1.0, true).covers(behind, DOWN));
    assert!(!shield(GuardFaces::Front, -1.0, true).covers(ahead, DOWN));
    let around = shield(GuardFaces::Around, 1.0, true);
    assert!(around.covers(ahead, DOWN) && around.covers(behind, DOWN));
    // Under gravity that points up, "above" is the other way.
    let up = ae::Vec2::new(0.0, -1.0);
    assert!(!shield(GuardFaces::Around, 1.0, true).covers(ae::Vec2::new(20.0, 120.0), up));
    assert!(shield(GuardFaces::Around, 1.0, true).covers(ae::Vec2::new(20.0, 60.0), up));
}

fn spec(breaks: Option<GuardBreak>) -> GuardSpec {
    GuardSpec {
        part: PartBox::Shield,
        faces: GuardFaces::Front,
        open_above: true,
        lowered_in: Vec::new(),
        inset_px: 0.0,
        deflect: Default::default(),
        breaks,
    }
}

/// Deflects in quick succession break a guard on the authored count; a slow
/// one starts the run again; a heavy strike breaks it at once; a guard that
/// never breaks only counts.
#[test]
fn quick_deflects_or_a_heavy_one_break_a_guard() {
    let breaks = GuardBreak {
        blocks: 3,
        within_s: 1.0,
        heavy: Some(4),
        stagger_s: 1.5,
    };
    let guard = spec(Some(breaks));
    let mut state = GuardState::default();
    assert!(!state.count_deflect(&guard, 1));
    state.since_deflect_s = 0.5;
    assert!(!state.count_deflect(&guard, 1));
    state.since_deflect_s = 0.5;
    assert!(state.count_deflect(&guard, 1), "the third quick deflect breaks it: {state:?}");
    assert!(state.broken() && state.broken_s == 1.5 && state.deflects == 3);

    let mut slow = GuardState::default();
    for _ in 0..5 {
        assert!(!slow.count_deflect(&guard, 1), "slow deflects never break it: {slow:?}");
        slow.since_deflect_s = 2.0;
    }

    let mut heavy = GuardState::default();
    assert!(heavy.count_deflect(&guard, 4), "a heavy strike breaks it at once");

    let mut never = GuardState::default();
    for _ in 0..10 {
        assert!(!never.count_deflect(&spec(None), 99));
    }
    assert_eq!(never.deflects, 10);
}

/// The deflect feel of the test guard: a throw with lift, so the frame it is
/// measured in shows.
const FEEL: ambition_characters::actor::Deflect = ambition_characters::actor::Deflect {
    rebound_speed: 300.0,
    rebound_lift: 120.0,
    lockout_s: 0.3,
    hitstop_s: 0.05,
    push: 40.0,
};

/// A guarded defender at (100, 100) in ordinary gravity and an attacker at
/// `attacker_at` whose unit down is `attacker_down`; the attacker's strike is
/// deflected. Returns the attacker's (staged launch, carried run).
fn deflected_attacker(attacker_at: ae::Vec2, attacker_down: ae::Vec2) -> (ae::Vec2, f32) {
    let mut app = App::new();
    app.add_message::<DeflectedBodyHit>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_vfx::vfx::VfxInRoom>();
    let mut definition = ambition_characters::actor::definition::CharacterDefinition::new("npc_guard", "Guard", "test");
    definition.vitals.max_health = Some(10);
    let mut finalized = ambition_characters::prepared::prepare_and_finalize_for_test(
        definition,
        &ambition_characters::prepared::CharacterBindings::default(),
    );
    finalized.prepared.guards = vec![GuardSpec { deflect: FEEL, ..spec(None) }];
    let mut registry = PreparedCharacterRegistry::default();
    registry.insert_prepared(finalized.prepared);
    app.insert_resource(registry);
    app.add_systems(Update, resolve_deflects);
    let body = |pos: ae::Vec2| {
        (
            ae::BodyKinematics { pos, ..Default::default() },
            BodyCombat::default(),
            ae::BodyFlightState::default(),
            ae::BodyGroundState { on_ground: true, ..Default::default() },
        )
    };
    let defender = app
        .world_mut()
        .spawn((body(ae::Vec2::new(100.0, 100.0)), WornCharacter::new("npc_guard"), GuardState::default()))
        .id();
    let mut frame = ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default();
    frame.publish_resolved_frame(ae::MotionFrame::from_direction(attacker_down, 0.0));
    let attacker = app.world_mut().spawn((body(attacker_at), frame)).id();
    app.world_mut().write_message(DeflectedBodyHit {
        defender,
        attacker,
        hitbox: attacker,
        guard: 0,
        contact: ae::Vec2::new(100.0, 80.0),
        damage: 1,
    });
    app.update();
    let flight = app.world().get::<ae::BodyFlightState>(attacker).expect("the attacker's flight state");
    (flight.pending_launch, flight.carried_run)
}

/// Review finding #1 (2026-10-19). An attacker in another gravity is thrown
/// in its own frame. It stands on a wall (its down is world +x), above the
/// guarded body. It is thrown away from that body, off its wall, and the run
/// it carries is the launch's run along its own right. Measured before: the
/// throw was built on the guarded body's axes, so the attacker was pushed
/// into its own wall and carried a run along an axis it was not thrown on.
/// Poison: measure the attacker's throw in the defender's frame.
#[test]
fn a_deflected_attacker_in_another_gravity_is_thrown_in_its_own_frame() {
    let down = ae::Vec2::new(1.0, 0.0);
    let right = ae::Vec2::new(down.y, -down.x);
    let at = ae::Vec2::new(100.0, 40.0);
    let (launch, run) = deflected_attacker(at, down);
    assert!(launch.dot(down) < 0.0, "the attacker is thrown off its wall, not into it: {launch:?}");
    assert!(launch.dot(at - ae::Vec2::new(100.0, 100.0)) > 0.0, "away from the guarded body: {launch:?}");
    assert!(
        (launch.dot(right) - run).abs() < 1e-3 && run.abs() > 0.0,
        "the carried run {run} is the launch's run along the attacker's right ({})",
        launch.dot(right)
    );
}

/// Control: in the guarded body's own gravity, the attacker on its right is
/// thrown to the right and up.
#[test]
fn a_deflected_attacker_in_the_same_gravity_is_thrown_away_and_up() {
    let (launch, run) = deflected_attacker(ae::Vec2::new(160.0, 100.0), DOWN);
    assert_eq!((launch, run), (ae::Vec2::new(FEEL.rebound_speed, -FEEL.rebound_lift), FEEL.rebound_speed));
}
