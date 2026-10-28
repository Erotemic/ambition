//! `IntroPlugin`: wires the intro story content into the live sandbox
//! resources without the sandbox naming the intro.
//!
//! The intro's cutscenes, room bindings and raider barks join their
//! registries when the plugin builds. Every contributor adds rows and
//! none replaces a registry, so the registries are complete before the first
//! tick, and the order in which the plugins are added does not matter.
//! (`Plugin::finish` is not used: an app driven by `App::update` does not run
//! it, and an app can run it twice. See `app_finalization`.)

use bevy::prelude::*;



/// The landing pad of the wake room: the zone the door of the hub arrives at.
/// It names no target, so nothing leaves through it.
pub const WAKE_ROOM_ARRIVAL_ZONE_ID: &str = "wake_room_arrival";

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        // The flag chains the selected pack states (`data/flag_chains.ron`).
        super::route_state::register(app);
        // The hub has a door to this room and this room has no door back: the
        // zone a body arrives at is a place and not a door, so it has no
        // picture.
        {
            use ambition_render::rendering::gate_portal_visuals::HiddenLoadingZonesAppExt;
            app.hide_loading_zone(WAKE_ROOM_ARRIVAL_ZONE_ID);
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
        // (`ambition_platformer2d_actor_monolith/src/body/mod.rs`), because the
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
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::GameplayEffects,
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
