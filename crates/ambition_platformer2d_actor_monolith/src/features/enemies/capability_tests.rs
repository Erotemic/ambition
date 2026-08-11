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

    let shark = crate::features::enemies::test_spec("burning_flying_shark").combat_capabilities();
    assert!(shark.charge_crash_explodes);

    let infinite = crate::features::enemies::test_spec("sandbag_infinite");
    assert!(infinite.never_dies);
    assert!(
        !matches!(
            infinite.respawn,
            ambition_entity_catalog::placements::RespawnPolicy::InPlace(_)
        ),
        "infinite sandbag never dies; it needs no revive timer"
    );

    let finite = crate::features::enemies::test_spec("sandbag_finite");
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
    let armed = crate::features::enemies::test_spec("pirate_shark_rider").combat_capabilities();
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

/// The PROTAGONIST as an actor body (roadmap S6a / invariant I7): the
/// `player_robot` archetype carries the FULL player kit as body movement
/// capabilities — blink / fly / shield / dash all appear in its
/// [`ae::AbilitySet`] movement kit, and it has both a melee strike and the
/// player's Hadouken ranged. This is what makes the player-robot droppable as a
/// boss and fieldable as the spectator-arena's second combatant. (Authoring
/// this is what forces the player kit to BE the body's `AbilitySet` — the same
/// vocabulary the live player runs — not a parallel enemy-only mirror; the live
/// player folds onto this same actor path in S6b.)
#[test]
fn player_robot_archetype_carries_the_full_player_kit() {
    let spec = crate::features::enemies::test_spec("player_robot");
    let kit = spec.movement_kit();
    assert!(
        kit.blink && kit.fly && kit.shield && kit.dash,
        "the player-robot body has the full movement kit in its AbilitySet: {kit:?}",
    );
    assert!(spec.melee.is_some(), "player-robot has a melee strike");
    assert!(
        spec.ranged.is_some(),
        "player-robot has the Hadouken ranged verb"
    );
    assert_eq!(
        spec.ranged_visual, "hadouken",
        "the player-robot fires the player's signature projectile",
    );
    assert_eq!(
        spec.brain_template,
        super::CharacterBrainTemplate::Smash,
        "the player-robot is driven by the unified Smash brain (the strong brain)",
    );
    // Its authored `movement` patch resolves to the PLAYER's snappier physics
    // (enemies rise to the player) — proving the per-archetype tuning data flows
    // RON patch -> hierarchy resolution -> the runtime `ActorTuning`.
    let movement = spec.tuning().movement;
    assert_eq!(
        movement.gravity, 2250.0,
        "player-robot falls like the player"
    );
    assert_eq!(
        movement.jump_speed, 630.0,
        "player-robot jumps like the player"
    );
    assert_ne!(
        movement,
        crate::combat::BodyMovementTuning::BASELINE,
        "the authored override differs from the generic baseline",
    );
}

/// The Stochastic Parrot's DUAL nature, proven from the authored data:
///   - the friendly cove bird is a catalog character (`stochastic_parrot`,
///     peaceful) — its sprite binds by `character_id`;
///   - the aggressive sky raiders are the `sky_parrot` enemy archetype —
///     hostile + aerial, reusing the charge-crash dive brain;
///   - both wear the SAME parrot sprite. The aggressive form binds by
///     DISPLAY NAME, so this pins that the enemy's authored spawn name
///     ("Stochastic Parrot", set on the sky `EnemySpawn`s) exactly equals
///     the catalog `display_name` — the fragile string join in P2 of the
///     content-authoring pain-points journal. If someone renames either
///     side, the sky parrots silently lose their sprite; this test screams.
#[test]
fn stochastic_parrot_is_friendly_in_the_cove_and_hostile_in_the_sky() {
    use super::CharacterBrainTemplate;

    // Aggressive sky form.
    let sky = test_spec("sky_parrot");
    assert!(sky.attacks_player, "sky_parrot is hostile by default");
    assert_eq!(
        sky.is_aerial,
        Some(true),
        "sky_parrot flies (aerial, no gravity), and says so EXPLICITLY — `None` \
         would mean it never stated an answer"
    );
    assert!(sky.melee.is_some(), "sky_parrot has a dive/peck melee");
    assert_eq!(
        sky.brain_template,
        CharacterBrainTemplate::Aerial,
        "sky_parrot uses the aerial dive-bomber brain",
    );

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

        let body = !matches!(
            key,
            "sandbag_infinite"
                | "sandbag_finite"
                | "pirate_shark_rider"
                | "pirate_heavy_shark_rider"
        ) && (attacks || key == "puppy_slug");
        assert_eq!(spec.body_contact_damage, body, "{key} body_contact");

        // ADR 0022: the enum is AUTHORED per row now. Mini-boss presences
        // rest-gate; sandbags revive in place; every other roster row is an
        // explicit OnRoomReenter mob (the Q29 triage) — the DeadStaysDead
        // default is for unique placements (NPCs pin it at spawn).
        let policy = if matches!(
            key,
            "large_brute"
                | "large_colossus"
                | "pirate_heavy"
                | "pirate_shark_rider"
                | "pirate_heavy_shark_rider"
        ) {
            RespawnPolicy::OnRest
        } else if key == "sandbag_finite" {
            RespawnPolicy::InPlace(0.85)
        } else if key == "sandbag_infinite" {
            RespawnPolicy::DeadStaysDead // never_dies; policy is moot
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
