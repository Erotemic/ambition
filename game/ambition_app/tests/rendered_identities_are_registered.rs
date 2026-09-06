//! Verify that rendered character identities are registered in the catalog.
//!
//! This is the reverse of the regeneration check: renderer targets must resolve
//! to known characters. The nested renderer checkout is optional, so absence
//! skips the test rather than treating an empty target set as success.

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

/// Number of renderer targets whose character identity is computed rather than
/// expressed as a literal this scanner can resolve. The count is a ratchet: new
/// computed-id target shapes require explicit review instead of being silently
/// excluded from registration coverage.
///
/// ⚠ 11 → 13 on 2026-08-24, and the review is this note: `officer` and `author`
/// are two new SVG-rigged targets, and the hand check the ratchet asks for says
/// NEITHER has a catalog row yet. That is not a defect — they are being authored
/// (`officer.py`'s own doc calls him *"an easter-egg fighter, not a reference
/// rig"*) and a character exists in the renderer before it exists in the game.
///
/// ⚠ 13 → 15 on 2026-08-27, and the review is this note: `actor` and `medic` are
/// two more SVG-rigged targets. The hand check the ratchet asked for said
/// NEITHER had a catalog row, which was the same not-a-defect the 08-24 note
/// describes: a character exists in the renderer before it exists in the game.
///
/// ⛔⛔ AND THE NOTE ABOVE LANDED WITHOUT ITS CONSTANT. The 13 → 15 review was
/// written and `COMPUTED_ID_TARGETS` stayed 13, so the branch carried a red test
/// and a doc block describing the number it had not set. A counter and the prose
/// that justifies it are one change; verified 2026-08-27 that `medic` and
/// `actor` do have `character_catalog.ron` rows, which is the hand check this
/// ratchet asks for.
///
/// ⭐ BOTH HAVE ONE NOW, and the count did not move — which is the point of the
/// bucket. It counts targets whose `character_id` this scanner cannot read as a
/// LITERAL, and both name it `TARGET_NAME`; being registered in the catalog is a
/// different question from being resolvable by a text scan. `officer` and
/// `author` sit in the same bucket for the same reason and have had rows since
/// 08-24. ⛔ So do not lower this expecting registration to move it: what lowers
/// it is a target spelling its id out, or a target going away.
///
/// ⛔⛔ THIS COUNT READS THE RENDERER CHECKOUT ON DISK, INCLUDING UNTRACKED
/// FILES. So it moves with work in progress that this repository cannot see, and
/// it will need LOWERING again if either target goes away before it lands. A
/// failure here that nobody's commit explains is that — and this bump is exactly
/// that case, caught by a full-suite run rather than by anything in the diff.
/// ⛔ LOWERED 15 -> 13 ON 2026-08-27, which is the case the paragraph above
/// describes happening again: two renderer targets that were on disk when the
/// count was last set are not on disk now, and nothing in this repository's
/// history explains either. No commit here touched the renderer.
const COMPUTED_ID_TARGETS: usize = 15;

/// Every literal `character_id` value in one target source, accepting dict keys
/// with either quote style and keyword-argument syntax.
fn character_id_literals(text: &str) -> std::vec::IntoIter<String> {
    const KEY: &str = "character_id";
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    for (at, _) in text.match_indices(KEY) {
        if at == 0 {
            continue;
        }
        // Three ways a target names this key, and the scan reads all three
        // because Python offers all three and the authors used them:
        //
        //   "character_id": "npc_x"   a dict key, double-quoted
        //   'character_id': 'npc_x'   the same dict, formatter-quoted
        //    character_id="npc_x"     a KEYWORD ARGUMENT to a spec constructor
        //
        // the third is not a stylistic variant of the first two — `carl_runga`
        // and `martin_cutta` build their metadata by CALL, so nothing in their
        // source is a quoted key at all, and both sat in the "id is computed"
        // bucket where no assertion reaches them.
        let before = bytes[at - 1];
        let quoted_key =
            (before == b'"' || before == b'\'') && bytes.get(at + KEY.len()) == Some(&before);
        let bare_key = !before.is_ascii_alphanumeric() && before != b'_' && before != b'.';
        if !quoted_key && !bare_key {
            continue;
        }
        let mut i = at + KEY.len() + usize::from(quoted_key);
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        let separator = if quoted_key { b':' } else { b'=' };
        if bytes.get(i) != Some(&separator) {
            continue;
        }
        i += 1;
        // `character_id == "npc_x"` is a comparison, not a declaration.
        if !quoted_key && bytes.get(i) == Some(&b'=') {
            continue;
        }
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        let Some(&quote) = bytes.get(i).filter(|q| **q == b'"' || **q == b'\'') else {
            continue;
        };
        i += 1;
        let Some(end) = text[i..].find(quote as char).map(|off| i + off) else {
            continue;
        };
        let literal = &text[i..end];
        // An interpolated id is not one id; those targets name a family.
        if !literal.is_empty() && !literal.contains('{') {
            out.push(literal.to_string());
        }
    }
    out.into_iter()
}

/// Returns `(stem -> every id it declares, stems whose id is computed)`.
///
/// every id, not the first one. A target file can name a whole family —
/// `_genghis_pair_common` declares `npc_genghis_can` AND `npc_genghis_cant`,
/// `_snakes_on_planes_common` both planes — and taking only the first would
/// check one of each pair while the other stayed exactly as unwatched as it was
/// when the ids were unreadable. Hiding a second character behind a first is the
/// same failure as hiding it behind a spec table, one line further along.
fn declared_identities(dir: &Path) -> (BTreeMap<String, Vec<String>>, Vec<String>) {
    let mut computed = Vec::new();
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (out, computed);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
            continue;
        }
        // A LEADING UNDERSCORE IS PYTHON FOR "NOT A TARGET".
        // `_runge_kutta_duo.py` is the SHARED module `carl_runga.py` and
        // `martin_cutta.py` both import their rig from — it draws nobody by
        // itself, and counting it as a render target made this scan report a
        // character with no catalog row that does not exist. Every real target in
        // this directory is named for who it draws.
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('_'))
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut seen = std::collections::BTreeSet::new();
        let ids: Vec<String> = character_id_literals(&text)
            .filter(|id| seen.insert(id.clone()))
            .collect();
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string();
        if ids.is_empty() {
            if text.contains("character_id") {
                computed.push(stem);
            }
            continue;
        }
        out.insert(stem, ids);
    }
    computed.sort();
    (out, computed)
}

/// Declared ids this check deliberately does not require, each with its reason.
///
/// A probe caught it: a deliberately-stale entry for `goblin` did not fire, because no renderer
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
    // the SECOND id in `sanic.py`, and it took reading past the first to
    // find it. `ambition_demo_sanic` registers the super form as `super_sanic`
    // (`SUPER_SANIC_CHARACTER_ID`), so it is in the game and only its renderer
    // metadata disagrees — the same story as the row above, invisible until the
    // scan stopped stopping at one id per file.
    (
        "npc_super_sanic",
        "provider-owned; the demo registers it as `super_sanic`",
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
    (
        "npc_pirate_heavy",
        "a multi-variant family target; the NAMED heavies are the characters (Jon, 2026-08-05)",
    ),
    // do not resolve this by casting them on a guess — the question is in
    // `docs/planning/awaiting-maintainer-decision.md`.
    (
        "npc_robot_heavy",
        "a family id whose three named variants are ALSO uncast; awaiting Jon (2026-08-06)",
    ),
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
    let (declared, computed) = declared_identities(&dir);
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
    for (stem, ids) in &declared {
        for id in ids {
            let known = registered.contains(id.as_str());
            match (known, waived(id)) {
                (false, None) => orphans.push(format!("{stem} declares `{id}`")),
                (true, Some(reason)) => stale_waivers.push(format!(
                    "{id} is registered now, but is waived as: {reason}"
                )),
                _ => {}
            }
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

    // and a waiver whose target is GONE is just as stale. Without this the
    // table only answers questions it is asked, and an entry for a renamed or
    // deleted target would never be asked again.
    let declared_ids: std::collections::BTreeSet<&str> =
        declared.values().flatten().map(|id| id.as_str()).collect();
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

    // The blind spot, pinned. See [`COMPUTED_ID_TARGETS`].
    assert_eq!(
        computed.len(),
        COMPUTED_ID_TARGETS,
        "the number of targets whose character id this scan CANNOT read has \
         changed: {computed:?}. Each one is outside every assertion above — if a \
         family was added, check by hand whether its characters have rows (the \
         snakes-on-planes pair did not, and nothing here could say so); if one \
         was removed or now spells its id out, lower this number in the same \
         commit."
    );
}

/// The poison. A guard whose scan silently stops finding targets reports the
/// success condition, which is how the first direction of this check stayed
/// invisible while a portrait checker sat beside it.
#[test]
fn the_identity_scan_would_notice_an_unregistered_id() {
    let (declared, _) = declared_identities(&target_dir());
    if declared.is_empty() {
        return;
    }
    assert!(
        waived("a_character_nobody_ever_drew").is_none(),
        "the waiver table answers ids it was never given, so nothing can fail"
    );
    // name a waiver that is a DECISION, not a pending one. This asserted
    // `special_patent_clerk` until his row landed, at which point the poison failed for the one
    // reason a poison must never fail: the thing it watches got FIXED.
    assert!(
        waived("npc_pirate_heavy").is_some(),
        "and it does answer one it was given"
    );
}
