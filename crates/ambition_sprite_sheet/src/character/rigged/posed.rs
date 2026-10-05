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

use bevy::math::{Affine2, Vec2};

use ambition_characters::actor::PreparedBodyRig;

use super::{PartDraw, RiggedSpriteAsset};

/// A track bound to the joint it rides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackBinding {
    /// The joint's index in the body rig.
    pub joint: u16,
    /// The draw's pivot in the joint's frame, in sheet pixels (+y down).
    pub at: Vec2,
    /// The draw's angle less the joint's, in radians (clockwise, +y down).
    pub rotation: f32,
    /// How far the measured placements spread around these: the largest
    /// distance of the pivot from `at`, in sheet pixels, over every frame the
    /// two files share.
    pub spread_px: f32,
    /// The same for the angle, in radians.
    pub spread_rad: f32,
}

/// Every track of a flipbook bound to the body-rig joint it rides, where one
/// does.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PosedParts {
    /// By track index; `None` for a track no joint holds within the tolerance
    /// (an effect layer, a part the rig does not name).
    bindings: Vec<Option<TrackBinding>>,
    /// The frames both files share that the measurement read.
    frames: usize,
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
    /// Bind each track of `flipbook` to the joint of `rig` it rides: over
    /// every frame of every clip the two share (same name, same frame count),
    /// the joint whose frame holds the track's pivot and angle stillest. A
    /// track binds when its pivot spreads no more than `tolerance_px` sheet
    /// pixels and its angle no more than `tolerance_rad`.
    pub fn bind(flipbook: &RiggedSpriteAsset, rig: &PreparedBodyRig, tolerance_px: f32, tolerance_rad: f32) -> Self {
        let joints = rig.joint_names().len();
        // samples[track][joint] = (pivot, angle) in the joint's frame, per frame.
        let mut samples: Vec<Vec<Vec<(Vec2, f32)>>> = vec![vec![Vec::new(); joints]; flipbook.tracks.len()];
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
                        samples[track][joint].push((local, wrapped(draw.rotation - angle_of(frame))));
                    }
                }
            }
        }
        let bindings = samples
            .iter()
            .map(|by_joint| {
                by_joint
                    .iter()
                    .enumerate()
                    .filter(|(_, samples)| !samples.is_empty())
                    .map(|(joint, samples)| {
                        let n = samples.len() as f32;
                        let at = samples.iter().map(|(at, _)| *at).sum::<Vec2>() / n;
                        let reference = samples[0].1;
                        let rotation = reference + samples.iter().map(|(_, angle)| wrapped(angle - reference)).sum::<f32>() / n;
                        let spread_px = samples.iter().map(|(p, _)| p.distance(at)).fold(0.0, f32::max);
                        let spread_rad = samples.iter().map(|(_, angle)| wrapped(angle - rotation).abs()).fold(0.0, f32::max);
                        TrackBinding {
                            joint: joint as u16,
                            at,
                            rotation: wrapped(rotation),
                            spread_px,
                            spread_rad,
                        }
                    })
                    .filter(|binding| binding.spread_px <= tolerance_px && binding.spread_rad <= tolerance_rad)
                    .min_by(|a, b| (a.spread_px + a.spread_rad.to_degrees() * 0.1).total_cmp(&(b.spread_px + b.spread_rad.to_degrees() * 0.1)))
            })
            .collect();
        Self { bindings, frames }
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
            let Some(binding) = draw.track.and_then(|track| self.binding(track)) else {
                return *draw;
            };
            let Some(joint) = joints.get(usize::from(binding.joint)) else {
                return *draw;
            };
            PartDraw {
                at: joint.transform_point2(binding.at),
                rotation: angle_of(joint) + binding.rotation,
                ..*draw
            }
        }));
    }
}
