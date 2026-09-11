//! Preparation's own tests, in preparation's own crate.
//!
//!  these test PREPARATION — what a definition resolves to, what it reports,
//! what it refuses. The tests that drive an `App` through
//! `try_register_character` and `finalize` test COMPOSITION and stay in the
//! monolith beside the plugin that does it; splitting by what a test tests is
//! what kept `prepare_and_finalize_for_test` off the production surface.

use super::*;
use crate::prepared_fixtures::{mary_o, moveset_with, slash};
use ambition_binding::Namespace;
use ambition_entity_catalog::{HurtboxKeyframe, HurtboxTimeline, HurtboxVolume, VolumeShape};

/// §4.6: the cue vocabulary is DERIVED from the moves that emit it, never
/// hand-listed beside them, because a hand-maintained list drifts.
#[test]
fn cue_vocabulary_is_derived_from_the_moves_that_emit_it() {
    let prepared = prepare_and_finalize_for_test(mary_o(), &CharacterBindings::default());
    assert!(prepared.is_clean(), "{:?}", prepared.report.unresolved());

    // Both halves: the move's own event cue AND the hit volume's strike sound.
    // Missing the second is how a sword and a claw stop sounding different.
    assert_eq!(
        prepared.prepared.cue_dependencies().collect::<Vec<_>>(),
        vec!["mary_o.stomp", "mary_o.stomp.land"]
    );
    assert_eq!(
        prepared.prepared.vfx_dependencies().collect::<Vec<_>>(),
        vec!["slash_arc"]
    );
    // There is no setter for either: authoring one is not possible, which is the
    // only way a derived inventory stays derived.
}

/// The §7.6 contract the goal guard names: a misspelled cue is reported AT
/// PREPARATION, with the namespace, the declarer, and what was available.
#[test]
fn misspelled_cue_is_named_at_preparation() {
    let authorized = ["mary_o.stomp", "mary_o.stomp.land", "mary_o.jump"];
    let typo =
        CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo").with_moveset(moveset_with(
            &[("attack", "stomp")],
            // `mary_o.stmop` is the whole bug: it plays nothing, forever, silently.
            vec![slash("stomp", "mary_o.stmop", "mary_o.stomp.land")],
        ));

    let prepared = prepare_and_finalize_for_test(
        typo,
        &CharacterBindings::default().with_authorized_cues(authorized),
    );
    assert!(
        !prepared.is_clean(),
        "a cue no session authorizes must not prepare clean"
    );
    let unresolved = prepared.report.unresolved();
    assert_eq!(unresolved.len(), 1, "{unresolved:?}");
    let report = format!("{:?}", unresolved[0]);
    assert!(
        report.contains("mary_o.stmop"),
        "must name the id: {report}"
    );
    assert!(
        report.contains("sfx cue"),
        "must name the namespace: {report}"
    );
    assert!(
        report.contains("mary_o"),
        "must name the declarer: {report}"
    );

    // And the correctly spelled sibling still resolves, so this is a real check
    // and not "every cue fails".
    let good = prepare_and_finalize_for_test(
        mary_o(),
        &CharacterBindings::default().with_authorized_cues(authorized),
    );
    assert!(good.is_clean(), "{:?}", good.report.unresolved());
}

/// An unchecked namespace must not read as a checked one.
#[test]
fn without_an_authorized_cue_set_cues_are_reported_as_unchecked() {
    let prepared = prepare_and_finalize_for_test(mary_o(), &CharacterBindings::default());
    assert!(
        !prepared.checked.contains(&"sfx cue"),
        "with no authorized set supplied, cues were NOT checked — saying otherwise \
         is the 'we did not look' / 'we looked and it was fine' confusion"
    );
    assert!(
        prepared.checked.contains(&"move"),
        "moves are always checkable: they come from the definition itself"
    );
}

/// A verb pointing at a move that does not exist resolves to "this character has
/// no attack", which at runtime is indistinguishable from a peaceful character.
#[test]
fn a_verb_naming_an_undeclared_move_is_named_not_silently_peaceful() {
    let broken =
        CharacterDefinition::new("sanic", "Sanic", "sanic_demo").with_moveset(moveset_with(
            &[("attack", "spindash")],
            vec![slash("roll", "sanic.roll", "sanic.roll.hit")],
        ));
    let prepared = prepare_and_finalize_for_test(broken, &CharacterBindings::default());
    let report = format!("{:?}", prepared.report.unresolved());
    assert!(report.contains("spindash"), "{report}");
    assert!(report.contains("move"), "{report}");
    // And it names the move that WAS declared, so the fix is obvious.
    assert!(
        report.contains("roll"),
        "must offer what was available: {report}"
    );
}

/// A move-time hurtbox override naming a move that does not exist is dead data
/// nothing will ever sample.
#[test]
fn a_hurtbox_override_for_an_undeclared_move_is_named() {
    let mut moves = BTreeMap::new();
    moves.insert(
        "nonexistent_move".to_string(),
        HurtboxTimeline {
            keyframes: vec![HurtboxKeyframe {
                at_s: 0.0,
                volumes: vec![HurtboxVolume {
                    shape: VolumeShape::Rect {
                        offset: (0.0, 0.0),
                        half_extents: (4.0, 8.0),
                    },
                }],
            }],
        },
    );
    let def = mary_o().with_hurtboxes(HurtboxDoc {
        default: None,
        poses: BTreeMap::new(),
        moves,
    });
    let prepared = prepare_and_finalize_for_test(def, &CharacterBindings::default());
    let report = format!("{:?}", prepared.report.unresolved());
    assert!(report.contains("nonexistent_move"), "{report}");
}

/// A character definition may name default autonomous policy, but it does not
/// store the session's current controller binding.
#[test]
fn a_definition_carries_no_controller_binding() {
    let def = mary_o();
    // If a CURRENT-controller field is ever added, this stops compiling as
    // written and the reviewer has to justify it against §4.7.
    let CharacterDefinition {
        id: _,
        display_name: _,
        provider: _,
        lineage: _,
        sheet: _,
        portrait: _,
        voice: _,
        body: _,
        hurtboxes: _,
        vitals: _,
        // What the body does when it DIES — a property of the creature, and one
        // no controller changes. A possessed mite still splits.
        death_traits: _,
        moveset: _,
        // A CAPABILITY, not a controller binding, and the distinction is the
        // whole of §4.7: this says what the body can reach for, and says nothing
        // about who decides to reach. A human and a CPU wearing this character
        // get the identical action set — which is exactly why it belongs on the
        // definition and `default_brain` does not.
        action_set: _,
        // Also a capability, and the same §4.7 reasoning: how a body MOVES is a
        // fact about the body, not about who is steering it. A human and a CPU
        // wearing this character move identically.
        motion_model: _,
        movement_tuning: _,
        // The strongest case of the same rule: WHICH VERBS THIS BODY HAS. A
        // human and a CPU wearing this character can jump, dash, shield and
        // grab a ledge identically, because a capability belongs to the body.
        // What a controller decides is which of them to use, and what a RULESET
        // decides is which of them are legal in this match — a mask, never a
        // grant.
        abilities: _,
        // How fast this body runs and whether touching it hurts. Both are the
        // creature's, not the driver's: a possessed crawler crawls, and a mite
        // is dangerous to touch whoever is steering it.
        locomotion: _,
        contact_damage: _,
        //  a DEFAULT policy, which §4.7 permits and the rule authorised, and the reason this
        // test survives rather than being deleted. What it guards is that the CURRENT
        // controller is nowhere on this type: a character may say what it does when nobody is
        // driving it, and may not say who is driving it now.
        autonomous_profile: _,
        // The same authority by NAME instead of by value — a shared policy
        // several characters point at. Still a character fact: it says what this
        // creature does when nobody drives it, not who is driving it.
        autonomous_profile_ref: _,
        ranged_vfx: _,
        // The default is the ordinary moveset verb. Characters opt into alternate
        // ranged execution explicitly; absence of an override must not grant one.
        ranged_execution: _,
        provoked_profile_ref: _,
        // The weapon the creature carries — an intrinsic like the sheet, not a
        // controller fact: a possessed raider is still holding a gun-sword.
        // A training dummy is a fact about the creature, not about who is
        // driving it: nothing drives a sandbag.
        practice_target: _,
        held_item: _,
        // Presentation, like the sheet: what this creature LOOKS like is a
        // property of the creature, and no controller changes it.
        dream_seed: _,
        // What this body can be ridden as and what it can ride (ADR 0020) — a
        // capability of the creature, and one no controller changes: a possessed
        // shark is still a shark somebody can sit on.
        mount: _,
        //  the field that reads most like a controller fact and is not one,
        // so it is justified here rather than ignored. It says that two
        // AUTONOMOUS twins of this character begin on one deterministic
        // cognitive stream — which is a fact about the creature, in the same
        // family as `autonomous_profile` above: what this character is like when
        // a driver is not a person. It names no driver, and a HUMAN wearing this
        // character is wholly unaffected by it, which is the §4.7 test.
        //
        //  it is deliberately NOT on `BrainProfile`. A profile is *reusable
        // across characters* by construction, so authoring it there would hand
        // the trait to whichever other characters happen to share the policy —
        // and this is Emmy's identity, not a difficulty rung's.
        preserves_mirror_symmetry: _,
    } = def;
}

/// A12. Sheets, portraits and the DERIVED vfx inventory are resolved too.
///
/// `CharacterBindings` carried only a cue resolver, so `was_checked` reported
/// honestly about four namespaces nobody checked. A misspelled sheet target was
/// reported later by the art pipeline as `NoSheetResolved` — true, but at load
/// time, without a did-you-mean, and indistinguishable from a legitimately
/// art-free build.
#[test]
fn sheets_portraits_and_derived_vfx_are_resolved_at_preparation() {
    let definition = CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo")
        .with_sheet("super_mary_o_sprtiesheet") // typo
        .with_moveset(moveset_with(
            &[("attack", "slash")],
            vec![slash("slash", "swing", "hit")],
        ));

    // Nothing supplied: every art namespace reports NOT CHECKED, and the report is
    // clean — because nobody looked, which must not read as "looked and fine".
    let unchecked =
        prepare_and_finalize_for_test(definition.clone(), &CharacterBindings::default());
    assert!(!unchecked.prepared.was_checked(SheetTarget::NAME));
    assert!(unchecked.is_clean());

    // Vocabulary supplied: the typo is NAMED at preparation.
    let checked = prepare_and_finalize_for_test(
        definition,
        &CharacterBindings::default()
            .with_available_sheets(["super_mary_o_spritesheet", "sanic_spritesheet"]),
    );
    assert!(checked.prepared.was_checked(SheetTarget::NAME));
    assert!(
        !checked.is_clean(),
        "a misspelled sheet target must be reported at preparation, not left for \
         the art pipeline to call `NoSheetResolved` at load time"
    );
}

/// A derived vfx tag nobody can draw is named — the same treatment cues get.
#[test]
fn a_derived_vfx_tag_no_renderer_knows_is_named() {
    let mut spec = slash("slash", "swing", "hit");
    spec.windows[0].volumes[0].vfx = Some("spark_blosom".to_string()); // typo
    let definition = CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo")
        .with_moveset(moveset_with(&[("attack", "slash")], vec![spec]));

    let prepared = prepare_and_finalize_for_test(
        definition,
        &CharacterBindings::default().with_known_vfx_tags(["spark_blossom"]),
    );
    assert!(prepared.prepared.was_checked(VfxTag::NAME));
    assert!(
        !prepared.is_clean(),
        "§4.6 derives the vfx inventory from the moves that request it; deriving it \
         faithfully into a list nobody resolves is only half the boundary"
    );
}

/// A verb the runtime cannot press is named at preparation.
///
/// The dangling-move-id check has always covered "the verb points at nothing".
/// This is the other side: the move exists, the binding is well-formed, and the
/// VERB is a word the trigger path never asks for — so the move is authored,
/// prepared, projected onto the body, and never triggered by anything.
#[test]
fn a_verb_the_runtime_never_presses_is_named_at_preparation() {
    let unreachable =
        CharacterDefinition::new("duelist", "Duelist", "arena").with_moveset(moveset_with(
            // `heavy` is not in the runtime vocabulary. `attack` / `smash` /
            // `ranged` / `special` are, with directional and airborne suffixes.
            &[("heavy", "big_swing")],
            vec![slash("big_swing", "arena.swing", "arena.hit")],
        ));

    let prepared = prepare_and_finalize_for_test(unreachable, &CharacterBindings::default());
    let problems: Vec<String> = prepared
        .report
        .unresolved()
        .iter()
        .map(|entry| format!("{entry:?}"))
        .collect();
    assert!(
        problems.iter().any(|entry| entry.contains("heavy")),
        "an unreachable verb was not reported. The move is authored and can \
         never fire, which looks exactly like a character that authored no \
         moves: {problems:?}"
    );

    // And the vocabulary really does accept what content legitimately writes —
    // otherwise this check would be a wall every real fighter walks into.
    for verb in [
        "attack",
        "attack_up",
        "attack_down",
        "attack_forward",
        "attack_back",
        "attack_air",
        "attack_air_down",
        "smash",
        "ranged",
        "special",
    ] {
        let ok =
            CharacterDefinition::new("duelist", "Duelist", "arena").with_moveset(moveset_with(
                &[(verb, "big_swing")],
                vec![slash("big_swing", "arena.swing", "arena.hit")],
            ));
        let prepared = prepare_and_finalize_for_test(ok, &CharacterBindings::default());
        assert!(
            prepared.is_clean(),
            "`{verb}` is a verb the runtime presses and preparation rejected it: {:?}",
            prepared.report.unresolved()
        );
    }
}

/// A ranged move needs something to throw, and the two halves live apart.
///
/// The projectile specification is on the ACTION SET; the move that fires it is
/// on the MOVESET. Once a definition can author both (C3 precedence), it can
/// author a `ranged` verb and an action set with no ranged payload — and each
/// half is individually valid. The verb is real, the move is real, the set is
/// real, and the button does nothing.
///
/// Only preparation holds both, so only preparation can see it.
#[test]
fn an_authored_ranged_move_with_no_ranged_payload_is_reported() {
    use crate::brain::ActionSet;

    let ranged_move = CharacterDefinition::new("gunner", "Gunner", "demo")
        .with_action_set(ActionSet::default())
        .with_moveset(moveset_with(
            &[("ranged", "bolt")],
            vec![slash("bolt", "swing", "hit")],
        ));

    let report = prepare_and_finalize_for_test(ranged_move, &CharacterBindings::default())
        .prepared
        .unresolved_references()
        .map(str::to_string)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        report.contains("ranged payload") && report.contains("bolt"),
        "a ranged move with no payload prepared cleanly; the fighter's ranged \
         button is dead and nothing said so. Report was:\n{report}"
    );
}

/// The same definition with a payload authored prepares silently — otherwise the
/// check above is just noise every ranged character has to live with.
#[test]
fn an_authored_ranged_move_with_a_payload_prepares_cleanly() {
    use crate::brain::action_set::{RangedActionSpec, RangedStyle};
    use crate::brain::ActionSet;

    let armed = CharacterDefinition::new("gunner", "Gunner", "demo")
        .with_action_set(ActionSet {
            ranged: Some(RangedActionSpec {
                style: RangedStyle::default(),
                speed: 300.0,
                damage: 1,
                flight: None,
                visual: None,
                charge: None,
                refire_s: crate::brain::action_set::DEFAULT_RANGED_REFIRE_S,
                aim_assist: None,
                discharge: None,
            }),
            ..ActionSet::default()
        })
        .with_moveset(moveset_with(
            &[("ranged", "bolt")],
            vec![slash("bolt", "swing", "hit")],
        ));

    let report = prepare_and_finalize_for_test(armed, &CharacterBindings::default())
        .prepared
        .unresolved_references()
        .map(str::to_string)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !report.contains("ranged payload"),
        "an armed ranged character was reported anyway:\n{report}"
    );
}

/// A character that authored NO action set is not asked about payloads.
///
/// Falling through to the catalog is the migration path, and its rows resolve
/// elsewhere. Complaining here would complain about a value this definition never
/// claimed — which is how a coherence check turns into noise and gets waived.
#[test]
fn a_ranged_move_without_an_authored_action_set_is_left_to_the_catalog() {
    let inheritor = CharacterDefinition::new("gunner", "Gunner", "demo").with_moveset(
        moveset_with(&[("ranged", "bolt")], vec![slash("bolt", "swing", "hit")]),
    );

    let report = prepare_and_finalize_for_test(inheritor, &CharacterBindings::default())
        .prepared
        .unresolved_references()
        .map(str::to_string)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !report.contains("ranged payload"),
        "preparation judged an action set the definition never authored:\n{report}"
    );
}

/// A body cannot end up with two owners of one press.
///
/// The whole verb FAMILY, not the base alone: `directional_verb_chain` resolves
/// a press through `ranged_air_forward` → `ranged_forward` → `ranged_air` →
/// `ranged`, so a suffixed binding owns that direction's press exactly as the
/// base owns the neutral one. Watching only `"ranged"` would leave the same
/// double-fire in the air.
#[test]
fn a_host_code_kit_cannot_also_carry_an_authored_ranged_verb() {
    let hybrid = CharacterDefinition::new("gunner", "Gunner", "demo").with_moveset(moveset_with(
        &[
            ("attack", "swing"),
            ("ranged", "bolt"),
            ("ranged_air", "bolt"),
            ("ranged_forward", "bolt"),
        ],
        vec![
            slash("swing", "swing", "hit"),
            slash("bolt", "swing", "hit"),
        ],
    ));

    // No catalog and no authored action set: the host-code kit, whose charge
    // path owns the ranged press.
    let prepared = prepare_and_finalize_for_test(hybrid, &CharacterBindings::default()).prepared;
    let PreparedKit::Unauthored { authored_moveset } = &prepared.kit else {
        panic!("expected the host-code kit, got {:?}", prepared.kit);
    };
    let verbs = &authored_moveset.as_ref().expect("authored moveset").verbs;

    assert!(
        !verbs.keys().any(|verb| verb.starts_with("ranged")),
        "the host kit owns the ranged press, so no ranged verb may survive: {verbs:?}"
    );
    // Everything else the author wrote is untouched — this revokes one press,
    // it does not discard the moveset.
    assert_eq!(verbs.get("attack").map(String::as_str), Some("swing"));
}

/// A cast has a version, so a derivation can know it went stale. (X4)
///
/// Since the finalization barrier it is published once, whole, and a late registration panics. The
/// generation is per PUBLICATION now; the hatch this test uses stamps each insert as its own
/// publication because a test has no barrier to publish for it.
#[test]
fn the_cast_generation_advances_on_every_published_change() {
    let mut registry = PreparedCharacterRegistry::default();
    let opening = registry.generation();

    registry.insert_prepared(
        prepare_and_finalize_for_test(mary_o(), &CharacterBindings::default()).prepared,
    );
    let after_first = registry.generation();
    assert!(
        after_first > opening,
        "publishing a character left the cast on {opening}"
    );

    // REPLACING a character is a new cast even though the count did not move —
    // a counter that only tracked insertions would call these two identical,
    // which is exactly the case a consumer needs to notice.
    registry.insert_prepared(
        prepare_and_finalize_for_test(
            CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"),
            &CharacterBindings::default(),
        )
        .prepared,
    );
    assert_eq!(registry.len(), 1, "the replacement did not add a character");
    assert!(
        registry.generation() > after_first,
        "a replaced cast reported the same generation as the one it replaced"
    );
}

/// Preparation resolves gravity freedom into the prepared character. Runtime
/// construction must not re-query catalog body-kind metadata to decide it.
#[test]
fn gravity_freedom_is_resolved_at_preparation_rather_than_at_construction() {
    use crate::actor::character_catalog::CharacterCatalog;

    const CATALOG: &str = r#"(
        brain_presets: { "stand_still": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk, melee: None, ranged: None, special: None) },
        characters: {
            "floater": (
                display_name: "Floater",
                spritesheet: "sprites/x.png",
                manifest: "sprites/x.ron",
                tier: MainHall,
                body_kind: Floating,
                composition: None,
                default_brain: "stand_still",
                default_action_set: "peaceful",
                tags: [],
                fallback_dialogue: [],
            ),
        },
    )"#;
    let catalog =
        CharacterCatalog::from_data(crate::actor::character_catalog::parse_catalog(CATALOG));
    let walking = crate::actor::CharacterLocomotion {
        run_speed: 90.0,
        ..Default::default()
    };
    let prepared = prepare_and_finalize_against_for_test(
        CharacterDefinition::new("floater", "Floater", "test").with_locomotion(walking),
        &CharacterBindings::default(),
        Some(&catalog),
    )
    .prepared;
    assert!(
        prepared
            .body_blueprint()
            .expect("it states its locomotion")
            .locomotion
            .baseline_free_flight
            //  `Some(false)`, not `None`: preparation RESOLVES the question
            // even when the answer is "no". A `None` reaching a body would mean
            // the barrier left it open for a constructor to rediscover, which is
            // exactly what §14 deletes.
            == Some(false),
        "this character said nothing about flight and its catalog row says \
         `Floating` — which is a SILHOUETTE claim. A row deciding locomotion is \
         the coupling D89 cut; preparation must resolve silence to GROUNDED and \
         carry that one concrete answer"
    );

    //  and a character that DOES say it flies keeps that, which is the
    // other half: cutting the catalog's fold must not also stop a bird flying.
    let stated = prepare_and_finalize_against_for_test(
        CharacterDefinition::new("floater", "Floater", "test").with_locomotion(
            crate::actor::CharacterLocomotion {
                run_speed: 90.0,
                baseline_free_flight: Some(true),
                ..Default::default()
            },
        ),
        &CharacterBindings::default(),
        Some(&catalog),
    )
    .prepared;
    assert_eq!(
        stated
            .body_blueprint()
            .expect("it states its locomotion")
            .locomotion
            .baseline_free_flight,
        Some(true),
        "an authored answer must survive preparation untouched"
    );

    // A character nobody's catalog knows keeps its own answer, which is the
    // ordinary case for a body that walks.
    let grounded = prepare_and_finalize_against_for_test(
        CharacterDefinition::new("stranger", "Stranger", "test").with_locomotion(walking),
        &CharacterBindings::default(),
        Some(&catalog),
    )
    .prepared;
    assert_eq!(
        grounded
            .body_blueprint()
            .expect("it states its locomotion")
            .locomotion
            .baseline_free_flight,
        //  `Some(false)`, and the difference is the whole three-state. An
        // unknown id has no catalog answer to fold, so preparation resolves the
        // silence to "does not fly" — and a reader must not have to tell that
        // apart from "nobody said".
        Some(false),
    );
}

/// Completeness is a NAMED contract, not an inferred bool.
///
/// Two terms, both observed: an incomplete character NAMES what it is missing,
/// and a complete one hands over a blueprint carrying the facts construction
/// actually reads.
#[test]
fn an_incomplete_character_names_the_fact_it_is_missing() {
    let bare = prepare_and_finalize_for_test(
        CharacterDefinition::new("wisp", "Wisp", "test"),
        &CharacterBindings::default(),
    )
    .prepared;
    let missing = bare
        .body_blueprint()
        .expect_err("a character that authored nothing cannot build a body");
    assert_eq!(missing.character_id, "wisp");
    assert_eq!(missing.missing, vec!["locomotion"]);
    assert!(
        missing.to_string().contains("locomotion"),
        "the diagnostic has to name the fact, or it is the bool again with a \
         longer type: {missing}"
    );

    let whole = prepare_and_finalize_for_test(
        CharacterDefinition::new("goblin", "Goblin", "test")
            .with_locomotion(crate::actor::CharacterLocomotion {
                run_speed: 170.0,
                ..Default::default()
            })
            .as_practice_target(),
        &CharacterBindings::default(),
    )
    .prepared;
    let body = whole
        .body_blueprint()
        .expect("a character that stated how it moves can build one");
    assert_eq!(body.character_id, "goblin");
    assert_eq!(body.locomotion.run_speed, 170.0);
    assert!(
        body.practice_target,
        "the blueprint carries what construction READS, and a dummy that lost \
         this on the way in is the D77 defect"
    );
}

/// The assembled shape a shared-policy fixture has to model: fragment keys are
/// namespaced `provider::local_name` by `CharacterCatalogRegistry` (cite-ok:
/// that is the SHAPE of a key, not a symbol -- neither half is an item).
const SHARED_POLICY_CATALOG: &str = r#"(
    autonomous_profiles: {
        //  NAMESPACED, because assembly namespaces every fragment key
        // (`registry.rs`: `namespaced(provider_id, local_name)`). A fixture
        // keying the bare local name models a catalog that cannot exist, and
        // it would hide exactly the mismatch `BrainProfileRef` was introduced
        // to stop — the author writing one spelling and the registry holding
        // another.
        "test::striker": (
            template: Skirmisher,
            aggro_radius: 620.0,
            attack_range: 96.0,
            patrol_effort: 0.44,
        ),
    },
    brain_presets: {},
    action_set_presets: {},
    characters: {},
)"#;

/// because that combination is the thing that could not be said before.
#[test]
fn two_characters_can_name_one_shared_policy() {
    use crate::actor::character_catalog::CharacterCatalog;
    use crate::brain::CharacterBrainTemplate;

    const CATALOG: &str = SHARED_POLICY_CATALOG;
    let catalog =
        CharacterCatalog::from_data(crate::actor::character_catalog::parse_catalog(CATALOG));

    let named = |id: &str, run_speed: f32| {
        prepare_and_finalize_against_for_test(
            CharacterDefinition::new(id, id, "test")
                .with_locomotion(crate::actor::CharacterLocomotion {
                    run_speed,
                    ..Default::default()
                })
                .with_autonomous_profile_named("striker"),
            &CharacterBindings::default(),
            Some(&catalog),
        )
        .prepared
    };
    let goblin = named("goblin", 120.0);
    let skitter = named("skitter", 260.0);

    for prepared in [&goblin, &skitter] {
        let profile = prepared
            .autonomous_profile
            .expect("the NAMED policy resolved into a value at preparation");
        assert_eq!(profile.template, CharacterBrainTemplate::Skirmisher);
        assert_eq!(profile.aggro_radius, 620.0);
        assert_eq!(
            profile.patrol_effort, 0.44,
            "the shared amble, as a fraction of each body's own top speed"
        );
    }
    assert_ne!(
        goblin.locomotion.map(|l| l.run_speed),
        skitter.locomotion.map(|l| l.run_speed),
        "two DIFFERENT bodies sharing one policy is the case that could not be \
         expressed before — a fixture where they matched would prove nothing"
    );
}

/// An author who writes both wants a patch type that does not exist, and telling them so is
/// cheaper than silently answering half their question.
#[test]
#[should_panic(expected = "authors an inline autonomous profile AND names")]
fn authoring_a_policy_twice_is_refused_rather_than_ranked() {
    use crate::actor::character_catalog::CharacterCatalog;
    use crate::brain::CharacterBrainTemplate;

    let catalog = CharacterCatalog::from_data(crate::actor::character_catalog::parse_catalog(
        SHARED_POLICY_CATALOG,
    ));
    let _ = prepare_and_finalize_against_for_test(
        CharacterDefinition::new("statue", "Statue", "test")
            .with_autonomous_profile_named("striker")
            .with_autonomous_profile(crate::brain::BrainProfile {
                template: CharacterBrainTemplate::StandStill,
                ..Default::default()
            }),
        &CharacterBindings::default(),
        Some(&catalog),
    );
}

/// A named policy nobody authored is a preparation failure.
#[test]
#[should_panic(expected = "is not published")]
fn a_named_policy_that_does_not_exist_is_a_failure_rather_than_silence() {
    use crate::actor::character_catalog::CharacterCatalog;

    let catalog = CharacterCatalog::from_data(crate::actor::character_catalog::parse_catalog(
        SHARED_POLICY_CATALOG,
    ));
    let _ = prepare_and_finalize_against_for_test(
        CharacterDefinition::new("ghost", "Ghost", "test")
            .with_autonomous_profile_named("no_such_policy"),
        &CharacterBindings::default(),
        Some(&catalog),
    );
}

/// A HOST THAT PUBLISHED NO POLICY AUTHORITY IS FINE — UNTIL A CHARACTER
/// NAMES ONE.
///
/// The same authoring error produced a content error or a silent absence depending on a composition
/// detail the author cannot see.
///
///  the first half is what stops this becoming "headless hosts must publish a
/// registry". They must not — a composition with no shared policies at all is
/// ordinary, and this asserts it still prepares. What is refused is an explicit
/// reference with nothing to resolve it, which is a different claim about a
/// different character.
#[test]
fn a_host_with_no_policy_registry_prepares_a_character_that_names_no_policy() {
    let prepared = prepare_and_finalize_for_test(
        CharacterDefinition::new("wanderer", "Wanderer", "test"),
        &CharacterBindings::default(),
    )
    .prepared;
    assert!(
        prepared.autonomous_profile.is_none(),
        "a character that named no shared policy came out of a registry-free \
         composition carrying one, so something is inventing a policy where the \
         author deliberately left the archetype in charge"
    );
}

/// The other half: the same registry-free composition, one character that DOES
/// name a policy. See [`a_host_with_no_policy_registry_prepares_a_character_that_names_no_policy`].
#[test]
#[should_panic(expected = "published no profile registry")]
fn naming_a_policy_in_a_composition_with_no_registry_is_a_composition_error() {
    let _ = prepare_and_finalize_for_test(
        CharacterDefinition::new("ghost", "Ghost", "test").with_autonomous_profile_named("striker"),
        &CharacterBindings::default(),
    );
}

/// THE AUTHORED MIRROR-SYMMETRY TRAIT SURVIVES THE WHOLE FOLD.
///
/// Every link is a hand-written field assignment, so a trait that is authored and never arrives
/// looks exactly like a trait nobody authored — the shape this repo calls *a hand-listed chain
/// pins the FUNCTION, not the WIRING*.
///
///  both directions, because a fold that hard-coded `true` would pass a
/// one-sided test while giving every character in the game Emmy's trait.
#[test]
fn mirror_symmetry_survives_preparation_and_reaches_the_body_blueprint() {
    for authored in [false, true] {
        let mut definition = mary_o();
        if authored {
            definition = definition.preserving_mirror_symmetry();
        }
        let prepared =
            prepare_and_finalize_for_test(definition, &CharacterBindings::default()).prepared;
        assert_eq!(
            prepared.preserves_mirror_symmetry, authored,
            "preparation lost or invented the mirror-symmetry trait (authored: \
             {authored})"
        );
        // A SEAT is the road that matters — this is a Smash CPU trait — and the
        // seat blueprint is the one a match builds.
        assert_eq!(
            prepared.seat_blueprint(200.0).preserves_mirror_symmetry,
            authored,
            "the seat blueprint dropped the mirror-symmetry trait, so a seated \
             fighter's brain could never read it (authored: {authored})"
        );
    }
}

/// ⛔⛔ **A BROKEN AUTHORED FLOW REACHED THE RUNTIME WITH NOTHING IN THE
/// PREPARATION PIPELINE LOOKING AT IT.**
///
/// `TechniqueFlow::problems` exists because every one of its failures is silent:
/// a transition past the end of the list, a `Wait` that can never expire, a
/// cycle, a node nothing arrives at. Until this, its only production callers were
/// per-crate roster tests walking hand-built `tables()` — so a character prepared
/// from a SERIALIZED definition, which is exactly the road the admission contract
/// is being built for, was validated by nobody.
///
/// ⭐ REPORTED, NOT REFUSED, which is this seam's established policy: preparation
/// publishes with its failures carried onto the value, and the
/// shipped-composition guard reads `unresolved_references`. A data error becomes
/// a red test rather than a boot failure.
#[test]
fn a_move_whose_flow_cannot_run_is_reported_by_preparation() {
    use crate::brain::ActionSet;
    use ambition_entity_catalog::{FlowNode, TechniqueFlow};

    let broken = |flow: Option<TechniqueFlow>| {
        let mut spec = slash("special", "swing", "hit");
        spec.flow = flow;
        prepare_and_finalize_for_test(
            CharacterDefinition::new("oni", "Oni", "demo")
                .with_action_set(ActionSet::default())
                .with_moveset(moveset_with(&[], vec![spec])),
            &CharacterBindings::default(),
        )
        .prepared
        .unresolved_references()
        .map(str::to_string)
        .collect::<Vec<_>>()
    };

    // ⛔ THE FLOOR. A move with a sound flow — and one with none at all — must
    // report nothing, or the row below is satisfied by preparation complaining
    // about every character it sees.
    assert_eq!(broken(None), Vec::<String>::new());
    assert_eq!(
        broken(Some(TechniqueFlow {
            nodes: vec![FlowNode::Finish],
        })),
        Vec::<String>::new(),
    );

    let reported = broken(Some(TechniqueFlow {
        nodes: vec![
            FlowNode::Emit {
                effect: ambition_entity_catalog::EffectRef {
                    key: "demo.thing".to_string(),
                    params: Default::default(),
                },
                // Past the end of a two-node list.
                then: 9,
            },
            FlowNode::Finish,
        ],
    }));
    assert!(
        reported
            .iter()
            .any(|line| line.contains("move `special` flow") && line.contains("past the last node")),
        "a flow whose transition leaves the list prepared cleanly, so the move \
         plays and silently stops. Report was: {reported:?}"
    );
}

// ---------------------------------------------------------------------------
// ⛔⛔ A NESTED REFERENCE WAS CHECKED AT FIRE TIME AND NOWHERE ELSE.
//
// `SummonRideParams::character_id` names another authored definition. Nothing
// verified it during preparation, so a summon pointing at a character this
// composition never prepared reached `preflight_planned_bodies`, which logs
// "summon batch rejected before mutation" and RETURNS — the move plays, the
// rider mounts nothing, and the only evidence is a log line during a fight.
//
// ⭐ THE OFFER DECLARES WHAT IT NAMES, so the checking loop never learns a
// technique's name. Matching on `smash.summon_ride` inside the validation pass
// would be the service locator the owner document forbids; asking the offer
// keeps this a bounded validation catalog.
// ---------------------------------------------------------------------------
mod nested_references {
    use super::*;
    use crate::prepared::unsupported_authored_effects;
    use ambition_entity_catalog::smash_ride::{summon_ride_character_refs, SUMMON_RIDE};
    use ambition_entity_catalog::{
        check_hydrates, EffectRef, NestedReferences, ParamValue, TechniqueDelivery, TechniqueOffer,
        TechniqueParams, TechniqueSupport,
    };

    /// A support table that installs the summon and declares its mount reference.
    fn supporting_summons() -> TechniqueSupport {
        let mut support = TechniqueSupport::default();
        support
            .declare(
                SUMMON_RIDE,
                TechniqueOffer {
                    owner: "test::shark_ride",
                    params: TechniqueParams::Checked(
                        check_hydrates::<ambition_entity_catalog::smash_ride::SummonRideParams>,
                    ),
                    references: NestedReferences::Characters(summon_ride_character_refs),
                    delivery: TechniqueDelivery::Action,
                },
            )
            .expect("a fresh table admits the first claim");
        support
    }

    fn summoning(mount: &str) -> EffectRef {
        EffectRef {
            key: SUMMON_RIDE.to_string(),
            params: ParamValue::parse(&format!(
                "(character_id: \"{mount}\", half_extents: (24.0, 16.0), seconds: 5.0, \
                 reach: 600.0)"
            ))
            .expect("the fixture's params parse"),
        }
    }

    /// The extractor is the whole mechanism; prove it reads the id before any
    /// registry question is asked of it.
    #[test]
    fn the_offer_reports_the_character_a_summon_names() {
        assert_eq!(
            summon_ride_character_refs(&summoning("burning_flying_shark")),
            vec!["burning_flying_shark".to_string()],
        );
    }

    /// ⭐ CONTROL. Params that do not hydrate name NOTHING here — whether they
    /// hydrate is `TechniqueParams::Checked`'s question, asked on the same
    /// effect by the same pass, and answering it twice reports one defect as
    /// two.
    #[test]
    fn malformed_params_name_no_character() {
        let effect = EffectRef {
            key: SUMMON_RIDE.to_string(),
            params: ParamValue::parse("(character_id: 12)").unwrap_or_default(),
        };
        assert!(summon_ride_character_refs(&effect).is_empty());
    }

    /// ⭐ CONTROL. A technique that declares no references names none, whatever
    /// its params say — otherwise every key would be scanned for ids.
    #[test]
    fn a_technique_declaring_no_references_names_nothing() {
        assert!(NestedReferences::None
            .characters(&summoning("burning_flying_shark"))
            .is_empty());
    }

    /// ⛔ THE CASE THAT USED TO REACH THE PLAYER. A summon naming a character
    /// this composition never prepared is now refused while an author is
    /// looking.
    #[test]
    fn a_summon_naming_an_unprepared_character_is_refused_at_preparation() {
        let registry = registry_with_summon("burning_flying_shark", &[]);
        let refusals = unsupported_authored_effects(&supporting_summons(), &registry);
        assert_eq!(
            refusals.len(),
            1,
            "expected exactly the missing mount; got {refusals:?}"
        );
        assert!(
            refusals[0].detail.contains("burning_flying_shark"),
            "the refusal must NAME the character that is missing, or an author \
             cannot act on it: {:?}",
            refusals[0]
        );
        // ⭐ AND IT MUST NAME THE RIDER, because that is the definition the fold
        // WITHHOLDS. A refusal that cannot be attributed to a character can only
        // be logged, which is the linting behaviour review #9 rejected.
        assert_eq!(
            refusals[0].character, "rider",
            "the refusal must attribute itself to the definition that carries the \
             effect: {:?}",
            refusals[0]
        );
    }

    /// ⭐ THE ANTI-VACUITY FLOOR. The same cast WITH the mount prepared must
    /// pass — without this, a check that refused every summon would satisfy the
    /// test above and break every rideable character in the game.
    #[test]
    fn the_same_summon_passes_once_its_mount_is_prepared() {
        let registry = registry_with_summon("burning_flying_shark", &["burning_flying_shark"]);
        let refusals = unsupported_authored_effects(&supporting_summons(), &registry);
        assert!(
            refusals.is_empty(),
            "a summon whose mount IS prepared was refused: {refusals:?}"
        );
    }

    /// A move whose ACTIVE WINDOW sustains `effect` — one of the four sites
    /// `MoveSpec::effect_refs` visits, chosen because it is the site
    /// `mary_o_grab` uses in the shipped content.
    fn move_emitting(id: &str, effect: EffectRef) -> ambition_entity_catalog::MoveSpec {
        let mut spec = crate::prepared_fixtures::slash(id, "cue", "strike");
        spec.windows[0].sustain_effect = Some(effect);
        spec
    }

    /// A rider whose move summons `mount`, plus `also` prepared beside it.
    fn registry_with_summon(
        mount: &str,
        also: &[&str],
    ) -> crate::prepared::PreparedCharacterRegistry {
        use crate::prepared_fixtures::moveset_with;
        let mut registry = crate::prepared::PreparedCharacterRegistry::default();
        let rider =
            CharacterDefinition::new("rider", "Rider", "test_demo").with_moveset(moveset_with(
                &[("special", "call_the_shark")],
                vec![move_emitting("call_the_shark", summoning(mount))],
            ));
        registry.insert(
            crate::prepared::prepare_and_finalize_for_test(rider, &CharacterBindings::default())
                .prepared,
        );
        for id in also {
            let mount_def = CharacterDefinition::new(*id, *id, "test_demo");
            registry.insert(
                crate::prepared::prepare_and_finalize_for_test(
                    mount_def,
                    &CharacterBindings::default(),
                )
                .prepared,
            );
        }
        registry
    }
}

// ---------------------------------------------------------------------------
// ⛔⛔ DETECTING A REFUSED TECHNIQUE AND THEN PUBLISHING IT ANYWAY IS NOT
// ADMISSION — IT IS STARTUP LINTING.
//
// The first production wiring walked every expanded move, accumulated refusals,
// logged them, and then inserted the registry unconditionally. The diagnostic
// even said what followed: the moves "will play and do nothing" — which is
// precisely the runtime failure A11 exists to prevent. GPT review #9 rejected
// it, correctly. The owner contract requires the checked result BEFORE
// active-definition publication.
//
// ⚠ PER DEFINITION, NOT PER CAST. Refusing the whole cast because ONE character
// names a technique this host did not install would take down compositions over
// a content/composition mismatch they did not create. Withholding exactly the
// definitions that carry unsupported effects is the literal reading of "invalid
// or uninstalled calls cannot publish definitions", and it is what these tests
// pin.
// ---------------------------------------------------------------------------
mod withholding {
    use super::*;
    use crate::prepared::{admit_and_finalize_cast, CharacterCatalogGeneration};
    use ambition_entity_catalog::smash_ride::{summon_ride_character_refs, SUMMON_RIDE};
    use ambition_entity_catalog::{
        check_hydrates, EffectRef, NestedReferences, ParamValue, TechniqueDelivery, TechniqueOffer,
        TechniqueParams, TechniqueSupport,
    };

    fn supporting_summons() -> TechniqueSupport {
        let mut support = TechniqueSupport::default();
        support
            .declare(
                SUMMON_RIDE,
                TechniqueOffer {
                    owner: "test::shark_ride",
                    params: TechniqueParams::Checked(
                        check_hydrates::<ambition_entity_catalog::smash_ride::SummonRideParams>,
                    ),
                    references: NestedReferences::Characters(summon_ride_character_refs),
                    delivery: TechniqueDelivery::Action,
                },
            )
            .expect("a fresh table admits the first claim");
        support
    }

    /// A move whose active window sustains one authored effect.
    fn move_emitting(id: &str, effect: EffectRef) -> ambition_entity_catalog::MoveSpec {
        let mut spec = crate::prepared_fixtures::slash(id, "cue", "strike");
        spec.windows[0].sustain_effect = Some(effect);
        spec
    }

    fn staged(definition: CharacterDefinition) -> crate::prepared::StagedCharacter {
        crate::prepared::prepare_for_registration(definition, &CharacterBindings::default()).staged
    }

    /// `who` authors `key`; `plain` authors nothing unusual.
    fn cast(who: &str, key: &str, plain: &str) -> Vec<crate::prepared::StagedCharacter> {
        use crate::prepared_fixtures::moveset_with;
        let effect = EffectRef {
            key: key.to_string(),
            params: ParamValue::parse(
                "(character_id: \"nobody\", half_extents: (1.0, 1.0), seconds: 1.0, reach: 1.0)",
            )
            .expect("params parse"),
        };
        vec![
            staged(
                CharacterDefinition::new(who, who, "test_demo").with_moveset(moveset_with(
                    &[("special", "the_move")],
                    vec![move_emitting("the_move", effect)],
                )),
            ),
            staged(CharacterDefinition::new(plain, plain, "test_demo")),
        ]
    }

    /// ⛔ THE CASE REVIEW #9 REJECTED. An unsupported key must keep its
    /// definition OUT of the published registry.
    #[test]
    fn a_definition_naming_an_uninstalled_technique_is_not_published() {
        let admitted = admit_and_finalize_cast(
            cast("rider", "smash.not_installed_here", "bystander"),
            None,
            None,
            CharacterCatalogGeneration::default(),
            &supporting_summons(),
        );

        assert!(
            admitted.registry.get("rider").is_none(),
            "the definition naming an uninstalled technique was PUBLISHED; its \
             moves would play and answer nothing, which is the exact failure \
             admission exists to prevent"
        );
        assert_eq!(
            admitted.refusals.len(),
            1,
            "expected one refusal naming the rider; got {:?}",
            admitted.refusals
        );
        assert_eq!(admitted.refusals[0].character, "rider");
    }

    /// ⭐ THE ANTI-VACUITY FLOOR, and the reason this is per-DEFINITION. A
    /// refusal must not take the rest of the cast down with it — a check that
    /// withheld everything would satisfy the test above while emptying the game.
    #[test]
    fn the_rest_of_the_cast_is_still_published() {
        let admitted = admit_and_finalize_cast(
            cast("rider", "smash.not_installed_here", "bystander"),
            None,
            None,
            CharacterCatalogGeneration::default(),
            &supporting_summons(),
        );

        assert!(
            admitted.registry.get("bystander").is_some(),
            "an unrelated character was withheld because a DIFFERENT definition \
             was refused: {:?}",
            admitted.refusals
        );
    }

    /// A summons `beast`; `beast` authors an uninstalled technique.
    fn a_summoner_and_the_beast_it_names() -> Vec<crate::prepared::StagedCharacter> {
        use crate::prepared_fixtures::moveset_with;
        let summon = EffectRef {
            key: SUMMON_RIDE.to_string(),
            params: ParamValue::parse(
                "(character_id: \"beast\", half_extents: (1.0, 1.0), seconds: 1.0, reach: 1.0)",
            )
            .expect("params parse"),
        };
        let unsupported = EffectRef {
            key: "smash.not_installed_here".to_string(),
            params: ParamValue::parse(
                "(character_id: \"nobody\", half_extents: (1.0, 1.0), seconds: 1.0, reach: 1.0)",
            )
            .expect("params parse"),
        };
        vec![
            staged(
                CharacterDefinition::new("summoner", "summoner", "test_demo").with_moveset(
                    moveset_with(
                        &[("special", "call_the_beast")],
                        vec![move_emitting("call_the_beast", summon)],
                    ),
                ),
            ),
            staged(
                CharacterDefinition::new("beast", "beast", "test_demo").with_moveset(moveset_with(
                    &[("special", "the_move")],
                    vec![move_emitting("the_move", unsupported)],
                )),
            ),
            staged(CharacterDefinition::new("bystander", "bystander", "test_demo")),
        ]
    }

    /// ⛔⛔ **A SUMMONER WHOSE BEAST WAS WITHHELD MUST NOT BE PUBLISHED EITHER —
    /// AND THE ONE-PASS VERSION PUBLISHED IT.**
    ///
    /// GPT review 2026-09-10 derived this from the algorithm rather than from a
    /// character: validation examined the FULL candidate registry, so
    /// `summoner`'s reference to `beast` resolved; `beast` was then refused and
    /// removed; `summoner` was retained. ⇒ **The published registry contained a
    /// definition pointing at one that is not in it** — and construction catches
    /// that only at the summon, as "body character not registered", which is
    /// *"play the move, summon nothing"*: the exact state A11 exists to
    /// eliminate.
    ///
    /// ⚠ THE EDIT THAT MAKES THIS FALSE is returning after the first filtering
    /// round instead of re-folding until the accepted set stops changing. The
    /// invariant is **references resolve in the PUBLISHED registry**, not in the
    /// candidate one.
    #[test]
    fn a_summoner_whose_beast_was_withheld_is_withheld_too() {
        let admitted = admit_and_finalize_cast(
            a_summoner_and_the_beast_it_names(),
            None,
            None,
            CharacterCatalogGeneration::default(),
            &supporting_summons(),
        );

        assert!(
            admitted.registry.get("beast").is_none(),
            "premise: the beast authors an uninstalled technique and must be \
             withheld before the summoner's reference can dangle at all"
        );
        assert!(
            admitted.registry.get("summoner").is_none(),
            "the summoner was PUBLISHED while the character its summon names was \
             withheld, so its move would play and summon nothing. Refusals: {:?}",
            admitted.refusals
        );
    }

    /// ⭐ AND THE FIXPOINT MUST NOT EAT THE CAST, which is the arm that stops the
    /// one above from passing on a check that withholds everything.
    ///
    /// ⚠ THE EDIT THAT MAKES THIS FALSE is refusing the whole cast on any
    /// refusal, or iterating on a set that never converges.
    #[test]
    fn the_bystander_survives_a_transitive_withholding() {
        let admitted = admit_and_finalize_cast(
            a_summoner_and_the_beast_it_names(),
            None,
            None,
            CharacterCatalogGeneration::default(),
            &supporting_summons(),
        );

        assert!(
            admitted.registry.get("bystander").is_some(),
            "a character unrelated to the refused pair was withheld by the \
             fixpoint: {:?}",
            admitted.refusals
        );
    }

    /// ⛔⛔ **AN EMPTY SUPPORT TABLE WITHHOLDS, AND THIS TEST USED TO ASSERT THE
    /// OPPOSITE.**
    ///
    /// It was `without_a_support_table_nothing_is_withheld`, and it deliberately
    /// locked in the escape a GPT review named as A11's last blocker: the fold
    /// took `Option<&TechniqueSupport>` and returned the whole cast unexamined on
    /// `None`, with an authored `"smash.not_installed_here"` in it. ⇒ **`None`
    /// carried two meanings that must not be conflated** — *"this composition
    /// supports zero techniques"*, which is a legitimate production state, and
    /// *"skip installed-technique validation entirely"*, which is not an
    /// admission mode at all.
    ///
    /// **A composition that installs no technique handlers does not have an
    /// UNKNOWN support set. It has the EMPTY one**, and a character naming a
    /// native effect there has a move that plays and answers nothing — exactly
    /// what A11 exists to prevent. The argument is no longer optional, so the
    /// escape cannot be selected.
    ///
    /// ⚠ MEASURED BEFORE INVERTING: the whole workspace passes either way (178
    /// blocks, 7703 passed, 0 failed with the empty table in production), so **no
    /// composition here was relying on the unchecked reading.**
    ///
    /// ⚠ THE EDIT THAT MAKES THIS FALSE is restoring an early return for an
    /// empty table.
    #[test]
    fn an_empty_support_table_withholds_a_native_effect_nobody_installed() {
        let admitted = admit_and_finalize_cast(
            cast("rider", "smash.not_installed_here", "bystander"),
            None,
            None,
            CharacterCatalogGeneration::default(),
            &TechniqueSupport::default(),
        );

        assert!(
            admitted.registry.get("rider").is_none(),
            "a composition that installs NOTHING published a definition naming a \
             native effect; its move would play and answer nothing, which is the \
             failure admission exists to prevent. Refusals: {:?}",
            admitted.refusals
        );
        assert_eq!(admitted.refusals.len(), 1);
        assert_eq!(admitted.refusals[0].character, "rider");
    }

    /// ⭐ AND THE ANTI-VACUITY ARM: an empty table must not withhold a character
    /// that names no technique at all. A check that refused everything would
    /// satisfy the test above while emptying the game.
    ///
    /// ⚠ THE EDIT THAT MAKES THIS FALSE is refusing the cast rather than the
    /// definitions that carry unsupported effects.
    #[test]
    fn an_empty_support_table_still_publishes_a_character_that_names_nothing() {
        let admitted = admit_and_finalize_cast(
            cast("rider", "smash.not_installed_here", "bystander"),
            None,
            None,
            CharacterCatalogGeneration::default(),
            &TechniqueSupport::default(),
        );

        assert!(
            admitted.registry.get("bystander").is_some(),
            "a character naming no technique was withheld by an EMPTY support \
             table, so admission is refusing the cast rather than the \
             definitions that carry unsupported effects: {:?}",
            admitted.refusals
        );
    }

    /// ⭐ CONTROL. A cast whose every effect IS supported publishes whole.
    #[test]
    fn a_fully_supported_cast_publishes_every_definition() {
        let mut staged_cast = cast("rider", SUMMON_RIDE, "bystander");
        // The mount the rider names, so the nested reference resolves too.
        staged_cast.push(staged(CharacterDefinition::new(
            "nobody",
            "nobody",
            "test_demo",
        )));
        let admitted = admit_and_finalize_cast(
            staged_cast,
            None,
            None,
            CharacterCatalogGeneration::default(),
            &supporting_summons(),
        );

        assert!(
            admitted.refusals.is_empty(),
            "a supported cast was refused: {:?}",
            admitted.refusals
        );
        assert!(admitted.registry.get("rider").is_some());
        assert!(admitted.registry.get("bystander").is_some());
    }
}

// ---------------------------------------------------------------------------
// ⛔⛔ `check_hydrates::<T>` IS NOT A SEMANTIC CHECK, AND THE DECLARATIONS SAID
// IT WAS.
//
// `TimeDilationParams::problems` rejects `scale >= 1` — a "slow" that speeds the
// victim up — and the LIVE HANDLER runs exactly that check at fire time. So a
// move authored `{scale: 2.0}` played, and was refused in the middle of a fight.
// The A11 declaration checked only that serde could build the struct, which the
// owner document says explicitly is insufficient. GPT review #9 named it.
// ---------------------------------------------------------------------------
mod domain_semantics {
    use ambition_entity_catalog::smash_time_dilation::{check_time_dilation_params, TimeDilationParams};
    use ambition_entity_catalog::{check_hydrates, ParamValue};

    fn params(ron_text: &str) -> ParamValue {
        ParamValue::parse(ron_text).expect("the fixture parses")
    }

    /// ⛔ THE CASE THAT REACHED THE PLAYER. A scale above 1 hydrates perfectly.
    #[test]
    fn a_slow_that_speeds_the_victim_up_is_refused_at_admission() {
        let authored = params("(scale: 2.0, seconds: 1.0)");

        assert!(
            check_hydrates::<TimeDilationParams>(&authored).is_ok(),
            "the premise is gone: if this no longer hydrates, the test below \
             passes for the wrong reason"
        );
        let refused = check_time_dilation_params(&authored);
        assert!(
            refused.is_err(),
            "`scale: 2.0` was admitted; the handler refuses it at fire time, so \
             the move plays and then does nothing"
        );
    }

    /// ⭐ THE ANTI-VACUITY FLOOR. Params the domain accepts must still be
    /// admitted — a check that refused everything would satisfy the test above
    /// and break every authored slow in the game.
    #[test]
    fn a_well_formed_slow_is_still_admitted() {
        assert_eq!(
            check_time_dilation_params(&params("(scale: 0.35, seconds: 0.5)")),
            Ok(())
        );
    }

    /// ⭐ CONTROL. Params that do not hydrate at all are still refused, and by
    /// the hydration error rather than a domain complaint — one defect, one
    /// sentence.
    #[test]
    fn params_that_do_not_hydrate_are_still_refused() {
        assert!(check_time_dilation_params(&params("(scale: \"fast\")")).is_err());
    }
}

// ---------------------------------------------------------------------------
// A11c — THE ACCEPTANCE ROW THAT COULD NOT BE WRITTEN.
//
// "Rejection leaves the active generation unchanged" was untestable, and the
// re-derivation that found out why is worth keeping: `PreparedCharacterRegistry`
// has exactly ONE production writer, guarded to run once, and
// `stage_authored_character` PANICS after the barrier closes. So nothing could
// produce a second generation to leave unchanged — the row was waiting on a
// republication road that did not exist, which is why three attempts at the
// fixture stopped on three different obstacles.
//
// ⚠ AND THE RULE IS NOT THE BARRIER'S. At initial activation there is no
// last-good, so a refused definition is WITHHELD and the rest of the cast still
// publishes. A revision is a transaction over a cast that is already live:
// applying half of it would leave a session in a state no author asked for, so
// the whole edit is refused and the previous registry — generation included —
// stays published.
// ---------------------------------------------------------------------------
mod revision_activation {
    use super::*;
    use crate::prepared::{
        activate_staged_revision, admit_and_finalize_cast, CharacterCatalogGeneration,
        PreparedCharacterRegistry, RevisionOutcome, StagedCastRevision,
    };
    use ambition_entity_catalog::smash_ride::{summon_ride_character_refs, SUMMON_RIDE};
    use ambition_entity_catalog::{
        check_hydrates, EffectRef, NestedReferences, ParamValue, TechniqueDelivery, TechniqueOffer,
        TechniqueParams, TechniqueSupport,
    };

    fn supporting_summons() -> TechniqueSupport {
        let mut support = TechniqueSupport::default();
        support
            .declare(
                SUMMON_RIDE,
                TechniqueOffer {
                    owner: "test::shark_ride",
                    params: TechniqueParams::Checked(
                        check_hydrates::<ambition_entity_catalog::smash_ride::SummonRideParams>,
                    ),
                    references: NestedReferences::Characters(summon_ride_character_refs),
                    delivery: TechniqueDelivery::Action,
                },
            )
            .expect("first claim");
        support
    }

    fn move_emitting(id: &str, effect: EffectRef) -> ambition_entity_catalog::MoveSpec {
        let mut spec = crate::prepared_fixtures::slash(id, "cue", "strike");
        spec.windows[0].sustain_effect = Some(effect);
        spec
    }

    fn effect(key: &str) -> EffectRef {
        EffectRef {
            key: key.to_string(),
            params: ParamValue::parse(
                "(character_id: \"mount\", half_extents: (1.0, 1.0), seconds: 1.0, reach: 1.0)",
            )
            .expect("params parse"),
        }
    }

    /// A definition whose one move carries `key`.
    fn authoring(id: &str, key: &str) -> CharacterDefinition {
        use crate::prepared_fixtures::moveset_with;
        CharacterDefinition::new(id, id, "test_demo").with_moveset(moveset_with(
            &[("special", "the_move")],
            vec![move_emitting("the_move", effect(key))],
        ))
    }

    /// A world with a published cast: `rider` (summoning `mount`) and `mount`.
    fn world_with_live_cast() -> bevy::ecs::world::World {
        let mut world = bevy::ecs::world::World::new();
        let staged = vec![
            crate::prepared::prepare_for_registration(
                authoring("rider", SUMMON_RIDE),
                &CharacterBindings::default(),
            )
            .staged,
            crate::prepared::prepare_for_registration(
                CharacterDefinition::new("mount", "mount", "test_demo"),
                &CharacterBindings::default(),
            )
            .staged,
        ];
        let sources = staged.clone();
        let admitted = admit_and_finalize_cast(
            staged,
            None,
            None,
            CharacterCatalogGeneration::default(),
            &supporting_summons(),
        );
        assert!(
            admitted.refusals.is_empty(),
            "the fixture's premise is gone: the starting cast is refused {:?}",
            admitted.refusals
        );
        world.insert_resource(admitted.registry);
        // ⛔⛤ **THE SOURCE THE REGISTRY IS A FOLD OF, WHICH THIS FIXTURE DID NOT
        // HAVE.** In production the preparation barrier retains
        // `StagedCharacterOverrides` past itself; this world called
        // `admit_and_finalize_cast` directly and so had the fold with no source
        // behind it. Two things read that resource —
        // `activate_staged_revision`'s no-op check and its write-back — and both
        // are `if let Some(..)`, so in a world without it they SILENTLY DO
        // NOTHING. The no-op test caught it by failing; the write-back had no
        // test that could.
        let mut overrides = crate::prepared::StagedCharacterOverrides::default();
        for character in sources {
            overrides.by_id.insert(
                ambition_entity_catalog::CharacterId::new(character.id()),
                character,
            );
        }
        world.insert_resource(overrides);
        world
    }

    fn stage(world: &mut bevy::ecs::world::World, definition: CharacterDefinition) {
        let staged =
            crate::prepared::prepare_for_registration(definition, &CharacterBindings::default())
                .staged;
        let id = ambition_entity_catalog::CharacterId::new(staged.id());
        world
            .get_resource_or_insert_with(StagedCastRevision::default)
            .insert_for_test(id, staged);
    }

    /// ⭐ THE PREMISE ARM. Without a revision that DOES activate, "refused leaves
    /// it unchanged" is satisfied by a mechanism that never activates anything.
    ///
    /// ⛔⛤ **IT STAGED THE DEFINITION THE FIXTURE WAS ALREADY LIVE WITH**, and
    /// passed because the old activation published unconditionally. So a test
    /// named *"publishes under a NEW generation"* could not tell "publishes a
    /// change" from "publishes anything" — the no-op rule is what exposed it, by
    /// correctly reporting `Unchanged` here. It stages a real edit now.
    #[test]
    fn an_admitted_revision_publishes_under_a_new_generation() {
        let mut world = world_with_live_cast();
        let before = world
            .resource::<PreparedCharacterRegistry>()
            .generation()
            .get();

        let mut edited = authoring("rider", SUMMON_RIDE);
        edited.display_name = "Rider, revised".to_string();
        stage(&mut world, edited);
        let outcome = activate_staged_revision(&mut world, &supporting_summons());

        let after = world.resource::<PreparedCharacterRegistry>().generation();
        assert!(
            matches!(outcome, RevisionOutcome::Activated { changed: 1, .. }),
            "expected one changed definition; got {outcome:?}"
        );
        assert_eq!(
            after.get(),
            before + 1,
            "an admitted revision must publish under a NEW generation, or nothing \
             downstream can tell the cast changed"
        );
    }

    /// ⛔ THE ROW ITSELF. A revision naming a technique nothing installed is
    /// refused, and the live cast keeps BOTH its definitions and its generation.
    #[test]
    fn a_refused_revision_leaves_the_active_generation_unchanged() {
        let mut world = world_with_live_cast();
        let before = world
            .resource::<PreparedCharacterRegistry>()
            .generation()
            .get();

        stage(&mut world, authoring("rider", "smash.not_installed_here"));
        let outcome = activate_staged_revision(&mut world, &supporting_summons());

        assert!(
            matches!(outcome, RevisionOutcome::Refused { .. }),
            "expected a refusal; got {outcome:?}"
        );
        let active = world.resource::<PreparedCharacterRegistry>();
        assert_eq!(
            active.generation().get(),
            before,
            "a REFUSED revision bumped the generation; every body stamped with the \
             previous one now reads as stale for an edit that never happened"
        );
        assert!(
            active.get("rider").is_some() && active.get("mount").is_some(),
            "the last-good cast lost a definition to a refused edit"
        );
    }

    /// ⭐ CONTROL. Nothing staged is not a refusal and not an activation.
    #[test]
    fn activating_nothing_is_not_a_refusal() {
        let mut world = world_with_live_cast();
        let before = world
            .resource::<PreparedCharacterRegistry>()
            .generation()
            .get();

        let outcome = activate_staged_revision(&mut world, &supporting_summons());

        assert_eq!(outcome, RevisionOutcome::NothingStaged);
        assert_eq!(
            world
                .resource::<PreparedCharacterRegistry>()
                .generation()
                .get(),
            before
        );
    }

    /// ⭐⭐ **RE-STAGING WHAT IS ALREADY LIVE DOES NOT MOVE THE GENERATION**
    /// (fast-iteration I3a, "no-op identity").
    ///
    /// ⛔ A FILE WATCHER FIRES ON A SAVE, NOT ON A CHANGE. Touch a file, re-run
    /// a formatter, save with no edit, and the same bytes arrive again.
    /// `CharacterCatalogGeneration` is what every staleness check in the session
    /// keys on, so publishing them would invalidate live bodies, cached plans
    /// and rollback diagnoses for nothing — the failure a reload loop produces
    /// constantly and a one-shot activation never does.
    #[test]
    fn re_staging_the_live_values_does_not_move_the_generation() {
        let mut world = world_with_live_cast();
        // First, a REAL edit, so the live cast has something to re-stage.
        let mut edited = authoring("rider", SUMMON_RIDE);
        edited.display_name = "Rider, revised".to_string();
        stage(&mut world, edited.clone());
        let first = activate_staged_revision(&mut world, &supporting_summons());
        assert!(
            matches!(first, RevisionOutcome::Activated { .. }),
            "the premise: an edit that DOES activate; got {first:?}"
        );
        let published = world.resource::<PreparedCharacterRegistry>().generation();

        // Now the same definition again — byte-for-byte what is live.
        stage(&mut world, edited);
        let second = activate_staged_revision(&mut world, &supporting_summons());

        assert_eq!(
            second,
            RevisionOutcome::Unchanged {
                generation: published
            },
            "re-staging the live values reported {second:?}"
        );
        assert_eq!(
            world.resource::<PreparedCharacterRegistry>().generation(),
            published,
            "the generation moved for a revision that proposed nothing new"
        );
    }

    /// ⛔ AND THE CONTROL, because "the generation did not move" is also what a
    /// mechanism that stopped activating ANYTHING would report. A revision that
    /// changes one field on the SAME character still publishes.
    #[test]
    fn a_revision_that_changes_one_field_still_publishes() {
        let mut world = world_with_live_cast();
        let mut first = authoring("rider", SUMMON_RIDE);
        first.display_name = "Rider, revised".to_string();
        stage(&mut world, first);
        let _ = activate_staged_revision(&mut world, &supporting_summons());
        let published = world.resource::<PreparedCharacterRegistry>().generation();

        let mut edited = authoring("rider", SUMMON_RIDE);
        edited.display_name = "Rider, renamed again".to_string();
        stage(&mut world, edited);
        let outcome = activate_staged_revision(&mut world, &supporting_summons());

        assert!(
            matches!(outcome, RevisionOutcome::Activated { changed: 1, .. }),
            "a changed field must still publish; got {outcome:?}"
        );
        assert_eq!(
            world
                .resource::<PreparedCharacterRegistry>()
                .generation()
                .get(),
            published.get() + 1,
            "a real edit did not move the generation"
        );
    }

    /// ⭐ CONTROL. A refused revision must not leave its edits staged to be
    /// applied by the NEXT activation — a transaction that fails is spent.
    #[test]
    fn a_refused_revision_does_not_linger_for_the_next_activation() {
        let mut world = world_with_live_cast();
        stage(&mut world, authoring("rider", "smash.not_installed_here"));
        let _ = activate_staged_revision(&mut world, &supporting_summons());

        let second = activate_staged_revision(&mut world, &supporting_summons());
        assert_eq!(
            second,
            RevisionOutcome::NothingStaged,
            "a refused edit was still staged and would have been retried silently"
        );
    }
}

// ---------------------------------------------------------------------------
// ⛔⛔ A HELD-ITEM ID NOBODY REGISTERED, CHECKED AT FIRE TIME AND NOWHERE ELSE.
//
// `DropBombParams::item_id` and `PlaceMineParams::item_id` both carry the id the
// dropped object becomes in somebody's hands, and both docs state the stake:
// "It must be a registered held item or nobody can pick [it] up — which is half
// the move." Nothing checked it. `bomb.rs` and `mine.rs` resolve it with
// `held_item_by_id` when the move FIRES and log on `None`, so an unregistered id
// produced an object nobody could take, mid-fight, with a log line as the only
// symptom.
//
// ⚠ I RECORDED THIS AS UNCHECKABLE AND IT WAS NOT. The note said these two could
// not be verified from `ambition_characters` because it cannot see the item
// vocabulary — the same layering wall the support table had to cross. Wrong:
// `held_item_by_id` is a static registry in `ambition_characters::brain::action_set`,
// the SAME crate as the barrier. The wall was assumed from the shape of an
// earlier problem rather than measured.
// ---------------------------------------------------------------------------
mod held_item_references {
    use super::*;
    use crate::brain::action_set::{held_item_by_id, held_item_ids};
    use crate::prepared::unsupported_authored_effects;
    use ambition_entity_catalog::smash_bomb::{bomb_held_item_refs, DropBombParams, DROP_BOMB};
    use ambition_entity_catalog::{
        check_hydrates, EffectRef, NestedReferences, ParamValue, TechniqueDelivery, TechniqueOffer,
        TechniqueParams, TechniqueSupport,
    };

    fn supporting_bombs() -> TechniqueSupport {
        let mut support = TechniqueSupport::default();
        support
            .declare(
                DROP_BOMB,
                TechniqueOffer {
                    owner: "test::bomb",
                    params: TechniqueParams::Checked(check_hydrates::<DropBombParams>),
                    references: NestedReferences::HeldItems(bomb_held_item_refs),
                    delivery: TechniqueDelivery::Action,
                },
            )
            .expect("first claim");
        support
    }

    fn dropping(item: &str) -> EffectRef {
        EffectRef {
            key: DROP_BOMB.to_string(),
            // ⚠ The real field list, from `DropBombParams`. My first version
            // carried a `knockback` this technique does not take — and the
            // params check caught it, reporting ONLY the hydration failure and
            // not a spurious item complaint, which is the "one defect, one
            // sentence" rule doing its job on my own fixture.
            params: ParamValue::parse(&format!(
                "(item_id: \"{item}\", fuse_s: 2.0, damage: 4, blast_radius: 24.0, \
                 impact_speed: 120.0, half_extents: (8.0, 8.0), offset: (0.0, 0.0))"
            ))
            .expect("the fixture's params parse"),
        }
    }

    fn cast_dropping(item: &str) -> crate::prepared::PreparedCharacterRegistry {
        use crate::prepared_fixtures::moveset_with;
        let mut spec = crate::prepared_fixtures::slash("the_move", "cue", "strike");
        spec.windows[0].sustain_effect = Some(dropping(item));
        let definition = CharacterDefinition::new("bomber", "Bomber", "test_demo")
            .with_moveset(moveset_with(&[("special", "the_move")], vec![spec]));
        let mut registry = crate::prepared::PreparedCharacterRegistry::default();
        registry.insert(
            crate::prepared::prepare_and_finalize_for_test(
                definition,
                &CharacterBindings::default(),
            )
            .prepared,
        );
        registry
    }

    /// ⭐ THE PREMISE. If the fixture's params stopped hydrating, the extractor
    /// would name nothing and every assertion below would pass vacuously.
    #[test]
    fn the_extractor_reports_the_item_a_bomb_names() {
        assert_eq!(
            bomb_held_item_refs(&dropping("gun_sword")),
            vec!["gun_sword".to_string()]
        );
    }

    /// ⛔ THE CASE THAT REACHED THE PLAYER.
    #[test]
    fn a_bomb_naming_an_unregistered_held_item_is_refused_at_preparation() {
        let item = "no_such_held_item";
        assert!(
            held_item_by_id(item).is_none(),
            "the fixture's premise is gone: '{item}' is registered, so the \
             refusal below would say nothing"
        );

        let refusals = unsupported_authored_effects(&supporting_bombs(), &cast_dropping(item));

        assert_eq!(refusals.len(), 1, "expected one refusal; got {refusals:?}");
        assert!(
            refusals[0].detail.contains(item),
            "the refusal must NAME the item an author has to fix: {:?}",
            refusals[0]
        );
    }

    /// ⭐ THE ANTI-VACUITY FLOOR. A REGISTERED item must still be admitted —
    /// a check that refused every item would satisfy the test above and break
    /// every bomb and mine in the game.
    #[test]
    fn a_bomb_naming_a_registered_held_item_is_admitted() {
        let registered = held_item_ids();
        let item = registered
            .first()
            .expect("the held-item registry is empty, so this arm proves nothing");
        assert!(held_item_by_id(item).is_some());

        let refusals = unsupported_authored_effects(&supporting_bombs(), &cast_dropping(item));

        assert!(
            refusals.is_empty(),
            "a bomb naming the registered item '{item}' was refused: {refusals:?}"
        );
    }
}

/// ⛔⛔ **A NON-FINITE AUTHORED NUMBER IS REFUSED AT ADMISSION, WHATEVER THE
/// TECHNIQUE.**
///
/// `NaN`, `inf` and `-inf` are valid RON and hydrate cleanly, so
/// `check_hydrates::<T>` — which is what twenty of the twenty-three shipped
/// declarations use — admits all three. MEASURED 2026-09-10 before this existed:
/// `(amount: NaN)` parsed, hydrated to `FillMeterParams { amount: NaN }`, and was
/// admitted.
///
/// ⇒ The cost is not one misbehaving move. `ResourceMeter::refill` is
/// `(current + amount).clamp(0.0, max)`, and `f32::clamp` returns `NaN` for a
/// `NaN` input, so one authored fill leaves the meter `NaN` FOREVER — every later
/// comparison against it false, and `body.mana` is rollback-canonical, so the
/// poison is snapshotted and restored across every rewind.
mod nonfinite_params {
    use ambition_entity_catalog::{
        check_hydrates, EffectRef, NestedReferences, ParamValue, TechniqueDelivery, TechniqueOffer,
        TechniqueParams, TechniqueRefusal, TechniqueSupport,
    };

    /// The meter fill, declared exactly the way the shipped composition declares
    /// it — `check_hydrates` and nothing else, which is the whole point.
    fn support() -> TechniqueSupport {
        let mut support = TechniqueSupport::default();
        support
            .declare(
                ambition_entity_catalog::smash_limit::FILL_METER,
                TechniqueOffer {
                    owner: "test::limit",
                    params: TechniqueParams::Checked(
                        check_hydrates::<ambition_entity_catalog::smash_limit::FillMeterParams>,
                    ),
                    references: NestedReferences::None,
                    delivery: TechniqueDelivery::Either,
                },
            )
            .expect("a fresh table admits the first claim");
        support
    }

    fn probe(text: &str) -> Result<(), TechniqueRefusal> {
        support().admit(&EffectRef {
            key: ambition_entity_catalog::smash_limit::FILL_METER.to_string(),
            params: ParamValue::parse(text).expect("the fixture params are valid RON"),
        })
    }

    #[test]
    fn nan_infinity_and_negative_infinity_are_all_refused() {
        for text in ["(amount: NaN)", "(amount: inf)", "(amount: -inf)"] {
            let refusal = probe(text).expect_err(&format!(
                "{text} was admitted — it is valid RON and serde builds it, so \
                 nothing downstream refuses it either"
            ));
            let said = refusal.to_string();
            assert!(
                said.contains("amount"),
                "the refusal must name the FIELD an author has to go fix, and it \
                 said: {said}"
            );
        }
    }

    /// ⭐ THE CONTROL, and without it the arm above passes on a check that
    /// refuses every number there is.
    #[test]
    fn an_ordinary_finite_amount_is_admitted() {
        probe("(amount: 25.0)").expect("an ordinary authored fill is admitted");
        // ⛔ AND A NEGATIVE ONE IS TOO. It is finite, so this check has nothing
        // to say about it — whether `smash.fill_meter` may DRAIN a meter is a
        // domain question for that technique's own validator, and conflating the
        // two would make this refusal mean something it cannot enforce
        // generally.
        probe("(amount: -25.0)")
            .expect("a negative fill is finite, so finiteness is not the check that refuses it");
    }

    /// ⛔ NESTED, because the params that matter are tuples and structs. A
    /// walk that only looked at top-level fields would pass every authored
    /// `half_extents`, `offset` and `launch_dir` in the game.
    #[test]
    fn a_nonfinite_number_nested_in_a_tuple_is_found() {
        let params = ParamValue::parse(
            "(item_id: \"x\", fuse_s: 2.0, damage: 4, blast_radius: 24.0, \
             impact_speed: 120.0, half_extents: (8.0, NaN), offset: (0.0, 0.0))",
        )
        .expect("valid RON");
        let found = params.nonfinite_fields();
        assert_eq!(
            found,
            vec!["half_extents[1]".to_string()],
            "a NaN inside a tuple has to be found and NAMED by its position"
        );
    }

    /// ⭐ AN INTEGER CAN NEVER FAIL THIS, and saying so is not padding: the walk
    /// converts every `ron::Number` through `into_f64`, so a bug there would
    /// reject `damage: 4` and take the whole roster down with it.
    #[test]
    fn integers_are_never_a_finding() {
        let params = ParamValue::parse(
            "(a: 0, b: -9000, c: 4294967295, d: 1.5)",
        )
        .expect("valid RON");
        assert!(
            params.nonfinite_fields().is_empty(),
            "an authored integer was reported as non-finite"
        );
    }
}

/// ⛔⛔ **A DOMAIN RULE THAT ONLY THE RUST AUTHORING ROAD ASKS IS NOT A RULE.**
///
/// `SteeredBoltParams` had three real constraints — a bolt must draw something,
/// its trail must be redrawn at some interval, and it must be steerable — and all
/// three lived as `assert!`s inside `author_steered_bolt`, the helper that Rust
/// content calls. The smash composition declared the technique with
/// `check_hydrates::<SteeredBoltParams>`, so a bolt arriving as an ordinary
/// `EffectRef` was checked for nothing but whether serde could build the struct.
///
/// ⇒ `SteeredBoltParams::problems` is now the one authority and BOTH roads ask
/// it: the helper asserts on it, the declaration refuses on it. That is what
/// makes these tests worth having — they hold the two roads to the same answer.
mod bolt_domain_rules {
    use ambition_entity_catalog::smash_bolt::{check_steered_bolt_params, SteeredBoltParams};
    use ambition_entity_catalog::ParamValue;

    fn ok_params() -> SteeredBoltParams {
        SteeredBoltParams {
            trail_vfx: "spark_row".to_string(),
            trail_every_s: 0.05,
            damage: 9,
            radius: 10.0,
            knockback: 1.0,
            turn_rate_deg: 240.0,
            speed: 300.0,
            lifetime_s: 3.0,
            self_launch: 0.0,
            offset: (18.0, 0.0),
        }
    }

    fn refusal_for(edit: impl FnOnce(&mut SteeredBoltParams)) -> String {
        let mut params = ok_params();
        edit(&mut params);
        let value = ParamValue::from_typed(&params).expect("params serialize");
        check_steered_bolt_params(&value)
            .expect_err("the declaration admitted a bolt its own authoring road panics on")
    }

    /// ⭐ THE CONTROL FIRST, because every arm below is an `expect_err` and all
    /// of them pass against a predicate that refuses everything.
    #[test]
    fn an_ordinary_bolt_is_admitted() {
        let value = ParamValue::from_typed(&ok_params()).expect("params serialize");
        check_steered_bolt_params(&value).expect("an ordinary authored bolt is admitted");
        assert!(
            ok_params().problems().is_empty(),
            "the fixture the other arms edit is itself invalid, so each of them \
             could be passing for the wrong reason"
        );
    }

    #[test]
    fn a_bolt_that_draws_nothing_is_refused() {
        let said = refusal_for(|p| p.trail_vfx = "   ".to_string());
        assert!(
            said.contains("trail_vfx"),
            "the refusal must name the field: {said}"
        );
    }

    #[test]
    fn a_trail_redrawn_never_is_refused() {
        for bad in [0.0, -1.0] {
            let said = refusal_for(|p| p.trail_every_s = bad);
            assert!(
                said.contains("trail_every_s"),
                "a trail interval of {bad} was not refused by name: {said}"
            );
        }
    }

    #[test]
    fn a_bolt_nobody_can_steer_is_refused() {
        for bad in [0.0, -30.0] {
            let said = refusal_for(|p| p.turn_rate_deg = bad);
            assert!(
                said.contains("turn_rate_deg"),
                "a turn rate of {bad} was not refused by name: {said}"
            );
        }
    }

    /// ⛔ AND THE TWO ROADS AGREE. The helper's assert and the declaration's
    /// refusal read the same `problems()`, so a rule cannot be added to one and
    /// forgotten in the other — this is the arm that fails if somebody
    /// re-inlines a check into either side.
    #[test]
    fn the_authoring_road_refuses_exactly_what_the_declaration_refuses() {
        for (name, edit) in [
            ("trail_vfx", Box::new(|p: &mut SteeredBoltParams| p.trail_vfx = String::new())
                as Box<dyn FnOnce(&mut SteeredBoltParams)>),
            ("trail_every_s", Box::new(|p: &mut SteeredBoltParams| p.trail_every_s = 0.0)),
            ("turn_rate_deg", Box::new(|p: &mut SteeredBoltParams| p.turn_rate_deg = 0.0)),
        ] {
            let mut params = ok_params();
            edit(&mut params);
            let value = ParamValue::from_typed(&params).expect("params serialize");
            assert!(
                !params.problems().is_empty(),
                "{name}: the authoring road accepts it"
            );
            assert!(
                check_steered_bolt_params(&value).is_err(),
                "{name}: the declaration accepts what the authoring road panics on"
            );
        }
    }
}

/// I2 step 4/6: a MOVE-ONLY edit reaches the live cast, through the road that
/// already exists.
mod moveset_revision {
    use super::*;
    use crate::prepared::{
        activate_staged_revision, close_preparation_barrier_without_admission,
        revise_staged_moveset, stage_authored_character, CharacterPreparationPlugin,
        MovesetRevisionError, PreparedCharacterRegistry, RevisionOutcome,
    };
    use crate::prepared_fixtures::moveset_with;
    use ambition_entity_catalog::TechniqueSupport;

    fn plain(id: &str, move_id: &str) -> CharacterDefinition {
        CharacterDefinition::new(id, id, "test_demo").with_moveset(moveset_with(
            &[("attack", move_id)],
            vec![crate::prepared_fixtures::slash(move_id, "cue", "land")],
        ))
    }

    /// An app whose cast went through the REAL barrier, which is what populates
    /// the staged source a move-only revision reads.
    fn app_with_a_prepared_cast() -> bevy::app::App {
        let mut app = bevy::app::App::new();
        app.add_plugins(CharacterPreparationPlugin);
        stage_authored_character(&mut app, plain("brawler", "jab"), &CharacterBindings::default())
            .expect("the cast stages");
        close_preparation_barrier_without_admission(app.world_mut());
        app
    }

    fn live_display_name(app: &bevy::app::App) -> String {
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .iter()
            .find(|(id, _)| id == &"brawler")
            .map(|(_, definition)| definition.display_name.clone())
            .unwrap_or_default()
    }

    fn live_move_ids(app: &bevy::app::App) -> Vec<String> {
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .iter()
            .find(|(id, _)| id == &"brawler")
            .map(|(_, definition)| {
                definition
                    .kit
                    .projectable_moveset()
                    .map(|m| m.moves.iter().map(|mv| mv.id.clone()).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// ⭐ THE ROW. A move section arrives, one character's table is replaced, and
    /// the live cast plays the new one — without a `CharacterDefinition`, which
    /// an artifact does not carry.
    #[test]
    fn a_move_only_edit_reaches_the_live_cast() {
        let mut app = app_with_a_prepared_cast();
        // ⚠ THE PREMISE: the barrier published the cast this edit revises.
        assert_eq!(live_move_ids(&app), vec!["jab".to_string()]);

        revise_staged_moveset(
            app.world_mut(),
            "brawler",
            moveset_with(
                &[("attack", "uppercut")],
                vec![crate::prepared_fixtures::slash("uppercut", "cue", "land")],
            ),
        )
        .expect("the character is staged");
        let outcome = activate_staged_revision(app.world_mut(), &TechniqueSupport::default());

        assert!(
            matches!(outcome, RevisionOutcome::Activated { changed: 1, .. }),
            "expected one changed definition; got {outcome:?}"
        );
        assert_eq!(live_move_ids(&app), vec!["uppercut".to_string()]);
    }

    /// ⛔⛔ TWO EDITS IN A ROW COMPOSE, AND THIS IS THE ARM THE WRITE-BACK EXISTS
    /// FOR. The registry is a FOLD of the staged source; a revision that
    /// published to the registry alone would leave the source at its pre-edit
    /// value, so the SECOND edit — which reads the source — would silently revert
    /// the first. The drift is invisible until somebody revises twice, which is
    /// exactly what a content-iteration loop does all day.
    #[test]
    fn a_second_move_edit_does_not_revert_the_first() {
        let mut app = app_with_a_prepared_cast();
        for move_id in ["uppercut", "haymaker"] {
            revise_staged_moveset(
                app.world_mut(),
                "brawler",
                moveset_with(
                    &[("attack", move_id)],
                    vec![crate::prepared_fixtures::slash(move_id, "cue", "land")],
                ),
            )
            .expect("the character is staged");
            let outcome = activate_staged_revision(app.world_mut(), &TechniqueSupport::default());
            assert!(
                matches!(outcome, RevisionOutcome::Activated { .. }),
                "edit `{move_id}` did not activate: {outcome:?}"
            );
        }
        assert_eq!(
            live_move_ids(&app),
            vec!["haymaker".to_string()],
            "the second edit did not stick — the staged source was left at its \
             pre-revision value, so the fold rebuilt the cast from stale truth"
        );
    }

    /// ⛔⛔ A MOVE EDIT MUST NOT REVERT AN EARLIER, UNRELATED REVISION — and
    /// THIS is the arm the write-back exists for.
    ///
    /// ⚠ **MY FIRST ATTEMPT AT IT WAS UNFALSIFIABLE.** Two successive MOVE edits
    /// compose with or without the write-back, because each one REPLACES the
    /// moveset wholesale and never reads the stale value. The poison did not
    /// fire, which is a finding about the fixture: a safeguard needs a case that
    /// READS the thing it keeps in step.
    ///
    /// ⇒ So the first revision changes something else — the display name,
    /// through the ordinary whole-definition road — and the move edit then has to
    /// carry it forward. Without the write-back the staged source still holds the
    /// OLD name, `revise_staged_moveset` builds its edit from that, and the
    /// fold republishes the old name: a user-visible revert nobody asked for.
    #[test]
    fn a_move_edit_carries_an_earlier_revision_forward() {
        use crate::prepared::stage_character_revision;

        let mut app = app_with_a_prepared_cast();
        let renamed = CharacterDefinition::new("brawler", "The Brawler", "test_demo")
            .with_moveset(moveset_with(
                &[("attack", "jab")],
                vec![crate::prepared_fixtures::slash("jab", "cue", "land")],
            ));
        stage_character_revision(&mut app, renamed, &CharacterBindings::default())
            .expect("the rename stages");
        let outcome = activate_staged_revision(app.world_mut(), &TechniqueSupport::default());
        assert!(
            matches!(outcome, RevisionOutcome::Activated { .. }),
            "the rename did not activate: {outcome:?}"
        );
        assert_eq!(live_display_name(&app), "The Brawler");

        revise_staged_moveset(
            app.world_mut(),
            "brawler",
            moveset_with(
                &[("attack", "uppercut")],
                vec![crate::prepared_fixtures::slash("uppercut", "cue", "land")],
            ),
        )
        .expect("the character is staged");
        activate_staged_revision(app.world_mut(), &TechniqueSupport::default());

        assert_eq!(live_move_ids(&app), vec!["uppercut".to_string()]);
        assert_eq!(
            live_display_name(&app),
            "The Brawler",
            "a MOVE edit reverted an earlier rename: it was built from a staged \
             source the previous revision never updated, so the registry and its \
             own source had drifted apart"
        );
    }

    /// ⛔⛔ A HALF-APPLIED PACK IS A CAST NOBODY AUTHORED.
    ///
    /// A loadable artifact's move section is ONE thing an author shipped.
    /// Staging its ids one at a time would leave the readable prefix applied
    /// when a later id turns out to be unknown — the same failure the artifact
    /// envelope refuses a duplicate section for. `stage_move_section` checks
    /// every id BEFORE staging any, and this is the arm that says so: the good
    /// id in the pack must NOT reach the live cast.
    #[test]
    fn a_section_naming_one_unknown_character_stages_none_of_it() {
        use ambition_entity_catalog::move_section::MoveSectionData;

        let mut app = app_with_a_prepared_cast();
        let mut section = MoveSectionData::new();
        section.insert(
            "brawler".to_string(),
            moveset_with(
                &[("attack", "uppercut")],
                vec![crate::prepared_fixtures::slash("uppercut", "cue", "land")],
            ),
        );
        section.insert("somebody_else".to_string(), moveset_with(&[], Vec::new()));

        let refusals = crate::prepared::stage_move_section(app.world_mut(), &section);
        assert_eq!(
            refusals,
            vec![MovesetRevisionError::UnknownCharacter("somebody_else".to_string())]
        );

        // ⚠ THE HALF THAT MATTERS: the GOOD id in the same pack is not staged,
        // so activating now changes nothing.
        let outcome = activate_staged_revision(app.world_mut(), &TechniqueSupport::default());
        assert!(
            matches!(outcome, RevisionOutcome::NothingStaged),
            "a refused pack left something staged: {outcome:?}"
        );
        assert_eq!(
            live_move_ids(&app),
            vec!["jab".to_string()],
            "the readable half of a refused pack reached the live cast"
        );
    }

    /// ⭐ THE CONTROL: a section every id of which is known DOES apply, whole.
    #[test]
    fn a_section_whose_characters_are_all_known_applies() {
        use ambition_entity_catalog::move_section::MoveSectionData;

        let mut app = app_with_a_prepared_cast();
        let mut section = MoveSectionData::new();
        section.insert(
            "brawler".to_string(),
            moveset_with(
                &[("attack", "uppercut")],
                vec![crate::prepared_fixtures::slash("uppercut", "cue", "land")],
            ),
        );

        assert_eq!(
            crate::prepared::stage_move_section(app.world_mut(), &section),
            Vec::new()
        );
        activate_staged_revision(app.world_mut(), &TechniqueSupport::default());
        assert_eq!(live_move_ids(&app), vec!["uppercut".to_string()]);
    }

    /// ⭐⭐ THE WHOLE LOOP: ENCODED BYTES CHANGE WHAT A LIVE FIGHTER PLAYS.
    ///
    /// I2's point is that a move edit should not need the host recompiled. This
    /// arm goes from a move-section PAYLOAD — the thing an out-of-workspace
    /// builder emits, text this crate never produced — through the codec, the
    /// all-or-nothing staging and the existing revision road, to the published
    /// cast. Nothing between the payload and the fighter is a compile step.
    ///
    /// ⚠ It stops short of I2's full claim ("a PREBUILT host plays it without
    /// invoking Cargo"): this is one process that already linked the engine.
    /// What it establishes is that the DATA path is complete and the values
    /// survive it.
    #[test]
    fn a_move_section_payload_changes_what_the_live_cast_plays() {
        use ambition_entity_catalog::move_section::{decode, encode, MoveSectionData};

        let mut app = app_with_a_prepared_cast();
        assert_eq!(live_move_ids(&app), vec!["jab".to_string()]);

        // The payload, built the way a builder builds it and then thrown away as
        // a VALUE — only the text crosses into the host below.
        let payload = {
            let mut section = MoveSectionData::new();
            section.insert(
                "brawler".to_string(),
                moveset_with(
                    &[("attack", "uppercut")],
                    vec![crate::prepared_fixtures::slash("uppercut", "cue", "land")],
                ),
            );
            encode(&section).expect("the section encodes")
        };
        assert!(
            payload.contains("uppercut"),
            "the payload does not carry the edited move, so this fixture is not \
             about the payload: {payload}"
        );

        let section = decode(&payload).expect("the payload decodes");
        assert_eq!(
            crate::prepared::stage_move_section(app.world_mut(), &section),
            Vec::new()
        );
        activate_staged_revision(app.world_mut(), &TechniqueSupport::default());

        assert_eq!(
            live_move_ids(&app),
            vec!["uppercut".to_string()],
            "text that named a new move reached the cast and the fighter still \
             plays the compiled one"
        );
    }

    /// ⚠ AN UNRESOLVED REFERENCE IS REFUSED, NOT INVENTED. A move edit naming a
    /// character this build did not prepare must not conjure a fighter to hang
    /// it on.
    #[test]
    fn an_edit_for_a_character_this_build_never_prepared_is_refused() {
        let mut app = app_with_a_prepared_cast();
        let outcome = revise_staged_moveset(
            app.world_mut(),
            "somebody_else",
            moveset_with(&[], Vec::new()),
        );
        assert_eq!(
            outcome,
            Err(MovesetRevisionError::UnknownCharacter("somebody_else".to_string()))
        );
        assert_eq!(live_move_ids(&app), vec!["jab".to_string()]);
    }
}
