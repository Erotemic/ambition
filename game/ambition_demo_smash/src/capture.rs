//! **The Smash ruleset's capture adapter: authored effect keys → typed requests.**
//!
//! ```text
//! Smash authoring     an EffectRef on a move's window or timeline
//!        ↓
//! THIS MODULE         recognises the key, hydrates the params
//!        ↓
//! combat/body runtime CaptureAttemptRequested / Pummel / Throw
//! ```
//!
//! ⛔ **the generic body runtime never matches `"smash.capture_throw"`**, and
//! this adapter never touches body ECS state. Each half does the thing it is the
//! right place for: a ruleset knows what its own authored strings mean, and a
//! body runtime knows how to hold and launch a body. Collapsing them would put
//! Smash vocabulary in the engine or body surgery in the game, and both are the
//! dependency this split exists to avoid.
//!
//! ⚠ an unrecognised key falls through untouched — other techniques ride the
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
        // ⚠ **irrefutable today, and destructured anyway.** `SpecialActionSpec`
        // has exactly one variant since the per-boss variants collapsed onto the
        // keyed effect seam. Naming it means the day a second variant arrives,
        // this becomes a compile error at the one place that has to decide
        // whether the new kind can carry a capture — rather than a silent
        // fall-through that stops recognising grabs.
        let SpecialActionSpec::Special(key) = spec;
        match key.as_str() {
            CAPTURE_ATTEMPT => {
                // ⚠ a params typo is a STARTUP error, not a silent default: the
                // key registers `check_hydrates` with the param-schema registry,
                // so a fighter's bad grab data fails the content pass. Reaching
                // here with unhydratable params means the registration is
                // missing, which is worth the log rather than a silent skip.
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
    use ambition_platformer2d::actors::features::ecs::capture::{
        acquire_captures, apply_capture_pummels, apply_capture_throws, constrain_captive_bodies,
        release_interrupted_captures,
    };
    use ambition_platformer2d::characters::actor::control::ActorControlFrame;
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
                // ⭐ **THE FAN-OUT, so this fixture HEARS what a match hears.**
                // `dispatch_move_events` only writes `FxRequest`s; the cue is
                // decided here. Without it the chain was silent and could not
                // have seen an audio defect at all.
                ambition_platformer2d::render::fx::process_fx_requests,
                translate_smash_capture_effects,
                acquire_captures,
                apply_capture_pummels,
                apply_capture_throws,
                constrain_captive_bodies,
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
                        on_ground: true,
                        contact_initialized: true,
                    },
                    // ⭐⭐ **A COMPLETE `CaptureParticipant`, at BOTH ends.**
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
            ambition_platformer2d::characters::brain::ActorControl(ActorControlFrame::neutral()),
        ));
        app.world_mut().entity_mut(victim).insert(
            // ⭐ SHIELDING, and it changes nothing — the third leg of the
            // triangle, asserted in the real chain rather than in isolation.
            ambition_platformer2d::engine_core::BodyShieldState::default(),
        );
        (captor, victim)
    }

    /// Press `f` on the captor for one tick, then run the chain.
    fn press(app: &mut App, captor: Entity, f: impl FnOnce(&mut ActorControlFrame)) {
        let mut control = app
            .world_mut()
            .get_mut::<ambition_platformer2d::characters::brain::ActorControl>(captor)
            .expect("the captor carries a control frame");
        control.0 = ActorControlFrame::neutral();
        f(&mut control.0);
        app.update();
    }

    /// Run ticks until `done`, or panic. Moves take tenths of a second and the
    /// clock is 1/60, so a bounded loop is what "play this move out" means here.
    ///
    /// ⚠ it presses NOTHING. An edge re-sent every tick would re-trigger the
    /// move under test, and the chain would be measuring a held button rather
    /// than a timeline playing out.
    fn run_until(app: &mut App, captor: Entity, label: &str, mut done: impl FnMut(&App) -> bool) {
        for _ in 0..120 {
            if done(app) {
                return;
            }
            // ⛔⛔ **THE EDGE MUST BE CLEARED, and forgetting it cost a debug
            // cycle.** In production the control pipeline consumes an edge once
            // (`ActorControlFrame::clear_edges`); a fixture that leaves
            // `grab_pressed` true re-triggers the grab EVERY tick, restarting it
            // at t=0 so it never reaches its own Active window at 0.16s. The
            // symptom is "the grab never catches anybody", which reads exactly
            // like a broken acquisition.
            if let Some(mut control) =
                app.world_mut()
                    .get_mut::<ambition_platformer2d::characters::brain::ActorControl>(captor)
            {
                control.0.clear_edges();
            }
            app.update();
        }
        panic!("{label} never happened within 2 seconds of sim time");
    }

    /// **⭐⭐ THE ACCEPTANCE SEQUENCE, END TO END, THROUGH THE REAL SYSTEMS.**
    ///
    /// Every stage of this was pinned in isolation as it landed. This is the one
    /// that would catch them being individually right and jointly wrong — an
    /// ordering that works in a hand-built app and not in the chain, an authored
    /// timing that never reaches its own event, a relationship that survives its
    /// unit test and not a real move ending.
    ///
    /// ```text
    /// Grab            → the authored grab plays, catches a SHIELDING opponent
    /// grab move ends  → CapturedBy SURVIVES it
    /// Attack          → pummel; hold survives
    /// Attack          → pummel again; hold survives
    /// Forward+Attack  → throw; the authored release ends the hold exactly once
    /// ```
    /// ⛔⛔ **IT WAS RED, AND THE FIRST DIAGNOSIS OF WHY WAS WRONG.**
    ///
    /// A tick-by-tick probe showed the grab reaching its own effect at exactly
    /// the authored frame (t=0.167 against an authored 0.16) and the adapter
    /// translating it — yet `CapturedBy` was never observed. That was read as
    /// *"acquisition declines this fixture's victim"*. It did not: acquisition
    /// SUCCEEDED, and `release_interrupted_captures`, later in the same chained
    /// update, dissolved the hold before anything could see it — because this
    /// fixture's captor carried no `BodyCombat`, and that rule asked
    /// `combat.get(captor).is_err()` as though it meant *the captor is gone*.
    ///
    /// ⭐ the lesson is in where the evidence pointed. Every observation was
    /// accurate and the conclusion drawn from them was not: a state observed
    /// only BETWEEN systems is invisible to a test that looks after the whole
    /// update, so "never established" and "established and destroyed" produce
    /// identical evidence at the only place anyone was looking.
    ///
    /// The fix was the architecture, not this fixture: acquisition now requires
    /// a `CaptureParticipant` of both ends, and existence is asked of the world
    /// rather than inferred from a component. The fixture builds whole bodies
    /// because a fighter is one.
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
            // ⚠ the RELATION must still be there, and the COUNT is the
            // ruleset's — two components since the 2026-08-19 split.
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
