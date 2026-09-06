//! Every minion a boss summons by NAME must resolve a body somebody authored.
//!
//! The Gradient Sentinel summons it — and `puppy_slug`, deleted the same week — from
//! `gradient_sentinel.rs`, so from that day the boss's minima traps and gradient cascades spawned
//! the generic `combatant` fallback: wrong health, wrong speed, wrong body, no crawl, no cling.
//! Nothing failed. A fallback is a real body, so the only tell was on screen, and nobody was
//! looking at that boss.
//!
//! the missing DIRECTION. `every_archetype_row_is_placed_somewhere_or_
//! deliberately_code_selected` asks *does every row have a placement?* This asks
//! the other one: *does every name the code SUMMONS still resolve?* A row and a
//! constant can each be individually defensible while the pair is broken, and
//! only the second question sees it.

/// It named the two Gradient Sentinel constants and the puppy-slug gun's, which is exactly the
/// set the scanner below finds on its own — and a transcription that duplicates a scan is a
/// second place to forget.
///
/// Every encounter file whose wave `kind`s are read STRAIGHT FROM THE SHIPPED
/// BYTES rather than transcribed into the list above.
///
/// a transcribed list is a snapshot, and a snapshot cannot see the wave
/// somebody adds tomorrow. The two boss constants are `const &str` in Rust and
/// have to be listed by hand; a `.ron` does not, so it is parsed. Where the guard
/// can read the source of truth, it reads it.
const ENCOUNTER_FILES: &[(&str, &str)] = &[(
    include_str!("../assets/data/encounters/goblin_encounter.ron"),
    "encounters/goblin_encounter.ron",
)];

/// The `kind: "..."` values an encounter file authors.
/// What each mob in a wave file will actually be built as.
///
/// this read `kind:` only, and that is not the road the runtime takes.
/// `spawn_encounter_mob` builds the body from the mob's prepared `character`;
/// `kind` is controller policy and never a body fallback. Reading only `kind`
/// therefore measures the wrong authority and can report a healthy encounter as
/// unresolved even when every mob names a buildable character.
///
/// a guard that measures a road nobody drives fails for the wrong reason, and
/// the temptation then is to add the id to `KNOWN_UNRESOLVED` with a story about
/// a decision that was never waiting.
///
/// `character:` must be read on the SAME mob, not anywhere in the file:
/// scanning for both fields globally would let one mob's character cover
/// another's missing row. Each entry is one line in these files, so the line is
/// the unit.
fn wave_kinds(ron: &str) -> Vec<&str> {
    ron.lines()
        .filter_map(|line| {
            let kind = field(line, "kind: \"")?;
            // The gameplay identity wins where the line states one, exactly as
            // the spawn road resolves it.
            Some(field(line, "character: Some(\"").unwrap_or(kind))
        })
        .collect()
}

/// The quoted value of `prefix` on this line, if it has one.
fn field<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let at = line.find(prefix)?;
    let rest = &line[at + prefix.len()..];
    rest.find('"').map(|end| &rest[..end])
}

/// Every `*_ARCHETYPE` constant in the workspace, found by SCANNING rather than
/// by transcription.
///
/// the list above is a snapshot and the class has already outrun it three
/// times. `puppy_slug` and `small_lurker` in the Gradient Sentinel, then
/// `puppy_slug` again in a PLAYER WEAPON one crate away — each found by a human
/// asking "who else names an archetype by string?", never by a guard. A list
/// somebody has to remember to extend is exactly as good as the memory.
///
///  this walks the source tree for `const …ARCHETYPE…: &str = "…"` and checks
/// each value the same way. A new constant is covered the moment it is written,
/// which is the only version of this guard that survives the next person who
/// adds one.
fn archetype_constants(root: &std::path::Path) -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for line in text.lines() {
                    let trimmed = line.trim_start();
                    if !trimmed.starts_with("const ") && !trimmed.starts_with("pub const ") {
                        continue;
                    }
                    // Match only names ending in `_ARCHETYPE`; broader substring
                    // matches capture unrelated constants such as file paths.
                    let Some(name_end) = trimmed.find(':') else {
                        continue;
                    };
                    if !trimmed[..name_end].trim_end().ends_with("_ARCHETYPE")
                        || !trimmed.contains("&str")
                    {
                        continue;
                    }
                    let Some(open) = trimmed.find('"') else {
                        continue;
                    };
                    let rest = &trimmed[open + 1..];
                    let Some(end) = rest.find('"') else { continue };
                    out.push((rest[..end].to_string(), path.display().to_string()));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

/// Each summoned id names a registered character that can build a body.
#[test]
fn every_summoned_minion_id_resolves_a_body() {
    const KNOWN_UNRESOLVED: &[(&str, &str)] = &[];

    let buildable: std::collections::BTreeSet<&str> =
        ambition_content::character_catalog::buildable_cast().collect();

    // the SCANNED half: every `*_ARCHETYPE` constant in the engine and the
    // games, wherever somebody writes the next one.
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("the content crate sits two levels under the repo root")
        .to_path_buf();
    let scanned: Vec<(String, String)> = ["crates", "game"]
        .into_iter()
        .flat_map(|dir| archetype_constants(&repo.join(dir)))
        .collect();
    assert!(
        !scanned.is_empty(),
        "the `*_ARCHETYPE` constant scan found NOTHING under {}, so this half of \
         the guard is reading an empty tree — check the path or the pattern",
        repo.display()
    );

    let mut named: Vec<(&str, &str)> = scanned
        .iter()
        .map(|(id, site)| (id.as_str(), site.as_str()))
        .collect();
    let before_waves = named.len();
    for (ron, site) in ENCOUNTER_FILES {
        for kind in wave_kinds(ron) {
            named.push((kind, site));
        }
    }
    assert!(
        named.len() > before_waves,
        "the encounter parse found no wave kinds at all, so that half of this \
         guard is reading nothing — the file's shape changed under it"
    );

    let mut unresolved = Vec::new();
    for (id, site) in &named {
        // one road (AC6). This also accepted an archetype ROW under the
        // id, because a row would build the body too; the rows are deleted and a
        // summon that names no character is refused at construction.
        if !buildable.contains(id) {
            unresolved.push((*id, *site));
        }
    }

    let unexpected: Vec<_> = unresolved
        .iter()
        .filter(|(id, _)| !KNOWN_UNRESOLVED.iter().any(|(known, _)| known == id))
        .collect();
    assert!(
        unexpected.is_empty(),
        "these summoned minion ids name no registered character, so the boss \
         casting them REFUSES mid-fight (it used to spawn a generic `combatant` \
         wearing the wrong body, which is worse): {unexpected:?}. Either register \
         the character, or add it to KNOWN_UNRESOLVED with the decision that is \
         waiting."
    );

    // and the exemption list cannot rot: an id that got FIXED must leave it,
    // or the next reader believes a resolved name is still broken.
    let stale: Vec<_> = KNOWN_UNRESOLVED
        .iter()
        .filter(|(id, _)| !unresolved.iter().any(|(broken, _)| broken == id))
        .collect();
    assert!(
        stale.is_empty(),
        "these ids are exempted as unresolved but they RESOLVE now — delete them \
         from KNOWN_UNRESOLVED: {stale:?}"
    );
}
