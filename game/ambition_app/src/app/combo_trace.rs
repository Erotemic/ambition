//! The HUD's movement combo readout ("J o D o D").
//!
//! Presentation state only: it is fed from the movement step's published
//! [`ae::FrameEvents`] (via [`PlayerBodyFrameOutput`]) after the kernel ran, it
//! is not rollback state, and nothing in the simulation reads it.

use bevy::prelude::*;

use ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput;
use ambition_platformer2d::engine_core as ae;

/// How many marks the readout keeps.
const COMBO_TRACE_CAPACITY: usize = 18;
/// How long a mark stays in the readout, in sim seconds. A reset mark stays.
const COMBO_MARK_LIFETIME_SECS: f32 = 4.0;

/// One op in the readout and how long ago it happened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComboMark {
    pub op: ae::MovementOp,
    pub age: f32,
}

/// The primary player's recent movement ops, for the HUD. Inserted by
/// [`record_combo_trace`] on the first tick it has something to show.
#[derive(Component, Clone, Debug, Default)]
pub struct ComboTrace {
    pub marks: Vec<ComboMark>,
}

impl ComboTrace {
    pub fn symbols(&self) -> String {
        if self.marks.is_empty() {
            return "-".to_string();
        }
        self.marks
            .iter()
            .map(|m| m.op.symbol())
            .collect::<Vec<_>>()
            .join(" o ")
    }

    fn advance(&mut self, dt: f32, ops: &[ae::MovementOp]) {
        for mark in &mut self.marks {
            mark.age += dt;
        }
        self.marks
            .retain(|m| m.age < COMBO_MARK_LIFETIME_SECS || m.op == ae::MovementOp::Reset);
        self.marks
            .extend(ops.iter().map(|&op| ComboMark { op, age: 0.0 }));
        let excess = self.marks.len().saturating_sub(COMBO_TRACE_CAPACITY);
        self.marks.drain(0..excess);
    }

    fn restart(&mut self) {
        self.marks.clear();
        self.marks.push(ComboMark {
            op: ae::MovementOp::Reset,
            age: 0.0,
        });
    }
}

/// Append this tick's movement ops to the primary player's readout.
///
/// A rollback resimulation replays ticks the readout has already shown, so
/// replayed ticks are skipped.
pub fn record_combo_trace(
    mut commands: Commands,
    world_time: Res<ambition_platformer2d::time::WorldTime>,
    replay: Option<Res<ambition_platformer2d::platformer::schedule::SimulationReplayState>>,
    mut bodies: Query<
        (Entity, &PlayerBodyFrameOutput, Option<&mut ComboTrace>),
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
) {
    if replay.is_some_and(|replay| replay.replaying_history) {
        return;
    }
    for (entity, frame_out, trace) in &mut bodies {
        let ops = &frame_out.events.operations;
        match trace {
            Some(mut trace) => trace.advance(world_time.scaled_dt, ops),
            None if !ops.is_empty() => {
                let mut trace = ComboTrace::default();
                trace.advance(0.0, ops);
                commands.entity(entity).insert(trace);
            }
            None => {}
        }
    }
}

/// A restarted body's readout starts again from a reset mark.
pub fn restart_combo_trace(restart: On<ae::BodyRestarted>, mut traces: Query<&mut ComboTrace>) {
    if let Ok(mut trace) = traces.get_mut(restart.entity) {
        trace.restart();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Marks age out, a reset mark does not, and the readout is bounded.
    #[test]
    fn the_readout_ages_out_and_keeps_its_reset_mark() {
        let mut trace = ComboTrace::default();
        trace.restart();
        trace.advance(0.0, &[ae::MovementOp::Jump, ae::MovementOp::Dash]);
        assert_eq!(trace.symbols().matches(" o ").count(), 2);

        trace.advance(COMBO_MARK_LIFETIME_SECS, &[]);
        assert_eq!(
            trace.marks.iter().map(|m| m.op).collect::<Vec<_>>(),
            vec![ae::MovementOp::Reset],
            "ops older than the lifetime leave; the reset mark stays"
        );

        let many = vec![ae::MovementOp::Jump; COMBO_TRACE_CAPACITY * 2];
        trace.advance(0.0, &many);
        assert_eq!(trace.marks.len(), COMBO_TRACE_CAPACITY);
    }
}
