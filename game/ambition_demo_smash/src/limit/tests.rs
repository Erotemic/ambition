use super::*;
use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::combat::hitbox::{BlockedBodyHit, ResolvedBodyHit};

fn app(fill: LimitMeterFill) -> App {
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.add_message::<ResolvedBodyHit>();
    app.add_message::<BlockedBodyHit>();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    app.insert_resource(SmashLimitFill(fill));
    {
        let mut time = app
            .world_mut()
            .resource_mut::<ambition_platformer2d::time::WorldTime>();
        time.scaled_dt = 1.0 / 60.0;
        time.raw_dt = 1.0 / 60.0;
    }
    app.add_systems(Update, (fill_limit_meters, apply_authored_meter_fills).chain());
    app
}

/// A seat as the match builds it: the rule's cap, empty.
fn fighter(app: &mut App) -> Entity {
    let cap = app.world().resource::<SmashLimitFill>().0.cap;
    let seat = app
        .world_mut()
        .query::<&ambition_platformer2d::actor::MatchSeat>()
        .iter(app.world())
        .count();
    app.world_mut()
        .spawn((
            ambition_platformer2d::actor::MatchSeat(seat),
            ae::BodyMana {
                meter: ae::ResourceMeter {
                    current: 0.0,
                    ..ae::ResourceMeter::new(cap, 0.0, 0.0)
                },
            },
        ))
        .id()
}

fn meter(app: &App, who: Entity) -> f32 {
    app.world().get::<ae::BodyMana>(who).expect("has a meter").meter.current
}

/// ⛔⛔ THE SHIPPED BASELINE MUST NOT MAKE GUARDING THE GREEDY PLAY.
///
/// ⭐ THE ONE RELATIONSHIP BETWEEN TWO SOURCES THAT IS A RULE RATHER THAN A
/// NUMBER. Every other field on `LimitMeterFill` is independent by construction
/// and that independence IS Jon's ruling — but if a block ever paid more than
/// eating the hit, the maximising play would be to guard and taking damage would
/// stop being a cost. ⇒ Asserted against the SHIPPED constant, because the way
/// this breaks is somebody tuning `on_block` up and nothing noticing.
#[test]
fn jons_baseline_keeps_guarding_the_safe_option_and_not_the_greedy_one() {
    let fill = LimitMeterFill::JONS_BASELINE;
    assert!(
        guarding_is_the_safe_option(&fill),
        "a block pays {} and eating the hit pays {} — so a fighter maximises \
         the Limit by blocking, which inverts the defensive read",
        fill.on_block,
        fill.on_damage_taken,
    );
    assert!(
        fill.on_block > 0.0,
        "the shipped baseline pays nothing for a successful block, so the \
         source ships turned off and every test below it proves only the road",
    );
    assert!(
        fill.problems().is_empty(),
        "the shipped baseline is not a legal fill: {:?}",
        fill.problems(),
    );
}

/// ⛔⛔ A BLOCKED STRIKE IS NOT A DAMAGE INSTANCE, WHICH IS WHY THE GAP LASTED.
///
/// The campaign's B1 row measured it: `BlockedBodyHit` was read in exactly ONE
/// place, to arm an `OnBlock` cancel on the ATTACKER. Nothing paid the fighter
/// whose guard ate the hit — so the hard defensive read had four vocabularies
/// and the soft one had none.
///
/// ⭐ THIS IS THE ARM THAT PROVES IT WAS A GAP AND NOT AN OVERSIGHT IN THE
/// FIXTURE: the same exchange under the FULL baseline, where every damage source
/// is non-zero, still moves nothing without `on_block`. A blocked strike writes
/// no `ResolvedBodyHit`, so `taken()` and `dealt()` are never consulted.
#[test]
fn a_block_moves_nothing_at_all_unless_the_block_source_is_authored() {
    let mut silent = app(LimitMeterFill {
        on_block: 0.0,
        per_second: 0.0,
        ..LimitMeterFill::JONS_BASELINE
    });
    let guard = fighter(&mut silent);
    let poker = fighter(&mut silent);
    silent.world_mut().write_message(BlockedBodyHit {
        victim: guard,
        attacker: Some(poker),
            attacker_move_instance: None,
    });
    silent.update();
    assert_eq!(
        (meter(&silent, guard), meter(&silent, poker)),
        (0.0, 0.0),
        "a blocked strike moved a meter through some OTHER source, so this \
         source is not the thing being measured",
    );
}

/// ⭐ THE STRIKER MAY BE UNKNOWN AND THE GUARD STILL ATE IT.
///
/// `BlockedBodyHit::attacker` is an `Option` because a hazard has no striker.
/// A fighter who blocks a stage spike blocked something, and the defender is the
/// half this road always knows — so the fill must not be gated on the other one.
#[test]
fn a_block_with_no_known_striker_still_pays_the_guard() {
    let mut app = app(LimitMeterFill {
        cap: 60.0,
        on_block: 1.0,
        ..Default::default()
    });
    let guard = fighter(&mut app);
    app.world_mut().write_message(BlockedBodyHit {
        victim: guard,
        attacker: None,
            attacker_move_instance: None,
    });
    app.update();
    assert!(
        (meter(&app, guard) - 1.0).abs() < 0.001,
        "a block with no known striker paid nothing: {}",
        meter(&app, guard),
    );
}

/// ⭐⭐ ALL FIVE SOURCES EXPRESS, WHICH IS THE RULING RATHER THAN THE NUMBERS.
///
/// Jon, 2026-09-05: *"make sure the meter doesn't push future uses of it into a
/// box"* — so the claim under test is not that his baseline is right, it is that
/// a mechanic wanting ONLY ONE source can author only that one and get nothing
/// else. ⇒ Four rules, four fighters, four different reasons the meter moved.
#[test]
fn each_fill_source_works_alone_so_no_mechanic_is_boxed_out() {
    let only = |f: LimitMeterFill| f;

    // 1. A PURE CLOCK. Nothing happens; the meter still fills.
    let mut clock = app(only(LimitMeterFill {
        cap: 60.0,
        per_second: 0.5,
        ..Default::default()
    }));
    let who = fighter(&mut clock);
    for _ in 0..120 {
        clock.update();
    }
    let ticked = meter(&clock, who);
    assert!(
        (ticked - 1.0).abs() < 0.05,
        "two seconds of a 0.5/s clock gave {ticked}, not one tick"
    );

    // 2. DAMAGE DEALT ONLY.
    let mut dealt = app(only(LimitMeterFill {
        cap: 60.0,
        on_damage_dealt: 1.0,
        per_damage_dealt: 0.1,
        ..Default::default()
    }));
    let hitter = fighter(&mut dealt);
    let hurt = fighter(&mut dealt);
    dealt.world_mut().write_message(ResolvedBodyHit {
        victim: hurt,
        attacker: Some(hitter),
        hitlag_seconds: 0.0,
        source: ambition_platformer2d::combat::HitSource::Melee,
        damage: 10,
            attacker_move_instance: None,
    });
    dealt.update();
    assert!(
        (meter(&dealt, hitter) - 2.0).abs() < 0.001,
        "1 + 0.1x10 should be 2, got {}",
        meter(&dealt, hitter)
    );
    assert_eq!(
        meter(&dealt, hurt),
        0.0,
        "a dealt-only rule paid the VICTIM, so the sources are not independent"
    );

    // 3. DAMAGE TAKEN ONLY.
    let mut taken = app(only(LimitMeterFill {
        cap: 60.0,
        on_damage_taken: 2.0,
        per_damage_taken: 0.2,
        ..Default::default()
    }));
    let a = fighter(&mut taken);
    let b = fighter(&mut taken);
    taken.world_mut().write_message(ResolvedBodyHit {
        victim: b,
        attacker: Some(a),
        hitlag_seconds: 0.0,
        source: ambition_platformer2d::combat::HitSource::Melee,
        damage: 10,
            attacker_move_instance: None,
    });
    taken.update();
    assert!(
        (meter(&taken, b) - 4.0).abs() < 0.001,
        "2 + 0.2x10 should be 4, got {}",
        meter(&taken, b)
    );
    assert_eq!(
        meter(&taken, a),
        0.0,
        "a taken-only rule paid the ATTACKER"
    );

    // 4. A SUCCESSFUL BLOCK ONLY — the soft defensive read, and the source that
    //    every other one reads zero for: a blocked strike deals no damage, so it
    //    writes no `ResolvedBodyHit` at all.
    let mut blocked = app(only(LimitMeterFill {
        cap: 60.0,
        on_block: 1.5,
        ..Default::default()
    }));
    let guard = fighter(&mut blocked);
    let poker = fighter(&mut blocked);
    blocked.world_mut().write_message(BlockedBodyHit {
        victim: guard,
        attacker: Some(poker),
            attacker_move_instance: None,
    });
    blocked.update();
    assert!(
        (meter(&blocked, guard) - 1.5).abs() < 0.001,
        "a block-only rule paid the fighter who blocked {}, not 1.5",
        meter(&blocked, guard),
    );
    assert_eq!(
        meter(&blocked, poker),
        0.0,
        "a block-only rule paid the fighter who SWUNG — so throwing attacks into \
         a shield charges your own meter, which is backwards",
    );

    // 5. A MOVE FILLS IT — the "cloud like meter", with every other source zero.
    let mut cloud = app(only(LimitMeterFill {
        cap: 60.0,
        ..Default::default()
    }));
    let caster = fighter(&mut cloud);
    cloud.world_mut().write_message(ActorActionMessage {
        actor: caster,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special(FILL_METER.to_string()),
            params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                &FillMeterParams { amount: 12.0 },
            )
            .expect("fill params serialize"),
        },
        move_instance: None,
    });
    cloud.update();
    assert_eq!(
        meter(&cloud, caster),
        12.0,
        "a meter filled ONLY by a move is exactly the case Jon named, and it did \
         not fill"
    );

    // 5. DECAY ONLY — the fifth source, and the only one that SUBTRACTS. A Limit
    // that must be spent rather than banked is the other obvious shape, and a
    // meter with no rising source at all is the cleanest way to watch it work.
    let mut fading = app(only(LimitMeterFill {
        cap: 60.0,
        decay_per_second: 5.0,
        ..Default::default()
    }));
    let holder = fighter(&mut fading);
    fading
        .world_mut()
        .get_mut::<ae::BodyMana>(holder)
        .expect("has a meter")
        .meter
        .current = 30.0;
    for _ in 0..120 {
        fading.update();
    }
    let left = meter(&fading, holder);
    assert!(
        (left - 20.0).abs() < 0.05,
        "two seconds of a 5/s decay should leave 20 of 30, got {left}"
    );

    // ⛔ AND IT FLOORS AT ZERO RATHER THAN GOING NEGATIVE. A meter in debt would
    // have to be refilled PAST zero before a priced move became reachable again,
    // which is a rule nobody authored and which looks exactly like the move
    // being broken.
    for _ in 0..600 {
        fading.update();
    }
    assert_eq!(
        meter(&fading, holder),
        0.0,
        "a decaying meter ran past empty into debt"
    );
}

/// ⛔⛔ A DECAY THAT OUTRUNS EVERY SOURCE IS A METER NOBODY CAN FILL, AND IT
/// FAILS SILENTLY: the fighter charges, the number falls back, and the priced
/// move simply never becomes available. Nothing errors, nothing logs, and the
/// special looks broken rather than unaffordable.
///
/// ⚠ ONLY THE UNARGUABLE CASE. Against the damage sources the question is
/// undecidable — how much a fighter will be hit is a match, not a number — so
/// this refuses the clock-only rule whose own arithmetic cannot reach the cap
/// and stays quiet otherwise. A validator that guessed at the rest would refuse
/// legitimate designs.
#[test]
fn a_decay_that_outruns_the_only_source_is_named_as_a_problem() {
    let unreachable = LimitMeterFill {
        cap: 60.0,
        per_second: 0.5,
        decay_per_second: 0.5,
        ..Default::default()
    };
    assert!(
        unreachable
            .problems()
            .iter()
            .any(|p| p.contains("can never reach its cap")),
        "a meter whose decay equals its only source was called fine: {:?}",
        unreachable.problems()
    );

    // ⛔ THE CONTROL, and without it the assertion above is satisfied by a
    // validator that refuses every decay. A decay SLOWER than the clock is a
    // perfectly ordinary Limit that simply fills more slowly.
    let slow = LimitMeterFill {
        decay_per_second: 0.2,
        ..unreachable
    };
    assert!(
        slow.problems().is_empty(),
        "a decay slower than the clock was refused, which makes the lever          unauthorable: {:?}",
        slow.problems()
    );

    // ⛔ AND A DECAY BESIDE A DAMAGE SOURCE IS NOT DECIDABLE HERE. This is the
    // case the validator must stay quiet about: the meter is reachable in any
    // match where somebody gets hit.
    let with_damage = LimitMeterFill {
        on_damage_taken: 2.0,
        ..unreachable
    };
    assert!(
        with_damage.problems().is_empty(),
        "a decay was refused even though damage fills the meter: {:?}",
        with_damage.problems()
    );
}

/// ⛔ NO FILL DECLARED, NO METER MOVEMENT — what every match did before this.
#[test]
fn a_match_that_declares_no_limit_fills_nothing() {
    let mut app = app(LimitMeterFill::default());
    // A seat carrying some other meter, full: nothing here may move it.
    let who = app
        .world_mut()
        .spawn((ambition_platformer2d::actor::MatchSeat(0), ae::BodyMana::default()))
        .id();
    let before = meter(&app, who);
    for _ in 0..120 {
        app.update();
    }
    // ⛔ UNCHANGED, NOT ZERO. With no Limit declared the system returns before
    // touching anything, so the component keeps whatever it had — which for
    // `BodyMana::default()` is a FULL 100-point mana pool. Asserting zero would
    // be asserting that this system reached in and emptied a meter it was told
    // nothing about.
    assert_eq!(
        meter(&app, who),
        before,
        "a match with no Limit rule moved a meter anyway"
    );
}

/// ⭐⭐ THE MECHANISM MUST PERMIT WHAT THIS RULESET REFUSES, and that separation
/// is the whole point of the split.
///
/// ⛔ `on_block >= on_damage_taken` WAS BRIEFLY A VALIDITY RULE inside
/// `LimitMeterFill::problems()` — the generic vocabulary of independent meter
/// sources. A review caught it: a coherent future meter may deliberately reward
/// defensive play (parry 10, damage taken 0), and the mechanism would have
/// refused to let it exist. **The generic type validates that a fill is WELL
/// FORMED; whether one source should outrank another is a balance doctrine and
/// belongs to whoever owns the balance.**
///
/// ⇒ So this asserts BOTH halves at once: the mechanism accepts the
/// defence-rewarding fill, and this ruleset's predicate still calls it greedy.
/// Either half alone would pass against a version that had simply deleted the
/// rule instead of relocating it.
#[test]
fn the_generic_meter_permits_a_fill_this_ruleset_calls_greedy() {
    let fill = LimitMeterFill {
        cap: 60.0,
        on_block: 10.0,
        on_damage_taken: 0.0,
        ..Default::default()
    };
    assert!(
        fill.problems().is_empty(),
        "the generic mechanism refused a defence-rewarding meter, so a ruleset's \
         balance doctrine is back inside the vocabulary: {:?}",
        fill.problems(),
    );
    assert!(
        !guarding_is_the_safe_option(&fill),
        "the ruleset predicate stopped recognising the inversion, so moving it \
         out of `problems()` lost the rule rather than relocating it",
    );
}

/// ⛔⛔ A NEGATIVE BLOCK REWARD IS MECHANICALLY INVALID, NOT A BALANCE CHOICE.
///
/// `fill_limit_meters` runs `mana.meter.refill(fill.blocked())` and
/// `ResourceMeter::refill` adds whatever it is handed and clamps. So `on_block =
/// -10` was accepted as WELL FORMED and a successful block DRAINED TEN METER.
///
/// ⚠ THE FIELD WAS ADDED TO THE MECHANISM AND NOT TO THE CHECK THAT VALIDATES
/// THE MECHANISM. Every other fill source is in that loop; `on_block` joined the
/// struct as a fourth source and the loop is DATA, so no compiler asked for it.
/// ⇒ Adding a field to an enumerated list means visiting every list that
/// enumerates its siblings — and the way to find them is to grep the field NEXT
/// to it, not the feature's name.
#[test]
fn a_negative_block_reward_is_rejected() {
    let mut fill = LimitMeterFill::JONS_BASELINE;
    fill.on_block = -1.0;
    let problems = fill.problems();
    assert!(
        problems.iter().any(|p| p.contains("on_block")),
        "a block that DRAINS the meter must be reported as malformed; got {problems:?}"
    );

    // ⭐ AND THE POSITIVE HALF: the baseline's own positive reward stays legal, so
    // this is not a check that simply refuses the field.
    let baseline = LimitMeterFill::JONS_BASELINE;
    assert!(
        baseline.on_block > 0.0 && !baseline.problems().iter().any(|p| p.contains("on_block")),
        "the shipping baseline rewards blocking and must remain valid"
    );
}

/// ⛔⛔ A METER FILLED ONLY BY BLOCKING IS REACHABLE, AND VALIDATION SAID IT WAS NOT.
///
/// The decay/reachability check asked whether decay cancels the passive clock and
/// no DAMAGE source remains — a sentence written when there were three sources.
/// A defensive scheme whose only reward is blocking was reported as impossible to
/// fill, and repeated blocks plainly fill it.
///
/// ⚠ THE ERROR MESSAGE WAS THE TELL: "no damage source fills this meter", in a
/// struct that had grown a block source. ⇒ When a message enumerates the world,
/// it is a specification, and it goes stale exactly like the code beside it.
#[test]
fn blocking_alone_can_fill_a_meter_the_clock_cannot() {
    let fill = LimitMeterFill {
        cap: 60.0,
        per_second: 0.5,
        decay_per_second: 0.5,
        on_block: 10.0,
        on_damage_dealt: 0.0,
        per_damage_dealt: 0.0,
        on_damage_taken: 0.0,
        per_damage_taken: 0.0,
    };
    assert!(
        !fill.problems().iter().any(|p| p.contains("never reach")),
        "a meter whose passive fill is cancelled by decay is still fillable by \
         blocking; got {:?}",
        fill.problems()
    );

    // ⭐ THE CASE THE CHECK IS FOR MUST STILL FAIL: same meter, no block reward.
    let unfillable = LimitMeterFill {
        on_block: 0.0,
        ..fill
    };
    assert!(
        unfillable.problems().iter().any(|p| p.contains("never reach")),
        "with no block reward and decay cancelling the clock, nothing fills this \
         meter — the check must still say so, or adding `on_block` has disabled it"
    );
}
