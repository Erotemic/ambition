//! `[census] rooms` — WHICH ROOM IS LIVE, AND WHAT THE CROSSING IS DOING.
//!
//! ⛔⛤ **TWENTY-FIVE CENSUS SURFACES AND NOT ONE OF THEM ANSWERED "WHERE AM
//! I".** Measured against the list on
//! `engine/inspection-diagnostics-and-workbench.md`: `assets camera churn
//! conditions config draws ecs frame ggrs_driver membership owners owners_in
//! phases phases_cpu phases_trust phases_warning populations portal
//! render_pass render_pass_summary render_targets schedules sim_phases views
//! visual_quality`. Every one of them describes the machine; none describes
//! the WORLD. A room transition that stalls, commits into the wrong room, or
//! opens a transaction nobody closes was diagnosable only by a debugger or by
//! reading four files in three crates.
//!
//! ⭐ **AND IT IS THE FIRST FACT THE OPEN-WORLD WORK NEEDS.**
//! `engine/open-world-runtime-and-residency.md`'s OW1 is *"two instances of one
//! room; audit selection/identity/query/teardown paths"*, and today the answer
//! to *"which room is live"* is `RoomSet::active: usize` — an INDEX into a list
//! of definitions, which is exactly the conflation OW1 has to unpick. A row
//! that prints the index beside the authored id is the cheapest way to watch
//! that distinction stop holding.
//!
//! ⚠ **DERIVED AND READ-ONLY, WHICH IS THE SANCTIONED HALF OF THE DISTINCTION
//! THAT PAGE DRAWS.** Nothing here owns anything: `RoomSet` owns the selection,
//! `RoomTransitionLoadState` owns the transaction, and this composes their
//! answers into one line. It must never become a place a consumer reads
//! instead of asking the owner.

use bevy::prelude::{App, Last, Plugin, Query, Res, With};

use ambition_dev_tools::runtime_census::RuntimeCensus;
use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;
use ambition_platformer2d_world::rooms::RoomSet;

use crate::room_transition::{ActiveRoomTransitionLoad, RoomTransitionLoadState};

/// Emit `[census] rooms` on the shared census clock.
///
/// ⛔ **A WORLD WITH NO SESSION PRINTS A ROW SAYING SO, rather than printing
/// nothing.** `SessionWorldRef` is a `Single`, and a system built on one
/// silently does not run when the session is absent — which for a diagnostic
/// is indistinguishable from the census being off, from the plugin not being
/// composed, and from a stall. "Where am I" has `nowhere` as a legitimate
/// answer and it is the answer during a teardown, which is when somebody is
/// most likely to be looking.
pub fn report_room_census(
    census: Res<RuntimeCensus>,
    sessions: Query<(&RoomSet, &SessionRoot)>,
    crossing: Option<Res<RoomTransitionLoadState>>,
) {
    let Some(at) = census.due() else {
        return;
    };
    eprintln!(
        "{}",
        room_census_row(
            at,
            sessions.iter(),
            crossing.as_deref().and_then(|state| state.active.as_ref()),
        )
    );
}

/// The row itself, as a value.
///
/// ⛔⛤ **SPLIT FROM THE PRINTING SO SOMETHING CAN ASSERT ON IT.** A census that
/// only exists as an `eprintln!` on a clock is a diagnostic nobody can test,
/// and the failure mode is the one that matters: a row that names the wrong
/// room, or that goes on naming the old one after a crossing commits, looks
/// exactly like a working row to a reader who has nothing to compare it
/// against. The composed witness calls this with the live world's own
/// `RoomSet` and checks the id against the session's.
pub fn room_census_row<'a>(
    at: f64,
    sessions: impl ExactSizeIterator<Item = (&'a RoomSet, &'a SessionRoot)>,
    crossing: Option<&ActiveRoomTransitionLoad>,
) -> String {
    let mut row = format!("[census] rooms t={at:.3} sessions={}", sessions.len());
    // ⚠ EVERY session, not the first. A composition with two session roots is
    // the state OW1 is heading for, and a row that silently reported one of
    // them would go on looking correct through the whole of that work.
    for (room_set, root) in sessions {
        // ⛔⛤ **THE SCOPE IS INSIDE `SessionRoot`, AND ASKING FOR IT AS A
        // SIBLING COMPONENT PRINTED `?` FOR EVERY REAL ROOT — REVIEWED
        // 2026-09-20.** `SessionRoot(pub SessionScopeId)` IS the scope; a root
        // does not normally also carry a standalone `SessionScopeId`, so
        // `Option<&SessionScopeId>` matched nothing and the field was a
        // constant wearing a variable's clothes. The test repeated the same
        // wrong query and never asserted the value, which is how a
        // diagnostic's own arm can agree with its defect.
        let scope = root.0 .0.to_string();
        let id_at = |index: usize| {
            room_set
                .rooms
                .get(index)
                .map_or("<out-of-range>", |room| room.id.as_str())
        };
        row.push_str(&format!(
            " [scope={scope} rooms={} active={}[{}] start={}[{}]",
            room_set.rooms.len(),
            id_at(room_set.active),
            room_set.active,
            id_at(room_set.start),
            room_set.start,
        ));
        // ⚠ **`active_metadata()` INDEXES DIRECTLY AND WOULD PANIC ON THE
        // STATE THIS ROW EXISTS TO REPORT.** `id_at` above is careful with an
        // out-of-range `active`; one line later the metadata read went through
        // `&self.rooms[self.active]`, so a world broken in exactly the way
        // that makes somebody run this would kill the instrument instead of
        // printing `active=<out-of-range>[73]`.
        if let Some(biome) = room_set
            .rooms
            .get(room_set.active)
            .and_then(|room| room.metadata.biome.as_deref())
        {
            row.push_str(&format!(" biome={biome}"));
        }
        row.push(']');
    }
    // ⭐ THE TRANSACTION IS A RESOURCE AND THE SELECTION IS A COMPONENT, which
    // is why they are printed as separate groups rather than joined: one
    // crossing is in flight for the world, not for a session root, and
    // pretending otherwise would be inventing a correspondence the types do
    // not have.
    match crossing {
        None => row.push_str(" crossing=none"),
        Some(active) => {
            let target = active
                .intent
                .target_room()
                .to_string();
            row.push_str(&format!(
                " crossing={}[{}]->{}[{}] seq={} epoch={} scope={} cover={} commit_not_before={} plan={}",
                active.source_room_id,
                active.source_room,
                target,
                active.target_room,
                active.sequence,
                active.content_epoch,
                active
                    .session_scope
                    .map_or_else(|| "?".to_string(), |id| id.0.to_string()),
                active.cover_required,
                active.commit_not_before_tick,
                // ⚠ `staged` is not `committed`: a plan that exists has been
                // built and not yet published, and the interesting stall is a
                // crossing that sits here for many samples.
                if active.construction_plan.is_some() {
                    "staged"
                } else {
                    "pending"
                },
            ));
        }
    }
    row
}

/// Registers [`report_room_census`], and only when the census is on.
///
/// ⛔ Registered only when asked, for the reason
/// `ambition_render::runtime_census` gives: `due_at` is only set while the
/// census is enabled, so this could never have reported when off — it simply
/// has no business in a shipped frame's schedule.
pub struct RoomCensusPlugin;

impl Plugin for RoomCensusPlugin {
    fn build(&self, app: &mut App) {
        if RuntimeCensus::from_env().enabled() {
            app.add_systems(Last, report_room_census);
        }
    }
}
