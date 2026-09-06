//! Authored moving platforms: the spec an editor writes, the motion it
//! resolves to, and the runtime state the simulation advances.
//!
//! Moving platforms are ordinary deterministic world geometry. They contribute
//! solid blocks to the collision world each frame, carry riders and ledge
//! contacts by [`MovingPlatformState::last_delta`], and can host a portal face.
//! The authoritative state lives here in the world crate; the Bevy visual is a
//! read-model projection of it.

use crate::rooms::KinematicPathSpec;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;

/// Sweep span used when an author places a platform and states no motion.
///
///  the engine owns its own defaults. These lived in the LDtk converter,
/// which made "what does an empty field mean" a question you answered by
/// reading the adapter rather than the capability.
pub const DEFAULT_SWEEP_DX: f32 = 240.0;
/// Travel speed used when an author states none.
pub const DEFAULT_PLATFORM_SPEED: f32 = 130.0;

/// How an authored platform moves — exactly one motion, decided when the room
/// is authored.
///
///  this replaced a bag of optional fields whose meaning was a PRECEDENCE
/// (a path beat a loop beat a sweep). Precedence makes every wrong combination
/// SILENT, and silence is the worst outcome for an editor field: a platform
/// authoring both a path and a loop ran the path and never said so, and a
/// `loop_min_y` written without `loop_dy` anchored a shaft that no motion ever
/// consulted. An author cannot see a precedence rule from inside LDtk, so the
/// ambiguous combinations are now REFUSED by [`AuthoredPlatformMotion::classify`]
/// with the offending field names in the message.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MovingPlatformMotionSpec {
    /// Horizontal ping-pong across `dx` from the authored position. Sign is the
    /// direction the platform first travels.
    Sweep { dx: f32, speed: f32 },
    /// Follow a room-local [`KinematicPathSpec`], which owns its own speed.
    Path { path_id: String },
    /// A wrapping vertical loop — the paternoster / "infinite elevator".
    ///
    /// `dy` mirrors a sweep's span: magnitude is the shaft, sign is the
    /// direction of travel.  positive travels DOWN — world y is
    /// down-positive here and the LDtk conversion preserves it, so a descending
    /// elevator is a POSITIVE `dy`.  a loop is not a vertical sweep: it
    /// never reverses, which is what makes a run of them read as one elevator
    /// instead of a row of lifts.
    ///
    /// Anchored, the authored position becomes a PHASE within a shared shaft. `None` anchors the
    /// shaft at the platform, which is right for a lone lift and keeps that authoring to one field.
    VerticalLoop {
        dy: f32,
        anchor_y: Option<f32>,
        speed: f32,
    },
}

/// The motion fields an editor can write on one platform, before they are known
/// to describe a coherent motion.
///
/// This is the adapter-facing shape: an LDtk converter fills in whichever fields
/// the author touched and asks [`Self::classify`] what they mean. Nothing
/// downstream of `classify` can observe an ambiguous platform.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AuthoredPlatformMotion {
    pub sweep_dx: Option<f32>,
    pub speed: Option<f32>,
    pub path_id: Option<String>,
    pub loop_dy: Option<f32>,
    pub loop_anchor_y: Option<f32>,
}

impl AuthoredPlatformMotion {
    /// Decide which motion these fields describe, or say why they describe none.
    ///
    /// Stating nothing is legal and means a default sweep — an author who drops
    /// a platform into a room gets a platform that moves. Stating two motions is
    /// not legal, because there is no honest way to guess which one was meant.
    pub fn classify(self) -> Result<MovingPlatformMotionSpec, String> {
        let path_id = self.path_id.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });

        let mut authored = Vec::new();
        if self.sweep_dx.is_some() {
            authored.push("sweep_dx");
        }
        if path_id.is_some() {
            authored.push("path_id");
        }
        if self.loop_dy.is_some() {
            authored.push("loop_dy");
        }
        if authored.len() > 1 {
            return Err(format!(
                "authors {} at once, but a platform has exactly one motion — \
                 keep the field for the motion you want and clear the others",
                authored.join(" and ")
            ));
        }

        if self.loop_anchor_y.is_some() && self.loop_dy.is_none() {
            return Err(
                "authors loop_min_y without loop_dy — the anchor names where a \
                 wrapping shaft starts, so on its own it describes no motion at \
                 all"
                .to_string(),
            );
        }

        if let Some(dy) = self.loop_dy {
            if dy.abs() <= f32::EPSILON {
                return Err(
                    "authors loop_dy of zero — a shaft with no span never moves; \
                     give it a signed height (positive travels DOWN) or clear it"
                        .to_string(),
                );
            }
            return Ok(MovingPlatformMotionSpec::VerticalLoop {
                dy,
                anchor_y: self.loop_anchor_y,
                speed: self.speed.unwrap_or(DEFAULT_PLATFORM_SPEED),
            });
        }

        if let Some(path_id) = path_id {
            //  the path owns its speed, so a `speed` written here does nothing.
            if self.speed.is_some() {
                return Err(format!(
                    "follows path '{path_id}' and also authors speed, but a \
                     path carries its own speed — set it on the KinematicPath"
                ));
            }
            return Ok(MovingPlatformMotionSpec::Path { path_id });
        }

        Ok(MovingPlatformMotionSpec::Sweep {
            dx: self.sweep_dx.unwrap_or(DEFAULT_SWEEP_DX),
            speed: self.speed.unwrap_or(DEFAULT_PLATFORM_SPEED),
        })
    }
}

/// An authored moving-platform declaration before path references are resolved.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MovingPlatformSpec {
    pub id: String,
    pub name: String,
    pub start_pos: ae::Vec2,
    pub size: ae::Vec2,
    pub motion: MovingPlatformMotionSpec,
}

impl MovingPlatformSpec {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        start_pos: ae::Vec2,
        size: ae::Vec2,
        motion: MovingPlatformMotionSpec,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            start_pos,
            size,
            motion,
        }
    }

    pub fn resolve(self, paths: &[KinematicPathSpec]) -> Result<MovingPlatformState, String> {
        match self.motion {
            MovingPlatformMotionSpec::Path { path_id } => {
                let Some(path_spec) = paths.iter().find(|path| path.matches_id(&path_id)) else {
                    let known = paths
                        .iter()
                        .flat_map(|path| path.resolution_aliases())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(format!(
                        "MovingPlatform '{}' references unknown path_id '{}' (known: [{}])",
                        self.name, path_id, known
                    ));
                };
                Ok(MovingPlatformState::from_path(
                    self.id,
                    self.name,
                    self.size,
                    path_spec.path.clone(),
                ))
            }
            MovingPlatformMotionSpec::VerticalLoop {
                dy,
                anchor_y,
                speed,
            } => {
                let (min_y, max_y) = match anchor_y {
                    Some(base) => (base, base + dy.abs()),
                    None => {
                        let end_y = self.start_pos.y + dy;
                        (self.start_pos.y.min(end_y), self.start_pos.y.max(end_y))
                    }
                };
                Ok(MovingPlatformState::from_vertical_loop(
                    self.id,
                    self.name,
                    self.start_pos,
                    self.size,
                    min_y,
                    max_y,
                    speed,
                    // positive dy travels toward +y, which is DOWN.
                    dy > 0.0,
                ))
            }
            MovingPlatformMotionSpec::Sweep { dx, speed } => Ok(MovingPlatformState::from_sweep(
                self.id,
                self.name,
                self.start_pos,
                self.size,
                dx,
                speed,
            )),
        }
    }
}

/// Runtime state for one LDtk-authored moving platform.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MovingPlatformState {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    motion: MovingPlatformMotion,
    /// Displacement applied by the most recent [`Self::update`] advance.
    last_delta: ae::Vec2,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum MovingPlatformMotion {
    Sweep {
        min_x: f32,
        max_x: f32,
        speed: f32,
        dir: f32,
    },
    Path {
        path: ambition_platformer2d_core::KinematicPath,
        segment: usize,
        dir: i32,
    },
    /// A one-way vertical loop — the paternoster / "infinite elevator".
    ///
    /// Moves continuously in one vertical direction and wraps to the opposite
    /// end instead of reversing.
    ///
    ///  it WRAPS where the other two REVERSE, and that is the whole reason
    /// it is a third variant rather than a `Sweep` with the axis swapped. A
    /// reversing platform is a lift; a wrapping one is a conveyor of lifts, and
    /// the player experience — step off the top, another arrives from below —
    /// only exists if the platform never turns around.
    Loop {
        min_y: f32,
        max_y: f32,
        speed: f32,
        /// `+1` travels toward +y, `-1` toward -y. Constant for the lifetime of
        /// the platform: this motion has no reversal, which is the point.
        ///
        ///  +y is DOWN here. The LDtk conversion does not flip the axis, so
        /// world y increases downward — a falling body's y grows. `+1` therefore
        /// DESCENDS on screen, which is the opposite of what "positive" reads
        /// like and is worth stating wherever the sign is chosen.
        dir: f32,
    },
}

impl MovingPlatformState {
    /// Build from LDtk-authored AABB + sweep range. Kept as a test/helper
    /// constructor for simple horizontal platforms; runtime LDtk conversion now
    /// goes through `MovingPlatformSpec` (see same module) so optional
    /// `path_id` references can be resolved against the active area's
    /// `KinematicPathSpec` index.
    pub fn from_authored(start_pos: ae::Vec2, size: ae::Vec2, sweep_dx: f32, speed: f32) -> Self {
        Self::from_sweep(
            "moving_platform",
            "Moving Platform",
            start_pos,
            size,
            sweep_dx,
            speed,
        )
    }

    pub fn from_sweep(
        id: impl Into<String>,
        name: impl Into<String>,
        start_pos: ae::Vec2,
        size: ae::Vec2,
        sweep_dx: f32,
        speed: f32,
    ) -> Self {
        let (min_x, max_x) = if sweep_dx >= 0.0 {
            (start_pos.x, start_pos.x + sweep_dx)
        } else {
            (start_pos.x + sweep_dx, start_pos.x)
        };
        let dir = if sweep_dx >= 0.0 { 1.0 } else { -1.0 };
        Self {
            id: id.into(),
            name: name.into(),
            pos: start_pos,
            size,
            motion: MovingPlatformMotion::Sweep {
                min_x,
                max_x,
                speed: speed.max(0.0),
                dir,
            },
            last_delta: ae::Vec2::ZERO,
        }
    }

    /// A wrapping vertical loop between `min_y` and `max_y`.
    ///
    /// `speed` is magnitude; `downward` picks the direction. A run of these with
    /// staggered `start_pos` values along the same span is the elevator shaft.
    ///
    ///  `downward` rather than `rising`, because +y is DOWN. The first
    /// version of this signature said `rising` and set `dir = +1` for it, which
    /// would have had every authored elevator travel the opposite way to its
    /// field's name — silent, and only visible by watching the game.
    pub fn from_vertical_loop(
        id: impl Into<String>,
        name: impl Into<String>,
        start_pos: ae::Vec2,
        size: ae::Vec2,
        min_y: f32,
        max_y: f32,
        speed: f32,
        downward: bool,
    ) -> Self {
        let (min_y, max_y) = if min_y <= max_y {
            (min_y, max_y)
        } else {
            (max_y, min_y)
        };
        Self {
            id: id.into(),
            name: name.into(),
            pos: start_pos,
            size,
            motion: MovingPlatformMotion::Loop {
                min_y,
                max_y,
                speed: speed.max(0.0),
                dir: if downward { 1.0 } else { -1.0 },
            },
            last_delta: ae::Vec2::ZERO,
        }
    }

    pub fn from_path(
        id: impl Into<String>,
        name: impl Into<String>,
        size: ae::Vec2,
        path: ambition_platformer2d_core::KinematicPath,
    ) -> Self {
        let pos = path.points.first().copied().unwrap_or(ae::Vec2::ZERO);
        Self {
            id: id.into(),
            name: name.into(),
            pos,
            size,
            motion: MovingPlatformMotion::Path {
                path,
                segment: 0,
                dir: 1,
            },
            last_delta: ae::Vec2::ZERO,
        }
    }

    /// Displacement applied by the most recent [`Self::update`] advance. Read by
    /// the per-entity player tick (platform-ride / ledge-carry) so the advance can
    /// run once per frame instead of being interleaved with per-body logic.
    pub fn last_delta(&self) -> ae::Vec2 {
        self.last_delta
    }

    /// Advance the platform and return its displacement this frame. Also records
    /// it as [`Self::last_delta`] for readers that run after the advance.
    pub fn update(&mut self, dt: f32) -> ae::Vec2 {
        let old = self.pos;
        //  a WRAP is a position change that is not a MOVEMENT, and
        // `last_delta` is the quantity a rider is carried by. An arm that
        // teleports must say what it actually travelled, or `pos - old` hands the
        // rider the whole span in one frame — in the direction opposite to
        // travel. Only the wrapping arm needs this; the reversing ones move
        // continuously, so their position difference IS their travel.
        let mut carried: Option<ae::Vec2> = None;
        match &mut self.motion {
            MovingPlatformMotion::Sweep {
                min_x,
                max_x,
                speed,
                dir,
            } => {
                self.pos.x += *speed * *dir * dt;
                if self.pos.x > *max_x {
                    self.pos.x = *max_x;
                    *dir = -1.0;
                } else if self.pos.x < *min_x {
                    self.pos.x = *min_x;
                    *dir = 1.0;
                }
            }
            MovingPlatformMotion::Path { path, segment, dir } => {
                self.pos = advance_path_position(path, segment, dir, self.pos, dt);
            }
            MovingPlatformMotion::Loop {
                min_y,
                max_y,
                speed,
                dir,
            } => {
                let step = ae::Vec2::new(0.0, *speed * *dir * dt);
                self.pos += step;
                let span = *max_y - *min_y;
                if span > 0.0 {
                    if self.pos.y > *max_y {
                        self.pos.y -= span;
                    } else if self.pos.y < *min_y {
                        self.pos.y += span;
                    }
                }
                // The TRAVEL, never the teleport.
                carried = Some(step);
            }
        }
        self.last_delta = carried.unwrap_or(self.pos - old);
        self.last_delta
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    /// The shaft a vertically-LOOPING platform runs in, as `(min_y, max_y)`.
    ///
    /// `None` for every other motion — a sweep and a path REVERSE at their limits, which is
    /// visible on purpose.
    pub fn vertical_loop_span(&self) -> Option<(f32, f32)> {
        match self.motion {
            MovingPlatformMotion::Loop { min_y, max_y, .. } => Some((min_y, max_y)),
            _ => None,
        }
    }

    /// Direction of travel, +1 or -1. For path-driven platforms this reports
    /// the playback direction (not a local tangent sign), which is enough for
    /// trace/HUD readers that want to surface motion phase.
    pub fn direction(&self) -> f32 {
        match &self.motion {
            MovingPlatformMotion::Sweep { dir, .. } => *dir,
            MovingPlatformMotion::Path { dir, .. } => *dir as f32,
            MovingPlatformMotion::Loop { dir, .. } => *dir,
        }
    }

    /// The collision face this platform presents this frame. Moving platforms are always
    /// two-axis `BlinkWall{Soft}` solids. One-way motion requires a frame-consistent crossing
    /// rule because the existing one-way test compares previous feet with the current support
    /// face; do not expose one-way authored motion until that rule exists.
    pub fn as_collision_block(&self) -> ae::Block {
        ae::Block {
            // The platform's LDtk iid IS its durable identity (§3.6
            // `GeoSource::Placement`) — the CC6 portal host ref resolves
            // moving hosts through it per frame.
            id: ae::GeoId::placement(ae::PlacementId::new(self.id.clone()), 0),
            name: self.name.clone(),
            aabb: self.aabb(),
            // This frame's displacement — the collision sweep carries any body
            // resting on this platform by it, so riding is emergent + uniform.
            velocity: self.last_delta,
            // Moving platforms are ordinary solids for walking/riding because
            // `BlockKind::BlinkWall` still resolves as solid collision on both
            // axes. They are deliberately *not* hard blink blockers: if the
            // player has the soft blink-through upgrade, blink pathing may pass
            // through the moving platform just like a soft blink membrane.
            kind: ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Soft,
            },
            art_color: None,
        }
    }

    /// The platform AABB before the latest [`Self::update`] displacement.
    ///
    /// Moving platforms advance once near the beginning of the frame, before the
    /// per-body simulation phase, so a contact fact stored last tick still
    /// describes where the platform WAS. [`Self::is_supporting_body`] matches
    /// both poses for that reason.
    pub fn previous_aabb(&self) -> ae::Aabb {
        self.aabb().translated(-self.last_delta)
    }

    /// Detect whether a body is supported by this platform under the active
    /// acceleration frame.
    ///
    /// `on_ground` remains a relative term: the caller has already decided that
    /// the body's feet are supported this frame. This helper answers whether this
    /// moving platform is the support by comparing the body's feet face to the
    /// platform's anti-feet/head face in side/down coordinates.
    pub fn is_supporting_body(
        &self,
        body: ae::Aabb,
        on_ground: bool,
        gravity_dir: ae::Vec2,
    ) -> bool {
        if !on_ground {
            return false;
        }
        support_contact_matches(body, self.aabb(), gravity_dir)
            || support_contact_matches(body, self.previous_aabb(), gravity_dir)
    }

    /// TODO(compat-remove): migrate trace callers to [`Self::is_supporting_body`] and delete
    /// this down-gravity wrapper.
    pub fn is_riding(&self, player_box: ae::Aabb, on_ground: bool) -> bool {
        self.is_supporting_body(player_box, on_ground, ae::Vec2::new(0.0, 1.0))
    }

}

fn projected_half(half: ae::Vec2, axis: ae::Vec2) -> f32 {
    half.x * axis.x.abs() + half.y * axis.y.abs()
}

fn side_overlap_len(a: ae::Aabb, b: ae::Aabb, frame: ae::AccelerationFrame) -> f32 {
    let a_center = a.center().dot(frame.side);
    let b_center = b.center().dot(frame.side);
    let a_half = projected_half(a.half_size(), frame.side);
    let b_half = projected_half(b.half_size(), frame.side);
    (a_center + a_half).min(b_center + b_half) - (a_center - a_half).max(b_center - b_half)
}

fn support_contact_matches(body: ae::Aabb, support: ae::Aabb, gravity_dir: ae::Vec2) -> bool {
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let overlap = side_overlap_len(body, support, frame);
    if overlap <= 3.0 {
        return false;
    }
    let body_down = body.center().dot(frame.down);
    let support_down = support.center().dot(frame.down);
    let body_feet = body_down + projected_half(body.half_size(), frame.down);
    let support_head = support_down - projected_half(support.half_size(), frame.down);
    (body_feet - support_head).abs() <= 6.0
}

fn advance_path_position(
    path: &ambition_platformer2d_core::KinematicPath,
    segment: &mut usize,
    dir: &mut i32,
    mut pos: ae::Vec2,
    dt: f32,
) -> ae::Vec2 {
    if !path.is_valid() || dt <= 0.0 {
        return pos;
    }
    let mut remaining = path.speed * dt;
    while remaining > 0.0 {
        let target_index = path_target_index(path, *segment, *dir);
        let Some(target) = path.points.get(target_index).copied() else {
            break;
        };
        let to_target = target - pos;
        let distance = to_target.length();
        if distance <= 0.001 {
            // This branch consumes no `remaining`; if advancing leaves the cursor
            // unchanged, stop rather than spinning forever on a zero-distance
            // target.
            let before = (*segment, *dir);
            advance_path_segment(path, segment, dir);
            if (*segment, *dir) == before {
                break;
            }
            continue;
        }
        let step = remaining.min(distance);
        pos += to_target / distance * step;
        remaining -= step;
        if step >= distance - 0.001 {
            advance_path_segment(path, segment, dir);
        }
    }
    pos
}

/// The waypoint a cursor is heading for.
///
/// `Loop` closes the circuit: a path of `n` points has `n` segments, including
/// the closing leg `p[n-1] → p[0]`.
///
///  reverse (`dir < 0`) is left un-wrapped deliberately: nothing sets a
/// backwards direction under `Loop` — only `PingPong` flips `dir` — so a modulo
/// there would be untested code serving no caller.
fn path_target_index(
    path: &ambition_platformer2d_core::KinematicPath,
    segment: usize,
    dir: i32,
) -> usize {
    if dir >= 0 {
        let next = segment + 1;
        if matches!(
            path.mode,
            ambition_platformer2d_core::KinematicPathMode::Loop
        ) {
            return next % path.points.len().max(1);
        }
        next
    } else {
        segment
    }
}

/// The highest segment index this mode may occupy.
///
/// `Loop` has one more than the others: the closing leg back to the first point.
fn path_last_segment(path: &ambition_platformer2d_core::KinematicPath) -> usize {
    match path.mode {
        ambition_platformer2d_core::KinematicPathMode::Loop => path.points.len().saturating_sub(1),
        _ => path.points.len().saturating_sub(2),
    }
}

fn advance_path_segment(
    path: &ambition_platformer2d_core::KinematicPath,
    segment: &mut usize,
    dir: &mut i32,
) {
    let last_segment = path_last_segment(path);
    match path.mode {
        ambition_platformer2d_core::KinematicPathMode::Once => {
            if *dir >= 0 && *segment < last_segment {
                *segment += 1;
            }
        }
        ambition_platformer2d_core::KinematicPathMode::Loop => {
            if *dir >= 0 {
                *segment = if *segment >= last_segment {
                    0
                } else {
                    *segment + 1
                };
            } else if *segment == 0 {
                *segment = last_segment;
            } else {
                *segment -= 1;
            }
        }
        ambition_platformer2d_core::KinematicPathMode::PingPong => {
            if *dir >= 0 {
                if *segment >= last_segment {
                    *dir = -1;
                } else {
                    *segment += 1;
                }
            } else if *segment == 0 {
                *dir = 1;
            } else {
                *segment -= 1;
            }
        }
    }
}

/// Return the active room's LDtk-authored moving platforms.
///
/// No compatibility platform is synthesized here: if an active area has no
/// `MovingPlatform` entities, the room has no moving platforms. That keeps LDtk
/// as the sole gameplay source of truth for platform placement.
pub fn moving_platforms_for_room(room: &crate::rooms::RoomSpec) -> Vec<MovingPlatformState> {
    room.moving_platforms.clone()
}

/// Return a temporary collision world with all current moving platforms inserted.
///
/// The inserted blocks are solid for normal collision, but blink-passable for
/// upgraded blink pathing. This keeps debug previews, blink destination
/// resolution, and actual movement collision in agreement.
pub fn world_with_moving_platforms(
    world: &ae::World,
    platforms: &[MovingPlatformState],
) -> ae::World {
    let mut collision_world = world.clone();
    collision_world.blocks.extend(
        platforms
            .iter()
            .map(MovingPlatformState::as_collision_block),
    );
    collision_world
}

#[cfg(test)]
mod tests;
