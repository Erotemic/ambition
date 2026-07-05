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
use ambition_characters::brain::ActorControl;
use bevy::prelude::{Component, Entity, Query};

/// Which limb of the rig a body is. Grows per content (a serpent boss adds
/// variants); ordered so `LimbIntents`' BTreeMap iterates deterministically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LimbSlot {
    HandLeft,
    HandRight,
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
