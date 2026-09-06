//! `SnapshotState` for this crate's own types — the rollback wire format.
//!
//! These impls live beside the types they encode. The orphan rule keeps the
//! snapshot implementation with either the trait or the type, so moving a type
//! forces its codec to move with it.
//!
//!  A field added to an encoded type is a WIRE FORMAT change. Encode and
//! decode must stay in the same order, and `snapshot_unit_enum!` codes are
//! authored per variant so inserting one never renumbers the rest.

use ambition_platformer2d_core::snapshot::{
    put_bool, put_f32, put_str, put_u32, put_u8, Reader, SnapshotResolve, SnapshotState,
};

impl SnapshotState for crate::EncounterLifecycle {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::EncounterPhase as P;
        put_u8(out, encounter_phase_tag(self.phase));
        if let P::Starting { remaining } = self.phase {
            put_f32(out, remaining);
        }
        put_f32(out, self.intro_seconds);
        put_f32(out, self.elapsed_active);
        // BTreeSet iterates sorted — canonical blob bytes by construction.
        put_u32(out, self.signals.len() as u32);
        for signal in &self.signals {
            put_str(out, signal);
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::EncounterPhase as P;
        let phase = match r.u8()? {
            0 => P::Inactive,
            1 => P::Starting {
                remaining: r.f32()?,
            },
            2 => P::Active,
            3 => P::Completed,
            4 => P::Failed,
            _ => return None,
        };
        let intro_seconds = r.f32()?;
        let elapsed_active = r.f32()?;
        let n = r.u32()? as usize;
        let mut signals = std::collections::BTreeSet::new();
        for _ in 0..n {
            signals.insert(r.str()?.to_string());
        }
        Some(crate::EncounterLifecycle {
            phase,
            intro_seconds,
            elapsed_active,
            signals,
        })
    }
}

impl SnapshotState for crate::EncounterParticipants {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.members.len() as u32);
        for member in &self.members {
            put_str(out, &member.id);
            put_u8(out, encounter_role_tag(member.role));
            put_bool(out, matches!(member.ownership, crate::Ownership::Spawned));
            put_bool(out, member.alive);
            // `member.entity` is deliberately NOT here — an entity index is an
            // allocator slot, not an identity. Re-resolved live from the id.
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let n = r.u32()? as usize;
        let mut members = Vec::with_capacity(n);
        for _ in 0..n {
            let id = r.str()?.to_string();
            let role = encounter_role_from_tag(r.u8()?)?;
            let ownership = if r.bool()? {
                crate::Ownership::Spawned
            } else {
                crate::Ownership::Adopted
            };
            let alive = r.bool()?;
            members.push(crate::EncounterParticipant {
                id,
                entity: None,
                role,
                ownership,
                alive,
            });
        }
        Some(crate::EncounterParticipants { members })
    }
}

impl SnapshotResolve for crate::EncounterWaves {
    fn encode_ref(&self, out: &mut Vec<u8>) {
        // The live run — the spec is authored content resolved from the
        // surviving component. Pending mobs are encoded verbatim (small POD;
        // their delays were already adjusted by the inter-wave rule, so they
        // are run state, not a pure spec subset).
        put_bool(out, self.run.wave_index.is_some());
        if let Some(wave_index) = self.run.wave_index {
            put_u32(out, wave_index as u32);
        }
        put_u32(out, self.run.pending.len() as u32);
        for mob in &self.run.pending {
            put_str(out, &mob.kind);
            //  VERBATIM means every field. `character` is authored content
            // and cannot diverge between peers, so omitting it would not have
            // desynced anything — but the comment above promises the whole mob,
            // and a field added to `EncounterMobSpec` that this loop silently
            // drops is how that promise stops being true one field at a time.
            put_str(out, mob.character.as_deref().unwrap_or(""));
            put_f32(out, mob.spawn[0]);
            put_f32(out, mob.spawn[1]);
            put_f32(out, mob.size[0]);
            put_f32(out, mob.size[1]);
            put_f32(out, mob.delay);
        }
        put_f32(out, self.run.wave_elapsed);
        put_bool(out, self.run.exhausted_signaled);
        put_u32(out, self.spawn_counter);
    }
}

fn encounter_phase_tag(phase: crate::EncounterPhase) -> u8 {
    use crate::EncounterPhase as P;
    match phase {
        P::Inactive => 0,
        P::Starting { .. } => 1,
        P::Active => 2,
        P::Completed => 3,
        P::Failed => 4,
    }
}

fn encounter_role_tag(role: crate::EncounterRole) -> u8 {
    use crate::EncounterRole as R;
    match role {
        R::PrimaryTarget => 0,
        R::Elite => 1,
        R::Minion => 2,
        R::Hazard => 3,
        R::Objective => 4,
        R::Protected => 5,
        R::Escort => 6,
        R::Narrative => 7,
        R::Rival => 8,
    }
}

fn encounter_role_from_tag(tag: u8) -> Option<crate::EncounterRole> {
    use crate::EncounterRole as R;
    Some(match tag {
        0 => R::PrimaryTarget,
        1 => R::Elite,
        2 => R::Minion,
        3 => R::Hazard,
        4 => R::Objective,
        5 => R::Protected,
        6 => R::Escort,
        7 => R::Narrative,
        8 => R::Rival,
        _ => return None,
    })
}
