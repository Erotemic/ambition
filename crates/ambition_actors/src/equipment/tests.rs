//! The grant reconcile: granted verbs appear, survive, and are REVOKED.
//!
//! The revocation half is what the one-shot equip path could never express, so it
//! is what these mostly prove — including the case the A3 contract used to declare
//! out of scope: armor that grants a verb, spent by a hit, downgrading into a row
//! that grants nothing.

use bevy::prelude::*;

use ambition_characters::brain::action_set::{
    ActionSet, IdentityKit, RangedActionSpec, RangedStyle,
};
use ambition_characters::equipment::{EquipmentGrant, EquipmentRow, OnHit, WornEquipment};
use ambition_combat::moveset::{ActorMoveset, RANGED_VERB};

use super::reconcile_equipment_grants;

fn granting_row(id: &str, on_hit: Option<OnHit>) -> EquipmentRow {
    EquipmentRow {
        id: id.to_string(),
        modifiers: Vec::new(),
        grants: vec![EquipmentGrant::Ranged(RangedActionSpec::bolt(400.0, 5))],
        on_hit,
        exclusive_slot: None,
    }
}

fn plain_row(id: &str) -> EquipmentRow {
    EquipmentRow {
        id: id.to_string(),
        modifiers: Vec::new(),
        grants: Vec::new(),
        on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
        exclusive_slot: None,
    }
}

/// A body with a peaceful identity (no ranged of its own), so any ranged verb it
/// ends up with can only have come from equipment.
fn app_with_body(worn: WornEquipment) -> (App, Entity) {
    let mut app = App::new();
    let body = app
        .world_mut()
        .spawn((
            IdentityKit::default(),
            ActionSet::peaceful(),
            ActorMoveset(Default::default()),
            worn,
        ))
        .id();
    app.add_systems(Update, reconcile_equipment_grants);
    (app, body)
}

fn has_ranged(app: &App, body: Entity) -> bool {
    app.world().get::<ActionSet>(body).unwrap().ranged.is_some()
}

fn has_ranged_move(app: &App, body: Entity) -> bool {
    app.world()
        .get::<ActorMoveset>(body)
        .unwrap()
        .0
        .move_for_verb(RANGED_VERB)
        .is_some()
}

#[test]
fn a_worn_grant_becomes_a_live_verb_and_a_fireable_move() {
    let (mut app, body) = app_with_body(WornEquipment::new(vec![granting_row("spark", None)]));
    app.update();

    assert!(
        has_ranged(&app, body),
        "the worn grant set the ranged action"
    );
    assert!(
        has_ranged_move(&app, body),
        "and the moveset gained a fireable ranged move"
    );
}

/// The case A3 v1 declared out of scope. A GRANTING armor row, spent by a hit,
/// downgrades into a grant-free row: the verb goes away, and the downgrade row
/// stays worn. Nothing here calls into the reconcile explicitly — spending the
/// armor is the whole trigger, exactly as it is inside the shared hit resolver.
#[test]
fn spending_a_granting_armor_row_revokes_its_verb_and_keeps_the_downgrade() {
    let spark = granting_row(
        "spark",
        Some(OnHit::ConsumeAsArmor {
            downgrade_to: Some(Box::new(plain_row("cap"))),
        }),
    );
    let (mut app, body) = app_with_body(WornEquipment::new(vec![spark]));
    app.update();
    assert!(has_ranged(&app, body), "granted before the hit");

    // A hit spends the armor — the only mutation.
    app.world_mut()
        .get_mut::<WornEquipment>(body)
        .unwrap()
        .consume_armor();
    app.update();

    assert!(
        !has_ranged(&app, body),
        "spending the granting row revokes its verb — no dangling action"
    );
    assert!(
        !has_ranged_move(&app, body),
        "and its move leaves the moveset with it"
    );
    let worn = app.world().get::<WornEquipment>(body).unwrap();
    assert!(worn.wears("cap"), "the downgrade row is worn");
    assert!(!worn.wears("spark"), "the spent row is gone");
}

/// The next hit finds only the grant-free downgrade and spends that, leaving the
/// body bare — the second step of a two-stage loss.
#[test]
fn the_next_hit_spends_the_downgrade_and_leaves_nothing_worn() {
    let spark = granting_row(
        "spark",
        Some(OnHit::ConsumeAsArmor {
            downgrade_to: Some(Box::new(plain_row("cap"))),
        }),
    );
    let (mut app, body) = app_with_body(WornEquipment::new(vec![spark]));
    app.update();
    for _ in 0..2 {
        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .consume_armor();
        app.update();
    }

    let worn = app.world().get::<WornEquipment>(body).unwrap();
    assert!(worn.rows.is_empty(), "both rows have been spent");
    assert!(!has_ranged(&app, body), "and no verb survives");
}

/// Unequip revokes too — the reconcile does not care WHY the worn set changed.
#[test]
fn unequipping_a_granting_row_revokes_its_verb() {
    let (mut app, body) = app_with_body(WornEquipment::new(vec![granting_row("spark", None)]));
    app.update();
    assert!(has_ranged(&app, body));

    app.world_mut()
        .get_mut::<WornEquipment>(body)
        .unwrap()
        .unequip("spark");
    app.update();

    assert!(!has_ranged(&app, body), "unequip revokes the grant");
}

/// A verb the body owns ITSELF is not collateral damage: revoking an equipment
/// grant must not strip the identity's own ranged action.
#[test]
fn revoking_a_grant_leaves_the_bodys_own_verb_intact() {
    let mut app = App::new();
    let identity = IdentityKit {
        action_set: ActionSet {
            ranged: Some(RangedActionSpec::arrow(300.0, 2)),
            ..ActionSet::peaceful()
        },
        ..IdentityKit::default()
    };
    let body = app
        .world_mut()
        .spawn((
            identity,
            ActionSet::peaceful(),
            ActorMoveset(Default::default()),
            WornEquipment::new(vec![granting_row("spark", None)]),
        ))
        .id();
    app.add_systems(Update, reconcile_equipment_grants);
    app.update();

    // The grant overlays the identity's own arrow.
    let set = app.world().get::<ActionSet>(body).unwrap();
    assert_eq!(
        set.ranged.as_ref().map(|r| r.style),
        Some(RangedStyle::Bolt),
        "the worn grant wins while it is worn"
    );

    app.world_mut()
        .get_mut::<WornEquipment>(body)
        .unwrap()
        .unequip("spark");
    app.update();

    let set = app.world().get::<ActionSet>(body).unwrap();
    assert_eq!(
        set.ranged.as_ref().map(|r| r.style),
        Some(RangedStyle::Arrow),
        "losing the grant falls back to the body's OWN verb, not to nothing"
    );
}

/// The derivation settles: writing the action set marks neither of its inputs, so
/// a quiet frame does no work and the result does not oscillate.
#[test]
fn the_reconcile_quiesces_when_nothing_changes() {
    let (mut app, body) = app_with_body(WornEquipment::new(vec![granting_row("spark", None)]));
    app.update();
    let before = app
        .world()
        .get::<ActionSet>(body)
        .unwrap()
        .ranged
        .as_ref()
        .map(|r| r.speed());

    for _ in 0..5 {
        app.update();
    }

    let after = app
        .world()
        .get::<ActionSet>(body)
        .unwrap()
        .ranged
        .as_ref()
        .map(|r| r.speed());
    assert_eq!(before, after, "a settled body keeps its derived kit");
}

/// Body-generic: nothing in the query names a controller or a player marker, so a
/// bare actor entity reconciles on the identical path.
#[test]
fn any_body_reconciles_not_only_a_player() {
    let (mut app, body) = app_with_body(WornEquipment::new(vec![granting_row("spark", None)]));
    app.update();
    assert!(
        has_ranged(&app, body),
        "an entity carrying no player marker at all still reconciles"
    );
}
