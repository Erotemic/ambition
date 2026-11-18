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
    /// The rig and its limbs, for [`Self::hold`]; `None` until
    /// [`Self::with_limbs`].
    limbs: Option<HeldLimbs>,
}

/// A limb: the indices of its upper, lower and end joints in the body rig.
pub type Limb = [u16; 3];

/// One frame of a follow-through (POSE-CONTINUITY item 3): how far the arms
/// and the head swing, in radians, and which way the hands go (sheet pixels,
/// +y down). Each arm turns about its shoulder so its hand moves toward
/// `hands_toward`, and its forearm lags a little further about the elbow;
/// the head turns about its joint by `head` (clockwise positive).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Swing {
    pub hands_toward: Vec2,
    pub arms: f32,
    pub head: f32,
}

/// How much further than the upper arm a forearm swings about its elbow.
const FOREARM_LAG: f32 = 0.6;

/// The rig whose poses plant a limb's end, and its limbs by the attachment
/// that ends each one (`hand_near`).
#[derive(Debug, Clone, PartialEq)]
struct HeldLimbs {
    rig: PreparedBodyRig,
    limbs: Vec<(String, Limb)>,
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
        Self { bindings, parts, frames, limbs: None }
    }

    /// These bindings with `rig`'s `limbs` (attachment name -> its upper,
    /// lower and end joint names, the rig's `limbs`), for [`Self::hold`]. A
    /// limb that names a joint the rig does not have is left out.
    pub fn with_limbs(mut self, rig: &PreparedBodyRig, limbs: &BTreeMap<String, [String; 3]>) -> Self {
        let index = |name: &String| rig.joint_names().iter().position(|joint| joint == name).map(|at| at as u16);
        let limbs: Vec<(String, Limb)> = limbs
            .iter()
            .filter_map(|(name, [upper, lower, end])| Some((name.clone(), [index(upper)?, index(lower)?, index(end)?])))
            .collect();
        self.limbs = (!limbs.is_empty()).then(|| HeldLimbs { rig: rig.clone(), limbs });
        self
    }

    /// The frame of the joint `draw` rides, as the draw puts it: the turn and
    /// pivot from the draw through its binding, the scale and handedness of
    /// `like` (that joint in the pose the draw belongs to).
    fn joint_frame_of(&self, draw: &PartDraw, like: &Affine2) -> Option<Affine2> {
        let track = draw.track?;
        let part = self.parts.get(&(track, draw.part, mirrored(like), draw.scale.x < 0.0))?;
        let shape = Mat2::from_angle(-angle_of(like)) * like.matrix2;
        let linear = Mat2::from_angle(draw.rotation - part.rotation) * shape;
        Some(Affine2::from_mat2_translation(linear, draw.at - linear * part.at))
    }

    /// `draw` placed by `joint`, the frame of the joint its track rides; `None`
    /// for a draw no binding places.
    fn placed(&self, draw: &PartDraw, joint: &Affine2) -> Option<PartDraw> {
        let track = draw.track?;
        let part = self.parts.get(&(track, draw.part, mirrored(joint), draw.scale.x < 0.0))?;
        Some(PartDraw {
            at: joint.transform_point2(part.at),
            rotation: angle_of(joint) + part.rotation,
            ..*draw
        })
    }

    /// Swing the arms (the limbs that end at a `hand_*` attachment) and the
    /// head (the joint of the `head` attachment) of the pose `row`[`index`]
    /// by `swing`, in `draws`: each draw that rides one of them turns rigidly
    /// about its shoulder (then elbow) or neck. Returns how many it swung:
    /// none without [`Self::with_limbs`] or for a row the rig does not author.
    pub fn follow_through(&self, row: &str, index: usize, swing: Swing, draws: &mut [PartDraw]) -> usize {
        let Some(held) = &self.limbs else { return 0 };
        let mut joints = Vec::new();
        if !held.rig.solve(row, index, &mut joints) {
            return 0;
        }
        let rides = |draw: &PartDraw, joint: u16| draw.track.and_then(|track| self.binding(track)).is_some_and(|b| b.joint == joint);
        let frame_of = |draws: &[PartDraw], joint: u16| {
            let like = joints.get(usize::from(joint))?;
            draws.iter().filter(|draw| rides(draw, joint)).find_map(|draw| self.joint_frame_of(draw, like))
        };
        let turn = |draw: &mut PartDraw, pivot: Vec2, angle: f32| {
            draw.at = pivot + Vec2::from_angle(angle).rotate(draw.at - pivot);
            draw.rotation += angle;
        };
        let mut count = 0;
        for (_, [upper, lower, end]) in held.limbs.iter().filter(|(name, _)| name.starts_with("hand_")) {
            let (Some(shoulder), Some(elbow), Some(hand)) = (frame_of(draws, *upper), frame_of(draws, *lower), frame_of(draws, *end)) else {
                continue;
            };
            // Each turn goes the way that carries the hand toward the swing
            // about its own pivot: a folded arm's forearm can turn the
            // other way from its upper arm and still lower the hand.
            let toward = |pivot: Vec2, point: Vec2, angle: f32| {
                if (point - pivot).perp_dot(swing.hands_toward) < 0.0 { -angle } else { angle }
            };
            let (s, e, w) = (shoulder.translation, elbow.translation, hand.translation);
            let angle = toward(s, w, swing.arms);
            let swung_elbow = s + Vec2::from_angle(angle).rotate(e - s);
            let swung_hand = s + Vec2::from_angle(angle).rotate(w - s);
            let lag = toward(swung_elbow, swung_hand, FOREARM_LAG * swing.arms);
            for draw in draws.iter_mut() {
                if rides(draw, *upper) || rides(draw, *lower) || rides(draw, *end) {
                    turn(draw, s, angle);
                }
                if rides(draw, *lower) || rides(draw, *end) {
                    turn(draw, swung_elbow, lag);
                }
            }
            count += 1;
        }
        if let Some(head) = held.rig.attachments().iter().find(|attachment| attachment.name == "head").map(|attachment| attachment.joint) {
            if let Some(neck) = frame_of(draws, head) {
                for draw in draws.iter_mut().filter(|draw| rides(draw, head)) {
                    turn(draw, neck.translation, swing.head);
                }
                count += 1;
            }
        }
        count
    }

    /// Hold the ends of the limbs named in `contacts` (`hand_near`) where the
    /// pose `row`[`index`] of the rig plants them, in `draws`: that pose's
    /// draws, moved part of the way from an earlier pose by a clip blend.
    ///
    /// Each end joint's parts go where the pose draws them, and the upper and
    /// lower bones are solved between the root of the limb (a shoulder, a
    /// hip), where the blended draws now put it, and that end, bent to the
    /// side the pose bends them. So a hand that catches a ledge is on the
    /// ledge from the first frame while the body is still coming up to it.
    /// Returns how many limbs it held: none without [`Self::with_limbs`], for
    /// a row the rig does not author, or for a limb no draw roots.
    pub fn hold(&self, row: &str, index: usize, contacts: &[&str], draws: &mut [PartDraw]) -> usize {
        let Some(held) = &self.limbs else { return 0 };
        let mut joints = Vec::new();
        if contacts.is_empty() || !held.rig.solve(row, index, &mut joints) {
            return 0;
        }
        let angle = |v: Vec2| v.y.atan2(v.x);
        let mut count = 0;
        for (_, [upper, lower, end]) in held.limbs.iter().filter(|(name, _)| contacts.contains(&name.as_str())) {
            let (Some(&fu), Some(&fl), Some(&fe)) =
                (joints.get(usize::from(*upper)), joints.get(usize::from(*lower)), joints.get(usize::from(*end)))
            else {
                continue;
            };
            let rides = |draw: &&PartDraw, joint: u16| draw.track.and_then(|track| self.binding(track)).is_some_and(|b| b.joint == joint);
            let Some(root) = draws.iter().filter(|draw| rides(draw, *upper)).find_map(|draw| self.joint_frame_of(draw, &fu)) else {
                continue;
            };
            let (s, e, w) = (fu.translation, fl.translation, fe.translation);
            let (a, b) = (s.distance(e), e.distance(w));
            if a <= 1e-3 || b <= 1e-3 {
                continue;
            }
            // The elbow (knee): a from the root, b from the held end, on the
            // side of the line root -> end that the pose puts it.
            let s2 = root.translation;
            let to_end = w - s2;
            let toward = to_end.try_normalize().unwrap_or_else(|| (e - s).normalize_or(Vec2::X));
            let reach = to_end.length().clamp((a - b).abs() + 1e-3, a + b - 1e-3);
            let opening = ((a * a + reach * reach - b * b) / (2.0 * a * reach)).clamp(-1.0, 1.0).acos();
            let side = if (w - s).perp_dot(e - s) < 0.0 { -1.0 } else { 1.0 };
            let elbow = s2 + Vec2::from_angle(side * opening).rotate(toward) * a;
            let upper_frame = Affine2::from_translation(s2)
                * Affine2::from_angle(angle(elbow - s2) - angle(e - s))
                * Affine2::from_translation(-s)
                * fu;
            let lower_frame = Affine2::from_translation(elbow)
                * Affine2::from_angle(angle(w - elbow) - angle(w - e))
                * Affine2::from_translation(-e)
                * fl;
            for draw in draws.iter_mut() {
                let Some(binding) = draw.track.and_then(|track| self.binding(track)) else { continue };
                let frame = match binding.joint {
                    joint if joint == *upper => upper_frame,
                    joint if joint == *lower => lower_frame,
                    joint if joint == *end => fe,
                    _ => continue,
                };
                if let Some(placed) = self.placed(draw, &frame) {
                    *draw = placed;
                }
            }
            count += 1;
        }
        count
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
            // A raster no shared frame drew under this handedness and flip
            // has no measured place: it stays where the frame put it.
            draw.track
                .and_then(|track| self.binding(track))
                .and_then(|binding| joints.get(usize::from(binding.joint)))
                .and_then(|joint| self.placed(draw, joint))
                .unwrap_or(*draw)
        }));
    }
}
