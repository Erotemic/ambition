//! What a match created, and what ends when the match does.
//!
//! Objects this ruleset spawns (`bomb`, `bolt`, `mine`, `portal`, `spring`)
//! each end by their own rule: a fuse, a trigger, a lifetime, the caster's next
//! cast. The end of a match is not one of those rules, so without this module a
//! mine laid in one match stays in the next.
//!
//! The owner is the match, not the move. `MatchScoped` is stamped once where an
//! object is created and swept once here. A despawn in each system, or a sweep
//! that knows each component type, puts this rule in many places, and the next
//! new technique would forget it.

use bevy::prelude::*;

use ambition_platformer2d::versus_match::{ActiveMatch, MatchScoped};

/// The localizer's window on a match-scoped object: the identity it carries.
pub fn match_scoped_probe(scoped: &MatchScoped) -> u64 {
    let (session, activated_on, ordinal) = scoped.0.parts();
    // Three optional facts in one word: the session, the activation tick, and
    // which match of the agreed session it is. An absent fact reads as zero,
    // which is what a composition with no session lifecycle stamps.
    //
    // This is a localizer window, not a checksum. It shows the local halves,
    // because a person debugging a stray object needs to see which activation
    // on this machine owns it. Do not build a peer comparison this way; see
    // `MatchInstance::peer_match_digest`.
    let session = session.map(|s| s.0 as u64).unwrap_or(0);
    session.rotate_left(32) ^ activated_on.unwrap_or(0) ^ ordinal.unwrap_or(0).rotate_left(16)
}

/// Stamp a freshly spawned object with the match that created it.
///
/// No active match means no stamp, and the sweep then ignores the object. That
/// is correct for a composition with no match lifecycle (a sandbox, a harness).
/// These systems only run while a move plays, so a live match always has one.
pub fn stamp(commands: &mut Commands, entity: Entity, active: Option<&ActiveMatch>) {
    if let Some(active) = active {
        commands
            .entity(entity)
            .insert(MatchScoped(active.instance()));
    }
}

/// Despawn anything a previous match created.
///
/// The rule is "not the active match", not "a match ended". A match abandoned
/// mid-frame never announces a verdict, and identity comparison does not care
/// how the last match finished.
///
/// No active match means nothing belongs. Between matches (select screen, the
/// shell) every match-scoped object is stale.
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

#[cfg(test)]
mod tests;
