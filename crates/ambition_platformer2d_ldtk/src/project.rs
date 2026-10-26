//! LDtk JSON deserialization types.
//!
//! Mirrors the slice of LDtk's JSON schema Ambition consumes. These
//! structs are pure data — no I/O, no validation, no conversion.
//! Loading policy lives in [`super::loading`]; validation in the
//! facade `validate()`; conversion in [`super::conversion`].

use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize)]
pub struct LdtkProject {
    #[serde(rename = "jsonVersion")]
    pub json_version: String,
    #[serde(default)]
    pub levels: Vec<LdtkLevel>,
}

/// Bevy resource wrapper so other systems (encounter loader) can read
/// the parsed LDtk project without re-parsing the file. Inserted in
/// `init_sandbox_resources`; refreshed by hot reload.
#[derive(bevy::prelude::Resource, Clone, Debug)]
pub struct ActiveLdtkProject(pub LdtkProject);

#[derive(Clone, Debug, Deserialize)]
pub struct LdtkLevel {
    pub identifier: String,
    pub iid: String,
    /// The `WorldSource` id of the world file this level came from. Each
    /// world file has its own coordinate space, so `world_x`/`world_y` of two
    /// levels compare only when this is equal. Set by the manifest loader;
    /// empty for a project parsed alone.
    #[serde(skip)]
    pub world_source: String,
    #[serde(rename = "worldX")]
    pub world_x: i32,
    #[serde(rename = "worldY")]
    pub world_y: i32,
    #[serde(rename = "pxWid")]
    pub px_wid: i32,
    #[serde(rename = "pxHei")]
    pub px_hei: i32,
    #[serde(default, rename = "fieldInstances")]
    pub field_instances: Vec<LdtkFieldInstance>,
    #[serde(default, rename = "layerInstances")]
    pub layer_instances: Vec<LdtkLayerInstance>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LdtkLayerInstance {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(rename = "__type", default)]
    pub layer_type: String,
    #[serde(rename = "__cWid", default)]
    pub c_wid: i32,
    #[serde(rename = "__cHei", default)]
    pub c_hei: i32,
    #[serde(rename = "__gridSize", default = "default_grid_size")]
    pub grid_size: i32,
    #[serde(default, rename = "entityInstances")]
    pub entity_instances: Vec<LdtkEntityInstance>,
    /// IntGrid cell values, row-major (`y * c_wid + x`), `0` = empty.
    /// Only populated for layers whose `__type == "IntGrid"`.
    #[serde(default, rename = "intGridCsv")]
    pub int_grid_csv: Vec<i32>,
    /// Hand-painted tile placements. Only populated for layers
    /// whose `__type == "Tiles"`. We don't yet decode each tile's
    /// individual fields here — the test/validator surface only
    /// needs to know that a layer has SOME tiles.
    #[serde(default, rename = "gridTiles")]
    pub grid_tiles: Vec<serde_json::Value>,
}

fn default_grid_size() -> i32 {
    16
}

#[derive(Clone, Debug, Deserialize)]
pub struct LdtkEntityInstance {
    pub iid: String,
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(default, rename = "__pivot")]
    pub pivot: Vec<f32>,
    pub px: [i32; 2],
    pub width: i32,
    pub height: i32,
    #[serde(default, rename = "fieldInstances")]
    pub field_instances: Vec<LdtkFieldInstance>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LdtkFieldInstance {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(rename = "__value")]
    pub value: Value,
    #[serde(default, rename = "realEditorValues")]
    pub real_editor_values: Vec<Value>,
}

impl LdtkLevel {

    /// Every layer that holds entity instances, regardless of which `__identifier` the author gave
    /// it. Order: file order of layer_instances, then file order of entityInstances within each
    /// layer.
    pub fn entity_layers(&self) -> impl Iterator<Item = &LdtkLayerInstance> {
        self.layer_instances
            .iter()
            .filter(|layer| layer.layer_type == "Entities")
    }

    /// All entity instances across every Entities-type layer in this
    /// level, in file order. Replaces
    /// `ambition_layer().entity_instances.iter()` at callsites that
    /// shouldn't care which entity layer a given entity lives on.
    pub fn all_entity_instances(&self) -> impl Iterator<Item = &LdtkEntityInstance> {
        self.entity_layers()
            .flat_map(|layer| layer.entity_instances.iter())
    }

    pub(super) fn field_string(&self, name: &str) -> Option<String> {
        field_value(&self.field_instances, name).and_then(value_to_string)
    }

    pub(super) fn field_i32(&self, name: &str) -> Option<i32> {
        field_value(&self.field_instances, name).and_then(|value| match value {
            Value::Number(number) => number.as_i64().map(|value| value as i32),
            Value::String(text) => text.parse::<i32>().ok(),
            _ => None,
        })
    }

    pub(super) fn field_bool(&self, name: &str) -> Option<bool> {
        field_value(&self.field_instances, name).and_then(|value| match value {
            Value::Bool(flag) => Some(*flag),
            Value::String(text) => text.parse::<bool>().ok(),
            _ => None,
        })
    }
}

#[cfg(test)]
mod project_lookup_tests {
    //! Layer lookups + the multi-Entities-layer support. all_entity_instances
    //! exists specifically so a level can split entities across more than one
    //! Entities layer (e.g. a dedicated camera layer); pin that it flattens
    //! every Entities layer in order and that the named IntGrid lookups match
    //! by identifier.
    use super::*;

    fn ent(iid: &str) -> LdtkEntityInstance {
        LdtkEntityInstance {
            iid: iid.into(),
            identifier: "Thing".into(),
            pivot: Vec::new(),
            px: [0, 0],
            width: 8,
            height: 8,
            field_instances: Vec::new(),
        }
    }

    fn layer(id: &str, ty: &str, ents: Vec<LdtkEntityInstance>) -> LdtkLayerInstance {
        LdtkLayerInstance {
            identifier: id.into(),
            layer_type: ty.into(),
            c_wid: 0,
            c_hei: 0,
            grid_size: 16,
            entity_instances: ents,
            int_grid_csv: Vec::new(),
            grid_tiles: Vec::new(),
        }
    }

    fn level(layers: Vec<LdtkLayerInstance>) -> LdtkLevel {
        LdtkLevel {
            identifier: "L".into(),
            iid: "l".into(),
            world_source: String::new(),
            world_x: 0,
            world_y: 0,
            px_wid: 100,
            px_hei: 100,
            field_instances: Vec::new(),
            layer_instances: layers,
        }
    }

    #[test]
    fn named_layers_are_found_by_identifier() {
        let lvl = level(vec![
            layer("Ambition", "Entities", vec![]),
            layer("Collision", "IntGrid", vec![]),
            layer("Water", "IntGrid", vec![]),
        ]);
        assert!(lvl.ambition_layer().is_some());
        assert!(lvl.collision_layer().is_some());
        assert!(lvl.water_layer().is_some());
        assert!(
            lvl.climbable_layer().is_none(),
            "no Climbable layer present"
        );
    }

    #[test]
    fn entity_layers_collects_every_entities_layer_and_skips_intgrid() {
        let lvl = level(vec![
            layer("Ambition", "Entities", vec![]),
            layer("Collision", "IntGrid", vec![]),
            layer("AmbitionCameras", "Entities", vec![]),
        ]);
        let ids: Vec<&str> = lvl.entity_layers().map(|l| l.identifier.as_str()).collect();
        assert_eq!(
            ids,
            vec!["Ambition", "AmbitionCameras"],
            "both Entities layers, file order"
        );
    }

    #[test]
    fn all_entity_instances_flattens_layer_then_entity_order() {
        let lvl = level(vec![
            layer("Ambition", "Entities", vec![ent("a1"), ent("a2")]),
            layer("Collision", "IntGrid", vec![]),
            layer("AmbitionCameras", "Entities", vec![ent("c1")]),
        ]);
        let iids: Vec<&str> = lvl.all_entity_instances().map(|e| e.iid.as_str()).collect();
        assert_eq!(iids, vec!["a1", "a2", "c1"]);
    }
}

pub(super) fn field_value<'a>(fields: &'a [LdtkFieldInstance], name: &str) -> Option<&'a Value> {
    fields
        .iter()
        .find(|field| field.identifier == name)
        .map(|field| &field.value)
}

pub(super) fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}
