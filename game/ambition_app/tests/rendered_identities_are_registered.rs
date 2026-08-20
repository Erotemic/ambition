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

/// Targets that carry a `character_id` key whose value this scan cannot read.
///
/// ⛔ **the check's population is "targets that SPELL an id out", not
/// "targets"** — and that distinction hid two characters. The snakes-on-planes
/// family was read as declaring only `spec.character_id`, so no literal seemed
/// to exist; both sheets were drawn, published and named by nothing for however
/// long, and this guard could not have said so.
///
/// ⚠ **a ratchet rather than a fix.** Teaching the scan to evaluate spec
/// constructors means teaching it every new spec SHAPE, which is a maintenance
/// tail nobody asked for. Pinning the count instead means the next family built
/// this way fails here and somebody LOOKS — at which point they can decide
/// whether that family needs rows, which is the actual question.
///
/// ⭐ **9 → 7 on 2026-08-06, and the two that left were never computed at all.**
/// `_genghis_pair_common` and `_snakes_on_planes_common` DO spell their ids out
/// — as keyword arguments to the spec constructor, four literals between them —
/// and the scan could not see a bare key. So the paragraph above was true of the
/// scanner and not of the targets: the snakes were readable the whole time, and
/// the ratchet was pinning the reader's blind spot rather than the authors'
/// cleverness. `carl_runga` and `martin_cutta` left for the same reason.
///
/// ⭐ **7 → 8 on 2026-08-18, and the hand-check the ratchet exists to force was
/// done before the number moved.** The eighth is `mary_o_v2_svg_poc`, whose own
/// module docstring says what it is: *"this target intentionally coexists with
/// `mary_o_v2`… so the editable SVG + rigid bone workflow can be judged without
/// changing game-facing Mary-O output"*. Its character IS `mary_o_v2`, which has
/// a row — so it needs no row of its own, and it is in this count rather than in
/// `WAIVED` only because the scan cannot read an id it never spells.
///
/// ⭐ **8 → 9 on 2026-08-20, hand-check first, as the ratchet demands.** The
/// ninth is `fighting_polygon_sword`, an SVG-rigged humanoid that is both a
/// playable fighter and the reference rig other humanoids start from. **It HAS a
/// catalog row** (`character_catalog.ron`, tier `MainHall`), and
/// `registered_character_art_resolves` now passes for it — which is the actual
/// question this ratchet exists to force somebody to ask. It is in this count
/// rather than in `WAIVED` for the same reason as the eighth: the scan cannot
/// read an id the target never spells as a literal.
///
/// ⛔ the number and the submodule pointer move TOGETHER. Raising it while the
/// POC is untracked turns CI red (a fresh checkout has seven); landing the POC
/// without raising it turns CI red the other way.
/// ⭐ **9 → 10 on 2026-08-20, hand-check first, as the ratchet demands.** The
/// tenth is `fighting_polygon_brawler`, the unarmed half of the reference pair.
/// **It HAS a catalog row** (`character_catalog.ron`, tier `MainHall`,
/// `default_brain: "melee_brute_striker"`), added in the same session as its
/// renderer target, and the superproject gitlink was bumped to `4dd065a` so the
/// target is reachable from a fresh clone rather than only on the machine that
/// authored it. It is in this count rather than in `WAIVED` for the same reason
/// as the eighth and ninth: the scan cannot read an id the target never spells
/// as a literal.
const COMPUTED_ID_TARGETS: usize = 10;

/// Every `character_id` VALUE spelled out in one target's source, in file order.
///
/// ⛔ **the COLON is load-bearing.** Splitting on the key alone and taking the
/// next quoted string reported nine bogus ids on the first run: `display_name`
/// (from targets that list the key names before the values) and
/// `pc_{form.target_name}` (from an f-string that builds ids per form). Both are
/// the same mistake — reading the next string in the file rather than this key's
/// VALUE.
///
/// ⛔ **and so is the QUOTE, which is how this check went blind for a week.**
/// The first version matched `"character_id"` literally, and Python does not
/// care which quote you write: `robot_heavy`, `bear_mauler`, `carl_stargan` and
/// `patent_clerk` all spell theirs with apostrophes because they were emitted by
/// a formatter rather than typed. So four targets declared ids this scan could
/// not see — and one of them, `npc_robot_heavy`, is a genuine unregistered
/// identity that the guard existed to find and reported nothing about.
///
/// ⚠ **the failure was worse than silence: it read as a FIX.** The stale
/// `special_patent_clerk` waiver went red for the "no target declares this"
/// reason, and the queue recorded the cause as Jon's single quotes — correctly —
/// but as a fact about ONE row rather than about the scanner. Deleting that
/// waiver turned the check green while three ids stayed invisible.
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
        // ⚠ the third is not a stylistic variant of the first two — `carl_runga`
        // and `martin_cutta` build their metadata by CALL, so nothing in their
        // source is a quoted key at all, and both sat in the "id is computed"
        // bucket where no assertion reaches them.
        //
        // ⛔ `spec.character_id` must NOT match: that one really is computed
        // (the snakes-on-planes pair), and reading the next string after it is
        // the exact bug the colon rule above was written for.
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
/// ⭐ **every id, not the first one.** A target file can name a whole family —
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
        // ⭐ **A LEADING UNDERSCORE IS PYTHON FOR "NOT A TARGET".**
        // `_runge_kutta_duo.py` is the SHARED module `carl_runga.py` and
        // `martin_cutta.py` both import their rig from — it draws nobody by
        // itself, and counting it as a render target made this scan report a
        // character with no catalog row that does not exist. Every real target in
        // this directory is named for who it draws.
        //
        // ⚠ the count below is the instrument, so a false positive here reads as
        // a content gap and gets "fixed" by registering a character nobody drew.
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
    // ⭐ **the SECOND id in `sanic.py`, and it took reading past the first to
    // find it.** `ambition_demo_sanic` registers the super form as `super_sanic`
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
    // ⭐ **`special_patent_clerk` used to sit here and is GONE, as designed.**
    // The waiver was written on 2026-08-05 saying "delete this the moment the
    // row lands"; the row landed on 2026-08-06 under that exact key, so the
    // check now enforces it rather than excusing it. This paragraph is the
    // receipt — a waiver that disappears without one reads as a check that got
    // quietly weakened.
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
    // Jon, 2026-08-05, asked directly: *"the named pirate heavies are the pirate
    // heavies."* broadside_bess, iron_mary and salt_annet ARE the characters;
    // the rest of what this target renders is variant art, not more people. The
    // bare id was dropped from the catalog on 2026-05-24 for the same reason and
    // there is no flat `pirate_heavy_spritesheet.png` to hang a row on.
    (
        "npc_pirate_heavy",
        "a multi-variant family target; the NAMED heavies are the characters (Jon, 2026-08-05)",
    ),
    // ⚠ **`npc_robot_heavy` is NOT the settled case above, and this waiver is a
    // placeholder for a question, not an answer.** The scanner started seeing it
    // on 2026-08-06 (single-quoted, see `character_id_literals`), and it is the
    // one real orphan that blindness was hiding. It LOOKS like `pirate_heavy` —
    // one target, a bare family id, three named variants (Bastion Bruiser,
    // Foundry Ram, Volt Crusher) — but the resemblance stops where it matters:
    // Jon's ruling cast the named pirate heavies, and NONE of the three named
    // robot heavies is in the catalog either. So the family reading would leave
    // all four unregistered, which is not what "the named ones are the
    // characters" means anywhere else.
    //
    // ⛔ do not resolve this by casting them on a guess — the question is in
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

    // ⛔ **and a waiver whose target is GONE is just as stale.** Without this the
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

/// **The poison.** A guard whose scan silently stops finding targets reports the
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
    // ⚠ **name a waiver that is a DECISION, not a pending one.** This asserted
    // `special_patent_clerk` until his row landed on 2026-08-06, at which point
    // the poison failed for the one reason a poison must never fail: the thing
    // it watches got FIXED. `npc_pirate_heavy` is a ruling of Jon's rather than
    // a queue item, so it is a stable thing to be answered about.
    assert!(
        waived("npc_pirate_heavy").is_some(),
        "and it does answer one it was given"
    );
}
