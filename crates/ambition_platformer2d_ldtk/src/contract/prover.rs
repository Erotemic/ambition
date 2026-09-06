//! The contract is not inspected. It is RUN.
//!
//! Every claim in `ldtk_entity_contract.json` is asserted here against the real
//! [`entity_to_runtime`], and asserted in BOTH directions, because a one-way check
//! is how the two lists drifted in the first place:
//!
//! - [`every_declared_entity_converts_from_its_required_fields_alone`] builds each
//!   entity out of nothing but its declared-required fields. A parser that
//!   requires something the table does not declare fails HERE, because the minimal
//!   instance the table describes does not convert.
//! - [`every_required_field_is_refused_when_absent`] removes each declared-required
//!   field in turn. A table that claims a requirement the parser does not enforce
//!   fails HERE, because the converter shrugs.
//!
//! Neither term is reasoned. `a check that cannot fail is worse than none`, so
//! every assertion below observes an actual `Result` from the actual converter,
//! and each collects ALL its disagreements before failing so one run names the
//! whole gap.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{contract, EntityContract, FieldContract, OnInvalid, Presence};
use crate::conversion::{entity_to_runtime, LdtkVocabulary};
use crate::project::{LdtkEntityInstance, LdtkFieldInstance};
use ambition_platformer2d_core as ae;

/// The one `KinematicPath` a probe area contains, so an `EntityRef` sample has
/// something real to resolve against and its POISON has nothing.
const PROBE_PATH_IID: &str = "__probe_kinematic_path__";

/// A value no declared grammar can spell.
const POISON: &str = "__contract_poison__";

type Fields = Vec<(String, String)>;

fn instance(identifier: &str, size: [i32; 2], fields: &Fields) -> LdtkEntityInstance {
    LdtkEntityInstance {
        iid: format!("{identifier}-probe"),
        identifier: identifier.to_string(),
        pivot: vec![0.0, 0.0],
        px: [0, 0],
        width: size[0],
        height: size[1],
        field_instances: fields
            .iter()
            .map(|(name, value)| LdtkFieldInstance {
                identifier: name.clone(),
                value: Value::String(value.clone()),
                real_editor_values: Vec::new(),
            })
            .collect(),
    }
}

/// Run the REAL converter over a probe instance.
fn convert(entity: &EntityContract, fields: &Fields) -> Result<(), String> {
    let mut paths = BTreeMap::new();
    paths.insert(PROBE_PATH_IID.to_string(), "probe_path".to_string());
    entity_to_runtime(
        &instance(&entity.identifier, entity.probe_size, fields),
        ae::Vec2::new(0.0, 0.0),
        &LdtkVocabulary::engine(),
        &paths,
    )
    .map(|_| ())
}

/// Entities whose converter is not compiled into this build.
fn compiled_out(entity: &EntityContract) -> bool {
    entity.feature.as_deref() == Some("portal_ldtk") && !cfg!(feature = "portal_ldtk")
}

/// The minimal instance the contract describes: every declared-required field at
/// its sample, and nothing else.
fn base_fields(entity: &EntityContract) -> Fields {
    entity
        .fields
        .iter()
        .filter(|field| field.presence == Presence::Required)
        .filter_map(|field| {
            field
                .sample
                .clone()
                .map(|sample| (field.name.clone(), sample))
        })
        .collect()
}

fn with(fields: &Fields, name: &str, value: &str) -> Fields {
    let mut out: Fields = fields
        .iter()
        .filter(|(field, _)| field != name)
        .cloned()
        .collect();
    out.push((name.to_string(), value.to_string()));
    out
}

fn without(fields: &Fields, name: &str) -> Fields {
    fields
        .iter()
        .filter(|(field, _)| field != name)
        .cloned()
        .collect()
}

/// Everything a field needs beside itself before it can convert: its
/// `requires_fields` companions and the sibling value its `requires_value_of`
/// names.
fn preconditions(entity: &EntityContract, field: &FieldContract, base: &Fields) -> Fields {
    let mut fields = base.clone();
    for companion in &field.requires_fields {
        if let Some(sample) = entity.field(companion).and_then(|f| f.sample.clone()) {
            fields = with(&fields, companion, &sample);
        }
    }
    if let Some(condition) = &field.requires_value_of {
        fields = with(&fields, &condition.field, &condition.equals);
    }
    fields
}

/// The illegal value this field's own declared grammar rejects. `None` = the
/// field declares no grammar, so there is nothing to poison it with.
fn poison_for(field: &FieldContract) -> Option<String> {
    if let Some(explicit) = &field.poison {
        return Some(explicit.clone());
    }
    if !field.values.is_empty()
        || !field.patterns.is_empty()
        || field.entity_ref_target.is_some()
    {
        return Some(POISON.to_string());
    }
    if field.positive || field.nonzero {
        return Some("0".to_string());
    }
    if field.min_points.is_some() {
        // One point where the grammar wants two.
        return Some("0,0".to_string());
    }
    None
}

fn report(what: &str, problems: Vec<String>) {
    assert!(
        problems.is_empty(),
        "{what}\n  - {}\n\nThe contract and the converters disagree. ONE of them is \
         wrong, and the fix is in whichever one changed: edit \
         `crates/ambition_platformer2d_ldtk/ldtk_entity_contract.json` if the parser \
         is right, or the converter if the contract is.",
        problems.join("\n  - ")
    );
}

/// The direction that catches an UNDECLARED requirement.
///
/// If a converter grows a refusal the table does not know about, the minimal
/// instance the table describes stops converting, and this is where that shows up
/// — long before a level author meets it as a panic on load.
#[test]
fn every_declared_entity_converts_from_its_required_fields_alone() {
    let mut problems = Vec::new();
    for entity in &contract().entities {
        if compiled_out(entity) {
            continue;
        }
        for field in &entity.fields {
            if field.presence == Presence::Required && field.sample.is_none() {
                problems.push(format!(
                    "{}.{} is declared required but carries no `sample`, so nothing \
                     below can probe it",
                    entity.identifier, field.name
                ));
            }
        }
        let base = base_fields(entity);
        if let Err(error) = convert(entity, &base) {
            problems.push(format!(
                "{} does NOT convert from its declared-required fields alone \
                 ({base:?}): {error}",
                entity.identifier
            ));
        }
    }
    report(
        "the contract's minimal instances do not all convert:",
        problems,
    );
}

/// The direction that catches a DECLARED-BUT-UNENFORCED requirement.
///
/// This is the `character_id` failure exactly: the table says the field is
/// required, so if the converter ever stops refusing its absence the Python
/// authoring loop would go on reporting an error the runtime no longer has.
#[test]
fn every_required_field_is_refused_when_absent() {
    let mut problems = Vec::new();
    for entity in &contract().entities {
        if compiled_out(entity) {
            continue;
        }
        let base = base_fields(entity);
        for field in &entity.fields {
            if field.presence != Presence::Required {
                continue;
            }
            if convert(entity, &without(&base, &field.name)).is_ok() {
                problems.push(format!(
                    "{}.{} is declared REQUIRED, but the converter accepted the \
                     entity without it",
                    entity.identifier, field.name
                ));
            }
        }
    }
    report("required fields the converter does not require:", problems);
}

/// Every `sample` in the table is a value the converter really accepts. Without
/// this the probes above could be poking with values that were never legal, and
/// every one of them would pass for the wrong reason.
#[test]
fn every_sample_value_converts() {
    let mut problems = Vec::new();
    for entity in &contract().entities {
        if compiled_out(entity) {
            continue;
        }
        let base = base_fields(entity);
        for field in &entity.fields {
            let Some(sample) = &field.sample else {
                continue;
            };
            let fields = with(&preconditions(entity, field, &base), &field.name, sample);
            if let Err(error) = convert(entity, &fields) {
                problems.push(format!(
                    "{}.{} sample {sample:?} does not convert: {error}",
                    entity.identifier, field.name
                ));
            }
        }
    }
    report("contract samples the converter rejects:", problems);
}

/// The `Custom(…)` verdict, observed rather than asserted.
///
/// `refused` says the converter errors on an unrecognised value; `silent_default`
/// and `open` say it does not. Which of the two silent ones a field is decides
/// whether the Python loop treats a typo as an error or says nothing, so getting
/// it wrong is the difference between catching `currancy:1` and shipping it.
#[test]
fn on_invalid_matches_what_the_converter_actually_does() {
    let mut problems = Vec::new();
    for entity in &contract().entities {
        if compiled_out(entity) {
            continue;
        }
        let base = base_fields(entity);
        for field in &entity.fields {
            let Some(poison) = poison_for(field) else {
                if field.on_invalid != OnInvalid::Open {
                    problems.push(format!(
                        "{}.{} declares on_invalid={:?} but no grammar to violate — \
                         only an `open` field may have no rule",
                        entity.identifier, field.name, field.on_invalid
                    ));
                }
                continue;
            };
            let fields = with(&preconditions(entity, field, &base), &field.name, &poison);
            let outcome = convert(entity, &fields);
            match (field.on_invalid, outcome) {
                (OnInvalid::Refused, Ok(())) => problems.push(format!(
                    "{}.{} declares on_invalid=refused, but the converter ACCEPTED \
                     {poison:?}. If the runtime really tolerates it the field is \
                     `silent_default` or `open`, and the authoring severity must \
                     change with it.",
                    entity.identifier, field.name
                )),
                (OnInvalid::SilentDefault | OnInvalid::Open, Err(error)) => {
                    problems.push(format!(
                        "{}.{} declares on_invalid={:?}, but the converter REFUSED \
                         {poison:?}: {error}",
                        entity.identifier, field.name, field.on_invalid
                    ))
                }
                _ => {}
            }
        }
    }
    report("on_invalid classifications the converter contradicts:", problems);
}

/// A retired spelling is refused OUT LOUD, not swallowed. `Patrol:<id>` is the
/// one that earned this: falling through to `CharacterBrain::Custom` left an
/// un-migrated placement looking exactly like a healthy one.
#[test]
fn refused_spellings_are_refused_out_loud() {
    let mut problems = Vec::new();
    for entity in &contract().entities {
        if compiled_out(entity) {
            continue;
        }
        let base = base_fields(entity);
        for field in &entity.fields {
            if !field.refused_patterns.is_empty() && field.refused_samples.is_empty() {
                problems.push(format!(
                    "{}.{} declares refused_patterns with no refused_samples, so \
                     nothing can probe them",
                    entity.identifier, field.name
                ));
            }
            for sample in &field.refused_samples {
                let fields = with(&preconditions(entity, field, &base), &field.name, sample);
                if convert(entity, &fields).is_ok() {
                    problems.push(format!(
                        "{}.{} declares {sample:?} refused, but the converter \
                         accepted it",
                        entity.identifier, field.name
                    ));
                }
            }
        }
    }
    report("retired spellings the converter still tolerates:", problems);
}

/// Two answers to one question are refused: a `path_ref` beside a `brain`, a
/// platform authoring two motions.
#[test]
fn conflicting_fields_are_refused() {
    let mut problems = Vec::new();
    for entity in &contract().entities {
        if compiled_out(entity) {
            continue;
        }
        let base = base_fields(entity);
        for field in &entity.fields {
            let Some(sample) = &field.sample else {
                continue;
            };
            for other_name in &field.conflicts_with {
                let Some(other) = entity.field(other_name) else {
                    problems.push(format!(
                        "{}.{} conflicts with {other_name}, which the contract does \
                         not declare",
                        entity.identifier, field.name
                    ));
                    continue;
                };
                let Some(other_sample) = &other.sample else {
                    problems.push(format!(
                        "{}.{other_name} needs a sample so the conflict with {} can \
                         be probed",
                        entity.identifier, field.name
                    ));
                    continue;
                };
                let fields = with(
                    &with(&base, &field.name, sample),
                    other_name,
                    other_sample,
                );
                if convert(entity, &fields).is_ok() {
                    problems.push(format!(
                        "{}: authoring {} and {other_name} together is declared a \
                         conflict, but the converter accepted both",
                        entity.identifier, field.name
                    ));
                }
            }
        }
    }
    report("declared conflicts the converter permits:", problems);
}

/// A field that needs a companion is refused without it — `loop_min_y` names
/// where a wrapping shaft starts and on its own describes no motion at all.
#[test]
fn fields_that_need_a_companion_are_refused_alone() {
    let mut problems = Vec::new();
    for entity in &contract().entities {
        if compiled_out(entity) {
            continue;
        }
        let base = base_fields(entity);
        for field in &entity.fields {
            let Some(sample) = &field.sample else {
                continue;
            };
            if !field.requires_fields.is_empty()
                && convert(entity, &with(&base, &field.name, sample)).is_ok()
            {
                problems.push(format!(
                    "{}.{} is declared to require {:?}, but the converter accepted \
                     it alone",
                    entity.identifier, field.name, field.requires_fields
                ));
            }
            if let Some(condition) = &field.requires_value_of {
                let fields = with(&base, &condition.field, &condition.equals);
                if convert(entity, &fields).is_ok() {
                    problems.push(format!(
                        "{}.{} is declared required when {}={:?}, but the converter \
                         accepted that combination without it",
                        entity.identifier, field.name, condition.field, condition.equals
                    ));
                }
            }
        }
    }
    report("conditional requirements the converter does not enforce:", problems);
}

/// The identifier list has ONE owner too.
///
/// Python's `KNOWN_ENTITIES` was a hand-typed copy of `standard_converters()` and
/// had already drifted: `SurfaceRamp` was a legal entity to Rust and an unknown
/// one to the validator, so authoring the engine's own fillet failed a green
/// check. The list Python reads is now this contract's, and this pins it to the
/// registry in both directions.
#[test]
fn the_contract_and_the_converter_registry_name_the_same_entities() {
    let vocabulary = LdtkVocabulary::engine();
    let registered: std::collections::BTreeSet<String> = vocabulary
        .identifiers()
        .map(|identifier| identifier.to_string())
        .collect();
    let declared: std::collections::BTreeSet<String> = contract()
        .entities
        .iter()
        .map(|entity| entity.identifier.clone())
        .collect();

    let missing: Vec<&String> = registered.difference(&declared).collect();
    let extra: Vec<&String> = declared.difference(&registered).collect();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "the LDtk authoring contract and the converter registry disagree about \
         which entities exist.\n  registered but undeclared: {missing:?}\n  \
         declared but unregistered: {extra:?}\nAn entity the registry converts and \
         the contract omits is invisible to every Python authoring check; one the \
         contract declares and the registry cannot convert fails a level that \
         passed validation."
    );
}
