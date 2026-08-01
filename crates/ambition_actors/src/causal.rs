//! This crate's causal facts.
//!
//! **Why did this body move this tick?** — the inspector's first required
//! question, answered for a seated body.
//!
//! ## Observer by construction, not by discipline
//!
//! [`record_player_movement_intent`] takes every component immutably and holds
//! no mutable handle to anything but the log. It CANNOT affect the simulation,
//! and that is a property of its signature rather than a promise in a comment —
//! which matters here more than usual, because a rollback host resimulates and
//! an instrument that nudged state would desync exactly when it was being used.
//!
//! It runs AFTER the brain tick rather than inside it for the same reason: the
//! alternative was threading a recorder through `tick_player_brains`, and a
//! system that only reads cannot be the thing that broke the tick.
//!
//! ## The subject is the SEAT
//!
//! Not an `Entity` — indices are recycled and `to_bits` ordering is a trap this
//! repo has already been bitten by — and not a `SimId`, which bodies do not
//! carry. A seat is stable across death and respawn, which is precisely the
//! window an investigation spans: "why did seat 1 walk off the stage" survives
//! the three respawns in the middle of the answer.

use ambition_causal::{CausalFact, CausalRecording, FactDetail, SubjectKey, domains};
use bevy::prelude::*;

use crate::avatar::movement_components::{BodyGroundState, BodyKinematics};
use ambition_characters::brain::{ActorControl, Brain};

/// Publish one movement-intent fact per seated body per tick.
///
/// Records the intent the brain EMITTED alongside the body state it was emitted
/// from, which is what makes the fact answer "why" rather than "what": a body
/// that did not move because its brain asked for nothing is a different finding
/// from one that asked and was refused, and the two are indistinguishable from
/// a position sample.
pub fn record_player_movement_intent(
    log: Option<ResMut<CausalRecording>>,
    bodies: Query<(&BodyKinematics, &BodyGroundState, &Brain, &ActorControl)>,
) {
    let Some(mut log) = log else {
        return;
    };
    if !log.is_recording() {
        return;
    }
    for (kin, ground, brain, control) in &bodies {
        // Only seated bodies: the seat IS the identity, so a body without one
        // has nothing an explanation could be keyed on. Publishing it under a
        // recycled entity index would be worse than not publishing it.
        let Some(slot) = brain.player_slot() else {
            continue;
        };
        let frame = &control.0;
        log.record(
            CausalFact::new(
                domains::MOVEMENT,
                0,
                FactDetail::new(
                    "movement_intent",
                    format!(
                        "seat {} asked for lateral {:+.2}{}",
                        slot.0,
                        frame.locomotion.x,
                        if frame.jump_pressed { " and a jump" } else { "" }
                    ),
                ),
            )
            .about(SubjectKey::Seat(slot.0))
            .by_participant(slot.0)
            .field("locomotion_x", frame.locomotion.x)
            .field("locomotion_y", frame.locomotion.y)
            .field("jump_pressed", frame.jump_pressed)
            .field("jump_held", frame.jump_held)
            .field("pos_x", kin.pos.x)
            .field("pos_y", kin.pos.y)
            .field("vel_x", kin.vel.x)
            .field("vel_y", kin.vel.y)
            .field("on_ground", ground.on_ground)
            .field("facing", kin.facing),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_causal::{FactValue, RecordingPolicy};
    use ambition_characters::brain::PlayerSlot;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<CausalRecording>();
        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_policy(RecordingPolicy::All);
        app.add_systems(Update, record_player_movement_intent);
        app
    }

    fn seated_body(app: &mut App, slot: u8, locomotion_x: f32) {
        let mut control = ActorControl::default();
        control.0.locomotion.x = locomotion_x;
        app.world_mut().spawn((
            BodyKinematics {
                pos: ambition_engine_core::Vec2::new(120.0, 300.0),
                ..Default::default()
            },
            BodyGroundState::default(),
            Brain::Player(PlayerSlot(slot)),
            control,
        ));
    }

    #[test]
    fn each_seated_body_explains_its_own_movement() {
        let mut app = app();
        seated_body(&mut app, 0, 1.0);
        seated_body(&mut app, 1, -1.0);
        app.world_mut().resource_mut::<CausalRecording>().set_tick(30);
        app.update();

        let log = app.world().resource::<CausalRecording>();
        for (slot, expected) in [(0u8, 1.0_f32), (1, -1.0)] {
            let explanation = log.explain(30, &SubjectKey::Seat(slot));
            let intent = explanation
                .first("movement_intent")
                .unwrap_or_else(|| panic!("seat {slot} published its intent"));
            assert_eq!(
                intent.get("locomotion_x"),
                Some(&FactValue::Float(expected.into())),
                "seat {slot} explains ITS OWN movement, not another seat's"
            );
            assert_eq!(intent.participant, Some(slot));
        }
    }

    #[test]
    fn a_body_with_no_seat_publishes_nothing_rather_than_a_recycled_index() {
        let mut app = app();
        app.world_mut().spawn((
            BodyKinematics::default(),
            BodyGroundState::default(),
            Brain::stand_still(),
            ActorControl::default(),
        ));
        app.update();
        assert!(
            app.world().resource::<CausalRecording>().is_empty(),
            "an unseated body has no stable identity, and an entity index is not one — \
             indices are recycled, so a later body would inherit this one's explanation"
        );
    }

    #[test]
    fn the_intent_distinguishes_asking_for_nothing_from_being_refused() {
        // A position sample cannot tell these apart, which is the whole reason
        // the fact records the EMITTED intent beside the body state.
        let mut app = app();
        seated_body(&mut app, 0, 0.0);
        app.update();
        let log = app.world().resource::<CausalRecording>();
        let intent = log
            .explain(0, &SubjectKey::Seat(0))
            .first("movement_intent")
            .cloned()
            .expect("a still body still explains itself");
        assert_eq!(intent.get("locomotion_x"), Some(&FactValue::Float(0.0)));
        assert_eq!(
            intent.get("vel_x"),
            Some(&FactValue::Float(0.0)),
            "asked for nothing, moving at nothing — the pair is the finding"
        );
    }
}
