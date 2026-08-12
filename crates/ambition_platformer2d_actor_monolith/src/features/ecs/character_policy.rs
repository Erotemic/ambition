//! **What a body's own CHARACTER says about driving itself** — the one lookup,
//! asked by IDENTITY.
//!
//! ⭐⭐ [`AutonomousDefault::CharacterProfile`] is deliberately PAYLOADLESS. It
//! does not carry the policy; it says *the character carries it*, and the durable
//! answer is recovered from the body's [`WornCharacter`] through the prepared
//! cast. This module is that recovery, and it is the only place it happens.
//!
//! ⛔⛔ **WHAT THIS REPLACES IS A READ OF `ActorConfig::brain_profile`, AND THAT
//! FIELD IS MUTABLE RUNTIME STATE.** Three separate consumers — the
//! `RestoreDefault` command, the rollback reconstruction, and the
//! resume-from-temporary-control seam — each asked the body what its default
//! policy was by reading the policy the body is running RIGHT NOW. Provocation
//! writes that same field (`provoke_actor_in_place`), so:
//!
//! ```text
//!   character default = Wanderer
//!     spawn      → binding.default = CharacterProfile,  config.brain_profile = Wanderer
//!     provoke    → binding.source  = ProvokedProfile,   config.brain_profile = PirateBoarder
//!     release    → "restore the character's default"  reads  PirateBoarder
//!                → rebuilds PirateBoarder, and LABELS it CharacterProfile
//! ```
//!
//! The binding then claims *I am back on my character's normal policy* while the
//! live mind is still the provoked one, and nothing in the world disagrees with
//! it. A released villager keeps hunting you, permanently, and the state that
//! says so is self-consistent.
//!
//! ⚠ **the field is not being retired** — `ActorConfig::brain_profile` remains
//! the CURRENT policy, which is a real thing a body needs and what the lowering
//! reads. It stops being the answer to a question about the character's default,
//! which was never its meaning.

use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::{Brain, BrainProfile};
use bevy::prelude::{Entity, World};

use crate::character_runtime::PreparedCharacterRegistry;
use crate::features::ecs::actor_clusters::ActorConfig;

/// The autonomous policy this character states, or `None` when it states none.
///
/// ⚠ `None` is also the answer for a character the cast does not contain, and
/// the two are deliberately the same answer: both mean *the character is not the
/// authority here*, and a caller's fallback is identical either way.
pub(crate) fn character_autonomous_profile(
    registry: &PreparedCharacterRegistry,
    worn: &WornCharacter,
) -> Option<BrainProfile> {
    registry.get(worn.id())?.autonomous_profile
}

/// Lower a policy against THIS body.
///
/// §4.7: a profile states normalized effort and the body states the speed, so
/// the lowering cannot happen without a body — which is precisely why
/// `resolve_initial_brain` (which has no body) had to redirect here. The
/// `ActorConfig` clone exists so the lowering reads one config whose
/// `brain_profile` is the policy being lowered, rather than a lowering that
/// takes two half-agreeing arguments.
pub(crate) fn brain_from_profile(
    config: &ActorConfig,
    profile: BrainProfile,
    abilities: ambition_platformer2d_core::AbilitySet,
) -> Brain {
    let mut body = config.clone();
    body.brain_profile = profile;
    crate::features::ecs::enemy_default_brain(&body, abilities)
}

/// The policy a body should be driven by when it returns to its character's own
/// default — resolved by identity, falling back to the body's current policy.
///
/// ⚠ **the fallback is the FIXTURE road, not a second authority.** A world with
/// no prepared cast (every headless brain-command fixture, and any composition
/// that registers none) cannot answer the identity question at all, and the
/// body's current policy is the only thing there is. In production the identity
/// road answers, which is what
/// `a_released_character_returns_to_its_own_policy_not_the_provoked_one` pins —
/// and that test's poison is a body whose live policy has been trampled, so a
/// silent regression back to the fallback fails it.
pub(crate) fn default_policy_for(
    registry: Option<&PreparedCharacterRegistry>,
    worn: Option<&WornCharacter>,
    config: &ActorConfig,
) -> BrainProfile {
    registry
        .zip(worn)
        .and_then(|(registry, worn)| character_autonomous_profile(registry, worn))
        .unwrap_or(config.brain_profile)
}

/// [`default_policy_for`], asked of a world — the rollback/resume road, which
/// holds an `&World` rather than a system's query items.
pub(crate) fn default_policy_in(world: &World, entity: Entity) -> Option<BrainProfile> {
    let config = world.get::<ActorConfig>(entity)?;
    Some(default_policy_for(
        world.get_resource::<PreparedCharacterRegistry>(),
        world.get::<WornCharacter>(entity),
        config,
    ))
}

/// The complete identity → policy → `Brain` road for one entity in a world.
/// Returns `None` only for a body with no `ActorConfig` to lower against.
pub(crate) fn character_default_brain_in(world: &World, entity: Entity) -> Option<Brain> {
    let config = world.get::<ActorConfig>(entity)?;
    let profile = default_policy_in(world, entity)?;
    let abilities = world
        .get::<ambition_platformer2d_core::BodyAbilities>(entity)
        .map(|abilities| abilities.abilities)
        .unwrap_or_default();
    Some(brain_from_profile(config, profile, abilities))
}
