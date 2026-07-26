//! One registration, one prepared authority (§7.6).

use super::definition::*;
use super::{CharacterLoadDemand, PreparedCharacterRegistry};
use ambition_entity_catalog::{
    ClipBinding, HitVolume, HurtboxDoc, HurtboxKeyframe, HurtboxTimeline, HurtboxVolume, MoveEvent,
    MoveEventKind, MoveGates, MoveSpec, MoveWindow, MovesetContract, VolumeShape, WindowTag,
};
use ambition_platformer_primitives::binding::Namespace;
use bevy::prelude::App;
use std::collections::BTreeMap;

/// A move that emits one cue and carries one strike sound on its hit volume.
fn slash(id: &str, cue: &str, strike: &str) -> MoveSpec {
    MoveSpec {
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
                kb_growth: 0.0,
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
    let prepared = prepare_character(mary_o(), &CharacterBindings::default());
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

    let prepared = prepare_character(
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
    let good = prepare_character(
        mary_o(),
        &CharacterBindings::default().with_authorized_cues(authorized),
    );
    assert!(good.is_clean(), "{:?}", good.report.unresolved());
}

/// An unchecked namespace must not read as a checked one.
#[test]
fn without_an_authorized_cue_set_cues_are_reported_as_unchecked() {
    let prepared = prepare_character(mary_o(), &CharacterBindings::default());
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
    let prepared = prepare_character(broken, &CharacterBindings::default());
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
    let prepared = prepare_character(def, &CharacterBindings::default());
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
    assert_eq!(
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .get("mary_o")
            .map(|c| c.display_name.as_str()),
        Some("Mary-O"),
        "the rejected registration must leave the previous authority active"
    );
}

/// §4.7: a definition describes a BODY. Control assignment is a session binding,
/// so there is nowhere on this type to put a brain — asserted structurally by the
/// fact that a full definition is constructible without one.
#[test]
fn a_definition_carries_no_controller_binding() {
    let def = mary_o();
    // If `default_brain` is ever added, this stops compiling as written and the
    // reviewer has to justify it against §4.7.
    let CharacterDefinition {
        id: _,
        display_name: _,
        provider: _,
        lineage: _,
        sheet: _,
        portrait: _,
        body: _,
        hurtboxes: _,
        vitals: _,
        moveset: _,
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
    let unchecked = prepare_character(definition.clone(), &CharacterBindings::default());
    assert!(!unchecked.prepared.was_checked(SheetTarget::NAME));
    assert!(unchecked.is_clean());

    // Vocabulary supplied: the typo is NAMED at preparation.
    let checked = prepare_character(
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

    let prepared = prepare_character(
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
