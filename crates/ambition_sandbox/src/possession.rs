//! Possession — Down + Interact takes over a nearby non-boss actor.
//!
//! The possessed actor's **own** `ActorControlFrame` is driven from the player's
//! `ControlFrame` (via [`crate::brain::player::tick_player_brain_from_control`]),
//! so it moves and attacks through its own update path in `update_ecs_actors` —
//! the honest "drive any actor's control path" unification the universal-brain
//! seam was built for (an `ActorControlFrame` field's doc literally calls out
//! "a possessed goblin"). The player's own control phase is suppressed while
//! possessing so input drives the possessed actor, not both bodies.
//!
//! Bounded to **non-boss** actors: bosses still `unreachable!()` on
//! `Brain::Player` (`content/features/ecs/bosses.rs`), so they're excluded from
//! the candidate set. The 2s hold, camera follow, and body-vacate visuals from
//! the TODO are a handoff; this is the control slice + its verification.

use bevy::prelude::*;

use crate::input::ControlFrame;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};

/// Marker on the actor the player is currently possessing. Carries the latest
/// player `ControlFrame`, synced each frame by [`sync_possession_input`] (which
/// holds `Res<ControlFrame>`), so the already-large `update_ecs_actors` reads it
/// as a query field instead of growing another top-level system param.
#[derive(Component, Clone, Copy, Default)]
pub struct Possessed {
    pub control: ControlFrame,
}

/// Who the player is possessing (`None` = controlling their own body).
#[derive(Resource, Default)]
pub struct PossessionState {
    pub possessed: Option<Entity>,
}

/// True while the player is possessing another actor.
pub fn possession_active(state: Res<PossessionState>) -> bool {
    state.possessed.is_some()
}

/// Inverse of [`possession_active`] — the player's own control phase runs only
/// while NOT possessing, so the same input doesn't drive both bodies.
pub fn not_possessing(state: Res<PossessionState>) -> bool {
    state.possessed.is_none()
}

/// Possession reach (px): Down+Interact possesses the nearest candidate within this.
const POSSESS_RADIUS: f32 = 150.0;

/// `Down + Interact` (rising edge) toggles possession: take over the nearest
/// non-boss actor in range, or release the current one. `Down` is `axis_y >
/// 0.35` (the same threshold drop-through uses).
pub fn possession_trigger_system(
    control: Res<ControlFrame>,
    mut prev_interact: Local<bool>,
    mut state: ResMut<PossessionState>,
    mut commands: Commands,
    players: Query<&PlayerKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    candidates: Query<
        (Entity, &crate::features::FeatureAabb),
        (
            With<crate::features::FeatureSimEntity>,
            With<crate::brain::ActorControl>,
            Without<crate::features::BossConfig>,
        ),
    >,
) {
    let down = control.axis_y > 0.35;
    let interact_edge = control.interact_pressed && !*prev_interact;
    *prev_interact = control.interact_pressed;
    if !(down && interact_edge) {
        return;
    }

    // Already possessing → release.
    if let Some(entity) = state.possessed.take() {
        if let Ok(mut ec) = commands.get_entity(entity) {
            ec.remove::<Possessed>();
        }
        return;
    }

    // Otherwise possess the nearest candidate in range.
    let Ok(kin) = players.single() else {
        return;
    };
    let nearest = candidates
        .iter()
        .map(|(e, aabb)| (e, (aabb.center - kin.pos).length()))
        .filter(|(_, d)| *d <= POSSESS_RADIUS)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((entity, _)) = nearest {
        commands.entity(entity).insert(Possessed::default());
        state.possessed = Some(entity);
    }
}

/// Mirror the player's input onto the possessed actor each frame, before
/// `update_ecs_actors` reads it to drive that actor.
pub fn sync_possession_input(control: Res<ControlFrame>, mut possessed: Query<&mut Possessed>) {
    for mut p in &mut possessed {
        p.control = *control;
    }
}

/// Clear the possession when the possessed actor is gone (despawned / died), so
/// the player isn't stranded controlling nothing. Runs each frame.
pub fn release_possession_if_target_lost(
    mut state: ResMut<PossessionState>,
    possessed: Query<(), With<Possessed>>,
) {
    if let Some(entity) = state.possessed {
        if possessed.get(entity).is_err() {
            state.possessed = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn down_interact_possesses_then_releases_the_nearest_candidate() {
        let mut app = App::new();
        app.insert_resource(ControlFrame::default());
        app.init_resource::<PossessionState>();
        app.add_systems(Update, possession_trigger_system);
        // A player at the origin and a candidate actor 80px away (in range).
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            PlayerKinematics {
                pos: crate::engine_core::Vec2::ZERO,
                vel: crate::engine_core::Vec2::ZERO,
                size: crate::engine_core::Vec2::new(24.0, 40.0),
                base_size: crate::engine_core::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
        ));
        let actor = app
            .world_mut()
            .spawn((
                crate::features::FeatureSimEntity,
                crate::features::FeatureAabb::new(
                    crate::engine_core::Vec2::new(80.0, 0.0),
                    crate::engine_core::Vec2::new(12.0, 16.0),
                ),
                crate::brain::ActorControl::default(),
            ))
            .id();

        // Down + Interact (rising edge) → possess the actor.
        {
            let mut c = app.world_mut().resource_mut::<ControlFrame>();
            c.axis_y = 1.0;
            c.interact_pressed = true;
        }
        app.update();
        assert_eq!(
            app.world().resource::<PossessionState>().possessed,
            Some(actor),
            "Down+Interact possesses the nearest candidate"
        );
        assert!(
            app.world().get::<Possessed>(actor).is_some(),
            "the actor gets the Possessed marker"
        );

        // Release the edge, then press again → un-possess.
        app.world_mut().resource_mut::<ControlFrame>().interact_pressed = false;
        app.update();
        app.world_mut().resource_mut::<ControlFrame>().interact_pressed = true;
        app.update();
        assert_eq!(
            app.world().resource::<PossessionState>().possessed,
            None,
            "a second Down+Interact releases possession"
        );
        assert!(
            app.world().get::<Possessed>(actor).is_none(),
            "the Possessed marker is removed on release"
        );
    }

    #[test]
    fn out_of_range_actors_are_not_possessed() {
        let mut app = App::new();
        app.insert_resource(ControlFrame::default());
        app.init_resource::<PossessionState>();
        app.add_systems(Update, possession_trigger_system);
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            PlayerKinematics {
                pos: crate::engine_core::Vec2::ZERO,
                vel: crate::engine_core::Vec2::ZERO,
                size: crate::engine_core::Vec2::new(24.0, 40.0),
                base_size: crate::engine_core::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
        ));
        // Far away (out of POSSESS_RADIUS).
        app.world_mut().spawn((
            crate::features::FeatureSimEntity,
            crate::features::FeatureAabb::new(
                crate::engine_core::Vec2::new(900.0, 0.0),
                crate::engine_core::Vec2::new(12.0, 16.0),
            ),
            crate::brain::ActorControl::default(),
        ));
        {
            let mut c = app.world_mut().resource_mut::<ControlFrame>();
            c.axis_y = 1.0;
            c.interact_pressed = true;
        }
        app.update();
        assert_eq!(
            app.world().resource::<PossessionState>().possessed,
            None,
            "no candidate in range → nothing possessed"
        );
    }
}
