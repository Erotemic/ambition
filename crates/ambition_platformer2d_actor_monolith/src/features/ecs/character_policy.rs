//! Resolve a body's character-authored autonomous policy.
//!
//! Character defaults come from the prepared character selected by [`WornCharacter`].
//! `ActorConfig::brain_profile` is mutable live policy and must not be used to restore a
//! character default after temporary overrides such as provocation.

use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::{Brain, BrainProfile};

use ambition_characters::prepared::PreparedCharacterRegistry;
use ambition_combat::actor_tuning::ActorConfig;

/// The character-authored autonomous policy, if the prepared cast provides one.
///
/// Missing characters and characters without an autonomous profile both leave policy to the
/// caller's fallback.
pub(crate) fn character_autonomous_profile(
    registry: &PreparedCharacterRegistry,
    worn: &WornCharacter,
) -> Option<BrainProfile> {
    registry.get(worn.id())?.autonomous_profile
}

/// Lower a brain profile against this body's movement/ability configuration.
///
/// The temporary config clone keeps the policy being lowered and the body's tuning in one input.
pub(crate) fn brain_from_profile(
    config: &ActorConfig,
    profile: BrainProfile,
    abilities: ambition_platformer2d_core::AbilitySet,
) -> Brain {
    let mut body = config.clone();
    body.brain_profile = profile;
    crate::features::ecs::enemy_default_brain(&body, abilities)
}
