//! Portal-specific transit systems: drive the player, non-player actors, and
//! in-flight items through a placed portal pair via the shared
//! [`super::placement::transit_step`] aperture machine, plus the carve / input /
//! ability-suppression guards that make a crossing feel right.

use bevy::prelude::*;

use crate::engine_core::{self as ae, AabbExt};
use crate::platformer_runtime::body::BodyKinematics;
use crate::platformer_runtime::orientation::ActorRoll;
use crate::platformer_runtime::transit::rotate_velocity_between_normals as portal_transform_velocity;
use crate::portal::pieces as pp;

use super::color::PortalChannel;
use super::placement::{transit_step, TransitStep};
use super::types::{
    find_portal, portal_exit_clearance, PlacedPortal, PortalTransitCooldown, TELEPORT_COOLDOWN_S,
};

/// Semantic transit message: a body's authoritative position just snapped to a
/// portal's exit (the centroid crossed). Replaces the old one-frame
/// `IntentionalTeleport` resource flag — consumers (the gameplay trace
/// position-delta detector) read this instead of polling a shared mutable flag,
/// so portal transit no longer owns trace simulation state. Carries the
/// teleported entity so a consumer can scope to a specific body (e.g. the
/// primary player).
#[derive(Message, Clone, Copy, Debug)]
pub struct BodyTeleported {
    /// The body whose position snapped to a portal exit this frame.
    pub body: Entity,
}

/// Content-agnostic movement intent the portal transit reads in place of the
/// Ambition `ControlFrame`. The content/input layer syncs this from its own
/// input each frame BEFORE transit runs (see `crate::ambition_content::portal`),
/// so portal core never imports the Ambition input type. Holds the primary
/// player's current held movement direction (raw, un-warped): transit uses it as
/// the anchor for the same-wall held-input warp.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PlayerMovementIntent {
    /// Raw held movement direction this frame (x = horizontal, y = vertical;
    /// `ZERO` when the player isn't pushing a direction).
    pub dir: Vec2,
}

/// Per-player transit state: the aperture latch / centroid-crossing machine
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
/// the geometry here; an Ambition bridge
/// (`crate::ambition_content::portal::bridge_portal_carves`) copies it into the
/// host's `FeatureEcsWorldOverlay.portal_carves` each frame, ordered identically,
/// so the collision world sees the same carves the same frame. Portal core thus
/// never names `FeatureEcsWorldOverlay` — it owns the carve geometry, Ambition
/// owns how a carve alters its collision representation.
#[derive(Resource, Clone, Debug, Default)]
pub struct PortalCarves {
    /// Aperture rectangles to carve this frame, in publish order.
    pub holes: Vec<ae::Aabb>,
}

/// Carve placed-portal apertures out of the host surface — but ONLY a portal a
/// transiting body currently occupies (its `PortalTransit.straddling`), so the
/// opening exists exactly while a body is passing through and re-seals the
/// instant it clears. A permanently-carved portal left a walk-in pocket in the
/// host wall (you could wiggle into the solid wall / ledge-grab the carved
/// edges); gating the carve on active transit closes that. Pair-gated — a lone
/// portal never carves.
///
/// Writes the carve geometry into the portal-owned [`PortalCarves`] resource (not
/// the host overlay); the Ambition bridge copies it into the collision overlay.
pub fn publish_portal_carves(
    portals: Query<&PlacedPortal>,
    transits: Query<&PortalTransit>,
    mut carves: ResMut<PortalCarves>,
) {
    carves.holes.clear();
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    // Carve each portal a body is actively transiting (deduped), but only if its
    // pair partner is placed — a lone portal must never open a bottomless hole.
    let mut carved: Vec<PortalChannel> = Vec::new();
    for t in &transits {
        if carved.contains(&t.straddling) {
            continue;
        }
        let Some(enter) = find_portal(&all, t.straddling) else {
            continue;
        };
        if find_portal(&all, t.straddling.partner()).is_none() {
            continue;
        }
        carves.holes.push(pp::carve_hole(&enter.frame()));
        carved.push(t.straddling);
    }
}

/// Marker: opts an entity into the one generic portal-transit algorithm
/// ([`portal_transit`]). Any body carrying [`BodyKinematics`] + this marker +
/// a [`PortalPolicy`] sinks into a carved aperture and transfers when its
/// centroid crosses, exactly like the player. Ambition adds it (and the policy)
/// to the entities that should transit — see
/// `crate::ambition_content::portal::ensure_portal_bodies`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PortalBody;

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
/// (`crate::ambition_content::portal::portal_player_input_adapter`) reads this
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
    /// True for a same-wall turn-around (the warp stays horizontally
    /// expressible); the input adapter only warps held input in this case.
    pub facing_flip: bool,
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
        ),
        With<PortalBody>,
    >,
    gravity: Option<Res<crate::platformer_runtime::gravity::GravityField>>,
    mut sfx: MessageWriter<ambition_sfx::SfxMessage>,
    mut transited: MessageWriter<PortalBodyTransited>,
) {
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    if all.is_empty() {
        return;
    }
    let gravity_dir = gravity.map_or(Vec2::new(0.0, 1.0), |g| g.dir);

    for (entity, mut kin, policy, mut transit, mut roll, cooldown) in &mut bodies {
        // The transit cooldown is a BODY latch (`PortalTransitCooldown`), ticked
        // by `tick_portal_cooldowns`; gun-independent so nothing can ping-pong
        // back through an authored pair.
        let cooldown_now = cooldown.map_or(0.0, |c| c.0);
        let step = transit_step(
            kin.pos,
            kin.size,
            kin.vel,
            transit.as_deref().copied(),
            cooldown_now,
            &all,
            gravity_dir,
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
                sfx.write(ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PORTAL_ENTER,
                    pos: portal_pos,
                });
            }
            TransitStep::Transfer {
                pos,
                vel,
                roll_delta,
                facing_flip,
                enter_normal,
                exit_normal,
                exit_channel,
                exit_pos,
            } => {
                kin.pos = pos;
                // Velocity rotation is core/default; the policy only chooses
                // whether to WRITE it (false = old boss no-velocity path).
                if policy.carry_velocity {
                    kin.vel = vel;
                }
                // Re-orientation is the optional part: flip facing to the exit
                // aperture on a same-wall turn-around, only if the policy asks.
                if policy.reorient && facing_flip {
                    kin.facing = -kin.facing;
                }
                if let Some(roll) = roll.as_deref_mut() {
                    roll.angle += roll_delta;
                }
                // Latch the body's transit cooldown so it can't ping-pong back
                // through the pair it just crossed — gun-independent.
                commands
                    .entity(entity)
                    .insert(PortalTransitCooldown(TELEPORT_COOLDOWN_S));
                if let Some(t) = transit.as_deref_mut() {
                    t.crossed = true;
                    t.straddling = exit_channel;
                }
                // The trace message + player-input bits (`PortalEmission`,
                // `PortalInputWarp`) are emitted by the Ambition player-input
                // adapter from this event — input/trace are not core concerns.
                transited.write(PortalBodyTransited {
                    body: entity,
                    enter_normal,
                    exit_normal,
                    facing_flip,
                    exit_pos,
                });
                sfx.write(ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PORTAL_EXIT,
                    pos: exit_pos,
                });
                bevy::log::info!(target: "ambition::portal", "transferred through the portal pair");
            }
            TransitStep::Clear => {
                commands.entity(entity).remove::<PortalTransit>();
            }
        }
    }
}

/// The input-layer fix for the same-wall ping-pong: after a portal crossing the
/// player's HELD movement input is warped by the same portal map as velocity, so
/// holding "right" into a left-facing pair keeps carrying you LEFT out the exit
/// instead of instantly fighting the warped velocity and pulling you back through.
/// Only set for the wall↔wall turn-around (where the warp stays horizontally
/// expressible). Soft, not a hard latch — see [`warp_portal_input`].
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
/// `crate::ambition_content::portal`). Resting bodies (`vel == ZERO`) are
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

/// In-flight transitable bodies (thrown axes / javelins) also travel through the
/// portal pair, carrying momentum through the rotation — throw a javelin into
/// the blue portal and it flies out of the orange one. Resting bodies are
/// ignored (only `vel != ZERO` bodies teleport), and a teleported body pops out
/// clear of the exit portal so it doesn't immediately re-enter.
///
/// Operates on the content-agnostic [`PortalTransitable`] component, not the
/// Ambition `GroundItem`: the item layer syncs its body into/out of this marker
/// around transit, so portal core teleports any transitable body.
pub fn portal_teleport_ground_items(
    portals: Query<&PlacedPortal>,
    mut items: Query<&mut PortalTransitable>,
) {
    use super::color::PortalGunColor;
    let blue = portals
        .iter()
        .find(|p| p.channel == PortalGunColor::Blue.channel())
        .copied();
    let orange = portals
        .iter()
        .find(|p| p.channel == PortalGunColor::Orange.channel())
        .copied();
    let (Some(blue), Some(orange)) = (blue, orange) else {
        return;
    };
    for mut item in &mut items {
        if item.vel == Vec2::ZERO {
            continue;
        }
        let item_aabb = ae::Aabb::new(item.pos, item.half_extent);
        for (enter, exit) in [(blue, orange), (orange, blue)] {
            if item_aabb.strict_intersects(ae::Aabb::new(enter.pos, enter.half_extent)) {
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
    time: Res<crate::WorldTime>,
    mut commands: Commands,
    mut cooldowns: Query<(Entity, &mut PortalTransitCooldown)>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (entity, mut cooldown) in &mut cooldowns {
        cooldown.0 -= dt;
        if cooldown.0 <= 0.0 {
            commands.entity(entity).remove::<PortalTransitCooldown>();
        }
    }
}
