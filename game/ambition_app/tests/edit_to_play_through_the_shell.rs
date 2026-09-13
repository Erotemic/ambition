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
//!    ["author"])` gives a body with 33 moves in the `author_*` vocabulary — and
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
/// EXIST and the re-preparation does not rebuild them, so
/// `activate_prepared_platformer_sessions` never constructs a session for the
/// edge to order against. The constraint is vacuously satisfied on this road,
/// and an arm cannot protect a constraint its road does not exercise.
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
