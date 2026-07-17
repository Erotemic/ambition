//! Materializing each body's [`ActorActionScheme`] — the OBSERVATION CACHE of its
//! derived slot→action scheme.
//!
//! **This component is NOT the authority.** The authoritative slot→gate
//! resolution is the shared [`derive_action_scheme`] called DIRECTLY, each tick,
//! on the body's immediate authorities at BOTH consumers: the gameplay persona
//! gate (`gate_worn_player_control`) and the `ControlPrompt` read-model. Because
//! both re-derive from the same current `AbilitySet` / moveset / `ActionSet` /
//! techniques, the button and what it fires cannot drift — there is no lagged
//! cache on the critical path.
//!
//! What this seam does is materialize that same derivation into a component, as a
//! convenience OBSERVATION for readers that want the resolved scheme without
//! re-deriving (RL observation, possession/debug tooling, the snapshot-coverage
//! ledger). It re-derives whenever a source authority changes and writes back
//! only when the result differs, and — ordered after the authority-mutation step
//! — reflects the CURRENT tick's kit, so `Changed<ActorActionScheme>` stays
//! honest. It is DERIVED state (a pure function of already-snapshotted
//! authorities), so a rollback reconstructs it for free; nothing scheme-shaped is
//! streamed or persisted.
//!
//! Content-declared TECHNIQUES ([`ActorTechniques`], e.g. Sanic's spin-dash) are
//! folded into the derivation, so they claim + label their slot AND the gate
//! routes their sanctioned edge; the technique's BEHAVIOR stays content code.

use ambition_characters::action_scheme::{
    derive_action_scheme, ActorActionScheme, ActorTechniques,
};
use ambition_characters::brain::action_set::ActionSet;
use ambition_platformer_primitives::schedule::{SandboxSet, SimScheduleExt};
use bevy::prelude::*;

use crate::actor::BodyAbilities;
use crate::combat::moveset::ActorMoveset;

/// Re-derive [`ActorActionScheme`] for any body whose authorities changed (or
/// that has no scheme yet). A no-op in steady state — the change-detection
/// guard skips bodies whose `BodyAbilities` / `ActorMoveset` didn't move this
/// tick, and the write is skipped when the derived scheme equals the existing
/// one.
pub fn reconcile_action_schemes(
    mut commands: Commands,
    bodies: Query<(
        Entity,
        Ref<BodyAbilities>,
        Option<Ref<ActorMoveset>>,
        Option<Ref<ActionSet>>,
        Option<Ref<ActorTechniques>>,
        Option<&ActorActionScheme>,
    )>,
) {
    for (entity, abilities, moveset, action_set, techniques, existing) in &bodies {
        let source_changed = abilities.is_changed()
            || moveset.as_ref().is_some_and(|m| m.is_changed())
            || action_set.as_ref().is_some_and(|a| a.is_changed())
            || techniques.as_ref().is_some_and(|t| t.is_changed());
        if existing.is_some() && !source_changed {
            continue;
        }
        // Combat is unioned from the moveset AND the ActionSet (the canonical
        // player still fires ranged/special via the legacy pipeline, so the
        // ActionSet is the authority that says those slots exist); content
        // techniques (Sanic's spin) override the base action on their slot.
        let derived = derive_action_scheme(
            &abilities.abilities,
            moveset.as_ref().map(|m| &m.0),
            action_set.as_deref(),
            techniques.as_ref().map_or(&[], |t| t.0.as_slice()),
        );
        let differs = existing.is_none_or(|s| s.0 != derived);
        if differs {
            commands.entity(entity).insert(ActorActionScheme(derived));
        }
    }
}

/// Wires [`reconcile_action_schemes`] into the sim schedule.
pub struct ActionSchemePlugin;

impl Plugin for ActionSchemePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Ordered AFTER the authority-mutation step (`apply_worn_character_gameplay`
        // rewrites `ActionSet`/`ActorMoveset` on a kit swap), so this observation
        // cache reflects the CURRENT tick's kit — not a one-tick-lagged view. It
        // is not on the drift-critical path (the gate and the prompt both re-derive
        // from the immediate authorities directly), but keeping the cache
        // same-tick-honest means any observer reading `ActorActionScheme` sees the
        // same thing gameplay and the HUD do.
        app.add_systems(
            sim,
            reconcile_action_schemes
                .in_set(SandboxSet::PlayerInput)
                .after(crate::avatar::apply_worn_character_gameplay),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core::AbilitySet;
    use ambition_entity_catalog::action_scheme::ControlSlot;
    use ambition_entity_catalog::MovesetContract;
    use std::collections::BTreeMap;

    fn ability_set(jump: bool, dash: bool) -> AbilitySet {
        let mut a = AbilitySet::default();
        a.jump = jump;
        a.dash = dash;
        a.dodge = false;
        a.blink = false;
        a.fly = false;
        a.shield = false;
        a
    }

    fn attack_moveset() -> ActorMoveset {
        let mut m = MovesetContract::default();
        m.verbs = BTreeMap::from([("attack".to_string(), "swat".to_string())]);
        ActorMoveset(m)
    }

    /// A minimal app that just runs the reconcile system once.
    fn app() -> App {
        let mut app = App::new();
        app.add_systems(Update, reconcile_action_schemes);
        app
    }

    #[test]
    fn attaches_a_scheme_derived_from_abilities_and_moveset() {
        let mut app = app();
        let e = app
            .world_mut()
            .spawn((
                BodyAbilities::new(ability_set(true, true)),
                attack_moveset(),
            ))
            .id();
        app.update();

        let scheme = app
            .world()
            .entity(e)
            .get::<ActorActionScheme>()
            .expect("scheme attached");
        assert!(scheme.0.has_slot(ControlSlot::Jump));
        assert!(scheme.0.has_slot(ControlSlot::Dash));
        assert!(scheme.0.has_slot(ControlSlot::Attack));
        assert!(!scheme.0.has_slot(ControlSlot::Special));
    }

    #[test]
    fn movement_only_body_gets_a_scheme_with_no_combat_slots() {
        let mut app = app();
        let e = app
            .world_mut()
            .spawn(BodyAbilities::new(ability_set(true, false)))
            .id();
        app.update();

        let scheme = app
            .world()
            .entity(e)
            .get::<ActorActionScheme>()
            .expect("scheme attached even without a moveset");
        assert!(scheme.0.has_slot(ControlSlot::Jump));
        assert!(!scheme.0.has_slot(ControlSlot::Attack));
    }

    #[test]
    fn a_content_technique_claims_and_labels_its_slot() {
        use ambition_entity_catalog::action_scheme::{ActionGate, ActionId, ActionSpec};
        let mut app = app();
        let spin = ActionSpec {
            id: ActionId::new("spin_dash"),
            slot: ControlSlot::Attack,
            display_name: Some("Spin Dash".to_owned()),
            visual: None,
            gate: ActionGate::Technique("spin_dash".to_owned()),
        };
        // A movement-only body (empty moveset) with a spin-dash technique: its
        // Attack slot is present and labelled by the technique, not a phantom.
        let e = app
            .world_mut()
            .spawn((
                BodyAbilities::new(ability_set(true, false)),
                ActorTechniques(vec![spin]),
            ))
            .id();
        app.update();

        let scheme = &app.world().entity(e).get::<ActorActionScheme>().unwrap().0;
        let attack = scheme
            .action_for_slot(ControlSlot::Attack)
            .expect("technique claims the Attack slot");
        assert_eq!(attack.display(), "Spin Dash");
        assert_eq!(attack.gate, ActionGate::Technique("spin_dash".to_owned()));
    }

    #[test]
    fn canonical_player_scheme_advertises_every_real_combat_slot() {
        // Built from the REAL default-player authorities, not hand-assembled
        // booleans (the review's requirement): the bundle's moveset (melee Swipe
        // + the folded bubble_shield special) plus the full ActionSet (Swipe +
        // Bolt + bubble_shield). The prompt MUST advertise Attack, Projectile,
        // AND Special — the protagonist fires all three. Ranged still comes only
        // from the ActionSet + legacy projectile pipeline (not the moveset).
        let abilities = AbilitySet::sandbox_all();
        let action_set = crate::avatar::bundles::default_player_action_set(abilities);
        let moveset = crate::combat::moveset::build_actor_moveset(
            None,
            action_set.melee.as_ref(),
            None,
            action_set.special.as_ref(),
        );
        let moveset_ref = moveset.as_ref().expect("player moveset");
        // The special is now a REAL moveset move (the Gate-1 fix): pressing
        // Special fires `move_for_verb("special")`, no longer a phantom slot.
        assert!(
            moveset_ref.verbs.contains_key("special"),
            "bubble_shield is folded into the player moveset as a real special move"
        );
        // Ranged is still NOT in the moveset — it rides the ActionSet + legacy
        // projectile pipeline; the scheme's Projectile slot comes from the union.
        assert!(
            !moveset_ref.verbs.contains_key("ranged"),
            "the player moveset has no ranged verb; the ActionSet carries ranged"
        );

        let scheme = derive_action_scheme(&abilities, moveset.as_ref(), Some(&action_set), &[]);
        assert!(scheme.has_slot(ControlSlot::Attack), "melee -> Attack");
        assert!(
            scheme.has_slot(ControlSlot::Projectile),
            "ranged Bolt -> Projectile"
        );
        assert!(
            scheme.has_slot(ControlSlot::Special),
            "bubble_shield -> Special"
        );
    }

    #[test]
    fn re_derives_when_abilities_change() {
        let mut app = app();
        let e = app
            .world_mut()
            .spawn(BodyAbilities::new(ability_set(true, false)))
            .id();
        app.update();
        assert!(!app
            .world()
            .entity(e)
            .get::<ActorActionScheme>()
            .unwrap()
            .0
            .has_slot(ControlSlot::Dash));

        // Grant dash → the scheme must pick it up on the next reconcile.
        app.world_mut()
            .entity_mut(e)
            .get_mut::<BodyAbilities>()
            .unwrap()
            .abilities
            .dash = true;
        app.update();
        assert!(app
            .world()
            .entity(e)
            .get::<ActorActionScheme>()
            .unwrap()
            .0
            .has_slot(ControlSlot::Dash));
    }

    /// A DERIVATION-LEVEL guard (not the resolver itself): a combat slot is in
    /// the derived scheme IFF the `ActionSet` authority that gates its behavior
    /// says the body has it. Both the gameplay gate (`gate_worn_player_control`)
    /// and the `ControlPrompt` read-model now call the SAME `derive_action_scheme`
    /// on the body's immediate authorities, so this equivalence is what makes the
    /// shared resolver's two consumers agree. The end-to-end, same-tick proof that
    /// they cannot drift across a kit swap lives in
    /// `ambition_sim_view::control_prompt` (`a_same_tick_kit_swap_cannot_drift_...`),
    /// which runs the real gate and the real prompt together; this test just pins
    /// the pure derivation both rely on.
    #[test]
    fn prompt_scheme_and_gameplay_gate_authority_cannot_drift() {
        use ambition_characters::brain::action_set::ActionSet;
        let ab = AbilitySet::sandbox_all();

        // Canonical player: gate keeps melee/ranged/special; scheme shows them.
        let action_set = crate::avatar::bundles::default_player_action_set(ab);
        let moveset = crate::combat::moveset::build_actor_moveset(
            None,
            action_set.melee.as_ref(),
            None,
            action_set.special.as_ref(),
        );
        let scheme = derive_action_scheme(&ab, moveset.as_ref(), Some(&action_set), &[]);
        assert_eq!(
            action_set.melee.is_some(),
            scheme.has_slot(ControlSlot::Attack)
        );
        assert_eq!(
            action_set.ranged.is_some(),
            scheme.has_slot(ControlSlot::Projectile)
        );
        assert_eq!(
            action_set.special.is_some(),
            scheme.has_slot(ControlSlot::Special)
        );

        // A peaceful persona: the gate strips all combat AND the scheme lacks it.
        let peaceful = ActionSet::default();
        let scheme = derive_action_scheme(&ab, None, Some(&peaceful), &[]);
        assert_eq!(
            peaceful.melee.is_some(),
            scheme.has_slot(ControlSlot::Attack)
        );
        assert_eq!(
            peaceful.ranged.is_some(),
            scheme.has_slot(ControlSlot::Projectile)
        );
        assert_eq!(
            peaceful.special.is_some(),
            scheme.has_slot(ControlSlot::Special)
        );
    }
}
