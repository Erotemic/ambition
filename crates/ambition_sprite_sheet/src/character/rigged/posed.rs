//! Parts placed by a body pose: the seam through which a pose that is not the
//! flipbook's (a ragdoll, a procedural reach) moves the same parts
//! (`docs/planning/engine/semantic-part-rendering-and-ragdolls.md`, phases 1
//! and 7).
//!
//! A character that publishes both a part flipbook and a semantic body rig
//! states its pose twice: as joint frames (the simulation's) and as one
//! transform per part per frame (the renderer's). They are ONE decomposition
//! only where every part rides a joint: its pivot and angle, taken in that
//! joint's frame, the same in every frame. [`PosedParts::bind`] measures that
//! from the two files, frame by frame, and keeps for each track the joint it
//! rides and its place in the joint's frame. [`PosedParts::place`] then puts a
//! frame's draws where any set of joint frames says, so the flipbook's own
//! transform table is a cache of what the rig's pose and these bindings derive.

use std::collections::{BTreeMap, BTreeSet};

use bevy::math::{Affine2, Mat2, Vec2};

use ambition_characters::actor::PreparedBodyRig;

use super::{PartDraw, RiggedSpriteAsset};

/// A track bound to the joint it rides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackBinding {
    /// The joint's index in the body rig.
    pub joint: u16,
    /// The largest placement error in sheet pixels over the shared frames, of
    /// any of its rasters. Each joint's transform converts the local error to
    /// sheet pixels.
    pub spread_px: f32,
    /// The same for the angle, in radians.
    pub spread_rad: f32,
}

/// One raster of a bound track in its joint's frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartBinding {
    /// The draw's pivot in the joint's frame, in sheet pixels (+y down).
    pub at: Vec2,
    /// The draw's angle less the joint's, in radians (clockwise, +y down).
    pub rotation: f32,
}

/// Every track of a flipbook bound to the body-rig joint it rides, where one
/// does.
///
/// ⛔ The place of a part in its joint's frame is kept PER RASTER, apart for
/// a joint frame that is a mirror image and for a draw that flips its raster.
/// One track draws many rasters (a supersampled rig reduces each subpixel
/// phase of a part to its own), and the mirrored rows of a sheet draw the
/// same raster flipped under a mirrored joint frame, whose angle reads with
/// the opposite sign. One raster, one handedness, one flip: one place.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PosedParts {
    /// By track index; `None` for a track no joint holds within the tolerance
    /// (an effect layer, a part the rig does not name).
    bindings: Vec<Option<TrackBinding>>,
    /// By (track, part, whether the joint frame is a mirror image, whether the
    /// draw flips its raster).
    parts: BTreeMap<(u16, u16, bool, bool), PartBinding>,
    /// The frames both files share that the measurement read.
    frames: usize,
}

#[derive(Clone, Copy)]
struct BindingSample {
    part: u16,
    mirrored: bool,
    flipped: bool,
    at: Vec2,
    rotation: f32,
    basis: Mat2,
}

/// The mean place of `samples` and how far the farthest is from it.
fn measured<'a>(samples: impl Iterator<Item = &'a BindingSample> + Clone) -> Option<(PartBinding, f32, f32)> {
    let n = samples.clone().count() as f32;
    let reference = samples.clone().next()?.rotation;
    let at = samples.clone().map(|sample| sample.at).sum::<Vec2>() / n;
    let rotation = reference + samples.clone().map(|sample| wrapped(sample.rotation - reference)).sum::<f32>() / n;
    let spread_px = samples.clone().map(|sample| (sample.basis * (sample.at - at)).length()).fold(0.0, f32::max);
    let spread_rad = samples.map(|sample| wrapped(sample.rotation - rotation).abs()).fold(0.0, f32::max);
    Some((PartBinding { at, rotation: wrapped(rotation) }, spread_px, spread_rad))
}

/// A joint frame's angle, in radians: the `rotation` of the pose that built
/// it. ⛔ Read off the Y AXIS. A pose mirrors a joint by a negative x scale
/// (Mary-O's head faces back in one frame), which turns the x axis half a turn
/// and leaves the y axis where the rotation put it; read off the x axis, that
/// one frame put the head's angle half a turn from every other and the head
/// rode no joint.
fn angle_of(frame: &Affine2) -> f32 {
    let y = frame.matrix2.y_axis;
    (-y.x).atan2(y.y)
}

/// Whether a joint frame is a mirror image (a negative scale on one axis).
fn mirrored(frame: &Affine2) -> bool {
    frame.matrix2.determinant() < 0.0
}

/// `angle` wrapped into `(-PI, PI]`.
fn wrapped(angle: f32) -> f32 {
    let turned = (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    if turned <= -std::f32::consts::PI {
        turned + std::f32::consts::TAU
    } else {
        turned
    }
}

impl PosedParts {
    /// [`Self::bind_named`] with no joint named by the art.
    pub fn bind(flipbook: &RiggedSpriteAsset, rig: &PreparedBodyRig, tolerance_px: f32, tolerance_rad: f32) -> Self {
        Self::bind_named(flipbook, rig, &BTreeMap::new(), tolerance_px, tolerance_rad)
    }

    /// Bind each track of `flipbook` to the joint of `rig` it rides, measured
    /// over every frame of every clip the two share (same name, same frame
    /// count).
    ///
    /// The joint is the one `named` gives the track's name (the rig's
    /// `part_bindings`, from the art that drew it); for a track it does not
    /// name, the joint whose frame holds the track's pivot and angle stillest.
    /// Each raster of the track then has its place in that joint's frame. A
    /// track binds when every raster's pivot spreads no more than
    /// `tolerance_px` sheet pixels and its angle no more than `tolerance_rad`
    /// (an unnamed track: the whole track, as one place).
    pub fn bind_named(
        flipbook: &RiggedSpriteAsset,
        rig: &PreparedBodyRig,
        named: &BTreeMap<String, String>,
        tolerance_px: f32,
        tolerance_rad: f32,
    ) -> Self {
        let joints = rig.joint_names().len();
        let mut samples: Vec<Vec<Vec<BindingSample>>> = vec![vec![Vec::new(); joints]; flipbook.tracks.len()];
        let mut frames = 0;
        let mut world = Vec::new();
        for row in flipbook.clip_names() {
            let (Some(clip), Some(rig_clip)) = (flipbook.clip(row), rig.clip(row)) else {
                continue;
            };
            if clip.frame_count() != rig_clip.frames.len() {
                continue;
            }
            for index in 0..clip.frame_count() {
                if !rig.solve(row, index, &mut world) || world.len() != joints {
                    continue;
                }
                let Some(draws) = flipbook.frame(row, index) else {
                    continue;
                };
                frames += 1;
                for draw in draws {
                    let Some(track) = draw.track.map(usize::from).filter(|track| *track < samples.len()) else {
                        continue;
                    };
                    for (joint, frame) in world.iter().enumerate() {
                        let local = frame.inverse().transform_point2(draw.at);
                        samples[track][joint].push(BindingSample {
                            part: draw.part,
                            mirrored: mirrored(frame),
                            flipped: draw.scale.x < 0.0,
                            at: local,
                            rotation: wrapped(draw.rotation - angle_of(frame)),
                            basis: frame.matrix2,
                        });
                    }
                }
            }
        }
        let within = |spread_px: f32, spread_rad: f32| spread_px <= tolerance_px && spread_rad <= tolerance_rad;
        let mut bindings = Vec::with_capacity(samples.len());
        let mut parts = BTreeMap::new();
        for (track, by_joint) in samples.iter().enumerate() {
            let named_joint = flipbook.tracks.get(track)
                .and_then(|name| named.get(name))
                .and_then(|joint| rig.joint_names().iter().position(|known| known == joint));
            // The joint: the art's, or the stillest within the tolerance.
            let joint = match named_joint {
                Some(joint) => Some(joint).filter(|joint| !by_joint[*joint].is_empty()),
                None => by_joint
                    .iter()
                    .enumerate()
                    .filter_map(|(joint, samples)| measured(samples.iter()).map(|(_, px, rad)| (joint, px, rad)))
                    .filter(|(_, px, rad)| within(*px, *rad))
                    .min_by(|a, b| (a.1 + a.2.to_degrees() * 0.1).total_cmp(&(b.1 + b.2.to_degrees() * 0.1)))
                    .map(|(joint, _, _)| joint),
            };
            let Some(joint) = joint else {
                bindings.push(None);
                continue;
            };
            // Each raster, under each handedness and flip, in that joint's
            // frame.
            let keys: BTreeSet<(u16, bool, bool)> =
                by_joint[joint].iter().map(|sample| (sample.part, sample.mirrored, sample.flipped)).collect();
            let mut binding = TrackBinding { joint: joint as u16, spread_px: 0.0, spread_rad: 0.0 };
            let mut placed = Vec::with_capacity(keys.len());
            for (part, mirror, flip) in keys {
                let of_key: Vec<&BindingSample> = by_joint[joint]
                    .iter()
                    .filter(|sample| sample.part == part && sample.mirrored == mirror && sample.flipped == flip)
                    .collect();
                // ⛔ A NAMED track's raster can sit in its joint's frame in
                // more than one place, each a true picture of the part. The
                // flipbook draws one raster for every raster that is a
                // lossless transform of it (a half turn of Scout's oval visor
                // or capsule shin, about the middle of its drawn extent), and
                // the art says the part is rigid on that joint, so a second
                // place can only be such a transform. Every place must be
                // tight; the binding is the one most frames drew. An unnamed
                // track has no such word from the art: one place.
                let groups: Vec<Vec<&BindingSample>> = if named_joint.is_some() {
                    let mut groups: Vec<Vec<&BindingSample>> = Vec::new();
                    for sample in of_key {
                        let near = |group: &&mut Vec<&BindingSample>| {
                            let first = group[0];
                            wrapped(sample.rotation - first.rotation).abs() <= tolerance_rad
                                && (sample.basis * (sample.at - first.at)).length() <= tolerance_px
                        };
                        match groups.iter_mut().find(near) {
                            Some(group) => group.push(sample),
                            None => groups.push(vec![sample]),
                        }
                    }
                    groups
                } else {
                    vec![of_key]
                };
                let mut most: Option<(usize, PartBinding)> = None;
                for group in &groups {
                    let Some((place, px, rad)) = measured(group.iter().copied()) else { continue };
                    binding.spread_px = binding.spread_px.max(px);
                    binding.spread_rad = binding.spread_rad.max(rad);
                    if most.is_none_or(|(count, _)| group.len() > count) {
                        most = Some((group.len(), place));
                    }
                }
                if let Some((_, place)) = most {
                    placed.push(((track as u16, part, mirror, flip), place));
                }
            }
            if within(binding.spread_px, binding.spread_rad) {
                parts.extend(placed);
                bindings.push(Some(binding));
            } else {
                bindings.push(None);
            }
        }
        Self { bindings, parts, frames }
    }

    /// The joint track `track` rides, if one holds it.
    pub fn binding(&self, track: u16) -> Option<&TrackBinding> {
        self.bindings.get(usize::from(track)).and_then(Option::as_ref)
    }

    /// How many of the flipbook's tracks ride a joint, of how many.
    pub fn bound(&self) -> (usize, usize) {
        (self.bindings.iter().filter(|binding| binding.is_some()).count(), self.bindings.len())
    }

    /// The frames the two files share that the binding was measured over.
    pub fn frames_measured(&self) -> usize {
        self.frames
    }

    /// `frame`'s draws placed by `joints` (the rig's joint frames in rig
    /// space: sheet pixels from the feet, +y down), into `out`: each draw of a
    /// bound track at its joint's frame composed with its binding; every other
    /// draw (an effect layer) where the frame put it. The part, its own scale,
    /// opacity and tint, and the draw order are the frame's.
    pub fn place(&self, frame: &[PartDraw], joints: &[Affine2], out: &mut Vec<PartDraw>) {
        out.clear();
        out.extend(frame.iter().map(|draw| {
            let Some((track, binding)) = draw.track.and_then(|track| Some((track, self.binding(track)?))) else {
                return *draw;
            };
            let Some(joint) = joints.get(usize::from(binding.joint)) else {
                return *draw;
            };
            // A raster no shared frame drew under this handedness and flip
            // has no measured place: it stays where the frame put it.
            let Some(part) = self.parts.get(&(track, draw.part, mirrored(joint), draw.scale.x < 0.0)) else {
                return *draw;
            };
            PartDraw {
                at: joint.transform_point2(part.at),
                rotation: angle_of(joint) + part.rotation,
                ..*draw
            }
        }));
    }
}
