//! Mana regeneration policy for driven bodies.

use bevy::prelude::*;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

/// **How fast a driven body's Mana refills, as the COMPOSITION's statement.**
///
/// ⛔⛔ ABSENT MEANS NO REFILL. This once defaulted to the platformer's own
/// 14/s when a composition stated nothing, and a Smash stage had to insert a
/// zero to keep that invented rate out of its Limit, which then shared the one
/// meter. Mana is now a named resource a body holds only when its experience
/// declared the pool, and the rate belongs to the same declaration: the
/// Ambition provider states `ambition_abilities::mana::REGEN_PER_SEC` beside
/// the pool it gives the home body.
///
/// ⭐⭐ **AND IT IS THE ONLY ONE OF ITS KIND — swept 2026-09-05, so nobody has to
/// wonder.** Two questions were asked of the whole tree:
///
/// 1. *Which rollback-registered component does more than one CRATE write?*
///    **66 of 256** — which is normal for an ECS and far too noisy to guard.
///    `BodyKinematics` alone has sixteen writers and every one is legitimate.
///    ⇒ Multiple writers was never the hazard. **Two POLICIES for one value
///    was**, and no static shape distinguishes those.
/// 2. *Is there a sibling — a continuous per-tick platformer rule writing a
///    meter a ruleset also owns?* **No.** There is no health regen, and
///    `regen_player_mana` is the only `* dt` writer in this module.
///
/// ⇒ So this resource closes the case rather than opening a family, and the
/// negative result is recorded because "is this the only one" is exactly what
/// the next reader will ask.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq)]
pub struct PlayerManaRegen(pub f32);

/// Mana slowly regenerates, at the composition's stated rate, so it is a
/// genuine spendable resource. Clamped at the pool's max and scaled by sim dt,
/// so bullet-time / pause slow it with the world.
///
/// Refills every DRIVEN body's mana — the bodies actually spending it on charge
/// attacks and held abilities — so possessing an actor regenerates that actor's
/// meter, not the vacated home avatar's, and a couch's second seat regenerates
/// at all.
///
/// ⛔⛔ IT REFILLED ONE `ControlledSubject`, which is one entity by construction.
/// Seat one spent mana on a gauntlet it could never get back — a slow leak
/// rather than a dead verb, which is why it outlived the verbs' own fix.
pub fn regen_player_mana(
    time: Res<ambition_time::WorldTime>,
    driven: ambition_held_items::DrivenBodies,
    mut banks: Query<&mut ambition_platformer2d_core::resources::ActorResources>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    policy: Option<Res<PlayerManaRegen>>,
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
    // ⚠ THE FALLBACK IS THE STARTUP FRAME and nothing else.
    let mut subjects = driven.entities();
    if subjects.is_empty() {
        subjects.extend(primary.single().ok());
    }
    for subject in subjects {
        if let Some(mana) = banks
            .get_mut(subject)
            .ok()
            .and_then(|bank| bank.into_inner().level_of_mut(&ambition_abilities::mana::MANA))
        {
            mana.refill(per_second * dt);
        }
    }
}

#[cfg(test)]
mod tests;
