//! Unit tests for hitbox AABB resolution and the despawn/overlap lifecycle.

use super::*;
use bevy::prelude::*;

fn dummy_entity() -> Entity {
    Entity::from_raw_u32(42).expect("nonzero raw entity index")
}

#[test]
fn hitbox_knockback_units_remain_distinct() {
    assert_eq!(
        resolved_hitbox_knockback_magnitude(HitboxKnockback::FeelScale(1.6), 80, 2.0, 0.0),
        HitKnockbackMagnitude::FeelScale(1.6),
        "world damage-box feel scales do not become engine-unit speeds"
    );
    assert_eq!(
        resolved_hitbox_knockback_magnitude(
            HitboxKnockback::LaunchSpeed {
                base: 120.0,
                growth: 2.0,
            },
            30,
            2.0,
            0.0,
        ),
        HitKnockbackMagnitude::LaunchSpeed(150.0),
        "melee launch speed growth resolves in engine units"
    );
}

/// FollowOwner anchor re-resolves position each tick: moving
/// the owner moves the hitbox without per-frame component update.
#[test]
fn follow_owner_hitbox_aabb_tracks_owner_position() {
    let hitbox = Hitbox {
        strike_sfx: None,
        owner: dummy_entity(),
        source: HitSide::Enemy,
        anchor: HitboxAnchor::FollowOwner {
            local_offset: ae::Vec2::new(-20.0, 0.0),
        },
        half_extent: ae::Vec2::new(10.0, 10.0),
        shape: None,
        facing: 1.0,
        damage: 1,
        knockback: crate::strike::HitboxKnockback::FeelScale(0.0),
        launch_dir: None,
        frame_down: ae::Vec2::new(0.0, 1.0),
    };
    let aabb_a = hitbox.world_aabb(ae::Vec2::new(100.0, 100.0));
    let aabb_b = hitbox.world_aabb(ae::Vec2::new(200.0, 100.0));
    assert_eq!(aabb_a.center(), ae::Vec2::new(80.0, 100.0));
    assert_eq!(aabb_b.center(), ae::Vec2::new(180.0, 100.0));
    // Half-extent translates into a full-size AABB; the local
    // offset doesn't change shape.
    assert_eq!(aabb_a.half_size(), ae::Vec2::new(10.0, 10.0));
}

/// World anchor is a fixed world rectangle regardless of owner.
#[test]
fn world_anchor_hitbox_ignores_owner_position() {
    let hitbox = Hitbox {
        strike_sfx: None,
        owner: dummy_entity(),
        source: HitSide::Boss,
        anchor: HitboxAnchor::World {
            center: ae::Vec2::new(500.0, 600.0),
        },
        half_extent: ae::Vec2::new(40.0, 40.0),
        shape: None,
        facing: 1.0,
        damage: 1,
        knockback: crate::strike::HitboxKnockback::FeelScale(0.0),
        launch_dir: None,
        frame_down: ae::Vec2::new(0.0, 1.0),
    };
    let aabb_a = hitbox.world_aabb(ae::Vec2::new(0.0, 0.0));
    let aabb_b = hitbox.world_aabb(ae::Vec2::new(9999.0, 9999.0));
    assert_eq!(aabb_a.center(), ae::Vec2::new(500.0, 600.0));
    assert_eq!(aabb_b.center(), ae::Vec2::new(500.0, 600.0));
}

/// `tick_and_despawn_hitboxes` advances `remaining_s` by
/// `world_time.sim_dt()` and despawns when it hits zero. A
/// short-lifetime hitbox should not survive a single tick at
/// the default 1/60s sim dt.
fn make_app_with_sim_dt(sim_dt: f32) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<WorldTime>();
    // WorldTime::default() leaves scaled_dt = 0, which would
    // freeze every gameplay timer; bump it so the despawn
    // assertions actually advance the lifetime.
    let mut world_time = app.world_mut().resource_mut::<WorldTime>();
    world_time.scaled_dt = sim_dt;
    world_time.raw_dt = sim_dt;
    app
}

#[test]
fn tick_and_despawn_drops_expired_hitboxes() {
    let mut app = make_app_with_sim_dt(0.05);
    app.add_systems(Update, tick_and_despawn_hitboxes);
    let hitbox = app
        .world_mut()
        .spawn((
            Hitbox {
                strike_sfx: None,
                owner: dummy_entity(),
                source: HitSide::Enemy,
                anchor: HitboxAnchor::FollowOwner {
                    local_offset: ae::Vec2::ZERO,
                },
                half_extent: ae::Vec2::new(10.0, 10.0),
                shape: None,
                facing: 1.0,
                damage: 1,
                knockback: crate::strike::HitboxKnockback::FeelScale(0.0),
                launch_dir: None,
                frame_down: ae::Vec2::new(0.0, 1.0),
            },
            HitboxLifetime { remaining_s: 0.01 },
            HitboxHits::default(),
        ))
        .id();
    // 50ms sim_dt burns through the 10ms lifetime in one tick.
    app.update();
    assert!(
        app.world().get_entity(hitbox).is_err(),
        "hitbox entity should be despawned after lifetime expired",
    );
}

/// A hitbox with `remaining_s` larger than one tick should
/// stay alive after a single update.
#[test]
fn tick_and_despawn_keeps_live_hitboxes() {
    let mut app = make_app_with_sim_dt(0.05);
    app.add_systems(Update, tick_and_despawn_hitboxes);
    let hitbox = app
        .world_mut()
        .spawn((
            Hitbox {
                strike_sfx: None,
                owner: dummy_entity(),
                source: HitSide::Enemy,
                anchor: HitboxAnchor::FollowOwner {
                    local_offset: ae::Vec2::ZERO,
                },
                half_extent: ae::Vec2::new(10.0, 10.0),
                shape: None,
                facing: 1.0,
                damage: 1,
                knockback: crate::strike::HitboxKnockback::FeelScale(0.0),
                launch_dir: None,
                frame_down: ae::Vec2::new(0.0, 1.0),
            },
            HitboxLifetime { remaining_s: 5.0 },
            HitboxHits::default(),
        ))
        .id();
    app.update();
    assert!(
        app.world().get_entity(hitbox).is_ok(),
        "hitbox with multi-second lifetime should survive a single tick",
    );
}

#[derive(Resource, Default)]
struct CapturedHits(Vec<HitEvent>);

impl CapturedHits {
    /// The hits that name a body. A body-owned strike publishes one of these per
    /// contact its shared resolver actually selected — and these are the only
    /// events any assertion about VICTIMS should count.
    fn body_hits(&self) -> Vec<&HitEvent> {
        self.0
            .iter()
            .filter(|e| matches!(e.target, HitTarget::Body(_)))
            .collect()
    }

    /// The unresolved half of the same strike: the geometry broadcast for the
    /// things a body resolver cannot name (a breakable, a boss encounter).
    fn unresolved_feature_hits(&self) -> Vec<&HitEvent> {
        self.0
            .iter()
            .filter(|e| matches!(e.target, HitTarget::UnresolvedFeatures))
            .collect()
    }

    /// A body-owned melee must never emit one; the wielded world-AOE primitive is a different
    /// anchor and has its own tests.
    fn assert_no_body_scanning_broadcast(&self) {
        assert!(
            !self
                .0
                .iter()
                .any(|e| matches!(e.target, HitTarget::Volume)),
            "a body-owned melee must not broadcast a body-scanning Volume hit — \
             bodies are resolved by identity and would be damaged twice"
        );
    }
}

#[derive(Resource, Default)]
struct CapturedLandedHits(Vec<LandedBodyHit>);

fn capture_hits(mut reader: MessageReader<HitEvent>, mut cap: ResMut<CapturedHits>) {
    for e in reader.read() {
        cap.0.push(e.clone());
    }
}

fn capture_landed_hits(
    mut reader: MessageReader<LandedBodyHit>,
    mut cap: ResMut<CapturedLandedHits>,
) {
    cap.0.extend(reader.read().cloned());
}

/// Structural tangibility gate: a dead body is an intangible corpse — a swing passes through it,
/// producing NO hit event. A live body in the exact same spot IS struck and emits one, so the
/// silence is the tangibility gate, not the geometry. Removing the `body_is_corpse` skip in
/// `apply_hitbox_damage` reintroduces the phantom hit (and, through it, the corpse flash).
#[test]
fn a_dead_victim_is_intangible_to_a_swing() {
    use ambition_characters::actor::{BodyHealth, Health};

    fn arena_with_victim_hp(current: i32) -> (App, Entity) {
        let mut relations = FactionRelations::default();
        relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
        let (mut app, victim) = arena_hitbox_app(relations, ActorFaction::Boss);
        app.world_mut()
            .entity_mut(victim)
            .insert(BodyHealth::new(Health {
                current,
                max: 3,
                invulnerable: Default::default(),
            }));
        app.update();
        (app, victim)
    }

    // Live control: HP 3 → the swing lands and emits exactly one hit event.
    let (live, _) = arena_with_victim_hp(3);
    assert_eq!(
        live.world().resource::<CapturedHits>().body_hits().len(),
        1,
        "a living body in the swing is struck"
    );

    // Corpse: HP 0 → intangible. No hit ON THE BODY, so nothing downstream
    // presents an impact at it. the swing itself still occurs and still
    // publishes its unresolved half — intangibility is a property of the corpse,
    // not a cancellation of the attack.
    let (dead, _) = arena_with_victim_hp(0);
    let captured = dead.world().resource::<CapturedHits>();
    assert!(
        captured.body_hits().is_empty(),
        "a corpse is not a hurtbox — the swing passes through, no hit lands"
    );
}

/// The unification keystone: a **Player-faction** hitbox (a wielded boss
/// AOE) emits exactly one attacker-side Volume `HitEvent` that
/// `apply_feature_hit_events` then resolves against enemies/bosses — the
/// same primitive a Boss-faction hitbox uses to hit the player.
#[test]
fn player_faction_hitbox_emits_an_attacker_side_feature_hit() {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());
    let owner = app
        .world_mut()
        .spawn(ae::CenteredAabb::new(
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(12.0, 16.0),
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            strike_sfx: None,
            owner,
            source: HitSide::Player,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::new(200.0, 80.0),
            },
            half_extent: ae::Vec2::new(60.0, 30.0),
            shape: None,
            facing: 1.0,
            damage: 5,
            knockback: crate::strike::HitboxKnockback::FeelScale(1.0),
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));
    app.update();
    let cap = app.world().resource::<CapturedHits>();
    assert_eq!(
        cap.0.len(),
        1,
        "player AOE emits exactly one feature-damaging hit"
    );
    assert!(
        matches!(cap.0[0].source, HitSource::Melee),
        "carries an attacker-side player source so apply_feature_hit_events applies it"
    );
    assert!(cap.0[0].source.seeks_victims());
    assert!(
        matches!(cap.0[0].target, HitTarget::Volume),
        "volume hit (every overlapping actor/boss)"
    );
    assert_eq!(cap.0[0].damage, 5);
}

// ── S3e: relational actor-vs-actor melee ────────────────────────────────────

use crate::targeting::FactionRelations;

/// Spawn an Enemy-source hitbox at `center` (World anchor) dealing `damage`, plus
/// an actor victim of `victim_faction` overlapping it. Returns (app, victim).
fn arena_hitbox_app(relations: FactionRelations, victim_faction: ActorFaction) -> (App, Entity) {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.init_resource::<CapturedHits>();
    app.insert_resource(relations);
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());
    let owner = app
        .world_mut()
        .spawn(ae::CenteredAabb::new(
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(12.0, 16.0),
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            strike_sfx: None,
            owner,
            source: HitSide::Enemy,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::new(100.0, 100.0),
            },
            half_extent: ae::Vec2::new(30.0, 30.0),
            shape: None,
            facing: 1.0,
            damage: 4,
            knockback: crate::strike::HitboxKnockback::FeelScale(0.0),
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));
    let victim = app
        .world_mut()
        .spawn((
            ae::CenteredAabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(14.0, 20.0)),
            victim_faction,
            ambition_platformer2d_core::BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            ambition_platformer2d_core::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
        ))
        .id();
    (app, victim)
}

/// An Enemy swing damages a Boss-faction body when the relations matrix marks
/// them mutually hostile (a spectator arena). The hit is PRE-RESOLVED to that
/// exact body via `HitTarget::Body`, so the actor-damage consumer lands it
/// without the bipartite player/enemy assumption.
#[test]
fn enemy_hitbox_damages_a_relationally_hostile_actor() {
    let mut relations = FactionRelations::default();
    relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
    let (mut app, victim) = arena_hitbox_app(relations, ActorFaction::Boss);
    app.update();
    // Victims only: a body-owned melee also publishes its unresolved half, which
    // names no body and is not a hit on one.
    let captured = app.world().resource::<CapturedHits>();
    let cap = captured.body_hits();
    assert_eq!(cap.len(), 1, "one relational actor-vs-actor hit");
    assert_eq!(
        cap[0].target,
        HitTarget::Body(victim),
        "pre-resolved to the hostile body"
    );
    assert!(matches!(cap[0].source, HitSource::Melee));
    assert_eq!(cap[0].damage, 4);
}

/// Same-faction actors don't fight: an Enemy swing does not hit another Enemy
/// even with the arena relation set (it only adds Enemy ↔ Boss).
#[test]
fn enemy_hitbox_ignores_a_same_faction_actor() {
    let mut relations = FactionRelations::default();
    relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
    let (mut app, _victim) = arena_hitbox_app(relations, ActorFaction::Enemy);
    app.update();
    let captured = app.world().resource::<CapturedHits>();
    assert!(
        captured.body_hits().is_empty(),
        "no friendly fire — an Enemy is not hostile to another Enemy"
    );
    assert_eq!(
        captured.unresolved_feature_hits().len(),
        1,
        "a spared victim does not cancel the strike's reach to non-bodies"
    );
}

/// Damage is PHYSICAL, not relational: an Enemy swing damages a DIFFERENT-faction
/// (Boss) body even with default relations — no targeting hostility required.
/// Targeting (who a brain aims at) is relational; a hit that LANDS deals damage to
/// any non-ally. (Friendly fire is off by default, so a SAME-faction body is spared
/// — see `enemy_hitbox_ignores_a_same_faction_actor`.)
#[test]
fn actor_vs_actor_damage_is_physical_for_different_factions() {
    let (mut app, victim) = arena_hitbox_app(FactionRelations::default(), ActorFaction::Boss);
    app.update();
    // Victims only: a body-owned melee also publishes its unresolved half, which
    // names no body and is not a hit on one.
    let captured = app.world().resource::<CapturedHits>();
    let cap = captured.body_hits();
    assert_eq!(
        cap.len(),
        1,
        "a different-faction body is hit regardless of relations (physical damage)"
    );
    assert_eq!(cap[0].target, HitTarget::Body(victim));
}

/// Spawn an Enemy-source hitbox over a vulnerable player; relations decide
/// whether the player is hit. Returns (app, player).
fn enemy_hitbox_over_player_app(relations: FactionRelations) -> (App, Entity) {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.init_resource::<CapturedHits>();
    app.insert_resource(relations);
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());
    let owner = app
        .world_mut()
        .spawn(ae::CenteredAabb::new(
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(12.0, 16.0),
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            strike_sfx: None,
            owner,
            source: HitSide::Enemy,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::new(100.0, 100.0),
            },
            half_extent: ae::Vec2::new(30.0, 30.0),
            shape: None,
            facing: 1.0,
            damage: 3,
            knockback: crate::strike::HitboxKnockback::LaunchSpeed {
                base: 120.0,
                growth: 0.0,
            },
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));
    let player = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            ActorFaction::Player,
            ambition_platformer2d_core::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
                ..Default::default()
            },
            // The published combat footprint every body carries (§A6).
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(28.0, 46.0),
            ),
            ambition_platformer2d_core::BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            ambition_platformer2d_core::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
        ))
        .id();
    (app, player)
}

/// Default (combat-baseline) relations keep Enemy hostile to Player, so an enemy
/// swing over the player lands — ordinary play is unchanged.
#[test]
fn enemy_hitbox_hits_the_player_by_default() {
    let (mut app, player) = enemy_hitbox_over_player_app(FactionRelations::default());
    app.update();
    // Victims only: a body-owned melee also publishes its unresolved half, which
    // names no body and is not a hit on one.
    let captured = app.world().resource::<CapturedHits>();
    let cap = captured.body_hits();
    assert_eq!(cap.len(), 1, "the player takes the hit by default");
    assert_eq!(cap[0].target, HitTarget::Body(player));
    assert!(matches!(cap[0].source, HitSource::Melee));
    assert_eq!(
        cap[0].knockback.as_ref().map(|k| k.magnitude),
        Some(crate::events::HitKnockbackMagnitude::LaunchSpeed(120.0)),
        "authored melee knockback crosses the hitbox seam as an absolute launch speed"
    );
}

/// Damage is physical, so an Enemy swing that OVERLAPS the player hits them even
/// when the Enemy is NOT hostile to Player (a duel combatant whose stray catches
/// the observer). Sparing the observer is a TARGETING property (the duelist won't
/// AIM at them), NOT a damage one — clearing hostility no longer makes the player
/// damage-immune. The player is only spared by friendly fire (same faction) or by
/// being out of range.
#[test]
fn enemy_hitbox_hits_a_non_targeted_player_strays_are_physical() {
    let mut relations = FactionRelations::default();
    relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Player, false);
    let (mut app, player) = enemy_hitbox_over_player_app(relations);
    app.update();
    // Victims only: a body-owned melee also publishes its unresolved half, which
    // names no body and is not a hit on one.
    let captured = app.world().resource::<CapturedHits>();
    let cap = captured.body_hits();
    assert_eq!(
        cap.len(),
        1,
        "a cross-faction swing over the player lands even with no targeting hostility"
    );
    assert_eq!(cap[0].target, HitTarget::Body(player));
}

/// The AOE fires once, not every tick of its lifetime — the owner doubles
/// as a fired-sentinel in `HitboxHits`.
#[test]
fn player_faction_hitbox_only_fires_once() {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());
    let owner = app
        .world_mut()
        .spawn(ae::CenteredAabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)))
        .id();
    app.world_mut().spawn((
        Hitbox {
            strike_sfx: None,
            owner,
            source: HitSide::Player,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::ZERO,
            },
            half_extent: ae::Vec2::splat(40.0),
            shape: None,
            facing: 1.0,
            damage: 3,
            knockback: crate::strike::HitboxKnockback::FeelScale(0.0),
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 1.0 },
        HitboxHits::default(),
    ));
    app.update();
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<CapturedHits>().0.len(),
        1,
        "the AOE emits its hit once across multiple live ticks"
    );
}

fn armed_player_melee() -> crate::BodyMelee {
    let view = crate::AttackView {
        pos: ae::Vec2::new(100.0, 100.0),
        size: ae::Vec2::new(20.0, 40.0),
        facing: 1.0,
        on_ground: true,
        wall_clinging: false,
        dashing: false,
        abilities_directional_primary: true,
    };
    let spec = crate::attack_spec_from_view(&view, crate::AttackIntent::Forward);
    crate::BodyMelee {
        swing: Some(crate::MeleeSwing::new(spec)),
        ..Default::default()
    }
}

#[test]
fn player_melee_never_targets_its_owner() {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.init_resource::<CapturedHits>();
    app.init_resource::<CapturedLandedHits>();
    app.insert_resource(crate::rules::ResolvedCombatTuning {
        friendly_fire: true,
        ..Default::default()
    });
    app.add_systems(
        Update,
        (apply_hitbox_damage, capture_hits, capture_landed_hits).chain(),
    );

    let owner = app
        .world_mut()
        .spawn((
            ActorFaction::Player,
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(20.0, 40.0),
            ),
            ambition_platformer2d_core::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                size: ae::Vec2::new(20.0, 40.0),
                ..Default::default()
            },
            ambition_platformer2d_core::BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            ambition_platformer2d_core::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
            armed_player_melee(),
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            strike_sfx: None,
            owner,
            source: HitSide::Player,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::ZERO,
            },
            half_extent: ae::Vec2::new(40.0, 40.0),
            shape: None,
            facing: 1.0,
            damage: 4,
            knockback: crate::strike::HitboxKnockback::LaunchSpeed {
                base: 120.0,
                growth: 2.0,
            },
            launch_dir: Some(ae::Vec2::new(0.6, -0.8)),
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));

    app.update();
    let cap = app.world().resource::<CapturedHits>();
    assert!(
        cap.body_hits().is_empty(),
        "a body-owned melee strike must never emit a hit targeting its own owner"
    );
    // The strike still publishes its unresolved half — a swing that overlaps only
    // itself can still smash a crate — but that half is barred from scanning
    // bodies, which is the route by which it used to come back around to its
    // owner. Both facts, or the regression re-enters through the other door.
    cap.assert_no_body_scanning_broadcast();
    assert!(
        app.world().resource::<CapturedLandedHits>().0.is_empty(),
        "self-exclusion must happen before the authoritative landed-hit fact is published"
    );
}

#[test]
fn player_melee_resolves_a_targeted_victim_with_authored_knockback() {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());

    let owner = app
        .world_mut()
        .spawn((
            ActorFaction::Player,
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(20.0, 40.0),
            ),
            armed_player_melee(),
        ))
        .id();
    let victim = app
        .world_mut()
        .spawn((
            ActorFaction::Enemy,
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(130.0, 100.0),
                ae::Vec2::new(20.0, 40.0),
            ),
            ambition_platformer2d_core::BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            ambition_platformer2d_core::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
            ambition_characters::actor::BodyHealth::restored(
                ambition_characters::actor::Health::new(100),
                30,
                Default::default(),
            ),
            crate::CombatTuning {
                weight: 2.0,
                ..Default::default()
            },
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            strike_sfx: None,
            owner,
            source: HitSide::Player,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::new(20.0, 0.0),
            },
            half_extent: ae::Vec2::new(30.0, 30.0),
            shape: None,
            facing: 1.0,
            damage: 4,
            knockback: crate::strike::HitboxKnockback::LaunchSpeed {
                base: 120.0,
                growth: 2.0,
            },
            launch_dir: Some(ae::Vec2::new(0.6, -0.8)),
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));

    app.update();
    let cap = app.world().resource::<CapturedHits>();
    let body_hits = cap.body_hits();
    assert_eq!(body_hits.len(), 1, "the strike resolves exactly one victim");
    cap.assert_no_body_scanning_broadcast();
    let hit = body_hits[0];
    assert!(matches!(hit.source, HitSource::Melee));
    assert_eq!(hit.target, HitTarget::Body(victim));
    assert_eq!(hit.attacker, Some(owner));
    assert_eq!(hit.damage, 4);
    let knockback = hit
        .knockback
        .as_ref()
        .expect("a landed moveset melee strike preserves authored knockback");
    assert_eq!(
        knockback.magnitude,
        HitKnockbackMagnitude::LaunchSpeed(150.0),
        "30 accumulated damage at weight 2.0 applies the authored growth"
    );
    assert_eq!(knockback.launch_dir, Some(ae::Vec2::new(0.6, -0.8)));
}

/// The contact resolver stamps the already-selected player-marked victim explicitly so downstream
/// routing never falls back to a broadcast scan.
#[test]
fn player_melee_targets_a_player_marked_opponent_on_another_match_team() {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.init_resource::<CapturedHits>();
    app.init_resource::<CapturedLandedHits>();
    app.add_systems(
        Update,
        (apply_hitbox_damage, capture_hits, capture_landed_hits).chain(),
    );

    let owner = app
        .world_mut()
        .spawn((
            ActorFaction::Player,
            crate::targeting::MatchTeam::new("seat-0"),
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(20.0, 40.0),
            ),
        ))
        .id();
    let victim = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            ActorFaction::Player,
            crate::targeting::MatchTeam::new("seat-1"),
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(130.0, 100.0),
                ae::Vec2::new(20.0, 40.0),
            ),
            ambition_platformer2d_core::BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            ambition_platformer2d_core::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
        ))
        .id();
    let hitbox = Hitbox {
        strike_sfx: None,
        owner,
        source: HitSide::Player,
        anchor: HitboxAnchor::FollowOwner {
            local_offset: ae::Vec2::new(20.0, 0.0),
        },
        half_extent: ae::Vec2::new(30.0, 30.0),
        shape: None,
        facing: 1.0,
        damage: 4,
        knockback: crate::strike::HitboxKnockback::LaunchSpeed {
            base: 120.0,
            growth: 0.0,
        },
        launch_dir: None,
        frame_down: ae::Vec2::new(0.0, 1.0),
    };
    let owner_body = app.world().get::<ae::CenteredAabb>(owner).unwrap().aabb();
    let victim_body = app.world().get::<ae::CenteredAabb>(victim).unwrap().aabb();
    assert!(
        !owner_body.strict_intersects(victim_body),
        "the regression requires separated fighter bodies"
    );
    assert!(
        hitbox
            .world_volume(app.world().get::<ae::CenteredAabb>(owner).unwrap().center)
            .intersects_aabb(victim_body),
        "the authored strike, not body contact, must be what reaches the victim"
    );
    let hitbox_entity = app
        .world_mut()
        .spawn((
            hitbox,
            HitboxLifetime { remaining_s: 0.2 },
            HitboxHits::default(),
        ))
        .id();

    app.update();
    let cap = app.world().resource::<CapturedHits>();
    let body_hits = cap.body_hits();
    assert_eq!(body_hits.len(), 1, "the other match team is a legal victim");
    assert_eq!(body_hits[0].target, HitTarget::Body(victim));
    assert!(body_hits[0].knockback.is_some());
    cap.assert_no_body_scanning_broadcast();

    let landed = app.world().resource::<CapturedLandedHits>();
    assert_eq!(landed.0.len(), 1, "one selected body contact publishes one landed fact");
    assert_eq!(landed.0[0].hitbox, hitbox_entity);
    assert_eq!(landed.0[0].attacker, owner);
    assert_eq!(landed.0[0].victim, victim);
    assert_eq!(landed.0[0].volume, body_hits[0].volume);
}

/// A live body-owned strike is authoritative gameplay state. The `BodyMelee`
/// projection exists for animation/HUD/telemetry and must not be a hidden
/// prerequisite for damage. This pins the exact failure mode where F1 showed the
/// correct red strike but the volume was inert because a secondary read-model was
/// absent.
#[test]
fn player_followowner_strike_does_not_require_a_body_melee_projection() {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());

    let owner_center = ae::Vec2::new(100.0, 100.0);
    let victim_center = ae::Vec2::new(145.0, 100.0);
    let owner = app
        .world_mut()
        .spawn((
            ActorFaction::Player,
            ae::CenteredAabb::new(owner_center, ae::Vec2::new(10.0, 20.0)),
        ))
        .id();
    let victim = app
        .world_mut()
        .spawn((
            ActorFaction::Enemy,
            ae::CenteredAabb::new(victim_center, ae::Vec2::new(10.0, 20.0)),
            ambition_platformer2d_core::BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            ambition_platformer2d_core::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
        ))
        .id();
    let hitbox = Hitbox {
        strike_sfx: None,
        owner,
        source: HitSide::Player,
        anchor: HitboxAnchor::FollowOwner {
            local_offset: ae::Vec2::new(32.0, 0.0),
        },
        half_extent: ae::Vec2::new(18.0, 18.0),
        shape: None,
        facing: 1.0,
        damage: 4,
        knockback: crate::strike::HitboxKnockback::LaunchSpeed {
            base: 120.0,
            growth: 0.0,
        },
        launch_dir: None,
        frame_down: ae::Vec2::new(0.0, 1.0),
    };
    let owner_body = app.world().get::<ae::CenteredAabb>(owner).unwrap().aabb();
    let victim_body = app.world().get::<ae::CenteredAabb>(victim).unwrap().aabb();
    assert!(
        !owner_body.strict_intersects(victim_body),
        "the bodies must not touch in this regression"
    );
    assert!(
        hitbox.world_volume(owner_center).intersects_aabb(victim_body),
        "the strike polygon/box must reach the separated victim"
    );

    app.world_mut().spawn((
        hitbox,
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));
    app.update();

    let cap = app.world().resource::<CapturedHits>();
    let body_hits = cap.body_hits();
    assert_eq!(body_hits.len(), 1, "the live strike itself is sufficient authority");
    assert_eq!(body_hits[0].target, HitTarget::Body(victim));
    cap.assert_no_body_scanning_broadcast();
}

/// A boss keeps its HP and phase on an encounter and a breakable is a feature; neither matches
/// `StrikeVictim`, so neither can be resolved by identity, and when the strike stopped broadcasting
/// they stopped being hittable.
///
/// The strike publishes its unresolved half as [`HitTarget::UnresolvedFeatures`], and this pins
/// BOTH halves plus the poison: the resolved body hit still names its victim, the unresolved
/// half is still published for the things that have no name, and neither is a body-scanning
/// `Volume` broadcast that would damage the victim twice.
#[test]
fn a_body_owned_strike_publishes_its_unresolved_half_beside_the_resolved_body_hit() {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());

    let owner = app
        .world_mut()
        .spawn((
            ActorFaction::Player,
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(20.0, 40.0),
            ),
            // The move's own authoritative per-strike accumulator.
            {
                let mut playback = crate::moveset::MovePlayback::new(
                    crate::moveset::simple_melee(
                        &crate::moveset::SimpleMeleeParams::default(),
                    ),
                    1.0,
                );
                playback.hit_targets.push("breakable:crate-7".to_string());
                playback
            },
        ))
        .id();
    let victim = app
        .world_mut()
        .spawn((
            ActorFaction::Enemy,
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(130.0, 100.0),
                ae::Vec2::new(20.0, 40.0),
            ),
            ambition_platformer2d_core::BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            ambition_platformer2d_core::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            strike_sfx: None,
            owner,
            source: HitSide::Player,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::new(20.0, 0.0),
            },
            half_extent: ae::Vec2::new(30.0, 30.0),
            shape: None,
            facing: 1.0,
            damage: 4,
            knockback: crate::strike::HitboxKnockback::LaunchSpeed {
                base: 120.0,
                growth: 0.0,
            },
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));

    app.update();
    let cap = app.world().resource::<CapturedHits>();

    let body_hits = cap.body_hits();
    assert_eq!(body_hits.len(), 1, "the body is still resolved by identity");
    assert_eq!(body_hits[0].target, HitTarget::Body(victim));

    let unresolved = cap.unresolved_feature_hits();
    assert_eq!(
        unresolved.len(),
        1,
        "the same strike must still reach a boss or a breakable, which no body \
         resolver can name — this is the reach whose loss broke three app tests"
    );
    assert_eq!(unresolved[0].attacker, Some(owner));
    assert_eq!(unresolved[0].volume, body_hits[0].volume);
    assert_eq!(
        unresolved[0].ignored_targets,
        vec!["breakable:crate-7".to_string()],
        "per-strike dedup rides the move's authoritative accumulator, so a \
         multi-tick active window smashes each crate once"
    );

    cap.assert_no_body_scanning_broadcast();
}

/// CM8: an authored strike sound on a hitbox rides the overlap onto the emitted
/// `HitEvent`, so the ONE victim-side reaction can play the ATTACK's sound (a
/// sword vs a claw) instead of the victim's default. This is the middle link of
/// the authoring chain volume → hitbox → event → reaction.
#[test]
fn the_authored_strike_sound_rides_the_overlap_onto_the_hit_event() {
    let sword = ambition_sfx::SfxId::new("weapon.sword");
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());
    let owner = app
        .world_mut()
        .spawn(ae::CenteredAabb::new(
            ae::Vec2::new(200.0, 0.0),
            ae::Vec2::new(12.0, 16.0),
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            strike_sfx: Some(sword),
            owner,
            source: HitSide::Enemy,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::new(0.0, 0.0),
            },
            half_extent: ae::Vec2::new(30.0, 30.0),
            shape: None,
            facing: 1.0,
            damage: 4,
            knockback: crate::strike::HitboxKnockback::FeelScale(0.0),
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));
    // A player victim overlapping the enemy strike (different faction → lands).
    app.world_mut().spawn((
        ae::CenteredAabb::new(ae::Vec2::new(0.0, 0.0), ae::Vec2::new(14.0, 20.0)),
        ActorFaction::Player,
        ambition_platformer2d_core::BodyOffense::default(),
        ambition_platformer2d_core::BodyMotionFacts::default(),
        ambition_platformer2d_core::BodyShieldState::default(),
        ambition_characters::actor::BodyCombat::default(),
        ambition_platformer2d_shared_tangle::markers::PlayerEntity,
    ));
    app.update();
    let cap = app.world().resource::<CapturedHits>();
    let body_hits = cap.body_hits();
    assert_eq!(body_hits.len(), 1, "the enemy strike lands on the player");
    assert_eq!(
        body_hits[0].strike_sfx,
        Some(sword),
        "the authored strike sound rides onto the HitEvent for the victim reaction"
    );
    assert!(matches!(body_hits[0].target, HitTarget::Body(_)));
}

/// Every basic swing in the game is derived from the `simple_melee` prefab, and a prefab swing
/// carries `knockback_growth: 0.0`.
mod ruleset_knockback_growth {
    use super::*;

    fn launch(growth: f32, ruleset_growth: f32, victim_damage: i32) -> f32 {
        match resolved_hitbox_knockback_magnitude(
            crate::strike::HitboxKnockback::LaunchSpeed {
                base: 120.0,
                growth,
            },
            victim_damage,
            1.0,
            ruleset_growth,
        ) {
            HitKnockbackMagnitude::LaunchSpeed(speed) => speed,
            other => panic!("a launch speed must resolve to one: {other:?}"),
        }
    }

    /// **PARITY FIRST.** An undeclared world is every Ambition room, and
    /// nothing there may start launching further because this seam exists.
    #[test]
    fn a_world_that_declares_no_growth_is_still_flat() {
        assert_eq!(launch(0.0, 0.0, 0), 120.0);
        assert_eq!(
            launch(0.0, 0.0, 200),
            120.0,
            "with no growth declared, a badly damaged body must launch exactly \
             as far as a fresh one — that is Ambition's PvE answer"
        );
    }

    /// The mechanic itself: `0.01` doubles the launch at 100 damage, because the
    /// ruleset term is a fraction of THIS move's base rather than an absolute
    /// px/s — which is what makes one number scale a jab and a smash correctly.
    #[test]
    fn a_declared_growth_makes_a_worn_opponent_fly() {
        assert_eq!(launch(0.0, 0.01, 0), 120.0, "a fresh opponent is unmoved");
        assert_eq!(launch(0.0, 0.01, 100), 240.0);
        assert_eq!(launch(0.0, 0.01, 200), 360.0);
    }

    /// **an authored move still wins.** The ruleset speaks for the swings that
    /// author nothing; a move with its own growth is a deliberate statement and
    /// must not be scaled twice.
    #[test]
    fn an_authored_move_growth_outranks_the_ruleset() {
        // Authored 2.0/point against a ruleset that would have given 1.2.
        assert_eq!(launch(2.0, 0.01, 100), 120.0 + 200.0);
    }

    /// weight still divides, and it must keep doing so through the new path —
    /// a heavy body is the reason growth is per-victim rather than per-hit.
    #[test]
    fn a_heavy_body_still_resists_a_grown_launch() {
        let heavy = match resolved_hitbox_knockback_magnitude(
            crate::strike::HitboxKnockback::LaunchSpeed {
                base: 120.0,
                growth: 0.0,
            },
            100,
            2.0,
            0.01,
        ) {
            HitKnockbackMagnitude::LaunchSpeed(speed) => speed,
            other => panic!("a launch speed must resolve to one: {other:?}"),
        };
        assert_eq!(heavy, 180.0, "twice the weight takes half the growth");
    }
}
