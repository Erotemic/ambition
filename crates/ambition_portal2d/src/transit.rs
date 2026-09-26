//! Portal-specific transit systems: drive every body and in-flight item
//! through a placed portal pair via the shared
//! [`super::placement::transit_step`] aperture machine, plus the carve / input /
//! ability-suppression guards that make a crossing feel right.

use bevy::prelude::*;

use crate::pieces as pp;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::body::BodyKinematics;
use ambition_platformer2d_shared_tangle::class_b::{ClassBRemap, ClassBRemapLog};
use ambition_platformer2d_shared_tangle::orientation::ActorRoll;
use ambition_platformer2d_shared_tangle::transit::rotate_velocity_between_normals as portal_transform_velocity;

use super::color::PortalChannel;
use super::placement::{transit_step_with_tuning, SweptSample, TransitStep, TRANSIT_BEGIN_MARGIN};
use super::tuning::PortalTuning;
use super::types::{
    find_portal, portal_exit_clearance, PlacedPortal, PortalHostDepths, PortalTransitCooldown,
};

/// A body's position just moved to a portal exit (the centroid crossed).
/// Carries the entity so a consumer can filter to one body (e.g. the locally
/// focused body).
#[derive(Message, Clone, Copy, Debug)]
pub struct BodyTeleported {
    /// The body whose position snapped to a portal exit this frame.
    pub body: Entity,
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

/// Output of [`publish_portal_carves`]: the aperture rectangles to carve out of
/// the host surface this frame, in publish order. A host bridge copies them into
/// the host collision overlay in the same frame and order. Portal core owns the
/// geometry; the host owns how a carve changes its collision.
#[derive(Resource, Clone, Debug, Default)]
pub struct PortalCarves {
    /// Aperture rectangles to carve this frame, in publish order.
    pub holes: Vec<ae::Aabb>,
}

/// Publish apertures that must be carved from host collision this frame.
///
/// A paired portal is carved while a body overlaps the opening,
/// approaches it inward, or is mid-transit. Approach uses a fixed geometric
/// reach, independent of dt. The host bridge applies the `PortalCarves`.
pub fn publish_portal_carves(
    portals: Query<&PlacedPortal>,
    bodies: Query<&BodyKinematics>,
    transits: Query<&PortalTransit>,
    host_depths: Option<Res<PortalHostDepths>>,
    mut carves: ResMut<PortalCarves>,
) {
    use super::placement::{approach_box, capture_box, portal_fits};

    carves.holes.clear();
    // Sort here: downstream loops take the first match, and `Query` order is
    // archetype order, which a rollback resimulation may not reproduce.
    let mut all: Vec<PlacedPortal> = portals.iter().cloned().collect();
    all.sort_by(crate::stable_portal_order);
    if all.is_empty() {
        return;
    }
    // Carve each channel once, and only if its partner is placed: a lone
    // portal must not open a hole.
    let mut carved: Vec<PortalChannel> = Vec::new();
    let mut carve = |channel: PortalChannel, holes: &mut Vec<ae::Aabb>| {
        if carved.contains(&channel) {
            return;
        }
        let Some(enter) = find_portal(&all, channel) else {
            return;
        };
        if find_portal(&all, channel.partner()).is_none() {
            return;
        }
        holes.push(pp::carve_hole(&enter.aperture()));
        carved.push(channel);
    };

    let depths = host_depths.as_deref();
    for kin in &bodies {
        let body = ae::Aabb::new(kin.pos, kin.size * 0.5);
        for p in &all {
            if !portal_fits(kin.size, p) {
                continue;
            }
            let ap = p.aperture();
            let front = pp::front_distance(kin.pos, &ap.frame);
            // Front side only: in the opening now, or approaching fast enough
            // to cross this frame. Otherwise a body behind a thin wall could
            // open a hole through it without transiting.
            let frontal = front >= -TRANSIT_BEGIN_MARGIN
                && (body.strict_intersects(capture_box(p))
                    || (kin.vel.dot(p.normal) < 0.0 && body.strict_intersects(approach_box(p))));
            // Fast crossing that skipped Begin: keep the hole open while the
            // body is inside the carve volume, bounded by the host depth.
            let hole = pp::carve_hole_with_depth(
                &ap,
                depths.map_or(f32::INFINITY, |d| d.depth(p.channel)),
            );
            let falling_through = kin.vel.dot(p.normal) < 0.0 && body.strict_intersects(hole);
            if frontal || falling_through {
                carve(p.channel, &mut carves.holes);
            }
        }
    }
    // Latch: keep a straddled portal open through the centroid crossing.
    for t in &transits {
        carve(t.straddling, &mut carves.holes);
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
/// How a body takes part is derived from what it is, so no system tags a body
/// before it can transit:
/// - Every body carries its momentum: the rotated exit velocity is written.
/// - Only a body in the player population reorients: on a same-wall
///   turn-around its facing flips, because its facing follows its seat's input.
///   A brain decides the facing of any other body.
///
/// Transit does not need the [`PortalGun`](super::gun::PortalGun). The
/// anti-ping-pong cooldown is on the body ([`PortalTransitCooldown`]).
pub fn portal_transit(
    mut commands: Commands,
    portals: Query<&PlacedPortal>,
    mut bodies: Query<
        (
            Entity,
            &mut BodyKinematics,
            Has<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Option<&mut PortalTransit>,
            Option<&mut ActorRoll>,
            Option<&PortalTransitCooldown>,
            Option<&ae::SweepSample>,
        ),
    >,
    // `GravityCtx::dir_for(aabb)` owns "which way is down for this body":
    // zones give a per-body lookup with a `BaseGravity` fallback. Do not
    // re-implement it from `GravityField`, which mirrors the primary body.
    // One param also keeps this system under Bevy's 16-param limit.
    gravity: ambition_platformer2d_shared_tangle::gravity::GravityCtx,
    tuning: Res<PortalTuning>,
    host_depths: Option<Res<PortalHostDepths>>,
    mut entered: MessageWriter<super::messages::PortalBodyEntered>,
    mut transited: MessageWriter<PortalBodyTransited>,
    // Optional: a minimal test app may not have it. Diagnostic only.
    mut class_b: Option<ResMut<ClassBRemapLog>>,
) {
    // Sort here: downstream loops take the first match, and `Query` order is
    // archetype order, which a rollback resimulation may not reproduce.
    let mut all: Vec<PlacedPortal> = portals.iter().cloned().collect();
    all.sort_by(crate::stable_portal_order);
    if all.is_empty() {
        return;
    }

    for (entity, mut kin, reorients, mut transit, mut roll, cooldown, sweep) in &mut bodies {
        // Per body, not once for all: `placement::wall_to_wall` classifies each
        // aperture as wall or floor/ceiling relative to this body's down.
        let gravity_dir = gravity.dir_for(ambition_platformer2d_core::Aabb::new(
            kin.pos,
            kin.size * 0.5,
        ));
        // Body latch (`PortalTransitCooldown`), ticked by
        // `tick_portal_cooldowns`, scoped to the pair just crossed.
        let cooldown_pair = cooldown.map(|c| c.pair);
        let default_depths = PortalHostDepths::default();
        // The swept segment comes from the movement kernel's `SweepSample`. It
        // is used only when its `curr` equals the live `kin.pos`, so teleports
        // outside the sim phase are not swept travel.
        let sweep = portal_sweep_sample(&*kin, sweep);
        let step = transit_step_with_tuning(
            kin.pos,
            kin.size,
            kin.vel,
            sweep,
            transit.as_deref().copied(),
            cooldown_pair,
            &all,
            gravity_dir,
            host_depths.as_deref().unwrap_or(&default_depths),
            &tuning,
        );
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
                // Flip facing on a same-wall turn-around only for a body that
                // reorients, and only if `tuning.reorient_facing` (mirrors the
                // `portal_reverses_facing` setting) allows it.
                if reorients && facing_flip && tuning.reorient_facing {
                    kin.facing = -kin.facing;
                }
                if let Some(roll) = roll.as_deref_mut() {
                    roll.angle += roll_delta;
                }
                // Latch the cooldown for this pair to stop ping-pong.
                commands.entity(entity).insert(PortalTransitCooldown {
                    remaining: tuning.teleport_cooldown_s,
                    pair: exit_channel,
                });
                if let Some(t) = transit.as_deref_mut() {
                    t.crossed = true;
                    t.straddling = exit_channel;
                }
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
/// movement expressible. Soft, not a hard latch; see the Ambition
/// `warp_portal_input` adapter.
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

/// Short guard set on every crossing by the Ambition player-input adapter:
/// held input cannot push back into the exit wall (against `exit_normal`), so
/// the exit velocity carries the body out. Works for any gravity direction.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalEmission {
    /// Outward normal of the exit portal (the emergence direction).
    pub exit_normal: Vec2,
    /// Remaining protection time (s).
    pub timer: f32,
}

/// A free in-flight body that travels through portal pairs (thrown axes,
/// javelins, other projectiles). It carries only the kinematics
/// [`portal_teleport_ground_items`] uses, so portal core never names the
/// Ambition `GroundItem`. The host portal adapter attaches it and syncs it each
/// frame. Resting bodies (`vel == ZERO`) are ignored.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalTransitable {
    /// Authoritative world position of the body's center.
    pub pos: Vec2,
    /// Current velocity; `ZERO` means "resting", which never transits.
    pub vel: Vec2,
    /// Half-extent (AABB) used for the portal overlap test and exit clearance.
    pub half_extent: Vec2,
}

/// Moving [`PortalTransitable`] bodies travel through every placed portal
/// pair (gun-fired, authored, or link-authored), keeping momentum through the
/// rotation. A body must move into the face (`vel · normal < 0`) to transit.
/// A teleported body is placed clear of the exit portal so it does not
/// re-enter at once.
pub fn portal_teleport_ground_items(
    portals: Query<&PlacedPortal>,
    mut items: Query<&mut PortalTransitable>,
    // The session's map convention, from the resource that owns it.
    tuning: Res<crate::tuning::PortalTuning>,
) {
    let convention = tuning.convention.map_convention();
    // Sort here: downstream loops take the first match, and `Query` order is
    // archetype order, which a rollback resimulation may not reproduce.
    let mut all: Vec<PlacedPortal> = portals.iter().cloned().collect();
    all.sort_by(crate::stable_portal_order);
    if all.is_empty() {
        return;
    }
    for mut item in &mut items {
        if item.vel == Vec2::ZERO {
            continue;
        }
        let item_aabb = ae::Aabb::new(item.pos, item.half_extent);
        for enter in &all {
            let Some(exit) = find_portal(&all, enter.channel.partner()) else {
                continue;
            };
            if item.vel.dot(enter.normal) < 0.0
                && item_aabb.strict_intersects(ae::Aabb::new(enter.pos, enter.half_extent))
            {
                // Rotation preserves speed, so momentum carries through.
                item.vel =
                    portal_transform_velocity(item.vel, enter.normal, exit.normal, convention);
                let clearance = portal_exit_clearance(item.half_extent, exit.normal);
                item.pos = exit.pos + exit.normal * clearance;
                break;
            }
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
        };

        assert_eq!(portal_sweep_sample(&kin, Some(&sweep)), None);
    }
}
