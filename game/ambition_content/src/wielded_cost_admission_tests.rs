//! An action and its cost are one admission: two admitted modules bound to
//! one held item see one trigger value (one mana level, one ready cooldown),
//! and both can ask to pay. The body pays for the first; the second is
//! refused WHOLE, so its lunge, its strike and its cues do not happen.
//!
//! The modules are the shipped `dive` and `blink`, each declared twice under
//! two keys: a supported composition of admitted entries, not an invalid
//! submission.

use ambition_characters::control::{ActorControl, DrivingParticipant, PlayerSlot};
use ambition_combat::held_items::HeldItem;
use ambition_extension_host::{ExtensionAppExt, ExtensionHostPlugin};
use ambition_extension_sdk::{ModuleDescriptor, ModuleKey};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

/// `module` again, under another key: a second entry bound to the same item.
fn twin(module: ModuleDescriptor, key: &'static str) -> ModuleDescriptor {
    ModuleDescriptor { key: ModuleKey::new("test", key), ..module }
}

/// An App with the game's ports and `modules`, and one driven body holding
/// `item` at x = 200, with `mana` (of 100).
fn world(modules: Vec<ModuleDescriptor>, item: &str, mana: f32) -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(ambition_platformer2d::sprite_sheet::test_support::shared_sprites());
    app.init_schedule(Sim);
    app.add_message::<ambition_projectiles::ProjectileSpawnRequest>()
        .add_message::<ambition_vfx::EffectRequest>()
        .add_message::<ambition_sfx::OwnedSfxMessage>()
        .add_message::<ambition_characters::brain::ActorActionMessage>()
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
    app.add_plugins(ExtensionHostPlugin::new(Sim));
    ambition_platformer2d_runtime::extension_composition::install_ports(&mut app);
    for module in modules {
        app.add_extension_module(module);
    }
    app.finish();
    let held = ambition_characters::brain::held_item_by_id(item).expect("a known item");
    let mut bank = ambition_platformer2d::abilities::mana::bank();
    assert!(ambition_platformer2d::abilities::mana::spend(Some(&mut bank), 100.0 - mana));
    let body = app
        .world_mut()
        .spawn((
            ae::BodyAbilities::default(),
            ae::BodyKinematics { pos: ae::Vec2::new(200.0, 300.0), size: ae::Vec2::new(24.0, 40.0), facing: 1.0, ..Default::default() },
            ae::BodyBaseSize::default(),
            ae::BodyGroundState::default(),
            ae::BodyWallState::default(),
            ae::BodyJumpState::default(),
            ae::BodyDashState::default(),
            ae::BodyFlightState::default(),
            ae::BodyBlinkState::default(),
            ae::BodyLedgeState::default(),
        ))
        .insert((
            ae::BodyDodgeState::default(),
            ae::BodyShieldState::default(),
            ae::BodyModeState::default(),
            ae::BodyEnvironmentContact::default(),
            ae::BodyActionBuffer::default(),
            ae::BodyRestartLatch::default(),
        ))
        .insert((
            ae::movement::MotionModel::default(),
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
            ActorControl::default(),
            HeldItem::new(held),
            ambition_characters::actor::ActorFaction::Player,
            DrivingParticipant(PlayerSlot(0)),
            bank,
        ))
        .id();
    app.insert_resource(ControlledSubject(None));
    (app, body)
}

/// Press Attack with the stick at `aim` for one tick: how far the body moved,
/// its mana after, and how many sounds the press played (one per action).
fn press(app: &mut App, body: Entity, aim: ae::LocalAxes) -> (f32, f32, usize) {
    let before = app.world().get::<ae::BodyKinematics>(body).unwrap().pos;
    {
        let mut control = app.world_mut().get_mut::<ActorControl>(body).unwrap();
        control.0.melee_pressed = true;
        control.0.aim = aim;
    }
    app.world_mut().run_schedule(Sim);
    let moved = app.world().get::<ae::BodyKinematics>(body).unwrap().pos.distance(before);
    let mana = ambition_platformer2d::abilities::mana::level(
        app.world().get::<ambition_platformer2d_core::resources::ActorResources>(body),
    )
    .map_or(f32::NAN, |level| level.current);
    let cues = app.world_mut().resource_mut::<Messages<ambition_sfx::OwnedSfxMessage>>().drain().count();
    (moved, mana, cues)
}

/// Two dive entries, 30 mana, a lunge costs 26: one lunge. Before, both lunged
/// (280 px, two cues) and only the first was paid for.
#[test]
fn two_entries_on_one_item_cannot_both_spend_one_mana() {
    use ambition_content_modules::dive;
    let (mut app, body) = world(vec![dive::module(), twin(dive::module(), "dive_twin")], dive::ITEM, 30.0);
    let (moved, mana, cues) = press(&mut app, body, ae::LocalAxes::ZERO);
    assert_eq!(
        (moved, mana, cues),
        (dive::LUNGE, 4.0, 1),
        "(moved, mana left, cues): one lunge, paid once"
    );

    // The control: with mana for both, both lunge.
    let (mut app, body) = world(vec![dive::module(), twin(dive::module(), "dive_twin")], dive::ITEM, 100.0);
    assert_eq!(press(&mut app, body, ae::LocalAxes::ZERO), (2.0 * dive::LUNGE, 48.0, 2), "with mana for two");
}

/// Two blink entries, one ready cooldown (none yet: the body has never
/// blinked): one blink. Before, both blinked, because each saw the cooldown
/// ready and the first one's cooldown was only a command.
#[test]
fn two_entries_on_one_item_cannot_both_use_one_cooldown() {
    use ambition_content_modules::blink;
    let (mut app, body) = world(vec![blink::module(), twin(blink::module(), "blink_twin")], "blink", 100.0);
    let (moved, _, _) = press(&mut app, body, ae::LocalAxes::X);
    assert_eq!(moved, blink::DISTANCE, "one blink: the second is refused by the cooldown the first armed");
}
