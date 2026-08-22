#![cfg(all(feature = "input", feature = "visible"))]

//! Changing visual quality may change a resident sheet tier, never its character identity.
//! The direct-gameplay fixture requires the primary player token to be resident and to
//! change path, then verifies its character file root is unchanged.

use bevy::asset::AssetServer;
use bevy::prelude::*;

fn boot() -> App {
    // `shell_hosted: false` — DIRECT gameplay, not the launcher.
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, false);
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

    // Require the primary player token so the reported character path is actually exercised.
    let worn = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d::characters::actor::WornCharacter,
            With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        q.iter(world).next().map(|w| w.0.as_str().to_owned())
    }
    .expect("a direct gameplay boot has a PrimaryPlayer wearing a character");

    let before = resident(&app);
    assert!(
        before.contains_key(&worn) || before.keys().any(|t| t.eq_ignore_ascii_case(&worn)),
        "the player wears `{worn}`, which is not among the {} resident token(s), \
         so this test would not cover the case Jon reported. Resident: {:?}",
        before.len(),
        before.keys().take(10).collect::<Vec<_>>(),
    );
    // An empty residency table would make the transition check vacuous.
    assert!(
        !before.is_empty(),
        "no character sheets are resident after boot, so this test would pass \
         over an empty transition"
    );

    {
        let mut settings =
            app.world_mut()
                .resource_mut::<ambition_platformer2d::persistence::settings::UserSettings>();
        settings.video.quality.profile =
            ambition_platformer2d::persistence::settings::VisualQualityProfile::Potato;
    }
    for _ in 0..240 {
        app.update();
    }

    let after = resident(&app);

    // Prove the quality transition changed resolved assets before checking
    // identity, because stable file roots alone cannot show the transition ran.
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
    // The primary player token itself must have moved tiers before checking identity.
    assert!(
        moved.iter().any(|t| t.eq_ignore_ascii_case(&worn)),
        "the player wears `{worn}`, whose sheet did NOT move across the quality \
         change — so the identity assertion below never examines the reported \
         character. {} other token(s) did move: {:?}",
        moved.len(),
        moved.iter().take(8).collect::<Vec<_>>(),
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
    // FALSIFIED, not assumed: swapping this comparator to full PATHS reports
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
