#![cfg(feature = "input")]
//! A game's death rules govern its own rooms and nobody else's.
//!
//! Three games in the shipped host state what a death means — Ambition
//! (`replay_level_after(0.0)`), Sanic (the same), and Mary-O (a 3.2s hold sized
//! by her death music). While `DeathRules` was a bare `Resource`, the type was
//! the key: three `insert_resource` calls in three `Plugin::build`s, and the one
//! the shell composed LAST governed the whole binary. `shell_host.rs` lists
//! Mary-O after Sanic, so every Smash match in the shipped host ran under
//! Mary-O's rules — in an arena whose own rules want [`LevelReset::Never`].
//!
//! the shape is the scope registry's, one authority over (see
//! `experience_scope_ownership.rs`): a claim that is unfalsifiable from inside
//! one plugin's `build`, checked over the composed host where all of them are
//! visible at once.

use ambition_platformer2d::combat::death_rules::{DeathRules, DeathRulesScope, DeclaredDeathRules};
use bevy::prelude::*;

/// Compose the shipped multi-game host and hand back its App.
///
/// Build-time only: rules are declared in plugin `build`, so no frame has to run
/// and none does.
fn compose_the_shipped_host() -> App {
    use ambition_app::app::shell_host;
    use bevy::asset::AssetPlugin;
    use bevy::image::ImagePlugin;
    use bevy::state::app::StatesPlugin;
    use bevy::transform::TransformPlugin;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::platformer::schedule::GameMode>();
    app.insert_resource(shell_host::AmbitionShellHosted);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);
    shell_host::compose_ambition_shell_host(&mut app);
    app
}

fn declared(app: &App) -> &DeclaredDeathRules {
    app.world()
        .get_resource::<DeclaredDeathRules>()
        .expect("the shipped host's games declare death rules")
}

/// Mary-O's three-second level replay reaches a Smash stage, or it does not.
///
/// The two terms are asserted separately and BOTH have to be observed: that she
/// really declares the long hold (otherwise "smash reads the default" is true
/// because nobody declared anything), and that a `smash`-tagged room reads the
/// engine default anyway.
#[test]
fn a_smash_stage_does_not_inherit_mary_os_level_replay() {
    let app = compose_the_shipped_host();
    let rules = declared(&app);

    let hers = rules.governing(Some(ambition_demo_mary_o::MARY_O_MODE));
    assert_eq!(
        hers.interlude,
        ambition_demo_mary_o::death::DEATH_DWELL,
        "Mary-O must still hold her own rooms for the length of her death \
         music; without that this test's other half proves nothing. Declared: \
         {:?}",
        rules.iter().collect::<Vec<_>>(),
    );
    assert_eq!(
        hers.level_reset,
        ambition_platformer2d::combat::death_rules::LevelReset::WhenNoParticipantRemains,
        "…and her level must still go back when nobody is left in play",
    );

    let stage = rules.governing(Some(ambition_demo_smash::SMASH_MODE));
    assert_eq!(
        stage,
        DeathRules::default(),
        "a Smash stage declares no death rules, so it must read the engine \
         default — hold for nothing, reset nothing. Reading {stage:?} means a \
         game that does not own the arena is governing it. Declared: {:?}",
        rules.iter().collect::<Vec<_>>(),
    );
}

/// Every game in the host declares for the rooms it authored, and only those.
///
/// The general form: whatever each game states, resolving a mode tag must return
/// that game's statement, and resolving an unclaimed tag must return the
/// default. A fourth game added tomorrow is covered by the same table.
#[test]
fn each_declared_mode_resolves_to_its_own_games_rules() {
    let app = compose_the_shipped_host();
    let rules = declared(&app);

    for (scope, stated) in rules.iter() {
        let DeathRulesScope::Mode(mode) = scope else {
            continue;
        };
        assert_eq!(
            rules.governing(Some(mode)),
            stated,
            "`{mode}` rooms must read the rules `{mode}` declared, not another \
             game's. Declared: {:?}",
            rules.iter().collect::<Vec<_>>(),
        );
    }

    // A mode nobody in this binary claims. Not a hypothetical: it is exactly the
    // position `smash` and `versus` are in.
    assert_eq!(
        rules.governing(Some("a_mode_no_game_in_this_binary_declares")),
        DeathRules::default(),
        "an unclaimed room reads the engine default; a stranger's rules are \
         never the fallback",
    );
}

/// the probe for the probe. The assertions above are worth exactly as much
/// as the composition behind them: a host that registered one game — or none —
/// would satisfy every one of them by having nothing to contest.
///
/// the floor is not "somebody claims every room". It is that the three
/// games that MEASURABLY collided are all present with their own scopes, and
/// that Ambition's own claim is the untagged rooms rather than the binary. If
/// `MaryOExperiencePlugin` ever stops being listed in `shell_host.rs`, the test
/// above goes green because it stopped looking.
#[test]
fn the_host_composes_three_games_each_scoped_to_its_own_rooms() {
    let app = compose_the_shipped_host();
    let scopes: Vec<DeathRulesScope> = declared(&app).iter().map(|(scope, _)| scope).collect();

    for expected in [
        DeathRulesScope::UntaggedRooms,
        DeathRulesScope::Mode(ambition_demo_sanic::SANIC_MODE),
        DeathRulesScope::Mode(ambition_demo_mary_o::MARY_O_MODE),
    ] {
        assert!(
            scopes.contains(&expected),
            "{expected:?} declares no death rules in the composed host, so the \
             collision this file exists to prevent cannot be seen. Declaring: \
             {scopes:?}",
        );
    }
    assert!(
        !scopes.contains(&DeathRulesScope::EveryRoom),
        "no game in a MULTI-GAME host may claim every room — that claim is a \
         standalone binary's, and it is the process-global this scoping \
         replaced. Declaring: {scopes:?}",
    );
}
