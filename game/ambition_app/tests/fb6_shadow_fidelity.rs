//! FB6e — the fidelity instrument (`docs/planning/engine/fighter-brain.md`
//! §12.6): does the fighter brain's shadow model agree with the REAL sim about
//! whether a swing lands?
//!
//! The shadow rollout (`brain::fighter::rollout`) is the brain's imagination —
//! deliberately not the sim. Its whole value rests on the imagination not
//! lying so hard that arg-max over rollout scores is noise, and this is the
//! instrument that measures exactly that, the way `motion_quality`'s boring
//! flat-ground baseline measured "it looks bad": as numbers against the real
//! thing.
//!
//! Shape: seat the shipped versus stage's two fighters (both on pads, so the
//! victim is a held-nothing statue), capture the attacker's REAL swing once to
//! learn its `MoveSpec` frame data — the same table the brain would read —
//! then, at several authored gaps, ask the shadow model "does this swing land
//! from here?" and let the real sim answer the same question with a real
//! swing. The floor is agreement on at least 3 of the 4 gaps: the reach-edge
//! case is allowed to disagree (the two hit tests will never share a boundary
//! pixel), and anything worse than one boundary miss means the model is
//! telling the brain fights that do not happen.
//!
//! The `WorldView` here is hand-filled from the fighters' real components —
//! the test is standing in for the perception pass, whose per-body view is
//! transient. `Perceived::cheating` is the designated fixture door (FB4a).

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use ambition_app::app::versus::{VERSUS_GAMEPLAY_ROUTE, VERSUS_ROOM_ID};
use ambition_app::app::versus_rules::{MatchPhase, VersusMatch};
use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::actors::actor::BodyKinematics;
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::characters::actor::{ActorFaction, BodyHealth};
use ambition_platformer2d::characters::brain::fighter::{
    shadow_step, ShadowEvent, ShadowIntent, ShadowState, ShadowTuning,
};
use ambition_platformer2d::characters::control::DrivingParticipant;
use ambition_platformer2d::characters::perception::{
    Perceived, PerceivedActor, SelfView, StageView, WorldView,
};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::MoveFrameData;
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId, ShellRouter};

fn pad_set(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
    app.world_mut()
        .write_message(bevy::input::gamepad::RawGamepadEvent::Button(
            bevy::input::gamepad::RawGamepadButtonChangedEvent::new(pad, button, value),
        ));
}

fn settle_to_launcher(app: &mut App) {
    for _ in 0..ambition_app::app::shared_host_startup_ticks() * 2 {
        app.update();
        if app
            .world()
            .resource::<ShellRouter>()
            .active
            .as_ref()
            .is_some_and(|active| active.route_id.as_str() == "ambition_launcher")
        {
            return;
        }
    }
    panic!("the host never reached its launcher");
}

fn settle_into_a_live_round(app: &mut App) {
    for _ in 0..600 {
        app.update();
        if matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Fighting
        ) {
            return;
        }
    }
    panic!("the round never went live");
}

fn kin(app: &mut App, body: Entity) -> BodyKinematics {
    *app.world().get::<BodyKinematics>(body).unwrap()
}

fn hp(app: &mut App, body: Entity) -> i32 {
    app.world().get::<BodyHealth>(body).unwrap().current()
}

/// Walk the attacker until the fighters' centers sit `target_gap` apart
/// (±8 px), then release the stick and let it come to rest.
/// Returns whether the requested gap was actually ESTABLISHED.
///
/// Two cases at the same arrived gap and different real outcomes then read as the model
/// contradicting itself, when the model had never been asked the same question twice.
///
/// the far gaps may be unreachable BY CONSTRUCTION: opening distance means
/// walking away from an opponent whose brain is chasing, so the retreat and the
/// pursuit cancel. That is a fact about the fixture, not about the model, and it
/// has to be reported as one.
#[must_use]
fn walk_to_gap(
    app: &mut App,
    pad: Entity,
    attacker: Entity,
    victim: Entity,
    target_gap: f32,
) -> bool {
    for _ in 0..900 {
        // A knockout mid-walk freezes the controls until the next round; a
        // walker that keeps pressing into the freeze burns its whole budget
        // standing still (run 3's case 3: target 234, arrived at 95).
        if !matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Fighting
        ) {
            settle_into_a_live_round(app);
        }
        let a = kin(app, attacker);
        let v = kin(app, victim);
        let gap = (v.pos.x - a.pos.x).abs();
        if (gap - target_gap).abs() <= 8.0 {
            break;
        }
        // Toward the victim to close, away to open.
        let closing = gap > target_gap;
        let toward = (v.pos.x - a.pos.x).signum();
        let dir = if closing { toward } else { -toward };
        let (press, release) = if dir > 0.0 {
            (GamepadButton::DPadRight, GamepadButton::DPadLeft)
        } else {
            (GamepadButton::DPadLeft, GamepadButton::DPadRight)
        };
        pad_set(app, pad, release, 0.0);
        pad_set(app, pad, press, 1.0);
        app.update();
    }
    pad_set(app, pad, GamepadButton::DPadLeft, 0.0);
    pad_set(app, pad, GamepadButton::DPadRight, 0.0);
    // Settle to TRUE rest, not a tick count: the instrument's question is
    // "does this swing land from HERE", and a body still sliding out of its
    // walk carries the answer somewhere else — run 3 disagreed at 102px
    // because the real attacker kept closing during the swing while the
    // shadow's view said everyone stood still.
    for _ in 0..240 {
        app.update();
        let a = kin(app, attacker);
        let v = kin(app, victim);
        if a.vel.length() < 2.0 && v.vel.length() < 2.0 {
            break;
        }
    }
    // Judged AFTER the settle, because the settle is part of establishing it: a
    // body still sliding out of its walk has not arrived anywhere yet.
    let a = kin(app, attacker);
    let v = kin(app, victim);
    ((v.pos.x - a.pos.x).abs() - target_gap).abs() <= 8.0
}

/// Stage the gap by PLACING the attacker, not by walking it there.
///
/// Walking could not reach the far gaps and never will: opening distance means
/// retreating from an opponent whose brain is chasing, so the retreat and the
/// pursuit cancel and the walker burns its budget arriving somewhere else. The
/// instrument's question is *"does this swing land from HERE"*, and HERE is the
/// fixture's to choose.
fn place_at_gap(app: &mut App, attacker: Entity, victim: Entity, target_gap: f32) -> bool {
    let victim_pos = kin(app, victim).pos;
    let attacker_pos = kin(app, attacker).pos;
    // THE SIDE IT FACES, not the side it stands on, and the difference is
    // the whole question this fixture asks. The intent was always "nothing has
    // to turn around" — but that only holds if the attacker is placed where its
    // CURRENT facing points at the victim. Reading its x instead worked only
    // while the two happened to agree, and they stopped agreeing when match
    // seating started PLACING both fighters (`seat_placement`) instead of
    // leaving seat zero wherever the session's home body already stood: the
    // attacker was staged 34px from a victim it was facing directly away from,
    // swung backwards, and the test reported the shadow model lying.
    let facing = kin(app, attacker).facing;
    let side = if facing >= 0.0 { -1.0 } else { 1.0 };
    let destination = ae::Vec2::new(victim_pos.x + side * target_gap, attacker_pos.y);

    let mut query = app.world_mut().query::<(
        Entity,
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d::actors::features::MotionModel,
    )>();
    let world = app.world_mut();
    let mut placed = false;
    for (entity, mut cluster_item, mut motion_model) in query.iter_mut(world) {
        if entity != attacker {
            continue;
        }
        let mut clusters = cluster_item.as_clusters_mut();
        ae::movement::transit_body(
            &mut motion_model,
            &mut clusters,
            destination,
            ae::movement::TransitVelocity::Zero,
        );
        placed = true;
    }
    if !placed {
        return false;
    }
    // One step so the placement is resolved against the world (a destination
    // inside geometry is pushed out, and the answer must be about where the body
    // ACTUALLY is), then read the gap that resulted.
    app.update();
    let a = kin(app, attacker);
    let v = kin(app, victim);
    ((v.pos.x - a.pos.x).abs() - target_gap).abs() <= 8.0
}

/// Hand-fill the view a perception pass would build for the attacker.
fn view_of(app: &mut App, attacker: Entity, victim: Entity) -> WorldView {
    let a = kin(app, attacker);
    let v = kin(app, victim);
    let facing = (v.pos.x - a.pos.x).signum();
    WorldView {
        self_view: SelfView {
            pos: a.pos,
            vel: a.vel,
            facing,
            half_extent: a.size * 0.5,
            gravity_down: ae::Vec2::new(0.0, 1.0),
            on_ground: true,
            alive: true,
            health_max: 100,
            ..Default::default()
        },
        stage: StageView {
            // Generous bounds: this instrument measures HIT fidelity, not KO
            // geometry, and a tight box would let a blastzone into a question
            // that is only about reach.
            bounds: ae::Aabb::new(a.pos, ae::Vec2::new(4000.0, 4000.0)),
        },
        actors: vec![PerceivedActor {
            id: "victim".to_string(),
            pos: v.pos,
            vel: v.vel,
            facing: -facing,
            half_extent: v.size * 0.5,
            faction: ActorFaction::Enemy,
            hostile_to_self: true,
            alive: true,
            on_ground: true,
            health_max: 100,
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// The shadow model's answer: thrown RIGHT NOW from this view, does the swing
/// connect within its own duration?
fn shadow_predicts_a_hit(view: &WorldView, frames: &MoveFrameData) -> bool {
    let mut s = ShadowState::from_perceived(Perceived::cheating(view))
        .expect("the victim is hostile and in view");
    let tuning = ShadowTuning::default();
    let dt = 1.0 / 60.0;
    let steps = (frames.total_s / dt).ceil() as u32 + 1;
    let mut intent = ShadowIntent::StartMove {
        frames: frames.clone(),
    };
    for _ in 0..steps {
        let events = shadow_step(&mut s, dt, &intent, &ShadowIntent::Hold, &tuning);
        if events
            .iter()
            .any(|e| matches!(e, ShadowEvent::Hit { on_me: false, .. }))
        {
            return true;
        }
        intent = ShadowIntent::Hold;
    }
    false
}

/// The real sim's answer: press the attack button and watch the victim's HP
/// over the swing (plus input-latch slack the shadow model does not have).
fn real_swing_lands(app: &mut App, pad: Entity, victim: Entity, frames: &MoveFrameData) -> bool {
    let before = hp(app, victim);
    pad_set(app, pad, GamepadButton::West, 1.0);
    for _ in 0..3 {
        app.update();
    }
    pad_set(app, pad, GamepadButton::West, 0.0);
    let window = (frames.total_s * 60.0).ceil() as u32 + 15;
    for _ in 0..window {
        app.update();
        if hp(app, victim) < before {
            return true;
        }
    }
    false
}

#[test]
fn the_shadow_model_agrees_with_the_real_sim_about_what_lands() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    let pad_one = app.world_mut().spawn(Gamepad::default()).id();
    let _pad_two = app.world_mut().spawn(Gamepad::default()).id();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let world = app.world_mut();
    let mut drivers = world.query::<(Entity, &DrivingParticipant)>();
    let mut seated: Vec<(u8, Entity)> = drivers
        .iter(world)
        .map(|(entity, driver)| (driver.0 .0, entity))
        .collect();
    seated.sort_by_key(|(slot, _)| *slot);
    assert_eq!(seated.len(), 2, "the arena did not seat two players");
    let (attacker, victim) = (seated[0].1, seated[1].1);

    // ── calibration: learn the swing from the sim's own move in flight ── The frame data comes
    // from the REAL MovePlayback the button starts, so the shadow model predicts the move the
    // fighter actually throws — no hand-typed table, exactly like L2. Walked rather than placed,
    // and the return is deliberately ignored: this one only needs the two close enough for a swing
    // to be worth throwing, and whether it arrived at exactly 30px does not change what
    // `MovePlayback` reports about the move.
    let _ = walk_to_gap(&mut app, pad_one, attacker, victim, 30.0);
    pad_set(&mut app, pad_one, GamepadButton::West, 1.0);
    let mut frames: Option<MoveFrameData> = None;
    for _ in 0..30 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MovePlayback)>();
        if let Some((_, playback)) = q.iter(world).find(|(owner, _)| *owner == attacker) {
            frames = Some(playback.spec.frame_data());
            break;
        }
    }
    pad_set(&mut app, pad_one, GamepadButton::West, 0.0);
    let frames = frames.expect("the attack button starts a MovePlayback on the attacker");
    assert!(
        frames.reach > 0.0 && frames.max_damage > 0,
        "the calibration swing has no priced strike: {frames:?}"
    );
    // Let the calibration swing and any i-frames fully clear.
    for _ in 0..180 {
        app.update();
    }

    // ── the four gaps: touching, comfortably in reach, out, far out ──
    let a_half = kin(&mut app, attacker).size.x * 0.5;
    let v_half = kin(&mut app, victim).size.x * 0.5;
    let touch = a_half + v_half + 4.0;
    let effective = frames.reach + v_half;
    let gaps = [
        touch.max(effective * 0.35),
        (effective * 0.75).max(touch + 8.0),
        effective * 1.8,
        effective * 3.5,
    ];

    let mut agreements = 0;
    let mut scored = 0;
    let mut unstaged: Vec<String> = Vec::new();
    let mut table = String::new();
    for (index, &gap) in gaps.iter().enumerate() {
        if !matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Fighting
        ) {
            settle_into_a_live_round(&mut app);
        }
        let established = place_at_gap(&mut app, attacker, victim, gap);
        let actual_gap = {
            let a = kin(&mut app, attacker);
            let v = kin(&mut app, victim);
            (v.pos.x - a.pos.x).abs()
        };
        let view = view_of(&mut app, attacker, victim);
        let predicted = shadow_predicts_a_hit(&view, &frames);
        let landed = real_swing_lands(&mut app, pad_one, victim, &frames);
        let agree = predicted == landed;
        // a case the fixture could not stage is NOT evidence about the model, in either
        // direction.
        if established {
            scored += 1;
            agreements += agree as u32;
        } else {
            unstaged.push(format!(
                "  case {index}: asked for {gap:.0}px, arrived at {actual_gap:.0}px"
            ));
        }
        table.push_str(&format!(
            "  case {index}: gap {actual_gap:.0}px (target {gap:.0}) — shadow: {predicted}, real: {landed}{}\n",
            if !established {
                "  ← NOT STAGED, not scored"
            } else if agree {
                ""
            } else {
                "  ← DISAGREE"
            }
        ));
        // Clear hitstun/i-frames before the next question.
        for _ in 0..150 {
            app.update();
        }
    }

    // The fixture's own health is asserted FIRST and separately, because a
    // fixture that cannot stage its question produces a number about nothing —
    // and a test that reports such a number as a verdict on the subject is the
    // failure mode this file has now hit three times.
    assert!(
        unstaged.is_empty(),
        "the fixture could not STAGE {} of 4 gaps, so those cases say nothing \
         about the shadow model:\n{}\n\n⚠ opening a gap means walking away from \
         an opponent whose brain is chasing, so the far cases may be unreachable \
         by construction — the repair is to PLACE the bodies through the engine's \
         relocation authority (as `room_replay.rs` does) or to quiesce the \
         victim, not to widen the tolerance.\n{table}",
        unstaged.len(),
        unstaged.join("\n"),
    );
    assert!(
        agreements * 4 >= scored * 3,
        "the shadow model agreed with the real sim on only {agreements}/{scored} \
         STAGED gaps — the imagination is lying hard enough that rollout arg-max \
         is noise. (reach {:.0}px, effective {effective:.0}px)\n{table}",
        frames.reach
    );
}
