//! Quality-tiered shared-page sprite packs (ultrapacks) — the runtime side.
//!
//! The regen ultrapack step pools every published per-target sheet into
//! shared, uniformly-sized atlas pages per quality tier and installs them
//! under `assets/sprite_packs/<tier>/` (`ultrapack_<n>.png` pages + an
//! `ultrapack.json` [`SpritePackCatalog`]). `build.rs` bakes each tier's
//! catalog into [`BAKED_PACK_CATALOGS`] the same way sheet RONs are baked, so
//! Android/wasm carry the same data desktop does.
//!
//! Tier dir names match the [`TextureResolutionScale`] vocabulary — `full` /
//! `half` / `quarter` / `potato` — so tier selection is a direct mapping from
//! the active quality budget, and the catalog + its page images switch
//! atomically (they live in the same tier dir; the catalog names its pages).
//!
//! A consumer never reads pack pixels for gameplay: the synthesized
//! [`SheetRecord`](ambition_sprite_sheet::SheetRecord) view carries no
//! `body_metrics` (packs are visual storage truth only — see
//! `docs/planning/engine/data-driven-sprites-and-characters.md`).

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::SpritePackCatalog;

use crate::character::TextureResolutionScale;

mod baked {
    include!(concat!(env!("OUT_DIR"), "/baked_pack_catalogs.rs"));
}

/// The pack tier directory for a texture-resolution scale. Total mapping —
/// every scale has a tier — but the tier's catalog may be absent on a
/// checkout that never ran regen (see [`catalog_for_tier`]).
pub fn pack_tier_for_scale(scale: TextureResolutionScale) -> &'static str {
    match scale {
        TextureResolutionScale::Full => "full",
        TextureResolutionScale::Half => "half",
        TextureResolutionScale::Quarter => "quarter",
        TextureResolutionScale::Potato => "potato",
    }
}

/// Parse-once index of every baked pack catalog, keyed by tier name.
///
/// §5 classification: **immutable asset cache** — derived from the
/// compile-time [`BAKED_PACK_CATALOGS`] table, pure and override-free, so a
/// process-global `OnceLock` (same shape as the sheet `record_index`).
fn catalogs() -> &'static HashMap<&'static str, SpritePackCatalog> {
    static CATALOGS: OnceLock<HashMap<&'static str, SpritePackCatalog>> = OnceLock::new();
    CATALOGS.get_or_init(|| {
        let mut map = HashMap::new();
        for (tier, json) in baked::BAKED_PACK_CATALOGS {
            match SpritePackCatalog::parse(json) {
                Ok(catalog) => {
                    let errors = catalog.validate();
                    if errors.is_empty() {
                        map.insert(*tier, catalog);
                    } else {
                            tracing::warn!(
                            target: "ambition::sprite_packs",
                            "pack catalog tier '{tier}' failed validation ({} error(s), first: {}) — tier disabled",
                            errors.len(),
                            errors[0],
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        target: "ambition::sprite_packs",
                        "pack catalog tier '{tier}' failed to parse: {err} — tier disabled",
                    );
                }
            }
        }
        map
    })
}

/// The baked catalog for a tier dir name, if regen produced one.
pub fn catalog_for_tier(tier: &str) -> Option<&'static SpritePackCatalog> {
    catalogs().get(tier)
}

/// Pick the pack catalog for a texture scale, falling back to the `full`
/// tier when the requested tier was not generated. Returns the tier name
/// actually chosen alongside the catalog so the caller's page paths match
/// the catalog it resolves frames from.
pub fn catalog_for_scale(
    scale: TextureResolutionScale,
) -> Option<(&'static str, &'static SpritePackCatalog)> {
    let tier = pack_tier_for_scale(scale);
    if let Some(catalog) = catalog_for_tier(tier) {
        return Some((tier, catalog));
    }
    catalog_for_tier("full").map(|catalog| ("full", catalog))
}

/// Bevy asset path (relative to `assets/`) of one page image in a tier pack.
pub fn pack_page_path(tier: &str, page_image: &str) -> String {
    format!("sprite_packs/{tier}/{page_image}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The baked pack table is optional (regen output is gitignored), but when
    /// it IS present every tier must parse, validate, and cover the same
    /// target set — a tier that silently lost targets would fall back
    /// per-target at runtime and hide a broken regen.
    #[test]
    fn baked_pack_tiers_parse_and_agree_on_coverage() {
        if baked::BAKED_PACK_CATALOGS.is_empty() {
            eprintln!("no baked pack catalogs (regen not run) — skipping");
            return;
        }
        // Every baked tier survived parse+validate into the index.
        assert_eq!(
            catalogs().len(),
            baked::BAKED_PACK_CATALOGS.len(),
            "a baked pack catalog failed parse/validate"
        );
        let mut coverage: Vec<(&str, usize)> = catalogs()
            .iter()
            .map(|(tier, c)| (*tier, c.targets.len()))
            .collect();
        coverage.sort();
        let (_, first) = coverage[0];
        assert!(
            coverage.iter().all(|(_, n)| *n == first),
            "tier target coverage differs: {coverage:?}"
        );
    }

    /// The W2 pilot end-to-end (headless half): `intro_cart` resolves a spec
    /// from the shared pack at TWO different quality tiers, the Idle row maps,
    /// and a Bevy atlas layout builds for every page the spec addresses. The
    /// other half (pixels on screen) is the in-app run.
    #[test]
    fn intro_cart_pack_spec_resolves_at_two_tiers() {
        if baked::BAKED_PACK_CATALOGS.is_empty() {
            eprintln!("no baked pack catalogs (regen not run) — skipping");
            return;
        }
        use crate::character::sheets::{try_load_pack_spec_for_target, SheetTuning};
        let tuning = SheetTuning::new(1.0, 2);
        for (scale, want_tier) in [
            (TextureResolutionScale::Full, "full"),
            (TextureResolutionScale::Potato, "potato"),
        ] {
            let (spec, tier) = try_load_pack_spec_for_target("intro_cart", &tuning, scale)
                .expect("intro_cart must be packed at every tier");
            assert_eq!(tier, want_tier);
            assert!(spec.page_count() >= 1);
            for page in 0..spec.page_count() {
                let layout = spec.build_atlas_for_page(page);
                assert!(
                    layout.size.x > 0 && layout.size.y > 0,
                    "tier {tier} page {page} layout is degenerate"
                );
            }
            // The page images the spec names exist on disk where the asset
            // path says they are (desktop check; profile gating is separate).
            let asset_owner_dir =
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ambition_actors");
            let page0 = asset_owner_dir
                .join("assets")
                .join(pack_page_path(tier, &spec.page_images[0]));
            assert!(page0.is_file(), "missing {page0:?}");
        }
        // Tier sizes really differ (the whole point of quality tiers).
        let (full_spec, _) =
            try_load_pack_spec_for_target("intro_cart", &tuning, TextureResolutionScale::Full)
                .unwrap();
        let (potato_spec, _) =
            try_load_pack_spec_for_target("intro_cart", &tuning, TextureResolutionScale::Potato)
                .unwrap();
        assert!(potato_spec.frame_width < full_spec.frame_width);
    }

    #[test]
    fn every_scale_maps_to_a_tier_dir_name() {
        for scale in [
            TextureResolutionScale::Full,
            TextureResolutionScale::Half,
            TextureResolutionScale::Quarter,
            TextureResolutionScale::Potato,
        ] {
            assert!(!pack_tier_for_scale(scale).is_empty());
        }
    }
}
