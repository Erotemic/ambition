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

use crate::banter::CombatBanterRegistry;
use crate::character_sprites::build_npc_sprite_asset;
use crate::cutscene::{CutsceneLibrary, RoomCutsceneBindings};
use crate::game_assets::{GameAssetConfig, GameAssets};
use crate::rooms::PortalRegistry;

use super::banter::install_intro_banter;
use super::cutscene::{install_intro_cutscenes, intro_room_cutscene_bindings};
use super::sprites::intro_npc_sprite_rows;

/// Intro portal IDs. The gate stack room places:
/// - `LoadingZone` id `intro_portal_zone` (activation: Door) at
///   the portal frame. Targets `central_hub_complex/intro_wake_door`.
/// - `Switch` id `intro_portal_switch` next to the gate. Toggles
///   the portal's boot/shutdown sequence.
/// - `NpcSpawn` "Interdimensional Gate Portal" — the portal sprite
///   (hidden while phase == Off).
/// - `NpcSpawn` "Interdimensional Gate Ring" — the ring sprite
///   (always visible; rotates during phase == Opening).
///
/// The portal's *own* phase (Off / Opening / On / Closing) decides
/// whether Interact actually fires the transition. The switch only
/// commands open vs close.
pub const INTRO_PORTAL_ZONE_ID: &str = "intro_portal_zone";
pub const INTRO_PORTAL_SWITCH_ID: &str = "intro_portal_switch";
pub const INTRO_PORTAL_SPRITE_NAME: &str = "Interdimensional Gate Portal";
pub const INTRO_PORTAL_RING_NAME: &str = "Interdimensional Gate Ring";

/// Marker zero-sized resource — flips `true` once
/// [`load_intro_npc_sprites_system`] has run. Keeps the system idempotent
/// across the multi-frame startup window.
#[derive(Resource, Default, Debug)]
pub(crate) struct IntroSpritesInstalled(bool);

/// Marker zero-sized resource for the cutscene installer.
#[derive(Resource, Default, Debug)]
pub(crate) struct IntroCutscenesInstalled(bool);

/// Marker zero-sized resource for the banter installer.
#[derive(Resource, Default, Debug)]
pub(crate) struct IntroBanterInstalled(bool);

/// Marker zero-sized resource for the gated-zone installer.
#[derive(Resource, Default, Debug)]
pub(crate) struct IntroGatedZonesInstalled(bool);

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IntroSpritesInstalled>()
            .init_resource::<IntroCutscenesInstalled>()
            .init_resource::<IntroBanterInstalled>()
            .init_resource::<IntroGatedZonesInstalled>()
            // All contributor systems must wait for the sandbox's own
            // startup resources, but the sandbox inserts those via
            // `Startup` schedule and per-frame Commands. Running the
            // installers in `Update` with a "first chance" guard
            // (`if !installed`) is the simplest pattern that survives
            // Bevy's deferred command application without us having
            // to wire explicit system ordering.
            .add_systems(
                Update,
                (
                    install_intro_cutscenes_system,
                    load_intro_npc_sprites_system,
                    install_intro_banter_system,
                    install_intro_gated_zones_system,
                ),
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

/// Extend [`CombatBanterRegistry`] with the intro raiders' hit-bark
/// lines. Runs once — guarded by [`IntroBanterInstalled`].
pub(crate) fn install_intro_banter_system(
    mut installed: ResMut<IntroBanterInstalled>,
    registry: Option<ResMut<CombatBanterRegistry>>,
) {
    if installed.0 {
        return;
    }
    let Some(mut registry) = registry else {
        return;
    };
    install_intro_banter(&mut registry);
    installed.0 = true;
}

/// Register the intro portal in [`PortalRegistry`] so its lifecycle
/// runs every frame and traversal is gated on `phase == On`. Runs
/// once — guarded by [`IntroGatedZonesInstalled`].
pub(crate) fn install_intro_gated_zones_system(
    mut installed: ResMut<IntroGatedZonesInstalled>,
    registry: Option<ResMut<PortalRegistry>>,
) {
    if installed.0 {
        return;
    }
    let Some(mut registry) = registry else {
        return;
    };
    registry.register(
        INTRO_PORTAL_ZONE_ID,
        INTRO_PORTAL_SWITCH_ID,
        INTRO_PORTAL_SPRITE_NAME,
        INTRO_PORTAL_RING_NAME,
    );
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
