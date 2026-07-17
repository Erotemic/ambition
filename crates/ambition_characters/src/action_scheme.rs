//! Runtime action scheme — deriving a body's [`ActionSchemeContract`] from the
//! SAME authorities that gate its behavior, and carrying it as an ECS
//! component.
//!
//! This is the character-crate half of the pure-data vocabulary in
//! [`ambition_entity_catalog::action_scheme`]: it holds the engine dep
//! (`AbilitySet`) the leaf crate cannot, and turns "what this body can do"
//! into "what each control slot does + is called."
//!
//! **Derived, not authored.** The scheme is a pure function of already-live
//! authorities — the body's `AbilitySet` (movement actions), its moveset
//! (combat actions), and any content-registered techniques. Because those are
//! all snapshotted state, a rollback reconstructs the tick-correct scheme by
//! re-deriving; nothing scheme-shaped is streamed or persisted (design doc
//! invariant 1). A reconcile system re-derives when a source authority changes
//! (P0 wiring lands with the first consumer in P2).
//!
//! **Precedence:** movement + combat actions occupy disjoint slots and form the
//! base; a content technique OVERRIDES whatever base action shares its slot
//! (Sanic's spin claims the Attack slot in place of any moveset attack).

use ambition_engine_core::AbilitySet;
use ambition_entity_catalog::action_scheme::{
    ids, ActionGate, ActionId, ActionSchemeContract, ActionSpec, ControlSlot,
};
use ambition_entity_catalog::MovesetContract;
use bevy::prelude::Component;

use crate::brain::action_set::ActionSet;

/// The Bevy-side carrier of a body's derived [`ActionSchemeContract`]. Mirrors
/// the [`ambition_combat::moveset::ActorMoveset`] pattern: a component wrapping
/// a headless contract. Read by the control-prompt read-model (P2) and, from
/// P3, by the input→action resolution.
#[derive(Component, Debug, Clone, Default)]
pub struct ActorActionScheme(pub ActionSchemeContract);

/// One movement ability → (slot, action-id, movement-action-id) mapping. The
/// bool is read off the `AbilitySet`; only enabled ones become actions, so a
/// body simply lacks a slot for a capability it doesn't have (no phantom
/// buttons, no post-hoc stripping).
fn movement_actions(abilities: &AbilitySet) -> Vec<ActionSpec> {
    // (has-ability, slot, id) — id doubles as the movement-action gate string.
    let table: [(bool, ControlSlot, &str); 5] = [
        (abilities.jump, ControlSlot::Jump, ids::JUMP),
        (
            abilities.dash || abilities.dodge,
            ControlSlot::Dash,
            ids::DASH,
        ),
        (abilities.blink, ControlSlot::Blink, ids::BLINK),
        (abilities.fly, ControlSlot::Utility, ids::FLY_TOGGLE),
        (abilities.shield, ControlSlot::QuickAction, ids::SHIELD),
    ];
    table
        .into_iter()
        .filter(|(has, _, _)| *has)
        .map(|(_, slot, id)| ActionSpec {
            id: ActionId::new(id),
            slot,
            display_name: None,
            visual: None,
            gate: ActionGate::Movement(id.to_owned()),
        })
        .collect()
}

/// Insert `spec`, replacing any existing action that shares its slot (one
/// action per slot; later inserts win — the precedence lever).
fn upsert(actions: &mut Vec<ActionSpec>, spec: ActionSpec) {
    actions.retain(|a| a.slot != spec.slot);
    actions.push(spec);
}

/// Combat actions unioned from BOTH authorities a body actually fires through:
/// the moveset (attack + its authored labels) AND the `ActionSet` (ranged /
/// special capability, which for the canonical player still fires via the
/// legacy projectile / shield pipeline rather than the moveset). A combat slot
/// is present if EITHER authority provides it — so the prompt can never
/// advertise the protagonist as lacking Projectile / Special when those verbs
/// actually work. Labels come from the moveset move when the verb is authored
/// there, else the title-cased verb id.
///
/// (The genuine one-authority unification — folding ranged/special into the
/// moveset and deleting the legacy paths — lands with the shared resolver; this
/// union is the honest, non-lying interim that matches real behavior.)
fn combat_actions(
    moveset: Option<&MovesetContract>,
    action_set: Option<&ActionSet>,
) -> Vec<ActionSpec> {
    let has_verb = |verb: &str| moveset.is_some_and(|m| m.verbs.contains_key(verb));
    let move_label = |verb: &str| {
        moveset
            .and_then(|m| m.move_for_verb(verb))
            .map(|mv| mv.display())
    };

    let mut out = Vec::new();
    let mut push = |present: bool, slot: ControlSlot, verb: &str| {
        if present {
            out.push(ActionSpec {
                id: ActionId::new(verb),
                slot,
                display_name: move_label(verb),
                visual: None,
                gate: ActionGate::Move(verb.to_owned()),
            });
        }
    };
    push(
        has_verb(ids::ATTACK) || action_set.is_some_and(|a| a.melee.is_some()),
        ControlSlot::Attack,
        ids::ATTACK,
    );
    push(
        has_verb(ids::RANGED) || action_set.is_some_and(|a| a.ranged.is_some()),
        ControlSlot::Projectile,
        ids::RANGED,
    );
    push(
        has_verb(ids::SPECIAL) || action_set.is_some_and(|a| a.special.is_some()),
        ControlSlot::Special,
        ids::SPECIAL,
    );
    out
}

/// Derive a body's action scheme from its live authorities.
///
/// - **Movement** actions from the `AbilitySet` (jump/dash/blink/fly/shield).
/// - **Interact** is universal — every controllable body can attempt it.
/// - **Combat** actions unioned from the moveset AND the `ActionSet`
///   (see [`combat_actions`]).
/// - **Techniques** (content-declared, already `Technique`-gated `ActionSpec`s)
///   are layered last and OVERRIDE any base action on the same slot.
///
/// The result is canonically ordered (deterministic iteration).
pub fn derive_action_scheme(
    abilities: &AbilitySet,
    moveset: Option<&MovesetContract>,
    action_set: Option<&ActionSet>,
    techniques: &[ActionSpec],
) -> ActionSchemeContract {
    let mut actions = movement_actions(abilities);

    // Interact is available to every controllable subject; the actual prompt
    // (Talk / Open / …) resolves against nearby interactables at press time.
    upsert(
        &mut actions,
        ActionSpec {
            id: ActionId::new(ids::INTERACT),
            slot: ControlSlot::Interact,
            display_name: None,
            visual: None,
            gate: ActionGate::Interact,
        },
    );

    for spec in combat_actions(moveset, action_set) {
        upsert(&mut actions, spec);
    }

    for technique in techniques {
        upsert(&mut actions, technique.clone());
    }

    ActionSchemeContract::new(actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::action_scheme::{ActionGate, VisualId};
    use ambition_entity_catalog::{ClipBinding, MoveSpec};
    use std::collections::BTreeMap;

    fn abilities(f: impl FnOnce(&mut AbilitySet)) -> AbilitySet {
        let mut a = AbilitySet::default();
        // Default may carry a baseline; zero the movement flags we assert on so
        // each test states its own capability profile explicitly.
        a.jump = false;
        a.dash = false;
        a.dodge = false;
        a.blink = false;
        a.fly = false;
        a.shield = false;
        f(&mut a);
        a
    }

    fn moveset(verbs: &[&str]) -> MovesetContract {
        let mut m = MovesetContract::default();
        m.moves = verbs
            .iter()
            .map(|v| MoveSpec {
                id: (*v).to_string(),
                clip: ClipBinding {
                    clip: (*v).to_string(),
                    fallbacks: vec![],
                },
                duration_s: 0.3,
                windows: vec![],
                events: vec![],
                gates: Default::default(),
                start_impulse: None,
                smash_charge_mult: 1.0,
            })
            .collect();
        m.verbs = verbs
            .iter()
            .map(|v| ((*v).to_string(), (*v).to_string()))
            .collect::<BTreeMap<_, _>>();
        m
    }

    fn slots(scheme: &ActionSchemeContract) -> Vec<ControlSlot> {
        scheme.iter().map(|a| a.slot).collect()
    }

    #[test]
    fn full_kit_body_yields_canonical_full_scheme() {
        let ab = abilities(|a| {
            a.jump = true;
            a.dash = true;
            a.blink = true;
        });
        let ms = moveset(&["attack", "special", "ranged"]);
        let scheme = derive_action_scheme(&ab, Some(&ms), None, &[]);
        assert_eq!(
            slots(&scheme),
            vec![
                ControlSlot::Jump,
                ControlSlot::Attack,
                ControlSlot::Special,
                ControlSlot::Projectile,
                ControlSlot::Dash,
                ControlSlot::Blink,
                ControlSlot::Interact,
            ]
        );
    }

    #[test]
    fn movement_only_body_has_no_phantom_combat_slots() {
        // Sanic-shaped: jump + dash, empty moveset. No Attack/Special/Projectile.
        let ab = abilities(|a| {
            a.jump = true;
            a.dash = true;
        });
        let scheme = derive_action_scheme(&ab, None, None, &[]);
        assert_eq!(
            slots(&scheme),
            vec![ControlSlot::Jump, ControlSlot::Dash, ControlSlot::Interact]
        );
        assert!(!scheme.has_slot(ControlSlot::Attack));
        assert!(!scheme.has_slot(ControlSlot::Special));
    }

    #[test]
    fn technique_overrides_the_base_action_on_its_slot() {
        // A body with a moveset attack AND a spin technique on the Attack slot:
        // the technique wins, and it keeps its authored label.
        let ab = abilities(|a| a.jump = true);
        let ms = moveset(&["attack"]);
        let spin = ActionSpec {
            id: ActionId::new("spin_dash"),
            slot: ControlSlot::Attack,
            display_name: Some("Spin Dash".to_owned()),
            visual: Some(VisualId("icon.spin".to_owned())),
            gate: ActionGate::Technique("spin_dash".to_owned()),
        };
        let scheme = derive_action_scheme(&ab, Some(&ms), None, std::slice::from_ref(&spin));
        let attack = scheme
            .action_for_slot(ControlSlot::Attack)
            .expect("attack slot claimed");
        assert_eq!(attack.gate, ActionGate::Technique("spin_dash".to_owned()));
        assert_eq!(attack.display(), "Spin Dash");
        // Exactly one action on the slot — the moveset attack was replaced.
        assert_eq!(
            scheme
                .iter()
                .filter(|a| a.slot == ControlSlot::Attack)
                .count(),
            1
        );
    }

    #[test]
    fn scheme_presence_equals_behavior_availability() {
        // The parity guard for the P0→P3 window: a slot is in the scheme IFF the
        // authority that gates its behavior says the body has it. If these ever
        // diverge, the prompt would advertise an action the body can't perform.
        for (jump, dash, blink, verbs) in [
            (true, false, false, vec![]),
            (true, true, true, vec!["attack"]),
            (false, true, false, vec!["special", "ranged"]),
        ] {
            let ab = abilities(|a| {
                a.jump = jump;
                a.dash = dash;
                a.blink = blink;
            });
            let ms = moveset(&verbs);
            let scheme = derive_action_scheme(&ab, Some(&ms), None, &[]);

            assert_eq!(scheme.has_slot(ControlSlot::Jump), jump);
            assert_eq!(scheme.has_slot(ControlSlot::Dash), dash);
            assert_eq!(scheme.has_slot(ControlSlot::Blink), blink);
            assert_eq!(
                scheme.has_slot(ControlSlot::Attack),
                verbs.contains(&"attack")
            );
            assert_eq!(
                scheme.has_slot(ControlSlot::Special),
                verbs.contains(&"special")
            );
            assert_eq!(
                scheme.has_slot(ControlSlot::Projectile),
                verbs.contains(&"ranged")
            );
        }
    }
}
