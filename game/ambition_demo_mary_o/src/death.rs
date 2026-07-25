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
/// **Sized by the music, not by taste.** `mary_o_you_died` is four bars of 2/4
/// at 150bpm — eight beats, 3.2 seconds — and the last chord is the comic low
/// thud the whole cue exists to land. At the old 1.6s this cut off halfway
/// through the tumble, so the sting never resolved; Jon reported it as "death
/// isn't long enough for the entire death music to play". If the score's tempo
/// or bar count changes, this number changes with it.
pub const DEATH_DWELL: f32 = 3.2;

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
    /// Whether the attempt this beat is playing has already cost a life.
    ///
    /// A lost attempt costs ONE life however many times it is reported, and a
    /// body pinned exactly where it died keeps getting reported: she is still in
    /// the pit. The beat is what knows an attempt is being lost right now, so it
    /// is what carries the debt. Armed false by [`Self::begin`] and spent once.
    pub life_spent: bool,
    /// This beat still owes the level a replay. Armed by [`Self::begin`],
    /// spent by [`restart_level_after_death`] when the dwell runs out.
    ///
    /// The edge used to live in a `Local<bool>` inside that system, which put
    /// "has the level already been restarted for this death" outside the
    /// rollback envelope — so a rewind across the end of a beat could replay
    /// twice or not at all, and nothing in the sim could tell which had
    /// happened. As a debt the beat carries it rides the same snapshot as the
    /// rest of the beat, and spending it is idempotent because it is a state
    /// rather than an observation of the previous frame.
    pub replay_pending: bool,
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
        self.replay_pending = true;
        self.life_spent = false;
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
    // The engine's shared reset cue voices the moment itself; the music the beat
    // plays over is `play_death_music` below.
    sfx.write(ambition::sfx::SfxMessage::Reset { pos: death.pos });
}

/// **Her death has its own music.**
///
/// Written into the encounter layer's PRIORITY tier — the same slot a focused
/// fight claims — because that is the one tier that outranks the room's own
/// theme. It is claimed every frame the beat is live and released the frame it
/// ends, so the level theme returns on its own with no second system to keep in
/// sync and nothing to leak if the beat is interrupted.
///
/// It CLAIMS rather than assigns. The tier used to be a bare `Option<String>`
/// that every writer cleared when it had nothing to say, and the boss system —
/// which runs later in the frame and has no run condition — cleared it on every
/// frame of a demo that contains no bosses. This music was written and wiped
/// within the same frame, forever, and the bare-`App` test could not see it
/// because the boss system was not in that app.
///
/// The track is authorized by Mary-O's audio fragment
/// ([`crate::provider::MARY_O_DEATH_MUSIC_TRACK`]); under provider-relative
/// playback an undeclared id is gated to silence however loudly it is requested.
pub fn play_death_music(
    sequences: Query<&MaryODeathSequence>,
    music: Option<
        ambition::platformer::lifecycle::SessionWorldMut<
            ambition::actors::encounter::EncounterMusicRequest,
        >,
    >,
) {
    let (Ok(sequence), Some(mut music)) = (sequences.single(), music) else {
        return;
    };
    if sequence.active() {
        music.claim_priority(DEATH_MUSIC_OWNER, crate::provider::MARY_O_DEATH_MUSIC_TRACK);
    } else {
        music.release_priority(DEATH_MUSIC_OWNER);
    }
}

/// This beat's claim on the encounter layer's priority music tier.
const DEATH_MUSIC_OWNER: &str = "mary_o_death";

/// Hold her at the place she died, in the death pose, for the beat.
///
/// The body is DRIVEN rather than frozen — the same choice the flag sequence
/// makes and for the same reason: a frozen body is still a body, and gravity
/// would walk it out from under the pose.
pub fn run_death_sequence(
    time: Res<ambition::time::WorldTime>,
    subject: Option<Res<ambition::platformer::markers::ControlledSubject>>,
    mut commands: Commands,
    mut sequences: Query<&mut MaryODeathSequence>,
    mut bodies: Query<(
        &mut ae::BodyKinematics,
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
    let Ok((mut kin, mut anim)) = bodies.get_mut(entity) else {
        return;
    };
    if let Some(at) = sequence.at {
        ae::movement::constrain_body_pose(&mut kin, at, ae::Vec2::ZERO);
    }
    // The beat OWNS the body while it plays. Blanking the control frame here
    // used to be this system's job and never worked: this phase runs after
    // everything that reads the frame, and the brain refills it before the next
    // reader anyway. `ScriptedControl` moves that blanking to the one position
    // where it is observable, and takes her out of the pickup pass with it.
    //
    // The early return above means the beat was live on entry, so the `else` is
    // exactly the frame the dwell ran out — she gets the body back for the
    // replay rather than staying blanked into the next attempt.
    if sequence.remaining > 0.0 {
        commands
            .entity(entity)
            .try_insert(ambition::characters::brain::ScriptedControl);
    } else {
        commands
            .entity(entity)
            .remove::<ambition::characters::brain::ScriptedControl>();
    }
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
///
/// "Has this beat already replayed?" is answered by the beat itself rather than
/// by comparing against the previous frame. A beat that has run down and not yet
/// asked is the ONE state that asks, so the request happens exactly once however
/// many times the frame is simulated.
pub fn restart_level_after_death(
    mut sequences: Query<&mut MaryODeathSequence>,
    mut replay: MessageWriter<ambition::actors::session::reset::RoomReplayRequested>,
) {
    let Ok(mut sequence) = sequences.single_mut() else {
        return;
    };
    if sequence.active() || !sequence.replay_pending {
        return;
    }
    sequence.replay_pending = false;
    replay.write(ambition::actors::session::reset::RoomReplayRequested);
}

#[cfg(test)]
mod tests;
