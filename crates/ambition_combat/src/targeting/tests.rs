//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::components::{ActorAggression, ActorFaction, ActorTarget, CenteredAabb};
use ambition_characters::brain::PlayerSlot;
use ambition_characters::brain::{Brain, StateMachineCfg};
use ambition_engine_core::BodyKinematics;
use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};

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
            CenteredAabb::from_center_size(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(20.0, 20.0)),
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
    use crate::components::ActorFaction;
    let mut app = App::new();
    let mut relations = FactionRelations::default();
    relations.set_hostile(ActorFaction::Enemy, ActorFaction::Npc, true);
    app.insert_resource(relations);

    // An Enemy-faction actor — no players anywhere ...
    let enemy = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            CenteredAabb::from_center_size(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(20.0, 20.0)),
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
            CenteredAabb::from_center_size(ae::Vec2::new(160.0, 100.0), ae::Vec2::new(20.0, 20.0)),
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
    use crate::components::ActorFaction;
    let mut app = App::new();
    app.insert_resource(FactionRelations::default());
    let enemy = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            CenteredAabb::from_center_size(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(20.0, 20.0)),
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
            CenteredAabb::from_center_size(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(20.0, 20.0)),
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
            CenteredAabb::from_center_size(ae::Vec2::new(140.0, 100.0), ae::Vec2::new(20.0, 20.0)),
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
    use crate::components::ActorFaction;
    let mut app = App::new();
    let mut relations = FactionRelations::default();
    relations.set_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
    app.insert_resource(relations);
    let fighter = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            CenteredAabb::from_center_size(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(20.0, 20.0)),
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
