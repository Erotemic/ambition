//! An edit reaches the game the SHIPPED composition plays (`build_visible_app`).
//!
//! The crate-level arms in `ambition_content::reload` hand-build a world,
//! insert a `ShellRouter`, and add `publish_staged_reload_on_activation`
//! themselves, so each of them stays green if the shipped composition never
//! registers the system, never reaches an active route, or registers a route
//! with no preparation plan. The rules a reload follows (staleness, no-op
//! candidates, supersession, quest compatibility, the publication lease) are
//! tested there; these tests ask the real app the questions only it can
//! answer:
//!
//! - a content reload's families land on the activation frame, together, and
//!   a candidate refused by its preparation, its content graph or its
//!   activation gate publishes nothing;
//! - a session is prepared hidden beside the one that is playing, and a
//!   handoff, a supersession, a cancellation or a refusal leaves no candidate,
//!   receipt, scope or hold behind;
//! - the developer world reload (LDtk) rebuilds every live room or none, runs
//!   its effects only behind its verdict, and moves the bodies that stay out
//!   of new solids.
//!
//! ⚠ THE EDITS ARE BUILT IN MEMORY OR IN THE TEMPORARY DIRECTORY, NEVER IN THE
//! TREE: `compile_pack_with` makes an edited pack without touching
//! `game/ambition_content/assets`, and world edits are copies.
use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId, ShellRouter};

/// Level one's reaction latency, as the LIVE composition holds it.
fn live_first_rung(app: &bevy::prelude::App) -> f32 {
    app.world()
        .resource::<ambition_platformer2d::world::scoped_rules::DeclaredRules<
            ambition_platformer2d::characters::brain::fighter::FighterBrainLadder,
        >>()
        .governing(ambition_platformer2d::world::scoped_rules::ActiveRoom::Untagged)
        .expect("Ambition declares a ladder for its own rooms")
        .level(1)
        .expect("the shipped ladder has a level 1")
        .reaction_ms
}

/// The second goblin wave's second mob delay, as the LIVE composition holds it.
///
/// ⭐ THE THIRD FAMILY, READ THE SAME WAY THE SHIPPED READERS DO.
/// `EncounterWaveBook` is the App-owned resource `systems.rs` takes as
/// `Option<Res<..>>`; the reload republishes it, and it is a DIFFERENT family
/// from the ladder with a different source file.
fn live_second_goblin_delay(app: &bevy::prelude::App) -> f32 {
    app.world()
        .resource::<ambition_platformer2d::encounter::EncounterWaveBook>()
        .waves("goblin_encounter")
        .expect("the shipped book authors the goblin encounter")[1]
        .mobs[1]
        .delay
}

/// The epoch of the ONE prepared session in this world.
///
/// ⛔ NOT "THE FIRST `PreparedContentIdentity` FOUND", which
/// `ambition_platformer2d_rollback_ggrs` already records as a defect: a world with
/// two sessions would give a global read the wrong one. This asserts there is
/// exactly one and then reads it, so a second session makes the arm fail rather
/// than answer about a session it did not mean.
fn the_only_prepared_epoch(app: &mut bevy::prelude::App) -> u64 {
    // ⚠ THE STATE IS BUILT FIRST, THEN THE WORLD IS READ. Chaining
    // `.query(..).iter(app.world())` holds the mutable borrow across the
    // immutable one and does not compile.
    let mut state = app
        .world_mut()
        .query::<&ambition_platformer2d::runtime::PreparedContentIdentity>();
    // ⛔ EVERY MATCH, NOT EVERY DISTINCT EPOCH. Two sessions that happen to
    // share an epoch are still two sessions, and deduplicating would hide
    // exactly the case this assertion exists for.
    let found: Vec<u64> = state
        .iter(app.world())
        .map(|identity| identity.epoch.0)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one prepared session to read an epoch from, found \
         {found:?}"
    );
    found[0]
}

/// The shell's activation counter — a NEW id means the route re-activated.
/// Every distinct peer-stable CONTENT term the live world's construction stamps
/// carry, ignoring the roads that legitimately name no content.
///
/// ⛔ THE TERM, NOT `peer_stable_checksum`. The projection folds `content ⊗
/// room`, so any two ROOMS differ whatever their content term says — which is
/// how a content term collapsing to a constant hid inside a projection that
/// still behaved well. `TransactionId::peer_content_term` is the one rule that
/// decides which content two peers compare.
fn stamped_content_terms(app: &mut bevy::prelude::App) -> Vec<String> {
    let mut query = app
        .world_mut()
        .query::<&ambition_platformer2d::platformer::construction::TransactionId>();
    let mut out: Vec<String> = query
        .iter(app.world())
        .map(|stamp| stamp.peer_content_term().to_string())
        .filter(|term| term != "runtime-dynamic")
        .collect();
    out.sort();
    out.dedup();
    out
}

fn activation_id(app: &bevy::prelude::App) -> Option<u64> {
    app.world()
        .get_resource::<ShellRouter>()
        .and_then(|router| router.active.as_ref())
        .map(|active| active.activation_id.0)
}

/// Step until the content generation the arm requested has ended (activated,
/// refused or cancelled: each terminal path removes `PendingGeneration`), then
/// step the frames the old fixed loop stepped after the activation.
///
/// A fixed 240 frames is not a wait. In company the first activation was seen
/// on frame 83 against frame 3 alone (TEST-LANES item 3), so a generation can
/// still be pending at frame 240. An arm that then asserts "the refused
/// candidate did not activate" passes before the refusal is decided.
///
/// # Panics
///
/// When no generation is pending at the call (the arm's premise is gone), or
/// when one is still pending after `common::ACTIVATION_CEILING`.
fn step_until_the_generation_settles(app: &mut bevy::prelude::App) {
    use ambition_content::reload::PendingGeneration;
    assert!(
        app.world().contains_resource::<PendingGeneration>(),
        "no content generation is pending, so this arm judges a reload that was never requested or already ended"
    );
    let started = std::time::Instant::now();
    let mut frames = 0usize;
    while app.world().contains_resource::<PendingGeneration>() {
        assert!(
            started.elapsed() < crate::common::ACTIVATION_CEILING,
            "the generation is still pending after {frames} frames ({:?})",
            started.elapsed()
        );
        app.update();
        frames += 1;
    }
    for _ in 0..crate::common::SETTLE_FRAMES_AFTER_ACTIVATION {
        app.update();
    }
}

fn active_route(app: &bevy::prelude::App) -> Option<String> {
    app.world()
        .get_resource::<ShellRouter>()
        .and_then(|router| router.active.as_ref())
        .map(|active| active.route_id.as_str().to_string())
}

/// ⭐⭐ **ONE CANDIDATE EDITING EVERY PARTICIPATING FAMILY LANDS WHOLE, ON THE
/// ACTIVATION FRAME.** It edits the fighter ladder, the encounter waves, an SFX
/// cue, the goblin lab's adaptive cue binding, the lab's cutscene banner and
/// the first quest's title (with the player on that
/// quest's second step). Nothing moves at the request; on every frame before
/// the activation every family is N's; on the frame the shell activates, every
/// family is N+1's; and the engine's identities (selection, fingerprint, epoch,
/// construction provenance) name N+1 too. The new session's quest book is never
/// N's, and settled it is N+1's with the player's progress.
///
/// It watches the fighter ladder rather than a moveset: the body the shipped
/// `ambition_gameplay` route constructs carries only DERIVED KIT moves, and
/// the authored `moveset` pack names no player robot, so a moveset edit would
/// prove the transaction ran and show nothing the route plays.
/// `fighter_brain_ladder` is installed by `AmbitionContentPlugin::build` in this
/// composition and republished at the activation boundary.
#[test]
fn an_edited_pack_reaches_the_cast_the_shipped_composition_plays() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    // What `App::run` does before the first update — see
    // `authored_feel_reaches_the_prepared_cast` for why a guard that skips this
    // takes a different barrier road than production.
    app.finish();
    app.update();

    // ── navigate to a route that PREPARES ────────────────────────────────────
    // ⛔ THE BOOT ROUTE IS THE LAUNCHER AND IT PREPARES NOTHING. MEASURED: the
    // shipped catalog registers seven routes and `ambition_launcher` — the one
    // the app is active on after its first update — has no preparation plan, so
    // `request_reload` would answer `RouteHasNoPreparation` and this test would
    // pass while witnessing a refusal. A reload is only meaningful on a route
    // with a preparation barrier to re-run.
    assert_eq!(
        active_route(&app).as_deref(),
        Some("ambition_launcher"),
        "the premise: the shipped app boots into the launcher"
    );
    // ⛔⛤ **ASSERTED, NOT ASSUMED, BECAUSE IT IS A CLAIM ABOUT A REGISTRY AND
    // REGISTRIES GAIN ROWS.** This test's meaning rests on landing somewhere that
    // re-prepares: without a preparation plan `request_reload` answers
    // `RouteHasNoPreparation` and every assertion below would pass on a REFUSAL.
    // The outcome check further down would catch it, but it would name the wrong
    // cause — so the premise says which fact it depends on, here, where a route
    // change would break it.
    let gameplay = ShellRouteId::new("ambition_gameplay");
    assert!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::ShellRouteCatalog>()
            .get(&gameplay)
            .is_some_and(|spec| spec.preparation.is_some()),
        "`ambition_gameplay` no longer declares a preparation plan, so a reload \
         requested on it would be REFUSED and this witness would testify about a \
         refusal instead of a publication"
    );
    app.world_mut()
        .write_message(ShellCommand::ReplaceWith {
            route: gameplay,
            // ⚠ PLAIN NAVIGATION: this test drives the shell to a route that
            // prepares, and nothing here has to recognise the transaction. The
            // RELOAD's own correlator is minted inside `request_reload`.
            request: None,
        });
    crate::common::step_until_route_is_active(&mut app, "ambition_gameplay");
    let settled = active_route(&app);
    assert_eq!(
        settled.as_deref(),
        Some("ambition_gameplay"),
        "the shipped composition never activated its gameplay route, so nothing \
         below is about a reload"
    );

    advance_first_steps_one_step(&mut app);

    // ── the premise: every live family is the shipped one ───────────────────
    let sfx_before = first_sfx_frequency_of(&app);
    assert_eq!(lab_banner(&app), "// cutscene proof", "the premise: N's banner");
    assert_eq!(quest_place(&app, "first_steps").map(|place| place.0).as_deref(), Some("First Steps"), "the premise: N's quest title");
    #[cfg(feature = "audio")]
    assert_eq!(
        goblin_binding(&app),
        ("first_goblin_tune_v2".to_string(), "intro".to_string()),
        "the premise: N starts the goblin cue on its intro"
    );
    // Which families have moved to N+1: (ladder, waves, sfx, cutscene, cue).
    // The quest book is session state and is read on its own.
    let moved = |app: &bevy::prelude::App| {
        #[cfg(feature = "audio")]
        let cue = goblin_binding(app).1 == "wave1";
        #[cfg(not(feature = "audio"))]
        let cue = true;
        [
            live_first_rung(app) == 499.0,
            live_second_goblin_delay(app) == 0.75,
            first_sfx_frequency_of(app) == sfx_before + 1.0,
            lab_banner(app) == "// cutscene proof, revised",
            cue,
        ]
    };
    let before = live_first_rung(&app);
    assert_eq!(
        before, 500.0,
        "the premise: this composition installed the shipped ladder"
    );
    let waves_before = live_second_goblin_delay(&app);
    assert_eq!(
        waves_before, 0.70,
        "the premise: this composition installed the shipped encounter waves"
    );
    let cast_before = app
        .world()
        .resource::<ambition_platformer2d::character::PreparedCharacterRegistry>()
        .generation();
    let epoch_before = the_only_prepared_epoch(&mut app);
    // ⛔⛤ THE SIXTEENTH ID-PEER ROAD, TAKEN HERE BECAUSE THIS IS THE ONLY
    // FIXTURE THAT HOLDS TWO PREPARED FINGERPRINTS. See the assertion at the
    // end of this arm: every other construction-provenance guard asserts two
    // hosts AGREE, and an equality assertion is satisfied by every function that
    // throws information away.
    let provenance_before = stamped_content_terms(&mut app);

    // ── an edit, in memory, touching every participating family ─────────────
    // One commit, one frame, every family: the crate-level arms inject
    // `ShellEvent::RouteActivated` themselves, so atomicity across families is
    // only asked of a real boundary here. A transaction that published one
    // family and deferred another would build a world half of N and half of N+1.
    let mut edited = std::collections::BTreeMap::new();
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            let (from, to) = match declared {
                "data/fighter_brain_ladder.ron" => ("reaction_ms: 500.0", "reaction_ms: 499.0"),
                "data/encounters/goblin_encounter.ron" => ("delay: 0.70", "delay: 0.75"),
                "audio/sfx_registry.ron" => (
                    "frequency: 460.0, frequency_end: 720.0,",
                    "frequency: 461.0, frequency_end: 720.0,",
                ),
                "audio/music_cues.ron" => (r#"starting_state: "intro""#, r#"starting_state: "wave1""#),
                "data/cutscenes/sandbox.ron" => ("// cutscene proof", "// cutscene proof, revised"),
                "data/quests.ron" => (r#"title: "First Steps""#, r#"title: "First Steps, revised""#),
                _ => return text,
            };
            let out = text.replacen(from, to, 1);
            edited.insert(declared.to_string(), out != text);
            out
        })
        .expect("the edited pack compiles"),
    );
    // A floor on each edit separately: a retuned value or a renamed source
    // leaves one arm a no-op, and the families that still changed would carry
    // the test for it.
    let stale: Vec<&String> = edited.iter().filter(|(_, changed)| !**changed).map(|(source, _)| source).collect();
    assert!(
        edited.len() == 6 && stale.is_empty(),
        "these sources no longer carry the text the edit replaces, so their half of this test is vacuous: {stale:?} \
         (sources edited: {edited:?})"
    );
    let base = ambition_content::pack::selected(app.world())
        .expect("the composition selected a pack")
        .fingerprint;
    assert_ne!(
        base, candidate.fingerprint,
        "the premise: the candidate differs from what is live"
    );

    // ── the request, on the production road ──────────────────────────────────
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(
            std::sync::Arc::clone(&candidate),
            Some(base),
        ),
    );
    assert!(
        matches!(
            outcome,
            ambition_content::reload::ReloadRequest::Requested { .. }
        ),
        "the shipped composition refused the reload: {outcome:?}"
    );
    assert_eq!(
        moved(&app),
        [false; 5],
        "the REQUEST published (ladder, waves, sfx, cutscene, cue) on the spot instead of staging: the old \
         generation must stay authoritative until the new one activates"
    );
    assert_eq!(
        quest_place(&app, "first_steps").map(|place| place.0).as_deref(),
        Some("First Steps"),
        "⛔ the request changed the live quest book"
    );
    assert_eq!(
        ambition_content::pack::selected(app.world())
            .expect("a selection")
            .fingerprint,
        base,
        "the REQUEST installed the candidate as the App's selection, so the \
         engine would fingerprint against a pack that is not live"
    );

    // ── and the shell's own lifecycle carries it the rest of the way ─────────
    // Nothing is hand-driven: only `update()`, so the router mints the
    // transaction, the provider re-prepares, and the activation authorizes the
    // publication exactly as a running game does.
    //
    // ⛔ The families must land ON the activation frame. A test that waited for
    // the value passed while the publication arrived one frame after the world
    // was built from it: a publication that lands after the world is built
    // publishes into a world already built without it. This asserts the frame,
    // not the outcome.
    let activation_before = activation_id(&app);
    let mut activated_on = None;
    for frame in 0..240 {
        app.update();
        if activation_id(&app) != activation_before {
            activated_on = Some(frame);
            break;
        }
        assert_eq!(moved(&app), [false; 5], "frame {frame}: (ladder, waves, sfx, cutscene, cue) moved before the activation");
        assert_eq!(
            quest_place(&app, "first_steps").map(|place| place.0).as_deref(),
            Some("First Steps"),
            "frame {frame}: N's session lost its quest book before the activation"
        );
    }
    let activated_on = activated_on.expect(
        "the shipped shell never re-activated the route, so the reload's \
         transaction never reached its boundary",
    );
    assert_eq!(
        moved(&app),
        [true; 5],
        "⛔ ON THE ACTIVATION FRAME {activated_on} THE FAMILIES WERE NOT ALL N+1's (ladder, waves, sfx, cutscene, cue)"
    );
    assert_ne!(
        quest_place(&app, "first_steps").map(|place| place.0).as_deref(),
        Some("First Steps"),
        "⛔ frame {activated_on}: THE NEW SESSION SERVES N'S QUEST BOOK"
    );
    assert_eq!(
        live_first_rung(&app),
        499.0,
        "the route re-activated on frame {activated_on} and the ladder was still \
         generation N's AT THE END OF THAT FRAME. The world is constructed in \
         `GameplaySessionSet::Providers` on this same frame, so it was built \
         from content the transaction had not published yet — the N/N+1 split \
         this road exists to prevent. ⇒ A commit that arrives on frame+1 makes \
         this assertion fail while a 'publishes eventually' one would pass."
    );

    // ⛔⛔ **AND THE SECOND FAMILY ON THE SAME FRAME — ATOMICITY, NOT ORDER.**
    // Review finding 4 asked whether families #2 and #3 are actually atomic in
    // production; one commit, one frame, both values is the answer. A publisher
    // loop that broke after the first family would redden here and NOWHERE else.
    assert_eq!(
        live_second_goblin_delay(&app),
        0.75,
        "the ladder moved to generation N+1 on frame {activated_on} and the \
         encounter waves did not, so the world was built half from N and half \
         from N+1 — the transaction is not atomic across its families"
    );

    // ⛔⛔ **AND EVERY IDENTITY THE ENGINE CARRIES NAMES N+1 TOO.** The families
    // are the visible half; these four are what the rollback boundary, the
    // fingerprint comparison and the next candidate's staleness check read. A
    // reload that moved the content and left any of them naming N is a split
    // nobody would see until a peer disagreed about the world.
    assert_eq!(
        ambition_content::pack::selected(app.world())
            .expect("a selection")
            .fingerprint,
        candidate.fingerprint,
        "the families published but the App's SELECTION still names the old \
         pack, so the next candidate compares against a base that is not live"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::runtime::SelectedContentIdentity>()
            .0,
        format!(
            "{} {} {}",
            candidate.id, candidate.version, candidate.fingerprint
        ),
        "the ENGINE is still fingerprinting against generation N's pack"
    );
    // ⛔ The cast generation must NOT move: this candidate edits no move table,
    // so `request_reload` stages no cast and a family that did not change is
    // not re-published. A cast clock that advanced here would mean the
    // transaction re-publishes the whole cast for every edit.
    let cast_after = app
        .world()
        .resource::<ambition_platformer2d::character::PreparedCharacterRegistry>()
        .generation();
    assert_eq!(
        cast_after, cast_before,
        "a generation that changes NO move table still moved the cast, so every \
         edit re-publishes the whole cast"
    );
    // ⛔⛤ **AND THE ROOM THE RELOAD REBUILT WAS PUBLISHED, NOT REFUSED.**
    //
    // MEASURED 2026-09-13, and it was FALSE here: a census of every
    // room-construction refusal across the whole suite found six, and the
    // largest was this test's — `central_hub_complex` refused with **18x
    // Duplicated, 18x ReconstructedOldSurvived**, the entire room. A shell
    // handoff retires the old scope and activates the new one in one frame, and
    // `GameplaySessionSet::Providers` was ordered BEFORE
    // `SessionScopeSet::Cleanup`, so the incoming room's transaction captured a
    // baseline that still held all 18 of the outgoing scope's placements.
    //
    // ⚠ **IT COST ALMOST NOTHING VISIBLE, WHICH IS THE ONLY REASON IT SURVIVED —
    // AND THE "NO PRODUCTION READER" HALF OF THAT WAS WRONG, CORRECTED
    // 2026-09-14.** `RoomLoaded` has three production readers, all through
    // `ambition_combat::events::FreshAttempt`, so a refusal also left staged hits
    // unvoided and per-attempt state un-re-armed. Both are the right outcome (no
    // attempt began), which is why nobody noticed.
    //
    // ⛔ **AND SINCE THE CANDIDATE BRACKET WENT ON THE SAME REFUSAL DROPS
    // EVERY ROOT**, so this reload would land the player in an EMPTY WORLD. That
    // is no longer a future tense: it is what this arm keeps from coming back. It
    // asks the production verdict, not the schedule; the schedule is asked by
    // `the_retired_scopes_sweep_precedes_the_incoming_sessions_construction`.
    let verification = app
        .world()
        .resource::<ambition_platformer2d::publication::LastConstructionVerification>()
        .clone();
    assert!(
        verification.published,
        "the room the reload rebuilt (`{}`) was REFUSED with {} violation(s): {:?}. \
         Under the candidate bracket that drops the whole room, so the reload \
         lands the player in an empty world.",
        verification.room_id,
        verification.violations.len(),
        verification.violations,
    );

    // ⚠ THE EPOCH IS THE PREPARED WORLD'S, NOT THE PACK'S — it advances because
    // the session was re-prepared, which is what a rollback timeline contract
    // compares. Asserting it MOVED is the checkable claim; asserting a value
    // would pin an allocator's counter.
    let epoch_after = the_only_prepared_epoch(&mut app);
    assert_ne!(
        epoch_after, epoch_before,
        "the prepared session kept generation N's content epoch, so a rollback \
         timeline would accept N's snapshots into N+1's world"
    );

    // ⛔⛤ **AND THE CONSTRUCTION PROVENANCE MUST HAVE MOVED WITH THE CONTENT —
    // THE ONE ID-PEER ARM THAT IS A DISAGREEMENT RATHER THAN AN AGREEMENT.**
    //
    // `TransactionId::peer_stable_checksum` keeps WHICH CONTENT and WHICH ROOM
    // and drops the app-local epoch and the session stamp. Every other guard
    // over it asserts two hosts holding the SAME content project the same value
    // — and on 2026-09-16 three production roads were found to have collapsed
    // the content term to the constant `"content-unstated"`, which agrees with
    // itself perfectly. The defect made every agreement arm MORE true.
    //
    // ⇒ **AN EQUALITY ASSERTION `f(a) == f(b)` IS SATISFIED BY EVERY `f` THAT
    // THROWS INFORMATION AWAY, THE CONSTANT FUNCTION INCLUDED.** So the claim
    // needs its other half: content that DIFFERS must project differently.
    //
    // ⭐⭐ **AND THAT IS MEASURED HERE, NOT ARGUED.** Poison
    // `ContentBinding::canonical_summary` to render a STATED BUT CONSTANT
    // content term (`format!("{epoch}|pinned")`) and every one of
    // `id_peer_audit`'s SIX arms passes — including
    // `two_hosts_at_different_content_epochs_share_one_construction_provenance`,
    // whose whole subject this is. This assertion is the only thing in the
    // workspace that reddens. ⇒ The campaign's standing guard has no power over
    // a construction identity that stops discriminating, and this arm is where
    // that power lives.
    //
    // ⚠ ONE PROCESS AT TWO FINGERPRINTS, NOT TWO PEERS, AND THE DIFFERENCE IS
    // STATED RATHER THAN GLOSSED. A materially changed reload is the only road
    // in this workspace that puts two distinct prepared fingerprints inside one
    // run, and the projection excludes the epoch and the session — so the ONLY
    // term that may move here is the content one. That makes this a real test of
    // the projection's discriminating power; it is not a test of two hosts
    // agreeing, which needs the P2P session `N2` records as absent.
    let provenance_after = stamped_content_terms(&mut app);
    assert!(
        provenance_before.iter().all(|term| term != "content-unstated")
            && !provenance_before.is_empty(),
        "before the reload the live world's construction stamps named no prepared \
         content ({provenance_before:?}), so the comparison below has no working \
         reference and would pass on two collapsed constants"
    );
    assert!(
        provenance_after.iter().all(|term| term != "content-unstated")
            && !provenance_after.is_empty(),
        "after the reload the live world's construction stamps named no prepared \
         content ({provenance_after:?}) — the rebuilt roots lost the content \
         identity the reload published"
    );
    assert_ne!(
        provenance_before, provenance_after,
        "the reload moved the prepared fingerprint from {} to {} and every root's \
         construction provenance projected the SAME value \
         ({provenance_before:?}). So `TransactionId::peer_stable_checksum` cannot \
         tell two mechanically different worlds apart, and every arm asserting \
         that two hosts AGREE about it would still be green.",
        base, candidate.fingerprint
    );

    // The quest registry is session state: the new session's first tick fills
    // it from the selected pack and the save.
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        quest_place(&app, "first_steps"),
        Some(("First Steps, revised".to_string(), true, 1)),
        "the new session's quest book is not N+1's with the player's progress"
    );
}

/// The number of entities matching `F` including hidden candidates.
fn count_including_hidden<F: bevy::ecs::query::QueryFilter>(app: &mut bevy::prelude::App) -> usize {
    ambition_platformer2d::platformer::construction::count_matching_including_hidden_candidates::<F>(app.world_mut())
}

/// ⛔⛤ **A SESSION IS PREPARED HIDDEN BESIDE THE ONE THAT IS PLAYING, AND ONLY
/// ADMISSION MAKES IT LIVE**, asked of the shipped composition across its first
/// activation and a shell handoff (the live route replaced with itself, which
/// retires and activates on one frame).
///
/// `Q132`: there is exactly one canonical `SessionRoot`. `SessionWorldRef` /
/// `SessionWorldMut` are `Single<.., With<SessionRoot>>` and match nothing on a
/// frame with two, so every one of those systems would be skipped silently. A
/// replacement may be PREPARED while the current session is live, but it carries
/// `CandidateSessionRoot` until it is adopted. So on every frame the count of
/// `SessionRoot` INCLUDING hidden entities is at most one, and a candidate root
/// must be seen beside it (otherwise "at most one" says nothing about a
/// candidate).
///
/// On the first activation, every frame between the candidate's first room
/// publishing and the session starting, an ordinary query (no
/// `Allow<InactiveCandidate>`) sees nothing the session owns: the room's own
/// publication must not release the session's invisibility.
///
/// On the handoff, while N is still the live scope:
/// - N's simulation population (entities with a `SimId`) is unchanged: the
///   candidate's first room plans the same authored ids and must not declare
///   N's holders superseded;
/// - no `RoomLoaded` reaches the world, because `FreshAttempt` reads it and
///   production clears rollback-registered staged hits on it; the room's
///   lifecycle is held as frozen effects and paid at admission;
/// - `FactionRelations` (rollback-registered) keeps a stance set on N while a
///   candidate room is constructed;
/// - N's minted-item baseline does not learn a mint only the save describes.
///
/// After the handoff the incoming room was published, the world holds
/// authoritative identities and can be described (no identity held twice), no
/// candidate or receipt outlives the transaction, and the adopted session has
/// its `SessionMechanics` and moving platforms (the controls for
/// `a_candidate_session_whose_route_is_cancelled_is_discarded`).
///
/// It also checks, at build, that each cast of the shipped composition is
/// stated again at a sheet publication: a cast without a restater keeps a
/// scale of the old records while its bodies are built from the new ones (the
/// road itself: `ambition_platformer2d --features content_pack --test
/// sheet_derived_cast`).
#[test]
fn the_shipped_app_never_holds_two_session_roots_across_a_handoff() {
    use ambition_platformer2d::actor::ActorFaction;
    use ambition_platformer2d::combat::targeting::FactionRelations;
    use ambition_platformer2d::item_runtime::pickup::minted_horizon::MintedItemBaseline;
    use ambition_platformer2d::platformer::lifecycle::{
        session_world_component, session_world_component_mut, CandidateSessionRoot,
        SessionRoot, SessionScopeId, SessionScopedEntity,
    };
    use ambition_platformer2d::platformer::sim_id::SimId;
    use bevy::prelude::With;

    fn population(app: &mut bevy::prelude::App, owner: SessionScopeId) -> std::collections::BTreeSet<SimId> {
        let world = app.world_mut();
        world
            .query::<(&SessionScopedEntity, &SimId)>()
            .iter(world)
            .filter(|(scoped, _)| scoped.0 == owner)
            .map(|(_, id)| id.clone())
            .collect()
    }

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    {
        use ambition_platformer2d::characters::actor::character_catalog::CHARACTER_CATALOG_SCHEMA;
        let mut expected = vec!["the Ambition cast".to_string()];
        expected.extend(
            app.world()
                .resource::<ambition_platformer2d::content::live_reload::EmbeddedPackWatches>()
                .0
                .iter()
                .map(|watch| watch.pack().prepared())
                .filter(|pack| pack.schemas.keys().any(|schema| schema.0 == CHARACTER_CATALOG_SCHEMA))
                .map(|pack| format!("the cast of `{}`", pack.id.0)),
        );
        assert!(expected.len() > 2, "the premise: the shipped composition watches the demo casts: {expected:?}");
        let restaters = app.world().resource::<ambition_platformer2d::content::sprite_reload::SheetDerivedRestaters>();
        let names: Vec<&str> = restaters.names().collect();
        for cast in &expected {
            assert!(names.contains(&cast.as_str()), "{cast} is not stated again at a sheet publication; restated: {names:?}");
        }
    }
    // `Last`: the room publishes from a command flush inside the frame, and an
    // `Update` reader registered here would see the message only on the next
    // frame, after the window has closed.
    app.insert_resource(RoomLoadsSeen::default());
    app.add_systems(bevy::prelude::Last, count_room_loads);
    app.finish();
    app.update();

    let gameplay = ShellRouteId::new("ambition_gameplay");
    app.world_mut().write_message(ShellCommand::ReplaceWith { route: gameplay.clone(), request: None });

    // ── the first activation ─────────────────────────────────────────────────
    let mut canonical_high_water = (0usize, 0usize);
    let mut saw_canonical = false;
    let mut candidate_frames = 0usize;
    let mut room_published_at = None;
    let mut session_started_at = None;
    let mut seen_early: Option<(usize, usize)> = None;
    let mut window = crate::common::ActivationWindow::new("ambition_gameplay");
    for frame in 0.. {
        if window.closed(&app) {
            break;
        }
        app.update();
        let canonical = count_including_hidden::<With<SessionRoot>>(&mut app);
        if canonical > canonical_high_water.0 {
            canonical_high_water = (canonical, frame);
        }
        saw_canonical |= canonical == 1;
        if count_including_hidden::<With<CandidateSessionRoot>>(&mut app) > 0 {
            candidate_frames += 1;
        }
        if room_published_at.is_none()
            && app
                .world()
                .get_resource::<ambition_platformer2d::publication::LastConstructionVerification>()
                .is_some_and(|verification| verification.published)
        {
            room_published_at = Some(frame);
        }
        if session_started_at.is_none() && live_scope(&app).is_some() {
            session_started_at = Some(frame);
        }
        if session_started_at.is_none() {
            let visible = {
                let world = app.world_mut();
                world.query::<&SessionScopedEntity>().iter(world).count()
            };
            if visible > 0 && seen_early.is_none() {
                seen_early = Some((frame, visible));
            }
        }
    }
    assert!(saw_canonical, "no canonical session root appeared in the first activation, so it activated nothing");
    assert!(
        candidate_frames > 0,
        "no frame of the first activation held a `CandidateSessionRoot`: the candidate road did not run, or the \
         candidate carries `SessionRoot` and is being counted as the canonical root"
    );
    let (published_at, started_at) = (
        room_published_at.expect("the candidate's first room never published"),
        session_started_at.expect("the session never started"),
    );
    assert!(
        started_at > published_at,
        "the room published at frame {published_at} and the session started at {started_at}: there is no interval \
         between the inner verdict and the outer one to judge"
    );
    assert_eq!(
        seen_early, None,
        "⛔ AN INNER PUBLICATION RELEASED THE CANDIDATE SESSION'S INVISIBILITY: ordinary queries saw (frame, count) \
         session-owned entities before the session started (room published at {published_at}, session started at \
         {started_at})"
    );

    // ── world N, with state a candidate must not touch ──────────────────────
    let live = live_scope(&app).expect("the first session owns a scope");
    let before = population(&mut app, live);
    assert!(!before.is_empty(), "the premise: world N is a populated session");
    app.world_mut().resource_mut::<FactionRelations>().set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
    const MINTED: &str = "minted:a10_candidate_horizon_mint";
    let minted = SimId::from_snapshot(MINTED.to_string());
    session_world_component_mut::<MintedItemBaseline>(app.world_mut()).unwrap().adopt(Default::default());
    app.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .0
        .set_minted_items(vec![ambition_platformer2d::persistence::save_data::PersistedMintedItem {
            occurrence: MINTED.to_string(),
            parent: "placement:ground_gun_sword".to_string(),
            sequence: 1,
            held_item: "ground_gun_sword".to_string(),
        }]);
    app.update();
    assert!(
        app.world().resource::<FactionRelations>().is_hostile(ActorFaction::Enemy, ActorFaction::Boss),
        "the premise: the stance set on N did not take"
    );
    assert!(
        session_world_component::<MintedItemBaseline>(app.world()).unwrap().description_of(&minted).is_none(),
        "the premise: N's minted baseline does not describe the mint"
    );
    let loads_before = app.world().resource::<RoomLoadsSeen>().0;
    let activation_before = activation_id(&app);

    // ── the handoff ──────────────────────────────────────────────────────────
    app.world_mut().write_message(ShellCommand::ReplaceWith { route: gameplay, request: None });
    let mut handoff_roots = (0usize, 0usize);
    let mut handoff_candidates = 0usize;
    let mut lost = std::collections::BTreeSet::new();
    let mut window_frames = 0usize;
    let mut frozen_high_water = 0usize;
    let mut loads_in_window = 0usize;
    let mut stance_frames = 0usize;
    let mut stance_survived = true;
    let mut minted_leaked = false;
    let started = std::time::Instant::now();
    let mut frame = 0usize;
    while activation_id(&app) == activation_before {
        assert!(
            started.elapsed() < crate::common::ACTIVATION_CEILING,
            "the handoff did not activate in {frame} frames ({:?})",
            started.elapsed()
        );
        app.update();
        frame += 1;
        let roots = count_including_hidden::<With<SessionRoot>>(&mut app);
        if roots > handoff_roots.0 {
            handoff_roots = (roots, frame);
        }
        if count_including_hidden::<With<CandidateSessionRoot>>(&mut app) > 0 {
            handoff_candidates += 1;
        }
        if live_scope(&app) != Some(live) {
            continue;
        }
        window_frames += 1;
        lost.extend(before.difference(&population(&mut app, live)).cloned());
        frozen_high_water = frozen_high_water
            .max(ambition_platformer2d::publication::publications_holding_frozen_effects(app.world_mut()));
        loads_in_window = app.world().resource::<RoomLoadsSeen>().0 - loads_before;
        if ambition_platformer2d::platformer::construction::outstanding_candidates(app.world_mut()) > 0 {
            stance_frames += 1;
            stance_survived &= app.world().resource::<FactionRelations>().is_hostile(ActorFaction::Enemy, ActorFaction::Boss);
        }
        minted_leaked |= session_world_component::<MintedItemBaseline>(app.world())
            .is_some_and(|baseline| baseline.description_of(&minted).is_some());
    }
    for _ in 0..crate::common::SETTLE_FRAMES_AFTER_ACTIVATION {
        app.update();
    }

    assert!(
        handoff_roots.0 <= 1 && canonical_high_water.0 <= 1,
        "THE SHIPPED APP HELD {} `SessionRoot`s counting hidden entities on frame {} of the first activation and {} \
         on frame {} of the handoff. `Single<.., With<SessionRoot>>` matches nothing on such a frame",
        canonical_high_water.0,
        canonical_high_water.1,
        handoff_roots.0,
        handoff_roots.1
    );
    assert!(handoff_candidates > 0, "no frame of the handoff held a `CandidateSessionRoot`");
    assert!(window_frames > 0, "the handoff activated on its first frame: no frame had a candidate beside the live session");
    assert!(
        lost.is_empty(),
        "⛔ THE CANDIDATE SESSION TOOK ENTITIES OUT OF THE WORLD THAT WAS STILL PLAYING: {lost:?}"
    );
    assert_eq!(
        loads_in_window, 0,
        "⛔ A ROOM PUBLISHED INSIDE A PENDING CANDIDATE SESSION ANNOUNCED ITSELF TO THE LIVE ONE: {loads_in_window} \
         `RoomLoaded` across {window_frames} frames while N was live"
    );
    // After the count above, so a reverted deferral fails on the count and not
    // on this premise.
    assert!(
        frozen_high_water > 0,
        "no publication held frozen effects while the candidate was pending, so the zero above says nothing about the deferral"
    );
    assert!(stance_frames > 0, "no candidate was outstanding in the handoff, so nothing was constructed beside N");
    assert!(stance_survived, "⛔ CONSTRUCTING A CANDIDATE ROOM RESET THE LIVE SESSION'S `FactionRelations`");
    assert!(
        !minted_leaked,
        "⛔ PREPARING A CANDIDATE INSTALLED ITS MINTED BASELINE OVER THE PLAYING SESSION'S"
    );

    // ── after the handoff ────────────────────────────────────────────────────
    assert_ne!(live_scope(&app), Some(live), "the handoff did not move the live scope");
    assert!(
        app.world().resource::<RoomLoadsSeen>().0 > loads_before,
        "⛔ THE DEFERRED ROOM LIFECYCLE WAS NEVER PAID: the admitted session's first room never announced itself"
    );
    assert_eq!(
        ambition_platformer2d::publication::publications_holding_frozen_effects(app.world_mut()),
        0,
        "a publication still holds frozen effects after its session was admitted"
    );
    let verification =
        app.world().resource::<ambition_platformer2d::publication::LastConstructionVerification>().clone();
    assert!(
        verification.published,
        "⛔ THE INCOMING SESSION'S ROOM WAS REFUSED: its roots are dropped and the handoff lands in an empty world: {verification:?}"
    );
    let roster = {
        let world = app.world_mut();
        world.query::<&SimId>().iter(world).count()
    };
    assert!(roster > 0, "the handoff published a room and the world holds no authoritative identities: {verification:?}");
    assert!(
        ambition_platformer2d::platformer::construction::TransactionBaseline::capture(app.world_mut()).is_ok(),
        "⛔ THE WORLD THE HANDOFF PUBLISHED CANNOT BE DESCRIBED: some identity has two authoritative holders, so the \
         outgoing session's world still stands beside the incoming one"
    );
    assert_eq!(
        ambition_platformer2d::platformer::construction::outstanding_candidates(app.world_mut()),
        0,
        "⛔ A HIDDEN CANDIDATE OUTLIVED THE HANDOFF"
    );
    assert_eq!(
        ambition_platformer2d::publication::outstanding_publications(app.world_mut()),
        0,
        "a publication receipt is still standing after the handoff settled"
    );
    assert!(
        app.world().get_resource::<ambition_platformer2d::construction::mechanics::SessionMechanics>().is_some(),
        "an adopted session has no `SessionMechanics`"
    );
    assert!(
        !ambition_platformer2d::session::sole_live_room_component::<ambition_platformer2d::world::collision::MovingPlatformSet>(app.world())
            .expect("the live room has moving platforms")
            .0
            .is_empty(),
        "an adopted session installed no moving platforms"
    );
}

/// ⭐ A RELOAD WHOSE BOUNDARY CLOSES WHILE IT WAITS IS CANCELLED WHOLE.
///
/// A content reload is legal against a timeline that is healthy and that this
/// host may rebase (`Q118`). Two owners ask again after the request: the lease
/// breaker (`break_the_publication_lease_when_the_boundary_closes`) on each
/// frame from the adoption, cancelling on the frame after the boundary closes;
/// and the activation gate (`answer_the_publication_gate`) on each frame the
/// route is ready but for its holds. Each owner's rules are tested in
/// `ambition_content::reload` (`reload_tests.rs`, the "mid flight" arms); this
/// asks the shipped shell, with its real load barrier and holds, for the
/// arms only the two together decide:
///
/// - a slow preparation with the boundary open publishes (the control);
/// - the boundary closes after the route is ready (an unhealthy authority):
///   cancelled on the next frame;
/// - the boundary closes while the route prepares and opens again before it is
///   ready: the gate alone would publish, so only the breaker can cancel this,
///   which makes the admission a lease for the life of the transaction.
///
/// "Cancelled" is: the route is not activated, the content is the old
/// generation's, nothing is staged, and the transaction's holds are released.
///
/// The fixture: a reload of the fighter ladder (500 ms to 499 ms), with a hold
/// that has no gate on the route, as the loading screen has, released 12
/// frames after the route is ready. A slow preparation keeps the route's
/// barrier not ready for 8 frames after the adoption.
#[test]
fn a_reload_whose_boundary_closes_while_it_waits_is_cancelled_whole() {
    use ambition_platformer2d::game_shell::{ShellHoldId, ShellRouteHolds};
    use ambition_platformer2d::rollback::{
        mechanical_mutation_boundary, ActiveRollbackAuthority, MechanicalMutationBoundary,
        RollbackSessionOwnership,
    };

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Kind {
        /// A timeline that this host may not rebase.
        Foreign,
        /// The authority records a divergence.
        Unhealthy,
    }
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Arm {
        /// Control: a slow preparation, the boundary open.
        SlowNever,
        /// Closed 3 frames after the route is ready, to the end.
        WhenReady(Kind),
        /// A slow preparation, closed 2 frames after the adoption.
        WhilePreparing { opens_again: bool },
    }
    #[derive(Debug, PartialEq)]
    enum End {
        Published,
        /// With the frames from the close to the end of the transaction.
        Cancelled(u32),
    }

    fn pending(app: &bevy::prelude::App) -> bool {
        app.world().resource::<ShellRouter>().pending.is_some()
    }
    fn ready(app: &bevy::prelude::App) -> bool {
        let world = app.world();
        world.resource::<ShellRouter>().ready_but_for_holds(
            world.resource::<ambition_platformer2d::load::LoadCoordinator>(),
            world.resource::<ambition_platformer2d::game_shell::PreparedSessionRegistry>(),
        )
    }
    fn close(app: &mut bevy::prelude::App, kind: Kind) {
        match kind {
            Kind::Foreign => {
                app.world_mut().insert_resource(RollbackSessionOwnership::External);
            }
            Kind::Unhealthy => app
                .world_mut()
                .resource_mut::<ActiveRollbackAuthority>()
                .invalidate("a test divergence".to_owned()),
        }
    }

    let route = ShellRouteId::new("ambition_gameplay");
    let screen = ShellHoldId::new("test:loading-screen");
    let mut wrong: Vec<String> = Vec::new();
    for (arm, expected) in [
        (Arm::SlowNever, End::Published),
        (Arm::WhenReady(Kind::Unhealthy), End::Cancelled(1)),
        (Arm::WhilePreparing { opens_again: true }, End::Cancelled(1)),
    ] {
        let mut app = a_running_shipped_session();
        // ⛔ THE PREMISE: the shipped ladder, and a boundary that is open
        // because this host may rebase its own timeline.
        assert_eq!(live_first_rung(&app), 500.0, "{arm:?}: the shipped ladder is not live");
        assert_eq!(
            mechanical_mutation_boundary(app.world()),
            MechanicalMutationBoundary::LocallyRebasable,
            "{arm:?}: the boundary is not open before the reload"
        );
        let activation = activation_id(&app);
        let owned = *app.world().resource::<RollbackSessionOwnership>();

        app.world_mut()
            .resource_mut::<ShellRouteHolds>()
            .hold(route.clone(), screen.clone());
        let candidate = std::sync::Arc::new(
            ambition_content::pack::compile_pack_with(|declared, text| match declared {
                "data/fighter_brain_ladder.ron" => {
                    text.replacen("reaction_ms: 500.0", "reaction_ms: 499.0", 1)
                }
                _ => text,
            })
            .expect("the edited pack compiles"),
        );
        let base = ambition_content::pack::selected(app.world())
            .expect("the composition selected a pack")
            .fingerprint;
        let outcome = ambition_content::reload::request_reload(
            app.world_mut(),
            ambition_content::CandidateGeneration::prepared_against(candidate, Some(base)),
        );
        assert!(
            matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }),
            "{arm:?}: the reload was refused when it was asked for: {outcome:?}"
        );

        let slow = matches!(arm, Arm::SlowNever | Arm::WhilePreparing { .. });
        let (mut adopted_at, mut ready_at, mut closed_at, mut ended_at) = (None, None, None, None);
        for frame in 0..120u32 {
            app.update();
            let adopted = app
                .world()
                .resource::<ShellRouteHolds>()
                .held(&route)
                .iter()
                .any(|hold| hold.as_str().starts_with("content-publication:"));
            if adopted && adopted_at.is_none() {
                adopted_at = Some(frame);
            }
            if ready(&app) && ready_at.is_none() {
                ready_at = Some(frame);
            }
            // The slow preparation: the barrier stays not ready for 8 frames.
            if slow && adopted_at.is_some_and(|at| frame <= at + 8) {
                let barrier = app
                    .world()
                    .resource::<ShellRouter>()
                    .pending
                    .as_ref()
                    .map(|pending| pending.barrier.clone());
                if let Some(barrier) = barrier {
                    app.world_mut()
                        .resource_mut::<ambition_platformer2d::load::LoadCoordinator>()
                        .apply(ambition_platformer2d::load::LoadCommand::SetDiscovery {
                            load_id: barrier.load_id,
                            barrier_id: barrier.barrier_id,
                            open: adopted_at.is_some_and(|at| frame < at + 8),
                            forecast: None,
                        });
                }
            }
            match arm {
                Arm::WhenReady(kind) if ready_at.is_some_and(|at| frame == at + 3) => {
                    close(&mut app, kind);
                    closed_at = Some(frame);
                }
                Arm::WhilePreparing { opens_again } => {
                    if adopted_at.is_some_and(|at| frame == at + 2) {
                        close(&mut app, Kind::Foreign);
                        closed_at = Some(frame);
                    }
                    if opens_again && closed_at.is_some_and(|at| frame == at + 2) {
                        app.world_mut().insert_resource(owned);
                    }
                }
                _ => {}
            }
            if ready_at.is_some_and(|at| frame == at + 12) {
                app.world_mut()
                    .resource_mut::<ShellRouteHolds>()
                    .release(&route, &screen);
            }
            if adopted_at.is_some() && !pending(&app) && ended_at.is_none() {
                ended_at = Some(frame);
            }
            if ended_at.is_some_and(|at| frame >= at + 5) {
                break;
            }
        }
        // ⛔ THE PREMISES OF THE FIXTURE.
        assert!(adopted_at.is_some() && ended_at.is_some(), "{arm:?}: the transaction did not end");
        if arm == Arm::SlowNever {
            assert!(
                ready_at.zip(adopted_at).is_some_and(|(ready, adopted)| ready >= adopted + 8),
                "the slow preparation was ready at {ready_at:?}, adopted at {adopted_at:?}: \
                 no frame is between the adoption and the ready route"
            );
        }

        let activated = activation_id(&app) != activation;
        let staged =
            app.world().contains_resource::<ambition_content::reload::PendingGeneration>();
        let holds = app.world().resource::<ShellRouteHolds>().held(&route);
        let state = (activated, live_first_rung(&app), staged);
        let end = match (state, closed_at.zip(ended_at)) {
            ((true, rung, false), _) if rung == 499.0 && holds.is_empty() => Some(End::Published),
            ((false, rung, false), Some((closed, ended)))
                if rung == 500.0 && holds == vec![screen.clone()] =>
            {
                Some(End::Cancelled(ended - closed))
            }
            _ => None,
        };
        if end.as_ref() != Some(&expected) {
            wrong.push(format!(
                "{arm:?}: {expected:?} is correct. The reload ended as {end:?}: (activated, \
                 rung, staged) = {state:?}, holds {holds:?}, closed at {closed_at:?}, ended \
                 at {ended_at:?}, ready at {ready_at:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The shipped app, gameplay activated, ready to be told to reload its world.
pub(crate) fn a_running_shipped_session() -> bevy::prelude::App {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: ShellRouteId::new("ambition_gameplay"),
        request: None,
    });
    crate::common::step_until_route_is_active_and_settled(&mut app, "ambition_gameplay");
    app
}

/// The developer overlay's transition flash: `1.0` is written by a committed
/// world reload and by nothing else in an idle frame.
fn dev_preset_flash(app: &bevy::prelude::App) -> f32 {
    app.world()
        .resource::<ambition_platformer2d::dev_tools::DeveloperRuntimeState>()
        .preset_flash
}

pub(crate) fn press_apply_reload(app: &mut bevy::prelude::App) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::dev_tools::DeveloperRuntimeState>()
        .preset_flash = 0.0;
    app.world_mut()
        .write_message(ambition_platformer2d::platformer::developer_hotkeys::DeveloperAction::ApplyLdtkReload);
    app.update();
}

/// What each live room is made of, by room id: its live instance, and the
/// distinct content terms of the entities that are stamped with it.
///
/// A room that a reload rebuilt has a new instance and the terms of the new
/// generation. A room that a reload did not rebuild keeps the two.
fn live_room_readings(
    app: &mut bevy::prelude::App,
) -> std::collections::BTreeMap<
    String,
    (
        ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
        std::collections::BTreeSet<String>,
    ),
> {
    use ambition_platformer2d::platformer::lifecycle::{
        InRoomInstance, LiveRoomInstance, RoomInstanceRoot,
    };
    let world = app.world_mut();
    let roots: Vec<_> = world
        .query_filtered::<
            (&LiveRoomInstance, &ambition_platformer2d::world::rooms::LiveRoomDefinition),
            bevy::prelude::With<RoomInstanceRoot>,
        >()
        .iter(world)
        .map(|(live, definition)| (*live, *definition))
        .collect();
    let stamps: Vec<(LiveRoomInstance, String)> = world
        .query::<(
            &InRoomInstance,
            &ambition_platformer2d::platformer::construction::TransactionId,
        )>()
        .iter(world)
        .map(|(room, stamp)| (room.0, stamp.peer_content_term().to_string()))
        .filter(|(_, term)| term != "runtime-dynamic")
        .collect();
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(world)
    .expect("the session keeps its room set");
    roots
        .into_iter()
        .map(|(live, definition)| {
            let terms = stamps
                .iter()
                .filter(|(room, _)| *room == live)
                .map(|(_, term)| term.clone())
                .collect();
            (rooms.spec(definition).id.clone(), (live, terms))
        })
        .collect()
}

/// The changed copy of the world file that a reload arm watches. The file is
/// removed when this is dropped, also when the arm fails.
struct EditedWorldCopy(std::path::PathBuf);

impl Drop for EditedWorldCopy {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// A copy of the watched world file, changed by `edit`, in the temporary
/// directory. The reload is pointed at it. ⚠ Nothing in the tree is written.
#[must_use = "the copy is removed when the value is dropped: keep it until the reload was pressed"]
fn watch_an_edited_copy(
    app: &mut bevy::prelude::App,
    tag: &str,
    edit: impl FnOnce(&mut serde_json::Value),
) -> EditedWorldCopy {
    use ambition_platformer2d::dev_tools::WorldSourceHotReload;
    let source = app
        .world()
        .resource::<WorldSourceHotReload>()
        .watch_path
        .clone()
        .expect("the shipped session watches a world file");
    let text = std::fs::read_to_string(&source).expect("the watched world file is readable");
    let mut project: serde_json::Value =
        serde_json::from_str(&text).expect("the watched world file is JSON");
    edit(&mut project);
    let copy = std::env::temp_dir().join(format!(
        "ambition_world_reload_{}_{tag}.ldtk",
        std::process::id()
    ));
    std::fs::write(&copy, serde_json::to_string(&project).expect("the project serializes"))
        .expect("the temporary directory is writable");
    app.world_mut().resource_mut::<WorldSourceHotReload>().watch_path = Some(copy.clone());
    EditedWorldCopy(copy)
}

/// Move the first `entity` of the level whose `activeArea` is `room` by 16 px
/// in x: a change of the authored world that keeps it valid.
fn move_an_entity(project: &mut serde_json::Value, room: &str, entity: &str) {
    let levels = project["levels"].as_array_mut().expect("the project has levels");
    for level in levels {
        let in_room = level["fieldInstances"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|field| field["__identifier"] == "activeArea" && field["__value"] == room);
        if !in_room {
            continue;
        }
        for layer in level["layerInstances"].as_array_mut().into_iter().flatten() {
            for instance in layer["entityInstances"].as_array_mut().into_iter().flatten() {
                if instance["__identifier"] == entity {
                    let x = instance["px"][0].as_i64().expect("an entity has a px");
                    instance["px"][0] = serde_json::json!(x + 16);
                    if let Some(world_x) = instance["__worldX"].as_i64() {
                        instance["__worldX"] = serde_json::json!(world_x + 16);
                    }
                    return;
                }
            }
        }
    }
    panic!("the project has no `{entity}` in a level of `{room}`");
}

/// The second live room of the two-room reload arms. It has authored
/// content that construction stamps, so a rebuilt room and a kept room read
/// differently (`tiny_chamber` has none: its reading is an empty set).
const SECOND_LIVE_ROOM: &str = "basement_npcs";

/// Which of the two bodies leaves the start room. Alice is the primary seat,
/// so the room she is in is the room a reload publishes first.
#[derive(Clone, Copy, Debug)]
enum Leaver {
    Alice,
    Bob,
}

/// The shipped session with Alice and Bob (driven by slot 1) in two live
/// rooms: one stays in the start room and `leaver` is in `second`. Returns
/// the id of the start room.
fn a_running_shipped_session_with_two_live_rooms(
    leaver: Leaver,
    second: &str,
) -> (bevy::prelude::App, String) {
    use crate::neighbor_prefetch_prepares_rooms::{
        alice, bob, cross, live_room_ids, put_bob_beside_alice,
    };
    use ambition_platformer2d::characters::control::PlayerSlot;

    let mut app = a_running_shipped_session();
    let start = put_bob_beside_alice(&mut app);
    match leaver {
        Leaver::Alice => {
            let body = alice(&mut app);
            cross(&mut app, body, PlayerSlot(0), second);
        }
        Leaver::Bob => {
            let body = bob(&mut app).expect("Bob is in the world");
            cross(&mut app, body, PlayerSlot(1), second);
        }
    }
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        live_room_ids(&mut app),
        vec![start.clone(), second.to_owned()],
        "{leaver:?}'s crossing did not leave two live rooms"
    );
    (app, start)
}

/// Make solid each `Collision` cell of the level of `room` that the box
/// (`center`, `half`) touches, and one cell around it.
fn put_a_solid_over(
    project: &mut serde_json::Value,
    room: &str,
    center: ambition_platformer2d::engine_core::Vec2,
    half: ambition_platformer2d::engine_core::Vec2,
) {
    let levels = project["levels"].as_array_mut().expect("the project has levels");
    for level in levels {
        let in_room = level["fieldInstances"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|field| field["__identifier"] == "activeArea" && field["__value"] == room);
        if !in_room {
            continue;
        }
        for layer in level["layerInstances"].as_array_mut().into_iter().flatten() {
            if layer["__identifier"] != "Collision" {
                continue;
            }
            let grid = layer["__gridSize"].as_f64().expect("a layer has a grid size") as f32;
            let wide = layer["__cWid"].as_i64().expect("a layer has a width");
            let high = layer["__cHei"].as_i64().expect("a layer has a height");
            let cells = layer["intGridCsv"].as_array_mut().expect("Collision is an IntGrid");
            let cell = |px: f32| (px / grid).floor() as i64;
            let mut made = 0;
            for y in (cell(center.y - half.y) - 1).max(0)..=(cell(center.y + half.y) + 1).min(high - 1) {
                for x in (cell(center.x - half.x) - 1).max(0)..=(cell(center.x + half.x) + 1).min(wide - 1) {
                    cells[(y * wide + x) as usize] = serde_json::json!(1);
                    made += 1;
                }
            }
            assert!(made > 0, "the box {center:?} is not in the level of `{room}`");
            return;
        }
    }
    panic!("the project has no `Collision` layer in a level of `{room}`");
}

/// Where `body` is, and whether a body of its size is clear of the solids of
/// its live room there. `None`: the body is not in the world, or in no live
/// room.
fn place_of(
    app: &bevy::prelude::App,
    body: bevy::prelude::Entity,
) -> Option<(ambition_platformer2d::engine_core::Vec2, bool)> {
    let kinematics = app
        .world()
        .get_entity(body)
        .ok()?
        .get::<ambition_platformer2d::engine_core::BodyKinematics>()?;
    let spec = ambition_platformer2d::world::rooms::live_room_spec_of(app.world(), body)?;
    let clear = ambition_platformer2d::world::rooms::validated_spawn(
        &spec.world,
        kinematics.pos,
        kinematics.size(),
    );
    Some((kinematics.pos, clear.distance(kinematics.pos) < 0.01))
}

/// A character id that no catalog has.
const STRANGER: &str = "a_character_nobody_registered";

/// Point the reload back at the world file the session watched at its start.
fn watch_the_source_again(app: &mut bevy::prelude::App, source: &std::path::Path) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::dev_tools::WorldSourceHotReload>()
        .watch_path = Some(source.to_path_buf());
}

fn watched_source(app: &bevy::prelude::App) -> std::path::PathBuf {
    app.world()
        .resource::<ambition_platformer2d::dev_tools::WorldSourceHotReload>()
        .watch_path
        .clone()
        .expect("the shipped session watches a world file")
}

/// Seat 1 joins through the join system, run once with a Start press in its
/// slot (the shipped composition opens one handle, so no device presses for
/// it). Answers the seat's home body.
fn join_seat_one(app: &mut bevy::prelude::App) -> bevy::prelude::Entity {
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot, SlotControls};
    use ambition_platformer2d::platformer::markers::PlayerEntity;
    use bevy::ecs::system::RunSystemOnce as _;
    app.world_mut().resource_mut::<SlotControls>().set(
        PlayerSlot(1),
        ambition_platformer2d::engine_core::ControlFrame { start_pressed: true, ..Default::default() },
    );
    app.world_mut()
        .run_system_once(ambition_platformer2d::session::join::seat_a_joining_participant)
        .expect("the join system runs");
    // No handle writes slot 1 in this composition, so the press would stay.
    app.world_mut().resource_mut::<SlotControls>().set(PlayerSlot(1), Default::default());
    let world = app.world_mut();
    world
        .query_filtered::<(bevy::prelude::Entity, &DrivingParticipant), bevy::prelude::With<PlayerEntity>>()
        .iter(world)
        .find(|(_, driver)| driver.0 == PlayerSlot(1))
        .map(|(body, _)| body)
        .expect("seat 1 joined")
}

/// Half the size of `body`'s box.
fn half_size_of(app: &bevy::prelude::App, body: bevy::prelude::Entity) -> ambition_platformer2d::engine_core::Vec2 {
    app.world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(body)
        .expect("the body has kinematics")
        .size()
        * 0.5
}

/// Each live room that `before` read was rebuilt by the reload just applied:
/// a new live instance and one content term, not the old one; the session's
/// generation moved; no receipt or hidden candidate is left; each room
/// can be described; and the rooms stay as the reload left them.
///
/// A live identity is the pair (live room, `SimId`) and two live room roots
/// wear one `SimId`, so a capture of the whole world is a `DuplicateIdentity`
/// with two live rooms. Each room is described on its own.
fn assert_each_live_room_was_rebuilt(
    app: &mut bevy::prelude::App,
    before: &std::collections::BTreeMap<
        String,
        (ambition_platformer2d::platformer::lifecycle::LiveRoomInstance, std::collections::BTreeSet<String>),
    >,
    epoch_before: u64,
    arm: &str,
) {
    let old_terms: std::collections::BTreeSet<&String> = before.values().flat_map(|(_, terms)| terms).collect();
    let after = live_room_readings(app);
    let mut wrong: Vec<String> = Vec::new();
    if after.keys().ne(before.keys()) {
        wrong.push(format!(
            "the live rooms changed: {:?} -> {:?}",
            before.keys().collect::<Vec<_>>(),
            after.keys().collect::<Vec<_>>()
        ));
    }
    for (room, (instance, terms)) in &after {
        let Some((old_instance, _)) = before.get(room) else {
            continue;
        };
        if instance == old_instance {
            wrong.push(format!("`{room}` kept its live instance {instance}: it was not rebuilt"));
        }
        if terms.iter().any(|term| old_terms.contains(term)) {
            wrong.push(format!("`{room}` still carries the content term of the old generation: {terms:?}"));
        }
        if terms.len() != 1 {
            wrong.push(format!("`{room}` does not carry one content term: {terms:?}"));
        }
    }
    let new_terms: std::collections::BTreeSet<&String> = after.values().flat_map(|(_, terms)| terms).collect();
    if new_terms.len() != 1 {
        wrong.push(format!("the live rooms are not of one generation: {new_terms:?}"));
    }
    assert!(wrong.is_empty(), "{arm}: a live room was not rebuilt from the reloaded world:\n  {}", wrong.join("\n  "));
    assert_ne!(the_only_prepared_epoch(app), epoch_before, "{arm}: the generation of the session did not move");
    assert_eq!(
        (
            ambition_platformer2d::publication::outstanding_publications(app.world_mut()),
            ambition_platformer2d::platformer::construction::outstanding_candidates(app.world_mut()),
        ),
        (0, 0),
        "{arm}: (publication receipts, hidden candidates) left after the reload settled"
    );
    for (room, (instance, _)) in &after {
        let described = ambition_platformer2d::platformer::construction::TransactionBaseline::capture_for_session(
            app.world_mut(),
            ambition_platformer2d::platformer::lifecycle::SessionSpawnScope::UNSCOPED,
            ambition_platformer2d::platformer::lifecycle::TransactionRooms::only(*instance),
        );
        assert!(described.is_ok(), "{arm}: the room `{room}` that the reload published cannot be described: {:?}", described.err());
    }
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(live_room_readings(app), after, "{arm}: the live rooms changed in the 30 frames after the reload");
}

/// ⛔⛤ **THE DEV WORLD RELOAD OF ONE LIVE ROOM: REFUSED WHOLE, OR APPLIED WITH
/// ALL ITS EFFECTS.** Every effect of the LDtk hot reload (the body transit,
/// the dialog close, the combat and cooldown resets, the developer flash, the
/// presentation spawns) runs behind its publication's verdict. In order, on
/// one running session:
///
/// 1. A world that cannot be described (two process-resident holders of one
///    canonical identity, so they are in the candidate's world too) is
///    refused: the flash does not move, nothing counts as applied, the room is
///    the same, the receipt is retired, and the live rollback timeline is NOT
///    torn down for a room that was never built.
/// 2. A person (`NpcSpawn`) naming a character nobody registered is refused by
///    the preflight, which names the room and the character, instead of
///    stopping the game when the room is built (`report_unprepared_character`).
/// 3. With each fault gone, an edited copy (an entity moved) applies: the
///    flash moves, no receipt or candidate is left, the world can be
///    described, the local rollback baseline is released for the owner to
///    rebase (the control for step 1), and the room is rebuilt.
/// 4. A copy with solids over Alice and over the home body of a joined seat
///    applies, and each body that stays (Alice, and the joined seat's home
///    body, owned by the session per Q153) is moved to a clear place in the
///    new geometry (`rehome_and_dress`).
/// 5. A live room root with no geometry makes the reload wait, and say so,
///    rather than refuse after the rooms before it published; with the room
///    gone, the same press applies. Last, because the waiting reload stays
///    pending and applies once the room is gone.
///
/// ⚠ Nothing in the tree is written: edited copies live in the temporary
/// directory.
#[test]
fn a_world_reload_of_one_live_room_is_refused_whole_or_applied_whole() {
    use crate::neighbor_prefetch_prepares_rooms::{alice, room_of};
    use ambition_platformer2d::dev_tools::WorldSourceHotReload;
    let reload = |app: &bevy::prelude::App| app.world().resource::<WorldSourceHotReload>().clone();

    let mut app = a_running_shipped_session();
    let start = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
        .expect("the session has a live room")
        .id
        .clone();
    let source = watched_source(&app);
    let applied = reload(&app).applied_count;
    let readings = live_room_readings(&mut app);
    let epoch = the_only_prepared_epoch(&mut app);

    // ── 1. a world that cannot be described ─────────────────────────────────
    assert!(
        ambition_platformer2d::rollback::session_is_active(app.world()),
        "the running session has no rollback timeline, so a refusal that tears one down would go unseen"
    );
    let twins: Vec<_> = (0..2)
        .map(|_| app.world_mut().spawn(ambition_platformer2d::platformer::sim_id::SimId::placement("corrupt_twin")).id())
        .collect();
    press_apply_reload(&mut app);
    let verdict = app.world().resource::<ambition_platformer2d::publication::LastConstructionVerification>().clone();
    assert!(!verdict.published, "the reload's room was not refused: {verdict:?}");
    let status = reload(&app);
    // The reload road ran and recorded its own refusal: a reload that never
    // happened would also leave everything below untouched.
    assert!(
        !status.last_errors.is_empty() && status.last_status.contains("rejected"),
        "the hot reload did not record a refusal: {:?} / {:?}",
        status.last_status,
        status.last_errors
    );
    assert_eq!(dev_preset_flash(&app), 0.0, "⛔ A REFUSED WORLD RELOAD RAN ITS EFFECTS (the flash moved)");
    assert_eq!(status.applied_count, applied, "a refused reload counted itself as applied");
    assert_eq!(
        ambition_platformer2d::world::rooms::sole_live_room_spec(app.world()).map(|spec| spec.id.clone()),
        Some(start.clone()),
        "⛔ A REFUSED WORLD RELOAD CHANGED THE ROOM THE PLAYER IS IN"
    );
    assert!(
        ambition_platformer2d::rollback::session_is_active(app.world()),
        "⛔ A REFUSED WORLD RELOAD TORE DOWN THE ROLLBACK TIMELINE OF THE WORLD IT LEFT STANDING"
    );
    assert_eq!(
        ambition_platformer2d::publication::outstanding_publications(app.world_mut()),
        0,
        "⛔ A REFUSED WORLD RELOAD'S PUBLICATION RECEIPT IS STILL STANDING"
    );
    for twin in twins {
        app.world_mut().despawn(twin);
    }

    // ── 2. a person naming a character nobody registered ────────────────────
    {
        let _copy = watch_an_edited_copy(&mut app, "one_room_stranger", |project| {
            name_an_unknown_character(project, &start, "NpcSpawn", STRANGER)
        });
        press_apply_reload(&mut app);
    }
    let status = reload(&app);
    assert_eq!(
        (status.applied_count, status.last_status.contains("rejected")),
        (applied, true),
        "a person naming an unknown character was not refused: {:?} / {:?}",
        status.last_status,
        status.last_errors
    );
    assert!(
        status.last_errors.iter().any(|error| error.contains(&format!("`{start}`")) && error.contains(STRANGER)),
        "the refusal does not name the room `{start}` and the character: {:?}",
        status.last_errors
    );
    assert_eq!(
        (live_room_readings(&mut app), the_only_prepared_epoch(&mut app)),
        (readings.clone(), epoch),
        "⛔ A REFUSED RELOAD REBUILT THE LIVE ROOM, or moved the generation of the session"
    );
    watch_the_source_again(&mut app, &source);

    // ── 3. an edited copy applies ────────────────────────────────────────────
    let before = live_room_readings(&mut app);
    let epoch_before = the_only_prepared_epoch(&mut app);
    assert!(
        before.len() == 1 && before.values().all(|(_, terms)| terms.len() == 1),
        "before the reload the live room does not carry one content term: {before:?}"
    );
    let _moved = watch_an_edited_copy(&mut app, "one_room", |project| move_an_entity(project, &start, "PogoOrb"));
    press_apply_reload(&mut app);
    let status = reload(&app);
    assert_eq!(
        (status.applied_count, status.last_status.contains("applied"), status.last_errors.is_empty()),
        (applied + 1, true, true),
        "the edited copy did not apply: {:?} / {:?}",
        status.last_status,
        status.last_errors
    );
    assert_eq!(dev_preset_flash(&app), 1.0, "a committed world reload did not flash the developer overlay");
    assert!(
        ambition_platformer2d::platformer::construction::TransactionBaseline::capture(app.world_mut()).is_ok(),
        "⛔ THE WORLD A COMMITTED RELOAD PUBLISHED CANNOT BE DESCRIBED: some identity has two authoritative holders"
    );
    // `restart_local_ggrs_after_hot_reload` stops the session in this frame's
    // `PostUpdate` and the owner rebases it.
    assert!(
        !ambition_platformer2d::rollback::session_is_active(app.world()),
        "a committed world reload did not release the local rollback baseline"
    );
    assert_each_live_room_was_rebuilt(&mut app, &before, epoch_before, "one live room");

    // ── 4. solids over the bodies that stay ─────────────────────────────────
    let seat = join_seat_one(&mut app);
    let alice = alice(&mut app);
    // The two bodies come to rest.
    for _ in 0..60 {
        app.update();
    }
    let (alice_was, seat_was) = match (place_of(&app, alice), place_of(&app, seat)) {
        (Some((alice_at, true)), Some((seat_at, true))) if room_of(&app, alice).as_deref() == Some(start.as_str()) => {
            (alice_at, seat_at)
        }
        other => panic!("before the reload Alice and seat 1 are not both clear, in `{start}`: {other:?}"),
    };
    let (alice_half, seat_half) = (half_size_of(&app, alice), half_size_of(&app, seat));
    let _solids = watch_an_edited_copy(&mut app, "one_room_solids", |project| {
        put_a_solid_over(project, &start, alice_was, alice_half);
        put_a_solid_over(project, &start, seat_was, seat_half);
    });
    press_apply_reload(&mut app);
    let status = reload(&app);
    assert_eq!(
        (status.applied_count, status.last_errors.is_empty()),
        (applied + 2, true),
        "the copy with solids did not apply: {:?} / {:?}",
        status.last_status,
        status.last_errors
    );
    for (who, body, was) in [("Alice", alice, alice_was), ("seat 1", seat, seat_was)] {
        match place_of(&app, body) {
            None => panic!("after the reload {who} is gone or in no live room"),
            Some((is, clear)) => {
                assert!(clear, "after the reload {who} is at {is:?}, inside a solid of the new geometry of `{start}`");
                assert!(is.distance(was) >= 2.0, "a solid was put over {who} and the reload left them where they were ({is:?})");
            }
        }
    }
    assert_eq!(room_of(&app, alice).as_deref(), Some(start.as_str()), "after the reload Alice is not in `{start}`");

    // ── 5. a live room root with no geometry ────────────────────────────────
    let definition = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(app.world())
    .expect("the session keeps its room set")
    .definition_by_id("basement_npcs")
    .expect("the shipped world has the room");
    let empty = ambition_platformer2d::platformer::lifecycle::spawn_live_room(
        app.world_mut(),
        ambition_platformer2d::platformer::lifecycle::LiveRoomInstance::from_ordinal(1_000),
        definition,
    );
    assert!(
        app.world().get::<ambition_platformer2d::engine_core::RoomGeometry>(empty).is_none(),
        "the new live room root has geometry, so this does not test the wait"
    );
    press_apply_reload(&mut app);
    let status = reload(&app);
    assert_eq!(
        (status.applied_count, status.last_status.starts_with("world reload waits: live room #1000 has no geometry")),
        (applied + 2, true),
        "a reload with a live room that has no geometry: (applied count, the status says it waits); status {:?}, errors {:?}",
        status.last_status,
        status.last_errors
    );
    app.world_mut().entity_mut(empty).despawn();
    // The control: with that room gone, the same press applies.
    press_apply_reload(&mut app);
    let status = reload(&app);
    assert_eq!(
        status.applied_count,
        applied + 3,
        "with the room that has no geometry gone, the reload did not apply: {:?} / {:?}",
        status.last_status,
        status.last_errors
    );
}

/// ⭐ A WORLD RELOAD REBUILDS EVERY LIVE ROOM.
///
/// A live room that the reload did not rebuild would keep the content of the
/// old generation under the new set's definition of its room. So the reload
/// publishes the room of the primary seat, which brings the set, and then each
/// other live room from that set, in one command flush; the session's
/// generation moves once, after the last room.
///
/// Alice (the primary seat) is in the second live room, so that room is
/// published first. The edited copy moves an NPC there:
/// - Alice stays where she was (the control for the solid below);
/// - Bob, a spawned actor that seat 1 drives, is a resident of his room: he is
///   removed with the room and nothing builds him again. A driven body that
///   stayed would be a second body the reload has to move out of new solids.
///
/// A second edited copy then puts a solid over Alice: she is moved to a clear
/// place in the new geometry of her room.
#[test]
fn a_world_reload_rebuilds_every_live_room() {
    use crate::neighbor_prefetch_prepares_rooms::{alice, bob, room_of};
    use ambition_platformer2d::dev_tools::WorldSourceHotReload;
    let applied = |app: &bevy::prelude::App| app.world().resource::<WorldSourceHotReload>().applied_count;
    let apply = |app: &mut bevy::prelude::App, what: &str| {
        let count = applied(app);
        press_apply_reload(app);
        let status = app.world().resource::<WorldSourceHotReload>().clone();
        assert_eq!(
            (status.applied_count, status.last_status.contains("applied"), status.last_errors.is_empty()),
            (count + 1, true, true),
            "{what}: the reload did not apply: {:?} / {:?}",
            status.last_status,
            status.last_errors
        );
    };

    let (mut app, _) = a_running_shipped_session_with_two_live_rooms(Leaver::Alice, SECOND_LIVE_ROOM);
    // Alice comes to rest.
    for _ in 0..60 {
        app.update();
    }
    let alice = alice(&mut app);
    let driven = bob(&mut app).expect("Bob is in the world");
    let was = match place_of(&app, alice) {
        Some((at, true)) if room_of(&app, alice).as_deref() == Some(SECOND_LIVE_ROOM) => at,
        other => panic!("before the reload Alice is not clear in `{SECOND_LIVE_ROOM}`: {other:?}"),
    };
    let before = live_room_readings(&mut app);
    let epoch_before = the_only_prepared_epoch(&mut app);
    let old_terms: std::collections::BTreeSet<&String> = before.values().flat_map(|(_, terms)| terms).collect();
    assert!(
        before.len() == 2 && before.values().all(|(_, terms)| terms.len() == 1) && old_terms.len() == 1,
        "before the reload, the two live rooms do not carry one content term, the same one: {before:?}"
    );

    let _moved = watch_an_edited_copy(&mut app, "two", |project| move_an_entity(project, SECOND_LIVE_ROOM, "NpcSpawn"));
    apply(&mut app, "an NPC moved");
    let Some((is, clear)) = place_of(&app, alice) else {
        panic!("after the reload Alice is in no live room");
    };
    assert!(clear && room_of(&app, alice).as_deref() == Some(SECOND_LIVE_ROOM), "after the reload Alice is at {is:?}, not clear in `{SECOND_LIVE_ROOM}`");
    assert!(is.distance(was) <= 2.0, "no solid was put over Alice, and the reload moved her {} px", is.distance(was));
    assert!(
        app.world().get_entity(driven).is_err(),
        "the driven body stayed in a room that the reload built again: it is a second body that stays, so move it out \
         of the new solids too"
    );
    // One move for the whole sequence, not one per room. Asked of a fresh
    // session: after refused reloads the epoch was measured to move by two.
    assert_eq!(the_only_prepared_epoch(&mut app), epoch_before + 1, "the generation of the session did not move once for two rooms");
    assert_each_live_room_was_rebuilt(&mut app, &before, epoch_before, "two live rooms");

    let was = place_of(&app, alice).expect("Alice is in a live room").0;
    let half = half_size_of(&app, alice);
    let _solid = watch_an_edited_copy(&mut app, "two_solid", |project| put_a_solid_over(project, SECOND_LIVE_ROOM, was, half));
    apply(&mut app, "a solid over Alice");
    let Some((is, clear)) = place_of(&app, alice) else {
        panic!("after the second reload Alice is in no live room");
    };
    assert_eq!(room_of(&app, alice).as_deref(), Some(SECOND_LIVE_ROOM), "after the second reload Alice left `{SECOND_LIVE_ROOM}`");
    assert!(clear, "after the second reload Alice is at {is:?}, inside a solid of the new geometry");
    assert!(is.distance(was) >= 2.0, "a solid was put over Alice and she is where she was ({is:?})");
}

/// ⛔ AN ENEMY NAMING A CHARACTER NOBODY REGISTERED, IN A LIVE ROOM THAT IS
/// NOT THE PRIMARY SEAT'S, REFUSES THE WHOLE RELOAD.
///
/// Each plan is prepared before the first room is staged; the first room's
/// publication moves the session to the new room set, so a room that failed
/// after it would leave a mixed world. The refusal names the room and the
/// character, and no live room and no generation changes. The person arms of
/// this refusal are in
/// `a_world_reload_with_two_live_rooms_publishes_all_or_none` and
/// `a_world_reload_of_one_live_room_is_refused_whole_or_applied_whole`.
#[test]
fn a_world_reload_that_cannot_rebuild_one_live_room_rebuilds_none() {
    use ambition_platformer2d::dev_tools::WorldSourceHotReload;
    let faulty_room = "basement_enemies";
    let (mut app, start) = a_running_shipped_session_with_two_live_rooms(Leaver::Bob, faulty_room);
    {
        use crate::neighbor_prefetch_prepares_rooms::{alice, room_of};
        let body = alice(&mut app);
        assert_eq!(room_of(&app, body), Some(start.clone()), "Alice is not in the start room");
    }
    let before = live_room_readings(&mut app);
    let epoch_before = the_only_prepared_epoch(&mut app);
    let applied = app.world().resource::<WorldSourceHotReload>().applied_count;
    let _copy = watch_an_edited_copy(&mut app, "refused_enemy", |project| {
        name_an_unknown_character(project, faulty_room, "EnemySpawn", STRANGER)
    });
    press_apply_reload(&mut app);
    let reload = app.world().resource::<WorldSourceHotReload>().clone();
    assert_eq!(
        (reload.applied_count, reload.last_status.contains("rejected")),
        (applied, true),
        "the reload was not refused: {:?} / {:?}",
        reload.last_status,
        reload.last_errors
    );
    assert!(
        reload.last_errors.iter().any(|error| error.contains(&format!("`{faulty_room}`")) && error.contains(STRANGER)),
        "the refusal does not name the room `{faulty_room}` and the character: {:?}",
        reload.last_errors
    );
    assert!(
        reload.last_errors.iter().any(|error| error.contains(&format!("live room '{faulty_room}'"))),
        "the refusal does not say which LIVE room cannot be rebuilt: {:?}",
        reload.last_errors
    );
    assert_eq!(
        (live_room_readings(&mut app), the_only_prepared_epoch(&mut app)),
        (before.clone(), epoch_before),
        "⛔ A REFUSED RELOAD REBUILT A LIVE ROOM, or moved the generation of the session"
    );
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(live_room_readings(&mut app), before, "the live rooms changed after the refusal");
}

/// Give the first `entity` of a level whose `activeArea` is `room`, and that
/// has a `character_id`, the id `stranger`, which no catalog has.
fn name_an_unknown_character(
    project: &mut serde_json::Value,
    room: &str,
    entity: &str,
    stranger: &str,
) {
    let levels = project["levels"].as_array_mut().expect("the project has levels");
    for level in levels {
        let in_room = level["fieldInstances"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|field| field["__identifier"] == "activeArea" && field["__value"] == room);
        if !in_room {
            continue;
        }
        for layer in level["layerInstances"].as_array_mut().into_iter().flatten() {
            for instance in layer["entityInstances"].as_array_mut().into_iter().flatten() {
                if instance["__identifier"] != entity {
                    continue;
                }
                for field in instance["fieldInstances"].as_array_mut().into_iter().flatten() {
                    if field["__identifier"] == "character_id" {
                        field["__value"] = serde_json::json!(stranger);
                        return;
                    }
                }
            }
        }
    }
    panic!("the project has no `{entity}` with a `character_id` in a level of `{room}`");
}

/// Two holders of one identity in the live room `live`: a room that no
/// transaction can describe (`BaselineCaptureError::DuplicateIdentity`).
///
/// The two carry the room's stamp, so only a transaction of that room has
/// them in its world (`TransactionRooms::admits`). The existing refusal arm
/// (`a_world_reload_of_one_live_room_is_refused_whole_or_applied_whole`) gives its two
/// holders no stamp, so each room's transaction has them and the first room
/// refuses.
fn put_two_holders_of_one_identity_in(
    world: &mut bevy::prelude::World,
    live: ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
) -> [bevy::prelude::Entity; 2] {
    use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
    use ambition_platformer2d::platformer::sim_id::SimId;
    [(); 2].map(|()| world.spawn((SimId::placement("corrupt_twin"), InRoomInstance(live))).id())
}

/// Whether a transaction of the one live room `live` can describe its world.
fn the_room_can_be_described(
    app: &mut bevy::prelude::App,
    live: ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
) -> bool {
    ambition_platformer2d::platformer::construction::TransactionBaseline::capture_for_session(
        app.world_mut(),
        ambition_platformer2d::platformer::lifecycle::SessionSpawnScope::UNSCOPED,
        ambition_platformer2d::platformer::lifecycle::TransactionRooms::only(live),
    )
    .is_ok()
}

/// The fault of the last step of
/// [`a_world_reload_with_two_live_rooms_publishes_all_or_none`]: when the
/// reload builds the first body of a live room that is not `first` and not
/// `into` (a candidate of the room it mints first), it puts two holders of one
/// identity in the live room `into`.
#[derive(bevy::prelude::Resource)]
struct FaultWhenTheFirstRoomIsBuilt {
    first: ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
    into: ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
    made: Vec<bevy::prelude::Entity>,
}

/// ⛔ A MULTI-ROOM RELOAD PUBLISHES ALL ITS ROOMS OR NONE.
///
/// Alice is in the start room (the room the reload publishes first) and Bob,
/// driven by seat 1, in `basement_npcs`. Each fault below is in Bob's room, so
/// the first room's own checks pass; each refuses every room, names the room,
/// and leaves each live room's instance, content and the session's generation
/// as they were, with no receipt or hidden candidate left. In order, on one
/// running session:
///
/// 1. A person (`NpcSpawn`) naming a character nobody registered: the preflight
///    looks at placements too, so the plan does not prepare (it used to prepare
///    and stop the game when the room was built).
/// 2. Two holders of one identity in Bob's room, present at the request: the
///    room's transaction cannot open, which is found before the first room.
/// 3. The same fault injected AFTER the request, by an observer, when the
///    reload builds the first body of its first room: the first room is held
///    after its check, and each later room is checked against the set and
///    counter the session will hold when the rooms before it are committed, so
///    the later refusal refuses the first room too.
///
/// With the fault gone, the same reload applies and brings each live room to
/// one new generation.
///
/// ⚠ A reload keeps the entity of a live room root and gives it its new
/// identity in place: `Add` and `Insert` of `LiveRoomInstance` and
/// `RoomInstanceRoot` do not fire in a reload. The stamp of a body
/// (`InRoomInstance`) is added, so the observer watches that.
#[test]
fn a_world_reload_with_two_live_rooms_publishes_all_or_none() {
    use ambition_platformer2d::dev_tools::WorldSourceHotReload;
    use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
    use bevy::prelude::*;

    let (mut app, start) = a_running_shipped_session_with_two_live_rooms(Leaver::Bob, SECOND_LIVE_ROOM);
    {
        use crate::neighbor_prefetch_prepares_rooms::{alice, room_of};
        let body = alice(&mut app);
        assert_eq!(room_of(&app, body), Some(start.clone()), "the premise: Alice is in the start room");
    }
    let source = watched_source(&app);
    let before = live_room_readings(&mut app);
    let epoch_before = the_only_prepared_epoch(&mut app);
    let (first_live, later_live) = (before[&start].0, before[SECOND_LIVE_ROOM].0);
    let old_term = before[&start].1.iter().next().expect("the first room has stamped content").clone();
    assert_eq!(
        before[SECOND_LIVE_ROOM].1.iter().collect::<Vec<_>>(),
        vec![&old_term],
        "premise: the two live rooms are of one generation"
    );
    let applied = app.world().resource::<WorldSourceHotReload>().applied_count;
    let unchanged = |app: &mut bevy::prelude::App, what: &str| {
        let reload = app.world().resource::<WorldSourceHotReload>().clone();
        assert!(
            !reload.last_errors.iter().any(|error| error.contains("MIXED")),
            "{what}: ⛔ THE RELOAD LEFT A MIXED WORLD: {:?}",
            reload.last_errors
        );
        assert_eq!(
            (reload.applied_count, reload.last_status.contains("rejected")),
            (applied, true),
            "{what}: the reload was not refused: {:?} / {:?}",
            reload.last_status,
            reload.last_errors
        );
        assert_eq!(
            (live_room_readings(app), the_only_prepared_epoch(app)),
            (before.clone(), epoch_before),
            "{what}: ⛔ A REFUSED RELOAD REBUILT A LIVE ROOM, or moved the generation of the session"
        );
        assert_eq!(
            (
                ambition_platformer2d::publication::outstanding_publications(app.world_mut()),
                ambition_platformer2d::platformer::construction::outstanding_candidates(app.world_mut()),
            ),
            (0, 0),
            "{what}: (publication receipts, hidden candidates) left after the refusal"
        );
        reload.last_errors
    };

    // ── 1. a person naming a character nobody registered ────────────────────
    {
        let _copy = watch_an_edited_copy(&mut app, "two_room_stranger", |project| {
            name_an_unknown_character(project, SECOND_LIVE_ROOM, "NpcSpawn", STRANGER)
        });
        press_apply_reload(&mut app);
    }
    let errors = unchanged(&mut app, "a person naming an unknown character");
    assert!(
        errors.iter().any(|error| error.contains(&format!("`{SECOND_LIVE_ROOM}`")) && error.contains(STRANGER))
            && errors.iter().any(|error| error.contains(&format!("live room '{SECOND_LIVE_ROOM}'"))),
        "the refusal does not name the live room `{SECOND_LIVE_ROOM}` and the character: {errors:?}"
    );
    watch_the_source_again(&mut app, &source);

    // ── 2. Bob's room cannot be described, at the request ───────────────────
    let twins = put_two_holders_of_one_identity_in(app.world_mut(), later_live);
    assert_eq!(
        (the_room_can_be_described(&mut app, first_live), the_room_can_be_described(&mut app, later_live)),
        (true, false),
        "premise: (the room of the primary seat can be described, Bob's room can be described)"
    );
    {
        let _copy = watch_an_edited_copy(&mut app, "undescribed", |project| {
            move_an_entity(project, SECOND_LIVE_ROOM, "NpcSpawn")
        });
        press_apply_reload(&mut app);
    }
    let errors = unchanged(&mut app, "a live room that cannot be described");
    assert!(
        errors.iter().any(|error| error.contains(&format!("live room '{SECOND_LIVE_ROOM}'")) && error.contains("corrupt_twin")),
        "the refusal does not name the live room and the identity that two bodies hold: {errors:?}"
    );
    for twin in twins {
        app.world_mut().despawn(twin);
    }
    watch_the_source_again(&mut app, &source);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(live_room_readings(&mut app), before, "the live rooms changed in the 30 frames after the refusals");

    // ── 3. the same fault, after the first room passed ──────────────────────
    app.insert_resource(FaultWhenTheFirstRoomIsBuilt { first: first_live, into: later_live, made: Vec::new() });
    app.world_mut().add_observer(
        |stamped: On<Add, InRoomInstance>, mut world: bevy::ecs::world::DeferredWorld| {
            // Asked of the entity and not of a query: a candidate is hidden
            // from a query.
            let Some(room) = world.get::<InRoomInstance>(stamped.entity).map(|stamp| stamp.0) else {
                return;
            };
            let fault = world.resource::<FaultWhenTheFirstRoomIsBuilt>();
            if !fault.made.is_empty() || room == fault.first || room == fault.into {
                return;
            }
            let into = fault.into;
            let twins = [(); 2].map(|()| {
                world
                    .commands()
                    .spawn((ambition_platformer2d::platformer::sim_id::SimId::placement("corrupt_twin"), InRoomInstance(into)))
                    .id()
            });
            world.resource_mut::<FaultWhenTheFirstRoomIsBuilt>().made.extend(twins);
        },
    );
    assert!(
        the_room_can_be_described(&mut app, first_live) && the_room_can_be_described(&mut app, later_live),
        "premise: each live room can be described before the reload"
    );
    let _copy = watch_an_edited_copy(&mut app, "mixed", |project| move_an_entity(project, SECOND_LIVE_ROOM, "NpcSpawn"));
    press_apply_reload(&mut app);
    let twins = app.world().resource::<FaultWhenTheFirstRoomIsBuilt>().made.clone();
    assert_eq!(twins.len(), 2, "premise: the fault was injected when the first room was built");
    let errors = unchanged(&mut app, "a later room refused after the first room passed");
    assert!(
        errors.iter().any(|error| error.contains(&format!("'{SECOND_LIVE_ROOM}'")) && error.contains("no room was reloaded")),
        "the status does not say that the later room was refused and no room was reloaded: {errors:?}"
    );
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(live_room_readings(&mut app), before, "the live rooms changed in the 30 frames after");

    // With the fault gone, the reload applies onto the world the refusal kept.
    for twin in twins {
        app.world_mut().despawn(twin);
    }
    press_apply_reload(&mut app);
    let reload = app.world().resource::<WorldSourceHotReload>().clone();
    assert!(
        reload.last_status.contains("applied") && reload.last_errors.is_empty(),
        "the reload did not apply once the fault was gone: {:?} / {:?}",
        reload.last_status,
        reload.last_errors
    );
    let repaired = live_room_readings(&mut app);
    let terms: std::collections::BTreeSet<&String> = repaired.values().flat_map(|(_, terms)| terms).collect();
    assert!(
        terms.len() == 1 && !terms.contains(&old_term) && repaired[SECOND_LIVE_ROOM].0 != later_live,
        "the reload did not bring each live room to one new generation: {repaired:?}"
    );
}

/// ⛔⛤ **A10'S ACCEPTANCE CRITERION AT SESSION SCOPE: A CANDIDATE SESSION THE
/// TRANSACTION REFUSES LEAVES THE SESSION YOU ARE PLAYING PLAYABLE.**
///
/// ⚠ **THE REFUSAL IS A PRODUCTION ONE AND NOTHING TEST-ONLY IS WIRED INTO
/// CONSTRUCTION.** An entity carrying a canonical `SimId` and NO session owner is
/// process-resident by definition, so it is in every session's world — including
/// the candidate's. Give it an identity the start room authors and the candidate's
/// first room finds two holders of one identity and refuses, through
/// `verify_committed_roster`, exactly as it would for any real duplicate.
///
/// ⭐ **THE CONTROL IS `the_shipped_app_never_holds_two_session_roots_across_a_handoff`**,
/// which drives the same two `ReplaceWith` commands with no duplicate standing
/// and lands the incoming session. Without it, "the handoff did not happen" would
/// be satisfied by a handoff that never worked.
#[test]
fn a_candidate_session_the_transaction_refuses_leaves_the_live_session_playable() {
    use ambition_platformer2d::platformer::lifecycle::{
        ActiveSessionScope, SessionScopeId, SessionScopedEntity,
    };

    fn scope(app: &bevy::prelude::App) -> Option<SessionScopeId> {
        app.world()
            .get_resource::<ActiveSessionScope>()
            .and_then(ActiveSessionScope::current)
    }
    // ⛔ IDENTITIES, NOT A COUNT. This counted every session-scoped entity,
    // particles included, and six dust particles expiring during 240 frames of
    // idle play read as "a refused candidate cost world N six entities". What a
    // candidate could take from N is its SIMULATION: the entities with a
    // `SimId`. A set also says which one went.
    fn population(
        app: &mut bevy::prelude::App,
        owner: SessionScopeId,
    ) -> std::collections::BTreeSet<ambition_platformer2d::platformer::sim_id::SimId> {
        let world = app.world_mut();
        world
            .query::<(&SessionScopedEntity, &ambition_platformer2d::platformer::sim_id::SimId)>()
            .iter(world)
            .filter(|(scoped, _)| scoped.0 == owner)
            .map(|(_, id)| id.clone())
            .collect()
    }
    fn live_room(app: &mut bevy::prelude::App) -> Option<String> {
        ambition_platformer2d::world::rooms::sole_live_room_spec(app.world()).map(|spec| spec.id.clone())
    }

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();

    let gameplay = ShellRouteId::new("ambition_gameplay");
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay.clone(),
        request: None,
    });
    crate::common::step_until_route_is_active_and_settled(&mut app, "ambition_gameplay");

    // ── world N, playable ────────────────────────────────────────────────────
    let live_activation = activation_id(&app).expect("the first session activated");
    let live = scope(&app).expect("the live session owns a scope");
    let before_population = population(&mut app, live);
    let before_room = live_room(&mut app);
    assert!(
        !before_population.is_empty() && before_room.is_some(),
        "the premise: world N is a real, populated, roomed session"
    );
    // ⛔⛤ **THE PREMISE FINDING 1 NEEDS: A'S DURABLE STATE MUST DIFFER FROM THE
    // SAVE.** MEASURED — without this the poison PASSES: a fresh session's ledger
    // and the save's are equal, so re-installing the save over A changes nothing
    // and every assertion below is vacuously true. The checkpoint baselines are
    // deliberately NOT the current projection, and that difference is the whole
    // thing this arm is about. One synthetic row is the cheapest way to make A's
    // horizon distinguishable; it establishes the PREMISE and fakes nothing about
    // the subject.
    {
        use ambition_platformer2d::platformer::lifecycle::{
            AuthoredOccurrences, OccurrenceWhereabouts,
        };
        // ⚠ THE BASELINE ONLY, NOT THE LIVE LEDGER. MEASURED: a row written into
        // `AuthoredOccurrences` is persisted into the save by
        // `persist_occurrence_horizon_to_save` within a frame, so the save and
        // the live ledger cannot be made to differ from a test — re-installing
        // the save over them is then a no-op and the poison passes. The CHECKPOINT
        // baseline is the thing that legitimately differs from the save, which is
        // exactly the state the finding is about.
        let mut rows: std::collections::BTreeMap<_, _> = app
            .world()
            .resource::<AuthoredOccurrences>()
            .rows()
            .map(|(id, where_)| (id.clone(), where_.clone()))
            .collect();
        rows.insert(
            ambition_platformer2d::platformer::sim_id::SimId::placement("a10_horizon_witness"),
            OccurrenceWhereabouts::Consumed,
        );
        let mut remembered = AuthoredOccurrences::default();
        remembered.adopt_rows(rows);
        ambition_platformer2d::platformer::lifecycle::session_world_component_mut::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>(app.world_mut()).unwrap()
            .adopt(remembered);
    }

    // A's MECHANICAL state as it stands while A is the only session. The
    // candidate carries its own `SessionMechanics` and installs it at adoption,
    // and its `MovingPlatformSet` is on its own hidden live room root; a
    // refused candidate must leave A's alone.
    let mechanics_before = app
        .world()
        .get_resource::<ambition_platformer2d::construction::mechanics::SessionMechanics>()
        .map(|mechanics| format!("{mechanics:?}"));
    let platforms_before = ambition_platformer2d::session::sole_live_room_component::<ambition_platformer2d::world::collision::MovingPlatformSet>(app.world()).expect("the live room has moving platforms")
        .0
        .len();
    assert!(
        mechanics_before.is_some(),
        "A has no `SessionMechanics`, so the assertion below about a refused \
         candidate not changing them says nothing"
    );

    // A's durable horizon as it stands while A is the only session.
    let durable_before = (
        app.world()
            .get_resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
            .cloned(),
        ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>(app.world())
            .cloned(),
        ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::platformer::lifecycle::CustodyBaseline>(app.world())
            .cloned(),
    );
    assert!(
        durable_before.0.is_some(),
        "this composition installs no occurrence ledger, so the assertions below \
         about a refused candidate not changing it say nothing"
    );

    // ── two holders of one identity, in every session's world ───────────────
    // ⛔ TWO, NOT ONE, AND THE DIFFERENCE IS THE WHOLE MECHANISM. A single extra
    // holder of an id the room PLANS is a predecessor: the candidate declares it
    // superseded and publication retires it, which is ordinary A10. A pair is a
    // world that cannot be described — `TransactionBaseline::capture` refuses a
    // duplicated identity outright — so the candidate's first room cannot be
    // verified at all.
    //
    // ⚠ Process-resident on purpose: an entity carrying a canonical `SimId` and
    // NO session owner is in every session's world by definition, including the
    // candidate's, which is what puts the corruption in front of the candidate
    // rather than in front of the session that is playing.
    for _ in 0..2 {
        app.world_mut()
            .spawn(ambition_platformer2d::platformer::sim_id::SimId::placement(
                "corrupt_twin",
            ));
    }

    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay,
        request: None,
    });
    // Until a room transaction was refused (under the shared ceiling), then
    // frames for the refusal's exits to run; the premise below names a
    // refusal that never came.
    let started = std::time::Instant::now();
    while app
        .world()
        .resource::<ambition_platformer2d::publication::LastConstructionVerification>()
        .published
        && started.elapsed() < crate::common::ACTIVATION_CEILING
    {
        app.update();
    }
    for _ in 0..30 {
        app.update();
    }

    // ⛔ THE PREMISE, BEFORE ANY CONCLUSION: a room transaction actually RAN and
    // was REFUSED. Without this, "the handoff did not happen" is equally true of
    // a route that never became ready, and every assertion below would be
    // satisfied by an app that simply did nothing.
    let verdict = app
        .world()
        .resource::<ambition_platformer2d::publication::LastConstructionVerification>()
        .clone();
    assert!(
        !verdict.published,
        "no room transaction was refused, so the candidate session was never the \
         thing that failed and this arm is about nothing: {verdict:?}"
    );

    // ── the candidate was refused, and N is still the session you are in ─────
    assert_eq!(
        activation_id(&app),
        Some(live_activation),
        "⛔ A CANDIDATE SESSION WHOSE FIRST ROOM COULD NOT BE BUILT ACTIVATED \
         ANYWAY: the shell retired the session that was playing for one that does \
         not exist"
    );
    assert_eq!(
        scope(&app),
        Some(live),
        "the live session scope moved even though no candidate was admitted"
    );
    let lost: Vec<_> = before_population.difference(&population(&mut app, live)).cloned().collect();
    assert!(lost.is_empty(), "⛔ A REFUSED CANDIDATE SESSION COST WORLD N ENTITIES: {lost:?}");
    assert_eq!(
        live_room(&mut app),
        before_room,
        "⛔ A REFUSED CANDIDATE SESSION CHANGED THE ROOM THE PLAYER IS IN"
    );

    assert_eq!(
        app.world()
            .get_resource::<ambition_platformer2d::construction::mechanics::SessionMechanics>()
            .map(|mechanics| format!("{mechanics:?}")),
        mechanics_before,
        "⛔ A REFUSED CANDIDATE CHANGED THE LIVE SESSION'S MECHANICS"
    );
    assert_eq!(
        ambition_platformer2d::session::sole_live_room_component::<ambition_platformer2d::world::collision::MovingPlatformSet>(app.world()).expect("the live room has moving platforms")
            .0
            .len(),
        platforms_before,
        "⛔ A REFUSED CANDIDATE CHANGED THE LIVE SESSION'S MOVING-PLATFORM STATE"
    );

    // ⛔⛤ **AND A'S DURABLE STATE IS UNCHANGED — REVIEW FINDING 1, 2026-09-15.**
    // Preparing a candidate used to install `AuthoredOccurrences`,
    // `OccurrenceBaseline` and `CustodyBaseline` PROCESS-WIDE, while the outgoing
    // session was still the live one. The two baselines are rollback-authoritative
    // checkpoint state and are deliberately NOT equal to the current save/live
    // projection, so a candidate that then refused left A playable with different
    // DEATH SEMANTICS than it had a moment earlier. The candidate carries its
    // horizon as a value now and adoption installs it; a refusal drops it.
    assert_eq!(
        app.world()
            .get_resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
            .cloned(),
        durable_before.0,
        "⛔ A REFUSED CANDIDATE CHANGED THE LIVE SESSION'S OCCURRENCE LEDGER"
    );
    assert_eq!(
        ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>(app.world())
            .cloned(),
        durable_before.1,
        "⛔ A REFUSED CANDIDATE CHANGED THE LIVE SESSION'S CHECKPOINT OCCURRENCE \
         BASELINE — rollback-authoritative state, for a world that was never built"
    );
    assert_eq!(
        ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::platformer::lifecycle::CustodyBaseline>(app.world())
            .cloned(),
        durable_before.2,
        "⛔ A REFUSED CANDIDATE CHANGED THE LIVE SESSION'S CHECKPOINT CUSTODY \
         BASELINE"
    );

    // ⛔⛤ **AND THE REFUSED CANDIDATE LEFT NO CONTROL-PLANE STATE BEHIND EITHER —
    // REVIEW FINDING 3, 2026-09-15.** The world half of this arm was asserted
    // from the start; the correlation half was not, and the refusal exit was the
    // one door of four that released neither the reserved SCOPE nor the route
    // HOLD. It leaked one `ShellActivationId -> SessionScopeId` per refusal,
    // permanently: the route never activates, so nothing ever calls `take`.
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::ReservedGameplayScopes>()
            .outstanding(),
        0,
        "⛔ A REFUSED CANDIDATE LEAKED ITS RESERVED SCOPE. The route will never \
         activate, so nothing will ever adopt this reservation"
    );
    assert_eq!(
        ambition_platformer2d::platformer::construction::outstanding_candidates(app.world_mut()),
        0,
        "⛔ A REFUSED CANDIDATE'S HIDDEN ENTITIES ARE STILL IN THE WORLD"
    );
    assert_eq!(
        ambition_platformer2d::publication::outstanding_publications(app.world_mut()),
        0,
        "⛔ A REFUSED CANDIDATE'S PUBLICATION RECEIPT IS STILL STANDING"
    );
    let gates = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellActivationGates>();
    let leaked: Vec<u32> = (1..=6)
        .filter(|n| {
            let id = ambition_platformer2d::game_shell::ShellHoldId::new(format!(
                "session-publication:{n}"
            ));
            gates.evaluator(&id).is_some()
        })
        .collect();
    assert!(
        leaked.is_empty(),
        "⛔ A REFUSED CANDIDATE'S GATE EVALUATOR OUTLIVED IT: {leaked:?}"
    );
    let held = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellRouteHolds>()
        .held(&ShellRouteId::new("ambition_gameplay"));
    assert!(
        held.is_empty(),
        "⛔ A REFUSED CANDIDATE'S ROUTE HOLD OUTLIVED IT: {held:?}"
    );
}

/// ⛔⛤ **A SUPERSEDED CANDIDATE SESSION IS DISCARDED, AND ITS TRANSACTION
/// CANNOT PUBLISH.**
///
/// `CandidateSessionSlot` is one deep: a second pending route replaces the
/// first. The superseded candidate (hidden root, hidden first room, publication
/// receipt, reserved scope, gate registration) must be discarded, not left
/// alive and unreachable: a candidate that is neither published nor discarded
/// is the state A10's lifecycle exists to make impossible.
///
/// And the superseded TRANSACTION, which is what a caller correlates its own
/// request on, ends named `Superseded` and cannot publish when the production
/// `PreparedSessionRegistry` is asked to (the router's logic is tested on a
/// hand-built `ShellRouter`; this asks the composed host).
#[test]
fn a_candidate_session_replaced_while_pending_is_discarded() {
    use ambition_platformer2d::game_shell::{PreparedSessionRegistry, ProviderLoadTransaction, ShellEvent, TransactionEnd};
    use bevy::ecs::message::Messages;

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();
    let gameplay = ShellRouteId::new("ambition_gameplay");
    let mut cursor = app.world().resource::<Messages<ShellEvent>>().get_cursor();
    let mut seen: Vec<ShellEvent> = Vec::new();
    let mut read = |app: &bevy::prelude::App, seen: &mut Vec<ShellEvent>| {
        seen.extend(cursor.read(app.world().resource::<Messages<ShellEvent>>()).cloned());
    };

    // Two routes requested with only a few frames between them, so the second
    // arrives while the first is still PENDING: the state that supersedes a
    // prepared candidate instead of adopting it.
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay.clone(),
        request: None,
    });
    for _ in 0..3 {
        app.update();
        read(&app, &mut seen);
    }
    let first: ProviderLoadTransaction = seen
        .iter()
        .find_map(|event| match event {
            ShellEvent::PreparationRequested(transaction) => Some(transaction.clone()),
            _ => None,
        })
        .expect("the shipped composition emitted no `PreparationRequested` in three frames, so there is no transaction to supersede");
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay,
        request: None,
    });
    // Until the second route is active and nothing is pending (under the shared
    // ceiling), then a few frames for the exits to run.
    let started = std::time::Instant::now();
    let mut after = 0;
    while after < 10 {
        assert!(started.elapsed() < crate::common::ACTIVATION_CEILING, "the second route did not settle in {:?}", started.elapsed());
        app.update();
        read(&app, &mut seen);
        let router = app.world().resource::<ShellRouter>();
        if router.active.is_some() && router.pending.is_none() {
            after += 1;
        }
    }

    // ⛔ A second, DIFFERENT transaction was minted: a reused one would mean
    // nothing was superseded.
    let minted: Vec<_> = seen
        .iter()
        .filter_map(|event| match event {
            ShellEvent::PreparationRequested(transaction) => Some(transaction.barrier.load_id.clone()),
            _ => None,
        })
        .collect();
    assert!(
        minted.len() >= 2 && minted[0] != minted[1],
        "the second request did not mint a fresh transaction ({minted:?}), so nothing was superseded"
    );
    let ended = seen.iter().find_map(|event| match event {
        ShellEvent::TransactionEnded { barrier, reason, .. } if *barrier == first.barrier => Some(reason.clone()),
        _ => None,
    });
    assert_eq!(
        ended,
        Some(TransactionEnd::Superseded),
        "the first transaction ended as {ended:?} rather than named superseded: a caller waiting on it has nothing \
         to match on"
    );
    let published = app.world_mut().resource_mut::<PreparedSessionRegistry>().publish(&first);
    assert!(
        published.is_none(),
        "the superseded transaction published in the shipped app ({published:?}): a generation nobody admitted"
    );

    use ambition_platformer2d::platformer::lifecycle::{session_root_for_scope, SessionScopeId};

    // ⛔⛤ **AND NOTHING A CANDIDATE REGISTERED OUTLIVES IT.** `gates.register` had
    // no matching `forget` anywhere on A10's road, so EVERY candidate ever
    // prepared — adopted, refused or superseded — left an evaluator entry behind
    // for the life of the process. MEASURED before the fix: entries for
    // activation 2 (superseded) AND 3 (adopted) both still registered.
    //
    // ⚠ AND THE ORDER IS LOAD-BEARING: releasing the hold must come BEFORE
    // forgetting its evaluator. Forgetting first leaves the router a hold it
    // cannot evaluate, and MEASURED, that wedges the route permanently — the
    // superseding session never started either.
    {
        let gates = app
            .world()
            .resource::<ambition_platformer2d::game_shell::ShellActivationGates>();
        let leaked: Vec<u32> = (1..=6)
            .filter(|n| {
                let id = ambition_platformer2d::game_shell::ShellHoldId::new(format!(
                    "session-publication:{n}"
                ));
                gates.evaluator(&id).is_some()
            })
            .collect();
        assert!(
            leaked.is_empty(),
            "⛔ CANDIDATE GATE REGISTRATIONS OUTLIVED THEIR CANDIDATES: {leaked:?}. \
             These hold ids can never be held again — activation ids are \
             monotonic — so every entry is permanent growth"
        );
        let held = app
            .world()
            .resource::<ambition_platformer2d::game_shell::ShellRouteHolds>()
            .held(&ShellRouteId::new("ambition_gameplay"));
        assert!(
            held.is_empty(),
            "the gameplay route is still held after both activations settled: \
             {held:?}"
        );
    }

    // ⭐ THE PREMISE, AND IT IS OBSERVABLE BECAUSE THE ALLOCATOR IS SEQUENTIAL.
    // `ActiveSessionScope::reserve` hands out ids in order and only on a
    // reservation, so a LIVE session at scope 1 means scope 0 was reserved by an
    // activation that never became live — which is the supersession this arm is
    // about. If the two routes had not overlapped, the live session would be at
    // scope 0 and everything below would be vacuous.
    let live = session_root_for_scope(app.world_mut(), SessionScopeId(1));
    assert!(
        live.is_some(),
        "no session is live at scope 1, so the second route never superseded a \
         prepared candidate and this arm says nothing"
    );

    // ⛔⛤ AND THE SUPERSEDED CANDIDATE IS GONE. `session_root_for_scope` looks
    // through the disabling marker (`Allow<InactiveCandidate>`), so a candidate
    // root that was merely HIDDEN would still be found here — which is exactly
    // what this must refuse.
    assert_eq!(
        session_root_for_scope(app.world_mut(), SessionScopeId(0)),
        None,
        "⛔ A CANDIDATE SESSION REPLACED WHILE PENDING IS STILL IN THE WORLD. It \
         was never published and never discarded, so its root, its first room and \
         its publication receipt are alive and unreachable forever"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::ReservedGameplayScopes>()
            .outstanding(),
        0,
        "the reservation ledger still holds a scope for an activation that will \
         never happen"
    );
}

/// ⛔⛤ **AND A CANCELLED ROUTE DISCARDS ITS CANDIDATE TOO — THE FOURTH EXIT.**
///
/// `ShellCommand::CancelPending` is `Q118`'s "breaking the authorization early
/// cancels both halves": a correlated requester whose own half became illegal
/// ends the pending transaction it issued. The router clears its pending
/// transaction and used to tell this provider nothing at all, so the prepared
/// candidate sat in its slot forever — entities hidden in the world, publication
/// receipt unretired, scope reserved, hold and evaluator registered. **A
/// cancelled route leaked exactly what a superseded one did.**
///
/// ⭐ Its control is `a_candidate_session_replaced_while_pending_is_discarded`,
/// which proves the same cleanup runs on the supersession road; the two share one
/// site, and this arm is what says the site's condition reaches the cancel.
#[test]
fn a_candidate_session_whose_route_is_cancelled_is_discarded() {
    use ambition_platformer2d::game_shell::ShellRequestId;
    use ambition_platformer2d::platformer::lifecycle::{session_root_for_scope, SessionScopeId};

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();

    let request = ShellRequestId::new("a10-cancel-witness");
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: ShellRouteId::new("ambition_gameplay"),
        request: Some(request.clone()),
    });
    // Long enough for the candidate to be prepared and to HOLD the route, short
    // enough that the gate has not admitted it.
    for _ in 0..3 {
        app.update();
    }
    assert!(
        session_root_for_scope(app.world_mut(), SessionScopeId(0)).is_some(),
        "no candidate session root was prepared at scope 0, so cancelling has \
         nothing to abandon and this arm says nothing"
    );
    // ⭐ THE POSITIVE CONTROL FOR THE COUNT ASSERTED AT THE END. A census that
    // reports 0 because its query is wrong reads exactly like a world with no
    // orphans; this is the moment a candidate is SUPPOSED to be standing, so a
    // count of 0 here would mean the instrument is blind rather than the world
    // clean.
    assert!(
        ambition_platformer2d::platformer::construction::outstanding_candidates(app.world_mut())
            > 0,
        "the candidate census counts nothing while a candidate session is prepared and hidden, so the zero it reports at the end of this arm would be a claim about the QUERY"
    );

    app.world_mut()
        .write_message(ShellCommand::CancelPending { request });
    for _ in 0..120 {
        app.update();
    }

    assert_eq!(
        session_root_for_scope(app.world_mut(), SessionScopeId(0)),
        None,
        "⛔ A CANDIDATE SESSION WHOSE ROUTE WAS CANCELLED IS STILL IN THE WORLD. \
         It was never published and never discarded, so its root, its first room \
         and its publication receipt are alive and unreachable forever"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::ReservedGameplayScopes>()
            .outstanding(),
        0,
        "the reservation ledger still holds a scope for a cancelled activation"
    );
    let held = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellRouteHolds>()
        .held(&ShellRouteId::new("ambition_gameplay"));
    assert!(
        held.is_empty(),
        "the cancelled route is still held by its candidate's gate: {held:?}"
    );
    // ⛔⛤ **AND NOTHING PROCESS-GLOBAL WAS INSTALLED BY PREPARING A CANDIDATE.**
    // This is report item 4 — "what remains OUTSIDE candidate ownership" —
    // asserted rather than argued. `SessionMechanics` is the generation's frozen
    // registries, a process-global RESOURCE installed by ADOPTION; a candidate
    // builder that wrote it during preparation would have changed live
    // mechanical state for a session that was never admitted.
    // `MovingPlatformSet`, the first room's platform state, is on the
    // candidate's own hidden live room root since OW1 cut 3b, so an abandoned
    // candidate leaves no VISIBLE live room at all. No session has ever been live in this arm, which is
    // what makes their ABSENCE meaningful.
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::construction::mechanics::SessionMechanics>()
            .is_none(),
        "⛔ PREPARING A CANDIDATE SESSION INSTALLED `SessionMechanics` PROCESS-WIDE.          The candidate was cancelled and never adopted, so the generation's frozen          registries belong to no session at all"
    );
    assert!(
        ambition_platformer2d::session::sole_live_room_component::<ambition_platformer2d::world::collision::MovingPlatformSet>(app.world())
            .is_none(),
        "⛔ PREPARING A CANDIDATE SESSION LEFT A VISIBLE LIVE ROOM, WITH ITS \
         MOVING PLATFORMS. The candidate was cancelled and never adopted, so \
         this is the mechanical state of a world nothing is playing"
    );
    assert_eq!(
        ambition_platformer2d::publication::outstanding_publications(app.world_mut()),
        0,
        "⛔ A PUBLICATION RECEIPT OUTLIVED ITS OPERATION. An `UntilOwnerRetires` \
         receipt is an entity that stands until its owner retires it, and a \
         cancelled candidate's owner is the discard"
    );
    // ⛔⛤ **A10'S INVARIANT AS A NUMBER, at a moment when nothing is in flight.**
    // A candidate that outlived its transaction is hidden by a DISABLING marker
    // from every ordinary query in the game, so it is invisible to exactly the
    // systems that would otherwise trip over it. This is the only thing that
    // looks.
    assert_eq!(
        ambition_platformer2d::platformer::construction::outstanding_candidates(app.world_mut()),
        0,
        "⛔ A HIDDEN CANDIDATE OUTLIVED ITS TRANSACTION after a cancelled route \
         settled: neither published nor discarded, and invisible to everything \
         but this count"
    );
}

/// ⛔⛤ **THE BEHAVIOURAL HALF OF REVIEW FINDING 2 / AUDIT FINDING 3, OWED SINCE
/// 2026-09-15 AND PAID HERE.**
///
/// A candidate needs TWO describers to rebuild a runtime-minted occurrence: the
/// ledger row saying WHERE it is, and the minted description saying WHAT it is.
/// Construction read the first from the candidate's own horizon and the second
/// from the LIVE `MintedItemBaseline` — B's whereabouts against A's
/// descriptions, a mixed-session durable horizon.
///
/// ⚠ **THE FIRST ATTEMPT AT THIS ARM DID NOT REACH ITS SUBJECT AND I WITHDREW
/// IT.** The mint reconstructed under NEITHER baseline, so it could not tell the
/// fix from the defect. What was missing was a fixture whose rows actually
/// describe something the start room reinstates, and the mechanism says exactly
/// what that takes: `AuthoredOccurrences::outlook_for` turns a
/// `Placed { room: <the room being built> }` row into `Reinstated`, no authored
/// record can settle a runtime mint's debt, and the fallback resolves
/// `description.held_item` through `held_spec_by_id`. So the row must name THIS
/// room, and the description must name an item the registry answers to —
/// `javelin` is one, and is the very id the reinstatement loop's own comment
/// records losing once.
///
/// ⭐ **A COLD START IS WHAT MAKES THE DISCRIMINATOR CLEAN.** The live baseline
/// is empty and the save is not, so the only thing that can describe M is the
/// candidate's own horizon. A `minted: None` poison at the construction site
/// takes this arm red.
#[test]
fn a_candidate_reconstructs_a_mint_only_its_own_save_describes() {
    use ambition_platformer2d::persistence::save_data::{
        PersistedMintedItem, PersistedOccurrence, PersistedWhereabouts,
    };
    use ambition_platformer2d::platformer::sim_id::SimId;

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();

    let minted_id = "minted:a10_candidate_javelin";
    {
        let mut save = app
            .world_mut()
            .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>();
        save.0.set_durable_horizon(
            vec![PersistedOccurrence::new(
                minted_id,
                PersistedWhereabouts::Placed {
                    room: "central_hub_complex".to_string(),
                    x: 96,
                    y: 96,
                },
            )],
            Vec::new(),
        );
        save.0.set_minted_items(vec![PersistedMintedItem {
            occurrence: minted_id.to_string(),
            // ⛔ A REAL PLACEMENT IN THIS ROOM. Construction refuses a mint whose
            // declared parent is "neither planned nor live" — measured, with an
            // invented id — and `placement:ground_gun_sword` is one
            // `central_hub_complex` authors.
            parent: "placement:ground_gun_sword".to_string(),
            sequence: 1,
            held_item: "javelin".to_string(),
        }]);
    }
    // ⭐ THE PREMISE ON THE OTHER SIDE: nothing live can describe M, so a
    // reconstruction can only have come from the candidate's own horizon. At a
    // cold start no session root exists, so no live baseline exists either.
    assert!(
        ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::item_runtime::pickup::minted_horizon::MintedItemBaseline,
        >(app.world())
        .is_none_or(|baseline| baseline.is_empty()),
        "the live minted baseline already describes something, so a rebuilt mint \
         would not prove which horizon supplied its description"
    );

    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: ShellRouteId::new("ambition_gameplay"),
        request: None,
    });

    // ⛔⛤ **THE SUBJECT IS THE CANDIDATE'S OWN POPULATION, WHILE IT IS STILL
    // HIDDEN — AND MEASURING THE END STATE INSTEAD IS HOW THE FIRST TWO
    // VERSIONS OF THIS ARM FAILED.** POISONED with `minted: None` at the
    // construction site, a query over the settled world STILL found the mint:
    // `complete_durable_restore` asks for a checkpoint resume whenever the save
    // carries rows, and that road rebuilds the room frames later from the LIVE
    // baseline, which has adopted the same file by then. So the settled world is
    // the union of both roads and can never tell them apart.
    //
    // ⭐ `candidate_carries_identity` asks the one question that can:
    // does the population behind the barrier — nothing else can see it —
    // already hold this identity, before anything was admitted or resumed.
    let wanted = SimId::from_snapshot(minted_id.to_string());
    let mut candidate_built_it = false;
    let mut pending_frames = 0usize;
    let mut window = crate::common::ActivationWindow::new("ambition_gameplay");
    for _ in 0.. {
        if window.closed(&app) {
            break;
        }
        app.update();
        if ambition_platformer2d::platformer::construction::outstanding_candidates(
            app.world_mut(),
        ) == 0
        {
            continue;
        }
        pending_frames += 1;
        candidate_built_it |=
            ambition_platformer2d::platformer::construction::candidate_carries_identity(
                app.world_mut(),
                &wanted,
            );
    }

    assert!(
        pending_frames > 0,
        "no candidate was ever outstanding, so nothing was built off to the side \
         and this arm says nothing"
    );
    assert!(
        candidate_built_it,
        "⛔ THE CANDIDATE DID NOT BUILD THE MINT ITS OWN SAVE DESCRIBES. Across \
         {pending_frames} frames in which a hidden candidate population existed, \
         none of it carried `{minted_id}` — the file says the object is lying in \
         the start room and says what it is, and the world the candidate \
         prepared does not contain it"
    );
}

/// How many `RoomLoaded` messages this composition has published since the arm
/// installed the counter.
#[derive(bevy::prelude::Resource, Default)]
struct RoomLoadsSeen(usize);

fn count_room_loads(
    mut loads: bevy::ecs::message::MessageReader<
        ambition_platformer2d::world::rooms::RoomLoaded,
    >,
    mut seen: bevy::prelude::ResMut<RoomLoadsSeen>,
) {
    seen.0 += loads.read().count();
}

fn live_scope(
    app: &bevy::prelude::App,
) -> Option<ambition_platformer2d::platformer::lifecycle::SessionScopeId> {
    app.world()
        .get_resource::<ambition_platformer2d::platformer::lifecycle::ActiveSessionScope>()
        .and_then(ambition_platformer2d::platformer::lifecycle::ActiveSessionScope::current)
}

// ─────────────────────────────────────────────────────────────────────────────
// CANDIDATE-GENERATION-ORDER: the candidate is prepared against ITS audio.
// ─────────────────────────────────────────────────────────────────────────────

fn provider_audio(app: &bevy::prelude::App) -> (bool, bool) {
    let catalogs = app
        .world()
        .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>();
    let provider = ambition_content::AMBITION_CONTENT_PROVIDER;
    (
        catalogs.music_for(provider).is_some(),
        catalogs.sfx_for(provider).is_some(),
    )
}

/// The reload holds still standing on the gameplay route (`content-publication:`
/// ones; the load presentation keeps its own ready-hold while a failure is on
/// screen).
fn reload_holds(app: &bevy::prelude::App) -> Vec<String> {
    app.world()
        .resource::<ambition_platformer2d::game_shell::ShellRouteHolds>()
        .held(&ShellRouteId::new("ambition_gameplay"))
        .iter()
        .map(|hold| format!("{hold:?}"))
        .filter(|hold| hold.contains("content-publication:"))
        .collect()
}

/// ⛔⛤ **A CANDIDATE ITS OWN PREPARATION REFUSES PUBLISHES NOTHING, AND THE
/// LIVE SESSION KEEPS EVERY FAMILY IT HAD.**
///
/// The provider EXPECTS its SFX and its adaptive cues, so a preparation that
/// sees a candidate without one fails ("... is not ready"). Preparation reads
/// the CANDIDATE's audio: read from the App's registry, which is N until the
/// commit, it would find the fragment, admit the session, and the commit would
/// publish a catalog without it. Nothing here fakes the boundary. On one
/// session:
///
/// 1. The candidate drops the SFX registry and edits the goblin cue binding,
///    the lab's cutscene banner and the first quest's title, with the player
///    on that quest's second step: none of it is published, the player keeps
///    their place, the selection stays N and the reload's hold is released.
/// 2. A candidate that drops the adaptive cues is refused the same way.
///
/// The control is `an_edited_pack_reaches_the_cast_the_shipped_composition_plays`,
/// where the same families, edited and not dropped, activate.
#[test]
fn a_candidate_its_preparation_refuses_leaves_every_live_family_at_n() {
    let mut app = a_running_shipped_session();
    advance_first_steps_one_step(&mut app);
    let audio_before = provider_audio(&app);
    assert!(audio_before.1, "the premise: N carries the provider's SFX registry");
    #[cfg(feature = "audio")]
    assert_eq!(goblin_binding(&app).1, "intro", "the premise: N starts the goblin cue on its intro");
    assert_eq!(lab_banner(&app), "// cutscene proof", "the premise: N's banner");
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let request_and_settle = |app: &mut bevy::prelude::App,
                              candidate: std::sync::Arc<ambition_platformer2d::content::PreparedContentPack>,
                              what: &str| {
        assert_ne!(candidate.fingerprint, base, "{what}: the premise: N+1 differs from N");
        let outcome = ambition_content::reload::request_reload(
            app.world_mut(),
            ambition_content::CandidateGeneration::prepared_against(candidate, Some(base)),
        );
        assert!(
            matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }),
            "{what}: refused at REQUEST time, so the preparation question never arises: {outcome:?}"
        );
        step_until_the_generation_settles(app);
        assert_eq!(
            activation_id(app),
            Some(live_activation),
            "{what}: ⛔ THE CANDIDATE ACTIVATED: its preparation was answered from the App's audio, which is still N"
        );
        assert_eq!(
            ambition_content::pack::selected(app.world()).expect("a selection").fingerprint,
            base,
            "{what}: a refused candidate became the App's selection"
        );
        assert!(ambition_content::reload::pending_pack(app.world()).is_none(), "{what}: a refused candidate is still pending");
        assert!(reload_holds(app).is_empty(), "{what}: a refused candidate's reload hold outlived it: {:?}", reload_holds(app));
    };

    // ── 1. without its SFX, and with three other families edited ────────────
    let mut edited = (false, false, false);
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_omitting_with(&["audio/sfx_registry.ron"], |declared, text| {
            let (from, to, flag) = match declared {
                "audio/music_cues.ron" => (r#"starting_state: "intro""#, r#"starting_state: "wave1""#, &mut edited.0),
                "data/cutscenes/sandbox.ron" => ("// cutscene proof", "// cutscene proof, revised", &mut edited.1),
                "data/quests.ron" => (r#"title: "First Steps""#, r#"title: "First Steps, revised""#, &mut edited.2),
                _ => return text,
            };
            let out = text.replacen(from, to, 1);
            *flag = out != text;
            out
        })
        .expect("a pack that stops declaring its SFX compiles"),
    );
    assert_eq!(edited, (true, true, true), "the pack no longer states the edited (cue binding, banner, quest title)");
    request_and_settle(&mut app, candidate, "without its SFX");
    assert_eq!(provider_audio(&app), audio_before, "⛔ A REFUSED CANDIDATE CHANGED THE LIVE AUDIO CATALOG");
    #[cfg(feature = "audio")]
    assert_eq!(goblin_binding(&app).1, "intro", "⛔ A REFUSED CANDIDATE'S CUE CATALOG WAS PUBLISHED");
    assert_eq!(lab_banner(&app), "// cutscene proof", "⛔ A REFUSED CANDIDATE'S CUTSCENES WERE PUBLISHED");
    assert_eq!(
        quest_place(&app, "first_steps"),
        Some(("First Steps".to_string(), true, 1)),
        "⛔ A REFUSED CANDIDATE CHANGED THE QUEST BOOK OR THE PLAYER'S PLACE IN IT"
    );

    // ── 2. without its adaptive cues ─────────────────────────────────────────
    #[cfg(feature = "audio")]
    {
        let candidate = std::sync::Arc::new(
            ambition_content::pack::compile_pack_omitting(&["audio/music_cues.ron"])
                .expect("a pack that stops declaring its cues compiles"),
        );
        request_and_settle(&mut app, candidate, "without its adaptive cues");
        assert_eq!(goblin_binding(&app).1, "intro", "⛔ A REFUSED CANDIDATE CHANGED THE LIVE CUES");
    }
}

/// A track whose file moves is played from its new file after the reload,
/// and a track the edit did not touch keeps the path it had.
#[cfg(feature = "audio")]
#[test]
fn a_reload_that_moves_a_tracks_file_plays_the_new_file() {
    let mut app = a_running_shipped_session();
    let base = ambition_content::pack::selected(app.world())
        .expect("a selection")
        .fingerprint;
    let path_of = |app: &bevy::prelude::App, id: &str| {
        app.world()
            .get_resource::<ambition_platformer2d::audio::library::AudioLibrary>()
            .expect("the gameplay app plays audio")
            .track(id)
            .map(|track| track.asset_path().to_string())
    };
    let untouched = path_of(&app, "first_goblin_tune_v2_radio");
    assert!(untouched.is_some(), "the premise: the library holds the untouched track");
    let mut edited = false;
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            if declared == "audio/music_registry.ron" {
                let out = text.replacen(
                    "asset_path: Some(\"audio/music/generated/lofi_study_loop/full.ogg\"),",
                    "asset_path: Some(\"audio/music/generated/lofi_study_loop/moved.ogg\"),",
                    1,
                );
                edited = out != text;
                return out;
            }
            text
        })
        .expect("compiles"),
    );
    assert!(edited, "the music registry no longer states the moved track");
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(candidate, Some(base)),
    );
    assert!(matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }), "{outcome:?}");
    step_until_the_generation_settles(&mut app);
    assert_eq!(
        path_of(&app, "original_lofi_loop").as_deref(),
        Some("audio/music/generated/lofi_study_loop/moved.ogg"),
        "the moved track is played from its old file"
    );
    assert_eq!(path_of(&app, "first_goblin_tune_v2_radio"), untouched, "an untouched track changed its path");
}

// ─────────────────────────────────────────────────────────────────────────────
// I2/I3: the adaptive music catalog takes part in a reload.
// ─────────────────────────────────────────────────────────────────────────────

/// The goblin lab's cue binding as the App's adaptive catalog states it.
#[cfg(feature = "audio")]
fn goblin_binding(app: &bevy::prelude::App) -> (String, String) {
    let binding = app
        .world()
        .resource::<ambition_platformer2d::audio::music::AdaptiveMusicCatalogRegistry>()
        .catalog_for(ambition_content::AMBITION_CONTENT_PROVIDER)
        .expect("the provider's adaptive catalog")
        .encounter_bindings()
        .iter()
        .find(|binding| binding.encounter_id == "goblin_encounter")
        .expect("the goblin lab binds a cue")
        .clone();
    (binding.cue_id, binding.starting_state)
}

// ─────────────────────────────────────────────────────────────────────────────
// I2/I3: the cutscene library takes part in a reload.
// ─────────────────────────────────────────────────────────────────────────────

/// The text of `cutscene_lab_intro`'s opening banner in the App's cutscene library.
fn lab_banner(app: &bevy::prelude::App) -> String {
    match app
        .world()
        .resource::<ambition_platformer2d::cutscene::CutsceneLibrary>()
        .get("cutscene_lab_intro")
        .expect("the lab's entry cutscene")
        .beats
        .first()
        .expect("a first beat")
    {
        ambition_platformer2d::cutscene::CutsceneBeat::Banner { text, .. } => text.clone(),
        other => panic!("cutscene_lab_intro no longer opens on a banner: {other:?}"),
    }
}

fn first_sfx_frequency_of(app: &bevy::prelude::App) -> f32 {
    app.world()
        .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>()
        .sfx_for(ambition_content::AMBITION_CONTENT_PROVIDER)
        .expect("the provider's SFX")
        .sfx[0]
        .frequency
}

// ─────────────────────────────────────────────────────────────────────────────
// I2/I3: the quest book takes part in a reload.
// ─────────────────────────────────────────────────────────────────────────────

/// The quest's place as the registry states it: its title, whether it is in
/// progress, and its step.
fn quest_place(app: &bevy::prelude::App, id: &str) -> Option<(String, bool, u8)> {
    app.world()
        .resource::<ambition_content::quest::QuestRegistry>()
        .get(id)
        .map(|state| (state.spec.title.clone(), state.is_active(), state.step))
}

/// A quest book whose `first_steps` loses its last two steps (so a player past
/// its first step has nowhere to stand), optionally with the SFX registry
/// dropped (a refusal downstream of admission).
fn pack_with_a_one_step_first_quest() -> std::sync::Arc<ambition_platformer2d::content::PreparedContentPack> {
    let mut edited = false;
    let pack = ambition_content::pack::compile_pack_with(|declared, text| {
        if declared == "data/quests.ron" {
            let out = text.replacen(
                r#"            (
                description: "Clear the goblin encounter.",
                condition: EncounterCleared("goblin_encounter"),
            ),
            (
                description: "Defeat the clockwork warden.",
                condition: BossDefeated("clockwork_warden"),
            ),
"#,
                "",
                1,
            );
            edited = out != text;
            return out;
        }
        text
    })
    .expect("a shorter quest compiles");
    assert!(edited, "the quest book no longer states the edited steps");
    std::sync::Arc::new(pack)
}

/// Move `first_steps` to its second step the way the game does: an advance
/// event, drained by the pump, mirrored into the save.
fn advance_first_steps_one_step(app: &mut bevy::prelude::App) {
    app.world_mut()
        .resource_mut::<ambition_content::quest::QuestRegistry>()
        .push_event(ambition_platformer2d::persistence::quest::QuestAdvanceEvent::FlagSet("met_any_hub_npc".to_string()));
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        quest_place(app, "first_steps").map(|(_, active, step)| (active, step)),
        Some((true, 1)),
        "the premise: first_steps stands on its second step"
    );
}

/// ⛔ **THE ACTIVATION GATE ASKS THE REQUEST-TIME QUESTIONS AGAIN WITH THE WORLD
/// AS IT IS AT THE ACTIVATION, AND CANCELS RATHER THAN CLAMPS OR OVERWRITES.**
/// Which precondition moved is `activation_refusal`'s answer
/// (`reload_tests::the_activation_refusal_names_the_precondition_that_moved`);
/// this asks the shipped shell to act on it. On one session:
///
/// 1. A candidate cutting `first_steps` to one step is admitted while the
///    player is on step 0. Before it activates the player reaches step 1, and
///    the book has no place for them: the generation is cancelled, its hold
///    released, and the player keeps their step in the live book.
/// 2. The control: the save moves the same way under a candidate that does not
///    touch the quest book (a reworded banner), and it activates with the
///    player's step.
/// 3. Another provider replaces the cutscene row (same id) a waiting candidate
///    rewords. The publication cannot refuse, so the gate cancels, and the
///    foreign row is still there.
#[test]
fn the_activation_gate_cancels_a_generation_whose_world_moved_while_it_waited() {
    use ambition_platformer2d::persistence::quest::QuestAdvanceEvent;
    let mut app = a_running_shipped_session();
    let advance = |app: &mut bevy::prelude::App, event: QuestAdvanceEvent| {
        app.world_mut().resource_mut::<ambition_content::quest::QuestRegistry>().push_event(event);
    };

    // ── 1. the save moves under a book that has no place for it ─────────────
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(pack_with_a_one_step_first_quest(), Some(base)),
    );
    assert!(
        matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }),
        "the premise: admitted while nobody has progressed: {outcome:?}"
    );
    assert!(ambition_content::reload::pending_pack(app.world()).is_some(), "the premise: it is in flight");
    advance(&mut app, QuestAdvanceEvent::FlagSet("met_any_hub_npc".to_string()));
    step_until_the_generation_settles(&mut app);
    assert_eq!(activation_id(&app), Some(live_activation), "⛔ THE GENERATION ACTIVATED OVER A SAVE IT HAS NO PLACE FOR");
    assert!(ambition_content::reload::pending_pack(app.world()).is_none(), "the refused generation is still pending");
    assert!(reload_holds(&app).is_empty(), "the refused generation's hold outlived it: {:?}", reload_holds(&app));
    assert_eq!(
        quest_place(&app, "first_steps").map(|(_, active, step)| (active, step)),
        Some((true, 1)),
        "the player was moved off their step"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_content::quest::QuestRegistry>()
            .get("first_steps")
            .map(|state| state.spec.steps.len()),
        Some(3),
        "the live book is not the one the player was admitted against"
    );

    // ── 2. the control: the same kind of move, no quest edit ────────────────
    let requested = request_the_banner(&mut app, "// a banner, no quest edit", false);
    assert!(matches!(requested, ambition_content::reload::ReloadRequest::Requested { .. }), "{requested:?}");
    advance(&mut app, QuestAdvanceEvent::EncounterCleared("goblin_encounter".to_string()));
    step_until_the_generation_settles(&mut app);
    assert_ne!(activation_id(&app), Some(live_activation), "a candidate with no quest edit was cancelled by a save that moved");
    assert_eq!(lab_banner(&app), "// a banner, no quest edit", "the candidate never published");
    assert_eq!(
        quest_place(&app, "first_steps").map(|(_, active, step)| (active, step)),
        Some((true, 2)),
        "the player's step did not survive the activation"
    );

    // ── 3. another provider takes the cutscene id ───────────────────────────
    let live_activation = activation_id(&app).expect("a live session");
    let requested = request_the_banner(&mut app, "// a banner the foreign row must survive", false);
    assert!(
        matches!(requested, ambition_content::reload::ReloadRequest::Requested { .. }),
        "the premise: admitted while the library held Ambition's own row: {requested:?}"
    );
    app.world_mut().resource_mut::<ambition_platformer2d::cutscene::CutsceneLibrary>().insert(
        ambition_platformer2d::cutscene::CutsceneScript::new(
            "cutscene_lab_intro",
            vec![ambition_platformer2d::cutscene::CutsceneBeat::Banner {
                text: "// the other provider".to_string(),
                seconds: 1.0,
            }],
        ),
    );
    step_until_the_generation_settles(&mut app);
    assert_eq!(activation_id(&app), Some(live_activation), "⛔ THE GENERATION ACTIVATED OVER ANOTHER PROVIDER'S CUTSCENE");
    assert!(ambition_content::reload::pending_pack(app.world()).is_none(), "the refused generation is still pending");
    assert_eq!(lab_banner(&app), "// the other provider", "⛔ THE FOREIGN ROW WAS OVERWRITTEN");
}

// ─────────────────────────────────────────────────────────────────────────────
// I3: a newer candidate supersedes the generation in flight.
// ─────────────────────────────────────────────────────────────────────────────

/// A pack that rewords the lab's banner to `text` (the observable the supersession
/// arms read) and optionally edits an SFX cue (a second family).
fn pack_with_the_lab_banner(text: &'static str, also_edit_sfx: bool) -> std::sync::Arc<ambition_platformer2d::content::PreparedContentPack> {
    let mut edited = false;
    let pack = ambition_content::pack::compile_pack_with(|declared, source| {
        if declared == "data/cutscenes/sandbox.ron" {
            let out = source.replacen("// cutscene proof", text, 1);
            edited = out != source;
            return out;
        }
        if also_edit_sfx && declared == "audio/sfx_registry.ron" {
            return source.replacen(
                "frequency: 460.0, frequency_end: 720.0,",
                "frequency: 461.0, frequency_end: 720.0,",
                1,
            );
        }
        source
    })
    .expect("an edited banner compiles");
    assert!(edited, "the cutscene file no longer states the edited banner");
    std::sync::Arc::new(pack)
}

fn request_the_banner(
    app: &mut bevy::prelude::App,
    text: &'static str,
    also_edit_sfx: bool,
) -> ambition_content::reload::ReloadRequest {
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(
            pack_with_the_lab_banner(text, also_edit_sfx),
            Some(base),
        ),
    )
}

/// ⭐ **A NEWER SAVE ENDS THE GENERATION IN FLIGHT, AND ONLY THE NEWEST DISK
/// STATE IS EVER SEEN.** Each time a candidate A (a reworded banner) is
/// requested and in flight; then, on one session:
///
/// 1. The content goes back to what is running (a revert): A is cancelled,
///    nothing replaces it, and the game never leaves N.
/// 2. The author saves B, which breaks the content graph: the lab's entry
///    cutscene is renamed (the room's binding would silently never play) and a
///    quest step names a boss that does not exist. B is refused by the judge
///    startup runs (`validate_content_graph` against the LDtk project the App
///    plays, so the shipped app must install that project or the judge is
///    skipped), the refusal names both faults, and A, which described a disk
///    that no longer exists, is cancelled.
/// 3. The author saves B, a valid edit: A is cancelled with its activation gate
///    forgotten, and B activates alone with its second family (an SFX cue).
///
/// A's banner is never seen on any frame.
#[test]
fn a_newer_save_ends_the_generation_in_flight_and_the_old_one_is_never_seen() {
    let mut app = a_running_shipped_session();
    let live_activation = activation_id(&app).expect("a live session");
    let sfx_before = first_sfx_frequency_of(&app);
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    // Step until the shell has no transaction in flight and `done` holds, then
    // a few frames more, holding on each that the banner is never `never`.
    let step_until_settled = |app: &mut bevy::prelude::App, never: &str, done: &dyn Fn(&bevy::prelude::App) -> bool| {
        let started = std::time::Instant::now();
        let mut after = 0;
        while after < 10 {
            assert!(started.elapsed() < crate::common::ACTIVATION_CEILING, "the shell did not settle in {:?}", started.elapsed());
            app.update();
            assert_ne!(lab_banner(app), never, "⛔ A CANDIDATE THAT WAS ENDED HAD ITS CUTSCENES PUBLISHED");
            let done = done(app);
            if app.world().resource::<ShellRouter>().pending.is_none() && done {
                after += 1;
            }
        }
    };
    let request_a = |app: &mut bevy::prelude::App, text: &'static str| {
        let a = request_the_banner(app, text, false);
        assert!(matches!(a, ambition_content::reload::ReloadRequest::Requested { .. }), "{a:?}");
        app.update();
        assert!(ambition_content::reload::pending_pack(app.world()).is_some(), "the premise: A is in flight");
    };

    // ── 1. a revert ──────────────────────────────────────────────────────────
    request_a(&mut app, "// A, which is reverted");
    let live = std::sync::Arc::new(ambition_content::pack::compile_pack_with(|_, text| text).expect("compiles"));
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(live, Some(base)),
    );
    assert!(matches!(outcome, ambition_content::reload::ReloadRequest::CancelledInFlight { .. }), "{outcome:?}");
    step_until_settled(&mut app, "// A, which is reverted", &|_| true);
    assert_eq!(activation_id(&app), Some(live_activation), "the reverted candidate's session activated");
    assert!(ambition_content::reload::pending_pack(app.world()).is_none(), "the reverted candidate is still pending");

    // ── 2. a save that breaks the content graph ─────────────────────────────
    request_a(&mut app, "// A, which the refused save replaces");
    let mut edited = (false, false);
    let broken = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            let (from, to, flag) = match declared {
                "data/cutscenes/sandbox.ron" => {
                    (r#"id: "cutscene_lab_intro","#, r#"id: "cutscene_lab_intro_renamed","#, &mut edited.0)
                }
                "data/quests.ron" => (r#"BossDefeated("clockwork_warden")"#, r#"BossDefeated("no_such_boss")"#, &mut edited.1),
                _ => return text,
            };
            let out = text.replacen(from, to, 1);
            *flag = out != text;
            out
        })
        .expect("a pack with a renamed script and an unknown boss compiles"),
    );
    assert_eq!(edited, (true, true), "the pack no longer states the (renamed script, edited boss)");
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(broken, Some(base)),
    );
    let ambition_content::reload::ReloadRequest::Refused(ambition_content::reload::MoveReload::ContentGraphRefused(errors)) =
        outcome
    else {
        panic!("a candidate that breaks the content graph was not refused as such: {outcome:?}");
    };
    assert!(
        errors.iter().any(|error| error.contains("cutscene_lab_intro") && error.contains("unknown cutscene")),
        "the refusal does not name the dead cutscene binding: {errors:?}"
    );
    assert!(
        errors.iter().any(|error| error.contains("first_steps") && error.contains("no_such_boss")),
        "the refusal does not name the quest and the boss: {errors:?}"
    );
    assert!(
        ambition_content::reload::pending_pack(app.world()).is_none(),
        "⛔ A STAYED PENDING BESIDE THE REFUSAL OF THE SAVE THAT REPLACED IT"
    );
    step_until_settled(&mut app, "// A, which the refused save replaces", &|_| true);
    assert_eq!(activation_id(&app), Some(live_activation), "the cancelled generation activated");
    assert_eq!(
        ambition_content::pack::selected(app.world()).expect("a selection").fingerprint,
        base,
        "a refused candidate became the App's selection"
    );

    // ── 3. a newer valid save ────────────────────────────────────────────────
    let a = request_the_banner(&mut app, "// A, which is superseded", false);
    let ambition_content::reload::ReloadRequest::Requested { request: a_request, superseded: None, .. } = a else {
        panic!("A was not requested: {a:?}");
    };
    let a_hold =
        ambition_platformer2d::game_shell::ShellHoldId::new(format!("content-publication:{}", a_request.as_str()));
    // Two frames: A's transaction exists and is ADOPTED (it holds the route and
    // has a claim), and it has not activated.
    app.update();
    app.update();
    assert!(ambition_content::reload::pending_pack(app.world()).is_some(), "the premise: A is in flight");
    assert!(!reload_holds(&app).is_empty(), "the premise: A has been adopted and holds the route");
    assert_eq!(activation_id(&app), Some(live_activation), "the premise: A has not activated");

    let b = request_the_banner(&mut app, "// B, which wins", true);
    let ambition_content::reload::ReloadRequest::Requested { superseded: Some(_), .. } = b else {
        panic!("B did not supersede A: {b:?}");
    };
    // A registration that outlives its transaction is a hold nobody will ever
    // release.
    assert!(
        app.world().resource::<ambition_platformer2d::game_shell::ShellActivationGates>().evaluator(&a_hold).is_none(),
        "⛔ THE SUPERSEDED GENERATION'S ACTIVATION GATE WAS LEFT REGISTERED"
    );
    let activations = std::cell::RefCell::new(std::collections::BTreeSet::from([Some(live_activation)]));
    step_until_settled(&mut app, "// A, which is superseded", &|app| {
        activations.borrow_mut().insert(activation_id(app));
        activation_id(app) != Some(live_activation)
    });
    assert_eq!(lab_banner(&app), "// B, which wins", "the newest candidate never published");
    assert_eq!(first_sfx_frequency_of(&app), sfx_before + 1.0, "B's second family did not arrive with it");
    assert_eq!(
        activations.borrow().len(),
        2,
        "the shell activated {:?}: the live session and B's, and no session of A's",
        activations.borrow()
    );
    assert!(ambition_content::reload::pending_pack(app.world()).is_none(), "a generation is still pending");
    assert!(reload_holds(&app).is_empty(), "a cancelled generation's hold outlived it: {:?}", reload_holds(&app));
}
