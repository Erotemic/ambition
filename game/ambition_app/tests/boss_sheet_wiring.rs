//! **The boss-sprite wiring, pinned as far as it can be pinned headlessly.**
//!
//! tracks.md's bug queue carries *"all bosses render the generic sheet"* with the
//! diagnosis path *"do a RUN with `boss_sprites.len()` logging, and do NOT apply
//! the disproven `sprite_target` dispatch."*
//!
//! `upgrade_boss_sprites` draws the generic gradient-sentinel body for a boss
//! exactly when `GameAssets::boss_sprite(&boss_key)` misses, where `boss_key` is
//! the boss's lowercased BEHAVIOR ID. There are three ways to miss, and only the
//! last needs a window:
//!
//! 1. **the sheet never loads** — its catalog id resolves no path;
//! 2. **the key disagrees** — the registry is keyed by something other than the
//!    behavior id a live boss carries;
//! 3. **the image is still loading** — `images.get(...)` is `None` this frame, so
//!    the system skips and retries. Benign; the boss upgrades a frame later.
//!
//! This file rules out (1) and (2) for every boss the game actually spawns, in
//! the gate, so a future report of the bug means (3) or a render-side ordering
//! problem — and nobody re-litigates the key. The permanent
//! `[boss_sprites] N dedicated sheet(s) loaded: …` line in `load_game_assets` is
//! the counterpart for a live run.

#![cfg(feature = "rl_sim")]

use std::collections::BTreeSet;

/// Install the content seams the asset catalog and the boss roster read.
/// First install wins, so this is safe to call from every test.
fn install_content() {
    ambition_content::worlds::install();
    ambition_content::character_catalog::install();
    ambition_content::bosses::install_boss_roster();
}

/// (1) Every dedicated boss sheet the renderer will look for resolves to a real
/// path under the desktop-dev profile. A `None` here IS the bug, and it names the
/// boss.
#[test]
fn every_dedicated_boss_sheet_resolves_a_catalog_path() {
    install_content();
    let catalog = ambition::actors::assets::sandbox_assets::desktop_dev_default_catalog();

    let mut missing = Vec::new();
    for (key, _spec) in ambition::actors::boss_encounter::sprites::dedicated_boss_sheets() {
        let id = ambition::asset_manager::sandbox_assets::ids::boss_sprite(key);
        if catalog.try_path_for_load(&id).is_none() {
            missing.push(key);
        }
    }
    assert!(
        missing.is_empty(),
        "these boss sheets resolve no asset path, so `GameAssets::boss_sprites` \
         will not carry them and their bosses will draw the GENERIC body: {missing:?}. \
         Run `./regen_sprites.sh`, or fix the row in `all_boss_sprite_filenames`."
    );
}

/// (2) The registry's keys and the renderer's lookup key are the same vocabulary.
///
/// `upgrade_boss_sprites` computes `boss_key = behavior.id.to_ascii_lowercase()`.
/// So every authored boss profile that HAS art must appear in
/// `dedicated_boss_sheets()` under its own id — not under its `sprite_target`,
/// which is a different key the SIM uses for hurtbox metrics. That divergence is
/// the "disproven `sprite_target` dispatch" the bug note warns against; this test
/// is why it stays disproven.
#[test]
fn the_render_key_is_the_behavior_id_not_the_sprite_target() {
    install_content();

    let sheet_keys: BTreeSet<&str> =
        ambition::actors::boss_encounter::sprites::dedicated_boss_sheets()
            .into_iter()
            .map(|(key, _)| key)
            .collect();

    // Bosses that ship their own art. The rest deliberately draw the generic
    // gradient-sentinel body, which is a design choice, not this bug.
    for id in [
        "mockingbird",
        "gnu_ton_rider",
        "smirking_behemoth_boss",
        "flying_spaghetti_monster_boss",
        "trex_boss",
    ] {
        let profile = ambition::actors::features::BossBehaviorProfile::from_data(id);
        assert_eq!(
            profile.id, id,
            "the profile registry must round-trip its own id"
        );
        assert!(
            sheet_keys.contains(id),
            "`upgrade_boss_sprites` looks up `boss_sprites[\"{id}\"]` (the lowercased \
             BEHAVIOR ID). It is not in `dedicated_boss_sheets()` — so this boss draws \
             the generic body. Its `sprite_target` is {:?}, which is a DIFFERENT key, \
             used by the sim for hurtbox metrics. Add the behavior id to the sheet \
             registry; do not re-route the renderer through `sprite_target`.",
            profile.sprite_target,
        );
    }
}

/// The generic fallback is deliberate, and stays: a boss with no authored sheet
/// draws the gradient-sentinel body rather than nothing. Pinning it keeps someone
/// from "fixing" the bug by registering every boss id against the generic sheet,
/// which would make the real regression invisible.
#[test]
fn a_boss_with_no_authored_sheet_is_absent_from_the_registry_on_purpose() {
    install_content();
    let sheet_keys: BTreeSet<&str> =
        ambition::actors::boss_encounter::sprites::dedicated_boss_sheets()
            .into_iter()
            .map(|(key, _)| key)
            .collect();
    for id in ["clockwork_warden", "mode_collapse_boss", "overflow_boss"] {
        assert!(
            !sheet_keys.contains(id),
            "{id} has no dedicated art; it must fall back to the generic body, and \
             `upgrade_boss_sprites` warns once so it is never silent"
        );
    }
}
