//! **The death beat.** She is hit with nothing left to lose, and the level stops
//! to say so before it starts again.
//!
//! Jon: *"if mary-o is small and she takes damage she should die and use her
//! death animation, then restart the level."*
//!
//! Two halves, and only one of them is code.
//!
//! ## The fragility is DATA
//!
//! Her catalog rows author `max_health: 1`. The classic ladder is already the
//! engine's armor precedence — spark blossom, then grow cap, then the body —
//! so authoring one point of health is the whole rule: whatever she is wearing
//! absorbs the hit, and when there is nothing left the next one is fatal. No
//! demo system reads "is she small"; being small IS having spent the armor.
//!
//! ## The beat is a sequence, exactly like the flagpole's
//!
//! The engine respawns a dead player IMMEDIATELY — that is why her `Death` row
//! was unreachable however good the sheet was. So this holds the level for a
//! beat after the death and drives the body the way [`crate::flag`] drives it
//! during the slide: the pose is an external kinematic constraint (ADR 0024),
//! the controls are blanked, and the death row plays through the shared
//! `BodyAnimFacts::death_anim_timer` the engine now carries for exactly this.
//!
//! She dies WHERE SHE DIED, not at spawn: [`ActorDiedMessage`] carries the
//! position, and holding her there is the difference between a death you read
//! and a body that blinks across the screen.
//!
//! When the beat ends the level replays, which is the same seam the flag tally
//! uses — a death and a clear leave by the same door.

use bevy::prelude::*;

use ambition::engine_core as ae;

/// How long the level holds on her death before it starts again.
///
/// Long enough to read the pose and hear the sting, short enough that a run of
/// deaths does not become a run of waiting.
pub const DEATH_DWELL: f32 = 1.6;

/// The death beat's live state. Rides the same owner entity as the level clock
/// and the flag sequence, so all three are torn down together.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct MaryODeathSequence {
    /// Seconds left in the beat, or `0.0` when nothing is happening.
    pub remaining: f32,
    /// Where she died — the body is held here for the whole beat.
    ///
    /// `None` when nothing could say where: a rules-only harness with no
    /// kinematic body, or a lost attempt reported without one. The beat still
    /// runs and still restarts the level; there is simply no pose to hold, which
    /// is the honest answer rather than holding it at the origin.
    pub at: Option<ae::Vec2>,
}

impl MaryODeathSequence {
    /// Is the beat playing? While true the level clock stops, the controls are
    /// ignored, and the level does not replay yet.
    pub fn active(&self) -> bool {
        self.remaining > 0.0
    }

    /// Start the beat at `at`, unless one is already playing.
    ///
    /// Returns whether it started, so a caller can voice the death exactly once.
    /// A second death landing mid-beat is ignored rather than restarting it: a
    /// body still overlapping whatever killed it would otherwise extend its own
    /// death indefinitely.
    pub fn begin(&mut self, at: Option<ae::Vec2>) -> bool {
        if self.active() {
            return false;
        }
        self.remaining = DEATH_DWELL;
        self.at = at;
        true
    }
}

/// Start the beat on the frame she dies.
///
/// Reads the engine's own death fact rather than watching her health, so every
/// way of dying — a hit that got past her armor, a pit, the clock — arrives
/// here by the same door.
pub fn begin_death_sequence(
    mut deaths: MessageReader<ambition::actors::ActorDiedMessage>,
    mut sequences: Query<&mut MaryODeathSequence>,
    mut sfx: ambition::sfx::SfxWriter,
) {
    // Drain unconditionally so a death that landed during a load cannot be
    // re-read and charged to the next attempt (the same rule the life counter
    // follows).
    let Some(death) = deaths.read().last().cloned() else {
        return;
    };
    let Ok(mut sequence) = sequences.single_mut() else {
        return;
    };
    if !sequence.begin(Some(death.pos)) {
        return;
    }
    // The engine's shared reset cue. AMBITION_REVIEW(audio): the authored
    // `mary_o_you_died` score has a `death_sting` section for this exact moment;
    // it attaches HERE once the track is rendered and registered. Deliberately
    // not named yet — a cue id with nothing behind it fails silently, which is
    // the failure mode this demo has already been bitten by twice.
    sfx.write(ambition::sfx::SfxMessage::Reset { pos: death.pos });
}

/// Hold her at the place she died, in the death pose, for the beat.
///
/// The body is DRIVEN rather than frozen — the same choice the flag sequence
/// makes and for the same reason: a frozen body is still a body, and gravity
/// would walk it out from under the pose.
pub fn run_death_sequence(
    time: Res<ambition::time::WorldTime>,
    subject: Option<Res<ambition::platformer::markers::ControlledSubject>>,
    mut sequences: Query<&mut MaryODeathSequence>,
    mut bodies: Query<(
        &mut ae::BodyKinematics,
        &mut ambition::characters::brain::ActorControl,
        &mut ambition::actors::actor::BodyAnimFacts,
    )>,
) {
    let Ok(mut sequence) = sequences.single_mut() else {
        return;
    };
    if !sequence.active() {
        return;
    }
    sequence.remaining = (sequence.remaining - time.scaled_dt).max(0.0);
    let Some(entity) = subject.and_then(|s| s.0) else {
        return;
    };
    let Ok((mut kin, mut control, mut anim)) = bodies.get_mut(entity) else {
        return;
    };
    if let Some(at) = sequence.at {
        ae::movement::constrain_body_pose(&mut kin, at, ae::Vec2::ZERO);
    }
    control.0 = ambition::characters::actor::control::ActorControlFrame::default();
    // Re-armed every tick rather than set once: the engine's respawn calls
    // `BodyAnimFacts::reset()`, so a single arming would be wiped on the very
    // frame the death happened.
    anim.death_anim_timer = sequence.remaining.max(time.scaled_dt);
}

/// End the beat: restart the level.
///
/// Split from the tick so the replay is requested on the frame the dwell runs
/// out and never on the frame the death lands — a replay in the same frame as
/// the death would cancel the beat it is supposed to follow.
pub fn restart_level_after_death(
    mut sequences: Query<&mut MaryODeathSequence>,
    mut was_active: Local<bool>,
    mut replay: MessageWriter<ambition::actors::session::reset::RoomReplayRequested>,
) {
    let Ok(sequence) = sequences.single_mut() else {
        *was_active = false;
        return;
    };
    let active = sequence.active();
    if *was_active && !active {
        replay.write(ambition::actors::session::reset::RoomReplayRequested);
    }
    *was_active = active;
}

#[cfg(test)]
mod tests;
