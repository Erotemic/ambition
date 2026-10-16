//! the engine tests pin the FUNCTION; this pins the WIRING, and that
//! distinction has cost this project a session before: enemy facing was plumbed,
//! tested and green the entire time enemies walked the wrong way, because
//! nothing asserted the authored world ever *said* which way.
//!
//! Here the equivalent failure is silent and total. `on_activate` is optional by
//! design — the encounter-arming switches, the reset switches and the sand sim's
//! switch carry none — so a world that lost the field would produce four kernel
//! faces that simply do nothing. No error, no warning; the puzzle just never
//! completes, and the only thing that changed is a level file.
//!
//! this reads the shipped `.ldtk` rather than a fixture, on purpose. A
//! regenerate, an editor session, or a careless merge is exactly what this
//! defends against, and none of those touch a fixture.

use super::SYMMETRY_ATTUNEMENT_ID;

/// THE SANDBOX WORLD SAYS WHAT EACH KERNEL FACE DOES — in the level, not in
/// Rust.
///
/// and it says it in the vocabulary the shared contract publishes: a command
/// id, a prepared `encounter:` reference, and the signal key the encounter's own
/// objective is built from. The last of those is checked against
/// the pack's `symmetry_attunement.signals` rather than a literal list, so a world and an objective
/// that drift apart fail here rather than in a playthrough.
#[test]
fn the_sandbox_world_authors_what_each_kernel_switch_does() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/worlds/sandbox.ldtk");
    let text = std::fs::read_to_string(path).expect("sandbox.ldtk is readable");
    let project: serde_json::Value = serde_json::from_str(&text).expect("sandbox.ldtk parses");

    let mut authored: std::collections::BTreeMap<String, String> = Default::default();
    let mut switches = 0usize;
    for level in project["levels"].as_array().into_iter().flatten() {
        for layer in level["layerInstances"].as_array().into_iter().flatten() {
            for entity in layer["entityInstances"].as_array().into_iter().flatten() {
                if entity["__identifier"] != "Switch" {
                    continue;
                }
                switches += 1;
                let fields: std::collections::BTreeMap<&str, &serde_json::Value> = entity
                    ["fieldInstances"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|f| Some((f["__identifier"].as_str()?, &f["__value"])))
                    .collect();
                let (Some(id), Some(line)) = (
                    fields.get("id").and_then(|v| v.as_str()),
                    fields.get("on_activate").and_then(|v| v.as_str()),
                ) else {
                    continue;
                };
                if !line.trim().is_empty() {
                    authored.insert(id.to_string(), line.trim().to_string());
                }
            }
        }
    }

    // the non-vacuity guard: a world that lost its Switches entirely would
    // satisfy every assertion below by having nothing to check.
    assert!(
        switches >= 4,
        "sandbox.ldtk authors only {switches} Switch(es); this test is about them"
    );

    let expected: Vec<(String, String)> = crate::tuning::shipped()
        .symmetry_attunement
        .signals
        .iter()
        .map(|signal| {
            (
                format!("kernel_switch_{}", signal.trim_start_matches("gravity_")),
                format!("encounter.signal encounter:{SYMMETRY_ATTUNEMENT_ID} {signal}"),
            )
        })
        .collect();
    for (switch_id, line) in &expected {
        assert_eq!(
            authored.get(switch_id).map(String::as_str),
            Some(line.as_str()),
            "the Noether Chamber's `{switch_id}` must author its own verb; this pair \
             used to live in a Rust const table and losing it in the level is silent"
        );
    }
}

/// The Noether Chamber starts the attunement in data: its `while_live` line,
/// read through the composed room set as the runtime reads it. No other
/// shipped room authors one, so the chamber is the only room that starts the
/// puzzle. The engine half (a line asked for while its room is live, also as
/// one of two live rooms) is pinned in the monolith's
/// `authored_room_commands` tests.
#[test]
fn the_chamber_starts_the_attunement_in_its_world_file() {
    use ambition_platformer2d_ldtk::{LdtkProject, LdtkVocabulary};
    let manifest = crate::worlds::world_manifest();
    let rooms = LdtkProject::load_default_for_dev(&manifest)
        .expect("the shipped worlds load")
        .to_room_set(&manifest, &LdtkVocabulary::engine())
        .expect("the shipped worlds compose")
        .rooms;
    let lines: Vec<(String, String)> = rooms
        .iter()
        .filter_map(|spec| Some((spec.id.clone(), spec.metadata.while_live.clone()?)))
        .collect();
    assert_eq!(
        lines,
        vec![(
            "symmetry_room".to_string(),
            format!("encounter.start encounter:{SYMMETRY_ATTUNEMENT_ID}")
        )]
    );
}

/// The attunement's win condition is the pack's `symmetry_attunement.signals`,
/// not a list of the code's own: a pack that names two signals gets a puzzle
/// of those two.
#[test]
fn the_attunement_completes_on_the_signals_the_pack_names() {
    use ambition_encounter::{Encounter, EncounterObjective, Objective};
    use bevy::ecs::system::RunSystemOnce as _;
    let mut rules = crate::tuning::shipped();
    rules.symmetry_attunement.signals = vec!["probe_a".to_string(), "probe_b".to_string()];
    let mut world = bevy::prelude::World::new();
    world.insert_resource(crate::tuning::AmbitionTuning(rules));
    world.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    world.run_system_once(super::spawn_symmetry_attunement).expect("the spawn runs");
    let mut found = world.query::<(&Encounter, &EncounterObjective)>();
    let objectives: Vec<Objective> = found
        .iter(&world)
        .filter(|(encounter, _)| encounter.id == SYMMETRY_ATTUNEMENT_ID)
        .map(|(_, objective)| objective.win.clone())
        .collect();
    assert_eq!(
        objectives,
        vec![Objective::All(vec![
            Objective::ReceiveSignal("probe_a".to_string()),
            Objective::ReceiveSignal("probe_b".to_string()),
        ])],
        "the attunement does not complete on the pack's signals"
    );
}
