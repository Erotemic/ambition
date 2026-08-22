//! Central conversion between [`ParticipantId`] and [`PlayerSlot`].
//!
//! The two ids are numerically aligned today but represent different concepts, so new
//! code routes through these functions rather than introducing additional arithmetic
//! equality assumptions. Physical input sources and rollback channels remain separate
//! mappings owned by `ambition_input::LocalChannelPlan`.

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

    /// the round trip is the whole contract. It is trivially true while
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
