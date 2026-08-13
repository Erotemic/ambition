//! One registration, one prepared authority (§7.6).

use super::*;
use crate::character_runtime::CharacterLoadDemand;
use ambition_entity_catalog::{
    ClipBinding, HitVolume, HurtboxDoc, HurtboxKeyframe, HurtboxTimeline, HurtboxVolume, MoveEvent,
    MoveEventKind, MoveGates, MoveSpec, MoveWindow, MovesetContract, VolumeShape, WindowTag,
};
use ambition_platformer2d_shared_tangle::app_finalization::finalize;
use ambition_platformer2d_shared_tangle::binding::Namespace;
use bevy::prelude::App;
use std::collections::BTreeMap;

/// A move that emits one cue and carries one strike sound on its hit volume.
fn slash(id: &str, cue: &str, strike: &str) -> MoveSpec {
    MoveSpec {
        landing_lag_s: None,
        autocancel_after_s: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.4,
        events: vec![MoveEvent {
            at_s: 0.1,
            kind: MoveEventKind::Sfx {
                cue: cue.to_string(),
            },
        }],
        windows: vec![MoveWindow {
            start_s: 0.1,
            end_s: 0.2,
            tag: WindowTag::Active,
            volumes: vec![HitVolume {
                shape: VolumeShape::Rect {
                    offset: (10.0, 0.0),
                    half_extents: (8.0, 8.0),
                },
                damage: 1,
                knockback: 0.0,
                knockback_growth: 0.0,
                launch_dir: None,
                on_hit: None,
                vfx: Some("slash_arc".to_string()),
                hit_sfx: Some(strike.to_string()),
            }],
            sustain_effect: None,
            motion_scale: 1.0,
        }],
        gates: MoveGates { grounded: None },
        start_impulse: None,
        smash_charge_mult: 1.0,
    }
}

fn moveset_with(verbs: &[(&str, &str)], moves: Vec<MoveSpec>) -> MovesetContract {
    MovesetContract {
        verbs: verbs
            .iter()
            .map(|(v, m)| (v.to_string(), m.to_string()))
            .collect::<BTreeMap<_, _>>(),
        moves,
    }
}

fn mary_o() -> CharacterDefinition {
    CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo")
        .with_sheet("super_mary_o_spritesheet")
        .with_moveset(moveset_with(
            &[("attack", "stomp")],
            vec![slash("stomp", "mary_o.stomp", "mary_o.stomp.land")],
        ))
}

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

/// **Registration DECLARES. It does not load.**
///
/// This test used to assert the opposite — that registering demanded the art —
/// on the reasoning that a provider should not need a second call. The reasoning
/// inverts as the plan succeeds: once fifty fighters are registered through this
/// seam, merely installing their providers would demand fifty sheets, which is
/// the startup decode storm §7.1 deleted, rebuilt from the other end.
///
/// Loading is driven by what a session STAGES (`StagesCharacters` — a room plan,
/// a match roster, a startup spec, a worn identity), never by what exists.
#[test]
fn registration_declares_without_demanding_art() {
    let mut app = App::new();
    app.register_character(mary_o());

    finalize(&mut app);
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    assert_eq!(registry.ids().collect::<Vec<_>>(), vec!["mary_o"]);
    let prepared = registry.get("mary_o").expect("published");
    assert_eq!(prepared.display_name, "Mary-O");
    assert_eq!(prepared.art_load_token(), "mary_o");

    assert!(
        app.world()
            .get_resource::<CharacterLoadDemand>()
            .is_none_or(CharacterLoadDemand::is_empty),
        "registering a character must not demand its art: a registry of what \
         EXISTS is not a list of what a session needs decoded now"
    );
}

/// **Preparation provenance survives registration.** (A5)
///
/// `PreparedCharacter::checked` names the namespaces preparation actually resolved,
/// and registration used to drop it — so after publication there was no way to
/// distinguish "these cues were verified against a real vocabulary" from "nobody
/// looked". Those must never read the same; that confusion is the whole reason the
/// binding boundary exists, and a distinction that survives only until the value is
/// stored is not a distinction.
#[test]
fn preparation_provenance_survives_registration() {
    // Registered with NO cue resolver: moves are checked, cues are not.
    let mut app = App::new();
    app.register_character(mary_o());
    finalize(&mut app);
    let unchecked = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .get("mary_o")
        .expect("published")
        .clone();
    assert!(
        unchecked.was_checked(MoveId::NAME),
        "verb targets are always resolvable from the character's own moves"
    );
    assert!(
        !unchecked.was_checked(SfxCueId::NAME),
        "no cue resolver was supplied, so cues are NOT CHECKED — which must not \
         read as 'checked and fine'"
    );

    // Registered WITH one: now the cue namespace is genuinely verified.
    let mut app = App::new();
    app.try_register_character(
        mary_o(),
        CharacterBindings::default().with_authorized_cues(["swing", "hit_flesh"]),
    )
    .expect("registers");
    finalize(&mut app);
    let checked = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .get("mary_o")
        .expect("published")
        .clone();
    assert!(
        checked.was_checked(SfxCueId::NAME),
        "a supplied resolver means the cues WERE checked, and the published value \
         must say so"
    );
}

/// The cast's cue inventory is the union over prepared characters — and §4.6 is
/// explicit that this is the cast's CONTRIBUTION, not a session's whole set.
#[test]
fn the_cast_cue_inventory_is_the_union_over_prepared_characters() {
    let mut app = App::new();
    app.register_character(mary_o());
    app.register_character(
        CharacterDefinition::new("sanic", "Sanic", "sanic_demo").with_moveset(moveset_with(
            &[("attack", "roll")],
            vec![slash("roll", "sanic.roll", "sanic.roll.hit")],
        )),
    );

    finalize(&mut app);
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    assert_eq!(
        registry
            .cast_cue_dependencies()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![
            "mary_o.stomp",
            "mary_o.stomp.land",
            "sanic.roll",
            "sanic.roll.hit"
        ]
    );
}

/// A stable id is what saves, replays, and peers key on, so two providers
/// claiming one is a rename, not a merge — and the loser leaves the authority
/// untouched.
#[test]
fn two_providers_cannot_author_the_same_stable_id() {
    let mut app = App::new();
    app.register_character(mary_o());
    let error = app
        .try_register_character(
            CharacterDefinition::new("mary_o", "Impostor", "other_provider"),
            CharacterBindings::default(),
        )
        .err()
        .expect("a duplicate stable id must be refused");
    assert_eq!(
        error,
        CharacterRegistrationError::DuplicateId {
            character_id: "mary_o".to_string(),
            first_provider: "mary_o_demo".to_string(),
            second_provider: "other_provider".to_string(),
        }
    );
    finalize(&mut app);
    assert_eq!(
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .get("mary_o")
            .map(|c| c.display_name.as_str()),
        Some("Mary-O"),
        "the rejected registration must leave the previous authority active"
    );
}

/// §4.7: a definition describes a BODY. **The CURRENT controller is a session
/// binding**, so there is nowhere on this type to put one — asserted structurally
/// by the fact that a full definition is constructible without one.
///
/// ⚠ **the invariant was narrowed by Jon on 2026-08-10, and this doc used to
/// state the wider one.** It said a definition may carry no brain at all. His
/// character-template ruling distinguishes the two: a definition MAY name a
/// default autonomous PROFILE — what this character normally does when nothing
/// overrides it — and may not name who is driving right now. *"Possessing a
/// Goblin changes who drives the Goblin. It does not change what a Goblin is."*
/// ⇒ the seam for that default is `resolve_initial_brain`'s
/// `definition_default` parameter (queue D73 phase 1); when the field lands on
/// this type, add it to the destructure with that reasoning rather than
/// deleting this test — the rule it guards, no CURRENT controller, still holds.
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
        // A DEFAULT autonomous profile, which §4.7 now permits and Jon's
        // 2026-08-10 ruling authorised: what this character does when nobody is
        // driving it. The CURRENT controller is still nowhere on this type.
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
        // ⚠ **a DEFAULT policy, which §4.7 permits and Jon's ruling authorised,
        // and the reason this test survives rather than being deleted.** What
        // it guards is that the CURRENT controller is nowhere on this type: a
        // character may say what it does when nobody is driving it, and may not
        // say who is driving it now.
        autonomous_profile: _,
        // The same authority by NAME instead of by value — a shared policy
        // several characters point at. Still a character fact: it says what this
        // creature does when nobody drives it, not who is driving it.
        autonomous_profile_ref: _,
        ranged_vfx: _,
        // Authored on the character since 2026-08-11; the DEFAULT is
        // `MovesetVerb`, so an unmigrated character keeps behaving as it did.
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
    } = def;
}

/// **A12.** Sheets, portraits and the DERIVED vfx inventory are resolved too.
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

/// **A PORTRAIT TARGET NOBODY AUTHORED IS NAMED — and until 2026-08-12 it was
/// not, in any composition** (ledger D106).
///
/// ⛔⛔ **the test above is titled *"Sheets, PORTRAITS and the derived vfx
/// inventory are resolved too"* and only ever asserted sheets.** That is how the
/// gap survived: the claim was in the name, the coverage was not, and a reader
/// checking whether portraits were handled would have found a test saying yes.
///
/// The mechanism was there — `with_available_portraits` populates the resolver
/// and `checked()` reports it — and nothing ever called it, so `self.portraits`
/// was `None` everywhere and preparation's honest *"we did not look"* was the
/// permanent answer.
///
/// ⚠ **this asserts the SEAM, not the resolver.** It goes through
/// `try_register_character`, which is where `with_engine_vocabularies` is
/// applied — a test that passed the resolver by hand would prove the check works
/// while leaving it disconnected, which is exactly the state this replaces.
#[test]
fn a_portrait_target_nobody_authored_is_named_at_registration() {
    let mut definition = mary_o();
    definition.portrait = Some("no_such_portrait_target".to_string());

    let mut app = App::new();
    app.register_character(definition);
    finalize(&mut app);

    let prepared = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .get("mary_o")
        .expect("published");
    assert!(
        prepared.was_checked(PortraitTarget::NAME),
        "the registration seam did not supply a portrait vocabulary, so nothing \
         looked — which is the D106 state, not a pass"
    );

    // ⛔⛔ **`was_checked` ALONE WAS THE WHOLE TEST until 2026-08-12, and GPT 5.6
    // named it.** "Somebody looked" and "somebody found the typo" are different
    // claims, and only the second is what a content author gets value from — a
    // vocabulary wired up but resolving nothing would satisfy the assertion
    // above perfectly. The vfx twin below has always asserted both halves.
    let unresolved: Vec<&str> = prepared.unresolved_references().collect();
    assert!(
        unresolved
            .iter()
            .any(|line| line.contains("no_such_portrait_target")),
        "a portrait target nobody authored must be REPORTED, naming the bad id \
         so the author can act on it. Got: {unresolved:#?}"
    );
    assert!(
        unresolved.iter().any(|line| line.contains("did you mean")),
        "and the report must carry a suggestion — the vocabulary is a fixed list \
         of a few dozen targets, so a near-miss is the ordinary case and the \
         nearest one is the useful half of the diagnostic. Got: {unresolved:#?}"
    );
}

/// **THE POISON for the portrait check: a target that DOES exist resolves
/// clean.**
///
/// ⛔ without this, the test above passes on a vocabulary that rejects
/// everything — including a build where `available_portrait_targets()` came back
/// empty and every shipped character was suddenly "unresolved". An absence
/// assertion needs the presence case beside it or it is measuring the resolver
/// being broken.
#[test]
fn a_portrait_target_the_engine_bakes_resolves_clean() {
    let target = ambition_sprite_sheet::portrait::available_portrait_targets()
        .first()
        .copied()
        .expect(
            "the engine bakes portrait targets at build time; an empty vocabulary \
             is itself the failure this test exists to distinguish from a typo",
        )
        .to_string();

    let mut definition = mary_o();
    definition.portrait = Some(target.clone());

    let mut app = App::new();
    app.register_character(definition);
    finalize(&mut app);

    let prepared = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .get("mary_o")
        .expect("published");
    assert!(prepared.was_checked(PortraitTarget::NAME));
    let unresolved: Vec<&str> = prepared.unresolved_references().collect();
    assert!(
        !unresolved.iter().any(|line| line.contains(&target)),
        "`{target}` is a target the engine itself bakes, and preparation called \
         it unknown — so the vocabulary the registration seam supplies is not the \
         one the renderer draws from. Got: {unresolved:#?}"
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

/// **A display name is an addressing key whether or not it was meant to be one.**
///
/// Rooms author `enemy.name`, interactables author `character_id`, rosters author
/// labels — and three authorities resolve those labels independently:
/// `PreparedCharacterRegistry::id_for_display_name` takes the first match in id
/// order, the catalog takes the first match in ITS order, and
/// `CharacterSpriteAssets::declare` inserts into a map so the LAST declaration wins.
/// With two "Hero"s, a demand for "Hero" could stage `alpha`, authorize `alpha`'s
/// provider, and decode `zeta`'s sheet: one character's sounds on another's body.
///
/// So the ambiguity is refused at the seam, rather than each resolver being taught
/// to break the tie the same way — which is the arrangement that produced the split
/// in the first place.
#[test]
fn two_characters_cannot_present_under_the_same_display_name() {
    let mut app = App::new();
    app.register_character(CharacterDefinition::new("alpha", "Hero", "provider_a"));
    let error = app
        .try_register_character(
            CharacterDefinition::new("zeta", "Hero", "provider_b"),
            CharacterBindings::default(),
        )
        .err()
        .expect("an ambiguous display name must be refused");
    assert_eq!(
        error,
        CharacterRegistrationError::AmbiguousDisplayName {
            display_name: "Hero".to_string(),
            first_id: "alpha".to_string(),
            second_id: "zeta".to_string(),
        }
    );

    finalize(&mut app);
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    assert_eq!(
        registry.id_for_display_name("Hero"),
        Some("alpha"),
        "the rejected registration leaves the first character addressable"
    );
    assert!(
        registry.get("zeta").is_none(),
        "and does not publish the second"
    );
}

/// Re-registering the SAME character is a duplicate id, not an ambiguous name.
///
/// Both checks look at the display name, and ordering them wrongly would report
/// `alpha` as ambiguous with itself — a confusing message for the ordinary mistake
/// of registering one character twice.
#[test]
fn re_registering_one_character_still_reports_the_duplicate_id() {
    let mut app = App::new();
    app.register_character(CharacterDefinition::new("alpha", "Hero", "provider_a"));
    let error = app
        .try_register_character(
            CharacterDefinition::new("alpha", "Hero", "provider_b"),
            CharacterBindings::default(),
        )
        .err()
        .expect("a duplicate id must be refused");
    assert!(matches!(
        error,
        CharacterRegistrationError::DuplicateId { .. }
    ));
}

/// A verb the runtime cannot press is named at preparation. (queue L10)
///
/// The dangling-move-id check has always covered "the verb points at nothing".
/// This is the other side: the move exists, the binding is well-formed, and the
/// VERB is a word the trigger path never asks for — so the move is authored,
/// prepared, projected onto the body, and never triggered by anything.
///
/// Worth a check rather than a convention because the failure has no symptom.
/// A character with an unreachable move reads exactly like a character with no
/// moves, which is a legitimate thing to be.
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

/// **A ranged move needs something to throw, and the two halves live apart.**
///
/// The projectile specification is on the ACTION SET; the move that fires it is
/// on the MOVESET. Once a definition can author both (C3 precedence), it can
/// author a `ranged` verb and an action set with no ranged payload — and each
/// half is individually valid. The verb is real, the move is real, the set is
/// real, and the button does nothing.
///
/// Only preparation holds both, so only preparation can see it (GPT 5.6,
/// 2026-07-28).
#[test]
fn an_authored_ranged_move_with_no_ranged_payload_is_reported() {
    use ambition_characters::brain::ActionSet;

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
    use ambition_characters::brain::action_set::{RangedActionSpec, RangedStyle};
    use ambition_characters::brain::ActionSet;

    let armed = CharacterDefinition::new("gunner", "Gunner", "demo")
        .with_action_set(ActionSet {
            ranged: Some(RangedActionSpec {
                style: RangedStyle::default(),
                speed: 300.0,
                damage: 1,
                flight: None,
                visual: None,
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

/// **A body cannot end up with two owners of one press.** (GPT 5.6)
///
/// A character that takes the host-code kit gets the host's charge-projectile
/// mechanic on its ranged press. If its authored moveset also binds a ranged
/// verb, both fire — the charge path because the kit installed it, the moveset
/// because the verb resolves. The first version of this guard REPORTED the
/// contradiction and published the kit anyway, which named the bug without
/// preventing it.
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

/// **A cast has a version, so a derivation can know it went stale.** (X4)
///
/// Nothing downstream could say WHICH cast a body's kit came from. "This was
/// built before the cast changed" was not a question the code could ask; it
/// could only compare values and guess, which is the shape of every
/// stale-derivation bug in this repo.
///
/// ⚠ this used to describe the registry as "a live resource — a room transition
/// builds a fresh one and registration mutates it in place". Since the
/// finalization barrier (2026-07-29) it is published once, whole, and a late
/// registration panics. The generation is per PUBLICATION now; the hatch this
/// test uses stamps each insert as its own publication because a test has no
/// barrier to publish for it.
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

/// **SEVERAL BODIES, ONE POLICY — which is the sentence Group B and Group C
/// were missing.** (ledger D80)
///
/// A character could carry a `BrainProfile` by VALUE, or name a catalog
/// `BrainPreset` by key — two vocabularies read by two different roads, so
/// "these five creatures fight alike" was expressible for the NPC road and not
/// for the enemy road. That is why `medium_striker` exists as a whole-BODY
/// archetype worn by five goblins, a lab raider and a skitter: sharing the
/// decision-making meant sharing the body too.
///
/// ⚠ the fixture gives the two characters DIFFERENT bodies and the same policy,
/// **A prepared character already knows whether it flies.**
///
/// ⛔ the constructor used to ask `catalog.body_kind(id)` for this, which is a
/// constructor rediscovering what the character IS (Jon's redirect §14) — and
/// the fact was fully determined at preparation, which holds the catalog anyway.
/// A "prepared" definition that still needs a second lookup to answer a body
/// question is partly prepared, and every caller has to remember the second
/// half.
///
/// ⛔⛔ **AND THE CATALOG NO LONGER FILLS IT** (ledger D89, 2026-08-11). This
/// test used to assert *fill-never-overrule*: `body_kind: Floating` supplied
/// flight for a character that "did not say". That fold is DELETED, because
/// `body_kind` is presentation/footprint vocabulary — it answers *how tall is
/// this* (`default_standing_height`), and a `Floating` row was quietly deciding
/// that a body ignores gravity as well.
///
/// ⚠ **the §14 intent it was written for is unchanged and is what this still
/// pins**: a PREPARED character carries one concrete answer, so no constructor
/// asks the catalog a second time. Only the source of the answer moved — from
/// the catalog row to the character itself.
#[test]
fn gravity_freedom_is_resolved_at_preparation_rather_than_at_construction() {
    use ambition_characters::actor::character_catalog::CharacterCatalog;

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
    let catalog = CharacterCatalog::from_data(
        ambition_characters::actor::character_catalog::parse_catalog(CATALOG),
    );
    let walking = ambition_characters::actor::CharacterLocomotion {
        run_speed: 90.0,
        ..Default::default()
    };
    let prepared = crate::character_runtime::prepare_and_finalize_against_for_test(
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
            // ⚠ `Some(false)`, not `None`: preparation RESOLVES the question
            // even when the answer is "no". A `None` reaching a body would mean
            // the barrier left it open for a constructor to rediscover, which is
            // exactly what §14 deletes.
            == Some(false),
        "this character said nothing about flight and its catalog row says \
         `Floating` — which is a SILHOUETTE claim. A row deciding locomotion is \
         the coupling D89 cut; preparation must resolve silence to GROUNDED and \
         carry that one concrete answer"
    );

    // ⭐ **and a character that DOES say it flies keeps that**, which is the
    // other half: cutting the catalog's fold must not also stop a bird flying.
    let stated = crate::character_runtime::prepare_and_finalize_against_for_test(
        CharacterDefinition::new("floater", "Floater", "test").with_locomotion(
            ambition_characters::actor::CharacterLocomotion {
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
    let grounded = crate::character_runtime::prepare_and_finalize_against_for_test(
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
        // ⚠ **`Some(false)`, and the difference is the whole three-state.** An
        // unknown id has no catalog answer to fold, so preparation resolves the
        // silence to "does not fly" — and a reader must not have to tell that
        // apart from "nobody said".
        Some(false),
    );
}

/// **Completeness is a NAMED contract, not an inferred bool.**
///
/// ⛔ this replaces `is_complete_body()` (Jon's redirect §5), which answered
/// `locomotion.is_some()`. That was true by coincidence — a character that had
/// stated its top speed had, so far, also stated everything else — and the day
/// it stopped being true nothing would have said so: the body would have taken
/// the archetype road and looked unmigrated rather than incomplete.
///
/// Two terms, both observed: an incomplete character NAMES what it is missing,
/// and a complete one hands over a blueprint carrying the facts construction
/// actually reads.
#[test]
fn an_incomplete_character_names_the_fact_it_is_missing() {
    let bare = crate::character_runtime::prepare_and_finalize_for_test(
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

    let whole = crate::character_runtime::prepare_and_finalize_for_test(
        CharacterDefinition::new("goblin", "Goblin", "test")
            .with_locomotion(ambition_characters::actor::CharacterLocomotion {
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
/// namespaced `provider::local_name` by `CharacterCatalogRegistry`.
const SHARED_POLICY_CATALOG: &str = r#"(
    autonomous_profiles: {
        // ⚠ NAMESPACED, because assembly namespaces every fragment key
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
    use ambition_characters::actor::character_catalog::CharacterCatalog;
    use ambition_characters::brain::CharacterBrainTemplate;

    const CATALOG: &str = SHARED_POLICY_CATALOG;
    let catalog = CharacterCatalog::from_data(
        ambition_characters::actor::character_catalog::parse_catalog(CATALOG),
    );

    let named = |id: &str, run_speed: f32| {
        crate::character_runtime::prepare_and_finalize_against_for_test(
            CharacterDefinition::new(id, id, "test")
                .with_locomotion(ambition_characters::actor::CharacterLocomotion {
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

/// **Inline AND named is a REFUSAL, not a precedence rule** (Jon's redirect §9).
///
/// ⛔ this assertion is the INVERSE of the one that stood here until
/// 2026-08-11, and the reversal is the point. The old behaviour — inline wins,
/// named is the fallback — was documented as "specialization", and nothing
/// merged: the shared policy was discarded whole. An author who writes both
/// wants a patch type that does not exist, and telling them so is cheaper than
/// silently answering half their question.
#[test]
#[should_panic(expected = "authors an inline autonomous profile AND names")]
fn authoring_a_policy_twice_is_refused_rather_than_ranked() {
    use ambition_characters::actor::character_catalog::CharacterCatalog;
    use ambition_characters::brain::CharacterBrainTemplate;

    let catalog = CharacterCatalog::from_data(
        ambition_characters::actor::character_catalog::parse_catalog(SHARED_POLICY_CATALOG),
    );
    let _ = crate::character_runtime::prepare_and_finalize_against_for_test(
        CharacterDefinition::new("statue", "Statue", "test")
            .with_autonomous_profile_named("striker")
            .with_autonomous_profile(ambition_characters::brain::BrainProfile {
                template: CharacterBrainTemplate::StandStill,
                ..Default::default()
            }),
        &CharacterBindings::default(),
        Some(&catalog),
    );
}

/// **A named policy nobody authored is a preparation failure.**
///
/// ⛔ also an inversion: this used to assert `None`, on the reading that a miss
/// means the same as saying nothing. It does not. Saying nothing leaves the
/// archetype in charge on purpose; naming a policy that does not exist leaves
/// the archetype in charge while the content file says the opposite — the
/// explicit-`CharacterId` mistake, one layer down.
#[test]
#[should_panic(expected = "is not published")]
fn a_named_policy_that_does_not_exist_is_a_failure_rather_than_silence() {
    use ambition_characters::actor::character_catalog::CharacterCatalog;

    let catalog = CharacterCatalog::from_data(
        ambition_characters::actor::character_catalog::parse_catalog(SHARED_POLICY_CATALOG),
    );
    let _ = crate::character_runtime::prepare_and_finalize_against_for_test(
        CharacterDefinition::new("ghost", "Ghost", "test")
            .with_autonomous_profile_named("no_such_policy"),
        &CharacterBindings::default(),
        Some(&catalog),
    );
}
