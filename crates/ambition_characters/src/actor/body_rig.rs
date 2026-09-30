//! A body's semantic rig: named joints, attachment points and simple hurt
//! parts, with one sampled pose per frame of each clip.
//!
//! This is SIMULATION data. It holds no texture, atlas rectangle, material or
//! authoring-tool concept, and a headless build reads it with no image loaded.
//! A character can have a rig and still draw from a baked sheet: the rig says
//! where a hand or a head IS, not how the body looks.
//!
//! Space: a rig is local to the body's FEET (the bottom centre of its body
//! box), +x is the direction the body faces, +y is DOWN (world +y), and the
//! units are world units after [`BodyRigDefinition::scaled`]. Rotations are in
//! radians, positive clockwise on screen. A joint pose is local to its parent;
//! a joint with no parent is local to the feet.
//!
//! Clips are keyed by the SAME names as the character's sheet rows (`idle`,
//! `walk`, `jump`, a move's clip binding), so the rig is selected with the
//! vocabulary the sheet is selected with. There is no second animation
//! vocabulary.
//!
//! [`BodyRigDefinition`] is the authored form. [`BodyRigDefinition::prepare`]
//! validates it once, at character preparation, into a [`PreparedBodyRig`]
//! whose joints are in parent-first order and whose references are indices.
//! Nothing at runtime looks a joint up by name.

use std::collections::{BTreeMap, BTreeSet};

use bevy::math::{Affine2, Vec2};

/// The attachment a hand-held muzzle fires from.
pub const HAND_NEAR: &str = "hand_near";
/// The hand behind the body.
pub const HAND_FAR: &str = "hand_far";
/// The centre of the head.
pub const HEAD: &str = "head";
/// The tip of the leg in front.
pub const FOOT_NEAR: &str = "foot_near";
/// The tip of the leg behind.
pub const FOOT_FAR: &str = "foot_far";

/// The published product's schema version this reader accepts.
pub const BODY_RIG_SCHEMA_VERSION: u32 = 1;

/// The environment variable that admits published body rigs when no
/// composition inserted a [`BodyRigAdmission`]: `1`, `true`, `on` or `yes`.
pub const BODY_RIG_ADMISSION_ENV: &str = "AMBITION_BODY_RIGS";

/// Does this composition give its characters their published body rigs?
///
/// ⛔ OFF BY DEFAULT, AND THAT IS THE CONTRACT. Articulated body rigs are on
/// trial (`docs/planning/engine/runtime-rigged-sprite-animation.md`): every
/// shipping game keeps its current body geometry until the rig road is
/// accepted. Off, no definition carries a rig, so no body has one, and
/// nothing a rig would drive (hurt parts, attachments) exists anywhere.
///
/// Read where a cast is REGISTERED, because a rig is part of the character's
/// content identity: an admitted rig changes the fingerprint a rollback
/// timeline compares, and a rig that is not admitted leaves it byte-identical.
/// A composition that wants rigs inserts this before it registers its cast; a
/// composition that inserts nothing follows [`BODY_RIG_ADMISSION_ENV`].
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodyRigAdmission {
    pub admit: bool,
}

impl BodyRigAdmission {
    pub const ADMIT: Self = Self { admit: true };

    /// The switch as [`BODY_RIG_ADMISSION_ENV`] sets it; off when unset.
    pub fn from_env() -> Self {
        let admit = std::env::var(BODY_RIG_ADMISSION_ENV).is_ok_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "on" | "yes"
            )
        });
        Self { admit }
    }

    /// The composition's answer: its resource when it inserted one, else the
    /// environment.
    pub fn of(world: &bevy::ecs::world::World) -> Self {
        world
            .get_resource::<Self>()
            .copied()
            .unwrap_or_else(Self::from_env)
    }
}

/// One joint's transform in one frame, local to its parent.
///
/// A point `p` in the joint's frame is placed at
/// `translation + rotate(rotation, (scale.0 * p.x, scale.1 * p.y))`.
/// A negative `scale.0` is a mirror about the joint's own pivot.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JointPose {
    pub translation: (f32, f32),
    pub rotation: f32,
    pub scale: (f32, f32),
}

impl JointPose {
    fn affine(&self) -> Affine2 {
        Affine2::from_scale_angle_translation(
            Vec2::new(self.scale.0, self.scale.1),
            self.rotation,
            Vec2::new(self.translation.0, self.translation.1),
        )
    }

    fn is_finite(&self) -> bool {
        [
            self.translation.0,
            self.translation.1,
            self.rotation,
            self.scale.0,
            self.scale.1,
        ]
        .iter()
        .all(|value| value.is_finite())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigJoint {
    pub name: String,
    pub parent: Option<String>,
}

/// A named semantic point on a joint: a hand, a head, a foot.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigAttachment {
    pub name: String,
    pub joint: String,
    /// In the joint's frame.
    pub offset: (f32, f32),
}

/// A simple hurt shape in its joint's frame.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RigShape {
    Circle { center: (f32, f32), radius: f32 },
    Capsule { a: (f32, f32), b: (f32, f32), radius: f32 },
    Rect { center: (f32, f32), half_extents: (f32, f32) },
}

impl RigShape {
    fn is_valid(&self) -> bool {
        let finite = |values: &[f32]| values.iter().all(|value| value.is_finite());
        match *self {
            Self::Circle { center, radius } => finite(&[center.0, center.1, radius]) && radius >= 0.0,
            Self::Capsule { a, b, radius } => {
                finite(&[a.0, a.1, b.0, b.1, radius]) && radius >= 0.0
            }
            Self::Rect {
                center,
                half_extents,
            } => {
                finite(&[center.0, center.1, half_extents.0, half_extents.1])
                    && half_extents.0 >= 0.0
                    && half_extents.1 >= 0.0
            }
        }
    }

    fn scaled(self, k: f32) -> Self {
        let s = |p: (f32, f32)| (p.0 * k, p.1 * k);
        match self {
            Self::Circle { center, radius } => Self::Circle {
                center: s(center),
                radius: radius * k,
            },
            Self::Capsule { a, b, radius } => Self::Capsule {
                a: s(a),
                b: s(b),
                radius: radius * k,
            },
            Self::Rect {
                center,
                half_extents,
            } => Self::Rect {
                center: s(center),
                half_extents: s(half_extents),
            },
        }
    }

    /// The axis-aligned bounds of this shape placed by `to_space`, as
    /// `(min, max)`.
    ///
    /// A rotated shape is bounded, not approximated from inside: a hurt volume
    /// errs toward hittable, never toward invulnerable.
    pub fn bounds_in(&self, to_space: Affine2) -> (Vec2, Vec2) {
        let v = |p: (f32, f32)| Vec2::new(p.0, p.1);
        // The joint's scale can be non-uniform; the radius grows by the
        // largest axis so the bound still contains the shape.
        let radius_scale = to_space
            .matrix2
            .x_axis
            .length()
            .max(to_space.matrix2.y_axis.length());
        match *self {
            Self::Circle { center, radius } => {
                let c = to_space.transform_point2(v(center));
                let r = Vec2::splat(radius * radius_scale);
                (c - r, c + r)
            }
            Self::Capsule { a, b, radius } => {
                let a = to_space.transform_point2(v(a));
                let b = to_space.transform_point2(v(b));
                let r = Vec2::splat(radius * radius_scale);
                (a.min(b) - r, a.max(b) + r)
            }
            Self::Rect {
                center,
                half_extents,
            } => {
                let (c, h) = (v(center), v(half_extents));
                let corners = [
                    c + Vec2::new(-h.x, -h.y),
                    c + Vec2::new(h.x, -h.y),
                    c + Vec2::new(h.x, h.y),
                    c + Vec2::new(-h.x, h.y),
                ]
                .map(|corner| to_space.transform_point2(corner));
                let min = corners.iter().copied().reduce(Vec2::min).unwrap_or_default();
                let max = corners.iter().copied().reduce(Vec2::max).unwrap_or_default();
                (min, max)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigHurtPart {
    pub name: String,
    pub joint: String,
    pub shape: RigShape,
}

/// One clip: a pose per frame, with the frame count and duration of the sheet
/// row of the same name.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigClip {
    pub looping: bool,
    pub frame_duration_s: f32,
    /// One [`JointPose`] per joint, in the rig's joint order.
    pub frames: Vec<Vec<JointPose>>,
}

impl RigClip {
    /// The frame a body shows `elapsed_s` seconds into this clip on its own
    /// clock. A looping clip wraps; a one-shot holds its last frame.
    pub fn frame_at_time(&self, elapsed_s: f32) -> usize {
        let count = self.frames.len().max(1);
        let index = (elapsed_s.max(0.0) / self.frame_duration_s).floor();
        let index = if index.is_finite() { index as usize } else { 0 };
        if self.looping {
            index % count
        } else {
            index.min(count - 1)
        }
    }

    /// The frame a MOVE shows at normalized progress `phase` in `[0, 1]`. The
    /// clip is slaved to the move, the same rule the sheet row follows.
    pub fn frame_at_phase(&self, phase: f32) -> usize {
        let count = self.frames.len().max(1);
        let index = (phase.clamp(0.0, 1.0) * count as f32).floor();
        (if index.is_finite() { index as usize } else { 0 }).min(count - 1)
    }
}

/// The authored rig of one character. See the module docs for its space.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyRigDefinition {
    pub joints: Vec<RigJoint>,
    #[serde(default)]
    pub attachments: Vec<RigAttachment>,
    #[serde(default)]
    pub hurt_parts: Vec<RigHurtPart>,
    pub clips: BTreeMap<String, RigClip>,
}

/// The file a sprite publisher writes (`<target>_body_rig.ron`), in sheet
/// pixels. The reader converts it with [`BodyRigDefinition::scaled`].
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PublishedBodyRig {
    schema_version: u32,
    #[allow(dead_code)]
    target: String,
    joints: Vec<RigJoint>,
    #[serde(default)]
    attachments: Vec<RigAttachment>,
    #[serde(default)]
    hurt_parts: Vec<RigHurtPart>,
    clips: BTreeMap<String, RigClip>,
}

impl BodyRigDefinition {
    /// Read a published `<target>_body_rig.ron`, in the sheet pixels it was
    /// written in.
    pub fn from_published_ron(text: &str) -> Result<Self, String> {
        let published: PublishedBodyRig =
            ron::from_str(text).map_err(|error| format!("body rig does not parse: {error}"))?;
        if published.schema_version != BODY_RIG_SCHEMA_VERSION {
            return Err(format!(
                "body rig schema {} is not the schema {BODY_RIG_SCHEMA_VERSION} this build reads",
                published.schema_version
            ));
        }
        Ok(Self {
            joints: published.joints,
            attachments: published.attachments,
            hurt_parts: published.hurt_parts,
            clips: published.clips,
        })
    }

    /// The same rig with every length multiplied by `k` (sheet pixels to world
    /// units). Rotations and per-joint scale factors are unitless.
    pub fn scaled(mut self, k: f32) -> Self {
        let s = |p: (f32, f32)| (p.0 * k, p.1 * k);
        for attachment in &mut self.attachments {
            attachment.offset = s(attachment.offset);
        }
        for part in &mut self.hurt_parts {
            part.shape = part.shape.scaled(k);
        }
        for clip in self.clips.values_mut() {
            for frame in &mut clip.frames {
                for pose in frame {
                    pose.translation = s(pose.translation);
                }
            }
        }
        self
    }

    /// Validate this rig and resolve it into its runtime form.
    ///
    /// Joints are reordered parent-first, so one pass over the joints solves
    /// the whole rig; each clip frame is reordered with them.
    pub fn prepare(&self) -> Result<PreparedBodyRig, BodyRigError> {
        let mut index_of = BTreeMap::new();
        for (index, joint) in self.joints.iter().enumerate() {
            if index_of.insert(joint.name.as_str(), index).is_some() {
                return Err(BodyRigError::DuplicateJoint(joint.name.clone()));
            }
        }
        if self.joints.len() > usize::from(u16::MAX) {
            return Err(BodyRigError::TooManyJoints(self.joints.len()));
        }
        let mut parent_of = Vec::with_capacity(self.joints.len());
        for joint in &self.joints {
            parent_of.push(match &joint.parent {
                None => None,
                Some(parent) => Some(*index_of.get(parent.as_str()).ok_or_else(|| {
                    BodyRigError::UnknownParent {
                        joint: joint.name.clone(),
                        parent: parent.clone(),
                    }
                })?),
            });
        }
        // Parent-first order by depth-first placement. A joint reached twice
        // on one path is a cycle.
        let mut order: Vec<usize> = Vec::with_capacity(self.joints.len());
        let mut placed = vec![false; self.joints.len()];
        for start in 0..self.joints.len() {
            let mut path = Vec::new();
            let mut current = Some(start);
            while let Some(index) = current {
                if placed[index] {
                    break;
                }
                if path.contains(&index) {
                    return Err(BodyRigError::ParentCycle(self.joints[index].name.clone()));
                }
                path.push(index);
                current = parent_of[index];
            }
            for &index in path.iter().rev() {
                placed[index] = true;
                order.push(index);
            }
        }
        let mut new_index = vec![0u16; self.joints.len()];
        for (position, &old) in order.iter().enumerate() {
            new_index[old] = position as u16;
        }
        let joint = |what: &'static str, name: &str, joint: &str| {
            index_of
                .get(joint)
                .map(|&old| new_index[old])
                .ok_or_else(|| BodyRigError::UnknownJoint {
                    what,
                    name: name.to_string(),
                    joint: joint.to_string(),
                })
        };
        let mut attachments = Vec::with_capacity(self.attachments.len());
        let mut attachment_names = BTreeSet::new();
        for attachment in &self.attachments {
            if !attachment_names.insert(attachment.name.as_str()) {
                return Err(BodyRigError::DuplicateAttachment(attachment.name.clone()));
            }
            if !(attachment.offset.0.is_finite() && attachment.offset.1.is_finite()) {
                return Err(BodyRigError::NonFinite(format!("attachment `{}`", attachment.name)));
            }
            attachments.push(PreparedAttachment {
                name: attachment.name.clone(),
                joint: joint("attachment", &attachment.name, &attachment.joint)?,
                offset: Vec2::new(attachment.offset.0, attachment.offset.1),
            });
        }
        let mut hurt_parts = Vec::with_capacity(self.hurt_parts.len());
        for part in &self.hurt_parts {
            if !part.shape.is_valid() {
                return Err(BodyRigError::NonFinite(format!("hurt part `{}`", part.name)));
            }
            hurt_parts.push(PreparedHurtPart {
                name: part.name.clone(),
                joint: joint("hurt part", &part.name, &part.joint)?,
                shape: part.shape,
            });
        }
        if self.clips.is_empty() {
            return Err(BodyRigError::NoClips);
        }
        let mut clips = BTreeMap::new();
        for (name, clip) in &self.clips {
            if !(clip.frame_duration_s.is_finite() && clip.frame_duration_s > 0.0) {
                return Err(BodyRigError::BadClip {
                    clip: name.clone(),
                    reason: format!("frame duration {}", clip.frame_duration_s),
                });
            }
            if clip.frames.is_empty() {
                return Err(BodyRigError::BadClip {
                    clip: name.clone(),
                    reason: "no frames".to_string(),
                });
            }
            let mut frames = Vec::with_capacity(clip.frames.len());
            for (index, frame) in clip.frames.iter().enumerate() {
                if frame.len() != self.joints.len() {
                    return Err(BodyRigError::BadClip {
                        clip: name.clone(),
                        reason: format!(
                            "frame {index} poses {} joints; the rig has {}",
                            frame.len(),
                            self.joints.len()
                        ),
                    });
                }
                if !frame.iter().all(JointPose::is_finite) {
                    return Err(BodyRigError::NonFinite(format!("clip `{name}` frame {index}")));
                }
                frames.push(order.iter().map(|&old| frame[old]).collect());
            }
            clips.insert(
                name.clone(),
                RigClip {
                    looping: clip.looping,
                    frame_duration_s: clip.frame_duration_s,
                    frames,
                },
            );
        }
        Ok(PreparedBodyRig {
            joint_names: order.iter().map(|&old| self.joints[old].name.clone()).collect(),
            parents: order
                .iter()
                .map(|&old| parent_of[old].map(|parent| new_index[parent]))
                .collect(),
            attachments,
            hurt_parts,
            clips,
        })
    }
}

/// Why a rig was refused at preparation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyRigError {
    DuplicateJoint(String),
    UnknownParent { joint: String, parent: String },
    ParentCycle(String),
    UnknownJoint {
        what: &'static str,
        name: String,
        joint: String,
    },
    DuplicateAttachment(String),
    NonFinite(String),
    NoClips,
    BadClip { clip: String, reason: String },
    TooManyJoints(usize),
}

impl std::fmt::Display for BodyRigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateJoint(name) => write!(f, "joint `{name}` is declared twice"),
            Self::UnknownParent { joint, parent } => {
                write!(f, "joint `{joint}` names parent `{parent}`, which is not a joint")
            }
            Self::ParentCycle(joint) => write!(f, "joint `{joint}` is its own ancestor"),
            Self::UnknownJoint { what, name, joint } => {
                write!(f, "{what} `{name}` names joint `{joint}`, which is not a joint")
            }
            Self::DuplicateAttachment(name) => write!(f, "attachment `{name}` is declared twice"),
            Self::NonFinite(what) => write!(f, "{what} has a non-finite or negative dimension"),
            Self::NoClips => write!(f, "the rig has no clips, so no pose can be resolved"),
            Self::BadClip { clip, reason } => write!(f, "clip `{clip}`: {reason}"),
            Self::TooManyJoints(count) => write!(f, "{count} joints is more than a rig indexes"),
        }
    }
}

impl std::error::Error for BodyRigError {}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedAttachment {
    pub name: String,
    pub joint: u16,
    pub offset: Vec2,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedHurtPart {
    pub name: String,
    pub joint: u16,
    pub shape: RigShape,
}

/// A validated rig: joints parent-first, references resolved to indices.
///
/// Immutable after preparation. A body holds it behind an `Arc`, so every body
/// of one character shares one copy.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedBodyRig {
    joint_names: Vec<String>,
    /// `parents[i] < i` for every joint that has a parent.
    parents: Vec<Option<u16>>,
    attachments: Vec<PreparedAttachment>,
    hurt_parts: Vec<PreparedHurtPart>,
    clips: BTreeMap<String, RigClip>,
}

impl PreparedBodyRig {
    pub fn joint_names(&self) -> &[String] {
        &self.joint_names
    }

    pub fn attachments(&self) -> &[PreparedAttachment] {
        &self.attachments
    }

    pub fn hurt_parts(&self) -> &[PreparedHurtPart] {
        &self.hurt_parts
    }

    pub fn clip(&self, name: &str) -> Option<&RigClip> {
        self.clips.get(name)
    }

    pub fn clip_names(&self) -> impl Iterator<Item = &str> {
        self.clips.keys().map(String::as_str)
    }

    /// The index of attachment `name`, for a consumer that asks every tick.
    pub fn attachment_index(&self, name: &str) -> Option<usize> {
        self.attachments.iter().position(|attachment| attachment.name == name)
    }

    /// The first clip of `chain` this rig has, with its name.
    pub fn first_clip<'a>(
        &'a self,
        chain: impl IntoIterator<Item = &'a str>,
    ) -> Option<(&'a str, &'a RigClip)> {
        chain
            .into_iter()
            .find_map(|name| self.clips.get_key_value(name))
            .map(|(name, clip)| (name.as_str(), clip))
    }

    /// Solve one frame of one clip into rig-space joint transforms, written
    /// into `joints` (resized to the joint count). Returns `false` and leaves
    /// `joints` empty when the clip or frame does not exist.
    pub fn solve(&self, clip: &str, frame: usize, joints: &mut Vec<Affine2>) -> bool {
        joints.clear();
        let Some(poses) = self.clips.get(clip).and_then(|clip| clip.frames.get(frame)) else {
            return false;
        };
        for (index, pose) in poses.iter().enumerate() {
            let local = pose.affine();
            let world = match self.parents[index] {
                Some(parent) => joints[usize::from(parent)] * local,
                None => local,
            };
            joints.push(world);
        }
        true
    }
}

#[cfg(test)]
mod tests;
