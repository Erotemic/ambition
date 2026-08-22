//! **The `ParticipantId` ↔ `PlayerSlot` correspondence, in ONE place.**
//!
//! **this module is NOT the rename.** The reviewer deferred that explicitly,
//! and it is a large cross-crate change; what it asks for instead is that new
//! code stop *adding* assumptions of numeric equality, so the future split has
//! one place to change rather than a grep. So: new code converts here. Existing
//! inline conversions are left where they are — sweeping them would be the
//! refactor that was deferred, wearing a smaller name.
//!
//! ## Why it lives in the monolith
//!
//! **not a placement preference — the crate graph forces it.**
//! `ambition_input` (which defines `ParticipantId`) and `ambition_characters`
//! (which defines `PlayerSlot`) are SIBLINGS: neither depends on the other, and
//! both sit on `ambition_platformer2d_core` + `ambition_entity_catalog`. Putting
//! the correspondence in either would add a dependency edge between two crates
//! that are deliberately independent. The monolith is the lowest place that
//! already sees both.
//!
//! When the split happens, the honest home is a seating/topology authority that
//! owns the mapping as DATA rather than as arithmetic — at which point these two
//! functions become lookups and every caller keeps compiling.
//!
//! ## there is a THIRD identity in the same number, and it is the one that bit
//!
//! `LocalChannelPlan` separated the *physical source* — which pad
//! somebody picked up — from the dense rollback channel, after a sparse source
//! number reached GGRS as a handle and a fighter was deaf for a whole match.
//! That fix is real and must not be undone. But it spells the CHANNEL
//! `ParticipantId` too, so the number now carries three concepts rather than
//! two:
//!
//! ```text
//! LocalInputSource   what somebody picked up          — sparse, separated ✔
//! ParticipantId      the PERSON                       — outlives the session
//! SessionSeatId      a seat in this session's topology — does not exist yet
//! ControlChannelId   a deterministic input channel    — does not exist yet
//! PlayerSlot         what the simulation reads
//! ```
//!
//! **so the rule above extends to the channel**: new code must not add arithmetic equality
//! between a participant and a channel/handle either. Route through
//! `ambition_input::LocalChannelPlan`, whose whole job is being that map.

use ambition_characters::control::PlayerSlot;
use ambition_input::ParticipantId;

/// The seat this participant's input is published into.
pub fn player_slot_of(id: ParticipantId) -> PlayerSlot {
    PlayerSlot(id.slot())
}

/// The participant whose input arrives in this seat.
pub fn participant_of(slot: PlayerSlot) -> ParticipantId {
    ParticipantId(slot.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **the round trip is the whole contract.** It is trivially true while
    /// the two are one number, and it is the assertion that keeps being true
    /// after they stop being — which is the point of routing through here.
    #[test]
    fn a_participant_and_its_seat_round_trip() {
        for raw in 0u8..4 {
            let participant = ParticipantId(raw);
            assert_eq!(participant_of(player_slot_of(participant)), participant);
        }
        for raw in 0u8..4 {
            let slot = PlayerSlot(raw);
            assert_eq!(player_slot_of(participant_of(slot)), slot);
        }
    }

    /// The primary participant is seat 0, which several systems rely on by name
    /// rather than by arithmetic. Pinned here so the reliance is visible in one
    /// place if the mapping ever stops being the identity.
    #[test]
    fn the_primary_participant_reads_seat_zero() {
        assert_eq!(player_slot_of(ParticipantId::PRIMARY), PlayerSlot(0));
    }
}
