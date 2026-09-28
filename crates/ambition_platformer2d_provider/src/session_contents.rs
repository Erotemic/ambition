//! What an experience puts into its own session while the engine builds it.
//!
//! An experience can own facts that no platformer has in general: a session
//! root that is a spacetime, a home body that is also a light emitter, a cast
//! that is not a room placement. It states them in one function, declared with
//! [`PlatformerExperienceAuthoring::with_session_contents`]. The engine runs
//! that function inside the transaction that builds the session root and the
//! home body, before the candidate session is admitted, so the session is
//! published whole.
//!
//! ⛔ The alternative is a system that finds a root without these facts and
//! adds them. That is a construction repair: for at least one tick the session
//! is something else, and a rebuild depends on the system noticing again.
//!
//! [`PlatformerExperienceAuthoring::with_session_contents`]:
//! crate::PlatformerExperienceAuthoring::with_session_contents

use std::collections::BTreeMap;

use ambition_platformer2d_actor_spawn::{RecordedFate, SpawnActorRequest};
use ambition_platformer2d_shared_tangle::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};
use bevy::ecs::system::EntityCommands;
use bevy::prelude::{Bundle, Commands, Entity, Resource};

/// An experience's session contents. See the module documentation.
///
/// A plain function, not a closure: the contents are the same for every
/// session of the experience, and they may read nothing but the session being
/// built.
pub type SessionContentsFn = fn(&mut SessionContents);

/// The declared contents, by experience id.
#[derive(Resource, Default)]
pub struct SessionContentsCatalog {
    by_experience: BTreeMap<String, SessionContentsFn>,
}

impl SessionContentsCatalog {
    /// Declare `contents` for `experience_id`.
    ///
    /// # Panics
    /// When the experience already declared its contents. Two declarations
    /// are two answers to one question, and neither can be chosen.
    pub fn declare(&mut self, experience_id: &str, contents: SessionContentsFn) {
        let previous = self.by_experience.insert(experience_id.to_owned(), contents);
        assert!(
            previous.is_none(),
            "experience `{experience_id}` declared its session contents twice"
        );
    }

    /// The contents `experience_id` declared, if it declared any.
    pub fn get(&self, experience_id: &str) -> Option<SessionContentsFn> {
        self.by_experience.get(experience_id).copied()
    }
}

/// The session being built, as its experience sees it.
///
/// Everything spawned here is owned by the session and is hidden with the rest
/// of the candidate until the engine admits it.
pub struct SessionContents<'a, 'w, 's> {
    pub(crate) commands: &'a mut Commands<'w, 's>,
    pub(crate) scope: SessionSpawnScope,
    pub(crate) root: Entity,
    pub(crate) home_body: Option<Entity>,
    pub(crate) actors: StagedActorAuthorities<'a>,
}

/// What the staged-actor constructor reads, from the generation being built.
pub(crate) struct StagedActorAuthorities<'a> {
    pub(crate) character_catalog:
        &'a ambition_characters::actor::character_catalog::CharacterCatalog,
    pub(crate) sheets: &'a ambition_sprite_sheet::character::sheets::AuthoredSheets,
    pub(crate) characters: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
    pub(crate) bosses: &'a ambition_boss_encounter::BossCatalog,
}

impl<'w, 's> SessionContents<'_, 'w, 's> {
    /// The session root.
    pub fn root(&mut self) -> EntityCommands<'_> {
        self.commands.entity(self.root)
    }

    /// The session's home body, when the experience declared one.
    pub fn home_body(&mut self) -> Option<EntityCommands<'_>> {
        self.home_body.map(|body| self.commands.entity(body))
    }

    /// Spawn an entity the session owns.
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityCommands<'_> {
        self.commands.spawn_session_scoped(self.scope, bundle)
    }

    /// Build an actor through the staged-actor constructor, the same one a
    /// [`SpawnActorRequest`] reaches, and give it back to add to.
    ///
    /// `None` when the constructor refuses the request, or when the
    /// generation has no prepared cast to build it from. Both are reported.
    pub fn stage_actor(&mut self, request: &SpawnActorRequest) -> Option<EntityCommands<'_>> {
        let Some(characters) = self.actors.characters else {
            bevy::log::error!(
                "session contents staged actor `{}`, but the generation has no prepared \
                 cast to build it from",
                request.id
            );
            return None;
        };
        let entity = ambition_platformer2d_actor_spawn::spawn_staged_actor(
            self.commands,
            self.actors.character_catalog,
            self.actors.sheets,
            characters,
            self.actors.bosses,
            self.scope,
            request,
            RecordedFate::AsAuthored,
        )?;
        Some(self.commands.entity(entity))
    }
}
