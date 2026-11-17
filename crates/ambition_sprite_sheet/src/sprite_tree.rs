//! Sprite quality keys and the geometry projection of part tables.

use std::path::Path;

pub const TIERS: [(&str, &str); 3] = [("sprites_0_5x", "0_5x"), ("sprites_0_25x", "0_25x"), ("sprites_potato", "potato")];

/// The key of a sheet whose file root is `root`: the root, and the tier
/// marker of a tier folder (`<root>.<marker>`).
pub fn sheet_key(root: &str, path: &Path) -> String {
    let marker = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .find_map(|component| TIERS.iter().find(|(dir, _)| *dir == component).map(|(_, marker)| *marker));
    match marker {
        Some(marker) => format!("{root}.{marker}"),
        None => root.to_owned(),
    }
}

/// The landmark table of one published part flipbook: the place of each
/// landmark track in each frame of each clip. `None` when the flipbook tracks
/// no landmark, so that such a target has no table.
///
/// A mirror row (`<row>~mirrored`) is not read: a body that faces the other
/// way mirrors the row's points about its feet.
pub fn landmarks_of(published: &super::published::Published) -> Option<super::landmarks_published::BakedLandmarks> {
    use super::landmarks_published as lp;
    published.validate_landmarks().ok()?;
    let track_name = |track: Option<u16>| track.and_then(|track| published.tracks.get(usize::from(track)));
    // The four corners of a draw's quad, from the feet: the part turned
    // (radians, clockwise with +y down) and scaled about its pivot, its pivot
    // at `at`. A table at another texel scale states its rect in its texels.
    let corners = |draw: &super::published::PublishedDraw| -> Option<[(f32, f32); 4]> {
        let part = published.parts.get(usize::from(draw.part))?;
        let scale = published.texel_scale.max(f32::EPSILON);
        let (w, h) = (part.rect.2 as f32 / scale, part.rect.3 as f32 / scale);
        let (px, py) = (part.pivot.0 / scale, part.pivot.1 / scale);
        let (c, s) = (draw.rotation.cos(), draw.rotation.sin());
        Some([(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)].map(|(x, y)| {
            let (lx, ly) = ((x - px) * draw.scale.0, (y - py) * draw.scale.1);
            (draw.at.0 + c * lx - s * ly, draw.at.1 + s * lx + c * ly)
        }))
    };
    let mut clips = std::collections::BTreeMap::new();
    let mut boxes = std::collections::BTreeMap::new();
    let mut any = false;
    for (name, clip) in &published.clips {
        if name.contains('~') {
            continue;
        }
        let mut frames: Vec<lp::BakedLandmarkFrame> = clip
            .frames
            .iter()
            .map(|frame| lp::landmark_frame(frame.iter().filter_map(|draw| track_name(draw.track).map(|name| (name.as_str(), draw.at)))))
            .collect();
        if let Some(points) = published.landmarks.get(name) {
            for (frame, points) in frames.iter_mut().zip(points) {
                for (point, at) in points {
                    if let Some(slot) = lp::LANDMARK_NAMES.iter().position(|name| *name == point) {
                        frame[slot] = Some(*at);
                    }
                }
            }
        }
        any |= frames.iter().flatten().any(Option::is_some);
        let framed: Vec<lp::BakedBoxFrame> = clip
            .frames
            .iter()
            .map(|frame| {
                lp::box_frame(frame.iter().filter_map(|draw| Some((track_name(draw.track)?.as_str(), corners(draw)?))))
            })
            .collect();
        if framed.iter().flatten().any(Option::is_some) {
            any = true;
            boxes.insert(name.clone(), framed);
        }
        clips.insert(
            name.clone(),
            lp::BakedLandmarkClip {
                frame_duration_s: clip.frame_duration_s,
                frames,
            },
        );
    }
    any.then_some(lp::BakedLandmarks {
        frame_height: published.frame_size.1,
        clips,
        boxes,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn named_points_keep_their_authored_pose_without_a_draw_track() {
        let text = r#"(
            schema_version: 4, target: "eyes", pages: [], frame_size: (32, 32), feet_pixel: (16.0, 32.0), parts: [],
            clips: { "crouch": (frame_duration_s: 0.1, frames: [[], []]) },
            landmarks: { "crouch": [{ "eye": (8.0, -12.0) }, { "eye": (9.0, -13.0) }] },
        )"#;
        let published = ambition_sprite_part_format::read_ron(text).unwrap();
        let projected = super::landmarks_of(&published).unwrap();
        let eye = super::super::landmarks_published::LANDMARK_NAMES.iter().position(|name| *name == "eye").unwrap();
        assert_eq!(projected.clips["crouch"].frames[0][eye], Some((8.0, -12.0)));
        assert_eq!(projected.clips["crouch"].frames[1][eye], Some((9.0, -13.0)));
        let bad = ambition_sprite_part_format::read_ron(&text.replace("[], []", "[]")).unwrap();
        assert!(super::landmarks_of(&bad).is_none(), "a missing frame cannot shift a point onto another pose");
    }
}
