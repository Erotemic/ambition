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
use ambition_characters::actor::BodyHealth;
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

/// Effective combat allegiance: a body currently under player control (it carries
/// [`ambition_characters::brain::Brain::Player`]) fights as [`ActorFaction::Player`]
/// regardless of its AUTHORED faction. This is why possession never overwrites
/// `ActorFaction` (no flip, no restore bookkeeping): every combat faction read —
/// targeting, damage gates, hitbox stamps — resolves through this, so a possessed
/// body attacks its former allies and is targeted by them, then reverts the
/// instant control leaves (the authored faction was never touched).
pub fn effective_faction(
    authored: ActorFaction,
    brain: Option<&ambition_characters::brain::Brain>,
) -> ActorFaction {
    if brain.is_some_and(ambition_characters::brain::Brain::is_player) {
        ActorFaction::Player
    } else {
        authored
    }
}

/// Whether an `attacker`-faction body's landed hit DAMAGES a specific `victim`
/// body — the faction baseline ([`can_damage`]) PLUS the per-entity grudge override.
///
/// A grudge is the DAMAGE-side counterpart to a [`FactionRelations`] entry: just as
/// relations make two FACTIONS hostile, a grudge makes one body hostile to one exact
/// ENTITY. So a grudge authorizes a hit even between SAME-faction bodies that
/// `can_damage` would otherwise spare — the mechanism behind two normal NPCs dueling
/// (both `Npc`, each grudging the other) without either being re-tagged a hostile
/// faction. Self-exclusion (`attacker_entity == victim_entity`) stays the caller's.
///
/// `attacker_grudge` is the firing body's [`ActorAggression::grudge`]; `None` (no
/// grudge, or a grudge-less attacker like the environment) falls straight back to the
/// faction rule. This is a strict SUPERSET of `can_damage`, so it never spares a hit
/// the faction baseline would have landed.
pub fn damage_lands(
    attacker: ActorFaction,
    victim: ActorFaction,
    friendly_fire: FriendlyFire,
    attacker_grudge: Option<Entity>,
    victim_entity: Entity,
) -> bool {
    can_damage(attacker, victim, friendly_fire) || attacker_grudge == Some(victim_entity)
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
    // The player carries an `ActorFaction` (Player) like every body — read it so the
    // player is a RELATIONAL candidate (a foe only if this actor's faction opposes
    // Player, or it holds a grudge against this player), never an unconditional one.
    players: Query<(Entity, &BodyKinematics, &BodyHealth, &ActorFaction), With<PlayerEntity>>,
    // Non-player actors are candidate targets too (the relational, non-player-
    // centric part): an actor can target another actor whose faction it's hostile
    // to. Snapshotted, so this read-only borrow ends before the mutable pass.
    // `Option<&Brain>` on both candidate and acting queries: a possessed body
    // (carrying `Brain::Player`) is a Player-EFFECTIVE candidate/actor without
    // its authored `ActorFaction` being mutated — so former allies target it and
    // it targets them, purely through effective allegiance.
    others: Query<
        (
            Entity,
            &CenteredAabb,
            &ActorFaction,
            &BodyHealth,
            Option<&ambition_characters::brain::Brain>,
        ),
        With<FeatureSimEntity>,
    >,
    mut actors: Query<
        (
            Entity,
            &CenteredAabb,
            &mut ActorTarget,
            &ActorAggression,
            Option<&ActorFaction>,
            Option<&ambition_characters::brain::Brain>,
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
    // ONE candidate set — the player is just another body, carrying faction Player.
    // No unconditional player special-case; nearest foe wins.
    let candidates: Vec<(Entity, ae::Vec2, ActorFaction)> = players
        .iter()
        .filter(|(_, _, hp, _)| hp.current() > 0)
        .map(|(e, kin, _, faction)| (e, kin.pos, *faction))
        .chain(
            others
                .iter()
                .filter(|(_, _, _, hp, _)| hp.current() > 0)
                .map(|(e, aabb, faction, _, brain)| {
                    (e, aabb.center, effective_faction(*faction, brain))
                }),
        )
        .collect();
    // Nothing to point at: leave every actor's target untouched so downstream
    // ticks keep last frame's value instead of zeroing (matches old behavior
    // when no candidates existed).
    if candidates.is_empty() {
        return;
    }
    for (self_entity, aabb, mut target, aggression, faction, brain) in actors.iter_mut() {
        let actor_pos = aabb.center;
        // The acting body's OWN effective allegiance (Player while possessed). A
        // body with neither an authored faction nor player control has no
        // faction-relational foes (only a personal grudge can point it) — same as
        // the old `faction.is_some()` gate.
        let player_controlled = brain.is_some_and(ambition_characters::brain::Brain::is_player);
        let has_allegiance = faction.is_some() || player_controlled;
        let self_faction = effective_faction(faction.copied().unwrap_or_default(), brain);
        let policy = aggression.target_policy();
        if policy == AggressionTarget::None {
            // Passive: no combat target. Point at self so a zero direction keeps
            // the actor's current facing.
            target.pos = actor_pos;
            target.entity = None;
            continue;
        }
        // One relational rule: a candidate is a FOE iff this actor's faction is
        // hostile to it (`FactionRelations`) OR this actor holds a grudge against
        // that exact entity (a provoked NPC chasing its attacker). The player is a
        // candidate like any other — it's hunted because the actor's faction opposes
        // Player (a born Enemy) or it's the grudge target (a provoked NPC), never
        // because it is "the player". Nearest foe wins.
        let mut best: Option<(Entity, ae::Vec2, f32)> = None;
        for (entity, pos, cand_faction) in &candidates {
            if *entity == self_entity {
                continue;
            }
            let is_foe = (has_allegiance && relations.is_hostile(self_faction, *cand_faction))
                || aggression.grudge == Some(*entity);
            if !is_foe {
                continue;
            }
            let d = distance_squared(*pos, actor_pos);
            // Deterministic nearest-foe selection: on an EXACT distance tie, prefer
            // the lower `Entity` so the chosen target is independent of the
            // (unstable) Query iteration order — RL/replay must not diverge on a
            // symmetric two-foe setup (fable review 2026-07-02 §B12; the
            // query-order-determinism rule). Entity is reproducible within a
            // deterministic sim; a content-stable id is only needed for cross-build
            // identity, which nearest-foe targeting does not require.
            let better = match best {
                None => true,
                Some((best_entity, _, best_d)) => d < best_d || (d == best_d && *entity < best_entity),
            };
            if better {
                best = Some((*entity, *pos, d));
            }
        }
        if let Some((entity, pos, _)) = best {
            target.pos = pos;
            target.entity = Some(entity);
        } else {
            // No valid foe (faction-neutral with no grudge, or its foe is gone):
            // point at self so facing math reads a zero direction (hold facing).
            target.pos = actor_pos;
            target.entity = None;
        }
    }
}

/// Dissolve grudges that have SETTLED, so a feud resolves to peace on its own.
///
/// A grudge is a per-entity hostility (the duel mechanism); like any feud it should
/// END once it's decided. Two rules, both keyed off the one uniform liveness
/// authority ([`BodyHealth`]):
///
/// - **You forget a slain foe.** When a body's grudge target is no longer alive, the
///   grudge clears. The targeting filter already drops a dead foe so the holder stands
///   down ([`select_actor_targets`]); clearing the grudge too means it won't re-aggro
///   if that foe later revives — the duel survivor settles into a normal NPC for good.
/// - **A defeated body forgets its feud.** When a body itself is down (health 0,
///   awaiting respawn), its own grudge clears, so it **revives grudgeless** — a
///   defeated duel fighter comes back behaving like a normal NPC, exactly as a loser
///   should, rather than resuming the fight the instant it's back on its feet.
///
/// Together these make a duel between two grudge-feuding `Npc`s resolve to mutual
/// peace with no bespoke "end the duel" code. Runs just before
/// [`select_actor_targets`] so a cleared grudge takes effect the same frame. The
/// `&BodyHealth` read overlaps the mutable-aggression query only on the (immutable)
/// health component, so there is no access conflict.
pub fn dissolve_settled_grudges(
    mut actors: Query<(&BodyHealth, &mut ActorAggression)>,
    healths: Query<&BodyHealth>,
) {
    for (self_health, mut aggression) in &mut actors {
        let Some(foe) = aggression.grudge else {
            continue;
        };
        let self_down = self_health.current() == 0;
        // An absent foe entity (despawned) counts as gone, so a grudge never dangles.
        let foe_down = healths.get(foe).map(|h| h.current() == 0).unwrap_or(true);
        if self_down || foe_down {
            aggression.grudge = None;
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
    use crate::combat::components::{ActorAggression, ActorFaction, ActorTarget, CenteredAabb};
    use crate::player::PlayerSlot;
    use ambition_characters::brain::{Brain, StateMachineCfg};

    /// Effective allegiance: a body carrying `Brain::Player` fights as `Player`
    /// regardless of its authored faction (that's why possession never mutates
    /// `ActorFaction`); any other brain — or none — keeps the authored faction.
    #[test]
    fn effective_faction_maps_player_brain_to_player_side() {
        let player_brain = Brain::Player(PlayerSlot::PRIMARY);
        let ai_brain = Brain::StateMachine(StateMachineCfg::StandStill);
        // A possessed enemy: authored Enemy, but player-controlled ⇒ Player.
        assert_eq!(
            effective_faction(ActorFaction::Enemy, Some(&player_brain)),
            ActorFaction::Player,
        );
        // Same body, autonomous AI brain ⇒ keeps authored Enemy.
        assert_eq!(
            effective_faction(ActorFaction::Enemy, Some(&ai_brain)),
            ActorFaction::Enemy,
        );
        // No brain ⇒ authored faction unchanged.
        assert_eq!(
            effective_faction(ActorFaction::Boss, None),
            ActorFaction::Boss,
        );
    }

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

    // A born-hostile enemy: faction Enemy (relationally hostile to Player by the
    // FactionRelations default), so it hunts the player along faction lines — no
    // grudge, no player-named mode.
    fn enemy_at(app: &mut App, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(pos, ae::Vec2::new(20.0, 20.0)),
                ActorTarget::default(),
                ActorAggression::hostile(),
                ActorFaction::Enemy,
                alive(),
            ))
            .id()
    }

    // Spawn a player body carrying faction Player — a relational candidate like any
    // other body (the production player always has this faction).
    fn spawn_player(app: &mut App, slot: u8, primary: bool, pos: ae::Vec2) -> Entity {
        let mut e = app.world_mut().spawn((
            PlayerEntity,
            PlayerSlot(slot),
            dummy_player_body(pos),
            ActorFaction::Player,
            alive(),
        ));
        if primary {
            e.insert(PrimaryPlayer);
        }
        e.id()
    }

    #[test]
    fn target_points_at_only_player_when_one_present() {
        let mut app = App::new();
        let player = spawn_player(&mut app, 0, true, ae::Vec2::new(300.0, 100.0));
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
        spawn_player(&mut app, 0, true, ae::Vec2::new(100.0, 100.0));
        let p2 = spawn_player(&mut app, 1, false, ae::Vec2::new(500.0, 100.0));
        let enemy = enemy_at(&mut app, ae::Vec2::new(450.0, 100.0));
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(enemy).get::<ActorTarget>().unwrap();
        assert_eq!(target.pos, ae::Vec2::new(500.0, 100.0));
        assert_eq!(target.entity, Some(p2));
    }

    #[test]
    fn nearest_foe_tie_breaks_to_lower_entity_deterministically() {
        // Two foes EXACTLY equidistant from the actor (x=100 and x=500 vs an
        // enemy at x=300 → both distance² 40000). The chosen target must be the
        // lower `Entity`, independent of the (unstable) Query iteration order —
        // a symmetric two-foe setup must not diverge across runs (§B12).
        let mut app = App::new();
        let p1 = spawn_player(&mut app, 0, true, ae::Vec2::new(100.0, 100.0));
        let p2 = spawn_player(&mut app, 1, false, ae::Vec2::new(500.0, 100.0));
        // The deterministic tie winner is the lesser `Entity` by Ord — a fixed
        // function of the candidate set, NOT of spawn or Query-visit order (Bevy's
        // Entity Ord is not spawn-monotonic, which is exactly why the winner must
        // be pinned to `min`, not "first seen").
        let expected = p1.min(p2);
        let enemy = enemy_at(&mut app, ae::Vec2::new(300.0, 100.0));
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(enemy).get::<ActorTarget>().unwrap();
        assert_eq!(
            target.entity,
            Some(expected),
            "an exact distance tie must resolve to the min Entity, not Query order",
        );
    }

    #[test]
    fn passive_aggression_targets_self_not_player() {
        let mut app = App::new();
        spawn_player(&mut app, 0, true, ae::Vec2::new(999.0, 999.0));
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
    fn a_peaceful_npc_ignores_the_player_until_it_holds_a_grudge() {
        // Relational targeting: a faction-Npc `RetaliatesWhenHit` NPC is NOT hostile
        // to Player (FactionRelations baseline), so before it's provoked it has no
        // foe and takes no target — it patrols/idles, it does not stalk the player.
        // Provoking it sets a GRUDGE against the attacker, and THEN it hunts that
        // exact entity (no faction-identity mutation).
        let mut app = App::new();
        let player = spawn_player(&mut app, 0, true, ae::Vec2::new(200.0, 100.0));
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
                ActorFaction::Npc,
            ))
            .id();
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(npc).get::<ActorTarget>().unwrap();
        assert_eq!(
            target.entity, None,
            "an unprovoked peaceful NPC has no foe — it does not track the player"
        );
        assert_eq!(
            target.pos,
            ae::Vec2::new(100.0, 100.0),
            "holds its own position"
        );

        // Provoke it: a grudge against the player makes it hunt that entity.
        app.world_mut()
            .get_mut::<ActorAggression>(npc)
            .unwrap()
            .grudge = Some(player);
        app.update();
        let target = app.world().entity(npc).get::<ActorTarget>().unwrap();
        assert_eq!(
            target.entity,
            Some(player),
            "once it holds a grudge it hunts that exact entity (the player)"
        );
    }

    #[test]
    fn an_actor_with_no_foe_points_at_itself() {
        // A born-hostile Enemy alone in the world (no player, no faction-foe) has no
        // one to chase: it points at itself so facing math holds (a zero direction),
        // and clears any stale target entity. (The "leave targets untouched" early
        // return only fires for a genuinely EMPTY candidate set — no body carries a
        // faction — a degenerate pre-spawn case.)
        let mut app = App::new();
        let enemy = enemy_at(&mut app, ae::Vec2::new(100.0, 100.0));
        app.world_mut()
            .entity_mut(enemy)
            .get_mut::<ActorTarget>()
            .unwrap()
            .pos = ae::Vec2::new(42.0, 42.0);
        app.add_systems(Update, select_actor_targets);
        app.update();
        let target = app.world().entity(enemy).get::<ActorTarget>().unwrap();
        assert_eq!(
            target.pos,
            ae::Vec2::new(100.0, 100.0),
            "no foe → point at self"
        );
        assert_eq!(target.entity, None);
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
                ActorAggression::hostile(),
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
                ActorAggression::hostile(),
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

    #[test]
    fn a_grudge_lands_a_hit_between_same_faction_bodies() {
        // The duel mechanism: two `Npc` bodies normally can't hurt each other
        // (`can_damage(Npc, Npc)` is false with friendly fire off), but a grudge
        // against the exact victim entity authorizes the hit anyway — without
        // re-tagging either as a hostile faction.
        let mut app = App::new();
        let rival = app.world_mut().spawn_empty().id();
        let bystander = app.world_mut().spawn_empty().id();
        let ff = FriendlyFire { enabled: false };

        // Same faction, no grudge → spared.
        assert!(
            !damage_lands(ActorFaction::Npc, ActorFaction::Npc, ff, None, rival),
            "same-faction non-grudged allies are spared (friendly fire off)"
        );
        // Same faction, grudge against THIS victim → lands.
        assert!(
            damage_lands(ActorFaction::Npc, ActorFaction::Npc, ff, Some(rival), rival),
            "a grudge against the victim authorizes a same-faction hit"
        );
        // Grudge against someone ELSE → this victim still spared.
        assert!(
            !damage_lands(
                ActorFaction::Npc,
                ActorFaction::Npc,
                ff,
                Some(bystander),
                rival
            ),
            "a grudge against a different entity does not authorize hitting this one"
        );
    }

    #[test]
    fn a_settled_grudge_dissolves_so_a_duel_ends_in_peace() {
        // Two `Npc` duelists grudging each other. When one is defeated (health 0),
        // BOTH grudges must dissolve: the slain fighter forgets its feud (revives
        // grudgeless → normal NPC), and the survivor forgets a foe it can no longer
        // see (won't re-aggro if the loser revives). The feud resolves to peace with
        // no bespoke duel-end code.
        let mut app = App::new();
        let a = app
            .world_mut()
            .spawn((alive(), ActorAggression::hostile()))
            .id();
        let b = app
            .world_mut()
            .spawn((alive(), ActorAggression::hostile()))
            .id();
        // Cross-wire the mutual grudge.
        app.world_mut()
            .get_mut::<ActorAggression>(a)
            .unwrap()
            .grudge = Some(b);
        app.world_mut()
            .get_mut::<ActorAggression>(b)
            .unwrap()
            .grudge = Some(a);
        app.add_systems(Update, dissolve_settled_grudges);

        // Both alive → grudges persist (the fight is on).
        app.update();
        assert_eq!(
            app.world().get::<ActorAggression>(a).unwrap().grudge,
            Some(b)
        );
        assert_eq!(
            app.world().get::<ActorAggression>(b).unwrap().grudge,
            Some(a)
        );

        // Defeat B (drain its health to 0).
        app.world_mut().get_mut::<BodyHealth>(b).unwrap().damage(10);
        app.update();
        assert_eq!(
            app.world().get::<ActorAggression>(a).unwrap().grudge,
            None,
            "the survivor forgets a slain foe (won't re-aggro if it revives)"
        );
        assert_eq!(
            app.world().get::<ActorAggression>(b).unwrap().grudge,
            None,
            "the defeated fighter forgets its own feud (revives a normal NPC)"
        );
    }

    #[test]
    fn damage_lands_is_a_strict_superset_of_can_damage() {
        // Every cross-faction hit the faction baseline lands, `damage_lands` also
        // lands — regardless of grudge. The grudge can only ADD authorization, never
        // remove it.
        let ff = FriendlyFire { enabled: false };
        let mut app = App::new();
        let some = app.world_mut().spawn_empty().id();
        for grudge in [None, Some(some)] {
            assert!(
                damage_lands(ActorFaction::Enemy, ActorFaction::Player, ff, grudge, some),
                "a cross-faction hit always lands (grudge={grudge:?})"
            );
        }
    }

    /// A drained body (health 0) for the dead-candidate filter.
    fn dead() -> BodyHealth {
        let mut h = BodyHealth::new(ambition_characters::actor::Health::new(10));
        h.damage(10);
        h
    }

    /// The general relational-targeting path: a fighter whose faction is hostile to
    /// another faction (here Enemy↔Boss, via `FactionRelations`) targets the nearest
    /// such foe, and a non-hostile bystander (the player, when relations don't oppose
    /// it) is only caught if it becomes the NEAREST candidate. (The spectator duel no
    /// longer rides this faction path — it uses a mutual grudge between two `Npc`s —
    /// but actor-vs-actor faction hostility is still a real capability, pinned here.)
    #[test]
    fn relational_fighter_targets_nearest_foe_observer_spared_by_distance() {
        let mut app = App::new();
        let mut relations = FactionRelations::default();
        relations.set_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
        app.insert_resource(relations);
        // The duel: fighter (Enemy) + its Boss foe stand NEAR each other; the
        // observing player is far off to the side (the real `<<duel>>` staging).
        let fighter = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(
                    ae::Vec2::new(100.0, 100.0),
                    ae::Vec2::new(20.0, 20.0),
                ),
                ActorTarget::default(),
                ActorAggression::hostile(),
                ActorFaction::Enemy,
                alive(),
            ))
            .id();
        let foe = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(
                    ae::Vec2::new(140.0, 100.0),
                    ae::Vec2::new(20.0, 20.0),
                ),
                ActorFaction::Boss,
                alive(),
            ))
            .id();
        let player = spawn_player(&mut app, 0, true, ae::Vec2::new(600.0, 100.0));
        app.add_systems(Update, select_actor_targets);
        app.update();
        // The Boss foe (40px away) is nearer than the far observer (500px) → the
        // fighter duels the Boss, sparing the distant player. The player IS a
        // relational candidate (Enemy opposes Player by default), so a player who
        // walks INTO the fight (becomes nearest) gets caught — the documented duel
        // behavior. Strict observer-immunity would need per-room relations scoping
        // (clear Enemy→Player only in the arena) — a separate follow-up.
        assert_eq!(
            app.world()
                .entity(fighter)
                .get::<ActorTarget>()
                .unwrap()
                .entity,
            Some(foe),
            "the fighter duels its nearer Boss foe, not the distant observer"
        );

        // Move the player on top of the fighter → it becomes the nearest foe.
        app.world_mut()
            .get_mut::<BodyKinematics>(player)
            .unwrap()
            .pos = ae::Vec2::new(101.0, 100.0);
        app.update();
        assert_eq!(
            app.world()
                .entity(fighter)
                .get::<ActorTarget>()
                .unwrap()
                .entity,
            Some(player),
            "a player who walks into the duel (nearest foe) gets caught"
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
                ActorAggression::hostile(),
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
