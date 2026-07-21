//! K2a acceptance: the world manifest is a VALUE, so two providers can prepare
//! two DIFFERENT worlds in one process.
//!
//! This is the exit criterion for deleting the `OnceLock` that used to back
//! `world_manifest()`. Under that seam the first `install_world_manifest` call
//! won and every later one was silently dropped (`OnceLock::set` returns an
//! ignored `Err`), so a process could hold exactly one world declaration — and
//! the SECOND provider to prepare would silently load the FIRST provider's
//! worlds and start in the FIRST provider's entry room, with no error anywhere.
//!
//! Every assertion below is written to fail under that old behavior: the two
//! manifests share no world file and no entry room, so "B got A's rooms" is
//! detectable rather than a coincidence.

use ambition::actors::ldtk_world::{LdtkProject, WorldManifest, WorldSource};
use ambition::asset_manager::AssetId;

/// A single-world manifest built directly against the content crate's
/// checked-in `.ldtk` files — no install, no global, just a value.
fn manifest_of(id: &str, file: &str, entry_room: &str) -> WorldManifest {
    let worlds_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ambition_content/assets/worlds");
    WorldManifest {
        entry_room: entry_room.to_string(),
        ron_rooms: Vec::new(),
        worlds: vec![WorldSource {
            id: AssetId::new(id),
            asset_path: format!("game://worlds/{file}"),
            loose_path: Some(worlds_dir.join(file)),
            embedded_text: None,
            embedded_bevy_path: None,
            required: true,
        }],
    }
}

/// The sandbox provider: starts in the hub, ships `sandbox.ldtk`.
fn provider_a() -> WorldManifest {
    manifest_of("world.sandbox_ldtk", "sandbox.ldtk", "central_hub_complex")
}

/// A second provider in the SAME process: a disjoint world file and a
/// disjoint entry room.
fn provider_b() -> WorldManifest {
    manifest_of("world.intro_ldtk", "intro.ldtk", "intro_wake_room")
}

fn room_ids(manifest: &WorldManifest) -> Vec<String> {
    let project = LdtkProject::load_default_for_dev(manifest)
        .unwrap_or_else(|error| panic!("manifest world should load: {error}"));
    let room_set = project
        .to_room_set(manifest)
        .unwrap_or_else(|errors| panic!("manifest world should compose: {errors:?}"));
    room_set.rooms.iter().map(|room| room.id.clone()).collect()
}

fn start_room(manifest: &WorldManifest) -> String {
    let project = LdtkProject::load_default_for_dev(manifest)
        .unwrap_or_else(|error| panic!("manifest world should load: {error}"));
    let room_set = project
        .to_room_set(manifest)
        .unwrap_or_else(|errors| panic!("manifest world should compose: {errors:?}"));
    room_set.active_spec().id.clone()
}

/// THE ORACLE: prepare both providers in ONE process, in both orders, and each
/// keeps its own worlds and its own entry room.
///
/// Preparing in both orders is the part that pins the fix. A global seam is
/// order-dependent by construction — whichever provider ran first would decide
/// for both — so an implementation that still had one would have to disagree
/// with itself between the two halves of this test.
#[test]
fn two_providers_prepare_different_manifests_in_one_process() {
    let a = provider_a();
    let b = provider_b();

    // A first, then B.
    let a_rooms = room_ids(&a);
    let b_rooms = room_ids(&b);

    // B did not inherit A's worlds.
    assert!(
        a_rooms.iter().any(|id| id == "central_hub_complex"),
        "provider A composes its own sandbox rooms; got {a_rooms:?}"
    );
    assert!(
        b_rooms.iter().any(|id| id == "intro_wake_room"),
        "provider B composes its OWN intro rooms — under the OnceLock it would \
         have re-composed A's sandbox instead; got {b_rooms:?}"
    );
    assert!(
        !b_rooms.iter().any(|id| id == "central_hub_complex"),
        "provider B must not see provider A's rooms; got {b_rooms:?}"
    );
    assert!(
        !a_rooms.iter().any(|id| id == "intro_wake_room"),
        "provider A must not see provider B's rooms; got {a_rooms:?}"
    );

    // B first, then A — a global seam could not produce the same answers.
    let b_again = room_ids(&b);
    let a_again = room_ids(&a);
    assert_eq!(
        b_rooms, b_again,
        "provider B's composition is independent of who prepared before it"
    );
    assert_eq!(
        a_rooms, a_again,
        "provider A's composition is independent of who prepared before it"
    );
}

/// The entry room travels with the manifest too — `to_room_set` reads
/// `entry_room` from the value it was handed, not from a process singleton.
#[test]
fn each_manifest_starts_play_in_its_own_entry_room() {
    let a = provider_a();
    let b = provider_b();

    assert_eq!(
        start_room(&b),
        "intro_wake_room",
        "provider B starts where ITS manifest says"
    );
    assert_eq!(
        start_room(&a),
        "central_hub_complex",
        "provider A starts where ITS manifest says, even after B prepared"
    );
}

/// A world-less manifest is a legal declaration, not a missing install.
///
/// The `OnceLock` could only express "no manifest" as a panic on first read,
/// which is why a procedural demo needed a whole parallel
/// `build_sandbox_catalog_without_worlds` entry point rather than a value. That
/// twin is now deleted; this is what replaced it.
#[test]
fn a_world_less_manifest_is_a_value_not_a_panic() {
    let empty = WorldManifest::default();
    assert!(empty.is_world_less());
    assert_eq!(empty.secondaries().count(), 0);
}
