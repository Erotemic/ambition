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
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, SessionRoot};
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

fn inventory_menus(app: &mut App) -> std::collections::BTreeMap<u8, (Entity, bool)> {
    use ambition_platformer2d::inventory_ui::{InventoryMenu, InventoryUiState};
    let world = app.world_mut();
    world.query::<(Entity, &InventoryMenu, &InventoryUiState)>().iter(world)
        .map(|(entity, menu, state)| (menu.owner.slot(), (entity, state.visible))).collect()
}

fn focused_menu_action(app: &mut App, menu: Entity) -> Option<ambition_app::menu::model::MenuPageAction> {
    use ambition_app::menu::model::MenuPageAction;
    use ambition_platformer2d::menu::{AmbitionMenuControl, MenuSurface, MenuVisualState};
    let world = app.world_mut();
    world.query::<(&MenuSurface, &AmbitionMenuControl<MenuPageAction>, &MenuVisualState)>()
        .iter(world).find(|(owner, _, visual)| owner.0 == menu && visual.focused)
        .and_then(|(_, control, _)| control.action)
}

fn menu_output_regions(app: &mut App, grid: bool) -> std::collections::BTreeMap<Entity, Rect> {
    use ambition_platformer2d::menu::render::{MenuPresentationOf, bevy_ui::BevyUiMenuRoot};
    let world = app.world_mut();
    if grid {
        let pixels = |value| match value { Val::Px(value) => value, _ => panic!("the host places menu roots in display pixels") };
        world.query_filtered::<(&MenuPresentationOf, &Node), With<BevyUiMenuRoot>>().iter(world)
            .map(|(owner, node)| {
                let min = Vec2::new(pixels(node.left), pixels(node.top));
                (owner.0, Rect::from_corners(min, min + Vec2::new(pixels(node.width), pixels(node.height))))
            }).collect()
    } else {
        world.query::<(&MenuPresentationOf, &Camera)>().iter(world)
            .filter_map(|(owner, camera)| camera.viewport.as_ref().map(|viewport| {
                let min = viewport.physical_position.as_vec2();
                (owner.0, Rect::from_corners(min, min + viewport.physical_size.as_vec2()))
            })).collect()
    }
}

#[test]
fn two_pads_join_open_and_close_their_own_menus_on_both_backends() {
    use ambition_platformer2d::input::{SeatInputContexts, INVENTORY_CONTEXT};
    use ambition_platformer2d::menu::backend::InventoryUiBackend;
    use ambition_platformer2d::platformer::schedule::GameMode;
    use ambition_app::menu::model::{MenuPage, MenuPageAction};
    use ambition_platformer2d::menu::ActiveMenuPages;
    use ambition_platformer2d::settings_menu::system::{SystemMenuEntryId, SystemOptionId};
    type Pages = ActiveMenuPages<MenuPage, MenuPageAction>;

    for backend in [InventoryUiBackend::Grid, InventoryUiBackend::LunexKaleidoscope]
        .into_iter().filter(|backend| backend.is_available())
    {
        let mut host = rendered_app();
        host.insert_resource(ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(
            ambition_platformer2d::engine_core::Vec2::new(1280.0, 720.0),
        ));
        let (mut app, pads) = session_in(host, 2, true);
        *app.world_mut().resource_mut::<InventoryUiBackend>() = backend;
        let initial = inventory_menus(&mut app);
        assert_eq!(initial.len(), 1);
        assert!(!initial[&0].1);
        tap(&mut app, pads[1], GamepadButton::Start);
        for _ in 0..6 { app.update(); }
        let joined = inventory_menus(&mut app);
        assert_eq!(joined.len(), 2, "{backend:?}: joining admits the second menu");
        assert!(joined.values().all(|(_, visible)| !visible), "the join press does not open a menu");
        assert_eq!(joined[&0].0, initial[&0].0);

        tap(&mut app, pads[1], GamepadButton::Start);
        assert!(inventory_menus(&mut app)[&1].1);
        assert!(!inventory_menus(&mut app)[&0].1);
        tap(&mut app, pads[0], GamepadButton::Start);
        assert!(inventory_menus(&mut app).values().all(|(_, visible)| *visible));
        for slot in [0, 1] {
            assert_eq!(app.world().resource::<SeatInputContexts>().for_seat(slot).owner(), Some(INVENTORY_CONTEXT));
        }
        let one = joined[&0].0;
        let two = joined[&1].0;
        let regions = menu_output_regions(&mut app, backend == InventoryUiBackend::Grid);
        let [one_rect, two_rect] = [regions[&one], regions[&two]];
        assert!(one_rect.width() > 0.0 && one_rect.height() > 0.0);
        assert!(two_rect.width() > 0.0 && two_rect.height() > 0.0);
        assert!(one_rect.max.x <= two_rect.min.x || two_rect.max.x <= one_rect.min.x
            || one_rect.max.y <= two_rect.min.y || two_rect.max.y <= one_rect.min.y,
            "{backend:?}: the two menu outputs must not overlap");
        for _ in 0..MenuPage::ALL.len() {
            if app.world().get::<Pages>(one).unwrap().active == Some(MenuPage::Items) { break; }
            tap(&mut app, pads[0], GamepadButton::RightTrigger);
        }
        assert_eq!(app.world().get::<Pages>(one).unwrap().active, Some(MenuPage::Items));
        assert_eq!(app.world().get::<Pages>(two).unwrap().active, Some(MenuPage::System));

        let rebind = MenuPageAction::OpenSystemEntry(SystemMenuEntryId::Rebind);
        for _ in 0..32 {
            if focused_menu_action(&mut app, two) == Some(rebind) { break; }
            tap(&mut app, pads[1], GamepadButton::DPadDown);
        }
        assert_eq!(focused_menu_action(&mut app, two), Some(rebind), "{backend:?}: the second pad reaches Rebind");
        assert_eq!(app.world().get::<Pages>(one).unwrap().active, Some(MenuPage::Items));
        tap(&mut app, pads[1], GamepadButton::South);
        assert_eq!(focused_menu_action(&mut app, two), Some(MenuPageAction::SystemOption(SystemOptionId::Rebind(0))));
        let before = {
            let bindings = app.world().resource::<ambition_platformer2d::input::SeatBindings>();
            [bindings.for_seat(0).clone(), bindings.for_seat(1).clone()]
        };
        tap(&mut app, pads[1], GamepadButton::South);
        tap(&mut app, pads[1], GamepadButton::RightThumb);
        app.update();
        let bindings = app.world().resource::<ambition_platformer2d::input::SeatBindings>();
        assert_eq!(bindings.for_seat(0), &before[0], "the first participant keeps its bindings");
        assert_ne!(bindings.for_seat(1), &before[1], "the second pad changes its own binding profile");
        assert_eq!(app.world().get::<Pages>(two).unwrap().active, Some(MenuPage::System),
            "capturing the Inventory button must keep the Rebind page open");
        tap(&mut app, pads[1], GamepadButton::Start);
        assert!(inventory_menus(&mut app)[&1].1, "Start first exits the Rebind page");
        tap(&mut app, pads[1], GamepadButton::Start);
        assert!(!inventory_menus(&mut app)[&1].1);
        assert!(inventory_menus(&mut app)[&0].1);
        assert_eq!(*app.world().resource::<State<GameMode>>().get(), GameMode::Paused);
        tap(&mut app, pads[0], GamepadButton::Start);
        assert!(inventory_menus(&mut app).values().all(|(_, visible)| !visible));
        assert_eq!(*app.world().resource::<State<GameMode>>().get(), GameMode::Playing);

        app.world_mut().entity_mut(pads[1]).despawn();
        for _ in 0..6 { app.update(); }
        assert_eq!(inventory_menus(&mut app), joined, "a lost device does not retire admitted menus");
        app.world_mut().write_message(ShellCommand::QuitToHome);
        settle(&mut app);
        assert!(inventory_menus(&mut app).is_empty(), "session retirement removes both menus");
    }
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
    let (app, pads) = gpu_session("ambition-p2-dialogue.png");
    let (mut app, _) = player_two_dialogue_at_choices(app, pads, false);
    capture_frame(&mut app);
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

fn gpu_app(name: &str) -> App {
    gpu_app_sized(name, UVec2::new(1280, 720))
}

fn gpu_app_sized(name: &str, size: UVec2) -> App {
    use ambition_platformer2d::render::capture::{
        CaptureProgress, CaptureSettings, CaptureCameraSet, CaptureTarget, install_capture_cameras, setup_capture_target,
    };
    let mut app = crate::shell_host_rendered::rendered_app_with(ambition_app::app::VisibleRenderMode::OffscreenGpu);
    let output = std::env::temp_dir().join(name);
    app.insert_resource(CaptureSettings { output, size, include_ui: true });
    app.insert_resource(ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(size.as_vec2()));
    app.init_resource::<CaptureProgress>();
    app.add_systems(Startup, setup_capture_target);
    install_capture_cameras(&mut app);
    app.add_systems(Update, (
            |mut commands: Commands, target: Option<Res<CaptureTarget>>,
             cameras: Query<(Entity, &bevy::camera::RenderTarget), (With<Camera3d>, With<ambition_platformer2d::menu::render::MenuPresentationOf>)>| {
                let Some(target) = target else { return; };
                let destination = bevy::camera::RenderTarget::Image(target.image.clone().into());
                for (entity, current) in &cameras {
                    if !matches!(current, bevy::camera::RenderTarget::Image(image) if image.handle == target.image) {
                        commands.entity(entity).insert((destination.clone(), Msaa::Off));
                    }
                }
            }
        ).after(CaptureCameraSet));
    app
}

fn gpu_session(name: &str) -> (App, Vec<Entity>) {
    session_in(gpu_app(name), 2, true)
}

fn capture_frame(app: &mut App) {
    use ambition_platformer2d::render::capture::{
        CaptureCameraSet, CaptureProgress, CaptureSettings, CaptureTarget, request_capture,
    };
    for _ in 0..30 { app.update(); }
    app.add_systems(Update, (|mut commands: Commands, target: Res<CaptureTarget>, mut progress: ResMut<CaptureProgress>| {
        request_capture(&mut commands, &target, &mut progress);
    }).after(CaptureCameraSet));
    let started = std::time::Instant::now();
    while !app.world().resource::<CaptureProgress>().completed {
        assert!(started.elapsed() < crate::common::ACTIVATION_CEILING, "the frame capture did not complete");
        app.update();
    }
    assert!(!app.world().resource::<CaptureProgress>().failed);
    assert!(app.world().resource::<CaptureSettings>().output.is_file());
}

#[test]
#[ignore = "needs a wgpu adapter; captures the two participants' menus on both backends"]
fn two_pad_menus_draw_in_their_own_regions() {
    use ambition_app::menu::model::{MenuPage, MenuPageAction};
    use ambition_platformer2d::menu::{ActiveMenuPages, backend::InventoryUiBackend};
    use ambition_platformer2d::settings_menu::system::SystemMenuEntryId;
    type Pages = ActiveMenuPages<MenuPage, MenuPageAction>;

    for (backend, output, size) in [
        (InventoryUiBackend::Grid, "ambition-two-pad-menus-grid.png", UVec2::new(1280, 720)),
        (InventoryUiBackend::LunexKaleidoscope, "ambition-two-pad-menus-cube.png", UVec2::new(1280, 720)),
        (InventoryUiBackend::LunexKaleidoscope, "ambition-two-pad-menus-cube-16by10.png", UVec2::new(1280, 800)),
    ].into_iter().filter(|(backend, _, _)| backend.is_available()) {
        let host = gpu_app_sized(output, size);
        let (mut app, pads) = session_in(host, 2, true);
        *app.world_mut().resource_mut::<InventoryUiBackend>() = backend;
        tap(&mut app, pads[1], GamepadButton::Start);
        for _ in 0..6 { app.update(); }
        tap(&mut app, pads[0], GamepadButton::Start);
        tap(&mut app, pads[1], GamepadButton::Start);
        let menus = inventory_menus(&mut app);
        let one = menus[&0].0;
        let two = menus[&1].0;
        for _ in 0..MenuPage::ALL.len() {
            if app.world().get::<Pages>(one).unwrap().active == Some(MenuPage::Items) { break; }
            tap(&mut app, pads[0], GamepadButton::RightTrigger);
        }
        let rebind = MenuPageAction::OpenSystemEntry(SystemMenuEntryId::Rebind);
        for _ in 0..32 {
            if focused_menu_action(&mut app, two) == Some(rebind) { break; }
            tap(&mut app, pads[1], GamepadButton::DPadDown);
        }
        assert_eq!(focused_menu_action(&mut app, two), Some(rebind));
        tap(&mut app, pads[1], GamepadButton::South);
        assert_eq!(app.world().get::<Pages>(one).unwrap().active, Some(MenuPage::Items));
        assert_eq!(app.world().get::<Pages>(two).unwrap().active, Some(MenuPage::System));
        if backend == InventoryUiBackend::LunexKaleidoscope {
            let world = app.world_mut();
            let mut query = world.query_filtered::<&Camera, With<ambition_menu_kaleidoscope::KaleidoscopePauseCamera>>();
            let mut regions: Vec<_> = query.iter(world).map(|camera| {
                let viewport = camera.viewport.as_ref().unwrap();
                (viewport.physical_position, viewport.physical_size)
            }).collect();
            regions.sort_by_key(|(position, _)| position.x);
            let half = UVec2::new(size.x / 2, size.y);
            assert_eq!(regions, vec![(UVec2::ZERO, half), (UVec2::new(half.x, 0), half)]);
        }
        capture_frame(&mut app);
    }
}

#[test]
#[ignore = "needs a wgpu adapter; writes /tmp/ambition-fixed-split.png for visual review"]
fn fixed_split_draws_both_same_room_views() {
    let (app, pads) = gpu_session("ambition-fixed-split.png");
    let mut app = fixed_split_from_settings(app, pads);
    capture_frame(&mut app);
    use ambition_platformer2d::platformer::camera_layers::MainCamera;
    use ambition_platformer2d::sim_view::{PresentsView, camera_snapshot::CameraViewport};
    let world = app.world_mut();
    let mut cameras = world.query_filtered::<(&Camera, &PresentsView), With<MainCamera>>();
    let mut rectangles = Vec::new();
    for (camera, presents) in cameras.iter(world) {
        assert!(camera.is_active);
        let physical = camera.viewport.as_ref().expect("each captured split camera needs a viewport");
        let logical = world.get::<CameraViewport>(presents.0).unwrap();
        assert_eq!(physical.physical_position, logical.origin_px.round().as_uvec2());
        assert_eq!(physical.physical_size, logical.px.round().as_uvec2());
        rectangles.push((physical.physical_position, physical.physical_size));
    }
    rectangles.sort_by_key(|(position, _)| position.x);
    assert_eq!(rectangles.len(), 2);
    assert!(rectangles[0].0.x + rectangles[0].1.x <= rectangles[1].0.x,
        "captured cameras must draw into separate physical rectangles");
    let output = &world.resource::<ambition_platformer2d::render::capture::CaptureSettings>().output;
    let image = Image::from_buffer(
        &std::fs::read(output).unwrap(),
        bevy::image::ImageType::Extension("png"),
        bevy::image::CompressedImageFormats::empty(),
        true,
        default(),
        default(),
    ).unwrap();
    for (position, size) in rectangles {
        let mut lit = 0;
        // Sample the room inside each pane, away from the HUD and touch controls.
        for x in 1..9 {
            for y in 1..9 {
                let color = image.get_color_at(
                    position.x + size.x * x / 10,
                    position.y + size.y / 4 + size.y * y / 20,
                ).unwrap().to_srgba();
                if color.red.max(color.green).max(color.blue) > 0.2 { lit += 1; }
            }
        }
        assert!(lit > 32, "a later camera cleared the room in pane {position:?}");
    }
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

fn camera_views(app: &mut App) -> Vec<(u8, bool)> {
    use ambition_platformer2d::platformer::camera_layers::MainCamera;
    use ambition_platformer2d::render::platformer_presentation::SplitViewCamera;
    use ambition_platformer2d::sim_view::{LocalViewId, PresentsView};
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
}

#[test]
fn fixed_split_uses_the_setting_without_changing_the_session() {
    let (app, pads) = session(2, true);
    drop(fixed_split_from_settings(app, pads));
}

fn fixed_split_from_settings(mut app: App, pads: Vec<Entity>) -> App {
    use ambition_platformer2d::settings_menu::settings::{apply_settings_option, SettingsOptionId};
    use ambition_platformer2d::persistence::settings::UserSettings;
    let root = {
        let world = app.world_mut();
        world.query_filtered::<Entity, With<SessionRoot>>().single(world).unwrap()
    };
    let one = the_primary(&mut app);
    apply_settings_option(SettingsOptionId::LocalViewLayout, 1, &mut app.world_mut().resource_mut::<UserSettings>());
    for _ in 0..6 { app.update(); }
    assert_eq!(camera_views(&mut app), vec![(0, false)], "one participant keeps one view");
    tap(&mut app, pads[1], GamepadButton::Start);
    for _ in 0..90 { app.update(); }
    let two = seat_one_bodies(&mut app)[0];
    assert_eq!(camera_views(&mut app), vec![(0, false), (1, true)], "nearby participants keep separate cameras");
    apply_settings_option(SettingsOptionId::LocalViewLayout, -1, &mut app.world_mut().resource_mut::<UserSettings>());
    for _ in 0..90 { app.update(); }
    assert_eq!(camera_views(&mut app), vec![(0, false)], "adaptive still merges nearby participants");
    apply_settings_option(SettingsOptionId::LocalViewLayout, 1, &mut app.world_mut().resource_mut::<UserSettings>());
    for _ in 0..6 { app.update(); }
    assert_eq!(camera_views(&mut app), vec![(0, false), (1, true)]);
    assert_eq!(the_primary(&mut app), one);
    assert_eq!(seat_one_bodies(&mut app), vec![two]);
    let world = app.world_mut();
    assert_eq!(world.query_filtered::<Entity, With<SessionRoot>>().single(world).unwrap(), root);
    app
}

/// The shipped host must supply a physical camera for every opened local view.
/// A view and a HUD alone cannot draw the right half of the display.
#[test]
fn adaptive_split_opens_and_retires_a_camera_in_the_shipped_host() {
    use ambition_platformer2d::platformer::camera_layers::MainCamera;
    use ambition_platformer2d::render::platformer_presentation::SplitViewCamera;
    use ambition_platformer2d::sim_view::{AdaptiveSplit, PresentsView};

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


fn rendered_pad_combat(enabled: bool) {
    use ambition_platformer2d::actor::{BodyHealth, Health};
    use ambition_platformer2d::combat::{events::{HitEvent, HitSource, HitTarget}, strike::Hitbox};
    use ambition_platformer2d::persistence::settings::UserSettings;
    use ambition_platformer2d::projectiles::ProjectileSpawnRequest;
    use ambition_platformer2d::settings_menu::settings::{apply_settings_option, SettingsOptionId};
    use crate::neighbor_prefetch_prepares_rooms::cross;
    use crate::the_note_travels_from_alice_to_bob::put_body_at;

    let mut app = gpu_app(if enabled { "ambition-pad-combat-on.png" } else { "ambition-pad-combat-off.png" });
    settle(&mut app);
    app.world_mut().resource_mut::<UserSettings>().gameplay.coop_friendly_fire = enabled;
    let (mut app, pads) = session_in(app, 2, true);
    apply_settings_option(SettingsOptionId::LocalViewLayout, 1,
        &mut app.world_mut().resource_mut::<UserSettings>());
    tap(&mut app, pads[1], GamepadButton::Start);
    for _ in 0..30 { app.update(); }
    let bodies = [the_primary(&mut app), seat_one_bodies(&mut app)[0]];
    for (seat, body) in bodies.into_iter().enumerate() {
        cross(&mut app, body, PlayerSlot(seat as u8), "switch_lab");
    }
    assert_eq!(camera_views(&mut app), vec![(0, false), (1, true)]);

    for projectile in [false, true] {
        for source_seat in 0..2 {
            for _ in 0..90 { app.update(); }
            let source = bodies[source_seat];
            let victim = bodies[1 - source_seat];
            for (seat, body) in bodies.into_iter().enumerate() {
                app.world_mut().entity_mut(body).insert(BodyHealth::new(Health::new(100)));
                let half_height = app.world().get::<BodyKinematics>(body).unwrap().size.y * 0.5;
                let separation = if projectile { 120.0 } else { 40.0 };
                put_body_at(&mut app, body, Vec2::new(220.0 + seat as f32 * separation, 352.0 - half_height));
                app.world_mut().get_mut::<BodyKinematics>(body).unwrap().facing = if seat == 0 { 1.0 } else { -1.0 };
            }
            for _ in 0..6 { app.update(); }
            let mut shots = app.world().resource::<Messages<ProjectileSpawnRequest>>().get_cursor_current();
            let mut hits = app.world().resource::<Messages<HitEvent>>().get_cursor_current();
            let mut emitted = false;
            let mut hit_victim = false;
            let button = if projectile { GamepadButton::North } else { GamepadButton::West };
            pad_button(&mut app, pads[source_seat], button, 1.0);
            for frame in 0..60 {
                if frame == 2 { pad_button(&mut app, pads[source_seat], button, 0.0); }
                app.update();
                if projectile {
                    emitted |= shots.read(app.world().resource::<Messages<ProjectileSpawnRequest>>())
                        .any(|shot| shot.owner == source);
                } else {
                    let world = app.world_mut();
                    emitted |= world.query::<(&Hitbox, &HitSource)>().iter(world)
                        .any(|(hitbox, cause)| hitbox.owner == source && *cause == HitSource::Melee);
                }
                hit_victim |= hits.read(app.world().resource::<Messages<HitEvent>>())
                    .any(|hit| hit.attacker == Some(source) && hit.target == HitTarget::Body(victim));
            }
            let damage = app.world().get::<BodyHealth>(victim).unwrap().damage_taken();
            eprintln!("pad combat: friendly_fire={enabled}, projectile={projectile}, source_seat={source_seat}, emitted={emitted}, hit_victim={hit_victim}, damage={damage}");
            assert!(emitted, "pad {source_seat} did not emit its attack; projectile={projectile}");
            assert_eq!(hit_victim, enabled, "wrong contact policy for pad {source_seat}; projectile={projectile}");
            assert_eq!(damage > 0, enabled, "wrong damage policy for pad {source_seat}; projectile={projectile}");
            assert_eq!(app.world().get::<BodyHealth>(source).unwrap().damage_taken(), 0,
                "the attack damaged its own source");
        }
    }
    capture_frame(&mut app);
}

#[test]
#[ignore = "needs a wgpu adapter; captures two-pad combat with friendly fire on"]
fn two_pads_damage_each_other_in_the_rendered_game() {
    rendered_pad_combat(true);
}

#[test]
#[ignore = "needs a wgpu adapter; captures two-pad combat with friendly fire off"]
fn two_pads_spare_each_other_in_the_rendered_game() {
    rendered_pad_combat(false);
}

#[test]
fn item_requests_restore_and_equip_the_joined_participant() {
    use ambition_platformer2d::actor::{BodyHealth, Health};
    use ambition_platformer2d::abilities::mana::{MANA, ManaRegen};
    use ambition_platformer2d::characters::equipment::WornEquipment;
    use ambition_platformer2d::engine_core::resources::ActorResources;
    use ambition_platformer2d::items::{Item, ItemEquipRequested, ItemUseRequested, OwnedItems};
    use ambition_platformer2d::platformer::host_intents::write_host_intent;

    for slot in [PlayerSlot(0), PlayerSlot(1)] {
        let (mut app, pads) = session(2, true);
        tap(&mut app, pads[1], GamepadButton::Start);
        for _ in 0..6 { app.update(); }
        let bodies = [the_primary(&mut app), seat_one_bodies(&mut app)[0]];
        app.insert_resource(ManaRegen(0.0));
        for body in bodies {
            app.world_mut().entity_mut(body).insert(BodyHealth::new(Health {
                current: 40, max: 100, invulnerable: Default::default(),
            }));
            let mut resources = app.world_mut().get_mut::<ActorResources>(body).expect("a joined home body has its resource bank");
            let mana = resources.level_of_mut(&MANA).expect("the home body declares mana");
            mana.drain(mana.current);
        }
        let catalog = ambition_platformer2d::items::item_catalog(app.world()).clone();
        for item in [Item::HealthCell, Item::ManaCell, Item::EyeLaser] {
            app.world_mut().resource_mut::<OwnedItems>().grant(&catalog, item, 1);
        }
        write_host_intent(app.world_mut(), ItemUseRequested { participant: slot, item: Item::HealthCell });
        write_host_intent(app.world_mut(), ItemUseRequested { participant: slot, item: Item::ManaCell });
        write_host_intent(app.world_mut(), ItemEquipRequested { participant: slot, item: Item::EyeLaser });
        for _ in 0..6 { app.update(); }
        for (seat, body) in bodies.into_iter().enumerate() {
            let requested = seat == slot.0 as usize;
            assert_eq!(app.world().get::<BodyHealth>(body).unwrap().current() > 40, requested,
                "health cell reached the wrong body for {slot:?}");
            assert_eq!(app.world().get::<ActorResources>(body).unwrap().level_of(&MANA).unwrap().current > 0.0, requested,
                "mana cell reached the wrong body for {slot:?}");
            let laser = app.world().get::<WornEquipment>(body)
                .is_some_and(|worn| worn.rows.iter().any(|row| row.id == "eye_laser"));
            assert_eq!(laser, requested, "ability equip reached the wrong body for {slot:?}");
        }
    }
}

#[test]
#[ignore = "needs a wgpu adapter; writes /tmp/ambition-single-player-cube.png"]
fn single_player_cube_draws_with_the_authored_canvas() {
    use ambition_platformer2d::menu::backend::InventoryUiBackend;
    if !InventoryUiBackend::LunexKaleidoscope.is_available() { return; }
    let (mut app, pads) = session_in(gpu_app("ambition-single-player-cube.png"), 1, false);
    *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::LunexKaleidoscope;
    tap(&mut app, pads[0], GamepadButton::Start);
    assert_eq!(inventory_menus(&mut app).values().filter(|(_, open)| *open).count(), 1);
    for _ in 0..120 { app.update(); }
    {
        let world = app.world_mut();
        let mut cameras = world.query_filtered::<(&Camera, &Projection), With<ambition_menu_kaleidoscope::KaleidoscopePauseCamera>>();
        let (camera, projection) = cameras.single(world).expect("one single-player menu camera");
        let viewport = camera.viewport.as_ref().unwrap();
        assert_eq!(viewport.physical_position, UVec2::ZERO);
        assert_eq!(viewport.physical_size, UVec2::new(1280, 720));
        let Projection::Perspective(projection) = projection else { panic!("cube camera needs perspective"); };
        assert_eq!(projection.fov, ambition_platformer2d::menu::MenuCubeGeometry::CAMERA_FOV_RADIANS);
    }
    capture_frame(&mut app);
}

fn record_crossing(app: &mut App, slot: u8, target: &str) {
    use ambition_platformer2d::session::lifecycle_commit::{LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent};
    let world = app.world_mut();
    let body = world.query::<(Entity, &DrivingParticipant)>().iter(world)
        .find(|(_, driver)| driver.0 == PlayerSlot(slot)).map(|(body, _)| body).unwrap();
    let subject = ambition_platformer2d::platformer::lifecycle::LiveBodyId::of_entity(world, body).unwrap();
    let rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>().single(world).unwrap();
    let arrival = rooms.rooms[rooms.room_index_by_id(target).unwrap()].world.spawn;
    let frame = world.get_resource::<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>().map_or(0, |boundary| boundary.current);
    assert!(world.resource_mut::<PendingLifecycleCommit>().record(frame, LifecycleIntent::Transition(RoomTransitionIntent {
        disposition: Default::default(),
        source_zone: None,
                initiator: ambition_platformer2d::session::lifecycle_commit::RoomBodyTransfer {
                    subject: subject,
                    arrival: ambition_platformer2d::session::lifecycle_commit::RoomArrival::At(arrival),
                    participant: Some(PlayerSlot(slot)),
                },
                companions: Vec::new(), target_room: target.into(),
        edge_exit: false, zone_sfx: None,
    })).admitted());
}

fn delayed_crossing(app: &mut App, slot: u8, target: &str) -> (u64, ambition_platformer2d::load::LoadBarrierRef) {
    use ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState;
    use ambition_platformer2d::load::{LoadCommand, LoadCoordinator, LoadWorkSpec};
    record_crossing(app, slot, target);
    for _ in 0..120 {
        app.update();
        if let Some(active) = app.world().resource::<RoomTransitionLoadState>().active.as_ref() {
            let sequence = active.sequence;
            let barrier = active.barrier.clone();
            let _ = app.world_mut().resource_mut::<LoadCoordinator>().apply(LoadCommand::UpsertWork {
                load_id: barrier.load_id.clone(),
                spec: LoadWorkSpec::required("fixture.delayed-room-art", "Delayed room art", barrier.barrier_id.clone()),
            });
            return (sequence, barrier);
        }
    }
    panic!("the accepted crossing opened no transaction");
}

fn split_session(mut host: App) -> (App, Vec<Entity>) {
    use ambition_platformer2d::persistence::settings::{UserSettings, video::LocalViewLayout};
    host.insert_resource(ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(Vec2::new(1280.0, 720.0)));
    let (mut app, pads) = session_in(host, 2, true);
    app.world_mut().resource_mut::<UserSettings>().video.local_view_layout = LocalViewLayout::AlwaysSplit;
    tap(&mut app, pads[1], GamepadButton::Start);
    for _ in 0..6 { app.update(); }
    (app, pads)
}

fn transition_in_one_view(host: App, slot: u8) -> (App, Vec<Entity>, ambition_platformer2d::load::LoadBarrierRef) {
    let (app, pads) = split_session(host);
    hold_crossing(app, pads, slot, "switch_lab")
}

fn hold_crossing(mut app: App, pads: Vec<Entity>, slot: u8, target: &str) -> (App, Vec<Entity>, ambition_platformer2d::load::LoadBarrierRef) {
    use ambition_platformer2d::input::{LOADING_CONTEXT, SeatInputContexts};
    use ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState;
    let bodies = [the_primary(&mut app), seat_one_bodies(&mut app)[0]];
    let (sequence, barrier) = delayed_crossing(&mut app, slot, target);
    for _ in 0..90 { app.update(); }
    let active = app.world().resource::<RoomTransitionLoadState>().active.as_ref().expect("the crossing must remain pending");
    assert_eq!(active.sequence, sequence);
    assert_eq!(active.phase, ambition_platformer2d::runtime::room_transition::RoomTransitionLoadPhase::AwaitingReadiness);
    assert!(active.failure.is_none(), "the loading fixture must not be a failure foreground");
    assert!(app.world().resource::<ambition_platformer2d::load_presentation::LoadPresentationModel>().visible);
    assert_eq!(app.world().resource::<SeatInputContexts>().for_seat(slot).owner(), Some(LOADING_CONTEXT));
    assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(1 - slot));
    let moved = push_right(&mut app, Device::Pad(pads[usize::from(1 - slot)]), &[bodies[usize::from(1 - slot)]]);
    assert!(moved[0] >= MOVED, "the other participant could not move during the crossing");
    tap(&mut app, pads[usize::from(slot)], GamepadButton::Start);
    assert!(inventory_menus(&mut app).values().all(|(_, open)| !open), "loading input opened an inventory underneath its foreground");
    (app, pads, barrier)
}

fn assert_transition_regions(app: &mut App, slot: u8) {
    use ambition_platformer2d::game_shell::{FrontendEntityOwner, FrontendOwnedEntity, FrontendPresentationKind};
    use ambition_platformer2d::render::rendering::room_transition_cover::RoomTransitionCover;
    use ambition_platformer2d::sim_view::{LocalViewId, camera_snapshot::CameraViewport};
    let world = app.world_mut();
    let viewport = world.query::<(&LocalViewId, &CameraViewport)>().iter(world)
        .find(|(id, _)| id.0 == slot).map(|(_, viewport)| *viewport)
        .expect("each participant in the fixed split must retain a view");
    let expected = Rect::from_corners(viewport.origin_px, viewport.origin_px + viewport.px);
    let active = world.resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>().active.as_ref().unwrap();
    let sequence = active.sequence;
    let load = active.barrier.load_id.clone();
    let mut roots = world.query::<(&Node, Option<&RoomTransitionCover>, Option<&FrontendOwnedEntity>)>();
    let mut found = 0;
    for (node, cover, owner) in roots.iter(world) {
        if !cover.is_some_and(|cover| cover.sequence == sequence && cover.is_opaque())
            && !owner.is_some_and(|owner| owner.owner == FrontendEntityOwner::Load(load.clone()) && owner.kind == FrontendPresentationKind::LoadingRoot) {
            continue;
        }
        let (Val::Px(left), Val::Px(top), Val::Px(width), Val::Px(height)) = (node.left, node.top, node.width, node.height) else {
            panic!("the transition root has no resolved view bounds: {node:?}");
        };
        let actual = Rect::from_corners(Vec2::new(left, top), Vec2::new(left + width, top + height));
        assert!((actual.min - expected.min).abs().max_element() < 1.0 && (actual.max - expected.max).abs().max_element() < 1.0,
            "transition {actual:?} extends beyond its view {expected:?}");
        found += 1;
    }
    assert_eq!(found, 2, "the cover and its loading foreground must both have a view");
}

#[test]
fn a_room_transition_keeps_the_other_participants_input_and_view() {
    for slot in [0, 1] {
        let (mut app, _, _) = transition_in_one_view(rendered_app(), slot);
        assert_transition_regions(&mut app, slot);
    }
}

#[test]
#[ignore = "needs a wgpu adapter; captures each participant's room loading foreground"]
fn room_loading_draws_only_in_the_crossing_participants_view() {
    for slot in [0, 1] {
        let (mut app, _, _) = transition_in_one_view(gpu_app(&format!("ambition-p{slot}-room-loading.png")), slot);
        capture_frame(&mut app);
        assert_transition_regions(&mut app, slot);
        assert!(app.world().resource::<bevy::render::render_asset::RenderAssetBytesPerFrame>().max_bytes.is_some(),
            "a visible gameplay viewport must keep the upload budget");
    }
}

#[derive(Resource)]
struct HoldCrossingFrame(bool);

#[test]
fn a_second_crossing_waits_until_the_first_destinations_frame_is_presentable() {
    for first in [0, 1] { queued_crossing(first); }
}

fn queued_crossing(first: u8) {
    use ambition_platformer2d::load::{LoadCommand, LoadCoordinator, LoadWorkState};
    use ambition_platformer2d::render::rendering::presentation_settling::{PresentationSettling, PresentationSettlingSet};
    use ambition_platformer2d::runtime::room_transition::{RoomTransitionLoadPhase, RoomTransitionLoadState};
    use ambition_platformer2d::world_flow::RoomTransitionCoverSet;
    let (mut app, _, barrier) = transition_in_one_view(rendered_app(), first);
    app.insert_resource(HoldCrossingFrame(true));
    app.add_systems(Update, (|hold: Res<HoldCrossingFrame>, state: Res<RoomTransitionLoadState>, mut settling: ResMut<PresentationSettling>| {
        if hold.0 {
            if let Some(active) = state.active.as_ref().filter(|active| active.phase == RoomTransitionLoadPhase::Committed) {
                settling.hold_in(active.arrived_in, "fixture target frame");
            }
        }
    }).after(PresentationSettlingSet).before(RoomTransitionCoverSet));
    let sequence = app.world().resource::<RoomTransitionLoadState>().active.as_ref().unwrap().sequence;
    let _ = app.world_mut().resource_mut::<LoadCoordinator>().apply(LoadCommand::SetWorkState {
        load_id: barrier.load_id, work_id: "fixture.delayed-room-art".into(), state: LoadWorkState::Complete,
    });
    let mut committed = false;
    for _ in 0..600 {
        app.update();
        let state = app.world().resource::<RoomTransitionLoadState>();
        let active = state.active.as_ref().expect("the held target frame must keep its transaction");
        assert!(active.failure.is_none(), "the first crossing failed: {:?}", active.failure);
        if active.phase == RoomTransitionLoadPhase::Committed { committed = true; break; }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(committed, "the first crossing never committed");
    record_crossing(&mut app, 1 - first, "portal_lab");
    for _ in 0..4 {
        app.update();
        assert_eq!(app.world().resource::<RoomTransitionLoadState>().active.as_ref().unwrap().sequence, sequence,
            "the second crossing replaced a committed destination before its first presentable frame");
        assert!(app.world().resource::<ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit>().peek().is_some());
    }
    app.world_mut().resource_mut::<HoldCrossingFrame>().0 = false;
    let mut next = None;
    for _ in 0..600 {
        app.update();
        if let Some(active) = app.world().resource::<RoomTransitionLoadState>().active.as_ref().filter(|active| active.sequence != sequence) {
            next = Some(active.intent.participant());
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(next, Some(Some(PlayerSlot(1 - first))), "the queued crossing must start after the first destination is presentable");
}

#[test]
#[ignore = "needs a wgpu adapter; captures room loading with participants in separate live rooms"]
fn separate_rooms_keep_the_other_participants_view_and_controls_while_loading() {
    for slot in [0, 1] {
        let (mut app, pads) = split_session(gpu_app(&format!("ambition-separated-p{slot}-room-loading.png")));
        let (bodies, rooms) = separate_participant(&mut app, 1 - slot);
        let (mut app, _, _) = hold_crossing(app, pads, slot, "portal_lab");
        capture_frame(&mut app);
        assert_transition_regions(&mut app, slot);
        for (body, room) in bodies.into_iter().zip(rooms) {
            assert_eq!(app.world().get::<InRoomInstance>(body), Some(&room),
                "waiting for the next room changed the residency of a participant");
        }
    }
}

fn separate_participant(app: &mut App, first: u8) -> ([Entity; 2], [ambition_platformer2d::platformer::lifecycle::InRoomInstance; 2]) {
    use ambition_platformer2d::load::{LoadCommand, LoadCoordinator, LoadWorkState};
    use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
    use ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState;
    let (_, barrier) = delayed_crossing(app, first, "switch_lab");
    let _ = app.world_mut().resource_mut::<LoadCoordinator>().apply(LoadCommand::SetWorkState {
        load_id: barrier.load_id, work_id: "fixture.delayed-room-art".into(), state: LoadWorkState::Complete,
    });
    let mut finished = false;
    for _ in 0..1200 {
        app.update();
        let state = app.world().resource::<RoomTransitionLoadState>();
        assert!(state.last_failure.is_none(), "the room separation crossing failed");
        if state.active.is_none() { finished = true; break; }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(finished, "the room separation crossing did not finish");
    let bodies = [the_primary(app), seat_one_bodies(app)[0]];
    let rooms = bodies.map(|body| *app.world().get::<InRoomInstance>(body).unwrap());
    assert_ne!(rooms[0], rooms[1], "the witness must use two real live rooms");
    (bodies, rooms)
}

#[test]
#[ignore = "needs a wgpu adapter; captures F1 in two separate live rooms"]
fn debug_geometry_and_labels_stay_in_their_rooms_and_camera_frames_stay_in_their_views() {
    for separate in [false, true] { capture_debug_views(separate); }
}

fn capture_debug_views(separate: bool) {
    use ambition_platformer2d::dev_tools::{DeveloperRuntimeState, dev_tools::DeveloperTools};
    use ambition_platformer2d::render::rendering::retained_drawings::DrawingScope;
    use ambition_platformer2d::sim_view::{LocalView, PresentsView, camera_snapshot::ResolvedCameraSnapshot};
    use ambition_platformer2d::platformer::camera_layers::MainCamera;
    use bevy::camera::visibility::RenderLayers;
    let file = if separate { "ambition-two-room-debug.png" } else { "ambition-shared-room-debug.png" };
    let (mut app, _) = split_session(gpu_app(file));
    if separate { separate_participant(&mut app, 1); }
    let bodies = [the_primary(&mut app), seat_one_bodies(&mut app)[0]];
    let mut portal_colors = Vec::new();
    for (index, body) in bodies.into_iter().enumerate() {
        use ambition_platformer2d::portal::{PlacedPortal, PortalChannel, PortalChannelColor};
        let room = app.world().get::<InRoomInstance>(body).unwrap().0;
        let pos = app.world().get::<BodyKinematics>(body).unwrap().pos + Vec2::new(100.0, 0.0);
        let channel = PortalChannel::Authored(PortalChannelColor::Indexed(240 + 2 * index as u8));
        portal_colors.push((room, channel.display().0.with_alpha(0.95).to_linear()));
        app.world_mut().spawn((InRoomInstance(room), PlacedPortal::fixed(channel, pos, Vec2::NEG_X, 30.0)));
    }
    app.world_mut().resource_mut::<DeveloperRuntimeState>().debug = true;
    {
        let mut tools = app.world_mut().resource_mut::<DeveloperTools>();
        tools.gizmos_enabled = true;
        tools.show_world_blocks = true;
        tools.show_micro_grid = true;
        tools.show_player_hitbox = true;
        tools.show_feature_hitboxes = true;
        tools.show_camera_frame = true;
        tools.show_loading_zones = true;
    }
    capture_frame(&mut app);
    let world = app.world_mut();
    let cameras: Vec<_> = world.query_filtered::<(&PresentsView, &RenderLayers), With<MainCamera>>()
        .iter(world).map(|(view, layers)| (view.0, layers.clone())).collect();
    assert_eq!(cameras.len(), 2);
    let visible: std::collections::BTreeMap<_, _> = world.query_filtered::<(Entity, &ResolvedCameraSnapshot), With<LocalView>>()
        .iter(world).map(|(view, snapshot)| (view, snapshot.frame().unwrap().room)).collect();
    let mut scopes = std::collections::BTreeSet::new();
    let mut portal_rooms = vec![std::collections::BTreeSet::new(); portal_colors.len()];
    for (scope, layers, gizmo) in world.query::<(&DrawingScope, Option<&RenderLayers>, &bevy::gizmos::retained::Gizmo)>().iter(world) {
        scopes.insert(*scope);
        let layers = layers.cloned().unwrap_or_default();
        let picture = world.resource::<Assets<bevy::gizmos::GizmoAsset>>().get(&gizmo.handle).unwrap();
        assert!(!picture.buffer().list_positions.is_empty() || !picture.buffer().strip_positions.is_empty());
        if let DrawingScope::Room(room) = scope {
            for (index, (_, color)) in portal_colors.iter().enumerate() {
                if picture.buffer().list_colors.contains(color) { portal_rooms[index].insert(*room); }
            }
        }
        for (view, camera_layers) in &cameras {
            let expected = match scope { DrawingScope::Room(room) => visible[view] == *room, DrawingScope::View(owner) => view == owner };
            assert_eq!(layers.intersects(camera_layers), expected, "{scope:?} has the wrong camera mask");
        }
    }
    for ((room, _), drawn_in) in portal_colors.into_iter().zip(portal_rooms) {
        assert_eq!(drawn_in, std::collections::BTreeSet::from([room]), "a portal diagnostic belongs only to its own room");
    }
    for (view, room) in visible {
        assert!(scopes.contains(&DrawingScope::Room(room)));
        assert!(scopes.contains(&DrawingScope::View(view)));
    }
}

#[test]
#[ignore = "needs a wgpu adapter; captures the launcher's measured typography"]
fn launcher_text_uses_its_rendered_page_and_keeps_a_visible_hierarchy() {
    use ambition_platformer2d::game_shell::{BasicShellUiRoot, ShellLauncherPresentation};
    let mut app = gpu_app("ambition-launcher-menu.png");
    settle(&mut app);
    capture_frame(&mut app);
    let world = app.world_mut();
    let root = world.query_filtered::<Entity, With<BasicShellUiRoot>>().single(world).unwrap();
    let presentation = world.resource::<ShellLauncherPresentation>();
    let title = presentation.title.clone();
    let footer = presentation.footer.clone();
    let parents: std::collections::HashMap<_, _> = world.query::<(Entity, &ChildOf)>().iter(world)
        .map(|(entity, parent)| (entity, parent.parent())).collect();
    let under_root = |mut entity| {
        while let Some(parent) = parents.get(&entity) {
            if *parent == root { return true; }
            entity = *parent;
        }
        false
    };
    let rem = world.resource::<bevy::text::RemSize>().0;
    let texts: Vec<_> = world.query::<(Entity, &Text, &TextFont, &ComputedNode, &bevy::ui::ComputedUiRenderTargetInfo)>()
        .iter(world).filter(|(entity, ..)| under_root(*entity))
        .map(|(_, text, font, node, target)| (text.0.clone(), font.font_size.eval(target.logical_size(), rem), node.size()))
        .collect();
    let measured = |label: &str| {
        let matches: Vec<_> = texts.iter().filter(|(text, ..)| text == label).collect();
        assert_eq!(matches.len(), 1, "the launcher must draw its authored text once");
        let (_, size, bounds) = matches[0];
        assert!(bounds.cmpgt(Vec2::ZERO).all(), "the text must have a rendered layout box");
        *size
    };
    let title_size = measured(&title);
    let footer_size = measured(&footer);
    let control_size = texts.iter().filter(|(text, ..)| text != &title && text != &footer)
        .map(|(_, size, _)| *size).fold(0.0f32, f32::max);
    assert!(title_size > control_size && control_size > 0.0, "the title must lead the control labels: {texts:?}");
    assert!(footer_size > 0.0 && footer_size < title_size, "the footer must remain subordinate: {texts:?}");
}


#[test]
fn gate_terminals_capture_and_close_only_their_own_participants() {
    use ambition_content::gate_terminal::{TerminalUi, GATE_TERMINAL_CONTEXT};
    use ambition_platformer2d::input::participant::{InputParticipant, SeatInputContexts};
    use ambition_platformer2d::platformer::lifecycle::LiveBodyId;
    use ambition_platformer2d::world::rooms::GateNetworkState;
    let (mut app, pads) = session(2, true);
    tap(&mut app, pads[0], GamepadButton::South);
    tap(&mut app, pads[1], GamepadButton::Start);
    for _ in 0..6 { app.update(); }
    let first_body = the_primary(&mut app);
    let second = seat_one_bodies(&mut app)[0];
    let first = LiveBodyId::of_entity(app.world(), first_body).unwrap();
    let second = LiveBodyId::of_entity(app.world(), second).unwrap();
    let ring = "hub_public_gate_zone";
    app.world_mut().resource_mut::<GateNetworkState>().terminals.insert(second.clone(), ring.into());
    for _ in 0..8 { app.update(); }
    let contexts = app.world().resource::<SeatInputContexts>();
    assert_eq!(contexts.for_seat(1).owner(), Some(GATE_TERMINAL_CONTEXT));
    assert_ne!(contexts.for_seat(0).owner(), Some(GATE_TERMINAL_CONTEXT));
    app.world_mut().resource_mut::<GateNetworkState>().terminals.insert(first.clone(), ring.into());
    for _ in 0..8 { app.update(); }
    let cursors = |app: &mut App| {
        let world = app.world_mut();
        world.query::<(&InputParticipant, &TerminalUi)>().iter(world)
            .map(|(participant, ui)| (participant.id.slot(), ui.list_cursor)).collect::<std::collections::BTreeMap<_, _>>()
    };
    let before = cursors(&mut app);
    tap(&mut app, pads[1], GamepadButton::DPadDown);
    let after = cursors(&mut app);
    assert_eq!(after[&0], before[&0]);
    assert_eq!(after[&1], before[&1] + 1);
    tap(&mut app, pads[1], GamepadButton::East);
    for _ in 0..8 { app.update(); }
    let network = app.world().resource::<GateNetworkState>();
    assert_eq!(network.terminals.get(&first).map(String::as_str), Some(ring));
    assert!(!network.terminals.contains_key(&second));
    let contexts = app.world().resource::<SeatInputContexts>();
    assert_eq!(contexts.for_seat(0).owner(), Some(GATE_TERMINAL_CONTEXT));
    assert_ne!(contexts.for_seat(1).owner(), Some(GATE_TERMINAL_CONTEXT));
    app.world_mut().resource_mut::<GateNetworkState>().terminals.insert(second.clone(), ring.into());
    for _ in 0..4 { app.update(); }
    let address = app.world().resource::<ambition_platformer2d::world::rooms::GateNetworkCatalog>().spell("TANGENT").unwrap();
    ambition_platformer2d::platformer::host_intents::write_host_intent(app.world_mut(),
        ambition_platformer2d::world::rooms::GateTerminalIntent {
            subject: second.clone(), ring: ring.into(),
            action: ambition_platformer2d::world::rooms::GateTerminalAction::Dial(address),
        });
    for _ in 0..10 { app.update(); }
    let network = app.world().resource::<GateNetworkState>();
    assert!(network.report_for(&second, ring).unwrap().outcome.is_ok());
    assert_eq!(network.terminals.get(&first).map(String::as_str), Some(ring));
    assert!(!network.terminals.contains_key(&second));
    app.world_mut().entity_mut(first_body).insert(ambition_platformer2d::combat::death_rules::OutOfPlay);
    for _ in 0..4 { app.update(); }
    assert!(!app.world().resource::<GateNetworkState>().terminals.contains_key(&first));

}

#[test]
#[ignore = "needs a wgpu adapter; writes /tmp/ambition-two-gate-terminals.png"]
fn two_gate_terminals_draw_in_their_participants_regions() {
    use ambition_content::gate_terminal::{TerminalFocus, TerminalUi};
    use ambition_platformer2d::input::participant::InputParticipant;
    use ambition_platformer2d::platformer::lifecycle::LiveBodyId;
    use ambition_platformer2d::world::rooms::GateNetworkState;
    let (mut app, pads) = session_in(gpu_app("ambition-two-gate-terminals.png"), 2, true);
    tap(&mut app, pads[0], GamepadButton::South);
    tap(&mut app, pads[1], GamepadButton::Start);
    for _ in 0..6 { app.update(); }
    let bodies = [the_primary(&mut app), seat_one_bodies(&mut app)[0]];
    for body in bodies {
        let subject = LiveBodyId::of_entity(app.world(), body).unwrap();
        app.world_mut().resource_mut::<GateNetworkState>().terminals.insert(subject, "hub_public_gate_zone".into());
    }
    for _ in 0..8 { app.update(); }
    {
        let world = app.world_mut();
        for (participant, mut ui) in world.query::<(&InputParticipant, &mut TerminalUi)>().iter_mut(world) {
            if participant.id.slot() == 1 { ui.focus = TerminalFocus::Wheel; }
        }
    }
    for _ in 0..8 { app.update(); }
    capture_frame(&mut app);
}


#[test]
fn terminal_presentation_retires_with_its_gameplay_session() {
    use ambition_content::gate_terminal::TerminalUi;
    use ambition_platformer2d::platformer::lifecycle::LiveBodyId;
    use ambition_platformer2d::render::rendering::gate_portal_visuals::GateDialPreview;
    use ambition_platformer2d::world::rooms::GateNetworkState;
    let (mut app, _) = session(0, false);
    let body = the_primary(&mut app);
    let subject = LiveBodyId::of_entity(app.world(), body).unwrap();
    app.world_mut().resource_mut::<GateNetworkState>().terminals.insert(subject, "hub_public_gate_zone".into());
    for _ in 0..8 { app.update(); }
    {
        let world = app.world_mut();
        let mut ui = world.query::<&mut TerminalUi>().single_mut(world).unwrap();
        ui.entered = vec![1, 2];
    }
    app.update();
    assert!(!app.world().resource::<GateDialPreview>().by_ring.is_empty());
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    for _ in 0..4 { app.update(); }
    let world = app.world_mut();
    assert_eq!(world.query::<&TerminalUi>().iter(world).count(), 0);
    assert!(world.resource::<GateDialPreview>().by_ring.is_empty());
}
