//! The cast a system reads: the running session's frozen cast, or the
//! published one when no session runs.
//!
//! A content reload publishes a new cast (`PreparedCharacterRegistry`) into the
//! App BEFORE the session that runs it is activated. A live system that reads
//! the published cast in that window spends values its session was not
//! prepared against, and a rollback resimulation of an earlier frame reads
//! them too (measured 2026-10-01: a cast published mid-timeline desynced the
//! sync test until the live readers read the frozen cast).
//!
//! ⭐ ONE OWNER FOR THE FROZEN CAST. [`ActiveSessionCast`] is the activated
//! generation's cast, inserted when the generation is activated and removed
//! with the session. It is in this crate, and not in the actor monolith's
//! `SessionMechanics`, so that every crate that reads a cast can name it.
//!
//! ⭐ NO SESSION MEANS THE PUBLISHED CAST. A shell menu or a select screen runs
//! with no session; what it shows is what the NEXT session will be prepared
//! from, which is the published cast. So [`SessionCast::get`] answers with the
//! frozen cast while a session runs and with the published cast otherwise. A
//! session whose generation froze NO cast reads no cast: the resource is
//! present and empty, which is not the same fact as an absent resource.

use bevy::ecs::system::SystemParam;
use bevy::prelude::{DetectChanges, Res, Resource, World};

use crate::prepared::PreparedCharacterRegistry;

/// The cast the running session was prepared against.
///
/// Present exactly while a session runs. `None` inside is a session whose
/// generation froze no cast: a composition that registers no characters.
#[derive(Resource, Clone, Debug, Default)]
pub struct ActiveSessionCast(pub Option<PreparedCharacterRegistry>);

impl ActiveSessionCast {
    /// The frozen cast, or `None` when the generation froze none.
    pub fn cast(&self) -> Option<&PreparedCharacterRegistry> {
        self.0.as_ref()
    }
}

/// The cast a system reads: see the module doc.
#[derive(SystemParam)]
pub struct SessionCast<'w> {
    active: Option<Res<'w, ActiveSessionCast>>,
    published: Option<Res<'w, PreparedCharacterRegistry>>,
}

impl SessionCast<'_> {
    /// The running session's cast, or the published cast when no session runs.
    pub fn get(&self) -> Option<&PreparedCharacterRegistry> {
        match &self.active {
            Some(active) => active.cast(),
            None => self.published.as_deref(),
        }
    }

    /// Did the cast [`Self::get`] answers with change since this system last
    /// ran? A session's cast changes when the session is activated; the
    /// published cast when a reload publishes one.
    pub fn is_changed(&self) -> bool {
        match (&self.active, &self.published) {
            (Some(active), _) => active.is_changed(),
            (None, Some(published)) => published.is_changed(),
            (None, None) => false,
        }
    }

    /// The PUBLISHED cast, whatever runs. For the questions that are about the
    /// publication itself ("has the published cast moved on since this match
    /// was prepared?"), not about which cast to play.
    pub fn published(&self) -> Option<&PreparedCharacterRegistry> {
        self.published.as_deref()
    }
}

/// [`SessionCast::get`] for exclusive code that holds a `&World`.
pub fn session_cast(world: &World) -> Option<&PreparedCharacterRegistry> {
    match world.get_resource::<ActiveSessionCast>() {
        Some(active) => active.cast(),
        None => world.get_resource::<PreparedCharacterRegistry>(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cast(max_health: i32) -> PreparedCharacterRegistry {
        let mut app = bevy::app::App::new();
        let mut definition =
            crate::actor::definition::CharacterDefinition::new("alpha", "Alpha", "fixture");
        definition.vitals.max_health = Some(max_health);
        crate::prepared::stage_authored_character(&mut app, definition, &Default::default())
            .expect("stages");
        crate::prepared::close_preparation_barrier(app.world_mut());
        app.world()
            .get_resource::<PreparedCharacterRegistry>()
            .cloned()
            .expect("the barrier publishes a cast")
    }

    fn health(registry: Option<&PreparedCharacterRegistry>) -> Option<i32> {
        registry
            .and_then(|cast| cast.get("alpha"))
            .and_then(|definition| definition.vitals.max_health)
    }

    /// A running session reads its frozen cast, a session that froze none reads
    /// none, and with no session the published cast is the answer. The system
    /// road and the `&World` road give the same answer.
    #[test]
    fn a_reader_is_given_the_running_sessions_cast() {
        #[derive(Resource, Default)]
        struct Seen(Option<i32>);
        fn read(cast: SessionCast, mut seen: bevy::prelude::ResMut<Seen>) {
            seen.0 = health(cast.get());
        }
        let run = |active: Option<Option<i32>>| {
            let mut app = bevy::app::App::new();
            app.init_resource::<Seen>().insert_resource(cast(3));
            if let Some(frozen) = active {
                app.insert_resource(ActiveSessionCast(frozen.map(cast)));
            }
            app.add_systems(bevy::app::Update, read);
            app.update();
            let by_world = health(session_cast(app.world()));
            (app.world().resource::<Seen>().0, by_world)
        };
        assert_eq!(run(Some(Some(9))), (Some(9), Some(9)), "the frozen cast outranks the published one");
        assert_eq!(run(Some(None)), (None, None), "a session that froze no cast reads none");
        assert_eq!(run(None), (Some(3), Some(3)), "with no session, the published cast");
    }
}
