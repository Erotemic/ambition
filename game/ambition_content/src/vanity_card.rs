//! Ambition's startup vanity card: the authored "I MADE THIS" comic beat.
//!
//! The pictures are generated (`tools/vanity_card_prep/export_sequence.py`) and
//! git-ignored like every other rendered payload. The MANIFEST beside them is
//! committed, because it is the contract the host composes the card from — and
//! because a checkout without the payload can then still play the card at the
//! right length, naming each absent frame in its own slot.

use std::time::Duration;

use serde::Deserialize;

pub const VANITY_CARD_RON: &str = include_str!("../assets/data/vanity_card.ron");

/// The `game://` asset source is the content crate's own `assets/` tree, which
/// is where the exporter writes the frames.
const ASSET_SOURCE: &str = "game://";

#[derive(Debug, Deserialize)]
struct VanityCardManifest {
    frames: Vec<VanityCardFrame>,
}

#[derive(Debug, Deserialize)]
struct VanityCardFrame {
    path: String,
    hold_ms: u64,
}

/// The card's frames as `(asset path, hold)`, in play order.
///
/// Frames repeat by design — a picture shown before and after its speech bubble
/// is ONE file addressed twice, which is why the sequence ships nine images for
/// twelve beats.
pub fn vanity_card_frames() -> Vec<(String, Duration)> {
    let manifest: VanityCardManifest = ron::from_str(VANITY_CARD_RON)
        .expect("vanity_card.ron is generated and compiled in; it must parse");
    manifest
        .frames
        .into_iter()
        .map(|frame| {
            (
                format!("{ASSET_SOURCE}{}", frame.path),
                Duration::from_millis(frame.hold_ms),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed manifest must parse with the REAL ron parser, not just a
    /// Python writer's idea of the format.
    #[test]
    fn the_committed_manifest_parses() {
        let frames = vanity_card_frames();
        assert!(!frames.is_empty(), "the vanity card must have frames");
        assert!(
            frames
                .iter()
                .all(|(path, _)| path.starts_with(ASSET_SOURCE)),
            "every frame must address the content crate's asset source",
        );
        assert!(
            frames.iter().all(|(_, hold)| !hold.is_zero()),
            "a zero-length frame would be invisible",
        );
    }

    /// The whole point of per-frame holds: the card is a few seconds of screen
    /// time, not a frame count.
    #[test]
    fn the_card_runs_for_a_plausible_startup_duration() {
        let total: Duration = vanity_card_frames().iter().map(|(_, hold)| *hold).sum();
        assert!(
            total > Duration::from_secs(2) && total < Duration::from_secs(8),
            "unexpected vanity card length: {total:?}",
        );
    }
}
