//! Character action scheme — the per-subject vocabulary of "what does each
//! control slot do, and what is it called."
//!
//! This is the DATA half of the character-actions design
//! (`docs/planning/engine/character-actions.md`). A subject (a character, or
//! later a menu context) declares an ordered set of [`ActionSpec`]s: for each
//! [`ControlSlot`] it claims, the action's stable id, its player-facing
//! label + optional visual, and its [`ActionGate`] — what pressing that slot
//! actually does.
//!
//! Two rules keep this crate a pure-data leaf (serde only, no engine, no Bevy):
//!
//! - **The gate is string-keyed.** `ActionGate::Movement("jump")` names a
//!   movement-action id; the TYPED resolution to the engine's `MovementAction`
//!   (and the kernel dispatch) lives in the character/runtime crate that has
//!   the engine dep. This crate never references the kernel.
//! - **Slots are device-free.** [`ControlSlot`] is the abstract button
//!   position (Jump / Attack / …), not a physical key or a leafwing
//!   `SandboxAction`. The physical-input → slot binding is owned by the input
//!   layer; this crate only says which slot an action lives on.
//!
//! The runtime scheme is a DERIVED cache of already-snapshotted authorities
//! (`AbilitySet` + moveset + techniques), so a rollback reconstructs it for
//! free — nothing scheme-shaped ever enters the input stream or the snapshot
//! ledger. See the design doc's invariant 1.

use serde::{Deserialize, Serialize};

use crate::MovesetContract;

/// The abstract, device-free control slots a character action can occupy.
///
/// A slot is a button POSITION, stable across characters so the on-screen
/// layout and the player's muscle memory don't move when the label changes.
/// The physical input bound to each slot (a key, a gamepad button) is the
/// input layer's concern; the `slot → action` mapping is the character's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ControlSlot {
    Jump,
    /// Primary melee slot (the moveset `"attack"` verb + its directional chain).
    Attack,
    /// Signature / special slot. Gets a DEDICATED input slot (design review):
    /// today blink double-fires the special, which the action model splits.
    Special,
    /// Ranged slot — the moveset `"ranged"` verb. (Invariant: the Projectile
    /// slot maps 1:1 to the `"ranged"` moveset verb, so the three combat slots
    /// Attack/Special/Projectile line up with the three moveset verbs.)
    Projectile,
    Dash,
    Blink,
    Interact,
    /// Utility slot — fly toggle / form toggle and similar mode switches.
    Utility,
    /// Quick-action slot — shield / guard.
    QuickAction,
    /// Modifier slot — a slot whose SUSTAIN is the point. Content binds a
    /// technique to holding it (a locomotion mode, a stance) and may bind a
    /// momentary action to its press edge. The engine reserves the position and
    /// carries the state; naming what it does is the character's job, which is why
    /// a scheme may label it `"Run"` on one body and `"Run / Spark"` on the same
    /// body once its kit grows.
    Modifier,
}

/// The canonical presentation order of the gameplay slots. Iterating this
/// gives a deterministic scheme ordering independent of insertion order or
/// any map hashing (query-order discipline).
pub const CANONICAL_SLOT_ORDER: [ControlSlot; 10] = [
    ControlSlot::Jump,
    ControlSlot::Modifier,
    ControlSlot::Attack,
    ControlSlot::Special,
    ControlSlot::Projectile,
    ControlSlot::Dash,
    ControlSlot::Blink,
    ControlSlot::Interact,
    ControlSlot::Utility,
    ControlSlot::QuickAction,
];

/// A stable, machine action id (`"jump"`, `"spin_dash"`, `"attack"`). Matches
/// the string keys used by [`MovesetContract::verbs`] and [`crate::MoveSpec::id`]
/// so an action's id can be a moveset verb or a content technique id without a
/// translation table.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ActionId(pub String);

impl ActionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ActionId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

/// Well-known built-in action ids. Content techniques and moveset verbs use
/// their own ids; these name the engine primitives the derivation emits.
pub mod ids {
    pub const JUMP: &str = "jump";
    pub const DASH: &str = "dash";
    pub const BLINK: &str = "blink";
    pub const FLY_TOGGLE: &str = "fly_toggle";
    pub const SHIELD: &str = "shield";
    pub const INTERACT: &str = "interact";
    pub const ATTACK: &str = "attack";
    pub const SPECIAL: &str = "special";
    pub const RANGED: &str = "ranged";
}

/// Opaque handle to a presentation visual (icon/glyph) for an action. No icon
/// catalog exists yet; this is the authoring hook so `Option<VisualId>` can be
/// carried today and resolved when a visual pipeline lands. Text-only until
/// then.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VisualId(pub String);

/// What pressing a slot's action actually does — the gate into behavior.
///
/// String-keyed on purpose: the typed dispatch (to the engine `MovementAction`,
/// the moveset runtime, or a content technique system) is resolved by the
/// crate that owns those types. This keeps the schema a pure-data leaf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionGate {
    /// Drives the movement kernel. String is the movement-action id
    /// (`"jump"`, `"dash"`, `"blink"`, `"fly_toggle"`).
    Movement(String),
    /// A content-defined movement technique (Sanic ball, ground-pound). String
    /// is the technique id its content system consumes.
    Technique(String),
    /// A moveset move. String is the moveset verb (`"attack"`, `"special"`,
    /// `"ranged"`); directional resolution happens in the moveset runtime.
    Move(String),
    /// World interaction, resolved against nearby interactables at press time.
    Interact,
}

/// One declared action: the slot it lives on, its presentation, and its gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionSpec {
    pub id: ActionId,
    pub slot: ControlSlot,
    /// Player-facing label. `None` falls back to a title-cased id via
    /// [`ActionSpec::display`].
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub visual: Option<VisualId>,
    pub gate: ActionGate,
}

impl ActionSpec {
    /// The label to show for this action: the authored name, else a title-cased
    /// id (`"spin_dash"` → `"Spin Dash"`).
    pub fn display(&self) -> String {
        self.display_name
            .clone()
            .unwrap_or_else(|| title_case_id(self.id.as_str()))
    }
}

/// A subject's full action repertoire: an ordered list of the slots it claims,
/// each with its action. `Vec` (not a map) for deterministic iteration.
///
/// The runtime carries this inside a Bevy component wrapper (in the character
/// crate); this type is the headless, serializable contract.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionSchemeContract {
    pub actions: Vec<ActionSpec>,
}

impl ActionSchemeContract {
    /// Build a normalized scheme: canonically ordered AND one-action-per-slot.
    /// If duplicate slots are passed, the FIRST in canonical order wins (callers
    /// wanting override semantics upsert before constructing) — so the invariant
    /// downstream code relies on (`action_for_slot` is unambiguous, a slot is
    /// shown once) holds by construction, not by caller discipline.
    pub fn new(actions: Vec<ActionSpec>) -> Self {
        let mut contract = Self { actions }.sorted();
        let mut seen_slots = Vec::new();
        contract.actions.retain(|a| {
            if seen_slots.contains(&a.slot) {
                false
            } else {
                seen_slots.push(a.slot);
                true
            }
        });
        contract
    }

    /// The action on a given slot, if the subject claims it.
    pub fn action_for_slot(&self, slot: ControlSlot) -> Option<&ActionSpec> {
        self.actions.iter().find(|a| a.slot == slot)
    }

    pub fn has_slot(&self, slot: ControlSlot) -> bool {
        self.actions.iter().any(|a| a.slot == slot)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ActionSpec> {
        self.actions.iter()
    }

    /// Reorder the actions into [`CANONICAL_SLOT_ORDER`] so iteration is
    /// deterministic regardless of how they were assembled. Stable for slots
    /// sharing a position (there should be at most one per slot).
    pub fn sorted(mut self) -> Self {
        self.actions.sort_by_key(|a| {
            CANONICAL_SLOT_ORDER
                .iter()
                .position(|s| *s == a.slot)
                .unwrap_or(usize::MAX)
        });
        self
    }

    /// Derive the COMBAT actions (attack / special / ranged) from a moveset.
    ///
    /// Only emits an action for a verb the moveset actually authors, and pulls
    /// each action's label from the bound move's own `display_name` (title-cased
    /// id fallback). Movement actions and content techniques are layered on by
    /// the character/runtime crate that holds the ability + technique data — this
    /// helper stays moveset-only so it can live in the pure-data crate.
    ///
    /// The Projectile slot maps to the `"ranged"` verb by construction, keeping
    /// the three combat slots aligned 1:1 with the three moveset verbs.
    pub fn combat_from_moveset(moveset: &MovesetContract) -> Vec<ActionSpec> {
        const COMBAT: [(&str, ControlSlot); 3] = [
            (ids::ATTACK, ControlSlot::Attack),
            (ids::SPECIAL, ControlSlot::Special),
            (ids::RANGED, ControlSlot::Projectile),
        ];
        COMBAT
            .into_iter()
            .filter(|(verb, _)| moveset.verbs.contains_key(*verb))
            .map(|(verb, slot)| {
                let display_name = moveset.move_for_verb(verb).map(|mv| mv.display());
                ActionSpec {
                    id: ActionId::new(verb),
                    slot,
                    display_name,
                    visual: None,
                    gate: ActionGate::Move(verb.to_owned()),
                }
            })
            .collect()
    }
}

/// Title-case a machine id for a fallback label: `"spin_dash"` → `"Spin Dash"`,
/// `"attack_air_down"` → `"Attack Air Down"`. Splits on `_`, uppercases the
/// first char of each word. ASCII-oriented (ids are ASCII by convention).
pub fn title_case_id(id: &str) -> String {
    id.split('_')
        .filter(|w| !w.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClipBinding, MoveSpec};
    use std::collections::BTreeMap;

    fn bare_move(id: &str) -> MoveSpec {
        MoveSpec {
            id: id.to_string(),
            clip: ClipBinding {
                clip: id.to_string(),
                fallbacks: vec![],
            },
            duration_s: 0.3,
            windows: vec![],
            events: vec![],
            gates: Default::default(),
            start_impulse: None,
            smash_charge_mult: 1.0,
        }
    }

    fn moveset(verbs: &[(&str, &str)]) -> MovesetContract {
        let mut m = MovesetContract::default();
        let mut ids: Vec<&str> = verbs.iter().map(|(_, id)| *id).collect();
        ids.sort();
        ids.dedup();
        m.moves = ids.into_iter().map(bare_move).collect();
        m.verbs = verbs
            .iter()
            .map(|(v, id)| (v.to_string(), id.to_string()))
            .collect::<BTreeMap<_, _>>();
        m
    }

    #[test]
    fn title_case_humanizes_machine_ids() {
        assert_eq!(title_case_id("jump"), "Jump");
        assert_eq!(title_case_id("spin_dash"), "Spin Dash");
        assert_eq!(title_case_id("attack_air_down"), "Attack Air Down");
        // Robust to stray underscores.
        assert_eq!(title_case_id("_bubble__shield_"), "Bubble Shield");
        assert_eq!(title_case_id(""), "");
    }

    #[test]
    fn action_spec_display_prefers_authored_name() {
        let authored = ActionSpec {
            id: ActionId::new("spin_dash"),
            slot: ControlSlot::Attack,
            display_name: Some("Spin Dash!".to_owned()),
            visual: None,
            gate: ActionGate::Technique("spin_dash".to_owned()),
        };
        assert_eq!(authored.display(), "Spin Dash!");

        let unnamed = ActionSpec {
            display_name: None,
            ..authored.clone()
        };
        assert_eq!(unnamed.display(), "Spin Dash"); // title-cased id fallback
    }

    #[test]
    fn combat_from_moveset_maps_three_verbs_to_three_slots() {
        // Projectile slot MUST bind the "ranged" verb (design invariant): the
        // three combat slots line up 1:1 with the three moveset verbs.
        let ms = moveset(&[
            ("attack", "swipe"),
            ("special", "bubble"),
            ("ranged", "bolt"),
        ]);
        let actions = ActionSchemeContract::combat_from_moveset(&ms);
        assert_eq!(actions.len(), 3);

        let by_slot = |slot| actions.iter().find(|a: &&ActionSpec| a.slot == slot);
        let attack = by_slot(ControlSlot::Attack).expect("attack slot present");
        assert_eq!(attack.gate, ActionGate::Move("attack".to_owned()));
        assert_eq!(attack.display(), "Swipe"); // from the bound move id

        let special = by_slot(ControlSlot::Special).expect("special on its OWN slot");
        assert_eq!(special.gate, ActionGate::Move("special".to_owned()));

        let ranged = by_slot(ControlSlot::Projectile).expect("ranged -> Projectile slot");
        assert_eq!(ranged.gate, ActionGate::Move("ranged".to_owned()));
        assert_eq!(ranged.display(), "Bolt");
    }

    #[test]
    fn combat_from_moveset_omits_unauthored_verbs() {
        // A movement-only character (empty moveset) contributes NO combat
        // actions — the slots are simply absent, never a phantom "Jab".
        assert!(ActionSchemeContract::combat_from_moveset(&MovesetContract::default()).is_empty());

        // Attack only: one action, no phantom special/ranged.
        let ms = moveset(&[("attack", "punch")]);
        let actions = ActionSchemeContract::combat_from_moveset(&ms);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].slot, ControlSlot::Attack);
    }

    #[test]
    fn scheme_iteration_is_canonical_regardless_of_insertion_order() {
        // Assemble deliberately out of canonical order; `new` sorts.
        let dash = ActionSpec {
            id: ActionId::new(ids::DASH),
            slot: ControlSlot::Dash,
            display_name: None,
            visual: None,
            gate: ActionGate::Movement(ids::DASH.to_owned()),
        };
        let jump = ActionSpec {
            slot: ControlSlot::Jump,
            id: ActionId::new(ids::JUMP),
            gate: ActionGate::Movement(ids::JUMP.to_owned()),
            ..dash.clone()
        };
        let attack = ActionSpec {
            slot: ControlSlot::Attack,
            id: ActionId::new(ids::ATTACK),
            gate: ActionGate::Move(ids::ATTACK.to_owned()),
            ..dash.clone()
        };
        let scheme = ActionSchemeContract::new(vec![dash, attack, jump]);
        let order: Vec<ControlSlot> = scheme.iter().map(|a| a.slot).collect();
        assert_eq!(
            order,
            vec![ControlSlot::Jump, ControlSlot::Attack, ControlSlot::Dash]
        );
        assert!(scheme.has_slot(ControlSlot::Jump));
        assert!(!scheme.has_slot(ControlSlot::Blink));
        assert_eq!(
            scheme
                .action_for_slot(ControlSlot::Attack)
                .map(|a| a.id.as_str()),
            Some("attack")
        );
    }

    #[test]
    fn new_enforces_one_action_per_slot() {
        // Two actions claiming the same slot → the constructor normalizes to one,
        // so `action_for_slot` is unambiguous and the slot renders once.
        let mk = |id: &str| ActionSpec {
            id: ActionId::new(id),
            slot: ControlSlot::Attack,
            display_name: None,
            visual: None,
            gate: ActionGate::Move("attack".to_owned()),
        };
        let scheme = ActionSchemeContract::new(vec![mk("swipe"), mk("cleave")]);
        assert_eq!(
            scheme
                .iter()
                .filter(|a| a.slot == ControlSlot::Attack)
                .count(),
            1
        );
    }
}
