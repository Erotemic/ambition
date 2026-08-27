//! Switch-arming gate for encounters. `EncounterSwitchIndex` is rebuilt each
//! frame from `SwitchFeature + SwitchOn` components and answers
//! `encounter_armed(id)` (semantics: off/red switch arms, green/on disables,
//! unlinked = always armed, any one off switch arms a multi-switch fight).
//! `SwitchActivationQueue` is the per-frame FIFO of activations the encounter
//! tick drains to apply resets.

use bevy::prelude::Resource;

use crate::registry::SwitchActivation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncounterSwitchLink {
    pub switch_id: String,
    pub target_encounter: String,
    pub on: bool,
}

/// Cached ECS switch state used by the encounter state machine.
///
/// Rebuilt from `SwitchFeature + SwitchOn` components each frame.
#[derive(Resource, Default, Clone, Debug)]
pub struct EncounterSwitchIndex {
    pub links: Vec<EncounterSwitchLink>,
}

impl EncounterSwitchIndex {
    /// Whether `encounter_id` is armed. Off/red switches arm their target;
    /// no linked switch means the encounter is always armed.
    pub fn encounter_armed(&self, encounter_id: &str) -> bool {
        let mut found = false;
        for link in &self.links {
            if link.target_encounter != encounter_id {
                continue;
            }
            found = true;
            if !link.on {
                return true;
            }
        }
        !found
    }

    /// First switch id linked to an encounter, used by the auto-green clear
    /// path. Multi-switch encounters can replace this with a richer policy.
    pub fn switch_id_for_encounter(&self, encounter_id: &str) -> Option<String> {
        self.links
            .iter()
            .find(|link| link.target_encounter == encounter_id)
            .map(|link| link.switch_id.clone())
    }
}

// ⛔ `rebuild_encounter_switch_index` DID NOT COME WITH THESE TYPES, and the
// reason is the only interesting thing about this move: it reads a switch's
// `FeatureId`, which belongs to `ambition_combat`, and this crate does not
// depend on combat. Taking the system would have bought a dependency edge to
// carry one field read. It stays where the vocabulary it reads lives — see
// `encounter/switch_index.rs` in the actor monolith.

/// FIFO queue of switch activations produced by the feature systems each frame.
/// The encounter system drains it and applies the matching reset.
///
/// NOT actually drained within the producing frame: the producer runs in
/// `Platformer2dSimulationPhaseMonolith::GameplayEffects` and the consumer in `EncounterSimulation`,
/// which is ordered EARLIER — so an activation pushed on frame N is applied on
/// frame N+1 and the queue is live state at a rollback save boundary. `Clone`
/// (and its rollback registration) exist for exactly that reason: without them
/// a rewind keeps predicted activations and resimulation pushes them again,
/// double-applying an encounter reset.
#[derive(Resource, Default, Clone)]
pub struct SwitchActivationQueue(pub Vec<SwitchActivation>);

#[cfg(test)]
mod switch_index_tests {
    //! Encounter arming from switch state. The authored semantics are
    //! "red (off) = armed, green (on) = disabled", an unlinked encounter
    //! is always armed, and any single off switch arms a multi-switch
    //! encounter. This is the gate the encounter state machine reads.
    use super::*;

    fn link(switch: &str, target: &str, on: bool) -> EncounterSwitchLink {
        EncounterSwitchLink {
            switch_id: switch.into(),
            target_encounter: target.into(),
            on,
        }
    }
    fn index(links: Vec<EncounterSwitchLink>) -> EncounterSwitchIndex {
        EncounterSwitchIndex { links }
    }

    #[test]
    fn unlinked_encounter_is_always_armed() {
        assert!(
            EncounterSwitchIndex::default().encounter_armed("anything"),
            "no linked switch -> always armed"
        );
    }

    #[test]
    fn off_switch_arms_on_switch_disarms() {
        assert!(index(vec![link("s", "enc", false)]).encounter_armed("enc"));
        assert!(!index(vec![link("s", "enc", true)]).encounter_armed("enc"));
    }

    #[test]
    fn any_off_switch_arms_a_multi_switch_encounter() {
        assert!(
            index(vec![link("a", "enc", true), link("b", "enc", false)]).encounter_armed("enc"),
            "one red switch is enough to arm"
        );
        assert!(
            !index(vec![link("a", "enc", true), link("b", "enc", true)]).encounter_armed("enc"),
            "all green -> disabled"
        );
    }

    #[test]
    fn links_for_other_encounters_are_ignored() {
        // An ON switch targeting a different encounter leaves "enc" unlinked -> armed.
        assert!(index(vec![link("s", "other", true)]).encounter_armed("enc"));
    }

    #[test]
    fn switch_id_for_encounter_finds_the_first_match() {
        let idx = index(vec![link("a", "enc", true), link("b", "enc", false)]);
        assert_eq!(idx.switch_id_for_encounter("enc").as_deref(), Some("a"));
        assert_eq!(idx.switch_id_for_encounter("missing"), None);
    }
}

use ambition_platformer2d_core as ae;
use bevy::prelude::{Component, Message};

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct SwitchFeature {
    pub activation: SwitchActivation,
}

impl SwitchFeature {
    pub fn new(activation: SwitchActivation) -> Self {
        Self { activation }
    }
}

/// Live switch state used by rendering and encounter reset logic.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwitchOn(pub bool);

/// A Switch interactable was activated. Carries the parsed
/// [`SwitchActivation`] directly — the `switch:<id>:<action>:<target>` wire
/// string lives only at the engine `InteractionKind::Custom` boundary.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct SwitchActivated {
    pub activation: SwitchActivation,
    pub pos: ae::Vec2,
}
