//! Which authored world documents a game ships, and where play starts.
//!
//! A [`WorldManifest`] is a game's declaration of its authored world payload:
//! a list of [`WorldSource`] rows (one per authored world document, each with
//! its catalog id, asset path, dev-loose path and optional embedded copy), the
//! baked [`RonRoomSource`] docs appended beside them, and the entry room a
//! fresh session starts in.
//!
//! This is an asset catalog, not an authoring-format vocabulary. Format adapters
//! read a manifest; none owns it, and consumers can use the manifest without
//! depending on a particular world backend.
//!
//! There is no install seam and no process global. A manifest is an
//! ordinary owned value that boot preparation constructs and hands to every
//! reader: the asset-catalog rows, a backend loader's disk/embedded fallback
//! chain, the Bevy `EmbeddedAssetRegistry` registration, the hot-reload
//! watcher, the tile-render spine, and room-set composition's entry room.
//! Readers that run inside a Bevy schedule take it as a `Res` ([`WorldManifest`]
//! is a `Resource`, inserted by the same preparation that threaded it
//! everywhere else); readers that run pre-`App`, at plugin-build time, or as
//! pure functions take `&WorldManifest` directly. Both routes carry the SAME
//! value, so two providers can prepare two different manifests in one process —
//! which the `OnceLock` this replaced made impossible.

use std::path::PathBuf;

use ambition_asset_manager::AssetId;
use bevy_ecs::prelude::Resource;

use crate::ron_room::RonRoomSource;

/// One authored world document a game ships. The FIRST row of a manifest is
/// the primary (boot-critical, hot-reload-watched) world; later rows are
/// secondaries the loader merges and tolerates missing.
#[derive(Clone, Debug)]
pub struct WorldSource {
    /// Catalog id (`world.*` by convention) — the row's identity for asset
    /// resolution and hot reload.
    pub id: AssetId,
    /// Bevy `AssetPath` for the file (the backend's tile-render spine loads
    /// it; a game typically roots it in its own registered asset source, e.g.
    /// `game://worlds/sandbox.ldtk`).
    pub asset_path: String,
    /// Absolute desktop-dev file path (hot reload + loose-filesystem
    /// profiles). The AUTHORING crate computes it against its own
    /// `CARGO_MANIFEST_DIR`, so the manifest works wherever the files live.
    pub loose_path: Option<PathBuf>,
    /// The world document's text embedded into the binary (web / Android /
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

/// A game's world declaration: which authored world documents exist and where
/// play starts.
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
    /// Baked `ron-room` docs appended to the composed room set.
    pub ron_rooms: Vec<RonRoomSource>,
}

impl WorldManifest {
    /// The boot-critical primary world (first row).
    ///
    /// Panics on a world-less manifest (the [`Default`] value). Only a
    /// backend's LOAD path calls this; compositions that own procedural rooms
    /// declare a world-less manifest and never reach here.
    pub fn primary(&self) -> &WorldSource {
        self.worlds
            .first()
            .expect("WorldManifest must declare at least one world")
    }

    /// Every non-primary world, in declaration order.
    pub fn secondaries(&self) -> impl Iterator<Item = &WorldSource> {
        self.worlds.iter().skip(1)
    }

    /// A world-less declaration — a composition that owns procedural rooms and loads no
    /// authored world document.
    pub fn is_world_less(&self) -> bool {
        self.worlds.is_empty()
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

    /// A manifest row's embedded copy WINS over its authored asset path.
    ///
    /// The bundled/web profiles register the embedded bytes under
    /// `embedded_bevy_path` and the loader must reach those rather than the
    /// `game://` source that only exists on a loose desktop checkout. Both
    /// halves are observed: the same row with no embedded copy resolves to the
    /// authored path, so a reader that ignored the embedding entirely would
    /// fail the first assertion and one that always embedded would fail the
    /// second.
    #[test]
    fn embedded_rows_resolve_to_the_embedded_path_and_loose_rows_do_not() {
        let mut source = WorldSource {
            id: AssetId::new("world.primary"),
            asset_path: "game://worlds/primary.ldtk".to_string(),
            loose_path: None,
            embedded_text: None,
            embedded_bevy_path: None,
            required: true,
        };
        assert_eq!(
            world_bevy_asset_path(&source),
            "game://worlds/primary.ldtk",
            "a row with no embedded copy loads its authored asset path"
        );
        source.embedded_text = Some("{}");
        source.embedded_bevy_path = Some("worlds/primary.ldtk");
        assert_eq!(
            world_bevy_asset_path(&source),
            "embedded://worlds/primary.ldtk",
            "a row that carries embedded bytes loads them, not the loose source"
        );
    }
}
