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
//! Techniques are not wired here yet: content-declared movement techniques
//! (Sanic's spin) become scheme actions in P3 when the input→action seam
//! lands; today the scheme reflects abilities + moveset.

use ambition_characters::action_scheme::{derive_action_scheme, ActorActionScheme};
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
        Option<&ActorActionScheme>,
    )>,
) {
    for (entity, abilities, moveset, existing) in &bodies {
        let source_changed =
            abilities.is_changed() || moveset.as_ref().is_some_and(|m| m.is_changed());
        if existing.is_some() && !source_changed {
            continue;
        }
        // Techniques: none until the P3 input→action seam gives them a home.
        let derived =
            derive_action_scheme(&abilities.abilities, moveset.as_ref().map(|m| &m.0), &[]);
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
        // Runs in the pre-player-tick input band, after `WorldPrep` has
        // finalized this tick's abilities/movesets and before the control-prompt
        // read-model (P2b) reads the scheme.
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
}
