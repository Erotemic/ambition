//! Capture's authored-effect chain, exercised end to end.
//!
//! The system under test lives in `ambition_combat`: capture is an engine
//! mechanic (`CapturedBy`, its behaviour systems, its authored vocabulary). The
//! tests stay here because the chain includes `ambition_render`'s FX and the
//! actor features, which `ambition_combat` must not depend on.
//!
//! This module is test-only. It is the end-to-end proof that an authored
//! `smash.capture_*` reaches an acquisition, a pummel, a throw and an
//! interruption.

#![cfg(test)]

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_capture::CaptureCarryParams;
use ambition_platformer2d::combat::capture::{
    CaptureAttemptRequested, CaptureCarryRequested, CapturePummelRequested, CaptureThrowRequested,
};
use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::characters::actor::control::ActorControlFrame;
    use ambition_platformer2d::combat::capture::systems::{
        acquire_captures, apply_capture_pummels, apply_capture_throws, finalize_new_capture_pose,
        release_interrupted_captures,
    };
    use ambition_platformer2d::combat::capture::CapturedBy;
    use ambition_platformer2d::combat::moveset::{
        advance_move_playback, dispatch_move_events, resolve_attack_gestures,
        trigger_moveset_moves, ActorMoveset, MoveEventMessage, MovePlayback,
    };

    /// The whole chain, in the order the plan's acceptance section states.
    /// Every system is the production one; only the app is a fixture.
    fn chain_app() -> App {
        let mut app = App::new();
        app.add_message::<MoveEventMessage>();
        app.add_message::<ambition_platformer2d::vfx::vfx::VfxMessage>();
        app.add_message::<ambition_platformer2d::vfx::FxRequest>();
        app.add_message::<ambition_platformer2d::characters::brain::ActorActionMessage>();
        app.add_message::<ambition_platformer2d::combat::capture::CaptureAttemptRequested>();
        app.add_message::<ambition_platformer2d::combat::capture::CapturePummelRequested>();
        app.add_message::<ambition_platformer2d::combat::capture::CaptureThrowRequested>();
        // The carry message: a system that writes four messages fails parameter
        // validation in a world that registers three, and George cannot grab.
        // (An `Option<MessageWriter>` would hide that.)
        app.add_message::<ambition_platformer2d::combat::capture::CaptureCarryRequested>();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        app.init_resource::<ambition_platformer2d::time::WorldTime>();
        app.insert_resource(
            ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog::empty(),
        );
        app.init_resource::<
            ambition_platformer2d::combat::authored_volumes::AuthoredAttackVolumeResolver,
        >();
        let mut time = app
            .world_mut()
            .resource_mut::<ambition_platformer2d::time::WorldTime>();
        time.scaled_dt = 1.0 / 60.0;
        time.raw_dt = 1.0 / 60.0;
        app.add_systems(
            Update,
            (
                resolve_attack_gestures,
                trigger_moveset_moves,
                advance_move_playback,
                dispatch_move_events,
                // The fan-out, so this fixture hears what a match hears:
                // `dispatch_move_events` only writes `FxRequest`s.
                ambition_platformer2d::render::fx::process_fx_requests,
                ambition_platformer2d::combat::capture::systems::translate_authored_capture_effects,
                acquire_captures,
                apply_capture_pummels,
                apply_capture_throws,
                ambition_platformer2d::combat::capture::systems::apply_capture_carries,
                finalize_new_capture_pose,
                release_interrupted_captures,
            )
                .chain(),
        );
        app
    }

    /// Spawn the two fighters. The captor carries George's real table, so the
    /// grab, pummel and throw that play are the ones a match would play.
    fn stage(app: &mut App) -> (Entity, Entity) {
        use ambition_platformer2d::characters::actor::ActorFaction;
        let body = |app: &mut App, x: f32, team: &str| {
            app.world_mut()
                .spawn((
                    ambition_platformer2d::engine_core::BodyKinematics {
                        pos: ambition_platformer2d::engine_core::Vec2::new(x, 0.0),
                        facing: 1.0,
                        size: ambition_platformer2d::engine_core::Vec2::new(16.0, 24.0),
                        ..Default::default()
                    },
                    ambition_platformer2d::combat::components::CenteredAabb::new(
                        ambition_platformer2d::engine_core::Vec2::new(x, 0.0),
                        ambition_platformer2d::engine_core::Vec2::new(8.0, 12.0),
                    ),
                    ActorFaction::Player,
                    ambition_platformer2d::combat::targeting::MatchTeam::new(team),
                    ambition_platformer2d::engine_core::BodyGroundState {
                        head_contact: false,
                        on_ground: true,
                        contact_initialized: true,
                    },
                    // A complete `CaptureParticipant` at both ends: acquisition
                    // needs the full body role, and without combat state the
                    // interruption rule reads the captor as despawned.
                    ambition_platformer2d::characters::actor::BodyCombat::default(),
                    ambition_platformer2d::characters::actor::BodyHealth::new(
                        ambition_platformer2d::characters::actor::Health {
                            current: 100,
                            max: 100,
                            invulnerable: Default::default(),
                        },
                    ),
                    ambition_platformer2d::engine_core::BodyFlightState::default(),
                    // The movement columns every integrated body carries from
                    // spawn. The throw hands the captive's air dodge to the
                    // shared hit reaction, which needs them.
                    ambition_platformer2d::engine_core::BodyAbilities::default(),
                    ambition_platformer2d::engine_core::BodyDashState::default(),
                    ambition_platformer2d::engine_core::BodyJumpState::default(),
                    ambition_platformer2d::engine_core::BodyDodgeState::default(),
                    ambition_platformer2d::engine_core::MotionModel::axis_swept(
                        ambition_platformer2d::engine_core::AxisSweptParams::default(),
                    ),
                    ambition_platformer2d::actors::features::ActorSurfaceState {
                        surface_normal: ambition_platformer2d::engine_core::Vec2::new(0.0, -1.0),
                        gravity_scale: 1.0,
                    },
                    ambition_platformer2d::platformer::sim_id::SimId::placement(team),
                ))
                .id()
        };
        let captor = body(app, 0.0, "captor");
        let victim = body(app, 20.0, "victim");
        app.world_mut().entity_mut(captor).insert((
            ActorMoveset(crate::smash_pack::shipped_moveset(crate::SMASH_GEORGE_BOOUL)),
            ambition_platformer2d::characters::control::ActorControl(ActorControlFrame::neutral()),
        ));
        app.world_mut().entity_mut(victim).insert(
            // Shielding changes nothing: the third leg of the triangle, in the
            // real chain.
            ambition_platformer2d::engine_core::BodyShieldState::default(),
        );
        (captor, victim)
    }

    /// Press `f` on the captor for one tick, then run the chain.
    fn press(app: &mut App, captor: Entity, f: impl FnOnce(&mut ActorControlFrame)) {
        let mut control = app
            .world_mut()
            .get_mut::<ambition_platformer2d::characters::control::ActorControl>(captor)
            .expect("the captor carries a control frame");
        control.0 = ActorControlFrame::neutral();
        f(&mut control.0);
        app.update();
    }

    /// Run ticks until `done`, or panic. Moves take tenths of a second at
    /// 1/60, so a bounded loop plays a move out.
    ///
    /// It presses nothing: an edge re-sent every tick would re-trigger the move.
    fn run_until(app: &mut App, captor: Entity, label: &str, mut done: impl FnMut(&App) -> bool) {
        for _ in 0..120 {
            if done(app) {
                return;
            }
            // Production consumes control edges once; so must the fixture, or
            // the grab restarts every tick before its active window.
            if let Some(mut control) =
                app.world_mut()
                    .get_mut::<ambition_platformer2d::characters::control::ActorControl>(captor)
            {
                control.0.clear_edges();
            }
            app.update();
        }
        panic!("{label} never happened within 2 seconds of sim time");
    }

    /// End-to-end capture sequence through the production systems: grab a
    /// shielding opponent, preserve the hold after the grab move ends, pummel
    /// twice without releasing, then release exactly once on forward throw.
    /// The fixture builds complete fighter bodies because capture-interruption
    /// rules depend on their combat state.
    #[test]
    fn george_grabs_pummels_twice_and_throws() {
        let mut app = chain_app();
        let (captor, victim) = stage(&mut app);

        // 1. The grab. Its Active window opens at 0.16s, so the capture cannot
        //    land on the press tick.
        press(&mut app, captor, |f| f.grab_pressed = true);
        assert_eq!(
            app.world()
                .get::<MovePlayback>(captor)
                .map(|pb| pb.spec.id.clone())
                .as_deref(),
            Some("george_grab"),
            "the Grab press did not start the authored grab"
        );
        assert!(
            app.world().get::<CapturedBy>(victim).is_none(),
            "the grab caught somebody on its startup frame — it has no tell"
        );

        run_until(
            &mut app,
            captor,
            "the grab's active window catches the victim",
            |app| app.world().get::<CapturedBy>(victim).is_some(),
        );

        // 2. The grab move ends and the relationship does not.
        run_until(&mut app, captor, "the grab move finishes", |app| {
            app.world().get::<MovePlayback>(captor).is_none()
        });
        assert!(
            app.world().get::<CapturedBy>(victim).is_some(),
            "the hold died with the move that made it — a capture that cannot \
             outlive its own grab is not a relationship"
        );

        // 3. Two pummels. The hold survives both, and the meter moves.
        for expected in 1..=2u8 {
            press(&mut app, captor, |f| f.melee_pressed = true);
            run_until(&mut app, captor, "the pummel finishes", |app| {
                app.world().get::<MovePlayback>(captor).is_none()
            });
            // The relation must still be there; the count is the ruleset's
            // (a separate component).
            app.world()
                .get::<CapturedBy>(victim)
                .expect("the pummel released the hold it belongs to");
            let state = app
                .world()
                .get::<ambition_platformer2d::characters::smash_hold_state::SmashHoldState>(victim)
                .expect("a held body carries this ruleset's hold state");
            assert_eq!(state.pummels_landed, expected);
        }
        let hurt = app
            .world()
            .get::<ambition_platformer2d::characters::actor::BodyHealth>(victim)
            .unwrap()
            .damage_taken();
        assert_eq!(hurt, 8, "two 4-damage pummels did not reach the meter");

        // 4. The throw. Forward + Attack, and the authored release ends it.
        press(&mut app, captor, |f| {
            f.melee_pressed = true;
            f.attack_axis = ambition_platformer2d::engine_core::LocalAxes::X;
        });
        assert_eq!(
            app.world()
                .get::<MovePlayback>(captor)
                .map(|pb| pb.spec.id.clone())
                .as_deref(),
            Some("george_fthrow"),
            "forward+attack in a capture did not start the throw"
        );
        assert!(
            app.world().get::<CapturedBy>(victim).is_some(),
            "the throw released on its PRESS — the authored release frame owns \
             that instant, and a wind-up that lets go early is not punishable"
        );

        run_until(&mut app, captor, "the throw's release frame", |app| {
            app.world().get::<CapturedBy>(victim).is_none()
        });
        let vel = app
            .world()
            .get::<ambition_platformer2d::engine_core::BodyKinematics>(victim)
            .unwrap()
            .vel;
        assert!(vel.length() > 1.0, "the throw launched nobody: {vel:?}");
        assert!(vel.x > 0.0, "the forward throw went backwards: {vel:?}");
        assert_eq!(
            app.world()
                .get::<ambition_platformer2d::characters::actor::BodyHealth>(victim)
                .unwrap()
                .damage_taken(),
            hurt + 11,
            "the throw's own damage did not land"
        );
    }

    /// The adapter arm itself. Other carry guards start from a
    /// `CaptureCarryRequested`, so a typo'd key would leave the feature dead
    /// with a green suite.
    #[test]
    fn an_authored_carry_key_becomes_a_carry_request() {
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_message::<CaptureAttemptRequested>();
        app.add_message::<CapturePummelRequested>();
        app.add_message::<CaptureThrowRequested>();
        app.add_message::<CaptureCarryRequested>();
        app.add_systems(Update, ambition_platformer2d::combat::capture::systems::translate_authored_capture_effects);
        let captor = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(ActorActionMessage {
            actor: captor,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::Special(
                    ambition_platformer2d::entity_catalog::smash_capture::CAPTURE_CARRY.to_string(),
                ),
                params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                    &CaptureCarryParams {
                        hold_offset: (6.0, -18.0),
                    },
                )
                .expect("carry params serialize"),
            },
            move_instance: None,
        });
        app.update();
        let messages = app.world().resource::<Messages<CaptureCarryRequested>>();
        let mut cursor = messages.get_cursor();
        let out: Vec<&CaptureCarryRequested> = cursor.read(messages).collect();
        assert_eq!(out.len(), 1, "the carry key produced no request");
        assert_eq!(out[0].captor, captor);
        assert_eq!(out[0].hold_offset, ae::Vec2::new(6.0, -18.0));
        // And it is not a throw: the wrong arm would end the hold.
        let throws = app.world().resource::<Messages<CaptureThrowRequested>>();
        assert_eq!(throws.get_cursor().read(throws).count(), 0);
    }
}
