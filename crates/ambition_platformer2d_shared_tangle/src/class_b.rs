//! Per-frame ledger for discontinuous Class-B body remaps.
//!
//! Portal transit, room transitions, death/reset, and scripted teleports record
//! remaps here. The ledger observes rather than arbitrates: every Class-B writer
//! must reset the movement-frame sample so a second remap cannot apply, while
//! tests use the log to detect violations. The log is cleared each sim frame.

use bevy::prelude::*;

/// Which Class-B authority remapped a body.
///
/// Declaration order is priority order — `DeathOrReset` is strongest. §3.2
/// ranks the first three (`death/reset > room-transition > portal-transit`);
/// `ScriptedTeleport` (blink, dive, mark-recall — the traversal abilities that
/// jump a body rather than accelerate it) is not ranked by the doctrine, so it
/// sorts weakest by extension: dying mid-blink is a death, not a blink.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClassBRemap {
    /// Death, hazard reset, or a whole-sandbox reset. Ends the frame's story.
    DeathOrReset,
    /// A loading-zone / door transition relocated the body into a new room.
    RoomTransition,
    /// The body's centroid crossed a portal's entry plane and it emerged at the
    /// exit aperture.
    PortalTransit,
    /// A traversal ability jumped the body: blink, dive, mark-recall, an
    /// authored teleport, or the Trap surfacing through a floor. ⛔ the last one
    /// is not a teleport in the fiction and is one here on purpose: what this
    /// enum sorts is WHO MOVED THE BODY, and a move that picks a destination and
    /// writes it is the same authority however it is dressed.
    ScriptedTeleport,
}

impl ClassBRemap {
    /// Doctrine priority, lower wins. Equals the declaration index.
    pub fn priority(self) -> u8 {
        self as u8
    }

    pub fn wins_over(self, other: Self) -> bool {
        self.priority() < other.priority()
    }

    /// Stable snake-case name for trace payloads and violation reports.
    pub fn label(self) -> &'static str {
        match self {
            Self::DeathOrReset => "death_or_reset",
            Self::RoomTransition => "room_transition",
            Self::PortalTransit => "portal_transit",
            Self::ScriptedTeleport => "scripted_teleport",
        }
    }
}

/// One recorded remap, in application order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassBRemapEntry {
    /// The body that was moved.
    pub body: Entity,
    /// Which authority moved it.
    pub kind: ClassBRemap,
}

/// A body that took two or more Class-B remaps in one frame: invariant 5's violation shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassBContention {
    /// The doubly-remapped body.
    pub body: Entity,
    /// The remap that applied first this frame.
    pub first: ClassBRemap,
    /// The remap that applied after it.
    pub second: ClassBRemap,
}

/// The frame-scoped ledger of Class-B remaps. Cleared at the head of every sim
/// frame by [`clear_class_b_remap_log`], appended to by the Class-B writers,
/// read by the CC3 oracle and by trace tooling.
#[derive(Resource, Default, Debug, Clone)]
pub struct ClassBRemapLog {
    entries: Vec<ClassBRemapEntry>,
}

impl ClassBRemapLog {
    /// A Class-B writer just remapped `body`. Call it at the moment the
    /// position is written, not when the intent is formed — an ability that
    /// *tries* to blink into a wall and clamps still moved the body.
    pub fn record(&mut self, body: Entity, kind: ClassBRemap) {
        self.entries.push(ClassBRemapEntry { body, kind });
    }

    /// Drop the frame's record. Called once, before `CoreSimulation`.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Everything recorded this frame, in application order.
    pub fn entries(&self) -> &[ClassBRemapEntry] {
        &self.entries
    }

    /// Was `body` remapped at all this frame? The CC3 oracle asks this to
    /// exempt its teleport probe on transit frames — a body that legally warped
    /// did not "clip", and §6.1 invariant 1 says so.
    pub fn was_remapped(&self, body: Entity) -> bool {
        self.entries.iter().any(|e| e.body == body)
    }

    /// The kinds applied to `body` this frame, in application order.
    pub fn kinds_for(&self, body: Entity) -> impl Iterator<Item = ClassBRemap> + '_ {
        self.entries
            .iter()
            .filter(move |e| e.body == body)
            .map(|e| e.kind)
    }

    /// Every invariant-5 violation this frame: one [`ClassBContention`] per
    /// doubly-remapped body, reporting that body's FIRST offending pair.
    ///
    /// Deterministic by construction — it scans the append-ordered `Vec` and
    /// never iterates a hash container (ADR 0023).
    pub fn contentions(&self) -> Vec<ClassBContention> {
        let mut out: Vec<ClassBContention> = Vec::new();
        for (i, entry) in self.entries.iter().enumerate() {
            // The first entry for this body, or nothing.
            let Some(first) = self.entries[..i].iter().find(|e| e.body == entry.body) else {
                continue;
            };
            if out.iter().any(|c| c.body == entry.body) {
                continue; // already reported this body's first offending pair
            }
            out.push(ClassBContention {
                body: entry.body,
                first: first.kind,
                second: entry.kind,
            });
        }
        out
    }
}

/// Clear the ledger at the head of the sim frame. Registered by the engine's
/// `Platformer2dSimulationFoundationPlugin` `.before(Platformer2dSimulationPhaseMonolith::CoreSimulation)`, which is upstream
/// of every Class-B writer including `ResetProcessing` (a tail set, but still
/// inside the same frame).
pub fn clear_class_b_remap_log(mut log: ResMut<ClassBRemapLog>) {
    log.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(n: u32) -> Entity {
        Entity::from_raw_u32(n).expect("a valid test entity id")
    }

    #[test]
    fn doctrine_priority_is_declaration_order() {
        assert!(ClassBRemap::DeathOrReset.wins_over(ClassBRemap::RoomTransition));
        assert!(ClassBRemap::RoomTransition.wins_over(ClassBRemap::PortalTransit));
        assert!(ClassBRemap::PortalTransit.wins_over(ClassBRemap::ScriptedTeleport));
        // and it is a strict order: nothing beats itself
        assert!(!ClassBRemap::DeathOrReset.wins_over(ClassBRemap::DeathOrReset));
    }

    #[test]
    fn one_remap_per_body_is_no_contention() {
        let mut log = ClassBRemapLog::default();
        log.record(body(1), ClassBRemap::PortalTransit);
        log.record(body(2), ClassBRemap::RoomTransition);
        log.record(body(3), ClassBRemap::DeathOrReset);
        assert!(log.contentions().is_empty());
        assert!(log.was_remapped(body(2)));
        assert!(!log.was_remapped(body(4)));
    }

    /// The shape invariant 5 exists to catch: a body door-warped *and* portal
    /// transited on one frame, so the door's arrival was computed against a
    /// position the portal had already invalidated.
    #[test]
    fn two_remaps_on_one_body_is_a_contention_reported_in_application_order() {
        let mut log = ClassBRemapLog::default();
        log.record(body(7), ClassBRemap::PortalTransit);
        log.record(body(9), ClassBRemap::RoomTransition);
        log.record(body(7), ClassBRemap::RoomTransition);

        let found = log.contentions();
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0],
            ClassBContention {
                body: body(7),
                first: ClassBRemap::PortalTransit,
                second: ClassBRemap::RoomTransition,
            }
        );
        // The doctrine says the room transition should have won the tie...
        assert!(found[0].second.wins_over(found[0].first));
    }

    /// Three remaps on one body report ONE contention (the first offending
    /// pair), so a cascade does not spam the trace.
    #[test]
    fn a_body_is_reported_once_no_matter_how_many_times_it_is_remapped() {
        let mut log = ClassBRemapLog::default();
        log.record(body(1), ClassBRemap::ScriptedTeleport);
        log.record(body(1), ClassBRemap::PortalTransit);
        log.record(body(1), ClassBRemap::DeathOrReset);
        let found = log.contentions();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].first, ClassBRemap::ScriptedTeleport);
        assert_eq!(found[0].second, ClassBRemap::PortalTransit);
        assert_eq!(log.kinds_for(body(1)).count(), 3);
    }

    #[test]
    fn clearing_is_what_makes_it_frame_scoped() {
        let mut log = ClassBRemapLog::default();
        log.record(body(1), ClassBRemap::PortalTransit);
        log.clear();
        log.record(body(1), ClassBRemap::RoomTransition);
        assert!(
            log.contentions().is_empty(),
            "a remap on the PREVIOUS frame must never contend with this one"
        );
    }
}
