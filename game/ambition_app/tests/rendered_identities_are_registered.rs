//! **The orphan check's MIRROR: art that declares a character nobody registers.**
//!
//! `scripts/tests/test_every_character_regenerates.py` asks one direction — a
//! catalog character whose sheet no regen batch publishes, which is a character
//! with no body on a fresh clone. That check has now caught 34 characters across
//! three separate discoveries.
//!
//! Nothing asked the other way. A sprite-renderer target declares a
//! `character_id` and a `display_name` in its own metadata; if no catalog
//! registers that id, the sheet renders (minutes of compute, every regen,
//! forever) and the game can never show it. **The failure is invisible for
//! exactly the reason the first direction was**: whoever renders it sees a
//! picture and assumes the rest is wired.
//!
//! ⛔ **the population is the ASSEMBLED CATALOG, not a regex over the RON.** A
//! first pass built the registered set by matching `"<id>": ( … spritesheet:`
//! across tracked sources and reported eighteen orphans, four of which —
//! `imperfect_cellular_automaton`, `sandbag`, `npc_pirate_raider`,
//! `npc_burning_flying_shark` — are registered characters this very binary can
//! list. A guard that cries wolf about four real characters is one nobody reads,
//! and the fix is to stop modelling the catalog and ask it (see
//! `hall_scale_spread`, which asks the same object for the same reason).
//!
//! ⚠ **the renderer is a nested checkout.** When it is absent the test SKIPS
//! rather than passing, because "found no targets" and "every target is
//! registered" must not look alike.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::character::CharacterCatalog;

/// Where the sprite renderer's per-character targets live.
fn target_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters",
    )
}

/// `<target stem> -> <declared character_id>` for every target that names one.
fn declared_identities(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        // `"character_id": "npc_whoever",` — the renderer's own metadata block.
        //
        // ⛔ **the COLON is load-bearing.** Splitting on the key alone and taking
        // the next quoted string reported nine bogus ids on the first run:
        // `display_name` (from targets that list the key names before the
        // values) and `pc_{form.target_name}` (from an f-string that builds ids
        // per form). Both are the same mistake — reading the next string in the
        // file rather than this key's VALUE.
        let Some(id) = text.split("\"character_id\"").skip(1).find_map(|rest| {
            let value = rest.trim_start().strip_prefix(':')?.trim_start();
            let literal = value.strip_prefix('"')?.split('"').next()?;
            // An interpolated id is not one id; those targets name a family.
            (!literal.is_empty() && !literal.contains('{')).then(|| literal.to_string())
        }) else {
            continue;
        };
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string();
        out.insert(stem, id.to_string());
    }
    out
}

/// Declared ids this check deliberately does not require, each with its reason.
///
/// ⚠ **a TABLE rather than a match, so the waivers can be ENUMERATED.** The
/// first draft was a `match`, and a match can only answer about ids somebody
/// asks it about — so a waiver whose target was renamed or deleted would sit
/// here forever, unaskable and therefore unfalsifiable. A probe caught it: a
/// deliberately-stale entry for `goblin` did not fire, because no renderer
/// target declares that id at all.
///
/// Every entry is checked THREE ways below: the id is still declared by a
/// target, it is still unregistered, and it still needs the excuse.
const WAIVED: &[(&str, &str)] = &[
    // Provider-owned bodies. The demo registers its own character under its OWN
    // id (`sanic`, `solid_snake`, `player_robot_v3`), so what is stale here is
    // the renderer's metadata rather than the wiring — the art is in the game.
    (
        "npc_sanic",
        "provider-owned; the demo registers this body under a different id",
    ),
    (
        "npc_solid_snake",
        "provider-owned; registered under a different id",
    ),
    (
        "player",
        "provider-owned; player_robot_v3's target names the seat, not a catalog id",
    ),
    // Drafted, not cast. A row exists under `character_drafts/` and nothing
    // assembles those yet.
    (
        "npc_george_booul",
        "a character DRAFT; drafts are not assembled into the catalog",
    ),
    // ⭐ **live work, 2026-08-05.** Jon added `patent_clerk.py` and its
    // `regen_sprites.sh` target during a run, having asked earlier the same day
    // whether a patent clerk existed. It has no row yet — this waiver is the
    // note saying which half is missing, and it should be DELETED the moment
    // the row lands.
    (
        "special_patent_clerk",
        "art in flight; the catalog row is not written yet (Jon, 2026-08-05)",
    ),
    // ⭐ **`pirate_heavy` is a FAMILY, not a character**, and this is a decision
    // Jon already made: `regen_sprites.sh` records that the catalog dropped its
    // bare `npc_pirate_heavy` entry on 2026-05-24 rather than shoehorning a
    // placeholder, because broadside_bess, iron_mary and salt_annet are the real
    // characters and all three are cast. There is no flat
    // `pirate_heavy_spritesheet.png` to give a row.
    //
    // ⚠ the other four that sat here — busy beaver, charley beagle, niels boar,
    // vera ruin — were CAST on 2026-08-05 when Jon answered "cast them", so
    // their entries are gone rather than reworded.
    ("npc_pirate_heavy", "a multi-variant family target; its variants are the characters, and they are cast"),
];

fn waived(id: &str) -> Option<&'static str> {
    WAIVED
        .iter()
        .find(|(waived, _)| *waived == id)
        .map(|(_, reason)| *reason)
}

#[test]
fn every_rendered_identity_is_a_character_the_game_can_show() {
    let dir = target_dir();
    let declared = declared_identities(&dir);
    if declared.is_empty() {
        eprintln!(
            "[skip] no sprite-renderer targets under {} — the nested checkout is \
             absent, and reporting 'all clear' over nothing is the failure this \
             check exists to avoid",
            dir.display()
        );
        return;
    }
    assert!(
        declared.len() >= 40,
        "only {} renderer targets declare a character_id — the scan is broken",
        declared.len()
    );

    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    let registered: std::collections::BTreeSet<&str> =
        catalog.iter().map(|(id, _)| id.as_str()).collect();

    let mut orphans: Vec<String> = Vec::new();
    let mut stale_waivers: Vec<String> = Vec::new();
    for (stem, id) in &declared {
        let known = registered.contains(id.as_str());
        match (known, waived(id)) {
            (false, None) => orphans.push(format!("{stem} declares `{id}`")),
            (true, Some(reason)) => stale_waivers.push(format!(
                "{id} is registered now, but is waived as: {reason}"
            )),
            _ => {}
        }
    }

    assert!(
        orphans.is_empty(),
        "{} sprite-renderer target(s) declare a character id no catalog \
         registers, so the sheet renders on every regen and the game can never \
         show it:\n  {}\n\nEither add the catalog row, or stop declaring the id \
         in the target's metadata.",
        orphans.len(),
        orphans.join("\n  ")
    );
    assert!(
        stale_waivers.is_empty(),
        "a waiver above has gone stale — the character is registered now, so the \
         reason it names is no longer true:\n  {}",
        stale_waivers.join("\n  ")
    );

    // ⛔ **and a waiver whose target is GONE is just as stale.** Without this the
    // table only answers questions it is asked, and an entry for a renamed or
    // deleted target would never be asked again.
    let declared_ids: std::collections::BTreeSet<&str> =
        declared.values().map(|id| id.as_str()).collect();
    let unasked: Vec<&str> = WAIVED
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| !declared_ids.contains(id))
        .collect();
    assert!(
        unasked.is_empty(),
        "no renderer target declares these waived id(s) any more, so their \
         excuses can never be tested: {unasked:?}"
    );
}

/// **The poison.** A guard whose scan silently stops finding targets reports the
/// success condition, which is how the first direction of this check stayed
/// invisible while a portrait checker sat beside it.
#[test]
fn the_identity_scan_would_notice_an_unregistered_id() {
    let declared = declared_identities(&target_dir());
    if declared.is_empty() {
        return;
    }
    assert!(
        waived("a_character_nobody_ever_drew").is_none(),
        "the waiver table answers ids it was never given, so nothing can fail"
    );
    assert!(
        waived("special_patent_clerk").is_some(),
        "and it does answer the one that is genuinely pending"
    );
}
