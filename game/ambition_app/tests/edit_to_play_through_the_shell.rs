//! ⭐⭐ **EDIT DISK -> THE CONSTRUCTED ACTOR PLAYS IT, THROUGH THE SHELL.**
//!
//! The 2026-09-12 review's final acceptance sentence, end to end: edit a file on
//! disk, compile a candidate, request a reload, let the SHELL re-prepare and
//! activate on its own, and show the body the game CONSTRUCTED is playing the
//! edited move. No Cargo, no linker, no direct publication API.
//!
//! ⛔ THE HOP THE CRATE-LEVEL ARM DOES NOT REACH.
//! `a_host_plays_a_move_edited_on_disk_after_it_was_built` asserts the edit
//! arrives in `PreparedCharacterRegistry` — the registry, not the fighter. This
//! asserts `ActorMoveset` on a BUILT BODY. Poisoning the call site that passes
//! the transaction into `content_identity_for` left 868 tests green because
//! catching it needs a REAL preparation, and this is the arm that runs one.
//!
//! ⛔⛤ **WHY THIS ROOM, MEASURED, AFTER TWO WRONG COMPOSITIONS.**
//! 1. The shipped `ambition_gameplay` DEFAULT room was the first candidate and
//!    is useless here: its player body (`slot:0`) wears the eight DERIVED KIT
//!    moves (`attack_up`, `attack_air`, ...), and `player_robot` has no authored
//!    move table at all, so the authored ids and that body's ids are DISJOINT.
//! 2. The smash composition DOES seat an authored fighter — `smash_roster(
//!    ["director"])` gives a body with 33 moves in the `director_*` vocabulary — and
//!    is STILL wrong, because it runs a live rollback timeline:
//!    `request_reload` there returns `Refused(RefusedDuringLiveTimeline)`. A
//!    healthy speculating timeline refuses publication BY DESIGN.
//! ⇒ The witness needs a body playing AUTHORED moves *and* a legal publication
//!    boundary. MEASURED in `proving_grounds`: route `ambition_gameplay` through
//!    the shell, NO `ActiveRollbackAuthority` (so the boundary is Legal), and
//!    three goblin bodies carrying 34 moves whose ids are `goblin.ron`'s own.
//!
//! ⚠ AND THE EDIT IS A TIMING, NEVER AN EXTENT. MEASURED: for ten of sixteen
//! fighters the authored strike GEOMETRY is overridden by a rendered sprite
//! stage that derives the hitbox from a bone segment, so an extents edit would
//! not reach the actor for most of the roster. Timing is not overridden.
use crate::common;

/// An AUTHORED move id — `goblin.ron`'s, not one of the derived kit ids.
const SUBJECT: &str = "jab";
/// The authored source it lives in, as `pack.ron` spells it.
const SUBJECT_SOURCE: &str = "data/movesets/goblin.ron";
/// What the edit adds. Large enough that no rounding could explain the result.
const BUMP: f32 = 0.5;

/// Every duration a CONSTRUCTED body is currently playing for [`SUBJECT`],
/// entity order removed.
///
/// ⛔ Re-found by moveset content every call, never cached as an `Entity`: a
/// re-preparation may rebuild the room's bodies, and a stale entity id would
/// read a despawned actor and look like "the edit never arrived".
///
/// ⛔⛤ **IT USED TO BE `.next()`, AND `.next()` OF A QUERY IS STORAGE ORDER.**
/// `SUBJECT` is `"jab"` — a move id `goblin.ron` authors and **so does the
/// player's own character**, at a different duration. The arm therefore sampled
/// whichever of the two happened to sort first, and it sorted a goblin only by
/// luck. MEASURED 2026-09-13: adding ONE resource to the App shifted every entity
/// id by one, moved `Player Robot v3` to the head of the archetype iteration, and
/// reddened this test **with every goblin carrying the edited value correctly**:
///
/// ```text
/// before: [(515, "Goblin", 0.71), (516, "Goblin", 0.71), (517, "Goblin", 0.71), (523, "Player Robot v3", 0.25)]
/// after:  [(524, "Player Robot v3", 0.25), (516, "Goblin", 0.71), (517, "Goblin", 0.71), (518, "Goblin", 0.71)]
/// ```
///
/// ⇒ A red that says "the edit did not arrive" when the edit arrived on every
/// body it was authored for is a false red, and a false red is obeyed faster
/// than a false green is questioned. The population is the measurement.
fn subject_durations_on_bodies(
    sim: &mut ambition_sim_harness::Platformer2dSimHarness,
) -> Vec<f32> {
    use ambition_platformer2d::combat::moveset::ActorMoveset;
    let world = sim.world_mut();
    let mut q = world.query::<&ActorMoveset>();
    let mut found: Vec<f32> = q
        .iter(world)
        .filter_map(|ms| ms.0.moves.iter().find(|m| m.id == SUBJECT))
        .map(|m| m.duration_s)
        .collect();
    // ⚠ SORTED so a comparison of two samples cannot depend on iteration order
    // either — the defect above, one level up.
    found.sort_by(f32::total_cmp);
    found
}

/// The duration every body that AUTHORS [`SUBJECT`] at `expected` is playing,
/// counted — and `None` when the population does not agree on one value.
///
/// ⛔ THE QUESTION THIS ARM IS ABOUT IS *"did the edit reach the bodies built
/// from the edited source"*, so the answer is a COUNT over a population, not one
/// body's reading.
fn subject_body_count_at(
    sim: &mut ambition_sim_harness::Platformer2dSimHarness,
    expected: f32,
) -> usize {
    subject_durations_on_bodies(sim)
        .into_iter()
        .filter(|found| (found - expected).abs() < 1e-4)
        .count()
}

/// Export this build's sources to `root` and move [`SUBJECT`]'s timing by
/// [`BUMP`]. Returns (sources written, moves edited, authored duration before).
///
/// ⛔ NO `tempfile` DEV-DEPENDENCY IS ADDED FOR THIS — a pid+nanos directory
/// under the system temp dir, removed on the way out.
///
/// ⛔⛤ **EDITED THROUGH THE TYPED DOCUMENT, AND THE EDIT IS COUNTED.** A fixture
/// edit that matches nothing and a passing test look identical; this repository
/// has been bitten by a zero-match substitution twice in one day.
fn export_with_the_subject_retimed(root: &std::path::Path) -> (usize, usize, f32) {
    let written = ambition_content::pack::export_sources_to(root).expect("sources export");
    let path = root.join(SUBJECT_SOURCE);
    let text = std::fs::read_to_string(&path).expect("the exported subject source reads");
    let mut doc = ambition_platformer2d::entity_catalog::EntityCatalogDoc::parse(&text)
        .expect("a shipped move table parses as its typed document");
    let (mut edited, mut before) = (0usize, f32::NAN);
    for entity in &mut doc.entities {
        let Some(moveset) = entity.contracts.moveset.as_mut() else {
            continue;
        };
        for spec in &mut moveset.moves {
            if spec.id != SUBJECT {
                continue;
            }
            before = spec.duration_s;
            let after = spec.duration_s + BUMP;
            // ⚠ THE LAST WINDOW FOLLOWS THE DURATION when it ended with the
            // move, so the edit cannot produce a timeline shorter than its own
            // length and be refused for a reason that is not the subject.
            for window in &mut spec.windows {
                if (window.end_s - spec.duration_s).abs() < 1e-6 {
                    window.end_s = after;
                }
            }
            spec.duration_s = after;
            edited += 1;
        }
    }
    std::fs::write(&path, doc.to_ron().expect("the edited document serializes"))
        .expect("the edited source writes");
    (written, edited, before)
}

/// The shell's current activation id, or `None` before any route is active.
fn activation_id(
    sim: &mut ambition_sim_harness::Platformer2dSimHarness,
) -> Option<ambition_platformer2d::game_shell::ShellActivationId> {
    sim.world()
        .get_resource::<ambition_platformer2d::game_shell::ShellRouter>()
        .and_then(|r| r.active.as_ref())
        .map(|a| a.activation_id)
}

/// The live cast's generation.
fn cast_generation(sim: &mut ambition_sim_harness::Platformer2dSimHarness) -> Option<u64> {
    sim.world()
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .map(|r| r.generation().get())
}

/// ⭐⭐ **THE ACCEPTANCE ARM.**
///
/// ⛔⛤ **WHAT THIS ARM DOES *NOT* WITNESS, STATED SO NOBODY TRUSTS IT FOR THE
/// WRONG THING.** It witnesses the ACTIVATION -> ACTOR publication: the shell
/// activates, and at the end of that frame the constructed body plays the edit.
/// It does NOT witness the provider-ORDERING edge. MEASURED: deleting
/// `.before(ambition_platformer2d::game_shell::GameplaySessionSet::Providers)`
/// from `ambition_content::reload::register` leaves this arm GREEN, even in the
/// activating-frame form above — because `proving_grounds`' goblin bodies ALREADY
/// EXIST and the re-preparation does not rebuild them, so nothing constructs a
/// session for the edge to order against. The constraint is vacuously satisfied
/// on this road, and an arm cannot protect a constraint its road does not
/// exercise.
///
/// ⚠ **THE MEASUREMENT NAMED `activate_prepared_platformer_sessions` UNTIL
/// 2026-09-19, AND THAT SYSTEM IS GONE.** A10.5 split it: construction is
/// `prepare_candidate_platformer_session`, which is
/// `.before(AmbitionGameShellSet::Pending)`, and what remains in
/// `GameplaySessionSet::Providers` is `adopt_candidate_platformer_session`,
/// which adopts an already-built world rather than building one. ⇒ The
/// vacuity above is a fact about THIS ROAD and still holds; what the
/// `.before(Providers)` edge now guarantees is a different question, and
/// `ambition_content::reload::register` answers it at the edge itself.
///
/// ⇒ That edge is held by
/// `an_edit_reaches_the_shipped_game::an_edited_pack_reaches_the_cast_the_shipped_composition_plays`
/// (`an_edit_reaches_the_shipped_game.rs`), where the world IS built from the
/// publication and removing the edge does redden it. If that arm is ever
/// deleted or repointed, the ordering loses its only witness — this one will
/// not notice.
#[test]
fn an_edit_on_disk_reaches_the_constructed_actor_through_the_shell() {
    let mut sim = common::fixed_60hz_room_sim("proving_grounds");
    for _ in 0..40 {
        sim.step(common::base());
    }

    // ── premise: constructed bodies are playing the AUTHORED move ───────────
    //
    // ⚠ THE POPULATION, and its SIZE is part of the premise: this room builds
    // three goblins from `goblin.ron`, and an arm that cannot say how many
    // bodies it is about cannot tell "the edit reached them" from "the edit
    // reached the one I happened to sample".
    let before = subject_durations_on_bodies(&mut sim).first().copied().unwrap_or_else(|| {
        panic!(
            "no constructed body plays `{SUBJECT}`, so this room cannot witness an authored edit"
        )
    });
    assert!(
        before.is_finite() && before > 0.0,
        "the body reports a nonsense duration for `{SUBJECT}`: {before}"
    );
    // ⛔⛔ **THE ANTI-VACUITY FLOOR, AND IT IS THE HALF THE OLD `.next()` COULD
    // NOT HAVE.** Every later assertion is a COUNT compared against this one, so
    // a run where the goblins silently stopped being built would fail here
    // rather than pass two counts of zero against each other.
    let subjects_before = subject_body_count_at(&mut sim, before);
    assert!(
        subjects_before >= 3,
        "this room is supposed to build three bodies from `{SUBJECT_SOURCE}` and \
         {subjects_before} carry `{SUBJECT}` at the authored {before}s — the arm \
         would be comparing counts of nearly nothing"
    );
    // ⛔ THE BOUNDARY MUST BE LEGAL, ASSERTED RATHER THAN HOPED. A live rollback
    // timeline refuses publication by design, and the smash composition — the
    // other place an authored body exists — is refused for exactly that reason.
    assert!(
        sim.world()
            .get_resource::<ambition_platformer2d::runtime::rollback::ActiveRollbackAuthority>()
            .is_none(),
        "this room has a live rollback authority, so the reload will be refused \
         at the publication boundary and the arm would be about that refusal"
    );

    let before_activation = activation_id(&mut sim);
    let before_generation = cast_generation(&mut sim);
    assert!(
        before_activation.is_some() && before_generation.is_some(),
        "the room has no active shell route or no published cast, so neither clock \
         this arm watches exists yet"
    );

    let root = std::env::temp_dir().join(format!(
        "ambition_edit_to_play_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (written, edited, authored_before) = export_with_the_subject_retimed(&root);
        assert!(
            written > 10,
            "the export wrote {written} source(s); the pack declares many more, so \
             the edit would be against a directory that is not this build's content"
        );
        // ⛔ THE EDIT MATCHED EXACTLY ONE MOVE.
        assert_eq!(
            edited, 1,
            "the retime matched {edited} authored move(s) named `{SUBJECT}` in \
             {SUBJECT_SOURCE}; it must match exactly one or this witness is not \
             about the move it names"
        );
        assert!(
            (authored_before - before).abs() < 1e-4,
            "the body plays `{SUBJECT}` at {before} but the authored source says \
             {authored_before} — the body is not playing the file this edits"
        );

        let pack = ambition_content::pack::compile_pack_from(&root)
            .expect("the edited directory compiles as a pack");
        let candidate = ambition_platformer2d::content::CandidateGeneration::prepared_against(
            std::sync::Arc::new(pack),
            None,
        );
        let request = ambition_content::reload::request_reload(sim.world_mut(), candidate);
        assert!(
            matches!(
                request,
                ambition_content::reload::ReloadRequest::Requested { .. }
            ),
            "the reload was not requested, so nothing downstream is about an edit; \
             got {request:?}"
        );

        // ⭐ HALF THE CLAIM: THE REQUEST PUBLISHES NOTHING. The staging boundary
        // is what makes the activation atomic instead of incremental.
        assert_eq!(
            subject_body_count_at(&mut sim, before),
            subjects_before,
            "the request alone changed what the live bodies play — the edit reached \
             the actors before any activation, so there is no boundary to be atomic at"
        );

        // ⛔⛔ **ASSERT AT THE ACTIVATING FRAME, NOT AT A FRAME BUDGET'S END.**
        // A loop that waits for the VALUE and then asserts it passes even when
        // the publication lands a frame AFTER the world was built from it — and
        // that one-frame gap is a real, shipped defect class here (activation on
        // frame N, session provider constructing the world on frame N, the family
        // landing on N+1). MEASURED: with this arm bounded by a 600-frame budget
        // and asserting only the final value, deleting
        // `.before(GameplaySessionSet::Providers)` from `reload::register` did
        // NOT redden it. A tolerance is a claim that the gap does not matter, and
        // here the gap IS the thing under test.
        // ⇒ So: step until the ACTIVATION ID moves, stop on that frame, and
        // assert the actor at the end of it.
        let mut activated_at = None;
        for frame in 0..600 {
            sim.step(common::base());
            if activation_id(&mut sim) != before_activation {
                activated_at = Some(frame);
                break;
            }
        }
        let activated_at = activated_at.unwrap_or_else(|| {
            panic!(
                "600 frames after the request the shell never activated a new \
                 generation (activation is still {before_activation:?}), so the break \
                 is before the activation and not after it — do NOT raise the frame \
                 count without finding out which hop"
            )
        });

        // ⛔ EVERY BODY BUILT FROM THE EDITED SOURCE, not the first one the
        // archetype iteration happens to yield — see `subject_durations_on_bodies`.
        let now = subject_body_count_at(&mut sim, before + BUMP);
        assert_eq!(
            now,
            subjects_before,
            "the shell activated on frame {activated_at} and the constructed body \
             is STILL playing `{SUBJECT}` at {now:?} rather than {}. The publication \
             and the world built from it are one frame apart, which is the ordering \
             `reload::register`'s `.before(GameplaySessionSet::Providers)` exists to \
             hold.",
            before + BUMP
        );

        // ⛔⛤ **AND THE CAST CLOCK MUST HAVE MOVED, BECAUSE THIS CANDIDATE EDITS A
        // MOVE TABLE.** An app-level witness whose candidate touches no moveset
        // sees this clock stand still — correctly, since nothing is staged. This
        // one stages a cast, so a generation that did NOT advance means the
        // staging block was skipped and the edit reached the actor by some other
        // road than the one under test.
        assert_ne!(
            cast_generation(&mut sim),
            before_generation,
            "the cast generation did not advance for a candidate that edits \
             `{SUBJECT}`, so no cast was staged and the value above arrived by a \
             road this arm does not describe"
        );
    }));
    let _ = std::fs::remove_dir_all(&root);
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}

/// ⭐ THE RUNNING GAME PLAYS A SAVED FILE, with no call from the test: the
/// content watch (`ambition_content::content_watch`) sees the source change,
/// compiles the pack from disk, and asks for the reload the arm above asks for
/// by hand. The watch is pointed at an exported copy, so the arm edits no file
/// another test reads.
#[cfg(not(feature = "static_content"))]
#[test]
fn a_content_file_saved_while_the_game_runs_is_played() {
    use ambition_content::content_watch::ContentSourceWatch;
    let mut sim = common::fixed_60hz_room_sim("proving_grounds");
    for _ in 0..40 {
        sim.step(common::base());
    }
    let before = subject_durations_on_bodies(&mut sim)
        .first()
        .copied()
        .expect("a constructed body plays the subject");
    let subjects_before = subject_body_count_at(&mut sim, before);
    assert!(subjects_before >= 3, "the premise: three bodies play `{SUBJECT}`");
    assert!(
        sim.world().get_resource::<ContentSourceWatch>().is_some(),
        "the premise: a build that reads content off disk watches it"
    );

    let root = std::env::temp_dir().join(format!("ambition_content_watch_{}", std::process::id()));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ambition_content::pack::export_sources_to(&root).expect("sources export");
        sim.world_mut().insert_resource(ContentSourceWatch::new(root.clone()));
        let requested = |sim: &ambition_sim_harness::Platformer2dSimHarness| {
            sim.world().resource::<ContentSourceWatch>().requested
        };
        for _ in 0..45 {
            sim.step(common::base());
        }
        assert_eq!(requested(&sim), 0, "an unchanged copy asks for nothing");

        // A file system's clock can be coarse: the save must look newer.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let (_, edited, _) = export_with_the_subject_retimed(&root);
        assert_eq!(edited, 1);
        let mut frames = 0;
        while subject_body_count_at(&mut sim, before + BUMP) < subjects_before {
            sim.step(common::base());
            frames += 1;
            assert!(
                frames < 600,
                "600 frames after the save the bodies still play `{SUBJECT}` at {:?}; \
                 reloads requested: {}",
                subject_durations_on_bodies(&mut sim),
                requested(&sim)
            );
        }
        assert_eq!(requested(&sim), 1, "one save, one reload");
        eprintln!("a saved move edit was played {frames} frames after the save");
    }));
    let _ = std::fs::remove_dir_all(&root);
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}

/// Each live mockingbird's maximum HP.
#[cfg(not(feature = "static_content"))]
fn mockingbird_max_hps(sim: &mut ambition_sim_harness::Platformer2dSimHarness) -> Vec<i32> {
    let world = sim.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::boss_encounter::BossConfig,
        &ambition_platformer2d::characters::actor::BodyHealth,
    )>();
    q.iter(world)
        .filter(|(config, _)| config.behavior.id == "mockingbird")
        .map(|(_, health)| health.max())
        .collect()
}

/// The live mockingbird's `strike_speed_scale`, and how many mockingbirds.
#[cfg(not(feature = "static_content"))]
fn mockingbird_strike_speed_scales(sim: &mut ambition_sim_harness::Platformer2dSimHarness) -> Vec<f32> {
    let world = sim.world_mut();
    let mut q = world.query::<&ambition_platformer2d::boss_encounter::BossConfig>();
    q.iter(world)
        .filter(|config| config.behavior.id == "mockingbird")
        .map(|config| config.behavior.strike_speed_scale)
        .collect()
}

/// ⭐ A BOSS TUNING EDIT IS PLAYED WITHOUT A RESTART. `boss_profiles.ron` is
/// a participating domain of the reload (2026-10-01): the candidate catalog is
/// admitted at request time, frozen by the preparation, and published at the
/// activation, so the boss the new session builds plays the saved value.
#[cfg(not(feature = "static_content"))]
#[test]
fn a_boss_tuning_saved_while_the_game_runs_is_played() {
    use ambition_content::content_watch::ContentSourceWatch;
    const EDITED: f32 = 0.37;
    let mut sim = common::fixed_60hz_room_sim("mockingbird_arena");
    for _ in 0..40 {
        sim.step(common::base());
    }
    let before = mockingbird_strike_speed_scales(&mut sim);
    assert_eq!(before.len(), 1, "the premise: the arena builds one mockingbird");
    assert!((before[0] - EDITED).abs() > 0.1, "the premise: the edit changes the value");
    // And its encounter: the HP is seeded from the encounter file.
    const EDITED_HP: i32 = 41;
    assert_eq!(mockingbird_max_hps(&mut sim).len(), 1);
    assert_ne!(mockingbird_max_hps(&mut sim)[0], EDITED_HP, "the premise: the edit changes the HP");

    let root = std::env::temp_dir().join(format!("ambition_boss_watch_{}", std::process::id()));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ambition_content::pack::export_sources_to(&root).expect("sources export");
        sim.world_mut().insert_resource(ContentSourceWatch::new(root.clone()));
        std::thread::sleep(std::time::Duration::from_millis(20));
        let path = root.join("data/boss_profiles.ron");
        let text = std::fs::read_to_string(&path).unwrap();
        let start = text.find("\"mockingbird\": (").expect("the roster has a mockingbird row");
        let at = start + text[start..].find("strike_speed_scale: ").expect("the row has a strike speed scale");
        let end = at + text[at..].find(',').unwrap();
        let edited = format!("{}strike_speed_scale: {EDITED}{}", &text[..at], &text[end..]);
        std::fs::write(&path, edited).unwrap();
        let encounter = root.join("data/boss_encounters/mockingbird.ron");
        let text = std::fs::read_to_string(&encounter).unwrap();
        let at = text.find("max_hp: ").expect("the encounter states its HP");
        let end = at + text[at..].find(',').unwrap();
        std::fs::write(&encounter, format!("{}max_hp: {EDITED_HP}{}", &text[..at], &text[end..])).unwrap();

        let mut frames = 0;
        while mockingbird_strike_speed_scales(&mut sim).first().is_none_or(|v| (v - EDITED).abs() > 1e-6) {
            sim.step(common::base());
            frames += 1;
            assert!(
                frames < 600,
                "600 frames after the save the mockingbird plays {:?}; reloads requested: {}",
                mockingbird_strike_speed_scales(&mut sim),
                sim.world().resource::<ContentSourceWatch>().requested
            );
        }
        assert_eq!(mockingbird_strike_speed_scales(&mut sim).len(), 1, "one mockingbird, rebuilt");
        let catalog = sim.world().resource::<ambition_platformer2d::boss_encounter::BossCatalog>();
        assert_eq!(
            catalog.behavior("mockingbird").map(|b| b.strike_speed_scale),
            Some(EDITED),
            "the App's catalog is the published one"
        );
        // ⛔ AND THE GENERATION THE SESSION FROZE. Until 2026-10-01
        // `update_boss_encounters` re-seeded the behaviour from the App's
        // catalog after construction, so a session that froze N's catalog
        // under N+1's identity still showed N+1 on the boss (poison "the claim
        // carries no catalog" stayed green). The live value witnesses the
        // freeze now; this asserts the frozen record directly as well.
        let frozen = sim
            .world()
            .resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
            .bosses
            .behavior("mockingbird")
            .map(|b| b.strike_speed_scale);
        assert_eq!(frozen, Some(EDITED), "the session froze the catalog it was prepared from");
        // ⛔ The encounter is seeded from the catalog the boss was built with
        // (`BossConfig::seed`), not from the App's on the boss's first tick.
        assert_eq!(mockingbird_max_hps(&mut sim), [EDITED_HP], "the rebuilt boss has the saved HP");
        eprintln!("a saved boss tuning was played {frames} frames after the save");
    }));
    let _ = std::fs::remove_dir_all(&root);
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}

/// ⭐ THE SAVED EDIT UNDER THE SHIPPED OWNERSHIP MODE: a timeline the local
/// maintainer owns is rebased onto the reloaded generation, not crossed by it.
#[cfg(not(feature = "static_content"))]
#[test]
fn a_content_file_saved_under_a_local_timeline_rebases_it() {
    use ambition_app::rl_sim::AmbitionSim as _;
    use ambition_content::content_watch::ContentSourceWatch;
    let mut sim = ambition_sim_harness::Platformer2dSimHarness::new_with_options(
        common::fixed_60hz_room_options("proving_grounds").with_sync_test_rollback_settings(4, 10),
    )
    .expect("the room builds under a sync-test session");
    common::hand_the_timeline_to_the_local_maintainer(&mut sim);
    for _ in 0..40 {
        sim.step(common::base());
    }
    let boundary = |sim: &ambition_sim_harness::Platformer2dSimHarness| {
        format!("{:?}", ambition_platformer2d::rollback::mechanical_mutation_boundary(sim.world()))
    };
    let bound_and_live = |sim: &mut ambition_sim_harness::Platformer2dSimHarness| {
        let bound = sim
            .world()
            .get_resource::<ambition_platformer2d::runtime::rollback::ActiveRollbackAuthority>()
            .and_then(|authority| authority.contract().content);
        let world = sim.world_mut();
        let mut q = world.query::<&ambition_platformer2d::runtime::PreparedContentIdentity>();
        (bound, q.single(world).ok().copied())
    };
    assert_eq!(boundary(&sim), "LocallyRebasable", "the premise: this host owns the timeline");
    let (bound_before, live) = bound_and_live(&mut sim);
    assert!(bound_before.is_some() && bound_before == live, "the premise: the timeline binds the session's content");
    let before = subject_durations_on_bodies(&mut sim).first().copied().expect("a body plays the subject");
    let subjects = subject_body_count_at(&mut sim, before);

    let root = std::env::temp_dir().join(format!("ambition_content_watch_local_{}", std::process::id()));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ambition_content::pack::export_sources_to(&root).expect("sources export");
        sim.world_mut().insert_resource(ContentSourceWatch::new(root.clone()));
        std::thread::sleep(std::time::Duration::from_millis(20));
        export_with_the_subject_retimed(&root);
        let mut frames = 0;
        while subject_body_count_at(&mut sim, before + BUMP) < subjects {
            sim.step(common::base());
            frames += 1;
            assert!(frames < 600, "600 frames after the save the bodies still play the old move");
        }
        for _ in 0..10 {
            sim.step(common::base());
        }
        assert_eq!(boundary(&sim), "LocallyRebasable", "the maintainer started the timeline again");
        let (bound, live) = bound_and_live(&mut sim);
        assert_eq!(bound, live, "the new timeline binds the reloaded content");
        assert_ne!(bound, bound_before, "and that content is a new generation");
        sim.rollback_health().expect("the rebased timeline is healthy");
    }));
    let _ = std::fs::remove_dir_all(&root);
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}

/// Each live goblin's maximum HP.
#[cfg(not(feature = "static_content"))]
fn goblin_max_hps(sim: &mut ambition_sim_harness::Platformer2dSimHarness) -> Vec<i32> {
    let world = sim.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::characters::actor::WornCharacter,
        &ambition_platformer2d::characters::actor::BodyHealth,
    )>();
    q.iter(world)
        .filter(|(worn, _)| worn.0.to_string() == "goblin")
        .map(|(_, health)| health.max())
        .collect()
}

/// ⭐ A CHARACTER CATALOG EDIT IS PLAYED WITHOUT A RESTART. A saved row is a
/// revision of the whole cast, folded against the candidate catalog, frozen by
/// the preparation and published with the catalog at the activation.
#[cfg(not(feature = "static_content"))]
#[test]
fn a_character_row_saved_while_the_game_runs_is_played() {
    use ambition_content::content_watch::ContentSourceWatch;
    const EDITED: i32 = 9;
    let mut sim = common::fixed_60hz_room_sim("proving_grounds");
    for _ in 0..40 {
        sim.step(common::base());
    }
    let before = goblin_max_hps(&mut sim);
    assert!(before.len() >= 3, "the premise: the room builds three goblins, got {before:?}");
    assert!(before.iter().all(|&hp| hp != EDITED), "the premise: the edit changes the HP: {before:?}");

    let root = std::env::temp_dir().join(format!("ambition_catalog_watch_{}", std::process::id()));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ambition_content::pack::export_sources_to(&root).expect("sources export");
        sim.world_mut().insert_resource(ContentSourceWatch::new(root.clone()));
        std::thread::sleep(std::time::Duration::from_millis(20));
        let path = root.join("data/character_catalog.ron");
        let text = std::fs::read_to_string(&path).unwrap();
        let start = text.find("\"goblin\": (").expect("the catalog has a goblin row");
        let at = start + text[start..].find("max_health: Some(").expect("the row states its HP");
        let end = at + text[at..].find(')').unwrap() + 1;
        std::fs::write(&path, format!("{}max_health: Some({EDITED}){}", &text[..at], &text[end..])).unwrap();

        let mut frames = 0;
        while goblin_max_hps(&mut sim).iter().any(|&hp| hp != EDITED) || goblin_max_hps(&mut sim).is_empty() {
            sim.step(common::base());
            frames += 1;
            assert!(
                frames < 600,
                "600 frames after the save the goblins have {:?}; reloads requested: {}",
                goblin_max_hps(&mut sim),
                sim.world().resource::<ContentSourceWatch>().requested
            );
        }
        assert_eq!(goblin_max_hps(&mut sim).len(), before.len(), "the same goblins, rebuilt");
        let catalog = sim.world().resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>();
        assert_eq!(
            catalog.get("goblin").and_then(|row| row.max_health),
            Some(EDITED),
            "the App's catalog is the published one"
        );
        // ⛔ AND THE CAST THE SESSION FROZE. Under the poison "the claim
        // carries no candidate cast" the session froze 5 while the App's
        // registry held 9. Before 2026-10-01 the goblins still had 9:
        // `apply_worn_character_gameplay` re-derived each body from the App
        // cast. It reads the generation's cast now (`worn_cast_for`), and the
        // same poison stops the loop above at [5, 5, 5].
        let frozen = sim
            .world()
            .resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
            .characters
            .as_ref()
            .and_then(|cast| cast.get("goblin"))
            .and_then(|definition| definition.vitals.max_health);
        assert_eq!(frozen, Some(EDITED), "the session froze the cast it was prepared from");
        eprintln!("a saved character row was played {frames} frames after the save");
    }));
    let _ = std::fs::remove_dir_all(&root);
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}

/// ⭐ A MOVEMENT-DEFAULTS EDIT SAVED WHILE THE GAME RUNS IS PLAYED, through the
/// developer-edit road (`MovementDefaultsWatch` writes the mirror the F3
/// inspector writes; the proposal is admitted and published in `PreUpdate`).
/// A save that does not parse is refused and the running tuning stays. The
/// watch is pointed at a copy, so the arm edits no file another test reads.
#[cfg(not(feature = "static_content"))]
#[test]
fn a_movement_tuning_saved_while_the_game_runs_is_played() {
    use ambition_app::app::movement_defaults_watch::MovementDefaultsWatch;
    use ambition_platformer2d::actors::assets::gameplay_defaults::PLATFORMER_DEFAULTS_FILE;
    use ambition_platformer2d::runtime::demo_fixture::ActiveMovementTuning;
    let mut sim = common::fixed_60hz_room_sim("proving_grounds");
    for _ in 0..40 {
        sim.step(common::base());
    }
    assert!(
        sim.world().get_resource::<MovementDefaultsWatch>().is_some(),
        "the premise: a build that reads the defaults off disk watches them"
    );
    let jump = |sim: &ambition_sim_harness::Platformer2dSimHarness| {
        sim.world().resource::<ActiveMovementTuning>().0.jump_speed
    };
    let shipped = std::fs::read_to_string(PLATFORMER_DEFAULTS_FILE).expect("the defaults file");
    assert_eq!(jump(&sim), 630.0, "the premise: the shipped jump speed is the one the edit changes");
    assert_eq!(shipped.matches("jump_speed: 630.0,").count(), 1, "the premise: the file states it once");

    let before = launch_speed(&mut sim);
    assert!(before > 0.0, "the premise: the player leaves the floor when it jumps");

    let dir = std::env::temp_dir().join(format!("ambition_movement_watch_{}", std::process::id()));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("platformer_defaults.ron");
        std::fs::write(&file, &shipped).unwrap();
        sim.world_mut().insert_resource(MovementDefaultsWatch::new(file.clone()));
        let applied = |sim: &ambition_sim_harness::Platformer2dSimHarness| {
            sim.world().resource::<MovementDefaultsWatch>().applied
        };

        // A file system's clock can be coarse: each save must look newer.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&file, "( this is not the defaults").unwrap();
        for _ in 0..45 {
            sim.step(common::base());
        }
        assert_eq!((applied(&sim), jump(&sim)), (0, 630.0), "a save that does not parse is refused");

        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&file, shipped.replace("jump_speed: 630.0,", "jump_speed: 700.0,")).unwrap();
        let mut frames = 0;
        while jump(&sim) != 700.0 {
            sim.step(common::base());
            frames += 1;
            assert!(frames < 120, "120 frames after the save the jump speed is {}", jump(&sim));
        }
        assert_eq!(applied(&sim), 1, "one save, one write");
        eprintln!("a saved movement tuning was played {frames} frames after the save");
        // ⭐ AND THE PLAYER JUMPS WITH IT: the resource alone would pass if
        // every body read an authored tuning of its own.
        let after = launch_speed(&mut sim);
        assert!(
            after > before * 1.05,
            "the player's jump launch did not follow the saved tuning: {before} before, {after} after"
        );
        eprintln!("the player's jump launch: {before} before the save, {after} after");
    }));
    let _ = std::fs::remove_dir_all(&dir);
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}

/// The primary player's fastest upward speed in the frames after a jump press,
/// from standing.
#[cfg(not(feature = "static_content"))]
fn launch_speed(sim: &mut ambition_sim_harness::Platformer2dSimHarness) -> f32 {
    use ambition_platformer2d::engine_core::BodyKinematics;
    use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
    let mut vertical = |sim: &mut ambition_sim_harness::Platformer2dSimHarness| {
        let world = sim.world_mut();
        let mut query = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        query.single(world).expect("the primary player").vel.y
    };
    // ⚠ A WALK FIRST. MEASURED 2026-10-01: an idle player in this room is hit
    // at frame 122 (60 -> 59 HP), and a press in the hitstun after it does not
    // launch. The walk moves the player out of that reach; the press then
    // launches at -555.
    for _ in 0..10 {
        sim.step(ambition_app::AgentAction { move_x: 1.0, right_pressed: true, ..common::base() });
    }
    for _ in 0..30 {
        sim.step(common::base());
    }
    let resting = vertical(sim);
    sim.step(ambition_app::AgentAction { jump: true, jump_held: true, ..common::base() });
    let mut peak: f32 = 0.0;
    for _ in 0..4 {
        sim.step(ambition_app::AgentAction { jump_held: true, ..common::base() });
        peak = peak.max((vertical(sim) - resting).abs());
    }
    peak
}
