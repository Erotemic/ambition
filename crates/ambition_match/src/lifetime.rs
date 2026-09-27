//! What a match created ends when the match does.
//!
//! Objects a ruleset spawns (a bomb, a mine, a placed portal) each end by their
//! own rule: a fuse, a trigger, a lifetime. The end of a match is not one of
//! those rules, so a mine laid in one match would stay in the next. A ruleset
//! says "this object belongs to the match" by stamping it with [`MatchScoped`]
//! where it is created ([`stamp_match_object`]); the engine sweeps every stamped
//! object once the match it belongs to is not the running one.

use bevy::prelude::*;

use crate::{ActiveMatch, MatchScoped};

/// Stamp a freshly spawned object with the match that created it.
///
/// No active match means no stamp, and the sweep then ignores the object. That
/// is correct for a composition with no match lifecycle (a sandbox, a harness).
pub fn stamp_match_object(commands: &mut Commands, entity: Entity, active: Option<&ActiveMatch>) {
    if let Some(active) = active {
        commands
            .entity(entity)
            .insert(MatchScoped(active.instance()));
    }
}

/// Despawn every stamped object whose match is not the running one.
///
/// The rule is "not the active match", not "a match ended". A match abandoned
/// mid-frame never announces a verdict, and identity comparison does not care
/// how the last match finished. With no active match, every stamped object is
/// stale.
///
/// Rollback-safe: `ActiveMatch` is rollback-registered and `MatchScoped` is
/// clone-snapshotted with the objects it marks, so a resimulated frame reaches
/// the same result for the same entity.
pub fn sweep_objects_from_ended_matches(
    mut commands: Commands,
    active: Option<Res<ActiveMatch>>,
    scoped: Query<(Entity, &MatchScoped)>,
) {
    for (entity, scope) in &scoped {
        if !scope.belongs_to(active.as_deref()) {
            commands.entity(entity).try_despawn();
        }
    }
}

impl MatchScoped {
    /// The localizer's window on a match-scoped object: the identity it carries.
    ///
    /// This is a localizer window, not a checksum. It shows the local halves,
    /// because a person debugging a stray object needs to see which activation
    /// on this machine owns it. Do not build a peer comparison this way; see
    /// `MatchInstance::peer_match_digest`.
    pub fn localizer_probe(&self) -> u64 {
        let (session, activated_on, ordinal) = self.0.parts();
        // Three optional facts in one word. An absent fact reads as zero, which
        // is what a composition with no session lifecycle stamps.
        let session = session.map(|s| s.0 as u64).unwrap_or(0);
        session.rotate_left(32) ^ activated_on.unwrap_or(0) ^ ordinal.unwrap_or(0).rotate_left(16)
    }
}

#[cfg(test)]
mod tests;
