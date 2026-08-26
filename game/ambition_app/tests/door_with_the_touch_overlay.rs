#![cfg(all(feature = "input", feature = "visible", feature = "mobile_touch"))]

//! THE DOOR, WITH THE TOUCH OVERLAY INSTALLED — because the shipped app has one.
//!
//! the sim harness, in the shell host, and under a rollback host. Every one of
//! those compositions is missing something the binary he runs has:
//! `add_presentation_plugins` installs [`TouchControlsPlugin`]
//! unconditionally whenever `mobile_touch` is compiled — and `desktop_dev`,
//! the default persona, compiles it. There is no runtime boolean gating it on a
//! desktop.
//!
//! and touch is a VIRTUAL DEVICE bound into the same participant map, not
//! a separate `ControlFrame` writer. So it shares the one seam a keyboard
//! interact press travels, and an overlay that publishes a neutral frame, holds
//! a stale `Interaction`, or claims a context would take the press without
//! anything logging a refusal.
//!
//! this test exists to be a NEGATIVE if it passes. Ruling the overlay out
//! is worth as much as catching it: it is the last structural difference between
//! the compositions that work and the binary that does not, and after this the
//! remaining suspects are all machine-local (persisted save, settings, devices).

use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::input::{InputParticipant, Platformer2dInputActionMonolith};
use ambition_app::app::shell_host;
use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;
use leafwing_input_manager::prelude::InputMap;

fn active_room(app: &mut App) -> Option<String> {
    let mut q = app
        .world_mut()
        .query::<&ambition_platformer2d::world::rooms::RoomSet>();
    q.iter(app.world())
        .next()
        .map(|set| set.active_spec().id.clone())
}

#[test]
fn a_door_still_opens_with_the_touch_overlay_installed() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::platformer::schedule::GameMode>();
    app.insert_resource(shell_host::AmbitionShellHosted);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);
    // THE DIFFERENCE UNDER TEST. Its HUD spawn orders after the font
    // load, so the Font asset type has to exist and `load_ui_fonts` has to run,
    // exactly as the app arranges them.
    app.init_asset::<bevy::text::Font>();
    app.add_systems(
        Startup,
        ambition_platformer2d::render::ui_fonts::load_ui_fonts,
    );
    app.add_plugins(ambition_platformer2d::touch_input::TouchControlsPlugin);
    shell_host::compose_ambition_shell_host(&mut app);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));

    for _ in 0..8 {
        app.update();
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    for _ in 0..40 {
        app.update();
    }

    // The interact key THIS build binds, read from the live map rather than
    // hardcoded — the key differs per preset.
    let interact_key = {
        let world = app.world_mut();
        let mut q = world
            .query_filtered::<&InputMap<Platformer2dInputActionMonolith>, With<InputParticipant>>();
        let map = q
            .iter(world)
            .next()
            .expect("the host spawns a primary input participant at boot");
        map.get_buttonlike(&Platformer2dInputActionMonolith::Interact)
            .and_then(|bindings| bindings.first().cloned())
            .expect("Interact has a binding, or no key opens a door at all")
    };

    // Stand in an authored Door zone of the live room.
    let before = {
        let door = {
            let world = app.world_mut();
            let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
            let zone = rooms
                .iter(world)
                .next()
                .expect("a live session room set")
                .active_loading_zones()
                .iter()
                .find(|zone| {
                    zone.activation
                        == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
                })
                .cloned()
                .expect("the gameplay start room authors a Door zone");
            let mut player = world.query_filtered::<&mut ambition_platformer2d::platformer::body::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
            if let Ok(mut kin) = player.single_mut(world) {
                kin.pos = ambition_platformer2d::engine_core::AabbExt::center(zone.aabb);
                kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
            }
            zone
        };
        let _ = door;
        active_room(&mut app).expect("a live room")
    };

    for _ in 0..40 {
        interact_key.press(app.world_mut());
        app.update();
        if active_room(&mut app).as_deref() != Some(before.as_str()) {
            return;
        }
    }
    panic!(
        "held the interact key inside a Door zone of '{before}' for 40 frames \
         with the touch overlay installed and the room never changed. The same \
         press opens the same door in `door_entry`'s shipped-host case, which \
         builds everything here EXCEPT `TouchControlsPlugin` — so the overlay is \
         taking the press."
    );
}
