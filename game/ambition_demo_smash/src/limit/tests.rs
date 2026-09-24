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

/// A bank holding exactly these declarations.
fn bank(declarations: &[ambition_platformer2d::resource_spec::ResourceDeclaration]) -> ActorResources {
    ActorResources::declared(declarations)
        .expect("valid")
        .expect("declared")
}

/// A resource that is not the Limit: what an exploration body might hold.
const OTHER: ambition_platformer2d::resource_spec::ResourceId =
    ambition_platformer2d::resource_spec::ResourceId::from_static("test.mana");

fn other_pool() -> ActorResources {
    bank(&[ambition_platformer2d::resource_spec::ResourceDeclaration::new(
        OTHER,
        100.0,
        ambition_platformer2d::resource_spec::ResourceStart::Full,
    )])
}

/// A seat as the match builds it: the rule's Limit declaration, empty.
fn fighter(app: &mut App) -> Entity {
    let declaration = app.world().resource::<SmashLimitFill>().0.declaration();
    let seat = app
        .world_mut()
        .query::<&ambition_platformer2d::actor::MatchSeat>()
        .iter(app.world())
        .count();
    app.world_mut()
        .spawn((
            ambition_platformer2d::actor::MatchSeat(seat),
            bank(&[declaration]),
        ))
        .id()
}

fn meter(app: &App, who: Entity) -> f32 {
    level(app, who, &LIMIT)
}

fn level(
    app: &App,
    who: Entity,
    resource: &ambition_platformer2d::resource_spec::ResourceId,
) -> f32 {
    app.world()
        .get::<ActorResources>(who)
        .expect("has a bank")
        .level_of(resource)
        .expect("holds the resource")
        .current
}

/// The shipped baseline must not make guarding the greedy play.
///
/// Fill sources are independent by design, with one rule between them: a
/// block must not pay more than eating the hit, or taking damage stops being
/// a cost. Asserted against the shipped constant, so raising `on_block`
/// breaks it.
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

/// A blocked strike is not a damage instance.
///
/// A blocked strike writes no `ResolvedBodyHit`, so even the full baseline,
/// with every damage source non-zero, moves nothing without `on_block`.
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

/// The striker may be unknown and the guard still ate it.
/// `BlockedBodyHit::attacker` is an `Option` because a hazard has no striker;
/// the fill must not depend on it.
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

/// Every fill source works alone.
///
/// Jon's ruling is that the meter must not box in future mechanics, so a
/// mechanic can author only one source and get nothing else.
#[test]
fn each_fill_source_works_alone_so_no_mechanic_is_boxed_out() {
    let only = |f: LimitMeterFill| f;

    // 1. A pure clock: nothing happens, and the meter still fills.
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

    // 2. Damage dealt only.
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

    // 3. Damage taken only.
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

    // 4. A successful block only: a blocked strike deals no damage, so it
    //    writes no `ResolvedBodyHit`.
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

    // 5. A move fills it, with every other source zero.
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

    // 6. Decay only: the one source that subtracts. A meter with no rising
    // source shows it most clearly.
    let mut fading = app(only(LimitMeterFill {
        cap: 60.0,
        decay_per_second: 5.0,
        ..Default::default()
    }));
    let holder = fighter(&mut fading);
    fading
        .world_mut()
        .get_mut::<ActorResources>(holder)
        .expect("has a bank")
        .level_of_mut(&LIMIT)
        .expect("holds a Limit")
        .current = 30.0;
    for _ in 0..120 {
        fading.update();
    }
    let left = meter(&fading, holder);
    assert!(
        (left - 20.0).abs() < 0.05,
        "two seconds of a 5/s decay should leave 20 of 30, got {left}"
    );

    // It floors at zero: a meter in debt would need refilling past zero
    // before a priced move became reachable, which looks like a broken move.
    for _ in 0..600 {
        fading.update();
    }
    assert_eq!(
        meter(&fading, holder),
        0.0,
        "a decaying meter ran past empty into debt"
    );
}

/// A decay that outruns every source makes a meter nobody can fill, and the
/// priced move just never becomes available, silently.
///
/// Only the decidable case: against damage sources the answer depends on the
/// match, so this refuses only the clock-only rule that cannot reach the cap.
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

    // The control: a decay slower than the clock is an ordinary, slower Limit.
    // Without it, a validator that refuses every decay would pass.
    let slow = LimitMeterFill {
        decay_per_second: 0.2,
        ..unreachable
    };
    assert!(
        slow.problems().is_empty(),
        "a decay slower than the clock was refused, which makes the lever          unauthorable: {:?}",
        slow.problems()
    );

    // A decay beside a damage source is not decidable here: the meter is
    // reachable in any match where somebody gets hit.
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

/// No fill declared, no meter movement.
#[test]
fn a_match_that_declares_no_limit_fills_nothing() {
    let mut app = app(LimitMeterFill::default());
    // A seat carrying some other resource, full: nothing here may move it.
    let who = app
        .world_mut()
        .spawn((ambition_platformer2d::actor::MatchSeat(0), other_pool()))
        .id();
    for _ in 0..120 {
        app.update();
    }
    // Unchanged, not zero: this system must not touch a resource it was told
    // nothing about.
    assert_eq!(
        level(&app, who, &OTHER),
        100.0,
        "a match with no Limit rule moved a meter anyway"
    );
}

/// The Limit is reached by name, never as "the body's meter". A body holding
/// no Limit gains nothing from any source, and its other resources (such as an
/// exploration Mana pool) do not move.
#[test]
fn a_body_that_holds_no_limit_gains_nothing_from_any_limit_source() {
    let mut app = app(LimitMeterFill::JONS_BASELINE);
    let limited = fighter(&mut app);
    let mut drained = other_pool();
    drained.level_of_mut(&OTHER).expect("held").drain(50.0);
    let unlimited = app
        .world_mut()
        .spawn((ambition_platformer2d::actor::MatchSeat(1), drained))
        .id();
    for who in [limited, unlimited] {
        app.world_mut().write_message(ActorActionMessage {
            actor: who,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::Special(FILL_METER.to_string()),
                params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                    &FillMeterParams { amount: 12.0 },
                )
                .expect("fill params serialize"),
            },
            move_instance: None,
        });
    }
    for _ in 0..60 {
        app.update();
    }
    assert!(
        meter(&app, limited) > 12.0,
        "control: the seat that holds a Limit was not filled by the clock and the \
         technique, so this fixture cannot tell a named fill from no fill"
    );
    assert_eq!(
        level(&app, unlimited, &OTHER),
        50.0,
        "a Limit source filled a body's OTHER resource — the fill did not name \
         what it fills"
    );
}

/// The mechanism permits what this ruleset refuses.
///
/// `LimitMeterFill::problems()` validates that a fill is well formed. Whether
/// one source should outrank another is balance, owned by the ruleset: a
/// future meter may reward defence (parry 10, damage taken 0). So this asserts
/// both halves: the mechanism accepts that fill, and this ruleset's predicate
/// calls it greedy.
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

/// A negative block reward is invalid, not a balance choice.
///
/// `fill_limit_meters` passes `fill.blocked()` to `ResourceMeter::refill`,
/// which adds whatever it gets, so `on_block = -10` would drain the meter on a
/// block. When adding a fill source, update every list that enumerates the
/// other sources.
#[test]
fn a_negative_block_reward_is_rejected() {
    let mut fill = LimitMeterFill::JONS_BASELINE;
    fill.on_block = -1.0;
    let problems = fill.problems();
    assert!(
        problems.iter().any(|p| p.contains("on_block")),
        "a block that DRAINS the meter must be reported as malformed; got {problems:?}"
    );

    // The positive half: the baseline's positive reward stays legal.
    let baseline = LimitMeterFill::JONS_BASELINE;
    assert!(
        baseline.on_block > 0.0 && !baseline.problems().iter().any(|p| p.contains("on_block")),
        "the shipping baseline rewards blocking and must remain valid"
    );
}

/// A meter filled only by blocking is reachable.
///
/// The reachability check must count every rising source, not only damage.
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

    // The case the check exists for still fails: same meter, no block reward.
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
