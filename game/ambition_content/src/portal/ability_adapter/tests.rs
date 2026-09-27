//! These drive the REAL adapter + the portal-owned transit latch.
use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_portal2d::{PortalChannel, PortalGunColor, PortalTransit, PortalTuning};

use super::withhold_wall_verbs_during_transit;

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);

fn abilities_app() -> App {
    let mut app = App::new();
    app.init_resource::<PortalTuning>();
    app.add_systems(
        Update,
        (
            withhold_wall_verbs_during_transit,
            ambition_platformer2d_core::project_body_abilities,
        )
            .chain(),
    );
    app
}

fn effective(app: &App, body: Entity) -> ambition_platformer2d_core::AbilitySet {
    app.world()
        .get::<ambition_platformer2d_core::BodyAbilities>(body)
        .unwrap()
        .abilities
}

/// The aperture-edge hazard is a property of TRANSITING, not of being the
/// primary player: a plain actor (no player markers) mid-transit loses its wall
/// verbs, and gets back exactly what its base grants when the latch goes.
#[test]
fn wall_verbs_are_withheld_for_any_transiting_body_and_come_back_from_its_base() {
    use ambition_platformer2d_core::{AbilityBase, AbilitySet, BodyAbilities};
    let mut app = abilities_app();
    let authored = AbilitySet {
        ledge_grab: true,
        wall_jump: true,
        ..AbilitySet::NONE
    };
    let actor = app
        .world_mut()
        .spawn((
            BodyAbilities::new(authored),
            AbilityBase::new(authored),
            // A body with verbs is an integrated body (ADR 0024 §1).
            ambition_platformer2d_core::movement::MotionModel::default(),
            PortalTransit {
                straddling: BLUE,
                crossed: false,
            },
        ))
        .id();

    app.update();
    let a = effective(&app, actor);
    assert!(
        !a.ledge_grab && !a.wall_jump,
        "a transiting ACTOR has its wall verbs withheld too"
    );

    app.world_mut().entity_mut(actor).remove::<PortalTransit>();
    app.update();
    assert_eq!(
        effective(&app, actor),
        authored,
        "transit end gives back exactly the base: its wall verbs, and nothing it never had"
    );
}

/// A crossing ends by WITHDRAWING its ceiling, not by restoring verbs, so a
/// verb another source withholds stays withheld. Restoring the four wall verbs
/// from the base handed back a verb the session mask had taken away.
#[test]
fn a_transit_ending_does_not_hand_back_a_verb_another_source_withholds() {
    use ambition_platformer2d_core::{
        AbilityBase, AbilityContribution, AbilityContributions, AbilitySet, BodyAbilities,
    };
    let mut app = abilities_app();
    let authored = AbilitySet {
        ledge_grab: true,
        wall_jump: true,
        ..AbilitySet::NONE
    };
    let mut contributions = AbilityContributions::default();
    contributions.set(
        "session.mask",
        AbilityContribution::Ceiling(AbilitySet {
            wall_jump: false,
            ..AbilitySet::ALL
        }),
    );
    let body = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyAbilities::new(authored),
            AbilityBase::new(authored),
            ambition_platformer2d_core::movement::MotionModel::default(),
            contributions,
            PortalTransit {
                straddling: BLUE,
                crossed: false,
            },
        ))
        .id();

    app.update();
    assert!(!effective(&app, body).ledge_grab, "the crossing withholds ledge-grab");

    app.world_mut().entity_mut(body).remove::<PortalTransit>();
    app.update();
    let a = effective(&app, body);
    assert!(a.ledge_grab, "the crossing's own withholding ended");
    assert!(!a.wall_jump, "the transit end handed back a verb the session mask withholds");
}
