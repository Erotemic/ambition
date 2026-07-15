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
    use crate::actor::{BodyEnvironmentContact, BodyGroundState, BodyModeState};
    use crate::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
    use ambition_characters::brain::{Brain, PlayerSlot, SlotControls};
    use ambition_input::ControlFrame;

    let mut app = App::new();
    // `detect_active_input_method` reads `Res<ButtonInput<KeyCode>>`
    // and `Res<Touches>`; Bevy normally creates them via
    // `InputPlugin`. Initialise them directly so the test app
    // doesn't depend on the full input plugin graph. `ControlFrame`
    // is no longer read by the compute chain (it reads the controlled
    // body's slot frame from `SlotControls`), but keep it so the
    // harness still mirrors the production resource set.
    app.init_resource::<ControlFrame>()
        .init_resource::<SlotControls>()
        .init_resource::<bevy::input::ButtonInput<KeyCode>>()
        .init_resource::<bevy::input::touch::Touches>()
        .add_plugins(AffordancesPlugin);
    // The affordance compute reads exactly these four body facts:
    // ground (on_ground), motion facts (ledge), body_mode
    // (body_mode), env_contact (water). Plus kinematics for the
    // intent system's facing read and `Brain::Player` for the slot
    // input lookup. Start with grounded baseline + neutral input.
    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            Brain::Player(PlayerSlot::PRIMARY),
            BodyKinematics::default(),
            BodyGroundState {
                on_ground: true,
                ..Default::default()
            },
            ae::BodyMotionFacts::default(),
            BodyModeState::default(),
            BodyEnvironmentContact::default(),
            crate::physics::ResolvedMotionFrame::default(),
        ))
        .id();
    (app, entity)
}

fn read_affordances(app: &App) -> PlayerAffordances {
    app.world().resource::<PlayerAffordances>().clone()
}

/// Stamp the primary slot's input axes (the intent compute reads the
/// controlled body's slot frame from `SlotControls` via its `Brain::Player`,
/// not the global `Res<ControlFrame>` and not a per-body input mirror).
fn set_axis(app: &mut App, _player: Entity, x: f32, y: f32) {
    use ambition_characters::brain::{PlayerSlot, SlotControls};
    let mut slots = app.world_mut().resource_mut::<SlotControls>();
    let mut frame = slots.get(PlayerSlot::PRIMARY);
    frame.axis_x = x;
    frame.axis_y = y;
    slots.set(PlayerSlot::PRIMARY, frame);
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
        // The affordance compute reads the published projection; no movement
        // step runs in this harness, so pin the fact directly.
        let mut entity = app.world_mut().entity_mut(player_entity);
        let mut facts = entity.get_mut::<ae::BodyMotionFacts>().unwrap();
        facts.ledge = Some(ae::LedgeFacts {
            climbing: false,
            getup_kind: ae::LedgeGetupKind::Climb,
        });
    }
    app.update();
    let aff = read_affordances(&app);
    assert_eq!(aff.jump, JumpVariant::Climb);
    assert_eq!(aff.shield, ShieldVariant::Roll);
}

/// The affordance table describes the body you are DRIVING. Possess a second
/// actor (via `ControlledSubject`) that is hanging on a ledge while the home
/// avatar stands on flat ground: the hints must read the possessed body's ledge
/// verbs (Climb / Roll), never the grounded home avatar's (Jump / Shield). This
/// is the relativity fix — the compute follows `ControlledSubject`, not
/// `PrimaryPlayer`.
#[test]
fn affordances_follow_the_possessed_body_not_the_home_avatar() {
    use crate::actor::{BodyEnvironmentContact, BodyGroundState, BodyKinematics, BodyModeState};
    use ambition_characters::brain::{Brain, PlayerSlot};
    use ambition_platformer_primitives::markers::ControlledSubject;

    let (mut app, home) = build_test_app();
    // The home avatar stays grounded, no ledge → its own verbs would be Jump/Shield.
    // Possess an actor hanging on a ledge: as in production, the player brain
    // TRANSFERS onto the target (which never carries `PlayerEntity` /
    // `PrimaryPlayer` / a `PlayerInputFrame` mirror) and `ControlledSubject`
    // names it.
    let possessed = app
        .world_mut()
        .spawn((
            Brain::Player(PlayerSlot::PRIMARY),
            BodyKinematics::default(),
            BodyGroundState {
                on_ground: false,
                ..Default::default()
            },
            ae::BodyMotionFacts {
                ledge: Some(ae::LedgeFacts {
                    climbing: false,
                    getup_kind: ae::LedgeGetupKind::Climb,
                }),
                ..Default::default()
            },
            BodyModeState::default(),
            BodyEnvironmentContact::default(),
            crate::physics::ResolvedMotionFrame::default(),
        ))
        .id();
    app.world_mut().entity_mut(home).remove::<Brain>();
    app.world_mut()
        .insert_resource(ControlledSubject(Some(possessed)));

    app.update();

    let aff = read_affordances(&app);
    assert_eq!(
        aff.jump,
        JumpVariant::Climb,
        "the hint reads the DRIVEN (possessed) body on a ledge, not the grounded home avatar"
    );
    assert_eq!(aff.shield, ShieldVariant::Roll);

    // Drop possession back to the home avatar: the hints snap back to its verbs.
    app.world_mut()
        .entity_mut(home)
        .insert(Brain::Player(PlayerSlot::PRIMARY));
    app.world_mut().insert_resource(ControlledSubject(None));
    app.update();
    let aff = read_affordances(&app);
    assert_eq!(aff.jump, JumpVariant::Jump, "back on the home avatar");
    assert_eq!(aff.shield, ShieldVariant::Shield);
}

/// The intent (aim) must ALSO follow the possessed body — live slot input
/// resolved against the DRIVEN body's facing. A possessed actor carries no
/// `PlayerInputFrame` mirror (possession transfers only the brain), so an
/// intent compute that reads the mirror component would silently freeze at the
/// home avatar's last pre-possession aim; reading the slot frame through the
/// body's own `Brain::Player` is what keeps it live.
#[test]
fn intent_reads_the_possessed_bodys_slot_input_and_facing() {
    use crate::actor::BodyKinematics;
    use ambition_characters::brain::{Brain, PlayerSlot};
    use ambition_platformer_primitives::markers::ControlledSubject;

    let (mut app, home) = build_test_app();
    // Pre-possession: stick right, home avatar faces right → Forward.
    set_axis(&mut app, home, 1.0, 0.0);
    app.update();
    assert_eq!(app.world().resource::<PlayerIntent>().aim, Aim::Forward);

    // Possess a LEFT-facing actor (brain transfer; no PlayerInputFrame mirror).
    let possessed = app
        .world_mut()
        .spawn((
            Brain::Player(PlayerSlot::PRIMARY),
            BodyKinematics {
                facing: -1.0,
                ..Default::default()
            },
            crate::physics::ResolvedMotionFrame::default(),
        ))
        .id();
    app.world_mut().entity_mut(home).remove::<Brain>();
    app.world_mut()
        .insert_resource(ControlledSubject(Some(possessed)));

    // Same stick-right input now reads BACK: facing-relative on the driven body.
    app.update();
    assert_eq!(
        app.world().resource::<PlayerIntent>().aim,
        Aim::Back,
        "aim is facing-relative on the POSSESSED body, not frozen at the home avatar's"
    );

    // And it tracks LIVE input, not a frozen pre-possession frame.
    set_axis(&mut app, home, -1.0, 0.0);
    app.update();
    assert_eq!(app.world().resource::<PlayerIntent>().aim, Aim::Forward);
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
