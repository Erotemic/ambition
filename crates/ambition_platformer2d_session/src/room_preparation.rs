//! Immutable room preparation inputs selected on the simulation timeline.

mod custody;

use crate::checkpoint::AcceptedCheckpointRestore;
use crate::lifecycle_commit::{PendingIntent, PendingLifecycleCommit};
use ambition_platformer2d_items::pickup::minted_horizon::MintedItemBaseline;
use ambition_platformer2d_shared_tangle::lifecycle::{
    AuthoredOccurrences, SessionWorldMut, SessionWorldRef,
};
use bevy::prelude::*;
use std::sync::Arc;

/// Continuity selected for one room operation. Host readiness cannot change it.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomContinuityInputs {
    pub occurrences: Option<AuthoredOccurrences>,
    pub minted: Option<MintedItemBaseline>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomPreparationError {
    MissingRestoreInputs,
}

impl std::fmt::Display for RoomPreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRestoreInputs => {
                f.write_str("the admitted checkpoint restore has no pinned inputs")
            }
        }
    }
}

#[derive(Debug, PartialEq)]
struct CapturedInputs {
    admission: PendingIntent,
    continuity: Result<RoomContinuityInputs, RoomPreparationError>,
    checksum: u64,
}

/// Snapshot clones share the selected ledger and mint descriptions.
#[derive(Clone, Debug)]
pub struct RoomPreparation(Arc<CapturedInputs>);

impl PartialEq for RoomPreparation {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}

impl RoomPreparation {
    pub fn admission(&self) -> &PendingIntent {
        &self.0.admission
    }

    pub fn checksum(&self) -> u64 {
        self.0.checksum
    }

    pub fn continuity(&self) -> Result<&RoomContinuityInputs, RoomPreparationError> {
        self.0.continuity.as_ref().map_err(|error| *error)
    }
}

/// Session state. A withdrawal clears the selection; rollback restores it.
#[derive(Component, Default, Clone, Debug, PartialEq)]
pub struct CapturedRoomPreparation(Option<RoomPreparation>);

impl CapturedRoomPreparation {
    pub fn for_operation(&self, operation: &PendingIntent) -> Option<&RoomPreparation> {
        self.0
            .as_ref()
            .filter(|selected| &selected.0.admission == operation)
    }

    pub fn checksum(&self) -> u64 {
        self.0.as_ref().map_or(0, |selected| selected.0.checksum)
    }
}

/// Run after lifecycle producers and durable mirrors, outside the mode gate.
/// Capture on admission: a peer hold can prevent the next simulation frame.
pub fn capture_room_preparation_inputs(
    pending: Res<PendingLifecycleCommit>,
    mut captured: SessionWorldMut<CapturedRoomPreparation>,
    accepted: SessionWorldRef<AcceptedCheckpointRestore>,
    ledger: Option<Res<AuthoredOccurrences>>,
    save: Option<Res<ambition_persistence::save::AmbitionGameSave>>,
    custody: custody::CustodyEndingAtCommit,
) {
    let Some(operation) = pending.peek() else {
        if captured.0.is_some() {
            captured.0 = None;
        }
        return;
    };
    if captured.for_operation(operation).is_some() {
        return;
    }
    let continuity = select_continuity(
        operation,
        &accepted,
        ledger.as_deref(),
        save.as_deref(),
        &custody,
    );
    use ambition_platformer2d_core::snapshot::{checksum_bytes, put_bool, put_u8, put_u64};
    let mut bytes = Vec::new();
    put_u64(&mut bytes, operation.checksum());
    match &continuity {
        Ok(inputs) => {
            put_u8(&mut bytes, 0);
            put_bool(&mut bytes, inputs.occurrences.is_some());
            if let Some(ledger) = &inputs.occurrences {
                put_u64(&mut bytes, ledger.peer_stable_checksum());
            }
            put_bool(&mut bytes, inputs.minted.is_some());
            if let Some(minted) = &inputs.minted {
                put_u64(&mut bytes, minted.checksum());
            }
        }
        Err(RoomPreparationError::MissingRestoreInputs) => put_u8(&mut bytes, 1),
    }
    captured.0 = Some(RoomPreparation(Arc::new(CapturedInputs {
        admission: operation.clone(),
        continuity,
        checksum: checksum_bytes(&bytes),
    })));
}

fn select_continuity(
    operation: &PendingIntent,
    accepted: &AcceptedCheckpointRestore,
    live: Option<&AuthoredOccurrences>,
    save: Option<&ambition_persistence::save::AmbitionGameSave>,
    custody: &custody::CustodyEndingAtCommit,
) -> Result<RoomContinuityInputs, RoomPreparationError> {
    let restore = operation
        .checkpoint_operation
        .map(|key| {
            accepted
                .inputs_for_key(key)
                .ok_or(RoomPreparationError::MissingRestoreInputs)
        })
        .transpose()?;
    let restored = restore
        .and_then(|accepted| accepted.lifecycle.as_ref())
        .map(|lifecycle| lifecycle.occurrences.remembered());
    let occurrences = if let Some(restored) = restored {
        let survivors = custody.surviving(&operation.kind, restore.is_some_and(|accepted| accepted.fresh));
        Some(if survivors.is_empty() {
            restored.clone()
        } else {
            restored.with_custody_held(&survivors)
        })
    } else {
        let released = custody.released_by(&operation.kind, false);
        live.map(|ledger| {
            if released.is_empty() {
                ledger.clone()
            } else {
                ledger.with_custody_released(&released)
            }
        })
    };
    let minted = match restore {
        Some(accepted) => accepted.item.as_ref().map(|item| item.minted.clone()),
        None => save.map(|save| {
            ambition_platformer2d_items::pickup::minted_horizon::minted_baseline_from_save(
                save.data(),
            )
        }),
    };
    Ok(RoomContinuityInputs {
        occurrences,
        minted,
    })
}

#[cfg(test)]
mod tests;
