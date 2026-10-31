//! Play each body's notice and windup sounds from its session's prepared cast.
//! The decision frame names this tick's beats. Strike sounds belong to moves.

use ambition_characters::session_cast::SessionCast;
use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::BrainBeat;
use ambition_platformer2d_core as ae;
use ambition_sfx::{BodySfxWriter, SfxId, SfxMessage};
use bevy::prelude::*;

use super::ActorDecisionFrames;

/// Play the cue of each beat a brain began this tick. (sim)
pub fn play_beat_cues(
    decisions: Res<ActorDecisionFrames>,
    cast: SessionCast,
    bodies: Query<(&WornCharacter, &ae::BodyKinematics)>,
    mut sfx: BodySfxWriter,
) {
    let Some(cast) = cast.get() else { return };
    for &(body, beat) in decisions.beats() {
        let Ok((worn, kin)) = bodies.get(body) else { continue };
        let Some(character) = cast.get(worn.id()) else { continue };
        let cues = &character.cues;
        let cue = match beat {
            BrainBeat::Notice => &cues.notice,
            BrainBeat::Tell => &cues.tell,
        };
        if let Some(cue) = cue {
            sfx.write_for(body, SfxMessage::Play { id: SfxId::new(cue), pos: kin.pos });
        }
    }
}
