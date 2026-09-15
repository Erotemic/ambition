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
    // ⚠ **IT COST ALMOST NOTHING VISIBLE, WHICH IS THE ONLY REASON IT SURVIVED —
    // AND THE "NO PRODUCTION READER" HALF OF THAT WAS WRONG, CORRECTED
    // 2026-09-14.** `RoomLoaded` has three production readers, all through
    // `ambition_combat::events::FreshAttempt`, so a refusal also left staged hits
    // unvoided and per-attempt state un-re-armed. Both are the right outcome (no
    // attempt began), which is why nobody noticed.
    //
    // ⛔ **AND SINCE `ROOM_CANDIDATE_BRACKET` WENT `true` THE SAME REFUSAL DROPS
    // EVERY ROOT**, so this reload would land the player in an EMPTY WORLD. That
    // is no longer a future tense: it is what this arm keeps from coming back. It
    // asks the production verdict, not the schedule; the schedule is asked by
    // `nothing_orders_the_retired_scopes_sweep_against_the_incoming_sessions_construction`.
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
}

/// ⛔⛤ **DOES THE SHIPPED APP EVER HOLD TWO `SessionRoot`s? MEASURED, BECAUSE 217
/// SYSTEM PARAMETERS DEPEND ON THE ANSWER AND NOBODY HAD ASKED IT.**
///
/// `SessionWorldRef` / `SessionWorldMut` are `Single<.., With<SessionRoot>>` —
/// **217 references across 108 files at HEAD** — and `Single` matches only when
/// there is EXACTLY ONE. Meanwhile `live_session_world_root` deliberately selects
/// the root owned by `ActiveSessionScope`, *"so a lingering retired root is not a
/// candidate rather than an ambiguity"*. ⇒ Two ownership semantics for one fact,
/// and a 2026-09-13 review said candidate coexistence would make the ordinary one
/// ambiguous.
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
/// If the count never exceeds one, `Single` is unambiguous in production and the
/// 217 sites are not exposed today; if it does, this arm names the frame.
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
         at 217 sites, and `Single` matches NOTHING when the count is not one — so \
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
fn a_running_shipped_session() -> bevy::prelude::App {
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

fn press_apply_reload(app: &mut bevy::prelude::App) {
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
/// `ROOM_CANDIDATE_BRACKET` is `true`, so a refused room is DROPPED. The handoff
/// is the road that used to produce the worst refusal in the whole suite — a
/// session retiring its old scope and starting the new one in the SAME frame, so
/// the incoming room's transaction captured a baseline still holding all 18 of
/// the outgoing scope's placements (`room-refused :: 18x Duplicated`). Under the
/// bracket that drops the entire room, and the new session wakes up in an empty
/// world.
///
/// ⚠ **THE ORDERING FIX IS WHAT CLOSED IT** (`SessionScopeSet` chains
/// `RetireAuthority -> Cleanup -> Activate -> Presentation`), and
/// `nothing_orders_the_retired_scopes_sweep_against_the_incoming_sessions_construction`
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
