//! Sandbox LDtk runtime tests, split by topic.
//!
//! - [`embedded_project`] — sanity checks against the embedded `sandbox.ldtk`
//!   (validation, biome metadata, audio cross-checks, intro authoring).
//! - [`metadata`] — synthetic `RoomMetadata` / `LdtkLevel` / IntGrid mapping
//!   helpers exercised without loading the embedded project.
//! - [`kinematic_paths`] — synthetic projects that compose moving platforms,
//!   camera zones, NPC / enemy / hazard kinematic-path resolution.
//! - [`intgrid`] — rect-merge of collision IntGrid runs, climbable IntGrid
//!   parsing, promoted-runtime-role indexes.
//! - [`surfaces`] — `compile_identifier` round-trip tests for the typed
//!   surface entities (Solid / OneWay / Pogo / Rebound / Breakable…).
//!
//! Helpers stay in this module file so submodules can reach them via
//! `super::`. Each submodule re-imports the parent module's pubic API via
//! `use super::super::{…};` so they stay independently readable.

use serde_json::Value;

use super::intgrid::*;
use super::project::*;
use super::surfaces::*;
use super::*;

mod embedded_project;
mod intgrid;
mod kinematic_paths;
mod metadata;
mod surfaces;

fn make_entity(identifier: &str, size: [i32; 2], fields: &[(&str, Value)]) -> LdtkEntityInstance {
    make_entity_at(identifier, [0, 0], size, fields)
}

fn make_entity_at(
    identifier: &str,
    px: [i32; 2],
    size: [i32; 2],
    fields: &[(&str, Value)],
) -> LdtkEntityInstance {
    LdtkEntityInstance {
        iid: format!("{identifier}-test-{}-{}", px[0], px[1]),
        identifier: identifier.to_string(),
        pivot: vec![0.0, 0.0],
        px,
        width: size[0],
        height: size[1],
        field_instances: fields
            .iter()
            .map(|(name, value)| LdtkFieldInstance {
                identifier: name.to_string(),
                value: value.clone(),
                real_editor_values: vec![Value::Null],
            })
            .collect(),
    }
}

fn compile_identifier(
    identifier: &str,
    size: [i32; 2],
    fields: &[(&str, Value)],
) -> SurfaceCompiled {
    let entity = make_entity(identifier, size, fields);
    let spec = parse_surface_spec(
        &entity,
        ae::Vec2::ZERO,
        ae::Vec2::new(size[0] as f32, size[1] as f32),
        identifier.to_string(),
    )
    .expect("surface spec parses");
    compile_surface(&spec).expect("surface compiles")
}

fn intgrid_layer(identifier: &str, c_wid: i32, c_hei: i32, csv: Vec<i32>) -> LdtkLayerInstance {
    LdtkLayerInstance {
        identifier: identifier.to_string(),
        layer_type: "IntGrid".to_string(),
        c_wid,
        c_hei,
        grid_size: GRID,
        entity_instances: Vec::new(),
        int_grid_csv: csv,
        grid_tiles: Vec::new(),
    }
}
