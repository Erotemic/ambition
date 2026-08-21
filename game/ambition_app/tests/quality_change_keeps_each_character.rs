#![cfg(all(feature = "input", feature = "visible"))]

//! **A QUALITY CHANGE MAY CHANGE THE TIER. IT MAY NOT CHANGE THE CHARACTER.**
//!
//! Jon, 2026-08-12: *"When I change the video quality in ambition, my sprite went
//! from the robot v3 character to the robot v2 character."*
//!
//! ⛔⛔ **nothing in this tree changed quality at runtime and looked at what
//! happened** — measured 2026-08-21. Nine candidate causes have been eliminated
//! by replicating the runtime over files (see
//! `sprite-residency-and-live-quality.md`); every layer that could CHOOSE the
//! wrong sheet is deterministic. What is left is WHEN, and a file cannot answer
//! that.
//!
//! ⚠ **two things made this unwritable until now**, both worth knowing:
//! `GameAssets` is ABSENT from a shell-host composition (character realizations
//! are presentation state), so this boots `build_visible_app(NoWindow)`; and
//! there was no accessor enumerating RESIDENT sheets —
//! `declared_character_ids()` is that set's COMPLEMENT, so reaching for it
//! yields a tautology.
//!
//! ⚠ **the assertion is on the FILE ROOT, not the path.** A tier change is
//! SUPPOSED to move `sprites/x.png` to `sprites_0_5x/x.png`. Asserting the path
//! would fail on correct behaviour; asserting the root fails only when the
//! character actually changed.

use bevy::asset::AssetServer;
use bevy::prelude::*;

fn boot() -> App {
    let mut app = ambition_app::app::build_visible_app(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app
}

/// Every resident token and the full sheet PATH it resolves to.
fn resident(app: &App) -> std::collections::BTreeMap<String, String> {
    let world = app.world();
    let Some(assets) =
        world.get_resource::<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>()
    else {
        return Default::default();
    };
    let server = world.resource::<AssetServer>();
    let mut out = std::collections::BTreeMap::new();
    for (token, asset) in assets.characters.resident_sheets() {
        if let Some(path) = server.get_path(&asset.texture) {
            let p = path.path().to_string_lossy().to_string();
            out.insert(token.to_string(), p);
        }
    }
    out
}

#[test]
fn changing_the_quality_profile_never_changes_which_character_a_token_resolves_to() {
    let mut app = boot();
    for _ in 0..240 {
        app.update();
    }

    let before = resident(&app);
    // ⚠ LOUD: a transition over an empty table proves nothing, and an empty
    // table is exactly what a silently-failed boot produces.
    assert!(
        !before.is_empty(),
        "no character sheets are resident after boot, so this test would pass \
         over an empty transition"
    );

    {
        let mut settings = app
            .world_mut()
            .resource_mut::<ambition_platformer2d::persistence::settings::UserSettings>();
        settings.video.quality.profile =
            ambition_platformer2d::persistence::settings::VisualQualityProfile::Potato;
    }
    for _ in 0..240 {
        app.update();
    }

    let after = resident(&app);

    // ⛔⛔ **PROVE THE TRANSITION HAPPENED FIRST.** The identity assertion below
    // compares FILE ROOTS, which a tier move does not change — so without this,
    // a quality change that did nothing at all would pass it. That is the
    // "check that cannot fail" this repo names outright, and the first draft of
    // this test had it.
    let moved: Vec<&String> = before
        .iter()
        .filter(|(token, path)| after.get(*token).is_some_and(|now| now != *path))
        .map(|(token, _)| token)
        .collect();
    assert!(
        !moved.is_empty(),
        "no resident token changed its sheet PATH across the quality change, so \
         the transition did not happen and the identity check below would pass \
         over nothing. {} tokens resident.",
        before.len(),
    );

    let file_root = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
    let swapped: Vec<String> = before
        .iter()
        .filter_map(|(token, path)| {
            after
                .get(token)
                .filter(|now| file_root(now) != file_root(path))
                .map(|now| format!("  {token}: {path} -> {now}"))
        })
        .collect();
    // ⛔ FALSIFIED, not assumed: swapping this comparator to full PATHS reports
    // 18 tokens moving `sprites/x.png -> sprites_potato/x.png` — the correct
    // transition — which proves both that the machinery detects a change and
    // that the message names the right thing.
    assert!(
        swapped.is_empty(),
        "a quality change re-pointed {} resident token(s) at a DIFFERENT \
         character's sheet file:\n{}\n\nA tier change may move `sprites/x.png` \
         to `sprites_0_5x/x.png`; it may never change `x`.",
        swapped.len(),
        swapped.join("\n"),
    );
}
