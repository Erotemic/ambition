//! The first-run witness of a sync test.
//!
//! A GGRS sync test rewinds on each step, runs the last `check_distance`
//! frames again, and compares the checksum of each saved frame against the
//! first checksum it saved for that frame. It does not save the state that
//! the FIRST run of a frame leaves: the next step loads an older frame before
//! it saves the newest one. So the first saved version of each frame comes
//! from the second run of its advance, and each compare is between two
//! resimulations.
//!
//! ⛔ AN EFFECT THAT ONLY THE FIRST RUN OF A FRAME HAS IS IN NO SAVED STATE.
//! Measured 2026-10-05 (`the_sync_test_sees_a_first_run_only_effect`, check
//! distance 4): a system that moves a body on the first run of one frame, and
//! on no resimulation, left the session healthy, and the body was not moved
//! 40 frames later. The same effect on the second run only was reported by
//! GGRS at that frame. A `Local` or a static that the first run of a
//! simulation system sets is this shape, and it is a peer divergence: on a
//! peer whose inputs for the frame were confirmed, the first run is the only
//! run.
//!
//! This module closes that. After the advances of a host tick it takes the
//! checksum of the state the first run left, and it compares that with the
//! first save of the same frame. A difference is recorded on the authority as
//! a sync-test mismatch is, so `session_health` reports it.
//!
//! ⚠ The witness is a checksum and nothing else. Its pass runs the checksum
//! half of `SaveWorld` and no snapshot system, so it moves no snapshot ring,
//! no frame counter and no save census.

use bevy::prelude::*;
use bevy_ggrs::{Checksum, RollbackFrameCount, SaveWorld, Session};

use ambition_platformer2d_runtime::rollback::{ActiveRollbackAuthority, RollbackTimelineGeneration};

use crate::session::AmbitionGgrsSession;
use crate::{ComponentCensus, RollbackChecksumProbes, RollbackRestoreAudit};

/// The checksum of the state that the first run of the newest frame left.
///
/// Host-local, and not rollback state: it is a record ABOUT one run of a
/// frame, so a rewind must not take it back.
#[derive(Resource, Default)]
pub struct FirstRunWitness {
    taken: Option<Taken>,
    /// True while the witness pass runs `SaveWorld`. The snapshot half of that
    /// schedule does not run then.
    witnessing: bool,
    /// How many first saves were compared with a witness. A test reads it as
    /// its premise: a witness that compared nothing found nothing.
    compared: u64,
}

struct Taken {
    generation: RollbackTimelineGeneration,
    frame: i32,
    checksum: u128,
    /// Each registered type as peers compare it, kept only while
    /// [`RollbackRestoreAudit`] is enabled: it names the types of a mismatch.
    census: Option<std::collections::BTreeMap<&'static str, ComponentCensus>>,
}

impl FirstRunWitness {
    /// How many first saves this host compared with the first run's checksum.
    pub fn compared(&self) -> u64 {
        self.compared
    }
}

/// Run condition of the snapshot half of `SaveWorld`.
pub(crate) fn not_witnessing(witness: Option<Res<FirstRunWitness>>) -> bool {
    !witness.is_some_and(|witness| witness.witnessing)
}

/// After the advances of this host tick: take the checksum of the state the
/// first run left.
///
/// Only for a sync test that rewinds. A session with a check distance of zero
/// saves nothing and compares nothing, and it is the shipped local session, so
/// it pays nothing here. A peer session saves the first run itself when no
/// rollback is owed.
pub(crate) fn take_the_first_run_witness(world: &mut World) {
    let Some(frame) = world.get_resource::<RollbackFrameCount>().map(|frame| frame.0) else {
        return;
    };
    // ⛔ ONLY A FRAME THAT THE NEXT STEP REWINDS BEFORE IT SAVES. In the first
    // `check_distance` frames of a session GGRS does not rewind, so the next
    // save of this frame is this same run, as the host left it. A compare
    // there asks a different question: what was written outside the timeline
    // between two advances. Measured 2026-10-05 without this condition:
    // `a_mount_dying_under_a_possession_survives_rewinds` went red at frame 1
    // of a session that was installed again.
    let rewinds = matches!(
        world.get_resource::<AmbitionGgrsSession>(),
        Some(Session::SyncTest(session))
            if session.check_distance() > 0 && frame > session.check_distance() as i32
    );
    let Some(authority) = world.get_resource::<ActiveRollbackAuthority>() else {
        return;
    };
    if !rewinds || !authority.status().is_healthy() {
        return;
    }
    let generation = authority.generation();
    // No advance ran in this host tick: the state is the one already taken.
    if world
        .resource::<FirstRunWitness>()
        .taken
        .as_ref()
        .is_some_and(|taken| taken.generation == generation && taken.frame == frame)
    {
        return;
    }
    world.resource_mut::<FirstRunWitness>().witnessing = true;
    world.run_schedule(SaveWorld);
    world.resource_mut::<FirstRunWitness>().witnessing = false;
    let Some(checksum) = world.get_resource::<Checksum>().map(|checksum| checksum.0) else {
        return;
    };
    let census = peer_census_if_audited(world);
    world.resource_mut::<FirstRunWitness>().taken = Some(Taken {
        generation,
        frame,
        checksum,
        census,
    });
}

fn peer_census_if_audited(
    world: &mut World,
) -> Option<std::collections::BTreeMap<&'static str, ComponentCensus>> {
    if !world
        .get_resource::<RollbackRestoreAudit>()
        .is_some_and(|audit| audit.enabled)
    {
        return None;
    }
    let probes = world.get_resource::<RollbackChecksumProbes>().cloned()?;
    Some(probes.census_all_as_peers_compare(world))
}

/// In `SaveWorld`, after the checksum of the frame is folded: the first save
/// of the witnessed frame is compared with the first run.
///
/// The save is the state after the first resimulation of the frame: a witness
/// is taken only for a frame that the next step rewinds. Later saves of the
/// frame are GGRS's to compare.
pub(crate) fn compare_the_first_save_with_the_witness(world: &mut World) {
    let Some(frame) = world.get_resource::<RollbackFrameCount>().map(|frame| frame.0) else {
        return;
    };
    let Some(generation) = world
        .get_resource::<ActiveRollbackAuthority>()
        .map(ActiveRollbackAuthority::generation)
    else {
        return;
    };
    let mut witness = world.resource_mut::<FirstRunWitness>();
    let Some(taken) = witness
        .taken
        .take_if(|taken| taken.generation != generation || taken.frame <= frame)
    else {
        return;
    };
    // A witness of another timeline, or of a frame this save is past, has no
    // save to meet: a session that was installed again starts at frame zero.
    if taken.generation != generation || taken.frame != frame {
        return;
    }
    witness.compared += 1;
    let Some(saved) = world.get_resource::<Checksum>().map(|checksum| checksum.0) else {
        return;
    };
    if saved == taken.checksum {
        return;
    }
    let types = match taken.census {
        Some(first) => {
            let again = peer_census_if_audited(world).unwrap_or_default();
            let differ: Vec<&str> = first
                .iter()
                .filter(|(name, census)| again.get(*name) != Some(census))
                .map(|(name, _)| *name)
                .collect();
            format!("; the types that differ: {differ:?}")
        }
        None => "; enable `RollbackRestoreAudit` to name the types".to_string(),
    };
    let reason = format!(
        "the first run of frame {frame} and its first resimulation differ \
         (checksum {:#x}, then {saved:#x}): an effect that only the first run \
         has{types}",
        taken.checksum,
    );
    bevy::log::warn!(target: "ambition_platformer2d::rollback", "{reason}");
    crate::session::record_timeline_mismatch_in(world, frame, reason);
}
