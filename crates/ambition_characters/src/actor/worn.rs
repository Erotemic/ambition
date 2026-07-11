//! The character a body **wears** — the canonical playable-persona identity.
//!
//! A player entity is a *control box*: it carries `Brain::Player(slot)`, the
//! movement clusters, and the player markers. WHICH catalog character that box
//! wears — its movement identity, its moveset, its name, and its sprite — is a
//! single simulation-owned relationship recorded by [`WornCharacter`].
//!
//! Before this component existed the worn id lived only in the app-local
//! `StartingCharacter` *resource* (read once at spawn) and a render-only
//! `PlayerSpriteCharacter` marker, so gameplay config and presentation each
//! rediscovered the selection from a different authority. [`WornCharacter`] is
//! the ONE identity both derive from:
//!
//! ```text
//! selected/worn character identity  (WornCharacter, on the canonical player)
//!     → character gameplay configuration  (moveset + movement identity)
//!     → generic selected-character presentation  (sprite + animation)
//! ```
//!
//! It is a plain component so ANY body could wear a character, and so
//! presentation (`ambition_render`) can read it without depending on the
//! player-spawn machinery (`ambition_actors`) — both crates depend on this one.

use bevy::ecs::component::Component;

/// The catalog `character_id` a body currently wears.
///
/// Simulation-owned and set at spawn from the selected character; changing it
/// (a re-wear / transformation) is the supported runtime path, and downstream
/// gameplay + presentation systems observe the change through Bevy's
/// `Changed<WornCharacter>` filter.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct WornCharacter(pub String);

impl WornCharacter {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The worn catalog id.
    pub fn id(&self) -> &str {
        &self.0
    }
}
