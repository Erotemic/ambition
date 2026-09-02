//! `IntroPlugin` — Bevy plugin that wires the intro story content into
//! the live sandbox resources without forcing the sandbox to know about
//! the intro by name.
//!
//! The plugin contributes via startup systems:
//!
//! - [`install_intro_cutscenes_system`] extends
//!   [`ambition_cutscene::CutsceneLibrary`] and
//!   [`ambition_cutscene::RoomCutsceneBindings`] with the intro scripts
//!   and room bindings from [`crate::intro::cutscene`].
//!
//! Both systems run after the sandbox's own startup systems insert the
//! resources they extend, so they layer on top without overwriting
//! anything sandbox-owned.

use bevy::prelude::*;

// `Platformer2dSimulationPhaseMonolith` import retired alongside the legacy
// `redirect_post_intro_dialog` ordering — the unified dialog redirect
// system in the sandbox `dialog` module owns its own scheduling.
use crate::banter::CombatBanterRegistry;
use ambition_cutscene::{CutsceneLibrary, RoomCutsceneBindings};
use ambition_platformer2d::world::rooms::GatePortalRegistry;
use ambition_platformer2d_actor_monolith::character_sprites::{
    build_prop_sprite_asset, build_prop_sprite_asset_packed,
};
use ambition_render::quality::ResolvedVisualQuality;
use ambition_sprite_sheet::game_assets::{GameAssetConfig, GameAssets};

use super::banter::install_intro_banter;
use super::cutscene::{install_intro_cutscenes, intro_room_cutscene_bindings};
use super::sprites::intro_prop_sprite_rows;

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

/// Marker zero-sized resource — guards
/// [`load_intro_prop_sprites_system`]. Props keep their loader because a
/// `Prop` is keyed by `Prop.kind`, which the world does author; the NPC
/// equivalent is gone (see `crate::intro::sprites`).
#[derive(Resource, Default, Debug)]
pub(crate) struct IntroPropSpritesInstalled(bool);

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
        // What is left here is content INSTALLATION, which is what an intro-content plugin
        // should be.
        app.init_resource::<IntroPropSpritesInstalled>()
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
                    load_intro_prop_sprites_system,
                    install_intro_banter_system,
                    install_intro_gated_zones_system,
                ),
            )
            // ⛔ THE FLAG CHAINS ARE NOT AN INSTALLER and do not belong in the
            // tuple above. The five beside them are one-shot latches guarded by
            // `if installed { return; }`, which MUST keep running so they can
            // observe `GameAssets` / `CutsceneLibrary` arriving. This one
            // re-derives a table from the save every frame FOREVER, long after
            // every flag in it is set.
            //
            // ⭐ AND ITS COST GROWS WITH THE SAVE. `SaveData::flag` is a linear
            // scan with a string compare over a flag vector that lengthens as
            // the player progresses, and this asks it twice per table row every
            // frame. Change detection is the right shape: the chains can only
            // fire when the flags they read have moved, and a flag this system
            // writes marks the save changed again, so a chain-of-chains still
            // resolves on the following frame.
            .add_systems(
                Update,
                super::route_state::emit_intro_flag_chains.run_if(
                    bevy::prelude::resource_exists_and_changed::<
                        ambition_persistence::save::AmbitionGameSave,
                    >,
                ),
            );
        // ⚠ THIS COMMENT DESCRIBES MACHINERY THAT NO LONGER EXISTS, and it is
        // left saying so rather than repointed, because there is nothing to
        // repoint AT. It read: "Intro dialog redirects are handled by the
        // unified `dialog::redirect_post_quest_dialog` system" (cite-ok), whose
        // ordering was after CoreSimulation and before DialogPresentationSet.
        // Checked 2026-09-02: no `redirect_post_quest_dialog` and no function
        // named `*redirect*` survives anywhere in `ambition_content`, and
        // `intro::dialog` exports only `intro_dialogue_ids`.
        //
        // ⇒ EITHER post-quest redirects moved to a mechanism nobody recorded, OR
        // they are gone and the intro quietly lost a behaviour. This block
        // registers nothing, so a reader cannot tell which from here. Worth one
        // person's five minutes who knows the dialogue road.
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

/// Register the intro portal in [`GatePortalRegistry`] so its lifecycle
/// runs every frame and traversal is gated on `phase == On`. Runs
/// once — guarded by [`IntroGatedZonesInstalled`].
pub(crate) fn install_intro_gated_zones_system(
    mut installed: ResMut<IntroGatedZonesInstalled>,
    registry: Option<ResMut<GatePortalRegistry>>,
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

/// Extend `GameAssets.characters.props` with intro prop sheets keyed
/// by `Prop.kind`. Runs once — guarded by
/// [`IntroPropSpritesInstalled`].
pub(crate) fn load_intro_prop_sprites_system(
    mut installed: ResMut<IntroPropSpritesInstalled>,
    config: Option<Res<GameAssetConfig>>,
    asset_server: Option<Res<AssetServer>>,
    layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
    game_assets: Option<ResMut<GameAssets>>,
    catalog: Option<Res<ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog>>,
    quality: Option<Res<ResolvedVisualQuality>>,
) {
    if installed.0 {
        return;
    }
    let (Some(config), Some(asset_server), Some(mut layouts), Some(mut game_assets), Some(catalog)) =
        (config, asset_server, layouts, game_assets, catalog)
    else {
        return;
    };
    if config.no_assets {
        installed.0 = true;
        return;
    }
    for (kind, filename, spec, pack_target) in intro_prop_sprite_rows() {
        if game_assets.characters.props.contains_key(kind) {
            continue;
        }
        // Shared-pack path first for opted-in props: the quality-tiered
        // ultrapack pages + catalog-synthesized spec. Falls back to the
        // per-target sheet below when no pack was generated / gated.
        if let Some(target) = pack_target {
            if let Some(asset) = build_prop_sprite_asset_packed(
                &catalog,
                &asset_server,
                &mut layouts,
                target,
                &spec,
                quality.as_deref().map(|q| &q.budget),
            ) {
                bevy::log::info!(
                    target: "ambition_platformer2d::sprite_packs",
                    "prop '{kind}' bound to shared sprite pack (target '{target}')",
                );
                game_assets
                    .characters
                    .props
                    .insert((*kind).to_string(), asset);
                continue;
            }
        }
        let id = crate::intro::sprites::intro_prop_asset_id(kind);
        if let Some(asset) =
            build_prop_sprite_asset(&catalog, &asset_server, &mut layouts, &id, &spec)
        {
            game_assets
                .characters
                .props
                .insert((*kind).to_string(), asset);
        } else {
            eprintln!(
                "[intro] Prop sheet '{kind}' (catalog id {id}) not loadable under {} \
                 profile (logical {}/{filename}) — falling back to colored rectangle",
                catalog.profile().label(),
                config.sprite_folder,
            );
        }
    }
    installed.0 = true;
}
