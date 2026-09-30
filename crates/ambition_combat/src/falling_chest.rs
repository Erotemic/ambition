//! Falling-chest physics for ECS reward chests.
//!
//! Reward chests spawned mid-air by `sync_boss_reward_chests_ecs` carry a [`FallingChest`] until
//! they land on a solid floor.

use super::*;
use super::{CHEST_FALL_GRAVITY, CHEST_FALL_MAX_SPEED};

/// Tick ECS reward chests that are still falling to the floor.
///
/// Each chest falls against the geometry of the live room it is in, so the
/// system runs while two rooms are live (OW1 cut 7a). A chest in no live
/// room does not move.
pub fn update_ecs_falling_chests(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    mut chests: Query<(Entity, &mut CenteredAabb, &mut FallingChest), With<ChestFeature>>,
) {
    // Sim clock: bullet-time / pause / hitstop must freeze a falling
    // chest mid-arc the same way they freeze the player. ADR 0010.
    let dt = world_time.sim_dt();
    for (entity, mut aabb, mut falling) in &mut chests {
        let Some(world) = rooms.of(entity) else {
            continue;
        };
        falling.vel_y = (falling.vel_y + CHEST_FALL_GRAVITY * dt).min(CHEST_FALL_MAX_SPEED);
        let step = falling.vel_y * dt;
        if step <= 0.0 {
            continue;
        }
        let max_substep = aabb.half_size.y.max(2.0);
        let mut remaining = step;
        while remaining > 0.0 {
            let advance = remaining.min(max_substep);
            let try_center = ae::Vec2::new(aabb.center.x, aabb.center.y + advance);
            let try_aabb = ae::Aabb::new(try_center, aabb.half_size);
            // ⭐ THE PUBLISHED PREDICATE, AND IT IS `is_support_surface` RATHER
            // THAN `is_full_collision_surface`: a chest asks what it can COME TO
            // REST ON, which includes a one-way, and the other predicate answers
            // a different question (what blocks both axes) with `OneWay` absent.
            // Two spellings of one question lived here and in `settled_chest_center`
            // below; collapsing them onto the WRONG one of the two published
            // predicates would have made falling chests pass through every
            // one-way platform in the game.
            let blocked = world.0.body_overlaps_any(try_aabb, |block| {
                ae::collision_semantics::is_support_surface(block.kind)
            });
            if blocked {
                commands.entity(entity).remove::<FallingChest>();
                break;
            }
            aabb.center = try_center;
            remaining -= advance;
        }
    }
}

/// Run the falling-chest tick virtually to find the chest's final
/// resting position. Used when a save says the boss reward is already
/// looted — the chest spawns pre-settled so the player doesn't see a
/// reward animation for an encounter they cleared in an earlier run.
pub fn settled_chest_center(world: &ae::World, start: ae::Vec2, size: ae::Vec2) -> ae::Vec2 {
    let mut center = start;
    let half_size = size * 0.5;
    let mut vel_y: f32 = 0.0;
    let virtual_dt = 1.0 / 60.0;
    for _ in 0..240 {
        vel_y = (vel_y + CHEST_FALL_GRAVITY * virtual_dt).min(CHEST_FALL_MAX_SPEED);
        let step = vel_y * virtual_dt;
        if step <= 0.0 {
            continue;
        }
        let max_substep = half_size.y.max(2.0);
        let mut remaining = step;
        while remaining > 0.0 {
            let advance = remaining.min(max_substep);
            let try_center = ae::Vec2::new(center.x, center.y + advance);
            let try_aabb = ae::Aabb::new(try_center, half_size);
            // The same question as the live tick above, and now the same call.
            let blocked = world.body_overlaps_any(try_aabb, |block| {
                ae::collision_semantics::is_support_surface(block.kind)
            });
            if blocked {
                return center;
            }
            center = try_center;
            remaining -= advance;
        }
    }
    center
}

#[cfg(test)]
mod falling_chest_tests {
    //! settled_chest_center drops a reward chest under gravity until its body would overlap a
    //! solid, then returns the last clear position.
    use super::*;

    fn world_with_floor() -> ae::World {
        ae::World::new(
            "t",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::new(50.0, 50.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 300.0),
                ae::Vec2::new(400.0, 100.0),
            )],
        )
    }

    /// OW1 cut 7a: a falling chest lands on the floor of the live room it is
    /// in. Live room #0's floor top is at y 300, and #1's at y 500. A chest
    /// falls in each from one place. The subject: each comes to rest on its
    /// own room's floor. The control: #0 alone, its chest rests at 300. When
    /// the system read the sole live room, it did not run while two rooms
    /// were live.
    #[test]
    fn each_chest_lands_on_the_floor_of_its_own_live_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
        let first = LiveRoomInstance::ACTIVATION;
        let mut lower = world_with_floor();
        lower.blocks[0] = ae::Block::solid("floor", ae::Vec2::new(0.0, 500.0), ae::Vec2::new(400.0, 100.0));
        let bottoms = |rooms: Vec<(LiveRoomInstance, ae::World)>| {
            let mut app = bevy::prelude::App::new();
            app.insert_resource(WorldTime {
                raw_dt: 1.0 / 60.0,
                scaled_dt: 1.0 / 60.0,
                ..Default::default()
            });
            app.add_systems(bevy::prelude::Update, update_ecs_falling_chests);
            let chests: Vec<_> = rooms
                .into_iter()
                .map(|(room, geometry)| {
                    app.world_mut()
                        .spawn((RoomInstanceRoot, room, ambition_platformer2d_core::RoomGeometry(geometry)));
                    app.world_mut()
                        .spawn((
                            ChestFeature::new(ambition_interaction::Chest::new("chest", None)),
                            CenteredAabb {
                                center: ae::Vec2::new(200.0, 50.0),
                                half_size: ae::Vec2::new(12.0, 12.0),
                            },
                            FallingChest::new(0.0),
                            InRoomInstance(room),
                        ))
                        .id()
                })
                .collect();
            for _ in 0..300 {
                app.update();
            }
            chests
                .into_iter()
                .map(|chest| {
                    let world = app.world();
                    let aabb = world.get::<CenteredAabb>(chest).expect("the chest stands");
                    (world.get::<FallingChest>(chest).is_none(), (aabb.center.y + aabb.half_size.y).ceil())
                })
                .collect::<Vec<_>>()
        };
        let landed_on = |floor: f32| move |(landed, bottom): &(bool, f32)| *landed && *bottom <= floor && *bottom > floor - 13.0;
        let control = bottoms(vec![(first, world_with_floor())]);
        assert!(control.iter().all(landed_on(300.0)), "control: the chest did not land on the floor: {control:?}");
        let both = bottoms(vec![(first, world_with_floor()), (first.next(), lower)]);
        assert!(
            landed_on(300.0)(&both[0]) && landed_on(500.0)(&both[1]),
            "the chests did not each land on the floor of their own live room: {both:?}"
        );
    }

    #[test]
    fn chest_settles_just_above_the_floor() {
        let world = world_with_floor();
        let half = ae::Vec2::new(12.0, 12.0);
        let settled = settled_chest_center(
            &world,
            ae::Vec2::new(200.0, 50.0),
            ae::Vec2::new(24.0, 24.0),
        );
        assert_eq!(settled.x, 200.0, "no horizontal drift");
        assert!(settled.y > 50.0, "the chest fell");
        let body = ae::Aabb::new(settled, half);
        assert!(
            !world.body_overlaps_any(body, |b| matches!(b.kind, ae::BlockKind::Solid)),
            "settled body must not overlap the floor (settled {settled:?})"
        );
        assert!(
            settled.y + half.y <= 300.0,
            "chest bottom stays above the floor top"
        );
        assert!(
            300.0 - (settled.y + half.y) <= 13.0,
            "chest comes to rest within a substep of the floor"
        );
    }

    /// ⛔⛤ A REWARD CHEST COMES TO REST ON A ONE-WAY PLATFORM, AND NOTHING SAID SO.
    ///
    /// The two sites above ask *"what can this chest come to rest on"*, and the
    /// answer is `is_support_surface` — the predicate with `OneWay` in it. Its
    /// sibling `is_full_collision_surface` answers a different question (what
    /// blocks both axes) and excludes one-ways.
    ///
    /// ⭐ MEASURED 2026-09-11 WHILE COLLAPSING TWO INLINE COPIES ONTO THE
    /// PUBLISHED PREDICATE: swapping in the WRONG one of the two left
    /// `ambition_combat` (633) and the monolith (1153) entirely green. **1,786
    /// tests and not one of them drops a chest onto a one-way.** A boss reward
    /// spawned over a one-way platform would have fallen straight through it and
    /// kept going — the floor below, or out of the room.
    ///
    /// ⇒ That is why the collapse ships with this test rather than on its own:
    /// two spellings of one question are safe to unify only when something says
    /// which of the two published answers is the right one.
    #[test]
    fn a_chest_settles_on_a_one_way_platform_not_through_it() {
        let world = ae::World::new(
            "t",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::new(50.0, 50.0),
            vec![ae::Block::one_way(
                "a one-way ledge",
                ae::Vec2::new(0.0, 300.0),
                ae::Vec2::new(400.0, 12.0),
            )],
        );
        let half = ae::Vec2::new(12.0, 12.0);
        let settled =
            settled_chest_center(&world, ae::Vec2::new(200.0, 50.0), ae::Vec2::new(24.0, 24.0));

        // ⛔ ANTI-VACUITY: a chest that never moved also "did not fall through".
        assert!(
            settled.y > 50.0,
            "the chest did not fall at all, so this says nothing about what \
             stopped it"
        );
        assert!(
            settled.y + half.y <= 300.0,
            "a reward chest fell THROUGH a one-way platform: it settled at \
             {settled:?}, past the ledge top at 300. A chest asks what it can \
             REST on, which includes a one-way — `is_support_surface`, not \
             `is_full_collision_surface`"
        );
    }

    #[test]
    fn chest_keeps_falling_without_a_floor() {
        let world = ae::World::new(
            "t",
            ae::Vec2::new(400.0, 9999.0),
            ae::Vec2::new(50.0, 50.0),
            Vec::new(),
        );
        let settled = settled_chest_center(
            &world,
            ae::Vec2::new(200.0, 50.0),
            ae::Vec2::new(24.0, 24.0),
        );
        assert!(
            settled.y > 100.0,
            "with no floor the chest keeps falling (settled {settled:?})"
        );
    }
}
