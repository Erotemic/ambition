//! The mark through its module on the module road, with the real adapters:
//! the strike, the status, the clock and the blast.

use bevy::prelude::*;

use crate::module_road::{app_on, runner_of, Road};
use ambition_demo_smash_modules::mark::{Mark, MARKED};
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::combat::death_rules::OutOfPlay;
use ambition_platformer2d::combat::extension::blast::{CreditStandIn, CREDIT_STAND_IN_MARGIN_S};
use ambition_platformer2d::combat::extension::status::BodyStatuses;
use ambition_platformer2d::combat::on_hit::OnHitEffectMessage;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::smash_mark::MarkBodyParams;
use ambition_platformer2d::extension::sdk::phases::HIT_REACTION;
use ambition_platformer2d::extension::{BodyRecords, ExtensionSet};
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::sim_view::BodyClocksView;

const DT: f32 = 1.0 / 60.0;

fn app_with(road: Road, dt: f32) -> App {
    let mut app = app_on(road, ambition_demo_smash_modules::mark::module);
    {
        let mut time = app.world_mut().resource_mut::<ambition_platformer2d::time::WorldTime>();
        time.set_sim_dt(dt);
        time.set_wall_dt(dt);
    }
    app.init_resource::<CapturedBlasts>();
    app.init_resource::<BodyClocksView>();
    // After every phase: the blast is asked for in `technique_execution` and
    // the view is the sim tail's.
    app.add_systems(
        Update,
        (
            capture_blasts,
            ambition_platformer2d::sim_view::rebuild_body_clocks_view,
            ambition_platformer2d::sim_view::publish_status_clocks,
        )
            .chain()
            .after(ExtensionSet::Lower(HIT_REACTION)),
    );
    app
}

fn app() -> App {
    app_with(Road::Linked, DT)
}

/// A fighter at `at` with a simulation identity, which the mark credits.
fn fighter(app: &mut App, seat: u8, at: ae::Vec2) -> Entity {
    let mut kin = ae::BodyKinematics::default();
    kin.pos = at;
    kin.size = ae::Vec2::new(24.0, 40.0);
    app.world_mut().spawn((kin, MatchSeat(seat as usize), SimId::player_slot(seat))).id()
}

fn params(fuse_s: f32) -> MarkBodyParams {
    MarkBodyParams {
        fuse_s,
        damage: 6,
        blast_radius: 44.0,
        knockback: 1.2,
        // Not the shipped 0.08: see
        // `the_blast_is_credited_to_the_fighter_who_landed_the_mark`.
        blast_s: 0.3,
    }
}

fn hit_with(app: &mut App, owner: Entity, victim: Entity, key: &str, fuse_s: f32) {
    let effect = ambition_platformer2d::entity_catalog::EffectRef {
        key: key.to_string(),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params(fuse_s))
            .expect("params serialize"),
    };
    app.world_mut().write_message(OnHitEffectMessage {
        owner,
        victim,
        volume: ae::CombatVolume::Circle { center: ae::Vec2::ZERO, radius: 1.0 },
        contact: ae::Vec2::ZERO,
        effect,
    });
}

fn land_a_marking_hit(app: &mut App, owner: Entity, victim: Entity, fuse_s: f32) {
    hit_with(app, owner, victim, ambition_platformer2d::entity_catalog::smash_mark::MARK_BODY, fuse_s);
}

/// Accumulated as they are written, not read at the end: messages are
/// double-buffered, so after a few updates a blast is no longer visible.
///
/// A count and the last blast, not a `Vec`: `per_attempt_resource_census`
/// flags any `Resource` holding a collection, test modules included.
#[derive(Resource, Default)]
struct CapturedBlasts {
    count: usize,
    last: ae::Vec2,
    last_owner: Option<Entity>,
    last_lifetime_s: f32,
}

fn capture_blasts(
    mut reader: MessageReader<ambition_platformer2d::vfx::EffectRequest>,
    mut cap: ResMut<CapturedBlasts>,
) {
    for request in reader.read() {
        if let ambition_platformer2d::vfx::Effect::DamageBox(b) = &request.effect {
            cap.count += 1;
            cap.last = b.center;
            cap.last_owner = Some(request.owner);
            cap.last_lifetime_s = b.lifetime_s;
        }
    }
}

fn blast_count(app: &App) -> usize {
    app.world().resource::<CapturedBlasts>().count
}

fn last_blast(app: &App) -> ae::Vec2 {
    app.world().resource::<CapturedBlasts>().last
}

fn last_owner(app: &App) -> Option<Entity> {
    app.world().resource::<CapturedBlasts>().last_owner
}

fn marked(app: &App, body: Entity) -> bool {
    app.world().get::<BodyStatuses>(body).is_some_and(|held| held.holds(MARKED))
}

fn mark_on(app: &App, body: Entity) -> Mark {
    let records = app.world().get::<BodyRecords>(body).expect("the body carries records");
    Mark::from_record(records.get(&Mark::KEY).expect("the body carries a mark")).expect("the mark reads")
}

/// The mark rides the body: the blast happens where the victim is when the
/// clock runs out, not where they were hit. Running away does not help.
#[test]
fn the_blast_follows_the_body_rather_than_the_contact_point() {
    let mut app = app();
    let owner = fighter(&mut app, 0, ae::Vec2::new(0.0, 0.0));
    let victim = fighter(&mut app, 1, ae::Vec2::new(100.0, 0.0));
    land_a_marking_hit(&mut app, owner, victim, 0.1);
    app.update();
    assert!(marked(&app, victim), "the struck body carries no mark");

    // They run before it goes off.
    app.world_mut().get_mut::<ae::BodyKinematics>(victim).expect("victim has kinematics").pos =
        ae::Vec2::new(400.0, 0.0);
    for _ in 0..12 {
        app.update();
    }

    assert_eq!(blast_count(&app), 1, "expected exactly one detonation");
    assert_eq!(
        last_blast(&app),
        ae::Vec2::new(400.0, 0.0),
        "the blast must land where the marked body ran TO, not where it was hit"
    );
    assert!(!marked(&app, victim), "a spent mark must be let go of, or it detonates every frame after");
}

/// A spent mark does not keep detonating: this keeps running after the first
/// blast.
#[test]
fn a_spent_mark_does_not_keep_detonating() {
    let mut app = app();
    let owner = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, owner, victim, 0.05);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(blast_count(&app), 1, "one mark must produce exactly one blast however long the match runs");
}

/// The mark waits out its fuse; otherwise `fuse_s` would be documentation.
#[test]
fn the_mark_waits_out_its_fuse() {
    let mut app = app();
    let owner = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, owner, victim, 0.5);
    app.update();
    assert_eq!(blast_count(&app), 0, "the mark detonated on the tick it was applied; the fuse buys nothing");
    for _ in 0..40 {
        app.update();
    }
    assert_eq!(blast_count(&app), 1, "and it must go off eventually");
}

/// The tick of the strike is tick zero of the fuse, so an N-tick fuse gives
/// the victim exactly N ticks. The long-fuse test above cannot see one tick.
///
/// `dt = 1/64` is exactly representable, so "N ticks" is arithmetic, not
/// rounding.
#[test]
fn a_fuse_of_n_ticks_gives_the_victim_exactly_n_ticks() {
    let dt = 1.0 / 64.0;
    for ticks in [1usize, 3, 8] {
        let mut app = app_with(Road::Linked, dt);
        let owner = fighter(&mut app, 0, ae::Vec2::ZERO);
        let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
        land_a_marking_hit(&mut app, owner, victim, dt * ticks as f32);
        // The tick of the strike: tick zero. No blast, however short the fuse.
        app.update();
        assert_eq!(blast_count(&app), 0, "a {ticks}-tick fuse went off on the tick it was applied");
        // Ticks 1..N-1: still waiting.
        for waited in 1..ticks {
            app.update();
            assert_eq!(blast_count(&app), 0, "a {ticks}-tick fuse went off after {waited} tick(s)");
        }
        // Tick N: the blast.
        app.update();
        assert_eq!(blast_count(&app), 1, "a {ticks}-tick fuse had not gone off {ticks} tick(s) after it was applied");
    }
}

/// An effect the module does not bind falls through untouched.
#[test]
fn a_hit_carrying_somebody_elses_effect_leaves_no_mark() {
    let mut app = app();
    let owner = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    // Mark-shaped params on purpose: only the key separates this from a mark.
    hit_with(&mut app, owner, victim, "smash.some_other_effect", 0.1);
    for _ in 0..30 {
        app.update();
    }
    assert!(!marked(&app, victim), "an unrelated on-hit effect marked the body");
    assert_eq!(blast_count(&app), 0, "and it produced a blast");
}

/// A second mark refreshes and does not stack, so "how long have I got" stays
/// readable.
#[test]
fn a_second_mark_refreshes_the_clock_and_does_not_add_a_second_blast() {
    let mut app = app();
    let owner = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, owner, victim, 0.2);
    for _ in 0..6 {
        app.update();
    }
    land_a_marking_hit(&mut app, owner, victim, 0.2);
    app.update();
    assert_eq!(mark_on(&app, victim).fuse_s, 0.2, "the second mark did not refresh the clock");
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(blast_count(&app), 1, "two marks on one body must still be one detonation");
}

/// Who is credited is not who was hit. The blast's owner must be the fighter
/// who landed the marking strike; kills, grudges, staleness and rage key on it.
#[test]
fn the_blast_is_credited_to_the_fighter_who_landed_the_mark() {
    let mut app = app();
    let attacker = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    // A bystander in the blast: whoever it KOs is credited to the owner.
    let _bystander = fighter(&mut app, 2, ae::Vec2::new(60.0, 0.0));
    land_a_marking_hit(&mut app, attacker, victim, 0.05);
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(blast_count(&app), 1, "premise: the mark detonated");
    assert_eq!(
        last_owner(&app),
        Some(attacker),
        "the blast is not owned by the striker, so a bystander it KOs is credited to somebody else"
    );
    let live = app.world().resource::<CapturedBlasts>().last_lifetime_s;
    assert_eq!(live, params(0.05).blast_s, "the blast is live for {live} s, not the mark's authored time");
}

/// The last striker owns the blast: the refresh replaces the mark, so the
/// refresher is credited when it goes off.
#[test]
fn a_refresh_by_a_second_attacker_hands_them_the_credit() {
    let mut app = app();
    let first = fighter(&mut app, 0, ae::Vec2::ZERO);
    let second = fighter(&mut app, 2, ae::Vec2::new(-40.0, 0.0));
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, first, victim, 0.2);
    for _ in 0..3 {
        app.update();
    }
    land_a_marking_hit(&mut app, second, victim, 0.05);
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(blast_count(&app), 1, "premise: one refreshed mark, one blast");
    assert_eq!(
        last_owner(&app),
        Some(second),
        "the refreshed mark still credits the FIRST marker; the refresh rule is being inherited rather than decided"
    );
}

/// No credit, no mark: a mark whose striker cannot be named would credit
/// somebody else.
#[test]
fn a_strike_from_a_body_with_no_identity_leaves_no_mark() {
    let mut app = app();
    let anonymous = app.world_mut().spawn(ae::BodyKinematics::default()).id();
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, anonymous, victim, 0.1);
    // On the strike's own tick: a mark whose blast the engine would refuse
    // ends by itself, so a later look cannot tell it was never applied.
    app.update();
    assert!(!marked(&app, victim), "a mark with nobody to credit was applied anyway");
    for _ in 0..12 {
        app.update();
    }
    assert_eq!(blast_count(&app), 0);
}

/// A mark lives as long as the stock it was put on. It is retired on the tick
/// after the body leaves play, so it cannot go off from an `OutOfPlay` body or
/// on the next stock.
///
/// The rewind is staged by hand: a restore brings back the body's status and
/// records and removes `OutOfPlay`. The rule must re-derive from state; a
/// one-shot message would not fire again on resimulation.
#[test]
fn a_mark_is_retired_when_its_body_leaves_play_and_comes_back_on_a_rewind() {
    let mut app = app();
    let attacker = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, attacker, victim, 0.1);
    app.update();
    assert!(marked(&app, victim), "premise: the victim is marked");
    let statuses = app.world().get::<BodyStatuses>(victim).cloned().unwrap();
    let records = app.world().get::<BodyRecords>(victim).cloned().unwrap();
    let live = mark_on(&app, victim);

    // The KO: the stock spend puts the body out of play.
    app.world_mut().entity_mut(victim).insert(OutOfPlay);
    app.update();
    assert!(!marked(&app, victim), "the body left play and its mark is still on it");
    // The old fuse runs out while the body is out: nothing may go off.
    for _ in 0..12 {
        app.update();
    }
    assert_eq!(blast_count(&app), 0, "a mark detonated from a body that is out of play");

    // The rewind to before the KO: the mark is restored; the KO is unmade.
    app.world_mut().entity_mut(victim).insert((statuses, records)).remove::<OutOfPlay>();
    app.update();
    assert!(marked(&app, victim), "on the rewound timeline the body is in play and the mark must tick, not retire");
    assert!(mark_on(&app, victim).fuse_s < live.fuse_s, "the restored mark is not ticking");

    // Resimulate the KO: retired again, from state.
    app.world_mut().entity_mut(victim).insert(OutOfPlay);
    app.update();
    assert!(
        !marked(&app, victim),
        "the resimulated KO did not retire the mark — the rule fired once and is not re-derived from the body's state"
    );
    assert_eq!(blast_count(&app), 0);
}

/// A live mark is a clock row on its body, with the fraction the telegraph
/// draws; a spent one is none.
#[test]
fn a_live_mark_is_a_readable_clock_on_its_body() {
    let mut app = app();
    let attacker = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, attacker, victim, 0.5);
    // The strike's tick, then the first tick of the fuse, which shows it.
    app.update();
    app.update();
    let rows = app.world().resource::<BodyClocksView>().0.clone();
    assert_eq!(rows.len(), 1, "one mark, one clock");
    assert_eq!(rows[0].body, victim);
    let fresh = 1.0 - DT / 0.5;
    assert!(
        (rows[0].remaining_fraction - fresh).abs() < 1e-4,
        "a mark one tick into its fuse reads {}, not {fresh}",
        rows[0].remaining_fraction
    );
    for _ in 0..15 {
        app.update();
    }
    let half = app.world().resource::<BodyClocksView>().0[0].remaining_fraction;
    assert!(half < 0.6 && half > 0.4, "halfway through the fuse the clock reads {half}, not about a half");
    for _ in 0..30 {
        app.update();
    }
    assert!(app.world().resource::<BodyClocksView>().0.is_empty(), "the mark went off and its clock is still drawn");
}

/// The credit outlives the body. A fighter eliminated during the fuse has
/// left; a stand-in owns the blast for its lifetime and then leaves. The
/// owner is not the victim, and the stand-in is not a participant.
#[test]
fn a_blast_whose_attacker_has_left_the_match_is_not_credited_to_the_victim() {
    let mut app = app();
    let attacker = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, attacker, victim, 0.1);
    app.update();
    assert!(marked(&app, victim), "premise: marked");

    // The attacker loses their last stock and is despawned, as
    // `take_eliminated_fighters_out_of_play` despawns an eliminated body.
    app.world_mut().entity_mut(attacker).despawn();
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(blast_count(&app), 1, "premise: the mark detonated");
    let owner = last_owner(&app).expect("the blast named an owner");
    assert_ne!(
        owner, victim,
        "the attacker's body was gone and the blast fell back to the VICTIM as its owner: a bystander it KOs is \
         credited to the fighter who was marked"
    );
    assert!(app.world().get::<CreditStandIn>(owner).is_some(), "the owner is not a credit stand-in");
    assert!(
        app.world().get::<MatchSeat>(owner).is_none(),
        "a credit stand-in must not be a PARTICIPANT, or the match counts a fighter who is out"
    );
    // It leaves once the blast can no longer land: the blast's own time and
    // the margin, counted from the detonation.
    let stays = ((params(0.1).blast_s + CREDIT_STAND_IN_MARGIN_S) / DT).ceil() as usize;
    for _ in 0..stays {
        app.update();
    }
    assert!(app.world().get_entity(owner).is_err(), "the credit stand-in outlived the blast it stood in for");
}

/// The loaded file's mark goes off as the linked one's: on the same tick,
/// where the body is, for the same time. Its runner is the loaded file.
#[test]
fn the_mark_from_its_module_file_goes_off_as_the_linked_one_does() {
    let mut blasts = Vec::new();
    for road in [Road::Linked, Road::Loaded] {
        let mut app = app_with(road, DT);
        let runner = runner_of(&app, ambition_demo_smash_modules::mark::MARK_BODY);
        assert_eq!(runner.starts_with("Loaded"), road == Road::Loaded, "{road:?}: {runner}");
        let attacker = fighter(&mut app, 0, ae::Vec2::ZERO);
        let victim = fighter(&mut app, 1, ae::Vec2::new(70.0, 5.0));
        land_a_marking_hit(&mut app, attacker, victim, 0.1);
        let mut on_tick = None;
        for tick in 0..20 {
            app.update();
            if on_tick.is_none() && blast_count(&app) > 0 {
                on_tick = Some(tick);
            }
        }
        let cap = app.world().resource::<CapturedBlasts>();
        blasts.push((on_tick, cap.count, cap.last, cap.last_lifetime_s, cap.last_owner == Some(attacker)));
    }
    assert_eq!(blasts[1], blasts[0], "the loaded mark went off unlike the linked one");
    assert_eq!(blasts[0].1, 1, "premise: the linked mark went off once");
}
