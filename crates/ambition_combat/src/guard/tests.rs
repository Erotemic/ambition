use super::*;
use ambition_characters::actor::{GuardBreak, PartBox};

const DOWN: ae::Vec2 = ae::Vec2::new(0.0, 1.0);

/// A shield 20 wide and 80 tall, its top at y = 20, held by a body facing
/// `facing`.
fn shield(faces: GuardFaces, facing: f32, open_above: bool) -> GuardPiece {
    GuardPiece {
        guard: 0,
        material: Ok(ae::frame::Sim2::IDENTITY),
        domain: ae::Aabb {
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
    assert!(front.covers(ahead).unwrap(), "a strike from the front is turned");
    assert!(!front.covers(behind).unwrap(), "a strike from behind gets past");
    assert!(!front.covers(above).unwrap(), "a strike from above its top goes over it");
    assert!(shield(GuardFaces::Front, 1.0, false).covers(above).unwrap(), "a guard closed above turns it");
    // Facing the other way, its front is the other side.
    assert!(shield(GuardFaces::Front, -1.0, true).covers(behind).unwrap());
    assert!(!shield(GuardFaces::Front, -1.0, true).covers(ahead).unwrap());
    let around = shield(GuardFaces::Around, 1.0, true);
    assert!(around.covers(ahead).unwrap() && around.covers(behind).unwrap());
    let inverted = GuardPiece {
        material: Ok(ae::frame::Sim2::new(1.0, -bevy::math::Mat2::IDENTITY, ae::Vec2::new(40.0, 120.0)).unwrap()),
        ..around
    };
    assert!(!inverted.covers(ae::Vec2::new(20.0, 120.0)).unwrap());
    assert!(inverted.covers(ae::Vec2::new(20.0, 60.0)).unwrap());
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

fn guard_app(feel: ambition_characters::actor::Deflect) -> App {
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
    finalized.prepared.guards = vec![GuardSpec { deflect: feel, ..spec(None) }];
    let mut registry = PreparedCharacterRegistry::default();
    registry.insert_prepared(finalized.prepared);
    app.insert_resource(registry);
    app.add_systems(Update, resolve_deflects);
    app
}

/// A guarded defender at (100, 100) in ordinary gravity and an attacker at
/// `attacker_at` whose unit down is `attacker_down`; the attacker's strike is
/// deflected. Returns the attacker's (staged launch, carried run).
fn deflected_attacker(attacker_at: ae::Vec2, attacker_down: ae::Vec2) -> (ae::Vec2, f32) {
    let mut app = guard_app(FEEL);
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
        .spawn((body(ae::Vec2::new(100.0, 100.0)), CharacterForm::new("npc_guard"), GuardState::default()))
        .id();
    let mut frame = ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default();
    frame.publish_resolved_frame(ae::MotionFrame::from_direction(attacker_down, 0.0));
    let attacker = app.world_mut().spawn((body(attacker_at), frame)).id();
    app.world_mut().write_message(DeflectedBodyHit {
        defender,
        attacker,
        hitbox: attacker,
        guard: 0,
        contact: DeflectContact::at(&VolumePiece::whole(ae::CombatVolume::aabb(ae::Aabb::new(attacker_at, ae::Vec2::ONE))),
            &VolumePiece::whole(ae::CombatVolume::aabb(ae::Aabb::new(ae::Vec2::splat(100.0), ae::Vec2::ONE))),
            attacker_at, ae::Vec2::splat(100.0), ae::Vec2::new(100.0, 80.0), None, None).unwrap(),
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

/// Review b2c36ae4, finding 4. The guarded body is at (100, 100) and the
/// attacker at (160, 100), with its down world +x: away from the guarded
/// body. Its lift is against its own down, so it pointed back at the guarded
/// body. Measured before: the launch was (-120, -300), toward the guarded
/// body (its dot with the way out is -7200). Poison: keep the lift when it
/// points inward.
#[test]
fn a_deflected_attacker_whose_down_points_away_is_not_lifted_back_in() {
    let at = ae::Vec2::new(160.0, 100.0);
    let (launch, _) = deflected_attacker(at, ae::Vec2::new(1.0, 0.0));
    assert!(
        launch.dot(at - ae::Vec2::new(100.0, 100.0)) > 0.0,
        "the attacker is thrown toward the guarded body: {launch:?}"
    );
}

/// Every attacker is thrown away from the guarded body, from each side of
/// it and in each gravity.
#[test]
fn a_deflected_attacker_is_thrown_away_in_every_gravity() {
    let defender = ae::Vec2::new(100.0, 100.0);
    let downs = [(0.0, 1.0), (0.0, -1.0), (1.0, 0.0), (-1.0, 0.0), (0.6, 0.8)].map(|(x, y)| ae::Vec2::new(x, y));
    let offsets = [(60.0, 0.0), (-60.0, 0.0), (0.0, 60.0), (0.0, -60.0), (40.0, -30.0), (-40.0, 30.0)]
        .map(|(x, y)| ae::Vec2::new(x, y));
    for down in downs {
        for offset in offsets {
            let (launch, _) = deflected_attacker(defender + offset, down);
            assert!(
                launch.dot(offset) > 0.0,
                "an attacker at {offset:?} from the guarded body, with its down {down:?}, is thrown \
                 toward it: {launch:?}"
            );
        }
    }
}

#[test]
fn guard_rebound_uses_the_contact_chart_for_scale_parity_and_surface_motion() {
    use ae::frame::{MapConvention, PortalAperture, PortalFrame};
    use ae::frame::thread::{MaterialPrimitive, MotionAdmission, PieceBudget, PortalLink, partition};
    for ratio in [0.5, 2.0] {
        for convention in [MapConvention::Reflection, MapConvention::Rotation] {
            for moving in [false, true] {
                for guard_carried in [false, true] {
                    let mut entry = PortalFrame::fixed(ae::Vec2::new(0.0, 100.0), -ae::Vec2::Y);
                    let mut exit = PortalFrame::fixed(ae::Vec2::new(-500.0, 100.0), ae::Vec2::Y);
                    if moving {
                        entry.velocity = ae::Vec2::new(15.0, -25.0);
                        entry.spin = 0.1;
                        exit.velocity = ae::Vec2::new(-30.0, 50.0);
                        exit.spin = -0.2;
                    }
                    let link = PortalLink::new(PortalAperture { frame: entry, half_length: 80.0 },
                        PortalAperture { frame: exit, half_length: 80.0 * ratio }, 0);
                    let links = [link];
                    let admission = MotionAdmission { links: &links, first: None, budget: PieceBudget::COMPOSED, convention };
                    let source = ae::Vec2::new(0.0, 90.0);
                    let volume = ae::CombatVolume::aabb(ae::Aabb::new(ae::Vec2::new(0.0, 105.0), ae::Vec2::splat(4.0)));
                    let pieces = partition(MaterialPrimitive { domain: &volume, to_world: ae::frame::Sim2::IDENTITY },
                        source, &links, None, admission.budget, convention).unwrap();
                    let carried = pieces.iter().find(|piece| !piece.path.is_empty()).expect("the material crosses the aperture");
                    let point = carried.center();
                    let far_source = ae::Vec2::new(-500.0 + 20.0 * ratio, 100.0 + 20.0 * ratio);
                    let far = VolumePiece::whole(ae::CombatVolume::aabb(ae::Aabb::new(point, ae::Vec2::splat(2.0))));
                    let (strike, guard, attacker_at, defender_at) = if guard_carried {
                        (&far, carried, far_source, source)
                    } else {
                        (carried, &far, source, far_source)
                    };
                    let contact = DeflectContact::at(strike, guard, attacker_at, defender_at, point,
                        Some(admission), Some(admission)).unwrap();
                    let out = contact.outward.unwrap();
                    let speed = contact.guard_material.length(FEEL.rebound_speed).unwrap();
                    let lift = contact.guard_material.length(FEEL.rebound_lift).unwrap();
                    for (piece, chart, out, speed, lift) in [
                        (strike, contact.attacker, out, speed, lift),
                        (guard, contact.defender, -out, contact.guard_material.length(FEEL.push).unwrap(), 0.0),
                    ] {
                        let map = link.map(convention).unwrap();
                        let root_point = if piece.is_carried() { map.inverse().unwrap().point(point).unwrap() } else { point };
                        for boost in [ae::Vec2::ZERO, ae::Vec2::new(70.0, -30.0)] {
                            let resting = if piece.is_carried() {
                                map.inverse().unwrap().velocity(&exit, &entry, point, boost).unwrap()
                            } else { boost };
                            let body = ae::BodyKinematics { vel: resting, ..Default::default() };
                            let prepared = chart.prepare_throw(&body, DOWN, Some(out), 1.0, speed, lift, boost).unwrap();
                            let (velocity, right) = if piece.is_carried() {
                                (map.velocity(&entry, &exit, root_point, prepared.launch).unwrap(), map.direction(ae::Vec2::X))
                            } else { (prepared.launch, ae::Vec2::X) };
                            let relative_velocity = velocity - boost;
                            assert!(relative_velocity.dot(out) > 0.0, "the visible contact separates: {velocity:?}, {out:?}");
                            assert!((relative_velocity.dot(right).abs() - speed).abs() < 0.001,
                                "the guard's speed is measured in contact units");
                            assert!((prepared.run - prepared.launch.x).abs() < 0.001, "carried run uses the body's root frame");
                            if moving && piece.is_carried() {
                                assert!((prepared.launch - map.inverse().unwrap().vector(velocity).unwrap()).length() > 1.0,
                                    "portal surface motion must survive the response conversion");
                            }
                        }
                    }
                }
            }
        }
    }
}


#[test]
fn a_refused_guard_response_commits_neither_body_nor_guard_state() {
    let mut app = guard_app(ambition_characters::actor::Deflect { rebound_speed: f32::MAX, ..FEEL });
    let initial = ae::BodyKinematics { pos: ae::Vec2::splat(100.0), vel: ae::Vec2::new(7.0, 9.0), ..Default::default() };
    let attacker = app.world_mut().spawn((initial, BodyCombat::default(), ae::BodyFlightState::default())).id();
    let defender = app.world_mut().spawn((initial, BodyCombat::default(), ae::BodyFlightState::default(),
        CharacterForm::new("npc_guard"), GuardState::default())).id();
    let volume = ae::CombatVolume::aabb(ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::ONE));
    let piece = VolumePiece::from_material(ae::frame::thread::MaterialPrimitive { domain: &volume,
        to_world: ae::frame::Sim2::new(2.0, bevy::math::Mat2::IDENTITY, initial.pos).unwrap() }).unwrap();
    let contact = DeflectContact::at(&piece, &piece, initial.pos + ae::Vec2::X, initial.pos, initial.pos, None, None).unwrap();
    app.world_mut().write_message(DeflectedBodyHit { attacker, defender, hitbox: attacker, guard: 0, contact, damage: 1 });
    app.update();
    for body in [attacker, defender] {
        assert_eq!(*app.world().get::<ae::BodyKinematics>(body).unwrap(), initial);
        assert_eq!(*app.world().get::<ae::BodyFlightState>(body).unwrap(), ae::BodyFlightState::default());
        let combat = app.world().get::<BodyCombat>(body).unwrap();
        assert_eq!(combat.recoil_lock_timer, 0.0);
        assert_eq!(combat.hitstop_timer, 0.0);
    }
    assert_eq!(*app.world().get::<GuardState>(defender).unwrap(), GuardState::default());
}
