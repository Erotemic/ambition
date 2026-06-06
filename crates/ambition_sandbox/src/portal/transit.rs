//! Portal-specific transit systems: drive the player, non-player actors, and
//! in-flight items through a placed portal pair via the shared
//! [`super::placement::transit_step`] aperture machine, plus the carve / input /
//! ability-suppression guards that make a crossing feel right.

use bevy::prelude::*;

use crate::engine_core::{self as ae, AabbExt};
use crate::input::ControlFrame;
use crate::platformer_runtime::orientation::ActorRoll;
use crate::platformer_runtime::transit::rotate_velocity_between_normals as portal_transform_velocity;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};
use crate::portal_pieces as pp;

use super::color::PortalColor;
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

/// Per-player transit state: the aperture latch / centroid-crossing machine
/// that replaces "touch = teleport". A body is mid-transit while any part of it
/// straddles a portal plane; the authoritative body transfers to the exit when
/// the CENTROID crosses, and transit ends (re-arming after a clear) once the
/// body fully clears the plane.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalTransit {
    /// Color of the portal whose plane the body currently straddles — the entry
    /// before the centroid crosses, the exit after.
    pub straddling: PortalColor,
    /// True once the centroid crossed the entry plane (authoritative body now
    /// on the exit side).
    pub crossed: bool,
}

/// Carve placed-portal apertures out of the host surface — but ONLY a portal a
/// transiting body currently occupies (its `PortalTransit.straddling`), so the
/// opening exists exactly while a body is passing through and re-seals the
/// instant it clears. A permanently-carved portal left a walk-in pocket in the
/// host wall (you could wiggle into the solid wall / ledge-grab the carved
/// edges); gating the carve on active transit closes that. Pair-gated — a lone
/// portal never carves.
pub fn publish_portal_carves(
    portals: Query<&PlacedPortal>,
    transits: Query<&PortalTransit>,
    mut overlay: ResMut<crate::features::FeatureEcsWorldOverlay>,
) {
    overlay.portal_carves.clear();
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    // Carve each portal a body is actively transiting (deduped), but only if its
    // pair partner is placed — a lone portal must never open a bottomless hole.
    let mut carved: Vec<PortalColor> = Vec::new();
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
        overlay.portal_carves.push(pp::carve_hole(&enter.frame()));
        carved.push(t.straddling);
    }
}

/// Runtime toggle for [`suppress_ledge_grab_during_transit`]. Default ON; flip it
/// off to play with ledge-grab / wall-movement INTO portals enabled (the
/// "ledge-grab through a portal" experiment — see TODO.md). Toggleable at runtime
/// (e.g. via the inspector) so both behaviors can be tried without a recompile.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SuppressWallAbilitiesInPortal(pub bool);

impl Default for SuppressWallAbilitiesInPortal {
    fn default() -> Self {
        Self(true)
    }
}

/// While the player is mid-transit, suppress the wall abilities (ledge-grab,
/// cling, wall-jump, wall-climb) so they don't latch onto the carved aperture
/// EDGES — the carve splits the host block, and those new edges read as grabbable
/// ledges / climbable walls, so you'd cling "into" a portal and pop back out the
/// entry instead of sinking through and crossing.
///
/// IMPORTANT — this must re-apply EVERY frame, not set-once. `PlayerAbilities` is
/// wholesale-reset to the editable loadout every frame
/// (`sync_live_ability_edits_clusters`: `abilities.abilities = desired`), so a
/// save-once/restore-on-exit pattern is clobbered after a single frame (that was
/// the "disable didn't work" bug). Re-applying each frame is robust against that
/// reset, AND needs no save/restore — when transit ends, the per-frame reset
/// restores the loadout automatically. (The wider structural smell — transient
/// ability mods fighting a per-frame wholesale reset — is noted in TODO.md.)
/// Gated on [`SuppressWallAbilitiesInPortal`]. Runs before the movement integration.
pub fn suppress_ledge_grab_during_transit(
    toggle: Res<SuppressWallAbilitiesInPortal>,
    mut players: Query<
        (&mut crate::player::PlayerAbilities, Option<&PortalTransit>),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
) {
    if !toggle.0 {
        return;
    }
    for (mut abilities, transiting) in &mut players {
        if transiting.is_some() {
            let a = &mut abilities.abilities;
            a.ledge_grab = false;
            a.wall_cling = false;
            a.wall_jump = false;
            a.wall_climb = false;
        }
    }
}

/// Drive the PLAYER through a portal as an **aperture**, not a trigger, via the
/// shared [`transit_step`] machine: the body physically sinks into the carved
/// opening (the movement integrator does that), transfers when the centroid
/// crosses (carrying momentum + a somersault roll), and clears on trailing-edge
/// out. The transfer's position snap emits a [`BodyTeleported`] message so the
/// trace detector treats it as intentional and doesn't auto-dump on it.
///
/// Transiting an existing placed portal pair is INDEPENDENT of holding the
/// [`PortalGun`](super::gun::PortalGun) — the gun creates/replaces portals, but once a pair exists any
/// body crosses it (the authored test lab already proves this for gate pairs;
/// here it extends to player-placed pairs). The anti-ping-pong cooldown lives on
/// the BODY ([`PortalTransitCooldown`]), not on the gun.
pub fn portal_transit_system(
    control: Option<Res<ControlFrame>>,
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &mut PlayerKinematics,
            Option<&mut PortalTransit>,
            Option<&mut ActorRoll>,
            Option<&PortalTransitCooldown>,
        ),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    portals: Query<&PlacedPortal>,
    gravity: Option<Res<crate::physics::GravityField>>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut teleported: MessageWriter<BodyTeleported>,
) {
    let Ok((entity, mut kin, mut transit, mut roll, cooldown)) = players.single_mut() else {
        return;
    };
    // The transit cooldown is a BODY latch (`PortalTransitCooldown`), ticked by
    // `tick_portal_cooldowns`; gun-independent so a gun-less player can't
    // ping-pong through an authored pair.
    let cooldown_now = cooldown.map_or(0.0, |c| c.0);
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    let gravity_dir = gravity.map_or(Vec2::new(0.0, 1.0), |g| g.dir);
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
        TransitStep::Begin { color, portal_pos } => {
            commands.entity(entity).insert(PortalTransit {
                straddling: color,
                crossed: false,
            });
            sfx.write(crate::audio::SfxMessage::Play {
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
            exit_color,
            exit_pos,
        } => {
            kin.pos = pos;
            kin.vel = vel;
            if facing_flip {
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
            // Protect the emergence: for a short window the held input can't push
            // back INTO the exit wall (so physics carries the body out — Jon's
            // "don't let input cancel the portal emission").
            commands.entity(entity).insert(PortalEmission {
                exit_normal,
                timer: PORTAL_EMISSION_TIME,
            });
            // Warp the held input ONLY when the warped direction stays
            // horizontally expressible — i.e. the same-wall turn-around
            // (`facing_flip`). For a floor↔wall 90° turn the warp would rotate a
            // horizontal hold into "up", which the controller can't use, so we
            // skip it and let the emission guard + physics do the work.
            let held = control
                .as_deref()
                .map_or(Vec2::ZERO, |c| Vec2::new(c.axis_x, c.axis_y));
            if facing_flip && held.length() > PORTAL_INPUT_HELD_EPS {
                commands.entity(entity).insert(PortalInputWarp {
                    n_in: enter_normal,
                    n_out: exit_normal,
                    anchor: held,
                });
            }
            if let Some(t) = transit.as_deref_mut() {
                t.crossed = true;
                t.straddling = exit_color;
            }
            teleported.write(BodyTeleported { body: entity });
            sfx.write(crate::audio::SfxMessage::Play {
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

/// Movement-axis magnitude above which input counts as "held" (stick deadzone).
const PORTAL_INPUT_HELD_EPS: f32 = 0.25;
/// While warped, the live raw input may drift this far (cosine) from the anchor
/// before it counts as a "clearly different" direction that drops the warp.
const PORTAL_INPUT_WARP_KEEP_COS: f32 = 0.5;

/// Seconds the [`PortalEmission`] guard protects a fresh exit. Long enough for
/// the floored exit velocity to carry the body clear of the opening.
const PORTAL_EMISSION_TIME: f32 = 0.18;

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

/// Short-lived guard set on every crossing: for [`PORTAL_EMISSION_TIME`] the held
/// movement input cannot push back INTO the exit wall (against `exit_normal`), so
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

/// Apply the active portal input effects at the input layer (so the player brain
/// / movement see the adjusted `ControlFrame`): the same-wall held-input warp
/// (soft — drops on release or a clearly different direction) and the emergence
/// guard (held input can't push back into the exit wall while it's fresh). Both
/// are deliberately mild so portals never feel like a hard input latch.
pub fn warp_portal_input(
    time: Option<Res<crate::WorldTime>>,
    mut commands: Commands,
    mut control: ResMut<ControlFrame>,
    mut player: Query<
        (
            Entity,
            Option<&PortalInputWarp>,
            Option<&mut PortalEmission>,
        ),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
) {
    let Ok((entity, warp, emission)) = player.single_mut() else {
        return;
    };

    // --- Same-wall held-input warp ---
    if let Some(warp) = warp {
        let raw = Vec2::new(control.axis_x, control.axis_y);
        if raw.length() < PORTAL_INPUT_HELD_EPS {
            commands.entity(entity).remove::<PortalInputWarp>();
        } else if warp.anchor.length() > 0.01
            && raw.normalize_or_zero().dot(warp.anchor.normalize_or_zero())
                < PORTAL_INPUT_WARP_KEEP_COS
        {
            commands.entity(entity).remove::<PortalInputWarp>();
        } else {
            let warped = pp::portal_map_vec(raw, warp.n_in, warp.n_out);
            control.axis_x = warped.x;
            control.axis_y = warped.y;
        }
    }

    // --- Emergence guard: strip any held input that pushes back into the wall ---
    if let Some(mut emission) = emission {
        emission.timer -= time.as_deref().map_or(0.0, |t| t.sim_dt());
        if emission.timer <= 0.0 {
            commands.entity(entity).remove::<PortalEmission>();
        } else {
            let raw = Vec2::new(control.axis_x, control.axis_y);
            let into = raw.dot(emission.exit_normal); // < 0 = pushing into the wall
            if into < 0.0 {
                let kept = raw - into * emission.exit_normal;
                control.axis_x = kept.x;
                control.axis_y = kept.y;
            }
        }
    }
}

/// In-flight ground items (thrown axes / javelins) also travel through the
/// portal pair, carrying momentum through the rotation — throw a javelin into
/// the blue portal and it flies out of the orange one. Resting items are
/// ignored (only `vel != ZERO` items teleport), and a teleported item pops out
/// clear of the exit portal so it doesn't immediately re-enter.
pub fn portal_teleport_ground_items(
    portals: Query<&PlacedPortal>,
    mut items: Query<&mut crate::item_pickup::GroundItem>,
) {
    let blue = portals
        .iter()
        .find(|p| p.color == PortalColor::Blue)
        .copied();
    let orange = portals
        .iter()
        .find(|p| p.color == PortalColor::Orange)
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

/// Teleport non-player actors (enemies / NPCs / bosses) through the portal
/// pair, **size-gated** so only actors that fit the opening pass. Enemies / NPCs
/// (`ActorKinematics`) carry their momentum through the rotation; bosses
/// (`BossKinematics`, no velocity field) are repositioned out the exit. A short
/// [`PortalTransitCooldown`] after each jump prevents ping-pong.
/// Send EVERY non-player actor (enemies / NPCs via `ActorKinematics`, bosses via
/// `BossKinematics`) through a portal with the SAME aperture / centroid-crossing
/// machine the player uses ([`transit_step`]) — the unification: a goblin or a
/// boss now sinks into the carved opening and transfers when its centroid
/// crosses, carrying momentum + a somersault roll, instead of instant-popping
/// out the far side. Size-gated (big bosses can't fit a small opening) and
/// latched by [`PortalTransitCooldown`] against ping-pong.
pub fn portal_transit_actors(
    mut commands: Commands,
    portals: Query<&PlacedPortal>,
    mut actors: Query<
        (
            Entity,
            &mut crate::features::ActorKinematics,
            Option<&mut PortalTransit>,
            Option<&mut ActorRoll>,
            Option<&PortalTransitCooldown>,
        ),
        // Exclude bosses so this query and the boss query below have disjoint
        // entity sets — both take `&mut ActorRoll` / `&mut PortalTransit`, so
        // Bevy needs them proven non-overlapping.
        Without<crate::features::BossKinematics>,
    >,
    mut bosses: Query<(
        Entity,
        &mut crate::features::BossKinematics,
        Option<&mut PortalTransit>,
        Option<&mut ActorRoll>,
        Option<&PortalTransitCooldown>,
    )>,
    gravity: Option<Res<crate::physics::GravityField>>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    if all.is_empty() {
        return;
    }
    let gravity_dir = gravity.map_or(Vec2::new(0.0, 1.0), |g| g.dir);

    for (entity, kin, mut transit, mut roll, cooldown) in &mut actors {
        let kin = kin.into_inner();
        let step = transit_step(
            kin.pos,
            kin.size,
            kin.vel,
            transit.as_deref().copied(),
            cooldown.map_or(0.0, |c| c.0),
            &all,
            gravity_dir,
        );
        apply_actor_transit(
            &mut commands,
            entity,
            &mut sfx,
            step,
            Some(&mut kin.pos),
            Some(&mut kin.vel),
            roll.as_deref_mut(),
            transit.as_deref_mut(),
        );
    }

    for (entity, kin, mut transit, mut roll, cooldown) in &mut bosses {
        // Bosses have no velocity field — feed ZERO and ignore the velocity the
        // step computes; they reposition + roll only.
        let kin = kin.into_inner();
        let step = transit_step(
            kin.pos,
            kin.size,
            Vec2::ZERO,
            transit.as_deref().copied(),
            cooldown.map_or(0.0, |c| c.0),
            &all,
            gravity_dir,
        );
        apply_actor_transit(
            &mut commands,
            entity,
            &mut sfx,
            step,
            Some(&mut kin.pos),
            None,
            roll.as_deref_mut(),
            transit.as_deref_mut(),
        );
    }
}

/// Apply a [`TransitStep`] to a non-player actor: the ECS-side of the shared
/// machine (insert / mutate / remove `PortalTransit`, latch `PortalTransitCooldown`,
/// move the body, somersault its roll, play sfx). `vel` is `None` for bodies
/// without a velocity field (bosses).
#[allow(clippy::too_many_arguments)]
fn apply_actor_transit(
    commands: &mut Commands,
    entity: Entity,
    sfx: &mut MessageWriter<crate::audio::SfxMessage>,
    step: TransitStep,
    pos: Option<&mut Vec2>,
    vel: Option<&mut Vec2>,
    roll: Option<&mut ActorRoll>,
    transit: Option<&mut PortalTransit>,
) {
    match step {
        TransitStep::Idle | TransitStep::Continue => {}
        TransitStep::Begin { color, portal_pos } => {
            commands.entity(entity).insert(PortalTransit {
                straddling: color,
                crossed: false,
            });
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_ENTER,
                pos: portal_pos,
            });
        }
        // Non-player actors have no held input to warp, so `warp_rot` is ignored.
        // (`facing_flip` is too — actor facing follows their own AI each tick.)
        TransitStep::Transfer {
            pos: new_pos,
            vel: new_vel,
            roll_delta,
            exit_color,
            exit_pos,
            ..
        } => {
            if let Some(pos) = pos {
                *pos = new_pos;
            }
            if let Some(vel) = vel {
                *vel = new_vel;
            }
            if let Some(roll) = roll {
                roll.angle += roll_delta;
            }
            if let Some(t) = transit {
                t.crossed = true;
                t.straddling = exit_color;
            }
            commands
                .entity(entity)
                .insert(PortalTransitCooldown(TELEPORT_COOLDOWN_S));
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_EXIT,
                pos: exit_pos,
            });
        }
        TransitStep::Clear => {
            commands.entity(entity).remove::<PortalTransit>();
        }
    }
}
