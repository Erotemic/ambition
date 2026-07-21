//! The `WorldManifest` VALUE (JD4 / AJ2, K2a): a GAME declares its LDtk
//! worlds and entry room; the engine keeps the room kit (`RoomSpec`/`RoomSet`,
//! projection, validators) and ships ZERO worlds — the R3.2 asset move
//! relocated the payload to `ambition_content::worlds`, which builds one.
//!
//! **There is no install seam and no process global.** A manifest is an
//! ordinary owned value that boot preparation constructs and hands to every
//! reader: the asset-catalog rows, the serde loader's disk/embedded fallback
//! chain, the Bevy `EmbeddedAssetRegistry` registration, the hot-reload
//! watcher, the bevy_ecs_ldtk tile-render spine, and `to_room_set`'s entry
//! room. Readers that run inside a Bevy schedule take it as a `Res`
//! ([`WorldManifest`] is a `Resource`, inserted by the same preparation that
//! threaded it everywhere else); readers that run pre-`App`, at plugin-build
//! time, or as pure functions take `&WorldManifest` directly. Both routes
//! carry the SAME value, so two providers can prepare two different manifests
//! in one process — which the `OnceLock` this replaced made impossible.

use std::path::PathBuf;

use ambition_asset_manager::AssetId;
pub use ambition_world::ron_room::RonRoomSource;
use bevy::prelude::Resource;

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
///
/// A `Resource` so in-schedule readers (the tile-render spine's handle load)
/// can take it as a `Res`; every pre-`App` and pure reader takes `&WorldManifest`
/// instead. Preparation owns the one value and feeds both routes.
#[derive(Clone, Debug, Default, Resource)]
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
    ///
    /// Panics on a world-less manifest (the [`Default`] value). Only the LDtk
    /// LOAD path calls this; compositions that own procedural rooms declare a
    /// world-less manifest and never reach here.
    pub fn primary(&self) -> &WorldSource {
        self.worlds
            .first()
            .expect("WorldManifest must declare at least one world")
    }

    /// Every non-primary world, in declaration order.
    pub fn secondaries(&self) -> impl Iterator<Item = &WorldSource> {
        self.worlds.iter().skip(1)
    }

    /// A world-less declaration — a composition that owns procedural rooms
    /// and loads no `.ldtk` file. Distinguishes "this game ships no worlds"
    /// from "somebody forgot to prepare one", which the old install seam
    /// could only express as a panic.
    pub fn is_world_less(&self) -> bool {
        self.worlds.is_empty()
    }
}

/// The cross-crate test fixture: the game's real worlds under
/// `game/ambition_content/assets/worlds`, entry room = the hub. Read
/// cross-crate (the explicit cross-crate fixture pattern) so this crate's
/// conversion / ron-room contract tests exercise real data without shipping
/// any. Tests now name it EXPLICITLY — it used to be handed to them behind
/// their back by a `cfg(test)` branch inside the global accessor.
#[cfg(test)]
pub(crate) fn test_fixture_manifest() -> WorldManifest {
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
