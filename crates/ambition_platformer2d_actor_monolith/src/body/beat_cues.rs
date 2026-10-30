//! A creature's beats heard as well as seen (Jon, 2026-10-30: "Do the enemies
//! have sound effects to help their telegraph and for their attacks?"): the
//! notice of a foe, and a windup its brain times (a volley's, the hang before
//! a dive), each play its character's cue (the catalog's `cues`) on the tick
//! the beat begins. The sounds of its strikes are its moves' own events
//! (`moveset_prefabs::voice_moveset`).
//!
//! The beats are this tick's decisions ([`ActorDecisionFrames::beats`]), read
//! off rollback state, so a resimulated tick plays what the first one did.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::BrainBeat;
use ambition_platformer2d_core as ae;
use ambition_sfx::{BodySfxWriter, SfxId, SfxMessage};
use bevy::prelude::*;

use super::ActorDecisionFrames;

/// Play the cue of each beat a brain began this tick. (sim)
pub fn play_beat_cues(
    decisions: Res<ActorDecisionFrames>,
    catalog: Option<Res<CharacterCatalog>>,
    bodies: Query<(&WornCharacter, &ae::BodyKinematics)>,
    mut sfx: BodySfxWriter,
) {
    let Some(catalog) = catalog else { return };
    for &(body, beat) in decisions.beats() {
        let Ok((worn, kin)) = bodies.get(body) else { continue };
        let Some(cues) = catalog.cues(worn.id()) else { continue };
        let cue = match beat {
            BrainBeat::Notice => &cues.notice,
            BrainBeat::Tell => &cues.tell,
        };
        if let Some(cue) = cue {
            sfx.write_for(body, SfxMessage::Play { id: SfxId::new(cue), pos: kin.pos });
        }
    }
}
