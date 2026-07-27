//! **Does this pass survive its target being torn down?** (queue L24)
//!
//! A presentation pass queues `commands.entity(body).insert(..)`. Between the
//! query that produced `body` and the frame's command flush, another system can
//! despawn it — session teardown on a provider switch, room teardown on a
//! transition, an actor cleanup on death. When that happens Bevy's default error
//! handler PANICS, and the crash names a bundle rather than a lifecycle.
//!
//! This is not hypothetical and it is not rare-but-survivable: it took down the
//! multi-provider acceptance cycle (L23), and it was surfaced by adding a system
//! that spawns nothing to the render chain — because that moved a flush
//! boundary. A hazard a no-op can trip is a hazard.
//!
//! ## Why a harness instead of a rule
//!
//! The obvious response is "use `try_insert` everywhere", and it is wrong.
//! `try_insert` on a target that should ALWAYS exist converts a real bug into
//! silence — the write simply stops happening and nothing says so. The decision
//! is per-site and it needs evidence, so this provides the evidence:
//! [`run_frame_despawning_targets`] runs a pass, despawns everything it could
//! have targeted before the flush, and reports whether the frame survives.
//!
//! Turning "I reasoned that this is safe" into "I ran it" is the whole point.

use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

/// Run `app` for one frame with every entity matching `Doomed` despawned
/// AFTER `pass` has run but BEFORE the frame's commands flush.
///
/// A pass whose deferred writes do not tolerate a vanished target fails inside
/// Bevy's command error handler, which panics — so callers assert by *not*
/// panicking, and a caller that wants the negative result wraps this in
/// [`std::panic::catch_unwind`].
///
/// `Doomed` is a marker the caller puts on the entities it wants torn down, so
/// a test can aim this at exactly the population a real teardown would take
/// (room-scoped, session-scoped) rather than at everything.
pub fn run_frame_despawning_targets<Doomed: Component, M, P>(
    app: &mut App,
    schedule: impl ScheduleLabel + Clone,
    pass: P,
) where
    P: bevy::ecs::schedule::IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
{
    app.add_systems(schedule.clone(), pass);
    // Chained AFTER the pass and still inside the same schedule, so both sets of
    // commands land in the same flush — which is precisely the frame shape that
    // produced the crash.
    app.add_systems(schedule, despawn_doomed::<Doomed>);
    app.update();
}

fn despawn_doomed<Doomed: Component>(mut commands: Commands, doomed: Query<Entity, With<Doomed>>) {
    for entity in &doomed {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct Doomed;

    #[derive(Component)]
    struct Decoration;

    /// The harness catches an intolerant deferred write.
    ///
    /// A meta-test, and worth the exception: this harness exists to produce
    /// evidence, so a harness that cannot fail would launder guesses into
    /// results. It asserts the shape it is built to detect, using a pass written
    /// to be wrong.
    #[test]
    fn an_intolerant_insert_on_a_torn_down_target_is_caught() {
        fn intolerant(mut commands: Commands, targets: Query<Entity, With<Doomed>>) {
            for entity in &targets {
                commands.entity(entity).insert(Decoration);
            }
        }

        let caught = std::panic::catch_unwind(|| {
            let mut app = App::new();
            app.world_mut().spawn(Doomed);
            run_frame_despawning_targets::<Doomed, _, _>(&mut app, Update, intolerant);
        });
        assert!(
            caught.is_err(),
            "the harness did not notice an `insert` landing on a despawned \
             target, so every result it produces is worthless"
        );
    }

    /// And passes a tolerant one, so the harness is not simply failing at
    /// everything.
    #[test]
    fn a_tolerant_insert_survives_the_same_teardown() {
        fn tolerant(mut commands: Commands, targets: Query<Entity, With<Doomed>>) {
            for entity in &targets {
                commands.entity(entity).try_insert(Decoration);
            }
        }

        let mut app = App::new();
        app.world_mut().spawn(Doomed);
        run_frame_despawning_targets::<Doomed, _, _>(&mut app, Update, tolerant);
    }
}

/// The harness pointed at a REAL pass. (queue L24)
///
/// `apply_placeholder_sprites_override` targets sprite entities, and sprite
/// entities are exactly what `despawn_dead_dynamic_feature_visuals` retires when
/// a feature's view disappears — so its deferred `SpriteOriginalState` write can
/// land on an entity that no longer exists.
///
/// This is the difference between the queue row's "reasoned" and "reproduced":
/// it runs the shipped system against a real teardown rather than arguing about
/// whether one is possible.
#[cfg(test)]
mod production_passes {
    use super::*;
    use crate::rendering::actors::apply_placeholder_sprites_override;

    #[derive(Component)]
    struct Doomed;

    /// Portal sprite marking targets `PropVisual` entities, which room teardown
    /// despawns with the room.
    #[test]
    fn portal_sprite_marking_survives_its_targets_being_retired() {
        use crate::rendering::gate_portal_visuals::sync_portal_sprite_visibility;
        use crate::rendering::primitives::PropVisual;

        let mut app = App::new();
        // A REGISTERED portal whose sprite name matches the prop below. Without
        // this the pass's outer loop is over an empty map, it never reaches the
        // insert, and the probe passes while proving nothing — which is exactly
        // what it did on the first run.
        let mut registry = ambition_world::rooms::GatePortalRegistry::default();
        registry.register("zone", "switch", "portal", "ring");
        app.insert_resource(registry);
        app.world_mut().spawn((
            PropVisual {
                id: "p".into(),
                kind: "portal".into(),
                // The pass matches on NAME, so this has to be a name it acts on
                // — otherwise the loop skips and the test proves nothing.
                name: "portal".into(),
                size: Vec2::splat(16.0),
                draw: Default::default(),
                flip_y: false,
            },
            Visibility::default(),
            Doomed,
        ));
        run_frame_despawning_targets::<Doomed, _, _>(
            &mut app,
            Update,
            sync_portal_sprite_visibility,
        );
    }

    #[test]
    fn the_placeholder_sprite_override_survives_its_targets_being_retired() {
        let mut app = App::new();
        app.insert_resource(ambition_dev_tools::dev_tools::DeveloperTools {
            // The branch that writes: without this the pass takes its early-out
            // and the test would pass without exercising anything.
            placeholder_sprites: true,
            ..Default::default()
        });
        app.init_resource::<ambition_sim_view::FeatureViewIndex>();
        app.init_resource::<ambition_projectiles::ProjectileVisualCatalog>();
        // A sprite entity that a teardown is about to take.
        app.world_mut().spawn((Sprite::default(), Doomed));

        run_frame_despawning_targets::<Doomed, _, _>(
            &mut app,
            Update,
            apply_placeholder_sprites_override,
        );
    }
}
