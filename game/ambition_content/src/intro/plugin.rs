//! `IntroPlugin`: wires the intro story content into the live sandbox
//! resources without the sandbox naming the intro.
//!
//! The plugin contributes via startup systems:
//!
//! - [`install_intro_cutscenes_system`] extends
//!   [`ambition_cutscene::CutsceneLibrary`] and
//!   [`ambition_cutscene::RoomCutsceneBindings`] with the intro scripts
//!   and room bindings from [`crate::intro::cutscene`].
//!
//! The installers run after the sandbox's own startup systems insert the
//! resources they extend, so they add to them without overwriting anything
//! sandbox-owned.

use bevy::prelude::*;

// The unified dialog redirect system in the sandbox `dialog` module owns its
// own scheduling.
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
        // What is left here is content installation.
        app.init_resource::<IntroPropSpritesInstalled>()
            .init_resource::<IntroCutscenesInstalled>()
            .init_resource::<IntroBanterInstalled>()
            .init_resource::<IntroGatedZonesInstalled>()
            // The installers must wait for the sandbox's startup resources, which
            // arrive through the `Startup` schedule and deferred Commands. Running them
            // in `Update` with a first-chance guard (`if !installed`) survives deferred
            // command application without explicit ordering.
            .add_systems(
                Update,
                (
                    install_intro_cutscenes_system,
                    load_intro_prop_sprites_system,
                    install_intro_banter_system,
                    install_intro_gated_zones_system,
                ),
            )
            // The flag chains are not an installer and do not belong in the tuple
            // above. The installers are one-shot latches (`if installed { return; }`)
            // that keep running to observe `GameAssets` / `CutsceneLibrary` arriving.
            // The flag chains re-derive a table from the save every frame.
            //
            // Its cost grows with the save: `SaveData::flag` is a linear scan with a
            // string compare, asked twice per table row. So it is change-gated. A chain
            // fires only when its flags move, and a flag this system writes marks the
            // save changed again, so a chain of chains resolves on the next frame.
            ;
        // In the rewinding schedule, not `Update`, because the consumer rewinds and
        // the message does not. `apply_flag_effects` reads `SetFlagRequested` and
        // writes `AmbitionGameSave` and `QuestRegistry`, both
        // `rollback_resource_clone_checksum`-registered. A message raised from
        // `Update` is outside the resimulated frame, so the replay would diverge on
        // a checksummed value. Deriving inside the schedule is possible because this
        // producer is a pure derivation over rollback state, not a latched input
        // edge; see `Q136` in `docs/planning/awaiting-maintainer-decision.md` for the
        // intents that cannot do this.
        //
        // The change gate survives the rewind: `bevy_ggrs` restores a resource with
        // `S::update(resource.as_mut(), snapshot)`, and `ResMut::as_mut` marks it
        // changed, so every restore re-arms the condition. The gate still saves the
        // linear flag scans on ordinary frames.
        //
        // A restore can therefore run the derivation on a tick the original
        // timeline did not. That is harmless: the system skips any target already
        // present, so an extra run writes nothing.
        //
        // After `GameplayEffects`, where `apply_flag_effects` runs
        // (`ambition_platformer2d_actor_monolith/src/features/mod.rs`), because the
        // behaviour is next-tick: a target flows through the ordinary flag-effect
        // path on the following tick, with its quest notification. Ordering it
        // before would collapse a chain into one tick and change when notifications
        // fire.
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            super::route_state::emit_intro_flag_chains
                .after(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::GameplayEffects,
                )
                .run_if(
                    bevy::prelude::resource_exists_and_changed::<
                        ambition_persistence::save::AmbitionGameSave,
                    >,
                ),
        );
        // Intro dialog redirects are authored in content, so nothing is registered
        // here. They are `<<if>>` branches in the `.yarn` files (for example
        // `<<if boss_cleared("mockingbird")>>` and
        // `<<if quest_active("pirate_treasure")>>` in `assets/dialogue/sandbox/`).
        // There is no redirect system in Rust.
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
    // `app/plugins.rs` inserts both resources at app build time, so they should
    // exist from the first Update tick. `Option<ResMut<_>>` tolerates the
    // narrow window where they might not.
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
    // A refusal here is a content bug: another portal already claimed this
    // loading zone. Logged, not panicked, because a missing portal leaves a
    // recoverable world.
    if let Err(conflict) = registry.try_register(
        INTRO_PORTAL_ZONE_ID,
        INTRO_PORTAL_SWITCH_ID,
        INTRO_PORTAL_SPRITE_NAME,
        INTRO_PORTAL_RING_NAME,
    ) {
        bevy::log::error!("the intro portal was refused: {conflict}");
    }
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
