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

/// Read a published part flipbook with the schema the crate reads it with.
pub fn read_part_flipbook(text: &str) -> Result<super::published::Published, ron::error::SpannedError> {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
        .from_str(text)
}

/// The landmark table of one published part flipbook: the place of each
/// landmark track in each frame of each clip. `None` when the flipbook tracks
/// no landmark, so that such a target has no table.
///
/// A mirror row (`<row>~mirrored`) is not read: a body that faces the other
/// way mirrors the row's points about its feet.
pub fn landmarks_of(published: &super::published::Published) -> Option<super::landmarks_published::BakedLandmarks> {
    use super::landmarks_published as lp;
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
        let frames: Vec<lp::BakedLandmarkFrame> = clip
            .frames
            .iter()
            .map(|frame| lp::landmark_frame(frame.iter().filter_map(|draw| track_name(draw.track).map(|name| (name.as_str(), draw.at)))))
            .collect();
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
