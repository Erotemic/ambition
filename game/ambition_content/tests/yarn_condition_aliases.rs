//! `ui`-gated acceptance: the two NAMED Yarn functions that survived the
//! condition-authority migration still branch correctly, through a real
//! interpreter.
//!
//! ⛔⛔ THE MIGRATION'S PROMISE WAS COMPATIBILITY, AND NOTHING TESTED IT.
//! `boss.cleared` moved to the boss domain and `quest.active` to the quest
//! domain, and `YarnStateMirrorData`'s `bosses_cleared` / `quests_active`
//! slices were deleted. The authored names `boss_cleared(id)` and
//! `quest_active(id)` were KEPT so shipped `.yarn` — `cove.yarn`,
//! `kernel.yarn` — would not have to be rewritten. That is a promise about a
//! surface authors use, and the only tests covering it called the Rust
//! evaluators directly.
//!
//! ⭐ A DIRECT CALL PROVES THE EVALUATOR AND PROVES NOTHING ABOUT THE PROMISE.
//! Between an authored `<<if boss_cleared("…")>>` and
//! `ask_boss_cleared` sit the registration NAME, the interpreter's dispatch,
//! its arity rules, its string→`String` conversion, and its branch selection.
//! `ambition_conversation`'s `yarn_harness` exists in that crate for exactly
//! this reason, in its own words: *"the interpreter's dispatch, its arity rules
//! and its value handling are exactly the parts that were believed impossible
//! and were not."*
//!
//! ⚠ WHY THIS DOES NOT IMPORT THAT HARNESS, since the duplication is visible
//! and deliberate. `yarn_harness` is `#[cfg(test)] mod` — private to
//! `ambition_conversation`'s own test build. Reaching it from here would mean
//! promoting it behind a feature that pulls `ambition_conversation/ui` into
//! this crate's graph, and because a dev-dependency's features unify with the
//! normal one, `ambition_content`'s library would then be COMPILED DIFFERENTLY
//! under test than it ships. ⇒ Paying that to share thirty lines of app setup
//! would trade a real hazard for a cosmetic one. What is reproduced here is the
//! harness's PROPERTY — drive the real interpreter through the production
//! installer seam — not its code: the vocabulary arrives through
//! `install_game_bindings`, the same function `AmbitionContentPlugin` pushes
//! into `YarnContentBindings`, so a change that breaks real installation breaks
//! these tests too.
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
/// test can ask what an authored line does when the domain is ABSENT — which is
/// a different question from "the fact is false" and has its own arm below.
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
    // ⭐ THE PRODUCTION INSTALLER, by function pointer through the seam the
    // content plugin uses. Not `register_functions` directly: the product
    // installs the whole vocabulary at once, and a test that installed half of
    // it could pass while the real install order broke.
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

/// The id every boss arm here uses — SYNTHETIC, and deliberately not a name any
/// shipped `.yarn` file types.
///
/// ⛔⛔ THIS FIXTURE USED TO SAY `"mockingbird"`, THE SHIPPED SPELLING, AND THAT
/// MADE IT READ AS EVIDENCE THE SHIPPED LINE WORKS. It is not, and the
/// difference is the whole of question 57: `boss_encounter/src/systems.rs:259`
/// writes the save under the PLACEMENT id (`BossSpawn-4308`) while
/// `kernel.yarn` asks under the BEHAVIOUR id, so the shipped branch has never
/// been able to open. This test seeds the save BY HAND under whatever id it
/// asks about, so it agrees with itself no matter which key production uses —
/// it can no more detect that mismatch than a test can detect a bug in the
/// fixture it wrote.
///
/// ⭐ WHAT IT DOES PROVE, exactly: an authored `boss_cleared("x")` reaches
/// `ask_boss_cleared` through the registration name, the interpreter's dispatch,
/// its arity rules and its string→`String` conversion, and selects the branch
/// the save's state implies. The ROUND TRIP, not the shipped content.
const SYNTHETIC_BOSS: &str = "yarn_alias_test_boss";

/// A save carrying one boss in the given state, under [`SYNTHETIC_BOSS`].
fn save_with_boss(state: PersistedEncounterState) -> AmbitionGameSave {
    let mut save = AmbitionGameSave::default();
    save.data_mut().set_boss(SYNTHETIC_BOSS.to_string(), state);
    save
}

/// ⭐ THE FIXTURE ID MUST STAY UNSPEAKABLE IN SHIPPED CONTENT, so nobody can
/// cite these tests as coverage of a line an author wrote. If someone ever names
/// a real boss this, the tests above quietly become a claim about the game.
#[test]
fn the_boss_fixture_id_is_not_a_name_any_shipped_dialogue_uses() {
    let spoken: Vec<&str> = ambition_content::dialogue::yarn::YARN_SOURCES
        .iter()
        .filter(|(_, text)| text.contains(SYNTHETIC_BOSS))
        .map(|(name, _)| *name)
        .collect();
    assert!(
        spoken.is_empty(),
        "{SYNTHETIC_BOSS:?} is now authored in {spoken:?} — these tests seed the \
         save by hand and would read as proof that authored line works. Rename \
         the fixture, not the content."
    );
    assert!(
        ambition_content::dialogue::yarn::YARN_SOURCES
            .iter()
            .any(|(_, text)| text.contains("boss_cleared(")),
        "no shipped .yarn calls `boss_cleared` at all, so the assertion above is \
         trivially true and this file guards nothing"
    );
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

/// ⭐⭐ AN AUTHORED `boss_cleared(...)` LINE BRANCHES ON THE LIVE SAVE, through
/// the interpreter — the compatibility promise, asked the way content asks it.
///
/// ⛔ THE CLOSED ARM IS ASSERTED FROM A BOSS THE SAVE KNOWS ABOUT, and
/// `Failed` rather than `Untouched` on purpose: "never fought" and "fought and
/// lost" are different facts and both must leave the door shut. A test that
/// used an unrecorded boss for its false arm would pass equally if the function
/// were wired to nothing at all.
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

/// ⭐⭐ `quest_active(...)` IS TRUE FOR `InProgress` AND FOR NOTHING ELSE.
///
/// ⛔ `Completed` IS THE ARM THAT MATTERS and it is why this test names three
/// states rather than two. *"Is this quest under way"* and *"did you finish
/// it"* are different questions; a `quest_active` that answered "yes" for a
/// finished quest would leave every "are you still looking for it?" line
/// running forever, and a two-state test (`NotStarted` vs `InProgress`) would
/// not notice.
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

/// ⛔⛔ A COMPOSITION THAT NEVER PUBLISHED THE QUESTION LEAVES THE BRANCH SHUT,
/// and this is the rule the alias functions document rather than an accident.
///
/// Yarn's `<<if>>` needs a bool, so *unanswerable* has to collapse one way. It
/// collapses to FALSE: the other direction would open a door in exactly the
/// world that understands the question least. ⚠ This arm is what makes the two
/// above meaningful — without it, a `boss_cleared` hard-wired to `false` would
/// satisfy every closed assertion in this file.
#[test]
fn an_alias_whose_domain_is_absent_leaves_the_authored_branch_shut() {
    let mut app = app_running(BOSS_SOURCE, |app| {
        // The save is present and says the boss WAS beaten; only the domain
        // that publishes `boss.cleared` is missing.
        app.insert_resource(save_with_boss(PersistedEncounterState::Cleared));
    });
    assert_eq!(
        play(&mut app, "boss_gate"),
        vec!["STANDING".to_string()],
        "with no boss domain composed nothing can answer, and a question nobody \
         can answer must not open a door"
    );
}

/// ⭐⭐ THE SHOP MENU'S TEN `can_afford` LINES BRANCH OFF THE LIVE WALLET.
///
/// This alias had the most authored callers of any fact question in the game
/// and no interpreter-level test at all, because the mirror migration was ruled
/// finished by enumerating the mirror's struct FIELDS — `wallet_balance` is a
/// number, so the field was exempt — while the fork lived in the FUNCTIONS
/// bound over that field.
///
/// ⛔ THE BOUNDARY IS ASSERTED, not just rich-and-poor. A player holding
/// exactly the asking price must be able to buy: the shop's own `buy` spends
/// `balance -= price` and refuses only when short, so `>` here would grey out a
/// line the purchase itself would allow — visible only to a player holding
/// exactly 25g, which is nobody's test playthrough.
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

/// ⛔ AND THE WALLET DOMAIN OBEYS THE SAME COLLAPSE RULE AS THE OTHER TWO: a
/// composition that never published the question leaves the purchase shut, even
/// with a full purse. Without this arm a `can_afford` wired to nothing would
/// satisfy the `TOO DEAR` assertion above.
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
