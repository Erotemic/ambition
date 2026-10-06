//! Compile-time landmark table of every published part flipbook that tracks a
//! landmark, `(target, bincode)` sorted by target, and the one digest that
//! covers them all. `build.rs` projects each full-resolution
//! `<target>_parts.ron` to its hands, head and feet
//! (`character::landmarks_published`), so a headless build reads a hand's
//! place with no image and no draw table loaded.
//!
//! ⚠ A landmark is a SIMULATION input (a pet mark, a muzzle), and this table
//! is compiled in, so no world resource holds it. Its identity reaches the
//! content fingerprint as [`BAKED_LANDMARKS_DIGEST`]; see
//! `MechanicalRegistries::baked_landmarks` in the platformer provider.

use std::sync::{Arc, OnceLock};

use ambition_characters::actor::landmarks::{BodyLandmarkTable, Landmark, LandmarkClip, LandmarkFrame};
use bevy::math::Vec2;

use crate::character::landmarks_published::{BakedLandmarks, LANDMARK_TRACKS};

include!(concat!(env!("OUT_DIR"), "/baked_landmarks.rs"));

// A slot of the embedded frame is a slot of the semantic frame.
const _: () = assert!(LANDMARK_TRACKS.len() == Landmark::ALL.len());

/// The flipbook tracks that publish `landmark`: one name for each rig family.
pub fn landmark_tracks(landmark: Landmark) -> &'static [&'static str] {
    LANDMARK_TRACKS[landmark.slot()]
}

/// The semantic table of one embedded table.
pub fn landmark_table(baked: BakedLandmarks) -> BodyLandmarkTable {
    BodyLandmarkTable {
        frame_height: baked.frame_height as f32,
        clips: baked
            .clips
            .into_iter()
            .map(|(name, clip)| {
                let frames = clip
                    .frames
                    .into_iter()
                    .map(|frame| -> LandmarkFrame { frame.map(|point| point.map(|(x, y)| Vec2::new(x, y))) })
                    .collect();
                // The loop statement is the animator's own rule for the row,
                // so the table and the art keep one time.
                let looping = crate::character::row_loops(&name);
                (
                    name,
                    LandmarkClip {
                        looping,
                        frame_duration_s: clip.frame_duration_s,
                        frames,
                    },
                )
            })
            .collect(),
    }
}

/// The landmarks the art of sheet `target` publishes, or `None` when its part
/// flipbook tracks none (or it publishes no flipbook).
///
/// Decoded on first use and shared after: an immutable cache of a
/// compile-time table, as the baked sheet index is.
///
/// # Panics
///
/// When an embedded table does not decode. `build.rs` wrote it with the same
/// schema, so that is a build defect and not a content error.
pub fn body_landmarks(target: &str) -> Option<Arc<BodyLandmarkTable>> {
    static DECODED: OnceLock<Vec<OnceLock<Arc<BodyLandmarkTable>>>> = OnceLock::new();
    let index = BAKED_LANDMARKS
        .binary_search_by(|(name, _)| (*name).cmp(target))
        .ok()?;
    let decoded = DECODED.get_or_init(|| BAKED_LANDMARKS.iter().map(|_| OnceLock::new()).collect());
    Some(
        decoded[index]
            .get_or_init(|| {
                let baked: BakedLandmarks = bincode::deserialize(BAKED_LANDMARKS[index].1)
                    .unwrap_or_else(|error| panic!("the embedded landmark table `{target}`: {error}"));
                Arc::new(landmark_table(baked))
            })
            .clone(),
    )
}

/// Every target that publishes a landmark table.
pub fn baked_landmark_targets() -> impl Iterator<Item = &'static str> {
    BAKED_LANDMARKS.iter().map(|(name, _)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::landmarks_published::{landmark_frame, landmark_tables_digest, BakedLandmarkClip};
    use crate::character::rigged::RiggedSpriteAsset;

    #[test]
    fn each_landmark_is_published_by_the_tracks_of_its_name() {
        assert_eq!(landmark_tracks(Landmark::HandNear), ["near_hand", "front_hand", "right_hand"]);
        assert_eq!(landmark_tracks(Landmark::HandFar), ["far_hand", "back_hand", "left_hand"]);
        assert_eq!(landmark_tracks(Landmark::Head), ["head"]);
        assert_eq!(landmark_tracks(Landmark::FootNear), ["near_foot", "front_foot"]);
        assert_eq!(landmark_tracks(Landmark::FootFar), ["far_foot", "back_foot"]);
    }

    /// The three rig families name one thing: the near hand is the hand drawn
    /// nearer the viewer, in every frame of every flipbook that draws both.
    /// A family whose names meant the other thing fails here.
    #[test]
    fn the_near_hand_of_every_family_is_drawn_after_the_far_hand() {
        let mut frames = 0;
        for target in baked_landmark_targets() {
            let flipbook = RiggedSpriteAsset::baked(target).expect("a landmark table has a flipbook");
            let drawn = |draws: &[crate::character::rigged::PartDraw], landmark: Landmark| {
                draws.iter().position(|draw| {
                    draw.track.is_some_and(|track| {
                        landmark_tracks(landmark).contains(&flipbook.tracks[usize::from(track)].as_str())
                    })
                })
            };
            let rows: Vec<&str> = flipbook.clip_names().collect();
            for row in rows {
                let count = flipbook.clip(row).expect("a named clip").frame_count();
                for index in 0..count {
                    let draws = flipbook.frame(row, index).expect("a frame of the clip");
                    if let (Some(near), Some(far)) = (drawn(draws, Landmark::HandNear), drawn(draws, Landmark::HandFar)) {
                        assert!(near > far, "{target}/{row}/{index}: the near hand is drawn under the far hand");
                        frames += 1;
                    }
                }
            }
        }
        if BAKED_LANDMARKS.is_empty() {
            eprintln!("no published part flipbook on this checkout: no draw order to compare");
        } else {
            assert!(frames > 1000, "only {frames} frames draw both hands");
        }
    }

    /// The frame the visual animator shows for pose `anim` of `target`,
    /// `lengths` clip lengths after the pose starts, and the frame this
    /// target's landmark table gives for the same row at the same time.
    fn animator_and_table_frames(target: &str, anim: crate::character::CharacterAnim, row: &str, lengths: f32) -> (usize, usize) {
        use crate::character::sheets::{try_load_spec_for_target, SheetTuning};
        use crate::character::{CharacterAnimator, CharacterSpriteAsset};
        let asset = CharacterSpriteAsset {
            texture: Default::default(),
            layout: Default::default(),
            spec: try_load_spec_for_target(target, &SheetTuning::new(1.0, 1)).expect("a published sheet"),
            pages: Vec::new(),
            requested_tier: Default::default(),
            resolved_tier: Default::default(),
            rigged: None,
        };
        let mut animator = CharacterAnimator::new(&asset);
        // Start from another pose, so the request below starts the row.
        animator.request(crate::character::CharacterAnim::Walk);
        animator.request(anim);
        assert_eq!(animator.current, anim, "{target} has its own `{row}` row");
        let sheet_row = animator.spec.row(anim).clone();
        let table = body_landmarks(target).expect("a landmark table");
        let clip = table.clip(row).unwrap_or_else(|| panic!("{target} publishes no `{row}` clip"));
        // The premise: the two keep one clock for this row.
        assert_eq!(sheet_row.frame_count, clip.frames.len(), "{target}/{row}: frame counts");
        assert!(
            (sheet_row.duration_secs - clip.frame_duration_s).abs() < 1e-6,
            "{target}/{row}: the sheet shows a frame for {} s and the table for {} s",
            sheet_row.duration_secs,
            clip.frame_duration_s,
        );
        // Eight steps a frame, to the middle of a frame: no step is on a
        // frame edge.
        let steps = (lengths * clip.frames.len() as f32 * 8.0).floor() as usize + 4;
        let dt = clip.frame_duration_s / 8.0;
        for _ in 0..steps {
            animator.tick(dt);
        }
        (animator.frame, clip.frame_at_time(steps as f32 * dt))
    }

    /// A package landmark clip keeps the time of the row it describes. A row
    /// the animator loops wraps, and a row the animator holds on its last
    /// frame holds: more than two clip lengths in, the two show one frame.
    ///
    /// The table always wrapped. After a one-shot row ended, the sprite held
    /// its last frame and the hand of the landmark table went back to the
    /// first.
    #[test]
    fn a_landmark_clip_loops_or_holds_as_the_animator_shows_its_row() {
        use crate::character::CharacterAnim;
        if BAKED_LANDMARKS.is_empty() {
            eprintln!("no published part flipbook on this checkout: no clip to compare");
            return;
        }
        const TARGET: &str = "player_robot_v3";
        let (shown, table) = animator_and_table_frames(TARGET, CharacterAnim::Idle, "idle", 2.3);
        assert_eq!(table, shown, "the looping `idle` row, 2.3 lengths in");
        let (shown, table) = animator_and_table_frames(TARGET, CharacterAnim::Shoot, "shoot", 2.3);
        let last = body_landmarks(TARGET).expect("a table").clip("shoot").expect("the clip").frames.len() - 1;
        assert_eq!(shown, last, "control: the animator holds the one-shot `shoot` row on its last frame");
        assert_eq!(table, shown, "the one-shot `shoot` row, 2.3 lengths in");
    }

    #[test]
    fn a_frame_takes_the_first_draw_of_each_landmark_track_and_no_other_track() {
        let frame = landmark_frame([
            ("torso", (9.0, 9.0)),
            ("near_hand", (1.0, -2.0)),
            ("near_hand", (5.0, -6.0)),
            ("head", (3.0, -40.0)),
        ]);
        assert_eq!(frame[Landmark::HandNear.slot()], Some((1.0, -2.0)));
        assert_eq!(frame[Landmark::Head.slot()], Some((3.0, -40.0)));
        assert_eq!(frame[Landmark::HandFar.slot()], None);
        // Another family's names fill the same slots.
        let frame = landmark_frame([("back_hand", (7.0, -8.0)), ("front_hand", (9.0, -10.0)), ("left_hand", (0.0, 0.0))]);
        assert_eq!(frame[Landmark::HandNear.slot()], Some((9.0, -10.0)));
        assert_eq!(frame[Landmark::HandFar.slot()], Some((7.0, -8.0)));
    }

    /// The embedded table is `build.rs`'s projection of the flipbook. This
    /// derives the same points a second way, from the draw table the renderer
    /// decodes, for every target and every frame.
    #[test]
    fn every_embedded_table_holds_the_tracked_points_of_its_flipbook() {
        let mut points = 0;
        for target in baked_landmark_targets() {
            let table = body_landmarks(target).expect("a listed target decodes");
            let flipbook = RiggedSpriteAsset::baked(target).expect("a landmark table has a flipbook");
            assert_eq!(table.frame_height, flipbook.frame_size.y as f32, "{target}");
            for (name, clip) in &table.clips {
                assert!(!name.contains('~'), "{target}: mirror row `{name}` was embedded");
                let drawn = flipbook.clip(name).unwrap_or_else(|| panic!("{target}: no clip `{name}`"));
                assert_eq!(clip.frames.len(), drawn.frame_count(), "{target}/{name}");
                assert_eq!(clip.frame_duration_s, drawn.frame_duration_s, "{target}/{name}");
                for (index, frame) in clip.frames.iter().enumerate() {
                    let draws = flipbook.frame(name, index).expect("a frame of the clip");
                    for landmark in Landmark::ALL {
                        let expected = draws
                            .iter()
                            .find(|draw| {
                                draw.track.is_some_and(|track| {
                                    landmark_tracks(landmark).contains(&flipbook.tracks[usize::from(track)].as_str())
                                })
                            })
                            .map(|draw| draw.at);
                        assert_eq!(frame[landmark.slot()], expected, "{target}/{name}/{index}/{landmark:?}");
                        points += usize::from(expected.is_some());
                    }
                }
            }
        }
        // A checkout with no published flipbook embeds nothing, and then this
        // arm compared nothing. Say so; do not pass in silence.
        if BAKED_LANDMARKS.is_empty() {
            eprintln!("no published part flipbook on this checkout: no embedded landmark table to compare");
        } else {
            assert!(points > 1000, "only {points} landmark points in {} tables", BAKED_LANDMARKS.len());
        }
    }

    #[test]
    fn the_digest_is_of_the_embedded_tables_and_moves_with_one_point() {
        assert_eq!(
            BAKED_LANDMARKS_DIGEST,
            landmark_tables_digest(BAKED_LANDMARKS.iter().copied()),
            "the digest `build.rs` wrote is not the digest of the tables it embedded",
        );
        let table = |hand_x: f32| {
            let mut frame = crate::character::landmarks_published::BakedLandmarkFrame::default();
            frame[Landmark::HandNear.slot()] = Some((hand_x, -30.0));
            bincode::serialize(&BakedLandmarks {
                frame_height: 64,
                clips: [(
                    "pet".to_string(),
                    BakedLandmarkClip {
                        frame_duration_s: 0.125,
                        frames: vec![frame],
                    },
                )]
                .into(),
            })
            .expect("the table encodes")
        };
        let (here, there) = (table(8.0), table(8.25));
        let digest = |bytes: &Vec<u8>, key: &'static str| landmark_tables_digest([(key, bytes.as_slice())]);
        assert_eq!(digest(&here, "dog"), digest(&here.clone(), "dog"));
        assert_ne!(digest(&here, "dog"), digest(&there, "dog"), "one hand moved a quarter pixel");
        assert_ne!(digest(&here, "dog"), digest(&here, "cat"), "the same table under another target");
    }
}
