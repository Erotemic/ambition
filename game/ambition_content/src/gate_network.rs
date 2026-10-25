//! Ambition's gate network (the pack's `data/gate_network.ron`): the
//! addresses a terminal dials and each ring of the network.
//!
//! The catalog is one App resource. A content reload replaces it whole: the
//! network is one game's, and no other provider adds rings to it.

use ambition_platformer2d::world::rooms::{GateNetworkCatalog, GATE_NETWORK_SCHEMA};
use bevy::prelude::App;

/// The gate network a pack states.
pub(crate) fn lowered_gate_network(pack: &ambition_content_pack::PreparedContentPack) -> GateNetworkCatalog {
    ambition_content_pack::lowered_rules::<GateNetworkCatalog>(pack, GATE_NETWORK_SCHEMA).cloned().unwrap_or_default()
}

/// Install the network the selected pack states.
pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let catalog = lowered_gate_network(&pack);
    app.world_mut().insert_resource(catalog);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::world::rooms::normalize_address;

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
