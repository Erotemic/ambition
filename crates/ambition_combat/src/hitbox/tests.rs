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
                growth: Some(2.0),
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
        reaction: None,
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
        reaction: None,
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
                reaction: None,
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
                reaction: None,
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
            !self.0.iter().any(|e| matches!(e.target, HitTarget::Volume)),
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

/// ⛔⛔ A BODY WAITING OUT ITS DEATH BEAT IS NOT A HURTBOX EITHER — AND IT IS
/// SPECIFICALLY NOT A CORPSE.
///
/// `OutOfPlay` promises the world's hands are off a body, and D201 made it
/// body-generic rather than player-only. The tangibility gate above could not
/// see it: `spend_fighter_stocks` calls `health.reset()` the instant the stock
/// is spent — a fighter comes back FRESH — so for the whole interlude the body
/// reads ALIVE while being explicitly out of the fight. It could be struck, it
/// could shield, the attacker got a connect, and the percent it accumulated came
/// back with it on its next stock.
///
/// ⭐ THE ARMS ARE THE SAME BODY, FULL HP, ONE COMPONENT APART. The live control
/// is what makes this the `OutOfPlay` gate rather than the geometry or the
/// health — both bodies are at HP 3 and both are in the swing.
#[test]
fn a_body_waiting_out_its_death_beat_is_intangible_though_it_is_at_full_health() {
    use ambition_characters::actor::{BodyHealth, Health};

    fn arena(out_of_play: bool) -> App {
        let mut relations = FactionRelations::default();
        relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
        let (mut app, victim) = arena_hitbox_app(relations, ActorFaction::Boss);
        app.world_mut()
            .entity_mut(victim)
            .insert(BodyHealth::new(Health {
                current: 3,
                max: 3,
                invulnerable: Default::default(),
            }));
        if out_of_play {
            app.world_mut()
                .entity_mut(victim)
                .insert(crate::death_rules::OutOfPlay);
        }
        app.update();
        app
    }

    assert_eq!(
        arena(false)
            .world()
            .resource::<CapturedHits>()
            .body_hits()
            .len(),
        1,
        "a living body in the swing is struck, so the silence below is the \
         `OutOfPlay` gate and not the geometry"
    );
    assert!(
        arena(true)
            .world()
            .resource::<CapturedHits>()
            .body_hits()
            .is_empty(),
        "a body waiting out its death beat was struck. It is at FULL HEALTH — \
         the stock spend resets the meter so the fighter returns fresh — so the \
         corpse gate could never have caught it, and the hit it took came back \
         with it as percent on the next stock"
    );
}

/// The unification keystone: a Player-faction hitbox (a wielded boss
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
            reaction: None,
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
            reaction: None,
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
    enemy_hitbox_over_player_app_dealing(relations, 3)
}

/// The same fixture with the volume's AUTHORED damage under the caller's control,
/// so a WINDBOX (`damage: 0`) can be built from the production path rather than
/// from a hand-shaped `HitEvent`.
fn enemy_hitbox_over_player_app_dealing(
    relations: FactionRelations,
    authored_damage: i32,
) -> (App, Entity) {
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
            damage: authored_damage,
            knockback: crate::strike::HitboxKnockback::LaunchSpeed {
                base: 120.0,
                growth: None,
            },
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
            reaction: None,
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
            reaction: None,
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
                growth: Some(2.0),
            },
            launch_dir: Some(ae::Vec2::new(0.6, -0.8)),
            frame_down: ae::Vec2::new(0.0, 1.0),
            reaction: None,
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
                growth: Some(2.0),
            },
            launch_dir: Some(ae::Vec2::new(0.6, -0.8)),
            frame_down: ae::Vec2::new(0.0, 1.0),
            reaction: None,
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
            growth: None,
        },
        launch_dir: None,
        frame_down: ae::Vec2::new(0.0, 1.0),
        reaction: None,
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
    assert_eq!(
        landed.0.len(),
        1,
        "one selected body contact publishes one landed fact"
    );
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
            growth: None,
        },
        launch_dir: None,
        frame_down: ae::Vec2::new(0.0, 1.0),
        reaction: None,
    };
    let owner_body = app.world().get::<ae::CenteredAabb>(owner).unwrap().aabb();
    let victim_body = app.world().get::<ae::CenteredAabb>(victim).unwrap().aabb();
    assert!(
        !owner_body.strict_intersects(victim_body),
        "the bodies must not touch in this regression"
    );
    assert!(
        hitbox
            .world_volume(owner_center)
            .intersects_aabb(victim_body),
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
    assert_eq!(
        body_hits.len(),
        1,
        "the live strike itself is sufficient authority"
    );
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
                    crate::moveset::simple_melee(&crate::moveset::SimpleMeleeParams::default()),
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
                growth: None,
            },
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
            reaction: None,
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
            reaction: None,
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
/// authors NO growth, so the ruleset speaks for it.
mod ruleset_knockback_growth {
    use super::*;

    fn launch(growth: Option<f32>, ruleset_growth: f32, victim_damage: i32) -> f32 {
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

    /// PARITY FIRST. An undeclared world is every Ambition room, and
    /// nothing there may start launching further because this seam exists.
    #[test]
    fn a_world_that_declares_no_growth_is_still_flat() {
        assert_eq!(launch(None, 0.0, 0), 120.0);
        assert_eq!(
            launch(None, 0.0, 200),
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
        assert_eq!(launch(None, 0.01, 0), 120.0, "a fresh opponent is unmoved");
        assert_eq!(launch(None, 0.01, 100), 240.0);
        assert_eq!(launch(None, 0.01, 200), 360.0);
    }

    /// an authored move still wins. The ruleset speaks for the swings that
    /// author nothing; a move with its own growth is a deliberate statement and
    /// must not be scaled twice.
    #[test]
    fn an_authored_move_growth_outranks_the_ruleset() {
        // Authored 2.0/point against a ruleset that would have given 1.2.
        assert_eq!(launch(Some(2.0), 0.01, 100), 120.0 + 200.0);
    }

    /// weight still divides, and it must keep doing so through the new path —
    /// a heavy body is the reason growth is per-victim rather than per-hit.
    #[test]
    fn a_heavy_body_still_resists_a_grown_launch() {
        let heavy = match resolved_hitbox_knockback_magnitude(
            crate::strike::HitboxKnockback::LaunchSpeed {
                base: 120.0,
                growth: None,
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

    /// FIXED knockback: a stated zero is a statement, not a silence.
    ///
    /// ⭐ the whole point of widening the field. `Some(0.0)` and `None` were one
    /// value, and the resolver read that value as "unspecified" — so the flat
    /// launch the doc comment promised was the one behaviour a volume could not
    /// author under a stage that declares growth. Every fixed-knockback move in
    /// the genre (a set-knockback multi-hit, a pull-in, a meteor with a constant
    /// spike) needs this.
    #[test]
    fn an_authored_zero_is_fixed_knockback_under_a_growing_ruleset() {
        // The same stage that carries a fresh body 120 and a worn one 240.
        assert_eq!(launch(None, 0.01, 0), 120.0);
        assert_eq!(launch(None, 0.01, 200), 360.0);
        // The move that says zero launches the same at both.
        assert_eq!(launch(Some(0.0), 0.01, 0), 120.0);
        assert_eq!(
            launch(Some(0.0), 0.01, 200),
            120.0,
            "a stated zero must hold the launch flat at any damage — this is \
             what `None` cannot express"
        );
    }
}

/// THE WINDBOX — a volume that moves a body without hurting it.
///
/// ⭐ a category the vocabulary already had a field for and the runtime could
/// not express. `HitVolume::damage` is an `i32` an author may write `0` into,
/// but the strike seam floored every published damage at one, so the runtime
/// dealt a point regardless. A gust, a suction pulse, a stage wind, a
/// pull-in — every move whose whole design is "displace, do not hurt" — came
/// out as a one-damage poke.
mod windbox {
    use super::*;

    fn published(authored_damage: i32) -> crate::events::HitEvent {
        let (mut app, player) =
            enemy_hitbox_over_player_app_dealing(FactionRelations::default(), authored_damage);
        app.update();
        let captured = app.world().resource::<CapturedHits>();
        let cap = captured.body_hits();
        assert_eq!(cap.len(), 1, "the volume overlaps the body, so it connects");
        assert_eq!(cap[0].target, HitTarget::Body(player));
        cap[0].clone()
    }

    /// The AUTHORED zero survives to the victim, and it still LAUNCHES.
    ///
    /// Both halves matter: damage 0 is the point of the category, and a windbox
    /// that published no knockback would be a no-op rather than a push.
    #[test]
    fn a_damageless_volume_pushes_without_hurting() {
        let hit = published(0);
        assert_eq!(hit.damage, 0, "a windbox authors no damage and deals none");
        assert_eq!(
            hit.knockback.as_ref().map(|k| k.magnitude),
            Some(crate::events::HitKnockbackMagnitude::LaunchSpeed(120.0)),
            "the push is the whole move — a windbox that did not launch would be nothing"
        );
    }

    /// THE OTHER SIDE OF THE FLOOR, and the reason it cannot simply be deleted.
    ///
    /// A volume that DOES author damage must never be worn to zero by staling
    /// or difficulty scaling. One authored point stays one point.
    #[test]
    fn a_volume_that_authors_damage_still_keeps_a_point() {
        assert_eq!(published(1).damage, 1);
        assert_eq!(
            damage_floor(1),
            1,
            "an authored hit floors at one however far it is scaled down"
        );
        assert_eq!(damage_floor(0), 0, "a windbox floors at nothing");
    }

    /// How many times this volume connects with the body over `ticks`.
    fn connections(repeating: Option<bool>, ticks: usize) -> usize {
        let (mut app, _) = enemy_hitbox_over_player_app_dealing(FactionRelations::default(), 0);
        if let Some(repeating) = repeating {
            let world = app.world_mut();
            let mut q = world.query::<&mut Hitbox>();
            for mut hitbox in q.iter_mut(world) {
                hitbox.reaction = Some(ambition_entity_catalog::VolumeReaction::Windbox(
                    ambition_entity_catalog::WindboxVolume { repeating },
                ));
            }
        }
        for _ in 0..ticks {
            app.update();
        }
        app.world().resource::<CapturedHits>().body_hits().len()
    }

    /// ⭐⭐ A GUST PUSHES FOR AS LONG AS YOU STAND IN IT.
    ///
    /// The hit-once set exists so a long active window cannot re-hit a
    /// stationary target every frame — exactly right for a strike, and exactly
    /// wrong for a wind. ⛔ BOTH ARMS ARE ASSERTED: a change that made every
    /// volume repeat would satisfy the windbox half and quietly turn every
    /// lingering sword into a multihit.
    #[test]
    fn only_a_repeating_windbox_connects_more_than_once() {
        assert_eq!(
            connections(None, 4),
            1,
            "an ordinary volume connected more than once — the hit-once set is \
             not holding, and every lingering strike is now a multihit"
        );
        assert_eq!(
            connections(Some(false), 4),
            1,
            "a windbox that did NOT ask to repeat still repeated: a one-shot \
             shove is a windbox too, and it wants the ordinary rule"
        );
        assert!(
            connections(Some(true), 4) > 1,
            "a repeating gust connected only once, so it pushes on the frame you \
             enter it and never again — which is a shove, not a wind"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The perfect shield.
//
// A parry is a NEGATION resolved at the strike seam, and the fact it publishes
// is about a CATCH, not about a window standing open. Both halves are pinned:
// a raised guard that catches nothing must stay cold.
// ─────────────────────────────────────────────────────────────────────────────

/// Attacker + victim + one strike volume between them, with the victim's guard
/// in whatever state the caller wants to test.
fn parry_fixture(shield: ae::BodyShieldState) -> (App, Entity) {
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
            shield,
            ambition_characters::actor::BodyCombat::default(),
            ambition_characters::actor::BodyHealth::restored(
                ambition_characters::actor::Health::new(100),
                0,
                Default::default(),
            ),
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
                growth: Some(2.0),
            },
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
            reaction: None,
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));
    (app, victim)
}

/// The same fixture, but the volume is a WINDBOX.
fn windbox_parry_fixture(shield: ae::BodyShieldState) -> (App, Entity) {
    let (mut app, victim) = parry_fixture(shield);
    let mut boxes = app.world_mut().query::<&mut Hitbox>();
    for mut hitbox in boxes.iter_mut(app.world_mut()) {
        hitbox.damage = 0;
        hitbox.reaction = Some(ambition_entity_catalog::VolumeReaction::Windbox(
            ambition_entity_catalog::WindboxVolume { repeating: false },
        ));
    }
    (app, victim)
}

/// A WINDBOX CANNOT BE PARRIED.
///
/// ⛔⛔ ITS AUTHORED CONTRACT IS "PUSHES, AND NOTHING ELSE" — no damage, no
/// hitstun, NO SHIELD — and the parry producer asked `shield.parrying()` for
/// every strike volume alike. Caught, a gust produced no push at all, so a
/// defender could parry the WIND.
///
/// ⚠ THE OTHER HALF IS STILL OPEN AND THIS TEST DOES NOT CLAIM IT: ordinary
/// guard resolution happens later in `resolve_body_hit`, which receives no fact
/// saying the contact is a windbox — so a gust can still be BLOCKED for shield
/// integrity, shieldstun and pushback.
#[test]
fn a_parry_window_does_not_catch_a_windbox() {
    let open = || ae::BodyShieldState {
        active: true,
        parry_window_timer: 0.05,
        ..Default::default()
    };

    // ⛔ THE CONTROL FIRST: the same window catches an ORDINARY strike, or this
    // fixture cannot tell a windbox from a parry that stopped working.
    let (mut app, victim) = parry_fixture(open());
    app.update();
    assert!(
        caught(&app, victim),
        "the parry window did not catch an ordinary strike, so the arm below \
         proves nothing"
    );

    let (mut app, victim) = windbox_parry_fixture(open());
    app.update();
    assert!(
        !caught(&app, victim),
        "a parry CAUGHT a windbox — a gust says it does not interact with a \
         shield, and a parry is the strongest form of exactly that"
    );
}

fn caught(app: &App, victim: Entity) -> bool {
    app.world()
        .get::<ae::BodyShieldState>(victim)
        .expect("the victim carries a guard")
        .parry_caught()
}

/// A strike into an open parry window is turned away outright: no damage event
/// reaches the victim, the attacker's move never CONNECTED, and the fact the
/// cue reads is armed on the tick it happened.
#[test]
fn a_strike_into_an_open_parry_window_is_caught_and_announced() {
    let guarding = ae::BodyShieldState {
        active: true,
        parry_window_timer: 0.05,
        ..Default::default()
    };
    let (mut app, victim) = parry_fixture(guarding);
    app.update();

    let cap = app.world().resource::<CapturedHits>();
    assert!(
        cap.body_hits().is_empty(),
        "a parried strike still produced a damage event, so the parry is a \
         block rather than a negation"
    );
    assert!(
        app.world().resource::<CapturedLandedHits>().0.is_empty(),
        "a parried strike counted as a CONNECTION, so it would confirm an \
         on-hit cancel and wear itself out on the stale queue"
    );
    assert!(
        caught(&app, victim),
        "the parry caught a strike and published nothing, so a cue has no \
         moment to fire on"
    );
    let shield = app.world().get::<ae::BodyShieldState>(victim).unwrap();
    assert_eq!(
        (shield.depleted, shield.stun_timer),
        (0.0, 0.0),
        "the perfect shield paid integrity and shieldstun, which is what an \
         ordinary block costs — the whole reward for the timing is that it does not"
    );
}

/// ⭐ THE BUG THIS FACT EXISTS FOR. `parrying()` is true for a few ticks after
/// EVERY shield raise, so a cue driven off it fires whenever anybody guards. The
/// published fact must stay cold until something is actually caught.
#[test]
fn an_open_parry_window_that_catches_nothing_stays_cold() {
    let guarding = ae::BodyShieldState {
        active: true,
        parry_window_timer: 0.05,
        ..Default::default()
    };
    let (mut app, victim) = parry_fixture(guarding);
    // Move the victim out of the strike's reach: the window is wide open and
    // nothing arrives.
    app.world_mut()
        .get_mut::<ae::CenteredAabb>(victim)
        .unwrap()
        .center = ae::Vec2::new(600.0, 100.0);
    app.update();
    assert!(
        app.world()
            .get::<ae::BodyShieldState>(victim)
            .unwrap()
            .parrying(),
        "the fixture meant to leave the window OPEN"
    );
    assert!(
        !caught(&app, victim),
        "merely holding the window open announced a parry, which fires the cue \
         on every shield raise"
    );
}

/// A guard that is up but past its window is an ordinary block: the strike
/// resolves normally and the parry fact stays cold.
#[test]
fn a_guard_past_its_window_does_not_catch() {
    let late = ae::BodyShieldState {
        active: true,
        parry_window_timer: 0.0,
        ..Default::default()
    };
    let (mut app, victim) = parry_fixture(late);
    app.update();
    assert_eq!(
        app.world().resource::<CapturedHits>().body_hits().len(),
        1,
        "a late guard swallowed the strike at the parry seam, so blocking and \
         parrying have become the same thing"
    );
    assert!(!caught(&app, victim));
}

/// ⭐ AN AUTHORED AUTOLINK REACHES THE VICTIM'S PAYLOAD, AND CARRIES THE
/// ATTACKER'S OWN MOTION WITH IT.
///
/// The kernel's `autolink_velocity` is unit-tested, and a primitive nothing can
/// feed is worth nothing — this drives the real producer, so it is the test that
/// fails if the authored field stops reaching `HitKnockback`.
///
/// ⛔⛔ THE VELOCITY IS THE HALF THAT CAN ONLY BE SAMPLED HERE. The reaction
/// holds a victim and no attacker entity, so the producer is the only honest
/// place to answer "how fast was the attacker moving at this pulse" — and it is
/// what makes a RISING multi-hit work at all, since the correction term only
/// closes a gap. A wiring that carried the anchor and dropped the velocity would
/// pass every kernel test and drop its victim in play.
#[test]
fn an_authored_autolink_reaches_the_hit_payload_with_the_attackers_velocity() {
    const RISE: ae::Vec2 = ae::Vec2::new(0.0, -640.0);

    let arena = |autolink: Option<ambition_entity_catalog::AutolinkVolume>| {
        let mut relations = FactionRelations::default();
        relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
        let (mut app, _victim) = arena_hitbox_app(relations, ActorFaction::Boss);
        // Give the swing owner a real body that is MOVING, and author the link.
        let owner = app
            .world_mut()
            .query::<(bevy::prelude::Entity, &Hitbox)>()
            .iter(app.world())
            .map(|(_, h)| h.owner)
            .next()
            .expect("the arena spawns one hitbox");
        app.world_mut()
            .entity_mut(owner)
            .insert(ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: RISE,
                ..Default::default()
            });
        let mut boxes = app.world_mut().query::<&mut Hitbox>();
        for mut hitbox in boxes.iter_mut(app.world_mut()) {
            hitbox.reaction = autolink.map(ambition_entity_catalog::VolumeReaction::Autolink);
        }
        app.update();
        let captured = app.world().resource::<CapturedHits>();
        captured
            .body_hits()
            .first()
            .and_then(|e| e.knockback.clone())
            .expect("the swing lands and carries a knockback")
    };

    // Poison: the same swing with nothing authored carries no follow at all, so
    // the assertion below cannot be satisfied by a field that is always set.
    assert!(
        arena(None).follow.is_none(),
        "an ordinary volume grew a follow it never authored"
    );

    let kb = arena(Some(ambition_entity_catalog::AutolinkVolume {
        anchor: (18.0, 4.0),
        carry: 1.0,
        pull: 20.0,
        max_speed: 900.0,
    }));
    let follow = kb
        .follow
        .expect("the authored autolink reached the payload");
    // ⭐ RESOLVED, not carried through. The arena's owner sits at (100,100)
    // facing +1 with gravity down, so an authored `(18, 4)` — 18 forward, 4
    // toward the attacker's feet — lands at (118, 104). Asserting the WORLD
    // point is what pins that the producer did the resolution, which is the
    // whole correction: the victim has neither the attacker's facing nor its
    // frame and must not be reconstructing this.
    assert_eq!(follow.anchor_world, ae::Vec2::new(118.0, 104.0));
    assert_eq!(follow.pull, 20.0);
    assert_eq!(
        follow.source_vel, RISE,
        "the producer did not sample the attacker's velocity, so a rising \
         multi-hit would leave its victim behind"
    );
}
