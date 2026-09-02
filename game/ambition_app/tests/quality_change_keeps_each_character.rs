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

/// The ROUND TRIP (asset open work 6): Full → Potato → Full converges back to
/// the sheets the session started with, the worn character's pages are LOADED
/// at the end of each leg (real decode, not a table entry), and no character
/// page is left resident without a realization owning it — a tier swap that
/// leaked the old tier's pages would show here as pages nobody owns.
#[test]
fn a_quality_round_trip_converges_back_with_every_page_loaded_and_nothing_orphaned() {
    use ambition_platformer2d::persistence::settings::{UserSettings, VisualQualityProfile};
    use ambition_platformer2d::sprite_sheet::game_assets::GameAssets;

    let mut app = boot();
    for _ in 0..240 {
        app.update();
    }
    let worn = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d::characters::actor::WornCharacter,
            With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        q.iter(world).next().map(|w| w.0.as_str().to_owned())
    }
    .expect("a direct gameplay boot has a PrimaryPlayer wearing a character");
    let original_profile = app
        .world()
        .resource::<UserSettings>()
        .video
        .quality
        .profile;
    assert_ne!(
        original_profile,
        VisualQualityProfile::Potato,
        "premise: the round trip needs two different tiers"
    );

    // Step until the table is quiet AND every page of the worn sheet is loaded,
    // so a leg is judged on decoded pixels rather than on a fresh table entry.
    let settle = |app: &mut App| {
        let mut quiet = 0;
        let mut last = resident(app);
        for _ in 0..1200 {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(2));
            let now = resident(app);
            let pages_loaded = {
                let assets = app.world().resource::<GameAssets>();
                let server = app.world().resource::<AssetServer>();
                assets.characters.sheet(&worn).is_some_and(|sheet| {
                    sheet
                        .pages
                        .iter()
                        .all(|page| server.is_loaded_with_dependencies(page.texture.id()))
                })
            };
            if now == last && pages_loaded {
                quiet += 1;
                if quiet >= 30 {
                    return now;
                }
            } else {
                quiet = 0;
                last = now;
            }
        }
        panic!("the residency table never settled with the worn sheet's pages loaded");
    };
    let orphans = |app: &App| -> Vec<String> {
        let assets = app.world().resource::<GameAssets>();
        let owned: std::collections::BTreeSet<String> = assets
            .characters
            .resident_sheets()
            .map(|(_, sheet)| sheet)
            .chain(assets.characters.props.values())
            .flat_map(|sheet| sheet.pages.iter())
            .filter_map(|page| page.texture.path().map(|path| path.to_string()))
            .collect();
        ambition_platformer2d::sprite_sheet::game_assets::image_stages::ledger()
            .resident_rows()
            .filter(|row| row.source == Some("character-sheet"))
            .filter_map(|row| row.path.clone())
            .filter(|path| !owned.contains(path))
            .collect()
    };

    let at_start = settle(&mut app);
    assert!(at_start.contains_key(&worn), "premise: `{worn}` is resident");

    app.world_mut()
        .resource_mut::<UserSettings>()
        .video
        .quality
        .profile = VisualQualityProfile::Potato;
    let at_potato = settle(&mut app);
    assert_ne!(
        at_potato.get(&worn),
        at_start.get(&worn),
        "premise: Potato moved the worn sheet's path"
    );
    let leaked = orphans(&app);
    assert!(
        leaked.is_empty(),
        "after the drop to Potato, {} character page(s) are resident with no realization: {:?}",
        leaked.len(),
        leaked.iter().take(5).collect::<Vec<_>>()
    );

    app.world_mut()
        .resource_mut::<UserSettings>()
        .video
        .quality
        .profile = original_profile;
    let back = settle(&mut app);
    assert_eq!(
        back.get(&worn),
        at_start.get(&worn),
        "the worn sheet did not come back to its original path after the round trip"
    );
    let drifted: Vec<String> = at_start
        .iter()
        .filter(|(token, path)| back.get(*token).is_some_and(|now| now != *path))
        .map(|(token, path)| format!("{token}: {path} -> {}", back[token]))
        .collect();
    assert!(
        drifted.is_empty(),
        "{} token(s) resolve to a different sheet after the round trip than before it: {:?}",
        drifted.len(),
        drifted.iter().take(8).collect::<Vec<_>>()
    );
    let leaked = orphans(&app);
    assert!(
        leaked.is_empty(),
        "after the return to {original_profile:?}, {} character page(s) are resident with no \
         realization: {:?}",
        leaked.len(),
        leaked.iter().take(5).collect::<Vec<_>>()
    );
}
