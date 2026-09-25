//! The body's effective verbs, projected from its base and what live situations
//! contribute.
//!
//! `BodyAbilities = (AbilityBase ∪ every lend) ∩ every ceiling`, recomputed by
//! [`project_body_abilities`] for each body carrying contributions. A source (a
//! room that forces swimming, a portal crossing that forbids wall verbs, the
//! developer ability mask) owns exactly one keyed entry in
//! [`AbilityContributions`] and never writes `BodyAbilities` itself, so no
//! source can undo another's effect or restore a value it did not set.

use bevy_ecs::prelude::*;

use crate::abilities::AbilitySet;
use crate::body_clusters::{
    AbilityBase, AuthoredMovementTuning, BodyAbilities, BodyDashState, BodyFlightState,
    BodyJumpState,
};
use crate::movement::{ActiveMovementTuning, MotionModel};

/// One source's effect on a body's verbs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AbilityContribution {
    /// Verbs the source adds on top of the base.
    Lend(AbilitySet),
    /// Verbs the source permits; everything else is withheld.
    Ceiling(AbilitySet),
}

/// Every live contribution to this body's verbs, keyed by source.
///
/// Required by [`AbilityBase`], so every body with an authored kit carries one.
/// A source writes only its own key; an absent key contributes nothing.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct AbilityContributions {
    entries: Vec<(&'static str, AbilityContribution)>,
}

impl AbilityContributions {
    /// Set `source`'s contribution. Unchanged values do not touch the list.
    pub fn set(&mut self, source: &'static str, contribution: AbilityContribution) {
        match self.entries.iter_mut().find(|(key, _)| *key == source) {
            Some((_, existing)) => *existing = contribution,
            None => self.entries.push((source, contribution)),
        }
    }

    /// Withdraw `source`'s contribution.
    pub fn clear(&mut self, source: &'static str) {
        self.entries.retain(|(key, _)| *key != source);
    }

    pub fn get(&self, source: &'static str) -> Option<AbilityContribution> {
        self.entries
            .iter()
            .find(|(key, _)| *key == source)
            .map(|(_, contribution)| *contribution)
    }

    /// `(base ∪ lends) ∩ ceilings`. Order-free: union and intersection are each
    /// commutative, and every lend is applied before any ceiling.
    pub fn apply(&self, base: AbilitySet) -> AbilitySet {
        let mut lent = base;
        let mut ceiling = AbilitySet::ALL;
        for (_, contribution) in &self.entries {
            match *contribution {
                AbilityContribution::Lend(set) => lent = lent.union(set),
                AbilityContribution::Ceiling(set) => ceiling = ceiling.intersect(set),
            }
        }
        lent.intersect(ceiling)
    }
}

/// Recompute each body's effective verbs from its base and contributions.
///
/// On a change it settles the state that a verb's loss or gain invalidates:
/// flight switches off without `fly`, a blink telegraph ends without `blink`,
/// and dash charges and air jumps follow their counts (a larger count refills,
/// a smaller one clamps).
#[allow(clippy::type_complexity)]
pub fn project_body_abilities(
    active_tuning: Option<Res<ActiveMovementTuning>>,
    mut bodies: Query<(
        &AbilityBase,
        &AbilityContributions,
        &mut BodyAbilities,
        Option<&mut BodyFlightState>,
        // Required, never optional (ADR 0024 §1): every body that carries
        // verbs is an integrated body, and integrated bodies carry a model.
        &mut MotionModel,
        Option<&mut BodyDashState>,
        Option<&mut BodyJumpState>,
        Option<&AuthoredMovementTuning>,
    )>,
) {
    for (base, contributions, mut abilities, flight, mut model, dash, jump, authored) in &mut bodies {
        let desired = contributions.apply(base.abilities);
        let previous = abilities.abilities;
        if previous == desired {
            continue;
        }
        abilities.abilities = desired;
        if !desired.fly {
            if let Some(mut flight) = flight {
                flight.fly_enabled = false;
            }
        }
        if !desired.blink {
            if let MotionModel::AxisSwept(axis) = &mut *model {
                axis.state.blink_hold_active = false;
                axis.state.blink_hold_timer = 0.0;
                axis.state.blink_aiming = false;
            }
        }
        if let Some(mut dash) = dash {
            dash.charges_available =
                follow_count(dash.charges_available, previous.dash_charge_count(), desired.dash_charge_count());
        }
        if let Some(mut jump) = jump {
            let air_jumps = authored
                .map(|t| t.0.air_jumps)
                .or(active_tuning.as_ref().map(|t| t.0.air_jumps))
                .unwrap_or_default();
            jump.air_jumps_available = follow_count(
                jump.air_jumps_available,
                previous.air_jump_count(air_jumps),
                desired.air_jump_count(air_jumps),
            );
        }
    }
}

fn follow_count(available: u8, previous: u8, desired: u8) -> u8 {
    if desired > previous {
        desired
    } else {
        available.min(desired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn swim_only() -> AbilitySet {
        AbilitySet { swim: true, ..AbilitySet::NONE }
    }

    /// A lend adds, a ceiling removes, and neither depends on insertion order.
    #[test]
    fn lends_add_ceilings_remove_in_any_order() {
        let base = AbilitySet::basic();
        let no_jump = AbilitySet { jump: false, ..AbilitySet::ALL };
        let mut a = AbilityContributions::default();
        a.set("room", AbilityContribution::Lend(swim_only()));
        a.set("mask", AbilityContribution::Ceiling(no_jump));
        let mut b = AbilityContributions::default();
        b.set("mask", AbilityContribution::Ceiling(no_jump));
        b.set("room", AbilityContribution::Lend(swim_only()));
        let applied = a.apply(base);
        assert_eq!(applied, b.apply(base));
        assert!(applied.swim && !applied.jump);
        assert_eq!(applied.move_horizontal, base.move_horizontal);
    }

    /// Withdrawing one source leaves the others' effects standing.
    #[test]
    fn clearing_one_source_keeps_the_rest() {
        let base = AbilitySet { ledge_grab: true, ..AbilitySet::NONE };
        let mut c = AbilityContributions::default();
        c.set("room", AbilityContribution::Lend(swim_only()));
        c.set("portal", AbilityContribution::Ceiling(AbilitySet { ledge_grab: false, ..AbilitySet::ALL }));
        assert!(c.apply(base).swim && !c.apply(base).ledge_grab);
        c.clear("portal");
        assert!(c.apply(base).swim && c.apply(base).ledge_grab);
        assert_eq!(AbilityContributions::default().apply(base), base);
    }

    #[test]
    fn counts_refill_on_growth_and_clamp_on_shrink() {
        assert_eq!(follow_count(0, 1, 2), 2);
        assert_eq!(follow_count(0, 2, 1), 0);
        assert_eq!(follow_count(2, 2, 1), 1);
        assert_eq!(follow_count(1, 1, 1), 1);
    }
}
