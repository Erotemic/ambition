//! The Smash ruleset's capture adapter: authored effect keys → typed requests.
//!
//! ```text
//! Smash authoring     an EffectRef on a move's window or timeline
//!        ↓
//! THIS MODULE         recognises the key, hydrates the params
//!        ↓
//! combat/body runtime CaptureAttemptRequested / Pummel / Throw
//! ```
//!
//! the generic body runtime never matches `"smash.capture_throw"`, and
//! this adapter never touches body ECS state. Each half does the thing it is the
//! right place for: a ruleset knows what its own authored strings mean, and a
//! body runtime knows how to hold and launch a body. Collapsing them would put
//! Smash vocabulary in the engine or body surgery in the game, and both are the
//! dependency this split exists to avoid.
//!
//! an unrecognised key falls through untouched — other techniques ride the
//! same channel, and a `continue` here is how they stay unaffected.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_capture::{
    CaptureAttemptParams, CapturePummelParams, CaptureThrowParams, CAPTURE_ATTEMPT, CAPTURE_PUMMEL,
    CAPTURE_THROW,
};
use ambition_platformer2d::combat::capture::{
    CaptureAttemptRequested, CapturePummelRequested, CaptureThrowRequested,
};
use ambition_platformer2d::engine_core as ae;

/// Translate this tick's authored capture effects into typed runtime requests.
pub fn translate_smash_capture_effects(
    mut actions: MessageReader<ActorActionMessage>,
    mut attempts: MessageWriter<CaptureAttemptRequested>,
    mut pummels: MessageWriter<CapturePummelRequested>,
    mut throws: MessageWriter<CaptureThrowRequested>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        // irrefutable today, and destructured anyway. `SpecialActionSpec`
        // has exactly one variant since the per-boss variants collapsed onto the
        // keyed effect seam. Naming it means the day a second variant arrives,
        // this becomes a compile error at the one place that has to decide
        // whether the new kind can carry a capture — rather than a silent
        // fall-through that stops recognising grabs.
        let SpecialActionSpec::Special(key) = spec;
        match key.as_str() {
            CAPTURE_ATTEMPT => {
                // ⛔⛔ THIS COMMENT PROMISED A STARTUP CHECK THAT DOES NOT
                // EXIST — corrected 2026-09-05 after ToothbrushAmbition counted
                // the callers. It said a params typo "is a STARTUP error, not a
                // silent default", because the key "registers `check_hydrates`
                // with the param-schema registry". Nothing does.
                // `ParamSchemaRegistry` has ZERO production users: `register`
                // and `validate_all` are called only from
                // `ambition_entity_catalog`'s own tests, and the type is not
                // mentioned outside that crate. The registry is never populated
                // in any shipped build.
                //
                // ⇒ SO THIS `Err` ARM IS NOT A LAST RESORT, IT IS THE ONLY
                // THING between a fighter's bad grab data and a grab that
                // silently does nothing. The old comment's conditional —
                // "reaching here means the registration is missing" — was
                // unconditionally true. The log fires; nothing fails at startup.
                //
                // ⚠ NOT KNOWN TO BE FIRING: whether any authored fighter has
                // params that fail to hydrate was NOT established, so this is a
                // missing guard rather than a broken grab. ⇒ Making it a real
                // startup error is O4's business (the installed-technique
                // catalog), and the first step is smaller than it looks —
                // something must tell the registry that anything exists at all
                // before it can answer "`smash.teleprot` does not exist".
                match params.hydrate::<CaptureAttemptParams>() {
                    Ok(p) => attempts.write(CaptureAttemptRequested {
                        captor: message.actor,
                        offset: ae::Vec2::new(p.offset.0, p.offset.1),
                        half_extents: ae::Vec2::new(p.half_extents.0, p.half_extents.1),
                        hold_offset: ae::Vec2::new(p.hold_offset.0, p.hold_offset.1),
                    }),
                    Err(err) => {
                        warn!("smash capture attempt params did not hydrate: {err}");
                        continue;
                    }
                };
            }
            CAPTURE_PUMMEL => match params.hydrate::<CapturePummelParams>() {
                Ok(p) => {
                    pummels.write(CapturePummelRequested {
                        captor: message.actor,
                        damage: p.damage,
                    });
                }
                Err(err) => warn!("smash pummel params did not hydrate: {err}"),
            },
            CAPTURE_THROW => match params.hydrate::<CaptureThrowParams>() {
                Ok(p) => {
                    throws.write(CaptureThrowRequested {
                        captor: message.actor,
                        damage: p.damage,
                        knockback: p.knockback,
                        knockback_growth: p.knockback_growth,
                        launch_dir: ae::Vec2::new(p.launch_dir.0, p.launch_dir.1),
                    });
                }
                Err(err) => warn!("smash throw params did not hydrate: {err}"),
            },
            _ => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                translate_smash_capture_effects,
                acquire_captures,
                apply_capture_pummels,
                apply_capture_throws,
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
}
