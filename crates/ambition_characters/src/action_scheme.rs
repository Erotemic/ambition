//! Runtime action scheme — deriving a body's [`ActionSchemeContract`] from the
//! SAME authorities that gate its behavior, and carrying it as an ECS
//! component.
//!
//! This is the character-crate half of the pure-data vocabulary in
//! [`ambition_entity_catalog::action_scheme`]: it holds the engine dep
//! (`AbilitySet`) the leaf crate cannot, and turns "what this body can do"
//! into "what each control slot does + is called."
//!
//! Derived, not authored. The scheme is a pure function of already-live
//! authorities — the body's `AbilitySet` (movement actions), its moveset
//! (combat actions), and any content-registered techniques. Because those are
//! all snapshotted state, a rollback reconstructs the tick-correct scheme by
//! re-deriving; nothing scheme-shaped is streamed or persisted (design doc
//! invariant 1). A reconcile system re-derives when a source authority changes
//! (P0 wiring lands with the first consumer in P2).
//!
//! Precedence: movement + combat actions occupy disjoint slots and form the
//! base; a content technique OVERRIDES whatever base action shares its slot
//! (Sanic's spin claims the Attack slot in place of any moveset attack).

use ambition_entity_catalog::action_scheme::{
    ids, ActionGate, ActionId, ActionSchemeContract, ActionSpec, ControlSlot, CANONICAL_SLOT_ORDER,
};
use ambition_entity_catalog::MovesetContract;
use ambition_platformer2d_core::{AbilitySet, Edge};
use bevy::prelude::Component;

use crate::actor::control::ActorControlFrame;
use crate::brain::action_set::ActionSet;

/// The Bevy-side carrier of a body's derived [`ActionSchemeContract`]. Mirrors
/// the `ActorMoveset` (`ambition_combat::moveset`) pattern: a component wrapping
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
/// Requires [`ResolvedTechniqueEdges`]: any body that DECLARES a technique
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

/// A technique the derived scheme declared on a control slot the combat frame has NO device
/// verb for — a movement slot (Jump / Burst / Blink) or Interact.
///
/// Sanic's form toggle then reached around the resolver for the raw verb because declaring a
/// technique there would have been rejected, which left its control wearing the engine's generic
/// "Fly Toggle" label. See the [`ControlSlot::Utility`] arm of [`resolve_control_slots`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnroutableTechnique {
    pub slot: ControlSlot,
    pub id: String,
}

/// Clear the Attack device verbs (a plain melee edge is not the content API once
/// the Attack slot is a technique, and a slot-less body cannot melee).
fn clear_attack(control: &mut ActorControlFrame) {
    control.melee_pressed = false;
    control.melee_held = false;
    control.melee_released = false;
    control.melee_strong_hint = false;
    control.pogo_pressed = false;
    control.attack_axis = ambition_platformer2d_core::LocalAxes::ZERO;
}

/// Clear the ranged/charge device verbs on the Projectile slot.
fn clear_projectile(control: &mut ActorControlFrame) {
    control.fire = None;
    control.projectile_pressed = false;
    control.projectile_held = false;
    control.projectile_released = false;
}

/// Apply an [`ActionSchemeContract`] to a live [`ActorControlFrame`].
///
/// `Technique(id)` routes a slot's device edge into `edges[id]` and clears the
/// raw verb; `Move` leaves the verb for the moveset runtime; absent slots strip
/// verbs the body does not own. Held items retain the attack/projectile/shield
/// verbs they repurpose. Techniques on slots with no device verb are returned as
/// [`UnroutableTechnique`] rather than dropped. `edges` is cleared before each
/// resolution so released techniques cannot leave stale state.
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
                            held: control.melee_held,
                            released: control.melee_released,
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
            // THE GUARD, and it asks the same question every other slot does.
            //
            //  the capability is `AbilitySet::shield`, which `movement_actions`
            // already turns into this slot. Absent slot → no guard; present slot →
            // the kernel's `resolve_shield` decides the rest. A held item keeps the
            // verb alive exactly as it does on Attack, because shield+attack is the
            // universal throw gesture.
            // Grab is stripped when the slot is absent, and never re-routed.
            // A body whose scheme has no Grab slot must not be able to attempt a
            // capture, which is what makes `AbilitySet::grab` a real permission
            // rather than a label — the same shape as Attack and Shield above.
            //
            //  no `ActionGate::Technique` arm, deliberately. Every other combat
            // slot can host a content technique on its press; a capture is not a
            // technique the press invokes, it is an authored MOVE whose active
            // window may establish a relationship. Routing the edge to a
            // technique id here would give the same button two meanings and let
            // content silently replace the capture with something that is not
            // one. When a customer wants a technique on this slot, it arrives
            // with the case that needs it.
            ControlSlot::Grab => {
                if gate.is_none() && !holds_item {
                    control.grab_pressed = false;
                }
            }
            // Same shape as Grab for the same reason: the press invokes an
            // authored MOVE, so binding a technique here would give one button
            // two meanings.
            ControlSlot::Taunt => {
                if gate.is_none() && !holds_item {
                    control.taunt_pressed = false;
                }
            }
            ControlSlot::Shield => match gate.as_ref() {
                Some(ActionGate::Technique(id)) => {
                    edges.set(
                        id,
                        Edge {
                            held: control.shield_held,
                            ..Edge::NONE
                        },
                    );
                    control.shield_held = false;
                }
                None if !holds_item => control.shield_held = false,
                _ => {}
            },
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
            // The MODE-SWITCH slot, whose device verb is `fly_toggle_pressed`.
            // Named for the engine's own fly toggle because that was the first
            // thing to claim it, but the slot is generic — "flip this body into
            // its other mode" — and a content technique bound here (Sanic's
            // transformation) is the same shape as the base action it replaces.
            //
            // A non-technique gate is left alone, exactly as on `Modifier`: an
            // absent Utility slot must not strip the verb, because a body with
            // flight and no technique still steers its own fly toggle through it.
            ControlSlot::Utility => {
                if let Some(ActionGate::Technique(id)) = gate.as_ref() {
                    edges.set(
                        id,
                        Edge {
                            pressed: control.fly_toggle_pressed,
                            ..Edge::NONE
                        },
                    );
                    control.fly_toggle_pressed = false;
                }
            }
            // Movement + Interact slots have NO device verb in this frame. A
            // technique placed there has no wired path yet → reject, never drop.
            ControlSlot::Jump | ControlSlot::Burst | ControlSlot::Blink | ControlSlot::Interact => {
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

/// One movement CAPABILITY → (slot, action-id, movement-action-id) mapping. The
/// bool is read off the `AbilitySet`; only enabled ones become actions, so a
/// body simply lacks a slot for a capability it doesn't have (no phantom
/// buttons, no post-hoc stripping).
fn movement_actions(abilities: &AbilitySet) -> Vec<ActionSpec> {
    // The burst button's player-facing word, and it follows the KERNEL'S
    // PRECEDENCE rather than a preference.
    //
    // The slot is `Burst` because dodge and dash are one press — but "Burst" is
    // engine vocabulary and no player has ever pressed one. Naming it after the
    // channel would trade a wrong word (`Dash` on a fighter that cannot dash)
    // for a meaningless one on every body.
    //
    //  `resolve_burst_maneuver` asks `available_dodge` FIRST and only reaches
    // `dash_available` when no dodge is on offer, so a body owning both mostly
    // DODGES when this is pressed. The label says the same thing the kernel
    // does, which is why it is derived from that order and not chosen.
    //
    //  this is a per-BODY fact, not a per-position one: it depends on which
    // capabilities the body owns, so it is stable while the player moves. That
    // is the distinction that makes it acceptable under `PromptNaming::ByMove`
    // where naming an attack slot after its currently-resolvable move is not.
    let burst_word = if abilities.dodge { "Dodge" } else { "Dash" };
    // (has-capability, slot, id) — id doubles as the movement-action gate string.
    let table: [(bool, ControlSlot, &str); 5] = [
        (abilities.jump, ControlSlot::Jump, ids::JUMP),
        (
            abilities.dash || abilities.dodge,
            ControlSlot::Burst,
            ids::BURST,
        ),
        (abilities.blink, ControlSlot::Blink, ids::BLINK),
        (
            abilities.fly && abilities.fly_toggle,
            ControlSlot::Utility,
            ids::FLY_TOGGLE,
        ),
        (abilities.shield, ControlSlot::Shield, ids::SHIELD),
    ];
    table
        .into_iter()
        .filter(|(has, _, _)| *has)
        .map(|(_, slot, id)| ActionSpec {
            id: ActionId::new(id),
            slot,
            // Only the burst row needs one; every other movement id title-cases
            // into the word a player already uses ("jump" -> "Jump").
            display_name: (slot == ControlSlot::Burst).then(|| burst_word.to_owned()),
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

/// Resolve combat actions from the authorities that currently execute them.
///
/// Melee moves come from the moveset but require `AbilitySet::attack`; ranged and
/// special slots may still come from `ActionSet`. Labels prefer authored moveset
/// labels and otherwise use the verb id.
/// TODO(compat-remove): fold ranged/special execution into the moveset resolver,
/// then remove the `ActionSet` combat union here.
fn combat_actions(
    abilities: &AbilitySet,
    moveset: Option<&MovesetContract>,
    action_set: Option<&ActionSet>,
) -> Vec<ActionSpec> {
    let has_verb = |verb: &str| moveset.is_some_and(|m| m.verbs.contains_key(verb));
    let has_directional_verb = |base: &str| {
        let prefix = format!("{base}_");
        moveset.is_some_and(|m| {
            m.verbs
                .keys()
                .any(|verb| verb == base || verb.starts_with(&prefix))
        })
    };
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
        abilities.attack
            && (has_directional_verb(ids::ATTACK) || action_set.is_some_and(|a| a.melee.is_some())),
        ControlSlot::Attack,
        ids::ATTACK,
    );
    push(
        has_verb(ids::RANGED) || action_set.is_some_and(|a| a.ranged.is_some()),
        ControlSlot::Projectile,
        ids::RANGED,
    );
    push(
        abilities.attack
            && (has_directional_verb(ids::SPECIAL)
                || action_set.is_some_and(|a| a.special.is_some())),
        ControlSlot::Special,
        ids::SPECIAL,
    );
    // Grab requires both capability permission and an authored exact `grab`
    // verb. Throws are capture-context moves, not directional grab variants.
    push(
        abilities.grab && has_verb(ids::GRAB),
        ControlSlot::Grab,
        ids::GRAB,
    );
    // No permission term, deliberately. A taunt is content and nothing else
    // — it grants no reach and threatens nobody — so a body gets the button on
    // the day it authors the move, exactly like the ranged slot above.
    push(
        has_directional_verb(ids::TAUNT),
        ControlSlot::Taunt,
        ids::TAUNT,
    );
    out
}

/// Derive a body's action scheme from its live authorities.
///
/// - Movement actions from the `AbilitySet` (jump/dash/blink/fly/shield).
/// - Interact when the body's `AbilitySet` grants it. Not universal: a
///   restricted kit (`RunJump`) has no talk verb, so no button is drawn for one.
/// - Combat actions unioned from the moveset AND the `ActionSet`, with the
///   melee family (Attack / Special) CEILINGED by `AbilitySet::attack` — see
///   [`combat_actions`] for why a table is not a permission.
/// - Techniques (content-declared, already `Technique`-gated `ActionSpec`s)
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

    // Interact is a CAPABILITY now, not a universal. The prompt (Talk / Open / …) still
    // resolves against nearby interactables at press time — that was always the world half.
    if abilities.interact {
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
    }

    for spec in combat_actions(abilities, moveset, action_set) {
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
    use ambition_platformer2d_core::AbilityGrant;
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
        a.grab = false;
        f(&mut a);
        a
    }

    fn moveset(verbs: &[&str]) -> MovesetContract {
        let mut m = MovesetContract::default();
        m.moves = verbs
            .iter()
            .map(|v| MoveSpec {
                id: (*v).to_string(),
                display_name: None,
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
                charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
                smash_charge: None,
                repeat: None,
                landing_lag_s: None,
                autocancel_after_s: None,
                sprite_spin_hz: None,
                equips: None,
            })
            .collect();
        m.verbs = verbs
            .iter()
            .map(|v| ((*v).to_string(), (*v).to_string()))
            .collect::<BTreeMap<_, _>>();
        m
    }

    /// An authored move label overrides the title-cased id in the control prompt.
    #[test]
    fn an_authored_move_label_beats_the_title_cased_id() {
        let mut m = moveset(&["attack"]);
        m.moves[0].display_name = Some("Down Air".to_string());
        let scheme = combat_actions(&abilities(|a| a.attack = true), Some(&m), None);
        let attack = scheme
            .iter()
            .find(|a| a.slot == ControlSlot::Attack)
            .expect("the attack slot is claimed");
        assert_eq!(attack.display_name.as_deref(), Some("Down Air"));

        let plain = combat_actions(
            &abilities(|a| a.attack = true),
            Some(&moveset(&["attack"])),
            None,
        );
        assert_eq!(
            plain
                .iter()
                .find(|a| a.slot == ControlSlot::Attack)
                .and_then(|a| a.display_name.as_deref()),
            Some("Attack"),
        );
    }

    fn slots(scheme: &ActionSchemeContract) -> Vec<ControlSlot> {
        scheme.iter().map(|a| a.slot).collect()
    }

    /// The Grab slot requires both permission and an authored exact move.
    /// Exercise all four truth-table cases so `&&` cannot regress to `||`.
    #[test]
    fn the_grab_slot_needs_the_permission_and_the_authored_move() {
        let with_grab = moveset(&["attack", "grab"]);
        let without = moveset(&["attack"]);

        let permitted = abilities(|a| {
            a.attack = true;
            a.grab = true;
        });
        let unpermitted = abilities(|a| a.attack = true);

        assert!(
            derive_action_scheme(&permitted, Some(&with_grab), None, &[])
                .has_slot(ControlSlot::Grab),
            "a body granted the verb AND authoring the move has no Grab slot"
        );
        assert!(
            !derive_action_scheme(&permitted, Some(&without), None, &[])
                .has_slot(ControlSlot::Grab),
            "the fighter KIT alone invented a grab for a character that authored \
             none — every fighter would advertise a button with no move behind it"
        );
        assert!(
            !derive_action_scheme(&unpermitted, Some(&with_grab), None, &[])
                .has_slot(ControlSlot::Grab),
            "an authored grab table armed itself without the ruleset granting the \
             verb — that is a crossover move leaking into a character's home game"
        );
        assert!(
            !derive_action_scheme(&unpermitted, Some(&without), None, &[])
                .has_slot(ControlSlot::Grab),
            "neither half present and the slot appeared anyway"
        );
    }

    /// A BODY WITH NO GRAB SLOT CANNOT ATTEMPT ONE.
    ///
    /// The permission above is only real if the resolver strips the edge. A
    /// scheme is advisory to a HUD; `resolve_control_slots` is what makes it
    /// binding on the body, and a grab that survived slot resolution would let
    /// any brain or pad reach a capture the character was never granted.
    #[test]
    fn slot_resolution_strips_a_grab_the_scheme_does_not_carry() {
        let mut control = ActorControlFrame::default();
        control.grab_pressed = true;
        let scheme = one_slot_scheme(ControlSlot::Attack, Some(ActionGate::Move("attack".into())));
        let mut edges = ResolvedTechniqueEdges::default();
        resolve_control_slots(&scheme, &mut control, &mut edges, false);
        assert!(
            !control.grab_pressed,
            "the capture edge survived a scheme with no Grab slot"
        );
    }

    #[test]
    fn full_kit_body_yields_canonical_full_scheme() {
        let ab = abilities(|a| {
            a.jump = true;
            a.dash = true;
            a.blink = true;
            // A FULL kit has been granted the melee verb; the moveset only says
            // what the swing is (see `combat_actions`).
            a.attack = true;
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
                ControlSlot::Burst,
                ControlSlot::Blink,
                ControlSlot::Interact,
            ]
        );
    }

    #[test]
    fn directional_only_moves_still_claim_their_control_slots() {
        let ab = abilities(|a| a.attack = true);
        let ms = moveset(&["attack_forward", "special_air_up"]);
        let scheme = derive_action_scheme(&ab, Some(&ms), None, &[]);
        assert!(scheme.has_slot(ControlSlot::Attack));
        assert!(scheme.has_slot(ControlSlot::Special));
    }

    #[test]
    fn permanent_free_flight_has_interact_but_no_jump_or_toggle_button() {
        let ab = AbilityGrant::FreeFlight.to_set();
        let scheme = derive_action_scheme(&ab, None, None, &[]);
        assert_eq!(slots(&scheme), vec![ControlSlot::Interact]);
        assert!(!scheme.has_slot(ControlSlot::Jump));
        assert!(!scheme.has_slot(ControlSlot::Utility));
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
            vec![ControlSlot::Jump, ControlSlot::Burst, ControlSlot::Interact]
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
        //
        // The last row is Mary-O at home — a full smash table on a `RunJump` body — and it must
        // show no Attack and no Special. Projectile takes only the verb: no ability flag describes
        // ranged (see [`combat_actions`]).
        for (jump, dash, blink, may_attack, verbs) in [
            (true, false, false, false, vec![]),
            (true, true, true, true, vec!["attack"]),
            (false, true, false, true, vec!["special", "ranged"]),
            (
                true,
                false,
                false,
                false,
                vec!["attack", "special", "ranged"],
            ),
        ] {
            let ab = abilities(|a| {
                a.jump = jump;
                a.dash = dash;
                a.blink = blink;
                a.attack = may_attack;
            });
            let ms = moveset(&verbs);
            let scheme = derive_action_scheme(&ab, Some(&ms), None, &[]);

            assert_eq!(scheme.has_slot(ControlSlot::Jump), jump);
            assert_eq!(scheme.has_slot(ControlSlot::Burst), dash);
            assert_eq!(scheme.has_slot(ControlSlot::Blink), blink);
            assert_eq!(
                scheme.has_slot(ControlSlot::Attack),
                may_attack && verbs.contains(&"attack")
            );
            assert_eq!(
                scheme.has_slot(ControlSlot::Special),
                may_attack && verbs.contains(&"special")
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
                    ambition_platformer2d_core::Vec2::X,
                    1.0,
                ));
            }
            ControlSlot::Shield => control.shield_held = true,
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
            ControlSlot::Shield => control.shield_held,
            _ => unreachable!("only combat slots carry a device verb"),
        }
    }

    /// The core dispatch matrix: for each slot that carries a device verb (Attack,
    /// Projectile, Special, Shield), an ABSENT slot strips the verb, a `Move` keeps
    /// it, and a `Technique` routes the device edge AND clears the raw verb.
    #[test]
    fn resolve_control_slots_dispatches_absent_move_and_technique_per_combat_slot() {
        for slot in [
            ControlSlot::Attack,
            ControlSlot::Projectile,
            ControlSlot::Special,
            ControlSlot::Shield,
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
                //  the Shield slot routes a HELD level, not a press edge — a
                // guard is a sustain, so `pressed` would read `false` on a
                // correctly routed shield technique.
                let edge = edges.edge("t");
                let observed = if slot == ControlSlot::Shield {
                    edge.held
                } else {
                    edge.pressed
                };
                assert_eq!(
                    observed, routed,
                    "{slot:?} with {gate:?}: technique edge routed == {routed}"
                );
            }
        }
    }

    #[test]
    fn attack_technique_receives_press_hold_and_release() {
        let scheme = one_slot_scheme(
            ControlSlot::Attack,
            Some(ActionGate::Technique("charge".into())),
        );
        let mut control = ActorControlFrame::default();
        control.melee_pressed = true;
        control.melee_held = true;
        control.melee_released = true;
        let mut edges = ResolvedTechniqueEdges::default();

        let unroutable = resolve_control_slots(&scheme, &mut control, &mut edges, false);

        assert!(unroutable.is_empty());
        assert_eq!(
            edges.edge("charge"),
            Edge {
                pressed: true,
                held: true,
                released: true,
            }
        );
        assert!(!control.melee_pressed);
        assert!(!control.melee_held);
        assert!(!control.melee_released);
    }

    /// A held item repurposes the Attack and Projectile verbs (throw / use) and
    /// shield+attack IS the throw gesture, so an ABSENT Attack / Projectile /
    /// Shield slot must NOT strip its verb while an item is held. Special has no
    /// such reuse and is always stripped when absent.
    #[test]
    fn held_item_keeps_attack_projectile_and_shield_but_not_special() {
        let empty = ActionSchemeContract::default();
        let mut control = ActorControlFrame::default();
        control.melee_pressed = true;
        control.projectile_pressed = true;
        control.special_pressed = true;
        control.shield_held = true;
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
            control.shield_held,
            "held item keeps the shield half of the throw gesture"
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
            ControlSlot::Burst,
            ControlSlot::Blink,
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

    /// A technique on the mode-switch slot routes, and EATS the fly toggle.
    ///
    /// Both halves matter and they are one mechanism. A body that names its own
    /// Utility action gets the press on its sanctioned edge (so it can stop
    /// reading `fly_toggle_pressed` behind the resolver's back), and the raw verb
    /// is consumed (so the same press cannot ALSO flip the generic flight mode on
    /// a body that happens to have wings).
    ///
    /// The fixture gives the body flight deliberately: `movement_actions` would
    /// otherwise leave Utility empty, and the test would pass without proving the
    /// technique OVERRODE anything.
    #[test]
    fn technique_on_the_utility_slot_routes_and_consumes_the_mode_switch_edge() {
        let morph = ActionSpec {
            id: ActionId::new("morph"),
            slot: ControlSlot::Utility,
            display_name: None,
            visual: None,
            gate: ActionGate::Technique("morph".into()),
        };
        let ab = abilities(|a| {
            a.jump = true;
            a.fly = true;
            a.fly_toggle = true;
        });
        let scheme = derive_action_scheme(&ab, None, None, std::slice::from_ref(&morph));

        let mut control = ActorControlFrame::default();
        control.fly_toggle_pressed = true;
        let mut edges = ResolvedTechniqueEdges::default();

        let unroutable = resolve_control_slots(&scheme, &mut control, &mut edges, false);

        assert!(
            unroutable.is_empty(),
            "a technique on Utility has a wired path, got {unroutable:?}"
        );
        assert!(
            edges.pressed("morph"),
            "the mode-switch press routes to the technique edge"
        );
        assert!(
            !control.fly_toggle_pressed,
            "the raw verb is consumed, so the press cannot also toggle generic flight"
        );
    }

    /// The other side of the arm above: a body whose Utility slot is the ENGINE's
    /// own fly toggle keeps its verb. Absent-slot stripping is not this arm's job
    /// (the flight limb gates on the ability), and a resolver that ate the verb
    /// unconditionally would silently disable every flyer.
    #[test]
    fn a_non_technique_utility_slot_keeps_its_device_verb() {
        let ab = abilities(|a| {
            a.fly = true;
            a.fly_toggle = true;
        });
        let scheme = derive_action_scheme(&ab, None, None, &[]);

        let mut control = ActorControlFrame::default();
        control.fly_toggle_pressed = true;
        let mut edges = ResolvedTechniqueEdges::default();

        resolve_control_slots(&scheme, &mut control, &mut edges, false);

        assert!(
            control.fly_toggle_pressed,
            "the generic fly toggle owns its own verb and must survive the resolver"
        );
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
