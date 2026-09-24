//! The wielded ability kit, split from the actor kernel (D33).
//!
//! * [`ranged`]: beam, meteor, shockwave, volley, vortex, sentry, bomb
//! * [`thrown`]: the gravity grenade
//! * [`traversal`]: blink, dive, grapple, mark/recall
//! * [`ability_cooldown`]: the shared cooldown
//!
//! Not here on purpose: `possession`, `teleport`, `trapdoor`, and `flyline`.
//! They sit in the kernel's `abilities/` directory, but they are control
//! authority: `ambition_platformer2d_runtime` registers their systems, and
//! `body_custody`, `control::authority`, `features::ecs::dormancy`, and
//! `control::input_systems` use them. Moving them here would rename the
//! coupling, not reduce it. See
//! `docs/planning/engine/actor-monolith-decomposition.md`.
//!
//! `thrown::puppy_slug_gun` is also not here: it spawns a body through the
//! kernel-private `features::spawn_runtime_minion`. The canonical seam is
//! `ambition_vfx::Effect::Summon` with a `SummonSpec`, materialized by the
//! kernel into `ActorConstructionParams::SummonedMinion`. Using it needs two
//! `SummonSpec` additions (the summon's `ActorAggression`, and the ally
//! marker), which is a behavior change, not a file move.
//!
//! Schedule: [`AbilitySimulationPlugin`] configures
//! `ItemPickupSet::ThrownItemEffects` and `ItemPickupSet::WieldedAbilities`
//! (nested in `PlayerSimulation`) and registers all their members. It does
//! not configure `ItemPickupSet::CoreHeldItems` (owned by
//! `ambition_held_items`) or the three-set `.chain()`, which orders sets from
//! two other crates and so belongs to the kernel. This plugin alone is right
//! for a unit fixture; the game also installs the kernel.

pub mod ability_cooldown;
pub mod mana;
pub mod ranged;
pub mod thrown;
pub mod traversal;

#[cfg(any(test, feature = "test-support"))]
// Test-only. `any(test, feature)`, not the feature alone: this crate's own
// tests use `crate::test_support`, so feature-only gating breaks
// `cargo test -p ambition_abilities`, while workspace feature unification
// hides that. It writes `ControlledSubject`, so ungated it would look like a
// production control authority.
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

use bevy::prelude::*;
use ambition_platformer2d_shared_tangle::schedule::{GameplayGated, ItemPickupSet, SimScheduleExt};

/// Registers the wielded-ability half of the item schedule.
pub struct AbilitySimulationPlugin;

impl Plugin for AbilitySimulationPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Both variants nest in the player phase. Their order relative to
        // `CoreHeldItems` is the kernel's; see the module header.
        app.configure_sets(
            sim,
            (ItemPickupSet::ThrownItemEffects, ItemPickupSet::WieldedAbilities).in_set(
                ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PlayerSimulation,
            ),
        );

        // Bombs and gravity grenades run after the held-item throw/physics group.
        app.add_systems(
            sim,
            (
                ranged::bomb::arm_thrown_bombs.in_set(GameplayGated),
                ranged::bomb::tick_bomb_fuses.in_set(GameplayGated),
                thrown::gravity_grenade::arm_thrown_gravity_grenades.in_set(GameplayGated),
                thrown::gravity_grenade::tick_gravity_grenade_fuses.in_set(GameplayGated),
                ambition_platformer2d_shared_tangle::gravity::tick_temporary_zones
                    .in_set(GameplayGated),
            )
                .chain()
                .in_set(ItemPickupSet::ThrownItemEffects),
        );

        // Wielded movement/combat items have their own group, to stay under
        // the chained tuple arity cap of the core held-item group.
        app.add_systems(
            sim,
            (
                traversal::mark_recall::mark_recall_system.in_set(GameplayGated),
                traversal::blink::blink_system.in_set(GameplayGated),
                traversal::grapple::grapple_system.in_set(GameplayGated),
                ranged::shockwave::fire_shockwave_system.in_set(GameplayGated),
                ranged::volley::fire_volley_system.in_set(GameplayGated),
                ranged::beam::fire_beam_system.in_set(GameplayGated),
                ranged::vortex::fire_vortex_system.in_set(GameplayGated),
                ranged::vortex::update_vortex_wells
                    .in_set(GameplayGated)
                    .in_set(ambition_platformer2d_shared_tangle::schedule::BodyPathSet::Carry),
                ranged::sentry::fire_sentry_system.in_set(GameplayGated),
                ranged::sentry::update_sentries.in_set(GameplayGated),
                traversal::dive::fire_dive_system.in_set(GameplayGated),
                ranged::meteor::fire_meteor_system.in_set(GameplayGated),
                ability_cooldown::tick_ability_cooldown,
            )
                .chain()
                .in_set(ItemPickupSet::WieldedAbilities),
        );
    }
}

#[cfg(test)]
mod schedule_tests;
