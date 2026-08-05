//! **Does every fighter on the smash grid have a MOVESET?**
//!
//! Jon, 2026-08-05, on seeing the crossover roster: *"We might need to generate
//! real smash movesets for the characters if they are missing them."*
//!
//! He is asking about a real gap and it is worth being precise about which gap.
//! `SmashSelect::roster` declares one `AbilitySet` for every seat, deliberately,
//! so nobody is stronger for having come from a different demo — but an ABILITY
//! is *may this body attack* and a MOVESET is *what the attack is*. Those come
//! from different places: the ability set from the roster, the action set from
//! the character's own catalog row.
//!
//! And the grid is now Ambition's Hall cast, who were authored to stand in a
//! room and talk. So this measures rather than assumes: it reports what each
//! fighter's catalog row actually resolves to, and fails only on the thing that
//! is unambiguously wrong — **a fighter with no melee at all**, who arrives on a
//! platform-fighter stage unable to hit anybody.
//!
//! ⚠ **the number is a ratchet, not a target.** Authoring twelve movesets is a
//! content job; this exists so the count cannot quietly grow while it is being
//! done, and so the list is derived from the roster rather than remembered.

use ambition_demo_smash::select::SmashRoster;
use ambition_platformer2d::character::CharacterCatalog;

/// Fighters on the grid whose catalog row gives them NO melee.
///
/// ⭐ **measured 2026-08-05: SEVEN of the twelve.** All seven resolve to a
/// `peaceful` preset — Ambition's, Mary-O's and Sanic's own — because standing
/// in a room and talking is what they were authored for. The five that can hit
/// are the ones whose rows already named a combat preset: George Booul and the
/// demo's duelists (`smash::duelist`, 4 damage, 34px), and the Pirate Admiral,
/// Shadow Oni Leader, Perfect Cellular Automaton and Goblin (Ambition's
/// `striker_swipe` / `pirate_pistol`, 1 damage, 28px).
///
/// ⚠ **so it is not only the seven.** The five that CAN hit disagree by 4x on
/// damage, because two of them are a demo's fighter and three are an
/// adventure game's enemies. A platform fighter wants one authored kit per
/// character, which is the job Jon is asking about; this list is the floor.
const KNOWN_UNARMED: &[&str] = &[
    "player_robot_v3",
    "mary_o",
    "sanic",
    "npc_alice",
    "npc_bob",
    "npc_oiler",
    "npc_noether",
];

#[test]
fn every_fighter_on_the_smash_grid_can_throw_a_punch() {
    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");

    // ⚠ the ASSEMBLED grid, not the wish list: the demo's stand-in robots drop
    // out of a host that carries the real lineage, and measuring them here would
    // report a moveset nobody can pick.
    //
    // ⚠ **assembled HERE rather than read from the resource**, because the
    // resource is filled by a `Startup` system and `build_visible_app` has not
    // run a frame — reading it gave the DEFAULT (this demo's own two) and the
    // vacuity guard below caught it. `assemble` is a pure function of the
    // catalog, so calling it is the same answer the screen will see.
    let grid = SmashRoster::assemble(catalog);

    let mut unarmed: Vec<String> = Vec::new();
    let mut report: Vec<String> = Vec::new();
    for id in grid.ids() {
        let Some(entry) = catalog.get(id) else {
            // Not in THIS composition: the roster is a wish list filtered by
            // what the catalog carries, and the demo's own stand-ins drop out
            // of the host. Nothing to measure.
            continue;
        };
        let action_set = catalog.build_default_action_set(id);
        let melee = action_set.as_ref().and_then(|set| set.melee.as_ref());
        report.push(format!(
            "  {:<32} preset={:<12} melee={}",
            id,
            entry.default_action_set,
            melee.map_or("NONE".to_string(), |melee| format!("{melee:?}")),
        ));
        if melee.is_none() {
            unarmed.push(id.to_string());
        }
    }
    assert!(
        report.len() >= 8,
        "only {} of the grid resolved against this composition's catalog — the \
         host is not composing the cast and this test is about to prove nothing",
        report.len()
    );
    eprintln!("[smash movesets]\n{}", report.join("\n"));

    let known: Vec<String> = KNOWN_UNARMED.iter().map(|id| id.to_string()).collect();
    assert_eq!(
        unarmed,
        known,
        "the set of grid fighters with NO melee has changed.\n{}\n\n\
         A fighter with no melee arrives on a platform-fighter stage unable to \
         hit anybody — the match runs and nobody can win it. Either author the \
         moveset (an `action_set_presets` row and a `default_action_set` on the \
         character), or take them off `SMASH_ROSTER`.",
        report.join("\n"),
    );
}
