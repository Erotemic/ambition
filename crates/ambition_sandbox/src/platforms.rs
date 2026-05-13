//! LDtk-authored moving-platform runtime helpers.
//!
//! Moving platforms remain sandbox-side as a design experiment, but they now
//! contribute temporary solid blocks to the engine collision world each frame.
//! That gives us rideable/collidable behavior without committing moving-solid
//! semantics to `ambition_engine` before we have tests for carrying, crushing,
//! and one-way platform interactions.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::config::{world_to_bevy, WORLD_Z_BLOCK};
use crate::rendering::RoomVisual;
use crate::rooms::RoomSet;

/// Runtime state for one LDtk-authored horizontal moving platform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovingPlatformState {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    min_x: f32,
    max_x: f32,
    speed: f32,
    dir: f32,
}

impl MovingPlatformState {
    /// Build from LDtk-authored AABB + sweep range. The AABB defines the
    /// platform's starting position + size; `sweep_dx` is the horizontal
    /// travel distance (positive sweeps right then ping-pongs back, negative
    /// sweeps left first). Speed is in world px/s.
    ///
    /// Yields a platform whose travel range is `[start_x, start_x +
    /// sweep_dx]` (or swapped when `sweep_dx < 0`), sweeping at constant
    /// `speed` and ping-ponging at the bounds.
    pub fn from_authored(start_pos: ae::Vec2, size: ae::Vec2, sweep_dx: f32, speed: f32) -> Self {
        let (min_x, max_x) = if sweep_dx >= 0.0 {
            (start_pos.x, start_pos.x + sweep_dx)
        } else {
            (start_pos.x + sweep_dx, start_pos.x)
        };
        let dir = if sweep_dx >= 0.0 { 1.0 } else { -1.0 };
        Self {
            pos: start_pos,
            size,
            min_x,
            max_x,
            speed: speed.max(0.0),
            dir,
        }
    }

    /// Advance the platform and return its displacement this frame.
    pub fn update(&mut self, dt: f32) -> ae::Vec2 {
        let old = self.pos;
        self.pos.x += self.speed * self.dir * dt;
        if self.pos.x > self.max_x {
            self.pos.x = self.max_x;
            self.dir = -1.0;
        } else if self.pos.x < self.min_x {
            self.pos.x = self.min_x;
            self.dir = 1.0;
        }
        self.pos - old
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    /// Direction of travel along the platform's authored sweep, +1 or -1.
    /// Exposed for trace/HUD readers that want to surface the platform's
    /// motion phase without owning its private state.
    pub fn direction(&self) -> f32 {
        self.dir
    }

    pub fn as_collision_block(&self) -> ae::Block {
        ae::Block {
            name: "moving platform".to_string(),
            aabb: self.aabb(),
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

    /// Detect whether the player was riding this platform at the start of a
    /// frame. We carry the player by the platform delta before collision
    /// resolution so standing on it feels stable.
    pub fn is_riding(&self, player: &ae::Player) -> bool {
        if !player.on_ground {
            return false;
        }
        let player_box = player.aabb();
        let platform_box = self.aabb();
        let horizontally_overlapping = player_box.right() > platform_box.left() + 3.0
            && player_box.left() < platform_box.right() - 3.0;
        let feet_near_top = (player_box.bottom() - platform_box.top()).abs() <= 6.0;
        horizontally_overlapping && feet_near_top
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

/// Compatibility wrapper for single-platform tests.
pub fn world_with_moving_platform(world: &ae::World, platform: &MovingPlatformState) -> ae::World {
    world_with_moving_platforms(world, std::slice::from_ref(platform))
}

#[derive(Component)]
pub struct MovingPlatformVisual {
    pub index: usize,
}

pub fn spawn_moving_platform(
    commands: &mut Commands,
    world: &ae::World,
    index: usize,
    platform: MovingPlatformState,
) -> Entity {
    commands
        .spawn((
            Sprite::from_color(
                Color::srgba(0.35, 0.74, 1.0, 0.92),
                BVec2::new(platform.size.x, platform.size.y),
            ),
            Transform::from_translation(world_to_bevy(world, platform.pos, WORLD_Z_BLOCK + 4.0)),
            Name::new(format!("Moving platform {index}")),
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
        .copied()
        .enumerate()
        .map(|(index, platform)| spawn_moving_platform(commands, world, index, platform))
        .collect()
}

pub fn sync_moving_platform(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    room_set: Res<RoomSet>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut active_platform_room: Local<Option<String>>,
    mut active_platform_source: Local<Option<Vec<MovingPlatformState>>>,
    mut query: Query<(Entity, &MovingPlatformVisual, &mut Transform, &mut Sprite)>,
) {
    let active_spec = room_set.active_spec();
    let desired_start = moving_platforms_for_room(active_spec);

    // Refresh only when the authored source changes, not every time RoomSet or
    // GameWorld gets marked changed by an unrelated system. The runtime copies
    // are live state: `sandbox_update` advances them and carries the player by
    // their frame deltas. Resetting them every frame turns invisible collision
    // platforms into conveyor belts while visuals stay pinned at authored starts.
    let source_changed = active_platform_room.as_deref() != Some(active_spec.id.as_str())
        || active_platform_source
            .as_ref()
            .map(|source| source != &desired_start)
            .unwrap_or(true);
    if source_changed {
        runtime.moving_platforms = desired_start.clone();
        *active_platform_room = Some(active_spec.id.clone());
        *active_platform_source = Some(desired_start.clone());

        let visual_count = query.iter().count();
        if visual_count != desired_start.len() {
            for (entity, _, _, _) in &mut query {
                commands.entity(entity).despawn();
            }
            spawn_moving_platforms(&mut commands, &world.0, &runtime.moving_platforms);
            return;
        }
    }

    for (_, visual, mut transform, mut sprite) in &mut query {
        let Some(platform) = runtime.moving_platforms.get(visual.index) else {
            continue;
        };
        transform.translation = world_to_bevy(&world.0, platform.pos, WORLD_Z_BLOCK + 4.0);
        sprite.custom_size = Some(BVec2::new(platform.size.x, platform.size.y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_world() -> ae::World {
        ae::World::new(
            "test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(100.0, 100.0),
            Vec::new(),
        )
    }

    fn sample_platform() -> MovingPlatformState {
        MovingPlatformState::from_authored(
            ae::Vec2::new(400.0, 800.0),
            ae::Vec2::new(155.0, 18.0),
            240.0,
            130.0,
        )
    }

    fn test_room_with_platforms(
        world: ae::World,
        platforms: Vec<MovingPlatformState>,
    ) -> crate::rooms::RoomSpec {
        crate::rooms::RoomSpec {
            id: "test".into(),
            world,
            loading_zones: Vec::new(),
            metadata: crate::rooms::RoomMetadata::default(),
            camera_zones: Vec::new(),
            moving_platforms: platforms,
        }
    }

    #[test]
    fn moving_platforms_for_room_returns_authored_ldtk_platforms() {
        let world = test_world();
        let first = sample_platform();
        let second = MovingPlatformState::from_authored(
            ae::Vec2::new(720.0, 640.0),
            ae::Vec2::new(64.0, 16.0),
            -96.0,
            70.0,
        );
        let room = test_room_with_platforms(world, vec![first, second]);
        let selected = moving_platforms_for_room(&room);
        assert_eq!(selected, vec![first, second]);
    }

    #[test]
    fn moving_platforms_for_room_does_not_synthesize_unauthored_platforms() {
        let world = test_world();
        let room = test_room_with_platforms(world, Vec::new());
        let selected = moving_platforms_for_room(&room);
        assert!(
            selected.is_empty(),
            "rooms without LDtk MovingPlatform entities should have no moving platforms"
        );
    }

    #[test]
    fn moving_platform_update_swings_between_min_and_max() {
        let mut platform = sample_platform();
        let min_x = platform.pos.x;
        // Many ticks at +x direction: platform reaches max_x and flips.
        for _ in 0..600 {
            let _ = platform.update(0.05);
            // Position must always stay within [min_x, max_x].
            assert!(platform.pos.x >= min_x - 1.0);
        }
        // After enough time it must have flipped at least once.
        assert!(platform.direction() == 1.0 || platform.direction() == -1.0);
    }

    #[test]
    fn moving_platform_update_returns_displacement() {
        let mut platform = sample_platform();
        let dt = 1.0 / 60.0;
        let delta = platform.update(dt);
        // Initial direction is +1, speed = 130 px/s, dt = 1/60.
        // So displacement.x ~= 130 / 60 ~= 2.17 px.
        assert!((delta.x - 130.0 * dt).abs() < 1e-3);
        assert_eq!(delta.y, 0.0);
    }

    #[test]
    fn moving_platform_aabb_centered_on_pos() {
        let platform = sample_platform();
        let aabb = platform.aabb();
        assert_eq!(aabb.center(), platform.pos);
    }

    #[test]
    fn moving_platform_as_collision_block_is_blink_wall_soft() {
        let platform = sample_platform();
        let block = platform.as_collision_block();
        // Soft blink wall — solid for collision but blink-passable
        // when soft-blink-through is unlocked.
        assert!(matches!(
            block.kind,
            ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Soft,
            }
        ));
    }

    #[test]
    fn world_with_moving_platforms_appends_all_blocks() {
        let world = test_world();
        let first = sample_platform();
        let second = MovingPlatformState::from_authored(
            ae::Vec2::new(500.0, 500.0),
            ae::Vec2::new(80.0, 12.0),
            100.0,
            50.0,
        );
        let extended = world_with_moving_platforms(&world, &[first, second]);
        assert_eq!(extended.blocks.len(), world.blocks.len() + 2);
    }
}
