//! Portal-specific transit systems: drive every body and in-flight item
//! through a placed portal pair via the shared
//! [`super::placement::transit_step`] aperture machine, plus the carve / input /
//! ability-suppression guards that make a crossing feel right.

use bevy::prelude::*;

use crate::pieces as pp;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::class_b::{ClassBRemap, ClassBRemapLog};
use ambition_platformer2d_shared_tangle::frame_env::{CarriedAcceleration, ResolvedMotionFrame};

use super::color::PortalChannel;
use super::placement::{transit_step_with_tuning, TransitStep, TRANSIT_BEGIN_MARGIN};
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
    /// A root crossing changes its chart. Both host openings must be ready.
    fn open_pair(&mut self, room: PortalRoom, enter: &PlacedPortal, exit: &PlacedPortal, body: Entity) {
        for end in [enter, exit] {
            self.open(room, pp::carve_face_with_depth(&end.aperture(), f32::INFINITY), body);
        }
    }

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
/// Run condition of [`publish_portal_carves`]: a portal stands, or last
/// tick's holes or glues are still to clear.
/// Run condition of the passes over placed portals and the bodies crossing
/// them: a portal stands, or a body still carries a transit latch, which the
/// next run that finds no portal in its room takes off.
pub fn portals_or_transits(portals: Query<(), With<PlacedPortal>>, transits: Query<(), With<PortalTransit>>) -> bool {
    !portals.is_empty() || !transits.is_empty()
}

pub fn portals_or_their_carves(portals: Query<(), With<PlacedPortal>>, carves: Res<PortalCarves>) -> bool {
    !portals.is_empty() || !carves.holes.is_empty() || !carves.glues.is_empty()
}

pub fn publish_portal_carves(
    portals: Query<(Entity, &PlacedPortal)>,
    bodies: Query<(Entity, &BodyKinematics, Option<&PortalBackside>), Without<ae::BodyInStorage>>,
    transits: Query<(Entity, &PortalTransit), Without<ae::BodyInStorage>>,
    live: crate::rooms::PortalRooms,
    host_depths: Option<Res<PortalHostDepthsByRoom>>,
    tuning: Option<Res<PortalTuning>>,
    mut carves: ResMut<PortalCarves>,
    time: Option<Res<ambition_platformer2d_shared_tangle::time::SimDt>>,
) {
    carves.holes.clear();
    carves.glues.clear();
    let dt = time.map_or(0.0, |time| time.get());
    let convention = tuning.as_deref().map(|t| t.convention.map_convention()).unwrap_or_default();
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
        let Some(exit) = find_portal(all, channel.partner()) else { return; };
        carves.open_pair(room, &enter, &exit, body);
    };

    let default_depths = PortalHostDepthsByRoom::default();
    let depths = host_depths.as_deref().unwrap_or(&default_depths);
    for (body_entity, kin, backside) in &bodies {
        let room = live.of(body_entity);
        let body = kin.aabb();
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
    for (body_entity, t) in &transits {
        let room = live.of(body_entity);
        carve(room, t.straddling, body_entity, &mut carves);
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
/// for every body that moves: it fits the opening, its centre is over the
/// opening, and it is in the opening from the front or approaching it, or
/// inside its hole falling through. A body whose centre is past an end opens
/// nothing (a robot on a ledge whose top is the opening's end keeps the
/// ledge). A body whose centre is over the opening and whose box is partly
/// past an end opens the hole and stands on the host past the end by its
/// own support rule: one that stands by its balance (`ae::Balance`) is
/// pushed off that end into the hole; one that stands on a lip stays on it
/// until it is in line, and the snag holds it there. "Approaching" and
/// "falling" are relative to the portal's surface, so a portal that moves or
/// turns onto a still body opens for it. `depth` bounds the hole by the
/// host's thickness.
pub(crate) fn body_opens(p: &PlacedPortal, body: ae::Aabb, center: Vec2, vel: Vec2, depth: f32) -> bool {
    use super::placement::{approach_face, capture_face, portal_fits};
    if !portal_fits(body.half_size() * 2.0, p) {
        return false;
    }
    let ap = p.aperture();
    if (center - ap.frame.origin).dot(ap.frame.tangent()).abs() > ap.half_length {
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


/// Emitted on every Transfer by [`portal_transit`], with what input, trace,
/// and audio adapters need. The host portal adapter reads it and, for the
/// player only, emits [`BodyTeleported`] and inserts `PortalEmission` /
/// `PortalInputWarp`.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalBodyTransited {
    /// The body that just transferred to a portal exit.
    pub body: Entity,
    pub transfer: super::placement::PortalTransfer,
    /// True when held horizontal movement maps to the opposite horizontal
    /// direction under the active portal convention.
    pub input_warp: bool,
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
    live: crate::rooms::PortalRooms,
    mut bodies: Query<
        (
            Entity,
            &mut BodyKinematics,
            Option<&mut CarriedAcceleration>,
            Option<&mut ae::BodyFlightState>,
            Option<&ResolvedMotionFrame>,
            Option<&mut PortalTransit>,
            Option<&PortalTransitCooldown>,
            Option<&ae::SweepSample>,
            Option<&PortalBackside>,
            Option<&ae::PoseOwnedExternally>,
            Has<ae::BodyInStorage>,
        ),
    >,
    frame_env: ambition_platformer2d_shared_tangle::frame_env::FrameEnv,
    tuning: Res<PortalTuning>,
    mut entered: MessageWriter<super::messages::PortalBodyEntered>,
    mut transited: MessageWriter<PortalBodyTransited>,
    // Optional: a minimal test app may not have it. Diagnostic only.
    mut class_b: Option<ResMut<ClassBRemapLog>>,
    // The measurement trace names its tick; a minimal app may not count.
    tick: Option<Res<ambition_time::SimTick>>,
) {
    let tick = tick.map_or(0, |tick| tick.get());
    let by_room = PortalsByRoom::collect(&portals, &live);

    for (entity, mut kin, mut acceleration, mut flight, frame, mut transit, cooldown, record, backside, held, stored) in &mut bodies {
        if stored || !live.allows(entity) {
            commands.entity(entity).remove::<(PortalTransit, PortalBackside, PortalTransitCooldown)>();
            continue;
        }
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
            let body = kin.aabb();
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
        // Body latch (`PortalTransitCooldown`), ticked by
        // `tick_portal_cooldowns`, scoped to the pair just crossed.
        let cooldown_pair = cooldown.map(|c| c.pair);
        // The swept segment comes from the movement kernel's `SweepSample`. It
        // is used only when its `curr` equals the live `kin.pos`, so teleports
        // outside the sim phase are not swept travel.
        let sweep = record.copied();
        let backside = backside.copied();
        let usable = usable_portals(all, backside);
        let step = transit_step_with_tuning(*kin,
            sweep,
            transit.as_deref().copied(),
            cooldown_pair,
            &usable,
            &tuning,);
        // A measurement trace (`RUST_LOG=ambition_portal::transit_trace=debug`,
        // read by `scripts/measure/portal_seam_lab.py`), as a free body's.
        bevy::log::debug!(
            target: "ambition_portal::transit_trace",
            "tick={tick} kernel body={entity:?} at={:.1},{:.1} vel={:.1},{:.1} step={step:?}",
            kin.pos.x,
            kin.pos.y,
            kin.vel.x,
            kin.vel.y
        );
        let (mapped_acceleration, carried_run, mapped_material) = match step {
            TransitStep::Transfer { transfer, .. } => {
                let payload = (|| {
                    let acceleration = acceleration.as_deref()
                        .map(|acceleration| transfer.map.vector(acceleration.0)).transpose()?;
                    let material = kin.material.carried(transfer.map)?;
                    let destination = BodyKinematics { pos: transfer.exit_pos, material, ..*kin };
                    let destination_side = frame_env.reference_basis(destination.aabb(), room).side;
                    let source_side = frame.map_or_else(|| frame_env.reference_basis(kin.aabb(), room).side, |frame| frame.basis().side);
                    let carried_run = flight.as_deref().map(|flight| {
                        transfer.carried_run(source_side, destination_side, flight.carried_run)
                    }).transpose()?;
                    Ok::<_, ae::frame::NumericFault>((acceleration, carried_run, material))
                })();
                let Ok((mapped_acceleration, carried_run, material)) = payload else { continue; };
                (mapped_acceleration, carried_run, Some(material))
            }
            TransitStep::Refused { .. } => continue,
            _ => (None, None, None),
        };
        let crossing = transit.is_some() || !matches!(step, TransitStep::Idle);
        update_backside(&mut commands, entity, backside, crossing, all, kin.aabb());
        match step {
            TransitStep::Idle | TransitStep::Continue | TransitStep::Refused { .. } => {}
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
                transfer,
                input_warp,
            } => {
                if let Some((acceleration, mapped)) = acceleration.as_deref_mut().zip(mapped_acceleration) {
                    acceleration.0 = mapped;
                }
                if let Some((flight, carried_run)) = flight.as_deref_mut().zip(carried_run) {
                    flight.carried_run = carried_run;
                }
                kin.pos = transfer.exit_pos;
                // Class-B transit authority (`docs/concepts/movement-collision.md`),
                // recorded when the position is written. The CC3 oracle uses it
                // to tell an aperture warp from a clip through solid geometry.
                if let Some(log) = class_b.as_mut() {
                    log.record(entity, ClassBRemap::PortalTransit);
                }
                kin.vel = transfer.exit_vel;
                if let Some(material) = mapped_material { kin.material = material; }
                // Latch the cooldown for this pair to stop ping-pong.
                commands.entity(entity).insert(PortalTransitCooldown {
                    remaining: tuning.teleport_cooldown_s,
                    pair: transfer.exit_channel,
                });
                // The body straddles the exit now, however it got here: a
                // rescue or a swept crossing that skipped `Begin` is drawn cut
                // at the exit as a straddle is, until it clears.
                set_crossed(&mut commands, entity, transit.as_deref_mut(), transfer.exit_channel);
                // Ambition adapters read this event for the exit cue (at
                // `exit_pos`), the trace message, and the player input bits
                // (`PortalEmission`, `PortalInputWarp`).
                transited.write(PortalBodyTransited {
                    body: entity,
                    transfer,
                    input_warp,
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
