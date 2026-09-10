//! Capture's authored-effect chain, exercised end to end.
//!
//! ⛔⛔ **THE SYSTEM THIS TESTS MOVED TO `ambition_combat`; THESE TESTS DID NOT,
//! AND THAT IS DELIBERATE.** Capture is an engine mechanic — its `CapturedBy`
//! component, its twelve behaviour systems and its authored vocabulary are all
//! engine-side — so the authored-key -> typed-request translation moved there
//! too (2026-09-10). The TESTS stayed here because they compose the whole chain
//! including `ambition_render`'s FX and the actor features, which
//! `ambition_combat` does not depend on and must not: a test that forced those
//! edges would buy coverage with an inverted dependency.
//!
//! ⚠ SO THIS MODULE IS TEST-ONLY. It holds no production code; the demo installs
//! nothing here any more. It exists because deleting the file with the system
//! would have deleted the only end-to-end proof that an authored `smash.capture_*`
//! reaches an acquisition, a pummel, a throw and an interruption.

#![cfg(test)]

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_capture::CaptureCarryParams;
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

    /// The whole chain, in the order the plan's acceptance section states it.
    /// Every system is the production one; only the app around them is a
    /// fixture.
    fn chain_app() -> App {
        let mut app = App::new();
        app.add_message::<MoveEventMessage>();
        app.add_message::<ambition_platformer2d::vfx::vfx::VfxMessage>();
        app.add_message::<ambition_platformer2d::vfx::FxRequest>();
        app.add_message::<ambition_platformer2d::characters::brain::ActorActionMessage>();
        app.add_message::<ambition_platformer2d::combat::capture::CaptureAttemptRequested>();
        app.add_message::<ambition_platformer2d::combat::capture::CapturePummelRequested>();
        app.add_message::<ambition_platformer2d::combat::capture::CaptureThrowRequested>();
        // ⛔⛔ AND THE CARRY, WHICH THIS FIXTURE CAUGHT THE ABSENCE OF. Adding a
        // fourth `MessageWriter` to `ambition_platformer2d::combat::capture::systems::translate_authored_capture_effects` made the
        // WHOLE system fail parameter validation here, so George stopped being
        // able to grab at all — a system that writes four messages does not run
        // in a world that registers three. ⇒ Exactly why this repo has ruled
        // twice against `Option<MessageWriter>`: the optional version would have
        // left this test green and the adapter silently dead.
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
                // THE FAN-OUT, so this fixture HEARS what a match hears.
                // `dispatch_move_events` only writes `FxRequest`s; the cue is decided here.
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
                    // A COMPLETE `CaptureParticipant`, at BOTH ends.
                    // Acquisition requires the body role the whole lifecycle
                    // operates on, so half a body is refused — and this fixture
                    // built its captor without combat state, which the
                    // interruption rule then read as *"the captor despawned"*
                    // and dissolved every hold on the tick it formed. The
                    // architecture states the requirement now; the fixture
                    // satisfies it because a fighter really does carry all of
                    // this.
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
                    // spawn (`MotionModel`'s own doc: *"absence is not a policy"*).
                    // The throw hands the captive's air dodge to the shared hit
                    // reaction, and a fixture without one is not a body the throw
                    // system can see at all.
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
            ActorMoveset(crate::george_booul_moveset::george_booul_moveset()),
            ambition_platformer2d::characters::control::ActorControl(ActorControlFrame::neutral()),
        ));
        app.world_mut().entity_mut(victim).insert(
            // SHIELDING, and it changes nothing — the third leg of the
            // triangle, asserted in the real chain rather than in isolation.
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

    /// Run ticks until `done`, or panic. Moves take tenths of a second and the
    /// clock is 1/60, so a bounded loop is what "play this move out" means here.
    ///
    /// it presses NOTHING. An edge re-sent every tick would re-trigger the
    /// move under test, and the chain would be measuring a held button rather
    /// than a timeline playing out.
    fn run_until(app: &mut App, captor: Entity, label: &str, mut done: impl FnMut(&App) -> bool) {
        for _ in 0..120 {
            if done(app) {
                return;
            }
            // Production consumes control edges once; fixtures must do the same or
            // they restart the grab every tick before its active window.
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
        //    land on the press tick — which is the tell being real.
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

        // 2. The grab move ENDS and the relationship does not.
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
            // the RELATION must still be there, and the COUNT is the
            // ruleset's — two components since the split.
            app.world()
                .get::<CapturedBy>(victim)
                .expect("the pummel released the hold it belongs to");
            let state = app
                .world()
                .get::<ambition_platformer2d::characters::smash_capture::SmashHoldState>(victim)
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

    /// ⛔ THE ADAPTER ARM ITSELF. Every other carry guard starts from a
    /// `CaptureCarryRequested` that this module is the only writer of, so
    /// without this one a typo'd key would leave the whole feature dead with a
    /// fully green suite behind it.
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
                    ambition_platformer2d::characters::smash_capture::CAPTURE_CARRY.to_string(),
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
        // ⛔ AND IT IS NOT A THROW. Same key, wrong arm, would end the hold.
        let throws = app.world().resource::<Messages<CaptureThrowRequested>>();
        assert_eq!(throws.get_cursor().read(throws).count(), 0);
    }
}
