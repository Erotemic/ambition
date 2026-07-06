//! LDtk-authored moving-platform runtime helpers.
//!
//! Moving platforms remain sandbox-side as a design experiment, but they now
//! contribute temporary solid blocks to the engine collision world each frame.
//! That gives us rideable/collidable behavior without committing moving-solid
//! semantics to `ambition_engine_core` before we have tests for carrying, crushing,
//! and one-way platform interactions.

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::platformer_runtime::lifecycle::RoomVisual;
use crate::rooms::{KinematicPathSpec, RoomSet};
use ambition_engine_core::config::{world_to_bevy, WORLD_Z_BLOCK};

/// LDtk-authored moving-platform declaration before path references are resolved.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MovingPlatformSpec {
    pub id: String,
    pub name: String,
    pub start_pos: ae::Vec2,
    pub size: ae::Vec2,
    pub sweep_dx: f32,
    pub speed: f32,
    pub path_id: Option<String>,
}

impl MovingPlatformSpec {
    pub fn from_authored(
        id: impl Into<String>,
        name: impl Into<String>,
        start_pos: ae::Vec2,
        size: ae::Vec2,
        sweep_dx: f32,
        speed: f32,
        path_id: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            start_pos,
            size,
            sweep_dx,
            speed,
            path_id: path_id.and_then(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }),
        }
    }

    pub fn resolve(self, paths: &[KinematicPathSpec]) -> Result<MovingPlatformState, String> {
        if let Some(path_id) = self.path_id.as_deref() {
            let Some(path_spec) = paths.iter().find(|path| path.matches_id(path_id)) else {
                let known = paths
                    .iter()
                    .flat_map(|path| path.aliases())
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
        } else {
            Ok(MovingPlatformState::from_sweep(
                self.id,
                self.name,
                self.start_pos,
                self.size,
                self.sweep_dx,
                self.speed,
            ))
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
    /// Displacement applied by the most recent [`Self::update`] advance. The
    /// platform advance is now a once-per-frame step BEFORE the per-entity player
    /// tick (so it can't multiply when the tick iterates multiple bodies); the
    /// per-entity ride / ledge-carry logic reads this instead of advancing the
    /// platform itself. `ZERO` until the first advance.
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
        path: ambition_engine_core::KinematicPath,
        segment: usize,
        dir: i32,
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

    pub fn from_path(
        id: impl Into<String>,
        name: impl Into<String>,
        size: ae::Vec2,
        path: ambition_engine_core::KinematicPath,
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
        }
        self.last_delta = self.pos - old;
        self.last_delta
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    /// Direction of travel, +1 or -1. For path-driven platforms this reports
    /// the playback direction (not a local tangent sign), which is enough for
    /// trace/HUD readers that want to surface motion phase.
    pub fn direction(&self) -> f32 {
        match &self.motion {
            MovingPlatformMotion::Sweep { dir, .. } => *dir,
            MovingPlatformMotion::Path { dir, .. } => *dir as f32,
        }
    }

    pub fn as_collision_block(&self) -> ae::Block {
        ae::Block {
            id: ae::GeoId::anon(),
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
        }
    }

    /// The platform AABB before the latest [`Self::update`] displacement.
    ///
    /// Moving platforms advance once near the beginning of the frame, before the
    /// per-body simulation phase. A ledge grab contact stored from the previous
    /// tick therefore matches this previous AABB until the carry step translates
    /// the contact by [`Self::last_delta`].
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

    /// Down-gravity compatibility wrapper for legacy trace callers. Prefer
    /// [`Self::is_supporting_body`] when the active acceleration frame is known.
    pub fn is_riding(&self, player_box: ae::Aabb, on_ground: bool) -> bool {
        self.is_supporting_body(player_box, on_ground, ae::Vec2::new(0.0, 1.0))
    }

    /// Detect whether an active ledge-grab contact is latched to this platform.
    ///
    /// Moving platforms are inserted into the collision world as ordinary solid
    /// blocks, so the engine's ledge probe records only geometric contact data.
    /// The sandbox uses this helper after platforms have advanced for the frame;
    /// matching both the current AABB and the previous AABB keeps an existing
    /// hang glued to the edge that moved this tick instead of losing the match
    /// before the carry step translates the stored `LedgeGrabState::contact`.
    pub fn matches_ledge_contact_in_frame(
        &self,
        contact: ae::LedgeContact,
        player_size: ae::Vec2,
        gravity_dir: ae::Vec2,
    ) -> bool {
        ledge_contact_matches_platform(contact, player_size, self.aabb(), gravity_dir)
            || ledge_contact_matches_platform(
                contact,
                player_size,
                self.previous_aabb(),
                gravity_dir,
            )
    }

    /// Down-gravity compatibility wrapper for tests / legacy callers that still
    /// construct ledge contacts in the old normal-gravity frame.
    pub fn matches_ledge_contact(&self, contact: ae::LedgeContact, player_size: ae::Vec2) -> bool {
        self.matches_ledge_contact_in_frame(contact, player_size, ae::Vec2::new(0.0, 1.0))
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

fn ledge_contact_matches_platform(
    contact: ae::LedgeContact,
    player_size: ae::Vec2,
    platform_box: ae::Aabb,
    gravity_dir: ae::Vec2,
) -> bool {
    if contact.wall_normal_x.abs() < 0.5 {
        return false;
    }
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let half = player_size * 0.5;
    let side_normal = contact.wall_normal_x.signum();
    let platform_half = platform_box.half_size();
    let platform_center = platform_box.center();
    let platform_side = platform_center.dot(frame.side);
    let platform_down = platform_center.dot(frame.down);
    let platform_side_half = projected_half(platform_half, frame.side);
    let platform_down_half = projected_half(platform_half, frame.down);
    let lip_down = platform_down - platform_down_half;
    let wall_side = platform_side + side_normal * platform_side_half;

    // Invert the local-frame formulas from
    // ambition_engine_core::ledge_grab::probe_ledge_grab_in_frame. These are expressed in
    // side/down coordinates so the same check works for down, up, left, and right
    // gravity wells.
    let expected_anchor_side = wall_side + side_normal * (half.x - 1.0);
    let expected_anchor_down = lip_down + half.y - 4.0;
    let expected_climb_side = wall_side - side_normal * (half.x + 4.0);
    let expected_climb_down = lip_down - half.y - 1.0;

    let anchor_side = contact.anchor.dot(frame.side);
    let anchor_down = contact.anchor.dot(frame.down);
    let climb_side = contact.climb_target.dot(frame.side);
    let climb_down = contact.climb_target.dot(frame.down);

    const TOL: f32 = 8.0;
    if (anchor_side - expected_anchor_side).abs() > TOL
        || (anchor_down - expected_anchor_down).abs() > TOL
        || (climb_side - expected_climb_side).abs() > TOL
        || (climb_down - expected_climb_down).abs() > TOL
    {
        return false;
    }

    // The climb target should be inboard of this platform, not on an unrelated
    // block sharing the same lip/edge coordinate.
    climb_side >= platform_side - platform_side_half - half.x - 12.0
        && climb_side <= platform_side + platform_side_half + half.x + 12.0
}

fn advance_path_position(
    path: &ambition_engine_core::KinematicPath,
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
        let target_index = if *dir >= 0 { *segment + 1 } else { *segment };
        let Some(target) = path.points.get(target_index).copied() else {
            break;
        };
        let to_target = target - pos;
        let distance = to_target.length();
        if distance <= 0.001 {
            advance_path_segment(path, segment, dir);
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

fn advance_path_segment(
    path: &ambition_engine_core::KinematicPath,
    segment: &mut usize,
    dir: &mut i32,
) {
    let last_segment = path.points.len().saturating_sub(2);
    match path.mode {
        ambition_engine_core::KinematicPathMode::Once => {
            if *dir >= 0 && *segment < last_segment {
                *segment += 1;
            }
        }
        ambition_engine_core::KinematicPathMode::Loop => {
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
        ambition_engine_core::KinematicPathMode::PingPong => {
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

#[derive(Component)]
pub struct MovingPlatformVisual {
    pub index: usize,
}

pub fn spawn_moving_platform(
    commands: &mut Commands,
    world: &ae::World,
    index: usize,
    platform: &MovingPlatformState,
) -> Entity {
    commands
        .spawn((
            Sprite::from_color(
                Color::srgba(0.35, 0.74, 1.0, 0.92),
                BVec2::new(platform.size.x, platform.size.y),
            ),
            Transform::from_translation(world_to_bevy(world, platform.pos, WORLD_Z_BLOCK + 4.0)),
            Name::new(format!("Moving platform {index}: {}", platform.name)),
            MovingPlatformVisual { index },
            RoomVisual,
        ))
        .id()
}

pub fn spawn_moving_platforms(
    commands: &mut Commands,
    world: &ae::World,
    platforms: &[MovingPlatformState],
) -> Vec<Entity> {
    platforms
        .iter()
        .enumerate()
        .map(|(index, platform)| spawn_moving_platform(commands, world, index, platform))
        .collect()
}

pub fn sync_moving_platform(
    mut commands: Commands,
    world: Res<ambition_engine_core::RoomGeometry>,
    room_set: Res<RoomSet>,
    mut platform_set: ResMut<crate::MovingPlatformSet>,
    mut active_platform_room: Local<Option<String>>,
    mut active_platform_source: Local<Option<Vec<MovingPlatformState>>>,
    mut query: Query<(Entity, &MovingPlatformVisual, &mut Transform, &mut Sprite)>,
) {
    let active_spec = room_set.active_spec();
    let desired_start = moving_platforms_for_room(active_spec);

    // Refresh only when the authored source changes, not every time RoomSet or
    // RoomGeometry gets marked changed by an unrelated system. The runtime copies
    // are live state: the player tick advances them and carries the player by
    // their frame deltas. Resetting them every frame turns invisible collision
    // platforms into conveyor belts while visuals stay pinned at authored starts.
    let source_changed = active_platform_room.as_deref() != Some(active_spec.id.as_str())
        || active_platform_source
            .as_ref()
            .map(|source| source != &desired_start)
            .unwrap_or(true);
    if source_changed {
        platform_set.0 = desired_start.clone();
        *active_platform_room = Some(active_spec.id.clone());
        *active_platform_source = Some(desired_start.clone());

        let visual_count = query.iter().count();
        if visual_count != desired_start.len() {
            for (entity, _, _, _) in &mut query {
                commands.entity(entity).despawn();
            }
            spawn_moving_platforms(&mut commands, &world.0, &platform_set.0);
            return;
        }
    }

    for (_, visual, mut transform, mut sprite) in &mut query {
        let Some(platform) = platform_set.0.get(visual.index) else {
            continue;
        };
        transform.translation = world_to_bevy(&world.0, platform.pos, WORLD_Z_BLOCK + 4.0);
        sprite.custom_size = Some(BVec2::new(platform.size.x, platform.size.y));
    }
}

#[cfg(test)]
mod tests;
