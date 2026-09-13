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
    // ⇒ A tolerance is a claim that the gap does not matter. Here the gap IS the
    // defect: `activate_prepared_platformer_sessions` reads
    // `PreparedCharacterRegistry` inside `GameplaySessionSet::Providers` on the
    // activation frame, so a family that publishes afterwards publishes into a
    // world already built without it. This asserts the frame, not the outcome.
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
    // ⚠ **IT COST NOTHING VISIBLE, WHICH IS THE ONLY REASON IT SURVIVED.** A
    // refusal today suppresses the `RoomLoaded` message and `RoomLoaded` has no
    // production reader. Under A10's candidate bracket the same refusal drops
    // every root, so a hot reload would land the player in an EMPTY WORLD — and
    // that is what this arm is here to keep from coming back. It asks the
    // production verdict, not the schedule; the schedule is asked separately by
    // `nothing_orders_the_retired_scopes_sweep_against_the_incoming_sessions_construction`.
    let verification = app
        .world()
        .resource::<ambition_platformer2d::actors::features::LastConstructionVerification>()
        .clone();
    assert!(
        verification.published,
        "the room the reload rebuilt (`{}`) was REFUSED with {} violation(s):          {:?}. Today that only suppresses `RoomLoaded`; under the candidate          bracket it drops the whole room and the reload lands in an empty world.",
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
}
