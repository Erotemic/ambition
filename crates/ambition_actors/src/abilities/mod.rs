//! Ambition's player ability / weapon kit.
//!
//! The 14 loose ability/weapon mechanics that used to live at the crate
//! root now share one home here, grouped by kind:
//!
//! * [`traversal`] — blink, dive, grapple, possession, mark/recall
//! * [`ranged`] — beam, meteor, shockwave, vortex, volley, bomb, sentry
//! * [`thrown`] — gravity grenade, puppy-slug gun
//!
//! Each submodule keeps its own self-contained logic; most are pure
//! functions invoked from combat / item-pickup / projectile code and
//! register nothing with the Bevy `App`. The one exception is
//! [`traversal::possession`], whose `PossessionState` + `ControlledSubject`
//! resources are initialized by [`AmbitionAbilitiesPlugin`]. The possession
//! *systems* stay chained inside
//! `crate::schedule::plugins::register_player_simulation_systems` alongside the
//! player tick; possession is now pure brain transfer, so there is no
//! `not_possessing` control gate — the vacated home avatar is inert because it
//! no longer carries a player brain.
//!
//! This module is a neutral, top-level ability layer (a sibling of
//! `crate::mechanics`), distinct from `ambition_engine_core::abilities` in
//! the engine-core crate.

#[cfg(test)]
pub(crate) mod test_support;

pub mod ranged;
pub mod thrown;
pub mod traversal;

use bevy::prelude::*;

/// Umbrella plugin composing every Ambition player ability's `App`
/// registration.
///
/// Today the only ability that owns Bevy `App` state is possession (its
/// `PossessionState` + `ControlledSubject` resources). All other abilities are
/// pure logic modules driven from combat / item-pickup / projectile systems and
/// need no registration. As abilities grow their own plugins/systems, fold them
/// in here so the ability layer keeps exactly one composition point.
pub struct AmbitionAbilitiesPlugin;

impl Plugin for AmbitionAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        // Possession's brain-transfer bookkeeping + the derived "who is the
        // player driving" handle every subject query reads. The possession
        // *systems* remain chained with the player tick in
        // `register_player_simulation_systems`.
        app.init_resource::<traversal::possession::PossessionState>();
        app.init_resource::<ambition_platformer_primitives::markers::ControlledSubject>();
    }
}
