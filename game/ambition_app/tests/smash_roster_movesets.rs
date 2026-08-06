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
         A fighter with no melee in its CATALOG row is fine — the match arms \
         it, see below — but this list is the record of who needs a real kit, \
         and it has changed.",
        report.join("\n"),
    );
}

/// **And the MATCH arms every one of them.**
///
/// The catalog gap above is real and stays real: a Hall NPC's `peaceful` row is
/// correct where they live, and the crossover stage is the one place allowed to
/// say otherwise. `MatchParticipant::action_set` is where it says it — so this
/// asserts the thing that actually reaches a body, rather than the row it came
/// from.
///
/// ⚠ **the seam, not the numbers.** One kit for everybody is a FLOOR and is
/// honestly a levelling; per-character kits are the content job. What must not
/// regress is that a fighter reaches the stage able to hit somebody.
/// **Every id on the wish list is a character somebody can actually pick.**
///
/// ⛔ `SmashRoster::assemble` FILTERS to what the composition carries, which is
/// correct — the other demos' protagonists are only there when a host composes
/// them — and it means a MISSPELLED id is indistinguishable from an absent one.
/// The grid silently comes back one fighter short and the screen looks fine.
/// Jon set this roster by hand and expects to keep editing it (*"we may go more
/// than 8"*), so a typo dropping a fighter he asked for is the likely mistake.
///
/// ⚠ **the SHIPPED composition is the population**, so ids that are genuinely
/// composition-dependent must be named below rather than assumed present.
#[test]
fn every_id_on_the_smash_wish_list_names_a_real_character() {
    /// Ids the shipped host legitimately does not carry.
    ///
    /// ⭐ empty today, and that is the finding: `ambition_app` composes Mary-O
    /// and Sanic, so every name on the list resolves in the real game. An entry
    /// here would mean "this fighter only exists in some other host", which is a
    /// claim worth making explicitly rather than by silence.
    const COMPOSITION_DEPENDENT: &[&str] = &[];

    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");

    let wish_list = ambition_demo_smash::select::SMASH_ROSTER;
    assert!(
        wish_list.len() >= 8,
        "the wish list has shrunk to {} — Jon sized this grid by hand",
        wish_list.len()
    );

    let mut absent: Vec<&str> = wish_list
        .iter()
        .copied()
        .filter(|id| catalog.get(id).is_none())
        .filter(|id| !COMPOSITION_DEPENDENT.contains(id))
        .collect();
    absent.sort_unstable();
    assert!(
        absent.is_empty(),
        "smash roster id(s) name no character in the shipped composition, so the \
         grid silently comes back short and the screen still looks correct: \
         {absent:?}. Either the id is misspelled, or the character really is \
         composition-dependent and belongs in COMPOSITION_DEPENDENT with a \
         reason."
    );

    let stale: Vec<&str> = COMPOSITION_DEPENDENT
        .iter()
        .copied()
        .filter(|id| catalog.get(id).is_some())
        .collect();
    assert!(
        stale.is_empty(),
        "these ids are waived as composition-dependent and the shipped host \
         carries them: {stale:?}"
    );
}

#[test]
fn the_match_gives_every_seat_a_kit_that_can_hit() {
    use ambition_demo_smash::select::{SlotOccupant, SmashSelect};

    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    let grid = SmashRoster::assemble(catalog);
    assert!(grid.len() >= 8, "the grid is too short to be the host's");

    // Two seats, and deliberately the two whose CATALOG rows are peaceful —
    // picking armed characters would prove nothing.
    let unarmed: Vec<usize> = grid
        .ids()
        .enumerate()
        .filter(|(_, id)| {
            catalog
                .build_default_action_set(id)
                .and_then(|set| set.melee)
                .is_none()
        })
        .map(|(index, _)| index)
        .collect();
    assert!(
        unarmed.len() >= 2,
        "fewer than two peaceful rows on the grid, so this test is picking          characters that were already armed and proving nothing"
    );

    let mut select = SmashSelect::default();
    select.set_occupant(0, SlotOccupant::Controller { device: 0 });
    select.set_pick(0, unarmed[0]);
    select.set_occupant(1, SlotOccupant::Cpu);
    select.set_pick(1, unarmed[1]);

    let roster = select.roster(&grid).expect("two decided seats are a match");
    for participant in &roster.participants {
        let kit = participant
            .action_set
            .as_ref()
            .unwrap_or_else(|| panic!("seat wearing `{}` got no kit", participant.character));
        assert!(
            kit.melee.is_some(),
            "seat wearing `{}` reaches the stage unable to hit anybody",
            participant.character
        );
    }
}
