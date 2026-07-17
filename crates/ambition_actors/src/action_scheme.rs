//! Attaching + reconciling each body's [`ActorActionScheme`] from its live
//! authorities.
//!
//! The scheme is DERIVED state (see
//! [`ambition_characters::action_scheme`]): a pure function of the body's
//! effective `AbilitySet` ([`BodyAbilities`]) and its moveset ([`ActorMoveset`]).
//! This module is the reconcile seam — it re-derives the scheme whenever a
//! source authority changes (equipment grant, ability mask, moveset swap) and
//! writes it back only when the result actually differs, so a rollback that
//! restores the authorities reconstructs the tick-correct scheme for free and
//! `Changed<ActorActionScheme>` stays honest for downstream readers.
//!
//! Content-declared TECHNIQUES ([`ActorTechniques`], e.g. Sanic's spin-dash) are
//! folded in too: they give a bespoke content mechanic an identity + label on a
//! control slot so the on-screen prompt can name it. The technique's BEHAVIOR
//! stays content code; this seam only carries its declaration.

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

/// Wires [`reconcile_action_schemes`] into the sim schedule. Registered beside
/// `AffordancesPlugin` (which the scheme + control-prompt read-model will
/// retire in P3).
pub struct ActionSchemePlugin;

impl Plugin for ActionSchemePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // `PlayerInput` is chained BEFORE `WorldPrep` (schedule.rs), so the
        // scheme reconciled here reflects authorities as finalized on the
        // PREVIOUS tick — a deterministic one-tick lag after an ability/moveset
        // change. That is the correct model: the P3 input→action resolver (also
        // in `PlayerInput`) and the control-prompt read-model (`FeatureViewSync`
        // tail) then consume the SAME scheme, and a one-frame delay before a
        // newly-granted action lights up on the HUD is imperceptible.
        app.add_systems(
            sim,
            reconcile_action_schemes.in_set(SandboxSet::PlayerInput),
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

    /// The shared-resolution DRIFT GUARD (review step 5): the on-screen prompt
    /// renders the derived `ActionScheme`, while gameplay's persona gate
    /// (`gate_worn_player_control`) reads the body's immediate `ActionSet`
    /// authority. Those are two views of ONE derivation, so a combat slot is in
    /// the prompt's scheme IFF gameplay would let its verb fire. This test locks
    /// them together: if the scheme derivation and the gate authority ever drift,
    /// a button would advertise an action gameplay strips (or hide one it fires)
    /// — and this fails. (Gameplay keeps the immediate `ActionSet` rather than
    /// the one-tick-derived scheme deliberately, to avoid a stale gate on a
    /// character swap; the guard is what makes that safe.)
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
