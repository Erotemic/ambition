//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod enemy_archetype_data_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module with
//! private access via `use super::*;` (a direct sibling, so `super` depth is
//! unchanged).

use super::integration::enemy_attack_aabb_dir;
use super::*;

/// The App-local [`CharacterRoster`] holder resolves a known brain key to
/// its spec and falls back for an unknown / non-`Custom` brain, and the
/// lib's embedded default reproduces `from_brain` exactly (the
/// replay-identity guarantee for the resolution inversion). Built
/// locally so the test controls its exact authority.
#[test]
fn enemy_roster_resolves_brain_keys_with_fallback() {
    use ambition_entity_catalog::placements::CharacterBrain;
    let mut by_brain = std::collections::BTreeMap::new();
    by_brain.insert("pirate_heavy".to_string(), fixture_spec("pirate_heavy"));
    let roster = CharacterRoster::new(by_brain, test_spec("combatant"));
    // Known key → its spec (PirateHeavy is peaceful by default).
    assert!(
        !roster
            .spec_for_brain(&CharacterBrain::Custom("pirate_heavy".into()))
            .attacks_player
    );
    // Unknown key + non-Custom → fallback (Combatant is hostile).
    assert!(
        roster
            .spec_for_brain(&CharacterBrain::Custom("does_not_exist".into()))
            .attacks_player
    );
}

/// The roster a test resolves against must carry a row for every brain key a
/// test names — a missing row resolves to the `combatant` fallback rather than
/// failing, so the test changes subject instead of going red.
///
/// ⚠ **asked of the ENGINE's roster now**, which is the shipped file plus the
/// rows the engine owns. `medium_striker` moved into the fixture on 2026-08-12
/// (see `fixture_roster_with_mount`): no world names it as a spawn brain key and
/// the goblin authors its own repertoire, so the only thing still needing the
/// row is a handful of tests about the archetype machinery.
#[test]
fn the_fixture_roster_carries_every_brain_key_a_test_names() {
    for key in ALL_BRAIN_KEYS {
        assert!(
            crate::features::enemies::fixture_roster_with_mount().contains_brain(key),
            "no fixture row for brain key '{key}' — `spec_for_brain` will answer \
             `combatant` and every test naming it will quietly measure that",
        );
    }
}

/// **WHAT IS LEFT OF THE SHIPPED ARCHETYPE FILE** — D73's acceptance signal is
/// that it be DELETED, and this is the countdown.
///
/// ⭐ two rows became one on 2026-08-12. `combatant` is the only survivor, and
/// it is not a creature anybody places: `ambition_combat`'s content schema
/// reserves it as `FALLBACK_BRAIN_KEY`, the answer to *what does an unknown
/// brain key resolve to*. Deleting the row means answering that question —
/// GPT 5.6's redirect says it should become a construction ERROR, the same rule
/// P0.1 established for an absent `CharacterId` (ledger D102).
///
/// ⛔ **a countdown, and it fails for the reason the campaign is succeeding.**
/// When the last row goes this test goes with the file; until then it is the one
/// place the number is written down where a reader will see it move.
#[test]
fn the_shipped_archetype_file_is_down_to_the_reserved_fallback() {
    let shipped = test_roster();
    assert!(
        shipped.contains_brain("combatant"),
        "the reserved fallback row is gone but `FALLBACK_BRAIN_KEY` still names \
         it — the schema check would have caught this, and so does this"
    );
    for key in [
        "medium_striker",
        "cellular_automaton_fighter",
        "sandbag_infinite",
    ] {
        assert!(
            !shipped.contains_brain(key),
            "`{key}` is back in the SHIPPED archetype file. Every row that left \
             did so because a character took its facts; a row reappearing is the \
             old ontology growing back, whatever the commit says it is doing"
        );
    }
}

/// ⛔⛔ **DELETED 2026-08-11 (ledger D89): `pca_fighter_authors_a_data_driven_
/// signature_move`.** It proved that `character_archetypes.ron`'s
/// `cellular_automaton_fighter` row carried "Cellular Pulse" as an inline
/// `signature_move` — the first data-driven move in the repository, and a good
/// proof at the time.
///
/// **That row is deleted.** The pulse is a real `MovesetContract` on the PCA's
/// character definition now (`ambition_content::cellular_automaton_moveset`),
/// with the same 0.40s tell, 0.14s active window and `pca.cellular_pulse` cue —
/// and a test in that crate asserts it there, which is where a character's moves
/// belong. A test of a table entry that no longer exists cannot fail usefully.

// ⛔ **`player_robot_authors_a_multi_hit_signature_combo` was deleted here on
// 2026-08-11 with the row it tested** (ledger D83). The claim did not go away —
// two Active windows on one timeline is still the proof that a moveset expresses
// a combo as DATA — it moved to the character that makes it:
// `ambition_content::player_robot_moveset::the_theorem_chain_is_two_hits_on_one_timeline`.

/// Spot-check the legacy pre-data values for two divergent
/// archetypes so a regen of the RON without re-tuning catches
/// accidental drift on the rows the player notices first.
#[test]
fn legacy_baseline_pins() {
    use ambition_characters::brain::MeleeActionSpec;
    let combatant = test_spec("combatant");
    assert_eq!(combatant.max_health, 4);
    // Read through `tuning()`, which is where the authored effort becomes the
    // px/s gameplay consumes. The number is unchanged across the C1 migration —
    // that is the point of pinning it here rather than pinning the fraction.
    assert!((combatant.tuning().chase_speed - 155.0).abs() < f32::EPSILON);
    assert!((combatant.aggro_radius - 460.0).abs() < f32::EPSILON);
    assert!(
        matches!(combatant.melee, Some(MeleeActionSpec::Swipe(_))),
        "Combatant melee should be Swipe; got {:?}",
        combatant.melee
    );
    // ⭐ **THE PUPPY SLUG'S PINS LEFT THIS TEST ON 2026-08-11, and where they
    // went is the point.** It had six assertions here — 2 HP, 55px/s patrol,
    // zero aggro, Wanderer, no melee, no ranged — every one of them read off an
    // ARCHETYPE row. That row is deleted: the slug authors all six on its
    // character definition (`ambition_content::character_catalog`), and its ten
    // placements author the disposition that made it ambient wildlife.
    //
    // ⛔ they were not dropped, they MOVED: `ambition_content` pins them beside
    // the definition that states them. Leaving them here would have been worse
    // than deleting them — `test_spec` answers an unknown key with `combatant`,
    // so these six assertions would have gone on passing about the wrong
    // creature until one of the numbers happened to differ.
}

/// The two gun-sword archetypes reference their weapon by id in the
/// RON; guard that the id resolves against the held-item registry
/// (a typo would silently drop the weapon, leaving them unarmed) and
/// that the resolved Bolt damage matches the authored per-archetype
/// scaling.
#[test]
fn gun_sword_archetypes_resolve_held_item_by_id() {
    use ambition_characters::brain::{action_set::RangedStyle, RangedActionSpec};
    let on_shark = fixture_spec("fixture_armed_rider")
        .held_item_spec()
        .expect("pirate_shark_rider should resolve a held item");
    assert_eq!(on_shark.id, "gun_sword");
    assert!(matches!(
        on_shark.ranged,
        Some(RangedActionSpec {
            style: RangedStyle::Bolt,
            damage: 2,
            ..
        })
    ));
    let heavy = fixture_spec("fixture_armed_rider_heavy")
        .held_item_spec()
        .expect("the heavy armed rider should resolve a held item");
    assert_eq!(heavy.id, "gun_sword_heavy");
    assert!(matches!(
        heavy.ranged,
        Some(RangedActionSpec {
            style: RangedStyle::Bolt,
            damage: 3,
            ..
        })
    ));
}

/// The Smash melee hit band is now authored per-archetype in the RON
/// (CharacterAI migration #194). Guard the values that drove the old
/// `smash_cfg_for_archetype` match arms so a RON re-tune can't silently
/// resize the goblin/brute hit bands, and confirm the 36px-default
/// archetypes correctly omit the field (fall through to the builder
/// fallback).
#[test]
fn smash_hit_band_is_data_authored() {
    assert_eq!(
        crate::features::enemies::test_spec("medium_striker").smash_hit_band,
        Some(32.0)
    );
    // 36px-default Smash archetypes omit the field on purpose.
    assert_eq!(
        crate::features::enemies::test_spec("combatant").smash_hit_band,
        None
    );
    assert_eq!(
        crate::features::enemies::fixture_spec("pirate_raider").smash_hit_band,
        None
    );
}

#[test]
fn body_contact_damage_is_explicitly_opted_in() {
    assert!(crate::features::enemies::test_spec("combatant").body_contact_damage);
    // ⛔ the second positive named `puppy_slug`, whose shipped row is gone — so it
    // resolved `combatant` and repeated the line above while appearing to add a
    // subject (ledger D94). A fixture row the engine owns is a real second case.
    assert!(
        crate::features::enemies::fixture_spec("cellular_automaton_fighter").body_contact_damage
    );
    assert!(!crate::features::enemies::fixture_spec("pirate_heavy").body_contact_damage);
    assert!(!crate::features::enemies::fixture_spec("fixture_armed_rider").body_contact_damage);
    assert!(!crate::features::enemies::fixture_spec("sandbag_infinite").body_contact_damage);
}

/// Regression for the cove bug "an aggressive PirateHeavy never gets
/// close enough to land a hit." `attack_range` is the
/// stop-and-swing distance read by `evaluate_character_ai_output`;
/// her horizontal melee hitbox (`attack_aabb_dir`) only reaches
/// `size.x*0.55 + 24 + 34` px from her center. If `attack_range`
/// exceeds that far edge she halts out of reach and swings into
/// empty air. Pin that `attack_range` stays inside the swing reach
/// so the strike can actually overlap a player standing at the
/// stop distance.
#[test]
fn pirate_heavy_stops_within_her_melee_reach() {
    let spec = fixture_spec("pirate_heavy");
    let authored_aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(36.0, 55.0));
    let pos = authored_aabb.center();
    let size = spec
        .default_size
        .unwrap_or_else(|| authored_aabb.half_size() * 2.0);
    let hitbox = enemy_attack_aabb_dir(
        pos,
        size,
        1.0,
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, 1.0),
    );
    let reach_edge = hitbox.center().x + hitbox.half_size().x - pos.x;
    let attack_range = spec.brain_profile().attack_range;
    assert!(
        attack_range <= reach_edge,
        "PirateHeavy attack_range {attack_range} must stay within her swing far \
         edge {reach_edge} so she stops inside her own reach instead of whiffing",
    );
}

/// **The engine's fixture rows are DISTINGUISHABLE from the fallback.**
///
/// ⛔⛔ this is the guard that would have caught twenty tests measuring nothing.
/// `spec_for_brain` answers `combatant` for any key it does not know, so a test
/// naming a deleted row keeps passing — and the respawn-policy tests were the
/// proof: `combatant` also authors `OnRoomReenter`, so *"this archetype respawns
/// on room re-entry"* stayed green while its subject had ceased to exist.
///
/// ⭐ a fixture row earns its keep by being TELLABLE APART. Each one below is
/// pinned on a fact the fallback does not have, so if the row is ever lost the
/// failure names the row rather than leaving a suite quietly agreeing with
/// itself.
#[test]
fn every_engine_fixture_row_differs_from_the_combatant_fallback() {
    use crate::features::enemies::{fixture_spec, test_spec};

    let fallback = test_spec("combatant");
    for key in [
        "cellular_automaton_fighter",
        "sandbag_infinite",
        "fixture_mount",
    ] {
        let spec = fixture_spec(key);
        assert!(
            spec.max_health != fallback.max_health
                || spec.never_dies != fallback.never_dies
                || spec.mount_class.is_some(),
            "fixture row `{key}` is indistinguishable from the `combatant` \
             fallback, so every test naming it would pass with the row deleted"
        );
    }

    // ⚠ and the control: an id NOBODY authors must actually land on the fallback,
    // or the comparison above proves nothing about how the miss behaves.
    let missing = test_spec("no_such_archetype_anywhere");
    assert_eq!(missing.max_health, fallback.max_health);
}
