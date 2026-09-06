//! Does a generated character actually SPEAK on its Hall pedestal?
//!
//! The unit tests prove the catalog resolves a line for a character whose sprite
//! target shipped only suggested dialogue. That is not the same claim as "the
//! player hears it": between the catalog and a speech bubble sit the LDtk spawn's
//! `character_id`, the ambient ticker's component filter, the gallery flag, and a
//! ~28s timer. A character standing silently on a pedestal is invisible to every
//! test that stops at the catalog, which is exactly how this was reported.
//!
//! So this drives the REAL Hall room and collects what reaches the VFX bus.

use ambition_platformer2d::vfx::vfx::VfxMessage;
use bevy::prelude::Messages;

/// The Hall bark cadence is ~28s base plus up to 24s of per-pedestal jitter, so
/// any window shorter than a minute proves nothing either way. 90s is comfortably
/// past the worst case while staying a cheap headless run.
const TICKS: usize = 60 * 90;

fn speech_in_the_hall() -> Vec<String> {
    let mut sim = crate::common::fixed_60hz_room_sim("hall_of_characters");
    let mut heard = Vec::new();
    for _ in 0..TICKS {
        sim.step(crate::common::base());
        let world = sim.world_mut();
        if let Some(mut messages) = world.get_resource_mut::<Messages<VfxMessage>>() {
            for message in messages.drain() {
                if let VfxMessage::SpeechBubble { text, .. } = message {
                    heard.push(text);
                }
            }
        }
    }
    heard
}

#[test]
fn a_generated_character_speaks_its_suggested_lines_on_its_pedestal() {
    let heard = speech_in_the_hall();
    assert!(
        !heard.is_empty(),
        "no pedestal said anything in {} sim-seconds -- the ambient ticker is not \
         reaching Hall NPCs at all",
        TICKS / 60
    );
    // Marie Curry authored `dialogue_hints.suggested_barks` on her sprite target
    // and NO per-situation pools, so every line she has comes through the
    // fallback path. If the Hall is talking but she is not, the break is in that
    // path and not in the ticker.
    let curry = [
        "Careful, it is still reactive.",
        "Let it simmer.",
        "A measured dose.",
        "Observe the glow.",
        "That mixture was unstable anyway.",
        "The spice is luminous today.",
    ];
    assert!(
        heard.iter().any(|line| curry.contains(&line.as_str())),
        "the Hall spoke {} time(s) but Marie Curry never did. Heard: {:?}",
        heard.len(),
        {
            let mut sample: Vec<&String> = heard.iter().take(12).collect();
            sample.dedup();
            sample
        }
    );
}
