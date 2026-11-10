#![cfg(feature = "input")]
//! A game's rules govern its own rooms and nobody else's: its death, dormancy,
//! combat, prompt, presentation and strike-weight rules, and the capabilities
//! whose systems sleep outside the rooms that use them. All of it is read from
//! one composition of the shipped host.
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

use ambition_platformer2d::combat::dormancy_rule::DormancyRule;
use ambition_platformer2d::combat::death_rules::DeathRules;
use ambition_platformer2d::world::scoped_rules::{DeclaredRules, RulesScope};
use bevy::prelude::*;

/// Compose the shipped multi-game host, simulated by `Rollback` as the shipped
/// app is (its simulation runs in its own schedule apart from the frame's
/// presentation), and hand back its App.
///
/// Build-time only: rules are declared in plugin `build`, so no frame has to run
/// and none does. One composition serves every check below, because each reads
/// the same composed declarations and schedule graphs.
fn compose_the_shipped_host() -> App {
    use ambition_app::app::shell_host;
    use ambition_platformer2d::runtime::SimulationHostAppExt as _;
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
    app.set_simulation_host(ambition_platformer2d::runtime::SimulationHost::Rollback);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);
    shell_host::compose_ambition_shell_host(&mut app);
    app
}

/// Every check of this file, over one composed host. Each check names its own
/// fact in its failure message.
#[test]
fn each_game_governs_only_its_own_rooms_in_the_shipped_host() {
    let mut app = compose_the_shipped_host();
    let app = &mut app;
    the_host_composes_three_games_each_scoped_to_its_own_rooms(app);
    a_smash_stage_does_not_inherit_mary_os_level_replay(app);
    each_declared_mode_resolves_to_its_own_games_rules(app);
    each_game_sleeps_its_hostiles_at_its_own_distance_in_its_own_rooms(app);
    each_fighting_stage_plays_under_its_own_combat_rules_in_its_own_rooms(app);
    only_smash_rooms_name_prompts_by_button(app);
    smashs_presentation_and_limit_govern_only_smash_rooms(app);
    each_game_draws_its_own_line_between_light_and_heavy_hits(app);
    only_ambitions_doors_play_a_passage(app);
    #[cfg(feature = "portal")]
    portals_sleep_in_mary_os_rooms_and_run_in_ambitions(app);
    mounts_sleep_in_mary_os_rooms_and_run_in_ambitions(app);
    grabs_sleep_in_the_runners_rooms_and_run_in_the_fighters(app);
    encounters_sleep_in_mary_os_and_sanics_rooms_and_run_in_ambitions(app);
    a_game_declares_each_capability_its_maps_author(app);
    yarn_sleeps_in_rooms_whose_game_uses_no_dialogue(app);
    a_games_simulation_systems_sleep_outside_its_rooms(app);
}

fn declared(app: &App) -> &DeclaredRules<DeathRules> {
    app.world()
        .get_resource::<DeclaredRules<DeathRules>>()
        .expect("the shipped host's games declare death rules")
}

/// Mary-O's three-second level replay reaches a Smash stage, or it does not.
///
/// The two terms are asserted separately and BOTH have to be observed: that she
/// really declares the long hold (otherwise "smash reads the default" is true
/// because nobody declared anything), and that a `smash`-tagged room reads the
/// engine default anyway.
fn a_smash_stage_does_not_inherit_mary_os_level_replay(app: &mut App) {
    let rules = declared(&app);

    let hers = rules
        .governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(Some(ambition_demo_mary_o::MARY_O_MODE))).copied()
        .unwrap_or_default();
    assert_eq!(
        hers.interlude,
        ambition_demo_mary_o::death::death_dwell(),
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

    let stage = rules.governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(Some(ambition_demo_smash::SMASH_MODE)));
    assert_eq!(
        stage,
        None,
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
fn each_declared_mode_resolves_to_its_own_games_rules(app: &mut App) {
    let rules = declared(&app);

    for (scope, stated) in rules.iter() {
        let RulesScope::Mode(mode) = scope else {
            continue;
        };
        assert_eq!(
            rules.governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(Some(mode))),
            Some(stated),
            "`{mode}` rooms must read the rules `{mode}` declared, not another \
             game's. Declared: {:?}",
            rules.iter().collect::<Vec<_>>(),
        );
    }

    // A mode nobody in this binary claims. Not a hypothetical: it is exactly the
    // position `smash` and `versus` are in.
    assert_eq!(
        rules.governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(Some("a_mode_no_game_in_this_binary_declares"))),
        None,
        "an unclaimed room reads the engine default; a stranger's rules are \
         never the fallback",
    );
}

/// The floor under every other check, so it runs first: the checks are worth
/// exactly as much as the composition behind them, and a host that registered
/// one game — or none — would satisfy them by having nothing to contest.
///
/// The floor is not "somebody claims every room". It is that the three games
/// that MEASURABLY collided are all present with their own scopes, and that
/// Ambition's own claim is the untagged rooms rather than the binary. If
/// `MaryOExperiencePlugin` ever stops being listed in `shell_host.rs`, the death
/// rule checks go green because they stopped looking.
fn the_host_composes_three_games_each_scoped_to_its_own_rooms(app: &mut App) {
    let scopes: Vec<RulesScope> = declared(&app).iter().map(|(scope, _)| scope).collect();

    for expected in [
        RulesScope::UntaggedRooms,
        RulesScope::Mode(ambition_demo_sanic::SANIC_MODE),
        RulesScope::Mode(ambition_demo_mary_o::MARY_O_MODE),
    ] {
        assert!(
            scopes.contains(&expected),
            "{expected:?} declares no death rules in the composed host, so the \
             collision this file exists to prevent cannot be seen. Declaring: \
             {scopes:?}",
        );
    }
    assert!(
        !scopes.contains(&RulesScope::EveryRoom),
        "no game in a MULTI-GAME host may claim every room — that claim is a \
         standalone binary's, and it is the process-global this scoping \
         replaced. Declaring: {scopes:?}",
    );
}

/// Each game's hostiles sleep at that game's distance, in that game's rooms.
///
/// Three games state three wake radii. One global resource would let the game
/// composed last set the distance for all of them, and a hosted game's rule
/// would reach the host's rooms. The expected values are each game's own
/// constants, and the arena that states no rule reads none: no brain sleeps
/// there.
fn each_game_sleeps_its_hostiles_at_its_own_distance_in_its_own_rooms(app: &mut App) {
    let rules = app
        .world()
        .get_resource::<DeclaredRules<DormancyRule>>()
        .expect("the shipped host's games declare dormancy rules");
    let radius = |mode: Option<&str>| rules.governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(mode)).map(|rule| rule.hostile_wake_radius);

    for (mode, expected) in [
        (None, Some(ambition_platformer2d::combat::room_rules::lowered_room_rules(ambition_content::pack::shipped()).and_then(|rules| rules.dormancy).expect("Ambition's rooms state a wake radius").hostile_wake_radius)),
        (
            Some(ambition_demo_sanic::SANIC_MODE),
            Some(ambition_demo_sanic::pack::room_rules().dormancy.expect("his rooms state a wake radius").hostile_wake_radius),
        ),
        (
            Some(ambition_demo_mary_o::MARY_O_MODE),
            Some(ambition_demo_mary_o::pack::room_rules().dormancy.expect("her rooms state a wake radius").hostile_wake_radius),
        ),
        (Some(ambition_demo_smash::SMASH_MODE), None),
    ] {
        assert_eq!(
            radius(mode),
            expected,
            "rooms tagged {mode:?} must read their own game's wake radius. \
             Declared: {:?}",
            rules.iter().collect::<Vec<_>>(),
        );
    }
}

/// Each fighting stage plays under its own combat rules, in its own rooms.
///
/// Smash and the versus stage each state combat rules. While the rules were a
/// resource the stage published on entry and released on exit, a route that
/// skipped the lobby needed a second publisher, and a release that missed left
/// the rules in the next game. Declared by room, the host's own rooms and the
/// hosted platformers read none: their knockback plays the world's baseline.
fn each_fighting_stage_plays_under_its_own_combat_rules_in_its_own_rooms(app: &mut App) {
    use ambition_platformer2d::combat::rules::CombatRules;

    let rules = app
        .world()
        .get_resource::<DeclaredRules<CombatRules>>()
        .expect("the shipped host's fighting stages declare combat rules");

    for (mode, expected) in [
        (
            Some(ambition_demo_smash::SMASH_MODE),
            Some(ambition_demo_smash::smash_combat_rules()),
        ),
        (
            Some(ambition_app::app::versus::VERSUS_EXPERIENCE),
            Some(ambition_app::app::versus::versus_combat_rules()),
        ),
        (None, None),
        (Some(ambition_demo_sanic::SANIC_MODE), None),
        (Some(ambition_demo_mary_o::MARY_O_MODE), None),
    ] {
        assert_eq!(
            rules.governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(mode)).copied(),
            expected,
            "rooms tagged {mode:?} must read their own game's combat rules. \
             Declared: {:?}",
            rules.iter().map(|(scope, _)| scope).collect::<Vec<_>>(),
        );
    }
}

/// Smash names prompts by button in its own rooms, and only there.
///
/// Smash inserted the naming as a global resource when a battle started, and
/// nothing gave it back, so after one match every game's prompts named buttons.
fn only_smash_rooms_name_prompts_by_button(app: &mut App) {
    use ambition_platformer2d::sim_view::PromptNaming;

    let naming = app
        .world()
        .get_resource::<DeclaredRules<PromptNaming>>()
        .expect("Smash declares its prompt vocabulary");
    for (mode, expected) in [
        (Some(ambition_demo_smash::SMASH_MODE), Some(PromptNaming::ByButton)),
        (None, None),
        (Some(ambition_app::app::versus::VERSUS_EXPERIENCE), None),
        (Some(ambition_demo_sanic::SANIC_MODE), None),
        (Some(ambition_demo_mary_o::MARY_O_MODE), None),
    ] {
        assert_eq!(
            naming.governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(mode)).copied(),
            expected,
            "rooms tagged {mode:?} name prompts by the wrong vocabulary"
        );
    }
}

/// Smash's portal presentation and Limit fill govern Smash's rooms, and only
/// there.
///
/// The stage inserted all three as process-wide resources on entry and put a
/// saved copy back on exit. The portal ones overwrote the host's own
/// configuration, which a developer may have changed, and a missed exit left
/// all three in the next game.
fn smashs_presentation_and_limit_govern_only_smash_rooms(app: &mut App) {
    use ambition_demo_smash::limit::{smash_limit, SmashLimitFill};
    use ambition_platformer2d::portal_presentation::{PortalCameraTransitMode, PortalViewConeMode};

    fn table<T: Copy + std::fmt::Debug + Send + Sync + 'static>(app: &App) -> &DeclaredRules<T> {
        app.world()
            .get_resource::<DeclaredRules<T>>()
            .unwrap_or_else(|| panic!("Smash declares {}", std::any::type_name::<T>()))
    }

    let smash = Some(ambition_demo_smash::SMASH_MODE);
    assert_eq!(table::<SmashLimitFill>(&app).governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(smash)).copied(), Some(SmashLimitFill(smash_limit())));
    assert_eq!(
        table::<PortalCameraTransitMode>(&app).governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(smash)).copied(),
        Some(PortalCameraTransitMode::Pop)
    );
    assert_eq!(
        table::<PortalViewConeMode>(&app).governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(smash)).copied(),
        Some(PortalViewConeMode::Static)
    );
    // Sanic states its own cones (`Static`) for its own rooms. That is
    // Sanic's rule and not Smash's leaking, and the host's untagged rooms
    // read no cone rule at all, so the host's configuration stands there.
    for (mode, cones) in [
        (None, None),
        (Some(ambition_app::app::versus::VERSUS_EXPERIENCE), None),
        (Some(ambition_demo_sanic::SANIC_MODE), Some(PortalViewConeMode::Static)),
        (Some(ambition_demo_mary_o::MARY_O_MODE), None),
    ] {
        assert_eq!(
            (
                table::<SmashLimitFill>(&app).governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(mode)).copied(),
                table::<PortalCameraTransitMode>(&app).governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(mode)).copied(),
                table::<PortalViewConeMode>(&app).governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::live(mode)).copied(),
            ),
            (None, None, cones),
            "rooms tagged {mode:?} read another game's presentation or Smash's Limit"
        );
    }
}

/// Each game draws its own line between light and heavy hits, and a room no
/// game drew one for has no heavy hits.
///
/// ⭐ The two lines must differ, or "each game reads its own" is also true of
/// one global number: Ambition's damage is health (3 is a smash attack) and
/// Smash's is percent (3 is a jab).
fn each_game_draws_its_own_line_between_light_and_heavy_hits(app: &mut App) {
    use ambition_platformer2d::world::scoped_rules::ActiveRoom;
    use ambition_platformer2d::combat::strike_weight::StrikeWeightRules;

    let lines = app
        .world()
        .get_resource::<DeclaredRules<StrikeWeightRules>>()
        .expect("the shipped host's games declare which hits are heavy");
    let ambition = lines.governing(ActiveRoom::Untagged).copied();
    let smash = lines.governing(ActiveRoom::live(Some(ambition_demo_smash::SMASH_MODE))).copied();
    assert_eq!(ambition, Some(StrikeWeightRules::heavy_at(3)));
    assert_eq!(smash, Some(StrikeWeightRules::heavy_at(12)));
    assert_eq!(
        lines.governing(ActiveRoom::live(Some(ambition_demo_mary_o::MARY_O_MODE))).copied(),
        None,
        "Mary-O drew no line, so her rooms must not borrow another game's"
    );
}

/// Ambition's doors play a passage and the hosted games' doors cross at once.
/// The rule was one global switch set when Ambition was installed, so with
/// Ambition in the hub every game's door played Ambition's passage.
fn only_ambitions_doors_play_a_passage(app: &mut App) {
    use ambition_platformer2d::session::rooms::passage::ZonePassageRules;
    use ambition_platformer2d::world::scoped_rules::ActiveRoom;

    let rules = app
        .world()
        .get_resource::<DeclaredRules<ZonePassageRules>>()
        .expect("Ambition declares its doors' passage");
    assert_eq!(rules.governing(ActiveRoom::Untagged).copied(), Some(ZonePassageRules));
    for mode in [
        ambition_demo_mary_o::MARY_O_MODE,
        ambition_demo_sanic::SANIC_MODE,
        ambition_demo_smash::SMASH_MODE,
    ] {
        assert_eq!(
            rules.governing(ActiveRoom::live(Some(mode))).copied(),
            None,
            "`{mode}`'s doors borrowed Ambition's passage"
        );
    }
}

/// Portals sleep in Mary-O's rooms and run in Ambition's. Her game declares
/// that her rooms use no portals, and the portal transit is in the portals'
/// capability set, whose condition asks the room's game (findings F31).
#[cfg(feature = "portal")]
fn portals_sleep_in_mary_os_rooms_and_run_in_ambitions(app: &mut App) {
    use ambition_platformer2d::world::scoped_rules::ActiveRoom;
    use ambition_platformer2d::platformer::capability::{Capability, CapabilitySet, UsedCapabilities};
    use ambition_platformer2d::platformer::schedule::SimScheduleExt;
    use bevy::ecs::schedule::graph::Direction;
    use bevy::ecs::schedule::{NodeId, Schedules, SystemSet};

    let rules = app
        .world()
        .get_resource::<DeclaredRules<UsedCapabilities>>()
        .expect("the shipped host's games declare the capabilities they use");
    let uses = |mode: Option<&'static str>| {
        rules.governing(ActiveRoom::live(mode)).is_none_or(|used| used.uses(Capability::Portals))
    };
    assert!(!uses(Some(ambition_demo_mary_o::MARY_O_MODE)), "Mary-O's rooms use portals");
    assert!(uses(None), "the premise: Ambition's own rooms use portals");

    let sim = app.sim_schedule();
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(sim).expect("the sim schedule").graph();
    let capability = graph
        .system_sets
        .get_key(CapabilitySet(Capability::Portals).intern())
        .expect("the portals' capability set");
    let transit = graph
        .system_sets
        .get_key(ambition_platformer2d::portal::PortalSet::Transit.intern())
        .expect("the portal transit set");
    // The transit set, or a set it is in, is in the capability set.
    let mut stack = vec![NodeId::Set(capability)];
    let mut inside = false;
    while let Some(node) = stack.pop() {
        for child in graph.hierarchy().graph().neighbors_directed(node, Direction::Outgoing) {
            if child == NodeId::Set(transit) {
                inside = true;
            }
            if matches!(child, NodeId::Set(_)) {
                stack.push(child);
            }
        }
    }
    assert!(inside, "the portal transit is not in the portals' capability set");
}

/// Mounts sleep in Mary-O's rooms and run in Ambition's, as portals do: her
/// game declares no mounts, and the mount crate's sets are in the mounts'
/// capability set.
fn mounts_sleep_in_mary_os_rooms_and_run_in_ambitions(app: &mut App) {
    use ambition_platformer2d::world::scoped_rules::ActiveRoom;
    use ambition_platformer2d::platformer::capability::{Capability, CapabilitySet, UsedCapabilities};
    use ambition_platformer2d::platformer::schedule::SimScheduleExt;
    use bevy::ecs::schedule::graph::Direction;
    use bevy::ecs::schedule::{NodeId, Schedules, SystemSet};

    let rules = app
        .world()
        .get_resource::<DeclaredRules<UsedCapabilities>>()
        .expect("the shipped host's games declare the capabilities they use");
    let uses = |mode: Option<&'static str>| {
        rules.governing(ActiveRoom::live(mode)).is_none_or(|used| used.uses(Capability::Mounts))
    };
    assert!(!uses(Some(ambition_demo_mary_o::MARY_O_MODE)), "Mary-O's rooms use mounts");
    assert!(uses(None), "the premise: Ambition's own rooms use mounts");

    let sim = app.sim_schedule();
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(sim).expect("the sim schedule").graph();
    let capability = graph
        .system_sets
        .get_key(CapabilitySet(Capability::Mounts).intern())
        .expect("the mounts' capability set");
    let children: Vec<NodeId> =
        graph.hierarchy().graph().neighbors_directed(NodeId::Set(capability), Direction::Outgoing).collect();
    for (name, set) in [
        ("boarding", ambition_platformer2d::mount::MountsBoarded.intern()),
        ("steering", ambition_platformer2d::mount::MountsSteeredByRiders.intern()),
        ("the rider's pose", ambition_platformer2d::mount::RidersSyncedToMounts.intern()),
    ] {
        let key = graph.system_sets.get_key(set).unwrap_or_else(|| panic!("the mount set for {name}"));
        assert!(children.contains(&NodeId::Set(key)), "{name} is not in the mounts' capability set");
    }
}

/// Grabs and throws sleep in Mary-O's and Sanic's rooms and run in Smash's
/// and Ambition's: at home neither runner has an attack, and every capture
/// system of the simulation is in the captures' capability set.
fn grabs_sleep_in_the_runners_rooms_and_run_in_the_fighters(app: &mut App) {
    use ambition_platformer2d::world::scoped_rules::ActiveRoom;
    use ambition_platformer2d::platformer::capability::{Capability, CapabilitySet, UsedCapabilities};
    use bevy::ecs::schedule::graph::Direction;
    use bevy::ecs::schedule::{NodeId, ScheduleLabel as _, Schedules, SystemSet};

    let rules = app
        .world()
        .get_resource::<DeclaredRules<UsedCapabilities>>()
        .expect("the shipped host's games declare the capabilities they use");
    let uses = |mode: Option<&'static str>| {
        rules.governing(ActiveRoom::live(mode)).is_none_or(|used| used.uses(Capability::Captures))
    };
    assert!(!uses(Some(ambition_demo_mary_o::MARY_O_MODE)), "Mary-O's rooms use grabs");
    assert!(!uses(Some("sanic")), "Sanic's rooms use grabs");
    assert!(uses(Some(ambition_demo_smash::SMASH_MODE)), "the premise: Smash's rooms use grabs");
    assert!(uses(None), "the premise: Ambition's own rooms use grabs");

    use ambition_platformer2d::platformer::schedule::SimScheduleExt;
    let sim = app.sim_schedule();
    assert_ne!(sim, bevy::prelude::Update.intern(), "the premise: the simulation has its own schedule");
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(sim).expect("the sim schedule").graph();
    let capability = graph
        .system_sets
        .get_key(CapabilitySet(Capability::Captures).intern())
        .expect("the captures' capability set");
    let hierarchy = graph.hierarchy().graph();
    let inside = |start: NodeId| {
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            for parent in hierarchy.neighbors_directed(node, Direction::Incoming) {
                if parent == NodeId::Set(capability) {
                    return true;
                }
                stack.push(parent);
            }
        }
        false
    };
    let mut captures = 0;
    let mut outside = Vec::new();
    for (key, system, _) in graph.systems.iter() {
        let name = system.name().to_string();
        if !name.starts_with("ambition_combat::capture::systems::") {
            continue;
        }
        captures += 1;
        if !inside(NodeId::System(key)) {
            outside.push(name);
        }
    }
    assert!(captures >= 10, "the premise: the simulation runs the capture systems ({captures} found)");
    assert!(outside.is_empty(), "these capture systems are outside the captures' capability set: {outside:#?}");
}

/// Wave encounters sleep in Mary-O's and Sanic's rooms and run in Ambition's:
/// their games declare no encounters, and each encounter step is in the
/// encounters' capability set. The switch drain and the banners are not: other
/// games' rooms press switches and show banners.
fn encounters_sleep_in_mary_os_and_sanics_rooms_and_run_in_ambitions(app: &mut App) {
    use ambition_platformer2d::world::scoped_rules::ActiveRoom;
    use ambition_platformer2d::platformer::capability::{Capability, CapabilitySet, UsedCapabilities};
    use ambition_platformer2d::platformer::schedule::SimScheduleExt;
    use bevy::ecs::schedule::graph::Direction;
    use bevy::ecs::schedule::{NodeId, Schedules, SystemSet};

    let rules = app
        .world()
        .get_resource::<DeclaredRules<UsedCapabilities>>()
        .expect("the shipped host's games declare the capabilities they use");
    let uses = |mode: Option<&'static str>| {
        rules.governing(ActiveRoom::live(mode)).is_none_or(|used| used.uses(Capability::Encounters))
    };
    assert!(!uses(Some(ambition_demo_mary_o::MARY_O_MODE)), "Mary-O's rooms use encounters");
    assert!(!uses(Some(ambition_demo_sanic::SANIC_MODE)), "Sanic's rooms use encounters");
    assert!(uses(None), "the premise: Ambition's own rooms use encounters");

    let sim = app.sim_schedule();
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(sim).expect("the sim schedule").graph();
    let capability = graph
        .system_sets
        .get_key(CapabilitySet(Capability::Encounters).intern())
        .expect("the encounters' capability set");
    // Each system in the capability set, directly or through a set in it.
    let mut inside: Vec<String> = Vec::new();
    let mut stack = vec![NodeId::Set(capability)];
    while let Some(node) = stack.pop() {
        for child in graph.hierarchy().graph().neighbors_directed(node, Direction::Outgoing) {
            match child {
                NodeId::Set(_) => stack.push(child),
                NodeId::System(key) => {
                    let name = format!("{}", graph.systems.get(key).expect("a system node").name());
                    inside.push(name.rsplit("::").next().unwrap_or_default().to_owned());
                }
            }
        }
    }
    for step in [
        "reduce_encounter_lifecycles",
        "publish_cleared_encounters",
        "drive_wave_encounters",
        "apply_wave_encounter_effects",
        "apply_encounter_cleanup",
        "project_live_encounter_occurrences",
        "contribute_encounter_lock_walls",
        "sync_encounter_reward_chests",
        "retire_rewards_for_rearmed_encounters",
    ] {
        assert!(inside.iter().any(|name| name == step), "{step} is not in the encounters' capability set");
    }
    for awake in ["drain_switch_activations", "rebuild_encounter_switch_index", "apply_gameplay_banner_requests"] {
        assert!(!inside.iter().any(|name| name == awake), "{awake} sleeps with the encounters");
    }
}

/// What a game declares it does not use, its maps do not author. Each map
/// entity that needs a capability is compared with the declaration of the game
/// whose worlds hold it. Sanic declared no portals while his Act 3 rifts are
/// portals, and the rift slept (`three_acts`).
fn a_game_declares_each_capability_its_maps_author(app: &mut App) {
    use ambition_platformer2d::world::scoped_rules::ActiveRoom;
    use ambition_platformer2d::platformer::capability::{Capability, UsedCapabilities};

    // The map entities whose work is a capability's.
    const NEEDS: [(&str, Capability); 4] = [
        ("Portal", Capability::Portals),
        ("PortalGunSpawn", Capability::Portals),
        ("EncounterTrigger", Capability::Encounters),
        ("BossSpawn", Capability::BossEncounters),
    ];
    fn authored(game: &str) -> std::collections::BTreeSet<String> {
        let worlds = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../ambition_map_assets")
            .join(game)
            .join("worlds");
        let mut found = std::collections::BTreeSet::new();
        let mut files = 0;
        for file in std::fs::read_dir(&worlds).unwrap_or_else(|error| panic!("{}: {error}", worlds.display())) {
            let path = file.expect("a world file").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("ldtk") {
                continue;
            }
            files += 1;
            let project: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).expect("the world reads")).expect("the world parses");
            assert_ne!(project["externalLevels"], serde_json::Value::Bool(true), "{}: levels in other files", path.display());
            for level in project["levels"].as_array().into_iter().flatten() {
                for layer in level["layerInstances"].as_array().into_iter().flatten() {
                    for entity in layer["entityInstances"].as_array().into_iter().flatten() {
                        found.insert(entity["__identifier"].as_str().unwrap_or_default().to_owned());
                    }
                }
            }
        }
        assert!(files > 0, "the premise: {game} has worlds");
        found
    }

    let rules = app
        .world()
        .get_resource::<DeclaredRules<UsedCapabilities>>()
        .expect("the shipped host's games declare the capabilities they use");
    let mut checked = 0;
    for (game, mode) in [
        ("ambition_demo_mary_o", ambition_demo_mary_o::MARY_O_MODE),
        ("ambition_demo_sanic", ambition_demo_sanic::SANIC_MODE),
    ] {
        let found = authored(game);
        assert!(!found.is_empty(), "the premise: {game}'s maps hold entities");

        for (entity, capability) in NEEDS {
            if found.contains(entity) {
                checked += 1;
                assert!(
                    rules.governing(ActiveRoom::live(Some(mode))).is_none_or(|used| used.uses(capability)),
                    "{game}'s maps author {entity}, but its rooms do not use {}",
                    capability.name()
                );
            }
        }
    }
    assert!(checked > 0, "the premise: some game's maps author an entity that needs a capability");
}

/// Yarn's own set has a condition in the shipped host: its per-frame work
/// sleeps where the active room's game uses no dialogue, once its project is
/// compiled (`app::plugins`). Mary-O declares no dialogue, so in her rooms it
/// sleeps.
fn yarn_sleeps_in_rooms_whose_game_uses_no_dialogue(app: &mut App) {
    use ambition_platformer2d::world::scoped_rules::ActiveRoom;
    use ambition_platformer2d::platformer::capability::{Capability, UsedCapabilities};
    use bevy::ecs::schedule::{Schedules, SystemSet};

    let rules = app
        .world()
        .get_resource::<DeclaredRules<UsedCapabilities>>()
        .expect("the shipped host's games declare the capabilities they use");
    assert!(
        rules.governing(ActiveRoom::live(Some(ambition_demo_mary_o::MARY_O_MODE))).is_some_and(|used| !used.uses(Capability::Dialogue)),
        "the premise: Mary-O's rooms use no dialogue"
    );
    // Read before the first update, while the conditions are in the graph.
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(bevy::prelude::Update).expect("the Update schedule").graph();
    let set = graph
        .system_sets
        .get_key(bevy_yarnspinner::prelude::YarnSpinnerSystemSet.intern())
        .expect("the premise: the shipped host installs Yarn");
    let conditions = graph.system_sets.get_conditions(set).map_or(0, <[_]>::len);
    assert_eq!(conditions, 1, "Yarn's set asks {conditions} conditions, not the one that puts it to sleep");
}

/// The crate a system was made for: for a generic system, the crate of its
/// innermost type argument (`forget_attempts_on_activation<mary_o::Bricks>`
/// is Mary-O's), as `scripts/measure/system_census_table.py` counts it.
fn owning_crate(name: &str) -> &str {
    let inner = name.rfind('<').map_or(name, |at| &name[at + 1..]);
    let inner = inner.trim_start_matches(['&', '(', ' ']);
    inner.split("::").next().unwrap_or(inner).trim_end_matches(['>', ')', ',', ' '])
}

/// A game's gameplay in the simulation sleeps while another game plays: each
/// simulation system a game crate registers has a run condition, on itself or
/// on a set it is in (its rules scope, its mode, a capability it uses, a
/// running match). A duty that must reach every room, such as releasing a
/// claim the game left behind, is conditioned on that claim standing.
///
/// Measured before this held (hub census, 2026-10-21): while Ambition, Mary-O
/// or Sanic played, Smash's sudden-death round ran, and Ambition's cut-rope
/// arena answered every other game's resets and replays.
fn a_games_simulation_systems_sleep_outside_its_rooms(app: &mut App) {
    use ambition_platformer2d::platformer::schedule::SimScheduleExt;
    use bevy::ecs::schedule::graph::Direction;
    use bevy::ecs::schedule::{NodeId, ScheduleLabel as _, Schedules};

    const GAMES: &[&str] = &["ambition_content", "ambition_demo_mary_o", "ambition_demo_sanic", "ambition_demo_smash"];

    let sim = app.sim_schedule();
    assert_ne!(sim, bevy::prelude::Update.intern(), "the premise: the simulation has its own schedule");
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(sim).expect("the sim schedule").graph();
    let hierarchy = graph.hierarchy().graph();
    let gated = |start: NodeId| {
        let mut stack = vec![start];
        let mut seen = std::collections::HashSet::new();
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            for parent in hierarchy.neighbors_directed(node, Direction::Incoming) {
                if let NodeId::Set(set) = parent {
                    if graph.system_sets.get_conditions(set).is_some_and(|c| !c.is_empty()) {
                        return true;
                    }
                    stack.push(parent);
                }
            }
        }
        false
    };
    let mut counted = std::collections::BTreeMap::<&str, usize>::new();
    let mut awake = Vec::new();
    for (key, system, conditions) in graph.systems.iter() {
        let name = system.name().to_string();
        let Some(game) = GAMES.iter().find(|game| owning_crate(&name) == **game) else {
            continue;
        };
        *counted.entry(game).or_default() += 1;
        if conditions.is_empty() && !gated(NodeId::System(key)) {
            awake.push(name);
        }
    }
    for game in GAMES {
        assert!(counted.get(game).is_some_and(|n| *n > 0), "the premise: `{game}` registers simulation systems ({counted:?})");
    }
    assert!(awake.is_empty(), "these games' simulation systems run in every game's rooms: {awake:#?}");
}

#[cfg(feature = "portal")]
#[test]
fn mary_o_body_admission_ignores_an_ambition_rooms_live_portals() {
    use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::platformer::body::geometry::{BodyGeometry, MaterialBudget, publish_body_geometry_before_motion};
    use ambition_platformer2d::platformer::body::pose::BodyPoseAdmission;
    use ambition_platformer2d::platformer::capability::UsedCapabilities;
    use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, RoomInstanceRoot, SessionRoot, SessionScopeId};
    use ambition_platformer2d::world::rooms::{RoomSet, RoomSpec};
    use ambition_platformer2d::portal::{PlacedPortal, PortalChannel, PortalGunColor};
    use bevy::ecs::system::RunSystemOnce;

    fn request(poses: BodyPoseAdmission, mut bodies: Query<(Entity, &mut ae::BodyKinematics, &mut ae::SweepSample)>) {
        for (entity, mut body, mut sweep) in &mut bodies {
            let desired = ae::BodyKinematics { pos: ae::Vec2::new(2.0, 0.0), ..*body };
            let _ = poses.constrain(entity, &mut body, Some(&mut sweep), desired);
        }
    }
    let host = compose_the_shipped_host();
    let rules = host.world().resource::<DeclaredRules<UsedCapabilities>>().clone();
    let mut app = App::new();
    app.insert_resource(rules).init_resource::<BodyGeometry>();
    let mut mary = RoomSpec::new("review_mary", ae::World::new("review_mary", ae::Vec2::splat(1000.0), ae::Vec2::ZERO, Vec::new()));
    mary.metadata.mode = Some(ambition_demo_mary_o::MARY_O_MODE.into());
    let ambition = RoomSpec::new("review_ambition", ae::World::new("review_ambition", ae::Vec2::splat(1000.0), ae::Vec2::ZERO, Vec::new()));
    app.world_mut().spawn((SessionRoot(SessionScopeId(1)), RoomSet::from_parts_or_panic("review_ambition", vec![ambition, mary], Vec::new())));
    ambition_platformer2d::world::rooms::seat_sole_live_room_by_id(app.world_mut(), "review_ambition").unwrap();
    let first = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<ambition_platformer2d::platformer::lifecycle::LiveRoomInstance>(app.world()).unwrap();
    let second = first.next();
    let mary_definition = ambition_platformer2d::platformer::lifecycle::session_world_component::<RoomSet>(app.world()).unwrap().definition_by_id("review_mary").unwrap();
    app.world_mut().spawn((RoomInstanceRoot, second, mary_definition));
    let mut owners = Vec::new();
    for room in [first, second] {
        for (color, pos, normal) in [(PortalGunColor::BLUE, ae::Vec2::ZERO, -ae::Vec2::X), (PortalGunColor::ORANGE, ae::Vec2::new(500.0, 0.0), ae::Vec2::X)] {
            app.world_mut().spawn((PlacedPortal::fixed(PortalChannel::Gun(color), pos, normal, 80.0).unwrap(), InRoomInstance(room)));
        }
        let body = ae::BodyKinematics { pos: ae::Vec2::new(-30.0, 0.0), canonical_size: ae::Vec2::splat(16.0), ..Default::default() };
        owners.push(app.world_mut().spawn((body, ae::SweepSample::at_rest(body, ae::Vec2::Y), InRoomInstance(room),
            MaterialBudget(ae::frame::thread::PieceBudget { max_hops: 0, ..ae::frame::thread::PieceBudget::COMPOSED }))).id());
    }
    app.world_mut().run_system_once(publish_body_geometry_before_motion).unwrap();
    app.world_mut().run_system_once(ambition_platformer2d::portal::publish_portal_apertures).unwrap();
    app.world_mut().run_system_once(request).unwrap();
    assert_eq!(app.world().get::<ae::BodyKinematics>(owners[0]).unwrap().pos.x, -30.0);
    assert_eq!(app.world().get::<ae::BodyKinematics>(owners[1]).unwrap().pos.x, 2.0);
    app.world_mut().entity_mut(owners[0]).insert(InRoomInstance(second));
    app.world_mut().run_system_once(request).unwrap();
    assert_eq!(app.world().get::<ae::BodyKinematics>(owners[0]).unwrap().pos.x, 2.0);
}
