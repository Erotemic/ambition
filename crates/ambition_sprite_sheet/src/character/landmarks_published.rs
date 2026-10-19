//! The embedded landmark table schema, and nothing else: serde types without
//! a dependency on the crate, so `build.rs` includes this same file to write
//! the bincode the game decodes ([`crate::baked_landmarks`]). One definition
//! of the schema for both.
//!
//! A landmark table is the SIMULATION's part of a published part flipbook
//! (`<target>_parts.ron`): where the tracked hands, head and feet are in each
//! frame of each clip. It holds no part, page or draw.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The flipbook tracks that publish each landmark slot, in slot order. The
/// order is `ambition_characters::actor::Landmark::ALL`.
///
/// A rig family names its limbs in one of three ways, and they mean one
/// thing. Measured on the published flipbooks (2026-10-05, mirror rows not
/// counted): in every frame that draws both hands, the `near_`, `front_` or
/// `right_` hand is drawn after the `far_`, `back_` or `left_` one (11195,
/// 1075 and 554 frames, no exception), so it is the hand nearer the viewer.
/// The art faces right, so a character's own right hand is the near one. A
/// test holds the draw order for the embedded tables (`baked_landmarks`).
///
/// ⚠ Nearer the viewer is not the hand that reaches: see
/// `ambition_characters::actor::GestureHand`.
pub const LANDMARK_TRACKS: [&[&str]; 5] = [
    &["near_hand", "front_hand", "right_hand"],
    &["far_hand", "back_hand", "left_hand"],
    &["head"],
    &["near_foot", "front_foot"],
    &["far_foot", "back_foot"],
];

/// The landmark points of one frame, by slot, in sheet pixels from the feet.
pub type BakedLandmarkFrame = [Option<(f32, f32)>; LANDMARK_TRACKS.len()];

/// The flipbook tracks that publish a BOX, in slot order: the bounds of what
/// the track draws in each frame, for a gameplay volume that follows the art
/// (a shield that turns strikes aside). The order is
/// `ambition_characters::actor::PartBox::ALL`.
pub const BOX_TRACKS: [&str; 1] = ["shield"];

/// The boxes of one frame, by slot: `(min_x, min_y, max_x, max_y)` in sheet
/// pixels from the feet, +y down. `None` when the frame does not draw the
/// track.
pub type BakedBoxFrame = [Option<(f32, f32, f32, f32)>; BOX_TRACKS.len()];

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BakedLandmarkClip {
    pub frame_duration_s: f32,
    pub frames: Vec<BakedLandmarkFrame>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BakedLandmarks {
    /// The height of one sheet frame, in sheet pixels.
    pub frame_height: u32,
    pub clips: BTreeMap<String, BakedLandmarkClip>,
    /// Each clip's boxes, frame by frame ([`BOX_TRACKS`]). A clip whose art
    /// draws no box track is absent.
    pub boxes: BTreeMap<String, Vec<BakedBoxFrame>>,
}

/// The landmark slots of one frame, from its tracked draws.
///
/// `tracked` gives each draw's track name and its place in sheet pixels from
/// the feet, in draw order. The first draw of a track is that track's place.
pub fn landmark_frame<'a>(tracked: impl IntoIterator<Item = (&'a str, (f32, f32))>) -> BakedLandmarkFrame {
    let mut frame = BakedLandmarkFrame::default();
    for (track, at) in tracked {
        if let Some(slot) = LANDMARK_TRACKS.iter().position(|names| names.contains(&track)) {
            frame[slot].get_or_insert(at);
        }
    }
    frame
}

/// The boxes of one frame, from its tracked draws.
///
/// `tracked` gives each draw's track name and the corners of its quad in
/// sheet pixels from the feet. A box is the bounds of every draw of its track.
pub fn box_frame<'a>(tracked: impl IntoIterator<Item = (&'a str, [(f32, f32); 4])>) -> BakedBoxFrame {
    let mut frame = BakedBoxFrame::default();
    for (track, corners) in tracked {
        let Some(slot) = BOX_TRACKS.iter().position(|name| *name == track) else {
            continue;
        };
        for (x, y) in corners {
            let (x0, y0, x1, y1) = frame[slot].get_or_insert((x, y, x, y));
            *x0 = x0.min(x);
            *y0 = y0.min(y);
            *x1 = x1.max(x);
            *y1 = y1.max(y);
        }
    }
    frame
}

/// The identity of a set of embedded tables: BLAKE3 over each table's key and
/// bytes, in the order given, each with its length so that no table's bytes
/// can read as another's key. `build.rs` writes it beside the tables it
/// covers, and the content fingerprint carries it.
pub fn landmark_tables_digest<'a>(tables: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> String {
    let mut hasher = blake3::Hasher::new();
    for (key, encoded) in tables {
        hasher.update(&(key.len() as u64).to_le_bytes());
        hasher.update(key.as_bytes());
        hasher.update(&(encoded.len() as u64).to_le_bytes());
        hasher.update(encoded);
    }
    hasher.finalize().to_hex().to_string()
}
