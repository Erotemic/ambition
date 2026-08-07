//! **The `ParticipantId` ↔ `PlayerSlot` correspondence, in ONE place.**
//!
//! ⚠ **these are two concepts that currently share a numbering, and the sharing
//! is not a fact about the world** (GPT 5.6 review through `c32e690`, finding 5).
//! `ParticipantId` names *a person in front of a controller*, which outlives a
//! session; `PlayerSlot` names *a seat the simulation reads a `ControlFrame`
//! from*, which is created and destroyed with seating needs. Today the primary
//! participant is seat 0 and the Nth participant is seat N, and every call site
//! that spells `PlayerSlot(participant.id.slot())` has quietly asserted that
//! they are the same thing.
//!
//! ⛔ **this module is NOT the rename.** The reviewer deferred that explicitly,
//! and it is a large cross-crate change; what it asks for instead is that new
//! code stop *adding* assumptions of numeric equality, so the future split has
//! one place to change rather than a grep. So: new code converts here. Existing
//! inline conversions are left where they are — sweeping them would be the
//! refactor that was deferred, wearing a smaller name.
//!
//! ## Why it lives in the monolith
//!
//! ⛔ **not a placement preference — the crate graph forces it.**
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

use ambition_characters::brain::PlayerSlot;
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

    /// ⭐ **the round trip is the whole contract.** It is trivially true while
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
