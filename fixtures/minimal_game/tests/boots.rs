//! **The smallest game stands up — on both faces, from one module.**
//!
//! Consumer-matrix row 2. Outlander already proves external composition, so
//! what these add is the part one consumer cannot: that the API works for a
//! game which asked for almost nothing.
//!
//! Each test below corresponds to a slice-B exit criterion, and none of them
//! asserts "it compiled" — the campaign's rule is that a proof is a test that
//! runs.

use ambition::app::prelude::*;
use minimal_game::MinimalModule;

/// One module, so a passing pair cannot secretly be two different games.
fn the_one_module() -> MinimalModule {
    MinimalModule
}

/// **It boots headless.**
#[test]
fn the_minimal_game_boots_headless() {
    let app = PlatformerApp::headless()
        .mount(the_one_module())
        .try_build()
        .expect("the smallest game composes headless");
    assert!(
        app.get_schedule(ambition::bevy::prelude::FixedUpdate)
            .is_some(),
        "a composed app runs a fixed-step simulation; its absence means no engine \
         was installed at all, which a did-it-panic test cannot distinguish from \
         success"
    );
}

/// **The SAME module reaches the windowed face.**
///
/// This is the slice-B leak, as a test. Before slice B the visible face
/// installed `PlatformerAssetsPlugin`, which panics without a
/// `CharacterCatalog`, and a minimal module had no way to supply one — so a
/// game that booted headless could not boot windowed, while
/// `api-prototype.md` §2b claimed the two faces differed only in policy. The
/// 2026-07-30 blind agent hit exactly this and recorded that the document
/// "actively told me the opposite would be true".
#[test]
fn the_minimal_game_boots_windowed() {
    let app = PlatformerApp::windowed(minimal_game::MINIMAL_WINDOW_TITLE)
        .without_gpu()
        .mount(the_one_module())
        .try_build()
        .expect("the smallest game composes windowed — this is slice B's leak");
    assert!(
        app.get_schedule(ambition::bevy::prelude::FixedUpdate)
            .is_some(),
        "the windowed face runs the same simulation; only the face differs"
    );
}

/// **A composition that prepares art and declares no cast is REFUSED.**
///
/// The counterpart to the test above, and the reason slice B did not simply
/// make the engine invent an empty catalog. `PlatformerAssetsPlugin`'s refusal
/// is deliberate — *"silently substituting an empty catalog is how a game ships
/// with its bosses drawn as the fallback body and nobody notices"* — so the fix
/// had to make the true answer SAYABLE, not make the demand disappear.
///
/// Saying nothing must therefore still fail, and fail where the consumer can
/// read it: a structured `CompositionError` naming both fixes, rather than a
/// panic from inside a plugin three installs later.
#[test]
fn preparing_art_with_no_declared_cast_is_refused_and_names_both_fixes() {
    struct Silent;

    impl GameModule for Silent {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("silent")
        }

        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience(minimal_game::MINIMAL_EXPERIENCE)
                .launcher_route(minimal_game::MINIMAL_LAUNCHER_ROUTE)
                .gameplay_route(minimal_game::MINIMAL_GAMEPLAY_ROUTE)
                .playable(
                    "probe",
                    "probe",
                    minimal_game::minimal_experience::MINIMAL_CHARACTER_ID,
                    minimal_game::minimal_experience::MINIMAL_ROOM_ID,
                    vec![minimal_game::minimal_experience::minimal_room()],
                )
                .no_audio();
            // and says nothing at all about characters
        }
    }

    let error = PlatformerApp::windowed("silent")
        .without_gpu()
        .mount(Silent)
        .try_build()
        .expect_err("a composition that prepares art with no declared cast must refuse");
    let reported = error.to_string();
    assert!(
        reported.contains("no_characters"),
        "the refusal must name the way to say 'this game has no cast'; got {reported:?}"
    );
    assert!(
        reported.contains("characters("),
        "the refusal must also name the way to declare one; got {reported:?}"
    );
}

/// **Declaring no cast AND a starting character is a contradiction, and it is
/// caught.**
///
/// This test used to assert that `no_characters()` composes. It did — until
/// build-time validation of `starting_character` landed, and then it failed,
/// because the module was declaring an empty roster and naming a protagonist in
/// the same breath. The test was wrong and the new check was right.
///
/// ⚠ It also surfaced a real limitation, recorded rather than papered over: a
/// genuinely CASTLESS game — a menu-only app — cannot be expressed today.
/// `playable()` requires a starting character, and without `playable()` no
/// gameplay route is registered, so rule 7 refuses the composition. `no_characters()`
/// is therefore only usable by a module that is not itself playable, and no
/// such module can currently stand up alone. That is a genuine gap in the
/// consumer matrix's "noncombat actor" direction and it belongs to a later
/// slice, not to a quiet `expect()` here.
#[test]
fn declaring_no_cast_and_a_starting_character_is_refused() {
    struct Contradiction;

    impl GameModule for Contradiction {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("contradiction")
        }

        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience(minimal_game::MINIMAL_EXPERIENCE)
                .launcher_route(minimal_game::MINIMAL_LAUNCHER_ROUTE)
                .gameplay_route(minimal_game::MINIMAL_GAMEPLAY_ROUTE)
                .no_characters()
                .no_audio()
                .playable(
                    "contradiction",
                    "declares an empty roster and then names a protagonist",
                    minimal_game::minimal_experience::MINIMAL_CHARACTER_ID,
                    minimal_game::minimal_experience::MINIMAL_ROOM_ID,
                    vec![minimal_game::minimal_experience::minimal_room()],
                );
        }
    }

    let error = PlatformerApp::headless()
        .with_game_assets()
        .mount(Contradiction)
        .try_build()
        .expect_err("an empty roster cannot supply a starting character");
    let reported = error.to_string();
    assert!(
        reported.contains("roster is empty"),
        "the refusal must say the roster is EMPTY rather than merely that the \
         character is absent — those are different mistakes; got {reported:?}"
    );
}

/// **A game that will never start says WHY, instead of hanging.**
///
/// This is slice B's failure, turned into the check that should have caught it.
///
/// A module that declares no audio is refused by preparation validation. Before
/// slice C the refusal reached `error!` and stopped there: a headless consumer
/// with no log subscriber saw `Activating` forever, and a poll loop spun 600
/// ticks on a decision the engine had reached on tick 3. That cost this
/// campaign a whole slice and a falsely-"proven" consumer-matrix row.
///
/// `ShellCommandRejection::LoadFailed`'s own doc comment had already recorded
/// the shape — without its carried failures "the route appeared to stall
/// forever with no diagnosable cause" — so the reasons existed. They were just
/// never anywhere a consumer could read them. A log line is an operator
/// affordance; this is the API one.
#[test]
fn a_game_that_will_never_start_reports_why() {
    struct Voiceless;

    impl GameModule for Voiceless {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("voiceless")
        }

        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience(minimal_game::MINIMAL_EXPERIENCE)
                .launcher_route(minimal_game::MINIMAL_LAUNCHER_ROUTE)
                .gameplay_route(minimal_game::MINIMAL_GAMEPLAY_ROUTE)
                .characters(minimal_game::minimal_experience::MINIMAL_ROSTER_RON)
                .playable(
                    "voiceless",
                    "declares no audio, so preparation will refuse it",
                    minimal_game::minimal_experience::MINIMAL_CHARACTER_ID,
                    minimal_game::minimal_experience::MINIMAL_ROOM_ID,
                    vec![minimal_game::minimal_experience::minimal_room()],
                );
            // and never says `no_audio()`
        }
    }

    let mut app = PlatformerApp::headless().mount(Voiceless).build();

    let mut status = host_status(&app);
    let mut ticks = 0;
    // A poll loop that can STOP. That is the affordance under test — the
    // previous version of this loop had no exit but exhaustion.
    for _ in 0..600 {
        app.update();
        ticks += 1;
        status = host_status(&app);
        if status.is_running() || status.is_refused() {
            break;
        }
    }

    assert!(
        status.is_refused(),
        "a host that can never start must SAY so; after {ticks} ticks it reported \
         {status:?}"
    );
    assert!(
        ticks < 600,
        "the refusal must arrive promptly — spinning the full budget is the \
         silent hang wearing a new type"
    );
    let reasons = status.refusal().join(" | ");
    assert!(
        reasons.contains("audio"),
        "the refusal must name the missing thing, not merely report failure; \
         got {reasons:?}"
    );
}

/// **A starting character nobody authored is refused at BUILD, not at tick 600.**
///
/// Blind run 2 (2026-07-30) found this and named it exactly: "the exact
/// silent-failure shape slice A closed for routes, left open for characters."
/// It declared a `starting_character` no roster contained, and `try_build`
/// SUCCEEDED — 120 ticks ran, the process exited 0, and the host had never
/// started. It reported that as a false positive against itself, which is the
/// only reason it was caught.
///
/// Slice C made that hang *legible* (`HostStatus::Refused`). This makes it
/// unreachable: the draft holds both the roster and the id, so it can answer at
/// build time with the same quality of message the route check gives.
#[test]
fn a_starting_character_no_roster_contains_is_refused_at_build() {
    struct Ghost;

    impl GameModule for Ghost {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("ghost")
        }

        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience(minimal_game::MINIMAL_EXPERIENCE)
                .launcher_route(minimal_game::MINIMAL_LAUNCHER_ROUTE)
                .gameplay_route(minimal_game::MINIMAL_GAMEPLAY_ROUTE)
                .characters(minimal_game::minimal_experience::MINIMAL_ROSTER_RON)
                .no_audio()
                .playable(
                    "ghost",
                    "starts as somebody who does not exist",
                    "nobody_authored_this_character",
                    minimal_game::minimal_experience::MINIMAL_ROOM_ID,
                    vec![minimal_game::minimal_experience::minimal_room()],
                );
        }
    }

    let error = PlatformerApp::headless()
        .mount(Ghost)
        .try_build()
        .expect_err("a starting character no roster contains must be refused at build");
    let reported = error.to_string();
    assert!(
        reported.contains("nobody_authored_this_character"),
        "the refusal must name the character that does not exist; got {reported:?}"
    );
    assert!(
        reported.contains(minimal_game::minimal_experience::MINIMAL_CHARACTER_ID),
        "the refusal must list the characters that DO exist, or it is a puzzle \
         rather than a fix; got {reported:?}"
    );
}

/// **The prelude carries the types its own signatures demand.**
///
/// `ModuleDraft::playable` takes `Vec<RoomSpec>`; `ModuleDraft::room` takes
/// `RoomMetadata`. Blind run 2 had to open `crates/ambition_world/src/lib.rs`
/// to find where they live — the ONE engine source file it opened, which under
/// §2c is the field that names the next leak.
///
/// This test imports NOTHING but `ambition::app::prelude` and uses both, so the
/// omission cannot come back quietly.
#[test]
fn the_app_prelude_carries_the_room_types_its_signatures_require() {
    // No `use ambition::world::...` anywhere in this function, deliberately.
    let room: RoomSpec = minimal_game::minimal_experience::minimal_room();
    let _: RoomMetadata = room.metadata.clone();
    let _ = PlatformerApp::headless();
}

/// **The game reports that it started — without counting raw Bevy entities.**
///
/// The affordance blind run 1 went looking for and did not find; it fell back
/// to `app.world().entities().len()`, which is raw Bevy and says nothing about
/// routes. Blind run 2 used `host_status` and caught its OWN false positive
/// with it: a host that exited 0 having never started.
#[test]
fn the_minimal_game_reports_that_it_started() {
    let mut app = PlatformerApp::headless()
        .mount(the_one_module())
        .build();

    assert_eq!(
        host_status(&app),
        HostStatus::Initializing,
        "before any update the router has not initialized"
    );

    let mut status = host_status(&app);
    for _ in 0..600 {
        app.update();
        status = host_status(&app);
        if status.is_running() || status.is_refused() {
            break;
        }
    }

    assert!(status.is_running(), "never reached a running host; got {status:?}");
    assert_eq!(
        status.route(),
        Some(minimal_game::MINIMAL_GAMEPLAY_ROUTE),
        "the host activated a route this game did not declare"
    );
}

/// **`is_running` is not satisfied by a route with nothing behind it.**
///
/// "A route is active" and "a session was prepared for it" are different facts,
/// and the gap between them IS the empty host — an earlier draft of Outlander's
/// headless binary "ran" 120 ticks of exactly that. A status type collapsing
/// them would agree with the bug it exists to expose.
#[test]
fn a_route_with_no_prepared_session_does_not_count_as_running() {
    let live = HostStatus::Running {
        route: "r".into(),
        experience: "e".into(),
        prepared: true,
    };
    let hollow = HostStatus::Running {
        route: "r".into(),
        experience: "e".into(),
        prepared: false,
    };
    assert!(live.is_running());
    assert!(
        !hollow.is_running(),
        "a route with no prepared session behind it is the empty host"
    );
    // A diagnosis needs to know WHICH route is hollow.
    assert_eq!(hollow.route(), Some("r"));
}
