//! Ambition's LDtk WORLD payload + its `WorldManifest` — CONTENT, evicted
//! from the engine core (R3.2, the #1 violation: the engine shipped the
//! game's worlds).
//!
//! The engine keeps the room kit (`RoomSpec`/`RoomSet`, projection,
//! validators) and the manifest-driven loader; THIS module declares which
//! `.ldtk` files exist, where play starts, and how each file is reachable:
//!
//! - `loose_path` — the checked-in file under this crate's `assets/worlds/`
//!   (desktop dev + hot reload; the LDtk python tooling edits these).
//! - `embedded_text` — the JSON embedded into the binary under the
//!   `static_map` feature (web / Android / bundled builds; also the
//!   desktop disk-failure fallback).
//! - `asset_path` — the Bevy `AssetPath` the bevy_ecs_ldtk tile-render
//!   spine loads, under the `game://` asset source the app registers
//!   (rooted at this crate's `assets/` in dev, the shipped `assets/` dir
//!   in installed builds).

use std::path::Path;

use ambition_asset_manager::AssetId;
use ambition_gameplay_core::ldtk_world::{WorldManifest, WorldSource};

macro_rules! static_world_text {
    ($name:ident, $path:literal) => {
        #[cfg(feature = "static_map")]
        const $name: Option<&'static str> = Some(include_str!($path));
        #[cfg(not(feature = "static_map"))]
        const $name: Option<&'static str> = None;
    };
}

static_world_text!(SANDBOX_LDTK_STATIC, "../assets/worlds/sandbox.ldtk");
static_world_text!(INTRO_LDTK_STATIC, "../assets/worlds/intro.ldtk");
static_world_text!(
    CUT_ROPE_LDTK_STATIC,
    "../assets/worlds/you_have_to_cut_the_rope.ldtk"
);
static_world_text!(HALL_LDTK_STATIC, "../assets/worlds/hall_of_characters.ldtk");

/// Install Ambition's world manifest into the engine's seam. Called from
/// the app's sim-entry choke points and `AmbitionContentPlugin::build`;
/// first install wins.
pub fn install() {
    ambition_gameplay_core::ldtk_world::install_world_manifest(world_manifest());
}

/// The game's world declaration. The first row (sandbox) is boot-critical
/// and hot-reload-watched; the story side-worlds are tolerated missing so
/// a partial checkout still boots.
pub fn world_manifest() -> WorldManifest {
    let worlds_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/worlds");
    let source = |id: &str,
                  file: &str,
                  embedded_text: Option<&'static str>,
                  embedded_bevy_path: &'static str,
                  required: bool| WorldSource {
        id: AssetId::new(id),
        asset_path: format!("game://worlds/{file}"),
        loose_path: Some(worlds_dir.join(file)),
        embedded_text,
        embedded_bevy_path: Some(embedded_bevy_path),
        required,
    };
    WorldManifest {
        entry_room: "central_hub_complex".to_string(),
        // No baked ron-rooms shipped yet: generated rooms land here when a
        // bake tool emits them (W2 loader is live; see world::ron_room).
        ron_rooms: Vec::new(),
        worlds: vec![
            source(
                "world.sandbox_ldtk",
                "sandbox.ldtk",
                SANDBOX_LDTK_STATIC,
                "ambition_content/worlds/sandbox.ldtk",
                true,
            ),
            source(
                "world.intro_ldtk",
                "intro.ldtk",
                INTRO_LDTK_STATIC,
                "ambition_content/worlds/intro.ldtk",
                false,
            ),
            source(
                "world.cut_rope_ldtk",
                "you_have_to_cut_the_rope.ldtk",
                CUT_ROPE_LDTK_STATIC,
                "ambition_content/worlds/you_have_to_cut_the_rope.ldtk",
                false,
            ),
            source(
                "world.hall_ldtk",
                "hall_of_characters.ldtk",
                HALL_LDTK_STATIC,
                "ambition_content/worlds/hall_of_characters.ldtk",
                false,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_names_the_four_worlds_and_the_hub_entry() {
        let manifest = world_manifest();
        assert_eq!(manifest.entry_room, "central_hub_complex");
        assert_eq!(manifest.worlds.len(), 4);
        assert!(manifest.primary().required);
        assert_eq!(manifest.primary().id.as_str(), "world.sandbox_ldtk");
        for world in &manifest.worlds {
            assert!(
                world.loose_path.as_ref().is_some_and(|path| path.is_file()),
                "world file missing on disk: {:?}",
                world.loose_path
            );
        }
    }
}
