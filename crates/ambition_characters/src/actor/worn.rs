//! The character a body wears — the canonical playable-persona identity.
//!
//! A player entity is a *control box*: it carries `DrivingParticipant(slot)`, the
//! movement clusters, and the player markers. WHICH catalog character that box
//! wears — its movement identity, its moveset, its name, and its sprite — is a
//! single simulation-owned relationship recorded by [`WornCharacter`].
//!
//! [`WornCharacter`] is the ONE identity both derive from:
//!
//! ```text
//! selected/worn character identity  (WornCharacter, on the canonical player)
//!     → character gameplay configuration  (moveset + movement identity)
//!     → generic selected-character presentation  (sprite + animation)
//! ```
//!
//! It is a plain component so ANY body could wear a character, and so
//! presentation (`ambition_render`) can read it without depending on the
//! player-spawn machinery (`ambition_platformer2d_actor_monolith`) — both crates depend on this one.

use bevy::ecs::component::Component;

/// Catalog character template instantiated by this body.
///
/// This component is stable identity only; applying the template requires an
/// explicit [`RecharacterizeBody`] request. Presentation may observe identity
/// changes independently. [`IdentityKit`](crate::brain::action_set::IdentityKit)
/// is required so equipment reconciliation always has the character-derived
/// baseline. Runtime instance identity remains separate in `SimId`.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
#[require(crate::brain::action_set::IdentityKit)]
pub struct WornCharacter(pub ambition_entity_catalog::CharacterId);

impl WornCharacter {
    pub fn new(id: impl Into<ambition_entity_catalog::CharacterId>) -> Self {
        Self(id.into())
    }

    /// The worn character id.
    pub fn id(&self) -> &str {
        self.0.as_str()
    }

    /// The worn identity itself, for callers that thread it onward as an id
    /// rather than as text.
    pub fn character(&self) -> &ambition_entity_catalog::CharacterId {
        &self.0
    }
}

/// One-shot request to reapply a body's character template after a deliberate
/// runtime identity change. The request is consumed after application.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecharacterizeBody;
