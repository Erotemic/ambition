//! What a body's own CHARACTER says about driving itself — the one lookup,
//! asked by IDENTITY.
//!
//! [`AutonomousDefault::CharacterProfile`] is deliberately PAYLOADLESS. It
//! does not carry the policy; it says *the character carries it*, and the durable
//! answer is recovered from the body's [`WornCharacter`] through the prepared
//! cast. This module is that recovery, and it is the only place it happens.
//!
//! Provocation writes that same field (`provoke_actor_in_place`), so:
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
//! the field is not being retired — `ActorConfig::brain_profile` remains
//! the CURRENT policy, which is a real thing a body needs and what the lowering
//! reads. It stops being the answer to a question about the character's default,
//! which was never its meaning.

use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::{Brain, BrainProfile};

use crate::character_runtime::PreparedCharacterRegistry;
use crate::features::ecs::actor_clusters::ActorConfig;

/// The autonomous policy this character states, or `None` when it states none.
///
/// `None` is also the answer for a character the cast does not contain, and
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

// The LIVE road reads [`character_autonomous_profile`] directly from a system's query items, which
// is the only road there ever was.
//
// the strictness they carried is not lost: `apply_brain_selection` rejects a
// `CharacterProfile` default it cannot resolve, and
// `a_character_first_default_that_cannot_be_resolved_is_rejected` pins it.
