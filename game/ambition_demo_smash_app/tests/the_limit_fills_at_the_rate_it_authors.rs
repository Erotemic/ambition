//! The Limit meter fills at the rate the smash ruleset authors, in the shipped
//! composition, and not at the platformer's rate.
//!
//! The smash ruleset authors `LimitMeterFill` (a 60-point cap and 0.5/s of
//! clock, so 120 s to fill from nothing). The platformer's
//! `avatar::regen_player_mana` refills every driven body at 14.0/s, and the
//! monolith registers it unconditionally in its `FeatureCollection` phase. The
//! Limit is its own named resource in the seat's bank, so the Mana refill
//! cannot reach it.
//!
//! `limit/tests.rs` installs the Limit systems directly and never composes the
//! monolith's feature plugin, so the 14/s producer does not exist there. This
//! test composes the real demo app and reads the meter.

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

/// Start a real match, as the repertoire census does. The stage opens
/// suspended and holds every fighter through a 3-2-1-GO, so wait past it.
///
/// `regen` overrides the composition's mana policy, so the same fight can be run
/// with and without the platformer's refill.
fn a_live_match(regen: Option<f32>) -> App {
    a_live_match_with(regen, |_| {})
}

/// `a_live_match`, with a hook to plant state before the stage is entered:
/// the only moment a prior owner's configuration can exist.
fn a_live_match_from(before: impl FnOnce(&mut App)) -> App {
    a_live_match_with(None, before)
}

fn a_live_match_with(regen: Option<f32>, before: impl FnOnce(&mut App)) -> App {
    let characters = [
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
    ];
    let mut app = build_demo_app();
    // Run the hook before any tick: a prior owner's configuration must exist
    // before Smash looks.
    before(&mut app);
    for _ in 0..30 {
        app.update();
    }
    // `smash_roster`, not `smash_roster_at_levels`: seat 0 is a human.
    // `regen_player_mana` refills `DrivenBodies`, and a CPU is not one, so an
    // all-CPU match cannot show the leak.
    //
    // A human seat with no controller deals and takes no damage, so the
    // meter's movement is pure clock and the authored 0.5/s is readable.
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
    // Insert the override after the stage is live. Smash removes its
    // declarations when the route is not the stage, so a value inserted on the
    // select screen is removed on the next tick. On the stage the ruleset
    // declares only when nothing is declared, so an override here survives.
    if let Some(rate) = regen {
        app.world_mut().insert_resource(
            ambition_platformer2d::actors::avatar::systems::PlayerManaRegen(rate),
        );
    }
    app
}

/// Limit gained by the fullest meter over `WINDOW` ticks of the same fight, and
/// the Mana the seats' drained pools gained over the same window.
///
/// A seat holds no Mana (the match declares only the Limit), so this gives
/// each seat a drained Mana pool beside its Limit. That is the harder case:
/// the refill's resource and the Limit share one bank, and the refill must
/// still reach only its own.
fn gained_over_the_window(regen: Option<f32>) -> (f32, usize, f32) {
    let mut app = a_live_match(regen);
    let seats: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<MatchSeat>>()
        .iter(app.world())
        .collect();
    for seat in &seats {
        let held = app
            .world()
            .get::<ActorResources>(*seat)
            .expect("a seat holds the match's bank")
            .clone();
        let mut declarations = held.layout().declarations().to_vec();
        declarations.push(ambition_platformer2d::abilities::mana::POOL);
        let mut both = ActorResources::declared(&declarations)
            .expect("the Limit and Mana are distinct resources")
            .expect("declared");
        *both.level_of_mut(&LIMIT).expect("kept") =
            held.level_of(&LIMIT).expect("a seat holds the Limit");
        both.level_of_mut(&ambition_platformer2d::abilities::mana::MANA)
            .expect("added")
            .current = 0.0;
        app.world_mut().entity_mut(*seat).insert(both);
    }
    let mana = |app: &App| -> f32 {
        seats
            .iter()
            .filter_map(|seat| {
                ambition_platformer2d::abilities::mana::level(app.world().get(*seat))
            })
            .map(|mana| mana.current)
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

/// An A/B on the same fight. An absolute ceiling cannot answer a rate
/// question while fighters also hit each other (authored damage sources
/// add Limit). The simulation is deterministic, so the same match run twice
/// differs only by the policy under test.
///
/// The Limit is named, so the two arms (Mana refill off, and on at the
/// platformer's rate) must agree exactly. The control is the seats' drained
/// Mana pool, which only the second arm may fill; without it, "equal" is
/// vacuous.
#[test]
fn the_platformers_mana_regen_does_not_reach_a_fighters_limit() {
    let (still, seated, mana_still) = gained_over_the_window(Some(0.0));
    // Anti-vacuity: a world with no metered body satisfies everything below.
    assert!(
        seated >= 2,
        "the live match composed {seated} bodies holding a Limit; this guard is \
         asking an empty world"
    );

    // Every seat is built with the match's Limit and nothing else: a fighter
    // holds no Mana merely by being a body.
    let mut app = a_live_match(None);
    let layouts: Vec<Vec<String>> = app
        .world_mut()
        .query_filtered::<Option<&ActorResources>, With<MatchSeat>>()
        .iter(app.world())
        .map(|bank| {
            bank.map(|bank| {
                bank.layout()
                    .declarations()
                    .iter()
                    .map(|declared| declared.resource.name().to_owned())
                    .collect()
            })
            .unwrap_or_default()
        })
        .collect();
    assert!(
        !layouts.is_empty() && layouts.iter().all(|held| held == &[LIMIT.name()]),
        "a seat holds something besides the match's Limit: {layouts:?}",
    );
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

/// Probe, print-only: does a real match ever reach the Limit?
///
/// The goblin's dive is priced at `cap` (60, the whole meter). The fill is
/// 0.5/s of clock (120 s on the clock alone), plus 1.0 and 0.1x per damage
/// instance dealt and 2.0 and 0.2x per instance taken, over three stocks. So
/// the answer depends on how much damage a real match trades.
///
/// Not an assertion: what "reachable enough" means is a balance decision.
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
    // Seat 0's charge, sampled every tick, so a wipe shows as a fall that no
    // spend explains.
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
                // Above the cap should not happen. Report who, and their meter's
                // own max: "current above cap" and "never adopted" look the same.
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
                // Did the entity change? A new entity means a fresh respawn; the
                // same entity means something reset the meter in place.
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

/// Leaving Smash restores what was there; it does not delete it.
///
/// Smash does not own the portal resources: `PortalPresentationPlugin`
/// calls `init_resource` for `PortalCameraContinuitySelection` and
/// `PortalViewConeConfig`, and `sync_portal_view_cones` requires
/// `Res<PortalViewConeConfig>`. In the aggregate app, removing it on leaving
/// would break that system and destroy a developer-selected configuration.
///
/// This demo has no portal plugin, so the test plants another owner's
/// configuration before Smash runs. Otherwise it could only prove that
/// `None` came back as `None`.
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
    // The Limit rule must also go when the mode ends. The "declares nothing"
    // test asks an app that never entered the stage, so it cannot see a rule
    // that was declared and then left standing.
    assert!(
        app.world()
            .get_resource::<ambition_demo_smash::limit::SmashLimitFill>()
            .is_none(),
        "leaving Smash left its Limit rule standing after the mode ended"
    );
}

/// What Smash declares, Smash gives back, and composing it declares nothing.
///
/// `ambition_app` installs `SmashExperiencePlugin` beside Ambition, Sanic,
/// and Mary-O. If `PlayerManaRegen(0.0)` and the portal presentation were
/// inserted in `Plugin::build`, merely linking Smash would zero mana regen
/// (which `ambition_abilities` consumers need) and set Smash's cones for
/// every experience. Zero generic fill is a rule of a running ruleset. This
/// asks the composed app before any match: nothing declared.
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
    // The Limit rule: declared at plugin build, it would run for a mode
    // nobody is in.
    assert!(
        app.world()
            .get_resource::<ambition_demo_smash::limit::SmashLimitFill>()
            .is_none(),
        "composing Smash declared its Limit rule for the whole process"
    );
}

/// On the stage, the ruleset declares its answers. Without this, the test
/// above is satisfied by a ruleset that declares nothing anywhere.
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
    // Absent off-stage is meaningful only if present here.
    assert!(
        app.world()
            .get_resource::<ambition_demo_smash::limit::SmashLimitFill>()
            .is_some(),
        "on the Smash stage the ruleset does not declare its Limit rule, so the \
         meter never fills and the move priced at the cap is unreachable"
    );
}
