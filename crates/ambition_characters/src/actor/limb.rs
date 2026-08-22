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
//! FollowOwner hitboxes, and damage attribution all pick them up unchanged.
//!
//! This is a MOUNT-level capability, not boss machinery: a mech with arms is
//! the same component set. The coordinator is whatever brain currently drives
//! the host — a scripted `BossPattern` through the ADR 0020 `ControlGrant`, or
//! the player after possession — because the fan-out reads only data on the
//! host.
//!
//! Determinism: limbs fan out in [`LimbSlot`] order — the rig is keyed by slot,
//! so iteration order is a property of the CONTENT rather than of when anything
//! spawned, and never `Entity` iteration order; a slot with no intent this
//! tick gets an explicit NEUTRAL frame, so stale intents can't drift.
//!
//! ⭐ The VOCABULARY lives here, in the character domain, because this is where
//! a limb route is AUTHORED: `LimbRoute` names the slots a strike drives, and
//! it named them as `String` for as long as `LimbSlot` lived up in the
//! platformer monolith — a stringly-typed round-trip whose only job was to
//! cross a crate boundary that should not have existed. The STRIKE ROUTER that
//! reads a mount's kinematics still lives with the mount; only the vocabulary
//! and the fan-out (which touch nothing but character control) live here.

use std::collections::BTreeMap;

use ambition_platformer2d_core as ae;
use bevy::prelude::{Component, Entity, Query, With};

use crate::actor::control::ActorControlFrame;
use crate::brain::ActorControl;

/// Which limb of the rig a body is. Grows per content (a serpent boss adds
/// variants); ordered so [`LimbIntents`]' BTreeMap iterates deterministically.
///
/// Authored directly in RON as a unit variant (`slots: [HandLeft, HandRight]`),
/// the same way its sibling `LimbMotion` is — so a slot name that does not
/// exist is a content LOAD error rather than a silently dropped route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize)]
pub enum LimbSlot {
    HandLeft,
    HandRight,
}

/// On the HOST body: its driven limbs, **keyed by the slot each one fills**. The
/// rig owns no behavior — it is a relationship, like `MountSlot`.
///
/// This was a `Vec<Entity>` "in spawn order (the stable fan-out order)", and
/// that description overstated what the order did: nothing reads the rig
/// positionally. [`fan_out_limb_intents`] looked up each limb's own [`Limb::slot`]
/// to find its intent, so the vector supplied membership and the limb supplied
/// meaning — two places holding one fact, with a vector able to contain the same
/// limb twice (driving it twice per frame) or two limbs claiming one slot.
///
/// A `BTreeMap<LimbSlot, Entity>` makes both unrepresentable, gives iteration a
/// deterministic order derived from the slot rather than from when anything
/// spawned, and makes "the host's rig composition" an exactly checkable value.
#[derive(Component, Clone, Default, Debug)]
pub struct LimbRig {
    pub limbs: BTreeMap<LimbSlot, Entity>,
}

impl LimbRig {
    /// A rig holding exactly these slot→limb pairs.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (LimbSlot, Entity)>) -> Self {
        Self {
            limbs: pairs.into_iter().collect(),
        }
    }

    pub fn get(&self, slot: LimbSlot) -> Option<Entity> {
        self.limbs.get(&slot).copied()
    }

    /// Which slot this limb occupies, if any. Answerable at all only because
    /// the rig is keyed by slot.
    pub fn slot_of(&self, limb: Entity) -> Option<LimbSlot> {
        self.limbs
            .iter()
            .find(|(_, &entity)| entity == limb)
            .map(|(&slot, _)| slot)
    }
}

/// On each LIMB body: which host it belongs to and which slot it fills.
#[derive(Component, Clone, Debug)]
pub struct Limb {
    pub of: Entity,
    pub slot: LimbSlot,
    /// Host-local (body-frame) idle anchor, in pixels. When the limb has no
    /// routed strike intent this tick, the strike router steers its
    /// `velocity_target` toward `host.pos + gravity_frame(home_offset)` — the
    /// idle pose source that replaces the deleted per-frame hand animation
    /// (station-keeping).
    pub home_offset: ae::Vec2,
}

/// On the HOST (mount) body carrying a [`LimbRig`]: the router's per-mount edge
/// memory. Holds the move id whose STRIKE currently drives limbs so a
/// `melee_pressed` edge fires exactly once — at the Active-window onset —
/// instead of every tick the strike is live. `None` when no routed strike is
/// active.
#[derive(Component, Clone, Default, Debug)]
pub struct LimbRouteState {
    active_move: Option<String>,
}

impl LimbRouteState {
    /// Advance the edge memo to this tick's actively-striking move (`None` for a
    /// telegraph or no strike at all) and report whether that is an ONSET — a
    /// different move than the one driving limbs last tick.
    ///
    /// The rule and the memo it reads are ONE step: a caller that could observe
    /// the memo and then forget to advance it would emit the strike edge every
    /// tick the strike is live, which is the exact bug the memo exists to stop.
    pub fn begin_strike(&mut self, active_strike_move: Option<String>) -> bool {
        let onset = matches!(&active_strike_move, Some(m)
            if self.active_move.as_deref() != Some(m.as_str()));
        self.active_move = active_strike_move;
        onset
    }
}

/// On the HOST body: the per-limb intent table its driving brain writes each
/// tick (a boss pattern maps attack steps onto per-limb velocity targets +
/// attack edges here; a possessing player's verb map writes here too).
#[derive(Component, Clone, Default, Debug)]
pub struct LimbIntents(pub BTreeMap<LimbSlot, ActorControlFrame>);

/// Copy each rigged limb's intent onto its `ActorControl` — the 1→N sibling
/// of `steer_mount_from_rider`'s 1→1 copy. A slot with no intent this tick is
/// explicitly neutralized (no stale frames). Runs after the host brain tick,
/// before body integration.
pub fn fan_out_limb_intents(
    hosts: Query<(&LimbRig, &LimbIntents)>,
    mut limbs: Query<&mut ActorControl, With<Limb>>,
) {
    for (rig, intents) in &hosts {
        // The RIG's key is the slot, so the intent a limb receives is decided by
        // the host's own record of what that limb is for. Reading the limb's
        // `Limb::slot` instead — as this used to — asked the driven body which
        // instrument it was playing, which is the wrong end of the relationship
        // and diverges silently if the two ever disagree.
        for (&slot, &limb_entity) in &rig.limbs {
            let Ok(mut control) = limbs.get_mut(limb_entity) else {
                continue; // despawned/unspawned limb: the rig tolerates gaps
            };
            control.0 = intents
                .0
                .get(&slot)
                .copied()
                .unwrap_or_else(ActorControlFrame::neutral);
        }
    }
}

impl bevy::ecs::entity::MapEntities for LimbRig {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        // Keys are slots, not entities, so remapping touches values only and
        // cannot collide two limbs onto one key.
        for entity in self.limbs.values_mut() {
            *entity = mapper.get_mapped(*entity);
        }
    }
}

impl bevy::ecs::entity::MapEntities for Limb {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.of = mapper.get_mapped(self.of);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        left.velocity_target = ae::WorldVec2::new(-300.0, 0.0);
        left.melee_pressed = true;
        intents.0.insert(LimbSlot::HandLeft, left);
        let mut right = ActorControlFrame::neutral();
        right.velocity_target = ae::WorldVec2::new(0.0, -200.0);
        intents.0.insert(LimbSlot::HandRight, right);
        app.world_mut().entity_mut(host).insert((
            LimbRig::from_pairs([(LimbSlot::HandLeft, hand_l), (LimbSlot::HandRight, hand_r)]),
            intents,
        ));

        app.update();

        let l = app.world().get::<ActorControl>(hand_l).unwrap();
        assert_eq!(l.0.velocity_target, ae::WorldVec2::new(-300.0, 0.0));
        assert!(l.0.melee_pressed, "left hand got its strike edge");
        let r = app.world().get::<ActorControl>(hand_r).unwrap();
        assert_eq!(r.0.velocity_target, ae::WorldVec2::new(0.0, -200.0));
        assert!(!r.0.melee_pressed, "intents do not bleed across slots");

        // Next tick the pilot only drives the right hand: the left hand is
        // explicitly neutralized, not left running its stale sweep.
        let mut only_right = LimbIntents::default();
        let mut r2 = ActorControlFrame::neutral();
        r2.velocity_target = ae::WorldVec2::new(150.0, 0.0);
        only_right.0.insert(LimbSlot::HandRight, r2);
        app.world_mut().entity_mut(host).insert(only_right);
        app.update();

        let l = app.world().get::<ActorControl>(hand_l).unwrap();
        assert_eq!(
            l.0.velocity_target,
            ae::WorldVec2::ZERO,
            "stale intent cleared"
        );
        assert!(!l.0.melee_pressed);
        let r = app.world().get::<ActorControl>(hand_r).unwrap();
        assert_eq!(r.0.velocity_target, ae::WorldVec2::new(150.0, 0.0));
    }

    /// The edge memo advances INSIDE the onset question: a strike that stays
    /// live reports onset exactly once, and a new move re-arms it.
    #[test]
    fn a_live_strike_reports_its_onset_exactly_once() {
        let mut state = LimbRouteState::default();
        assert!(state.begin_strike(Some("hand_slam".into())), "first tick");
        assert!(
            !state.begin_strike(Some("hand_slam".into())),
            "the same strike, still live, is not a second edge"
        );
        assert!(
            state.begin_strike(Some("hand_sweep".into())),
            "a different move re-arms"
        );
        assert!(!state.begin_strike(None), "no strike is never an onset");
        assert!(
            state.begin_strike(Some("hand_sweep".into())),
            "and clears the memo, so the same move re-arms after a gap"
        );
    }
}
