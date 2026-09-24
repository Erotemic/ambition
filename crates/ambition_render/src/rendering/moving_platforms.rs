//! A moving platform's picture, drawn the way every other room feature's is.
//!
//! This family derives the picture from the authoritative
//! `MovingPlatformSet`. The visual is not spawned inside the
//! room-construction transaction (`transaction::open` to
//! `transaction::close`).
//!
//! Every room feature is drawn reactively: each render family discovers its
//! own population, and [`super::features`] draws a marked rectangle for any
//! published id no family claims. Moving platforms follow the same model.
//!
//! Nothing here writes platform state. The set is only read, and the visuals
//! are reconciled to it. A restore that rewinds `MovingPlatformSet` is
//! followed on the next frame by matching visuals.

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
/// The index is the identity: the set is a positional roster rebuilt by room
/// construction, so a platform has no id of its own. A room change replaces
/// the whole roster and all its visuals.
#[derive(Component)]
pub struct MovingPlatformVisual {
    pub index: usize,
}

/// Reconcile the moving-platform visuals against the authoritative set.
///
/// Spawns what is missing, retires what the set no longer has, and moves and
/// resizes the rest. Idempotent: it compares populations instead of reacting
/// to events, so it needs no change detection and cannot double-spawn during
/// a rollback resimulation.
pub fn sync_moving_platform_visuals(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ae::RoomGeometry>,
    platform_set: Res<MovingPlatformSet>,
    mut existing: Query<(Entity, &MovingPlatformVisual, &mut Transform, &mut Sprite)>,
) {
    // Retire first, so a vanished index is not mistaken for a survivor when a
    // shorter roster reuses its slot.
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

    // Spawning needs a session scope; retiring does not, so a mid-frame
    // teardown still clears the population.
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

    /// A platform gets its visual without the room construction transaction
    /// spawning one: the set exists, and the family draws it.
    #[test]
    fn a_platform_in_the_set_gets_a_visual_without_any_construction_commit() {
        let mut app = app_with_platforms(vec![platform("a", 100.0), platform("b", 400.0)]);
        app.update();
        let drawn = visuals(&mut app);
        assert_eq!(drawn.len(), 2, "one visual per platform in the set");
        assert_eq!(drawn[0].0, 0);
        assert_eq!(drawn[1].0, 1);
    }

    /// It follows the authoritative set instead of remembering.
    ///
    /// A reconcile keeps no state, so it cannot overwrite restored platform
    /// state after a cross-room restore. A platform moved by any means (a tick,
    /// a room change, a rollback restore) is followed.
    #[test]
    fn the_visual_follows_a_restored_set_instead_of_remembering_a_start() {
        let mut app = app_with_platforms(vec![platform("a", 100.0)]);
        app.update();
        let before = visuals(&mut app)[0].1;

        // A jump like a rollback restore or room change: the set says somewhere
        // else, with no event.
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
