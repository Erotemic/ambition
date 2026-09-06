//! Does every fighter on the smash grid have a MOVESET?
//!
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
//! is unambiguously wrong — a fighter with no melee at all, who arrives on a
//! platform-fighter stage unable to hit anybody.
//!
//! the number is a ratchet, not a target. Authoring twelve movesets is a
//! content job; this exists so the count cannot quietly grow while it is being
//! done, and so the list is derived from the roster rather than remembered.

use ambition_demo_smash::select::SmashRoster;
use ambition_platformer2d::character::CharacterCatalog;

/// Fighters on the grid whose catalog row gives them NO melee.
///
/// The five that can hit are the ones whose rows already named a combat preset: George Booul and
/// the demo's duelists (`smash::duelist` (cite-ok: authored key), 4 damage, 34px), and the Pirate Admiral, Shadow Oni
/// Leader, Perfect Cellular Automaton and Goblin (Ambition's `striker_swipe` / `pirate_pistol`, 1
/// damage, 28px).
///
/// so it is not only the seven. The five that CAN hit disagree by 4x on damage, because two of
/// them are a demo's fighter and three are an adventure game's enemies.
const KNOWN_UNARMED: &[&str] = &[
    "player_robot_v3",
    // TALL Mary-O is the one on the grid now; her kit is byte-identical to the
    // short form's, so she arrives here unarmed exactly as `mary_o` did.
    "mary_o_tall",
    "sanic",
    "npc_alice",
    "npc_bob",
    // AND EMMY LEFT IT THE SAME DAY, for the same two reasons.
    //
    // he is NOT unarmed in a match: the stage arms every seat (see the companion test below).
    //
    // ⭐ AND THIS ONE IS HERE FOR A DIFFERENT REASON THAN THE FIVE ABOVE, which
    // is why it is not simply appended. The others are on this list because
    // nobody has given them a kit yet. `projectile_polygon` is RANGED BY
    // DESIGN — its catalog row names `ranger_arrow` and its combat distinction
    // is a body-authored projectile from a head-mounted cannon — and it already
    // carries a full authored moveset (`projectile_polygon_moveset`). It has no
    // catalog-row MELEE, which is what this census measures, and it does not
    // need one.
    //
    // ⛔ So do not read this list as "fighters that cannot fight". It is
    // "fighters whose CATALOG ROW gives them no melee", and those are two
    // different populations now that the grid has a ranged character on it. If
    // a third reason ever lands here, this list has outlived its question.
    "projectile_polygon",
];

#[test]
fn every_fighter_on_the_smash_grid_can_throw_a_punch() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // ONE frame first, and it is load-bearing: the seatable registry is filled by a `Startup`
    // system, so a build that has never updated has a catalog and no registry at all.
    app.update();
    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    let registry = app
        .world()
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .expect("the composed host has a prepared-character registry");

    // the ASSEMBLED grid, not the wish list: the demo's stand-in robots drop
    // out of a host that carries the real lineage, and measuring them here would
    // report a moveset nobody can pick.
    //
    // assembled HERE rather than read from the resource, because the
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

/// And the MATCH arms every one of them.
///
/// The catalog gap above is real and stays real: a Hall NPC's `peaceful` row is correct where
/// they live, and the crossover stage is the one place allowed to say otherwise.
///
/// the seam, not the numbers. One kit for everybody is a FLOOR and is
/// honestly a levelling; per-character kits are the content job. What must not
/// regress is that a fighter reaches the stage able to hit somebody.
/// Every id on the wish list is a character somebody can actually pick.
///
/// `SmashRoster::assemble` FILTERS to what the composition carries, which is
/// correct — the other demos' protagonists are only there when a host composes
/// them — and it means a MISSPELLED id is indistinguishable from an absent one.
/// The grid silently comes back one fighter short and the screen looks fine.
/// than 8"*), so a typo dropping a fighter he asked for is the likely mistake.
///
/// the SHIPPED composition is the population, so ids that are genuinely
/// composition-dependent must be named below rather than assumed present.
#[test]
fn every_id_on_the_smash_wish_list_names_a_real_character() {
    /// Ids the shipped host legitimately does not carry.
    ///
    /// empty today, and that is the finding: `ambition_app` composes Mary-O
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
    // ONE frame first, and it is load-bearing: the seatable registry is filled by a `Startup`
    // system, so a build that has never updated has a catalog and no registry at all.
    app.update();
    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    let registry = app
        .world()
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
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

    let roster = select
        .roster_seeded(
            &grid,
            0,
            ambition_platformer2d::input::sources::InputAssignmentPolicy::UnifiedPrimary,
            &Default::default(),
            // Both sides call one function now, so a stage that stops declaring a floor turns
            // this red instead of passing.
            Some(ambition_demo_smash::smash_seating_melee()),
            ambition_demo_smash::STARTING_STOCKS,
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

/// The grid offers only ids the roster NAMES and the host can SEAT.
///
/// A row says what a character IS; `register_character` is what makes one BUILDABLE, and eight of
/// the twelve shipped portraits were rows nothing had registered — seatable as player one, where
/// the adopted home body consulted the registry OPTIONALLY, and unbuildable in every other seat.
/// The demo crate cannot fill a registry (that needs the preparation barrier, which needs a
/// composition), so the claim belongs against the REAL one.
///
/// Both directions, because each alone is satisfiable by a broken filter: a
/// grid of everything passes "only named ids" if it drops nothing, and an empty
/// grid passes "only seatable ids" trivially.
/// ⛔⛔ EVERY FIGHTER WHOSE MOVES SOMEBODY AUTHORED MUST REACH THE GRID, and the
/// test beside this one cannot say so. That one checks that everything ON the
/// grid is named and seatable plus a length floor — both of which a grid that
/// silently DROPPED a fighter still passes. ⇒ Container versus contents: a guard
/// that validates what survived a filter can never notice what the filter ate.
///
/// ⭐ This matters because a dropped id is invisible in exactly the way that
/// costs most: the fighter's moveset still compiles, its content tests still
/// pass, and nobody can pick it. On 2026-09-05 ten fighters gained authored
/// techniques, and every one of them is only worth anything if it is selectable.
#[test]
fn every_fighter_this_composition_can_build_reaches_the_grid() {
    use ambition_demo_smash::select::{SmashRoster, SMASH_ROSTER, STAND_INS};

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let registry =
        app.world()
            .resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>();
    let grid = SmashRoster::assemble(registry);
    let offered: Vec<&str> = grid.ids().collect();

    let mut missing: Vec<&str> = Vec::new();
    for id in SMASH_ROSTER {
        // Not buildable in this composition — correctly dropped, and the row
        // beside this one already says why a hole is worse than an absence.
        if registry.get(id).is_none() {
            continue;
        }
        // A stand-in steps aside once the character it stands in for is present.
        let stood_down = STAND_INS
            .iter()
            .any(|(copy, real)| copy == id && registry.get(real).is_some());
        if stood_down {
            continue;
        }
        if !offered.contains(id) {
            missing.push(id);
        }
    }
    assert!(
        missing.is_empty(),
        "these fighters are buildable in the shipped composition and are NOT on \
         the select grid, so nobody can pick them and their authored moves never \
         run: {missing:?}"
    );
}

#[test]
fn the_grid_offers_only_named_and_seatable_fighters() {
    use ambition_demo_smash::select::{SmashRoster, SMASH_ROSTER};

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let registry =
        app.world()
            .resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>();
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

// Every fighter on the grid now authors a repertoire; no generic-repertoire
// migration ratchet remains.

/// Stand-in robots are used only when the canonical robot lineage is unavailable.
/// `SmashRoster::assemble` drops each stand-in when its canonical character resolves.
///
/// TODO(compat-remove): remove the stand-ins once the standalone Smash demo can
/// consume the canonical robot lineage without depending on a game-level content crate.
#[test]
fn the_demos_robot_copies_step_aside_for_the_real_lineage() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let registry = app
        .world()
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .expect("the composed host has a prepared-character registry");
    let assembled = SmashRoster::assemble(registry);
    let grid: Vec<String> = assembled.ids().map(str::to_string).collect();

    // The real lineage is what this host offers.
    assert!(
        grid.iter().any(|id| id == "player_robot_v3"),
        "the composed host must actually carry the real robot, or 'the copy \
         stepped aside' is a statement about a grid that has neither: {grid:?}"
    );

    for copy in ambition_demo_smash::select::OWN_FIGHTERS {
        assert!(
            !grid.iter().any(|id| id == copy),
            "`{copy}` is a stand-in for a character THIS composition registers, \
             so the grid is showing two robots and one of them has a made-up \
             name: {grid:?}"
        );
        assert!(
            ambition_demo_smash::select::SmashRoster::default()
                .ids()
                .any(|id| id == *copy),
            "`{copy}` is gone from the standalone demo's own cast, so the drop \
             rule above is not what removed it from the host grid"
        );
    }
}

/// a MASK could not promise this, and the gap was invisible because
/// almost nobody exercised it. `MatchParticipantRoster::fighter_abilities`
/// (cite-ok: a field that has since collapsed into `rules`; the history is the
/// point) was one set, intersected:
///
/// ```text
///   character authors nothing  ->  the stage's set verbatim   (twelve of fourteen)
///   character authors a kit    ->  kit ∩ stage                (two of fourteen)
/// ```
///
/// So the stage's `ledge_grab: true` reached twelve fighters unchanged and nobody noticed the
/// rule had teeth. The one fighter on the grid whose sheet has ten ledge rows drawn for it was
/// the one who could not use them.
///
/// ⭐⭐ THE STAGE NO LONGER LEVELS — it declares a FLOOR and a CEILING, and the
/// gap between them is character identity. Jon, W8 playtest: *"Do not make Pogo
/// a universal Smash action. Robot v3 has Pogo because Robot v3 owns that
/// capability."* So the property this test asserts changed shape, from an
/// equality to a pair of containments:
///
/// ```text
///   floor ⊆ effective        nobody is short of what the stage promised
///   effective ⊆ ceiling      nobody smuggles its home game onto the stage
/// ```
///
/// ⛔ AND THE ASSERTION THIS REPLACED WAS AN EQUALITY, which is exactly the
/// world Jon rejected: it could only pass while every fighter played the same
/// kit, so it would have had to be weakened by any character keeping anything of
/// its own. ⭐ the two non-vacuity clauses are what keep the containments
/// honest — some fighter must reach the ceiling and differ from the floor (or
/// the gap is decorative), and some fighter must sit exactly on the floor (or
/// the floor is doing nothing).
///
/// it asks the ENGINE's own function. That a seat actually WEARS this
/// answer is pinned separately, on a live body, by `the_stage_kills.rs`'s
/// `a_seated_fighter_carries_the_verbs_its_character_authored_and_not_the_engines`.
#[test]
fn every_smash_fighter_lands_between_the_stages_floor_and_its_ceiling() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // One frame, for the same reason the census above needs it: the seatable
    // registry is filled by a `Startup` system.
    app.update();
    let registry = app
        .world()
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .expect("the composed host has a prepared-character registry");
    let grid = SmashRoster::assemble(registry);

    // The stage's own declaration, from the stage's own roster builder rather
    // than a literal. the CAST does not matter: `smash_roster` states one rule
    // for the match, which is the whole reason the fighters below can be
    // compared against a single kit.
    let rules = ambition_demo_smash::smash_roster(grid.ids().take(2))
        .rules
        .abilities
        .expect("the smash stage declares what a fighter on it may do");
    assert!(
        rules.is_coherent(),
        "the stage GRANTS a verb it does not PERMIT, so `apply` drops it and          every fighter below is short of the kit this stage promised: {rules:?}"
    );

    let floor = ambition_demo_smash::SMASH_FIGHTER_KIT;
    let ceiling = ambition_demo_smash::SMASH_FIGHTER_CEILING;
    assert_ne!(
        floor, ceiling,
        "the stage's floor and ceiling are the same set, so this test is about \
         to assert an equality wearing a containment's clothes"
    );

    let mut short_of_the_floor: Vec<String> = Vec::new();
    let mut over_the_ceiling: Vec<String> = Vec::new();
    let mut report: Vec<String> = Vec::new();
    // NON-VACUITY, both directions. Somebody must USE the gap between the two
    // sets, and somebody must sit exactly on the floor — a grid where every
    // fighter reached the ceiling would mean the floor promised nothing, and one
    // where none did would mean the ceiling permitted nothing.
    let mut somebody_reached_past_the_floor = false;
    let mut somebody_sat_on_the_floor = false;
    for id in grid.ids() {
        let authored = registry.get(id).and_then(|prepared| prepared.abilities);
        let effective =
            ambition_platformer2d::versus_match::effective_abilities(authored, Some(rules))
                .expect("a declaring stage always answers");
        if effective == floor {
            somebody_sat_on_the_floor = true;
        } else {
            somebody_reached_past_the_floor = true;
        }
        report.push(format!(
            "  {:<34} authored={:<4} kit={}",
            id,
            authored.map_or("no", |_| "yes"),
            if effective == floor {
                "the stage's floor".to_string()
            } else if effective == ceiling {
                "the stage's ceiling".to_string()
            } else {
                format!("{effective:?}")
            },
        ));
        // `union` is the floor's containment and `intersect` is the ceiling's,
        // asked as set equalities so the failure names the fighter rather than a
        // bitfield diff.
        if effective.union(floor) != effective {
            short_of_the_floor.push(id.to_string());
        }
        if effective.intersect(ceiling) != effective {
            over_the_ceiling.push(id.to_string());
        }
    }

    assert!(
        report.len() >= 8,
        "only {} fighters resolved against this composition — the host is not \
         composing the cast and this test is about to prove nothing",
        report.len()
    );
    assert!(
        somebody_reached_past_the_floor,
        "every fighter on the grid resolved to exactly the floor, so the stage's \
         ceiling permits nothing the floor does not already grant and character \
         identity is not surviving preparation at all:\n{}",
        report.join("\n")
    );
    assert!(
        somebody_sat_on_the_floor,
        "every fighter reached past the floor, so the floor is not the thing \
         being measured and a fighter arriving short of it would not be \
         noticed:\n{}",
        report.join("\n")
    );
    assert!(
        short_of_the_floor.is_empty(),
        "{} of the smash grid arrive SHORT of the verbs the stage guarantees: \
         {short_of_the_floor:?}\n{}",
        short_of_the_floor.len(),
        report.join("\n")
    );
    assert!(
        over_the_ceiling.is_empty(),
        "{} of the smash grid arrive carrying verbs the stage does not permit: \
         {over_the_ceiling:?}\n{}\n\nThat is a home-game kit reaching the body \
         through some other door.",
        over_the_ceiling.len(),
        report.join("\n")
    );
}

/// AND THE PLATFORMER PROTAGONISTS DO NOT TAKE THE FIGHTER'S KIT HOME.
///
/// ability in their games"* (the ledge grab), and then *"Sanic should never have
/// fly, blink, or wall climb in any iteration"*.
///
/// the other half of the test above, and the reason both are needed. A
/// character reaches a body down two different roads and they are not the same
/// road:
///
/// ```text
///   its own game    catalog GRANT LIST -> the session's avatar   (`session/setup`)
///   a smash seat    character DEFINITION under the match's `MatchAbilities`
/// ```
///
/// So "everybody on the grid has the fighter's kit" and "the platformer
/// protagonists have their own at home" are both satisfiable, and each is one
/// edit away from breaking the other. Asserting them in one file is what makes
/// the split legible instead of a coincidence.
///
/// He was carrying flight, blink, wall climb, a ledge grab, swim, glide, dodge and a shield around
/// his own speedway; his control gate resolves Attack and Utility onto spin dash and transform, so
/// nothing on screen ever said so.
///
/// The super form is included too: transformation changes speed, not available verbs.
#[test]
fn the_platformer_protagonists_keep_their_own_kits_at_home() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");

    for id in ["mary_o", "sanic", "super_sanic"] {
        let kit = catalog.ability_set(id).unwrap_or_else(|| {
            panic!(
                "`{id}` authors no grant list, so its own game hands it the dev \
                 sandbox set — which is `sandbox_all`: flight, blink, wall climb \
                 and a ledge grab included"
            )
        });
        assert!(
            !kit.ledge_grab,
            "`{id}` can grab a ledge in its own game: {kit:?}"
        );
        assert!(
            !kit.fly && !kit.fly_toggle,
            "`{id}` can fly in its own game: {kit:?}"
        );
        assert!(
            !kit.blink
                && !kit.precision_blink
                && !kit.blink_through_soft_walls
                && !kit.blink_through_hard_walls,
            "`{id}` can blink in its own game: {kit:?}"
        );
        assert!(
            !kit.wall_climb,
            "`{id}` can climb walls in its own game: {kit:?}"
        );
        // NON-VACUITY: a kit of nothing would satisfy every line above and
        // would mean the character cannot move.
        assert!(
            kit.move_horizontal && kit.jump,
            "`{id}`'s authored kit cannot run and jump, so this asserted the \
             absence of verbs on a body that has none of them: {kit:?}"
        );
    }
}

/// WHO HAS A FULL SMASH KIT, AND WHAT IS EACH MISSING?
///
/// The census above answers *does this fighter author a table at all*. This one
/// answers the question that follows it: a table with a jab and one aerial is
/// authored, and it is not a moveset. Sixteen presses is what a platform fighter
/// asks a character for.
///
/// IT RESOLVES EACH PRESS THE WAY A BODY DOES, through
/// `move_for_directional_verb`, rather than asking whether a verb key exists.
/// The difference is the whole finding: `directional_verb_chain` FALLS BACK, so
/// a fighter with no forward tilt does not press nothing — it presses its jab
/// again. A key-existence census reports that as "missing" and a player
/// experiences it as "this character has no forward tilt", and only one of those
/// two readings tells you what to author.
///
/// and it reads the MERGED kit (`PreparedKit::projectable_moveset`), not
/// `authored_moveset`. Authored moves OVERLAY the kit derived from the action
/// set rather than replacing it, so a character can reach a press through either
/// — the body resolves the merge, so the merge is what a report about the body
/// has to read.
const SMASH_KIT: &[(&str, &str, ambition_platformer2d::entity_catalog::AttackDir)] = {
    use ambition_platformer2d::entity_catalog::AttackDir::*;
    &[
        ("jab", "attack", Neutral),
        ("ftilt", "attack", Forward),
        ("utilt", "attack", Up),
        ("dtilt", "attack", Down),
        ("fsmash", "smash", Forward),
        ("usmash", "smash", Up),
        ("dsmash", "smash", Down),
        ("nair", "attack", Neutral),
        ("fair", "attack", Forward),
        ("bair", "attack", Back),
        ("uair", "attack", Up),
        ("dair", "attack", Down),
        ("nspecial", "special", Neutral),
        ("sspecial", "special", Forward),
        ("uspecial", "special", Up),
        ("dspecial", "special", Down),
    ]
};

/// THE CAPTURE HALF OF THE KIT — `(label, verb)`, resolved by verb rather
/// than by direction.
///
/// Grabs are the first of that list to become authorable across the whole roster: until
/// fourteen movesets authored only `forward_throw` and left back/up/down `None`, so this list
/// could not have been asserted on anybody.
///
/// resolved with `move_for_verb`, NOT `move_for_directional_verb`, and that
/// is not a detail. A throw is selected by the attack press INSIDE a capture
/// relationship, not by a directional grab press — `smash_capture::verbs` says
/// so explicitly, because naming them `grab_forward` would invite the
/// directional matcher to light the Grab slot for a fighter that authored only
/// throws. Asking the directional road here would reproduce that exact
/// confusion inside the census.
const SMASH_CAPTURE_KIT: &[(&str, &str)] = &[
    ("grab", ambition_platformer2d::entity_catalog::GRAB_VERB),
    (
        "pummel",
        ambition_platformer2d::entity_catalog::CAPTURE_PUMMEL_VERB,
    ),
    (
        "fthrow",
        ambition_platformer2d::entity_catalog::CAPTURE_THROW_FORWARD_VERB,
    ),
    (
        "bthrow",
        ambition_platformer2d::entity_catalog::CAPTURE_THROW_BACK_VERB,
    ),
    (
        "uthrow",
        ambition_platformer2d::entity_catalog::CAPTURE_THROW_UP_VERB,
    ),
    (
        "dthrow",
        ambition_platformer2d::entity_catalog::CAPTURE_THROW_DOWN_VERB,
    ),
];

/// Presses plus captures. The ratchet reads THIS, so both halves grow it.
const KIT_TOTAL: usize = SMASH_KIT.len() + SMASH_CAPTURE_KIT.len();

/// Which postures a press is asked in.
///
/// ```text
///   jab, tilts, smashes   grounded
///   aerials               airborne   — same verbs, told apart by nothing else
///   specials              BOTH
/// ```
///
/// asking only one posture INVENTS A GAP and HIDES another. George Booul's down-B is a
/// commanded plunge and is `airborne_only`; probed standing on the ground it is skipped by its
/// own gate, falls down the directional chain to his neutral-B, and reads as "missing" in a
/// census that never left the floor. He had it all along.
fn postures(label: &str) -> &'static [bool] {
    match label {
        "nair" | "fair" | "bair" | "uair" | "dair" => &[false],
        "nspecial" | "sspecial" | "uspecial" | "dspecial" => &[true, false],
        _ => &[true],
    }
}

/// Whether EVERY posture this press is asked in owes an answer of its own.
///
///  a special gated to one posture is NOT covered by answering in that one.
/// Pressed in the other, the chain walks past it to the character's neutral
/// special — or to nothing at all — and the player pressed down-B and got
/// something else. `special_air_down` is the verb that expresses the two-form
/// move and it has been in the chain the whole time.
fn every_posture_must_answer(label: &str) -> bool {
    matches!(label, "nspecial" | "sspecial" | "uspecial" | "dspecial")
}

#[test]
fn report_the_smash_kit_every_selectable_fighter_has() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let registry =
        app.world()
            .resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>();
    let grid = SmashRoster::assemble(registry);

    let mut rows: Vec<String> = Vec::new();
    for id in grid.ids() {
        let Some(prepared) = registry.get(id) else {
            continue;
        };
        let authored = prepared.authored_moveset.is_some();
        let Some(moveset) = prepared.kit.projectable_moveset() else {
            rows.push(format!("  {id:<34} NO KIT AT ALL"));
            continue;
        };
        let mut distinct: Vec<&str> = Vec::new();
        let mut wrong: Vec<String> = Vec::new();
        let mut seen: std::collections::BTreeMap<String, &str> = Default::default();
        for (label, base, dir) in SMASH_KIT {
            let reached: Vec<Option<String>> = postures(label)
                .iter()
                .map(|grounded| {
                    moveset
                        .move_for_directional_verb(base, *dir, *grounded)
                        .map(|mv| mv.id.clone())
                })
                .collect();
            // A press that reaches only moves an EARLIER press already claimed
            // is a fallback, not a move of its own: the body swings the same
            // timeline for both.
            let its_own = |id: &Option<String>| {
                id.as_ref().is_some_and(|id| {
                    !seen.contains_key(id.as_str()) || seen[id.as_str()] == *label
                })
            };
            let answered = if every_posture_must_answer(label) {
                reached.iter().all(its_own)
            } else {
                reached.iter().any(its_own)
            };
            if answered {
                distinct.push(label);
            } else {
                for (grounded, id) in postures(label).iter().zip(reached.iter()) {
                    let posture = if *grounded { "ground" } else { "air" };
                    match id {
                        None => wrong.push(format!("{label}/{posture}=nothing")),
                        Some(other) if !its_own(id) => {
                            wrong.push(format!("{label}/{posture}={}", seen[other.as_str()]))
                        }
                        Some(_) => {}
                    }
                }
            }
            for id in reached.iter().flatten() {
                seen.entry(id.clone()).or_insert(label);
            }
        }
        // The capture half. Same "its own move" rule: a throw that resolves to
        // a move an earlier entry already claimed is a fallback wearing a
        // throw's name, which is the thing `bound()` refuses to produce.
        for (label, verb) in SMASH_CAPTURE_KIT {
            match moveset.move_for_verb(verb).map(|mv| mv.id.clone()) {
                Some(id) if !seen.contains_key(id.as_str()) => {
                    distinct.push(label);
                    seen.insert(id, label);
                }
                Some(id) => wrong.push(format!("{label}={}", seen[id.as_str()])),
                None => wrong.push(format!("{label}=nothing")),
            }
        }
        rows.push(format!(
            "  {id:<34} {}  {:>2}/{} presses  moves={:<3} | not its own: {}",
            if authored { "authored" } else { "DERIVED " },
            distinct.len(),
            KIT_TOTAL,
            moveset.moves.len(),
            if wrong.is_empty() {
                "-".to_string()
            } else {
                wrong.join(" ")
            }
        ));
    }
    eprintln!("[smash kit census]\n{}", rows.join("\n"));
    assert!(rows.len() >= 8, "the grid did not assemble: {rows:?}");

    // it fails on a DROP, never on the target moving. When trips, grabs
    // and techs join the vocabulary, `SMASH_KIT` grows and this reads the new
    // length by itself — the number is not a copy of the target, it IS the
    // target.
    let short: Vec<&String> = rows
        .iter()
        .filter(|row| !row.contains(&format!("{:>2}/{} presses", KIT_TOTAL, KIT_TOTAL)))
        .collect();
    assert!(
        short.is_empty(),
        "{} selectable fighter(s) are short of the full {}-entry kit:\n{}\n\n\
         A press with no move of its own is not silence — `directional_verb_chain` \
         falls back, so it swings something ELSE and reads as a character missing \
         a move rather than as a bug. An unauthored THROW is different and worse \
         in its own way: it resolves to NOTHING on purpose, so the direction is \
         simply a dead input.",
        short.len(),
        KIT_TOTAL,
        rows.join("\n")
    );
}

/// Fighters on the grid the stage's body cannot reach, because they do not
/// move by the axis-swept model at all.
///
/// Supplying him a body changes nothing; he cannot air dodge on this stage whatever the stage says,
/// and neither can he shield-parry or tumble.
///
/// a RATCHET: the list may only SHRINK. What must never happen is a new
/// fighter joining it silently.
const NOT_AN_AXIS_BODY: &[&str] = &["sanic"];

/// THE PLATFORM FIGHTER'S BODY REACHES NO GAME THAT DID NOT ASK FOR ONE. ( slice 1b)
///
/// the whole risk of a stage-supplied body is a floor LEAKING. The
/// numbers `SMASH_FIGHTER_BODY` carries are wrong everywhere else and the engine
/// says so in its own defaults: a jump squat is a different game's jump (Mary-O's
/// SMB1 convergence needs the leap on the press tick), an air dodge steals the
/// airborne burst press from every exploration body, and a tumble floor makes a
/// wandering enemy stand up after every hit.
///
/// a supply is opt-in by CONSTRUCTION, and this asserts the construction
/// rather than a sample of its consequences: a match receives a body only
/// where its own roster declares one, so the two facts that matter are that the
/// DEFAULT is no supply, and that the stage next door — the other match in this
/// host — declares none.
#[test]
fn the_smash_stages_body_reaches_no_game_that_did_not_ask_for_one() {
    use ambition_platformer2d::engine_core as ae;

    assert!(
        ambition_platformer2d::actor::MatchParticipantRoster::default()
            .rules
            .body
            .is_none(),
        "a roster now supplies a fighter's body by DEFAULT, so every scripted \
         encounter, boss and fixture in the engine just became a platform fighter"
    );
    assert!(
        ambition_app::app::versus::versus_roster(1)
            .rules
            .body
            .is_none(),
        "the versus stage supplies a body it never declared — its cast is two \
         characters authored FOR it, and a duel wants none of a platform \
         fighter's extras"
    );

    // AND THE ENGINE DEFAULT IS STILL THE ENGINE'S. The cheapest way to
    // make the grid census above pass is to move these four numbers, and doing
    // it would change Mary-O, the exploration protagonist and every wandering
    // enemy in the game at once. Each is zero (or the explorer's recoil) for a
    // reason written at `DEFAULT_TUNING`.
    assert_eq!(ae::DEFAULT_TUNING.air_dodge_time, 0.0);
    assert_eq!(ae::DEFAULT_TUNING.tumble_speed, 0.0);
    assert_eq!(ae::DEFAULT_TUNING.jump_squat_time, 0.0);
    assert_eq!(
        ae::DEFAULT_TUNING.slash_recoil,
        ae::SLASH_RECOIL,
        "the engine default lost the exploration protagonist's melee recoil — \
         the smash stage authors 0.0 for ITSELF, and that is the point"
    );
}

/// EVERY FIGHTER ON THE SMASH GRID GETS A BODY THAT CAN ACTUALLY DODGE. ( slice 1b)
///
/// the companion above proves the stage GRANTS `dodge` to all fourteen,
/// and that was not enough. A verb needs a WINDOW, and the window was
/// authorable only on a CHARACTER:
///
/// ```text
///   DEFAULT_TUNING.air_dodge_time = 0.0     deliberately - an air dodge that
///                                           was on by default would steal the
///                                           airborne burst press from every
///                                           exploration body in the game
/// ```
///
/// it asks the ENGINE's own resolver, not a threshold of its own:
/// `resolve_burst_maneuver` is the one expression the kernel and the autonomous
/// driver both read, so a fighter this test calls capable is one the sim will
/// actually air-dodge.
#[test]
fn every_fighter_on_the_smash_grid_gets_a_body_that_can_air_dodge() {
    use ambition_platformer2d::engine_core as ae;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // One frame, for the same reason the census above needs it: the seatable
    // registry is filled by a `Startup` system.
    app.update();
    let registry = app
        .world()
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .expect("the composed host has a prepared-character registry");
    let grid = SmashRoster::assemble(registry);

    // The stage's own declaration, from the stage's own roster builder rather
    // than a literal — the same source the kit census reads.
    let roster = ambition_demo_smash::smash_roster(grid.ids().take(2));
    let rules = ambition_platformer2d::versus_match::MatchRules {
        body: roster.rules.body,
        ..Default::default()
    };
    assert!(
        rules.body.is_some(),
        "the smash stage declares no body at all, so nothing below is measuring \
         a supply — see `MatchParticipantRoster::fighter_body`"
    );

    let mut report: Vec<String> = Vec::new();
    let mut shut: Vec<String> = Vec::new();
    // NON-VACUITY: at least one fighter must reach the stage's numbers over a
    // body its CHARACTER authored, or this is only measuring the default case
    // and the composition could be a wholesale replacement.
    let mut composed_over_an_authored_body = false;
    for id in grid.ids() {
        let prepared = registry
            .get(id)
            .expect("the grid is assembled from the registry");
        let authored = prepared.movement_tuning;
        // the ENGINE's composition, asked once — not a second copy of it here.
        // the base for a character that authored none is `DEFAULT_TUNING`
        // rather than the seat's built tuning: this census is about the WINDOWS,
        // and none of them is a number a construction seed states.
        let body = rules
            .body_over(authored, ae::DEFAULT_TUNING)
            .expect("a stage that declares a body always answers");
        if authored.is_some() {
            composed_over_an_authored_body = true;
        }
        // the MODEL decides whether the window is even asked about. A
        // non-axis body reads `AxisSweptMotion::default()` in
        // `perception_body_for`, so reading the tuning alone here would call a
        // momentum body capable of an evade it structurally cannot perform.
        let axis = matches!(prepared.motion_model, ae::MotionModelSpec::AxisSwept(_));
        let params = if axis {
            body.axis_swept_params()
        } else {
            ae::AxisSweptParams::default()
        };
        // Airborne, nothing spent, nothing on cooldown: the press a player makes
        // on the way up. The only term that varies across the grid is the body.
        let maneuver = ae::resolve_burst_maneuver(
            &ae::BodyAbilities {
                abilities: ambition_demo_smash::SMASH_FIGHTER_KIT,
                ..Default::default()
            },
            &ae::BodyGroundState {
                head_contact: false,
                on_ground: false,
                contact_initialized: true,
            },
            &ae::BodyDodgeState::default(),
            &ae::AxisManeuverState::default(),
            &ae::BodyDashState::default(),
            params,
        );
        report.push(format!(
            "  {:<34} brought={:<8} model={:<8} air_dodge_time={:<5} -> {maneuver:?}",
            id,
            if authored.is_some() {
                "its own"
            } else {
                "nothing"
            },
            if axis { "axis" } else { "momentum" },
            body.air_dodge_time,
        ));
        if maneuver != ae::BurstManeuver::AirDodge {
            shut.push(id.to_string());
        }
    }

    assert!(
        report.len() >= 8,
        "only {} fighters resolved against this composition — the host is not \
         composing the cast and this test is about to prove nothing",
        report.len()
    );
    assert!(
        composed_over_an_authored_body,
        "no fighter on the grid brought a body of its own, so the composition \
         was never exercised and a wholesale replacement would pass this:\n{}",
        report.join("\n")
    );
    let unexpected: Vec<&String> = shut
        .iter()
        .filter(|id| !NOT_AN_AXIS_BODY.contains(&id.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "{} fighter(s) are GRANTED the dodge and cannot use it in the air:\n{}\n\n\
         A granted verb whose window is zero is a dead grant — see \
         `apply_smash_match_rules`, which is where the stage supplies the body \
         those verbs run on.",
        unexpected.len(),
        report.join("\n")
    );
    // THE OTHER DIRECTION, so the list cannot quietly outlive what it names.
    let cured: Vec<&&str> = NOT_AN_AXIS_BODY
        .iter()
        .filter(|id| !shut.iter().any(|shut| shut == *id) && grid.ids().any(|on| on == **id))
        .collect();
    assert!(
        cured.is_empty(),
        "{cured:?} can air-dodge now and is still listed as moving by a model \
         that has no evade window — the ratchet only turns one way, so take it \
         off the list"
    );
}

/// EVERY FIGHTER'S GROWTH IS IN THE STAGE'S UNITS — not just the stand-ins'.
///
/// the guard for this existed and swept the wrong population.
/// `ambition_demo_smash:moveset`'s
/// `an_authored_growth_is_the_stage_declaration_in_the_stage_units` iterates `fighter_moveset`,
/// which is the ELEVEN-VERB FALLBACK the two robot stand-ins carry.
///
/// What it looks for is a UNIT SLIP, and the slip is expensive: a volume's
/// `knockback_growth` is absolute px/s per point, while the ruleset's is a
/// FRACTION of the move's base. Both are `f32`, both are called "growth", and an
/// authored move outranks the ruleset — so fraction-shaped numbers make a move
/// grow ~40× slower than the stage declared while every test stays green. That
/// is twice now that it surfaced as *"there does not seem to be any knockback"*.
///
/// a BAND, not an equality, and the original guard's own doc says why: *"A move MAY
/// deliberately differ — that is what authoring is for — but it has to differ by a factor a reader
/// can see, not by a unit."* Its `< 0.01` tolerance forbade the very latitude that sentence grants.
/// George Booul's table sits at 0.85–1.05× of the declaration (`130 → 2.20`, `50 → 1.05`), which is
/// a fighter being tuned; a unit slip is 40×.
#[test]
fn every_fighters_growth_is_a_tuning_choice_and_never_a_unit_slip() {
    /// How far from the declaration an authored growth may sit and still be
    /// read as a deliberate choice. Four is far wider than any fighter needs
    /// (the widest today is 1.05) and far tighter than the ~40 a unit slip
    /// produces, so it discriminates the two without policing taste.
    const MAX_TUNING_FACTOR: f32 = 4.0;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let registry =
        app.world()
            .resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>();
    let grid = SmashRoster::assemble(registry);
    let declared = ambition_demo_smash::SMASH_KNOCKBACK_GROWTH;

    let mut volumes_seen = 0usize;
    let mut fighters_seen = 0usize;
    let mut offenders: Vec<String> = Vec::new();

    for id in grid.ids() {
        let Some(prepared) = registry.get(id) else {
            continue;
        };
        let Some(moveset) = prepared.kit.projectable_moveset() else {
            continue;
        };
        let mut this_fighter = 0usize;
        for mv in &moveset.moves {
            for volume in mv.windows.iter().flat_map(|w| w.volumes.iter()) {
                // A volume that launches nothing has no base to grow from, so
                // the ratio is undefined rather than wrong.
                if volume.knockback <= 0.0 {
                    continue;
                }
                // Unauthored growth defers to the ruleset's `base *
                // ruleset_growth`, and a stated zero is FIXED knockback — a
                // deliberate choice, not a unit slip. Every fighter carries
                // seven prefab-derived swings (`attack`, `attack_up`, the
                // aerials) that author no growth at all, and flagging those
                // would have made this guard fire on the whole grid for the one
                // case that needs no fixing. The slip this hunts is an AUTHORED
                // NON-ZERO number in the wrong unit.
                let Some(authored) = volume.knockback_growth.filter(|g| *g > 0.0) else {
                    continue;
                };
                this_fighter += 1;
                let expected = volume.knockback * declared;
                let ratio = authored / expected;
                if ratio < 1.0 / MAX_TUNING_FACTOR || ratio > MAX_TUNING_FACTOR {
                    offenders.push(format!(
                        "{id}/{} launches at {} and grows {}/point, but the stage \
                         declares {declared} of base = {expected}/point ({ratio:.3}× off)",
                        mv.id, volume.knockback, authored,
                    ));
                }
            }
        }
        if this_fighter > 0 {
            fighters_seen += 1;
            volumes_seen += this_fighter;
        }
    }

    // the non-vacuity half, and it is the point of widening the sweep. The
    // guard this replaces passed for years while looking at one table nobody
    // fights with. A sweep that reached no fighter, or one fighter, would pass
    // here for exactly the same reason.
    assert!(
        fighters_seen >= 10,
        "the growth sweep reached only {fighters_seen} fighters with launching \
         volumes — it is measuring a population, and this one is too small to be \
         the grid"
    );
    assert!(
        volumes_seen >= 100,
        "the growth sweep examined {volumes_seen} launching volumes across the \
         whole grid, which is fewer than one fighter's kit"
    );
    assert!(
        offenders.is_empty(),
        "a growth that is off by a FACTOR is the fraction-vs-absolute unit slip, \
         and it silently opts the move out of the percent loop:\n  {}",
        offenders.join("\n  ")
    );
}
