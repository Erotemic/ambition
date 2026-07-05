//! The limb rig — driven limb bodies fanned out from ONE pilot intent
//! (fable review 2026-07-05, AJ12 / R10.1).
//!
//! Generalizes `steer_mount_from_rider` from 1→1 to 1→N: a HOST body (the
//! giant gnu; any mount or actor with articulated parts) carries a
//! [`LimbRig`] naming its limb bodies; the brain that drives the host writes
//! a per-limb intent table ([`LimbIntents`]); [`fan_out_limb_intents`] copies
//! each slot's frame onto that limb's `ActorControl`. Limbs are ORDINARY
//! actor bodies — `ActorControl` + `ActorMoveset`, **no `Brain`, no
//! `BossConfig`, no `BodyHealth`** — so integration, moveset triggering,
//! FollowOwner hitboxes, and damage attribution all pick them up unchanged
//! (the multi-limb draft's research finding: the downstream sim is already
//! N-body-safe; per-entity `MovePlayback` singletons are exactly WHY two
//! hands attacking at once are two entities).
//!
//! This is a MOUNT-level capability, not boss machinery: a mech with arms is
//! the same component set. The coordinator is whatever brain currently drives
//! the host — gnuton's scripted `BossPattern` through the ADR 0020
//! `ControlGrant`, or the player after possession (M5) — because the fan-out
//! reads only data on the host.
//!
//! Determinism: limbs fan out in `LimbRig::limbs` order (spawn order — a
//! stable id, never `Entity` iteration order); a slot with no intent this
//! tick gets an explicit NEUTRAL frame, so stale intents can't drift.
//!
//! Schedule contract (registration lands with the first production rig,
//! R10.3/R10.4): after the host's brain tick (which writes `LimbIntents`) and
//! `steer_mount_from_rider`, before `integrate_sim_bodies` — the same slot
//! the mount steer occupies.

use std::collections::BTreeMap;

use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::brain::{ActorControl, BossAttackState};
use ambition_engine_core as ae;
use bevy::prelude::{Component, Entity, Query};

use crate::boss_encounter::{LimbMotion, LimbRoute};
use crate::features::{ActorSurfaceState, BodyKinematics, BossConfig, MountSlot};

/// Which limb of the rig a body is. Grows per content (a serpent boss adds
/// variants); ordered so `LimbIntents`' BTreeMap iterates deterministically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LimbSlot {
    HandLeft,
    HandRight,
}

impl LimbSlot {
    /// Map an authored route slot name (`"hand_left"` / `"hand_right"`) onto a
    /// slot. Returns `None` for an unknown name so a route to a slot the rig
    /// doesn't carry is simply inert (Q18). Snake_case matches the RON authoring.
    pub fn from_route_str(name: &str) -> Option<LimbSlot> {
        match name {
            "hand_left" => Some(LimbSlot::HandLeft),
            "hand_right" => Some(LimbSlot::HandRight),
            _ => None,
        }
    }
}

/// On the HOST body: its driven limbs, in spawn order (the stable fan-out
/// order). The rig owns no behavior — it is a relationship, like `MountSlot`.
#[derive(Component, Default, Debug)]
pub struct LimbRig {
    pub limbs: Vec<Entity>,
}

/// On each LIMB body: which host it belongs to and which slot it fills.
#[derive(Component, Debug)]
pub struct Limb {
    pub of: Entity,
    pub slot: LimbSlot,
    /// Host-local (body-frame) idle anchor, in pixels. When the limb has no
    /// routed strike intent this tick, `route_boss_strikes_to_limbs` steers its
    /// `velocity_target` toward `host.pos + gravity_frame(home_offset)` — the
    /// idle pose source that replaces the deleted per-frame hand animation (Q18,
    /// station-keeping).
    pub home_offset: ae::Vec2,
}

/// On the HOST (mount) body carrying a [`LimbRig`]: the router's per-mount edge
/// memory. Holds the move id whose STRIKE currently drives limbs so a
/// `melee_pressed` edge fires exactly once — at the Active-window onset — instead
/// of every tick the strike is live (Q18: "a `melee_pressed` edge at Active
/// onset"). `None` when no routed strike is active.
#[derive(Component, Default, Debug)]
pub struct LimbRouteState {
    active_move: Option<String>,
}

/// On the HOST body: the per-limb intent table its driving brain writes each
/// tick (the boss pattern maps attack steps onto per-limb velocity targets +
/// attack edges here; a possessing player's verb map writes here via M5).
#[derive(Component, Default, Debug)]
pub struct LimbIntents(pub BTreeMap<LimbSlot, ActorControlFrame>);

/// Copy each rigged limb's intent onto its `ActorControl` — the 1→N sibling
/// of `steer_mount_from_rider`'s 1→1 copy. A slot with no intent this tick is
/// explicitly neutralized (no stale frames). Runs after the host brain tick,
/// before `integrate_sim_bodies`.
pub fn fan_out_limb_intents(
    hosts: Query<(&LimbRig, &LimbIntents)>,
    mut limbs: Query<(&Limb, &mut ActorControl)>,
) {
    for (rig, intents) in &hosts {
        for &limb_entity in &rig.limbs {
            let Ok((limb, mut control)) = limbs.get_mut(limb_entity) else {
                continue; // despawned/unspawned limb: the rig tolerates gaps
            };
            control.0 = intents
                .0
                .get(&limb.slot)
                .copied()
                .unwrap_or_else(ActorControlFrame::neutral);
        }
    }
}

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

/// The hand on the host's facing side (`+` / rightward ⇒ right hand).
fn facing_side_slot(facing: f32) -> LimbSlot {
    if facing >= 0.0 {
        LimbSlot::HandRight
    } else {
        LimbSlot::HandLeft
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
    let slots = route
        .slots
        .iter()
        .filter_map(|s| LimbSlot::from_route_str(s))
        .collect();
    Some(ActiveLimbRoute {
        move_id,
        motion: route.motion,
        phase,
        slots,
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
    frame.velocity_target = velocity_target;
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
    frame.velocity_target = (home_world - limb_kin.pos) * LIMB_STATION_GAIN;
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
        let active_strike_move = active
            .as_ref()
            .filter(|r| r.phase == LimbPhase::Active)
            .map(|r| r.move_id.clone());
        let onset = matches!(&active_strike_move, Some(m)
            if route_state.active_move.as_deref() != Some(m.as_str()));
        route_state.active_move = active_strike_move;

        for &limb_entity in &rig.limbs {
            let Ok((limb, limb_kin)) = limbs.get(limb_entity) else {
                continue; // despawned/unspawned limb: the rig tolerates gaps
            };
            let frame = match &active {
                Some(route) if route.engages(limb.slot, host_kin.facing) => strike_frame(
                    route.motion,
                    route.phase,
                    onset,
                    gravity_dir,
                    host_kin.facing,
                ),
                _ => station_frame(limb, host_kin, limb_kin, gravity_dir),
            };
            intents.0.insert(limb.slot, frame);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core as ae;
    use bevy::prelude::{App, Update};

    #[test]
    fn pilot_intents_fan_out_to_the_right_limbs_and_absent_slots_neutralize() {
        let mut app = App::new();
        app.add_systems(Update, fan_out_limb_intents);

        let host = app.world_mut().spawn_empty().id();
        let hand_l = app
            .world_mut()
            .spawn((
                Limb {
                    of: host,
                    slot: LimbSlot::HandLeft,
                    home_offset: ae::Vec2::ZERO,
                },
                ActorControl(ActorControlFrame::neutral()),
            ))
            .id();
        let hand_r = app
            .world_mut()
            .spawn((
                Limb {
                    of: host,
                    slot: LimbSlot::HandRight,
                    home_offset: ae::Vec2::ZERO,
                },
                ActorControl(ActorControlFrame::neutral()),
            ))
            .id();

        // The pilot's brain writes two DIVERGING limb intents: left hand
        // sweeps left and strikes; right hand climbs.
        let mut intents = LimbIntents::default();
        let mut left = ActorControlFrame::neutral();
        left.velocity_target = ae::Vec2::new(-300.0, 0.0);
        left.melee_pressed = true;
        intents.0.insert(LimbSlot::HandLeft, left);
        let mut right = ActorControlFrame::neutral();
        right.velocity_target = ae::Vec2::new(0.0, -200.0);
        intents.0.insert(LimbSlot::HandRight, right);
        app.world_mut().entity_mut(host).insert((
            LimbRig {
                limbs: vec![hand_l, hand_r],
            },
            intents,
        ));

        app.update();

        let l = app.world().get::<ActorControl>(hand_l).unwrap();
        assert_eq!(l.0.velocity_target, ae::Vec2::new(-300.0, 0.0));
        assert!(l.0.melee_pressed, "left hand got its strike edge");
        let r = app.world().get::<ActorControl>(hand_r).unwrap();
        assert_eq!(r.0.velocity_target, ae::Vec2::new(0.0, -200.0));
        assert!(!r.0.melee_pressed, "intents do not bleed across slots");

        // Next tick the pilot only drives the right hand: the left hand is
        // explicitly neutralized, not left running its stale sweep.
        let mut only_right = LimbIntents::default();
        let mut r2 = ActorControlFrame::neutral();
        r2.velocity_target = ae::Vec2::new(150.0, 0.0);
        only_right.0.insert(LimbSlot::HandRight, r2);
        app.world_mut().entity_mut(host).insert(only_right);
        app.update();

        let l = app.world().get::<ActorControl>(hand_l).unwrap();
        assert_eq!(l.0.velocity_target, ae::Vec2::ZERO, "stale intent cleared");
        assert!(!l.0.melee_pressed);
        let r = app.world().get::<ActorControl>(hand_r).unwrap();
        assert_eq!(r.0.velocity_target, ae::Vec2::new(150.0, 0.0));
    }
}
