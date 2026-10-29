//! Select shared atlas pages from the target's library and quality tier.

use std::collections::HashMap;

use crate::SpritePackCatalog;

use crate::character::TextureResolutionScale;


/// The pack tier directory for a texture-resolution scale. Every scale has a
/// tier, but the tier's catalog may be absent on a checkout that never ran
/// generation.
pub fn pack_tier_for_scale(scale: TextureResolutionScale) -> &'static str {
    match scale {
        TextureResolutionScale::Full => "full",
        TextureResolutionScale::Half => "half",
        TextureResolutionScale::Quarter => "quarter",
        TextureResolutionScale::Potato => "potato",
    }
}

/// The scale a pack tier directory name denotes: the inverse of
/// [`pack_tier_for_scale`], and `None` for a name that is not a tier.
///
/// [`catalog_for_scale`] may return `full` instead of the requested tier. The
/// caller then has the physical tier only as a directory name, and needs it
/// as a scale to stamp the realization. Use this instead of repeating the
/// fallback rule (see [`crate::character::CharacterSpriteAsset::resolved_tier`]).
pub fn scale_for_pack_tier(tier: &str) -> Option<TextureResolutionScale> {
    match tier {
        "full" => Some(TextureResolutionScale::Full),
        "half" => Some(TextureResolutionScale::Half),
        "quarter" => Some(TextureResolutionScale::Quarter),
        "potato" => Some(TextureResolutionScale::Potato),
        _ => None,
    }
}

/// Each pack catalog of the current sheet generation, keyed by tier name.
fn catalogs(content: &crate::SpriteContent) -> &HashMap<&'static str, SpritePackCatalog> {
    crate::baked_sheet_rons::current(&content).pack_catalogs()
}

/// The catalogs of `texts` (`(tier, json_text)`), or each tier that does not
/// parse or validate with its first error, named by its file.
pub(crate) fn checked_catalogs<T: AsRef<str>>(texts: &[(&'static str, T)]) -> Result<HashMap<&'static str, SpritePackCatalog>, Vec<(String, String)>> {
    let mut map = HashMap::new();
    let mut failed = Vec::new();
    for (tier, json) in texts {
        let file = format!("sprite_packs/{tier}/ultrapack.json");
        match SpritePackCatalog::parse(json.as_ref()) {
            Ok(catalog) => match catalog.validate().first() {
                None => {
                    map.insert(*tier, catalog);
                }
                Some(error) => failed.push((file, error.to_string())),
            },
            Err(error) => failed.push((file, error.to_string())),
        }
    }
    if failed.is_empty() {
        Ok(map)
    } else {
        Err(failed)
    }
}

/// The catalogs of `texts` (`(tier, json_text)`) that parse and validate: a
/// tier that does not is disabled, with a warning.
pub(crate) fn catalogs_of<T: AsRef<str>>(texts: &[(&'static str, T)]) -> HashMap<&'static str, SpritePackCatalog> {
    {
        let mut map = HashMap::new();
        for (tier, json) in texts {
            match SpritePackCatalog::parse(json.as_ref()) {
                Ok(catalog) => {
                    let errors = catalog.validate();
                    if errors.is_empty() {
                        map.insert(*tier, catalog);
                    } else {
                            tracing::warn!(
                            target: "ambition_platformer2d::sprite_packs",
                            "pack catalog tier '{tier}' failed validation ({} error(s), first: {}) — tier disabled",
                            errors.len(),
                            errors[0],
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        target: "ambition_platformer2d::sprite_packs",
                        "pack catalog tier '{tier}' failed to parse: {err} — tier disabled",
                    );
                }
            }
        }
        map
    }
}

/// Pick the pack catalog for a texture scale. Fall back to the `full` tier
/// when the requested tier was not generated. Returns the chosen tier name
/// with the catalog, so the caller's page paths match the catalog.
pub fn catalog_for_scale<'a>(content: &'a crate::SpriteContent, target: &str,
    scale: TextureResolutionScale,
) -> Option<(&'static str, &'a SpritePackCatalog)> {
    let key = crate::shared_baked_sheet_registry(content).canonical_key(target)?;
    let library = key.strip_prefix("content://").and_then(|rest| rest.split_once('/')).map(|(library, _)| library);
    let find = |tier: &str| {
        let key = library.map_or_else(|| tier.to_owned(), |library| format!("content://{library}/{tier}"));
        catalogs(content).get(key.as_str())
    };
    let tier = pack_tier_for_scale(scale);
    find(tier).map(|catalog| (tier, catalog)).or_else(|| find("full").map(|catalog| ("full", catalog)))
}

/// Bevy asset path (relative to `assets/`) of one page image in a tier pack.
pub fn pack_page_path(tier: &str, page_image: &str) -> String {
    if page_image.contains("://") { page_image.to_owned() } else { format!("sprite_packs/{tier}/{page_image}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The baked pack table is optional (regen output is gitignored). When it
    /// is present, every tier must parse, validate, and cover the same targets.
    /// A tier that lost targets would fall back per target at runtime and hide
    /// a broken regen.
    ///
    /// `has_baked_packs` is a build-script cfg over the same table, so a skip
    /// shows as `ignored` with a reason.
    #[test]
    #[ignore = "requires the generated shared library"]
    fn baked_pack_tiers_parse_and_agree_on_coverage() {
        let content = &crate::test_support::shared_sprites();
        // Every baked tier survived parse+validate into the index.
        assert_eq!(
            catalogs(content).len(),
            crate::baked_sheet_rons::current(content).pack_catalog_texts().len(),
            "a baked pack catalog failed parse/validate"
        );
        let mut coverage: Vec<(&str, usize)> = catalogs(&content)
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

    /// The other half (pixels on screen) is the in-app run.
    ///
    /// Skips as `ignored`, not with `eprintln!` and `return`, so a skip is
    /// visible on a passing run.
    ///
    /// This test needs every tier (it asks for `full` and `potato` by name). A
    /// partial pack set fails it on purpose: a regen that produced only some
    /// tiers is broken (see `baked_pack_tiers_parse_and_agree_on_coverage`).
    #[test]
    #[ignore = "requires the generated shared library"]
    fn intro_cart_pack_spec_resolves_at_two_tiers() {
        let content = &crate::test_support::shared_sprites();
        use crate::character::sheets::{try_load_pack_spec_for_target, SheetTuning};
        let tuning = SheetTuning::new(1.0, 2);
        for (scale, want_tier) in [
            (TextureResolutionScale::Full, "full"),
            (TextureResolutionScale::Potato, "potato"),
        ] {
            let (spec, tier) = try_load_pack_spec_for_target(&content, "intro_cart", &tuning, scale)
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
            let libraries = crate::test_support::shared_libraries();
            for page in &spec.page_images {
                assert!(libraries.resolve(page).unwrap().native_path().unwrap().is_file());
            }
        }
        // Tier sizes differ.
        let (full_spec, _) =
            try_load_pack_spec_for_target(&content, "intro_cart", &tuning, TextureResolutionScale::Full)
                .unwrap();
        let (potato_spec, _) =
            try_load_pack_spec_for_target(&content, "intro_cart", &tuning, TextureResolutionScale::Potato)
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

    /// Both directions are one mapping. A tier that cannot map back to a scale
    /// is a realization that cannot record what it loaded.
    #[test]
    fn a_tier_dir_name_round_trips_back_to_its_scale() {
        for scale in [
            TextureResolutionScale::Full,
            TextureResolutionScale::Half,
            TextureResolutionScale::Quarter,
            TextureResolutionScale::Potato,
        ] {
            // The exhaustive match (no `_` arm) is the guard, not the array
            // above. A new `TextureResolutionScale` variant causes a compile
            // error (E0004) here, so it cannot skip the round trip.
            //
            // It does not repeat the tier names; `pack_tier_for_scale` is the
            // only authority for them.
            match scale {
                TextureResolutionScale::Full
                | TextureResolutionScale::Half
                | TextureResolutionScale::Quarter
                | TextureResolutionScale::Potato => {}
            }
            assert_eq!(
                scale_for_pack_tier(pack_tier_for_scale(scale)),
                Some(scale),
                "{scale:?} does not survive the round trip"
            );
        }
        // A name that is not a tier is not a scale.
        assert_eq!(scale_for_pack_tier("Full"), None);
        assert_eq!(scale_for_pack_tier("0_5x"), None);
        assert_eq!(scale_for_pack_tier(""), None);
    }
}
