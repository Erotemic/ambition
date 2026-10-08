//! Ledge trumping enforces one hanging body per edge.
//!
//! The most recent grab keeps the ledge; older holders are knocked off and lose
//! ledge-grab intangibility. This implements the contested-edge rule without the
//! outward helpless pop used by some platform fighters.
//!
//! ⭐ IT LIVES BESIDE THE RULE IT ENFORCES. `CombatRules::ledge_trump_pop`
//! was already this crate's; the resolver that reads it sat in the actor monolith
//! for no reason anyone had written down. Moved 2026-08-28 (D33) — 152 lines that
//! reached the monolith through no `crate::` path, no `super::` path and no glob,
//! and needed no dependency this crate did not already have.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// How close two edge keys must be to be the same edge, in world px. The
/// key (`LedgeContact::edge_key`) is fixed to the corner whatever the body's
/// size, so this is a float-equality tolerance rather than a reach.
const SAME_EDGE_EPSILON: f32 = 1.0;

/// The set [`resolve_ledge_trumps`] runs in. The engine installs it, so a
/// ruleset chooses who keeps an edge in data (`ledge_occupancy`,
/// `ledge_trump_pop`) and schedules nothing.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LedgeTrumpsResolved;

/// Keep the newest holder of each edge and knock older holders off.
///
/// Sorting by `(elapsed, SimId)` gives same-tick grabs a deterministic winner
/// independent of query/archetype order.
pub fn resolve_ledge_trumps(
    mut bodies: Query<(
        Entity,
        &SimId,
        &mut ambition_platformer2d_core::movement::MotionModel,
        &mut ae::BodyLedgeState,
        // ⛔⛔ OPTIONAL, AND THAT IS NOT TIDINESS. Adding this as a REQUIRED
        // column silently narrowed the population the rule sees: a hanging body
        // without kinematics stopped being trumped at all, and two existing
        // tests went red with "two bodies shared one edge". Trumping is about
        // the EDGE; the pop is a bonus a body with a velocity can receive.
        Option<&mut ae::BodyKinematics>,
        // THE BODY'S OWN FRAME, for the pop's axis. `Option` for the same reason
        // the kinematics above are: a body without a resolved frame still loses
        // the edge, it simply falls under screen-down like it always did.
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        // OPTIONAL for the reason the kinematics are: a body with no combat
        // state still loses the edge; it only has no lock to receive.
        Option<&mut ambition_characters::actor::BodyCombat>,
    )>,
    // The match's own answer to *what does losing the edge cost*, per live
    // room. A world that declares no combat rules still trumps — it simply
    // drops the loser, which is what every trump did before the knob.
    rules: crate::rules::CombatTuningOf,
) {
    // (room, edge, elapsed, id, entity) for every body currently hanging.
    // Two bodies share an edge only in one live room (OW1): the same corner in
    // two live rooms is two edges. The edge is the ledge's corner and face,
    // not the hanging body's centre: that centre depends on the body's size,
    // so two fighters of different sizes on one corner had two "edges".
    type Holder = (
        Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        (ae::Vec2, f32),
        f32,
        SimId,
        Entity,
    );
    let mut holders: Vec<Holder> = Vec::new();
    for (entity, id, model, _, _, _, _) in bodies.iter() {
        let ae::MotionModel::AxisSwept(axis) = &*model else {
            continue;
        };
        // HANGING, not climbing: a body already pulling itself up has left the
        // edge as far as this rule is concerned, and trumping it would cancel a
        // getup that is no longer contesting anything.
        let Some(hang) = axis.state.ledge_grab.as_ref().filter(|l| !l.climbing) else {
            continue;
        };
        let edge = (hang.contact.edge_key(), hang.contact.wall_normal_x.signum());
        holders.push((rules.room_of(entity), edge, hang.elapsed, id.clone(), entity));
    }
    if holders.len() < 2 {
        return;
    }
    // ⭐⭐ THE POLICY IS THIS ONE COMPARISON. Whoever sorts FIRST keeps the
    // edge, so trumping and hogging are the same authority read in opposite
    // directions: Ultimate keeps the NEWEST grab (smallest `elapsed`) and
    // knocks the old holder off; Melee keeps the body that got there FIRST and
    // the newcomer is the one who loses. Both are coherent games.
    //
    // ⛔ NO SECOND RULE ABOUT WHO MAY GRAB. The loser is knocked off by the
    // same path with the same pop either way — a hog that refused the grab
    // outright would be a second ledge authority, which is what the parity row
    // rules out.
    //
    // The `SimId` tiebreak stays ascending in both, so a same-tick contest has
    // a deterministic winner independent of query or archetype order.
    //
    // Each room's own rules choose its order, so the holders sort by room
    // first.
    let hog = |room| {
        matches!(
            rules.in_room(room).map(|r| r.ledge_occupancy),
            Some(crate::rules::LedgeOccupancy::Hog)
        )
    };
    holders.sort_by(|a, b| {
        let by_time = a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal);
        let by_time = if hog(a.0) { by_time.reverse() } else { by_time };
        a.0.cmp(&b.0)
            .then(by_time)
            .then_with(|| a.3.cmp(&b.3))
    });

    let mut kept: Vec<(
        Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        (ae::Vec2, f32),
    )> = Vec::new();
    let mut trumped: Vec<(Entity, f32, f32)> = Vec::new();
    for (room, (key, face), _, _, entity) in &holders {
        if kept.iter().any(|(held_room, (held_key, held_face))| {
            held_room == room
                && held_face == face
                && held_key.distance_squared(*key) <= SAME_EDGE_EPSILON * SAME_EDGE_EPSILON
        }) {
            let room_rules = rules.in_room(*room);
            let pop = room_rules.map_or(0.0, |rules| rules.ledge_trump_pop);
            let lockout = room_rules.map_or(0.0, |rules| rules.ledge_trump_lockout);
            trumped.push((*entity, pop, lockout));
        } else {
            kept.push((*room, (*key, *face)));
        }
    }

    for (entity, pop, lockout) in trumped {
        let Ok((_, _, mut model, mut ledge, mut kin, frame, combat)) = bodies.get_mut(entity) else {
            continue;
        };
        let body_frame = frame
            .map(|frame| frame.basis())
            .unwrap_or_else(|| ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR));
        // ⭐ THE OUTWARD DIRECTION IS THE HANG'S, read BEFORE the knock-off
        // clears it. `wall_normal_x` is the way the wall pushes, so it already
        // points away from the stage — and it is right for a body hanging
        // FACING OUT, which a reading off `kin.facing` would get backwards.
        //
        // ⛔⛔ AND IT IS A BODY-LOCAL SIDE SIGN DESPITE ITS NAME. `_x` is
        // historical: `probe_ledge_grab_in_frame` says in as many words that it
        // is *"the side-face normal expressed in the controlled body's local
        // side axis"*, and the producer computes it as
        // `world_normal.dot(frame.side).signum()`. A 2026-08-25 review reported
        // the pop below as a world-axis bug and the finding was REFUSED on the
        // reading that its input was world-X too — from the NAME. The name was
        // the stale thing; the consumer was the bug.
        let outward = if let ae::MotionModel::AxisSwept(axis) = &*model {
            axis.state
                .ledge_grab
                .as_ref()
                .map(|hang| hang.contact.wall_normal_x)
        } else {
            None
        };
        if ae::movement::knock_off_ledge(&mut model, &mut ledge) {
            // ⭐⭐ THE POP, and it is a declared rule rather than the law:
            // trumping exists in every platform fighter, being thrown off it
            // does not. `0.0` drops the loser where it hung.
            if pop > 0.0 {
                if let (Some(outward), Some(kin)) =
                    (outward.filter(|n| n.abs() > 0.0), kin.as_mut())
                {
                    // The body's OWN side axis. Under sideways gravity a
                    // fighter's outward IS world Y, and `vel.x` popped it along
                    // the axis it falls on instead.
                    let side = body_frame.side;
                    let along = kin.vel.dot(side);
                    kin.vel += (outward.signum() * pop - along) * side;
                }
            }
            // the window goes with the edge. It was bought with airtime this
            // body no longer has, and a falling fighter that kept it would be
            // the safest thing on the stage.
            if let ae::MotionModel::AxisSwept(axis) = &mut *model {
                axis.state.ledge_invuln_timer = 0.0;
            }
            // The declared lockout: how long the loser cannot act.
            if let Some(mut combat) = combat {
                combat.ledge_trump_lock_timer = combat.ledge_trump_lock_timer.max(lockout);
            }
        }
    }
}

#[cfg(test)]
mod tests;
