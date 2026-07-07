//! The `ron-room` loader: rooms as serialized world IR.
//!
//! A `ron-room` is a [`RoomSpec`] plus its graph links serialized as RON.
//! It is a backend-neutral path for generated rooms and fixtures: a bake
//! tool emits room IR, and the loader appends it beside rooms produced by
//! an authoring backend such as LDtk.

use std::path::PathBuf;

use crate::rooms::{RoomLink, RoomSpec};

/// One baked `ron-room` a game ships.
#[derive(Clone, Debug)]
pub struct RonRoomSource {
    /// Row identity for diagnostics (`ron_room.*` by convention).
    pub id: String,
    /// Absolute desktop-dev file path.
    pub loose_path: Option<PathBuf>,
    /// The doc's RON text embedded into the binary.
    pub embedded_text: Option<&'static str>,
    /// Required rooms abort composition when unresolvable; optional ones
    /// warn and are skipped.
    pub required: bool,
}

impl RonRoomSource {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            loose_path: None,
            embedded_text: None,
            required: true,
        }
    }
}

/// One serialized room document: the spec plus the graph links it contributes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RonRoomDoc {
    pub spec: RoomSpec,
    #[serde(default)]
    pub links: Vec<RoomLink>,
}

/// Serialize a room doc to RON text.
pub fn room_doc_to_ron(doc: &RonRoomDoc) -> Result<String, String> {
    ron::ser::to_string_pretty(doc, ron::ser::PrettyConfig::default())
        .map_err(|error| format!("could not serialize ron-room: {error}"))
}

/// Parse a `ron-room` document.
pub fn room_doc_from_ron(text: &str) -> Result<RonRoomDoc, String> {
    ron::from_str(text).map_err(|error| format!("could not parse ron-room: {error}"))
}

/// Load every declared `ron-room` under the manifest tolerance contract.
pub fn load_ron_rooms(rows: &[RonRoomSource]) -> Result<Vec<RonRoomDoc>, Vec<String>> {
    let mut docs = Vec::new();
    let mut errors = Vec::new();
    for row in rows {
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
    use ambition_engine_core as ae;

    #[test]
    fn a_generated_room_spec_bakes_and_reloads_without_any_backend() {
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
