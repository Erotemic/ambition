//! `IntroPlugin`: wires the intro story content into the live sandbox
//! resources without the sandbox naming the intro.
//!
//! The intro's cutscenes, room bindings, raider barks and gate portal join
//! their registries when the plugin builds. Every contributor adds rows and
//! none replaces a registry, so the registries are complete before the first
//! tick, and the order in which the plugins are added does not matter.
//! (`Plugin::finish` is not used: an app driven by `App::update` does not run
//! it, and an app can run it twice. See `app_finalization`.)

use bevy::prelude::*;

// The unified dialog redirect system in the sandbox `dialog` module owns its
// own scheduling.
use crate::banter::CombatBanterRegistry;
use ambition_platformer2d::world::rooms::GatePortalRegistry;
use ambition_platformer2d_actor_monolith::assets::game_assets::{PropSheetSource, PropSheetsAppExt};

use super::banter::install_intro_banter;
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

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        // The intro props' sheets load with the rest of the art.
        for (kind, _file, spec, pack_target) in intro_prop_sprite_rows() {
            app.register_prop_sheet(
                kind,
                PropSheetSource::Catalog {
                    asset: crate::intro::sprites::intro_prop_asset_id(kind),
                    spec,
                    pack_target: pack_target.map(str::to_string),
                },
            );
        }
        // The flag chains are not an installer. They re-derive a table from
        // the save every frame.
        //
        // Its cost grows with the save: `SaveData::flag` is a linear scan with a
        // string compare, asked twice per table row. So it is change-gated. A chain
        // fires only when its flags move, and a flag this system writes marks the
        // save changed again, so a chain of chains resolves on the next frame.
        //
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
        // The intro's rows join the shared registries here. Every contributor
        // adds rows and none replaces a registry, so the rows are there before
        // the first tick whatever the plugin order.
        let world = app.world_mut();
        install_intro_banter(&mut world.get_resource_or_init::<CombatBanterRegistry>());
        // A refusal is a content bug: another portal already claimed this
        // loading zone. Logged, not panicked, because a missing portal leaves
        // a recoverable world.
        if let Err(conflict) = world.get_resource_or_init::<GatePortalRegistry>().try_register(
            INTRO_PORTAL_ZONE_ID,
            INTRO_PORTAL_SWITCH_ID,
            INTRO_PORTAL_SPRITE_NAME,
            INTRO_PORTAL_RING_NAME,
        ) {
            bevy::log::error!("the intro portal was refused: {conflict}");
        }

        // Intro dialog redirects are authored in content, so nothing is registered
        // here. They are `<<if>>` branches in the `.yarn` files (for example
        // `<<if boss_cleared("mockingbird")>>` and
        // `<<if quest_active("pirate_treasure")>>` in `assets/dialogue/sandbox/`).
        // There is no redirect system in Rust.
    }
}
