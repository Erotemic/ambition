//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

//! The portal input-warp + wall-ability suppression adapters (Stage 19
//! Phase 5a — moved out of the portal crate because they shape INPUT /
//! mutate the PLAYER's abilities, neither of which the crate owns). These
//! drive the REAL adapters + the portal-owned marker components.
use bevy::prelude::*;

use ambition_actors::actor::{PlayerEntity, PrimaryPlayer};
use ambition_input::ControlFrame;
use ambition_portal::{
    PlayerMovementIntent, PortalChannel, PortalEmission, PortalGunColor, PortalInputWarp,
    PortalTransit, PortalTuning,
};

use super::{
    restore_wall_abilities_after_transit, suppress_ledge_grab_during_transit, warp_portal_input,
};

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);

#[test]
fn portal_input_warp_transforms_held_input_then_clears() {
    use crate::portal::{apply_movement_intent_to_control, sync_movement_intent_from_control};
    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.init_resource::<PlayerMovementIntent>();
    app.init_resource::<PortalTuning>();
    // The content adapter brackets the core warp: mirror ControlFrame -> intent
    // before the warp, and the warped intent -> ControlFrame after, so this
    // exercises the full content+core chain on the ControlFrame surface exactly
    // as the game does.
    app.add_systems(
        Update,
        (
            sync_movement_intent_from_control,
            warp_portal_input,
            apply_movement_intent_to_control,
        )
            .chain(),
    );
    // A 180° warp (a same-wall pair). Player holds RIGHT (anchor right).
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            PortalInputWarp {
                n_in: Vec2::new(-1.0, 0.0),
                n_out: Vec2::new(-1.0, 0.0),
                anchor: Vec2::new(1.0, 0.0),
            },
        ))
        .id();

    // Still holding right → input is warped to LEFT (keeps you moving out).
    app.world_mut().resource_mut::<ControlFrame>().axis_x = 1.0;
    app.update();
    assert!(
        app.world().resource::<ControlFrame>().axis_x < -0.5,
        "held right is warped to left while the warp is active"
    );
    assert!(
        app.world().get::<PortalInputWarp>(player).is_some(),
        "warp persists while held"
    );

    // Release movement → warp drops, input passes through untouched next frame.
    app.world_mut().resource_mut::<ControlFrame>().axis_x = 0.0;
    app.update();
    assert!(
        app.world().get::<PortalInputWarp>(player).is_none(),
        "release drops the warp"
    );

    // Re-arm, then press a clearly different direction (left) → warp drops.
    app.world_mut().entity_mut(player).insert(PortalInputWarp {
        n_in: Vec2::new(-1.0, 0.0),
        n_out: Vec2::new(-1.0, 0.0),
        anchor: Vec2::new(1.0, 0.0),
    });
    app.world_mut().resource_mut::<ControlFrame>().axis_x = -1.0;
    app.update();
    assert!(
        app.world().get::<PortalInputWarp>(player).is_none(),
        "a clearly different direction drops the warp"
    );
}

#[test]
fn wall_ability_suppression_reapplies_every_frame_against_the_loadout_reset() {
    use ambition_actors::actor::BodyAbilities;
    let mut app = App::new();
    app.init_resource::<PortalTuning>();
    // Stand in for the per-frame loadout reset that clobbered the old
    // save-once suppression: re-enable ledge_grab BEFORE the suppressor runs.
    fn reenable_ledge_grab(mut q: Query<&mut BodyAbilities>) {
        for mut a in &mut q {
            a.abilities.ledge_grab = true;
        }
    }
    app.add_systems(
        Update,
        (reenable_ledge_grab, suppress_ledge_grab_during_transit).chain(),
    );
    let player = app
        .world_mut()
        .spawn((PlayerEntity, PrimaryPlayer, BodyAbilities::default()))
        .id();
    app.world_mut()
        .get_mut::<BodyAbilities>(player)
        .unwrap()
        .abilities
        .ledge_grab = true;

    // Not transiting: the reset wins, ledge_grab stays enabled.
    app.update();
    assert!(
        app.world()
            .get::<BodyAbilities>(player)
            .unwrap()
            .abilities
            .ledge_grab
    );

    // Transiting: even though the reset re-enables it first, the suppressor
    // re-applies every frame, so it stays disabled across MANY frames.
    app.world_mut().entity_mut(player).insert(PortalTransit {
        straddling: BLUE,
        crossed: false,
    });
    for _ in 0..5 {
        app.update();
        assert!(
            !app.world()
                .get::<BodyAbilities>(player)
                .unwrap()
                .abilities
                .ledge_grab,
            "ledge_grab must stay suppressed every frame while transiting"
        );
    }

    // Transit ends: the per-frame reset restores it (no save/restore needed).
    app.world_mut().entity_mut(player).remove::<PortalTransit>();
    app.update();
    assert!(
        app.world()
            .get::<BodyAbilities>(player)
            .unwrap()
            .abilities
            .ledge_grab
    );
}

/// The aperture-edge hazard is a property of TRANSITING, not of being the
/// primary player: a plain actor (no player markers) mid-transit has its wall
/// verbs suppressed, and — because no per-frame F3 re-sync covers it — the
/// paired restore must put them back from its authored `AbilityBase` when the
/// latch is removed. Without the restore the actor stays stripped forever.
#[test]
fn wall_ability_suppression_is_body_generic_and_restores_from_the_base() {
    use ambition_actors::actor::BodyAbilities;
    let mut app = App::new();
    app.init_resource::<PortalTuning>();
    app.add_systems(
        Update,
        (
            suppress_ledge_grab_during_transit,
            restore_wall_abilities_after_transit,
        )
            .chain(),
    );
    // An actor: NO PlayerEntity/PrimaryPlayer. Authored with ledge_grab +
    // wall_jump (its base), currently transiting.
    let mut authored = BodyAbilities::default();
    authored.abilities.ledge_grab = true;
    authored.abilities.wall_jump = true;
    let actor = app
        .world_mut()
        .spawn((
            authored.clone(),
            ambition_engine_core::AbilityBase::new(authored.abilities),
            PortalTransit {
                straddling: BLUE,
                crossed: false,
            },
        ))
        .id();

    app.update();
    let a = &app.world().get::<BodyAbilities>(actor).unwrap().abilities;
    assert!(
        !a.ledge_grab && !a.wall_jump,
        "a transiting ACTOR has its wall verbs suppressed too"
    );

    // Transit ends: the verbs come back from the authored base (no F3 re-sync
    // exists for this body).
    app.world_mut().entity_mut(actor).remove::<PortalTransit>();
    app.update();
    let a = &app.world().get::<BodyAbilities>(actor).unwrap().abilities;
    assert!(
        a.ledge_grab && a.wall_jump,
        "transit end restores the actor's wall verbs from its AbilityBase"
    );
    assert!(
        !a.wall_cling && !a.wall_climb,
        "verbs the base never granted stay off"
    );
}

/// The emergence guard follows the DRIVEN body: possess an actor, send it
/// through a portal, and ITS `PortalEmission` shapes the local input stream —
/// previously only the primary player's guard was consulted, so a possessed
/// emergence had no input protection at all.
#[test]
fn emission_guard_follows_the_possessed_body() {
    use crate::portal::{apply_movement_intent_to_control, sync_movement_intent_from_control};
    use ambition_platformer_primitives::markers::ControlledSubject;
    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.init_resource::<PlayerMovementIntent>();
    app.init_resource::<PortalTuning>();
    app.add_systems(
        Update,
        (
            sync_movement_intent_from_control,
            warp_portal_input,
            apply_movement_intent_to_control,
        )
            .chain(),
    );
    // Home avatar has NO emission; the possessed actor is the one emerging
    // from a right-wall portal (exit normal LEFT, into the room).
    app.world_mut().spawn((PlayerEntity, PrimaryPlayer));
    let possessed = app
        .world_mut()
        .spawn(PortalEmission {
            exit_normal: Vec2::new(-1.0, 0.0),
            timer: 1.0,
        })
        .id();
    app.world_mut()
        .insert_resource(ControlledSubject(Some(possessed)));

    // Holding RIGHT (back into the wall) is stripped for the DRIVEN body.
    app.world_mut().resource_mut::<ControlFrame>().axis_x = 1.0;
    app.update();
    assert!(
        app.world().resource::<ControlFrame>().axis_x.abs() < 0.01,
        "the POSSESSED body's emergence guard shapes the input stream"
    );
}

#[test]
fn emission_guard_strips_input_pushing_back_into_the_exit_wall() {
    use crate::portal::{apply_movement_intent_to_control, sync_movement_intent_from_control};
    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.init_resource::<PlayerMovementIntent>();
    app.init_resource::<PortalTuning>();
    app.add_systems(
        Update,
        (
            sync_movement_intent_from_control,
            warp_portal_input,
            apply_movement_intent_to_control,
        )
            .chain(),
    );
    // Emerging from a right-wall portal — exit_normal points LEFT (into room).
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            PortalEmission {
                exit_normal: Vec2::new(-1.0, 0.0),
                timer: 1.0,
            },
        ))
        .id();
    // Holding RIGHT (back into the wall) is stripped so physics carries you out.
    app.world_mut().resource_mut::<ControlFrame>().axis_x = 1.0;
    app.update();
    assert!(
        app.world().resource::<ControlFrame>().axis_x.abs() < 0.01,
        "input pushing back into the exit wall is stripped during emergence"
    );
    // Holding LEFT (the emergence direction) passes through untouched.
    app.world_mut().resource_mut::<ControlFrame>().axis_x = -1.0;
    app.update();
    assert!(
        app.world().resource::<ControlFrame>().axis_x < -0.5,
        "input in the emergence direction is preserved"
    );
    let _ = player;
}
