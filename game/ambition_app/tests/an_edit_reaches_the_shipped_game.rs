//! An edit on disk reaches the game the SHIPPED composition plays.
//!
//! ⛔⛤ **THIS IS THE ONE HOP EVERY OTHER RELOAD WITNESS LEAVES OUT, AND THE
//! QUEUE SAYS SO IN ITS OWN RECEIPT:** *"STILL CRATE-LEVEL, not through
//! `build_visible_app`; the app-level version is what would witness the one hop
//! the identity packet could not."* The crate-level arms in
//! `ambition_content::reload` hand-build a world, insert a `ShellRouter`, and add
//! `publish_staged_reload_on_activation` themselves — so every one of them stays
//! green if the shipped composition never registers the system, never reaches an
//! active route, or registers a route with no preparation plan. This one asks the
//! real `build_visible_app`.
//!
//! ⭐⭐ **AND IT WATCHES THE FIGHTER LADDER RATHER THAN A MOVESET, WHICH IS A
//! MEASURED CHOICE AND NOT A CONVENIENCE.** MEASURED 2026-09-12 and recorded in
//! `queue.md`: the body the shipped `ambition_gameplay` route constructs carries
//! eight moves and every one of them is a DERIVED KIT move, while the authored
//! `moveset` pack holds a different vocabulary and names no player robot. Their
//! intersection is EMPTY. So a moveset edit is the wrong subject here — it would
//! prove the transaction ran and show nothing the shipped route plays.
//! `fighter_brain_ladder` is the second content family, it IS installed by
//! `AmbitionContentPlugin::build` in this composition, and it is republished at
//! the activation boundary.
//!
//! ⚠ THE EDIT IS BUILT IN MEMORY, NOT WRITTEN TO THE TREE. A guard whose subject
//! MUTATES THE REPOSITORY is one this project has already been bitten by;
//! `compile_pack_with` is the road that makes the edited pack without touching
//! `game/ambition_content/assets`.

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId, ShellRouter};

/// Level one's reaction latency, as the LIVE composition holds it.
fn live_first_rung(app: &bevy::prelude::App) -> f32 {
    app.world()
        .resource::<ambition_platformer2d::characters::brain::fighter::AuthoredFighterLadder>()
        .0
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

fn active_route(app: &bevy::prelude::App) -> Option<String> {
    app.world()
        .get_resource::<ShellRouter>()
        .and_then(|router| router.active.as_ref())
        .map(|active| active.route_id.as_str().to_string())
}

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
    let mut settled = None;
    for _ in 0..240 {
        app.update();
        if active_route(&app).as_deref() == Some("ambition_gameplay") {
            settled = active_route(&app);
            break;
        }
    }
    assert_eq!(
        settled.as_deref(),
        Some("ambition_gameplay"),
        "the shipped composition never activated its gameplay route, so nothing \
         below is about a reload"
    );

    // ── the premise: both live families are the shipped ones ────────────────
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

    // ── an edit, in memory, TOUCHING TWO FAMILIES AT ONCE ───────────────────
    // ⭐⭐ **TWO FAMILIES IN ONE CANDIDATE IS THE POINT, AND IT IS THE 2026-09-12
    // REVIEW'S FINDING 4.** The crate-level arms proved each family publishes,
    // but they INJECTED `ShellEvent::RouteActivated` — so "the transaction is
    // atomic across families" was asserted about a boundary the test itself
    // fired. Here one edit moves the ladder AND the wave book, nothing but
    // `update()` drives the shell, and both must land on the SAME frame as the
    // activation. A transaction that published one family and deferred the other
    // would build a world half of generation N and half of N+1.
    let mut edited_ladder = false;
    let mut edited_waves = false;
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| match declared {
            "data/fighter_brain_ladder.ron" => {
                let out = text.replacen("reaction_ms: 500.0", "reaction_ms: 499.0", 1);
                edited_ladder = out != text;
                out
            }
            "data/encounters/goblin_encounter.ron" => {
                let out = text.replacen("delay: 0.70", "delay: 0.75", 1);
                edited_waves = out != text;
                out
            }
            _ => text,
        })
        .expect("the edited pack compiles"),
    );
    // ⛔ A FLOOR ON EACH EDIT SEPARATELY. A retuned value or a renamed source
    // leaves one closure arm a no-op, and a single combined flag would let the
    // family that still changed carry the arm for the one that did not.
    assert!(
        edited_ladder,
        "`data/fighter_brain_ladder.ron` no longer carries `reaction_ms: 500.0`, \
         so the ladder half of this witness is vacuous"
    );
    assert!(
        edited_waves,
        "`data/encounters/goblin_encounter.ron` no longer carries `delay: 0.70`, \
         so the encounter-wave half of this witness is vacuous"
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
        "the shipped composition refused a ladder-only reload: {outcome:?}"
    );
    assert_eq!(
        live_first_rung(&app),
        500.0,
        "the REQUEST published on the spot instead of staging — the old \
         generation must stay authoritative until the new one activates"
    );
    assert_eq!(
        live_second_goblin_delay(&app),
        0.70,
        "the REQUEST published the wave book on the spot instead of staging"
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
    // ⭐ NOTHING IS HAND-DRIVEN HERE. The crate-level arms write
    // `ShellEvent::RouteActivated` themselves; this one only calls `update()`,
    // so the router mints the transaction, the provider re-prepares, and the
    // activation authorizes the publication exactly as a running game does.
    //
    // ⛔⛔⛔ **THE FAMILY MUST LAND ON THE SAME FRAME AS THE ACTIVATION, AND
    // "EVENTUALLY" IS WHAT HID A PRODUCTION BUG.** The first version of this
    // test looped up to 240 updates and asserted only the final value — so it
    // passed while the publication arrived ONE FRAME AFTER the world was built
    // from it. MEASURED 2026-09-12 before the fix: the activation id moved on
    // frame 2, `[sprite-bind] worn character` (the session provider constructing
    // the new world) logged on frame 2, and the ladder moved on frame **3**.
    //
    // ⇒ A tolerance is a claim that the gap does not matter. Here the gap IS
    // the defect: a publication that lands after the world is built publishes
    // into a world already built without it. This asserts the frame, not the
    // outcome.
    //
    // ⚠ **THE MECHANISM NAMED HERE UNTIL 2026-09-19 NO LONGER EXISTS**, and it
    // was the same sentence `reload.rs` carried — one fact with two owners, so
    // the correction had to be made twice. It read
    // *"`activate_prepared_platformer_sessions` reads `PreparedCharacterRegistry`
    // inside `GameplaySessionSet::Providers` on the activation frame"*: that
    // system was deleted by A10.5 (`c89c68747`), construction moved EARLIER to
    // `prepare_candidate_platformer_session`, and the builder holds no registry
    // field — the cast arrives as a `PreparedContent` argument. ⛔ The frame
    // this test pins is therefore still the right thing to pin, but the reason
    // it matters is now an open question rather than a settled one; see
    // CANDIDATE-GENERATION-ORDER in `docs/planning/queue.md`.
    let activation_before = activation_id(&app);
    let mut activated_on = None;
    for frame in 0..240 {
        app.update();
        if activation_id(&app) != activation_before {
            activated_on = Some(frame);
            break;
        }
    }
    let activated_on = activated_on.expect(
        "the shipped shell never re-activated the route, so the reload's \
         transaction never reached its boundary",
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
    // ⛔⛔ **AND THE CAST GENERATION MUST *NOT* MOVE, WHICH IS THE OPPOSITE OF
    // WHAT I FIRST ASSERTED HERE — THE TEST FOUND IT.** This candidate edits the
    // fighter ladder and the encounter waves and no move table, so
    // `moveset_changed` is false, `request_reload` stages no cast, and
    // `publish_admitted_revision` never runs. MEASURED: the arm asserting the
    // generation MOVED failed with `CharacterCatalogGeneration(1)` on both sides.
    //
    // ⇒ That is the 2026-09-12 review's item 5 holding in production: the
    // transaction is no longer moveset-centric, so a family that did not change
    // is not re-published and does not consume a cast generation. An arm that
    // demanded the cast clock advance would be demanding the defect back.
    let cast_after = app
        .world()
        .resource::<ambition_platformer2d::character::PreparedCharacterRegistry>()
        .generation();
    assert_eq!(
        cast_after, cast_before,
        "a generation that changes NO move table still moved the cast, so every \
         ladder or waves edit re-publishes the whole cast and the transaction is \
         still moveset-centric"
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
        .resource::<ambition_platformer2d::actors::features::LastConstructionVerification>()
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
}

/// ⛔⛤ **DOES THE SHIPPED APP EVER HOLD TWO `SessionRoot`s? MEASURED, BECAUSE
/// EVERY `SessionWorldRef`/`SessionWorldMut` SITE DEPENDS ON THE ANSWER AND
/// NOBODY HAD ASKED IT.**
///
/// ⚠ **THE SIZE OF THAT POPULATION IS NOT THIS ARM'S TO STATE**, and it used to
/// be: the docstring carried *"217 references across 108 files at HEAD"* while
/// the census row `BEVY-SESSION-ROOT` carried 193, and on 2026-09-19 the tree
/// answered 183 or 199 depending on whether comments and test modules are
/// stripped. Four numbers, two owners, none of them current. The count now lives
/// once, with its method, in
/// `docs/planning/consolidation/architecture-census.md`; this arm needs only
/// that the population is large and that every member is a `Single`.
///
/// `SessionWorldRef` / `SessionWorldMut` are `Single<.., With<SessionRoot>>`, and
/// `Single` matches only when there is EXACTLY ONE. Meanwhile
/// `live_session_world_root` deliberately selects the root owned by
/// `ActiveSessionScope`, *"so a lingering retired root is not a candidate rather
/// than an ambiguity"*. ⇒ Two ownership semantics for one fact, and a 2026-09-13
/// review said candidate coexistence would make the ordinary one ambiguous.
///
/// ⭐ **`Q132` DECIDED IT ON 2026-09-19: there is exactly one canonical live
/// `SessionRoot`.** So the `Single` reading is the engine's meaning and a
/// two-root frame is INVALID rather than ambiguous — which makes this arm a
/// witness of an invariant instead of a measurement of a risk. Its companion
/// `a_prepared_candidate_never_counts_as_a_canonical_session_root` holds the
/// other half the ruling asks for.
///
/// ⭐⭐ **BUT "TWO ROOTS CAN EXIST" IS A CLAIM ABOUT THIS COMPOSITION, NOT A
/// THEOREM — AND IT IS CHEAPER TO MEASURE THAN TO DESIGN AROUND.** The one
/// recorded occurrence was a BUILD-TIME root coexisting with an activation's, and
/// that root is gone (`app/resources.rs` records its removal). The other source is
/// a retired scope's root surviving into the next activation, which
/// `SessionScopeSet`'s `RetireAuthority -> Cleanup -> Activate` ordering closed
/// on 2026-09-13.
///
/// ⇒ This drives a real shell handoff — the road whose world log shows
/// `session-end` and `session-start` on ONE frame — and counts roots every frame.
/// If the count never exceeds one, `Single` is unambiguous in production and no
/// site is exposed today; if it does, this arm names the frame.
#[test]
fn the_shipped_app_never_holds_two_session_roots_across_a_handoff() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();

    let gameplay = ShellRouteId::new("ambition_gameplay");
    let roots_now = |app: &mut bevy::prelude::App| -> usize {
        let world = app.world_mut();
        let mut query = world
            .query::<&ambition_platformer2d::platformer::lifecycle::SessionRoot>();
        query.iter(world).count()
    };

    app.world_mut()
        .write_message(ShellCommand::ReplaceWith {
            route: gameplay.clone(),
            request: None,
        });

    let mut seen_one = false;
    let mut worst = 0usize;
    let mut worst_frame = 0usize;
    for frame in 0..240 {
        app.update();
        let roots = roots_now(&mut app);
        if roots > worst {
            worst = roots;
            worst_frame = frame;
        }
        if roots == 1 {
            seen_one = true;
        }
    }

    // ⚠ THE PREMISE, and without it "never two" is satisfied by "never one":
    // a run that failed to activate any session at all would pass silently.
    assert!(
        seen_one,
        "no session root ever appeared in 240 frames, so this measured a route \
         that never activated rather than a handoff"
    );

    // ── the handoff: replace the live route with itself, which is the road the
    //    world log shows retiring and activating on ONE frame ──────────────────
    let activation_before = activation_id(&app);
    app.world_mut()
        .write_message(ShellCommand::ReplaceWith {
            route: gameplay,
            request: None,
        });
    let mut handoff_worst = 0usize;
    let mut handoff_frame = 0usize;
    for frame in 0..240 {
        app.update();
        let roots = roots_now(&mut app);
        if roots > handoff_worst {
            handoff_worst = roots;
            handoff_frame = frame;
        }
    }

    // ⚠ **AND THE SECOND PREMISE, WHICH IS THE ONE THAT MATTERS HERE:** "never
    // two roots during a handoff" is trivially true of a handoff that never
    // happened. The activation id must have MOVED.
    let activation_after = activation_id(&app);
    assert!(
        activation_before.is_some() && activation_after != activation_before,
        "the activation id did not move across the `ReplaceWith` ({activation_before:?} \
         -> {activation_after:?}), so the second half measured no handoff at all"
    );

    assert!(
        worst <= 1 && handoff_worst <= 1,
        "THE SHIPPED APP HOLDS {worst} SESSION ROOT(S) (first activation, frame \
         {worst_frame}) and {handoff_worst} (handoff, frame {handoff_frame}). \
         `SessionWorldRef`/`SessionWorldMut` are `Single<.., With<SessionRoot>>` \
         at every site, and `Single` matches NOTHING when the count is not one — so \
         on that frame every one of those systems is silently skipped while \
         `live_session_world_root` would have resolved the live root by scope. \
         That is the two-semantics gap, and this names the frame it opens on."
    );
}

/// Which canonical identities in the SHIPPED VISIBLE COMPOSITION have no session
/// owner, across a handoff.
///
/// ⚠ **THE COMPOSITION IS THE POINT.** Its sibling in
/// `walking_into_a_loading_zone` censuses the `fixed_60hz_sim` harness, which is
/// a different program: a claim about "the shipped app" made there is a claim
/// about the wrong population. This one boots `build_visible_app` and samples
/// after a handoff, because the question it exists to answer — why three
/// identities read as unowned to a candidate session's first room — was asked
/// during a handoff in this composition.
#[test]
#[ignore = "probe: census of canonical identities with no session owner, visible composition"]
fn probe_process_resident_canonical_identities_in_the_visible_app() {
    fn census(app: &mut bevy::prelude::App, when: &str) {
        let world = app.world_mut();
        let mut q = world.query::<(
            &ambition_platformer2d::platformer::sim_id::SimId,
            Option<&ambition_platformer2d::platformer::lifecycle::SessionScopedEntity>,
        )>();
        let mut rows: Vec<(String, Option<u64>)> = q
            .iter(world)
            .map(|(id, owner)| (id.as_str().to_string(), owner.map(|owner| owner.0 .0)))
            .collect();
        rows.sort();
        let unscoped = rows.iter().filter(|(_, owner)| owner.is_none()).count();
        eprintln!(
            "[probe] {when}: {unscoped} of {} canonical identities carry NO session owner",
            rows.len()
        );
        for (id, owner) in rows.iter().filter(|(_, owner)| owner.is_none()) {
            eprintln!("[probe]   UNSCOPED {id} {owner:?}");
        }
        for (id, owner) in rows.iter().filter(|(id, _)| {
            id.starts_with("encounter:") || id.starts_with("slot:") || id.starts_with("session:")
        }) {
            eprintln!("[probe]   {id} -> scope {owner:?}");
        }
    }

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();
    let gameplay = ShellRouteId::new("ambition_gameplay");
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay.clone(),
        request: None,
    });
    for _ in 0..240 {
        app.update();
    }
    census(&mut app, "first session live");

    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay,
        request: None,
    });
    for _ in 0..240 {
        app.update();
    }
    census(&mut app, "after a handoff");
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
    for _ in 0..240 {
        app.update();
    }
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

/// ⛔⛤ **THE DEV WORLD RELOAD, AT LAST WITNESSED — AND IN BOTH DIRECTIONS.**
///
/// A10.3 moved every effect of the LDtk hot reload behind its publication's
/// exact verdict: the body transit, the dialog close, the combat and cooldown
/// resets, the developer flash and the presentation spawns. That landed
/// compile-verified and REASONED, because this road had no end-to-end coverage in
/// either direction and the queue row said so for a day.
///
/// ⭐ **NO FILE IS WRITTEN.** Pressing Apply re-reads the same project, which is
/// an equivalent reload: it still prepares a candidate, still builds the room as
/// hidden candidates, and still publishes it — the whole A10 bracket, without
/// touching a shared tree's content.
#[test]
fn a_committed_world_reload_applies_its_effects() {
    let mut app = a_running_shipped_session();
    let before = app
        .world()
        .resource::<ambition_platformer2d::dev_tools::WorldSourceHotReload>()
        .applied_count;

    press_apply_reload(&mut app);

    let reload = app
        .world()
        .resource::<ambition_platformer2d::dev_tools::WorldSourceHotReload>()
        .clone();
    assert!(
        reload.applied_count > before,
        "the reload did not apply, so this arm says nothing about what a \
         COMMITTED reload does: {:?} / {:?}",
        reload.last_status,
        reload.last_errors
    );
    assert_eq!(
        dev_preset_flash(&app),
        1.0,
        "a committed world reload did not flash the developer overlay — either \
         its effects no longer run behind the verdict at all, or this \
         discriminator is dead and the refusal arm below certifies nothing"
    );
    // ⛔⛤ AND NO RECEIPT OUTLIVES ITS OPERATION ON THE ORDINARY ROAD EITHER. The
    // session handoff, the first room and this reload each began a publication;
    // every one of them is `UntilOwnerRetires`, so every one owes a
    // `retire_publication`. MEASURED here rather than argued from five call
    // sites — which is exactly how the candidate slot's missing exits hid.
    assert_eq!(
        ambition_platformer2d::actors::rooms::outstanding_publications(app.world_mut()),
        0,
        "a publication receipt is still standing after a committed reload settled"
    );
    assert_eq!(
        ambition_platformer2d::platformer::construction::outstanding_candidates(app.world_mut()),
        0,
        "⛔ A HIDDEN CANDIDATE OUTLIVED ITS TRANSACTION after a committed reload \
         settled: neither published nor discarded"
    );
    // ⛔⛤ **AND NO DUPLICATE AUTHORITATIVE IDENTITY IN THE PUBLISHED WORLD — the
    // success arm's half of A10's acceptance scenario.** Asked of the SAME
    // authority publication is verified against rather than of a census written
    // for the test: `TransactionBaseline::capture` is precisely the operation
    // that cannot describe a world holding one identity twice, and
    // `BaselineCaptureError::DuplicateIdentity` is the refusal these arms induce
    // ON PURPOSE elsewhere. ⇒ Its control is
    // `a_refused_world_reload_leaves_the_running_game_untouched`, whose two
    // process-resident twins make this exact call fail.
    assert!(
        ambition_platformer2d::platformer::construction::TransactionBaseline::capture(
            app.world_mut()
        )
        .is_ok(),
        "⛔ THE WORLD A COMMITTED RELOAD PUBLISHED CANNOT BE DESCRIBED: some \
         identity has two authoritative holders, which is the one thing \
         publication must never produce"
    );
    assert!(
        reload.last_status.contains("applied") && reload.last_errors.is_empty(),
        "the status a developer reads does not say a committed reload applied: \
         {:?} / {:?}",
        reload.last_status,
        reload.last_errors
    );
    // ⭐ THE CONTROL FOR THE REFUSAL ARM'S ROLLBACK ASSERTION. A committed reload
    // DOES release the local baseline — `restart_local_ggrs_after_hot_reload`
    // stops the session in the same frame's `PostUpdate` and the session owner
    // rebases it. Without this, "a refused reload left the timeline alone" would
    // be equally true of a build where nothing ever touches it.
    assert!(
        !ambition_platformer2d::rollback::session_is_active(&app.world()),
        "a committed world reload did not release the local rollback baseline, \
         so the refusal arm's rollback assertion has no discriminating power"
    );
}

/// ⛔⛤ **AND A REFUSED RELOAD COSTS THE RUNNING GAME NOTHING.**
///
/// The refusal is a production one: two process-resident holders of one identity
/// make a world `TransactionBaseline::capture` cannot describe, so the reload's
/// candidate room cannot be verified. Nothing test-only is wired into
/// construction, and nothing is written to disk.
///
/// ⭐ Its control is the arm above, which proves the flash moves on a reload that
/// DOES commit — without it, "the flash did not move" would also be true of a
/// reload that never happened.
#[test]
fn a_refused_world_reload_leaves_the_running_game_untouched() {
    let mut app = a_running_shipped_session();
    let before_room = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(app.world())
    .map(|rooms| rooms.active_spec().id.clone())
    .expect("the running session carries a room set");
    let before_applied = app
        .world()
        .resource::<ambition_platformer2d::dev_tools::WorldSourceHotReload>()
        .applied_count;

    // A world that cannot be described: two holders of one canonical identity,
    // process-resident so they are in the candidate's world too.
    for _ in 0..2 {
        app.world_mut()
            .spawn(ambition_platformer2d::platformer::sim_id::SimId::placement(
                "corrupt_twin",
            ));
    }

    assert!(
        ambition_platformer2d::rollback::session_is_active(app.world()),
        "the running session has no rollback timeline, so the assertion below \
         about a refused reload not tearing one down says nothing"
    );

    press_apply_reload(&mut app);

    let verdict = app
        .world()
        .resource::<ambition_platformer2d::actors::features::LastConstructionVerification>()
        .clone();
    assert!(
        !verdict.published,
        "the reload's room was not refused, so this arm is about a committed \
         reload rather than a refused one: {verdict:?}"
    );
    // ⭐ THE PREMISE, ASSERTED: the reload ROAD ran and recorded a refusal of its
    // own. Without this the arm below is equally true of a reload that never
    // happened — a missing player, an unset watch path, a silent early return.
    let reload = app
        .world()
        .resource::<ambition_platformer2d::dev_tools::WorldSourceHotReload>()
        .clone();
    assert!(
        !reload.last_errors.is_empty() && reload.last_status.contains("rejected"),
        "the hot reload did not record a refusal, so nothing here is a statement \
         about a REFUSED reload: {:?} / {:?}",
        reload.last_status,
        reload.last_errors
    );
    assert_eq!(
        dev_preset_flash(&app),
        0.0,
        "⛔ A REFUSED WORLD RELOAD RAN ITS EFFECTS. The flash is the cheapest of \
         them; the same verdict gates the body transit, the dialogue close, the \
         combat and cooldown resets and the presentation spawns"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::dev_tools::WorldSourceHotReload>()
            .applied_count,
        before_applied,
        "a refused reload counted itself as applied"
    );
    assert_eq!(
        ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::world::rooms::RoomSet,
        >(app.world())
        .map(|rooms| rooms.active_spec().id.clone()),
        Some(before_room),
        "⛔ A REFUSED WORLD RELOAD CHANGED THE ROOM THE PLAYER IS IN"
    );
    // ⛔⛤ MEASURED 2026-09-15, AND IT WAS FALSE: a refused reload stopped the
    // live GGRS session, because the restart was requested before a single root
    // was built. The world it was playing was intact and its rollback timeline
    // was torn down for a room that does not exist. Its control is the arm above,
    // which shows a COMMITTED reload does rebase the baseline.
    assert!(
        ambition_platformer2d::rollback::session_is_active(app.world()),
        "⛔ A REFUSED WORLD RELOAD TORE DOWN THE ROLLBACK TIMELINE OF THE WORLD \
         IT LEFT STANDING"
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
/// ⭐ **THE CONTROL IS `a_shell_handoff_publishes_the_incoming_sessions_room`**,
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
    fn population(app: &mut bevy::prelude::App, owner: SessionScopeId) -> usize {
        let world = app.world_mut();
        world
            .query::<&SessionScopedEntity>()
            .iter(world)
            .filter(|scoped| scoped.0 == owner)
            .count()
    }
    fn live_room(app: &mut bevy::prelude::App) -> Option<String> {
        ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::world::rooms::RoomSet,
        >(app.world())
        .map(|rooms| rooms.active_spec().id.clone())
    }

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();

    let gameplay = ShellRouteId::new("ambition_gameplay");
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay.clone(),
        request: None,
    });
    for _ in 0..240 {
        app.update();
    }

    // ── world N, playable ────────────────────────────────────────────────────
    let live_activation = activation_id(&app).expect("the first session activated");
    let live = scope(&app).expect("the live session owns a scope");
    let before_population = population(&mut app, live);
    let before_room = live_room(&mut app);
    assert!(
        before_population > 0 && before_room.is_some(),
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
        app.world_mut()
            .resource_mut::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>()
            .adopt(remembered);
    }

    // A's MECHANICAL state as it stands while A is the only session. The
    // candidate carries its own `SessionMechanics` and `MovingPlatformSet` and
    // installs them at adoption; a refused candidate must leave A's alone.
    let mechanics_before = app
        .world()
        .get_resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
        .map(|mechanics| format!("{mechanics:?}"));
    let platforms_before = app
        .world()
        .resource::<ambition_platformer2d::world::collision::MovingPlatformSet>()
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
        app.world()
            .get_resource::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>()
            .cloned(),
        app.world()
            .get_resource::<ambition_platformer2d::platformer::lifecycle::CustodyBaseline>()
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
    for _ in 0..240 {
        app.update();
    }

    // ⛔ THE PREMISE, BEFORE ANY CONCLUSION: a room transaction actually RAN and
    // was REFUSED. Without this, "the handoff did not happen" is equally true of
    // a route that never became ready, and every assertion below would be
    // satisfied by an app that simply did nothing.
    let verdict = app
        .world()
        .resource::<ambition_platformer2d::actors::features::LastConstructionVerification>()
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
    assert_eq!(
        population(&mut app, live),
        before_population,
        "⛔ A REFUSED CANDIDATE SESSION COST WORLD N ENTITIES"
    );
    assert_eq!(
        live_room(&mut app),
        before_room,
        "⛔ A REFUSED CANDIDATE SESSION CHANGED THE ROOM THE PLAYER IS IN"
    );

    assert_eq!(
        app.world()
            .get_resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
            .map(|mechanics| format!("{mechanics:?}")),
        mechanics_before,
        "⛔ A REFUSED CANDIDATE CHANGED THE LIVE SESSION'S MECHANICS"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::world::collision::MovingPlatformSet>()
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
        app.world()
            .get_resource::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>()
            .cloned(),
        durable_before.1,
        "⛔ A REFUSED CANDIDATE CHANGED THE LIVE SESSION'S CHECKPOINT OCCURRENCE \
         BASELINE — rollback-authoritative state, for a world that was never built"
    );
    assert_eq!(
        app.world()
            .get_resource::<ambition_platformer2d::platformer::lifecycle::CustodyBaseline>()
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
        ambition_platformer2d::actors::rooms::outstanding_publications(app.world_mut()),
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

/// ⛔⛤ **A10 AT SESSION SCOPE, IN THE SHIPPED APP: THE CANDIDATE DOES NOT TOUCH
/// THE WORLD THAT IS PLAYING.**
///
/// A10.5 prepares the incoming session — its hidden root, its hidden player, its
/// first room — while the outgoing session is still live and the route is still
/// pending. That is the whole guarantee and it is also the whole hazard: the
/// candidate's first room plans the same authored placement ids the playing
/// session is standing on, and a process-wide baseline declares them SUPERSEDED.
/// MEASURED 2026-09-15, before the verifiers were taught whose world they were
/// looking at: *"18 declared departures retired"* one frame BEFORE the incoming
/// session started — the candidate despawning the world it was meant to replace
/// only if it succeeded.
///
/// ⇒ This samples EVERY FRAME of the handoff rather than the ends: while the
/// live scope is still N, N's population must be exactly what it was. A world
/// that is corrected afterwards is a world that was broken.
#[test]
fn a_candidate_session_does_not_retire_the_playing_sessions_world() {
    use ambition_platformer2d::platformer::lifecycle::{
        ActiveSessionScope, SessionScopeId, SessionScopedEntity,
    };

    fn scope(app: &bevy::prelude::App) -> Option<SessionScopeId> {
        app.world()
            .get_resource::<ActiveSessionScope>()
            .and_then(ActiveSessionScope::current)
    }
    fn population(app: &mut bevy::prelude::App, owner: SessionScopeId) -> usize {
        let world = app.world_mut();
        world
            .query::<&SessionScopedEntity>()
            .iter(world)
            .filter(|scoped| scoped.0 == owner)
            .count()
    }

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();

    let gameplay = ShellRouteId::new("ambition_gameplay");
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay.clone(),
        request: None,
    });
    for _ in 0..240 {
        app.update();
    }
    let live = scope(&app).expect("the first session activated");
    let before = population(&mut app, live);
    assert!(
        before > 0,
        "the premise: world N is a populated session. Without it 'N was not \
         touched' would be true of an empty world"
    );

    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay,
        request: None,
    });
    let mut handed_over = false;
    let mut low_water = before;
    for _ in 0..240 {
        app.update();
        match scope(&app) {
            // ⛔ THE ONLY FRAMES THIS ARM JUDGES. Once the active scope has
            // MOVED, N is retired by its own lifecycle and is supposed to empty.
            Some(current) if current == live => {
                low_water = low_water.min(population(&mut app, live));
            }
            _ => {
                handed_over = true;
                break;
            }
        }
    }
    assert!(
        handed_over,
        "the handoff never happened, so no candidate was ever prepared beside \
         world N and this arm measured nothing"
    );
    assert_eq!(
        low_water, before,
        "⛔ THE CANDIDATE SESSION TOOK ENTITIES OUT OF THE WORLD THAT WAS STILL \
         PLAYING. Its first room plans the same authored ids, and a verifier that \
         does not ask WHOSE world it is looking at declares them superseded and \
         despawns them — before anything has decided the candidate may be played"
    );
}

/// ⛔⛤ **A10 ON THE HANDOFF ROAD: THE INCOMING SESSION GETS A ROOM WITH THINGS
/// IN IT.**
///
/// Every room root is minted hidden, so a refused room is DROPPED. The handoff
/// is the road that used to produce the worst refusal in the whole suite — a
/// session retiring its old scope and starting the new one in the SAME frame, so
/// the incoming room's transaction captured a baseline still holding all 18 of
/// the outgoing scope's placements (`room-refused :: 18x Duplicated`). Under the
/// bracket that drops the entire room, and the new session wakes up in an empty
/// world.
///
/// ⚠ **THE ORDERING FIX IS WHAT CLOSED IT** (`SessionScopeSet` chains
/// `RetireAuthority -> Cleanup -> Activate -> Presentation`), and
/// `the_retired_scopes_sweep_precedes_the_incoming_sessions_construction`
/// asks the SCHEDULE. This asks the RESULT, which is the half a schedule
/// assertion cannot cover: an ordering can be right and the room still refused
/// for some other reason.
///
/// ⭐ The roster assertion is what makes it non-vacuous. `published` alone could
/// be a stale verdict from the first activation — both activations build the same
/// room, so the id cannot tell them apart — but a live authoritative roster after
/// the handoff can only come from a room that actually arrived.
#[test]
fn a_shell_handoff_publishes_the_incoming_sessions_room() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();

    let gameplay = ShellRouteId::new("ambition_gameplay");
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay.clone(),
        request: None,
    });
    for _ in 0..240 {
        app.update();
    }
    let before = activation_id(&app);

    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay,
        request: None,
    });
    for _ in 0..240 {
        app.update();
    }
    let after = activation_id(&app);
    assert!(
        before.is_some() && after != before,
        "the activation id did not move ({before:?} -> {after:?}), so no handoff \
         happened and everything below is about the FIRST session"
    );

    let verification = app
        .world()
        .resource::<ambition_platformer2d::actors::features::LastConstructionVerification>()
        .clone();
    assert!(
        verification.published,
        "⛔ THE INCOMING SESSION'S ROOM WAS REFUSED. Under the candidate bracket \
         its roots are dropped, so the handoff lands the player in an empty \
         world: {verification:?}"
    );

    let roster = {
        let world = app.world_mut();
        world
            .query::<&ambition_platformer2d::platformer::sim_id::SimId>()
            .iter(world)
            .count()
    };
    assert!(
        roster > 0,
        "the handoff published a room and the world holds no authoritative \
         identities at all: {verification:?}"
    );
    // ⛔⛤ **AND N+1 IS DESCRIBABLE: no duplicate authoritative identity after the
    // replacement.** The other half of A10's success arm, asked of the SAME
    // authority publication is verified against rather than of a census written
    // for the test — `TransactionBaseline::capture` is exactly the operation that
    // cannot describe a world holding one identity twice. ⇒ The retired session's
    // world is genuinely gone rather than coexisting with the one that replaced
    // it, which a roster COUNT alone cannot tell you.
    assert!(
        ambition_platformer2d::platformer::construction::TransactionBaseline::capture(
            app.world_mut()
        )
        .is_ok(),
        "⛔ THE WORLD THE HANDOFF PUBLISHED CANNOT BE DESCRIBED: some identity has \
         two authoritative holders, so the outgoing session's world is still \
         standing beside the incoming one"
    );
    assert_eq!(
        ambition_platformer2d::platformer::construction::outstanding_candidates(app.world_mut()),
        0,
        "⛔ A HIDDEN CANDIDATE OUTLIVED THE HANDOFF: neither published nor \
         discarded"
    );
    assert_eq!(
        ambition_platformer2d::actors::rooms::outstanding_publications(app.world_mut()),
        0,
        "a publication receipt is still standing after the handoff settled"
    );
    // ⭐ THE CONTROL FOR THE CANCEL ARM'S ITEM-4 ASSERTION. `SessionMechanics` is
    // installed by ADOPTION, so a live session has one; without this, "a
    // cancelled candidate left no `SessionMechanics` behind" would be equally
    // true of a composition that never installs it at all.
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
            .is_some(),
        "an adopted session has no `SessionMechanics`, so the cancel arm's claim          that preparation installs none is a claim about nothing"
    );
    // ⭐ AND THE SAME CONTROL FOR THE PLATFORM STATE. MEASURED: this handoff
    // installs 1 moving platform and the cancel arm ends with 0, so the pair is
    // a real discriminator rather than two readings of an always-empty resource.
    assert!(
        !app.world()
            .resource::<ambition_platformer2d::world::collision::MovingPlatformSet>()
            .0
            .is_empty(),
        "an adopted session installed no moving platforms, so the cancel arm's          claim that preparation installs none is a claim about nothing"
    );
}

/// ⛔⛤ **A SUPERSEDED CANDIDATE SESSION IS DISCARDED, NOT DROPPED.**
///
/// `CandidateSessionSlot` is one deep. A second pending route replaces the first,
/// and the assignment that does it used to overwrite a whole prepared session —
/// hidden root, hidden first room, publication receipt and reserved scope, all
/// alive and none of them reachable again. A candidate that is neither PUBLISHED
/// nor DISCARDED is the state A10's lifecycle exists to make impossible.
#[test]
fn a_candidate_session_replaced_while_pending_is_discarded() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();
    let gameplay = ShellRouteId::new("ambition_gameplay");

    // Two routes requested with only a few frames between them, so the second
    // arrives while the first is still PENDING — the state that supersedes a
    // prepared candidate instead of adopting it.
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay.clone(),
        request: None,
    });
    for _ in 0..3 {
        app.update();
    }
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay,
        request: None,
    });
    for _ in 0..360 {
        app.update();
    }

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
    // registries and `MovingPlatformSet` the first room's platform state; both
    // are process-global RESOURCES, and a candidate builder that wrote them
    // during preparation would have changed live mechanical state for a session
    // that was never admitted. They belong to the candidate aggregate and are
    // installed by ADOPTION, so a candidate prepared and then abandoned must
    // leave neither behind. No session has ever been live in this arm, which is
    // what makes their ABSENCE meaningful.
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
            .is_none(),
        "⛔ PREPARING A CANDIDATE SESSION INSTALLED `SessionMechanics` PROCESS-WIDE.          The candidate was cancelled and never adopted, so the generation's frozen          registries belong to no session at all"
    );
    assert!(
        app.world()
            .resource::<ambition_platformer2d::world::collision::MovingPlatformSet>()
            .0
            .is_empty(),
        "⛔ PREPARING A CANDIDATE SESSION PUBLISHED ITS FIRST ROOM'S MOVING          PLATFORMS PROCESS-WIDE. The candidate was cancelled and never adopted,          so this is the mechanical state of a world nothing is playing"
    );
    assert_eq!(
        ambition_platformer2d::actors::rooms::outstanding_publications(app.world_mut()),
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

/// ⛔⛤ **REVIEW FINDING 2: AN INNER PUBLICATION MUST NOT RELEASE AN OUTER ONE'S
/// INVISIBILITY.**
///
/// A candidate ROOM inside a candidate SESSION is hidden for two independent
/// reasons. With a unit `InactiveCandidate` those were the same component, so the
/// room's own `publish_candidate` — which succeeds well before the shell decides
/// anything — removed the SESSION's barrier with it, and the candidate session's
/// room roots became visible to ordinary gameplay queries while the route was
/// still pending.
///
/// ⚠ **IT IS NOT ENOUGH THAT NOTHING CURRENTLY LOOKS IN THAT INTERVAL.** That
/// makes correctness depend on "nothing interesting happens between inner
/// publication and outer publication" rather than on representing the nesting,
/// and it does not survive `session -> region -> room`.
///
/// ⭐ The window is real and measurable on a COLD app: the first room publishes
/// around frame 2 and the session starts around frame 12.
#[test]
fn a_published_room_inside_a_pending_candidate_session_stays_invisible() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: ShellRouteId::new("ambition_gameplay"),
        request: None,
    });

    // Every frame from the room's verdict to the session's start, an ORDINARY
    // query — one with no `Allow<InactiveCandidate>` — must see nothing the
    // candidate owns.
    let mut room_published_at = None;
    let mut session_started_at = None;
    let mut worst: Option<(usize, usize)> = None;
    for frame in 0..240 {
        app.update();
        let published = app
            .world()
            .get_resource::<ambition_platformer2d::actors::features::LastConstructionVerification>()
            .is_some_and(|verification| verification.published);
        if published && room_published_at.is_none() {
            room_published_at = Some(frame);
        }
        let live = app
            .world()
            .get_resource::<ambition_platformer2d::platformer::lifecycle::ActiveSessionScope>()
            .and_then(
                ambition_platformer2d::platformer::lifecycle::ActiveSessionScope::current,
            );
        if live.is_some() && session_started_at.is_none() {
            session_started_at = Some(frame);
        }
        // Before the session is live, NOTHING it owns may answer an ordinary
        // query. `population` is exactly such a query.
        if session_started_at.is_none() {
            let visible = {
                let world = app.world_mut();
                let mut query = world.query::<&ambition_platformer2d::platformer::lifecycle::SessionScopedEntity>();
                query.iter(world).count()
            };
            if visible > 0 && worst.is_none() {
                worst = Some((frame, visible));
            }
        }
    }

    // ⭐ THE PREMISE: the window this arm is about actually opened.
    let (published_at, started_at) = (
        room_published_at.expect("the candidate's first room never published"),
        session_started_at.expect("the session never started"),
    );
    assert!(
        started_at > published_at,
        "the room published at frame {published_at} and the session started at \
         {started_at}, so there is no interval between the inner verdict and the \
         outer one and this arm says nothing"
    );
    assert_eq!(
        worst, None,
        "⛔ AN INNER PUBLICATION RELEASED THE CANDIDATE SESSION'S INVISIBILITY. \
         Ordinary gameplay queries saw session-owned entities while the route was \
         still pending (room published at frame {published_at}, session started \
         at {started_at})"
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
    // reconstruction can only have come from the candidate's own horizon.
    assert!(
        app.world()
            .resource::<ambition_platformer2d::actors::items::pickup::minted_horizon::MintedItemBaseline>()
            .is_empty(),
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
    for _ in 0..240 {
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

/// ⛔⛤ **THE PREREQUISITE THAT MAKES CANDIDATE CONSTRUCTION SAFE, ASKED OF THE
/// SHIPPED COMPOSITION — 2026-09-15 AUDIT, FINDING 4.**
///
/// `InactiveCandidate` hides an entity ONLY because
/// `register_inactive_candidate_filter` made it a bevy disabling component.
/// Without that call the marker is an ordinary inert component: every root the
/// room bracket stamps is stamped, and every one of them is LIVE — a half-built
/// room participating in the running world while it is being validated, and
/// silently.
///
/// ⛔⛔ **AND THE GUARD FOR IT DID NOT EXIST.** `ambition_platformer2d_runtime`
/// names `the_shipped_app_hides_candidates_before_they_are_verified` in a
/// comment beside the registration, as the thing that would catch its removal.
/// MEASURED 2026-09-15: a repo-wide grep finds that name in exactly ONE place —
/// that comment. **A test cited by a comment is not a test**, and a `//` comment
/// is not scanned by the citation gate, so the claim had been standing
/// unchallenged.
///
/// ⭐ **THIS IS THE PREFLIGHT, AND ITS MOMENT IS APP BUILD.** The audit asks for
/// prerequisites checked *before any candidate spawn can be queued*; the
/// prerequisite is a property of the WORLD, established once at composition
/// build, and that is strictly earlier than any command a room queues.
/// `transaction::open`'s refusal remains the backstop for a hand-built world —
/// and `a_candidate_room_is_refused_by_a_world_that_cannot_hide_a_candidate` now
/// asserts that backstop leaves nothing standing.
#[test]
fn the_shipped_app_hides_candidates_before_they_are_verified() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();
    assert!(
        ambition_platformer2d::platformer::construction::inactive_candidate_filter_installed(
            app.world_mut()
        ),
        "⛔ THE SHIPPED COMPOSITION DOES NOT HIDE CANDIDATES. `InactiveCandidate` \
         is not a disabling component in this world, so every root the room \
         bracket calls invisible is LIVE while it is being validated, and a \
         REFUSED room is visible debris rather than a drop"
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

/// ⛔⛤ **A10 NESTED ENTITY VISIBILITY WITHOUT NESTING PUBLICATION EFFECTS —
/// 2026-09-15 HOLISTIC AUDIT, FINDING 1.**
///
/// A candidate session's first room reaches its own verdict and admits its own
/// population while the session around it is still hidden and still refusable.
/// That is correct. What was not correct is that the same success arm then did
/// everything the room owed the world OUTSIDE its population — retired declared
/// predecessors, applied custody handoffs, replaced the world-defining state,
/// and wrote a `RoomLoaded` carrying nothing but a room id.
///
/// ⚠ **`RoomLoaded` ALONE BREAKS THE INVARIANT, ON THE SHIPPED HANDOFF.**
/// `FreshAttempt` treats any `RoomLoaded` as a fresh attempt, and production's
/// `void_pending_player_hits_at_lifecycle_boundaries` answers it by clearing
/// `PendingPlayerHitEvents` — rollback-registered and checksummed. MEASURED in
/// the world log before the fix: on a route handoff B's room published at frame
/// 242 while A was still the live session, which activated at 243. A candidate
/// that then refused would have left A playable but NOT unchanged, and unchanged
/// is exactly what the last-good-world invariant promises.
///
/// ⭐ **THE SUBJECT IS ASSERTED TO EXIST BEFORE ITS ABSENCE IS.**
/// `publications_holding_frozen_effects` counts the state the fix creates, so
/// this arm fails loudly rather than passing vacuously if the candidate never
/// reached its inner verdict inside the window.
#[test]
fn a_pending_candidate_sessions_room_publishes_no_lifecycle_into_the_live_session() {
    use ambition_platformer2d::game_shell::ShellRequestId;

    let mut app = a_running_shipped_session();
    app.insert_resource(RoomLoadsSeen::default());
    // ⛔⛤ **`Last`, NOT `Update` — THE POISON SAID SO.** With the counter in
    // `Update` the reverted fix still measured ZERO escaped loads: the room
    // publishes from a command flush inside the frame, and an `Update` reader
    // registered afterwards does not see the message until the NEXT frame — by
    // which time the session has been admitted and the window has closed. The
    // instrument was reporting the absence of its own cursor position.
    app.add_systems(bevy::prelude::Last, count_room_loads);
    app.update();

    let live_before = live_scope(&app);
    assert!(
        live_before.is_some(),
        "no session is live, so there is no last-good world for a candidate to \
         disturb and this arm says nothing"
    );
    let loads_before = app.world().resource::<RoomLoadsSeen>().0;

    let request = ShellRequestId::new("a10-nested-effects-witness");
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: ShellRouteId::new("ambition_gameplay"),
        request: Some(request),
    });

    // The window: B prepared and hidden, A still the live session.
    let mut window_frames = 0usize;
    let mut frozen_high_water = 0usize;
    let mut loads_in_window = 0usize;
    for _ in 0..60 {
        app.update();
        if live_scope(&app) != live_before {
            break;
        }
        window_frames += 1;
        frozen_high_water = frozen_high_water.max(
            ambition_platformer2d::actors::rooms::publications_holding_frozen_effects(
                app.world_mut(),
            ),
        );
        loads_in_window = app.world().resource::<RoomLoadsSeen>().0 - loads_before;
    }

    assert!(
        window_frames > 0,
        "the route activated on the very first frame, so there is no interval in \
         which a candidate is pending and this arm says nothing"
    );
    assert_eq!(
        loads_in_window, 0,
        "⛔ A ROOM PUBLISHED INSIDE A PENDING CANDIDATE SESSION ANNOUNCED ITSELF \
         TO THE LIVE ONE. {loads_in_window} `RoomLoaded` message(s) reached the \
         world across {window_frames} frames while session {live_before:?} was \
         still the one being played. `FreshAttempt` reads exactly this and \
         production clears rollback-authoritative staged hits on it, so a \
         candidate that later refused would have changed the world it promised \
         to leave alone"
    );
    // ⛔⛤ **THE PREMISE COMES SECOND, AND THE ORDER IS THE POINT.** Asserting it
    // FIRST made the poison run fire on the premise rather than on the subject:
    // reverting the deferral removes the frozen state, so *"no publication held
    // frozen effects"* masked the `RoomLoaded` that was escaping one line below.
    // A poison that fires on a vacuity guard proves the guard works and says
    // nothing about the assertion it guards. MEASURED with `deferred = false`:
    // this order fails on the count above.
    assert!(
        frozen_high_water > 0,
        "⛔ PREMISE: no publication ever held frozen effects while the candidate \
         session was pending across {window_frames} frames, so the zero asserted \
         above is a claim about the WINDOW rather than about the deferral"
    );

    // ⭐ THE OTHER HALF: the notification is not LOST, it is OWED. Admission is
    // what pays it.
    for _ in 0..120 {
        app.update();
    }
    assert_ne!(
        live_scope(&app),
        live_before,
        "the candidate never became the live session, so the admission half of \
         this arm says nothing"
    );
    assert!(
        app.world().resource::<RoomLoadsSeen>().0 > loads_before,
        "⛔ THE DEFERRED ROOM LIFECYCLE WAS NEVER PAID. The candidate session was \
         admitted and its first room's `RoomLoaded` never reached the world, so \
         every per-attempt reader still believes it is in the previous room"
    );
    assert_eq!(
        ambition_platformer2d::actors::rooms::publications_holding_frozen_effects(
            app.world_mut()
        ),
        0,
        "a publication is still holding frozen effects after its session was \
         admitted, so something the room owes the world is owed forever"
    );
}

/// ⛔⛤ **FINDING 1's SECOND MANIFESTATION: A ROOM RESET `FactionRelations`.**
///
/// `RoomFeatureConstructionPlan::spawn` inserted
/// `FactionRelations::default()` on every room spawn, a hidden candidate's
/// included — an App-global resource that is rollback-registered and
/// checksummed. So preparing candidate B rewrote live session A's combat
/// relations before B had a verdict, let alone an admission.
///
/// ⚠ **IT WAS LATENT, MEASURED: every `set_hostile`/`set_mutual_hostile` in the
/// workspace outside `Default` is in a test, so no production road makes the
/// shipped table non-default.** This arm makes it non-default ON PURPOSE, which
/// is the only way the reset is observable at all — and the audit's rule is that
/// the effect must be unexpressible rather than merely unobserved.
#[test]
fn preparing_a_candidate_does_not_reset_the_live_sessions_faction_relations() {
    use ambition_platformer2d::actor::ActorFaction;
    use ambition_platformer2d::actors::features::FactionRelations;
    use ambition_platformer2d::game_shell::ShellRequestId;

    let mut app = a_running_shipped_session();
    app.world_mut()
        .resource_mut::<FactionRelations>()
        .set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
    app.update();
    // ⭐ THE PREMISE: the value this arm watches really is not the default one.
    assert!(
        app.world()
            .resource::<FactionRelations>()
            .is_hostile(ActorFaction::Enemy, ActorFaction::Boss),
        "the stance this arm watches did not take, so a later equality against \
         the default would pass for the wrong reason"
    );

    let request = ShellRequestId::new("a10-faction-relations-witness");
    let live_before = live_scope(&app);
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: ShellRouteId::new("ambition_gameplay"),
        request: Some(request.clone()),
    });
    let mut window_frames = 0usize;
    let mut survived_every_frame = true;
    for _ in 0..30 {
        app.update();
        if ambition_platformer2d::platformer::construction::outstanding_candidates(
            app.world_mut(),
        ) == 0
        {
            continue;
        }
        window_frames += 1;
        survived_every_frame &= app
            .world()
            .resource::<FactionRelations>()
            .is_hostile(ActorFaction::Enemy, ActorFaction::Boss);
    }

    assert!(
        window_frames > 0,
        "no candidate was ever outstanding, so nothing was constructed off to the \
         side and this arm says nothing"
    );
    assert!(
        survived_every_frame,
        "⛔ CONSTRUCTING A CANDIDATE ROOM RESET THE LIVE SESSION'S \
         `FactionRelations`. The stance this arm set on the session that was \
         PLAYING was gone while a candidate nobody had admitted was being built, \
         and the table is rollback-registered and checksummed"
    );

    // ⚠ **AND THE CANCEL VARIANT THE AUDIT ASKS FOR DOES NOT BELONG ON THIS ARM
    // — MEASURED, NOT ASSUMED.** I extended it with a production
    // `ShellCommand::CancelPending` and an assertion that A's scope was
    // unchanged, and it failed with `Some(SessionScopeId(1)) != Some(0)`: on a
    // WARM handoff the route activates within the window and the candidate is
    // ADMITTED before a cancellation issued from the test can win the race. So
    // the arm would have been asserting about an admitted session while claiming
    // to be about an abandoned one.
    //
    // ⇒ The cancellation road's witness is
    // `a_candidate_session_whose_route_is_cancelled_is_discarded`, which starts
    // COLD — the pending window is wide there and narrow here, which is itself
    // the measured fact. This arm keeps the half it can state honestly: the
    // stance survived every frame in which a candidate existed, admitted or not.
    let _ = (request, live_before);
}

/// ⛔⛤ **REVIEW FINDING 2: THE CANDIDATE'S MINTED DESCRIPTIONS ARE ITS OWN.**
///
/// `OccurrenceContinuity` needs two descriptors to rebuild a runtime-minted
/// occurrence: the ledger row saying WHERE it is, and the minted description
/// saying WHAT it is. The candidate carried its own ledger and then read the LIVE
/// `MintedItemBaseline` for the second half — B's whereabouts against A's
/// descriptions, a mixed-session durable horizon.
///
/// ⛔⛤ **THE POSITIVE HALF IS GUARANTEED STRUCTURALLY, NOT BY THIS ARM, AND THAT
/// IS DELIBERATE.** `PlatformerSessionBuilder` no longer HOLDS the live
/// `AuthoredOccurrences` or `MintedItemBaseline` — both fields are deleted, and
/// the compiler reported them dead the moment candidate construction stopped
/// reading them. "Candidate construction reads a live durable resource" is not a
/// mistake that type can express any more, which is stronger than a test.
///
/// ⚠ **AND I TRIED TO WITNESS IT BEHAVIOURALLY AND WITHDREW THE ARM.** A save
/// fixture that names a runtime mint and expects the candidate's hidden first
/// room to rebuild it did not reach its subject: the mint was reconstructed under
/// NEITHER baseline, so the arm could not tell the fix from the defect. Making it
/// real needs a save whose ledger, custody and minted rows together describe a
/// mint the start room actually reinstates — a fixture job, not an assertion. An
/// arm that passes without reaching its subject is worse than no arm.
///
/// ⇒ What this DOES witness is the half that is observable: preparing a candidate
/// installs nothing over the playing session's minted baseline.
#[test]
fn preparing_a_candidate_does_not_install_its_minted_baseline_over_the_live_one() {
    use ambition_platformer2d::persistence::save_data::PersistedMintedItem;

    let mut app = a_running_shipped_session();
    let minted_id = "minted:a10_candidate_horizon_mint";
    let id =
        ambition_platformer2d::platformer::sim_id::SimId::from_snapshot(minted_id.to_string());

    // A's baseline cannot describe M; B's save can. If preparing B installs B's
    // horizon process-wide — which is what finding 1 was about, in the resource
    // finding 2 added — A's baseline learns about M.
    app.world_mut()
        .resource_mut::<ambition_platformer2d::actors::items::pickup::minted_horizon::MintedItemBaseline>()
        .adopt(Default::default());
    {
        let mut save = app
            .world_mut()
            .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>();
        save.0.set_minted_items(vec![PersistedMintedItem {
            occurrence: minted_id.to_string(),
            parent: "placement:ground_gun_sword".to_string(),
            sequence: 1,
            held_item: "ground_gun_sword".to_string(),
        }]);
    }
    assert!(
        app.world()
            .resource::<ambition_platformer2d::actors::items::pickup::minted_horizon::MintedItemBaseline>()
            .description_of(&id)
            .is_none(),
        "the premise: A's live baseline does not describe this mint"
    );

    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: ShellRouteId::new("ambition_gameplay"),
        request: None,
    });
    for _ in 0..3 {
        app.update();
    }

    assert!(
        app.world()
            .resource::<ambition_platformer2d::actors::items::pickup::minted_horizon::MintedItemBaseline>()
            .description_of(&id)
            .is_none(),
        "⛔ PREPARING A CANDIDATE INSTALLED ITS MINTED BASELINE OVER THE PLAYING \
         SESSION'S. The candidate is not admitted; its durable horizon belongs to \
         it until adoption"
    );
}

/// ⛔⛤ **A SUPERSEDED TRANSACTION CANNOT PUBLISH — IN THE COMPOSED HOST.**
///
/// The A-supersedes-B hold race had three witnesses and all three hand-built a
/// `ShellRouter::default()` and registered their own catalog:
/// `provider_retry_supersedes_the_failed_transaction_and_rejects_stale_publication`,
/// `a_superseded_transaction_names_the_request_it_cancelled` and
/// `superseded_load_cannot_authorize_commit`. They pin the ROUTER'S LOGIC. None
/// of them asks the shipped composition anything, and MEASURED 2026-09-16, no
/// `app_it` arm read `ShellEvent::PreparationRequested` or a transaction's
/// `barrier.load_id` at all — the only `ShellEvent::` mentions in the whole
/// integration suite were in comments. So the question *"can a superseded
/// transaction still publish HERE"* had never been put to a real app.
///
/// ⭐ **THE SESSION HALF WAS ALREADY COVERED**, by
/// `a_candidate_session_replaced_while_pending_is_discarded` above, which drives
/// the same overlap and asserts the superseded CANDIDATE is discarded. This arm
/// is the other half: the TRANSACTION identity, which is what a caller
/// correlates its own request on.
///
/// ⚠ **THE CLAIM IS MADE BY CALLING `publish` ON THE REAL REGISTRY.** Asserting
/// that a record is absent would test a lookup; asking the production
/// `PreparedSessionRegistry` to publish the superseded transaction and requiring
/// `None` tests the refusal itself.
#[test]
fn a_superseded_transaction_cannot_publish_in_the_shipped_app() {
    use ambition_platformer2d::game_shell::{
        PreparedSessionRegistry, ProviderLoadTransaction, ShellEvent, TransactionEnd,
    };
    use bevy::ecs::message::Messages;

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();
    let gameplay = ShellRouteId::new("ambition_gameplay");

    let mut seen: Vec<ShellEvent> = Vec::new();
    let drain = |app: &mut bevy::prelude::App, seen: &mut Vec<ShellEvent>| {
        if let Some(mut messages) = app.world_mut().get_resource_mut::<Messages<ShellEvent>>() {
            seen.extend(messages.drain());
        }
    };

    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay.clone(),
        request: None,
    });
    // Few enough frames that the first transaction is still PENDING when the
    // second request lands — the state that supersedes rather than adopts.
    for _ in 0..3 {
        app.update();
        drain(&mut app, &mut seen);
    }

    let first: ProviderLoadTransaction = seen
        .iter()
        .find_map(|event| match event {
            ShellEvent::PreparationRequested(transaction) => Some(transaction.clone()),
            _ => None,
        })
        .expect(
            "the shipped composition emitted no `PreparationRequested` in three frames, so \
             there is no transaction for a second request to supersede and this arm measures \
             nothing",
        );

    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay,
        request: None,
    });
    for _ in 0..360 {
        app.update();
        drain(&mut app, &mut seen);
    }

    // ⚠ **THE PREMISE, AND WITHOUT IT "CANNOT PUBLISH" IS SATISFIED BY "NEVER
    // EXISTED".** A second, DIFFERENT transaction must have been minted; if the
    // router had reused the first one there would be no supersession here at all
    // and every assertion below would pass for the wrong reason.
    let minted: Vec<_> = seen
        .iter()
        .filter_map(|event| match event {
            ShellEvent::PreparationRequested(transaction) => {
                Some(transaction.barrier.load_id.clone())
            }
            _ => None,
        })
        .collect();
    assert!(
        minted.len() >= 2 && minted[0] != minted[1],
        "the second request did not mint a fresh transaction ({minted:?}), so nothing was \
         superseded and this arm is vacuous"
    );

    // ⛔ THE FIRST TRANSACTION ENDED, IT WAS NAMED, AND THE REASON IS SUPERSESSION.
    let ended = seen.iter().find_map(|event| match event {
        ShellEvent::TransactionEnded {
            barrier, reason, ..
        } if *barrier == first.barrier => Some(reason.clone()),
        _ => None,
    });
    assert_eq!(
        ended,
        Some(TransactionEnd::Superseded),
        "the shipped composition ended the first transaction as {ended:?} rather than \
         naming it superseded. A caller waiting on its own transaction has nothing to \
         match on for the one terminal state that produces no error at all"
    );

    // ⛔⛤ AND IT CANNOT PUBLISH. Asked of the PRODUCTION registry, not a fixture.
    let published = app
        .world_mut()
        .resource_mut::<PreparedSessionRegistry>()
        .publish(&first);
    assert!(
        published.is_none(),
        "the superseded transaction published in the shipped app ({published:?}). A \
         publication from a cancelled transaction hands the live session a generation \
         nobody admitted"
    );
}

/// ⛔⛤ **THE `Q132` RULING MADE THIS ARM REQUIRED RATHER THAN NICE: A CANDIDATE
/// MUST NOT COUNT AS A CANONICAL ROOT IN THE WITNESS OF THE ONE-ROOT INVARIANT.**
///
/// Decided 2026-09-19 — there is exactly one canonical live `SessionRoot`, two
/// published roots are invalid, and preparing a replacement while the current
/// session is live is allowed *provided the candidate carries a distinct
/// prepared identity and does not masquerade as a root*. The ruling asks for
/// production-composition witnesses of both halves.
///
/// ⚠ **THE EXISTING ARM CANNOT SUPPLY THE SECOND HALF, AND THAT IS WHY THIS
/// EXISTS.** `the_shipped_app_never_holds_two_session_roots_across_a_handoff`
/// counts roots with an ordinary query, and `InactiveCandidate` is a DISABLING
/// component — so a hidden candidate is excluded by construction and that arm is
/// green whether or not one was ever prepared. It cannot tell *"the candidate
/// road ran and was correctly invisible"* from *"no candidate was prepared at
/// all"*: the population-blind reading `Q132` itself warned about, where "it ran
/// and found nothing" and "it never ran" are the same string.
///
/// ⛔⛤ **AND THE FIRST VERSION OF THIS ARM PROVED THE MASQUERADE AND READ IT AS
/// AGREEMENT — CORRECTED 2026-09-19 WHEN THE RULING WAS IMPLEMENTED.** It
/// asserted `visible = 0, including hidden = 1` and called that the candidate
/// "not counting". What those two numbers actually said is that the candidate
/// WAS a `SessionRoot` and was merely invisible — which is the weaker claim the
/// ruling refuses, because every construction query that legitimately says
/// `Allow<InactiveCandidate>` saw two canonical roots for one live world.
///
/// ⇒ The assertion the ruling asks for is on the count INCLUDING hidden
/// entities: at most one `SessionRoot` on every frame of the handoff, candidate
/// or not. A prepared candidate carries `CandidateSessionRoot` instead, and
/// that count is the anti-vacuity premise — without it, "never two" would be
/// satisfied by a route where no replacement was ever prepared.
///
/// ⭐ MEASURED 2026-09-19 on this route with the representation in place:
/// `SessionRoot` including hidden never exceeds one, and the frames before
/// adoption hold a `CandidateSessionRoot` beside a canonical count of zero.
///
/// ⛔⛤ **AND THE FIRST POISON PASSED, WHICH IS A FACT ABOUT THE TREE RATHER
/// THAN ABOUT THIS ARM.** Neutering `hide_candidate_session_root` changed
/// nothing here: it has exactly ONE production caller
/// (`actor_monolith/src/world/rooms/stage.rs:1576`) and every other caller is a
/// test. The road that hides THIS root is `SessionSpawnScope::apply_to`
/// (`shared_tangle/src/lifecycle/session.rs:293-300`), which applies the
/// candidate policy to every session-owned spawn — *"which is why the candidate
/// policy lives on the scope rather than at each spawn site"*. Poisoning THAT
/// fires both arms. ⇒ Two hiding roads exist and only one governs a session
/// root's visibility on the shipped handoff; a reader looking for the mechanism
/// by name would find the wrong one first.
#[test]
fn a_prepared_candidate_never_counts_as_a_canonical_session_root() {
    use bevy::prelude::With;
    type Root = ambition_platformer2d::platformer::lifecycle::SessionRoot;
    type Candidate = ambition_platformer2d::platformer::lifecycle::CandidateSessionRoot;

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();

    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: ShellRouteId::new("ambition_gameplay"),
        request: None,
    });

    let mut canonical_high_water = 0usize;
    let mut worst_frame = 0usize;
    let mut saw_canonical = false;
    let mut candidate_frames = Vec::new();
    for frame in 0..240 {
        app.update();
        let world = app.world_mut();
        // ⛔ INCLUDING HIDDEN, WHICH IS THE WHOLE ASSERTION. An ordinary query
        // cannot see a candidate at all, so counting that way answers a
        // question about VISIBILITY when the ruling is about IDENTITY.
        let canonical = ambition_platformer2d::platformer::construction::
            count_matching_including_hidden_candidates::<With<Root>>(world);
        let candidates = ambition_platformer2d::platformer::construction::
            count_matching_including_hidden_candidates::<With<Candidate>>(world);
        if canonical > canonical_high_water {
            canonical_high_water = canonical;
            worst_frame = frame;
        }
        if canonical == 1 {
            saw_canonical = true;
        }
        if candidates > 0 {
            candidate_frames.push((frame, canonical, candidates));
        }
    }

    // ⚠ THE PREMISE, and without it "never two" is satisfied by "never one".
    assert!(
        saw_canonical,
        "no canonical session root ever appeared in 240 frames, so this measured \
         a route that never activated rather than a handoff"
    );

    assert!(
        canonical_high_water <= 1,
        "frame {worst_frame} held {canonical_high_water} `SessionRoot`s counting \
         HIDDEN entities. Two roots are invalid whether or not one of them is \
         visible: a replacement may be PREPARED while the current session is \
         live, but it carries `CandidateSessionRoot` until it is adopted and must \
         not stand as a second canonical root in the meantime"
    );

    // ⛔ THE PREMISE THE CANONICAL COUNT CANNOT SUPPLY. "At most one root" is
    // trivially true of a route that never prepared a replacement, so the arm
    // has to see a candidate exist beside the count it is constraining.
    assert!(
        !candidate_frames.is_empty(),
        "no frame held a `CandidateSessionRoot`, so the count above is green for \
         an unknown reason — either the candidate road did not run, or the \
         candidate is carrying `SessionRoot` again and the count is one because \
         the two are being conflated. Both are the defect this arm exists to \
         separate"
    );
}
