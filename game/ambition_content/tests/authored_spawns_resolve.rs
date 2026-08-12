//! **EVERY AUTHORED `EnemySpawn` CAN BE BUILT AS SOMETHING SOMEBODY NAMED.**
//!
//! ⛔⛔ **this guard did not exist, and its absence cost a live regression on
//! 2026-08-12.** `medium_striker`'s archetype row was deleted after a census
//! that measured Rust callers — `cargo check --all-targets` was clean, and the
//! whole app suite stayed green. It could not see LDtk. One placement in
//! `under_town_pipes` names that brain key with no `character_id`, so it
//! silently stopped being a 5-HP rock-thrower and became the 4-HP melee-only
//! `combatant` fallback. Nothing said so, because `spec_for_brain` cannot fail.
//!
//! ⭐ **the measurement this replaces the census with**: 65 authored
//! `EnemySpawn` placements in `assets/worlds`, all but TWO of which carry a
//! `character_id` and go character-first. Those two are the entire surface where
//! a deleted archetype row can still change what a body is.
//!
//! ⚠ the count is unique by `iid`. A world file stores its levels more than
//! once, so a naive walk reports 186 and any ratio drawn from it is wrong.
//!
//! ⚠ **it walks the LDtk with a JSON reader and never writes one back.** A
//! `.ldtk` round-tripped through `json.dumps` loses its formatting and the
//! editor's repair does not restore it.

use std::collections::BTreeSet;

/// A placement that names no character: the brain key is the only thing
/// deciding what its body is.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CharacterlessSpawn {
    world: String,
    level: String,
    name: String,
    brain: String,
}

fn worlds() -> Vec<(String, serde_json::Value)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/worlds");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&root) else {
        panic!("no authored worlds under {}", root.display());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ldtk") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("world file reads");
        let value: serde_json::Value = serde_json::from_str(&text).expect("world file parses");
        out.push((
            path.file_stem().unwrap().to_string_lossy().to_string(),
            value,
        ));
    }
    assert!(
        !out.is_empty(),
        "the world scan found no `.ldtk` files — this guard is reading an empty \
         tree, which is the silent-pass failure of every source scanner"
    );
    out
}

/// Every `EnemySpawn` in a world, as `(level, name, brain, has_character)`.
fn enemy_spawns(world: &serde_json::Value) -> Vec<(String, String, String, bool)> {
    fn walk(
        node: &serde_json::Value,
        level: &str,
        seen: &mut BTreeSet<String>,
        out: &mut Vec<(String, String, String, bool)>,
    ) {
        match node {
            serde_json::Value::Object(map) => {
                // A level's own identifier, carried down into its layers.
                let level = map
                    .get("identifier")
                    .and_then(|v| v.as_str())
                    .filter(|_| map.contains_key("layerInstances"))
                    .unwrap_or(level);
                if map.get("__identifier").and_then(|v| v.as_str()) == Some("EnemySpawn") {
                    // ⚠ **deduplicated by `iid`.** A world file stores its
                    // levels more than once (the project and the level files
                    // agree), so a naive walk double-counts every placement and
                    // a census built on it is wrong by a factor.
                    if let Some(iid) = map.get("iid").and_then(|v| v.as_str()) {
                        if seen.insert(iid.to_string()) {
                            let mut name = String::new();
                            let mut brain = String::new();
                            let mut has_character = false;
                            for field in map
                                .get("fieldInstances")
                                .and_then(|v| v.as_array())
                                .into_iter()
                                .flatten()
                            {
                                let id = field.get("__identifier").and_then(|v| v.as_str());
                                let value = field.get("__value");
                                match id {
                                    Some("name") => {
                                        name = value
                                            .and_then(|v| v.as_str())
                                            .unwrap_or_default()
                                            .to_string()
                                    }
                                    Some("brain") => {
                                        brain = value
                                            .and_then(|v| v.as_str())
                                            .unwrap_or_default()
                                            .to_string()
                                    }
                                    Some("character_id") | Some("character") => {
                                        has_character = value
                                            .and_then(|v| v.as_str())
                                            .is_some_and(|id| !id.is_empty())
                                    }
                                    _ => {}
                                }
                            }
                            out.push((level.to_string(), name, brain, has_character));
                        }
                    }
                }
                for value in map.values() {
                    walk(value, level, seen, out);
                }
            }
            serde_json::Value::Array(items) => {
                for value in items {
                    walk(value, level, seen, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(world, "<unknown>", &mut BTreeSet::new(), &mut out);
    out
}

/// **A placement that names NEITHER a buildable character NOR an archetype row
/// is built as the reserved `combatant` fallback, silently.**
///
/// ⭐ the exemption list is the honest part: each entry is a placement whose
/// creature is an OPEN CONTENT DECISION, carrying the ledger row that decides
/// it. They are not bugs — they are bodies nobody has cast yet — and writing
/// them a creature to empty the list would be inventing content.
#[test]
fn every_authored_spawn_names_a_character_or_a_row_that_exists() {
    /// `(name, why it is not cast yet)`.
    const KNOWN_UNCAST: &[(&str, &str)] = &[
        (
            "under_town_skitter",
            "WHAT a small skitter IS is ledger D96 item 3, open and Jon's. Its \
             placement named the `medium_striker` archetype, whose row moved \
             into the engine's test fixture on 2026-08-12 — so it builds as the \
             `combatant` fallback today: 4 HP and melee-only where it used to be \
             5 HP with a thrown rock. A REGRESSION, recorded rather than \
             papered over, and the fix is the casting decision rather than \
             restoring a row no character needs.",
        ),
        (
            // ⚠ its authored `name` is literally `Target` — matched on the name
            // the FILE holds, not on the one a reader would guess. A guard keyed
            // to a guessed spelling exempts nothing and reads as though it does.
            "Target",
            "authored `brain: Passive`, which names no archetype at all and \
             never did — it has resolved the fallback since long before this \
             campaign. Whether the dive-drill Target is a sandbag is ledger D96 \
             item 4.",
        ),
    ];

    let rows = ambition_content::enemy_roster::CHARACTER_ROSTER_RON;
    let buildable: BTreeSet<&str> = ambition_content::character_catalog::buildable_cast().collect();

    let mut characterless = Vec::new();
    let mut total = 0usize;
    for (world, value) in worlds() {
        for (level, name, brain, has_character) in enemy_spawns(&value) {
            total += 1;
            if has_character {
                continue;
            }
            characterless.push(CharacterlessSpawn {
                world: world.clone(),
                level,
                name,
                brain,
            });
        }
    }
    characterless.sort();
    characterless.dedup_by(|a, b| a.level == b.level && a.name == b.name && a.brain == b.brain);

    // ⚠ **65 as of 2026-08-12**, counted here rather than remembered: unique
    // by `iid` across `assets/worlds`. A floor rather than an equality, because
    // authoring a level is ordinary and a guard that fails on new content is a
    // guard people delete. It fails if the scan stops reading, which is the
    // thing worth catching.
    assert!(
        total >= 50,
        "the placement scan found only {total} `EnemySpawn`s, far fewer than the \
         authored worlds hold (65 when this was written) — the shape changed \
         under this guard and it is now measuring almost nothing"
    );

    let unresolved: Vec<&CharacterlessSpawn> = characterless
        .iter()
        .filter(|spawn| {
            // The brain key is the only authority left, so it must name a row.
            !rows.contains(&format!("\"{}\": (", spawn.brain))
        })
        .collect();

    let unexpected: Vec<&&CharacterlessSpawn> = unresolved
        .iter()
        .filter(|spawn| !KNOWN_UNCAST.iter().any(|(known, _)| *known == spawn.name))
        .collect();

    assert!(
        unexpected.is_empty(),
        "these authored placements name NEITHER a character nor an archetype row, \
         so each is built as the reserved `combatant` fallback wearing whatever \
         its name resolves for art: {unexpected:#?}\n\nEither give the placement \
         a `character_id` whose character can build a body, or add it to \
         KNOWN_UNCAST with the decision that is waiting. ⛔ deleting an archetype \
         row is what puts a placement here, and `cargo check` cannot see it.",
    );

    // ⭐ THE POISON, and the reason the first assertion is not enough: if the
    // scan stopped finding characterless placements at all — a renamed field, a
    // moved directory — the filter would be empty and this would pass while
    // measuring nothing.
    assert!(
        !characterless.is_empty(),
        "the scan found no characterless placements at all. There are known ones \
         ({}), so either they were cast (delete them from KNOWN_UNCAST and this \
         message) or the scan stopped reading the field.",
        KNOWN_UNCAST
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join(", ")
    );

    assert!(
        buildable.contains("goblin"),
        "sanity: the buildable cast is populated, or a future extension of this \
         guard to the character half would be vacuous from the day it is written"
    );
}

/// **A LEGACY ROW MAY ONLY OUTLIVE ITS REASON BY ZERO COMMITS.**
///
/// ⭐ `medium_striker`'s archetype row exists for exactly one thing: the
/// `under_town_skitter` placement names it and carries no `character_id`, so
/// the row is the only thing deciding what that body is. Deleting the row
/// without casting the skitter nerfed it from 5 HP with a thrown rock to the
/// 4-HP melee-only `combatant` fallback, silently, and that is what happened on
/// 2026-08-12.
///
/// ⛔ **the converse is the half a countdown cannot express.** The monolith's
/// `the_shipped_archetype_file_holds_only_rows_that_state_why` asserts the row
/// is still there; nothing asserted that its REASON still is. So the day
/// somebody casts the skitter — answering D96 item 3 — the row becomes dead
/// weight that every future census has to re-litigate, and a legacy row nobody
/// can justify is exactly how 843 lines of this file survived to August.
///
/// ⚠ this test lives HERE because it is the only place both facts are readable:
/// the levels and the roster are both this crate's content.
#[test]
fn the_striker_row_lives_exactly_as_long_as_the_placement_that_needs_it() {
    let rows = ambition_content::enemy_roster::CHARACTER_ROSTER_RON;
    let row_exists = rows.contains("\"medium_striker\": (");

    let mut skitter_needs_it = false;
    for (_, value) in worlds() {
        for (_, name, brain, has_character) in enemy_spawns(&value) {
            if name == "under_town_skitter" && brain == "medium_striker" && !has_character {
                skitter_needs_it = true;
            }
        }
    }

    assert_eq!(
        row_exists, skitter_needs_it,
        "the `medium_striker` row and the placement that needs it disagree \
         (row: {row_exists}, placement still uncast: {skitter_needs_it}).\n\n\
         If the skitter was CAST — it names a character now — delete the row, \
         its note in `character_archetypes.ron`, its SURVIVORS entry in the \
         monolith's countdown test, and this test.\n\n\
         If the row was DELETED while the placement still names it, that body \
         just became the `combatant` fallback: 4 HP and melee-only where it was \
         5 HP with a thrown rock. `cargo check` cannot see that and neither can \
         the app suite."
    );
}
