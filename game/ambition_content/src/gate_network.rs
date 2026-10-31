//! Ambition's gate network (the pack's `data/gate_network.ron`): the
//! addresses a terminal dials and each ring of the network.
//!
//! The catalog is one App resource. A content reload replaces it whole: the
//! network is one game's, and no other provider adds rings to it.

use ambition_platformer2d::world::rooms::{GateNetworkCatalog, GateRingPositions, RingPosition, GATE_NETWORK_SCHEMA};
use bevy::prelude::{App, Commands, DetectChanges, IntoScheduleConfigs, PreUpdate, Res};

/// The gate network a pack states.
pub(crate) fn lowered_gate_network(pack: &ambition_content_pack::PreparedContentPack) -> GateNetworkCatalog {
    ambition_content_pack::lowered_rules::<GateNetworkCatalog>(pack, GATE_NETWORK_SCHEMA).cloned().unwrap_or_default()
}

/// Install the network the selected pack states, and keep each ring's place
/// on the flat map (`GateRingPositions`) up to date with the world files: a
/// portal's reach is measured there, and a room moved in the editor moves
/// its ring.
pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let catalog = lowered_gate_network(&pack);
    app.world_mut().insert_resource(catalog);
    app.init_resource::<GateRingPositions>();
    app.add_systems(PreUpdate, place_gate_rings.run_if(the_rings_or_the_worlds_changed));
}

fn the_rings_or_the_worlds_changed(
    catalog: Option<Res<GateNetworkCatalog>>,
    project: Option<Res<ambition_platformer2d_ldtk::ActiveLdtkProject>>,
) -> bool {
    catalog.is_some_and(|catalog| catalog.is_changed()) || project.is_some_and(|project| project.is_changed())
}

fn place_gate_rings(
    catalog: Option<Res<GateNetworkCatalog>>,
    project: Option<Res<ambition_platformer2d_ldtk::ActiveLdtkProject>>,
    mut commands: Commands,
) {
    let (Some(catalog), Some(project)) = (catalog, project) else {
        return;
    };
    commands.insert_resource(ring_positions(&catalog, &project.0));
}

/// Where each ring of `catalog` stands on its world's flat map: the middle of
/// its loading zone, in the level of its room that holds the zone. A ring
/// whose zone no level holds is not placed (the world validator names it).
pub fn ring_positions(
    catalog: &GateNetworkCatalog,
    project: &ambition_platformer2d_ldtk::LdtkProject,
) -> GateRingPositions {
    let mut positions = GateRingPositions::default();
    for ring in &catalog.rings {
        let found = project.levels.iter().filter(|level| level.active_area() == ring.room).find_map(|level| {
            level
                .all_entity_instances()
                .find(|entity| {
                    entity.identifier == "LoadingZone"
                        && ambition_platformer2d_ldtk::field_string(entity, "id").as_deref() == Some(ring.zone.as_str())
                })
                .map(|zone| RingPosition {
                    world: level.world_source.clone(),
                    at: (
                        (level.world_x + zone.px[0]) as f32 + zone.width as f32 * 0.5,
                        (level.world_y + zone.px[1]) as f32 + zone.height as f32 * 0.5,
                    ),
                })
        });
        if let Some(position) = found {
            positions.0.insert(ring.zone.clone(), position);
        }
    }
    positions
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::world::rooms::normalize_address;

    /// Every ring of the shipped network stands somewhere on its world's flat
    /// map, so a portal's reach can be measured to it.
    #[test]
    fn every_ring_is_placed_on_its_worlds_flat_map() {
        let catalog = lowered_gate_network(crate::pack::shipped());
        let manifest = crate::worlds::world_manifest();
        let project = ambition_platformer2d_ldtk::LdtkProject::load_default_for_dev(&manifest).expect("the shipped worlds load");
        let positions = ring_positions(&catalog, &project);
        let unplaced: Vec<&str> =
            catalog.rings.iter().map(|ring| ring.zone.as_str()).filter(|zone| !positions.0.contains_key(*zone)).collect();
        assert!(unplaced.is_empty(), "rings whose zone no level of their room holds: {unplaced:?}");
    }

    /// The working addresses of the network are stable: a save, a sign and a
    /// speedrun route all name them. Each reaches the ring of its place.
    #[test]
    fn the_working_addresses_reach_their_places() {
        let catalog = lowered_gate_network(crate::pack::shipped());
        for (address, room) in [
            ("LEGAL", "central_hub_complex"),
            ("TANGENT", "math_gate_hall"),
            ("GENESIS", "primeval_trailhead"),
            ("CITADEL", "evil_gate_forest"),
            ("ASCENT", "gnu_ton_orchard"),
            ("SAILING", "pirate_port_gate_plaza"),
            ("SILENT", "ninja_village_gate_shrine"),
            ("GALACTIC", "orbital_station_gate_dock"),
            ("ALDENTE", "noodly_narthex"),
        ] {
            let ring = catalog.ring_named(address).unwrap_or_else(|| panic!("no ring answers `{address}`"));
            assert_eq!(ring.room, room, "`{address}`");
            assert!(!ring.hidden, "`{address}` is a public address");
        }
        let short: Vec<_> = catalog
            .rings
            .iter()
            .filter(|ring| ring.hidden)
            .filter_map(|ring| ring.address.as_deref())
            .filter(|address| address.len() <= 5)
            .collect();
        assert!(short.len() >= 2, "a few short hidden addresses: {short:?}");
    }

    /// The people and signs that teach an address teach one the network has:
    /// each `gate.known.<ADDRESS>` flag a shipped dialogue sets names an
    /// address of the catalog, spelled as a dial matches it.
    #[test]
    fn each_address_a_dialogue_teaches_is_an_address_of_the_network() {
        let catalog = lowered_gate_network(crate::pack::shipped());
        let prefix = ambition_platformer2d::world::rooms::gate_address_known_flag("");
        let mut taught = Vec::new();
        let mut dirs = vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/dialogue")];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).expect("the dialogue folder reads") {
                let path = entry.expect("an entry").path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().is_some_and(|ext| ext == "yarn") {
                    let text = std::fs::read_to_string(&path).expect("a dialogue reads");
                    for piece in text.split(&format!("\"{prefix}")).skip(1) {
                        let address = piece.split('"').next().unwrap_or_default().to_string();
                        taught.push((path.file_name().unwrap().to_string_lossy().to_string(), address));
                    }
                }
            }
        }
        assert!(taught.len() >= 3, "the shipped dialogue teaches a few addresses: {taught:?}");
        for (file, address) in &taught {
            assert!(
                catalog.ring_named(address).is_some() && normalize_address(address) == *address,
                "`{file}` teaches `{address}`, which no ring answers"
            );
        }
    }

    /// Every terminal lists a way home: each ring with a terminal outside the
    /// home dimension posts an address of a home ring.
    #[test]
    fn every_terminal_away_from_home_posts_a_way_home() {
        let catalog = lowered_gate_network(crate::pack::shipped());
        let dimension = |zone: &str| {
            let ring = catalog.ring(zone).expect("a ring");
            catalog.facility(&ring.facility).expect("a facility").dimension.clone()
        };
        for ring in catalog.rings.iter().filter(|ring| ring.terminal.is_some()) {
            if dimension(&ring.zone) == "home" {
                continue;
            }
            assert!(
                ring.posted.iter().any(|address| catalog
                    .ring_named(address)
                    .is_some_and(|home| dimension(&home.zone) == "home" && normalize_address(address) == *address)),
                "`{}` posts no way home: {:?}",
                ring.zone,
                ring.posted
            );
        }
    }

    /// The catalog is the pack's file: an edit is the lowered catalog, and a
    /// catalog the rules refuse is refused at admission with its ring named.
    #[test]
    fn the_catalog_is_the_packs_file_and_a_bad_edit_is_refused() {
        let mut app = App::new();
        register(&mut app);
        assert_eq!(app.world().resource::<GateNetworkCatalog>(), &lowered_gate_network(crate::pack::shipped()));
        let with = |from: &'static str, to: &'static str| {
            crate::pack::compile_pack_with(move |path, text| {
                if path != "data/gate_network.ron" {
                    return text;
                }
                assert_eq!(text.matches(from).count(), 1, "the probe edits one line");
                text.replace(from, to)
            })
        };
        let refused = format!("{:?}", with(r#"address: Some("GENESIS")"#, r#"address: Some("TANGENT")"#).expect_err("two rings, one address"));
        assert!(refused.contains("the address `TANGENT` reaches both"), "{refused}");
        let refused = format!("{:?}", with(r#"address: Some("CITADEL")"#, r#"address: Some("CITADEO")"#).expect_err("a symbol off the chevrons"));
        assert!(refused.contains("'O', which is on no chevron"), "{refused}");
    }
}
