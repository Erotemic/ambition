//! CI guard for runtime-projection spatial bugs across every room.
//!
//! The LDtk validator checks LDtk-level data; this checks the lowered
//! `RoomSpec` that the game actually runs: no authored entity center
//! may sit outside the room bounds (it would fall/float forever) and no
//! player spawn may be embedded in a Solid block (the player would load
//! stuck). The `render_room_geometry -- report` example prints the same
//! check for humans; this fails the build if a future room regresses.

use ambition_sandbox as sb;
use sb::engine_core::{self as ae, AabbExt};

fn entity_aabbs(room: &sb::rooms::RoomSpec) -> Vec<(&'static str, ae::Aabb)> {
    let mut v: Vec<(&'static str, ae::Aabb)> = Vec::new();
    v.extend(room.enemy_spawns.iter().map(|e| ("enemy", e.aabb)));
    v.extend(room.boss_spawns.iter().map(|b| ("boss", b.aabb)));
    v.extend(room.interactables.iter().map(|i| ("interactable", i.aabb)));
    v.extend(room.pickups.iter().map(|p| ("pickup", p.aabb)));
    v.extend(room.chests.iter().map(|c| ("chest", c.aabb)));
    v.extend(room.breakables.iter().map(|b| ("breakable", b.aabb)));
    v.extend(room.hazards.iter().map(|h| ("hazard", h.aabb)));
    v.extend(room.loading_zones.iter().map(|z| ("loading_zone", z.aabb)));
    v
}

#[test]
fn no_room_has_out_of_bounds_entities_or_spawn_in_solid() {
    let project =
        sb::ldtk_world::LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
        for block in &world.blocks {
            let bb = block.aabb;
            let inside = world.spawn.x >= bb.min.x
                && world.spawn.x <= bb.max.x
                && world.spawn.y >= bb.min.y
                && world.spawn.y <= bb.max.y;
            if matches!(block.kind, ae::BlockKind::Solid) && inside {
                anomalies.push(format!(
                    "{}: spawn ({:.1},{:.1}) embedded in a Solid block",
                    room.id, world.spawn.x, world.spawn.y
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
