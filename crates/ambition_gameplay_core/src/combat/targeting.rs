//! Per-frame `ActorTarget` selection for non-player actors.
//!
//! Runs at the top of the actor simulation chain so each enemy /
//! boss / NPC's downstream tick reads "who am I looking at right
//! now" from its `ActorTarget` component rather than from the global
//! primary-player query. Today's policy is "nearest alive player-
//! faction entity"; co-op / split-screen builds can later swap a
//! sticky-target or role-based selector here without touching any
//! actor update signatures (OVERNIGHT-TODO #17.8).

use ambition_engine_core as ae;
use bevy::prelude::*;

use super::components::{
    ActorAggression, ActorFaction, ActorTarget, AggressionTarget, CenteredAabb,
};
use super::FeatureSimEntity;
use crate::actor::BodyHealth;
use crate::actor::BodyKinematics;
use crate::actor::PlayerEntity;

/// Number of [`ActorFaction`] variants (Player / Enemy / Npc / Boss / Neutral).
/// The relations matrix is indexed by `faction as usize`.
const FACTION_COUNT: usize = 5;

/// Who-fights-whom, as DATA rather than hard-coded actor types — the relational
/// targeting seam. `hostile[from][to] == true` means a `from`-faction actor
/// treats `to`-faction actors as a combat target this frame.
///
/// This is the seam future stealth / bounty / grudge / alliance systems write
/// to: revealing yourself flips the player's row, a bounty makes a faction
/// hostile to the player, an alliance clears two factions' mutual hostility — all
/// without touching the brains or the actor spawn path.
///
/// The default encodes the **combat baseline**: Player ↔ Enemy and Player ↔ Boss
/// are mutually hostile (the player and the hostile world fight), and nothing else
/// is — Npc / Neutral are peaceful, and same-faction actors don't fight. This is
/// the single source of truth the damage paths consult (melee + projectile),
/// so it reproduces today's player-vs-enemy combat with no behavior change while
/// making actor-vs-actor hostility expressible (a room sets, e.g.,
/// `set_mutual_hostile(Enemy, Boss, true)` for a spectator arena, and may *clear*
/// `Enemy → Player` so the combatants ignore the observing player).
#[derive(Resource, Clone, Debug)]
pub struct FactionRelations {
    hostile: [[bool; FACTION_COUNT]; FACTION_COUNT],
}

impl Default for FactionRelations {
    fn default() -> Self {
        let mut relations = Self {
            hostile: [[false; FACTION_COUNT]; FACTION_COUNT],
        };
        // The combat baseline: the player and the hostile world are at war.
        relations.set_mutual_hostile(ActorFaction::Player, ActorFaction::Enemy, true);
        relations.set_mutual_hostile(ActorFaction::Player, ActorFaction::Boss, true);
        relations
    }
}

impl FactionRelations {
    /// True iff `from`-faction actors currently treat `to`-faction actors as
    /// combat targets.
    pub fn is_hostile(&self, from: ActorFaction, to: ActorFaction) -> bool {
        self.hostile[from as usize][to as usize]
    }

    /// Set the one-directional stance `from → to`. Stealth/bounty/alliance
    /// systems call this; for mutual hostility call it both ways.
    pub fn set_hostile(&mut self, from: ActorFaction, to: ActorFaction, hostile: bool) {
        self.hostile[from as usize][to as usize] = hostile;
    }

    /// Set mutual hostility between two factions (both directions).
    pub fn set_mutual_hostile(&mut self, a: ActorFaction, b: ActorFaction, hostile: bool) {
        self.set_hostile(a, b, hostile);
        self.set_hostile(b, a, hostile);
    }
}

/// Friendly-fire policy — the DAMAGE-side counterpart to [`FactionRelations`]
/// (which is the TARGETING side). Targeting decides whom a brain *aims at*;
/// this decides whether a hit that *lands* deals damage.
///
/// Damage is physical: a hit damages any body it overlaps that is NOT the
/// attacker (self is excluded at every call site by entity). The one default
/// exclusion is **same-faction allies** — friendly fire is OFF by default, so a
/// pirate's stray shot can't hurt another pirate. A different-faction bystander
/// (e.g. the player observing a duel) IS hit by strays; that's deliberate.
/// Set `enabled = true` to opt INTO friendly fire (free-for-all): same-faction
/// bodies then damage each other too. Per-entity grudges/charm overrides would
/// layer on top of this faction baseline later.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default)]
pub struct FriendlyFire {
    pub enabled: bool,
}

/// Whether an `attacker`-faction hit may damage a `victim`-faction body. The
/// engine rule (see [`FriendlyFire`]): damage lands on any DIFFERENT faction;
/// same-faction is blocked unless friendly fire is enabled. Self-exclusion
/// (attacker entity == victim entity) is handled by the caller.
pub fn can_damage(
    attacker: ActorFaction,
    victim: ActorFaction,
    friendly_fire: FriendlyFire,
) -> bool {
    friendly_fire.enabled || attacker != victim
}

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
    relations: Option<Res<FactionRelations>>,
    players: Query<(Entity, &BodyKinematics, &BodyHealth), With<PlayerEntity>>,
    // Non-player actors are candidate targets too (the relational, non-player-
    // centric part): an actor can target another actor whose faction it's hostile
    // to. Snapshotted, so this read-only borrow ends before the mutable pass.
    others: Query<(Entity, &CenteredAabb, &ActorFaction, &BodyHealth), With<FeatureSimEntity>>,
    mut actors: Query<
        (
            Entity,
            &CenteredAabb,
            &mut ActorTarget,
            &ActorAggression,
            Option<&ActorFaction>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let relations = relations.map(|r| r.clone()).unwrap_or_default();
    // ALIVE candidates only: a dead body (health drained to 0) is never a valid
    // target. So the instant a foe dies the actor goes target-less — it stops
    // swinging at the corpse and (downstream) stands down — instead of chasing a
    // dead entity until it despawns. Death zeroes `BodyHealth` on every body
    // (player + actor), so this is the one uniform liveness gate.
    let player_snapshots: Vec<(Entity, ae::Vec2)> = players
        .iter()
        .filter(|(_, _, hp)| hp.current() > 0)
        .map(|(e, kin, _)| (e, kin.pos))
        .collect();
    let candidates: Vec<(Entity, ae::Vec2, ActorFaction)> = others
        .iter()
        .filter(|(_, _, _, hp)| hp.current() > 0)
        .map(|(e, aabb, faction, _)| (e, aabb.center, *faction))
        .collect();
    // Nothing to point at: leave every actor's target untouched so downstream
    // ticks keep last frame's value instead of zeroing (matches old behavior
    // when no players existed).
    if player_snapshots.is_empty() && candidates.is_empty() {
        return;
    }
    for (self_entity, aabb, mut target, aggression, faction) in actors.iter_mut() {
        let actor_pos = aabb.center;
        let policy = aggression.target_policy();
        if policy == AggressionTarget::None {
            // Passive: no combat target. Point at self so a zero direction keeps
            // the actor's current facing.
            target.pos = actor_pos;
            target.entity = None;
            continue;
        }
        // The player baseline is a candidate ONLY for `NearestPlayer` (hostile
        // enemies + retaliating NPCs that chase/face the player). `NearestFoe`
        // (a faction-feud fighter, e.g. the duel) tracks relational foes ONLY —
        // it never falls back to the observing player, so it goes target-less the
        // moment its foe is gone. Either way, relational faction-foes are always
        // candidates; nearest wins.
        let include_players = matches!(policy, AggressionTarget::NearestPlayer);
        let mut best: Option<(Entity, ae::Vec2, f32)> = None;
        let mut consider = |entity: Entity, pos: ae::Vec2| {
            if entity == self_entity {
                return;
            }
            let d = distance_squared(pos, actor_pos);
            if best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                best = Some((entity, pos, d));
            }
        };
        if include_players {
            for (entity, pos) in &player_snapshots {
                consider(*entity, *pos);
            }
        }
        if let Some(faction) = faction {
            for (entity, pos, other_faction) in &candidates {
                if relations.is_hostile(*faction, *other_faction) {
                    consider(*entity, *pos);
                }
            }
        }
        if let Some((entity, pos, _)) = best {
            target.pos = pos;
            target.entity = Some(entity);
        } else {
            // No valid target (foe dead/gone, or no players for a player-hunter):
            // point at self so facing math reads a zero direction (hold facing).
            target.pos = actor_pos;
            target.entity = None;
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
    use crate::actor::BodyKinematics;
    use crate::actor::{PlayerEntity, PrimaryPlayer};
    use crate::combat::components::{ActorAggression, ActorTarget, CenteredAabb};
    use crate::player::PlayerSlot;

    fn dummy_player_body(pos: ae::Vec2) -> BodyKinematics {
        BodyKinematics {
            pos,
            size: ae::Vec2::new(28.0, 46.0),
            facing: 1.0,
            ..Default::default()
        }
    }

    /// Live `BodyHealth` — every candidate body needs it now that targeting filters
    /// out the dead (a drained body is never a target).
    fn alive() -> BodyHealth {
        BodyHealth::new(ambition_characters::actor::Health::new(10))
    }

    fn enemy_at(app: &mut App, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(pos, ae::Vec2::new(20.0, 20.0)),
                ActorTarget::default(),
                ActorAggression::hostile_to_player(),
                alive(),
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
                alive(),
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
            alive(),
        ));
        let p2 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(1),
                dummy_player_body(ae::Vec2::new(500.0, 100.0)),
                alive(),
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
            alive(),
        ));
        let actor_pos = ae::Vec2::new(40.0, 60.0);
        let passive = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(actor_pos, ae::Vec2::new(20.0, 20.0)),
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
                alive(),
            ))
            .id();
        // A RetaliatesWhenHit NPC still tracks the player (for facing /
        // approach) even before it has been provoked — this reproduces
        // the old `faction.needs_target()` behavior for peaceful NPCs.
        let npc = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(
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

    /// The relational seam: with no player present, an actor targets the nearest
    /// NON-PLAYER actor of a faction `FactionRelations` marks it hostile to. This
    /// is the non-player-centric capability — "aggressive to whoever they're
    /// normally aggressive toward," driven by data, not a player hard-code.
    #[test]
    fn actor_targets_relationally_hostile_faction_when_no_player() {
        use crate::combat::components::ActorFaction;
        let mut app = App::new();
        let mut relations = FactionRelations::default();
        relations.set_hostile(ActorFaction::Enemy, ActorFaction::Npc, true);
        app.insert_resource(relations);

        // An Enemy-faction actor — no players anywhere ...
        let enemy = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(
                    ae::Vec2::new(100.0, 100.0),
                    ae::Vec2::new(20.0, 20.0),
                ),
                ActorTarget::default(),
                ActorAggression::hostile_to_player(),
                ActorFaction::Enemy,
                alive(),
            ))
            .id();
        // ... and an Npc-faction actor it's now relationally hostile to.
        let npc = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(
                    ae::Vec2::new(160.0, 100.0),
                    ae::Vec2::new(20.0, 20.0),
                ),
                ActorFaction::Npc,
                alive(),
            ))
            .id();

        app.add_systems(Update, select_actor_targets);
        app.update();

        let target = app.world().entity(enemy).get::<ActorTarget>().unwrap();
        assert_eq!(
            target.entity,
            Some(npc),
            "an Enemy hostile-to-Npc should target the Npc actor with no player present"
        );
        assert_eq!(target.pos, ae::Vec2::new(160.0, 100.0));
    }

    /// Default relations add NO actor-vs-actor hostility, so the same pair with
    /// no player + no relation produces no target (the actor faces itself) —
    /// proving the relational pool is opt-in and nothing regresses by default.
    #[test]
    fn no_relation_no_player_yields_no_target() {
        use crate::combat::components::ActorFaction;
        let mut app = App::new();
        app.insert_resource(FactionRelations::default());
        let enemy = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(
                    ae::Vec2::new(100.0, 100.0),
                    ae::Vec2::new(20.0, 20.0),
                ),
                ActorTarget::default(),
                ActorAggression::hostile_to_player(),
                ActorFaction::Enemy,
                alive(),
            ))
            .id();
        app.world_mut().spawn((
            FeatureSimEntity,
            CenteredAabb::from_center_size(ae::Vec2::new(160.0, 100.0), ae::Vec2::new(20.0, 20.0)),
            ActorFaction::Npc,
            alive(),
        ));
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(enemy).get::<ActorTarget>().unwrap();
        assert_eq!(
            target.entity, None,
            "no relation + no player → no combat target by default"
        );
        assert_eq!(target.pos, ae::Vec2::new(100.0, 100.0));
    }

    /// A drained body (health 0) for the dead-candidate filter.
    fn dead() -> BodyHealth {
        let mut h = BodyHealth::new(ambition_characters::actor::Health::new(10));
        h.damage(10);
        h
    }

    /// A `HostileToFaction` fighter (the duel mode) targets its relational
    /// faction-foe and NEVER the player baseline — even a player standing right on
    /// top of it. This is the fix for "the duel winner hunts the observer": the
    /// player can only be caught by a stray (physical damage) or by provoking the
    /// fighter (strikes), not by being the nearest body.
    #[test]
    fn relational_fighter_ignores_the_player_baseline() {
        use crate::combat::components::ActorFaction;
        let mut app = App::new();
        let mut relations = FactionRelations::default();
        relations.set_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
        app.insert_resource(relations);
        // A player stands RIGHT NEXT to the fighter (nearest body by far)...
        app.world_mut().spawn((
            PlayerEntity,
            PlayerSlot(0),
            PrimaryPlayer,
            dummy_player_body(ae::Vec2::new(105.0, 100.0)),
            alive(),
        ));
        let fighter = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(
                    ae::Vec2::new(100.0, 100.0),
                    ae::Vec2::new(20.0, 20.0),
                ),
                ActorTarget::default(),
                ActorAggression::hostile_to_faction(),
                ActorFaction::Enemy,
                alive(),
            ))
            .id();
        // ... but the fighter targets its (much farther) Boss foe, not the player.
        let foe = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(
                    ae::Vec2::new(300.0, 100.0),
                    ae::Vec2::new(20.0, 20.0),
                ),
                ActorFaction::Boss,
                alive(),
            ))
            .id();
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(fighter).get::<ActorTarget>().unwrap();
        assert_eq!(
            target.entity,
            Some(foe),
            "a HostileToFaction fighter targets its faction-foe, never the nearer player baseline"
        );
    }

    /// A dead foe is never targeted: once the foe's health is drained, the fighter
    /// goes target-less (→ stands down to peaceful downstream) instead of swinging
    /// at the corpse. Replaces the old manual pacify-on-death hack.
    #[test]
    fn a_dead_foe_is_dropped_so_the_fighter_goes_target_less() {
        use crate::combat::components::ActorFaction;
        let mut app = App::new();
        let mut relations = FactionRelations::default();
        relations.set_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
        app.insert_resource(relations);
        let fighter = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(
                    ae::Vec2::new(100.0, 100.0),
                    ae::Vec2::new(20.0, 20.0),
                ),
                ActorTarget::default(),
                ActorAggression::hostile_to_faction(),
                ActorFaction::Enemy,
                alive(),
            ))
            .id();
        // The only foe is DEAD (health 0) — and a live player is present too, but a
        // HostileToFaction fighter never falls back to it.
        app.world_mut().spawn((
            PlayerEntity,
            PlayerSlot(0),
            PrimaryPlayer,
            dummy_player_body(ae::Vec2::new(120.0, 100.0)),
            alive(),
        ));
        app.world_mut().spawn((
            FeatureSimEntity,
            CenteredAabb::from_center_size(ae::Vec2::new(300.0, 100.0), ae::Vec2::new(20.0, 20.0)),
            ActorFaction::Boss,
            dead(),
        ));
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(fighter).get::<ActorTarget>().unwrap();
        assert_eq!(
            target.entity, None,
            "a dead foe is dropped and the relational fighter goes target-less (stands down)"
        );
    }
}
