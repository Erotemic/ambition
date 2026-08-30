//! What the CURRENT ATTEMPT at a room produced.
//!
//! Room construction owns the room. This owns the one lifetime question a room
//! scope cannot answer: *does this survive REPLAYING the room?* See
//! [`SpawnedThisAttempt`].
//!
//! The same-room reset itself is not here any more. It is
//! [`crate::rooms::reconstitute_the_active_room`], which rebuilds the room
//! through the canonical construction plan instead of mutating survivors back
//! toward a presumed spawn state through a hand-kept list.

/// Spawned by THIS attempt at the room, and cleared when the attempt is.
///
/// re-scoping the drop to the ROOM would be the wrong fix. A weapon you
/// drop in one room and find again when you walk back is intended behaviour, and
/// room scope deletes it on an ordinary transition. The two questions are
/// genuinely different — *does this survive leaving the room* and *does this
/// survive REPLAYING it* — and one scope cannot answer both. So the attempt is
/// named explicitly rather than inferred from a lifetime that means something
/// else.
///
/// it marks what the ATTEMPT produced, not everything spawned at runtime.
/// A summon a participant is still commanding or an item a body threw can be
/// somebody's durable live state; loot on the ground is the residue of a fight
/// that is about to be un-fought. In-flight projectiles are different: every
/// shot belongs to the combat timeline being reset, so the replay clears all
/// `LiveProjectile` occurrences explicitly rather than encoding their producer
/// category into this marker — see `rooms::reconstitution`'s `AttemptResidue`,
/// which is where the three session-scoped families a replay retires are named.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpawnedThisAttempt;
