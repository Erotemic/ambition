//! **Every minion a boss summons by NAME must resolve a body somebody authored.**
//!
//! ⛔⛔ **THE CENSUS THAT DELETED `small_lurker` COUNTED LDTK PLACEMENTS AND WAS
//! BLIND TO A RUST CONSTANT.** On 2026-08-11 a sweep reported it "PLACED IN ZERO
//! LEVELS" and removed its archetype row. The Gradient Sentinel summons it — and
//! `puppy_slug`, deleted the same week — from `gradient_sentinel.rs`, so from
//! that day the boss's minima traps and gradient cascades spawned the generic
//! `combatant` fallback: wrong health, wrong speed, wrong body, no crawl, no
//! cling. Nothing failed. A fallback is a real body, so the only tell was on
//! screen, and nobody was looking at that boss.
//!
//! ⭐ **the missing DIRECTION.** `every_archetype_row_is_placed_somewhere_or_
//! deliberately_code_selected` asks *does every row have a placement?* This asks
//! the other one: *does every name the code SUMMONS still resolve?* A row and a
//! constant can each be individually defensible while the pair is broken, and
//! only the second question sees it.

/// The minion ids `ambition_content`'s bosses summon at runtime, with where each
/// one is named. ⚠ a new summon constant belongs here the day it is written.
const SUMMONED_MINIONS: &[(&str, &str)] = &[
    (
        "npc_puppy_slug",
        "gradient_sentinel.rs MINIMA_TRAP_MINION_ARCHETYPE",
    ),
    (
        "small_lurker",
        "gradient_sentinel.rs GRADIENT_CASCADE_MINION_ARCHETYPE",
    ),
];

/// **Each summoned id names a registered character, or an archetype row that
/// still exists.**
///
/// ⚠ `small_lurker` is KNOWN BROKEN and is listed below with the reason, because
/// a guard that goes red on a defect nobody has decided how to fix gets muted.
/// What this test protects is the OTHER direction: it must not be possible to
/// break a second one silently, and the exemption names exactly one id.
#[test]
fn every_summoned_minion_id_resolves_a_body() {
    const KNOWN_UNRESOLVED: &[(&str, &str)] = &[(
        "small_lurker",
        "its archetype row was deleted 2026-08-11 by a placement census blind to \
         this constant, and WHAT a small lurker is as a character is a content \
         decision (ledger D93). The cascade spawns generic combatants until then, \
         and the summon road warns every time it does.",
    )];

    let buildable: std::collections::BTreeSet<&str> =
        ambition_content::character_catalog::buildable_cast().collect();
    let rows = ambition_content::enemy_roster::CHARACTER_ROSTER_RON;

    let mut unresolved = Vec::new();
    for (id, site) in SUMMONED_MINIONS {
        let is_character = buildable.contains(id);
        let is_row = rows.contains(&format!("\"{id}\": ("));
        if !is_character && !is_row {
            unresolved.push((*id, *site));
        }
    }

    let unexpected: Vec<_> = unresolved
        .iter()
        .filter(|(id, _)| !KNOWN_UNRESOLVED.iter().any(|(known, _)| known == id))
        .collect();
    assert!(
        unexpected.is_empty(),
        "these summoned minion ids resolve NOTHING — no registered character and \
         no archetype row — so the boss casting them spawns the generic \
         `combatant` fallback wearing the wrong body: {unexpected:?}. Either \
         register the character, or add it to KNOWN_UNRESOLVED with the decision \
         that is waiting."
    );

    // ⛔ and the exemption list cannot rot: an id that got FIXED must leave it,
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
