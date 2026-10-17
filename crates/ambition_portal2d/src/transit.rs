//! Portal-specific transit systems: drive every body and in-flight item
//! through a placed portal pair via the shared
//! [`super::placement::transit_step`] aperture machine, plus the carve / input /
//! ability-suppression guards that make a crossing feel right.

use bevy::prelude::*;

use crate::pieces as pp;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::class_b::{ClassBRemap, ClassBRemapLog};
use ambition_platformer2d_shared_tangle::orientation::ActorRoll;

use super::color::PortalChannel;
use super::placement::{transit_step_with_tuning, SweptSample, TransitStep, TRANSIT_BEGIN_MARGIN};
use super::tuning::PortalTuning;
use super::rooms::{PortalHostDepthsByRoom, PortalRoom, PortalsByRoom};
use super::types::{find_portal, PlacedPortal, PortalTransitCooldown};

/// A body's position just moved to a portal exit (the centroid crossed).
/// Carries the entity so a consumer can filter to one body (e.g. the locally
/// focused body).
#[derive(Message, Clone, Copy, Debug)]
pub struct BodyTeleported {
    /// The body whose position snapped to a portal exit this frame.
    pub body: Entity,
}

/// A body on the back of a pair: it cuts the plane of one of the pair's
/// openings from behind (its centre behind the plane), as a body that comes
/// up through a one-way platform does. A crossing is begun only from a
/// portal's face, so while it has this the pair does not begin its
/// crossing, open its hole for it, snag it, or take it: it must be wholly
/// past the plane first, and then it is on one side or the other like any
/// body (Jon 2026-10-16).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortalBackside {
    /// The channel of the opening whose plane it cuts from behind.
    pub channel: PortalChannel,
}

impl PortalBackside {
    /// Is `channel` one end of the pair this body is on the back of?
    pub fn of_pair(self, channel: PortalChannel) -> bool {
        channel == self.channel || channel == self.channel.partner()
    }
}

/// The portals of `all` a body on the back of `backside` may use: all but
/// that pair.
pub(crate) fn usable_portals(all: &[PlacedPortal], backside: Option<PortalBackside>) -> std::borrow::Cow<'_, [PlacedPortal]> {
    match backside {
        Some(back) => std::borrow::Cow::Owned(all.iter().filter(|p| !back.of_pair(p.channel)).cloned().collect()),
        None => std::borrow::Cow::Borrowed(all),
    }
}

/// Keep `entity`'s [`PortalBackside`] true after its step: a body not
/// crossing that cuts a paired opening's plane with its centre behind it is
/// on its back; it stays so while it cuts that plane, and is not once it is
/// past it.
pub(crate) fn update_backside(
    commands: &mut Commands,
    entity: Entity,
    backside: Option<PortalBackside>,
    crossing: bool,
    all: &[PlacedPortal],
    body: ae::Aabb,
) {
    let now = if crossing {
        None
    } else if let Some(back) = backside.filter(|back| {
        find_portal(all, back.channel).is_some_and(|p| crate::snag::cuts_opening(body, &p.aperture()))
    }) {
        Some(back)
    } else {
        all.iter()
            .filter(|p| find_portal(all, p.channel.partner()).is_some())
            .find(|p| {
                pp::front_distance(body.center(), &p.frame()) < 0.0 && crate::snag::cuts_opening(body, &p.aperture())
            })
            .map(|p| PortalBackside { channel: p.channel })
    };
    match (backside, now) {
        (Some(was), Some(now)) if was == now => {}
        (_, Some(now)) => {
            commands.entity(entity).insert(now);
        }
        (Some(_), None) => {
            commands.entity(entity).remove::<PortalBackside>();
        }
        (None, None) => {}
    }
}

/// Per-body transit state. A body is mid-transit while any part of it
/// straddles a portal plane. It transfers to the exit when the centroid
/// crosses, and transit ends when the body fully clears the plane.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalTransit {
    /// Channel of the portal whose plane the body currently straddles — the entry
    /// before the centroid crosses, the exit after.
    pub straddling: PortalChannel,
    /// True once the centroid crossed the entry plane (the body is now on the
    /// exit side).
    pub crossed: bool,
}

/// Output of [`publish_portal_carves`]: the holes open this frame, in publish
/// order, each with the live room of its portal and the bodies it is open
/// for. A host bridge copies each into that room's collision overlay in the
/// same frame and order. Portal core owns the geometry; the host owns how a
/// hole changes its collision.
#[derive(Resource, Clone, Debug, Default)]
pub struct PortalCarves {
    pub holes: Vec<PortalHole>,
    /// The pairs bodies are in this frame, each in its live room: a body in a
    /// pair meets the far side with its part past the plane
    /// (`docs/planning/engine/portals-a-body-in-two-places.md`).
    pub glues: Vec<(PortalRoom, ambition_platformer2d_shared_tangle::feature_overlay::PortalGluePass)>,
}

/// One portal's hole this frame: out of the host's boxes (`face.bounds()`)
/// and out of its lines (a portal on a slope), for the bodies that opened it
/// only (Jon 2026-10-16). The host stays whole for every other body: a body
/// standing on the host beside the opening keeps its floor while another
/// goes through.
#[derive(Clone, Debug, PartialEq)]
pub struct PortalHole {
    pub room: PortalRoom,
    pub face: ae::frame::FaceRect,
    /// The bodies it is open for, in entity order.
    pub open_for: Vec<Entity>,
}

impl PortalCarves {
    /// `body` is in the pair `glue` in `room`: one glue per pair end, for each
    /// body in it.
    fn glue(&mut self, room: PortalRoom, glue: ambition_platformer2d_shared_tangle::feature_overlay::PortalGlue, body: Entity) {
        let pass = match self.glues.iter().position(|(r, pass)| *r == room && pass.glue == glue) {
            Some(i) => &mut self.glues[i].1,
            None => {
                self.glues.push((
                    room,
                    ambition_platformer2d_shared_tangle::feature_overlay::PortalGluePass { glue, bodies: Vec::new() },
                ));
                &mut self.glues.last_mut().expect("just pushed").1
            }
        };
        if let Err(at) = pass.bodies.binary_search(&body) {
            pass.bodies.insert(at, body);
        }
    }

    /// Open `face` in `room` for `body`: one hole per face, open for each
    /// body that opened it.
    fn open(&mut self, room: PortalRoom, face: ae::frame::FaceRect, body: Entity) {
        let hole = match self.holes.iter().position(|h| h.room == room && h.face == face) {
            Some(i) => &mut self.holes[i],
            None => {
                self.holes.push(PortalHole { room, face, open_for: Vec::new() });
                self.holes.last_mut().expect("just pushed")
            }
        };
        if let Err(at) = hole.open_for.binary_search(&body) {
            hole.open_for.insert(at, body);
        }
    }
}

/// Publish apertures that must be carved from host collision this frame.
///
/// A paired portal is carved while a body overlaps the opening,
/// approaches it inward, or is mid-transit. Approach uses a fixed geometric
/// reach, independent of dt. The host bridge applies the `PortalCarves`.
///
/// ⭐ BY LIVE ROOM (OW1): a pair is two portals of one room, a body asks only
/// for the portals of its own room, and each hole names its room. Before, a
/// blue in one room paired with an orange in another, and a body in one room
/// opened a portal of the other room that stood at the same coordinates.
pub fn publish_portal_carves(
    portals: Query<(Entity, &PlacedPortal)>,
    // The record of a body's last step has its DOWN: the core has no frame
    // of a body, and it asks the box the body has.
    bodies: Query<(Entity, &BodyKinematics, Option<&ae::SweepSample>, Option<&PortalBackside>)>,
    transits: Query<(Entity, &PortalTransit, Option<&ambition_platformer2d_shared_tangle::portal_passages::PortalPieceBudget>)>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    host_depths: Option<Res<PortalHostDepthsByRoom>>,
    tuning: Option<Res<PortalTuning>>,
    mut carves: ResMut<PortalCarves>,
    time: Option<Res<ambition_platformer2d_shared_tangle::time::SimDt>>,
) {
    carves.holes.clear();
    carves.glues.clear();
    let dt = time.map_or(0.0, |time| time.get());
    let convention = tuning.as_deref().map(|t| t.convention.map_convention()).unwrap_or_default();
    let geometry = tuning.as_deref().map(|t| t.geometry).unwrap_or_default();
    let by_room = PortalsByRoom::collect(&portals, &live);
    if by_room.is_empty() {
        return;
    }
    // A channel of a room opens only if its partner is placed in that room: a
    // lone portal must not open a hole.
    let carve = |room: PortalRoom, channel: PortalChannel, body: Entity, carves: &mut PortalCarves| {
        let all = by_room.in_room(room);
        let Some(enter) = find_portal(all, channel) else {
            return;
        };
        if find_portal(all, channel.partner()).is_none() {
            return;
        }
        carves.open(room, pp::carve_face_with_depth(&enter.aperture(), f32::INFINITY), body);
    };

    let default_depths = PortalHostDepthsByRoom::default();
    let depths = host_depths.as_deref().unwrap_or(&default_depths);
    for (body_entity, kin, record, backside) in &bodies {
        let room = live.of(body_entity);
        let body = kin.collision_box(record);
        for p in by_room.in_room(room) {
            // A body on the back of a pair does not open it.
            if backside.is_some_and(|back| back.of_pair(p.channel)) {
                continue;
            }
            if body_opens(p, body, kin.pos, kin.vel, depths.in_room(room).depth(p.channel)) {
                carve(room, p.channel, body_entity, &mut carves);
            }
        }
    }
    // Latch: a body that straddles a pair is at both of its ends, so both
    // stay open through the crossing, in the straddling body's room. With
    // only the end it entered open, a body the pair took this tick and that
    // moves later in the tick (a shot) met the closed exit's wall.
    for (body_entity, t, own) in &transits {
        let room = live.of(body_entity);
        carve(room, t.straddling, body_entity, &mut carves);
        carve(room, t.straddling.partner(), body_entity, &mut carves);
        // And it meets the far side: behind the end its centre is in front
        // of, as deep as a hole is, is what is in front of the other. A body
        // the portals move whole (`PortalGeometry::TransferOnly`, its own or
        // the game's) meets only its own side.
        if own.map_or(geometry.budget(), |own| own.0).is_none() {
            continue;
        }
        let all = by_room.in_room(room);
        if let (Some(front), Some(back)) = (find_portal(all, t.straddling), find_portal(all, t.straddling.partner())) {
            let glue = ambition_platformer2d_shared_tangle::feature_overlay::PortalGlue {
                enter: front.aperture(),
                exit: back.aperture(),
                convention,
                depth: pp::CARVE_DEPTH,
                dt,
            };
            carves.glue(room, glue, body_entity);
        }
    }
}

/// Does an owner hold the body's pose (`PoseOwnedExternally` with a claim)?
/// Then the owner places it: the portal cuts it and does not move it.
pub(crate) fn pose_is_held(held: Option<&ae::PoseOwnedExternally>) -> bool {
    held.is_some_and(|held| !held.is_empty())
}

/// The portal of a pair in `all` that a held body is in: the one it was in
/// while it still reaches into its opening, else one it reaches into from
/// that portal's front (its centre in front).
fn threaded_through(body: ae::Aabb, center: Vec2, transit: Option<PortalTransit>, all: &[PlacedPortal]) -> Option<PortalChannel> {
    use ae::frame::thread::reaches_through;
    let volume = ae::CombatVolume::aabb(body);
    let paired = |p: &PlacedPortal| find_portal(all, p.channel.partner()).is_some();
    if let Some(t) = transit {
        return find_portal(all, t.straddling)
            .filter(|p| paired(p) && reaches_through(&volume, &p.aperture()))
            .map(|p| p.channel);
    }
    all.iter()
        .filter(|p| paired(p))
        .find(|p| pp::front_distance(center, &p.aperture().frame) > 0.0 && reaches_through(&volume, &p.aperture()))
        .map(|p| p.channel)
}

/// Does a body (its box, its centre and its velocity) open `p`? The one rule
/// for every body that moves: in line with the opening and in it from the
/// front or approaching it, or inside its hole falling through. A body not in
/// line with the opening (part of it past an end) could not pass whole: the
/// snag would hold it, so it opens nothing (Jon 2026-10-16: a robot on a
/// ledge whose top is the opening's end lost the ledge to the hole). "Approaching" and "falling" are
/// relative to the portal's surface, so a portal that moves or turns onto a
/// still body opens for it. `depth` bounds the hole by the host's thickness.
pub(crate) fn body_opens(p: &PlacedPortal, body: ae::Aabb, center: Vec2, vel: Vec2, depth: f32) -> bool {
    use super::placement::{approach_face, capture_face, portal_fits};
    if !portal_fits(body.half_size() * 2.0, p) {
        return false;
    }
    let ap = p.aperture();
    // Only a body in line with the opening can pass whole, so only it opens
    // the hole: one beside it (on a ledge whose top is the opening's end, on
    // a ramp past its end) leaves the host whole under it.
    if !crate::snag::in_line_with(body, &ap) {
        return false;
    }
    let front = pp::front_distance(center, &ap.frame);
    let into = p.relative_velocity(center, vel).dot(p.normal) < 0.0;
    // Front side only: in the opening now, or approaching fast enough to
    // cross this frame. Otherwise a body behind a thin wall could open a hole
    // through it without transiting.
    let frontal = front >= -TRANSIT_BEGIN_MARGIN
        && (capture_face(p).strict_intersects(body) || (into && approach_face(p).strict_intersects(body)));
    // A fast crossing that skipped Begin keeps the hole open while the body
    // is inside the carve volume, bounded by the host's thickness.
    let falling_through = into && pp::carve_face_with_depth(&ap, depth).strict_intersects(body);
    frontal || falling_through
}

/// The carves free bodies `B` open, added to the ones
/// [`publish_portal_carves`] published this frame (it runs first): a thrown
/// item opens a portal by the same rule a kernel body does, and keeps the
/// portal it straddles open.
pub fn publish_free_body_carves<B: FreePortalBody>(
    portals: Query<(Entity, &PlacedPortal)>,
    bodies: Query<(Entity, &B, Option<&PortalTransit>, Option<&B::Presence>, Option<&PortalBackside>)>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    host_depths: Option<Res<PortalHostDepthsByRoom>>,
    mut carves: ResMut<PortalCarves>,
) {
    let by_room = PortalsByRoom::collect(&portals, &live);
    if by_room.is_empty() {
        return;
    }
    let default_depths = PortalHostDepthsByRoom::default();
    let depths = host_depths.as_deref().unwrap_or(&default_depths);
    for (entity, body, transit, presence, backside) in &bodies {
        if !B::present(presence) {
            continue;
        }
        let room = live.of(entity);
        let all = by_room.in_room(room);
        let aabb = ae::Aabb::new(body.pos(), body.half_extent());
        // A body that straddles a pair is at both of its ends.
        let straddling = transit.map(|t| t.straddling);
        for p in all {
            if backside.is_some_and(|back| back.of_pair(p.channel)) {
                continue;
            }
            let opens = straddling.is_some_and(|c| c == p.channel || c.partner() == p.channel)
                || body_opens(p, aabb, body.pos(), body.vel(), depths.in_room(room).depth(p.channel));
            if !opens || find_portal(all, p.channel.partner()).is_none() {
                continue;
            }
            carves.open(room, pp::carve_face_with_depth(&p.aperture(), f32::INFINITY), entity);
        }
    }
}

/// Convert the movement-kernel sample into swept transit input. Valid only
/// when the sample endpoint still equals the live `kin.pos`. If an earlier
/// post-sim system teleported the body, the segment is not travel through an
/// aperture.
fn portal_sweep_sample(
    kin: &BodyKinematics,
    sweep: Option<&ae::SweepSample>,
) -> Option<SweptSample> {
    let sweep = sweep?.ending_at(kin.pos)?;
    Some(SweptSample {
        pos: sweep.prev,
        vel: sweep.vel,
    })
}

/// Emitted on every Transfer by [`portal_transit`], with what input, trace,
/// and audio adapters need. The host portal adapter reads it and, for the
/// player only, emits [`BodyTeleported`] and inserts `PortalEmission` /
/// `PortalInputWarp`.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalBodyTransited {
    /// The body that just transferred to a portal exit.
    pub body: Entity,
    /// Outward normal of the ENTRY portal.
    pub enter_normal: Vec2,
    /// Outward normal of the EXIT portal (the emergence direction).
    pub exit_normal: Vec2,
    /// True when this convention's orientation policy applies a separate
    /// horizontal facing mirror.
    pub facing_flip: bool,
    /// True when held horizontal movement maps to the opposite horizontal
    /// direction under the active portal convention.
    pub input_warp: bool,
    /// World position the body snapped to (the exit-side centroid).
    pub exit_pos: Vec2,
}

/// The generic transit algorithm: drive every body through a portal aperture
/// with [`transit_step`]. The movement integrator sinks the body into the
/// carved opening. It transfers when the centroid crosses (with rotated
/// momentum and a roll) and clears when the trailing edge is out.
///
/// Every body transits the same way, so no system tags a body before it can
/// transit, and every body carries its momentum: the rotated exit velocity is
/// written. Facing is not written here. On a same-wall turn-around the event
/// states `facing_flip`, and the host turns around the body a seat drives,
/// whose facing follows that seat's input; a brain decides the facing of any
/// other body. Who drives a body is control authority, which this core does not
/// know.
///
/// Transit does not need the [`PortalGun`](super::gun::PortalGun). The
/// anti-ping-pong cooldown is on the body ([`PortalTransitCooldown`]).
///
/// ⭐ A body transits only through a pair of its own live room (OW1), so a
/// body in one room does not cross a portal of another room at the same
/// coordinates, and a blue and an orange in two rooms are not a pair.
pub fn portal_transit(
    mut commands: Commands,
    portals: Query<(Entity, &PlacedPortal)>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut bodies: Query<
        (
            Entity,
            &mut BodyKinematics,
            Option<&mut PortalTransit>,
            Option<&mut ActorRoll>,
            Option<&PortalTransitCooldown>,
            Option<&ae::SweepSample>,
            Option<&PortalBackside>,
            Option<&ae::PoseOwnedExternally>,
        ),
    >,
    // `GravityCtx::dir_for(aabb)` owns "which way is down for this body":
    // zones give a per-body lookup with a `BaseGravity` fallback. Do not
    // re-implement it from `GravityField`, which mirrors the primary body.
    // One param also keeps this system under Bevy's 16-param limit.
    gravity: ambition_platformer2d_shared_tangle::gravity::GravityCtx,
    tuning: Res<PortalTuning>,
    host_depths: Option<Res<PortalHostDepthsByRoom>>,
    mut entered: MessageWriter<super::messages::PortalBodyEntered>,
    mut transited: MessageWriter<PortalBodyTransited>,
    // Optional: a minimal test app may not have it. Diagnostic only.
    mut class_b: Option<ResMut<ClassBRemapLog>>,
    // The measurement trace names its tick; a minimal app may not count.
    tick: Option<Res<ambition_time::SimTick>>,
) {
    let tick = tick.map_or(0, |tick| tick.get());
    let by_room = PortalsByRoom::collect(&portals, &live);
    let default_depths = PortalHostDepthsByRoom::default();
    let depths = host_depths.as_deref().unwrap_or(&default_depths);

    for (entity, mut kin, mut transit, mut roll, cooldown, record, backside, held) in &mut bodies {
        if transit.is_none() && by_room.is_empty() {
            continue;
        }
        let room = live.of(entity);
        let all = by_room.in_room(room);
        // No portal in the body's room: nothing to cross, and a straddle of a
        // pair that closed ends here (the machine's `Clear`, without asking it).
        if all.is_empty() {
            if transit.is_some() {
                commands.entity(entity).remove::<PortalTransit>();
            }
            continue;
        }
        // ⭐ A BODY WHOSE POSE AN OWNER HOLDS is posed in its own chart, as if
        // there were no portal (`docs/planning/engine/portals-a-body-in-two-places.md`):
        // the portal does not move it. It is in the pair it reaches into from
        // a portal's front while it reaches into it, and its pieces carry the
        // consequences at the far end.
        if pose_is_held(held) {
            let body = kin.collision_box(record);
            let into = threaded_through(body, kin.pos, transit.as_deref().copied(), all);
            match (into, transit.as_deref()) {
                (Some(channel), Some(t)) if t.straddling == channel => {}
                (Some(channel), _) => {
                    commands.entity(entity).insert(PortalTransit { straddling: channel, crossed: false });
                }
                (None, Some(_)) => {
                    commands.entity(entity).remove::<PortalTransit>();
                }
                (None, None) => {}
            }
            continue;
        }
        // Per body, not once for all: `placement::wall_to_wall` classifies each
        // aperture as wall or floor/ceiling relative to this body's down.
        let gravity_dir = gravity.dir_for(room, ambition_platformer2d_core::Aabb::new(
            kin.pos,
            kin.size * 0.5,
        ));
        // Body latch (`PortalTransitCooldown`), ticked by
        // `tick_portal_cooldowns`, scoped to the pair just crossed.
        let cooldown_pair = cooldown.map(|c| c.pair);
        // The swept segment comes from the movement kernel's `SweepSample`. It
        // is used only when its `curr` equals the live `kin.pos`, so teleports
        // outside the sim phase are not swept travel.
        let sweep = portal_sweep_sample(&*kin, record);
        // The size of the box the body has: its collision box, turned as its
        // last step turned it. The core has no frame of a body.
        let size = kin.collision_box(record).half_size() * 2.0;
        let backside = backside.copied();
        let usable = usable_portals(all, backside);
        let step = transit_step_with_tuning(
            kin.pos,
            size,
            kin.vel,
            sweep,
            transit.as_deref().copied(),
            cooldown_pair,
            &usable,
            gravity_dir,
            depths.in_room(room),
            &tuning,
        );
        // A measurement trace (`RUST_LOG=ambition_portal::transit_trace=debug`,
        // read by `scripts/portal_seam_lab.py`), as a free body's.
        bevy::log::debug!(
            target: "ambition_portal::transit_trace",
            "tick={tick} kernel body={entity:?} at={:.1},{:.1} vel={:.1},{:.1} step={step:?}",
            kin.pos.x,
            kin.pos.y,
            kin.vel.x,
            kin.vel.y
        );
        let crossing = transit.is_some() || !matches!(step, TransitStep::Idle);
        update_backside(&mut commands, entity, backside, crossing, all, kin.collision_box(record));
        match step {
            TransitStep::Idle | TransitStep::Continue => {}
            TransitStep::Begin {
                channel,
                portal_pos,
            } => {
                commands.entity(entity).insert(PortalTransit {
                    straddling: channel,
                    crossed: false,
                });
                // An Ambition audio adapter plays the enter cue.
                entered.write(super::messages::PortalBodyEntered { pos: portal_pos });
            }
            TransitStep::Transfer {
                pos,
                vel,
                roll_delta,
                facing_flip,
                input_warp,
                enter_normal,
                exit_normal,
                exit_channel,
                exit_pos,
            } => {
                kin.pos = pos;
                // Class-B transit authority (`docs/concepts/movement-collision.md`),
                // recorded when the position is written. The CC3 oracle uses it
                // to tell an aperture warp from a clip through solid geometry.
                if let Some(log) = class_b.as_mut() {
                    log.record(entity, ClassBRemap::PortalTransit);
                }
                kin.vel = vel;
                if let Some(roll) = roll.as_deref_mut() {
                    roll.angle += roll_delta;
                }
                // Latch the cooldown for this pair to stop ping-pong.
                commands.entity(entity).insert(PortalTransitCooldown {
                    remaining: tuning.teleport_cooldown_s,
                    pair: exit_channel,
                });
                // The body straddles the exit now, however it got here: a
                // rescue or a swept crossing that skipped `Begin` is drawn cut
                // at the exit as a straddle is, until it clears.
                set_crossed(&mut commands, entity, transit.as_deref_mut(), exit_channel);
                // Ambition adapters read this event for the exit cue (at
                // `exit_pos`), the trace message, and the player input bits
                // (`PortalEmission`, `PortalInputWarp`).
                transited.write(PortalBodyTransited {
                    body: entity,
                    enter_normal,
                    exit_normal,
                    facing_flip,
                    input_warp,
                    exit_pos,
                });
                bevy::log::info!(target: "ambition_platformer2d::portal", "transferred through the portal pair");
            }
            TransitStep::Clear => {
                commands.entity(entity).remove::<PortalTransit>();
            }
        }
    }
}

/// Complete a transit for a kernel body (ADR 0024). The transit wrote the
/// pose, so the facts of the departure point no longer hold: its contacts are
/// invalidated, wall cling and ledge grab are released, a riding momentum body
/// arrives airborne, an attached crawler arrives detached, and the motion
/// record collapses to the arrival point. It runs in the same set after
/// [`portal_transit`], so the next movement step sees the reconciled state.
///
/// A body without movement clusters (a projectile) has nothing to reconcile.
pub fn reconcile_transited_bodies(
    mut transited: MessageReader<PortalBodyTransited>,
    mut bodies: Query<(ae::BodyClusterQueryData, &mut ae::movement::MotionModel)>,
) {
    for event in transited.read() {
        let Ok((mut clusters, mut model)) = bodies.get_mut(event.body) else {
            continue;
        };
        ae::movement::reconcile_transit(&mut model, &mut clusters.as_clusters_mut());
    }
}

/// Stops portal ping-pong from held input: after a crossing, held movement is
/// warped by the same portal map as velocity, when that map keeps horizontal
/// movement expressible. Soft, not a hard latch; see the runtime's
/// `warp_portal_input`.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalInputWarp {
    /// Entry and exit portal normals. Held movement is mapped through the
    /// tangent-preserving portal map.
    pub n_in: Vec2,
    pub n_out: Vec2,
    /// Raw (un-warped) movement direction held when the warp was set; the warp
    /// drops once the live raw input releases or clearly diverges from this.
    pub anchor: Vec2,
}

/// Short guard set on every crossing of a driven body by the runtime's
/// `guard_the_driven_body_after_portal_transit`:
/// held input cannot push back into the exit wall (against `exit_normal`), so
/// the exit velocity carries the body out. Works for any gravity direction.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalEmission {
    /// Outward normal of the exit portal (the emergence direction).
    pub exit_normal: Vec2,
    /// Remaining protection time (s).
    pub timer: f32,
}

/// A free body that moves on its own (a thrown axe, a javelin, a dropped
/// item): it straddles a portal and crosses it when its centre does, as every
/// body does. Its owner implements this on the component that IS the body,
/// so portal core moves the body itself and never names the owner's type or
/// holds a copy of it.
pub trait FreePortalBody: Component<Mutability = bevy::ecs::component::Mutable> {
    /// World position of the body's centre.
    fn pos(&self) -> Vec2;
    /// Current velocity.
    fn vel(&self) -> Vec2;
    /// Half-extent of the body's box.
    fn half_extent(&self) -> Vec2;
    /// Put the body where the exit portal sends it, moving as it sends it.
    fn emerge(&mut self, pos: Vec2, vel: Vec2);
    /// The body's last step of its own motion, `(from, to)`: the segment a
    /// fast body crosses a plane on between two samples. `None` once it was
    /// put somewhere else (`to` is no longer [`pos`](Self::pos)) or did not
    /// move.
    fn last_step(&self) -> Option<(Vec2, Vec2)> {
        None
    }
    /// The component that says whether the body is in the world.
    type Presence: Component;
    /// A body something else holds is not a free body: it opens no portal,
    /// crosses none, and straddles none.
    fn present(presence: Option<&Self::Presence>) -> bool;
}

/// Free bodies `B` travel through every placed portal pair by the one transit
/// machine ([`transit_step_with_tuning`]) every body uses. A body that enters
/// an opening straddles it ([`PortalTransit`]); the pair takes it when its
/// centre crosses, to the image of its centre, keeping its velocity relative
/// to the surfaces it crosses; and the pair it crossed is latched by a
/// [`PortalTransitCooldown`]. Nothing teleports from the face of a portal.
///
/// A composition that has both portals and the body type registers
/// `portal_transit_free_bodies::<B>` in [`crate::PortalSet::Transit`] and
/// [`publish_free_body_carves::<B>`] in [`crate::PortalSet::Carves`].
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn portal_transit_free_bodies<B: FreePortalBody>(
    mut commands: Commands,
    portals: Query<(Entity, &PlacedPortal)>,
    mut bodies: Query<(
        Entity,
        &mut B,
        Option<&mut PortalTransit>,
        Option<&mut ActorRoll>,
        Option<&PortalTransitCooldown>,
        Option<&B::Presence>,
        Option<&PortalBackside>,
    )>,
    // A free body crosses only a pair of its own live room (OW1).
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    gravity: ambition_platformer2d_shared_tangle::gravity::GravityCtx,
    tuning: Res<crate::tuning::PortalTuning>,
    host_depths: Option<Res<PortalHostDepthsByRoom>>,
    mut entered: MessageWriter<super::messages::PortalBodyEntered>,
    mut transited: MessageWriter<PortalBodyTransited>,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    let by_room = PortalsByRoom::collect(&portals, &live);
    let tick = tick.map_or(0, |tick| tick.get());
    let default_depths = PortalHostDepthsByRoom::default();
    let depths = host_depths.as_deref().unwrap_or(&default_depths);
    for (entity, mut body, mut transit, mut roll, cooldown, presence, backside) in &mut bodies {
        if transit.is_none() && by_room.is_empty() {
            continue;
        }
        let room = live.of(entity);
        let all = by_room.in_room(room);
        // A held body, or one with no portal in its room, straddles nothing:
        // a straddle it had ends here, so it is released clear of any pair.
        if !B::present(presence) || all.is_empty() {
            if transit.is_some() {
                commands.entity(entity).remove::<PortalTransit>();
            }
            continue;
        }
        let (pos, vel, half) = (body.pos(), body.vel(), body.half_extent());
        let gravity_dir = gravity.dir_for(room, ae::Aabb::new(pos, half));
        // Its last step, while it still ends where the body is: a fast throw
        // that crossed a plane between two samples is taken through it.
        let sweep = body
            .last_step()
            .filter(|(_, to)| *to == pos)
            .map(|(from, _)| SweptSample { pos: from, vel });
        let backside = backside.copied();
        let usable = usable_portals(all, backside);
        let step = transit_step_with_tuning(
            pos,
            half * 2.0,
            vel,
            sweep,
            transit.as_deref().copied(),
            cooldown.map(|c| c.pair),
            &usable,
            gravity_dir,
            depths.in_room(room),
            &tuning,
        );
        // A measurement trace (`RUST_LOG=ambition_portal::transit_trace=debug`).
        bevy::log::debug!(
            target: "ambition_portal::transit_trace",
            "tick={tick} free body={entity:?} at={:.1},{:.1} vel={:.1},{:.1} step={step:?}",
            pos.x,
            pos.y,
            vel.x,
            vel.y
        );
        let crossing = transit.is_some() || !matches!(step, TransitStep::Idle);
        update_backside(&mut commands, entity, backside, crossing, all, ae::Aabb::new(pos, half));
        match step {
            TransitStep::Idle | TransitStep::Continue => {}
            TransitStep::Begin { channel, portal_pos } => {
                commands.entity(entity).insert(PortalTransit { straddling: channel, crossed: false });
                entered.write(super::messages::PortalBodyEntered { pos: portal_pos });
            }
            TransitStep::Transfer {
                pos,
                vel,
                roll_delta,
                facing_flip,
                input_warp,
                enter_normal,
                exit_normal,
                exit_channel,
                exit_pos,
            } => {
                body.emerge(pos, vel);
                // Turned as the pair turns it, as a kernel body is: its image
                // on the far side is drawn turned, and the body must be the
                // image it was drawn as, not snap upright.
                if let Some(roll) = roll.as_deref_mut() {
                    roll.angle += roll_delta;
                }
                commands.entity(entity).insert(PortalTransitCooldown {
                    remaining: tuning.teleport_cooldown_s,
                    pair: exit_channel,
                });
                // The body straddles the exit now, however it got here: a
                // rescue or a swept crossing that skipped `Begin` is drawn cut
                // at the exit as a straddle is, until it clears.
                set_crossed(&mut commands, entity, transit.as_deref_mut(), exit_channel);
                transited.write(PortalBodyTransited {
                    body: entity,
                    enter_normal,
                    exit_normal,
                    facing_flip,
                    input_warp,
                    exit_pos,
                });
            }
            TransitStep::Clear => {
                commands.entity(entity).remove::<PortalTransit>();
            }
        }
    }
}

/// After a transfer the body straddles the exit, crossed: updated in place,
/// or established when the crossing skipped `Begin`.
fn set_crossed(
    commands: &mut Commands,
    entity: Entity,
    transit: Option<&mut PortalTransit>,
    exit_channel: PortalChannel,
) {
    match transit {
        Some(t) => {
            t.crossed = true;
            t.straddling = exit_channel;
        }
        None => {
            commands.entity(entity).insert(PortalTransit { straddling: exit_channel, crossed: true });
        }
    }
}

/// Tick down (and clear) per-actor [`PortalTransitCooldown`]s.
pub fn tick_portal_cooldowns(
    time: Res<ambition_platformer2d_shared_tangle::time::SimDt>,
    mut commands: Commands,
    mut cooldowns: Query<(Entity, &mut PortalTransitCooldown)>,
) {
    let dt = time.get();
    if dt <= 0.0 {
        return;
    }
    for (entity, mut cooldown) in &mut cooldowns {
        cooldown.remaining -= dt;
        if cooldown.remaining <= 0.0 {
            commands.entity(entity).remove::<PortalTransitCooldown>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_sweep_sample_uses_kernel_segment_when_live_endpoint_matches() {
        let kin = BodyKinematics {
            pos: Vec2::new(10.0, 20.0),
            vel: Vec2::new(0.0, 30.0),
            size: Vec2::new(24.0, 40.0),
            facing: 1.0,
        };
        let sweep = ae::SweepSample {
            prev: Vec2::new(10.0, 12.0),
            curr: kin.pos,
            vel: Vec2::new(0.0, 240.0),
            half: kin.size * 0.5,
            down: Vec2::new(0.0, 1.0),
        };

        let sample = portal_sweep_sample(&kin, Some(&sweep)).expect("matching sample is valid");
        assert_eq!(sample.pos, sweep.prev);
        assert_eq!(sample.vel, sweep.vel);
    }

    #[test]
    fn portal_sweep_sample_rejects_post_sim_teleport_gap() {
        let kin = BodyKinematics {
            pos: Vec2::new(1000.0, 1000.0),
            vel: Vec2::ZERO,
            size: Vec2::new(24.0, 40.0),
            facing: 1.0,
        };
        let sweep = ae::SweepSample {
            prev: Vec2::new(10.0, 12.0),
            curr: Vec2::new(10.0, 20.0),
            vel: Vec2::new(0.0, 240.0),
            half: Vec2::new(12.0, 20.0),
            down: Vec2::new(0.0, 1.0),
        };

        assert_eq!(portal_sweep_sample(&kin, Some(&sweep)), None);
    }
}
