use super::*;
use crate::components::{
    ActiveCombatant, ActorAggression, ActorDisposition, ActorFaction, ActorTarget, CenteredAabb,
};
use ambition_characters::control::DrivingParticipant;
use ambition_characters::control::PlayerSlot;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

/// Effective allegiance: a body a participant drives fights as `Player`
/// regardless of its authored faction (that's why possession never mutates
/// `ActorFaction`); a body nobody drives keeps the authored faction.
#[test]
fn effective_faction_maps_a_driven_body_to_the_player_side() {
    let driven = DrivingParticipant(PlayerSlot::PRIMARY);
    // A possessed enemy: authored Enemy, but participant-driven  Player.
    assert_eq!(
        effective_faction(ActorFaction::Enemy, Some(&driven)),
        ActorFaction::Player,
    );
    // Nobody drives it  keeps authored Enemy, whatever its AI policy is.
    assert_eq!(
        effective_faction(ActorFaction::Enemy, None),
        ActorFaction::Enemy,
    );
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
fn nearest_foe_tie_breaks_on_stable_identity_not_entity_id() {
    // Two foes EXACTLY equidistant from the actor (x=100 and x=500 vs an enemy
    // at x=300 → both distance² 40000). The winner must be a function of STABLE
    // SEMANTIC IDENTITY, never of `Entity`.
    //
    // Why not `Entity`: bevy_ggrs destroys and recreates rollback entities, so
    // an actor's raw id is not preserved across a rewind. A tie-break that
    // compares ids can therefore pick a DIFFERENT target mid-resimulation than
    // the confirmed timeline did — a desync that only shows up on a symmetric
    // setup, which is precisely the case this test builds.
    let mut app = App::new();
    let p1 = spawn_player(&mut app, 0, true, ae::Vec2::new(100.0, 100.0));
    let p2 = spawn_player(&mut app, 1, false, ae::Vec2::new(500.0, 100.0));
    // Give the two candidates stable identities in the OPPOSITE order to their
    // entity ids, so "sorted by SimId" and "min Entity" disagree and the
    // assertion can only pass for the identity rule.
    let (low_entity, high_entity) = (p1.min(p2), p1.max(p2));
    app.world_mut()
        .entity_mut(low_entity)
        .insert(SimId::player_slot(9));
    app.world_mut()
        .entity_mut(high_entity)
        .insert(SimId::player_slot(1));
    let enemy = enemy_at(&mut app, ae::Vec2::new(300.0, 100.0));
    app.add_systems(Update, select_actor_targets);
    app.update();
    let target = app.world().entity(enemy).get::<ActorTarget>().unwrap();
    assert_eq!(
        target.entity,
        Some(high_entity),
        "an exact distance tie must resolve by stable SimId (slot:1 sorts before \
         slot:9), NOT by comparing raw Entity ids — those are not stable across \
         a GGRS rollback's entity recreation",
    );
}

#[test]
fn nearest_foe_tie_is_still_deterministic_without_stable_ids() {
    // Bodies with no `SimId` are not snapshot-relevant, but the tie must still
    // land somewhere reproducible rather than following Query order. They fall
    // back to entity order among themselves.
    let mut app = App::new();
    let p1 = spawn_player(&mut app, 0, true, ae::Vec2::new(100.0, 100.0));
    let p2 = spawn_player(&mut app, 1, false, ae::Vec2::new(500.0, 100.0));
    let expected = p1.min(p2);
    let enemy = enemy_at(&mut app, ae::Vec2::new(300.0, 100.0));
    app.add_systems(Update, select_actor_targets);
    app.update();
    let target = app.world().entity(enemy).get::<ActorTarget>().unwrap();
    assert_eq!(target.entity, Some(expected));
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

/// A dead foe is never targeted: once the foe's health is drained, targeting
/// drops it and stands a non-match fighter down instead of swinging at the corpse.
/// Replaces the old manual pacify-on-death hack.
#[test]
fn a_dead_foe_is_dropped_so_the_fighter_goes_target_less() {
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
            ActorDisposition::Hostile,
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
        "a dead foe is dropped and the relational fighter goes target-less"
    );
    assert_eq!(
        *app.world().entity(fighter).get::<ActorDisposition>().unwrap(),
        ActorDisposition::Peaceful,
        "target selection owns stand-down, so the disposition changes in the same tick"
    );
}

/// ⭐ AND IT STANDS BACK UP. The stand-down above is a TEMPORARY standing, so
/// the same authority owes the other direction: a fighter that lost its foe and
/// went `Peaceful` must be `Hostile` again on the tick it reacquires one.
///
/// ⛔ WHAT THIS DEFENDS. Reacquisition reads `aggression.target_policy()`, NOT
/// the disposition — so before this, a body came back to the fight without
/// coming back to being hostile, and then attacked a foe while `Peaceful` told
/// the interaction system it was talkable and `CombatStanding` called it a
/// Bystander. Two authorities disagreeing about one fact.
#[test]
fn a_fighter_that_reacquires_a_foe_is_hostile_again() {
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
            ActorDisposition::Hostile,
            ActorFaction::Enemy,
            alive(),
        ))
        .id();
    app.add_systems(Update, select_actor_targets);

    // No foe in the world at all: it stands down. This is the PREMISE, and
    // without it the assertion below would pass on a body that never moved.
    app.update();
    assert_eq!(
        *app.world().entity(fighter).get::<ActorDisposition>().unwrap(),
        ActorDisposition::Peaceful,
        "the premise: a target-less hostile stands down"
    );

    // A live faction foe arrives.
    app.world_mut().spawn((
        FeatureSimEntity,
        CenteredAabb::from_center_size(ae::Vec2::new(300.0, 100.0), ae::Vec2::new(20.0, 20.0)),
        ActorFaction::Boss,
        alive(),
    ));
    app.update();

    let entity = app.world().entity(fighter);
    assert!(
        entity.get::<ActorTarget>().unwrap().entity.is_some(),
        "the aggression reacquired a foe, which is what makes the standing question live"
    );
    assert_eq!(
        *entity.get::<ActorDisposition>().unwrap(),
        ActorDisposition::Hostile,
        "it is fighting again while its disposition still says Peaceful - dialogue would be \
         offered to a body that is mid-swing, and CombatStanding would call it a Bystander"
    );
}

#[test]
fn active_match_combatant_stays_hostile_when_target_less() {
    let mut app = App::new();
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
            ActorDisposition::Hostile,
            ActorFaction::Enemy,
            alive(),
            ActiveCombatant,
        ))
        .id();

    app.add_systems(Update, select_actor_targets);
    app.update();

    assert_eq!(
        app.world().entity(fighter).get::<ActorTarget>().unwrap().entity,
        None,
        "a fighter with no live foe is target-less"
    );
    assert_eq!(
        *app.world().entity(fighter).get::<ActorDisposition>().unwrap(),
        ActorDisposition::Hostile,
        "match combatants keep match hostility even while target-less"
    );
}

/// `effective_faction` maps ANY participant-driven body to `Player`, which is
/// load-bearing for possession and fatal for a match: two humans are always the
/// same faction no matter what the roster declared. A versus stage got around
/// that by switching on GLOBAL friendly fire, which is right for a free-for-all
/// and wrong the moment a 2v2 exists — it makes teammates hittable too, and it
/// is a world-wide rule change made by one stage.
#[test]
fn teams_decide_between_two_bodies_that_share_a_faction() {
    use super::{damage_lands_between, ActorFaction, FriendlyFire, MatchTeam};

    let blue = MatchTeam::new("blue");
    let red = MatchTeam::new("red");
    let victim = Entity::from_raw_u32(7).expect("a valid test entity id");
    let no_ff = FriendlyFire { enabled: false };

    // Same faction (two humans), DIFFERENT teams: the hit lands.
    assert!(damage_lands_between(
        ActorFaction::Player,
        ActorFaction::Player,
        Some(&blue),
        Some(&red),
        no_ff,
        None,
        victim,
    ));

    // Same faction, SAME team: it does not. This is the 2v2 case global
    // friendly fire could never express.
    assert!(!damage_lands_between(
        ActorFaction::Player,
        ActorFaction::Player,
        Some(&blue),
        Some(&blue),
        no_ff,
        None,
        victim,
    ));

    // DIFFERENT factions but the same team — an escorted ally, a possessed
    // teammate. The team wins: it is the ruleset's statement and the faction is
    // the world's.
    assert!(!damage_lands_between(
        ActorFaction::Player,
        ActorFaction::Enemy,
        Some(&blue),
        Some(&blue),
        no_ff,
        None,
        victim,
    ));

    // A grudge still overrides a shared team: a per-entity feud is deliberately
    // stronger than any group rule.
    assert!(damage_lands_between(
        ActorFaction::Player,
        ActorFaction::Player,
        Some(&blue),
        Some(&blue),
        no_ff,
        Some(victim),
        victim,
    ));

    // And a body with NO team is judged exactly as before — nothing outside a
    // match notices teams exist.
    assert!(damage_lands_between(
        ActorFaction::Player,
        ActorFaction::Enemy,
        None,
        None,
        no_ff,
        None,
        victim,
    ));
    assert!(!damage_lands_between(
        ActorFaction::Player,
        ActorFaction::Player,
        Some(&blue),
        None,
        no_ff,
        None,
        victim,
    ));
}

/// ⛔⛔ A FIGHTER WAITING OUT ITS DEATH BEAT IS NOT A TARGET — AND IT IS AT FULL
/// HEALTH, WHICH IS WHY THE OLD GATE COULD NOT SEE IT.
///
/// Selection filtered on `hp.current() > 0` under a comment calling health "the
/// one uniform liveness gate". D201 made that false: the stock spend calls
/// `health.reset()` the instant the stock is spent, because a fighter comes back
/// FRESH — so a body lying untouchable at the blast line reads full health for
/// the whole respawn interval. A surviving CPU went on selecting, chasing and
/// aiming at it, and the hit filters that stop it HURTING that body say nothing
/// about where it walks.
///
/// ⭐ THE NEARER FOE IS THE OUT-OF-PLAY ONE, deliberately. Nearest wins, so if
/// the gate does nothing the hunter locks the dead body — and the farther live
/// foe is the only answer that proves the gate ran rather than the geometry.
#[test]
fn a_body_waiting_to_respawn_is_not_hunted_though_it_is_at_full_health() {
    let mut app = App::new();
    let hunter = enemy_at(&mut app, ae::Vec2::new(0.0, 0.0));
    let waiting = spawn_player(&mut app, 0, true, ae::Vec2::new(100.0, 0.0));
    let live = spawn_player(&mut app, 1, false, ae::Vec2::new(400.0, 0.0));
    app.add_systems(Update, select_actor_targets);

    // Premise: with both in play the NEARER one is chosen, so the swap below is
    // the out-of-play gate and not a change of mind about distance.
    app.update();
    assert_eq!(
        app.world().entity(hunter).get::<ActorTarget>().unwrap().entity,
        Some(waiting),
        "the nearer foe was not chosen, so this fixture cannot show the gate"
    );

    app.world_mut()
        .entity_mut(waiting)
        .insert(crate::death_rules::OutOfPlay);
    app.update();
    assert_eq!(
        app.world().entity(hunter).get::<ActorTarget>().unwrap().entity,
        Some(live),
        "the hunter kept aiming at a body that had left play — it is at FULL \
         HEALTH for the whole respawn interval, so the health gate never saw it"
    );

    // …and it is a foe again the moment it comes back.
    app.world_mut()
        .entity_mut(waiting)
        .remove::<crate::death_rules::OutOfPlay>();
    app.update();
    assert_eq!(
        app.world().entity(hunter).get::<ActorTarget>().unwrap().entity,
        Some(waiting),
        "a returned fighter stayed invisible to targeting"
    );
}

/// ⛔ AND AN OUT-OF-PLAY HUNTER DOES NOT ACQUIRE EITHER. The world's hands are
/// off it, which has to mean its hands are off the world: a fighter that
/// refreshed its own `ActorTarget` while dead came back holding a lock it picked
/// during the interlude.
#[test]
fn a_hunter_that_has_left_play_does_not_pick_up_a_target() {
    let mut app = App::new();
    let hunter = enemy_at(&mut app, ae::Vec2::new(0.0, 0.0));
    app.world_mut()
        .entity_mut(hunter)
        .insert(crate::death_rules::OutOfPlay);
    let prey = spawn_player(&mut app, 0, true, ae::Vec2::new(100.0, 0.0));
    app.add_systems(Update, select_actor_targets);
    app.update();
    assert_eq!(
        app.world().entity(hunter).get::<ActorTarget>().unwrap().entity,
        None,
        "a body that has left play acquired a target while it was out"
    );

    app.world_mut()
        .entity_mut(hunter)
        .remove::<crate::death_rules::OutOfPlay>();
    app.update();
    assert_eq!(
        app.world().entity(hunter).get::<ActorTarget>().unwrap().entity,
        Some(prey),
        "the hunter never acquired at all, so the arm above proves nothing"
    );
}
