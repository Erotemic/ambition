//! Local play modes (Q153, ruled 2026-10-19), in the shipped composition with
//! real device events.
//!
//! - Single-player (the default): the keyboard and every pad drive player one,
//!   also a pad connected after the session started, and no button of a
//!   second pad seats a second player.
//! - Accepting joins: a pad that presses Start claims seat 1 and joins; then
//!   it moves player two only, and the other devices move player one only.
//! - The setting changes while the session runs, with no restart.

use bevy::input::gamepad::{RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent, RawGamepadEvent};
use bevy::prelude::*;
use leafwing_input_manager::prelude::Buttonlike;

use ambition_platformer2d::characters::control::{
    DrivingParticipant, PlayerSlot, SeatRawFrames, SlotControls,
};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::platformer::lifecycle::SessionRoot;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

use crate::shell_host_rendered::{rendered_app, settle};

const SEAT: u8 = 1;

/// A device that can push a body to the right.
#[derive(Clone, Copy, Debug)]
enum Device {
    Keyboard,
    Pad(Entity),
}

fn connect_a_pad(app: &mut App) -> Entity {
    let pad = app.world_mut().spawn(bevy::input::gamepad::Gamepad::default()).id();
    app.update();
    pad
}

/// Connect a pad with the name the OS gives it, which stays the same when the
/// controller is unplugged and plugged in again.
fn connect_named_pad(app: &mut App, name: &str) -> Entity {
    let pad = app
        .world_mut()
        .spawn((bevy::input::gamepad::Gamepad::default(), Name::new(name.to_string())))
        .id();
    app.update();
    pad
}

fn pad_button(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
    app.world_mut()
        .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(pad, button, value)));
}

/// Move one connected pad's physical left stick, as the OS backend reports it.
///
/// A RAW axis event, not a write to the `Gamepad` component: Bevy's event
/// processing applies the pad's deadzone and livezone settings on the way in,
/// and that is the road a real stick travels.
fn pad_left_stick_x(app: &mut App, pad: Entity, value: f32) {
    app.world_mut().write_message(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
        pad,
        bevy::input::gamepad::GamepadAxis::LeftStickX,
        value,
    )));
}

fn tap(app: &mut App, pad: Entity, button: GamepadButton) {
    pad_button(app, pad, button, 1.0);
    app.update();
    pad_button(app, pad, button, 0.0);
    app.update();
}

/// The setting, as the settings screen writes it.
fn accept_joins(app: &mut App, accept: bool) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::settings::UserSettings>()
        .gameplay
        .accept_local_joins = accept;
}

/// An Ambition session in the rendered host with `pads` pads connected before
/// it starts and the setting `accept`. Pad `n` is named "pad n".
fn session(pads: usize, accept: bool) -> (App, Vec<Entity>) {
    session_in(rendered_app(), pads, accept)
}

fn session_in(mut app: App, pads: usize, accept: bool) -> (App, Vec<Entity>) {
    settle(&mut app);
    accept_joins(&mut app, accept);
    let connected = (0..pads).map(|index| connect_named_pad(&mut app, &format!("pad {index}"))).collect();
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle(&mut app);
    for _ in 0..30 {
        app.update();
    }
    (app, connected)
}

fn the_primary(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<PrimaryPlayer>>()
        .single(world)
        .expect("one primary body")
}

fn seat_one_bodies(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    world
        .query::<(Entity, &DrivingParticipant)>()
        .iter(world)
        .filter(|(_, driver)| driver.0 == PlayerSlot(SEAT))
        .map(|(entity, _)| entity)
        .collect()
}

#[test]
fn player_two_controls_its_dialogue_while_player_one_keeps_playing() {
    let (app, pads) = session(2, true);
    let (mut app, pads) = player_two_dialogue_at_choices(app, pads, false);
    tap(&mut app, pads[0], GamepadButton::East);
    assert!(app.world().resource::<ambition_platformer2d::dialog::DialogState>().active());
    tap(&mut app, pads[1], GamepadButton::East);
    assert!(!app.world().resource::<ambition_platformer2d::dialog::DialogState>().active());
}

#[test]
fn player_two_can_talk_while_player_one_plays_in_the_same_room() {
    let (app, pads) = session(2, true);
    let (app, _) = player_two_dialogue_at_choices(app, pads, true);
    assert_eq!(
        app.world().resource::<State<ambition_platformer2d::platformer::schedule::GameMode>>().get(),
        &ambition_platformer2d::platformer::schedule::GameMode::Playing,
        "a conversation must not suspend the other participant's world actions",
    );
}

fn player_two_dialogue_at_choices(mut app: App, pads: Vec<Entity>, together: bool) -> (App, Vec<Entity>) {
    use ambition_platformer2d::dialog::DialogState;
    use ambition_platformer2d::input::InputOwner;
    use ambition_platformer2d::input::ParticipantId;
    use crate::neighbor_prefetch_prepares_rooms::cross;
    use crate::the_note_travels_from_alice_to_bob::{npc, put_body_at};
    tap(&mut app, pads[1], GamepadButton::Start);
    for _ in 0..6 { app.update(); }
    let two = seat_one_bodies(&mut app)[0];
    let one = the_primary(&mut app);
    cross(&mut app, two, PlayerSlot(1), "alice_relay");
    if together {
        cross(&mut app, one, PlayerSlot(0), "alice_relay");
    }
    let alice = npc(&mut app, "npc_alice");
    let at = app.world().get::<BodyKinematics>(alice).unwrap().pos;
    put_body_at(&mut app, two, at);
    for _ in 0..10 { app.update(); }
    tap(&mut app, pads[1], GamepadButton::RightTrigger);
    assert_eq!(app.world().resource::<DialogState>().input_owner(), Some(InputOwner::Participant(ParticipantId(1))));
    for _ in 0..80 {
        if app.world().resource::<DialogState>().options().len() >= 2 { break; }
        tap(&mut app, pads[1], GamepadButton::South);
        app.update();
    }
    assert!(app.world().resource::<DialogState>().options().len() >= 2, "player two could not reach Alice's choices");
    let selected = app.world().resource::<DialogState>().selected_option();
    tap(&mut app, pads[0], GamepadButton::DPadDown);
    assert_eq!(app.world().resource::<DialogState>().selected_option(), selected);
    pad_button(&mut app, pads[1], GamepadButton::DPadRight, 1.0);
    app.update();
    assert!(!app.world().resource::<ambition_platformer2d::input::SeatInputContexts>().gameplay_owned(1));
    assert_eq!(app.world().resource::<SlotControls>().get(PlayerSlot(1)).axis_x, 0.0);
    pad_button(&mut app, pads[1], GamepadButton::DPadRight, 0.0);
    app.update();
    let [one_moved, _] = push_right(&mut app, Device::Pad(pads[0]), &[one, two])[..] else { unreachable!() };
    assert!(one_moved >= MOVED, "the unrelated participant could not move");
    assert!(app.world().resource::<DialogState>().active());
    tap(&mut app, pads[1], GamepadButton::DPadDown);
    assert_ne!(app.world().resource::<DialogState>().selected_option(), selected);
    let view = app.world().resource::<ambition_platformer2d::sim_view::DialogView>();
    assert_eq!(view.input_owner, Some(InputOwner::Participant(ParticipantId(1))));
    assert!(view.instance.is_some());
    let region = app.world().resource::<ambition_platformer2d::ui_nav::DialogPointerRegion>().0.unwrap();
    if !together {
        use ambition_platformer2d::sim_view::{LocalView, ViewParticipant};
        use ambition_platformer2d::sim_view::camera_snapshot::CameraViewport;
        let world = app.world_mut();
        let mut views = world.query_filtered::<(&ViewParticipant, &CameraViewport), With<LocalView>>();
        let (_, viewport) = views.iter(world).find(|(participant, _)| participant.0 == PlayerSlot(1))
            .expect("the talker's room has a view");
        let min = Vec2::new(viewport.origin_px.x, viewport.origin_px.y);
        let max = min + Vec2::new(viewport.px.x, viewport.px.y);
        assert!(region.min.cmpge(min).all() && region.max.cmple(max).all(),
            "dialogue {region:?} extends outside the talker's camera {viewport:?}");
    }
    (app, pads)
}

#[test]
#[ignore = "needs a wgpu adapter; writes /tmp/ambition-p2-dialogue.png for visual review"]
fn player_two_dialogue_draws_in_its_own_view() {
    use ambition_platformer2d::render::capture::{
        CaptureProgress, CaptureSettings, CaptureTarget, adopt_cameras_into_capture_target,
        request_capture, setup_capture_target,
    };
    let mut app = crate::shell_host_rendered::rendered_app_with(ambition_app::app::VisibleRenderMode::OffscreenGpu);
    let output = std::env::temp_dir().join("ambition-p2-dialogue.png");
    app.insert_resource(CaptureSettings { output: output.clone(), size: UVec2::new(1280, 720), include_ui: true });
    app.init_resource::<CaptureProgress>();
    app.add_systems(Startup, setup_capture_target);
    app.add_systems(Update, adopt_cameras_into_capture_target);
    let (app, pads) = session_in(app, 2, true);
    let (mut app, _) = player_two_dialogue_at_choices(app, pads, false);
    for _ in 0..30 { app.update(); }
    app.add_systems(Update, |mut commands: Commands, target: Res<CaptureTarget>, mut progress: ResMut<CaptureProgress>| {
        request_capture(&mut commands, &target, &mut progress);
    });
    let started = std::time::Instant::now();
    while !app.world().resource::<CaptureProgress>().completed {
        assert!(started.elapsed() < crate::common::ACTIVATION_CEILING, "the dialogue capture did not complete");
        app.update();
    }
    assert!(!app.world().resource::<CaptureProgress>().failed);
    assert!(output.is_file());
    let world = app.world_mut();
    let region = world.resource::<ambition_platformer2d::ui_nav::DialogPointerRegion>().0.unwrap();
    let mut roots = world.query_filtered::<(&ComputedNode, &bevy::ui::UiGlobalTransform), With<ambition_platformer2d::render::dialog_ui::DialogOverlayRoot>>();
    let (node, transform) = roots.single(world).unwrap();
    let (_, _, center) = transform.to_scale_angle_translation();
    let bounds = Rect::from_center_size(center * node.inverse_scale_factor(), node.size() * node.inverse_scale_factor());
    eprintln!("dialogue input region: {region:?}; computed UI bounds: {bounds:?}");
    assert!((bounds.min - region.min).abs().max_element() <= 1.0
        && (bounds.max - region.max).abs().max_element() <= 1.0,
        "the rendered dialogue extends outside its input region");
}

fn x_of(app: &App, body: Entity) -> f32 {
    app.world().get::<BodyKinematics>(body).expect("a body").pos.x
}

/// Hold right on `device` for 24 frames, then let go and settle. Returns how
/// far each of `bodies` moved.
fn push_right(app: &mut App, device: Device, bodies: &[Entity]) -> Vec<i32> {
    let before: Vec<f32> = bodies.iter().map(|body| x_of(app, *body)).collect();
    for _ in 0..24 {
        match device {
            Device::Keyboard => Buttonlike::press(&KeyCode::ArrowRight, app.world_mut()),
            Device::Pad(pad) => pad_button(app, pad, GamepadButton::DPadRight, 1.0),
        }
        app.update();
    }
    match device {
        Device::Keyboard => Buttonlike::release(&KeyCode::ArrowRight, app.world_mut()),
        Device::Pad(pad) => pad_button(app, pad, GamepadButton::DPadRight, 0.0),
    }
    for _ in 0..20 {
        app.update();
    }
    bodies
        .iter()
        .zip(before)
        .map(|(body, x)| (x_of(app, *body) - x).round() as i32)
        .collect()
}

/// A device moved a body when the body went this far right.
const MOVED: i32 = 12;

/// SINGLE-PLAYER. With two pads connected before the session, the keyboard
/// and each pad move player one, and so does a pad connected after the
/// session started. The second pad's Jump and Start build no body for seat 1.
/// Control: player one stands still when nothing is pressed.
#[test]
fn in_single_player_mode_every_device_drives_player_one() {
    let (mut app, pads) = session(2, false);
    let primary = the_primary(&mut app);
    let still = x_of(&app, primary);
    for _ in 0..30 {
        app.update();
    }
    assert!((x_of(&app, primary) - still).abs() < 1.0, "control: player one stands still with no input");

    let late = connect_a_pad(&mut app);
    for device in [Device::Keyboard, Device::Pad(pads[0]), Device::Pad(pads[1]), Device::Pad(late)] {
        let moved = push_right(&mut app, device, &[primary])[0];
        assert!(moved >= MOVED, "{device:?} moved player one {moved} px (want at least {MOVED})");
    }
    for button in [GamepadButton::South, GamepadButton::Start] {
        tap(&mut app, pads[1], button);
        for _ in 0..10 {
            app.update();
        }
        if button == GamepadButton::Start {
            // Start on player one's pad is the pause; close it again.
            tap(&mut app, pads[1], GamepadButton::Start);
        }
    }
    assert!(seat_one_bodies(&mut app).is_empty(), "a second pad's Jump or Start seated a second player");
}

/// ACCEPTING JOINS. Pad A plays player one, then pad B presses Start: seat 1
/// gets a body. After the claim, pad B moves player two and not player one,
/// and pad A and the keyboard move player one and not player two.
#[test]
fn a_pad_that_claims_seat_one_drives_only_player_two() {
    let (mut app, pads) = session(2, true);
    let primary = the_primary(&mut app);
    let moved = push_right(&mut app, Device::Pad(pads[0]), &[primary])[0];
    assert!(moved >= MOVED, "premise: pad A moves player one ({moved} px)");
    assert!(seat_one_bodies(&mut app).is_empty(), "control: nobody joined before a Start press");

    tap(&mut app, pads[1], GamepadButton::Start);
    for _ in 0..6 {
        app.update();
    }
    let joined = seat_one_bodies(&mut app);
    assert_eq!(joined.len(), 1, "pad B's Start did not seat player two");
    let two = joined[0];
    let before_visual = app
        .world()
        .get::<Transform>(two)
        .expect("player two has a sprite transform")
        .translation
        .x;

    let [one_moved, two_moved] = push_right(&mut app, Device::Pad(pads[1]), &[primary, two])[..] else {
        unreachable!()
    };
    assert!(
        two_moved >= MOVED && one_moved.abs() < MOVED,
        "pad B after its claim: player one moved {one_moved} px, player two {two_moved} px"
    );
    let visual_moved = app
        .world()
        .get::<Transform>(two)
        .expect("player two has a sprite transform")
        .translation
        .x
        - before_visual;
    assert!(
        (visual_moved - two_moved as f32).abs() < 20.0,
        "player two's simulation body moved {two_moved} px but its drawn transform moved {visual_moved:.2} px",
    );
    for device in [Device::Pad(pads[0]), Device::Keyboard] {
        let [one_moved, two_moved] = push_right(&mut app, device, &[primary, two])[..] else {
            unreachable!()
        };
        assert!(
            one_moved >= MOVED && two_moved.abs() < MOVED,
            "{device:?} after the claim: player one moved {one_moved} px, player two {two_moved} px"
        );
    }
}

/// A claimed pad's ANALOG left stick, not only its D-pad, drives its body.
/// This uses the same rendered host and device input route as the shipped game.
#[test]
fn a_claimed_pads_left_stick_moves_player_two() {
    let (mut app, pads) = session(2, true);
    let primary = the_primary(&mut app);
    // Seat zero must first claim the first pad as its active device.
    let moved = push_right(&mut app, Device::Pad(pads[0]), &[primary])[0];
    assert!(moved >= MOVED, "premise: pad A moved the primary body {moved} px");
    tap(&mut app, pads[1], GamepadButton::Start);
    for _ in 0..6 {
        app.update();
    }
    let joined = seat_one_bodies(&mut app);
    assert_eq!(joined.len(), 1, "pad B did not join");
    let two = joined[0];

    let before_one = x_of(&app, primary);
    let before_two = x_of(&app, two);
    for _ in 0..24 {
        pad_left_stick_x(&mut app, pads[1], 1.0);
        app.update();
    }
    let raw = app.world().resource::<SeatRawFrames>().get(PlayerSlot(SEAT));
    let latched = app.world().resource::<SlotControls>().get(PlayerSlot(SEAT));
    pad_left_stick_x(&mut app, pads[1], 0.0);
    for _ in 0..20 {
        app.update();
    }
    let one_moved = (x_of(&app, primary) - before_one).round() as i32;
    let two_moved = (x_of(&app, two) - before_two).round() as i32;
    assert!(
        two_moved >= MOVED && one_moved.abs() < MOVED,
        "pad B left stick: player one moved {one_moved} px, player two moved {two_moved} px; \
         seat 1 raw axis=({}, {}), latched axis=({}, {})",
        raw.axis_x, raw.axis_y, latched.axis_x, latched.axis_y,
    );
}

/// The shipped host must supply a physical camera for every opened local view.
/// A view and a HUD alone cannot draw the right half of the display.
#[test]
fn adaptive_split_opens_and_retires_a_camera_in_the_shipped_host() {
    use ambition_platformer2d::platformer::camera_layers::MainCamera;
    use ambition_platformer2d::render::platformer_presentation::SplitViewCamera;
    use ambition_platformer2d::sim_view::{AdaptiveSplit, LocalViewId, PresentsView};

    let (mut app, pads) = session(2, true);
    tap(&mut app, pads[1], GamepadButton::Start);
    for _ in 0..6 {
        app.update();
    }
    assert_eq!(seat_one_bodies(&mut app).len(), 1);

    // Force the existing distance policy open without requiring a long room.
    app.world_mut().resource_mut::<AdaptiveSplit>().split_beyond = -1.0;
    for _ in 0..6 {
        app.update();
    }
    let camera_views = |app: &mut App| {
        let world = app.world_mut();
        let mut cameras = world.query_filtered::<
            (&PresentsView, Option<&SplitViewCamera>),
            With<MainCamera>,
        >();
        let mut bindings: Vec<_> = cameras
            .iter(world)
            .map(|(presents, split)| {
                (
                    world.get::<LocalViewId>(presents.0)
                        .expect("camera has a live view")
                        .0,
                    split.is_some(),
                )
            })
            .collect();
        bindings.sort();
        bindings
    };
    assert_eq!(
        camera_views(&mut app),
        vec![(0, false), (1, true)],
        "each adaptive split view needs its own host gameplay camera",
    );

    // A bound camera is necessary, not sufficient: the split rig must draw the
    // same world the primary does (layer parity), take part in screen effects,
    // and the two views must own separate halves of the display.
    {
        use ambition_platformer2d::platformer::camera_layers::{
            PARALLAX_BACKGROUND_LAYER, PORTAL_WINDOW_RENDER_LAYER,
        };
        use ambition_platformer2d::render::screen_effects::ScreenEffectCamera;
        use ambition_platformer2d::sim_view::camera_snapshot::CameraViewport;
        use bevy::camera::visibility::RenderLayers;

        let world = app.world_mut();
        let mut cameras = world.query_filtered::<
            (&PresentsView, Option<&RenderLayers>, Has<ScreenEffectCamera>, Has<SplitViewCamera>),
            With<MainCamera>,
        >();
        let rigs: Vec<_> = cameras
            .iter(world)
            .map(|(presents, layers, effects, split)| (presents.0, layers.cloned(), effects, split))
            .collect();
        for (view, layers, effects, split) in &rigs {
            let layers = layers.as_ref().expect("a gameplay camera states its layers");
            for (layer, what) in [
                (0, "the world"),
                (PARALLAX_BACKGROUND_LAYER, "the parallax backdrop"),
                (PORTAL_WINDOW_RENDER_LAYER, "portal windows"),
            ] {
                assert!(
                    layers.intersects(&RenderLayers::layer(layer)),
                    "the {} camera of view {view:?} does not draw {what} (layer {layer}): {layers:?}",
                    if *split { "split" } else { "primary" },
                );
            }
            assert!(*effects, "the camera of view {view:?} takes no part in screen effects");
        }
        let mut rects: Vec<_> = rigs
            .iter()
            .map(|(view, ..)| {
                let rect = world.get::<CameraViewport>(*view).expect("each view has a rectangle");
                (rect.origin_px.x, rect.origin_px.y, rect.px.x, rect.px.y)
            })
            .collect();
        rects.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(rects.len(), 2);
        let [a, b] = [rects[0], rects[1]];
        assert!(
            a.2 > 0.0 && b.2 > 0.0 && (a.0 + a.2 <= b.0 || a.1 + a.3 <= b.1),
            "the two views must own separate parts of the display: {rects:?}",
        );
    }

    // Merge is a camera lifecycle event, not just a change in HUD facts.
    {
        let mut split = app.world_mut().resource_mut::<AdaptiveSplit>();
        split.split_beyond = 1001.0;
        split.merge_within = 1000.0;
        split.merge_after = 0.0;
    }
    for _ in 0..6 {
        app.update();
    }
    assert_eq!(camera_views(&mut app), vec![(0, false)]);
}

/// A CLAIMED PAD RECONNECTS (review finding #6, 2026-10-19). Pad B claims
/// seat 1, disconnects, and comes back as a new entity: it moves player two
/// again, with no restart and no second Start. Measured before: the claim held
/// B's old entity, so seat 1 heard no pad and could not be claimed again.
/// Control: while B is away, a different pad in B's slot does not move player
/// two.
#[test]
fn a_claimed_pad_that_reconnects_drives_player_two_again() {
    let (mut app, pads) = session(2, true);
    let primary = the_primary(&mut app);
    push_right(&mut app, Device::Pad(pads[0]), &[primary]);
    tap(&mut app, pads[1], GamepadButton::Start);
    for _ in 0..6 {
        app.update();
    }
    let joined = seat_one_bodies(&mut app);
    assert_eq!(joined.len(), 1, "premise: pad B's Start seated player two");
    let two = joined[0];

    app.world_mut().entity_mut(pads[1]).despawn();
    for _ in 0..6 {
        app.update();
    }
    // Its Start would be player one's pause (an unclaimed pad is the shared
    // seat's); the unit arm `a_different_controller_does_not_take_a_claimed_seat`
    // presses it.
    let stranger = connect_named_pad(&mut app, "another pad");
    let [_, two_moved] = push_right(&mut app, Device::Pad(stranger), &[primary, two])[..] else {
        unreachable!()
    };
    assert!(two_moved.abs() < MOVED, "control: a different pad moved player two {two_moved} px");
    app.world_mut().entity_mut(stranger).despawn();
    for _ in 0..6 {
        app.update();
    }

    let pad_b_again = connect_named_pad(&mut app, "pad 1");
    assert_ne!(pad_b_again, pads[1], "premise: pad B came back as a new entity");
    for _ in 0..6 {
        app.update();
    }
    assert_eq!(seat_one_bodies(&mut app), vec![two], "player two's body did not stay");
    let [one_moved, two_moved] = push_right(&mut app, Device::Pad(pad_b_again), &[primary, two])[..] else {
        unreachable!()
    };
    assert!(
        two_moved >= MOVED && one_moved.abs() < MOVED,
        "pad B after it came back: player one moved {one_moved} px, player two {two_moved} px"
    );
}

/// THE SETTING CHANGES WHILE THE SESSION RUNS. Off: pad B's Jump seats
/// nobody (and makes pad B player one's pad, as any press does in single-player
/// mode). On, in the same session (the session root is the same entity, so
/// nothing restarted): player one plays on pad A, so pad B is free, and pad B's
/// Start seats player two.
#[test]
fn turning_joins_on_mid_session_lets_a_pad_join_without_a_restart() {
    let (mut app, pads) = session(2, false);
    let root = {
        let world = app.world_mut();
        world.query_filtered::<Entity, With<SessionRoot>>().single(world).expect("one session")
    };
    let primary = the_primary(&mut app);
    tap(&mut app, pads[1], GamepadButton::South);
    for _ in 0..10 {
        app.update();
    }
    assert!(seat_one_bodies(&mut app).is_empty(), "control: with the setting off nobody joins");

    accept_joins(&mut app, true);
    for _ in 0..10 {
        app.update();
    }
    // The pad player one used last is player one's: play on pad A.
    push_right(&mut app, Device::Pad(pads[0]), &[primary]);
    tap(&mut app, pads[1], GamepadButton::Start);
    for _ in 0..6 {
        app.update();
    }
    assert_eq!(seat_one_bodies(&mut app).len(), 1, "after the setting went on, pad B's Start did not seat player two");
    let world = app.world_mut();
    assert_eq!(
        world.query_filtered::<Entity, With<SessionRoot>>().single(world).ok(),
        Some(root),
        "the session restarted"
    );
    assert_eq!(the_primary(&mut app), primary, "player one's body was rebuilt");
}
