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
