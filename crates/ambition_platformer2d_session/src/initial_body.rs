//! Session startup body selection and its prepared resource bank.

use bevy::prelude::Component;

/// The catalog `character_id` the local player spawns as.
///
/// Read at session setup by both the simulation (moveset + name) and
/// presentation (sprite) halves. An EMPTY `character_id` means "no override —
/// wear the provider-relative default supplied by the session builder.
/// [`Default`] is exactly that. The engine names no specific character (C2):
/// which row is the default is CONTENT's choice, resolved lazily at spawn.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct StartingCharacter {
    /// A `character_catalog.ron` row id, or EMPTY for the content default.
    /// Ids without a renderable sheet still spawn a controllable player (the
    /// sprite falls back to the colored rectangle) — the sim side never depends
    /// on presentation.
    ///
    /// typed (P0.3): this is the RUNTIME seam of the same rule the prepared
    /// registry and the match participant already hold — an id is an id, and a
    /// display name passed here is a mistake the compiler can catch. empty
    /// still means *the content default*; `CharacterId` carries an empty string
    /// as happily as `String` did, so that meaning is unchanged (see
    /// `is_content_default` below, which is the ONE reader of the emptiness).
    pub character_id: ambition_entity_catalog::CharacterId,
}


impl StartingCharacter {
    pub fn new(character_id: impl Into<ambition_entity_catalog::CharacterId>) -> Self {
        Self {
            character_id: character_id.into(),
        }
    }

    /// True when the player spawns as the canonical protagonist (no override) —
    /// an empty id routes through the untouched `from_scratch` bundle.
    pub fn is_default(&self) -> bool {
        self.character_id.as_str().is_empty()
    }

    /// The concrete catalog id to wear: the explicit override, or the
    /// content-installed default when unset. Resolve at spawn time, never at
    /// component construction (the content default installs at the catalog choke point).
    pub fn effective_id<'a>(&'a self, default_character_id: &'a str) -> &'a str {
        if self.character_id.as_str().is_empty() {
            default_character_id
        } else {
            self.character_id.as_str()
        }
    }
}


/// Does this session build a home body at all?
///
/// A MATCH is not that shape — it realizes its own cast from a roster — so the engine handed
/// one an extra controllable actor it had no use for, and match seating grew a whole adoption
/// path to reinterpret that body as a fighter.
///
/// NOT an `Option<StartingCharacter>`, and not an empty id. An empty
/// [`StartingCharacter::character_id`] already means *"wear the provider-relative
/// default"* — absence is taken, and a second meaning on the same emptiness is
/// exactly how a silent default becomes a silent bug.
///
/// and it is a different question from `starting_character`, which is why
/// both exist. That field also names the experience's catalog DEFAULT — the id a
/// worn body falls back to, which a match experience legitimately still has even
/// though it builds no body of its own.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub enum InitialBodyPolicy {
    /// Spawn the session's home body wearing this character. Every exploration
    /// experience, and the default.
    SpawnCharacter(StartingCharacter),
    /// Build no session body. The experience realizes whatever actors it
    /// needs — a match builds its cast from a prepared roster — and there is no
    /// privileged avatar for anything to adopt, re-dress, or follow by accident.
    NoInitialBody,
}


/// What the session's home body HOLDS, as the experience declared it — the
/// resources it is built with and resets to (Ambition's Mana pool, say).
///
/// Empty is a complete answer: a home body that holds nothing. It sits beside
/// [`InitialBodyPolicy`] on the session root for the same reason that does —
/// an activation input the experience authors, read once when the body is
/// built — and is a separate component because a match, which builds no home
/// body, has nothing to say here.
///
/// It holds the PREPARED bank, so a bad declaration is refused where the
/// experience wrote it and setup only clones a finished answer.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct HomeBodyResources(Option<ambition_platformer2d_core::resources::ActorResources>);


impl HomeBodyResources {
    pub fn declared(
        declarations: &[ambition_resource_spec::ResourceDeclaration],
    ) -> Result<Self, ambition_platformer2d_core::resources::ResourceLayoutError> {
        ambition_platformer2d_core::resources::ActorResources::declared(declarations).map(Self)
    }

    /// The bank the home body is built holding, at its declared start.
    pub fn bank(&self) -> Option<&ambition_platformer2d_core::resources::ActorResources> {
        self.0.as_ref()
    }
}


impl Default for InitialBodyPolicy {
    fn default() -> Self {
        Self::SpawnCharacter(StartingCharacter::default())
    }
}


impl InitialBodyPolicy {
    /// The character the home body wears, or `None` when there is no home body.
    pub fn starting_character(&self) -> Option<&StartingCharacter> {
        match self {
            Self::SpawnCharacter(character) => Some(character),
            Self::NoInitialBody => None,
        }
    }

    pub fn spawns_a_body(&self) -> bool {
        matches!(self, Self::SpawnCharacter(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_character_selection_uses_the_provider_default() {
        let selection = StartingCharacter::default();
        assert!(selection.character_id.as_str().is_empty());
        assert!(selection.is_default());
        assert_eq!(selection.effective_id("authored_default"), "authored_default");
        let selected = StartingCharacter::new("selected");
        assert!(!selected.is_default());
        assert_eq!(selected.effective_id("authored_default"), "selected");
    }
}
