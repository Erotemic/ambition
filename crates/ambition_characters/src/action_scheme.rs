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

use ambition_engine_core::{AbilitySet, Edge};
use ambition_entity_catalog::action_scheme::{
    ids, ActionGate, ActionId, ActionSchemeContract, ActionSpec, ControlSlot, CANONICAL_SLOT_ORDER,
};
use ambition_entity_catalog::MovesetContract;
use bevy::prelude::Component;

use crate::actor::control::ActorControlFrame;
use crate::brain::action_set::ActionSet;

/// The Bevy-side carrier of a body's derived [`ActionSchemeContract`]. Mirrors
/// the [`ambition_combat::moveset::ActorMoveset`] pattern: a component wrapping
/// a headless contract. Read by the control-prompt read-model (P2) and, from
/// P3, by the input→action resolution.
#[derive(Component, Debug, Clone, Default)]
pub struct ActorActionScheme(pub ActionSchemeContract);

/// Content-declared movement/action TECHNIQUES a body exposes — the seam by
/// which a demo (Sanic's spin-dash, a ground-pound) gives its bespoke technique
/// an identity in the action scheme: a slot, a display name, and a
/// `Technique`-gated action the on-screen prompt renders. Each entry OVERRIDES
/// any base action on its slot (derivation precedence). The technique's BEHAVIOR
/// stays content code (e.g. `ball_dash`); this only declares "what it is called
/// and where it lives," so the button can't lie about it.
///
/// **Requires [`ResolvedTechniqueEdges`]**: any body that DECLARES a technique
/// gets the routed-edge component for free (Bevy required-components), so the
/// shared resolver always has somewhere to write the technique's edge. Without
/// this a technique-bearing body could silently drop its input on the tick
/// before a separate ensure-system attached the edge component.
#[derive(Component, Debug, Clone, Default)]
#[require(ResolvedTechniqueEdges)]
pub struct ActorTechniques(pub Vec<ActionSpec>);

/// The per-tick resolved edges for the content TECHNIQUES a body's scheme puts on
/// its control slots — the SANCTIONED seam a content technique consumes, in place
/// of intercepting a raw combat verb in a fragile schedule window.
///
/// The shared resolver (the persona gate, [`resolve_worn_control`] semantics)
/// fills this each tick: when a slot's action is [`ActionGate::Technique`], the
/// slot's device edge is routed here under the technique id AND the raw combat
/// verb (e.g. `melee_pressed`) is cleared — so a technique fires ONLY from its
/// keyed edge, and a plain melee edge is no longer the content API. Derived state
/// (rebuilt every tick from the scheme + control), never streamed or snapshotted:
/// a rollback reconstructs it by re-resolving, exactly like the scheme itself.
#[derive(Component, Debug, Clone, Default)]
pub struct ResolvedTechniqueEdges(pub Vec<(String, Edge)>);

impl ResolvedTechniqueEdges {
    /// The edge routed to `id` this tick (`Edge::NONE` if the technique is not on
    /// the scheme / not pressed).
    pub fn edge(&self, id: &str) -> Edge {
        self.0
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, e)| *e)
            .unwrap_or(Edge::NONE)
    }

    /// True iff technique `id` was pressed this tick.
    pub fn pressed(&self, id: &str) -> bool {
        self.edge(id).pressed
    }

    /// Route `edge` to technique `id`, replacing any prior entry this tick.
    pub fn set(&mut self, id: &str, edge: Edge) {
        if let Some(slot) = self.0.iter_mut().find(|(k, _)| k == id) {
            slot.1 = edge;
        } else {
            self.0.push((id.to_owned(), edge));
        }
    }

    /// Clear all routed edges — the resolver rebuilds them from scratch each tick,
    /// so a released technique leaves no stale edge behind.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// A technique the derived scheme declared on a control slot the combat frame
/// has NO device verb for yet — a movement slot (Jump / Dash / Blink / Utility)
/// or Interact. Those need the Phase-3 kernel re-key before a technique can fire
/// from them; until then [`resolve_control_slots`] REFUSES to pretend it wired
/// one, and returns it here so the caller can surface the mistake (a
/// debug-assert) instead of silently discarding the press.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnroutableTechnique {
    pub slot: ControlSlot,
    pub id: String,
}

/// Clear the Attack device verbs (a plain melee edge is not the content API once
/// the Attack slot is a technique, and a slot-less body cannot melee).
fn clear_attack(control: &mut ActorControlFrame) {
    control.melee_pressed = false;
    control.pogo_pressed = false;
    control.attack_axis = ambition_engine_core::Vec2::ZERO;
}

/// Clear the ranged/charge device verbs on the Projectile slot.
fn clear_projectile(control: &mut ActorControlFrame) {
    control.fire = None;
    control.projectile_pressed = false;
    control.projectile_held = false;
    control.projectile_released = false;
}

/// THE per-slot dispatch half of the shared resolver: [`derive_action_scheme`]
/// says *what* each control slot does; this applies that to a body's live combat
/// frame, routing techniques and stripping verbs the body doesn't own.
///
/// For every slot that carries a device verb in [`ActorControlFrame`] (Attack,
/// Special, Projectile, and the shield on QuickAction):
///
/// - **`Technique(id)`** → route the slot's device edge into `edges[id]` and
///   CLEAR the raw verb, so the content system reads the sanctioned edge and a
///   bare melee/special/projectile press is no longer the API.
/// - **`Move`** → keep the verb (the moveset runtime owns it).
/// - **absent** → strip the verb, so a body without the slot cannot fire it.
///   `holds_item` keeps Attack/Projectile alive (a held item repurposes them for
///   throw/use), matching the persona-gate exception; Special has no such reuse.
///
/// A `Technique` declared on a slot WITHOUT a device verb here (a movement or
/// Interact slot) is returned in the [`UnroutableTechnique`] list rather than
/// dropped: those cannot fire until the kernel consumes actions (Phase 3).
///
/// Pure and Bevy-free (takes the frame + edges by reference) so the whole slot
/// matrix is unit-testable without a world; `edges` is cleared first, so a
/// released technique leaves no stale edge behind. The QuickAction shield's
/// *presence* gating (special-key / held-item driven) stays with the caller —
/// this routes only a technique explicitly placed there.
pub fn resolve_control_slots(
    scheme: &ActionSchemeContract,
    control: &mut ActorControlFrame,
    edges: &mut ResolvedTechniqueEdges,
    holds_item: bool,
) -> Vec<UnroutableTechnique> {
    edges.clear();
    let mut unroutable = Vec::new();

    for slot in CANONICAL_SLOT_ORDER {
        let gate = scheme.action_for_slot(slot).map(|a| a.gate.clone());
        match slot {
            ControlSlot::Attack => match gate.as_ref() {
                Some(ActionGate::Technique(id)) => {
                    edges.set(
                        id,
                        Edge {
                            pressed: control.melee_pressed,
                            ..Edge::NONE
                        },
                    );
                    clear_attack(control);
                }
                // Absent AND no held item → strip. Move / held-item → keep.
                None if !holds_item => clear_attack(control),
                _ => {}
            },
            ControlSlot::Special => match gate.as_ref() {
                Some(ActionGate::Technique(id)) => {
                    edges.set(
                        id,
                        Edge {
                            pressed: control.special_pressed,
                            ..Edge::NONE
                        },
                    );
                    control.special_pressed = false;
                }
                // The scheme lacks Special → a special press must not survive.
                None => control.special_pressed = false,
                // Move → the moveset "special" verb owns the press; keep it.
                _ => {}
            },
            ControlSlot::Projectile => match gate.as_ref() {
                Some(ActionGate::Technique(id)) => {
                    edges.set(
                        id,
                        Edge {
                            pressed: control.projectile_pressed,
                            held: control.projectile_held,
                            released: control.projectile_released,
                        },
                    );
                    clear_projectile(control);
                }
                // Absent → strip the resolved ranged request (raw charge verbs are
                // additionally gated by the caller's capability marker). Held item
                // keeps the throw/use path alive.
                None if !holds_item => control.fire = None,
                _ => {}
            },
            ControlSlot::QuickAction => {
                if let Some(ActionGate::Technique(id)) = gate.as_ref() {
                    edges.set(
                        id,
                        Edge {
                            held: control.shield_held,
                            ..Edge::NONE
                        },
                    );
                    control.shield_held = false;
                }
                // Non-technique QuickAction (the shield ability) is governed by the
                // caller's special-key / held-item shield policy.
            }
            // The SUSTAIN slot. A technique bound here is a MODE, not a moment, so
            // the routing differs from every arm above in two ways: the edge
            // carries `held` as well as `pressed`, and neither is cleared off the
            // frame afterwards. Clearing is how a one-shot press is prevented from
            // being consumed twice; a sustained technique has the opposite need —
            // the body's own rules read the level every tick for as long as it is
            // down, so consuming it would end the technique on the frame it began.
            ControlSlot::Modifier => {
                if let Some(ActionGate::Technique(id)) = gate.as_ref() {
                    edges.set(
                        id,
                        Edge {
                            pressed: control.modifier_pressed,
                            held: control.modifier_held,
                            released: false,
                        },
                    );
                }
            }
            // Movement + Interact slots have NO device verb in this frame. A
            // technique placed there has no wired path yet → reject, never drop.
            ControlSlot::Jump
            | ControlSlot::Dash
            | ControlSlot::Blink
            | ControlSlot::Utility
            | ControlSlot::Interact => {
                if let Some(ActionGate::Technique(id)) = gate.as_ref() {
                    unroutable.push(UnroutableTechnique {
                        slot,
                        id: id.clone(),
                    });
                }
            }
        }
    }

    unroutable
}

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

    // ---- The per-slot dispatch resolver (`resolve_control_slots`) ---------------

    /// Build a one-action scheme claiming `slot` with `gate` (or an empty scheme
    /// when `gate` is `None`, i.e. the body does not own the slot).
    fn one_slot_scheme(slot: ControlSlot, gate: Option<ActionGate>) -> ActionSchemeContract {
        match gate {
            Some(gate) => ActionSchemeContract::new(vec![ActionSpec {
                id: ActionId::new("t"),
                slot,
                display_name: None,
                visual: None,
                gate,
            }]),
            None => ActionSchemeContract::default(),
        }
    }

    /// Set the slot's device state hot: the press verb the resolver keeps/strips,
    /// and (for Projectile) the resolved `fire` request the absence-strip clears.
    fn set_hot(control: &mut ActorControlFrame, slot: ControlSlot) {
        use crate::actor::control::ActorFireRequest;
        match slot {
            ControlSlot::Attack => control.melee_pressed = true,
            ControlSlot::Special => control.special_pressed = true,
            ControlSlot::Projectile => {
                control.projectile_pressed = true;
                control.fire = Some(ActorFireRequest::world_space(
                    ambition_engine_core::Vec2::X,
                    1.0,
                ));
            }
            _ => unreachable!("only combat slots carry a device verb"),
        }
    }

    /// The slot's keep/strip OBSERVABLE after the resolver: the melee/special press
    /// verb, or (for Projectile) the resolved ranged `fire` request. The raw
    /// `projectile_*` charge verbs are NOT the resolver's to strip on absence —
    /// the caller's capability-marker block owns those — so Projectile is observed
    /// through `fire`.
    fn slot_kept(control: &ActorControlFrame, slot: ControlSlot) -> bool {
        match slot {
            ControlSlot::Attack => control.melee_pressed,
            ControlSlot::Special => control.special_pressed,
            ControlSlot::Projectile => control.fire.is_some(),
            _ => unreachable!("only combat slots carry a device verb"),
        }
    }

    /// The core dispatch matrix: for each of the three combat slots (Attack,
    /// Projectile, Special), an ABSENT slot strips the verb, a `Move` keeps it,
    /// and a `Technique` routes the device edge AND clears the raw verb.
    #[test]
    fn resolve_control_slots_dispatches_absent_move_and_technique_per_combat_slot() {
        for slot in [
            ControlSlot::Attack,
            ControlSlot::Projectile,
            ControlSlot::Special,
        ] {
            // Each row: (gate, kept-after, is-routed).
            let rows = [
                (None, false, false),
                (Some(ActionGate::Move("v".into())), true, false),
                (Some(ActionGate::Technique("t".into())), false, true),
            ];
            for (gate, kept, routed) in rows {
                let scheme = one_slot_scheme(slot, gate.clone());
                let mut control = ActorControlFrame::default();
                set_hot(&mut control, slot);
                let mut edges = ResolvedTechniqueEdges::default();

                let unroutable = resolve_control_slots(&scheme, &mut control, &mut edges, false);

                assert!(
                    unroutable.is_empty(),
                    "combat slot {slot:?} with {gate:?} must route cleanly, got {unroutable:?}"
                );
                assert_eq!(
                    slot_kept(&control, slot),
                    kept,
                    "{slot:?} with {gate:?}: kept-after == {kept}"
                );
                assert_eq!(
                    edges.pressed("t"),
                    routed,
                    "{slot:?} with {gate:?}: technique edge routed == {routed}"
                );
            }
        }
    }

    /// A held item repurposes the Attack and Projectile verbs (throw / use), so an
    /// ABSENT combat slot must NOT strip them while an item is held. Special has no
    /// such reuse and is always stripped when absent.
    #[test]
    fn held_item_keeps_attack_and_projectile_but_not_special() {
        let empty = ActionSchemeContract::default();
        let mut control = ActorControlFrame::default();
        control.melee_pressed = true;
        control.projectile_pressed = true;
        control.special_pressed = true;
        let mut edges = ResolvedTechniqueEdges::default();

        let unroutable =
            resolve_control_slots(&empty, &mut control, &mut edges, /*holds_item*/ true);

        assert!(unroutable.is_empty());
        assert!(
            control.melee_pressed,
            "held item keeps the throw/attack verb"
        );
        assert!(
            control.projectile_pressed,
            "held item keeps the projectile verb"
        );
        assert!(
            !control.special_pressed,
            "special is stripped even with a held item"
        );
    }

    /// A technique declared on a slot with NO device verb in the combat frame (a
    /// movement or Interact slot) is REJECTED — returned so the caller can
    /// debug-assert — rather than silently swallowed. Those slots wait on the
    /// Phase-3 kernel re-key.
    #[test]
    fn technique_on_a_non_combat_slot_is_rejected_not_dropped() {
        for slot in [
            ControlSlot::Jump,
            ControlSlot::Dash,
            ControlSlot::Blink,
            ControlSlot::Utility,
            ControlSlot::Interact,
        ] {
            let scheme = one_slot_scheme(slot, Some(ActionGate::Technique("warp".into())));
            let mut control = ActorControlFrame::default();
            let mut edges = ResolvedTechniqueEdges::default();

            let unroutable = resolve_control_slots(&scheme, &mut control, &mut edges, false);

            assert_eq!(
                unroutable,
                vec![UnroutableTechnique {
                    slot,
                    id: "warp".to_owned(),
                }],
                "technique on {slot:?} must be reported, not routed"
            );
            assert!(
                !edges.pressed("warp"),
                "an unroutable technique routes NO edge"
            );
        }
    }

    /// The Sanic-shaped content proof, at the resolver level: a `spin_dash`
    /// technique on the Attack slot routes the melee press into
    /// `edges["spin_dash"]` and clears the raw melee verb, so `capture_ball_dash_input`
    /// reads the sanctioned edge and a plain melee press is no longer the API.
    #[test]
    fn spin_dash_technique_routes_the_attack_edge() {
        let spin = ActionSpec {
            id: ActionId::new("spin_dash"),
            slot: ControlSlot::Attack,
            display_name: Some("Spin Dash".into()),
            visual: None,
            gate: ActionGate::Technique("spin_dash".into()),
        };
        // Full Sanic-ish scheme: jump + dash + the spin technique on Attack.
        let ab = abilities(|a| {
            a.jump = true;
            a.dash = true;
        });
        let scheme = derive_action_scheme(&ab, None, None, std::slice::from_ref(&spin));

        let mut control = ActorControlFrame::default();
        control.melee_pressed = true;
        control.pogo_pressed = true;
        let mut edges = ResolvedTechniqueEdges::default();

        let unroutable = resolve_control_slots(&scheme, &mut control, &mut edges, false);

        assert!(unroutable.is_empty());
        assert!(
            edges.pressed("spin_dash"),
            "the rev routes to the technique edge"
        );
        assert!(!control.melee_pressed, "the raw melee verb is cleared");
        assert!(
            !control.pogo_pressed,
            "the pogo verb is cleared with the melee kit"
        );
    }

    /// Declaring a technique auto-attaches [`ResolvedTechniqueEdges`] (Bevy
    /// required-components), so the resolver always has an edge sink — a
    /// technique-bearing body can never silently lose its input for lack of the
    /// component.
    #[test]
    fn declaring_a_technique_auto_attaches_the_edge_component() {
        use bevy::prelude::World;
        let mut world = World::new();
        let e = world.spawn(ActorTechniques(vec![])).id();
        assert!(
            world.get::<ResolvedTechniqueEdges>(e).is_some(),
            "ActorTechniques must pull in ResolvedTechniqueEdges via #[require]"
        );
    }
}
