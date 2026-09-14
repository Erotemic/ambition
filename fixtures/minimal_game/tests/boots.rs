//! Runtime checks for the smallest game on both supported faces from one module.
//! The fixture proves that a minimally configured game can boot through the public API.

use ambition_platformer2d::app::prelude::*;
use minimal_game::MinimalModule;

/// One module, so a passing pair cannot secretly be two different games.
fn the_one_module() -> MinimalModule {
    MinimalModule
}

/// It boots headless.
#[test]
fn the_minimal_game_boots_headless() {
    let app = PlatformerApp::headless()
        .mount(the_one_module())
        .try_build()
        .expect("the smallest game composes headless");
    assert!(
        app.get_schedule(ambition_platformer2d::bevy::prelude::FixedUpdate)
            .is_some(),
        "a composed app runs a fixed-step simulation; its absence means no engine \
         was installed at all, which a did-it-panic test cannot distinguish from \
         success"
    );
}

/// The same minimal module must compose on the windowed face.
#[test]
fn the_minimal_game_boots_windowed() {
    let app = PlatformerApp::windowed(minimal_game::MINIMAL_WINDOW_TITLE)
        .without_gpu()
        .mount(the_one_module())
        .try_build()
        .expect("the smallest game composes windowed");
    assert!(
        app.get_schedule(ambition_platformer2d::bevy::prelude::FixedUpdate)
            .is_some(),
        "the windowed face runs the same simulation; only the face differs"
    );
}

/// Art preparation with no declared cast must return a structured refusal that
/// names both supported fixes instead of inventing an empty catalog.
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

/// Declaring an empty cast while naming a starting character is contradictory
/// and must be refused explicitly.
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

/// A game that cannot start exposes the preparation rejection programmatically.
/// Headless and no-log consumers must not depend on log output to diagnose a
/// refused activation.
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
    // A refused host must let polling stop before the timeout.
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

/// A starting character absent from the authored roster is refused during
/// composition rather than surfacing later as a host that never starts.
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

/// The app prelude exports the room types required by its own public signatures.
#[test]
fn the_app_prelude_carries_the_room_types_its_signatures_require() {
    // No `use ambition_platformer2d::world::...` anywhere in this function, deliberately.
    let room: RoomSpec = minimal_game::minimal_experience::minimal_room();
    let _: RoomMetadata = room.metadata.clone();
    let _ = PlatformerApp::headless();
}

/// `host_status` reports whether the declared game route actually started.
#[test]
fn the_minimal_game_reports_that_it_started() {
    let mut app = PlatformerApp::headless().mount(the_one_module()).build();

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

    assert!(
        status.is_running(),
        "never reached a running host; got {status:?}"
    );
    assert_eq!(
        status.route(),
        Some(minimal_game::MINIMAL_GAMEPLAY_ROUTE),
        "the host activated a route this game did not declare"
    );
}

/// `is_running` is not satisfied by a route with nothing behind it.
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
    // The status must retain the hollow route for diagnosis.
    assert_eq!(hollow.route(), Some("r"));
}

/// A noncombat authored character must not receive combat state. Check the live
/// body's components rather than its generic movement capability mask.
#[test]
fn a_noncombat_character_gets_no_combat_state() {
    use ambition_platformer2d::bevy::prelude::{Entity, With};

    let mut app = PlatformerApp::headless().mount(the_one_module()).build();
    for _ in 0..600 {
        app.update();
        if host_status(&app).is_running() {
            break;
        }
    }
    assert!(
        host_status(&app).is_running(),
        "the body has to exist before its components mean anything"
    );

    let world = app.world_mut();
    let mut players =
        world.query_filtered::<Entity, With<ambition_platformer2d::actor::PrimaryPlayer>>();
    let bodies: Vec<Entity> = players.iter(world).collect();
    assert_eq!(
        bodies.len(),
        1,
        "expected exactly one primary player to inspect"
    );

    let components: Vec<String> = world
        .inspect_entity(bodies[0])
        .expect("the player entity exists")
        .map(|info| info.name().to_string())
        .collect();

    // Non-vacuity first: an empty component list would satisfy every assertion
    // below while proving nothing.
    assert!(
        components.len() > 20,
        "expected a fully built body; got {} components, so this is inspecting \
         something that was never assembled",
        components.len()
    );

    let combat: Vec<&String> = components
        .iter()
        .filter(|name| {
            let n = name.to_lowercase();
            n.contains("melee")
                || n.contains("combat")
                || n.contains("hitbox")
                || n.contains("health")
                || n.contains("moveset")
        })
        .collect();
    assert!(
        combat.is_empty(),
        "a character that authored no combat verbs was given combat state \
         anyway, so `actor` is combat-shaped: {combat:?}"
    );
}

/// Distinct experience IDs compose together. The first mounted experience owns
/// the host's default initial route.
#[test]
fn two_modules_with_distinct_experiences_compose_together() {
    struct Second;

    impl GameModule for Second {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("second_game")
        }
        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience("second_game")
                .gameplay_route("second_game/play")
                .characters(MINIMAL_CHARACTER_ROSTER_RON)
                .no_audio()
                .playable(
                    "Second Game",
                    "mounted beside the minimal game",
                    "my_hero",
                    minimal_game::minimal_experience::MINIMAL_ROOM_ID,
                    vec![minimal_game::minimal_experience::minimal_room()],
                );
        }
    }

    let mut app = PlatformerApp::headless()
        .mount(the_one_module())
        .mount(Second)
        .try_build()
        .expect("two modules with distinct experiences must compose");

    let mut status = host_status(&app);
    for _ in 0..600 {
        app.update();
        status = host_status(&app);
        if status.is_running() || status.is_refused() {
            break;
        }
    }
    assert!(
        status.is_running(),
        "the composition must still reach a running host; got {status:?} / {:?}",
        status.refusal()
    );
    assert_eq!(
        status.route(),
        Some(minimal_game::MINIMAL_GAMEPLAY_ROUTE),
        "the FIRST mounted experience owns the host's initial route"
    );
}

/// Duplicate experience IDs are refused, and the error names both owners.
#[test]
fn two_modules_claiming_one_experience_id_conflict_and_the_error_names_both() {
    struct Squatter;

    impl GameModule for Squatter {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("squatter")
        }
        fn define(&self, module: &mut ModuleDraft) {
            module.experience(minimal_game::MINIMAL_EXPERIENCE);
        }
    }

    let error = PlatformerApp::headless()
        .mount(the_one_module())
        .mount(Squatter)
        .try_build()
        .expect_err("two modules cannot own one experience id");
    let reported = error.to_string();
    for expected in [minimal_game::MINIMAL_EXPERIENCE, "squatter"] {
        assert!(
            reported.contains(expected),
            "the conflict must name {expected:?} so it can be fixed without a \
             debugger; got {reported:?}"
        );
    }
}

/// A running host is not enough to prove authored content is playable. The live
/// player must land and settle on the authored floor.
#[test]
fn the_walker_lands_on_the_floor_instead_of_falling_through_it() {
    use ambition_platformer2d::bevy::prelude::With;

    let mut app = PlatformerApp::headless().mount(the_one_module()).build();
    for _ in 0..600 {
        app.update();
        if host_status(&app).is_running() {
            break;
        }
    }
    assert!(host_status(&app).is_running(), "the host must start first");

    // Long enough to fall, land, and settle — and long enough that a body which
    // is falling-dying-respawning has gone round that loop several times.
    let mut samples = Vec::new();
    for _ in 0..300 {
        app.update();
        let world = app.world_mut();
        let mut bodies = world
            .query_filtered::<&ambition_platformer2d::actor::BodyKinematics, With<ambition_platformer2d::actor::PrimaryPlayer>>();
        if let Ok(kin) = bodies.single(world) {
            samples.push(kin.pos.y);
        }
    }
    assert!(samples.len() > 200, "lost the body mid-run");

    let last_fifty = &samples[samples.len() - 50..];
    let lo = last_fifty.iter().cloned().fold(f32::INFINITY, f32::min);
    let hi = last_fifty.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        (hi - lo) < 2.0,
        "the walker never settled: y ranged {lo}..{hi} over the last 50 ticks, \
         which is what falling through the floor and respawning looks like"
    );
}

/// The published one-character roster must compose with the character ID it
/// declares.
#[test]
fn the_published_one_character_roster_composes_and_runs() {
    struct FromTheDocs;

    impl GameModule for FromTheDocs {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("from_the_docs")
        }
        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience("from_the_docs")
                .launcher_route("from_the_docs/menu")
                .gameplay_route("from_the_docs/play")
                .characters(MINIMAL_CHARACTER_ROSTER_RON)
                .no_audio()
                .playable(
                    "From The Docs",
                    "built from the published roster constant",
                    // Keep the published roster and its documented character ID aligned.
                    "my_hero",
                    minimal_game::minimal_experience::MINIMAL_ROOM_ID,
                    vec![minimal_game::minimal_experience::minimal_room()],
                );
        }
    }

    let mut app = PlatformerApp::headless()
        .mount(FromTheDocs)
        .try_build()
        .expect("the roster we publish must parse and validate");

    let mut status = host_status(&app);
    for _ in 0..600 {
        app.update();
        status = host_status(&app);
        if status.is_running() || status.is_refused() {
            break;
        }
    }
    assert!(
        status.is_running(),
        "the published roster must reach a RUNNING host, not merely parse; got \
         {status:?} / {:?}",
        status.refusal()
    );
}

/// A multi-game host can start at its launcher. The test also proves that this
/// policy differs from the default first-game route.
#[test]
fn a_multi_game_host_can_start_at_its_launcher() {
    struct Second;

    impl GameModule for Second {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("second_game")
        }
        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience("second_game")
                .gameplay_route("second_game/play")
                .characters(MINIMAL_CHARACTER_ROSTER_RON)
                .no_audio()
                .playable(
                    "Second Game",
                    "the other game this host ships",
                    "my_hero",
                    minimal_game::minimal_experience::MINIMAL_ROOM_ID,
                    vec![minimal_game::minimal_experience::minimal_room()],
                );
        }
    }

    fn settle(app: &mut ambition_platformer2d::bevy::prelude::App) -> HostStatus {
        let mut status = host_status(app);
        for _ in 0..600 {
            app.update();
            status = host_status(app);
            if status.is_running() || status.is_refused() {
                break;
            }
        }
        status
    }

    let mut default_host = PlatformerApp::headless()
        .mount(the_one_module())
        .mount(Second)
        .build();
    let default_status = settle(&mut default_host);
    assert_eq!(
        default_status.route(),
        Some(minimal_game::MINIMAL_GAMEPLAY_ROUTE),
        "by default a host starts in its first game"
    );

    let mut launcher_host = PlatformerApp::headless()
        .start_at_launcher()
        .mount(the_one_module())
        .mount(Second)
        .build();
    let launcher_status = settle(&mut launcher_host);

    assert!(
        !launcher_status.is_refused(),
        "the launcher policy must compose: {:?}",
        launcher_status.refusal()
    );
    assert_eq!(
        launcher_status.route(),
        Some(minimal_game::MINIMAL_LAUNCHER_ROUTE),
        "with the launcher policy the host lands on the launcher route, not in a game"
    );
    assert_ne!(
        launcher_status.route(),
        default_status.route(),
        "the two policies must differ, or the flag is being ignored"
    );
}

/// The SDK's worked room example compiles and runs.
///
/// Keep this executable example aligned with the README so public vocabulary
/// changes fail in CI rather than for readers.
#[test]
fn the_sdk_worked_room_example_compiles_and_runs() {
    // Exactly the README's imports for a room: the domain prelude, nothing else.
    use ambition_platformer2d::world::prelude::*;

    fn my_room() -> RoomSpec {
        let size = Vec2::new(640.0, 360.0);
        let world = AuthoredWorld::new(
            "My Room",
            size,
            Vec2::new(64.0, 256.0),
            vec![Block::solid(
                "floor",
                Vec2::new(0.0, 320.0),
                Vec2::new(size.x, 40.0),
            )],
        );
        RoomSpec::new("my_room", world)
    }

    struct FromTheReadme;

    impl GameModule for FromTheReadme {
        fn manifest(&self) -> ModuleManifest {
            // No AssetSource — the README says one is optional, so the test
            // that proves the README must not quietly add one.
            ModuleManifest::new("from_the_readme")
        }
        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience("from_the_readme")
                .launcher_route("from_the_readme/menu")
                .gameplay_route("from_the_readme/play")
                .characters(MINIMAL_CHARACTER_ROSTER_RON)
                .no_audio()
                // `my_hero` is the id MINIMAL_CHARACTER_ROSTER_RON declares —
                // the connection the README now states and this pins.
                .playable(
                    "From The Readme",
                    "…",
                    "my_hero",
                    "my_room",
                    vec![my_room()],
                );
        }
    }

    let mut app = PlatformerApp::headless()
        .mount(FromTheReadme)
        .try_build()
        .expect("the SDK's own worked example must compose");

    let mut status = host_status(&app);
    for _ in 0..600 {
        app.update();
        status = host_status(&app);
        if status.is_running() || status.is_refused() {
            break;
        }
    }
    assert!(
        status.is_running(),
        "the SDK's worked example must RUN, not merely compile; got {status:?} / {:?}",
        status.refusal()
    );
}

/// A declaration-stage refusal must say that capability-dependent checks have
/// not run yet. Later checks require successful assembly and cannot be folded
/// into the declaration pass.
#[test]
fn a_declaration_refusal_says_the_later_checks_have_not_run() {
    struct Silent;

    impl GameModule for Silent {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("silent")
        }
        fn define(&self, _module: &mut ModuleDraft) {}
    }

    let error = PlatformerApp::headless()
        .mount(Silent)
        .try_build()
        .expect_err("a module declaring no experience id cannot compose");
    assert_eq!(
        error.stage,
        ambition_platformer2d::app::CompositionStage::Declaration,
        "a draft refusal was reported as an assembly one, so it claims every \
         capability-dependent check already passed"
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains("have not run yet"),
        "the refusal does not say that later checks are still pending, so its \
         problem count reads as the complete list: {rendered}"
    );
}

/// The SDK exposes both primary-seat and explicit-seat input driving without
/// requiring an implementation-crate import.
#[test]
fn a_consumer_can_name_both_input_seams_without_leaving_the_sdk() {
    use ambition_platformer2d::sim::{
        drive_control_frame, drive_slot_frame, ControlFrame, PlayerSlot,
    };

    macro_rules! a_game {
        () => {
            PlatformerApp::headless()
                .mount(the_one_module())
                .try_build()
                .expect("the smallest game composes headless")
        };
    }
    let pressed = ControlFrame {
        axis_x: 1.0,
        ..Default::default()
    };

    // Seat 0 by name, seat 1 by number. Neither call needs to know which host is
    // running — that is the whole point of the seam, and a driver that guessed
    // wrong would move nothing and be told nothing.
    let mut app = a_game!();
    drive_control_frame(app.world_mut(), ControlFrame::default());
    drive_slot_frame(app.world_mut(), PlayerSlot(1), ControlFrame::default());

    // Compare the two public seams through the same observable result. The host
    // owns which underlying input resource receives the frame.
    let mut by_name = a_game!();
    let mut by_number = a_game!();
    for _ in 0..8 {
        drive_control_frame(by_name.world_mut(), pressed);
        by_name.update();
        drive_slot_frame(by_number.world_mut(), PlayerSlot(0), pressed);
        by_number.update();
    }
    let named = *by_name.world_mut().resource::<ControlFrame>();
    let numbered = *by_number.world_mut().resource::<ControlFrame>();
    assert_eq!(
        named.axis_x, pressed.axis_x,
        "the fixture never delivered the press through EITHER seam, so nothing \
         below distinguishes them",
    );
    assert_eq!(
        numbered, named,
        "a press driven at PlayerSlot(0) through the general seam did not reach \
         the seat that `drive_control_frame` reaches: {numbered:?} against \
         {named:?}",
    );
}
