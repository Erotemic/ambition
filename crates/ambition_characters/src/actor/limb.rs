//! Limb-control vocabulary shared by articulated actors.
//!
//! A host carries a [`LimbRig`] and [`LimbIntents`]; [`fan_out_limb_intents`]
//! copies each slot's frame to the corresponding limb's `ActorControl`. Limbs
//! are ordinary actor bodies without their own brain or health.
//!
//! The rig is keyed by [`LimbSlot`] for deterministic fan-out, and a missing
//! intent explicitly neutralizes that limb. Mount-specific strike routing stays
//! with the mount code.

use std::collections::BTreeMap;

use ambition_platformer2d_core as ae;
use bevy::prelude::{Component, Entity, Query, With};

use crate::actor::control::ActorControlFrame;
use crate::brain::ActorControl;

/// Stable authored limb slot. Enum ordering defines deterministic rig iteration.
/// Invalid RON slot names fail deserialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize)]
pub enum LimbSlot {
    HandLeft,
    HandRight,
}

/// Host-owned mapping from limb slot to limb entity.
/// Slot keys prevent duplicate slots and give deterministic iteration order.
#[derive(Component, Clone, Default, Debug)]
pub struct LimbRig {
    pub limbs: BTreeMap<LimbSlot, Entity>,
}

impl LimbRig {
    pub fn from_pairs(pairs: impl IntoIterator<Item = (LimbSlot, Entity)>) -> Self {
        Self {
            limbs: pairs.into_iter().collect(),
        }
    }

    pub fn get(&self, slot: LimbSlot) -> Option<Entity> {
        self.limbs.get(&slot).copied()
    }

    /// Return the slot assigned to `limb`, if present.
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
    /// Host-local idle anchor in pixels. With no routed strike, the router steers
    /// the limb toward this offset in the host's gravity frame.
    pub home_offset: ae::Vec2,
}

/// Remembers the move currently driving limbs so `melee_pressed` fires only on
/// the active-window onset.
#[derive(Component, Clone, Default, Debug)]
pub struct LimbRouteState {
    active_move: Option<String>,
}

impl LimbRouteState {
    /// Advance the active-move memo and report whether a new strike began.
    pub fn begin_strike(&mut self, active_strike_move: Option<String>) -> bool {
        let onset = matches!(&active_strike_move, Some(m)
            if self.active_move.as_deref() != Some(m.as_str()));
        self.active_move = active_strike_move;
        onset
    }
}

/// Host-owned per-limb control intents for the current tick.
#[derive(Component, Clone, Default, Debug)]
pub struct LimbIntents(pub BTreeMap<LimbSlot, ActorControlFrame>);

/// Copy host limb intents into each rigged limb's `ActorControl`.
/// Missing slot intents are neutralized. Runs after host brain updates and before integration.
pub fn fan_out_limb_intents(
    hosts: Query<(&LimbRig, &LimbIntents)>,
    mut limbs: Query<&mut ActorControl, With<Limb>>,
) {
    for (rig, intents) in &hosts {
        // The host rig is authoritative for slot membership; do not derive the
        // slot from the limb's forward link, which may disagree with the rig.
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
