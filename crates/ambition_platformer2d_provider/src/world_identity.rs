//! Canonical identity sections for a prepared world and its session rules.

use ambition_platformer2d_runtime::{ContentDiagnostic, PreparedContentBuilder, PreparedPlatformerSource};

pub(crate) fn add_world_fingerprint_sections(
    builder: &mut PreparedContentBuilder,
    source: &PreparedPlatformerSource,
) -> Result<(), ContentDiagnostic> {
    if let Some(rule) = source.participant_friendly_fire() {
        let canonical = ron::ser::to_string(&rule)
            .map_err(|error| ContentDiagnostic::new("world.participant-damage", error.to_string()))?;
        builder.add_section("world.participant-damage", canonical.into_bytes())
            .map_err(|error| ContentDiagnostic::new("world.participant-damage", error.to_string()))?;
    }
    let mut rooms = source.room_set().rooms.clone();
    rooms.sort_by(|a, b| a.id.cmp(&b.id));
    let rooms = ron::ser::to_string(&rooms).map_err(|error| {
        ContentDiagnostic::new(
            "world.rooms",
            format!("canonical room serialization failed: {error}"),
        )
    })?;
    builder
        .add_section("world.rooms", rooms.into_bytes())
        .map_err(|error| ContentDiagnostic::new("world.rooms", error.to_string()))?;
    let links = ron::ser::to_string(&source.room_set().canonical_links()).map_err(|error| {
        ContentDiagnostic::new(
            "world.graph",
            format!("canonical room-link serialization failed: {error}"),
        )
    })?;
    builder
        .add_section("world.graph", links.into_bytes())
        .map_err(|error| ContentDiagnostic::new("world.graph", error.to_string()))?;
    let active_geometry = ron::ser::to_string(&source.geometry().0).map_err(|error| {
        ContentDiagnostic::new(
            "world.active-geometry",
            format!("canonical active geometry serialization failed: {error}"),
        )
    })?;
    builder
        .add_section("world.active-geometry", active_geometry.into_bytes())
        .map_err(|error| ContentDiagnostic::new("world.active-geometry", error.to_string()))?;
    let active_metadata = ron::ser::to_string(source.active_room()).map_err(|error| {
        ContentDiagnostic::new(
            "world.active-metadata",
            format!("canonical active metadata serialization failed: {error}"),
        )
    })?;
    builder
        .add_section("world.active-metadata", active_metadata.into_bytes())
        .map_err(|error| ContentDiagnostic::new("world.active-metadata", error.to_string()))?;
    let start_room = source
        .room_set()
        .rooms
        .get(source.room_set().start())
        .map(|room| room.id.as_str())
        .unwrap_or("<missing>");
    builder
        .add_section(
            "world.initial-state",
            format!(
                "active_room={}\nstart_room={}\nstarting_character={}\n",
                source.active_room_id(),
                start_room,
                source.starting_character().character_id,
            )
            .into_bytes(),
        )
        .map_err(|error| ContentDiagnostic::new("world.initial-state", error.to_string()))?;

    // Include deterministic area membership and bounds when an LDtk index exists.
    // Mutable revision/sync cursors are excluded; RON-authored games have no index.
    #[cfg(feature = "ldtk")]
    let area_rows = |area_id: &str| -> (Vec<String>, String) {
        let index = source.installed_ldtk_index();
        let iids = index.map(|i| i.level_iids_for(area_id)).unwrap_or_default();
        let bounds = index.and_then(|i| i.area_bounds(area_id)).map_or_else(
            || "\t-".to_string(),
            |b| format!("\t{},{},{},{}", b.min_x, b.min_y, b.max_x, b.max_y),
        );
        (iids, bounds)
    };
    #[cfg(not(feature = "ldtk"))]
    let area_rows = |_area_id: &str| -> (Vec<String>, String) { (Vec::new(), "\t-".to_string()) };
    let mut runtime_index = String::new();
    let mut area_ids = source
        .room_set()
        .rooms
        .iter()
        .map(|room| room.id.as_str())
        .collect::<Vec<_>>();
    area_ids.sort_unstable();
    area_ids.dedup();
    for area_id in area_ids {
        let (mut level_iids, bounds) = area_rows(area_id);
        level_iids.sort();
        runtime_index.push_str("area\t");
        runtime_index.push_str(area_id);
        runtime_index.push('\t');
        runtime_index.push_str(&level_iids.join(","));
        runtime_index.push_str(&bounds);
        runtime_index.push('\n');
    }
    builder
        .add_section("world.runtime-index", runtime_index.into_bytes())
        .map_err(|error| ContentDiagnostic::new("world.runtime-index", error.to_string()))?;
    Ok(())
}
