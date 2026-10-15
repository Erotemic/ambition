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
//! There is no install seam and no process global. A manifest is an ordinary
//! owned value that boot preparation constructs and passes to every reader:
//! the asset-catalog rows, a backend loader's disk/embedded fallback chain,
//! the Bevy `EmbeddedAssetRegistry` registration, the hot-reload watcher, the
//! tile-render spine, and room-set composition's entry room. In-schedule
//! readers take it as a `Res`; pre-`App`, plugin-build and pure readers take
//! `&WorldManifest`. Both routes carry the same value, so two providers can
//! prepare two different manifests in one process.

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

/// The authored FILE kind: a game's world declaration, as its pack states
/// it. The engine's schema registry registers it (the world crate does not
/// link the content pack).
pub const WORLD_MANIFEST_SCHEMA: &str = "world_manifest";

/// A game's world declaration as authored: where play starts, and each world
/// file by its id. The game turns it into a [`WorldManifest`] with the roots
/// it ships its files under ([`AuthoredWorldManifest::manifest`]).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredWorldManifest {
    pub entry_room: String,
    /// The first world is the primary one: boot-critical and hot-reload
    /// watched.
    pub worlds: Vec<AuthoredWorld>,
}

/// One world file as authored.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredWorld {
    /// The row's catalog id (`world.*` by convention).
    pub id: String,
    /// The file's name in the game's worlds directory.
    pub file: String,
    /// A required world aborts the boot when it cannot be read; an optional
    /// one is skipped with a warning, so a partial checkout still boots.
    #[serde(default)]
    pub required: bool,
}

/// Where a game ships its world files: the loose directory, the Bevy asset
/// path prefix, the embedded asset path prefix, and the embedded text of a
/// file when this build carries one.
pub struct WorldRoots<'a> {
    pub loose_dir: &'a std::path::Path,
    pub asset_prefix: &'a str,
    pub embedded_prefix: &'a str,
    pub embedded_text: &'a dyn Fn(&str) -> Option<&'static str>,
}

impl AuthoredWorldManifest {
    /// The [`WorldManifest`] of this declaration under `roots`.
    pub fn manifest(&self, roots: &WorldRoots<'_>) -> WorldManifest {
        WorldManifest {
            entry_room: self.entry_room.clone(),
            worlds: self
                .worlds
                .iter()
                .map(|world| WorldSource {
                    id: AssetId::new(world.id.as_str()),
                    asset_path: format!("{}{}", roots.asset_prefix, world.file),
                    loose_path: Some(roots.loose_dir.join(&world.file)),
                    embedded_text: (roots.embedded_text)(&world.file),
                    // The embedded registry keys by a `&'static str`; a
                    // manifest is built a few times per process.
                    embedded_bevy_path: Some(Box::leak(
                        format!("{}{}", roots.embedded_prefix, world.file).into_boxed_str(),
                    )),
                    required: world.required,
                })
                .collect(),
            ron_rooms: Vec::new(),
        }
    }
}

/// Each value of a world declaration the loader cannot use, named by its
/// field.
pub fn world_manifest_problems(manifest: &AuthoredWorldManifest) -> Vec<String> {
    let mut problems = Vec::new();
    if manifest.entry_room.trim().is_empty() {
        problems.push("`entry_room` is empty; a fresh session starts in it".to_string());
    }
    match manifest.worlds.first() {
        None => problems.push("`worlds` is empty; a game with authored rooms ships a world file".to_string()),
        Some(primary) if !primary.required => problems.push(format!(
            "`worlds[0].required` is false; `{}` is the primary world, which the boot cannot do without",
            primary.id
        )),
        Some(_) => {}
    }
    for (index, world) in manifest.worlds.iter().enumerate() {
        if world.id.trim().is_empty() {
            problems.push(format!("`worlds[{index}].id` is empty"));
        }
        if !world.file.ends_with(".ldtk") || world.file.contains('/') || world.file.contains('\\') {
            problems.push(format!(
                "`worlds[{index}].file` is `{}`; it is the name of a `.ldtk` file in the worlds directory",
                world.file
            ));
        }
        let earlier = &manifest.worlds[..index];
        if earlier.iter().any(|other| other.id == world.id) {
            problems.push(format!("`worlds[{index}].id` `{}` is stated twice", world.id));
        }
        if earlier.iter().any(|other| other.file == world.file) {
            problems.push(format!("`worlds[{index}].file` `{}` is stated twice", world.file));
        }
    }
    problems
}

/// The Bevy `AssetPath` string the tile-render spine loads for a manifest
/// row: the embedded copy when this build carries one, else the row's
/// authored `asset_path` (typically a game-registered asset source on
/// desktop, e.g. `game://worlds/sandbox.ldtk`).
///
/// A program that carries its asset tree
/// ([`ambition_asset_manager::exe_bundle`]) loads the world file of that tree
/// ([`world_bevy_asset_path_in`]).
pub fn world_bevy_asset_path(source: &WorldSource) -> String {
    world_bevy_asset_path_in(source, ambition_asset_manager::exe_bundle::ExeBundle::of_running_exe())
}

/// [`world_bevy_asset_path`] for a program whose asset tree is `bundle`.
///
/// A world file names each tileset image by a path from itself
/// (`../sprites/tiles.png`). The embedded copy is one file with nothing
/// beside it, so its images resolve to `embedded://<crate>/sprites/tiles.png`
/// and do not load. The world file of the bundle has the images beside it,
/// so a row that the bundle holds loads from there, embedded copy or not.
pub fn world_bevy_asset_path_in(
    source: &WorldSource,
    bundle: Option<&ambition_asset_manager::exe_bundle::ExeBundle>,
) -> String {
    // Each asset source of a packaged program reads the one flat tree, so
    // the path in the tree is the authored path with no source name.
    let in_the_tree = source
        .asset_path
        .split_once("://")
        .map_or(source.asset_path.as_str(), |(_, path)| path);
    if bundle.is_some_and(|bundle| bundle.contains(in_the_tree)) {
        return source.asset_path.clone();
    }
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
    /// `embedded_bevy_path`; the `game://` source exists only on a loose
    /// desktop checkout. The same row with no embedded copy must resolve to
    /// the authored path, so a reader that always or never embeds fails.
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

    /// A packaged program holds the world file beside the tileset images the
    /// file names, and the embedded copy has no file beside it. The first
    /// packaged build loaded the embedded copy and logged `Path not found:
    /// ambition_content/sprites/intro_lab_tileset.png`.
    #[test]
    fn a_packaged_program_loads_the_world_file_of_its_own_tree() {
        let bundle = |name: &str, files: &[(&str, &[u8])]| {
            let exe = std::env::temp_dir().join(format!("ambition-world-manifest-{}-{name}", std::process::id()));
            ambition_asset_manager::exe_bundle::write_bundle(b"program", files, &exe).expect("the bundle is written");
            let bundle = ambition_asset_manager::exe_bundle::ExeBundle::open(&exe)
                .expect("the file is read")
                .expect("the file carries a bundle");
            // The index is in memory, and this arm asks only what it holds.
            std::fs::remove_file(&exe).expect("the file is removed");
            bundle
        };
        let source = WorldSource {
            id: AssetId::new("world.primary"),
            asset_path: "game://worlds/primary.ldtk".to_string(),
            loose_path: None,
            embedded_text: Some("{}"),
            embedded_bevy_path: Some("worlds/primary.ldtk"),
            required: true,
        };
        assert_eq!(
            world_bevy_asset_path_in(&source, None),
            "embedded://worlds/primary.ldtk",
            "the control: with no bundle the embedded copy wins, as before"
        );
        let holds_the_world = bundle("holds", &[("worlds/primary.ldtk", b"{}"), ("sprites/tiles.png", b"png")]);
        assert_eq!(
            world_bevy_asset_path_in(&source, Some(&holds_the_world)),
            "game://worlds/primary.ldtk"
        );
        let holds_another_world = bundle("another", &[("worlds/another.ldtk", b"{}")]);
        assert_eq!(
            world_bevy_asset_path_in(&source, Some(&holds_another_world)),
            "embedded://worlds/primary.ldtk",
            "a bundle that does not hold this world cannot serve it"
        );
    }

    fn authored(worlds: &[(&str, &str, bool)]) -> AuthoredWorldManifest {
        AuthoredWorldManifest {
            entry_room: "hub".into(),
            worlds: worlds
                .iter()
                .map(|(id, file, required)| AuthoredWorld { id: (*id).into(), file: (*file).into(), required: *required })
                .collect(),
        }
    }

    /// A declaration lowers each row under the game's roots, and a value the
    /// loader cannot use is named by its field.
    #[test]
    fn an_authored_world_declaration_lowers_under_the_games_roots() {
        let good = authored(&[("world.a", "a.ldtk", true), ("world.b", "b.ldtk", false)]);
        assert!(world_manifest_problems(&good).is_empty(), "{:?}", world_manifest_problems(&good));
        let embedded = |file: &str| (file == "b.ldtk").then_some("{}");
        let manifest = good.manifest(&WorldRoots {
            loose_dir: std::path::Path::new("/w"),
            asset_prefix: "game://worlds/",
            embedded_prefix: "g/worlds/",
            embedded_text: &embedded,
        });
        assert_eq!(manifest.entry_room, "hub");
        assert_eq!(manifest.primary().asset_path, "game://worlds/a.ldtk");
        assert_eq!(manifest.primary().loose_path.as_deref(), Some(std::path::Path::new("/w/a.ldtk")));
        assert!(manifest.primary().required && manifest.primary().embedded_text.is_none());
        let b = manifest.secondaries().next().unwrap();
        assert_eq!((b.embedded_text, b.embedded_bevy_path, b.required), (Some("{}"), Some("g/worlds/b.ldtk"), false));

        let bad = authored(&[("world.a", "a.ldtk", false), ("world.a", "dir/b.json", false)]);
        let problems = world_manifest_problems(&bad);
        for field in ["`worlds[0].required` is false", "`worlds[1].file` is `dir/b.json`", "`worlds[1].id` `world.a` is stated twice"] {
            assert!(problems.iter().any(|problem| problem.starts_with(field)), "{field}: {problems:?}");
        }
        assert_eq!(problems.len(), 3, "{problems:?}");
    }
}
