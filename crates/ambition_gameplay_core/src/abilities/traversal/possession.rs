//! Possession — Down + Interact takes over a nearby non-boss actor.
//!
//! The possessed actor's **own** `ActorControlFrame` is driven from the player's
//! `ControlFrame` (via [`ambition_characters::brain::player::tick_player_brain_from_control`]),
//! so it moves and attacks through its own update path in `update_ecs_actors` —
//! the honest "drive any actor's control path" unification the universal-brain
//! seam was built for (an `ActorControlFrame` field's doc literally calls out
//! "a possessed goblin"). The player's own control phase is suppressed while
//! possessing so input drives the possessed actor, not both bodies.
//!
//! Bounded to **non-boss** actors: bosses still `unreachable!()` on
//! `Brain::Player` (`crate::features::ecs::bosses::tick`), so they're excluded
//! from the candidate set. The 2s hold, camera follow, and body-vacate visuals from
//! the TODO are a handoff; this is the control slice + its verification.

use bevy::prelude::*;

use ambition_input::ControlFrame;
use crate::player::{BodyKinematics, PlayerEntity, PrimaryPlayer};

/// Marker on the actor the player is currently possessing. Carries the latest
/// player `ControlFrame`, synced each frame by [`sync_possession_input`] (which
/// holds `Res<ControlFrame>`), so the already-large `update_ecs_actors` reads it
/// as a query field instead of growing another top-level system param.
///
/// `original_faction` is the actor's faction before possession; while possessed
/// the actor is flipped to [`ActorFaction::Player`] so it fights its former
/// allies (its attacks become player-faction through the shared
/// `apply_hitbox_damage` / faction-aware projectile paths), restored on release.
#[derive(Component, Clone, Copy)]
pub struct Possessed {
    pub control: ControlFrame,
    pub original_faction: crate::features::ActorFaction,
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

/// Seconds the player must **hold** Down+Interact (with a candidate in range) to
/// commit a possession. A deliberate gesture so you don't possess by brushing
/// the button mid-fight; releasing fully is instant (a single press).
const POSSESS_HOLD_S: f32 = 2.0;

/// Walk speed (px/s) a possessed actor moves at. `tick_player_brain_from_control`
/// emits `desired_vel` as a *direction* (the player's input axis, in `[-1, 1]`)
/// because the player's own integration scales it — but an enemy/NPC integration
/// approaches `desired_vel` directly, so the possession path must scale the axis
/// to a real speed or the body crawls. Shared by the enemy + NPC drive paths.
pub const POSSESSED_MOVE_SPEED: f32 = 180.0;

/// `Down + Interact` controls possession: **hold ~2s** (with a candidate in
/// range) to take over the nearest non-boss actor; press it again to release.
/// `Down` is `axis_y > 0.35` (the same threshold drop-through uses). The hold
/// runs on real time (`raw_dt`) so bullet-time doesn't change the feel.
pub fn possession_trigger_system(
    control: Res<ControlFrame>,
    gravity_field: Option<Res<crate::physics::GravityField>>,
    // Optional: headless / unit-test apps may omit the settings resource. Absent →
    // Hybrid (the historical behavior).
    user_settings: Option<Res<crate::persistence::settings::UserSettings>>,
    world_time: Res<crate::WorldTime>,
    mut hold_timer: Local<f32>,
    mut prev_down_interact: Local<bool>,
    mut state: ResMut<PossessionState>,
    mut commands: Commands,
    mut players: Query<&mut BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    candidates: Query<
        (Entity, &crate::features::CenteredAabb),
        (
            With<crate::features::FeatureSimEntity>,
            With<ambition_characters::brain::ActorControl>,
            Without<crate::features::BossConfig>,
        ),
    >,
    mut factions: Query<&mut crate::features::ActorFaction>,
    possessed_q: Query<&Possessed>,
    // Read-only AABB lookup for the *vacate exit*: on release the player steps
    // out where the possessed actor stands, so the camera (which follows the
    // actor while possessing) doesn't snap back to the abandoned body.
    actor_aabb: Query<&crate::features::CenteredAabb>,
) {
    let gravity_dir = gravity_field
        .as_deref()
        .map_or(ambition_engine_core::Vec2::new(0.0, 1.0), |g| g.dir);
    let movement_mode = user_settings
        .as_deref()
        .map_or(ambition_engine_core::InputFrameMode::Hybrid, |s| {
            s.gameplay.movement_frame_mode
        });
    let descend = ambition_engine_core::AccelerationFrame::new(gravity_dir)
        .resolve_input(movement_mode, control.axis_x, control.axis_y)
        .y;
    let down_interact = descend > 0.35 && control.interact_pressed;
    let release_edge = down_interact && !*prev_down_interact;
    *prev_down_interact = down_interact;

    // Already possessing → a fresh Down+Interact press releases (no hold).
    if state.possessed.is_some() {
        *hold_timer = 0.0;
        if release_edge {
            if let Some(entity) = state.possessed.take() {
                // Vacate exit: step the player's body out where the possessed
                // actor stands. The camera was following the actor, so without
                // this the view (and your body) would snap back to wherever you
                // first possessed from — jarring if the actor roamed. You leave
                // the actor where it is; it reverts to its own brain beside you.
                if let (Ok(aabb), Ok(mut pk)) = (actor_aabb.get(entity), players.single_mut()) {
                    pk.pos = aabb.center;
                    pk.vel = ambition_engine_core::Vec2::ZERO;
                }
                // Restore the actor's original faction, then drop the marker.
                let original = possessed_q.get(entity).ok().map(|p| p.original_faction);
                if let (Some(original), Ok(mut faction)) = (original, factions.get_mut(entity)) {
                    *faction = original;
                }
                if let Ok(mut ec) = commands.get_entity(entity) {
                    ec.remove::<Possessed>();
                }
            }
        }
        return;
    }

    // Not possessing → accumulate the hold; commit at the threshold.
    if !down_interact {
        *hold_timer = 0.0;
        return;
    }
    *hold_timer += world_time.raw_dt;
    if *hold_timer < POSSESS_HOLD_S {
        return;
    }
    *hold_timer = 0.0;

    let Ok(kin) = players.single() else {
        return;
    };
    let nearest = candidates
        .iter()
        .map(|(e, aabb)| (e, (aabb.center - kin.pos).length()))
        .filter(|(_, d)| *d <= POSSESS_RADIUS)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((entity, _)) = nearest {
        // Flip the actor to the player's side so it now fights its former
        // allies; remember the original faction to restore on release.
        if let Ok(mut faction) = factions.get_mut(entity) {
            let original = *faction;
            *faction = crate::features::ActorFaction::Player;
            commands.entity(entity).insert(Possessed {
                control: *control,
                original_faction: original,
            });
            state.possessed = Some(entity);
        }
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
    use crate::player::PlayerBaseSize;

    fn vec2(x: f32, y: f32) -> ambition_engine_core::Vec2 {
        ambition_engine_core::Vec2::new(x, y)
    }

    /// App with the trigger + 1s/frame real time, so 2 held frames clear the 2s hold.
    fn trigger_app() -> App {
        let mut app = App::new();
        app.insert_resource(ControlFrame::default());
        app.insert_resource(crate::WorldTime {
            raw_dt: 1.0,
            scaled_dt: 1.0,
        });
        app.init_resource::<PossessionState>();
        app.add_systems(Update, possession_trigger_system);
        app
    }

    fn spawn_player(app: &mut App) {
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos: vec2(0.0, 0.0),
                vel: vec2(0.0, 0.0),
                size: vec2(24.0, 40.0),
                facing: 1.0,
            },
            PlayerBaseSize {
                base_size: vec2(24.0, 40.0),
            },
        ));
    }

    fn spawn_candidate(app: &mut App, pos: ambition_engine_core::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                crate::features::FeatureSimEntity,
                crate::features::CenteredAabb::new(pos, vec2(12.0, 16.0)),
                ambition_characters::brain::ActorControl::default(),
                crate::features::ActorFaction::Enemy,
            ))
            .id()
    }

    fn faction_of(app: &App, e: Entity) -> crate::features::ActorFaction {
        *app.world().get::<crate::features::ActorFaction>(e).unwrap()
    }

    fn hold_down_interact(app: &mut App, held: bool) {
        let mut c = app.world_mut().resource_mut::<ControlFrame>();
        c.axis_y = if held { 1.0 } else { 0.0 };
        c.interact_pressed = held;
    }

    fn possessed(app: &App) -> Option<Entity> {
        app.world().resource::<PossessionState>().possessed
    }

    #[test]
    fn holding_down_interact_possesses_then_a_press_releases() {
        let mut app = trigger_app();
        spawn_player(&mut app);
        let actor = spawn_candidate(&mut app, vec2(80.0, 0.0)); // in range

        // Hold Down+Interact: 1s, then 2s → crosses the 2s threshold → possess.
        hold_down_interact(&mut app, true);
        app.update(); // hold_timer = 1.0
        assert_eq!(possessed(&app), None, "not possessed mid-hold");
        app.update(); // hold_timer = 2.0 ≥ threshold → possess
        assert_eq!(
            possessed(&app),
            Some(actor),
            "a full ~2s hold possesses the nearest candidate"
        );
        assert!(app.world().get::<Possessed>(actor).is_some());
        // The possessed enemy flips to the player's side so it fights its allies.
        assert_eq!(
            faction_of(&app, actor),
            crate::features::ActorFaction::Player,
            "possession flips the actor to the player's faction"
        );

        // Release the button, then a fresh press releases possession.
        hold_down_interact(&mut app, false);
        app.update();
        hold_down_interact(&mut app, true);
        app.update();
        assert_eq!(
            possessed(&app),
            None,
            "a fresh Down+Interact press releases"
        );
        assert!(app.world().get::<Possessed>(actor).is_none());
        assert_eq!(
            faction_of(&app, actor),
            crate::features::ActorFaction::Enemy,
            "release restores the actor's original faction"
        );
        // Vacate exit: the player's body stepped out where the actor stood
        // (candidate spawned at x=80), not back at its origin (x=0), so the
        // camera that was following the actor doesn't snap back to the old body.
        let player_pos = app
            .world_mut()
            .query_filtered::<&BodyKinematics, With<PlayerEntity>>()
            .single(app.world())
            .unwrap()
            .pos;
        assert_eq!(
            player_pos,
            vec2(80.0, 0.0),
            "player vacates to the possessed actor's position on release"
        );
    }

    #[test]
    fn a_brief_tap_does_not_possess() {
        let mut app = trigger_app();
        spawn_player(&mut app);
        spawn_candidate(&mut app, vec2(80.0, 0.0));
        // One frame held (1s < 2s), then released → no possession.
        hold_down_interact(&mut app, true);
        app.update();
        hold_down_interact(&mut app, false);
        app.update();
        assert_eq!(
            possessed(&app),
            None,
            "a brief tap doesn't commit a possession"
        );
    }

    #[test]
    fn out_of_range_actors_are_not_possessed() {
        let mut app = trigger_app();
        spawn_player(&mut app);
        spawn_candidate(&mut app, vec2(900.0, 0.0)); // far out of POSSESS_RADIUS
        hold_down_interact(&mut app, true);
        app.update();
        app.update();
        app.update();
        assert_eq!(
            possessed(&app),
            None,
            "no candidate in range → nothing possessed"
        );
    }
}
