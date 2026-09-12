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

    // ── the premise: the live ladder is the shipped one ──────────────────────
    let before = live_first_rung(&app);
    assert_eq!(
        before, 500.0,
        "the premise: this composition installed the shipped ladder"
    );

    // ── an edit, in memory ───────────────────────────────────────────────────
    let mut edited = false;
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            if declared != "data/fighter_brain_ladder.ron" {
                return text;
            }
            let out = text.replacen("reaction_ms: 500.0", "reaction_ms: 499.0", 1);
            edited = out != text;
            out
        })
        .expect("the edited pack compiles"),
    );
    // ⛔ THE FLOOR ON THE EDIT. A retuned level 1 or a renamed source would leave
    // the closure a no-op and every assertion below would pass on the SHIPPED
    // pack while testing nothing.
    assert!(
        edited,
        "`data/fighter_brain_ladder.ron` no longer carries `reaction_ms: 500.0`, \
         so the candidate is the shipped pack and this witness is vacuous"
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
}
