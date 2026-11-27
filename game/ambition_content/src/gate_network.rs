//! Ambition's gate network (the pack's `data/gate_network.ron`): the
//! addresses a terminal dials and each ring of the network.
//!
//! The file lowers into the engine's numbered catalog (one App resource) and
//! Ambition's words ([`GateWordsResource`]: the inscription, and each ring's
//! address word). A content reload replaces both whole: the network is one
//! game's, and no other provider adds rings to it.

use ambition_content_schemas::gate_directory::{lowered_gate_directory, GateDirectory, GateStartKnowledge};
use ambition_content_schemas::gate_network::{AuthoredGateNetwork, GateWords};
use ambition_platformer2d::world::rooms::{
    GateAddressLearned, GateNetworkCatalog, GateRing, GateRingPositions, RingPosition,
};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::{App, Commands, DetectChanges, IntoScheduleConfigs, MessageReader, PreUpdate, Res, ResMut};

/// The save flag that records that the player knows `address` (its word). A
/// dial that answers sets it (the engine reports [`GateAddressLearned`]); a
/// sign, a person or a document can set it too (Yarn `world.set_flag`).
/// Knowledge changes what a terminal lists, never what a dial does.
pub fn gate_address_known_flag(address: &str) -> String {
    format!("gate.known.{address}")
}

/// Whether a traveller knows the address of `ring`: under
/// [`GateStartKnowledge::All`] every address, else the ones whose known flag
/// `flag` reads as set.
pub fn knows(directory: &GateDirectory, words: &GateWords, ring: &GateRing, flag: impl Fn(&str) -> bool) -> bool {
    matches!(directory.start_known, GateStartKnowledge::All) || flag(&gate_address_known_flag(words.address_key(ring)))
}

/// What Ambition's terminals list, as the pack states it (its own file, not
/// the engine's network).
pub fn lowered_directory(pack: &ambition_content_pack::PreparedContentPack) -> GateDirectory {
    lowered_gate_directory(pack).cloned().unwrap_or_default()
}

/// [`GateDirectory`] as an App resource.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct GateDirectoryResource(pub GateDirectory);

/// [`GateWords`] as an App resource: what a terminal shows of the chevrons
/// the engine dials.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct GateWordsResource(pub GateWords);

/// Record each address the network taught this tick as its known flag.
fn remember_learned_addresses(
    (catalog, words): (Option<Res<GateNetworkCatalog>>, Option<Res<GateWordsResource>>),
    mut learned: MessageReader<GateAddressLearned>,
    mut save: ResMut<ambition_platformer2d::persistence::save::AmbitionGameSave>,
) {
    let (Some(catalog), Some(words)) = (catalog, words) else {
        learned.clear();
        return;
    };
    for learned in learned.read() {
        if let Some(ring) = catalog.ring(&learned.ring) {
            save.data_mut().set_flag(gate_address_known_flag(words.0.address_key(ring)), true);
        }
    }
}

/// The gate network a pack states, lowered: the engine's catalog and
/// Ambition's words.
pub(crate) fn lowered_gate_network_and_words(
    pack: &ambition_content_pack::PreparedContentPack,
) -> (GateNetworkCatalog, GateWords) {
    ambition_content_schemas::gate_network::lowered_gate_network(pack).map(AuthoredGateNetwork::lower).unwrap_or_default()
}

/// The engine's catalog of the gate network a pack states.
#[cfg(test)]
pub(crate) fn lowered_gate_network(pack: &ambition_content_pack::PreparedContentPack) -> GateNetworkCatalog {
    lowered_gate_network_and_words(pack).0
}

/// Ambition's words of the gate network a pack states.
#[cfg(test)]
pub(crate) fn lowered_gate_words(pack: &ambition_content_pack::PreparedContentPack) -> GateWords {
    lowered_gate_network_and_words(pack).1
}

/// Install the network the selected pack states, and keep each ring's place
/// on the flat map (`GateRingPositions`) up to date with the world files: a
/// portal's reach is measured there, and a room moved in the editor moves
/// its ring.
pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let (catalog, words) = lowered_gate_network_and_words(&pack);
    app.world_mut().insert_resource(catalog);
    app.world_mut().insert_resource(GateWordsResource(words));
    app.world_mut().insert_resource(GateDirectoryResource(lowered_directory(&pack)));
    // The dial asks Ambition's spelling whether an address can be dialed.
    app.insert_resource(crate::gate_inscription::address_rule());
    app.init_resource::<GateRingPositions>();
    app.add_systems(PreUpdate, place_gate_rings.run_if(the_rings_or_the_worlds_changed));
    // In the tick the network taught it, right after the dial.
    let sim = app.sim_schedule();
    app.add_message::<GateAddressLearned>();
    app.add_systems(
        sim,
        remember_learned_addresses
            .after(ambition_platformer2d::session::rooms::gate_network::run_gate_network)
            .in_set(ambition_platformer2d_shared_tangle::schedule::ProgressionSet::WorldSync),
    );
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
    use ambition_content_schemas::gate_network::normalize_address;

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

    /// Magical's local arches (WORLD-LAYERS MG-2) list only what they reach:
    /// the heights' arch lists the keep's and not the heart's, which is
    /// farther than a portal reaches, nor any gate; the keep's lists both.
    /// When the catalog lists only what was discovered, the heart's arch is
    /// absent from the keep's list until its address is learned, while the
    /// heights' arch, posted on the keep's terminal, is listed from the start.
    #[test]
    fn magicals_arches_list_only_the_arches_they_reach() {
        let (catalog, gate_words) = lowered_gate_network_and_words(crate::pack::shipped());
        let mut directory = lowered_directory(crate::pack::shipped());
        let manifest = crate::worlds::world_manifest();
        let project = ambition_platformer2d_ldtk::LdtkProject::load_default_for_dev(&manifest).expect("the shipped worlds load");
        let positions = ring_positions(&catalog, &project);
        let words = |catalog: &GateNetworkCatalog, directory: &GateDirectory, ring: &str, learned: &[&str]| -> Vec<String> {
            let learned: Vec<String> = learned.iter().map(|word| gate_address_known_flag(word)).collect();
            crate::gate_terminal::known_addresses(catalog, &gate_words, directory, &positions, ring, |flag| learned.iter().any(|known| known == flag))
                .into_iter()
                .filter_map(|address| gate_words.word_of(catalog, &address).map(str::to_string))
                .collect()
        };
        let reach = |a: &str, b: &str| positions.distance(a, b).expect("both arches are placed");
        assert!(reach("magical_heights_arch", "magical_heart_arch") > ambition_platformer2d::world::rooms::DEFAULT_PORTAL_REACH as f32);
        assert_eq!(words(&catalog, &directory, "magical_heights_arch", &[]), ["CASTLE"]);
        assert_eq!(words(&catalog, &directory, "magical_keep_arch", &[]), ["STEADY", "CANDLE"]);
        directory.start_known = GateStartKnowledge::Discovered;
        assert_eq!(words(&catalog, &directory, "magical_keep_arch", &[]), ["STEADY"], "an arch nobody attuned is not listed");
        assert_eq!(words(&catalog, &directory, "magical_keep_arch", &["CANDLE"]), ["STEADY", "CANDLE"]);
    }

    /// The Jungle's arches (WORLD-LAYERS JL-2) list only what they reach,
    /// a chain along the trail: each lists its neighbours and no arch two
    /// links away, nor any gate. When the catalog lists only what was
    /// discovered, an arch lists what its terminal posts (the way back down
    /// the chain) and an arch up the chain only once its address is learned.
    #[test]
    fn the_jungles_arches_list_only_their_neighbours_on_the_trail() {
        let (catalog, gate_words) = lowered_gate_network_and_words(crate::pack::shipped());
        let mut directory = lowered_directory(crate::pack::shipped());
        let manifest = crate::worlds::world_manifest();
        let project = ambition_platformer2d_ldtk::LdtkProject::load_default_for_dev(&manifest).expect("the shipped worlds load");
        let positions = ring_positions(&catalog, &project);
        let words = |catalog: &GateNetworkCatalog, directory: &GateDirectory, ring: &str, learned: &[&str]| -> Vec<String> {
            let learned: Vec<String> = learned.iter().map(|word| gate_address_known_flag(word)).collect();
            let mut words: Vec<String> = crate::gate_terminal::known_addresses(catalog, &gate_words, directory, &positions, ring, |flag| learned.iter().any(|known| known == flag))
                .into_iter()
                .filter_map(|address| gate_words.word_of(catalog, &address).map(str::to_string))
                .collect();
            words.sort();
            words
        };
        assert_eq!(words(&catalog, &directory, "primeval_trailhead_arch", &[]), ["TALLEST"]);
        assert_eq!(words(&catalog, &directory, "primeval_treetops_arch", &[]), ["GIANTS", "LIANAS"]);
        assert_eq!(words(&catalog, &directory, "primeval_canopy_arch", &[]), ["DIGSITE", "TALLEST"]);
        assert_eq!(words(&catalog, &directory, "primeval_tar_pits_arch", &[]), ["LIANAS"]);
        directory.start_known = GateStartKnowledge::Discovered;
        assert_eq!(words(&catalog, &directory, "primeval_trailhead_arch", &[]), Vec::<String>::new(), "an arch nobody attuned is not listed");
        assert_eq!(words(&catalog, &directory, "primeval_trailhead_arch", &["TALLEST"]), ["TALLEST"]);
        assert_eq!(words(&catalog, &directory, "primeval_canopy_arch", &[]), ["TALLEST"], "the canopy's terminal posts the way back");
        assert_eq!(words(&catalog, &directory, "primeval_canopy_arch", &["DIGSITE"]), ["DIGSITE", "TALLEST"]);
    }

    /// The working addresses of the network are stable: a save, a sign and a
    /// speedrun route all name them. Each reaches the ring of its place.
    #[test]
    fn the_working_addresses_reach_their_places() {
        let (catalog, words) = lowered_gate_network_and_words(crate::pack::shipped());
        let ring_named = |word: &str| words.zone_named(word).and_then(|zone| catalog.ring(zone));
        for (address, room) in [
            ("LEGAL", "authority_concourse"),
            ("TESTING", "central_hub_complex"),
            ("TANGENT", "math_gate_hall"),
            ("GENESIS", "primeval_trailhead"),
            ("CITADEL", "evil_gate_forest"),
            ("ASCENT", "gnu_ton_orchard"),
            ("SAILING", "pirate_port_gate_plaza"),
            ("SILENT", "ninja_village_gate_shrine"),
            ("GALACTIC", "orbital_station_gate_dock"),
            ("ALDENTE", "noodle_village_square"),
        ] {
            let ring = ring_named(address).unwrap_or_else(|| panic!("no ring answers `{address}`"));
            assert_eq!(ring.room, room, "`{address}`");
            assert!(!ring.hidden, "`{address}` is a public address");
        }
        let short: Vec<_> = catalog
            .rings
            .iter()
            .filter(|ring| ring.hidden)
            .filter_map(|ring| words.word(&ring.zone))
            .filter(|address| address.len() <= 5)
            .collect();
        assert!(short.len() >= 2, "a few short hidden addresses: {short:?}");
    }

    /// The people and signs that teach an address teach one the network has:
    /// each `gate.known.<ADDRESS>` flag a shipped dialogue sets names an
    /// address of the catalog, spelled as a dial matches it.
    #[test]
    fn each_address_a_dialogue_teaches_is_an_address_of_the_network() {
        let words = lowered_gate_words(crate::pack::shipped());
        let prefix = gate_address_known_flag("");
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
                words.zone_named(address).is_some() && normalize_address(address) == *address,
                "`{file}` teaches `{address}`, which no ring answers"
            );
        }
    }

    /// Every terminal lists a way home: from each ring with a terminal
    /// outside the home dimension, following posted addresses alone (no
    /// knowledge) reaches a home ring. Every terminal posts LEGAL, the
    /// capital, and the capital's terminal posts the sandbox hub (Q185), so
    /// the walk is at most two dials. A portal arch is not held to it: it
    /// reaches only its own world's arches, so no home address would answer
    /// it, and its way home is the walk to its world's gate.
    #[test]
    fn every_terminal_away_from_home_posts_a_way_home() {
        let (catalog, words) = lowered_gate_network_and_words(crate::pack::shipped());
        let directory = lowered_directory(crate::pack::shipped());
        let dimension = |zone: &str| {
            let ring = catalog.ring(zone).expect("a ring");
            catalog.facility(&ring.facility).expect("a facility").world.clone()
        };
        let posted_rings = |zone: &str| -> Vec<String> {
            directory
                .posted(zone)
                .iter()
                .filter(|address| normalize_address(address) == **address)
                .filter_map(|address| words.zone_named(address))
                .map(str::to_string)
                .collect()
        };
        for ring in catalog.rings.iter().filter(|ring| ring.terminal.is_some()) {
            if dimension(&ring.zone) == "home" || ring.kind == ambition_platformer2d::world::rooms::RingKind::Portal {
                continue;
            }
            let first = posted_rings(&ring.zone);
            let home = first.iter().any(|zone| dimension(zone) == "home")
                || first.iter().any(|zone| posted_rings(zone).iter().any(|far| dimension(far) == "home"));
            assert!(home, "`{}` posts no way home within two dials: {:?}", ring.zone, directory.posted(&ring.zone));
        }
    }

    /// LEGAL is the Gate Authority capital's public address (Q185,
    /// 2026-11-26), and the sandbox hub has its own: every terminal away
    /// from the capital posts LEGAL, and the capital's posts the sandbox hub.
    #[test]
    fn legal_is_the_capitals_and_the_capital_posts_the_sandbox() {
        let words = lowered_gate_words(crate::pack::shipped());
        let legal = words.zone_named("LEGAL").expect("a ring answers LEGAL");
        assert_eq!(legal, "authority_concourse_gate");
        let hub = "hub_public_gate_zone";
        let sandbox = words.word(hub).expect("the sandbox hub has an address word").to_string();
        assert_ne!(sandbox, "LEGAL", "the sandbox hub keeps its own address");
        let directory = lowered_directory(crate::pack::shipped());
        assert!(directory.posted(legal).contains(&sandbox), "the capital posts the sandbox hub: {:?}", directory.posted(legal));
        assert!(directory.posted(hub).iter().any(|address| address == "LEGAL"), "the sandbox posts the capital");
    }

    /// The directory names only rings of the network, and posts only
    /// addresses a ring answers (the engine's catalog check does not know the
    /// directory).
    #[test]
    fn the_directory_posts_only_addresses_of_the_network() {
        let (catalog, words) = lowered_gate_network_and_words(crate::pack::shipped());
        let directory = lowered_directory(crate::pack::shipped());
        assert!(!directory.posted.is_empty(), "premise: the directory posts addresses");
        for (zone, addresses) in &directory.posted {
            assert!(catalog.ring(zone).is_some(), "the directory posts on `{zone}`, which is no ring");
            for address in addresses {
                let spelled = words.spell(address).unwrap_or_else(|error| panic!("`{zone}` posts `{address}`: {error:?}"));
                assert!(catalog.ring_at(&spelled).is_some(), "`{zone}` posts `{address}`, which no ring answers");
            }
        }
    }

    /// The catalog is the pack's file: an edit is the lowered catalog, and a
    /// catalog the rules refuse is refused at admission with its ring named.
    #[test]
    fn the_catalog_is_the_packs_file_and_a_bad_edit_is_refused() {
        let mut app = App::new();
        register(&mut app);
        assert_eq!(app.world().resource::<GateNetworkCatalog>(), &lowered_gate_network(crate::pack::shipped()));
        assert_eq!(app.world().resource::<GateWordsResource>().0, lowered_gate_words(crate::pack::shipped()));
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
