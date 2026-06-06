//! Per-frame `ActorTarget` selection for non-player actors.
//!
//! Runs at the top of the actor simulation chain so each enemy /
//! boss / NPC's downstream tick reads "who am I looking at right
//! now" from its `ActorTarget` component rather than from the global
//! primary-player query. Today's policy is "nearest alive player-
//! faction entity"; co-op / split-screen builds can later swap a
//! sticky-target or role-based selector here without touching any
//! actor update signatures (OVERNIGHT-TODO #17.8).

use crate::engine_core as ae;
use bevy::prelude::*;

use super::super::components::{ActorAggression, ActorTarget, AggressionTarget, FeatureAabb};
use super::FeatureSimEntity;
use crate::player::{BodyKinematics, PlayerEntity};

/// Pick each non-player actor's `ActorTarget` for this frame.
///
/// Selection is driven by each actor's [`ActorAggression`], not by its
/// [`ActorFaction`]: `ActorAggression::target_policy` says whether the
/// actor wants a target and which one. A non-passive actor
/// (`HostileToPlayer` / `RetaliatesWhenHit`) tracks the nearest alive
/// player-faction entity by straight-line distance — the same set of
/// actors the old `faction.needs_target()` shortcut targeted. A passive
/// actor takes no combat target and is pointed at itself so its facing
/// math keeps the current facing instead of snapping toward the origin.
///
/// When no player entities exist (pre-spawn, post-death-of-all-players,
/// headless probe with no player) every actor's `ActorTarget` is left
/// untouched so downstream ticks see the previous frame's target rather
/// than zeroing out.
///
/// Today's production game has exactly one player so this loop is
/// O(n) over actors. A many-player build can swap in a spatial
/// index here without changing the consumer side.
pub fn select_actor_targets(
    players: Query<(Entity, &BodyKinematics), With<PlayerEntity>>,
    mut actors: Query<(&FeatureAabb, &mut ActorTarget, &ActorAggression), With<FeatureSimEntity>>,
) {
    let player_snapshots: Vec<(Entity, ae::Vec2)> =
        players.iter().map(|(e, kin)| (e, kin.pos)).collect();
    if player_snapshots.is_empty() {
        return;
    }
    for (aabb, mut target, aggression) in actors.iter_mut() {
        let actor_pos = aabb.center;
        match aggression.target_policy() {
            AggressionTarget::None => {
                // Passive: no combat target. Point at self so a
                // zero direction keeps the actor's current facing.
                target.pos = actor_pos;
                target.entity = None;
            }
            AggressionTarget::NearestPlayer => {
                let (best_entity, best_pos) = player_snapshots
                    .iter()
                    .copied()
                    .min_by(|(_, a), (_, b)| {
                        let da = distance_squared(*a, actor_pos);
                        let db = distance_squared(*b, actor_pos);
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .expect("player_snapshots non-empty above");
                target.pos = best_pos;
                target.entity = Some(best_entity);
            }
        }
    }
}

fn distance_squared(a: ae::Vec2, b: ae::Vec2) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::features::components::{ActorAggression, ActorTarget, FeatureAabb};
    use crate::player::{BodyKinematics, PlayerEntity, PlayerSlot, PrimaryPlayer};

    fn dummy_player_body(pos: ae::Vec2) -> BodyKinematics {
        BodyKinematics {
            pos,
            size: ae::Vec2::new(28.0, 46.0),
            facing: 1.0,
            ..Default::default()
        }
    }

    fn enemy_at(app: &mut App, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureAabb::from_center_size(pos, ae::Vec2::new(20.0, 20.0)),
                ActorTarget::default(),
                ActorAggression::hostile_to_player(),
            ))
            .id()
    }

    #[test]
    fn target_points_at_only_player_when_one_present() {
        let mut app = App::new();
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(0),
                PrimaryPlayer,
                dummy_player_body(ae::Vec2::new(300.0, 100.0)),
            ))
            .id();
        let enemy = enemy_at(&mut app, ae::Vec2::new(100.0, 100.0));
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(enemy).get::<ActorTarget>().unwrap();
        assert_eq!(target.pos, ae::Vec2::new(300.0, 100.0));
        assert_eq!(target.entity, Some(player));
    }

    #[test]
    fn target_picks_nearest_when_two_players_present() {
        let mut app = App::new();
        // p1 at (100, 100), p2 at (500, 100). Enemy at (450, 100)
        // → nearest is p2.
        app.world_mut().spawn((
            PlayerEntity,
            PlayerSlot(0),
            PrimaryPlayer,
            dummy_player_body(ae::Vec2::new(100.0, 100.0)),
        ));
        let p2 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(1),
                dummy_player_body(ae::Vec2::new(500.0, 100.0)),
            ))
            .id();
        let enemy = enemy_at(&mut app, ae::Vec2::new(450.0, 100.0));
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(enemy).get::<ActorTarget>().unwrap();
        assert_eq!(target.pos, ae::Vec2::new(500.0, 100.0));
        assert_eq!(target.entity, Some(p2));
    }

    #[test]
    fn passive_aggression_targets_self_not_player() {
        let mut app = App::new();
        app.world_mut().spawn((
            PlayerEntity,
            PlayerSlot(0),
            PrimaryPlayer,
            dummy_player_body(ae::Vec2::new(999.0, 999.0)),
        ));
        let actor_pos = ae::Vec2::new(40.0, 60.0);
        let passive = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureAabb::from_center_size(actor_pos, ae::Vec2::new(20.0, 20.0)),
                ActorTarget::default(),
                ActorAggression::passive(),
            ))
            .id();
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(passive).get::<ActorTarget>().unwrap();
        // Passive actors take no combat target: the selector points
        // them at themselves (zero facing direction) instead of the
        // far-away player at (999, 999).
        assert_eq!(target.pos, actor_pos);
        assert_eq!(target.entity, None);
    }

    #[test]
    fn retaliating_actor_tracks_nearest_player() {
        let mut app = App::new();
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(0),
                PrimaryPlayer,
                dummy_player_body(ae::Vec2::new(200.0, 100.0)),
            ))
            .id();
        // A RetaliatesWhenHit NPC still tracks the player (for facing /
        // approach) even before it has been provoked — this reproduces
        // the old `faction.needs_target()` behavior for peaceful NPCs.
        let npc = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureAabb::from_center_size(
                    ae::Vec2::new(100.0, 100.0),
                    ae::Vec2::new(20.0, 20.0),
                ),
                ActorTarget::default(),
                ActorAggression::retaliates_when_hit(3),
            ))
            .id();
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(npc).get::<ActorTarget>().unwrap();
        assert_eq!(target.pos, ae::Vec2::new(200.0, 100.0));
        assert_eq!(target.entity, Some(player));
    }

    #[test]
    fn no_players_leaves_target_unchanged() {
        let mut app = App::new();
        let enemy = enemy_at(&mut app, ae::Vec2::new(100.0, 100.0));
        // Prime the target to a known sentinel so we can prove the
        // selector didn't touch it.
        app.world_mut()
            .entity_mut(enemy)
            .get_mut::<ActorTarget>()
            .unwrap()
            .pos = ae::Vec2::new(42.0, 42.0);
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(enemy).get::<ActorTarget>().unwrap();
        assert_eq!(target.pos, ae::Vec2::new(42.0, 42.0));
    }
}
