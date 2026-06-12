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
//! [`traversal::possession`], whose `PossessionState` resource is
//! initialized by [`AmbitionAbilitiesPlugin`]. The possession *systems*
//! stay chained inside `crate::app::plugins::register_player_simulation_systems`
//! because they are interleaved (via `not_possessing` run conditions and a
//! single `.chain()`) with the player control / simulation tick; lifting
//! them would change execution order, so they are deliberately left in
//! place (Stage 17 — preserve ordering over tidiness).
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
/// `PossessionState` resource). All other abilities are pure logic modules
/// driven from combat / item-pickup / projectile systems and need no
/// registration. As abilities grow their own plugins/systems, fold them in
/// here so the ability layer keeps exactly one composition point.
pub struct AmbitionAbilitiesPlugin;

impl Plugin for AmbitionAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        // Possession's per-frame state. The possession *systems* remain
        // chained with the player tick in
        // `register_player_simulation_systems` (interleaved ordering); only
        // this standalone resource init is lifted here. Resource-init order
        // is independent of the system chain, so this is byte-equivalent to
        // the prior inline `init_resource` call.
        app.init_resource::<traversal::possession::PossessionState>();
    }
}
