//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition_engine_core as ae;

/// Minimal app harness: spawns a primary player + drives one
/// `app.update()` so the affordance compute chain runs end-to-end
/// without pulling in the whole sandbox plugin graph.
fn build_test_app() -> (App, Entity) {
    use crate::actor::{BodyEnvironmentContact, BodyGroundState, BodyLedgeState, BodyModeState};
    use crate::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
    use crate::control::PlayerInputFrame;
    use ambition_input::ControlFrame;

    let mut app = App::new();
    // `detect_active_input_method` reads `Res<ButtonInput<KeyCode>>`
    // and `Res<Touches>`; Bevy normally creates them via
    // `InputPlugin`. Initialise them directly so the test app
    // doesn't depend on the full input plugin graph. `ControlFrame`
    // is no longer read by the compute chain (it reads the actor's
    // `PlayerInputFrame`), but keep it so the harness still mirrors
    // the production resource set.
    app.init_resource::<ControlFrame>()
        .init_resource::<bevy::input::ButtonInput<KeyCode>>()
        .init_resource::<bevy::input::touch::Touches>()
        .add_plugins(AffordancesPlugin);
    // The affordance compute reads exactly these four cluster
    // components: ground (on_ground), ledge (grab), body_mode
    // (body_mode), env_contact (water). Plus kinematics for the
    // intent system's facing read and `PlayerInputFrame` for the
    // actor-local aim. Start with grounded baseline + neutral input.
    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            PlayerInputFrame::default(),
            BodyKinematics::default(),
            BodyGroundState {
                on_ground: true,
                ..Default::default()
            },
            BodyLedgeState::default(),
            BodyModeState::default(),
            BodyEnvironmentContact::default(),
        ))
        .id();
    (app, entity)
}

fn read_affordances(app: &App) -> PlayerAffordances {
    app.world().resource::<PlayerAffordances>().clone()
}

/// Stamp the controlled actor's local input axes (the intent compute
/// reads `PlayerInputFrame`, not the global `Res<ControlFrame>`).
fn set_axis(app: &mut App, player: Entity, x: f32, y: f32) {
    let mut input = app
        .world_mut()
        .get_mut::<crate::control::PlayerInputFrame>(player)
        .unwrap();
    input.frame.axis_x = x;
    input.frame.axis_y = y;
}

#[test]
fn default_grounded_neutral_player_reads_baseline_labels() {
    let (mut app, _) = build_test_app();
    app.update();
    let aff = read_affordances(&app);
    assert_eq!(aff.jump, JumpVariant::Jump);
    assert_eq!(aff.attack, AttackVariant::Jab);
    assert_eq!(aff.shield, ShieldVariant::Shield);
    assert_eq!(aff.dash, DashVariant::Dash);
    assert_eq!(aff.interact, InteractVariant::None);
    // Neutral aim → `NeutralSpecial` (today: fireball under the
    // hood). The resolver's neutral arm is the cold-start label
    // a player sees before pushing the stick.
    assert_eq!(aff.special, SpecialVariant::NeutralSpecial);
}

#[test]
fn special_dispatches_on_aim_direction() {
    let (mut app, player_entity) = build_test_app();
    // Push axis_y down → DownSpecial.
    set_axis(&mut app, player_entity, 0.0, 1.0);
    app.update();
    assert_eq!(read_affordances(&app).special, SpecialVariant::DownSpecial);

    // Push axis_y up → UpSpecial.
    set_axis(&mut app, player_entity, 0.0, -1.0);
    app.update();
    assert_eq!(read_affordances(&app).special, SpecialVariant::UpSpecial);

    // Side-stick (forward relative to right-facing) → SideSpecial.
    {
        let mut entity = app.world_mut().entity_mut(player_entity);
        let mut kin = entity.get_mut::<crate::actor::BodyKinematics>().unwrap();
        kin.facing = 1.0;
    }
    set_axis(&mut app, player_entity, 1.0, 0.0);
    app.update();
    assert_eq!(read_affordances(&app).special, SpecialVariant::SideSpecial);
}

#[test]
fn airborne_player_with_down_aim_reads_as_dair() {
    let (mut app, player_entity) = build_test_app();
    // Push axis_y down (sim convention: +Y is down).
    set_axis(&mut app, player_entity, 0.0, 1.0);
    // Lift the player off the ground.
    {
        let mut entity = app.world_mut().entity_mut(player_entity);
        let mut ground = entity.get_mut::<crate::actor::BodyGroundState>().unwrap();
        ground.on_ground = false;
    }
    app.update();
    let aff = read_affordances(&app);
    assert_eq!(aff.attack, AttackVariant::DAir);
    // Dash also flips when aerial.
    assert_eq!(aff.dash, DashVariant::Dodge);
}

#[test]
fn ledge_grab_flips_jump_and_shield() {
    let (mut app, player_entity) = build_test_app();
    {
        let mut entity = app.world_mut().entity_mut(player_entity);
        let mut ledge = entity.get_mut::<crate::actor::BodyLedgeState>().unwrap();
        ledge.grab = Some(ae::LedgeGrabState::hanging(ae::LedgeContact {
            wall_normal_x: 1.0,
            anchor: ae::Vec2::ZERO,
            climb_target: ae::Vec2::ZERO,
        }));
    }
    app.update();
    let aff = read_affordances(&app);
    assert_eq!(aff.jump, JumpVariant::Climb);
    assert_eq!(aff.shield, ShieldVariant::Roll);
}

#[test]
fn b_air_fires_when_aim_opposes_facing_aerial() {
    let (mut app, player_entity) = build_test_app();
    {
        let mut ent = app.world_mut().entity_mut(player_entity);
        ent.get_mut::<crate::actor::BodyGroundState>()
            .unwrap()
            .on_ground = false;
        ent.get_mut::<crate::actor::BodyKinematics>()
            .unwrap()
            .facing = 1.0;
    }
    // Push stick left (negative X) — opposing facing-right.
    set_axis(&mut app, player_entity, -1.0, 0.0);
    app.update();
    let aff = read_affordances(&app);
    assert_eq!(aff.attack, AttackVariant::BAir);
}
