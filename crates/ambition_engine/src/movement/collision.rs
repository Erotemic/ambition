use crate::geometry::{Aabb, AabbExt};
use crate::world::{BlockKind, World};
use crate::Vec2;

use super::player::Player;
use super::tuning::MovementTuning;

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

fn is_solid_for_axis(kind: BlockKind, axis: Axis) -> bool {
    match kind {
        BlockKind::Solid | BlockKind::BlinkWall { .. } => true,
        BlockKind::OneWay => matches!(axis, Axis::Y),
        BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => false,
    }
}

fn block_passable_during_climb(player: &Player, block: &crate::world::Block) -> bool {
    if !matches!(player.body_mode, crate::player_state::BodyMode::Climbing) {
        return false;
    }
    let Some(contact) = player.climbable_contact else {
        return false;
    };
    if matches!(block.kind, BlockKind::Hazard) {
        return false;
    }
    contact.region_aabb.strict_intersects(block.aabb)
}

fn sweep_fraction(time_of_impact: f32) -> f32 {
    time_of_impact.clamp(0.0, 1.0)
}

pub(super) fn sweep_player_x(world: &World, player: &mut Player, delta_x: f32) {
    let delta = Vec2::new(delta_x, 0.0);
    if delta.x.abs() <= 1.0e-5 {
        resolve_axis(world, player, Axis::X);
        return;
    }

    if let Some(hit) = world.first_body_sweep(player.aabb(), delta, |block| {
        is_solid_for_axis(block.kind, Axis::X)
            && !matches!(block.kind, BlockKind::OneWay)
            && !block_passable_during_climb(player, block)
    }) {
        let toi_fraction = sweep_fraction(hit.time_of_impact);
        player.pos.x += delta.x * toi_fraction;
        let body = player.aabb();
        let immediate_contact = hit.time_of_impact <= 1.0e-5;
        let overlap_x = (body.right().min(hit.block.aabb.right())
            - body.left().max(hit.block.aabb.left()))
        .max(0.0);
        let overlap_y = (body.bottom().min(hit.block.aabb.bottom())
            - body.top().max(hit.block.aabb.top()))
        .max(0.0);
        // Skip the horizontal snap in two failure-mode cases:
        // 1. The contact is dominantly *vertical* (player's head poking
        //    into a wide ceiling, or feet poking into a wide floor). The
        //    perpendicular `resolve_vertical` pass owns this contact;
        //    pushing horizontally toward the block's far edge would
        //    catapult the player across the entire room.
        // 2. The contact is dominantly horizontal but the player is
        //    *already moving away* from the block (e.g. wall-jump pushed
        //    them off a wall they were sub-pixel-penetrating). The
        //    delta-direction snap logic uses delta.x sign to pick a face;
        //    when the player is on the far side from where delta.x points
        //    that pick is wrong and pushes them through the block.
        let vertical_dominant = immediate_contact && overlap_y > 0.0 && overlap_x > overlap_y;
        let body_to_right_of_block = body.center().x > hit.block.aabb.center().x;
        let moving_away_from_block =
            (body_to_right_of_block && delta.x > 0.0) || (!body_to_right_of_block && delta.x < 0.0);
        let horizontal_overlap_moving_away =
            immediate_contact && overlap_x > 0.0 && moving_away_from_block;
        if vertical_dominant || horizontal_overlap_moving_away {
            player.pos.x += delta.x * (1.0 - toi_fraction);
        } else {
            // Pick the snap face from the player's *position relative to
            // the block*, not from delta.x sign. The two only agree when
            // the player is approaching from the side delta.x implies;
            // for a pre-existing overlap they can disagree, which is the
            // tunneling failure mode addressed above.
            if body_to_right_of_block {
                player.pos.x += hit.block.aabb.right() - body.left();
                player.wall_normal_x = 1.0;
            } else {
                player.pos.x += hit.block.aabb.left() - body.right();
                player.wall_normal_x = -1.0;
            }
            player.vel.x = 0.0;
            player.on_wall = true;
        }
    } else {
        player.pos.x += delta.x;
    }

    // Shape casts catch fast motion; positional resolution remains as a cheap
    // penetration repair for starts inside geometry or stacked contacts.
    resolve_axis(world, player, Axis::X);
}

pub(super) fn sweep_player_y(
    world: &World,
    player: &mut Player,
    delta_y: f32,
    prev_bottom: f32,
    drop_through: bool,
) {
    let delta = Vec2::new(0.0, delta_y);
    if delta.y.abs() <= 1.0e-5 {
        resolve_vertical(world, player, prev_bottom, drop_through);
        return;
    }

    let start_body = player.aabb();
    if let Some(hit) = world.first_body_sweep(player.aabb(), delta, |block| {
        if !is_solid_for_axis(block.kind, Axis::Y) {
            return false;
        }
        if block_passable_during_climb(player, block) {
            return false;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            let landing_from_above = delta.y >= 0.0 && prev_bottom <= block.aabb.top() + 8.0;
            return landing_from_above && !drop_through;
        }
        // AMBITION_REVIEW(spatial): reject side-wall contacts before asking
        // Parry for a y-sweep. Shape casts can report TOI=0 for boxes that are
        // merely edge-touching on x while the player's y-range is fully nested
        // inside a tall wall. The vertical sweep cannot resolve that contact:
        // snapping to the wall top/bottom teleports the player hundreds of
        // pixels in full-world repros. `strict_intersects` below is not enough
        // because the failing trace is exact-edge-touching, not penetrating.
        if body_is_side_contact(start_body, block.aabb) {
            return false;
        }
        // AMBITION_REVIEW(spatial): reject blocks the body is already
        // overlapping. Two repros:
        //
        // 1. Wall-clinging on a tall left-side wall whose top is at
        //    world y=0: a TOI=0 hit on the wall during the downward
        //    y-sweep snapped the body's bottom to the wall's top —
        //    teleporting the player from `(62, 1678)` to `(62, -23)`.
        //    This was the *fully-nested* case (body.y-range inside
        //    wall.y-range), caught by `body_is_side_contact`.
        //
        // 2. Player straddles a column's bottom edge — body's top
        //    sticks 4 px above column.bottom, but body.bottom is
        //    well below it. `body_is_side_contact` returns false
        //    (not fully nested), TOI=0 from the existing penetration,
        //    and the falling-branch snaps body.bottom to column.top
        //    (high above the player), again teleporting OOB.
        //
        // Generalized guard: if the body strictly intersects the
        // block at the start of the sweep, this is a pre-existing
        // penetration the y-sweep cannot resolve — the x-resolver
        // (or the next frame's normal y-sweep) owns it. Skipping
        // here matches the spirit of `resolve_axis(Axis::X)`'s
        // overlap-shape guard.
        if start_body.strict_intersects(block.aabb) {
            return false;
        }
        true
    }) {
        player.pos.y += delta.y * sweep_fraction(hit.time_of_impact);
        let body = player.aabb();
        // Landing-from-above branch: only snap body.bottom onto the
        // block's top edge when the body was actually approaching the
        // block from above this frame. The original `delta.y > 0.0`
        // alone fired the snap unconditionally on any downward motion,
        // which catapulted the player upward when delta.y was tiny
        // and the block sat far above the body (the symmetric repro
        // to bug #2 in the predicate above). Mirroring the OneWay
        // landing test (`prev_bottom <= block.top + tol`) keeps real
        // floor landings working and rejects the fake one.
        let approaching_from_above = delta.y > 0.0 && prev_bottom <= hit.block.aabb.top() + 4.0;
        if approaching_from_above || body.center().y < hit.block.aabb.center().y {
            player.pos.y += hit.block.aabb.top() - body.bottom();
            player.on_ground = true;
        } else {
            player.pos.y += hit.block.aabb.bottom() - body.top();
        }
        player.vel.y = 0.0;
    } else {
        player.pos.y += delta.y;
    }

    resolve_vertical(world, player, prev_bottom, drop_through);
}

pub(super) fn body_is_side_contact(body: Aabb, block: Aabb) -> bool {
    const Y_NESTED_EPS: f32 = 1.0e-4;
    body.top() >= block.top() - Y_NESTED_EPS && body.bottom() <= block.bottom() + Y_NESTED_EPS
}

pub(super) fn standing_on_one_way(world: &World, player: &Player) -> bool {
    standing_on_one_way_aabb(world, player.aabb())
}

/// AABB-only variant of [`standing_on_one_way`]. Cluster-aware
/// callers pass `PlayerKinematics::aabb()` directly.
pub fn standing_on_one_way_aabb(world: &World, body: Aabb) -> bool {
    for block in &world.blocks {
        if !matches!(block.kind, BlockKind::OneWay) {
            continue;
        }
        let horizontally_overlaps =
            body.right() > block.aabb.left() + 1.0 && body.left() < block.aabb.right() - 1.0;
        let near_top = (body.bottom() - block.aabb.top()).abs() <= 4.0;
        if horizontally_overlaps && near_top {
            return true;
        }
    }
    false
}

fn resolve_axis(world: &World, player: &mut Player, axis: Axis) {
    let mut aabb = player.aabb();
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, axis) || !aabb.strict_intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            continue;
        }
        match axis {
            Axis::X => {
                // Only resolve as a horizontal contact when the overlap is
                // shallower in x than in y. Otherwise this is a vertical
                // contact (player's head poking into a wide ceiling, or feet
                // poking into a wide floor) and the appropriate axis is the
                // perpendicular `resolve_vertical` pass — pushing
                // horizontally instead can catapult the player across the
                // entire room (the floor/ceiling block spans the whole
                // width, so its near edge is far away). Concrete repro: a
                // wall-jump off the left wall while feet barely overlap the
                // floor used to teleport the player tens of pixels left
                // through the wall.
                let overlap_x = (aabb.right().min(block.aabb.right())
                    - aabb.left().max(block.aabb.left()))
                .max(0.0);
                let overlap_y = (aabb.bottom().min(block.aabb.bottom())
                    - aabb.top().max(block.aabb.top()))
                .max(0.0);
                if overlap_x > overlap_y {
                    continue;
                }
                if aabb.center().x < block.aabb.center().x {
                    let push = block.aabb.left() - aabb.right();
                    player.pos.x += push;
                    player.wall_normal_x = -1.0;
                } else {
                    let push = block.aabb.right() - aabb.left();
                    player.pos.x += push;
                    player.wall_normal_x = 1.0;
                }
                player.vel.x = 0.0;
                player.on_wall = true;
            }
            Axis::Y => {}
        }
        aabb = player.aabb();
    }
}

fn resolve_vertical(world: &World, player: &mut Player, prev_bottom: f32, drop_through: bool) {
    let mut aabb = player.aabb();
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, Axis::Y) || !aabb.strict_intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            let landing_from_above = player.vel.y >= 0.0 && prev_bottom <= block.aabb.top() + 8.0;
            if !landing_from_above || drop_through {
                continue;
            }
        }
        // AMBITION_REVIEW(spatial): symmetric to `resolve_axis(Axis::X)`.
        // If the body's y-range is entirely nested inside the block's
        // y-range, this is a side-wall contact — the x-axis sweep /
        // resolve owns it. Pushing vertically here can catapult the
        // player to the wall block's top edge if the wall spans the
        // full room height (concrete repro: wall-clinging on a tall
        // left wall whose top is at world y=32 used to teleport the
        // player to y = top - half_height = 9). Skipping OneWay
        // because OneWay is by construction wider than tall.
        if !matches!(block.kind, BlockKind::OneWay) && body_is_side_contact(aabb, block.aabb) {
            continue;
        }
        if aabb.center().y < block.aabb.center().y {
            let push = block.aabb.top() - aabb.bottom();
            player.pos.y += push;
            player.on_ground = true;
        } else {
            let push = block.aabb.bottom() - aabb.top();
            player.pos.y += push;
        }
        player.vel.y = 0.0;
        aabb = player.aabb();
    }
}

pub(super) fn try_pogo(world: &World, player: &mut Player, tuning: MovementTuning) -> Option<Aabb> {
    let feet = player.aabb();
    let hitbox = Aabb::new(
        Vec2::new(feet.center().x, feet.bottom() + 18.0),
        Vec2::new(feet.half_size().x * 0.76, 22.0),
    );
    let hit = world.blocks.iter().find(|block| {
        let valid_target = matches!(
            block.kind,
            BlockKind::PogoOrb
                | BlockKind::Solid
                | BlockKind::BlinkWall { .. }
                | BlockKind::Rebound { .. }
        );
        valid_target && hitbox.strict_intersects(block.aabb)
    });
    if let Some(block) = hit {
        let aabb = block.aabb;
        player.vel.y = -tuning.pogo_speed;
        player.refresh_movement_resources(tuning);
        player.on_ground = false;
        Some(aabb)
    } else {
        None
    }
}

pub(super) fn touching_hazard(world: &World, player: &Player) -> bool {
    touching_hazard_aabb(world, player.aabb())
}

/// AABB-only variant of [`touching_hazard`]. Cluster-aware callers
/// pass `PlayerKinematics::aabb()` directly without building an
/// `ae::Player`.
pub fn touching_hazard_aabb(world: &World, aabb: crate::Aabb) -> bool {
    world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, BlockKind::Hazard) && aabb.strict_intersects(b.aabb))
}

pub(super) fn touching_rebound(world: &World, player: &Player) -> Option<Vec2> {
    touching_rebound_aabb(world, player.aabb())
}

/// AABB-only variant of [`touching_rebound`].
pub fn touching_rebound_aabb(world: &World, aabb: crate::Aabb) -> Option<Vec2> {
    world.blocks.iter().find_map(|b| match b.kind {
        BlockKind::Rebound { impulse } if aabb.strict_intersects(b.aabb) => Some(impulse),
        _ => None,
    })
}

/// Cluster-ref variant of [`try_pogo`]. Mutates kinematics velocity,
/// refreshes movement resources on the dash/jump clusters, and
/// clears the ground flag.
pub fn try_pogo_clusters(
    world: &World,
    kinematics: &mut crate::player_clusters::PlayerKinematics,
    abilities: &crate::player_clusters::PlayerAbilities,
    dash: &mut crate::player_clusters::PlayerDashState,
    jump_state: &mut crate::player_clusters::PlayerJumpState,
    ground: &mut crate::player_clusters::PlayerGroundState,
    tuning: MovementTuning,
) -> Option<Aabb> {
    let feet = kinematics.aabb();
    let hitbox = Aabb::new(
        Vec2::new(feet.center().x, feet.bottom() + 18.0),
        Vec2::new(feet.half_size().x * 0.76, 22.0),
    );
    let hit = world.blocks.iter().find(|block| {
        let valid_target = matches!(
            block.kind,
            BlockKind::PogoOrb
                | BlockKind::Solid
                | BlockKind::BlinkWall { .. }
                | BlockKind::Rebound { .. }
        );
        valid_target && hitbox.strict_intersects(block.aabb)
    });
    if let Some(block) = hit {
        let aabb = block.aabb;
        kinematics.vel.y = -tuning.pogo_speed;
        crate::player_clusters::refresh_movement_resources_clusters(
            abilities, dash, jump_state, tuning,
        );
        ground.on_ground = false;
        Some(aabb)
    } else {
        None
    }
}
