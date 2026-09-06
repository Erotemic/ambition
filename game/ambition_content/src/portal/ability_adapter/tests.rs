//! These drive the REAL adapters + the portal-owned marker components.
use bevy::prelude::*;

use ambition_characters::control::{DrivingParticipant, PlayerSlot, SeatRawFrames, SlotControls};
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

/// Seat-local input surface read and written by the portal warp.
fn hold_x(app: &mut App, axis_x: f32) {
    let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
    let mut frame = raw.get(PlayerSlot::PRIMARY);
    frame.axis_x = axis_x;
    raw.set(PlayerSlot::PRIMARY, frame);
}

fn seat_axis_x(app: &App) -> f32 {
    app.world()
        .resource::<SeatRawFrames>()
        .get(PlayerSlot::PRIMARY)
        .axis_x
}
use ambition_portal2d::{
    PortalChannel, PortalEmission, PortalGunColor, PortalInputWarp, PortalTransit, PortalTuning,
};

use super::{
    restore_wall_abilities_after_transit, suppress_ledge_grab_during_transit, warp_portal_input,
};

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);

#[test]
fn portal_input_warp_transforms_held_input_then_clears() {
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    app.init_resource::<PortalTuning>();
    // The content adapter brackets the core warp: mirror ControlFrame -> intent
    // before the warp, and the warped intent -> ControlFrame after, so this
    // exercises the full content+core chain on the ControlFrame surface exactly
    // as the game does.
    app.add_systems(Update, warp_portal_input);
    // A 180° warp (a same-wall pair). Player holds RIGHT (anchor right).
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            DrivingParticipant(PlayerSlot::PRIMARY),
            PortalInputWarp {
                n_in: Vec2::new(-1.0, 0.0),
                n_out: Vec2::new(-1.0, 0.0),
                anchor: Vec2::new(1.0, 0.0),
            },
        ))
        .id();

    // Still holding right → input is warped to LEFT (keeps you moving out).
    hold_x(&mut app, 1.0);
    app.update();
    assert!(
        seat_axis_x(&app) < -0.5,
        "held right is warped to left while the warp is active"
    );
    assert!(
        app.world().get::<PortalInputWarp>(player).is_some(),
        "warp persists while held"
    );

    // Release movement → warp drops, input passes through untouched next frame.
    hold_x(&mut app, 0.0);
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
    hold_x(&mut app, -1.0);
    app.update();
    assert!(
        app.world().get::<PortalInputWarp>(player).is_none(),
        "a clearly different direction drops the warp"
    );
}

#[test]
fn wall_ability_suppression_reapplies_every_frame_against_the_loadout_reset() {
    use ambition_platformer2d_core::BodyAbilities;
    let mut app = App::new();
    app.init_resource::<PortalTuning>();
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
    use ambition_platformer2d_core::BodyAbilities;
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
            ambition_platformer2d_core::AbilityBase::new(authored.abilities),
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

/// The emergence guard follows the DRIVEN body: possess an actor, send it through a portal, and
/// ITS `PortalEmission` shapes the local input stream
#[test]
fn emission_guard_follows_the_possessed_body() {
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    app.init_resource::<PortalTuning>();
    app.add_systems(Update, warp_portal_input);
    // Home avatar has NO emission and no seat while possessed; the possessed
    // actor carries the seat and is the one emerging from a right-wall portal
    // (exit normal LEFT, into the room).
    app.world_mut().spawn((PlayerEntity, PrimaryPlayer));
    let _possessed = app
        .world_mut()
        .spawn((
            DrivingParticipant(PlayerSlot::PRIMARY),
            PortalEmission {
                exit_normal: Vec2::new(-1.0, 0.0),
                timer: 1.0,
            },
        ))
        .id();

    // Holding RIGHT (back into the wall) is stripped for the DRIVEN body.
    hold_x(&mut app, 1.0);
    app.update();
    assert!(
        seat_axis_x(&app).abs() < 0.01,
        "the POSSESSED body's emergence guard shapes the input stream"
    );
}

#[test]
fn emission_guard_strips_input_pushing_back_into_the_exit_wall() {
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    app.init_resource::<PortalTuning>();
    app.add_systems(Update, warp_portal_input);
    // Emerging from a right-wall portal — exit_normal points LEFT (into room).
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            DrivingParticipant(PlayerSlot::PRIMARY),
            PortalEmission {
                exit_normal: Vec2::new(-1.0, 0.0),
                timer: 1.0,
            },
        ))
        .id();
    // Holding RIGHT (back into the wall) is stripped so physics carries you out.
    hold_x(&mut app, 1.0);
    app.update();
    assert!(
        seat_axis_x(&app).abs() < 0.01,
        "input pushing back into the exit wall is stripped during emergence"
    );
    // Holding LEFT (the emergence direction) passes through untouched.
    hold_x(&mut app, -1.0);
    app.update();
    assert!(
        seat_axis_x(&app) < -0.5,
        "input in the emergence direction is preserved"
    );
    let _ = player;
}
