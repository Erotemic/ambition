//! Spawn-point resolution: where the player lands after a room transition.
//!
//! `validated_spawn` clamps/repairs a proposed spawn out of walls and solids;
//! `arrival_from_target_zone` picks the landing point per `LoadingZoneActivation`
//! (edge inset for `EdgeExit`, target-zone center for `Door`/`Walk`). Helpers
//! and the player-half/inset constants are `pub(super)` for sibling `graph`.

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;

use super::{LoadingZone, LoadingZoneActivation};

pub(super) fn block_kind_label(kind: ae::BlockKind) -> &'static str {
    match kind {
        ae::BlockKind::Solid => "solid",
        ae::BlockKind::BlinkWall { .. } => "blink wall",
        ae::BlockKind::OneWay => "one-way platform",
        ae::BlockKind::Hazard => "hazard",
        ae::BlockKind::PogoOrb => "pogo orb",
        ae::BlockKind::Rebound { .. } => "rebound pad",
    }
}

pub(super) const WALL: f32 = 36.0;
pub(super) const EDGE_ARRIVAL_INSET: f32 = 92.0;
pub(super) const PLAYER_HALF_W: f32 = 14.0;
pub(super) const PLAYER_HALF_H: f32 = 23.0;
pub(super) const SPAWN_MARGIN: f32 = 3.0;

pub(super) fn arrival_from_target_zone(world: &ae::World, zone: &LoadingZone) -> ae::Vec2 {
    match zone.activation {
        // `Walk` uses the same arrival rule as `Door` — the target
        // zone defines where the player lands, not an edge inset —
        // since `Walk` zones are placed anywhere in the room (not
        // bound to edges).
        LoadingZoneActivation::Door | LoadingZoneActivation::Walk => door_arrival(zone.aabb),
        LoadingZoneActivation::EdgeExit => edge_arrival(world, zone.aabb),
    }
}

fn edge_arrival(world: &ae::World, zone: ae::Aabb) -> ae::Vec2 {
    // Classify by shape: a tall narrow zone is a side seam
    // (left/right edge); a wide short zone is a top/bottom seam.
    // This is how a top-edge zone (player jumps UP through the
    // ceiling and pops into the bottom of the room above) and a
    // bottom-edge zone are distinguished from the historical
    // side-scroll left/right exits without requiring the author to
    // declare the edge explicitly.
    let zone_w = zone.right() - zone.left();
    let zone_h = zone.bottom() - zone.top();
    if zone_w >= zone_h {
        // Top/bottom edge: inset Y, center X over the zone.
        let y = if zone.top() <= WALL + 1.0 {
            EDGE_ARRIVAL_INSET
        } else if zone.bottom() >= world.size.y - WALL - 1.0 {
            world.size.y - EDGE_ARRIVAL_INSET
        } else {
            zone.center().y
        };
        ae::Vec2::new(zone.center().x, y)
    } else {
        // Left/right edge: inset X, center Y over the zone.
        let x = if zone.left() <= WALL + 1.0 {
            EDGE_ARRIVAL_INSET
        } else if zone.right() >= world.size.x - WALL - 1.0 {
            world.size.x - EDGE_ARRIVAL_INSET
        } else {
            zone.center().x
        };
        ae::Vec2::new(x, zone.center().y)
    }
}

fn door_arrival(zone: ae::Aabb) -> ae::Vec2 {
    ae::Vec2::new(
        zone.center().x,
        zone.bottom() - PLAYER_HALF_H - SPAWN_MARGIN,
    )
}

/// Clamp and repair a proposed player spawn so transitions never place the
/// player outside the room or embedded in solids.
pub fn validated_spawn(world: &ae::World, desired: ae::Vec2, player_size: ae::Vec2) -> ae::Vec2 {
    let half = player_size * 0.5;
    let base = clamp_spawn_to_room(world, desired, half);
    if player_body_clear(world, base, half) {
        return base;
    }

    const STEP: f32 = 8.0;
    for y_step in 0..=96 {
        let dy = -(y_step as f32) * STEP;
        for x_step in 0..=96 {
            if x_step == 0 {
                let candidate =
                    clamp_spawn_to_room(world, ae::Vec2::new(base.x, base.y + dy), half);
                if player_body_clear(world, candidate, half) {
                    return candidate;
                }
            } else {
                for sign in [-1.0_f32, 1.0] {
                    let dx = sign * x_step as f32 * STEP;
                    let candidate =
                        clamp_spawn_to_room(world, ae::Vec2::new(base.x + dx, base.y + dy), half);
                    if player_body_clear(world, candidate, half) {
                        return candidate;
                    }
                }
            }
        }
    }

    base
}

fn clamp_spawn_to_room(world: &ae::World, pos: ae::Vec2, half: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new(
        pos.x
            .clamp(half.x + SPAWN_MARGIN, world.size.x - half.x - SPAWN_MARGIN),
        pos.y
            .clamp(half.y + SPAWN_MARGIN, world.size.y - half.y - SPAWN_MARGIN),
    )
}

fn player_body_clear(world: &ae::World, center: ae::Vec2, half: ae::Vec2) -> bool {
    let body = ae::Aabb::new(center, half);
    !world.body_overlaps_any(body, |block| {
        matches!(
            block.kind,
            ae::BlockKind::Solid
                | ae::BlockKind::BlinkWall { .. }
                | ae::BlockKind::OneWay
                | ae::BlockKind::Hazard
                | ae::BlockKind::Rebound { .. }
        )
    })
}

#[cfg(test)]
mod spawn_tests {
    //! Spawn placement safety: clamping into the room and the spiral
    //! search that nudges a player out of a solid. Embedding the player
    //! in a solid on a room transition is a real bug class (there's even
    //! a CI guard for authored spawns), so the repair path is pinned.
    use super::*;

    fn world_with(blocks: Vec<ae::Block>) -> ae::World {
        ae::World::new(
            "test",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::new(50.0, 50.0),
            blocks,
        )
    }

    fn solid_filter(b: &ae::Block) -> bool {
        matches!(b.kind, ae::BlockKind::Solid)
    }

    #[test]
    fn door_arrival_sits_just_below_the_zone() {
        let zone = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(20.0, 40.0));
        let pos = door_arrival(zone);
        assert_eq!(pos.x, zone.center().x);
        assert_eq!(pos.y, zone.bottom() - PLAYER_HALF_H - SPAWN_MARGIN);
        assert!(pos.y < zone.bottom(), "arrival is above the doorway sill");
    }

    #[test]
    fn clamp_keeps_spawn_inside_the_room_margins() {
        let world = world_with(Vec::new());
        let half = ae::Vec2::new(14.0, 23.0);
        let clamped = clamp_spawn_to_room(&world, ae::Vec2::new(-100.0, 1000.0), half);
        assert!(clamped.x >= half.x + SPAWN_MARGIN);
        assert!(clamped.y >= half.y + SPAWN_MARGIN);
        assert!(clamped.x <= world.size.x - half.x - SPAWN_MARGIN);
        assert!(clamped.y <= world.size.y - half.y - SPAWN_MARGIN);
    }

    #[test]
    fn validated_spawn_in_open_room_keeps_a_valid_point() {
        let world = world_with(Vec::new());
        let desired = ae::Vec2::new(200.0, 200.0);
        let got = validated_spawn(&world, desired, ae::Vec2::new(28.0, 46.0));
        assert_eq!(
            got, desired,
            "an already-clear in-bounds spawn is unchanged"
        );
    }

    #[test]
    fn validated_spawn_pushes_the_player_out_of_a_solid() {
        // Solid covering (100,100)..(200,200); desired is dead center.
        let block = ae::Block::solid(
            "wall",
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(100.0, 100.0),
        );
        let world = world_with(vec![block]);
        let player_size = ae::Vec2::new(28.0, 46.0);
        let half = player_size * 0.5;
        let got = validated_spawn(&world, ae::Vec2::new(150.0, 150.0), player_size);
        let body = ae::Aabb::new(got, half);
        assert!(
            !world.body_overlaps_any(body, solid_filter),
            "validated spawn must not embed the player in a solid (got {got:?})",
        );
    }

    #[test]
    fn block_kind_label_is_nonempty() {
        assert!(!block_kind_label(ae::BlockKind::Solid).is_empty());
    }
}
