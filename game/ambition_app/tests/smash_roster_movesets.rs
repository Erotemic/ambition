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
    // ⭐ **ARRIVED 2026-08-12, and arriving is the point.** Jon added Stargan to
    // the grid on 2026-08-11 and he was never on it: `SmashRoster::assemble`
    // filters on the prepared REGISTRY, nothing registered him, and a dropped
    // portrait is silent by design. Registering him put him on the grid and this
    // census immediately reported what the grid had been hiding — his row says
    // `peaceful`, so he stands there with no melee.
    //
    // ⚠ he is NOT unarmed in a match: the stage arms every seat (see the
    // companion test below). This list is the record of who needs a real kit,
    // and whether Stargan FIGHTS is a product question already filed as D96
    // item 5 — so he belongs here rather than being given a swipe by a test
    // that noticed him.
    "npc_carl_stargan",
];

#[test]
fn every_fighter_on_the_smash_grid_can_throw_a_punch() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // ⚠ ONE frame first, and it is load-bearing: the seatable registry is filled
    // by a `Startup` system, so a build that has never updated has a catalog and
    // no registry at all. The grid is filtered by what can be SEATED — see
    // `SmashRoster::assemble` — so reading it before that frame would report an
    // empty screen and call it content.
    app.update();
    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    let registry = app
        .world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>()
        .expect("the composed host has a prepared-character registry");

    // ⚠ the ASSEMBLED grid, not the wish list: the demo's stand-in robots drop
    // out of a host that carries the real lineage, and measuring them here would
    // report a moveset nobody can pick.
    //
    // ⚠ **assembled HERE rather than read from the resource**, because the
    // resource is filled by a `Startup` system and `build_visible_app` has not
    // run a frame — reading it gave the DEFAULT (this demo's own two) and the
    // vacuity guard below caught it. `assemble` is a pure function of the
    // SEATABLE registry, so calling it is the same answer the screen will see.
    let grid = SmashRoster::assemble(registry);

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

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // ⚠ ONE frame first, and it is load-bearing: the seatable registry is filled
    // by a `Startup` system, so a build that has never updated has a catalog and
    // no registry at all. The grid is filtered by what can be SEATED — see
    // `SmashRoster::assemble` — so reading it before that frame would report an
    // empty screen and call it content.
    app.update();
    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    let registry = app
        .world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>()
        .expect("the composed host has a prepared-character registry");
    let grid = SmashRoster::assemble(registry);
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

    // ⚠ **the floor is DECLARED here because a stage declares it** (2026-08-12).
    // `roster()` is the convenience wrapper and it declares nothing, so calling
    // it seats a kit-less character with no kit — correctly. What the shipped
    // smash experience does is put this swipe on
    // `DeclaredCombatRules::unarmed_melee`, and a fixture that skipped that step
    // would be asserting about a stage nobody ships.
    let roster = select
        .roster_seeded(
            &grid,
            0,
            ambition_platformer2d::input::sources::InputAssignmentPolicy::UnifiedPrimary,
            &Default::default(),
            Some(ambition_platformer2d::character::MeleeActionSpec::Swipe(
                ambition_platformer2d::character::SwipeSpec {
                    windup_s: 0.22,
                    active_s: 0.08,
                    damage: 4,
                    reach_px: 34.0,
                    recover_s: 0.26,
                },
            )),
        )
        .expect("two decided seats are a match");
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

/// **The grid offers only ids the roster NAMES and the host can SEAT.**
///
/// ⛔ moved here from `ambition_demo_smash`'s own unit tests when the filter
/// moved from the CATALOG to the prepared registry. A row says what a character
/// IS; `register_character` is what makes one BUILDABLE, and eight of the twelve
/// shipped portraits were rows nothing had registered — seatable as player one,
/// where the adopted home body consulted the registry OPTIONALLY, and
/// unbuildable in every other seat. The demo crate cannot fill a registry (that
/// needs the preparation barrier, which needs a composition), so the claim
/// belongs against the REAL one.
///
/// Both directions, because each alone is satisfiable by a broken filter: a
/// grid of everything passes "only named ids" if it drops nothing, and an empty
/// grid passes "only seatable ids" trivially.
#[test]
fn the_grid_offers_only_named_and_seatable_fighters() {
    use ambition_demo_smash::select::{SmashRoster, SMASH_ROSTER};

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let registry = app
        .world()
        .resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>(
    );
    let grid = SmashRoster::assemble(registry);

    assert!(
        grid.len() >= 8,
        "the assembled grid is {} fighters — too short to be the shipped host's, \
         so the assertions below would pass over an empty screen",
        grid.len()
    );
    for id in grid.ids() {
        assert!(
            SMASH_ROSTER.contains(&id),
            "`{id}` is on the grid and the roster never named it"
        );
        assert!(
            registry.get(id).is_some(),
            "`{id}` is a portrait a player can pick and a seat the match would \
             then refuse: nothing registered it"
        );
    }
}

/// **AND THE QUESTION THIS FILE IS NAMED FOR — who actually has a REPERTOIRE?**
///
/// ⛔ the file is titled *"Does every fighter on the smash grid have a
/// MOVESET?"*, quotes Jon asking for movesets, and then measures ACTION SETS: a
/// preset melee, one swing, the thing a body reaches for. That was the honest
/// measurement in August when nobody authored a moveset — a census of a set
/// everyone was empty in reports nothing. Four characters author one now, so the
/// question the title asks is finally answerable, and it is the one P3.24's
/// count is supposed to be falling on.
///
/// ```text
///   action set   CAN this body swing            (a preset, one melee)
///   moveset      what its eleven presses ARE    (jab, tilts, smashes, aerials)
/// ```
///
/// ⚠ **a ratchet, not a target**, exactly like `KNOWN_UNARMED` above: the list
/// is the record of who still takes the generic floor, and it may only shrink.
/// Authoring a repertoire is a content job and this exists so the count cannot
/// quietly grow while one is being done.
#[test]
fn the_grid_fighters_with_a_real_repertoire_only_grow() {
    /// Grid fighters that author their OWN move timelines.
    ///
    /// ⚠ **measured 2026-08-12 against the shipped host: SEVEN of fourteen.**
    /// The seven on the generic floor are Mary-O and Sanic (other demos'
    /// protagonists, who bring their own bodies but no smash table) and the five
    /// Hall NPCs who were authored to stand in a room and talk.
    const WITH_REPERTOIRE: &[&str] = &[
        // The protagonist's canonical table, consumed identically by both games
        // (redirect §15) — the reason it lives on the Robot provider at all.
        "player_robot_v3",
        // Ambition's own, written to remove adopters from the generic floor
        // (P3.24): shorter/faster/weaker than the robot, longer/slower/harder,
        // and the heavyweight controller respectively.
        "goblin",
        "npc_pirate_admiral",
        "special_patent_clerk",
        // ⭐ **THE TWO I DID NOT EXPECT, and the test found them rather than my
        // memory.** I wrote this list from what I knew had been authored — the
        // robot's table plus the three written to remove adopters — and it was
        // wrong by two, in the direction that matters least and proves the most:
        // there is MORE authored content than the person writing the ratchet
        // believed.
        //
        // The demo's own fighter, whose eleven-move table is built on the law of
        // the excluded middle its catalog row quotes. It reaches the HOST grid
        // because the smash provider registers it, which is the crossover
        // working.
        "smash_george_booul",
        // The Perfect Cellular Automaton's Cellular Pulse — a real
        // `MovesetContract`, authored when its ninety-line archetype row was
        // deleted (D89).
        "perfect_cellular_automaton",
        // The counter-puncher, authored from his own BARKS — the fourth adopter
        // removed from the generic floor, and this ratchet is what asked for him
        // by name the moment his table landed.
        "npc_ninja_shadow_oni_leader",
    ];

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let registry = app
        .world()
        .resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>(
    );
    let grid = SmashRoster::assemble(registry);
    assert!(
        grid.len() >= 8,
        "the assembled grid is {} fighters — too short to be the shipped host's",
        grid.len()
    );

    let mut authored: Vec<&str> = Vec::new();
    let mut generic: Vec<&str> = Vec::new();
    for id in grid.ids() {
        let has = registry
            .get(id)
            .is_some_and(|definition| definition.authored_moveset.is_some());
        if has {
            authored.push(id);
        } else {
            generic.push(id);
        }
    }
    eprintln!("[repertoires] authored={authored:?}\n              generic={generic:?}");

    let lost: Vec<&&str> = WITH_REPERTOIRE
        .iter()
        .filter(|id| !authored.contains(id))
        .collect();
    assert!(
        lost.is_empty(),
        "these fighters authored their own moves and no longer reach the grid \
         with them: {lost:?}. A repertoire that stops arriving is silent — the \
         body still swings, it just swings the generic floor's swipe."
    );

    // ⭐ the ratchet's other half: a NEW repertoire must be recorded here, so
    // the count is a fact rather than a memory.
    let unrecorded: Vec<&&str> = authored
        .iter()
        .filter(|id| !WITH_REPERTOIRE.contains(id))
        .collect();
    assert!(
        unrecorded.is_empty(),
        "these fighters author a repertoire and the ledger above does not say \
         so: {unrecorded:?}. Add them — the list is what makes P3.24's count \
         mean something."
    );

    // ⛔ and the poison: some fighter must still be on the generic floor, or
    // this test has stopped distinguishing anything and P3.24 is DONE — in
    // which case delete it rather than leave it passing.
    assert!(
        !generic.is_empty(),
        "every grid fighter authors a repertoire, so the generic floor has no \
         adopters left: P3.24 is complete and this ratchet should go with it"
    );
}
