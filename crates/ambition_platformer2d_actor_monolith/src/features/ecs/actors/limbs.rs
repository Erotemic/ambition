//! Translate a rider-boss strike into per-limb intents on its mount.
//!
//! Limb vocabulary and fan-out live in `ambition_characters::actor::limb`; this
//! module owns the mount-specific routing that reads mount kinematics and rider
//! attack state. It runs after host brain/mount steering and before body integration.

use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::actor::limb::{Limb, LimbIntents, LimbRig, LimbRouteState, LimbSlot};
use ambition_characters::brain::BossAttackState;
use ambition_platformer2d_core as ae;
use bevy::prelude::Query;

use ambition_mount::MountSlot;
use crate::features::{ActorSurfaceState, BodyKinematics};
use ambition_boss_encounter::BossConfig;
use ambition_boss_encounter::{LimbMotion, LimbRoute};

/// Idle station-keeping gain (1/s): how hard a limb steers back toward its home
/// anchor when it has no strike this tick. `velocity_target = (home - pos) * gain`.
const LIMB_STATION_GAIN: f32 = 10.0;
/// Windup lift speed (px/s) — a limb rising during a strike's Startup phase.
const LIMB_LIFT_SPEED: f32 = 320.0;
/// Overhead-slam speed (px/s) — a limb driving down during a `SlamDown` Active.
const LIMB_SLAM_SPEED: f32 = 640.0;
/// Lateral sweep speed (px/s) — the facing-side hand during a `SweepAcross`.
const LIMB_SWEEP_SPEED: f32 = 520.0;

/// Which phase of a strike the limb router is projecting this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LimbPhase {
    /// The rider's move is in its telegraph window (windup — no hitbox yet).
    Startup,
    /// The rider's move is in its Active (strike) window.
    Active,
}

/// The rider's live routed strike, resolved once per mount from the rider's
/// `BossAttackState` + its profile's `limb_routing`.
struct ActiveLimbRoute {
    move_id: String,
    motion: LimbMotion,
    phase: LimbPhase,
    slots: Vec<LimbSlot>,
}

impl ActiveLimbRoute {
    /// Does this route drive `slot` THIS tick? `SweepAcross` engages only the
    /// host's facing-side hand (deterministic from facing, Q18); every other
    /// motion drives all named slots.
    fn engages(&self, slot: LimbSlot, facing: f32) -> bool {
        if !self.slots.contains(&slot) || self.motion == LimbMotion::Hold {
            return false;
        }
        match self.motion {
            LimbMotion::SweepAcross => slot == facing_side_slot(facing),
            _ => true,
        }
    }
}

/// The hand on the host's facing side (`+` / rightward  right hand).
fn facing_side_slot(facing: f32) -> LimbSlot {
    if facing >= 0.0 {
        LimbSlot::HAND_RIGHT
    } else {
        LimbSlot::HAND_LEFT
    }
}

/// Resolve the rider's ACTIVE routed strike (Active takes priority over the
/// telegraph Startup), or `None` when the rider isn't striking anything the
/// profile routes to limbs.
fn resolve_active_route(state: &BossAttackState, cfg: &BossConfig) -> Option<ActiveLimbRoute> {
    let (profile, phase) = if let Some(p) = &state.active_profile {
        (p, LimbPhase::Active)
    } else if let Some(p) = &state.telegraph_profile {
        (p, LimbPhase::Startup)
    } else {
        return None;
    };
    let move_id = profile.move_id();
    let route: &LimbRoute = cfg
        .behavior
        .limb_routing
        .iter()
        .find(|(key, _)| key == &move_id)
        .map(|(_, route)| route)?;
    Some(ActiveLimbRoute {
        move_id,
        motion: route.motion,
        phase,
        slots: route.slots.clone(),
    })
}

/// The per-limb control frame for an ENGAGED strike limb — a `velocity_target`
/// arc for `motion`/`phase`, plus a single `melee_pressed` edge at Active onset.
fn strike_frame(
    motion: LimbMotion,
    phase: LimbPhase,
    onset: bool,
    gravity_dir: ae::Vec2,
    facing: f32,
) -> ActorControlFrame {
    let down = gravity_dir;
    let up = -gravity_dir;
    let side = ae::Vec2::new(if facing >= 0.0 { 1.0 } else { -1.0 }, 0.0);
    let (velocity_target, striking) = match (motion, phase) {
        (LimbMotion::SlamDown, LimbPhase::Startup) => (up * LIMB_LIFT_SPEED, false),
        (LimbMotion::SlamDown, LimbPhase::Active) => (down * LIMB_SLAM_SPEED, true),
        (LimbMotion::Raise, LimbPhase::Startup) => (up * LIMB_LIFT_SPEED, false),
        (LimbMotion::Raise, LimbPhase::Active) => (up * LIMB_LIFT_SPEED, true),
        (LimbMotion::SweepAcross, LimbPhase::Startup) => (up * (LIMB_LIFT_SPEED * 0.4), false),
        (LimbMotion::SweepAcross, LimbPhase::Active) => (side * LIMB_SWEEP_SPEED, true),
        // Hold never reaches here (filtered by `engages`), but stay total.
        (LimbMotion::Hold, _) => (ae::Vec2::ZERO, false),
    };
    let mut frame = ActorControlFrame::neutral();
    frame.velocity_target = ae::WorldVec2(velocity_target);
    frame.facing = facing;
    frame.melee_pressed = striking && onset;
    frame
}

/// The per-limb HOLD-STATION frame — steer the limb's `velocity_target` toward
/// its home anchor in the host's gravity frame (Q18 idle pose source).
fn station_frame(
    limb: &Limb,
    host_kin: &BodyKinematics,
    limb_kin: &BodyKinematics,
    gravity_dir: ae::Vec2,
) -> ActorControlFrame {
    // Rotate the host-local home offset into world through the gravity frame:
    // `down` = gravity_dir, `right` = perpendicular (identity under down-gravity).
    let down = gravity_dir;
    let right = ae::Vec2::new(down.y, -down.x);
    let home_world = host_kin.pos + right * limb.home_offset.x + down * limb.home_offset.y;
    let mut frame = ActorControlFrame::neutral();
    // `home_world` is world; the station command it produces is world.
    frame.velocity_target = ae::WorldVec2((home_world - limb_kin.pos) * LIMB_STATION_GAIN);
    frame
}

/// Q18 (G3): TRANSLATE a rider-boss's live strike into per-limb intents on its
/// linked mount. For each mount carrying a [`LimbRig`], bridge across
/// `MountSlot.rider` to read the RIDER's [`BossAttackState`] (the sim-owned
/// projection) + its profile's `limb_routing`, turn the ACTIVE strike's
/// [`LimbRoute`] into per-limb `velocity_target` arcs (+ a `melee_pressed` edge at
/// Active onset), and write them onto the mount's [`LimbIntents`].
/// [`fan_out_limb_intents`] then copies each slot's frame onto its limb body.
///
/// This is the Q18 split wrinkle: the fused-host spec assumes the attack state
/// and the limbs share one entity; here the state lives on the RIDER and the
/// limbs on the MOUNT, so the router crosses the `RidingOn`/`MountSlot` link. A
/// limb with no routed strike this tick (unrouted move, `Hold`, or no strike at
/// all) gets a hold-station frame toward its home anchor — never a stale arc.
/// `tick_boss_pattern` stays limb-ignorant: the brain keeps emitting ONE body's
/// frame, and THIS system is the only limb coordinator (which is what keeps the
/// player-piloted giant free later).
pub fn route_boss_strikes_to_limbs(
    mut mounts: Query<(
        &LimbRig,
        &BodyKinematics,
        &ActorSurfaceState,
        &MountSlot,
        &mut LimbIntents,
        &mut LimbRouteState,
    )>,
    riders: Query<(&BossAttackState, &BossConfig)>,
    limbs: Query<(&Limb, &BodyKinematics)>,
) {
    for (rig, host_kin, surface, slot, mut intents, mut route_state) in &mut mounts {
        intents.0.clear();

        // Gravity-down unit vector from the host's clung surface (floor normal
        // (0,-1) → down (0,1)); default straight down when the surface is unset.
        let gravity_dir = if surface.surface_normal.length_squared() > 1e-4 {
            (-surface.surface_normal).normalize()
        } else {
            ae::Vec2::new(0.0, 1.0)
        };

        // Bridge to the rider (Q18 split): its BossAttackState + limb_routing.
        let active = slot
            .rider
            .and_then(|rider| riders.get(rider).ok())
            .and_then(|(state, cfg)| resolve_active_route(state, cfg));

        // A routed STRIKE (Active phase) whose move id differs from last tick's is
        // an onset → one `melee_pressed` edge. Startup / no-strike clears the memo.
        let onset = route_state.begin_strike(
            active
                .as_ref()
                .filter(|r| r.phase == LimbPhase::Active)
                .map(|r| r.move_id.clone()),
        );

        for (&slot, &limb_entity) in &rig.limbs {
            let Ok((limb, limb_kin)) = limbs.get(limb_entity) else {
                continue; // despawned/unspawned limb: the rig tolerates gaps
            };
            let frame = match &active {
                Some(route) if route.engages(slot, host_kin.facing) => strike_frame(
                    route.motion,
                    route.phase,
                    onset,
                    gravity_dir,
                    host_kin.facing,
                ),
                _ => station_frame(limb, host_kin, limb_kin, gravity_dir),
            };
            intents.0.insert(slot, frame);
        }
    }
}
