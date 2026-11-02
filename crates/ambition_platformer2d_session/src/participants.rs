//! Admitted session participants, independent of devices and controlled bodies.

use ambition_characters::control::PlayerSlot;
use ambition_platformer2d_core::snapshot::{put_u32, put_u8, Reader, SnapshotState};
use bevy::prelude::*;

/// Membership belongs to the session root and survives body replacement.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionParticipants {
    slots: Vec<PlayerSlot>,
}

impl SessionParticipants {
    pub fn contains(&self, slot: PlayerSlot) -> bool {
        self.slots
            .binary_search_by_key(&slot.0, |slot| slot.0)
            .is_ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = PlayerSlot> + '_ {
        self.slots.iter().copied()
    }

    /// Return false when the participant was already admitted.
    pub fn admit(&mut self, slot: PlayerSlot) -> bool {
        match self.slots.binary_search_by_key(&slot.0, |slot| slot.0) {
            Ok(_) => false,
            Err(index) => {
                self.slots.insert(index, slot);
                true
            }
        }
    }

    /// Remove membership as part of an explicit participant retirement.
    pub fn remove(&mut self, slot: PlayerSlot) -> bool {
        match self.slots.binary_search_by_key(&slot.0, |slot| slot.0) {
            Ok(index) => {
                self.slots.remove(index);
                true
            }
            Err(_) => false,
        }
    }
}

impl SnapshotState for SessionParticipants {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.slots.len() as u32);
        for slot in &self.slots {
            put_u8(out, slot.0);
        }
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        let count = reader.u32()? as usize;
        if count > usize::from(u8::MAX) + 1 {
            return None;
        }
        let mut slots = Vec::with_capacity(count);
        for _ in 0..count {
            let slot = PlayerSlot(reader.u8()?);
            if slots
                .last()
                .is_some_and(|last: &PlayerSlot| last.0 >= slot.0)
            {
                return None;
            }
            slots.push(slot);
        }
        Some(Self { slots })
    }
}

/// Admit a participant to the exact session that owns this construction.
pub fn admit_participant(
    commands: &mut Commands,
    scope: ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope,
    slot: PlayerSlot,
) {
    let Some(scope) = scope.id() else {
        return;
    };
    commands.queue(move |world: &mut World| {
        let root =
            ambition_platformer2d_shared_tangle::lifecycle::session_root_for_scope(world, scope)
                .expect("participant admission requires the owning session root");
        let mut entity = world.entity_mut(root);
        let mut members = entity.entry::<SessionParticipants>().or_default().into_mut();
        if !members.contains(slot) {
            members.admit(slot);
        }
    });
}

pub fn require_participants_on_session_root(app: &mut App) {
    ambition_platformer2d_shared_tangle::lifecycle::require_on_session_root::<SessionParticipants>(
        app,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::snapshot::{decode_state, encode_state};

    #[test]
    fn membership_has_one_canonical_order_and_round_trips() {
        let mut first = SessionParticipants::default();
        let mut second = SessionParticipants::default();
        for slot in [3, 0, 1, 3] {
            first.admit(PlayerSlot(slot));
        }
        for slot in [1, 3, 0] {
            second.admit(PlayerSlot(slot));
        }
        assert_eq!(first, second);
        let bytes = encode_state(&first);
        assert_eq!(bytes, encode_state(&second));
        assert_eq!(decode_state::<SessionParticipants>(&bytes), Some(first));
        assert!(second.remove(PlayerSlot(1)));
        assert!(!second.contains(PlayerSlot(1)));
        assert!(!second.remove(PlayerSlot(1)));
    }
}
