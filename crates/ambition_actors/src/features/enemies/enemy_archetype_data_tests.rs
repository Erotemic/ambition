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
    let mut by_brain = std::collections::HashMap::new();
    by_brain.insert("pirate_heavy".to_string(), test_spec("pirate_heavy"));
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

/// The fixture roster must carry a row for every authored spawn brain key
/// (a missing row would resolve to the `combatant` fallback rather than
/// the intended enemy).
#[test]
fn ron_carries_every_known_brain_key() {
    for key in ALL_BRAIN_KEYS {
        assert!(
            test_roster().contains_brain(key),
            "character_archetypes.ron missing row for brain key '{key}'",
        );
    }
}

/// Phase-0 authoring proof (fable review §A1, Path B): the PCA
/// (`cellular_automaton_fighter`) authors a data-driven signature MOVE on its
/// archetype — a normal actor carrying a boss-grade move as DATA. Guards that
/// the `character_archetypes.ron` moveset deserializes into a well-formed
/// `MovesetContract`: the `special` verb resolves the "cellular_pulse" move, and
/// that move has an Active window with a hit volume (so it lands damage through
/// the shared moveset runtime). A regen or a schema drift that dropped the move
/// trips here.
#[test]
fn pca_fighter_authors_a_data_driven_signature_move() {
    use ambition_entity_catalog::WindowTag;
    let pca = test_spec("cellular_automaton_fighter");
    let moveset = pca
        .signature_move
        .as_ref()
        .expect("the PCA authors a signature move on its archetype");
    let mv = moveset
        .move_for_verb("special")
        .expect("the `special` verb resolves a move");
    assert_eq!(mv.id, "cellular_pulse");
    assert!(mv.duration_s > 0.0, "the move has a positive timeline");
    assert!(
        mv.windows
            .iter()
            .any(|w| { matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty() }),
        "the Cellular Pulse has an Active window carrying a hit volume"
    );
    // Most archetypes carry NO moveset — the field is opt-in data.
    assert!(
        test_spec("combatant").signature_move.is_none(),
        "a plain archetype authors no signature move"
    );
}

/// The moveset generalizes beyond the PCA: the PROTAGONIST's body archetype
/// (`player_robot`, invariant I7) authors a data-driven signature move too — a
/// TWO-HIT combo ("Theorem Chain"), proving the system expresses smash-like
/// multi-hit moves as data across characters, not a PCA one-off (fable review §A1).
#[test]
fn player_robot_authors_a_multi_hit_signature_combo() {
    use ambition_entity_catalog::WindowTag;
    let robot = test_spec("player_robot");
    let mv = robot
        .signature_move
        .as_ref()
        .and_then(|m| m.move_for_verb("special"))
        .expect("the player-robot authors a `special`-verb signature move");
    assert_eq!(mv.id, "theorem_chain");
    let active_windows = mv
        .windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty())
        .count();
    assert_eq!(
        active_windows, 2,
        "Theorem Chain is a two-hit combo (two Active windows with volumes)"
    );
}

/// Spot-check the legacy pre-data values for two divergent
/// archetypes so a regen of the RON without re-tuning catches
/// accidental drift on the rows the player notices first.
#[test]
fn legacy_baseline_pins() {
    use ambition_characters::brain::MeleeActionSpec;
    let combatant = test_spec("combatant");
    assert_eq!(combatant.max_health, 4);
    assert!((combatant.chase_speed - 155.0).abs() < f32::EPSILON);
    assert!((combatant.aggro_radius - 460.0).abs() < f32::EPSILON);
    assert!(
        matches!(combatant.melee, Some(MeleeActionSpec::Swipe(_))),
        "Combatant melee should be Swipe; got {:?}",
        combatant.melee
    );
    let slug = test_spec("puppy_slug");
    assert_eq!(slug.max_health, 2);
    assert!((slug.patrol_speed - 55.0).abs() < f32::EPSILON);
    assert_eq!(slug.aggro_radius, 0.0);
    assert_eq!(slug.brain_template, CharacterBrainTemplate::Wanderer);
    assert!(slug.melee.is_none());
    assert!(slug.ranged.is_none());
}

/// The two gun-sword archetypes reference their weapon by id in the
/// RON; guard that the id resolves against the held-item registry
/// (a typo would silently drop the weapon, leaving them unarmed) and
/// that the resolved Bolt damage matches the authored per-archetype
/// scaling.
#[test]
fn gun_sword_archetypes_resolve_held_item_by_id() {
    use ambition_characters::brain::{action_set::RangedStyle, RangedActionSpec};
    let on_shark = test_spec("pirate_shark_rider")
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
    let heavy = test_spec("pirate_heavy_shark_rider")
        .held_item_spec()
        .expect("pirate_heavy_shark_rider should resolve a held item");
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
    assert_eq!(
        crate::features::enemies::test_spec("small_skitter").smash_hit_band,
        Some(32.0)
    );
    assert_eq!(
        crate::features::enemies::test_spec("small_lurker").smash_hit_band,
        Some(32.0)
    );
    assert_eq!(
        crate::features::enemies::test_spec("large_brute").smash_hit_band,
        Some(48.0)
    );
    assert_eq!(
        crate::features::enemies::test_spec("large_colossus").smash_hit_band,
        Some(48.0)
    );
    // 36px-default Smash archetypes omit the field on purpose.
    assert_eq!(
        crate::features::enemies::test_spec("combatant").smash_hit_band,
        None
    );
    assert_eq!(
        crate::features::enemies::test_spec("gradient_seeker").smash_hit_band,
        None
    );
    assert_eq!(
        crate::features::enemies::test_spec("pirate_raider").smash_hit_band,
        None
    );
}

#[test]
fn body_contact_damage_is_explicitly_opted_in() {
    assert!(crate::features::enemies::test_spec("combatant").body_contact_damage);
    assert!(crate::features::enemies::test_spec("puppy_slug").body_contact_damage);
    assert!(!crate::features::enemies::test_spec("pirate_heavy").body_contact_damage);
    assert!(!crate::features::enemies::test_spec("pirate_shark_rider").body_contact_damage);
    assert!(!crate::features::enemies::test_spec("sandbag_finite").body_contact_damage);
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
    let spec = test_spec("pirate_heavy");
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
    let attack_range = spec.tuning().attack_range;
    assert!(
        attack_range <= reach_edge,
        "PirateHeavy attack_range {attack_range} must stay within her swing far \
         edge {reach_edge} so she stops inside her own reach instead of whiffing",
    );
}
