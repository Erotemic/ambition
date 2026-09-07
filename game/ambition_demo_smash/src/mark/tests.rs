use super::*;
use ambition_platformer2d::characters::smash_mark::MarkBodyParams;
use ambition_platformer2d::combat::on_hit::OnHitEffectMessage;
use ambition_platformer2d::engine_core as ae;

const DT: f32 = 1.0 / 60.0;

fn app_with_dt(dt: f32) -> App {
    let mut app = App::new();
    app.add_message::<OnHitEffectMessage>();
    app.add_message::<ambition_platformer2d::vfx::EffectRequest>();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    {
        let mut time = app
            .world_mut()
            .resource_mut::<ambition_platformer2d::time::WorldTime>();
        time.scaled_dt = dt;
        time.raw_dt = dt;
    }
    app.init_resource::<CapturedBlasts>();
    app.init_resource::<ambition_platformer2d::sim_view::BodyClocksView>();
    // ⭐ THE SHIPPED ORDER: tick, then apply, then the observer. See
    // `detonate_body_marks` for why the tick goes first.
    app.add_systems(
        Update,
        (
            detonate_body_marks,
            apply_authored_body_marks,
            capture_blasts,
            ambition_platformer2d::sim_view::rebuild_body_clocks_view,
            publish_mark_clocks,
        )
            .chain(),
    );
    app
}

fn app() -> App {
    app_with_dt(DT)
}

/// A seated fighter at `at`. ⛔ Seated: a mark names its attacker by seat, and
/// an unseated striker leaves no mark at all (see the test that says so).
fn fighter(app: &mut App, seat: usize, at: ae::Vec2) -> Entity {
    let mut kin = ae::BodyKinematics::default();
    kin.pos = at;
    kin.size = ae::Vec2::new(24.0, 40.0);
    app.world_mut().spawn((kin, MatchSeat(seat))).id()
}

fn params(fuse_s: f32) -> MarkBodyParams {
    MarkBodyParams {
        fuse_s,
        damage: 6,
        blast_radius: 44.0,
        knockback: 1.2,
    }
}

fn land_a_marking_hit(app: &mut App, owner: Entity, victim: Entity, fuse_s: f32) {
    let effect = ambition_platformer2d::entity_catalog::EffectRef {
        key: ambition_platformer2d::characters::smash_mark::MARK_BODY.to_string(),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params(fuse_s))
            .expect("params serialize"),
    };
    app.world_mut().write_message(OnHitEffectMessage {
        owner,
        victim,
        volume: ae::CombatVolume::Circle {
            center: ae::Vec2::ZERO,
            radius: 1.0,
        },
        contact: ae::Vec2::ZERO,
        effect,
    });
}

/// ⛔⛔ ACCUMULATED AS THEY ARE WRITTEN, NOT READ AT THE END — and the first
/// draft of this file did it the wrong way and reported ZERO blasts from tests
/// whose marks plainly detonated. Bevy's messages are double-buffered: after a
/// couple of `app.update()` calls the write is gone, so "no blast" and "a blast I
/// can no longer see" are the same number.
///
/// ⇒ Second time in one day I have written that bug — the hostile-threshold
/// poison did it too. A test that steps a clock cannot ask a message buffer what
/// happened; it has to have been listening.
///
/// ⚠ A COUNT AND THE LAST ONE, NOT A `Vec` — and that is not style. A `Resource`
/// holding a collection is what `per_attempt_resource_census` exists to find,
/// because a collection in a resource is the shape that survives a match it
/// should not; the census does not exempt test modules, and it is right not to
/// (a fixture is where the shape gets normalised). Every assertion here needs
/// "how many", "where was the last" and "who owned the last", so nothing is lost.
#[derive(Resource, Default)]
struct CapturedBlasts {
    count: usize,
    last: ae::Vec2,
    last_owner: Option<Entity>,
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

/// ⛔ THE MARK RIDES THE BODY, WHICH IS THE ONLY REASON THIS IS NOT A MINE.
/// The blast happens where the victim IS when the clock runs out, not where they
/// were when they were hit — so running away does not help, and that is the whole
/// mechanic. A mark that detonated at the contact point would be a slow mine with
/// extra steps.
#[test]
fn the_blast_follows_the_body_rather_than_the_contact_point() {
    let mut app = app();
    let owner = fighter(&mut app, 0, ae::Vec2::new(0.0, 0.0));
    let victim = fighter(&mut app, 1, ae::Vec2::new(100.0, 0.0));
    land_a_marking_hit(&mut app, owner, victim, 0.1);
    app.update();
    assert!(
        app.world().get::<BodyMark>(victim).is_some(),
        "the struck body carries no mark"
    );

    // They run before it goes off.
    app.world_mut()
        .get_mut::<ae::BodyKinematics>(victim)
        .expect("victim has kinematics")
        .pos = ae::Vec2::new(400.0, 0.0);
    for _ in 0..12 {
        app.update();
    }

    assert_eq!(blast_count(&app), 1, "expected exactly one detonation");
    assert_eq!(
        last_blast(&app),
        ae::Vec2::new(400.0, 0.0),
        "the blast must land where the marked body ran TO, not where it was hit"
    );
    assert!(
        app.world().get::<BodyMark>(victim).is_none(),
        "a spent mark must be removed, or it detonates every frame after"
    );
}

/// ⛔⛔ A MARK THAT NEVER CLEARS IS A BLAST EVERY FRAME. The removal above is one
/// line and its absence would be invisible to a test that only checks the FIRST
/// detonation — so this one keeps running afterwards.
#[test]
fn a_spent_mark_does_not_keep_detonating() {
    let mut app = app();
    let owner = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, owner, victim, 0.05);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        blast_count(&app),
        1,
        "one mark must produce exactly one blast however long the match runs"
    );
}

/// ⚠ THE FUSE IS A DECISION AND THEREFORE MUST BE OBSERVABLE. A mark that went
/// off on the tick it was applied would be a hit with extra steps, and the
/// authored `fuse_s` would be documentation.
#[test]
fn the_mark_waits_out_its_fuse() {
    let mut app = app();
    let owner = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, owner, victim, 0.5);
    app.update();
    assert_eq!(
        blast_count(&app),
        0,
        "the mark detonated on the tick it was applied; the fuse buys nothing"
    );
    for _ in 0..40 {
        app.update();
    }
    assert_eq!(blast_count(&app), 1, "and it must go off eventually");
}

/// ⛔⛔ THE TICK OF APPLICATION IS TICK ZERO OF THE FUSE — exactly. The shipped
/// order used to be apply-then-tick, so a fresh mark lost one `sim_dt` on the
/// tick it was attached: a one-tick fuse went off THAT tick, and an N-tick fuse
/// gave the victim N-1 ticks. The long-fuse test above cannot see one tick.
///
/// ⭐ `dt = 1/64`, which is exactly representable, so "N ticks" is arithmetic
/// rather than a rounding argument: the assertion is about interval ownership,
/// not about floating point.
#[test]
fn a_fuse_of_n_ticks_gives_the_victim_exactly_n_ticks() {
    let dt = 1.0 / 64.0;
    for ticks in [1usize, 3, 8] {
        let mut app = app_with_dt(dt);
        let owner = fighter(&mut app, 0, ae::Vec2::ZERO);
        let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
        land_a_marking_hit(&mut app, owner, victim, dt * ticks as f32);
        // The tick that applies it: tick zero. No blast, however short the fuse.
        app.update();
        assert_eq!(
            blast_count(&app),
            0,
            "a {ticks}-tick fuse went off on the tick it was applied"
        );
        // Ticks 1..N-1: still waiting.
        for waited in 1..ticks {
            app.update();
            assert_eq!(
                blast_count(&app),
                0,
                "a {ticks}-tick fuse went off after {waited} tick(s)"
            );
        }
        // Tick N: the blast.
        app.update();
        assert_eq!(
            blast_count(&app),
            1,
            "a {ticks}-tick fuse had not gone off {ticks} tick(s) after it was applied"
        );
    }
}

/// ⛔ AN UNRECOGNISED KEY MUST FALL THROUGH UNTOUCHED. Every on-hit effect in the
/// game passes through this system; one that marked on any key would attach a
/// detonation to every technique that authors an on-hit at all.
#[test]
fn a_hit_carrying_somebody_elses_effect_leaves_no_mark() {
    let mut app = app();
    let owner = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    app.world_mut().write_message(OnHitEffectMessage {
        owner,
        victim,
        volume: ae::CombatVolume::Circle {
            center: ae::Vec2::ZERO,
            radius: 1.0,
        },
        contact: ae::Vec2::ZERO,
        // ⛔⛔ CARRYING MARK-SHAPED PARAMS, DELIBERATELY, and the first version of
        // this test did not — it sent an EMPTY payload, which fails to hydrate
        // whatever the key says. ⇒ Deleting the key check entirely still passed,
        // because hydration was refusing the payload rather than the filter
        // refusing the key, and the test could not tell the two apart.
        //
        // A real collision is another technique whose params happen to have this
        // shape, so that is what this sends: only the KEY separates them.
        effect: ambition_platformer2d::entity_catalog::EffectRef {
            key: "smash.some_other_effect".to_string(),
            params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params(0.1))
                .expect("params serialize"),
        },
    });
    for _ in 0..30 {
        app.update();
    }
    assert!(
        app.world().get::<BodyMark>(victim).is_none(),
        "an unrelated on-hit effect marked the body"
    );
    assert_eq!(blast_count(&app), 0, "and it produced a blast");
}

/// ⭐ A SECOND MARK REFRESHES RATHER THAN STACKS, which the authored params say
/// too. Stacking would turn one read ("how long have I got") into arithmetic
/// nobody can do mid-match, and the read is what the move sells.
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
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        blast_count(&app),
        1,
        "two marks on one body must still be one detonation"
    );
}

/// ⛔⛔ WHO IS CREDITED IS NOT WHO WAS HIT. The blast used to be owned by the
/// marked body, so a third fighter KO'd by it was credited to the VICTIM of the
/// mark — and the resolver keys kills, grudges, staleness and rage on that
/// owner. The bystander is not needed to witness it: the request names its
/// owner, and that owner must be the fighter who landed the marking strike.
#[test]
fn the_blast_is_credited_to_the_fighter_who_landed_the_mark() {
    let mut app = app();
    let attacker = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    // A bystander in the blast, to make the case the review named concrete:
    // whoever this blast KOs is credited to the request's owner.
    let _bystander = fighter(&mut app, 2, ae::Vec2::new(60.0, 0.0));
    land_a_marking_hit(&mut app, attacker, victim, 0.05);
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(blast_count(&app), 1, "premise: the mark detonated");
    assert_eq!(
        last_owner(&app),
        Some(attacker),
        "the blast is owned by the marked body, so the victim is their own \
         attacker and any bystander it KOs is credited to them"
    );
}

/// ⭐ THE LAST STRIKER OWNS THE BLAST, stated rather than inherited. Two
/// fighters mark one victim; the refresh replaces the mark, so the fighter who
/// refreshed it is the one whose pressure the clock now represents, and the one
/// credited when it goes off.
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
    assert_eq!(
        blast_count(&app),
        1,
        "premise: one refreshed mark, one blast"
    );
    assert_eq!(
        last_owner(&app),
        Some(second),
        "the refreshed mark still credits the FIRST marker; the refresh rule is \
         being inherited from the insert rather than decided"
    );
}

/// ⛔ NO SEAT, NO MARK — the mine's ruling, for the same reason. A mark whose
/// attacker cannot be named would credit somebody else for its blast.
#[test]
fn a_strike_from_an_unseated_body_leaves_no_mark() {
    let mut app = app();
    let mut kin = ae::BodyKinematics::default();
    kin.pos = ae::Vec2::ZERO;
    let unseated = app.world_mut().spawn(kin).id();
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, unseated, victim, 0.1);
    for _ in 0..12 {
        app.update();
    }
    assert!(
        app.world().get::<BodyMark>(victim).is_none(),
        "a mark with no seat to credit was applied anyway"
    );
    assert_eq!(blast_count(&app), 0);
}

/// ⛔⛔ A MARK LIVES EXACTLY AS LONG AS THE STOCK IT WAS PUT ON. With a 1.4s
/// fuse and a 1.0s death interlude, a mark on a KO'd fighter kept ticking
/// through the interlude and went off on their NEXT stock — or during the
/// interlude, blasting from an `OutOfPlay` body. Retired on the tick after the
/// body leaves play, on whichever road put it there.
///
/// ⭐ AND THE REWIND, staged the way this repo stages rewinds: a restore puts the
/// registered mark back and takes the registered `OutOfPlay` away, so this does
/// both by hand and asks whether the rule re-derives from STATE. A retire keyed
/// on a one-shot message would fire once and never again on resimulation.
#[test]
fn a_mark_is_retired_when_its_body_leaves_play_and_comes_back_on_a_rewind() {
    let mut app = app();
    let attacker = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, attacker, victim, 0.1);
    app.update();
    let live = app
        .world()
        .get::<BodyMark>(victim)
        .cloned()
        .expect("premise: the victim is marked");

    // The KO: the stock spend puts the body out of play.
    app.world_mut().entity_mut(victim).insert(OutOfPlay);
    app.update();
    assert!(
        app.world().get::<BodyMark>(victim).is_none(),
        "the body left play and its mark is still on it"
    );
    // The old fuse runs out while the body is out: nothing may go off.
    for _ in 0..12 {
        app.update();
    }
    assert_eq!(
        blast_count(&app),
        0,
        "a mark detonated from a body that is out of play"
    );

    // THE REWIND to before the KO. The mark is restored; the KO is unmade.
    app.world_mut().entity_mut(victim).insert(live.clone());
    app.world_mut().entity_mut(victim).remove::<OutOfPlay>();
    app.update();
    assert!(
        app.world().get::<BodyMark>(victim).is_some(),
        "on the rewound timeline the body is in play and the mark must tick, not retire"
    );
    assert!(
        app.world().get::<BodyMark>(victim).unwrap().fuse_s < live.fuse_s,
        "the restored mark is not ticking"
    );

    // RESIMULATE the KO: retired again, from state, not from a memory of the
    // first time.
    app.world_mut().entity_mut(victim).insert(OutOfPlay);
    app.update();
    assert!(
        app.world().get::<BodyMark>(victim).is_none(),
        "the resimulated KO did not retire the mark — the rule fired once and \
         is not re-derived from the body's state"
    );
    assert_eq!(blast_count(&app), 0);
}

/// ⭐ THE READ. A live mark publishes a clock row for its body, with the
/// fraction the telegraph draws, and a spent one publishes nothing.
#[test]
fn a_live_mark_is_a_readable_clock_on_its_body() {
    use ambition_platformer2d::sim_view::BodyClocksView;

    let mut app = app();
    let attacker = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, attacker, victim, 0.5);
    app.update();
    let rows = app.world().resource::<BodyClocksView>().0.clone();
    assert_eq!(rows.len(), 1, "one mark, one clock");
    assert_eq!(rows[0].body, victim);
    assert!(
        (rows[0].remaining_fraction - 1.0).abs() < 1e-5,
        "a fresh mark reads as a full clock, got {}",
        rows[0].remaining_fraction
    );
    for _ in 0..15 {
        app.update();
    }
    let half = app.world().resource::<BodyClocksView>().0[0].remaining_fraction;
    assert!(
        half < 0.6 && half > 0.4,
        "halfway through the fuse the clock reads {half}, not about a half"
    );
    for _ in 0..30 {
        app.update();
    }
    assert!(
        app.world().resource::<BodyClocksView>().0.is_empty(),
        "the mark went off and its clock is still published"
    );
}

/// ⛔⛔ THE CREDIT OUTLIVES THE BODY. A fighter eliminated inside the fuse has
/// no live `MatchSeat` when the mark goes off, and the first fix fell back to
/// the marked VICTIM as the blast's owner -- the original defect one case over.
/// The seat is the credit; a `SeatCredit` stand-in names it for the blast's
/// lifetime and then leaves. Never the victim.
#[test]
fn a_blast_whose_attacker_has_left_the_match_is_credited_to_their_seat_not_the_victim() {
    let mut app = app();
    let attacker = fighter(&mut app, 0, ae::Vec2::ZERO);
    let victim = fighter(&mut app, 1, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, attacker, victim, 0.1);
    app.update();
    assert!(
        app.world().get::<BodyMark>(victim).is_some(),
        "premise: marked"
    );

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
        "the attacker's body was gone and the blast fell back to the VICTIM as \
         its owner: a bystander it KOs is credited to the fighter who was marked"
    );
    assert_eq!(
        app.world().get::<SeatCredit>(owner).copied(),
        Some(SeatCredit(0)),
        "the owner does not name the attacker's seat"
    );
    assert!(
        app.world().get::<MatchSeat>(owner).is_none(),
        "a credit stand-in must not be a PARTICIPANT, or the match counts a \
         fighter who is out"
    );
    // And it leaves once the blast can no longer land.
    for _ in 0..20 {
        app.update();
    }
    assert!(
        app.world().get_entity(owner).is_err(),
        "the credit stand-in outlived the blast it stood in for"
    );
}
