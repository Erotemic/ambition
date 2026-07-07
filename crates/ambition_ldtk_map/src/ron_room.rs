//! The `ron-room` loader — rooms as serialized IR (W2, decomposition.md).
//!
//! A `ron-room` is a [`RoomSpec`] (+ its graph links) serialized as RON:
//! the room IR itself, with no authoring backend behind it. It exists for
//! GENERATED rooms and fixtures — a bake tool or generator emits the doc,
//! the loader appends it to the composed [`ambition_world::rooms::RoomSet`] next to
//! the LDtk-composed rooms. Authored space stays in backend files (LDtk);
//! this is deliberately NOT an alternative authoring format.
//!
//! This is the IR proof for W3: a room can enter the runtime graph through
//! serde alone, so the composition tier demonstrably has no LDtk
//! dependency in its data path. The manifest rows
//! ([`crate::RonRoomSource`]) let a game ship baked rooms;
//! the pure functions below are the seam generators and tests use.

use ambition_world::rooms::{RoomLink, RoomSpec};

/// One serialized room document: the spec plus the graph links it
/// contributes. Links live on the doc (not the spec) because a link is a
/// property of the room GRAPH — the LDtk path collects them across levels
/// the same way.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RonRoomDoc {
    pub spec: RoomSpec,
    #[serde(default)]
    pub links: Vec<RoomLink>,
}

/// Serialize a room doc to RON text (the bake half; pretty so generated
/// fixtures diff sanely).
pub fn room_doc_to_ron(doc: &RonRoomDoc) -> Result<String, String> {
    ron::ser::to_string_pretty(doc, ron::ser::PrettyConfig::default())
        .map_err(|error| format!("could not serialize ron-room: {error}"))
}

/// Parse a `ron-room` document.
pub fn room_doc_from_ron(text: &str) -> Result<RonRoomDoc, String> {
    ron::from_str(text).map_err(|error| format!("could not parse ron-room: {error}"))
}

/// Load every `ron-room` the installed [`WorldManifest`] declares, under the
/// manifest tolerance contract: a REQUIRED row that fails to resolve or parse
/// is a composition error; an optional one warns and is skipped.
///
/// [`WorldManifest`]: crate::WorldManifest
pub fn load_manifest_ron_rooms() -> Result<Vec<RonRoomDoc>, Vec<String>> {
    let mut docs = Vec::new();
    let mut errors = Vec::new();
    for row in &crate::world_manifest().ron_rooms {
        let text = match (&row.loose_path, row.embedded_text) {
            (Some(path), embedded) => match std::fs::read_to_string(path) {
                Ok(text) => Some(text),
                Err(error) => embedded.map(str::to_string).or_else(|| {
                    eprintln!(
                        "ron-room warning: could not read '{}' from {}: {error}",
                        row.id,
                        path.display()
                    );
                    None
                }),
            },
            (None, embedded) => embedded.map(str::to_string),
        };
        let Some(text) = text else {
            if row.required {
                errors.push(format!("required ron-room '{}' is unresolvable", row.id));
            }
            continue;
        };
        match room_doc_from_ron(&text) {
            Ok(doc) => docs.push(doc),
            Err(error) if row.required => errors.push(format!("ron-room '{}': {error}", row.id)),
            Err(error) => eprintln!("ron-room warning: '{}': {error}; skipping", row.id),
        }
    }
    if errors.is_empty() {
        Ok(docs)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ron-room built from a PURE IR value (no backend anywhere): the
    /// "second backend" seed for W4's fixture test.
    #[test]
    fn a_generated_room_spec_bakes_and_reloads_without_any_backend() {
        use ambition_engine_core as ae;
        let world = ae::World::new(
            "generated: twin chamber",
            ae::Vec2::new(640.0, 480.0),
            ae::Vec2::new(96.0, 96.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 448.0),
                ae::Vec2::new(640.0, 32.0),
            )],
        );
        let spec = RoomSpec::new("twin_chamber", world);
        let baked = room_doc_to_ron(&RonRoomDoc {
            spec,
            links: Vec::new(),
        })
        .expect("bakes");
        let reloaded = room_doc_from_ron(&baked).expect("parses");
        assert_eq!(reloaded.spec.id, "twin_chamber");
        assert_eq!(reloaded.spec.world.blocks.len(), 1);
    }
}
