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
        .resource::<ambition_platformer2d::actors::world::rooms::LastConstructionVerification>()
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

/// ⭐ A RELOAD WHOSE BOUNDARY CLOSES WHILE IT WAITS IS CANCELLED WHOLE, AND
/// WHICH OF TWO OWNERS CANCELS IT IS MEASURED.
///
/// A content reload is legal against a timeline that is healthy and that this
/// host may rebase (`Q118`). Two things ask again after the request:
///
/// - the lease breaker (`break_the_publication_lease_when_the_boundary_closes`)
///   asks on each frame from the adoption of the transaction, and cancels the
///   transaction on the frame after the boundary closes;
/// - the activation gate (`answer_the_publication_gate`) is asked on each
///   frame that the route is ready but for its holds, and at the activation.
///
/// Measured 2026-10-05 in this fixture with each owner removed in turn (the
/// breaker an early return; the gate an `Admit` each time):
///
/// | the boundary closes                                | breaker and gate | gate only        | neither   |
/// |----------------------------------------------------|------------------|------------------|-----------|
/// | never                                              | published        | published        | published |
/// | when the route is ready, and stays (two kinds)     | cancelled, +1    | cancelled, +1    | published |
/// | at the request, for one update (before adoption)   | published        | published        | published |
/// | while the route prepares, and stays                | cancelled, +1    | cancelled, ready | published |
/// | while the route prepares, open again before ready  | cancelled, +1    | PUBLISHED        | published |
///
/// "+1" is the frame after the boundary closed; "ready" is the frame the
/// route became ready, 6 frames later in this fixture. "Cancelled" is the same
/// state in each cell: the route is not activated, the content is of the old
/// generation, nothing is staged, and the two holds of the transaction are
/// released.
///
/// So the gate alone gives the same result wherever the boundary is closed on
/// a frame that the route is ready. The breaker is the one owner of the last
/// row, and of the early frame in the row above it: the admission is a lease
/// for the life of the transaction, not only a question at the activation.
/// With the breaker removed, the last two closed arms here fail.
///
/// The fixture: a reload of the fighter ladder (500 ms to 499 ms) on the
/// shipped app, with a hold that has no gate on the route, as the loading
/// screen has, released 12 frames after the route is ready. A slow preparation
/// keeps the barrier of the route not ready for 8 frames after the adoption.
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
        /// Control: the boundary stays open.
        Never,
        /// Control of the slow preparation.
        SlowNever,
        /// Closed 3 frames after the route is ready, to the end.
        WhenReady(Kind),
        /// Closed at the request, open again after one update: before the
        /// transaction is adopted, so nothing is owed yet.
        BeforeAdoption,
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
        (Arm::Never, End::Published),
        (Arm::SlowNever, End::Published),
        (Arm::WhenReady(Kind::Foreign), End::Cancelled(1)),
        (Arm::WhenReady(Kind::Unhealthy), End::Cancelled(1)),
        (Arm::BeforeAdoption, End::Published),
        (Arm::WhilePreparing { opens_again: false }, End::Cancelled(1)),
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
        if arm == Arm::BeforeAdoption {
            close(&mut app, Kind::Foreign);
        }

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
            if arm == Arm::BeforeAdoption && frame == 0 {
                app.world_mut().insert_resource(owned);
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
        eprintln!(
            "[lease] {arm:?}: adopted {adopted_at:?} ready {ready_at:?} closed {closed_at:?} \
             ended {ended_at:?}: {end:?}"
        );
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

/// A reload waits for a live room that has no geometry, and says so. The
/// publication of that room would be refused when it is applied, after the
/// rooms before it published. `pending` is not cleared. When the room is
/// gone, the same press applies, which is the control.
///
/// This arm was `a_world_reload_asked_for_while_two_rooms_are_live_waits`:
/// a reload rebuilt one live room, so with two it waited. It rebuilds each
/// live room now (`a_world_reload_rebuilds_every_live_room`), and this is
/// the one wait that is left.
#[test]
fn a_world_reload_waits_for_a_live_room_with_no_geometry() {
    use ambition_platformer2d::dev_tools::WorldSourceHotReload;
    let mut app = a_running_shipped_session();
    let applied = |app: &bevy::prelude::App| app.world().resource::<WorldSourceHotReload>().applied_count;
    let before = applied(&app);
    // A live room root of a room that is not live, with no geometry.
    let definition = {
        let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::world::rooms::RoomSet,
        >(app.world())
        .expect("the session keeps its room set");
        rooms.definition_by_id("basement_npcs").expect("the shipped world has the room")
    };
    let second = ambition_platformer2d::platformer::lifecycle::spawn_live_room(
        app.world_mut(),
        ambition_platformer2d::platformer::lifecycle::LiveRoomInstance::from_ordinal(1_000),
        definition,
    );
    assert!(
        app.world().get::<ambition_platformer2d::engine_core::RoomGeometry>(second).is_none(),
        "the new live room root has geometry, so this arm does not test the wait"
    );

    press_apply_reload(&mut app);
    let reload = app.world().resource::<WorldSourceHotReload>().clone();
    assert_eq!(
        (
            reload.applied_count,
            reload.last_status.starts_with("world reload waits: live room #1000 has no geometry"),
        ),
        (before, true),
        "a reload with a live room that has no geometry: (applied count, the status says it waits); status {:?}, errors {:?}",
        reload.last_status,
        reload.last_errors
    );

    app.world_mut().entity_mut(second).despawn();
    press_apply_reload(&mut app);
    let reload = app.world().resource::<WorldSourceHotReload>().clone();
    assert!(
        reload.applied_count > before,
        "control: with that room gone, the reload did not apply: {:?} / {:?}",
        reload.last_status,
        reload.last_errors
    );
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
        kinematics.size,
    );
    Some((kinematics.pos, clear.distance(kinematics.pos) < 0.01))
}

/// ⭐ A WORLD RELOAD MOVES THE BODY THAT STAYS OUT OF THE NEW SOLIDS.
///
/// A reload replaces the geometry under each body that is not a resident of
/// its room. The reload moves that body to a clear place in the new geometry
/// of its room (`rehome_and_dress`). This had no witness.
///
/// The file that is reloaded is a copy whose `Collision` layer has solid
/// cells over the place where Alice stands. The reading is her place on the
/// frame the reload is applied, and whether it is clear in the geometry of
/// her room.
///
/// - One live room.
/// - Two live rooms, and Alice in the second one. The room of the primary
///   seat is the room the reload publishes first, whichever it is.
/// - Control: two live rooms and no solid over Alice. She is where she was.
///
/// ⛔ A RESIDENT DOES NOT STAY, AND THIS ARM SAYS SO. A body that is a
/// resident of its room is removed with the room, and the reload builds the
/// authored bodies again. Measured 2026-10-05: Bob of these fixtures, a
/// spawned actor that seat 1 drives, is a resident. The reload of his room
/// removes him, also in the control, and nothing builds him again. The last
/// assertion fails when such a driven body stays. The home body of a joined
/// seat is not a resident and does stay; its move has its own arm,
/// `a_world_reload_moves_the_body_of_a_joined_seat_out_of_the_new_solids`.
#[test]
fn a_world_reload_moves_the_body_that_stays_out_of_the_new_solids() {
    use crate::neighbor_prefetch_prepares_rooms::{alice, bob, live_room_ids, room_of};
    use ambition_platformer2d::dev_tools::WorldSourceHotReload;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Arm {
        OneRoom,
        SecondRoom,
        Control,
    }
    let mut wrong: Vec<String> = Vec::new();
    for arm in [Arm::Control, Arm::OneRoom, Arm::SecondRoom] {
        let (mut app, room) = match arm {
            Arm::SecondRoom | Arm::Control => {
                let (app, _) =
                    a_running_shipped_session_with_two_live_rooms(Leaver::Alice, SECOND_LIVE_ROOM);
                (app, SECOND_LIVE_ROOM.to_owned())
            }
            Arm::OneRoom => {
                let app = a_running_shipped_session();
                let start = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
                    .expect("the session has a live room")
                    .id
                    .clone();
                (app, start)
            }
        };
        // Alice comes to rest.
        for _ in 0..60 {
            app.update();
        }
        let alice = alice(&mut app);
        let driven = bob(&mut app);
        let half = app
            .world()
            .get::<ambition_platformer2d::engine_core::BodyKinematics>(alice)
            .expect("Alice has a body")
            .size
            * 0.5;
        // ⛔ THE PREMISE: before the reload Alice is clear, in the room this
        // arm changes.
        let before = place_of(&app, alice);
        assert_eq!(
            (before.map(|(_, clear)| clear), room_of(&app, alice)),
            (Some(true), Some(room.clone())),
            "{arm:?}: before the reload (Alice is clear, her room): she is at {before:?}"
        );
        let (was, _) = before.expect("checked");
        let rooms_before = live_room_ids(&mut app);

        let _copy = watch_an_edited_copy(&mut app, &format!("seat_{arm:?}"), |project| match arm {
            Arm::Control => move_an_entity(project, &room, "NpcSpawn"),
            _ => put_a_solid_over(project, &room, was, half),
        });
        let applied = app.world().resource::<WorldSourceHotReload>().applied_count;
        press_apply_reload(&mut app);
        let reload = app.world().resource::<WorldSourceHotReload>().clone();
        assert_eq!(
            (reload.applied_count, reload.last_errors.is_empty()),
            (applied + 1, true),
            "{arm:?}: the reload did not apply: {:?} / {:?}",
            reload.last_status,
            reload.last_errors
        );

        let after = place_of(&app, alice);
        let driven_stays = driven.map(|body| app.world().get_entity(body).is_ok());
        eprintln!(
            "[re-seat] {arm:?}: Alice {before:?} -> {after:?} in {:?}; the driven body stays: \
             {driven_stays:?}; live rooms {rooms_before:?} -> {:?}",
            room_of(&app, alice),
            live_room_ids(&mut app)
        );
        let Some((is, clear)) = after else {
            wrong.push(format!("{arm:?}: after the reload Alice is in no live room"));
            continue;
        };
        if room_of(&app, alice).as_deref() != Some(room.as_str()) {
            wrong.push(format!(
                "{arm:?}: after the reload Alice is in {:?}, not in `{room}`",
                room_of(&app, alice)
            ));
        }
        if !clear {
            wrong.push(format!(
                "{arm:?}: after the reload Alice is at {is:?}, inside a solid of the new \
                 geometry of `{room}`"
            ));
        }
        let moved = is.distance(was);
        match arm {
            Arm::Control if moved > 2.0 => wrong.push(format!(
                "control: no solid was put over Alice, and the reload moved her {moved} px"
            )),
            Arm::OneRoom | Arm::SecondRoom if moved < 2.0 => wrong.push(format!(
                "{arm:?}: a solid was put over Alice and she is where she was ({is:?})"
            )),
            _ => {}
        }
        // The premise of "one body stays". See the note above.
        if driven_stays == Some(true) {
            wrong.push(format!(
                "{arm:?}: the driven body stayed in a room that the reload built again. It is \
                 a second body that stays: move it out of the new solids too"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A WORLD RELOAD MOVES THE BODY OF A JOINED SEAT OUT OF THE NEW SOLIDS.
///
/// The home body of a joined seat is owned by the session, so a reload keeps
/// it (Q153 default join). The reload moves it to a clear place in the new
/// geometry of its room, as it moves the primary body. Seat 1 joins through
/// the join system, run once on the world with a Jump press in seat 1's slot
/// (the shipped composition opens one handle, so no device presses for it).
///
/// - Control: the reload moves an NPC and puts no solid over the seat; the
///   body of seat 1 is where it was.
/// - The reload puts a solid over the body of seat 1; it is moved and clear.
#[test]
fn a_world_reload_moves_the_body_of_a_joined_seat_out_of_the_new_solids() {
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot, SlotControls};
    use ambition_platformer2d::platformer::markers::PlayerEntity;
    use bevy::ecs::system::RunSystemOnce as _;
    let mut wrong: Vec<String> = Vec::new();
    for solid in [false, true] {
        let mut app = a_running_shipped_session();
        for _ in 0..60 {
            app.update();
        }
        let room = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
            .expect("the session has a live room")
            .id
            .clone();
        app.world_mut().resource_mut::<SlotControls>().set(
            PlayerSlot(1),
            ambition_platformer2d::engine_core::ControlFrame {
                jump_pressed: true,
                ..Default::default()
            },
        );
        app.world_mut()
            .run_system_once(ambition_platformer2d::actors::session::join::seat_a_joining_participant)
            .expect("the join system runs");
        // No handle writes slot 1 in this composition, so the press would
        // stay and the body would jump for ever.
        app.world_mut().resource_mut::<SlotControls>().set(PlayerSlot(1), Default::default());
        let world = app.world_mut();
        let seat = world
            .query_filtered::<(bevy::prelude::Entity, &DrivingParticipant), bevy::prelude::With<PlayerEntity>>()
            .iter(world)
            .find(|(_, driver)| driver.0 == PlayerSlot(1))
            .map(|(body, _)| body)
            .expect("seat 1 joined");
        // The body of seat 1 comes to rest.
        for _ in 0..60 {
            app.update();
        }
        let half = app
            .world()
            .get::<ambition_platformer2d::engine_core::BodyKinematics>(seat)
            .expect("the body of seat 1 has kinematics")
            .size
            * 0.5;
        let before = place_of(&app, seat);
        assert_eq!(
            before.map(|(_, clear)| clear),
            Some(true),
            "solid={solid}: before the reload the body of seat 1 is clear: {before:?}"
        );
        let (was, _) = before.expect("checked");
        let _copy = watch_an_edited_copy(&mut app, &format!("joined_seat_{solid}"), |project| {
            if solid {
                put_a_solid_over(project, &room, was, half)
            } else {
                move_an_entity(project, &room, "NpcSpawn")
            }
        });
        press_apply_reload(&mut app);
        let Some((is, clear)) = place_of(&app, seat) else {
            wrong.push(format!("solid={solid}: after the reload the body of seat 1 is gone or in no live room"));
            continue;
        };
        let moved = is.distance(was);
        if !clear {
            wrong.push(format!("solid={solid}: the body of seat 1 is at {is:?}, inside a solid of `{room}`"));
        }
        if solid && moved < 2.0 {
            wrong.push(format!("a solid was put over seat 1 and it is where it was ({is:?})"));
        }
        if !solid && moved > 2.0 {
            wrong.push(format!("control: no solid over seat 1, and the reload moved it {moved} px"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// ⭐ A WORLD RELOAD REBUILDS EVERY LIVE ROOM.
///
/// A reload replaces the room set. A live room that it did not rebuild would
/// keep the content of the old generation under the new set's definition of
/// its room. So the reload publishes the room of the primary seat, which
/// brings the set, and then each other live room from that set, in one
/// command flush. The generation of the session moves one time, after the
/// last room.
///
/// The file that is reloaded is a changed copy (one entity moved), so the
/// reload is a new generation and its content term is a new one. The reading
/// for each live room is its live instance and the content terms that its
/// stamped entities carry.
///
/// - Control, one live room: it is rebuilt, as before.
/// - Two live rooms: each is rebuilt. Before, the reload waited (`world
///   reload waits: 2 rooms are live`), and the two rooms kept their content.
#[test]
fn a_world_reload_rebuilds_every_live_room() {
    use ambition_platformer2d::dev_tools::WorldSourceHotReload;

    for two_rooms in [false, true] {
        let arm = if two_rooms { "two live rooms" } else { "one live room" };
        let (mut app, edited_room, moved) = if two_rooms {
            let (app, _) =
                a_running_shipped_session_with_two_live_rooms(Leaver::Alice, SECOND_LIVE_ROOM);
            (app, SECOND_LIVE_ROOM.to_owned(), "NpcSpawn")
        } else {
            let app = a_running_shipped_session();
            let start = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
                .expect("the session has a live room")
                .id
                .clone();
            (app, start, "PogoOrb")
        };
        let before = live_room_readings(&mut app);
        let epoch_before = the_only_prepared_epoch(&mut app);
        // ⛔ THE PREMISE. Each live room has stamped content, and all of it is
        // of one generation. A room with no stamped entity reads the same
        // rebuilt or kept.
        let old_terms: std::collections::BTreeSet<&String> =
            before.values().flat_map(|(_, terms)| terms).collect();
        assert!(
            before.len() == if two_rooms { 2 } else { 1 }
                && before.values().all(|(_, terms)| terms.len() == 1)
                && old_terms.len() == 1,
            "{arm}: before the reload, each live room does not carry one content term, the \
             same one: {before:?}"
        );
        let old_term = (*old_terms.iter().next().expect("one term")).clone();

        let _copy = watch_an_edited_copy(&mut app, if two_rooms { "two" } else { "one" }, |project| {
            move_an_entity(project, &edited_room, moved)
        });
        let applied = app.world().resource::<WorldSourceHotReload>().applied_count;
        press_apply_reload(&mut app);

        let reload = app.world().resource::<WorldSourceHotReload>().clone();
        assert_eq!(
            (reload.applied_count, reload.last_status.contains("applied"), reload.last_errors.is_empty()),
            (applied + 1, true, true),
            "{arm}: the reload did not apply: {:?} / {:?}",
            reload.last_status,
            reload.last_errors
        );
        let after = live_room_readings(&mut app);
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
            if terms.contains(&old_term) {
                wrong.push(format!(
                    "`{room}` still carries the content term of the old generation: {terms:?}"
                ));
            }
            if terms.len() != 1 {
                wrong.push(format!("`{room}` does not carry one content term: {terms:?}"));
            }
        }
        let new_terms: std::collections::BTreeSet<&String> =
            after.values().flat_map(|(_, terms)| terms).collect();
        if new_terms.len() != 1 {
            wrong.push(format!("the live rooms are not of one generation: {new_terms:?}"));
        }
        assert!(
            wrong.is_empty(),
            "{arm}: a live room was not rebuilt from the reloaded world:\n  {}",
            wrong.join("\n  ")
        );
        // The session is on the new generation, one time.
        assert_eq!(
            the_only_prepared_epoch(&mut app),
            epoch_before + 1,
            "{arm}: the generation of the session did not move by one"
        );
        // Each publication of the sequence is ended: no receipt and no hidden
        // candidate is left, and each live room can be described.
        assert_eq!(
            (
                ambition_platformer2d::actors::rooms::outstanding_publications(app.world_mut()),
                ambition_platformer2d::platformer::construction::outstanding_candidates(app.world_mut()),
            ),
            (0, 0),
            "{arm}: (publication receipts, hidden candidates) left after the reload settled"
        );
        // ⚠ ROOM BY ROOM. A live identity is the pair (live room, `SimId`), and
        // two live room roots wear one `SimId`, so a capture of the whole
        // world is a `DuplicateIdentity` with two live rooms, before a reload
        // and after it (measured; the "Root identity" row of the open-world
        // plan).
        for (room, (instance, _)) in &after {
            let described =
                ambition_platformer2d::platformer::construction::TransactionBaseline::capture_for_session(
                    app.world_mut(),
                    ambition_platformer2d::platformer::lifecycle::SessionSpawnScope::UNSCOPED,
                    ambition_platformer2d::platformer::lifecycle::TransactionRooms::only(*instance),
                );
            assert!(
                described.is_ok(),
                "{arm}: the room `{room}` that the reload published cannot be described: {:?}",
                described.err()
            );
        }
        // The rooms stay as the reload left them.
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(
            live_room_readings(&mut app),
            after,
            "{arm}: the live rooms changed in the 30 frames after the reload"
        );
    }
}

/// ⛔ A RELOAD THAT CANNOT REBUILD ONE LIVE ROOM REBUILDS NONE.
///
/// Each plan is prepared before the first room is staged. The first room's
/// publication moves the session to the new room set, so a room that fails
/// after it leaves a mixed world. In each arm the changed copy gives one
/// live room a body that names a character nobody registered, so the plan of
/// that room does not prepare. The refusal names the room and the character,
/// and no live room and no generation changes.
///
/// - An ENEMY in Bob's room, which is not the room of the primary seat. The
///   control of the two below: an enemy was always refused when its room was
///   planned.
/// - A PERSON (`NpcSpawn`) in Bob's room.
/// - A PERSON in the one live room, the plain reload.
///
/// ⛔ The two person arms were a PANIC. A person is a placement, and the
/// preflight did not look at a placement, so the plan prepared and the
/// recipe stopped the game when the room was built
/// (`report_unprepared_character`).
#[test]
fn a_world_reload_that_cannot_rebuild_one_live_room_rebuilds_none() {
    use ambition_platformer2d::dev_tools::WorldSourceHotReload;
    const STRANGER: &str = "a_character_nobody_registered";

    // (the arm, Bob's room if there are two live rooms, the body that names
    // the stranger)
    for (arm, bobs_room, entity) in [
        ("an enemy in the room that is not the primary seat's", Some("basement_enemies"), "EnemySpawn"),
        ("a person in the room that is not the primary seat's", Some("basement_npcs"), "NpcSpawn"),
        ("a person in the one live room", None, "NpcSpawn"),
    ] {
        let (mut app, start) = match bobs_room {
            Some(room) => a_running_shipped_session_with_two_live_rooms(Leaver::Bob, room),
            None => {
                let app = a_running_shipped_session();
                let start = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
                    .expect("the session has a live room")
                    .id
                    .clone();
                (app, start)
            }
        };
        // ⛔ THE PREMISE: Alice is in the start room, so with two live rooms
        // the room that cannot be rebuilt is not the first room of the reload.
        {
            use crate::neighbor_prefetch_prepares_rooms::{alice, room_of};
            let body = alice(&mut app);
            assert_eq!(room_of(&app, body), Some(start.clone()), "{arm}: Alice is not in the start room");
        }
        let faulty_room = bobs_room.unwrap_or(start.as_str()).to_owned();
        let before = live_room_readings(&mut app);
        let epoch_before = the_only_prepared_epoch(&mut app);
        let applied = app.world().resource::<WorldSourceHotReload>().applied_count;

        let _copy = watch_an_edited_copy(&mut app, "refused", |project| {
            name_an_unknown_character(project, &faulty_room, entity, STRANGER)
        });
        press_apply_reload(&mut app);

        let reload = app.world().resource::<WorldSourceHotReload>().clone();
        assert_eq!(
            (reload.applied_count, reload.last_status.contains("rejected")),
            (applied, true),
            "{arm}: the reload was not refused: {:?} / {:?}",
            reload.last_status,
            reload.last_errors
        );
        assert!(
            reload
                .last_errors
                .iter()
                .any(|error| error.contains(&format!("`{faulty_room}`")) && error.contains(STRANGER)),
            "{arm}: the refusal does not name the room `{faulty_room}` and the character: {:?}",
            reload.last_errors
        );
        if bobs_room.is_some() {
            assert!(
                reload.last_errors.iter().any(|error| error.contains(&format!("live room '{faulty_room}'"))),
                "{arm}: the refusal does not say which LIVE room cannot be rebuilt: {:?}",
                reload.last_errors
            );
        }
        assert_eq!(
            (live_room_readings(&mut app), the_only_prepared_epoch(&mut app)),
            (before.clone(), epoch_before),
            "{arm}: ⛔ A REFUSED RELOAD REBUILT A LIVE ROOM, or moved the generation of the session"
        );
        // The game runs on.
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(live_room_readings(&mut app), before, "{arm}: the live rooms changed after the refusal");
    }
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
    let before_room = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
    .map(|spec| spec.id.clone())
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
        .resource::<ambition_platformer2d::actors::world::rooms::LastConstructionVerification>()
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
        ambition_platformer2d::world::rooms::sole_live_room_spec(app.world()).map(|spec| spec.id.clone()),
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
    // ⛔ A refused receipt is as consumed as an admitted one. The committed arm
    // above counts receipts; without this one, an owner that retired only on
    // success would pass every test on this road.
    assert_eq!(
        ambition_platformer2d::actors::rooms::outstanding_publications(app.world_mut()),
        0,
        "⛔ A REFUSED WORLD RELOAD'S PUBLICATION RECEIPT IS STILL STANDING"
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
    for _ in 0..240 {
        app.update();
    }

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
        app.world_mut()
            .resource_mut::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>()
            .adopt(remembered);
    }

    // A's MECHANICAL state as it stands while A is the only session. The
    // candidate carries its own `SessionMechanics` and installs it at adoption,
    // and its `MovingPlatformSet` is on its own hidden live room root; a
    // refused candidate must leave A's alone.
    let mechanics_before = app
        .world()
        .get_resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
        .map(|mechanics| format!("{mechanics:?}"));
    let platforms_before = ambition_platformer2d::session::sole_live_room_component::<ambition_platformer2d::world::collision::MovingPlatformSet>(app
        .world()).expect("the live room has moving platforms")
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
        .resource::<ambition_platformer2d::actors::world::rooms::LastConstructionVerification>()
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
            .get_resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
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
        !before.is_empty(),
        "the premise: world N is a populated session. Without it 'N was not \
         touched' would be true of an empty world"
    );

    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: gameplay,
        request: None,
    });
    let mut handed_over = false;
    let mut lost = std::collections::BTreeSet::new();
    for _ in 0..240 {
        app.update();
        match scope(&app) {
            // ⛔ THE ONLY FRAMES THIS ARM JUDGES. Once the active scope has
            // MOVED, N is retired by its own lifecycle and is supposed to empty.
            Some(current) if current == live => {
                lost.extend(before.difference(&population(&mut app, live)).cloned());
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
    assert!(
        lost.is_empty(),
        "⛔ THE CANDIDATE SESSION TOOK ENTITIES OUT OF THE WORLD THAT WAS STILL \
         PLAYING. Its first room plans the same authored ids, and a verifier that \
         does not ask WHOSE world it is looking at declares them superseded and \
         despawns them — before anything has decided the candidate may be played: \
         {lost:?}"
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
        .resource::<ambition_platformer2d::actors::world::rooms::LastConstructionVerification>()
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
    // installs 1 moving platform on the live room's root, and the cancel arm
    // ends with no live room at all, so the pair is a real discriminator.
    assert!(
        !ambition_platformer2d::session::sole_live_room_component::<ambition_platformer2d::world::collision::MovingPlatformSet>(app.world()).expect("the live room has moving platforms")
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
    // registries, a process-global RESOURCE installed by ADOPTION; a candidate
    // builder that wrote it during preparation would have changed live
    // mechanical state for a session that was never admitted.
    // `MovingPlatformSet`, the first room's platform state, is on the
    // candidate's own hidden live room root since OW1 cut 3b, so an abandoned
    // candidate leaves no VISIBLE live room at all. No session has ever been live in this arm, which is
    // what makes their ABSENCE meaningful.
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
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
            .get_resource::<ambition_platformer2d::actors::world::rooms::LastConstructionVerification>()
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

// ─────────────────────────────────────────────────────────────────────────────
// CANDIDATE-GENERATION-ORDER: the candidate is prepared against ITS audio.
// ─────────────────────────────────────────────────────────────────────────────

/// The shipped app, activated on its gameplay route and settled.
fn app_playing_gameplay() -> bevy::prelude::App {
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
    assert_eq!(
        active_route(&app).as_deref(),
        Some("ambition_gameplay"),
        "the shipped composition never activated its gameplay route"
    );
    app
}

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

/// ⛔⛤ **A CANDIDATE THAT DROPS A PROVIDER'S WHOLE MUSIC OR SFX FRAGMENT IS
/// REFUSED BY ITS OWN PREPARATION, AND THE LIVE SESSION KEEPS ITS AUDIO.**
///
/// N has Ambition's music and SFX. N+1 stops declaring one registry. The
/// provider EXPECTS both (`with_music`, `with_procedural_sfx`), so a preparation
/// that sees N+1's audio fails with "Provider music is not ready". Preparation
/// used to read the App's registry, which is N until the commit, found the
/// fragment, admitted the session, and the commit then published a catalog with
/// no music.
///
/// ⭐ Nothing here fakes the boundary: the request, the shell's lifecycle and
/// the provider's preparation all run as in the game. The control below edits
/// the SAME registry without dropping it and activates, so the refusal is about
/// presence and not about the audio domain being unreloadable.
#[test]
fn a_candidate_that_drops_a_providers_audio_is_refused_and_the_live_audio_survives() {
    for (omitted, what) in [
        ("audio/sfx_registry.ron", "SFX"),
    ] {
        let mut app = app_playing_gameplay();
        assert_eq!(
            provider_audio(&app),
            (true, true),
            "the premise: N carries both of the provider's audio registries"
        );
        let live_activation = activation_id(&app).expect("a live session");
        let base = ambition_content::pack::selected(app.world())
            .expect("a selection")
            .fingerprint;

        let candidate = std::sync::Arc::new(
            ambition_content::pack::compile_pack_omitting(&[omitted])
                .expect("a pack that stops declaring one registry compiles"),
        );
        assert_ne!(candidate.fingerprint, base, "the premise: N+1 differs from N");
        let outcome = ambition_content::reload::request_reload(
            app.world_mut(),
            ambition_content::CandidateGeneration::prepared_against(
                std::sync::Arc::clone(&candidate),
                Some(base),
            ),
        );
        assert!(
            matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }),
            "dropping the {what} registry was refused at REQUEST time, so the \
             candidate-preparation question this arm asks never arises: {outcome:?}"
        );
        for _ in 0..240 {
            app.update();
        }

        assert_eq!(
            activation_id(&app),
            Some(live_activation),
            "⛔ A CANDIDATE WITHOUT THE PROVIDER'S {what} ACTIVATED: its preparation \
             was answered from the App's audio, which is still N"
        );
        assert_eq!(
            provider_audio(&app),
            (true, true),
            "⛔ A REFUSED CANDIDATE CHANGED THE LIVE AUDIO CATALOG"
        );
        assert_eq!(
            ambition_content::pack::selected(app.world())
                .expect("a selection")
                .fingerprint,
            base,
            "a refused candidate became the App's selection"
        );
        assert!(
            ambition_content::reload::pending_pack(app.world()).is_none(),
            "a refused candidate is still pending"
        );
        let held = app
            .world()
            .resource::<ambition_platformer2d::game_shell::ShellRouteHolds>()
            .held(&ShellRouteId::new("ambition_gameplay"));
        // The load presentation keeps its own ready-hold while the failure is
        // on screen; the reload's holds are the `content-publication:` ones.
        let leaked: Vec<_> = held
            .iter()
            .filter(|hold| format!("{hold:?}").contains("content-publication:"))
            .collect();
        assert!(leaked.is_empty(), "a refused candidate's reload hold outlived it: {leaked:?}");
    }
}

/// The control for the arm above: the SAME registry, edited and not dropped,
/// activates and publishes. Without it the refusal could be the audio domain
/// being unreloadable, or a harness that never activates anything.
#[test]
fn a_candidate_that_edits_a_providers_audio_activates_and_publishes_it() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world())
        .expect("a selection")
        .fingerprint;
    let first_frequency = |app: &bevy::prelude::App| {
        app.world()
            .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>()
            .sfx_for(ambition_content::AMBITION_CONTENT_PROVIDER)
            .expect("the provider's SFX")
            .sfx[0]
            .frequency
    };
    let before = first_frequency(&app);
    let mut edited = false;
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            if declared == "audio/sfx_registry.ron" {
                let out = text.replacen(
                    "frequency: 460.0, frequency_end: 720.0,",
                    "frequency: 461.0, frequency_end: 720.0,",
                    1,
                );
                edited = out != text;
                return out;
            }
            text
        })
        .expect("compiles"),
    );
    assert!(edited, "the SFX registry no longer states the edited cue");
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(candidate, Some(base)),
    );
    assert!(
        matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }),
        "{outcome:?}"
    );
    assert_eq!(first_frequency(&app), before, "the request published on the spot");
    for _ in 0..240 {
        app.update();
    }
    assert_ne!(activation_id(&app), Some(live_activation), "the edit never activated");
    assert_eq!(first_frequency(&app), before + 1.0, "the edited cue was not published");
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

/// The pack with the goblin binding starting on `wave1` (and, when asked, the
/// SFX cue's frequency edited, so a second participating family changes with it).
#[cfg(feature = "audio")]
fn pack_with_goblin_starting_on_wave1(
    also_edit_sfx: bool,
) -> std::sync::Arc<ambition_platformer2d::content::PreparedContentPack> {
    let mut cue_edited = false;
    let pack = ambition_content::pack::compile_pack_with(|declared, text| {
        if declared == "audio/music_cues.ron" {
            let out = text.replacen(r#"starting_state: "intro""#, r#"starting_state: "wave1""#, 1);
            cue_edited = out != text;
            return out;
        }
        if also_edit_sfx && declared == "audio/sfx_registry.ron" {
            return text.replacen(
                "frequency: 460.0, frequency_end: 720.0,",
                "frequency: 461.0, frequency_end: 720.0,",
                1,
            );
        }
        text
    })
    .expect("an edited binding compiles");
    assert!(cue_edited, "the cue file no longer states the edited binding");
    std::sync::Arc::new(pack)
}

#[cfg(feature = "audio")]
fn first_sfx_frequency(app: &bevy::prelude::App) -> f32 {
    app.world()
        .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>()
        .sfx_for(ambition_content::AMBITION_CONTENT_PROVIDER)
        .expect("the provider's SFX")
        .sfx[0]
        .frequency
}

/// ⭐ **A CUE EDIT IS PLAYED, AND IT BECOMES VISIBLE AT THE ACTIVATION BOUNDARY
/// TOGETHER WITH EVERY OTHER CHANGED FAMILY.**
///
/// One candidate edits the goblin lab's cue binding AND an SFX cue. Frame by
/// frame the test records three facts: has the shell activated a new session,
/// does the adaptive catalog state N+1, does the SFX catalog state N+1. All
/// three must flip on the SAME frame, and not before the request's frame has
/// passed. A cue catalog published at request time flips first; one published
/// a frame late (an ordering edge instead of the commit) flips last.
#[cfg(feature = "audio")]
#[test]
fn an_adaptive_cue_edit_is_visible_with_its_session_and_with_the_other_changed_families() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let sfx_before = first_sfx_frequency(&app);
    assert_eq!(
        goblin_binding(&app),
        ("first_goblin_tune_v2".to_string(), "intro".to_string()),
        "the premise: N starts the goblin cue on its intro"
    );

    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(
            pack_with_goblin_starting_on_wave1(true),
            Some(base),
        ),
    );
    assert!(
        matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        goblin_binding(&app).1,
        "intro",
        "⛔ the request published the cue catalog on the spot"
    );

    let mut first_flip = None;
    for frame in 0..240 {
        app.update();
        let seen = (
            activation_id(&app) != Some(live_activation),
            goblin_binding(&app).1 == "wave1",
            first_sfx_frequency(&app) == sfx_before + 1.0,
        );
        if seen != (false, false, false) {
            first_flip = Some((frame, seen));
            break;
        }
    }
    let (frame, seen) = first_flip.expect("the edit never reached the game");
    assert_eq!(
        seen,
        (true, true, true),
        "⛔ ON FRAME {frame} THE FAMILIES WERE NOT VISIBLE TOGETHER \
         (activation, cue catalog, sfx): {seen:?}"
    );
}

/// ⛔ **A CANDIDATE THAT CHANGES THE CUES AND IS THEN REFUSED LEAVES THE CUES
/// AT N.**
///
/// N+1 starts the goblin cue on wave1 AND drops the SFX registry. The cue edit
/// is valid, the dropped registry is refused by the candidate's own preparation
/// (the arm above), so nothing may publish: the catalog still states `intro`,
/// the selection is N, and the live session is the one that was running.
#[cfg(feature = "audio")]
#[test]
fn a_refused_candidate_leaves_the_adaptive_cues_at_the_live_generation() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;

    let mut cue_edited = false;
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_omitting_with(&["audio/sfx_registry.ron"], |declared, text| {
            if declared == "audio/music_cues.ron" {
                let out = text.replacen(r#"starting_state: "intro""#, r#"starting_state: "wave1""#, 1);
                cue_edited = out != text;
                return out;
            }
            text
        })
        .expect("compiles"),
    );
    assert!(cue_edited, "the cue file no longer states the edited binding");
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(candidate, Some(base)),
    );
    assert!(
        matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }),
        "refused at request time, so the preparation arm never runs: {outcome:?}"
    );
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(activation_id(&app), Some(live_activation), "the refused candidate activated");
    assert_eq!(
        goblin_binding(&app).1,
        "intro",
        "⛔ A REFUSED CANDIDATE'S CUE CATALOG WAS PUBLISHED"
    );
    assert_eq!(
        ambition_content::pack::selected(app.world()).expect("a selection").fingerprint,
        base,
        "a refused candidate became the App's selection"
    );
}

/// Identical content is a no-op, and a candidate prepared against a generation
/// that has since moved is refused as stale, so it cannot fold its cue edit
/// into a catalog it did not read.
#[cfg(feature = "audio")]
#[test]
fn an_unchanged_or_stale_cue_candidate_publishes_nothing() {
    let mut app = app_playing_gameplay();
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;

    // No-op: the shipped pack again.
    let same = std::sync::Arc::new(ambition_content::pack::compile_pack_with(|_, text| text).expect("compiles"));
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(same, Some(base)),
    );
    assert!(
        matches!(outcome, ambition_content::reload::ReloadRequest::Unchanged),
        "identical content minted a generation: {outcome:?}"
    );

    // Publish a real edit, so `base` is now behind.
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(
            pack_with_goblin_starting_on_wave1(false),
            Some(base),
        ),
    );
    assert!(matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }), "{outcome:?}");
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(goblin_binding(&app).1, "wave1", "the premise: the edit published");
    let published = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    assert_ne!(published, base);

    // Stale: another cue edit, still prepared against the OLD base.
    let stale = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            if declared == "audio/music_cues.ron" {
                return text.replacen(r#"starting_state: "intro""#, r#"starting_state: "wave2""#, 1);
            }
            text
        })
        .expect("compiles"),
    );
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(stale, Some(base)),
    );
    assert!(
        matches!(
            outcome,
            ambition_content::reload::ReloadRequest::Refused(
                ambition_content::reload::MoveReload::StaleGeneration { .. }
            )
        ),
        "a stale candidate was not refused as stale: {outcome:?}"
    );
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(goblin_binding(&app).1, "wave1", "a stale candidate changed the cue catalog");
    assert_eq!(
        ambition_content::pack::selected(app.world()).expect("a selection").fingerprint,
        published
    );
}

/// ⛔⛤ **A CANDIDATE THAT DROPS THE PROVIDER'S ADAPTIVE CUES IS REFUSED BY ITS
/// OWN PREPARATION, AND THE LIVE SESSION KEEPS ITS CUES.**
///
/// A pack without `audio/music_cues.ron` compiles (measured; the lowering used
/// to claim otherwise). The provider EXPECTS adaptive cues, so a preparation
/// that sees N+1's catalog fails with "Adaptive music is not ready". Preparation
/// used to read the App's registry, which is N until the commit, found the
/// catalog, admitted the session, and the commit then published a registry
/// without it. The control is the edited-binding arm above: the same family,
/// changed and not dropped, activates.
#[cfg(feature = "audio")]
#[test]
fn a_candidate_that_drops_the_adaptive_cues_is_refused_and_the_live_cues_survive() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    assert_eq!(goblin_binding(&app).1, "intro", "the premise: N has the provider's cues");

    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_omitting(&["audio/music_cues.ron"])
            .expect("a pack that stops declaring its cues compiles"),
    );
    assert_ne!(candidate.fingerprint, base, "the premise: N+1 differs from N");
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(candidate, Some(base)),
    );
    assert!(
        matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }),
        "refused at REQUEST time, so the preparation question this arm asks never arises: {outcome:?}"
    );
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(
        activation_id(&app),
        Some(live_activation),
        "⛔ A CANDIDATE WITHOUT THE PROVIDER'S ADAPTIVE CUES ACTIVATED: its preparation was \
         answered from the App's cue catalog, which is still N"
    );
    assert_eq!(goblin_binding(&app).1, "intro", "⛔ A REFUSED CANDIDATE CHANGED THE LIVE CUES");
    assert_eq!(
        ambition_content::pack::selected(app.world()).expect("a selection").fingerprint,
        base,
        "a refused candidate became the App's selection"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// I2/I3: the cutscene library takes part in a reload.
// ─────────────────────────────────────────────────────────────────────────────

/// The text of `test_intro`'s opening banner in the App's cutscene library.
fn boot_banner(app: &bevy::prelude::App) -> String {
    match app
        .world()
        .resource::<ambition_platformer2d::cutscene::CutsceneLibrary>()
        .get("test_intro")
        .expect("the boot cutscene")
        .beats
        .first()
        .expect("a first beat")
    {
        ambition_platformer2d::cutscene::CutsceneBeat::Banner { text, .. } => text.clone(),
        other => panic!("test_intro no longer opens on a banner: {other:?}"),
    }
}

/// `test_intro`'s banner reworded, optionally with an SFX cue edited too (a
/// second participating family) or the SFX registry dropped (a refusal).
fn pack_with_a_reworded_boot_banner(
    also_edit_sfx: bool,
    drop_sfx: bool,
) -> std::sync::Arc<ambition_platformer2d::content::PreparedContentPack> {
    let mut edited = false;
    let edit = |declared: &str, text: String| {
        if declared == "data/cutscenes/sandbox.ron" {
            let out = text.replacen("// boot sequence", "// boot sequence, revised", 1);
            edited = out != text;
            return out;
        }
        if also_edit_sfx && declared == "audio/sfx_registry.ron" {
            return text.replacen(
                "frequency: 460.0, frequency_end: 720.0,",
                "frequency: 461.0, frequency_end: 720.0,",
                1,
            );
        }
        text
    };
    let pack = if drop_sfx {
        ambition_content::pack::compile_pack_omitting_with(&["audio/sfx_registry.ron"], edit)
    } else {
        ambition_content::pack::compile_pack_with(edit)
    }
    .expect("an edited cutscene compiles");
    assert!(edited, "the cutscene file no longer states the edited banner");
    std::sync::Arc::new(pack)
}

/// ⭐ **A CUTSCENE EDIT IS VISIBLE WITH ITS SESSION AND WITH EVERY OTHER CHANGED
/// FAMILY, ON THE SAME FRAME.** The candidate rewords the boot banner and edits
/// an SFX cue; frame by frame the test records whether the shell activated a new
/// session, whether the library states N+1, and whether the SFX catalog does.
#[test]
fn a_cutscene_edit_is_visible_with_its_session_and_with_the_other_changed_families() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let sfx_before = first_sfx_frequency_of(&app);
    assert_eq!(boot_banner(&app), "// boot sequence", "the premise: N's banner");

    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(
            pack_with_a_reworded_boot_banner(true, false),
            Some(base),
        ),
    );
    assert!(matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }), "{outcome:?}");
    assert_eq!(boot_banner(&app), "// boot sequence", "⛔ the request published the library on the spot");

    let mut first_flip = None;
    for frame in 0..240 {
        app.update();
        let seen = (
            activation_id(&app) != Some(live_activation),
            boot_banner(&app) == "// boot sequence, revised",
            first_sfx_frequency_of(&app) == sfx_before + 1.0,
        );
        if seen != (false, false, false) {
            first_flip = Some((frame, seen));
            break;
        }
    }
    let (frame, seen) = first_flip.expect("the edit never reached the game");
    assert_eq!(
        seen,
        (true, true, true),
        "⛔ ON FRAME {frame} THE FAMILIES WERE NOT VISIBLE TOGETHER (activation, cutscenes, sfx): {seen:?}"
    );
}

fn first_sfx_frequency_of(app: &bevy::prelude::App) -> f32 {
    app.world()
        .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>()
        .sfx_for(ambition_content::AMBITION_CONTENT_PROVIDER)
        .expect("the provider's SFX")
        .sfx[0]
        .frequency
}

/// ⛔ A candidate that edits a cutscene and is then REFUSED (its SFX registry is
/// dropped; the provider expects it) leaves the library at N: the same
/// retention law as the audio and cue families.
#[test]
fn a_refused_candidate_leaves_the_cutscene_library_at_the_live_generation() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(
            pack_with_a_reworded_boot_banner(false, true),
            Some(base),
        ),
    );
    assert!(matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }), "{outcome:?}");
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(activation_id(&app), Some(live_activation), "the refused candidate activated");
    assert_eq!(boot_banner(&app), "// boot sequence", "⛔ A REFUSED CANDIDATE'S CUTSCENES WERE PUBLISHED");
    assert_eq!(
        ambition_content::pack::selected(app.world()).expect("a selection").fingerprint,
        base,
        "a refused candidate became the App's selection"
    );
}

/// Identical content is a no-op, and a candidate prepared against a generation
/// that has since moved is refused as stale and cannot fold its edit in.
#[test]
fn an_unchanged_or_stale_cutscene_candidate_publishes_nothing() {
    let mut app = app_playing_gameplay();
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let same = std::sync::Arc::new(ambition_content::pack::compile_pack_with(|_, text| text).expect("compiles"));
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(same, Some(base)),
    );
    assert!(matches!(outcome, ambition_content::reload::ReloadRequest::Unchanged), "{outcome:?}");

    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(
            pack_with_a_reworded_boot_banner(false, false),
            Some(base),
        ),
    );
    assert!(matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }), "{outcome:?}");
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(boot_banner(&app), "// boot sequence, revised", "the premise: the edit published");

    let stale = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            if declared == "data/cutscenes/sandbox.ron" {
                return text.replacen("// boot sequence", "// boot sequence, stale", 1);
            }
            text
        })
        .expect("compiles"),
    );
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(stale, Some(base)),
    );
    assert!(
        matches!(
            outcome,
            ambition_content::reload::ReloadRequest::Refused(
                ambition_content::reload::MoveReload::StaleGeneration { .. }
            )
        ),
        "a stale candidate was not refused as stale: {outcome:?}"
    );
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(boot_banner(&app), "// boot sequence, revised", "a stale candidate changed the library");
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

/// ⭐ **A QUEST EDIT IS PLAYED FROM THE NEXT SESSION, WITH THE PLAYER'S PROGRESS,
/// AND NEVER N'S BOOK IN THE NEW SESSION.**
///
/// The registry is session state: the new session's first tick fills it from the
/// selected pack and the save. The candidate retitles a quest AND edits an SFX
/// cue; the player stands on the quest's second step. Frame by frame: when the
/// shell activates the new session and the SFX catalog states N+1, the quest
/// registry must not state N (it is empty until its first tick, then N+1), and
/// settled it says N+1's title at the SAME place.
#[test]
fn a_quest_edit_is_played_from_the_next_session_with_the_players_progress() {
    let mut app = app_playing_gameplay();
    advance_first_steps_one_step(&mut app);
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let sfx_before = first_sfx_frequency_of(&app);
    assert_eq!(quest_place(&app, "first_steps").map(|place| place.0).as_deref(), Some("First Steps"));

    let mut edited = (false, false);
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            if declared == "data/quests.ron" {
                let out = text.replacen(r#"title: "First Steps""#, r#"title: "First Steps, revised""#, 1);
                edited.0 = out != text;
                return out;
            }
            if declared == "audio/sfx_registry.ron" {
                let out = text.replacen(
                    "frequency: 460.0, frequency_end: 720.0,",
                    "frequency: 461.0, frequency_end: 720.0,",
                    1,
                );
                edited.1 = out != text;
                return out;
            }
            text
        })
        .expect("compiles"),
    );
    assert_eq!(edited, (true, true), "the pack no longer states the edited quest title and cue");
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(candidate, Some(base)),
    );
    assert!(matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }), "{outcome:?}");
    assert_eq!(
        quest_place(&app, "first_steps").map(|place| place.0).as_deref(),
        Some("First Steps"),
        "⛔ the request changed the live quest book"
    );

    let mut flip = None;
    for frame in 0..240 {
        app.update();
        let activated = activation_id(&app) != Some(live_activation);
        let title = quest_place(&app, "first_steps").map(|place| place.0);
        if !activated {
            assert_eq!(title.as_deref(), Some("First Steps"), "N's session lost its quest book early");
            continue;
        }
        flip = Some((frame, first_sfx_frequency_of(&app) == sfx_before + 1.0, title));
        break;
    }
    let (frame, sfx_is_new, title) = flip.expect("the edit never activated");
    assert!(sfx_is_new, "frame {frame}: the session activated before the SFX family published");
    assert_ne!(
        title.as_deref(),
        Some("First Steps"),
        "⛔ frame {frame}: THE NEW SESSION SERVES N'S QUEST BOOK"
    );
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        quest_place(&app, "first_steps"),
        Some(("First Steps, revised".to_string(), true, 1)),
        "the new session's quest book is not N+1's with the player's progress"
    );
}

/// ⛔ A candidate that edits the quest book and is then REFUSED (its SFX registry
/// is dropped) leaves the quest book, and the player's place in it, at N.
#[test]
fn a_refused_candidate_leaves_the_quest_book_at_the_live_generation() {
    let mut app = app_playing_gameplay();
    advance_first_steps_one_step(&mut app);
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let mut edited = false;
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_omitting_with(&["audio/sfx_registry.ron"], |declared, text| {
            if declared == "data/quests.ron" {
                let out = text.replacen(r#"title: "First Steps""#, r#"title: "First Steps, revised""#, 1);
                edited = out != text;
                return out;
            }
            text
        })
        .expect("compiles"),
    );
    assert!(edited);
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(candidate, Some(base)),
    );
    assert!(matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }), "{outcome:?}");
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(activation_id(&app), Some(live_activation), "the refused candidate activated");
    assert_eq!(
        quest_place(&app, "first_steps"),
        Some(("First Steps".to_string(), true, 1)),
        "⛔ A REFUSED CANDIDATE CHANGED THE QUEST BOOK OR THE PLAYER'S PLACE IN IT"
    );
}

/// ⛔ **A QUEST BOOK THAT HAS NO PLACE FOR A RECORDED STEP IS REFUSED, NOT
/// CLAMPED.** The same candidate (`first_steps` cut to one step) is admitted
/// while the player is on step 0 and refused, at request time, once the save
/// records step 1, with the live book and the staged state untouched.
#[test]
fn a_quest_book_with_no_place_for_a_recorded_step_is_refused() {
    // The control: nobody has progressed, so one step is enough.
    let mut app = app_playing_gameplay();
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(pack_with_a_one_step_first_quest(), Some(base)),
    );
    assert!(
        matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }),
        "the control candidate was refused although no progress is lost: {outcome:?}"
    );
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(
        app.world()
            .resource::<ambition_content::quest::QuestRegistry>()
            .get("first_steps")
            .map(|state| state.spec.steps.len()),
        Some(1),
        "the control candidate never reached the new session"
    );

    // The arm: the player is past step 0.
    let mut app = app_playing_gameplay();
    advance_first_steps_one_step(&mut app);
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(pack_with_a_one_step_first_quest(), Some(base)),
    );
    assert!(
        matches!(
            outcome,
            ambition_content::reload::ReloadRequest::Refused(
                ambition_content::reload::MoveReload::QuestBookRefused(ref why)
            ) if why.contains("first_steps")
        ),
        "a quest book with no place for step 1 was not refused: {outcome:?}"
    );
    assert!(ambition_content::reload::pending_pack(app.world()).is_none(), "a refused candidate is pending");
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(activation_id(&app), Some(live_activation), "a refused candidate activated");
    assert_eq!(quest_place(&app, "first_steps"), Some(("First Steps".to_string(), true, 1)));
}

/// ⛔ **A SAVE THAT MOVES WHILE A GENERATION WAITS CANCELS IT, NOT CLAMPS IT.**
/// The candidate (`first_steps` cut to one step) is admitted while the player is
/// on step 0. Before it activates the player reaches step 1, and the book has no
/// place for them. The activation gate asks the same question again with the
/// save as it is then, and refuses: the session is not replaced and the player
/// stays on their step in the live book.
#[test]
fn a_quest_book_that_loses_its_place_while_the_generation_waits_is_cancelled() {
    let mut app = app_playing_gameplay();
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
    // The save moves under the waiting generation.
    app.world_mut()
        .resource_mut::<ambition_content::quest::QuestRegistry>()
        .push_event(ambition_platformer2d::persistence::quest::QuestAdvanceEvent::FlagSet("met_any_hub_npc".to_string()));
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(
        activation_id(&app),
        Some(live_activation),
        "⛔ THE GENERATION ACTIVATED OVER A SAVE IT HAS NO PLACE FOR"
    );
    assert!(ambition_content::reload::pending_pack(app.world()).is_none(), "the refused generation is still pending");
    let leaked: Vec<_> = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellRouteHolds>()
        .held(&ShellRouteId::new("ambition_gameplay"))
        .iter()
        .filter(|hold| format!("{hold:?}").contains("content-publication:"))
        .map(|hold| format!("{hold:?}"))
        .collect();
    assert!(leaked.is_empty(), "the refused generation's hold outlived it: {leaked:?}");
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
}

/// ⛔ **A PROVIDER THAT TAKES AN AMBITION CUTSCENE ID WHILE A GENERATION WAITS
/// CANCELS IT, NOT OVERWRITTEN BY IT.** The candidate rewords `test_intro` and is
/// admitted while the library holds Ambition's row. Before it activates another
/// provider replaces that row (same id). The publication cannot refuse, so the
/// activation gate asks the ownership question again and cancels: the foreign row
/// is still there afterwards and the session is not replaced.
#[test]
fn a_provider_that_takes_a_cutscene_id_while_the_generation_waits_cancels_it() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let requested = request_the_banner(&mut app, "// a banner the foreign row must survive", false);
    assert!(
        matches!(requested, ambition_content::reload::ReloadRequest::Requested { .. }),
        "the premise: admitted while the library held Ambition's own row: {requested:?}"
    );
    app.world_mut()
        .resource_mut::<ambition_platformer2d::cutscene::CutsceneLibrary>()
        .insert(ambition_platformer2d::cutscene::CutsceneScript::new(
            "test_intro",
            vec![ambition_platformer2d::cutscene::CutsceneBeat::Banner {
                text: "// the other provider".to_string(),
                seconds: 1.0,
            }],
        ));
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(
        activation_id(&app),
        Some(live_activation),
        "⛔ THE GENERATION ACTIVATED OVER ANOTHER PROVIDER'S CUTSCENE"
    );
    assert!(ambition_content::reload::pending_pack(app.world()).is_none(), "the refused generation is still pending");
    assert_eq!(boot_banner(&app), "// the other provider", "⛔ THE FOREIGN ROW WAS OVERWRITTEN");
}

/// The control for the gate's quest question: the SAME moving save under a
/// candidate that does not touch the quest book activates. Without it the
/// cancel above could be any cause; with it the quest question is the only
/// thing that differs.
#[test]
fn a_save_that_moves_while_a_generation_without_quest_changes_waits_does_not_cancel_it() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let requested = request_the_banner(&mut app, "// a banner, no quest edit", false);
    assert!(matches!(requested, ambition_content::reload::ReloadRequest::Requested { .. }), "{requested:?}");
    app.world_mut()
        .resource_mut::<ambition_content::quest::QuestRegistry>()
        .push_event(ambition_platformer2d::persistence::quest::QuestAdvanceEvent::FlagSet("met_any_hub_npc".to_string()));
    for _ in 0..240 {
        app.update();
    }
    assert_ne!(
        activation_id(&app),
        Some(live_activation),
        "a candidate with no quest edit was cancelled by a save that moved"
    );
    assert_eq!(boot_banner(&app), "// a banner, no quest edit", "the candidate never published");
    assert_eq!(
        quest_place(&app, "first_steps").map(|(_, active, step)| (active, step)),
        Some((true, 1)),
        "the player's step did not survive the activation"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// I3: a newer candidate supersedes the generation in flight.
// ─────────────────────────────────────────────────────────────────────────────

/// A pack that rewords the boot banner to `text` (the observable the supersession
/// arms read) and optionally edits an SFX cue (a second family).
fn pack_with_the_boot_banner(text: &'static str, also_edit_sfx: bool) -> std::sync::Arc<ambition_platformer2d::content::PreparedContentPack> {
    let mut edited = false;
    let pack = ambition_content::pack::compile_pack_with(|declared, source| {
        if declared == "data/cutscenes/sandbox.ron" {
            let out = source.replacen("// boot sequence", text, 1);
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
            pack_with_the_boot_banner(text, also_edit_sfx),
            Some(base),
        ),
    )
}

/// ⭐ **THE NEWEST CANDIDATE WINS, AND THE SUPERSEDED ONE NEVER BECOMES VISIBLE.**
///
/// Candidate A is in flight (requested, adopted, not yet activated) when B is
/// requested. A is cancelled; B activates alone. Frame by frame the library's
/// banner is read, and A's text must never be seen; the shell activates exactly
/// one new session (B's), and B's second family (an SFX cue) arrives with it.
#[test]
fn a_newer_candidate_supersedes_the_one_in_flight_and_the_old_one_is_never_seen() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let sfx_before = first_sfx_frequency_of(&app);

    let a = request_the_banner(&mut app, "// A, which is superseded", false);
    let ambition_content::reload::ReloadRequest::Requested { request: a_request, superseded: None, .. } = a else {
        panic!("A was not requested: {a:?}");
    };
    let a_hold = ambition_platformer2d::game_shell::ShellHoldId::new(format!(
        "content-publication:{}",
        a_request.as_str()
    ));
    // Two frames: A's transaction exists and is ADOPTED (so it holds the route
    // and has a claim), and it has not activated.
    app.update();
    app.update();
    assert!(ambition_content::reload::pending_pack(app.world()).is_some(), "the premise: A is in flight");
    assert!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::ShellRouteHolds>()
            .held(&ShellRouteId::new("ambition_gameplay"))
            .iter()
            .any(|hold| format!("{hold:?}").contains("content-publication:")),
        "the premise: A has been adopted and holds the route"
    );
    assert_eq!(activation_id(&app), Some(live_activation), "the premise: A has not activated");

    let b = request_the_banner(&mut app, "// B, which wins", true);
    let ambition_content::reload::ReloadRequest::Requested { superseded: Some(_), .. } = b else {
        panic!("B did not supersede A: {b:?}");
    };

    // A's activation gate is forgotten with A: a registration that outlives its
    // transaction is a hold nobody will ever release.
    assert!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::ShellActivationGates>()
            .evaluator(&a_hold)
            .is_none(),
        "⛔ THE SUPERSEDED GENERATION'S ACTIVATION GATE WAS LEFT REGISTERED"
    );
    let mut activations = std::collections::BTreeSet::new();
    for _ in 0..240 {
        app.update();
        let banner = boot_banner(&app);
        assert_ne!(banner, "// A, which is superseded", "⛔ THE SUPERSEDED CANDIDATE'S CUTSCENES WERE PUBLISHED");
        activations.insert(activation_id(&app));
    }
    assert_eq!(boot_banner(&app), "// B, which wins", "the newest candidate never published");
    assert_eq!(first_sfx_frequency_of(&app), sfx_before + 1.0, "B's second family did not arrive with it");
    assert_eq!(
        activations.len(),
        2,
        "the shell activated {activations:?}: the live session and B's, and no session of A's"
    );
    assert!(ambition_content::reload::pending_pack(app.world()).is_none(), "a generation is still pending");
    let held = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellRouteHolds>()
        .held(&ShellRouteId::new("ambition_gameplay"));
    let leaked: Vec<_> = held
        .iter()
        .filter(|hold| format!("{hold:?}").contains("content-publication:"))
        .collect();
    assert!(leaked.is_empty(), "a cancelled generation's hold outlived it: {leaked:?}");
}

/// A revert in flight: A is requested, then the content goes back to what is
/// running. A is cancelled, nothing replaces it, and the game never leaves N.
#[test]
fn reverting_the_content_while_a_generation_is_in_flight_cancels_it() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let a = request_the_banner(&mut app, "// A, which is reverted", false);
    assert!(matches!(a, ambition_content::reload::ReloadRequest::Requested { .. }), "{a:?}");
    app.update();
    assert!(ambition_content::reload::pending_pack(app.world()).is_some(), "the premise: A is in flight");

    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let live = std::sync::Arc::new(ambition_content::pack::compile_pack_with(|_, text| text).expect("compiles"));
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(live, Some(base)),
    );
    assert!(
        matches!(outcome, ambition_content::reload::ReloadRequest::CancelledInFlight { .. }),
        "{outcome:?}"
    );
    for _ in 0..240 {
        app.update();
        assert_eq!(boot_banner(&app), "// boot sequence", "⛔ THE REVERTED CANDIDATE'S CUTSCENES WERE PUBLISHED");
    }
    assert_eq!(activation_id(&app), Some(live_activation), "the cancelled candidate's session activated");
    assert!(ambition_content::reload::pending_pack(app.world()).is_none());
}


// ─────────────────────────────────────────────────────────────────────────────
// I2/I3: a candidate is judged against the world that is running.
// ─────────────────────────────────────────────────────────────────────────────

/// Request `candidate` against the shipped game and say what happened, with the
/// live generation untouched afterwards.
fn request_and_expect_a_graph_refusal(
    candidate: std::sync::Arc<ambition_platformer2d::content::PreparedContentPack>,
) -> Vec<String> {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let started = std::time::Instant::now();
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(candidate, Some(base)),
    );
    eprintln!("[content-graph] the candidate was judged in {:?}", started.elapsed());
    let ambition_content::reload::ReloadRequest::Refused(ambition_content::reload::MoveReload::ContentGraphRefused(errors)) =
        outcome
    else {
        panic!("a candidate that breaks the content graph was not refused as such: {outcome:?}");
    };
    assert!(ambition_content::reload::pending_pack(app.world()).is_none(), "a refused candidate is pending");
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(activation_id(&app), Some(live_activation), "a refused candidate activated");
    assert_eq!(
        ambition_content::pack::selected(app.world()).expect("a selection").fingerprint,
        base,
        "a refused candidate became the App's selection"
    );
    errors
}

/// ⛔ **A CANDIDATE THAT REMOVES A CUTSCENE THE WORLD STILL NAMES IS REFUSED.**
/// The hub's `entry_cutscene` field names `test_intro`; the candidate renames the
/// script, so the room's binding would silently never play
/// (`drain_cutscene_triggers` skips a missing script). Startup refuses the same
/// graph; the reload now asks the same judge.
#[test]
fn a_candidate_that_removes_a_cutscene_a_room_names_is_refused() {
    let mut edited = false;
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            if declared == "data/cutscenes/sandbox.ron" {
                let out = text.replacen(r#"id: "test_intro","#, r#"id: "test_intro_renamed","#, 1);
                edited = out != text;
                return out;
            }
            text
        })
        .expect("a renamed script compiles"),
    );
    assert!(edited, "the cutscene file no longer states the renamed script");
    let errors = request_and_expect_a_graph_refusal(candidate);
    assert!(
        errors.iter().any(|error| error.contains("test_intro") && error.contains("unknown cutscene")),
        "the refusal does not name the dead binding: {errors:?}"
    );
}

/// ⛔ **A CANDIDATE REFUSED AFTER ADMISSION ALSO ENDS THE GENERATION IN FLIGHT.**
/// A is requested, then the author saves B, which breaks the content graph. B is
/// refused, and A, which described a disk that no longer exists, is cancelled
/// rather than left to publish an edit the newest disk state does not carry.
#[test]
fn a_candidate_refused_by_the_content_graph_cancels_the_generation_in_flight() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    let a = request_the_banner(&mut app, "// A, which the refused save replaces", false);
    assert!(matches!(a, ambition_content::reload::ReloadRequest::Requested { .. }), "{a:?}");
    app.update();
    assert!(ambition_content::reload::pending_pack(app.world()).is_some(), "the premise: A is in flight");

    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let mut edited = false;
    let broken = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            if declared == "data/cutscenes/sandbox.ron" {
                let out = text.replacen(r#"id: "test_intro","#, r#"id: "test_intro_renamed","#, 1);
                edited = out != text;
                return out;
            }
            text
        })
        .expect("a renamed script compiles"),
    );
    assert!(edited, "the cutscene file no longer states the renamed script");
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(broken, Some(base)),
    );
    assert!(
        matches!(
            outcome,
            ambition_content::reload::ReloadRequest::Refused(
                ambition_content::reload::MoveReload::ContentGraphRefused(_)
            )
        ),
        "the premise: B is refused by the content graph: {outcome:?}"
    );
    assert!(
        ambition_content::reload::pending_pack(app.world()).is_none(),
        "⛔ A STAYED PENDING BESIDE THE REFUSAL OF THE SAVE THAT REPLACED IT"
    );
    for _ in 0..240 {
        app.update();
        assert_eq!(boot_banner(&app), "// boot sequence", "⛔ THE CANCELLED GENERATION'S CUTSCENES WERE PUBLISHED");
    }
    assert_eq!(activation_id(&app), Some(live_activation), "the cancelled generation activated");
}

/// ⛔ **A QUEST STEP THAT NAMES A BOSS THAT DOES NOT EXIST IS REFUSED**, with the
/// quest named, instead of becoming a step nobody can complete.
#[test]
fn a_candidate_whose_quest_names_a_boss_that_does_not_exist_is_refused() {
    let mut edited = false;
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            if declared == "data/quests.ron" {
                let out = text.replacen(
                    r#"BossDefeated("clockwork_warden")"#,
                    r#"BossDefeated("no_such_boss")"#,
                    1,
                );
                edited = out != text;
                return out;
            }
            text
        })
        .expect("a quest that names an unknown boss compiles"),
    );
    assert!(edited, "the quest book no longer states the edited boss");
    let errors = request_and_expect_a_graph_refusal(candidate);
    assert!(
        errors.iter().any(|error| error.contains("first_steps") && error.contains("no_such_boss")),
        "the refusal does not name the quest and the boss: {errors:?}"
    );
}

/// A pack without its quest file COMPILES (measured: `quest_specs_of` used to
/// panic for it), so a reload can propose it. The next session starts with an
/// empty book and does not panic in `populate_quest_registry`.
#[test]
fn a_candidate_without_a_quest_file_starts_the_next_session_with_no_quests() {
    let mut app = app_playing_gameplay();
    let live_activation = activation_id(&app).expect("a live session");
    assert!(quest_place(&app, "first_steps").is_some(), "the premise: N has the quest");
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_omitting(&["data/quests.ron"]).expect("a pack without quests compiles"),
    );
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(candidate, Some(base)),
    );
    assert!(matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }), "{outcome:?}");
    for _ in 0..240 {
        app.update();
    }
    assert_ne!(activation_id(&app), Some(live_activation), "the candidate never activated");
    let registry = app.world().resource::<ambition_content::quest::QuestRegistry>();
    assert!(registry.initialized, "the new session never populated its registry");
    assert!(registry.quests.is_empty(), "the new session kept quests the candidate does not declare");
}
