//! The `WorldManifest` install seam (JD4 / AJ2): a GAME declares its LDtk
//! worlds and entry room; the engine keeps the room kit (`RoomSpec`/`RoomSet`,
//! projection, validators) and ships ZERO worlds — the R3.2 asset move
//! relocated the payload to `ambition_content::worlds`, which installs here.
//!
//! Content installs the manifest at every sim-entry choke point via
//! [`install_world_manifest`] (first install wins — the `install_enemy_roster`
//! seam contract). Every world-loading site derives from the installed rows:
//! the asset-catalog entries, the serde loader's disk/embedded fallback
//! chain, the Bevy `EmbeddedAssetRegistry` registration, the hot-reload
//! watcher, the bevy_ecs_ldtk tile-render spine, and `to_room_set`'s entry
//! room. Production PANICS loudly without an install; core tests read the
//! game's real worlds via the cross-crate `cfg(test)` fixture.

use std::path::PathBuf;
use std::sync::OnceLock;

use ambition_asset_manager::AssetId;
pub use ambition_world::ron_room::RonRoomSource;

/// One LDtk world a game ships. The FIRST row of a manifest is the primary
/// (boot-critical, hot-reload-watched) world; later rows are secondaries the
/// loader merges and tolerates missing.
#[derive(Clone, Debug)]
pub struct WorldSource {
    /// Catalog id (`world.*` by convention) — the row's identity for asset
    /// resolution and hot reload.
    pub id: AssetId,
    /// Bevy `AssetPath` for the file (the bevy_ecs_ldtk tile-render spine
    /// loads it; a game typically roots it in its own registered asset
    /// source, e.g. `game://worlds/sandbox.ldtk`).
    pub asset_path: String,
    /// Absolute desktop-dev file path (hot reload + loose-filesystem
    /// profiles). The AUTHORING crate computes it against its own
    /// `CARGO_MANIFEST_DIR`, so the manifest works wherever the files live.
    pub loose_path: Option<PathBuf>,
    /// The world's JSON text embedded into the binary (web / Android /
    /// bundled builds). `None` on builds that only read from disk.
    pub embedded_text: Option<&'static str>,
    /// URL path inside Bevy's `EmbeddedAssetRegistry` the catalog's
    /// `EmbeddedBinary` candidate points at (registered from
    /// `embedded_text`'s bytes when present).
    pub embedded_bevy_path: Option<&'static str>,
    /// Required worlds abort the boot when unresolvable
    /// (`MissingAssetPolicy::Error`); optional ones warn and are skipped so
    /// a partial checkout still boots.
    pub required: bool,
}

/// A game's world declaration: which LDtk files exist and where play starts.
#[derive(Clone, Debug)]
pub struct WorldManifest {
    /// The room (active-area id) a fresh session starts in. Falls back to
    /// the first composed area when the id is absent from the loaded
    /// project (synthetic fixtures, partial checkouts).
    pub entry_room: String,
    pub worlds: Vec<WorldSource>,
    /// Baked `ron-room` docs appended to the composed room set (W2).
    pub ron_rooms: Vec<RonRoomSource>,
}

impl WorldManifest {
    /// The boot-critical primary world (first row).
    pub fn primary(&self) -> &WorldSource {
        self.worlds
            .first()
            .expect("WorldManifest must declare at least one world")
    }

    /// Every non-primary world, in declaration order.
    pub fn secondaries(&self) -> impl Iterator<Item = &WorldSource> {
        self.worlds.iter().skip(1)
    }
}

/// Game-installed world manifest. Set once at plugin-build time; first
/// install wins. Deliberately a process-global `OnceLock`, not a Bevy
/// `Resource`: the readers (catalog builders, the serde loader, pure
/// `to_room_set`) run from non-system code with no `World` in hand.
static WORLD_MANIFEST: OnceLock<WorldManifest> = OnceLock::new();

/// Install the game's [`WorldManifest`] — the content layer calls this at
/// every sim-entry choke point (before any catalog build or world load).
/// First install wins; later calls are ignored.
pub fn install_world_manifest(manifest: WorldManifest) {
    let _ = WORLD_MANIFEST.set(manifest);
}

/// The active manifest. Public READ view — the app assembly iterates the rows
/// to spawn one tile-render world root per world.
pub fn world_manifest() -> &'static WorldManifest {
    #[cfg(test)]
    {
        // Test fixture = the game's REAL worlds, read cross-crate (the
        // install_enemy_roster fixture pattern) so this crate's conversion /
        // ron-room contract tests exercise real data without shipping any.
        // Restored by the fable final audit (F7): the W3 carve dropped it,
        // which is what orphaned the ruled contract tests.
        WORLD_MANIFEST.get_or_init(test_fixture_manifest)
    }
    #[cfg(not(test))]
    {
        WORLD_MANIFEST.get().unwrap_or_else(|| {
            panic!(
                "world manifest not installed — the game's content must call \
                 install_world_manifest() before any world load \
                 (AmbitionContentPlugin / the app's sim-entry choke points do)"
            )
        })
    }
}

/// The cross-crate test fixture: the game's real worlds under
/// `game/ambition_content/assets/worlds`, entry room = the hub.
#[cfg(test)]
fn test_fixture_manifest() -> WorldManifest {
    let worlds_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../game/ambition_content/assets/worlds");
    let source = |id: &str, file: &str, required: bool| WorldSource {
        id: AssetId::new(id),
        asset_path: format!("game://worlds/{file}"),
        loose_path: Some(worlds_dir.join(file)),
        embedded_text: None,
        embedded_bevy_path: None,
        required,
    };
    WorldManifest {
        entry_room: "central_hub_complex".to_string(),
        ron_rooms: Vec::new(),
        worlds: vec![
            source("world.sandbox_ldtk", "sandbox.ldtk", true),
            source("world.intro_ldtk", "intro.ldtk", false),
            source(
                "world.cut_rope_ldtk",
                "you_have_to_cut_the_rope.ldtk",
                false,
            ),
            source("world.hall_ldtk", "hall_of_characters.ldtk", false),
        ],
    }
}

/// The Bevy `AssetPath` string the tile-render spine loads for a manifest
/// row: the embedded copy when this build carries one, else the row's
/// authored `asset_path` (typically a game-registered asset source on
/// desktop, e.g. `game://worlds/sandbox.ldtk`).
pub fn world_bevy_asset_path(source: &WorldSource) -> String {
    match (source.embedded_text, source.embedded_bevy_path) {
        (Some(_), Some(path)) => format!("embedded://{path}"),
        _ => source.asset_path.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> WorldManifest {
        let source = |id: &str, file: &str, required: bool| WorldSource {
            id: AssetId::new(id),
            asset_path: format!("game://worlds/{file}"),
            loose_path: None,
            embedded_text: None,
            embedded_bevy_path: None,
            required,
        };
        WorldManifest {
            entry_room: "start".to_string(),
            ron_rooms: Vec::new(),
            worlds: vec![
                source("world.primary", "primary.ldtk", true),
                source("world.side_a", "side_a.ldtk", false),
                source("world.side_b", "side_b.ldtk", false),
            ],
        }
    }

    #[test]
    fn primary_is_the_first_row() {
        let manifest = sample_manifest();
        assert_eq!(manifest.primary().id.as_str(), "world.primary");
        assert!(manifest.primary().required);
    }

    #[test]
    fn primary_is_the_first_row_and_secondaries_keep_order() {
        let manifest = sample_manifest();
        let secondary_ids: Vec<_> = manifest
            .secondaries()
            .map(|source| source.id.as_str().to_string())
            .collect();
        assert_eq!(
            secondary_ids,
            vec!["world.side_a", "world.side_b"],
            "declaration order is merge order"
        );
    }
}
