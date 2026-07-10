//! Portal-specific transit systems: drive opted-in actors and in-flight items
//! through a placed portal pair via the shared
//! [`super::placement::transit_step`] aperture machine, plus the carve / input /
//! ability-suppression guards that make a crossing feel right.

use bevy::prelude::*;

use crate::pieces as pp;
use ambition_engine_core::{self as ae, AabbExt};
use ambition_platformer_primitives::body::BodyKinematics;
use ambition_platformer_primitives::class_b::{ClassBRemap, ClassBRemapLog};
use ambition_platformer_primitives::orientation::ActorRoll;
use ambition_platformer_primitives::transit::rotate_velocity_between_normals as portal_transform_velocity;

use super::color::PortalChannel;
use super::placement::{transit_step_with_tuning, SweptSample, TransitStep, TRANSIT_BEGIN_MARGIN};
use super::tuning::PortalTuning;
use super::types::{
    find_portal, portal_exit_clearance, PlacedPortal, PortalHostDepths, PortalTransitCooldown,
};

/// Semantic transit message: a body's authoritative position just snapped to a
/// portal's exit (the centroid crossed). Replaces the old one-frame
/// `IntentionalTeleport` resource flag — consumers (the gameplay trace
/// position-delta detector) read this instead of polling a shared mutable flag,
/// so portal transit no longer owns trace simulation state. Carries the
/// teleported entity so a consumer can scope to a specific body (e.g. the
/// locally focused body).
#[derive(Message, Clone, Copy, Debug)]
pub struct BodyTeleported {
    /// The body whose position snapped to a portal exit this frame.
    pub body: Entity,
}

/// Content-agnostic movement intent the portal transit reads in place of a
/// concrete host input frame. The host input layer syncs this from its own
/// input each frame BEFORE transit runs, so portal core never imports that input
/// type. Holds the focused actor's current held movement direction (raw,
/// un-warped): transit uses it as the anchor for same-wall held-input warp.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PlayerMovementIntent {
    /// Raw held movement direction this frame (x = horizontal, y = vertical;
    /// `ZERO` when no movement direction is held.
    pub dir: Vec2,
}

/// Per-body transit state: the aperture latch / centroid-crossing machine
/// that replaces "touch = teleport". A body is mid-transit while any part of it
/// straddles a portal plane; the authoritative body transfers to the exit when
/// the CENTROID crosses, and transit ends (re-arming after a clear) once the
/// body fully clears the plane.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalTransit {
    /// Channel of the portal whose plane the body currently straddles — the entry
    /// before the centroid crosses, the exit after.
    pub straddling: PortalChannel,
    /// True once the centroid crossed the entry plane (authoritative body now
    /// on the exit side).
    pub crossed: bool,
}

/// Portal-owned output of [`publish_portal_carves`]: the aperture rectangles to
/// carve OUT of the host surface this frame, in publish order. Portal core writes
/// the geometry here; a host bridge copies it into the host collision overlay
/// each frame, ordered identically, so the collision world sees the same carves
/// the same frame. Portal core owns the carve geometry; the host owns how a
/// carve alters its collision representation.
#[derive(Resource, Clone, Debug, Default)]
pub struct PortalCarves {
    /// Aperture rectangles to carve this frame, in publish order.
    pub holes: Vec<ae::Aabb>,
}

/// Carve placed-portal apertures out of the host surface — but ONLY a portal an
/// opted-in body currently occupies, so the opening exists exactly while a body
/// is passing through and re-seals the instant it clears. A permanently-carved
/// portal left a walk-in pocket in the host wall (you could wiggle into the solid
/// wall / ledge-grab the carved edges); gating the carve on a present body closes
/// that. Pair-gated — a lone portal never carves.
///
/// A portal is carved when ANY of:
/// * **a [`PortalBody`] currently overlaps its capture opening** — the walk-in /
///   resting case: a body in the opening keeps it open, no velocity required.
/// * **a [`PortalBody`] is inside its [approach box](super::placement::approach_box)
///   AND moving into the portal** (`vel · normal < 0`). The approach box extends
///   a fixed `APPROACH_CARVE_REACH` outward of the face — deliberately
///   **dt-independent**. (Two prior schemes failed here: keying the carve off the
///   transit latch lagged one frame, and sweeping the body by `vel * dt` read a
///   STALE dt — this system runs `.before(CoreSimulation)` but the sim clock
///   refreshes inside it — and pre-gravity velocity, so a frame hitch at re-entry
///   under-swept, left the floor solid for one frame, and the integrator grounded
///   the body, killing its entry momentum. A fixed geometric reach sized to the
///   worst per-frame travel cannot be cheated by frame-time jitter.)
/// * **a body is mid-transit straddling it** ([`PortalTransit`]) — keeps the hole
///   open through the deep sink/cross even after the body's centroid has dropped
///   past the thin capture box.
///
/// Writes the carve geometry into the portal-owned [`PortalCarves`] resource (not
/// the host overlay); the Ambition bridge copies it into the collision overlay.
pub fn publish_portal_carves(
    portals: Query<&PlacedPortal>,
    bodies: Query<&BodyKinematics, With<PortalBody>>,
    transits: Query<&PortalTransit>,
    host_depths: Option<Res<PortalHostDepths>>,
    mut carves: ResMut<PortalCarves>,
) {
    use super::placement::{approach_box, capture_box, portal_fits};

    carves.holes.clear();
    let all: Vec<PlacedPortal> = portals.iter().cloned().collect();
    if all.is_empty() {
        return;
    }
    // Carve a channel once (deduped), and only if its pair partner is placed — a
    // lone portal must never open a bottomless hole.
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
            // FRONT-side engagement only: in the opening now (walk-in /
            // resting), or closing in fast enough that this frame's
            // integration may cross it. Without the front gate, a body
            // pressed against the BACK of a thin host wall reached the
            // capture box THROUGH the material and opened a hole it could
            // then walk through without ever transiting.
            let frontal = front >= -TRANSIT_BEGIN_MARGIN
                && (body.strict_intersects(capture_box(p))
                    || (kin.vel.dot(p.normal) < 0.0 && body.strict_intersects(approach_box(p))));
            // Mid-fall-through (a fast crossing that skipped Begin): keep the
            // hole open while the body is inside the aperture VOLUME — the
            // carve hole bounded by the measured host material, so the open
            // room behind a thin wall never counts.
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

/// Marker: opts an entity into the one generic portal-transit algorithm
/// ([`portal_transit`]). Any body carrying [`BodyKinematics`] + this marker +
/// a [`PortalPolicy`] sinks into a carved aperture and transfers when its
/// centroid crosses, exactly like the player. Ambition adds it (and the policy)
/// to the entities that should transit — see
/// the host portal adapter.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PortalBody;

/// Convert the canonical movement-kernel sample into the portal crate's swept
/// transit input. The sample is only valid for portal CCD when its recorded
/// post-sim endpoint still matches the body's live position at this system: if
/// some earlier post-sim system teleported the body, the movement record remains
/// correct, but it is no longer the segment ending at `kin.pos` and must not be
/// interpreted as travel through an aperture.
fn portal_sweep_sample(
    kin: &BodyKinematics,
    sweep: Option<&ae::SweepSample>,
) -> Option<SweptSample> {
    let sweep = sweep?;
    if (sweep.curr - kin.pos).length_squared() > 1.0 {
        return None;
    }
    Some(SweptSample {
        pos: sweep.prev,
        vel: sweep.vel,
    })
}

/// HOW a body participates in transit — behavioral, never identity. The core
/// transit reads only these flags; it never names Player / Boss / Projectile.
/// Ambition maps its game identities → policy when it tags an entity.
///
/// **Velocity rotation is core/default** (it lives in [`transit_step`]'s `vel`
/// output) — this only chooses whether to *write* that rotated velocity and
/// whether to re-orient the body's facing.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalPolicy {
    /// Flip the body's `facing` to the exit aperture on a same-wall turn-around
    /// (`facing_flip`). Players/enemies re-orient; a boss whose facing follows
    /// its AI does not.
    pub reorient: bool,
    /// Write the rotated exit velocity into the body. `false` is the old boss
    /// no-velocity path (the boss floats; its `vel` stays as the brain set it).
    pub carry_velocity: bool,
}

/// Emitted on every Transfer by the generic [`portal_transit`] core, carrying
/// what an input/trace adapter needs without the core touching input or trace
/// state. The Ambition player-input adapter
/// (the host portal adapter) reads this
/// and — for the player only — emits [`BodyTeleported`] and inserts the
/// `PortalEmission` / `PortalInputWarp` input bits.
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

/// The ONE generic transit algorithm: drive **any** [`PortalBody`] through a
/// portal as an **aperture**, not a trigger, via the shared [`transit_step`]
/// machine. The body physically sinks into the carved opening (the movement
/// integrator does that), transfers when the centroid crosses (carrying the
/// rotated momentum + a somersault roll per its [`PortalPolicy`]), and clears
/// on trailing-edge out.
///
/// This replaces the old player-specific `portal_transit_system` and the
/// actor-specific `portal_transit_actors`: a single query over every body that
/// opted in, with one `&mut BodyKinematics` (no self-conflict). Identity →
/// behavior is supplied entirely by [`PortalPolicy`]; the player-input bits and
/// the [`BodyTeleported`] trace message moved to the Ambition adapters that read
/// [`PortalBodyTransited`].
///
/// Transiting a placed pair is INDEPENDENT of holding the
/// [`PortalGun`](super::gun::PortalGun) — once a pair exists any opted-in body
/// crosses it. The anti-ping-pong cooldown lives on the BODY
/// ([`PortalTransitCooldown`]), not on the gun.
pub fn portal_transit(
    mut commands: Commands,
    portals: Query<&PlacedPortal>,
    mut bodies: Query<
        (
            Entity,
            &mut BodyKinematics,
            &PortalPolicy,
            Option<&mut PortalTransit>,
            Option<&mut ActorRoll>,
            Option<&PortalTransitCooldown>,
            Option<&ae::SweepSample>,
        ),
        With<PortalBody>,
    >,
    gravity: Option<Res<ambition_platformer_primitives::gravity::GravityField>>,
    tuning: Res<PortalTuning>,
    host_depths: Option<Res<PortalHostDepths>>,
    mut entered: MessageWriter<super::messages::PortalBodyEntered>,
    mut transited: MessageWriter<PortalBodyTransited>,
    // Optional: a minimal test app that never added the engine's schedule
    // plugin still runs transit. The ledger is diagnostic, never load-bearing.
    mut class_b: Option<ResMut<ClassBRemapLog>>,
) {
    let all: Vec<PlacedPortal> = portals.iter().cloned().collect();
    if all.is_empty() {
        return;
    }
    let gravity_dir =
        ambition_platformer_primitives::gravity::gravity_dir_or_default(gravity.as_deref());

    for (entity, mut kin, policy, mut transit, mut roll, cooldown, sweep) in &mut bodies {
        // The transit cooldown is a BODY latch (`PortalTransitCooldown`),
        // ticked by `tick_portal_cooldowns` and scoped to the PAIR the body
        // just crossed; gun-independent so nothing can ping-pong back through
        // an authored pair, while a different pair stays enterable.
        let cooldown_pair = cooldown.map(|c| c.pair);
        let default_depths = PortalHostDepths::default();
        // The swept (CCD) tier's segment start comes from the movement kernel's
        // §3.1 `SweepSample`: the TRUE sim-phase entry point and the velocity
        // that produced it. No portal-local anchor is maintained here; teleports
        // outside the sim phase never become swept travel because the sample is
        // used only when its `curr` still equals this body's live `kin.pos`.
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
                // The crate emits the ENTER signal; an Ambition audio adapter
                // plays the cue (the crate owns neither audio nor sfx ids).
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
                // Class-B transit authority (`collision-and-ccd.md` §3.2),
                // recorded at the moment the position is written — not when the
                // crossing is detected. The CC3 oracle reads this to tell a
                // legal aperture warp from a clip through solid geometry.
                if let Some(log) = class_b.as_mut() {
                    log.record(entity, ClassBRemap::PortalTransit);
                }
                // Velocity rotation is core/default; the policy only chooses
                // whether to WRITE it (false = old boss no-velocity path).
                if policy.carry_velocity {
                    kin.vel = vel;
                }
                // Re-orientation is the optional part: flip facing to the exit
                // aperture on a same-wall turn-around, only if the policy asks AND
                // the global `reorient_facing` knob is on (the Ambition
                // `portal_reverses_facing` gameplay setting mirrors into it).
                if policy.reorient && facing_flip && tuning.reorient_facing {
                    kin.facing = -kin.facing;
                }
                if let Some(roll) = roll.as_deref_mut() {
                    roll.angle += roll_delta;
                }
                // Latch the body's transit cooldown so it can't ping-pong back
                // through the pair it just crossed — gun-independent and
                // scoped to THIS pair (other pairs stay enterable).
                commands.entity(entity).insert(PortalTransitCooldown {
                    remaining: tuning.teleport_cooldown_s,
                    pair: exit_channel,
                });
                if let Some(t) = transit.as_deref_mut() {
                    t.crossed = true;
                    t.straddling = exit_channel;
                }
                // The trace message + player-input bits (`PortalEmission`,
                // `PortalInputWarp`) are emitted by the Ambition player-input
                // adapter from this event — input/trace are not core concerns.
                // The EXIT cue rides this event's `exit_pos` (an Ambition audio
                // adapter plays it); the trace message + player-input bits
                // (`PortalEmission`, `PortalInputWarp`) are likewise emitted by
                // the Ambition adapters from this event — audio/input/trace are
                // not core concerns.
                transited.write(PortalBodyTransited {
                    body: entity,
                    enter_normal,
                    exit_normal,
                    facing_flip,
                    input_warp,
                    exit_pos,
                });
                bevy::log::info!(target: "ambition::portal", "transferred through the portal pair");
            }
            TransitStep::Clear => {
                commands.entity(entity).remove::<PortalTransit>();
            }
        }
    }
}

/// The input-layer fix for portal ping-pong: after a portal crossing the
/// player's HELD movement input is warped by the same portal map as velocity
/// when that map keeps horizontal movement expressible. Soft, not a hard latch —
/// see the Ambition `warp_portal_input` adapter.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalInputWarp {
    /// Entry + exit portal normals — the held movement axis is mapped through the
    /// tangent-preserving portal map (so a horizontal hold mirrors horizontally
    /// and a vertical hold is left alone).
    pub n_in: Vec2,
    pub n_out: Vec2,
    /// Raw (un-warped) movement direction held when the warp was set; the warp
    /// drops once the live raw input releases or clearly diverges from this.
    pub anchor: Vec2,
}

/// Short-lived guard set on every crossing by the Ambition player-input adapter:
/// for a brief window the held movement input cannot push back INTO the exit wall
/// (against `exit_normal`), so
/// the floored exit velocity carries the body out instead of the input cancelling
/// the emergence. Gravity-general — it works off the exit normal vector, not a
/// hard-coded axis.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalEmission {
    /// Outward normal of the exit portal (the emergence direction).
    pub exit_normal: Vec2,
    /// Remaining protection time (s).
    pub timer: f32,
}

/// A free in-flight body that should travel through a portal pair (thrown axes /
/// javelins / any content-owned projectile). This is portal core's
/// content-agnostic transit body: it carries exactly the kinematics
/// [`portal_teleport_ground_items`] reads and writes (position, velocity,
/// half-extent), so portal core never names the Ambition `GroundItem`. The
/// content/item layer attaches this marker to its transitable bodies and keeps
/// it in sync with its own body component each frame (see
/// the host portal adapter). Resting bodies (`vel == ZERO`) are
/// ignored; a transited body pops out clear of the exit portal.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalTransitable {
    /// Authoritative world position of the body's center.
    pub pos: Vec2,
    /// Current velocity; `ZERO` means "resting", which never transits.
    pub vel: Vec2,
    /// Half-extent (AABB) used for the portal overlap test and exit clearance.
    pub half_extent: Vec2,
}

/// In-flight transitable bodies (thrown axes / javelins) also travel through
/// EVERY placed portal pair — gun-fired, authored, or link-authored — carrying
/// momentum through the rotation: throw a javelin into one end and it flies out
/// of the partner. Resting bodies are ignored (only `vel != ZERO` bodies
/// teleport), a body must be moving INTO the face (`vel · normal < 0`) to
/// transit — grazing past a portal parallel to its surface is not an entry —
/// and a teleported body pops out clear of the exit portal so it doesn't
/// immediately re-enter.
///
/// Operates on the content-agnostic [`PortalTransitable`] component, not the
/// Ambition `GroundItem`: the item layer syncs its body into/out of this marker
/// around transit, so portal core teleports any transitable body.
pub fn portal_teleport_ground_items(
    portals: Query<&PlacedPortal>,
    mut items: Query<&mut PortalTransitable>,
) {
    let all: Vec<PlacedPortal> = portals.iter().cloned().collect();
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
                item.vel = portal_transform_velocity(item.vel, enter.normal, exit.normal);
                let clearance = portal_exit_clearance(item.half_extent, exit.normal);
                item.pos = exit.pos + exit.normal * clearance;
                break;
            }
        }
    }
}

/// Tick down (and clear) per-actor [`PortalTransitCooldown`]s.
pub fn tick_portal_cooldowns(
    time: Res<ambition_platformer_primitives::time::SimDt>,
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
