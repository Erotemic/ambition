//! ONE LOGICAL ASSET, ONE LOGICAL PATH, EVERY PLATFORM.
//!
//! An `AssetId` names a thing the game needs. A profile decides how the bytes
//! get there — a loose dev tree, an APK, an embedded blob, an HTTP fetch from
//! the page origin. Those are *delivery* answers, and the day a platform starts
//! answering the *naming* question differently is the day the browser and the
//! desktop are playing two different games out of two different trees.
//!
//! The composed publication seam that merges BOTH roots — `scripts/package_asset_guard.py`,
//! which Android and the Steam Deck deploy already used — arrived and the web script was never
//! migrated to it. So everything the content crate owned was absent from the browser's
//! `/assets/` — including every `.ldtk` world — and nothing measured the gap because nothing
//! compared the two platforms' views.
//!
//! the property is about the LOGICAL path, not the string. A desktop dev
//! checkout may hand back an absolute `LocalPath` so the file watcher can see
//! it, and a browser may hand back `embedded://…` for the same entry; those are
//! two mechanisms carrying one name, which is the intended design. What must
//! never differ is *which file is meant*.

use bevy::prelude::*;

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::asset_manager::{AssetId, AssetLocation, AssetProfile};
use ambition_platformer2d::view::Platformer2dAssetCatalog;

/// Drop a `<source>://` qualifier. A manifest entry may name its logical path
/// through an asset SOURCE — the four `.ldtk` worlds are authored as
/// `game://worlds/<file>` — and the source is a delivery answer like any other:
/// the desktop reaches that file through the layered `game://` reader, the
/// browser through the copy embedded in the wasm. Which file is meant is the
/// part after the scheme.
fn strip_scheme(path: &str) -> &str {
    path.split_once("://").map_or(path, |(_, rest)| rest)
}

/// The path a resolution names, normalized to forward slashes and stripped of
/// its source qualifier.
///
/// `None` means the profile resolved no location at all — a real answer, and a
/// different question from "resolved somewhere else".
fn logical_target(location: &AssetLocation) -> Option<String> {
    let raw = match location {
        AssetLocation::BevyPath(path) => path.clone(),
        AssetLocation::BevySourcePath { path, .. } => path.clone(),
        AssetLocation::Embedded(path) => path.clone(),
        AssetLocation::LocalPath(path) => path.to_string_lossy().into_owned(),
        other => format!("{other:?}"),
    };
    let normalized = raw.replace('\\', "/");
    Some(strip_scheme(&normalized).to_string())
}

/// Do two resolutions name the same file? A logical path is a relative path, so
/// one side may carry a longer prefix (an absolute dev root, a package
/// namespace) than the other; agreement means one tail is a suffix of the other
/// at a path boundary, never a bare substring.
fn names_the_same_file(left: &str, right: &str) -> bool {
    let ends_at_boundary = |long: &str, short: &str| {
        long == short || (long.ends_with(short) && long[..long.len() - short.len()].ends_with('/'))
    };
    ends_at_boundary(left, right) || ends_at_boundary(right, left)
}

fn production_catalog() -> Platformer2dAssetCatalog {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    for _ in 0..ambition_app::app::shared_host_startup_ticks() {
        app.update();
    }
    app.world()
        .get_resource::<Platformer2dAssetCatalog>()
        .expect(
            "the shipped host publishes `Platformer2dAssetCatalog`; without it nothing \
             owns asset-source policy and this audit has no production manifest to read",
        )
        .clone()
}

fn representative_ids(catalog: &Platformer2dAssetCatalog) -> Vec<AssetId> {
    let namespaces = [
        "data",       // platformer_defaults.ron
        "world",      // an authored .ldtk — the content crate's root
        "sprite",     // a controlled body / boss sheet — the engine's root
        "background", // parallax art
        "font",       // UI text
        "audio",      // the SFX bank
        "music",      // a generated track
    ];
    let manifest = catalog.catalog().manifest();
    namespaces
        .iter()
        .filter_map(|namespace| {
            let prefix = format!("{namespace}.");
            manifest
                .iter()
                .map(|(id, _)| id)
                .filter(|id| id.as_str().starts_with(&prefix))
                .min_by_key(|id| id.as_str().to_string())
                .cloned()
        })
        .collect()
}

/// Every divergence between the two platforms, as a readable report.
fn divergences(catalog: &Platformer2dAssetCatalog, ids: &[AssetId]) -> Vec<String> {
    let manifest = catalog.catalog().manifest();
    let mut report = Vec::new();
    for id in ids {
        let Some(entry) = manifest.get(id) else {
            report.push(format!("  {id}: not in the manifest at all"));
            continue;
        };
        let resolve = |profile| {
            catalog
                .catalog()
                .resolve(id, profile)
                .ok()
                .and_then(|resolved| logical_target(&resolved.location))
        };
        let desktop = resolve(AssetProfile::DesktopDevLoose);
        let web = resolve(AssetProfile::WebServedAssets);
        // the entry's own `logical_path` is the arbiter, not the desktop
        // answer. Comparing the two platforms to each other would pass happily
        // if BOTH drifted off the manifest together.
        let logical = strip_scheme(&entry.logical_path.replace('\\', "/")).to_string();
        for (platform, resolved) in [("desktop", &desktop), ("web", &web)] {
            if let Some(path) = resolved {
                if !names_the_same_file(path, &logical) {
                    report.push(format!(
                        "  {id}: {platform} resolved {path:?}, which is not the manifest's \
                         logical path {logical:?}"
                    ));
                }
            }
        }
        if desktop.is_none() && web.is_none() {
            report.push(format!(
                "  {id}: NEITHER platform can resolve it; the manifest lists an asset no \
                 build can load"
            ));
        }
    }
    report
}

#[test]
fn desktop_and_served_web_name_the_same_runtime_path_for_the_same_asset_id() {
    let catalog = production_catalog();
    let sampled = representative_ids(&catalog);
    assert!(
        sampled.len() >= 7,
        "expected one representative asset per namespace in the production manifest, \
         found {}: {sampled:?}",
        sampled.len()
    );
    let report = divergences(&catalog, &sampled);
    assert!(
        report.is_empty(),
        "the browser and the desktop do not name the same file for the same logical \
         asset. Only the DELIVERY may differ — a divergent name means a platform branch \
         has duplicated manifest knowledge instead of consuming it:\n{}",
        report.join("\n")
    );
}

/// The same question asked of everything, not just the representative set:
/// ~967 entries across seven namespaces.
#[test]
fn every_manifest_entry_names_the_same_file_on_both_platforms() {
    let catalog = production_catalog();
    let all: Vec<AssetId> = catalog
        .catalog()
        .manifest()
        .iter()
        .map(|(id, _)| id.clone())
        .collect();
    assert!(
        all.len() > 100,
        "the production manifest holds only {} entries; this audit is measuring an empty \
         catalog rather than the shipped one",
        all.len()
    );
    let report = divergences(&catalog, &all);
    assert!(
        report.is_empty(),
        "{} of {} manifest entries name a different file in the browser than on the \
         desktop:\n{}",
        report.len(),
        all.len(),
        report
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// THE POISON. Every assertion above passes when two resolutions agree,
/// so a comparison that cannot tell two different files apart would pass
/// vacuously — which is exactly the failure mode of a suffix match written
/// without a boundary check (`".../boss.png"` "matching" `".../miniboss.png"`).
#[test]
fn the_comparison_can_tell_two_different_files_apart() {
    assert!(names_the_same_file(
        "/abs/dev/root/sprites/boss.png",
        "sprites/boss.png"
    ));
    assert!(names_the_same_file("sprites/boss.png", "sprites/boss.png"));
    assert!(
        !names_the_same_file("sprites/miniboss.png", "sprites/boss.png"),
        "a bare suffix match would call these the same file; the boundary check is what \
         stops the whole audit from being decorative"
    );
    assert!(!names_the_same_file(
        "sprites/boss.png",
        "sprites/other.png"
    ));
    // A source qualifier is delivery, not identity.
    assert_eq!(
        strip_scheme("game://worlds/sandbox.ldtk"),
        "worlds/sandbox.ldtk"
    );
    assert_eq!(strip_scheme("worlds/sandbox.ldtk"), "worlds/sandbox.ldtk");
}
