//! `IntroPlugin` — Bevy plugin that wires the intro story content into
//! the live sandbox resources without forcing the sandbox to know about
//! the intro by name.
//!
//! The plugin contributes via startup systems:
//!
//! - [`install_intro_cutscenes_system`] extends
//!   [`crate::cutscene::CutsceneLibrary`] and
//!   [`crate::cutscene::RoomCutsceneBindings`] with the intro scripts
//!   and room bindings from [`crate::intro::cutscene`].
//! - [`load_intro_npc_sprites_system`] extends
//!   [`crate::game_assets::GameAssets`]`.characters.npcs` with the
//!   intro placeholder sprite rows from [`crate::intro::sprites`].
//!
//! Both systems run after the sandbox's own startup systems insert the
//! resources they extend, so they layer on top without overwriting
//! anything sandbox-owned.

use bevy::prelude::*;

use crate::character_sprites::build_npc_sprite_asset;
use crate::cutscene::{CutsceneLibrary, RoomCutsceneBindings};
use crate::game_assets::{GameAssetConfig, GameAssets};

use super::cutscene::{install_intro_cutscenes, intro_room_cutscene_bindings};
use super::sprites::intro_npc_sprite_rows;

/// Marker zero-sized resource — flips `true` once
/// [`load_intro_npc_sprites_system`] has run. Keeps the system idempotent
/// across the multi-frame startup window.
#[derive(Resource, Default, Debug)]
pub(crate) struct IntroSpritesInstalled(bool);

/// Marker zero-sized resource for the cutscene installer.
#[derive(Resource, Default, Debug)]
pub(crate) struct IntroCutscenesInstalled(bool);

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IntroSpritesInstalled>()
            .init_resource::<IntroCutscenesInstalled>()
            // Both systems must wait for the sandbox's own startup
            // resources, but the sandbox inserts those via `Startup`
            // schedule and via per-frame Commands. Running the
            // installers in `Update` with a "first chance" guard
            // (`if !installed`) is the simplest pattern that survives
            // Bevy's deferred command application without us having to
            // wire explicit system ordering.
            .add_systems(
                Update,
                (install_intro_cutscenes_system, load_intro_npc_sprites_system),
            );
    }
}

/// Extend [`CutsceneLibrary`] + [`RoomCutsceneBindings`] with the intro
/// scripts and bindings. Runs once — guarded by [`IntroCutscenesInstalled`].
pub(crate) fn install_intro_cutscenes_system(
    mut installed: ResMut<IntroCutscenesInstalled>,
    library: Option<ResMut<CutsceneLibrary>>,
    bindings: Option<ResMut<RoomCutsceneBindings>>,
) {
    if installed.0 {
        return;
    }
    // Both resources are inserted by `app/plugins.rs` at app build
    // time, so they should be present from the first Update tick.
    // The `Option<ResMut<_>>` keeps the system tolerant during the
    // narrow window where they might not be — and matches how
    // sandbox optional resources are usually accessed elsewhere.
    let (Some(mut library), Some(mut bindings)) = (library, bindings) else {
        return;
    };
    install_intro_cutscenes(&mut library);
    for (room_id, cutscene_id) in intro_room_cutscene_bindings() {
        bindings
            .bindings
            .push(((*room_id).to_string(), (*cutscene_id).to_string()));
    }
    installed.0 = true;
}

/// Extend `GameAssets.characters.npcs` with intro placeholder NPC sheets.
/// Runs once — guarded by [`IntroSpritesInstalled`].
pub(crate) fn load_intro_npc_sprites_system(
    mut installed: ResMut<IntroSpritesInstalled>,
    config: Option<Res<GameAssetConfig>>,
    asset_server: Option<Res<AssetServer>>,
    layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
    game_assets: Option<ResMut<GameAssets>>,
) {
    if installed.0 {
        return;
    }
    // `GameAssets` is inserted by `setup_presentation_system` partway
    // through startup. Wait for it before installing intro sprites.
    let (
        Some(config),
        Some(asset_server),
        Some(mut layouts),
        Some(mut game_assets),
    ) = (config, asset_server, layouts, game_assets)
    else {
        return;
    };
    if config.no_assets {
        // `--no-assets` short-circuits every disk load. Skip without
        // marking installed so a later toggle still wires sprites in.
        installed.0 = true;
        return;
    }
    for (name, filename, spec) in intro_npc_sprite_rows() {
        if game_assets.characters.npcs.contains_key(*name) {
            continue;
        }
        if let Some(asset) = build_npc_sprite_asset(
            &asset_server,
            &mut layouts,
            &config.sprite_folder,
            filename,
            *spec,
        ) {
            game_assets.characters.npcs.insert(*name, asset);
        } else {
            eprintln!(
                "[intro] NPC sheet '{name}' not found at assets/{}/{} — falling back to colored rectangle",
                config.sprite_folder, filename
            );
        }
    }
    installed.0 = true;
}
