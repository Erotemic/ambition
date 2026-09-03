//! The WIELDED ability kit, carved out of the actor kernel (D33, 2026-09-03).
//!
//! * [`ranged`] — beam, meteor, shockwave, volley, vortex, sentry, bomb
//! * [`thrown`] — the gravity grenade
//! * [`traversal`] — blink, dive, grapple, mark/recall
//! * [`ability_cooldown`] — the cooldown every one of them shares
//!
//! ⛔⛔ WHAT IS DELIBERATELY NOT HERE, and the reason is the whole shape of the
//! carve. `possession`, `teleport`, `trapdoor` and `flyline` live in a directory
//! called `abilities/` in the kernel and are NOT abilities: their systems are
//! registered by `ambition_platformer2d_runtime`, not by the item-pickup family,
//! and `possession` is named 87 times outside that directory (`teleport` 61) by
//! `body_custody`, `control::authority`, `features::ecs::dormancy` and
//! `control::input_systems`. That is control authority. Moving it here would
//! give this crate a home the RUNTIME registers systems out of and the KERNEL
//! depends on for `PossessionState` — the coupling renamed rather than reduced,
//! which is exactly what a carve must not do.
//! `docs/planning/engine/actor-monolith-decomposition.md` carries the argument
//! and the numbers so nobody carves them by line count later.
//!
//! ⚠ `thrown::puppy_slug_gun` is also NOT here, for a different and more
//! interesting reason: it SPAWNS A BODY, through the kernel-private
//! `features::spawn_runtime_minion`. The cross-crate seam for that already
//! exists — `ambition_vfx::Effect::Summon` carrying a `SummonSpec`, which
//! `ambition_combat` emits and the kernel's actor-construction executor
//! materialises into `ActorConstructionParams::SummonedMinion` — so the gun is
//! the one caller BYPASSING the canonical construction model rather than a
//! caller missing an abstraction. Routing it through needs two additions to
//! `SummonSpec` (the summon's `ActorAggression`, and the ally marker the gun
//! inserts after the spawn), which is a behaviour change and not a file move.
//! It stays until that is done deliberately.
//!
//! ⭐ THE SCHEDULE, END TO END, WHICH IS D33'S ACTUAL REQUIREMENT.
//! [`AbilitySimulationPlugin`] configures `ItemPickupSet::ThrownItemEffects` and
//! `ItemPickupSet::WieldedAbilities` — their nesting in `PlayerSimulation` — AND
//! registers every one of their members. It does NOT configure
//! `ItemPickupSet::CoreHeldItems` (that belongs to `ambition_held_items`) and it
//! does NOT declare the three-variant `.chain()`: that edge orders sets owned by
//! two other crates, so only the kernel can name both sides, and the kernel
//! keeps it. A composition that installs this plugin alone gets both variants
//! correctly nested and no chain to `CoreHeldItems`, which is right for a unit
//! fixture and wrong for the game — the game installs the kernel too.

pub mod ability_cooldown;
pub mod ranged;
pub mod thrown;
pub mod traversal;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

use bevy::prelude::*;
use ambition_platformer2d_shared_tangle::schedule::{GameplayGated, ItemPickupSet, SimScheduleExt};

/// Registers the wielded-ability half of the item schedule.
pub struct AbilitySimulationPlugin;

impl Plugin for AbilitySimulationPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Both variants nest in the player phase. The ORDER between them and
        // `CoreHeldItems` is the kernel's to declare — see the module header.
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

        // Wielded movement/combat items live in their own group to avoid the
        // chained tuple arity cap in the core held-item group.
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
                ranged::vortex::update_vortex_wells.in_set(GameplayGated),
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
