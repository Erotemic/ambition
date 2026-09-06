//! Switch-arming gate for encounters. `EncounterSwitchIndex` is rebuilt each
//! frame from `SwitchFeature + SwitchOn` components and answers
//! `encounter_armed(id)` (semantics: off/red switch arms, green/on disables,
//! unlinked = always armed, any one off switch arms a multi-switch fight).
//! ⭐ COMPLETION IS THE SAME RULE READ BACKWARDS: `switch_ids_for_encounter`
//! returns EVERY linked switch so the clear path greens all of them, because
//! "any red arms" can only be satisfied by leaving none red.
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

    /// EVERY switch id linked to an encounter, in link order — the auto-green
    /// clear path greens all of them.
    ///
    /// ⛔ THIS RETURNED ONLY THE FIRST UNTIL 2026-09-03, AND THE PAIR WAS
    /// INCOHERENT. [`Self::encounter_armed`] arms on ANY red link, so with two
    /// switches on one encounter, completion greened the first, the second
    /// stayed red, `encounter_armed` stayed true, and the driver's *"a terminal
    /// encounter still armed is reset and started again"* re-armed the fight
    /// under a player still standing in the trigger. Greening one switch could
    /// never satisfy a rule that asks about all of them.
    ///
    /// ⇒ The two halves now share one policy, and it is the arming rule's:
    /// **all green disarms, any red arms.** The alternative — a single
    /// controlling switch — would have meant changing `encounter_armed` instead,
    /// and that rule is the authored one, tested by
    /// `any_off_switch_arms_a_multi_switch_encounter`.
    ///
    /// ⚠ No authored room links two switches to one encounter today, so this
    /// was a latent defect in a supported API rather than a shipped bug. It is
    /// fixed here because the arming side already promises the behaviour.
    pub fn switch_ids_for_encounter(&self, encounter_id: &str) -> Vec<String> {
        self.links
            .iter()
            .filter(|link| link.target_encounter == encounter_id)
            .map(|link| link.switch_id.clone())
            .collect()
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

impl SwitchActivationQueue {
    /// Canonical projection for the session checksum: length, then each entry
    /// in queue order.
    ///
    /// ORDER IS PART OF THE VALUE — this is a queue, and two peers holding the
    /// same activations in a different order have diverged.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u64};
        let Self(entries) = self;
        let mut bytes = Vec::new();
        put_u64(&mut bytes, entries.len() as u64);
        for entry in entries {
            // Destructured so a new field on an activation must be answered for
            // here rather than silently escaping the checksum.
            let SwitchActivation {
                id,
                action,
                target_encounter,
            } = entry;
            put_str(&mut bytes, id);
            put_str(&mut bytes, action);
            put_str(&mut bytes, target_encounter);
        }
        checksum_bytes(&bytes)
    }
}

#[cfg(test)]
mod queue_checksum_tests {
    use super::{SwitchActivation, SwitchActivationQueue};

    fn activation(id: &str) -> SwitchActivation {
        SwitchActivation {
            id: id.into(),
            action: "reset".into(),
            target_encounter: "boss".into(),
        }
    }

    /// ⭐ The case the type's doc comment is about: a resimulation that pushes an
    /// activation again must not hash like one that did not.
    #[test]
    fn a_duplicated_activation_moves_the_checksum() {
        let one = SwitchActivationQueue(vec![activation("a")]);
        let twice = SwitchActivationQueue(vec![activation("a"), activation("a")]);
        assert_ne!(one.checksum(), twice.checksum());
    }

    /// A queue's ORDER is part of its value.
    #[test]
    fn reordering_the_queue_moves_the_checksum() {
        let ab = SwitchActivationQueue(vec![activation("a"), activation("b")]);
        let ba = SwitchActivationQueue(vec![activation("b"), activation("a")]);
        assert_ne!(ab.checksum(), ba.checksum());
    }

    /// ⛔ And the arm that catches a checksum that can never agree.
    #[test]
    fn equal_queues_agree() {
        let a = SwitchActivationQueue(vec![activation("a"), activation("b")]);
        let b = SwitchActivationQueue(vec![activation("a"), activation("b")]);
        assert_eq!(a.checksum(), b.checksum());
    }
}

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
    fn switch_ids_for_encounter_returns_every_link_not_the_first() {
        let idx = index(vec![link("a", "enc", true), link("b", "enc", false)]);
        assert_eq!(idx.switch_ids_for_encounter("enc"), vec!["a", "b"]);
        assert!(idx.switch_ids_for_encounter("missing").is_empty());
    }

    /// ⛔ THE TWO HALVES MUST AGREE, IN BOTH DIRECTIONS. Arming asks about every
    /// link; the clear path must therefore green every link, or a completed
    /// encounter stays armed and the driver restarts it under the player.
    #[test]
    fn greening_every_linked_switch_disarms_and_one_red_re_arms() {
        let ids = index(vec![link("a", "enc", false), link("b", "enc", false)])
            .switch_ids_for_encounter("enc");
        assert_eq!(ids, vec!["a", "b"], "the clear path must see BOTH switches");

        // Complete: the adapter greens every id the accessor returned.
        let after_clear = index(ids.iter().map(|id| link(id, "enc", true)).collect());
        assert!(
            !after_clear.encounter_armed("enc"),
            "greening every linked switch must DISARM — this is the bug: \
             greening only the first left `enc` armed and the driver re-started it"
        );

        // And the arming rule still bites the moment one goes back to red.
        let one_re_armed = index(vec![link("a", "enc", true), link("b", "enc", false)]);
        assert!(
            one_re_armed.encounter_armed("enc"),
            "one red switch must re-arm, or a multi-switch fight could never be replayed"
        );
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

/// What a switch activation ASKS FOR, as a value rather than a string.
///
/// ⛔ THE STRING WAS WHY AN UNKNOWN ACTION WAS SILENTLY A NO-OP. The drained
/// activation carries `action: String`, and the single consumer matched it three
/// ways — `== "FlipGravity"`, `strip_prefix("SetGravity")`, then
/// `!matches!(.., "ResetEncounter") { continue }`. Anything else fell through the
/// last guard and did nothing, with no warning and no test that could notice.
/// Parsing once into this enum makes an unhandled kind visible at the parse
/// site instead of invisible at the end of a chain of guards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SwitchAction {
    /// Invert ambient gravity.
    FlipGravity,
    /// Set ambient gravity to a cardinal face. The token is the authored
    /// suffix — `Down`, `Up`, `Left`, `Right` — and an unrecognised one means
    /// `Down`, which is the behaviour the string road had.
    SetGravity(GravityFace),
    /// Re-arm or clear an encounter.
    ResetEncounter,
    /// Authored but not a kind this engine acts on. Carried rather than dropped
    /// so a consumer can report it; the string road could not tell this apart
    /// from a handled action that did nothing.
    Unhandled(String),
}

/// The cardinal faces `SetGravity<Face>` can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GravityFace {
    Down,
    Up,
    Left,
    Right,
}

impl GravityFace {
    /// The unit direction this face makes "down".
    pub fn direction(self) -> [f32; 2] {
        match self {
            Self::Up => [0.0, -1.0],
            Self::Left => [-1.0, 0.0],
            Self::Right => [1.0, 0.0],
            Self::Down => [0.0, 1.0],
        }
    }
}

impl SwitchAction {
    /// Classify an authored action string.
    pub fn parse(action: &str) -> Self {
        if action == "FlipGravity" {
            return Self::FlipGravity;
        }
        if let Some(face) = action.strip_prefix("SetGravity") {
            return Self::SetGravity(match face {
                "Up" => GravityFace::Up,
                "Left" => GravityFace::Left,
                "Right" => GravityFace::Right,
                // `Down` and anything else: the string road's own fallback.
                _ => GravityFace::Down,
            });
        }
        if action == "ResetEncounter" {
            return Self::ResetEncounter;
        }
        Self::Unhandled(action.to_string())
    }
}

/// One activation, resolved: what was pressed, what it asks for, and — for the
/// kinds that toggle — the switch's value AFTER the toggle.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedSwitchActivation {
    /// The switch's authored id.
    pub id: String,
    /// What it asks for, parsed once.
    pub action: SwitchAction,
    /// The encounter this activation targets, empty for "the active room's".
    pub target_encounter: String,
    /// The persisted switch value AFTER this tick's toggle.
    ///
    /// ⛔ CARRIED, NOT RE-DERIVED. Every consumer that used to ask
    /// `save.switch(id)` for itself would read a value that depends on whether
    /// it ran before or after the writer. The drain toggles once and publishes
    /// the result, so "is it on now" is the same answer for everyone.
    pub on: bool,
}

/// This tick's activations, resolved — the switch domain's published answer to
/// *"what did the player just press?"*.
///
/// ⚠ REPLACED each tick, never appended: an activation acted on twice is an
/// encounter reset applied twice, which is the defect
/// [`SwitchActivationQueue`]'s rollback registration exists to prevent.
#[derive(bevy::prelude::Resource, Default, Clone, Debug)]
pub struct ResolvedSwitchActivations(pub Vec<ResolvedSwitchActivation>);

/// The set [`drain_switch_activations`] runs in.
///
/// Consumers of [`ResolvedSwitchActivations`] order `.after()` this rather than
/// against the function, so the drain can move without every reactor moving.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SwitchActivationDrained;

/// THE drain. One system takes the queue, parses each action, performs the
/// persisted toggle, and publishes the result.
///
/// ⛔ **ONE ORDERED DRAIN, AND ONE AUTHOR FOR THE TOGGLE.** Order is part of the
/// queue's value — [`SwitchActivationQueue::checksum`] says so, and the queue is
/// rollback-registered because a rewind that re-pushes predicted activations
/// double-applies a reset. Four consumers each draining what interests them
/// would race that order and each would need its own toggle; this drains once,
/// in queue order, and every consumer reads [`ResolvedSwitchActivations`].
pub fn drain_switch_activations(
    mut queue: bevy::prelude::ResMut<SwitchActivationQueue>,
    mut resolved: bevy::prelude::ResMut<ResolvedSwitchActivations>,
    mut save: bevy::prelude::ResMut<ambition_persistence::save::AmbitionGameSave>,
) {
    resolved.0.clear();
    if queue.0.is_empty() {
        return;
    }
    for activation in std::mem::take(&mut queue.0) {
        let action = SwitchAction::parse(&activation.action);
        // THIS ROAD'S persisted write, in its one place: three arms of one
        // match, so an action's meaning and its durable consequence cannot
        // drift apart. `ResetEncounter` TOGGLES; the gravity kinds latch ON so
        // their sprite reads engaged; an unhandled action must not touch
        // persisted state at all.
        //
        // ⛔⛔ IT IS NOT THE ONLY WRITER OF THE `switches` SAVE FAMILY, and this
        // comment said so until 2026-09-05 — true of this function, false of
        // the tree, and exactly what the author of a FOURTH writer would read
        // first. Measured: `encounter_features/src/systems.rs:496` greens every
        // switch of a completed encounter, and `content/src/falling_sand_sim.rs`
        // writes the spout switches.
        //
        // ⛔⛔ AND THEY ARE NOT DISJOINT. An earlier version of this comment
        // claimed the roads separated by ACTION KIND — inferred from the
        // `_ => continue` below rather than measured, and wrong.
        // `content/src/falling_sand_sim.rs` keys off the switch ID, not the
        // action, and the four falling-sand spouts are authored
        // `action: ResetEncounter` — the arm right below. ⇒ Both roads write
        // `set_switch` for the same id on the same activation, and the content
        // one says in place that it means to win: *"without this write the
        // save's switch flag stays whatever the encounter pipeline set it to"*.
        //
        // ⚠ NOTHING ORDERS THEM. This system is `.in_set(SwitchActivationDrained)`,
        // which is not placed in any simulation phase; the falling-sand reader is
        // `.in_set(Platformer2dSimulationPhaseMonolith::GameplayEffects)`. Both
        // are downstream of one `SwitchActivated` from `features/ecs/interact.rs`.
        // Whichever runs last wins, and the executor's order is stable but
        // arbitrary — so a behavioural test passes either way. Recorded in
        // `world-facts-observations-and-memory.md`; the fix is an ordering edge
        // and it is a content/engine boundary decision, not a local one.
        let on = match &action {
            SwitchAction::ResetEncounter => {
                let next = !save.data().switch(&activation.id);
                save.data_mut().set_switch(&activation.id, next);
                next
            }
            SwitchAction::FlipGravity => {
                let next = !save.data().switch(&activation.id);
                save.data_mut().set_switch(&activation.id, next);
                next
            }
            SwitchAction::SetGravity(_) => {
                save.data_mut().set_switch(&activation.id, true);
                true
            }
            SwitchAction::Unhandled(_) => save.data().switch(&activation.id),
        };
        resolved.0.push(ResolvedSwitchActivation {
            id: activation.id,
            action,
            target_encounter: activation.target_encounter,
            on,
        });
    }
}

#[cfg(test)]
mod one_drain_one_author {
    use super::*;
    use ambition_persistence::save::AmbitionGameSave;
    use bevy::prelude::*;

    fn app_with(activations: Vec<SwitchActivation>) -> App {
        let mut app = App::new();
        app.insert_resource(AmbitionGameSave::default());
        app.insert_resource(SwitchActivationQueue(activations));
        app.init_resource::<ResolvedSwitchActivations>();
        app.add_systems(Update, drain_switch_activations);
        app
    }

    fn activation(id: &str, action: &str) -> SwitchActivation {
        SwitchActivation {
            id: id.to_string(),
            action: action.to_string(),
            target_encounter: String::new(),
        }
    }

    /// ⛔ THE TOGGLE HAS EXACTLY ONE AUTHOR, and the evidence is that it happens
    /// exactly ONCE per activation.
    ///
    /// A `ResetEncounter` press flips the persisted switch. While four policies
    /// shared one loop, the flip lived inside it and every reader that wanted to
    /// know "is it on now" either read the loop's local or re-derived it from the
    /// save — and got a different answer depending on whether it ran before or
    /// after. Two authors is the failure this asserts against: run the drain
    /// twice over one queue and the switch must NOT return to where it started,
    /// because the second run has nothing left to drain.
    #[test]
    fn one_activation_toggles_the_switch_once_however_often_the_drain_runs() {
        let mut app = app_with(vec![activation("gate", "ResetEncounter")]);

        app.update();
        let after_one = app.world().resource::<AmbitionGameSave>().data().switch("gate");
        assert!(after_one, "a first press turns the switch on");
        assert_eq!(
            app.world().resource::<ResolvedSwitchActivations>().0.len(),
            1,
            "the drain publishes the activation it consumed"
        );

        app.update();
        assert_eq!(
            app.world().resource::<AmbitionGameSave>().data().switch("gate"),
            after_one,
            "a SECOND drain of an EMPTY queue must not toggle again — a second \
             author, or a re-drained queue, shows up here as the switch flipping \
             back with nobody pressing it"
        );
        assert!(
            app.world().resource::<ResolvedSwitchActivations>().0.is_empty(),
            "the published facts are REPLACED each tick, never appended: an \
             activation acted on twice is an encounter reset applied twice"
        );
    }

    /// The published value is the POST-toggle one, so no consumer re-derives it.
    #[test]
    fn the_published_fact_carries_the_value_after_the_toggle() {
        let mut app = app_with(vec![activation("gate", "ResetEncounter")]);
        app.update();
        let published = app.world().resource::<ResolvedSwitchActivations>().0[0].clone();
        assert_eq!(published.action, SwitchAction::ResetEncounter);
        assert!(
            published.on,
            "the fact must carry the value AFTER the toggle — a consumer that \
             re-derived it from the save would get a different answer depending \
             on whether it ran before or after the writer"
        );
    }

    /// An action this engine does not act on must not touch persisted state.
    ///
    /// ⚠ The string road could not tell an unhandled action apart from a handled
    /// one that did nothing: it fell through the last `matches!` guard silently.
    #[test]
    fn an_unhandled_action_is_carried_and_changes_no_persisted_state() {
        let mut app = app_with(vec![activation("gate", "SummonKraken")]);
        app.update();
        let published = app.world().resource::<ResolvedSwitchActivations>().0[0].clone();
        assert_eq!(
            published.action,
            SwitchAction::Unhandled("SummonKraken".into()),
            "an unrecognised action is CARRIED, so a consumer can report it"
        );
        assert!(
            !app.world().resource::<AmbitionGameSave>().data().switch("gate"),
            "an action the engine does not handle must not flip a persisted switch"
        );
    }

    /// ⛔ ORDER IS PART OF THE VALUE. The queue's own checksum says so, and it is
    /// rollback-registered because a rewind that re-pushes predicted activations
    /// double-applies a reset. The drain must publish in queue order.
    #[test]
    fn the_drain_publishes_in_queue_order() {
        let mut app = app_with(vec![
            activation("first", "ResetEncounter"),
            activation("second", "FlipGravity"),
            activation("third", "SetGravityLeft"),
        ]);
        app.update();
        let ids: Vec<String> = app
            .world()
            .resource::<ResolvedSwitchActivations>()
            .0
            .iter()
            .map(|a| a.id.clone())
            .collect();
        assert_eq!(
            ids,
            vec!["first", "second", "third"],
            "two peers holding the same activations in a different order have \
             diverged — SwitchActivationQueue::checksum says so"
        );
    }
}
