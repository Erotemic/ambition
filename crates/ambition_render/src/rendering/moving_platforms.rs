//! A moving platform's picture, drawn the way every other room feature's is.
//!
//! this replaces a spawn inside the room-construction transaction, and that is the whole
//! point of the slice. `spawn_moving_platforms` ran between `transaction::open` and
//! `transaction::close`, so the platform's VISUAL was installed by the commit that produced its
//! STATE.
//!
//! the seam already existed. Every other room feature is drawn reactively:
//! *"every render family discovers its own population"*, and
//! [`super::features`] draws a marked rectangle for any published id no family
//! claims, so an unclaimed feature is LOUD. The moving platform was simply the
//! one piece of authored room geometry that never joined that model. Joining it
//! deletes the problem rather than designing around it — a family that only
//! derives pictures cannot split a transaction it does not participate in.
//!
//! Nothing here writes platform state: the set is read, never touched, and the visuals are
//! reconciled to whatever it says. A restore that rewinds `MovingPlatformSet` is followed on the
//! next frame by visuals that agree with it, which is the property the old reset broke.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_BLOCK};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, RoomVisual, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::collision::MovingPlatformSet;
use bevy::prelude::*;

/// The picture of one moving platform, tied to its index in the authoritative
/// [`MovingPlatformSet`].
///
/// the index IS the identity, exactly as the deleted monolith component's
/// was: the set is a positional roster rebuilt by room construction, so a
/// platform has no id of its own to key on. A room change replaces the whole
/// roster and retires the whole visual population with it.
#[derive(Component)]
pub struct MovingPlatformVisual {
    pub index: usize,
}

/// Reconcile the moving-platform visuals against the authoritative set.
///
/// Spawns what is missing, retires what the set no longer has, and moves and
/// resizes the rest. Idempotent by construction — it compares populations
/// rather than reacting to an event — which is why it needs no change detection
/// and cannot double-spawn across a rollback resimulation.
pub fn sync_moving_platform_visuals(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ae::RoomGeometry>,
    platform_set: Res<MovingPlatformSet>,
    mut existing: Query<(Entity, &MovingPlatformVisual, &mut Transform, &mut Sprite)>,
) {
    // Retire first, so an index that vanished cannot be mistaken for one of the
    // survivors when a shorter roster reuses its slot.
    let mut drawn = vec![false; platform_set.0.len()];
    for (entity, visual, mut transform, mut sprite) in &mut existing {
        let Some(platform) = platform_set.0.get(visual.index) else {
            commands.entity(entity).despawn();
            continue;
        };
        drawn[visual.index] = true;
        transform.translation = world_to_bevy(&world.0, platform.pos, WORLD_Z_BLOCK + 4.0);
        sprite.custom_size = Some(Vec2::new(platform.size.x, platform.size.y));
    }

    // Spawning needs a session to be scoped to; retiring above does not, so a
    // teardown mid-frame still clears the population.
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    for (index, platform) in platform_set.0.iter().enumerate() {
        if drawn[index] {
            continue;
        }
        commands.spawn_session_scoped(
            session_scope,
            (
                Sprite::from_color(
                    Color::srgba(0.35, 0.74, 1.0, 0.92),
                    Vec2::new(platform.size.x, platform.size.y),
                ),
                Transform::from_translation(world_to_bevy(
                    &world.0,
                    platform.pos,
                    WORLD_Z_BLOCK + 4.0,
                )),
                Name::new(format!("Moving platform {index}: {}", platform.name)),
                MovingPlatformVisual { index },
                RoomVisual,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_world::platforms::MovingPlatformState;

    fn platform(name: &str, x: f32) -> MovingPlatformState {
        let mut state = MovingPlatformState::from_authored(
            ae::Vec2::new(x, 200.0),
            ae::Vec2::new(96.0, 16.0),
            240.0,
            130.0,
        );
        state.name = name.to_string();
        state
    }

    fn app_with_platforms(states: Vec<MovingPlatformState>) -> App {
        let mut app = App::new();
        app.init_resource::<ActiveSessionScope>();
        app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            ae::RoomGeometry(ae::World::new(
                "moving platform fixture",
                ae::Vec2::new(1280.0, 720.0),
                ae::Vec2::ZERO,
                Vec::new(),
            )),
        );
        app.insert_resource(MovingPlatformSet(states));
        app.add_systems(Update, sync_moving_platform_visuals);
        app
    }

    fn visuals(app: &mut App) -> Vec<(usize, ae::Vec2)> {
        let mut q = app
            .world_mut()
            .query::<(&MovingPlatformVisual, &Transform)>();
        let world = app.world();
        let mut rows: Vec<(usize, ae::Vec2)> = q
            .iter(world)
            .map(|(visual, transform)| {
                (
                    visual.index,
                    ae::Vec2::new(transform.translation.x, transform.translation.y),
                )
            })
            .collect();
        rows.sort_by_key(|(index, _)| *index);
        rows
    }

    /// THE CARVE'S CLAIM: a platform gets its picture without the room
    /// construction transaction spawning one.
    ///
    /// Here nothing constructs anything — the authoritative set simply exists, and the family draws
    /// it.
    #[test]
    fn a_platform_in_the_set_gets_a_visual_without_any_construction_commit() {
        let mut app = app_with_platforms(vec![platform("a", 100.0), platform("b", 400.0)]);
        app.update();
        let drawn = visuals(&mut app);
        assert_eq!(drawn.len(), 2, "one visual per platform in the set");
        assert_eq!(drawn[0].0, 0);
        assert_eq!(drawn[1].0, 1);
    }

    /// It follows the authoritative set rather than remembering.
    ///
    /// this is the property the deleted `sync_moving_platform` LOST once:
    /// it carried a room-change reset of its own, and that hidden second
    /// authority clobbered freshly RESTORED platform state after a staged
    /// cross-room restore. A pure reconcile cannot — it has nothing to
    /// remember. Moving a platform by any means (a tick, a room change, a
    /// rollback restore) is followed, not overwritten.
    #[test]
    fn the_visual_follows_a_restored_set_instead_of_remembering_a_start() {
        let mut app = app_with_platforms(vec![platform("a", 100.0)]);
        app.update();
        let before = visuals(&mut app)[0].1;

        // The kind of jump a rollback restore or a room change produces: the
        // authoritative set says somewhere else, with no event to react to.
        app.world_mut().resource_mut::<MovingPlatformSet>().0[0].pos = ae::Vec2::new(900.0, 200.0);
        app.update();
        let after = visuals(&mut app)[0].1;

        assert!(
            (after.x - before.x).abs() > 100.0,
            "the visual must follow the authoritative set ({before:?} -> {after:?}); \
             a family that remembered its own start would still be at the old place"
        );
        assert_eq!(visuals(&mut app).len(), 1, "and it must not double-spawn");
    }

    /// A shorter roster retires the visuals it no longer has. A room change
    /// replaces the whole set; nothing may be left drawing the old room's
    /// platforms.
    #[test]
    fn a_platform_that_leaves_the_set_stops_being_drawn() {
        let mut app = app_with_platforms(vec![platform("a", 100.0), platform("b", 400.0)]);
        app.update();
        assert_eq!(visuals(&mut app).len(), 2);

        app.world_mut().resource_mut::<MovingPlatformSet>().0.pop();
        app.update();
        let drawn = visuals(&mut app);
        assert_eq!(drawn.len(), 1, "the departed platform's visual is retired");
        assert_eq!(drawn[0].0, 0);
    }
}
