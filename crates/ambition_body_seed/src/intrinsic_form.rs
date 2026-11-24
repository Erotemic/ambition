//! What a prepared form gives a body by itself, and how a change of form
//! moves a live body from one to the next (FORM-REBIND).
//!
//! Construction and replacement read the same value: the actor verbs the
//! form grants and whether the body flies without being asked. Construction
//! seeds the ability base, the flight mode, the surface gravity scale and
//! `ActorTuning::is_aerial` from it; a change of form rebinds those same
//! fields through [`IntrinsicForm::rebind`].
//!
//! A role, a match seat or a home experience may state other values over the
//! form, and those stay theirs: a home experience or a match seat owns the
//! ability base ([`FormOwners::verbs`]), and a role that states its own
//! flight marks the body [`FlightStatedByRole`] (a boss flies whatever its
//! character does).

use ambition_combat::actor_tuning::ActorTuning;
use ambition_platformer2d_core::body_clusters::{AbilityBase, ActorSurfaceState, BodyFlightState};
use ambition_platformer2d_core::{AbilityContributions, AbilitySet};

/// The intrinsic configuration of one prepared form.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntrinsicForm {
    /// The verbs an actor body wearing the form has; `fly` is folded in when
    /// the form flies by itself.
    pub abilities: AbilitySet,
    /// The form ignores gravity in ordinary locomotion
    /// (`CharacterLocomotion::baseline_free_flight`; silence reads grounded).
    pub free_flight: bool,
}

impl IntrinsicForm {
    pub fn new(actor_abilities: AbilitySet, free_flight: bool) -> Self {
        let mut abilities = actor_abilities;
        abilities.fly = free_flight || abilities.fly;
        Self { abilities, free_flight }
    }

    /// The form a prepared character gives an actor body. A body that wears
    /// no prepared character has no author, and gets no verbs and no flight.
    pub fn of(prepared: Option<&ambition_characters::prepared::PreparedCharacterDefinition>) -> Self {
        Self::new(
            prepared.map_or(AbilitySet::NONE, |prepared| prepared.actor_abilities),
            prepared
                .and_then(|prepared| prepared.locomotion)
                .and_then(|locomotion| locomotion.baseline_free_flight)
                .unwrap_or(false),
        )
    }

    /// The gravity response of the form's ordinary locomotion.
    pub fn gravity_scale(&self) -> f32 {
        if self.free_flight { 0.0 } else { 1.0 }
    }

    /// Move a live body from the form `self` gave it to `next`.
    ///
    /// The same form again changes nothing, so velocity, a pending launch and
    /// the live flight toggle are kept. A field another owner holds
    /// ([`FormOwners`]) is read, never written.
    ///
    /// Flight is settled here against the effective permissions the new base
    /// and the live contributions give, because the ability projector leaves
    /// the flight mode alone when the effective set does not change. Entering
    /// intrinsic flight turns the mode on where flight is permitted; leaving it
    /// keeps the mode only where the body may still hold it by its own toggle,
    /// so a separately granted flight is not taken away.
    pub fn rebind(&self, next: &IntrinsicForm, owners: FormOwners, body: FormRebindTargets<'_>) {
        if self == next {
            return;
        }
        let FormRebindTargets { mut base, contributions, surface, tuning, flight } = body;
        if owners.verbs {
            if let Some(base) = base.as_deref_mut() {
                if base.abilities != next.abilities {
                    base.abilities = next.abilities;
                }
            }
        }
        if !owners.flight || self.free_flight == next.free_flight {
            return;
        }
        if let Some(surface) = surface {
            surface.gravity_scale = next.gravity_scale();
        }
        if let Some(tuning) = tuning {
            tuning.is_aerial = next.free_flight;
        }
        if let Some(flight) = flight {
            let granted = base.as_deref().map_or(next.abilities, |base| base.abilities);
            let effective = contributions.map_or(granted, |contributions| contributions.apply(granted));
            flight.fly_enabled = if next.free_flight {
                effective.fly
            } else {
                flight.fly_enabled && effective.fly && effective.fly_toggle
            };
        }
    }
}

/// Which of the rebound facts are the form's on this body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormOwners {
    /// The ability base. False where a home experience or a match seat owns
    /// it.
    pub verbs: bool,
    /// The flight mode, gravity response and aerial tuning. False where a
    /// role stated its own ([`FlightStatedByRole`]).
    pub flight: bool,
}

/// A role stated this body's flight over its character's (a boss floats):
/// a change of form leaves the flight mode, gravity response and aerial
/// tuning as the role set them.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlightStatedByRole;

/// The live fields a change of form rebinds. Each is optional: a body that
/// carries no such field has nothing of it to rebind.
pub struct FormRebindTargets<'a> {
    pub base: Option<&'a mut AbilityBase>,
    pub contributions: Option<&'a AbilityContributions>,
    pub surface: Option<&'a mut ActorSurfaceState>,
    pub tuning: Option<&'a mut ActorTuning>,
    pub flight: Option<&'a mut BodyFlightState>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::{AbilityContribution, AbilitySet};

    const FORM: FormOwners = FormOwners { verbs: true, flight: true };

    fn sheep() -> IntrinsicForm {
        IntrinsicForm::new(AbilitySet { move_horizontal: true, ..AbilitySet::NONE }, false)
    }

    fn cloud() -> IntrinsicForm {
        IntrinsicForm::new(AbilitySet { move_horizontal: true, ..AbilitySet::NONE }, true)
    }

    /// The live fields of a body, as construction seeds them for `form`.
    #[derive(Clone, Debug, PartialEq)]
    struct Live {
        base: AbilitySet,
        surface: ActorSurfaceState,
        tuning: bool,
        flight: bool,
    }

    fn built(form: IntrinsicForm) -> Live {
        let body = crate::ActorBody::from_abilities(form.abilities, form.free_flight, ambition_platformer2d_core::Vec2::ONE);
        Live {
            base: body.0.abilities.abilities,
            surface: ActorSurfaceState { surface_normal: ambition_platformer2d_core::Vec2::new(0.0, -1.0), gravity_scale: form.gravity_scale() },
            tuning: form.free_flight,
            flight: body.0.flight.fly_enabled,
        }
    }

    fn rebind(live: &mut Live, from: IntrinsicForm, to: IntrinsicForm, owners: FormOwners, contributions: Option<&AbilityContributions>) {
        let mut tuning = ActorTuning { is_aerial: live.tuning, ..Default::default() };
        let mut flight = BodyFlightState { fly_enabled: live.flight, ..Default::default() };
        let mut base = AbilityBase::new(live.base);
        from.rebind(
            &to,
            owners,
            FormRebindTargets {
                base: Some(&mut base),
                contributions,
                surface: Some(&mut live.surface),
                tuning: Some(&mut tuning),
                flight: Some(&mut flight),
            },
        );
        live.base = base.abilities;
        live.tuning = tuning.is_aerial;
        live.flight = flight.fly_enabled;
    }

    /// A sheep worn as a cloud carries the configuration a cloud is built
    /// with, and back again; the same form again changes nothing.
    #[test]
    fn a_changed_form_carries_the_configuration_a_fresh_body_of_it_is_built_with() {
        let mut live = built(sheep());
        rebind(&mut live, sheep(), cloud(), FORM, None);
        assert_eq!(live, built(cloud()), "sheep worn as a cloud");
        rebind(&mut live, cloud(), sheep(), FORM, None);
        assert_eq!(live, built(sheep()), "and back");

        let mut moved = built(cloud());
        moved.flight = false;
        rebind(&mut moved, cloud(), cloud(), FORM, None);
        assert!(!moved.flight, "the same form again rewrote the live flight mode");
    }

    /// A role that states its own flight (a boss flies whatever its character
    /// does) keeps it through a change of form; a home or seat base is read,
    /// never written.
    #[test]
    fn a_role_or_seat_keeps_what_it_stated() {
        let mut boss = built(cloud());
        let role = boss.clone();
        let role_flight = FormOwners { verbs: true, flight: false };
        rebind(&mut boss, sheep(), cloud(), role_flight, None);
        rebind(&mut boss, cloud(), sheep(), role_flight, None);
        assert_eq!(boss.surface, role.surface);
        assert_eq!((boss.tuning, boss.flight), (role.tuning, role.flight));

        let seat = AbilitySet { jump: true, ..AbilitySet::NONE };
        let mut seated = built(sheep());
        seated.base = seat;
        rebind(&mut seated, sheep(), cloud(), FormOwners { verbs: false, flight: true }, None);
        assert_eq!(seated.base, seat, "a seat's verbs were rewritten by the form");
    }

    /// Entering a flying form while a ceiling denies flight leaves the mode
    /// off; leaving it while only the toggle is denied does not keep flight.
    #[test]
    fn flight_is_settled_against_the_effective_permissions() {
        let mut denied = AbilityContributions::default();
        denied.set("room", AbilityContribution::Ceiling(AbilitySet { fly: false, ..AbilitySet::ALL }));
        let mut live = built(sheep());
        rebind(&mut live, sheep(), cloud(), FORM, Some(&denied));
        assert!(!live.flight, "a ceiling that denies flight was ignored");
        assert_eq!(live.surface.gravity_scale, 0.0, "denied permission replaced the gravity response");

        let mut no_toggle = AbilityContributions::default();
        no_toggle.set("mask", AbilityContribution::Ceiling(AbilitySet { fly_toggle: false, ..AbilitySet::ALL }));
        let mut live = built(cloud());
        rebind(&mut live, cloud(), sheep(), FORM, Some(&no_toggle));
        assert!(!live.flight, "leaving a flying form kept flight");
    }
}
