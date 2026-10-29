//! Mana regeneration for the driven bodies.

use bevy::prelude::*;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use super::ManaRegen;

/// Refill declared Mana at the composition's rate in simulation time.
pub fn regenerate_driven_mana(
    time: Res<ambition_time::WorldTime>,
    driven: ambition_held_items::DrivenBodies,
    mut banks: Query<&mut ambition_platformer2d_core::resources::ActorResources>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    policy: Option<Res<ManaRegen>>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    // The composition's rate; a composition that states none refills nothing.
    let Some(per_second) = policy.map(|p| p.0) else {
        return;
    };
    if per_second <= 0.0 {
        return;
    }
    // Use the home body while the driving projection is unavailable.
    let mut subjects = driven.entities();
    if subjects.is_empty() {
        subjects.extend(primary.single().ok());
    }
    for subject in subjects {
        if let Some(mana) = banks
            .get_mut(subject)
            .ok()
            .and_then(|bank| bank.into_inner().level_of_mut(&super::MANA))
        {
            mana.refill(per_second * dt);
        }
    }
}

#[cfg(test)]
mod tests;
