//! Body control acceptance through the real systems and rule resolver.

use super::*;
use ambition_characters::actor::WornCharacter;
use ambition_platformer2d_core::{self as ae, movement::MotionModel};

/// A peaceful authored persona must be peaceful at the final body-control seam,
/// not only in its nominal ActionSet. Legacy player mechanics consume these raw
/// fields directly, bypassing the generic action resolver unless they are gated.
#[test]
fn peaceful_worn_kit_gates_direct_player_combat_verbs() {
    use ambition_characters::actor::control::{ActorControlFrame, ActorFireRequest};
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use bevy::prelude::*;

    let mut frame = ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.pogo_pressed = true;
    frame.attack_axis = ambition_platformer2d_core::LocalAxes::new(1.0, -1.0);
    frame.fire = Some(ActorFireRequest::world_space(Vec2::X));
    frame.shield_held = true;
    frame.projectile_pressed = true;
    frame.projectile_held = true;
    frame.projectile_released = true;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, gate_body_control);
    let entity = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            WornCharacter::new("sanic"),
            MotionModel::default(),
            // The gate resolves the body's live authorities through the shared
            // scheme; a peaceful `ActionSet` (no melee/ranged/special) + no
            // moveset yields no combat slots, so the gate strips every combat
            // verb below. `BodyAbilities` supplies the movement slots.
            ambition_platformer2d_core::BodyAbilities::new(
                ambition_platformer2d_core::AbilitySet::sandbox_all(),
            ),
            ActionSet::peaceful(),
            ActorControl(frame),
        ))
        .id();
    app.update();

    let gated = &app.world().get::<ActorControl>(entity).unwrap().0;
    assert!(!gated.melee_pressed);
    assert!(!gated.pogo_pressed);
    assert_eq!(
        gated.attack_axis,
        ambition_platformer2d_core::LocalAxes::ZERO
    );
    assert!(gated.fire.is_none());
    // Peaceful is a claim about ATTACKING; a guard is defensive, and this body's `BodyAbilities`
    // grants `shield`, which is what puts the Shield slot on its scheme. A body that lacks the
    // ability still loses the verb — that is
    // `a_body_without_the_shield_ability_loses_its_guard_verb` below, and keeping the strip here
    // would have been the gate asking about the wrong authority again.
    assert!(
        gated.shield_held,
        "a peaceful persona that OWNS the shield ability lost its guard — the \
         persona gate is judging a defensive capability by an offensive kit"
    );
    assert!(!gated.projectile_pressed);
    assert!(!gated.projectile_held);
    assert!(!gated.projectile_released);
}

/// A CHARACTER THAT CHARGES KEEPS ITS PROJECTILE PRESS.
///
/// The hour Robot v3 started authoring its own kit — the row flipping to `Authored` — that gate
/// began silently disabling the Hadouken. Nothing failed: pressing the projectile button as the
/// protagonist was covered nowhere, so a full green suite said the protagonist could still fire.
///
/// the gate asks the CHARACTER how it fires now (`ranged_execution`), which is
/// what §4 made an authored fact for. Two terms, because the permissive half
/// alone would pass with the gate deleted: a charging character keeps the press,
/// and a `MovesetVerb` character still loses it.
#[test]
fn an_authored_charging_character_keeps_its_projectile_press() {
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::actor::definition::CharacterDefinition;
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use ambition_characters::prepared::CharacterBindings;
    use bevy::prelude::*;

    let charge_frame = || {
        let mut frame = ActorControlFrame::neutral();
        frame.projectile_pressed = true;
        frame.projectile_held = true;
        frame.projectile_released = true;
        frame
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, gate_body_control);

    // Two characters, identical but for how they fire.
    let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
    for (id, execution) in [
        (
            "gunner",
            ambition_characters::brain::RangedExecution::ChargedProjectile,
        ),
        (
            "swordfighter",
            ambition_characters::brain::RangedExecution::MovesetVerb,
        ),
    ] {
        registry.insert_prepared(
            ambition_characters::prepared::prepare_and_finalize_against_for_test(
                CharacterDefinition::new(id, id, "test")
                    .with_ranged_execution(execution)
                    .with_action_set(ActionSet {
                        ranged: Some(ambition_characters::brain::RangedActionSpec::bolt(600.0, 1)),
                        ..ActionSet::default()
                    }),
                &CharacterBindings::default(),
                None,
            )
            .prepared,
        );
    }
    app.insert_resource(registry);

    // Each body carries the charge marker exactly when its character charges —
    // the state every construction and re-wear road installs from
    // `ranged_execution`, and the one fact the gate reads.
    let spawn = |app: &mut App, id: &str| {
        let mut body = app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            WornCharacter::new(id),
            MotionModel::default(),
            ambition_platformer2d_core::BodyAbilities::new(
                ambition_platformer2d_core::AbilitySet::sandbox_all(),
            ),
            ActionSet {
                ranged: Some(ambition_characters::brain::RangedActionSpec::bolt(600.0, 1)),
                ..ActionSet::default()
            },
            ActorControl(charge_frame()),
        ));
        if id == "gunner" {
            body.insert(ambition_characters::brain::ChargesProjectiles);
        }
        body.id()
    };
    let gunner = spawn(&mut app, "gunner");
    let swordfighter = spawn(&mut app, "swordfighter");
    app.update();

    let read = |app: &App, e: Entity| {
        app.world()
            .get::<ActorControl>(e)
            .unwrap()
            .0
            .projectile_held
    };
    assert!(
        read(&app, gunner),
        "a character that AUTHORS a charged projectile lost its press — this is \
         the Hadouken going quiet the moment the robot stopped saying `HostCode`"
    );
    // the poison: the gate must still strip the press from a character that
    // does not charge, or it is not a gate.
    assert!(
        !read(&app, swordfighter),
        "a character that fires through its MOVESET kept the charge press, so \
         one button would be owned twice"
    );
}

/// The gate CONSUMES the shared resolver: a body whose scheme puts a
/// `Technique` on the Attack slot has its melee device edge ROUTED into the
/// sanctioned `ResolvedTechniqueEdges` (and the raw verb cleared) — proving the
/// resolver drives both gating AND the content-technique seam. A body with a
/// real melee `Move` keeps its verb.
#[test]
fn gate_routes_a_technique_attack_slot_into_the_sanctioned_edge() {
    use ambition_characters::action_scheme::{ActorTechniques, ResolvedTechniqueEdges};
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use ambition_entity_catalog::action_scheme::{ActionGate, ActionId, ActionSpec, ControlSlot};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, gate_body_control);

    // A Sanic-shaped body: movement abilities, empty ActionSet, and a spin_dash
    // technique claiming the Attack slot. Pressing melee this tick.
    let mut frame = ActorControlFrame::neutral();
    frame.melee_pressed = true;
    let spin = ActionSpec {
        id: ActionId::new("spin_dash"),
        slot: ControlSlot::Attack,
        display_name: Some("Spin Dash".to_owned()),
        visual: None,
        gate: ActionGate::Technique("spin_dash".to_owned()),
    };
    let body = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            WornCharacter::new("sanic"),
            ambition_platformer2d_core::BodyAbilities::new(
                ambition_platformer2d_core::AbilitySet::sandbox_all(),
            ),
            ActionSet::peaceful(),
            ActorTechniques(vec![spin]),
            ResolvedTechniqueEdges::default(),
            ActorControl(frame),
        ))
        .id();
    app.update();

    let control = &app.world().get::<ActorControl>(body).unwrap().0;
    assert!(
        !control.melee_pressed,
        "the raw melee verb is cleared — no longer the content API"
    );
    let edges = app.world().get::<ResolvedTechniqueEdges>(body).unwrap();
    assert!(
        edges.pressed("spin_dash"),
        "the Attack device edge is routed into the sanctioned spin_dash technique edge"
    );
}

/// The rules' driven techniques go with the driven body. When the player drives
/// another body, the first body loses them on the same tick; nothing wrote them
/// onto it. `ActionSet` alone gives each body its edge sink.
#[test]
fn the_rules_techniques_follow_the_driven_body() {
    use ambition_characters::action_scheme::{DrivenTechniques, ResolvedTechniqueEdges};
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use ambition_combat::scoped_rules::{DeclareRulesExt, RulesScope};
    use ambition_entity_catalog::action_scheme::{ActionGate, ActionId, ActionSpec, ControlSlot};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.declare_rules(
        RulesScope::EveryRoom,
        DrivenTechniques(vec![ActionSpec {
            id: ActionId::new("spin_dash"),
            slot: ControlSlot::Attack,
            display_name: None,
            visual: None,
            gate: ActionGate::Technique("spin_dash".to_owned()),
        }]),
    );
    app.add_systems(Update, gate_body_control);
    let mut body = || {
        app.world_mut()
            .spawn((
                ambition_platformer2d_core::BodyAbilities::new(
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
                ActionSet::peaceful(),
                ActorControl(ActorControlFrame::neutral()),
            ))
            .id()
    };
    let (first, second) = (body(), body());

    let spun_after_both_press = |app: &mut App, driven: Entity| -> [bool; 2] {
        for entity in [first, second] {
            app.world_mut()
                .entity_mut(entity)
                .remove::<DrivingParticipant>();
        }
        app.world_mut()
            .entity_mut(driven)
            .insert(DrivingParticipant(
                ambition_characters::control::PlayerSlot::PRIMARY,
            ));
        for entity in [first, second] {
            app.world_mut()
                .get_mut::<ActorControl>(entity)
                .unwrap()
                .0
                .melee_pressed = true;
        }
        app.update();
        [first, second].map(|entity| {
            app.world()
                .get::<ResolvedTechniqueEdges>(entity)
                .expect("an action set brings its edge sink")
                .pressed("spin_dash")
        })
    };
    assert_eq!(spun_after_both_press(&mut app, first), [true, false]);
    assert_eq!(
        spun_after_both_press(&mut app, second),
        [false, true],
        "the body the player left kept the rules' technique",
    );
}

/// A worn row's technique gates its slot on the body that wears it, driven or
/// not, and only while the row is worn.
#[test]
fn a_worn_technique_gates_the_slot_of_the_body_that_wears_it() {
    use ambition_characters::action_scheme::ResolvedTechniqueEdges;
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use ambition_characters::equipment::{EquipmentGrant, EquipmentRow, WornEquipment};
    use ambition_entity_catalog::action_scheme::{ActionGate, ActionId, ActionSpec, ControlSlot};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, gate_body_control);
    let body = app
        .world_mut()
        .spawn((
            ambition_platformer2d_core::BodyAbilities::new(
                ambition_platformer2d_core::AbilitySet::sandbox_all(),
            ),
            ActionSet::peaceful(),
            ActorControl(ActorControlFrame::neutral()),
            WornEquipment::new(vec![EquipmentRow {
                id: "spinner".into(),
                grants: vec![EquipmentGrant::Technique(ActionSpec {
                    id: ActionId::new("spin_dash"),
                    slot: ControlSlot::Attack,
                    display_name: None,
                    visual: None,
                    gate: ActionGate::Technique("spin_dash".to_owned()),
                })],
                ..Default::default()
            }]),
        ))
        .id();
    let spun_after_a_press = |app: &mut App| {
        app.world_mut()
            .get_mut::<ActorControl>(body)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        app.world()
            .get::<ResolvedTechniqueEdges>(body)
            .unwrap()
            .pressed("spin_dash")
    };
    assert!(
        spun_after_a_press(&mut app),
        "the worn row's technique did not take the Attack edge"
    );
    app.world_mut()
        .get_mut::<WornEquipment>(body)
        .unwrap()
        .rows
        .clear();
    assert!(
        !spun_after_a_press(&mut app),
        "the row is off and its technique still gates the slot"
    );
}

/// THE PROBE FOR — A BODY'S SHIELD IS ITS OWN CAPABILITY, NOT A PROPERTY OF ITS SPECIAL.
///
/// So *which special do you carry* stood where *can you shield at all* belongs, and any persona
/// holding `AbilitySet::shield` alongside an ordinary special had its guard erased every single
/// frame. It is an independent participant control/action."*
///
/// Three bodies, because one proves nothing on its own:
/// * a shield-capable body with an ORDINARY special keeps the verb (the case the
///   old gate refused);
/// * a shield-capable body with the BUBBLE special keeps it too (the behaviour
///   that must not regress);
/// * the poison — a body with NO shield ability loses it, so this is a gate and
///   not a deletion.
#[test]
fn the_shield_verb_follows_the_ability_not_the_special() {
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::{ActionSet, SpecialActionSpec};
    use ambition_characters::control::ActorControl;
    use bevy::prelude::*;

    let guarding = || {
        let mut frame = ActorControlFrame::neutral();
        frame.shield_held = true;
        frame
    };
    let kit = |special: Option<&str>, shield: bool| {
        let mut abilities = ambition_platformer2d_core::AbilitySet::sandbox_all();
        abilities.shield = shield;
        let actions = ActionSet {
            special: special.map(|key| SpecialActionSpec::Special(key.to_owned())),
            ..ActionSet::default()
        };
        (abilities, actions)
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, gate_body_control);

    let mut spawn = |abilities, actions| {
        app.world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::markers::PlayerEntity,
                WornCharacter::new("sanic"),
                MotionModel::default(),
                ambition_platformer2d_core::BodyAbilities::new(abilities),
                actions,
                ActorControl(guarding()),
            ))
            .id()
    };
    let (abilities, actions) = kit(Some("meteor_kick"), true);
    let ordinary_special = spawn(abilities, actions);
    let (abilities, actions) = kit(Some("bubble_shield"), true);
    let bubble = spawn(abilities, actions);
    let (abilities, actions) = kit(Some("meteor_kick"), false);
    let no_shield_ability = spawn(abilities, actions);

    app.update();

    let guard = |app: &App, e: Entity| app.world().get::<ActorControl>(e).unwrap().0.shield_held;
    assert!(
        guard(&app, ordinary_special),
        "a body that OWNS the shield ability lost its guard because its special \
         is not the bubble shield. Shield is its own participant action — a gate \
         that reads the special key is asking Special whether Shield may happen"
    );
    assert!(
        guard(&app, bubble),
        "the bubble-shield persona lost the guard the old exception gave it"
    );
    assert!(
        !guard(&app, no_shield_ability),
        "a body with NO shield ability kept the guard verb, so nothing is gating \
         it and the assertions above prove nothing"
    );
}

/// The held-item exception survives the move: shield+attack is the universal
/// "throw the held item" gesture, so a body holding an item keeps the shield verb
/// even with no shield ability of its own.
#[test]
fn a_held_item_keeps_the_shield_verb_alive_without_the_ability() {
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use bevy::prelude::*;

    let mut frame = ActorControlFrame::neutral();
    frame.shield_held = true;
    let mut abilities = ambition_platformer2d_core::AbilitySet::sandbox_all();
    abilities.shield = false;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, gate_body_control);
    let body = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            WornCharacter::new("sanic"),
            MotionModel::default(),
            ambition_platformer2d_core::BodyAbilities::new(abilities),
            ActionSet::peaceful(),
            ambition_combat::held_items::HeldItem::new(ambition_characters::brain::HeldItemSpec {
                id: "rock".to_owned(),
                melee: None,
                ranged: None,
                use_behavior: Default::default(),
            }),
            ActorControl(frame),
        ))
        .id();
    app.update();

    assert!(
        app.world().get::<ActorControl>(body).unwrap().0.shield_held,
        "a body holding an item lost the shield half of the throw gesture"
    );
}

/// A SEAT-DRIVEN BODY OUTSIDE THE PLAYER POPULATION IS GATED BY ITS OWN SCHEME.
///
/// Possession and multi-seat drive an actor through the same `ActorControl`
/// frame as the home avatar, and content grants techniques to whatever body is
/// the controlled subject. A gate filtered to `PlayerEntity` left that body's
/// frame raw: its technique never received its edge, and it kept a guard verb
/// its abilities do not grant. No `WornCharacter` either, because a possessed
/// boss or a staged actor need not wear one.
#[test]
fn a_driven_actor_outside_the_player_population_is_gated_by_its_own_scheme() {
    use ambition_characters::action_scheme::{ActorTechniques, ResolvedTechniqueEdges};
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use ambition_entity_catalog::action_scheme::{ActionGate, ActionId, ActionSpec, ControlSlot};
    use bevy::prelude::*;

    let mut frame = ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.shield_held = true;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, gate_body_control);
    let mut abilities = ambition_platformer2d_core::AbilitySet::sandbox_all();
    abilities.shield = false;
    let body = app
        .world_mut()
        .spawn((
            ambition_platformer2d_core::BodyAbilities::new(abilities),
            ActionSet::peaceful(),
            ActorTechniques(vec![ActionSpec {
                id: ActionId::new("spin_dash"),
                slot: ControlSlot::Attack,
                display_name: None,
                visual: None,
                gate: ActionGate::Technique("spin_dash".to_owned()),
            }]),
            ActorControl(frame),
        ))
        .id();
    app.update();

    let world = app.world();
    let edges = world
        .get::<ResolvedTechniqueEdges>(body)
        .expect("ActorTechniques requires its edge sink");
    assert!(
        edges.pressed("spin_dash"),
        "the driven actor's Attack press never reached its technique: {edges:?}"
    );
    let gated = &world.get::<ActorControl>(body).unwrap().0;
    assert!(
        !gated.melee_pressed,
        "a technique-owned Attack press must not also survive as a raw melee verb"
    );
    assert!(
        !gated.shield_held,
        "a body without the shield ability kept its guard verb"
    );
}

/// OW1: the driven body wears the techniques of its OWN live room's rules.
/// Smash drives a Spin Dash in its own rooms only. With the hall (#0, no
/// mode) and the Smash stage (#1) live, the driven body on the stage spins on
/// an attack press, and the same body driven in the hall does not. Before,
/// the gate read THE live room, which with two rooms live is no room.
#[test]
fn the_driven_body_wears_the_techniques_of_its_own_live_room() {
    use ambition_characters::action_scheme::{DrivenTechniques, ResolvedTechniqueEdges};
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use ambition_combat::scoped_rules::{DeclareRulesExt, RulesScope};
    use ambition_entity_catalog::action_scheme::{ActionGate, ActionId, ActionSpec, ControlSlot};
    use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
    use bevy::prelude::*;

    let spins_in = |stage: bool| {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.declare_rules(
            RulesScope::Mode("smash"),
            DrivenTechniques(vec![ActionSpec {
                id: ActionId::new("spin_dash"),
                slot: ControlSlot::Attack,
                display_name: None,
                visual: None,
                gate: ActionGate::Technique("spin_dash".to_owned()),
            }]),
        );
        app.add_systems(Update, gate_body_control);
        let hall =
            ambition_combat::governing_rules::test_fixtures::two_game_session(&mut app, true);
        let room = if stage { hall.next() } else { hall };
        let mut frame = ActorControlFrame::neutral();
        frame.melee_pressed = true;
        let body = app
            .world_mut()
            .spawn((
                ambition_platformer2d_core::BodyAbilities::new(
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
                ActionSet::peaceful(),
                ActorControl(frame),
                InRoomInstance(room),
                DrivingParticipant(ambition_characters::control::PlayerSlot::PRIMARY),
            ))
            .id();
        app.update();
        app.world()
            .get::<ResolvedTechniqueEdges>(body)
            .expect("an action set brings its edge sink")
            .pressed("spin_dash")
    };
    assert!(
        spins_in(true),
        "the driven body on the Smash stage (#1) did not wear Smash's technique"
    );
    assert!(
        !spins_in(false),
        "the driven body in the hall (#0) wore Smash's technique"
    );
}

/// The gate keeps the scheme it derived (findings F16). A quiet tick does not
/// derive it again. A changed, added or removed authority derives it again,
/// and the result is the scheme a fresh derivation of the body gives.
#[test]
fn the_gate_derives_the_scheme_again_only_when_an_authority_changed() {
    use ambition_characters::action_scheme::{
        ActorTechniques, DerivedActionScheme, derive_action_scheme,
    };
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use ambition_entity_catalog::action_scheme::{ActionGate, ActionId, ActionSpec, ControlSlot};
    use ambition_platformer2d_core::{AbilitySet, BodyAbilities};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, gate_body_control);
    let body = app
        .world_mut()
        .spawn((
            BodyAbilities::new(AbilitySet::sandbox_all()),
            ActionSet::peaceful(),
            ActorControl(ActorControlFrame::neutral()),
        ))
        .id();
    let spin = ActionSpec {
        id: ActionId::new("spin_dash"),
        slot: ControlSlot::Attack,
        display_name: None,
        visual: None,
        gate: ActionGate::Technique("spin_dash".to_owned()),
    };
    // The authority: a fresh derivation from the body as it is now.
    let fresh = |app: &App| {
        let world = app.world();
        let techniques = world
            .get::<ActorTechniques>(body)
            .map(|t| t.0.clone())
            .unwrap_or_default();
        derive_action_scheme(
            &world.get::<BodyAbilities>(body).unwrap().abilities,
            None,
            world.get::<ActionSet>(body),
            &techniques,
        )
    };
    let derived_tick = |app: &App| {
        app.world()
            .entity(body)
            .get_ref::<DerivedActionScheme>()
            .unwrap()
            .last_changed()
    };
    let derived = |app: &App| {
        app.world()
            .get::<DerivedActionScheme>(body)
            .unwrap()
            .scheme
            .clone()
            .unwrap()
    };

    app.update();
    assert_eq!(
        derived(&app),
        fresh(&app),
        "the first tick derives the scheme"
    );
    let first = derived_tick(&app);
    app.update();
    app.update();
    assert_eq!(derived_tick(&app), first, "a quiet tick reuses the scheme");

    app.world_mut()
        .entity_mut(body)
        .insert(ActorTechniques(vec![spin]));
    app.update();
    assert!(
        derived(&app)
            .actions
            .iter()
            .any(|a| a.id.as_str() == "spin_dash"),
        "an added technique is on the scheme"
    );
    assert_eq!(
        derived(&app),
        fresh(&app),
        "an added authority derives again"
    );

    app.world_mut().entity_mut(body).remove::<ActorTechniques>();
    app.update();
    assert_eq!(
        derived(&app),
        fresh(&app),
        "a removed authority derives again"
    );

    let before = derived(&app);
    app.world_mut()
        .get_mut::<BodyAbilities>(body)
        .unwrap()
        .abilities = AbilitySet::basic();
    app.update();
    assert_ne!(derived(&app), before, "fewer abilities give another scheme");
    assert_eq!(
        derived(&app),
        fresh(&app),
        "a changed authority derives again"
    );
}

#[test]
fn two_driving_seats_resolve_their_own_room_techniques_and_release_independently() {
    use ambition_characters::action_scheme::{DrivenTechniques, ResolvedTechniqueEdges};
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::{ActorControl, DrivingParticipant, PlayerSlot};
    use ambition_combat::scoped_rules::{DeclareRulesExt, RulesScope};
    use ambition_entity_catalog::action_scheme::{ActionGate, ActionId, ActionSpec, ControlSlot};
    use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
    use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
    use bevy::prelude::*;

    let mut app = App::new();
    for (scope, id) in [
        (RulesScope::UntaggedRooms, "hall_attack"),
        (RulesScope::Mode("smash"), "stage_attack"),
    ] {
        app.declare_rules(
            scope,
            DrivenTechniques(vec![ActionSpec {
                id: ActionId::new(id),
                slot: ControlSlot::Attack,
                display_name: None,
                visual: None,
                gate: ActionGate::Technique(id.to_owned()),
            }]),
        );
    }
    let hall = ambition_combat::governing_rules::test_fixtures::two_game_session(&mut app, true);
    let mut spawn = |room| {
        app.world_mut()
            .spawn((
                ambition_platformer2d_core::BodyAbilities::new(
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
                ActionSet::peaceful(),
                ActorControl(ActorControlFrame::neutral()),
                InRoomInstance(room),
            ))
            .id()
    };
    let first = spawn(hall);
    let second = spawn(hall.next());
    let autonomous = spawn(hall.next());
    app.world_mut()
        .entity_mut(first)
        .insert(DrivingParticipant(PlayerSlot::PRIMARY));
    app.world_mut()
        .entity_mut(second)
        .insert(DrivingParticipant(PlayerSlot(1)));
    app.insert_resource(ControlledSubject(Some(first)));
    app.add_systems(Update, gate_body_control);
    let press = |app: &mut App| {
        for entity in [first, second, autonomous] {
            app.world_mut()
                .get_mut::<ActorControl>(entity)
                .unwrap()
                .0
                .melee_pressed = true;
        }
        app.update();
    };
    let routed = |app: &App, entity, id| {
        app.world()
            .get::<ResolvedTechniqueEdges>(entity)
            .unwrap()
            .pressed(id)
    };
    press(&mut app);
    assert!(routed(&app, first, "hall_attack"));
    assert!(
        routed(&app, second, "stage_attack"),
        "the second driven body did not receive its room's technique"
    );
    assert!(!routed(&app, first, "stage_attack"));
    assert!(!routed(&app, second, "hall_attack"));
    assert!(!routed(&app, autonomous, "stage_attack"));

    app.world_mut().remove_resource::<ControlledSubject>();
    app.world_mut()
        .entity_mut(second)
        .remove::<DrivingParticipant>();
    app.world_mut()
        .entity_mut(autonomous)
        .insert(DrivingParticipant(PlayerSlot(1)));
    press(&mut app);
    assert!(routed(&app, first, "hall_attack"));
    assert!(
        !routed(&app, second, "stage_attack"),
        "the released body retained the room's driven technique"
    );
    assert!(routed(&app, autonomous, "stage_attack"));
}

/// A lured boss on the one walk road ([`drive_commanded_moves`]): a boss
/// carrying `BossAttackIntent` walks toward its mark, and starts no attack.
fn a_lured_boss(
    app: &mut App,
    current_hp: i32,
) -> (Entity, ambition_characters::brain::BossAttackState) {
    let wanted = ambition_characters::brain::BossAttackProfile::Strike("floor_slam".into());
    let projected = ambition_characters::brain::BossAttackState {
        active_profile: Some(wanted.clone()),
        active_remaining: 0.5,
        ..Default::default()
    };
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    let boss = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::splat(40.0),
                facing: -1.0,
            },
            BodyHealth::new(ambition_characters::actor::Health {
                current: current_hp,
                max: 100,
                invulnerable: Default::default(),
            }),
            ActorControl(frame),
            projected.clone(),
            ambition_characters::brain::BossAttackIntent {
                telegraph_profile: Some(wanted.clone()),
                active_profile: Some(wanted),
                ..Default::default()
            },
            ambition_characters::control::CommandedMove {
                target: ae::Vec2::new(300.0, 0.0),
                speed: 150.0,
                arrive_tolerance: 10.0,
            },
        ))
        .id();
    (boss, projected)
}

#[test]
fn a_lured_boss_walks_to_its_mark_and_starts_no_attack() {
    let mut app = App::new();
    app.add_systems(Update, drive_commanded_moves);
    let (boss, projected) = a_lured_boss(&mut app, 100);
    app.update();

    let world = app.world();
    let control = world.get::<ActorControl>(boss).expect("control").0;
    assert!(
        control.velocity_target.x > 0.0,
        "the boss is lured toward the +x mark"
    );
    assert_eq!(control.facing, 1.0, "and faces the mark");
    assert!(!control.melee_pressed, "a lured boss presses nothing");
    assert_eq!(
        world.get::<ambition_characters::brain::BossAttackIntent>(boss),
        Some(&ambition_characters::brain::BossAttackIntent::default()),
        "a lured boss still wants to attack, so the trigger would start its move"
    );
    assert_eq!(
        world
            .get::<ambition_characters::brain::BossAttackState>(boss)
            .map(|state| &state.active_profile),
        Some(&projected.active_profile),
        "the walk wrote the move projection's read-model"
    );
}

#[test]
fn a_dead_body_is_not_walked() {
    let mut app = App::new();
    app.add_systems(Update, drive_commanded_moves);
    let (boss, _) = a_lured_boss(&mut app, 0);
    app.update();

    let world = app.world();
    let control = world.get::<ActorControl>(boss).expect("control").0;
    assert_eq!(control.velocity_target.x, 0.0, "a dead boss was walked");
    assert!(
        world
            .get::<ambition_characters::brain::BossAttackIntent>(boss)
            .is_some_and(|intent| intent.active_profile.is_some()),
        "a dead boss's intent was cleared by a walk it does not take"
    );
}
