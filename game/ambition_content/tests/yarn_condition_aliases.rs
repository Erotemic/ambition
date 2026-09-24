//! `ui`-gated acceptance: the two named Yarn functions kept after the
//! condition-authority migration still branch correctly, through a real
//! interpreter.
//!
//! `boss.cleared` lives in the boss domain and `quest.active` in the quest
//! domain. The authored names `boss_cleared(id)` and `quest_active(id)` stay,
//! so shipped `.yarn` (`cove.yarn`, `kernel.yarn`) need not change. That is a
//! promise about a surface authors use.
//!
//! A direct call proves the evaluator, not the promise. Between an authored
//! `<<if boss_cleared("…")>>` and `ask_boss_cleared` sit the registration
//! name, the interpreter's dispatch, its arity rules, its string→`String`
//! conversion and its branch selection. `ambition_conversation`'s
//! `yarn_harness` exists for the same reason.
//!
//! This does not import that harness. `yarn_harness` is a `#[cfg(test)] mod`,
//! private to `ambition_conversation`'s test build. Exposing it behind a
//! feature would pull `ambition_conversation/ui` into this crate's graph, and
//! because dev-dependency features unify with normal ones, `ambition_content`
//! would compile differently under test than it ships. So this file copies the
//! harness's property, not its code: it drives the real interpreter through
//! `install_game_bindings`, the function `AmbitionContentPlugin` pushes into
//! `YarnContentBindings`. A change that breaks real installation breaks these
//! tests.
#![cfg(feature = "ui")]

use ambition_boss_encounter::conditions::BossConditionsPlugin;
use ambition_platformer2d_actor_monolith::items::wallet_conditions::WalletConditionsPlugin;
use ambition_characters::actor::BodyWallet;
use ambition_content::quests::conditions::QuestConditionsPlugin;
use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::{PersistedEncounterState, PersistedQuestState};
use bevy::prelude::*;
use bevy_yarnspinner::events::PresentLine;
use bevy_yarnspinner::prelude::*;

#[derive(Resource, Default)]
struct PresentedLines(Vec<String>);

fn record_line(event: On<PresentLine>, mut lines: ResMut<PresentedLines>) {
    lines.0.push(event.line.text.clone());
}

/// An app running `source` with Ambition's real Yarn vocabulary installed.
///
/// `publish` decides which condition providers this composition carries, so a
/// test can ask what an authored line does when the domain is absent, which
/// differs from "the fact is false".
fn app_running(source: &str, publish: impl FnOnce(&mut App)) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..default()
    });
    app.add_plugins(YarnSpinnerPlugin::with_yarn_source(
        YarnFileSource::InMemory(YarnFile::new("condition_alias_test.yarn", source)),
    ));
    app.init_resource::<PresentedLines>();
    app.init_resource::<ambition_dialog::YarnStateMirror>();
    app.init_resource::<ambition_dialog::YarnContentBindings>();
    // The production installer, through the seam the content plugin uses. Not
    // `register_functions` directly: the product installs the whole vocabulary
    // at once, and a test that installed half could pass while the real install
    // order broke.
    app.world_mut()
        .resource_mut::<ambition_dialog::YarnContentBindings>()
        .installers
        .push(ambition_content::yarn_vocabulary::install_game_bindings);
    app.add_observer(record_line);
    publish(&mut app);
    app
}

/// Spawn the runner with the vocabulary installed, and run `node` to its end.
fn play(app: &mut App, node: &str) -> Vec<String> {
    while !app.world().contains_resource::<YarnProject>() {
        app.update();
    }
    let mirror = app
        .world()
        .resource::<ambition_dialog::YarnStateMirror>()
        .clone();
    let installers = app
        .world()
        .resource::<ambition_dialog::YarnContentBindings>()
        .installers
        .clone();
    let mut state: bevy::ecs::system::SystemState<(Commands, Res<YarnProject>)> =
        bevy::ecs::system::SystemState::new(app.world_mut());
    let (mut commands, project) = state.get_mut(app.world_mut()).expect("yarn params");
    let mut runner = project.create_dialogue_runner(&mut commands);
    for install in &installers {
        install(&mut commands, &mut runner, &mirror);
    }
    state.apply(app.world_mut());
    let entity = app.world_mut().spawn(runner).id();

    app.world_mut()
        .get_mut::<DialogueRunner>(entity)
        .expect("runner")
        .start_node(node);
    app.update();
    // Drive to the end of the node; the sources below are a handful of lines.
    for _ in 0..12 {
        let mut runner = app
            .world_mut()
            .get_mut::<DialogueRunner>(entity)
            .expect("runner");
        if !runner.is_running() {
            break;
        }
        runner.continue_in_next_update();
        app.update();
    }
    app.world().resource::<PresentedLines>().0.clone()
}

/// It lives in an ungated module so its guard survives this file's
/// `#![cfg(feature = "ui")]`; see `dialogue_lint::SYNTHETIC_BOSS`.
use crate::dialogue_lint::SYNTHETIC_BOSS;

/// A save carrying one boss in the given state, under [`SYNTHETIC_BOSS`].
fn save_with_boss(state: PersistedEncounterState) -> AmbitionGameSave {
    let mut save = AmbitionGameSave::default();
    save.data_mut().set_boss(SYNTHETIC_BOSS.to_string(), state);
    save
}


/// A save carrying one quest in the given state.
fn save_with_quest(state: PersistedQuestState) -> AmbitionGameSave {
    let mut save = AmbitionGameSave::default();
    save.data_mut()
        .set_quest("pirate_treasure".to_string(), state, 0);
    save
}

const BOSS_SOURCE: &str = "title: boss_gate\n---\n\
    <<if boss_cleared(\"yarn_alias_test_boss\")>>\n\
    BEATEN\n\
    <<else>>\n\
    STANDING\n\
    <<endif>>\n\
    ===\n";

const SHOP_SOURCE: &str = "title: shop_gate\n---\n\
    <<if can_afford(25)>>\n\
    AFFORDABLE\n\
    <<else>>\n\
    TOO DEAR\n\
    <<endif>>\n\
    ===\n";

const QUEST_SOURCE: &str = "title: quest_gate\n---\n\
    <<if quest_active(\"pirate_treasure\")>>\n\
    UNDERWAY\n\
    <<else>>\n\
    NOT UNDERWAY\n\
    <<endif>>\n\
    ===\n";

/// An authored `boss_cleared(...)` line branches on the live save, through
/// the interpreter.
///
/// The closed arm uses a boss the save knows, as `Failed`, not `Untouched`:
/// "fought and lost" must also leave the door shut. A false arm from an
/// unrecorded boss would pass even if the function were wired to nothing.
#[test]
fn an_authored_boss_cleared_line_branches_on_the_live_save() {
    let mut app = app_running(BOSS_SOURCE, |app| {
        app.add_plugins(BossConditionsPlugin);
        app.insert_resource(save_with_boss(PersistedEncounterState::Failed));
    });
    assert_eq!(
        play(&mut app, "boss_gate"),
        vec!["STANDING".to_string()],
        "a boss the player lost to is not cleared, and the authored gate stays shut"
    );

    let mut app = app_running(BOSS_SOURCE, |app| {
        app.add_plugins(BossConditionsPlugin);
        app.insert_resource(save_with_boss(PersistedEncounterState::Cleared));
    });
    assert_eq!(
        play(&mut app, "boss_gate"),
        vec!["BEATEN".to_string()],
        "the same authored line opens once the save records the defeat"
    );
}

/// `quest_active(...)` is true for `InProgress` and nothing else.
///
/// `Completed` is the arm that matters, so the test names three states. A
/// `quest_active` that said yes for a finished quest would keep every "are you
/// still looking for it?" line running, and a two-state test would not
/// notice.
#[test]
fn an_authored_quest_active_line_is_true_only_while_the_quest_is_in_progress() {
    for (state, expected) in [
        (PersistedQuestState::NotStarted, "NOT UNDERWAY"),
        (PersistedQuestState::InProgress, "UNDERWAY"),
        (PersistedQuestState::Completed, "NOT UNDERWAY"),
    ] {
        let mut app = app_running(QUEST_SOURCE, |app| {
            app.add_plugins(QuestConditionsPlugin);
            app.insert_resource(save_with_quest(state));
        });
        assert_eq!(
            play(&mut app, "quest_gate"),
            vec![expected.to_string()],
            "an authored quest_active gate in state {state:?}"
        );
    }
}

/// A composition that never published the question leaves the branch shut.
///
/// Yarn's `<<if>>` needs a bool, so unanswerable collapses to false: the other
/// direction would open a door in the world that understands the question
/// least. Without this arm, a `boss_cleared` hard-wired to `false` would pass
/// every closed assertion in this file.
#[test]
fn an_alias_whose_domain_is_absent_leaves_the_authored_branch_shut() {
    let mut app = app_running(BOSS_SOURCE, |app| {
        // The save says the boss was beaten; only the domain that publishes
        // `boss.cleared` is missing.
        app.insert_resource(save_with_boss(PersistedEncounterState::Cleared));
    });
    assert_eq!(
        play(&mut app, "boss_gate"),
        vec!["STANDING".to_string()],
        "with no boss domain composed nothing can answer, and a question nobody \
         can answer must not open a door"
    );
}

/// The shop menu's `can_afford` lines branch off the live wallet.
///
/// The boundary is asserted. A player holding exactly the price must be able
/// to buy: `buy` spends `balance -= price` and refuses only when short, so `>`
/// here would grey out a purchase that `buy` allows.
#[test]
fn an_authored_can_afford_line_branches_on_the_live_wallet() {
    for (balance, expected) in [(24, "TOO DEAR"), (25, "AFFORDABLE"), (99, "AFFORDABLE")] {
        let mut app = app_running(SHOP_SOURCE, move |app| {
            app.add_plugins(WalletConditionsPlugin);
            app.world_mut().spawn((
                ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
                BodyWallet { balance },
            ));
        });
        assert_eq!(
            play(&mut app, "shop_gate"),
            vec![expected.to_string()],
            "an authored `can_afford(25)` line with {balance}g in the purse"
        );
    }
}

/// The wallet domain follows the same collapse rule: a composition that never
/// published the question leaves the purchase shut, even with a full purse.
/// Without this arm a `can_afford` wired to nothing would pass the `TOO DEAR`
/// assertion above.
#[test]
fn a_shop_line_is_shut_when_no_wallet_domain_is_composed() {
    let mut app = app_running(SHOP_SOURCE, |app| {
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
            BodyWallet { balance: 9_000 },
        ));
    });
    assert_eq!(
        play(&mut app, "shop_gate"),
        vec!["TOO DEAR".to_string()],
        "with no wallet domain composed nothing can answer, and an unanswerable \
         question must not open a purchase"
    );
}
