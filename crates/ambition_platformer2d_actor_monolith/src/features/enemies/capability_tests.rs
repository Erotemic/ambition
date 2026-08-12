//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod capability_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module with
//! private access via `use super::*;` (a direct sibling, so `super` depth is
//! unchanged).

use super::{test_spec, ALL_BRAIN_KEYS};
use crate::features::enemies::ArchetypeSpecExt;

/// Pin the authored capability rows in `character_archetypes.ron` to the
/// behavior the actor layer used to hardcode by archetype identity
/// (Stage 20: the named checks became data-driven capabilities).
#[test]
fn archetype_capabilities_match_the_legacy_identity_checks() {
    // ⭐ **THE MITES ARE GONE FROM THIS TEST BECAUSE THEY ARE GONE FROM THE
    // ROSTER** (D73 phase 2, group A, 2026-08-10). `explodes_on_death` and
    // `divides_on_death` are authored on `npc_exploding_mite` /
    // `npc_dividing_mite` as CHARACTERS now and deleted from their archetype
    // rows, so asserting them here would assert the authority they left.
    //
    // ⛔ **the coverage did not drop, it MOVED**: `ambition_content`'s
    // `the_migrated_mites_author_their_own_death_and_health` pins the same two
    // facts where they now live, and this row's remaining assertions still
    // guard every trait that has NOT migrated.
    let mite = crate::features::enemies::test_spec("exploding_mite").combat_capabilities();
    assert_eq!(
        mite,
        Default::default(),
        "the mite's archetype must state NOTHING about death now — a trait \
         surviving here is the same fact in two authorities, which is what the \
         migration exists to end"
    );

    // ⚠ the shark's row is DELETED (D73 group A, 2026-08-11): its
    // `charge_crash_explodes` is authored on `npc_burning_flying_shark` and
    // pinned beside that definition. The engine's own rideable fixture carries
    // the same trait, so what this asserts is the PROJECTION — that a row
    // stating it produces a capability — rather than any game's shark.
    let shark = crate::features::enemies::fixture_roster_with_mount()
        .archetype_for("fixture_mount")
        .expect("the engine's rideable fixture")
        .combat_capabilities();
    assert!(shark.charge_crash_explodes);

    let infinite = crate::features::enemies::fixture_spec("sandbag_infinite");
    assert!(infinite.never_dies);
    assert!(
        !matches!(
            infinite.respawn,
            ambition_entity_catalog::placements::RespawnPolicy::InPlace(_)
        ),
        "infinite sandbag never dies; it needs no revive timer"
    );

    let finite = crate::features::enemies::fixture_spec("fixture_in_place_respawner");
    assert!(!finite.never_dies);
    assert_eq!(
        finite.tuning().respawn,
        ambition_entity_catalog::placements::RespawnPolicy::InPlace(0.85),
        "finite sandbag revives in place (the InPlace arm of ADR 0022)"
    );

    // A plain combatant has no special capabilities.
    let base = crate::features::enemies::test_spec("combatant").combat_capabilities();
    assert_eq!(base, Default::default());
}

/// **The archetype says WHETHER a corpse drops its weapon, never WHICH one.**
///
/// `CombatCapabilities::drops_held_item` was `Option<HeldItemSpec>` and was
/// populated from the archetype's INTRINSIC weapon, snapshotted at construction
/// — so a body that picked up something else still dropped what it was authored
/// with. It is a `bool` now and the death path reads the body's live `HeldItem`,
/// which is what `ambition_combat::held_items`' module doc always said it was
/// for.
///
/// ⛔ this pins the PROJECTION, which is the half that can silently invert:
/// the behavioural half is structural now, because a `bool` cannot name an item
/// and the old bug is therefore unrepresentable.
#[test]
fn an_archetype_with_an_intrinsic_weapon_drops_one_and_says_nothing_about_which() {
    let armed = crate::features::enemies::fixture_spec("fixture_armed_rider").combat_capabilities();
    assert!(
        armed.drops_held_item,
        "the cove raider authors a gun_sword, so its corpse leaves a weapon"
    );

    let unarmed = crate::features::enemies::test_spec("combatant").combat_capabilities();
    assert!(
        !unarmed.drops_held_item,
        "an archetype with no intrinsic weapon drops none — otherwise every \
         empty-handed body would try to leave a corpse item"
    );
}

// ⛔ **`player_robot_archetype_carries_the_full_player_kit` was deleted here on
// 2026-08-11 with the row it tested** (ledger D83). Its three claims are now the
// CHARACTER's, and asserted where they live: the movement kit by
// `player_robot_moveset::the_robot_authors_its_verbs_rather_than_taking_a_match_s_word_for_them`,
// the melee and ranged verbs by the `robot_duelist_kit` catalog preset, and the
// Hadouken by `the_robot_states_what_its_projectile_looks_like`.

/// The Stochastic Parrot's DUAL nature — ONE character, two dispositions.
///
/// ⭐ **rewritten 2026-08-11, and what it lost is the achievement.** It used to
/// pin a fragile string: the cove bird was a catalog character and the sky
/// raiders were a separate `sky_parrot` ARCHETYPE whose sprite bound by DISPLAY
/// NAME, so this test existed mostly to scream if either side was renamed. The
/// archetype is deleted. The sky placements name `character_id:
/// stochastic_parrot`, so both forms are the same character and the join they
/// depended on is not fragile — it is not a join at all.
///
/// What remains here is the CATALOG half: the display name (still the art
/// fallback for anything unmigrated), the sheet, and the peaceful default brain.
/// The parrot's authored BODY is pinned beside the definition that states it,
/// in `ambition_content`.
#[test]
fn stochastic_parrot_is_friendly_in_the_cove_and_hostile_in_the_sky() {
    // Friendly cove form: a catalog character with a peaceful default.
    let catalog = crate::character_roster::catalog();
    let display = catalog.display_name("stochastic_parrot");
    assert_eq!(
        display,
        Some("Stochastic Parrot"),
        "the catalog display_name MUST equal the sky EnemySpawn name, or the \
         aggressive parrot loses its sprite (P2 name-join)",
    );
    // Both forms wear the same parrot sheet (the friendly form binds it by
    // character_id; the sheet must actually resolve).
    assert!(
        crate::character_sprites::sheet_for_character_id_in(
            &Default::default(),
            &catalog,
            "stochastic_parrot"
        )
        .is_some(),
        "the parrot catalog row must resolve a sprite sheet",
    );

    // Friendly form is authored ENTIRELY in data as a lively flyer (the
    // commit-3 refactor payoff): the catalog default_brain resolves to a
    // PEACEFUL Aerial brain, and body_kind is Floating so it's gravity-free.
    let friendly = catalog
        .build_default_brain("stochastic_parrot", 0.0)
        .expect("parrot has a catalog default brain");
    assert!(
        matches!(
            friendly,
            ambition_characters::brain::Brain::StateMachine(ambition_characters::brain::StateMachineCfg::Aerial {
                cfg,
                ..
            }) if cfg.aggressiveness == 0.0
        ),
        "the cove parrot is authored as a peaceful Aerial flyer in data",
    );
    assert_eq!(
        catalog.body_kind("stochastic_parrot"),
        Some(ambition_characters::actor::character_catalog::CharacterBodyKind::Floating),
        "the cove parrot is Floating (gravity-free) so the Aerial brain flies it",
    );
}

/// Parity net for the Session-6/7 data migration: the four behaviors
/// that used to be hardcoded `match self { … }` arms on the enum are now
/// authored RON fields (`attacks_player`, `body_contact_damage`,
/// `respawn_on_rest`, the smash/provoke flags). Re-encode the OLD
/// identity formulas here as the oracle and assert every archetype's
/// RON row reproduces them — replay only exercises the archetypes in the
/// fixture, so this guards the exotic rows (sandbags, mites, composites)
/// against a silent mis-migration.
#[test]
fn ron_derived_behaviors_match_the_legacy_identity_formulas() {
    use super::RespawnPolicy;
    for &key in ALL_BRAIN_KEYS {
        let spec = test_spec(key);
        let attacks = !matches!(key, "puppy_slug" | "pirate_heavy");
        assert_eq!(spec.attacks_player, attacks, "{key} attacks_player");

        // ⚠ the `sandbag_infinite` arm left with its shipped row (2026-08-12):
        // the lab's dummies name a character now, and the row this loop iterates
        // is gone. The immortal shape is still exercised, against the fixture's
        // own row, by `infinite_sandbag_never_dies` above.
        let body = attacks || key == "puppy_slug";
        assert_eq!(spec.body_contact_damage, body, "{key} body_contact");

        // ADR 0022: the enum is AUTHORED per row now. Mini-boss presences
        // rest-gate; every other roster row is an explicit OnRoomReenter mob
        // (the Q29 triage) — the DeadStaysDead default is for unique placements
        // (NPCs pin it at spawn).
        //
        // ⚠ **the `InPlace` arm left with `sandbag_finite`** (2026-08-11): that
        // dummy is a character now and its three placements author their own
        // respawn, which is where a respawn policy belongs. The POLICY is still
        // exercised — against `fixture_in_place_respawner`, a row this crate
        // owns, by `finite_sandbag_revives_in_place` above.
        let policy = if matches!(key, "large_brute" | "large_colossus" | "pirate_heavy") {
            RespawnPolicy::OnRest
        } else {
            RespawnPolicy::OnRoomReenter
        };
        assert_eq!(spec.respawn, policy, "{key} respawn policy");

        let bs = spec.brain_profile();
        assert_eq!(
            bs.smash_heavy,
            matches!(key, "large_brute" | "large_colossus"),
            "{key} smash_heavy"
        );
        assert_eq!(
            bs.smash_dash_to_close,
            key == "medium_striker",
            "{key} smash_dash_to_close"
        );
        assert_eq!(
            bs.provoke_forced_brute_min_aggro,
            if key == "pirate_heavy" {
                Some(500.0)
            } else {
                None
            },
            "{key} provoke_forced_brute_min_aggro"
        );
    }
}
