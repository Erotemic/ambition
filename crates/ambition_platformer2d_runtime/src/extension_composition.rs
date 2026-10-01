//! The extension host in the platformer composition (fast-iteration I4).
//!
//! The runtime chooses where each public phase sits in the simulation
//! schedule, and which domain adapters answer which ports. The host and the
//! adapters name no game algorithm; the game declares modules from its own
//! plugins.
//!
//! | phase | placement | ports |
//! |---|---|---|
//! | `technique_execution` | `CombatSet::ContentSpecials`, gameplay-gated | trigger `ambition.boss.special_cast` (boss domain); request `ambition.projectiles.spawn` (projectile domain) |

use ambition_extension_host::{ExtensionHostPlugin, ExtensionSet};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_platformer2d_shared_tangle::schedule::{CombatSet, GameplayGated, SimScheduleExt};
use bevy::prelude::*;

pub struct ExtensionCompositionPlugin;

impl Plugin for ExtensionCompositionPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_plugins(ExtensionHostPlugin::new(sim));
        // `technique_execution` guarantees: the asking body's facts are
        // settled, and a request is consumed this tick. `ContentSpecials`
        // sits before the effect and projectile executors that drain it.
        app.configure_sets(
            sim,
            (
                ExtensionSet::Collect(TECHNIQUE_EXECUTION),
                ExtensionSet::Invoke(TECHNIQUE_EXECUTION),
                ExtensionSet::Lower(TECHNIQUE_EXECUTION),
            )
                .in_set(GameplayGated)
                .in_set(CombatSet::ContentSpecials),
        );
        ambition_boss_encounter::extension::install(app);
        ambition_projectiles::extension::install(app);
    }
}
