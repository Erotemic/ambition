//! The character a body **wears** — the canonical playable-persona identity.
//!
//! A player entity is a *control box*: it carries `DrivingParticipant(slot)`, the
//! movement clusters, and the player markers. WHICH catalog character that box
//! wears — its movement identity, its moveset, its name, and its sprite — is a
//! single simulation-owned relationship recorded by [`WornCharacter`].
//!
//! Before this component existed the worn id lived only in the app-local
//! `StartingCharacter` session component (read once at spawn) and a render-only
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
//! player-spawn machinery (`ambition_platformer2d_actor_monolith`) — both crates depend on this one.

use bevy::ecs::component::Component;

/// The catalog `character_id` a body currently wears.
///
/// **A STABLE STATEMENT OF WHICH TEMPLATE THIS BODY INSTANTIATES**, and nothing
/// more (Jon's redirect §2, 2026-08-11).
///
/// ⛔ **it used to mean two things at once**: *this body is an instance of
/// character X*, and *please apply character X's template to this body*. The
/// second meaning was carried by Bevy's change tick — a body was populated
/// because its worn id had just been WRITTEN — which made ordinary construction
/// depend on an observation edge. Two consequences, and the second is the one
/// that cost sessions: a body was incomplete for a tick by design, and change
/// ticks do not rewind, so a rollback could restore a body whose population had
/// been driven by an edge that would never fire again.
///
/// Asking for the template to be applied is now an explicit
/// [`RecharacterizeBody`] request. Carrying this component, or changing it,
/// populates nothing on its own.
///
/// ⚠ presentation still reads it with `Changed<WornCharacter>`, and that is
/// fine: re-binding a sprite when the identity changes is an observation, not a
/// construction.
///
/// Requires [`IdentityKit`]: the identity derivation writes what this worn id
/// alone produced into it, and the equipment reconcile re-derives the live kit
/// from it. Requiring it means a body that can change identity can never be
/// missing the baseline — a missing one would silently skip both systems rather
/// than fail loudly.
///
/// [`IdentityKit`]: crate::brain::action_set::IdentityKit
/// ⭐ **the inner value is a typed [`CharacterId`], not a `String`** — which
/// character template this body instantiates, in the same type every placement
/// and spawn plan names it by. A body's runtime identity is its `SimId`; two
/// bodies wearing one `CharacterId` are two instances of one template, which is
/// the ordinary case rather than a collision.
///
/// [`CharacterId`]: ambition_entity_catalog::CharacterId
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

/// **An explicit request to (re)apply this body's character template.**
///
/// ⭐ **the second half of what [`WornCharacter`] used to mean**, separated so
/// that ordinary construction does not go through it (Jon's redirect §2). A
/// normal character actor is built COMPLETE and never carries this; what needs
/// it is a genuine re-template:
///
/// ```text
/// a transformation that changes which character a body IS   (Mary-O's powerups)
/// character-select adoption onto a live body
/// a cast hot reload
/// any deliberate runtime re-wear
/// ```
///
/// ⚠ **it is CONSUMED**, so a request is one application rather than a state a
/// body can get stuck in. A writer that wants it again asks again.
///
/// ⛔ do not reintroduce `Changed<WornCharacter> → populate the body`. Renaming
/// the identity component without splitting the request off would have moved the
/// defect, not removed it.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecharacterizeBody;
