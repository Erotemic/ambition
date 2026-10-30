//! The wielded ability kit, split from the actor kernel (D33).
//!
//! * [`ranged`]: the bomb
//! * [`extension`] and [`module_entity`]: the extension adapters of the
//!   wielded abilities that are procedural modules (shockwave, beam, volley,
//!   meteor, sentry, vortex, blink, dive, mark/recall, grapple)
//! * [`ride`]: the extension adapters of a summoned ride
//! * [`thrown`]: the gravity grenade, and the puppy-slug gun (a summon of a
//!   companion through `ambition_vfx::Effect::Summon`)
//! * [`traversal`]: the blink rule, the mark of mark/recall
//! * [`ability_cooldown`]: the shared cooldown
//!
//! Not here on purpose: `possession`, `teleport`, `trapdoor`, and `flyline`.
//! They sit in the kernel's `abilities/` directory, but they are control
//! authority: `ambition_platformer2d_runtime` registers their systems, and
//! `body_custody`, `control::authority`, `body::dormancy`, and
//! `control::input_systems` use them. Moving them here would rename the
//! coupling, not reduce it. See
//! `docs/planning/engine/actor-monolith-decomposition.md`.
//!
//! Schedule: [`AbilitySimulationPlugin`] configures
//! `ItemPickupSet::ThrownItemEffects` and `ItemPickupSet::WieldedAbilities`
//! (nested in `PlayerSimulation`) and registers all their members. It does
//! not configure `ItemPickupSet::CoreHeldItems` (owned by
//! `ambition_held_items`) or the three-set `.chain()`, which orders sets from
//! two other crates and so belongs to the kernel. This plugin alone is right
//! for a unit fixture; the game also installs the kernel.

pub mod ability_cooldown;
pub mod body_conditions;
pub mod extension;
pub mod mana;
pub mod module_entity;
pub mod ranged;
pub mod ride;
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
            (ItemPickupSet::ThrownItemEffects, ItemPickupSet::WieldedAbilities)
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::PlayerSimulation,
                )
                // Thrown-item effects and wielded abilities (with the wielded
                // extension phases in that set) are held use: they sleep where
                // the active room's game uses no held items.
                .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(
                    ambition_platformer2d_shared_tangle::capability::Capability::HeldUse,
                )),
        );

        // Bombs and gravity grenades run after the held-item throw/physics group.
        app.add_systems(
            sim,
            (
                ranged::bomb::arm_thrown_bombs.in_set(GameplayGated),
                // The fuse passes visit lit fuses; the arming passes visit every
                // ground item, so they stay awake.
                ranged::bomb::tick_bomb_fuses.run_if(any_with_component::<ranged::bomb::BombFuse>).in_set(GameplayGated),
                thrown::gravity_grenade::arm_thrown_gravity_grenades.in_set(GameplayGated),
                thrown::gravity_grenade::tick_gravity_grenade_fuses.run_if(any_with_component::<thrown::gravity_grenade::GravityGrenadeFuse>).in_set(GameplayGated),
                ambition_platformer2d_shared_tangle::gravity::tick_temporary_zones
                    .run_if(any_with_component::<ambition_platformer2d_shared_tangle::gravity::TemporaryZone>)
                    .in_set(GameplayGated),
            )
                .chain()
                .in_set(ItemPickupSet::ThrownItemEffects),
        );

        // A bomb's body is given in the item step BEFORE the physics step, so
        // a bomb thrown this tick moves as a bomb on its first step.
        app.add_systems(
            sim,
            ranged::bomb::weigh_bombs
                .run_if(any_with_component::<ambition_held_items::GroundItem>)
                .in_set(GameplayGated)
                .in_set(ambition_platformer2d_shared_tangle::schedule::HeldItemStep::Settle),
        );

        // Wielded movement/combat items have their own group, to stay under
        // the chained tuple arity cap of the core held-item group.
        app.add_systems(
            sim,
            (
                ability_cooldown::tick_ability_cooldown,
                // After the whole held-item chain: nothing in it moves, arms or
                // takes the summoner's hand on a plain Attack, and Shield+Attack
                // (the throw) is refused by the gun itself.
                thrown::puppy_slug_gun::fire_puppy_slug_gun_system.in_set(GameplayGated),
            )
                .in_set(ItemPickupSet::WieldedAbilities),
        );
    }
}

#[cfg(test)]
mod schedule_tests;

mod rollback_registration;
pub use rollback_registration::register_rollback_state;
