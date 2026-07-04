//! The `WorldManifest` install seam (JD4 / AJ2): a GAME declares its LDtk
//! worlds and entry room; the engine keeps the room kit (`RoomSpec`/`RoomSet`,
//! projection, validators) and knows no world by name.
//!
//! Content installs the manifest at plugin-build time via
//! [`install_world_manifest`] (first install wins — the `install_enemy_roster`
//! seam contract). Every world-loading site derives from the installed rows:
//! the asset-catalog entries, the serde loader's disk/embedded fallback
//! chain, the Bevy `EmbeddedAssetRegistry` registration, the hot-reload
//! watcher, and `to_room_set`'s entry room.
//!
//! Until the R3.2 asset-payload move lands, the BUILT-IN default manifest
//! still names the sandbox worlds shipped in this crate's `assets/` dir —
//! the seam exists first, the payload moves through it second.

use std::path::PathBuf;
use std::sync::OnceLock;

use ambition_asset_manager::AssetId;

use crate::assets::sandbox_assets::{
    ids, EMBEDDED_CUT_ROPE_LDTK_ASSET_PATH, EMBEDDED_HALL_LDTK_ASSET_PATH,
    EMBEDDED_INTRO_LDTK_ASSET_PATH, EMBEDDED_SANDBOX_LDTK_ASSET_PATH,
};

use super::hot_reload::SANDBOX_LDTK_ASSET;

/// One LDtk world a game ships. The FIRST row of a manifest is the primary
/// (boot-critical, hot-reload-watched) world; later rows are secondaries the
/// loader merges and tolerates missing.
#[derive(Clone, Debug)]
pub struct WorldSource {
    /// Catalog id (`world.*` by convention) — the row's identity for asset
    /// resolution and hot reload.
    pub id: AssetId,
    /// Catalog-relative asset path (e.g. `ambition/worlds/sandbox.ldtk`) —
    /// also the bevy_ecs_ldtk `AssetPath` for the primary world.
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
/// plugin-build time (before any catalog build or world load). First install
/// wins; later calls are ignored.
pub fn install_world_manifest(manifest: WorldManifest) {
    let _ = WORLD_MANIFEST.set(manifest);
}

/// The active manifest: the installed one, else the built-in sandbox
/// default (which lives here only until the R3.2 asset move relocates the
/// world payload into content).
pub(crate) fn world_manifest() -> &'static WorldManifest {
    WORLD_MANIFEST.get_or_init(builtin_sandbox_manifest)
}

macro_rules! static_world_text {
    ($name:ident, $path:literal) => {
        #[cfg(feature = "static_map")]
        const $name: Option<&'static str> = Some(include_str!($path));
        #[cfg(not(feature = "static_map"))]
        const $name: Option<&'static str> = None;
    };
}

static_world_text!(
    SANDBOX_LDTK_STATIC,
    "../../../assets/ambition/worlds/sandbox.ldtk"
);
static_world_text!(
    INTRO_LDTK_STATIC,
    "../../../assets/ambition/worlds/intro.ldtk"
);
static_world_text!(
    CUT_ROPE_LDTK_STATIC,
    "../../../assets/ambition/worlds/you_have_to_cut_the_rope.ldtk"
);
static_world_text!(
    HALL_LDTK_STATIC,
    "../../../assets/ambition/worlds/hall_of_characters.ldtk"
);

/// The sandbox's own manifest — Ambition's authored worlds, verbatim from
/// the pre-seam hardcoded sites (`secondary_world_ids`,
/// `merge_static_secondary_worlds`, `extend_with_world_entries`, the
/// `to_room_set` start room).
pub(crate) fn builtin_sandbox_manifest() -> WorldManifest {
    let assets_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let secondary = |id: AssetId,
                     asset_path: &str,
                     embedded_text: Option<&'static str>,
                     embedded_bevy_path: &'static str| WorldSource {
        loose_path: Some(assets_root.join(asset_path)),
        id,
        asset_path: asset_path.to_string(),
        embedded_text,
        embedded_bevy_path: Some(embedded_bevy_path),
        required: false,
    };
    WorldManifest {
        entry_room: "central_hub_complex".to_string(),
        worlds: vec![
            WorldSource {
                id: ids::sandbox_ldtk(),
                asset_path: SANDBOX_LDTK_ASSET.to_string(),
                loose_path: Some(assets_root.join(SANDBOX_LDTK_ASSET)),
                embedded_text: SANDBOX_LDTK_STATIC,
                embedded_bevy_path: Some(EMBEDDED_SANDBOX_LDTK_ASSET_PATH),
                required: true,
            },
            secondary(
                ids::intro_ldtk(),
                "ambition/worlds/intro.ldtk",
                INTRO_LDTK_STATIC,
                EMBEDDED_INTRO_LDTK_ASSET_PATH,
            ),
            secondary(
                ids::cut_rope_ldtk(),
                "ambition/worlds/you_have_to_cut_the_rope.ldtk",
                CUT_ROPE_LDTK_STATIC,
                EMBEDDED_CUT_ROPE_LDTK_ASSET_PATH,
            ),
            secondary(
                ids::hall_ldtk(),
                "ambition/worlds/hall_of_characters.ldtk",
                HALL_LDTK_STATIC,
                EMBEDDED_HALL_LDTK_ASSET_PATH,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: no test here calls `install_world_manifest` — the OnceLock is
    // process-global and an install would clobber the built-in default the
    // rest of this test binary (embedded_project, catalog identity) relies
    // on. Install semantics are exercised by the content crate, which
    // installs for real.

    #[test]
    fn builtin_manifest_declares_the_sandbox_worlds() {
        let manifest = builtin_sandbox_manifest();
        assert_eq!(manifest.entry_room, "central_hub_complex");
        assert_eq!(manifest.worlds.len(), 4);
        assert!(
            manifest.primary().required,
            "primary world is boot-critical"
        );
        assert_eq!(manifest.primary().id, ids::sandbox_ldtk());
        assert!(
            manifest.secondaries().all(|source| !source.required),
            "secondaries are tolerated missing"
        );
        for source in &manifest.worlds {
            assert!(
                source.embedded_bevy_path.is_some(),
                "every sandbox world authors an EmbeddedBinary candidate"
            );
            assert!(
                source
                    .loose_path
                    .as_ref()
                    .is_some_and(|path| path.is_absolute()),
                "loose paths are absolute (authoring-crate CARGO_MANIFEST_DIR)"
            );
        }
    }

    #[test]
    fn primary_is_the_first_row_and_secondaries_keep_order() {
        let manifest = builtin_sandbox_manifest();
        let secondary_ids: Vec<_> = manifest
            .secondaries()
            .map(|source| source.id.as_str().to_string())
            .collect();
        assert_eq!(
            secondary_ids,
            vec!["world.intro_ldtk", "world.cut_rope_ldtk", "world.hall_ldtk"],
            "declaration order is merge order"
        );
    }
}
