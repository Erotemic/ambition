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
/// default — resolved by identity, or `None`.
///
/// ⛔⛔ **`None` IS AN INVARIANT VIOLATION, NOT A CASE TO COVER FOR.** This
/// returned `config.brain_profile` when the identity road could not answer, and
/// that fallback was scaffolding that recreated the exact bug it was written to
/// fix: `ActorConfig::brain_profile` is the policy the body is running NOW, and
/// provocation writes it. A binding that says `default = CharacterProfile` is a
/// claim that the CHARACTER can answer; if `WornCharacter` or the registry
/// wiring ever goes missing, "ask the character and otherwise trust whatever
/// mind is currently installed" is a released body that keeps hunting you.
///
/// ⇒ callers REJECT — the same answer every other unresolvable brain command
/// gets, with a warning naming the body. A composition that seats a
/// character-first body without a cast to resolve it is broken, and finding out
/// loudly is the point.
pub(crate) fn default_policy_for(
    registry: Option<&PreparedCharacterRegistry>,
    worn: Option<&WornCharacter>,
) -> Option<BrainProfile> {
    registry
        .zip(worn)
        .and_then(|(registry, worn)| character_autonomous_profile(registry, worn))
}

/// [`default_policy_for`], asked of a world — the rollback/resume road, which
/// holds an `&World` rather than a system's query items.
pub(crate) fn default_policy_in(world: &World, entity: Entity) -> Option<BrainProfile> {
    default_policy_for(
        world.get_resource::<PreparedCharacterRegistry>(),
        world.get::<WornCharacter>(entity),
    )
}

/// The complete identity → policy → `Brain` road for one entity in a world.
///
/// `None` for a body with no `ActorConfig` to lower against, and for one whose
/// character cannot be resolved — see [`default_policy_for`] for why the second
/// is deliberately not covered for.
pub(crate) fn character_default_brain_in(world: &World, entity: Entity) -> Option<Brain> {
    let config = world.get::<ActorConfig>(entity)?;
    let profile = default_policy_in(world, entity)?;
    let abilities = world
        .get::<ambition_platformer2d_core::BodyAbilities>(entity)
        .map(|abilities| abilities.abilities)
        .unwrap_or_default();
    Some(brain_from_profile(config, profile, abilities))
}
