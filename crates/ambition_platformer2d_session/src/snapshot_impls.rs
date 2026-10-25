//! The lifecycle snapshot encoding. Field order and tags define the wire format.

use ambition_platformer2d_core::snapshot::{put_bool, put_i32, put_str, put_u32, put_u8, put_vec2, Reader, SnapshotState};

impl SnapshotState for crate::lifecycle_commit::PendingLifecycleCommit {
    fn encode(&self, out: &mut Vec<u8>) {
        match &self.pending {
            None => put_bool(out, false),
            Some(intent) => {
                put_bool(out, true);
                put_i32(out, intent.frame);
                put_lifecycle_intent(out, &intent.kind);
            }
        }
        match &self.withdrawn {
            None => put_bool(out, false),
            Some(withdrawn) => {
                put_bool(out, true);
                put_i32(out, withdrawn.recorded_on);
                put_i32(out, withdrawn.withdrawn_on);
                put_lifecycle_intent(out, &withdrawn.kind);
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::lifecycle_commit::{PendingIntent, PendingLifecycleCommit, WithdrawnIntent};
        let pending = if r.bool()? {
            let frame = r.i32()?;
            Some(PendingIntent { frame, kind: read_lifecycle_intent(r)? })
        } else {
            None
        };
        let withdrawn = if r.bool()? {
            let recorded_on = r.i32()?;
            let withdrawn_on = r.i32()?;
            Some(WithdrawnIntent { recorded_on, withdrawn_on, kind: read_lifecycle_intent(r)? })
        } else {
            None
        };
        Some(PendingLifecycleCommit { pending, withdrawn })
    }
}

fn put_lifecycle_intent(out: &mut Vec<u8>, intent: &crate::lifecycle_commit::LifecycleIntent) {
    use crate::lifecycle_commit::LifecycleIntent;
    // ⭐ THE TAG STAYS 3. Four sibling variants were deleted, and
    // renumbering would be a second wire change for no gain: a
    // reader that meets tag 0, 1, 2 or 4 is reading a snapshot from
    // a build that had them, and `decode` refusing it is the honest
    // answer.
    match intent {
        LifecycleIntent::Transition(
            crate::lifecycle_commit::RoomTransitionIntent {
                subject,
                target_room,
                arrival,
                edge_exit,
                zone_sfx,
                participant,
            },
        ) => {
            put_u8(out, 3);
            put_str(out, subject.sim_id.as_str());
            put_bool(out, subject.room.is_some());
            put_u32(out, subject.room.map_or(0, |room| room.ordinal()));
            put_str(out, target_room);
            put_vec2(out, *arrival);
            put_bool(out, *edge_exit);
            put_bool(out, zone_sfx.is_some());
            put_str(out, zone_sfx.as_deref().unwrap_or(""));
            put_bool(out, participant.is_some());
            put_u8(out, participant.map_or(0, |slot| slot.0));
        }
        // ⭐ TAG 5, NOT 4. Tags 0-4 all belonged to the deleted
        // variants, so 4 would make a v139 snapshot's `FullReset`
        // decode as a room reconstitution instead of being refused.
        // A new variant takes the first tag no build ever wrote.
        LifecycleIntent::ReconstituteRoom(
            crate::lifecycle_commit::RoomReconstitutionIntent { target_room },
        ) => {
            put_u8(out, 5);
            put_str(out, target_room);
        }
    }
}

fn read_lifecycle_intent(r: &mut Reader<'_>) -> Option<crate::lifecycle_commit::LifecycleIntent> {
    use crate::lifecycle_commit::LifecycleIntent;
    let kind = match r.u8()? {
        3 => LifecycleIntent::Transition(
            crate::lifecycle_commit::RoomTransitionIntent {
                subject: {
                    let sim_id = ambition_platformer2d_shared_tangle::sim_id::SimId::from_snapshot(
                        r.str()?.to_string(),
                    );
                    let in_a_room = r.bool()?;
                    let ordinal = r.u32()?;
                    ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(
                        sim_id,
                        in_a_room.then(|| {
                            ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::from_ordinal(
                                ordinal,
                            )
                        }),
                    )
                },
                target_room: r.str()?.to_string(),
                arrival: r.vec2()?,
                edge_exit: r.bool()?,
                zone_sfx: {
                    let present = r.bool()?;
                    let cue = r.str()?.to_string();
                    present.then_some(cue)
                },
                participant: {
                    let present = r.bool()?;
                    let slot = r.u8()?;
                    present.then_some(ambition_characters::control::PlayerSlot(slot))
                },
            },
        ),
        5 => LifecycleIntent::ReconstituteRoom(
            crate::lifecycle_commit::RoomReconstitutionIntent {
                target_room: r.str()?.to_string(),
            },
        ),
        // Tags 0, 1, 2 and 4 were the deleted reset variants. Refusing them
        // is the point: a snapshot that carries one came from a build with a
        // different lifecycle model, and decoding it into anything would be
        // inventing an operation.
        _ => return None,
    };
    Some(kind)
}
