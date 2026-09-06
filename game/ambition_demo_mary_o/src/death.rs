//! The death beat. She is hit with nothing left to lose, and the level stops
//! to say so before it starts again.
//!
//! death animation, then restart the level."*
//!
//! Two halves, and neither of them is a state machine any more.
//!
//! ## The fragility is DATA
//!
//! Her catalog rows author `max_health: 1`. The classic ladder is already the
//! engine's armor precedence — cinder beacon, then star wand, then the body —
//! so authoring one point of health is the whole rule: whatever she is wearing
//! absorbs the hit, and when there is nothing left the next one is fatal. No
//! demo system reads "is she small"; being small IS having spent the armor.
//!
//! ## The beat is ENGINE vocabulary now (ADR 0033)
//!
//! all six are gone, and Mary-O states a rule instead. The engine holds
//! the body out of play for the interlude, plays the death row, refuses hits,
//! blanks control, and asks the ROSTER whether the level goes back. She dies
//! where she died because nothing moves her — there is nothing left to pin.
//!
//! What remains here is what is genuinely hers: how long the beat lasts, the
//! music that plays over it, and the lives it costs.

use bevy::prelude::*;

/// How long the level holds on her death before it starts again.
///
/// Sized by the music, not by taste. `mary_o_you_died` is four bars of 2/4 at 150bpm — eight
/// beats, 3.2 seconds — and the last chord is the comic low thud the whole cue exists to land. If
/// the score's tempo or bar count changes, this number changes with it.
///
/// This is the value Mary-O hands the engine as her interlude
/// ([`ambition_platformer2d::combat::death_rules::DeathRules`]).
pub const DEATH_DWELL: f32 = 3.2;

/// Her death has its own music.
///
/// Written into the encounter layer's PRIORITY tier — the same slot a focused
/// fight claims — because that is the one tier that outranks the room's own
/// theme. It is claimed every frame a death interlude is open on her body and
/// released the frame it closes, so the level theme returns on its own with no
/// second system to keep in sync and nothing to leak if the beat is interrupted.
///
/// The track is authorized by Mary-O's audio fragment
/// ([`crate::provider::MARY_O_DEATH_MUSIC_TRACK`]); under provider-relative
/// playback an undeclared id is gated to silence however loudly it is requested.
///
/// it reads the BODY's window, not a level-owned flag. In co-op the
/// interlude belongs to the participant who died, and a level-owned beat could
/// not say which of two players is dying.
pub fn play_death_music(
    dying: Query<&ambition_platformer2d::combat::death_rules::DeathInterlude>,
    music: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
            ambition_platformer2d::encounter::EncounterMusicRequest,
        >,
    >,
) {
    let Some(mut music) = music else {
        return;
    };
    if dying.iter().any(|window| window.open()) {
        music.claim_priority(DEATH_MUSIC_OWNER, crate::provider::MARY_O_DEATH_MUSIC_TRACK);
    } else {
        music.release_priority(DEATH_MUSIC_OWNER);
    }
}

/// This beat's claim on the encounter layer's priority music tier.
const DEATH_MUSIC_OWNER: &str = "mary_o_death";

/// Voice the moment she dies.
///
/// The one line of presentation the engine does not owe her: the kernel's death
/// road (a pit, the clock, a hazard tile) never reaches the hit resolver, so
/// nothing plays a cue for it. The music the beat runs over is
/// [`play_death_music`] above.
pub fn voice_her_death(
    mut deaths: MessageReader<ambition_platformer2d::combat::death_rules::ActorDiedMessage>,
    subject: Option<
        bevy::prelude::Res<ambition_platformer2d::platformer::markers::ControlledSubject>,
    >,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
) {
    // filter by VICTIM, not "the last death". This used to take whatever died most recently
    // and apply it to the controlled subject, which is right only while one body can die at
    // all.
    let mine = subject.as_deref().and_then(|subject| subject.0);
    let Some(death) = deaths
        .read()
        .filter(|death| mine.is_none_or(|body| death.victim == body))
        .last()
        .cloned()
    else {
        return;
    };
    match mine {
        // H2: hers. The cue belongs to the body that died.
        Some(body) => sfx.write_for(
            body,
            ambition_platformer2d::sfx::SfxMessage::Reset { pos: death.pos },
        ),
        // I3: the COURSE, not the session. A shell host's session provider is the
        // launcher, which does not author this cue.
        None => sfx.write_from(
            crate::provider::MARY_O_EXPERIENCE,
            ambition_platformer2d::sfx::SfxMessage::Reset { pos: death.pos },
        ),
    }
}

#[cfg(test)]
mod tests;
