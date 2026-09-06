//! The Limit meter fills at the rate the smash ruleset AUTHORS, in the shipped
//! composition — not at the platformer's.
//!
//! ⛔⛔ TWO RULESETS WERE BOTH FILLING ONE `BodyMana`. The smash ruleset authors
//! `LimitMeterFill` (Jon's baseline: a 60-point cap and 0.5/s of clock, so 120 s
//! to fill from nothing). The platformer's `avatar::regen_player_mana` refills
//! every DRIVEN body at 14.0/s so that mana is a spendable resource for charge
//! attacks, and it is registered unconditionally in the monolith's
//! `FeatureCollection` phase. A composition carrying both got both: about 4.1 s
//! to a full Limit, and a different economy for a driven fighter than for an
//! otherwise identical undriven one.
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
use ambition_platformer2d::engine_core::BodyMana;
use bevy::prelude::*;

/// Two seconds of LIVE match at the demo's own tick.
const WINDOW: usize = 120;

/// The platformer's own rate, forced back on for the control arm.
const PLATFORMER_RATE: f32 = 14.0;

/// The highest `BodyMana` in the world, and how many bodies carry one.
fn meters(app: &mut App) -> (f32, usize) {
    let mut query = app.world_mut().query::<&BodyMana>();
    let values: Vec<f32> = query
        .iter(app.world())
        .map(|mana| mana.meter.current)
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

/// Limit gained by the fullest meter over `WINDOW` ticks of the same fight.
fn gained_over_the_window(regen: Option<f32>) -> (f32, usize) {
    let mut app = a_live_match(regen);
    let (before, seated) = meters(&mut app);
    for _ in 0..WINDOW {
        app.update();
    }
    let (after, _) = meters(&mut app);
    (after - before, seated)
}

/// ⛔⛔ AN A/B AGAINST THE SAME FIGHT, because the absolute number cannot answer
/// a RATE question in a world where fighters are also hitting each other.
///
/// My first version of this test asserted a ceiling on the gain and failed at
/// **22.3 over two seconds** with the fix already in place — because Jon's
/// authored damage sources (+2.0 and 0.2x per instance TAKEN, +1.0 and 0.1x
/// DEALT) legitimately produce that much when two level-5 CPUs trade five hits.
/// The threshold could not tell an authored fill from a leaked one.
///
/// ⇒ The simulation is deterministic, so the SAME match run twice differs only
/// by the resource under test. That difference is the leak, isolated.
#[test]
fn the_platformers_mana_regen_does_not_reach_a_fighters_limit() {
    let (shipped, seated) = gained_over_the_window(None);
    // ⛔ ANTI-VACUITY. A world with no metered body satisfies everything below
    // forever, and it is what a match that never started looks like. It caught
    // exactly that on this test's first run.
    assert!(
        seated >= 2,
        "the live match composed {seated} bodies carrying a `BodyMana`; this \
         guard is asking an empty world"
    );

    let (leaking, _) = gained_over_the_window(Some(PLATFORMER_RATE));
    // Two seconds of 14.0/s is 28 points of Limit that nobody authored.
    let leak = leaking - shipped;
    assert!(
        leak > 10.0,
        "forcing the platformer's {PLATFORMER_RATE}/s back on changed the Limit \
         gained by only {leak} ({leaking} against {shipped}). This CONTROL is \
         what makes the assertion below meaningful: if the two arms agree, the \
         policy resource is not reaching `regen_player_mana` at all and this \
         test proves nothing about the shipped build."
    );
    assert!(
        shipped < leaking,
        "the shipped composition gained {shipped} and the leaking one {leaking}. \
         The smash ruleset states `PlayerManaRegen(0.0)` precisely so the \
         platformer's refill — which exists to make mana spendable for charge \
         attacks — does not also fill an authored Limit. Jon's 60-point meter is \
         written to take 120 s of clock; with the leak it fills in about four."
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
                let mut q = world.query::<(
                    &ambition_platformer2d::engine_core::BodyMana,
                    Option<&MatchSeat>,
                )>();
                for (mana, seat) in q.iter(world) {
                    if mana.meter.current > cap {
                        println!(
                            "[limit-probe] tick {tick}: seat {:?} reads {:.1} with max {:.1}",
                            seat.map(|s| s.0),
                            mana.meter.current,
                            mana.meter.max
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
            let mut q = world.query::<(
                Entity,
                &ambition_platformer2d::engine_core::BodyMana,
                &MatchSeat,
            )>();
            if let Some((entity, mana, _)) = q.iter(world).find(|(_, _, seat)| seat.0 == 0) {
                let now = mana.meter.current;
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
}

/// ⛔ AND ON THE STAGE IT IS DECLARED. Without this arm the one above is
/// satisfied by a ruleset that declares nothing anywhere, which is the original
/// Limit bug wearing the opposite sign.
#[test]
fn the_stage_declares_the_rulesets_own_answers() {
    let app = a_live_match(None);
    let policy = app
        .world()
        .get_resource::<ambition_platformer2d::actors::avatar::systems::PlayerManaRegen>()
        .copied();
    assert_eq!(
        policy.map(|p| p.0),
        Some(0.0),
        "on the Smash stage the ruleset does not state its mana policy, so the \
         platformer's 14.0/s applies to every driven fighter and the authored \
         Limit is meaningless"
    );
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
}
