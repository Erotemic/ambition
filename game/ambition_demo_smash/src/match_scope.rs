//! What a match created, and what ends when the match does.
//!
//! ⛔⛔ THE DEFECT THIS CLOSES, in Jon's words while playing (2026-09-05): *"a
//! mine laid in a match still persists into the next match, that sounds like an
//! issue with architecture expression. Ending a match should be cleaning
//! everything up, don't hack in a solution to this, we need to find the right
//! solution."*
//!
//! Measured the same day: this ruleset spawns into the world at five sites —
//! `bomb`, `bolt`, `mine`, `portal`, `spring` — and no system despawned any of
//! them at a match boundary. Each object ended only by its own rule: a fuse, a
//! trigger, a lifetime, a caster's next cast. A match ending is not one of those
//! rules.
//!
//! ⭐ THE OWNER IS THE MATCH, NOT THE MOVE. `MatchScoped` is stamped once where
//! an object is created and swept once here. The alternative — a despawn in each
//! of the five systems, or one sweep that knows the five component types — puts
//! the end of a match's objects in N places that must each remember, which is
//! exactly how the mine came to outlive a match while the fighters did not. The
//! sixth technique somebody authors would be the sixth thing to forget.

use bevy::prelude::*;

use ambition_platformer2d::versus_match::{ActiveMatch, MatchScoped};

/// The localizer's window on a match-scoped object: the identity it carries.
pub fn match_scoped_probe(scoped: &MatchScoped) -> u64 {
    let (session, activated_on) = scoped.0.parts();
    // Two optional facts folded into one word: the session it belongs to and the
    // tick it was activated on. Absent reads as zero, which is what a
    // composition with no session lifecycle stamps.
    let session = session.map(|s| s.0 as u64).unwrap_or(0);
    session.rotate_left(32) ^ activated_on.unwrap_or(0)
}

/// Stamp a freshly spawned object with the match that created it.
///
/// ⚠ NO ACTIVE MATCH MEANS NO STAMP, and the object then behaves exactly as it
/// did before this existed — the sweep only claims what a match marked. That is
/// the right answer for a composition with no match lifecycle at all (a sandbox,
/// a harness), and these systems only run while a move is playing, so the live
/// case always has one.
pub fn stamp(commands: &mut Commands, entity: Entity, active: Option<&ActiveMatch>) {
    if let Some(active) = active {
        commands.entity(entity).insert(MatchScoped(active.instance()));
    }
}

/// Despawn anything the PREVIOUS match created.
///
/// ⛔ THE RULE IS "NOT THE ACTIVE MATCH", not "a match ended". Stated because the
/// two differ, and the difference is the case nobody has ruled on: a match
/// ABANDONED mid-frame never announces a verdict, so a teardown beat hung off the
/// verdict would leave its objects standing. Identity comparison does not care
/// how the last match finished.
///
/// ⚠ NO ACTIVE MATCH MEANS NOTHING BELONGS. Between matches — the select screen,
/// the shell — every match-scoped object is stale, and leaving a live mine on the
/// select screen is the same defect wearing a different hat.
///
/// ⭐ ROLLBACK-SAFE BY CONSTRUCTION, and this is why the identity had to be
/// registered: both sides of the comparison rewind. `ActiveMatch` is
/// rollback-registered and `MatchScoped` is clone-snapshotted beside the objects
/// it marks, so a resimulated frame reaches the same verdict about the same
/// entity. A despawn decided from un-rewound state would be a desync.
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
