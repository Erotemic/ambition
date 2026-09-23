//! The Limit meter fills at the rate the smash ruleset AUTHORS, in the shipped
//! composition — not at the platformer's.
//!
//! ⛔⛔ TWO RULESETS WERE ONCE BOTH FILLING ONE METER. The smash ruleset authors
//! `LimitMeterFill` (Jon's baseline: a 60-point cap and 0.5/s of clock, so 120 s
//! to fill from nothing). The platformer's `avatar::regen_player_mana` refills
//! every DRIVEN body at 14.0/s so that mana is a spendable resource for charge
//! attacks, and it is registered unconditionally in the monolith's
//! `FeatureCollection` phase. A composition carrying both got both: about 4.1 s
//! to a full Limit, and a different economy for a driven fighter than for an
//! otherwise identical undriven one. The Limit is now its own named resource in
//! the seat's bank, so the Mana refill has nothing of the Limit's to reach.
//!
//! ⭐⭐ THIS TEST EXISTS BECAUSE THE UNIT TESTS COULD NOT SEE IT. `limit/tests.rs`
//! installs the Limit systems directly and never composes the monolith's feature
//! plugin, so the 14/s producer does not exist in that world at all. Its
//! pure-clock assertion was green the entire time the shipped game was wrong —
//! a guard whose world lacks the thing it is guarding against.
//!
//! ⇒ So this one composes the REAL demo app and asks the meter.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::engine_core::resources::ActorResources;
use ambition_platformer2d::entity_catalog::smash_limit::LIMIT;
use bevy::prelude::*;

/// Two seconds of LIVE match at the demo's own tick.
const WINDOW: usize = 120;

/// The platformer's own rate, forced back on for the control arm.
const PLATFORMER_RATE: f32 = 14.0;

/// The highest Limit in the world, and how many bodies hold one.
fn meters(app: &mut App) -> (f32, usize) {
    let mut query = app.world_mut().query::<&ActorResources>();
    let values: Vec<f32> = query
        .iter(app.world())
        .filter_map(|bank| bank.level_of(&LIMIT))
        .map(|limit| limit.current)
        .collect();
    let highest = values.iter().copied().fold(f32::MIN, f32::max);
    (highest, values.len())
}

/// Start a real match, exactly as the repertoire census does: the stage opens
/// SUSPENDED and holds every fighter through a 3-2-1-GO, so a window taken
/// before the countdown ends measures bodies that are forbidden to act.
///
/// `regen` overrides the composition's mana policy, so the same fight can be run
/// with and without the platformer's refill.
fn a_live_match(regen: Option<f32>) -> App {
    a_live_match_with(regen, |_| {})
}

/// `a_live_match`, with a chance to plant state BEFORE the stage is entered —
/// which is the only moment a prior owner's configuration can exist.
fn a_live_match_from(before: impl FnOnce(&mut App)) -> App {
    a_live_match_with(None, before)
}

fn a_live_match_with(regen: Option<f32>, before: impl FnOnce(&mut App)) -> App {
    let characters = [
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
    ];
    let mut app = build_demo_app();
    // ⛔ BEFORE ANY TICK, because a prior owner's configuration has to exist
    // before Smash ever looks. ⚠ The first version of this hook was accepted as a
    // parameter and never CALLED — it compiled, the closure silently never ran,
    // and the test failed for a reason that had nothing to do with the code under
    // test. Its `println` not appearing is what gave it away.
    before(&mut app);
    for _ in 0..30 {
        app.update();
    }
    // ⛔⛔ `smash_roster`, NOT `smash_roster_at_levels` — SEAT 0 IS A HUMAN, and
    // that is the whole fixture. `regen_player_mana` refills `DrivenBodies`, and
    // a CPU is not one: an all-CPU match showed IDENTICAL gain with the
    // platformer's rate forced on and off, because it was never reaching those
    // bodies at all. ⇒ The leak is specific to DRIVEN seats, which is precisely
    // the 1v1 human-versus-human case this game is for.
    //
    // ⭐ AND A HUMAN SEAT WITH NO CONTROLLER IS THE CLEANEST INSTRUMENT AVAILABLE:
    // it takes and deals no damage, so the meter's movement is pure CLOCK and the
    // authored 0.5/s is directly readable instead of buried under Jon's damage
    // sources.
    let roster = ambition_demo_smash::smash_roster(characters);
    let countdown = ambition_demo_smash::smash_roster(characters)
        .rules
        .opening_countdown_ticks;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }
    // ⛔⛔ THE OVERRIDE GOES ON AFTER THE STAGE IS LIVE, and the first version put
    // it on before. Smash GIVES ITS DECLARATIONS BACK when the route is not the
    // stage, so a value inserted during the select screen was correctly removed
    // on the next tick and both arms of the A/B ended identical — the control
    // caught it, which is the second time that control has caught this test
    // measuring nothing.
    //
    // ⚠ On the stage the ruleset only declares when NOTHING is declared, so an
    // override standing here survives: it is the "already declared" case.
    if let Some(rate) = regen {
        app.world_mut().insert_resource(
            ambition_platformer2d::actors::avatar::systems::PlayerManaRegen(rate),
        );
    }
    app
}

/// Limit gained by the fullest meter over `WINDOW` ticks of the same fight, and
/// the Mana the seats' drained pools gained over the same window.
fn gained_over_the_window(regen: Option<f32>) -> (f32, usize, f32) {
    let mut app = a_live_match(regen);
    let seats: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<MatchSeat>>()
        .iter(app.world())
        .collect();
    for seat in &seats {
        app.world_mut()
            .get_mut::<ambition_platformer2d::engine_core::BodyMana>(*seat)
            .expect("a seat carries a mana pool")
            .meter
            .current = 0.0;
    }
    let mana = |app: &App| -> f32 {
        seats
            .iter()
            .filter_map(|seat| {
                app.world()
                    .get::<ambition_platformer2d::engine_core::BodyMana>(*seat)
            })
            .map(|mana| mana.meter.current)
            .sum()
    };
    let (before, seated) = meters(&mut app);
    let mana_before = mana(&app);
    for _ in 0..WINDOW {
        app.update();
    }
    let (after, _) = meters(&mut app);
    (after - before, seated, mana(&app) - mana_before)
}

/// ⛔⛔ AN A/B AGAINST THE SAME FIGHT, because the absolute number cannot answer
/// a RATE question in a world where fighters are also hitting each other.
///
/// An absolute ceiling on the gain once failed at **22.3 over two seconds**
/// with the fix in place — Jon's authored damage sources legitimately produce
/// that much when two fighters trade hits. The simulation is deterministic, so
/// the SAME match run twice differs only by the policy under test.
///
/// ⭐ THE LIMIT IS NAMED, SO THE TWO ARMS — Mana refill off, and on at the
/// platformer's rate — MUST AGREE EXACTLY: the refill has nothing of the
/// Limit's to reach. ⛔ And an "equal" verdict is vacuous unless the refill
/// demonstrably differs between the arms — so the control is the same seats'
/// drained Mana pool, which only the second arm may fill.
#[test]
fn the_platformers_mana_regen_does_not_reach_a_fighters_limit() {
    let (still, seated, mana_still) = gained_over_the_window(Some(0.0));
    // ⛔ ANTI-VACUITY. A world with no metered body satisfies everything below
    // forever, and it is what a match that never started looks like.
    assert!(
        seated >= 2,
        "the live match composed {seated} bodies holding a Limit; this guard is \
         asking an empty world"
    );

    // Every seat is BUILT with the match's Limit, not adopted into it.
    let mut app = a_live_match(None);
    let caps: Vec<Option<f32>> = app
        .world_mut()
        .query_filtered::<Option<&ActorResources>, With<MatchSeat>>()
        .iter(app.world())
        .map(|bank| bank.and_then(|bank| bank.level_of(&LIMIT)).map(|limit| limit.max))
        .collect();
    assert!(
        !caps.is_empty()
            && caps
                .iter()
                .all(|cap| *cap == Some(ambition_demo_smash::limit::SMASH_LIMIT.cap)),
        "a seat does not hold the match's Limit at its cap: {caps:?}",
    );

    let (refilled, _, mana_refilled) = gained_over_the_window(Some(PLATFORMER_RATE));
    assert!(
        mana_refilled - mana_still > 10.0,
        "control: the platformer's {PLATFORMER_RATE}/s refill against none moved the \
         seats' drained Mana by only {} ({mana_refilled} against {mana_still}), so \
         the refill is not reaching these bodies and the equality below proves \
         nothing",
        mana_refilled - mana_still
    );
    assert_eq!(
        refilled, still,
        "the platformer's Mana refill changed the Limit gained ({refilled} against \
         {still}): something fills the Limit that does not name it"
    );
}

/// ⭐ PROBE, PRINT-ONLY: does a real match ever REACH the Limit?
///
/// The goblin's dive is priced at `cap` — 60, the whole meter — which is what
/// makes "usable when it fills" a number rather than a mechanism. But a price
/// nobody can pay inside a match is a move that does not exist, and nothing has
/// measured the fill against a match's actual LENGTH.
///
/// Jon's baseline: 0.5/s of clock (120 s to fill on the clock ALONE), plus 1.0
/// and 0.1x per damage instance DEALT and 2.0 and 0.2x per instance TAKEN. Three
/// stocks. So the answer depends entirely on how much damage a real match trades,
/// which is not a number anybody has written down.
///
/// ⚠ NOT AN ASSERTION. What "reachable enough" means is Jon's call, and a
/// threshold invented here would be a balance ruling smuggled in as a test. This
/// prints what happened and stops.
///
/// Run: `--test smash_it -- --ignored probe_how_long_the_limit_takes --nocapture`
#[test]
#[ignore = "PROBE, print-only: how long a real match takes to fill the Limit"]
fn probe_how_long_the_limit_takes() {
    const WINDOW: usize = 5_400;
    let mut app = a_live_match(None);

    let cap = app
        .world()
        .get_resource::<ambition_demo_smash::limit::SmashLimitFill>()
        .map(|fill| fill.0.cap)
        .unwrap_or(0.0);

    let mut peak = 0.0f32;
    let mut first_full: Option<usize> = None;
    // Seat 0's charge, sampled every tick, so a WIPE is visible as a fall that
    // no spend explains.
    let mut seat0_prev = 0.0f32;
    let mut seat0_entity: Option<Entity> = None;
    let mut drops: Vec<(usize, f32, f32, bool)> = Vec::new();
    for tick in 0..WINDOW {
        app.update();
        let (highest, seated) = meters(&mut app);
        if seated == 0 {
            println!("[limit-probe] the cast left the world at tick {tick}");
            break;
        }
        if highest > peak {
            peak = highest;
            if highest > cap {
                // ⛔ ABOVE THE CAP IS NOT SUPPOSED TO HAPPEN. Report WHO and what
                // their meter's own max says, because "current above cap" and
                // "this body was never adopted" look identical from the outside.
                let world = app.world_mut();
                let mut q = world.query::<(&ActorResources, Option<&MatchSeat>)>();
                for (bank, seat) in q.iter(world) {
                    let Some(limit) = bank.level_of(&LIMIT) else {
                        continue;
                    };
                    if limit.current > cap {
                        println!(
                            "[limit-probe] tick {tick}: seat {:?} reads {:.1} with max {:.1}",
                            seat.map(|s| s.0),
                            limit.current,
                            limit.max
                        );
                    }
                }
            }
        }
        if first_full.is_none() && cap > 0.0 && highest >= cap {
            first_full = Some(tick);
        }
        {
            let world = app.world_mut();
            let mut q = world.query::<(Entity, &ActorResources, &MatchSeat)>();
            if let Some((entity, bank, _)) = q.iter(world).find(|(_, _, seat)| seat.0 == 0) {
                let now = bank.level_of(&LIMIT).map_or(0.0, |limit| limit.current);
                // ⛔ THE DISCRIMINATOR: did the ENTITY change? A new entity means
                // the fighter was respawned fresh; the same entity means
                // something RESET the meter in place. The fix differs.
                let new_body = seat0_entity.is_some_and(|was| was != entity);
                if seat0_prev - now > 1.0 {
                    drops.push((tick, seat0_prev, now, new_body));
                }
                seat0_entity = Some(entity);
                seat0_prev = now;
            }
        }
    }

    let secs = |ticks: usize| ticks as f32 / 60.0;
    println!("[limit-probe] seat 0 charge FALLS ({} of them):", drops.len());
    for (tick, before, after, new_body) in drops.iter().take(8) {
        println!(
            "[limit-probe]   tick {tick} ({:.1}s): {before:.1} -> {after:.1}  {}",
            secs(*tick),
            if *new_body { "NEW ENTITY (respawned fresh)" } else { "same entity (reset in place)" }
        );
    }
    println!("[limit-probe] cap {cap}, window {WINDOW} ticks ({:.1}s)", secs(WINDOW));
    println!("[limit-probe] peak meter reached: {peak:.1}");
    match first_full {
        Some(tick) => println!(
            "[limit-probe] first full at tick {tick} ({:.1}s) — the dive is reachable",
            secs(tick)
        ),
        None => println!(
            "[limit-probe] NEVER filled in {:.1}s. On the clock alone 60 points takes 120s, so \
             whether this is a problem depends on how long a stock match runs.",
            secs(WINDOW)
        ),
    }
}

/// ⛔⛔ LEAVING SMASH PUTS BACK WHAT WAS THERE — IT DOES NOT DELETE IT.
///
/// The first version of the override REMOVED the portal resources on leaving,
/// and Smash does not own them: `PortalPresentationPlugin` calls `init_resource`
/// for `PortalCameraContinuitySelection` and `PortalViewConeConfig`, and
/// `sync_portal_view_cones` takes `config: Res<PortalViewConeConfig>` — REQUIRED,
/// not `Option`. In the aggregate app the portal plugin is installed globally, so
/// leaving Smash deleted a resource a live system needs. ⚠ And even where nothing
/// fails, "remove" is not "restore": a developer-selected configuration was
/// destroyed rather than put back.
///
/// ⭐ THE SENTINEL IS HOW A STANDALONE COMPOSITION WITNESSES THE AGGREGATE CASE.
/// This demo has no portal plugin creating a baseline, so the interesting state —
/// somebody ELSE'S configuration standing before Smash overrides it — is planted
/// here. Without it the test could only prove "None came back as None", which is
/// exactly the case the bug got right.
#[test]
fn leaving_the_stage_restores_another_owners_portal_config() {
    use ambition_platformer2d::portal_presentation as portal_view;

    let mut app = a_live_match_from(|app| {
        // Somebody else's baseline, standing before Smash ever runs.
        app.world_mut().insert_resource(portal_view::PortalViewConeConfig {
            mode: portal_view::PortalViewConeMode::Dynamic,
            dynamic_depth_close: 999.0,
            ..Default::default()
        });
    });

    // On the stage, Smash's answer wins.
    let on_stage = app
        .world()
        .get_resource::<portal_view::PortalViewConeConfig>()
        .map(|c| c.mode);
    assert_eq!(
        on_stage,
        Some(portal_view::PortalViewConeMode::Static),
        "Smash did not take the cone while on its own stage"
    );

    // Leave.
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_SELECT_ROUTE,
            ),
        ));
    for _ in 0..30 {
        app.update();
    }

    let after = app
        .world()
        .get_resource::<portal_view::PortalViewConeConfig>()
        .cloned();
    let after = after.expect(
        "leaving Smash DELETED the portal cone config. Smash does not own it — \
         `PortalPresentationPlugin` creates it and `sync_portal_view_cones` takes \
         it as a required `Res`, so in the aggregate app that system now has a \
         missing parameter.",
    );
    assert_eq!(
        after.mode,
        portal_view::PortalViewConeMode::Dynamic,
        "the prior owner's cone MODE was not restored"
    );
    assert_eq!(
        after.dynamic_depth_close, 999.0,
        "the cone config came back as a DEFAULT rather than as the value that was \
         there. Restoring a default is not restoring: a developer-selected \
         configuration is still destroyed, just less visibly."
    );
    // ⛔⛔ AND THE LIMIT RULE MUST GO WITH IT, which nothing checked until a
    // poison walked out of this test unharmed. Removing the give-back branch for
    // `SmashLimitFill` failed NO test: the "declares nothing" arm asks an app
    // that never entered the stage, so it cannot see a rule that was declared
    // and then left standing. ⇒ A pair of arms for arrival is not a pair for
    // DEPARTURE, and a rule for a mode must not outlive the mode.
    assert!(
        app.world()
            .get_resource::<ambition_demo_smash::limit::SmashLimitFill>()
            .is_none(),
        "leaving Smash left its Limit rule standing after the mode ended"
    );
}

/// ⛔⛔ WHAT SMASH DECLARES, SMASH GIVES BACK — and composing it declares nothing.
///
/// `PlayerManaRegen(0.0)` and the portal presentation were inserted in
/// `Plugin::build`, and `ambition_app` installs `SmashExperiencePlugin` alongside
/// Ambition, Sanic and Mary-O. So merely COMPOSING Smash set the mana rate to
/// zero and the portal cone to `Static` for the whole process: a player who
/// launched the aggregate app and walked into ordinary Ambition got no mana
/// regeneration — `ambition_abilities` has real consumers, dive through volley —
/// and Ambition, the portal game, drew Smash's cones. They never enter a match.
/// Smash being LINKED was enough.
///
/// ⭐ THE DECISION WAS RIGHT AND THE LIFETIME WAS WRONG. Zero generic fill is a
/// claim about a RULESET that is running, not about a binary that can reach one.
/// This asks the composed app BEFORE any match: nothing declared.
#[test]
fn composing_smash_declares_nothing_until_the_stage_is_active() {
    let app = build_demo_app();
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actors::avatar::systems::PlayerManaRegen>()
            .is_none(),
        "composing Smash zeroed the mana rate for the whole process. Every other \
         experience in the same binary loses its charge attacks, and none of \
         them ever enters a Smash match."
    );
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::portal_presentation::PortalViewConeConfig>()
            .is_none(),
        "composing Smash chose the portal presentation for the whole process. \
         Ambition IS the portal game and would draw Smash's cones because Smash \
         happens to be linked."
    );
    // ⛔⛔ AND THE LIMIT RULE: declared at plugin build, it would run for a
    // mode nobody is in.
    assert!(
        app.world()
            .get_resource::<ambition_demo_smash::limit::SmashLimitFill>()
            .is_none(),
        "composing Smash declared its Limit rule for the whole process"
    );
}

/// ⛔ AND ON THE STAGE IT IS DECLARED. Without this arm the one above is
/// satisfied by a ruleset that declares nothing anywhere, which is the original
/// Limit bug wearing the opposite sign.
#[test]
fn the_stage_declares_the_rulesets_own_answers() {
    let app = a_live_match(None);
    let cone = app
        .world()
        .get_resource::<ambition_platformer2d::portal_presentation::PortalViewConeConfig>()
        .map(|config| config.mode);
    assert_eq!(
        cone,
        Some(ambition_platformer2d::portal_presentation::PortalViewConeMode::Static),
        "on the Smash stage the cone is {cone:?}. `Dynamic` is the engine default \
         and means a viewer-dependent window, which is undefined with two seats."
    );
    // The paired arm for the Limit: absent off-stage is only meaningful if it is
    // PRESENT here, or "nothing declared" would be satisfied by a rule that was
    // never declared at all.
    assert!(
        app.world()
            .get_resource::<ambition_demo_smash::limit::SmashLimitFill>()
            .is_some(),
        "on the Smash stage the ruleset does not declare its Limit rule, so the \
         meter never fills and the move priced at the cap is unreachable"
    );
}
