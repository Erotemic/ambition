//! CI guard for runtime-projection spatial bugs across every room.
//!
//! The LDtk validator checks LDtk-level data; this checks the lowered
//! `RoomSpec` that the game actually runs: no authored entity center
//! may sit outside the room bounds (it would fall/float forever) and no
//! player spawn may be embedded in a Solid block (the player would load
//! stuck). The `render_room_geometry -- report` example prints the same
//! check for humans; this fails the build if a future room regresses.

use ambition::actors as sb;
use ambition::engine_core::{self as ae, AabbExt};

/// Footprints of placement records of a given kind (families migrated to the
/// single `placements` channel — fable audit F9.2).
fn placement_aabbs(
    room: &sb::rooms::RoomSpec,
    label: &'static str,
    kind: ambition::entity_catalog::placements::PlacementKind,
) -> Vec<(&'static str, ae::Aabb)> {
    room.placements
        .iter()
        .filter(|r| r.kind() == kind)
        .map(|r| (label, r.aabb))
        .collect()
}

fn entity_aabbs(room: &sb::rooms::RoomSpec) -> Vec<(&'static str, ae::Aabb)> {
    use ambition::entity_catalog::placements::PlacementKind;
    let mut v: Vec<(&'static str, ae::Aabb)> = Vec::new();
    v.extend(room.enemy_spawns.iter().map(|e| ("enemy", e.aabb)));
    v.extend(room.boss_spawns.iter().map(|b| ("boss", b.aabb)));
    v.extend(placement_aabbs(
        room,
        "interactable",
        PlacementKind::Interactable,
    ));
    v.extend(placement_aabbs(room, "pickup", PlacementKind::Pickup));
    v.extend(placement_aabbs(room, "chest", PlacementKind::Chest));
    v.extend(placement_aabbs(room, "breakable", PlacementKind::Breakable));
    v.extend(placement_aabbs(room, "hazard", PlacementKind::Hazard));
    v.extend(room.loading_zones.iter().map(|z| ("loading_zone", z.aabb)));
    v
}

#[test]
fn no_room_has_out_of_bounds_entities_or_spawn_in_solid() {
    let project = load_project_for_test().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("room_set should build");
    assert!(
        !room_set.rooms.is_empty(),
        "no rooms loaded — the integrity scan would pass vacuously"
    );

    let mut anomalies: Vec<String> = Vec::new();
    for room in &room_set.rooms {
        let world = &room.world;
        for (label, aabb) in entity_aabbs(room) {
            let c = aabb.center();
            if c.x < 0.0 || c.y < 0.0 || c.x > world.size.x || c.y > world.size.y {
                anomalies.push(format!(
                    "{}: {label} center ({:.1},{:.1}) outside bounds ({:.0},{:.0})",
                    room.id, c.x, c.y, world.size.x, world.size.y
                ));
            }
        }
        let point_in_solid = |p: ae::Vec2| {
            world.blocks.iter().any(|block| {
                matches!(block.kind, ae::BlockKind::Solid)
                    && p.x >= block.aabb.min.x
                    && p.x <= block.aabb.max.x
                    && p.y >= block.aabb.min.y
                    && p.y <= block.aabb.max.y
            })
        };

        // Spawn embedded in a Solid block → player loads stuck.
        if point_in_solid(world.spawn) {
            anomalies.push(format!(
                "{}: spawn ({:.1},{:.1}) embedded in a Solid block",
                room.id, world.spawn.x, world.spawn.y
            ));
        }

        // Small open-space entities embedded in a Solid block →
        // unreachable / stuck. Bosses (large, specially placed) and
        // interactables / loading zones (legitimately mounted on walls)
        // are excluded to avoid false positives.
        let mut embeddable: Vec<(&'static str, ae::Aabb)> = Vec::new();
        embeddable.extend(room.enemy_spawns.iter().map(|e| ("enemy", e.aabb)));
        embeddable.extend(placement_aabbs(
            room,
            "pickup",
            ambition::entity_catalog::placements::PlacementKind::Pickup,
        ));
        embeddable.extend(placement_aabbs(
            room,
            "chest",
            ambition::entity_catalog::placements::PlacementKind::Chest,
        ));
        embeddable.extend(placement_aabbs(
            room,
            "breakable",
            ambition::entity_catalog::placements::PlacementKind::Breakable,
        ));
        for (label, aabb) in embeddable {
            if point_in_solid(aabb.center()) {
                anomalies.push(format!(
                    "{}: {label} center ({:.1},{:.1}) embedded in a Solid block",
                    room.id,
                    aabb.center().x,
                    aabb.center().y
                ));
            }
        }
    }

    assert!(
        anomalies.is_empty(),
        "spatial anomalies found in {} room(s):\n{}",
        anomalies.len(),
        anomalies.join("\n")
    );
}

/// Load the game's merged LDtk project the way a sim entry point does:
/// install the world manifest first — post-R3.2 the engine ships no worlds
/// and panics without a provider-owned manifest.
fn load_project_for_test() -> Result<ambition::actors::ldtk_world::LdtkProject, String> {
    ambition_content::worlds::install();
    ambition::actors::ldtk_world::LdtkProject::load_default_for_dev()
}
