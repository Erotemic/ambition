//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

fn self_view_at(pos: ae::Vec2, faction: ActorFaction) -> SelfView {
    SelfView {
        pos,
        vel: ae::Vec2::ZERO,
        facing: 1.0,
        half_extent: ae::Vec2::new(10.0, 16.0),
        gravity_down: ae::Vec2::new(0.0, 1.0),
        on_ground: true,
        aerial: false,
        alive: true,
        faction,
        can_fire: true,
        can_blink: false,
        can_dash: false,
        can_shield: false,
        ..Default::default()
    }
}

fn perceived(id: &str, pos: ae::Vec2, faction: ActorFaction, hostile: bool) -> PerceivedActor {
    PerceivedActor {
        id: id.to_string(),
        pos,
        vel: ae::Vec2::ZERO,
        facing: 1.0,
        half_extent: ae::Vec2::new(10.0, 16.0),
        faction,
        hostile_to_self: hostile,
        alive: true,
        on_ground: true,
        shield_raised: false,
        ..Default::default()
    }
}

fn wall(center: ae::Vec2, half: ae::Vec2) -> PerceivedSolid {
    PerceivedSolid {
        aabb: ae::Aabb::new(center, half),
        kind: SolidKind::Solid,
    }
}

#[test]
fn viewport_contains_is_axis_aligned() {
    let v = Viewport::around(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(50.0, 30.0));
    assert!(v.contains(ae::Vec2::new(140.0, 120.0)));
    assert!(!v.contains(ae::Vec2::new(160.0, 100.0))); // outside x
    assert!(!v.contains(ae::Vec2::new(100.0, 140.0))); // outside y
}

#[test]
fn nearest_hostile_picks_closest_alive_foe() {
    let view = WorldView {
        self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
        viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(500.0)),
        actors: vec![
            perceived("far", ae::Vec2::new(300.0, 0.0), ActorFaction::Boss, true),
            perceived("near", ae::Vec2::new(80.0, 0.0), ActorFaction::Boss, true),
            perceived("ally", ae::Vec2::new(20.0, 0.0), ActorFaction::Enemy, false),
        ],
        projectiles: vec![],
        terrain: vec![],
        portals: vec![],
        sim_time: 0.0,
        ..Default::default()
    };
    assert_eq!(view.nearest_hostile().map(|a| a.id.as_str()), Some("near"));
    // The closer-but-non-hostile ally is ignored.
    assert_eq!(view.hostiles().count(), 2);
}

#[test]
fn line_of_fire_blocked_by_wall_clear_otherwise() {
    // Self at origin, target straight right at (200, 0). A wall at x=100 blocks.
    let view = WorldView {
        self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
        viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(500.0)),
        actors: vec![],
        projectiles: vec![],
        terrain: vec![wall(ae::Vec2::new(100.0, 0.0), ae::Vec2::new(8.0, 40.0))],
        portals: vec![],
        sim_time: 0.0,
        ..Default::default()
    };
    assert!(
        !view.line_of_fire(ae::Vec2::new(200.0, 0.0)),
        "a wall between self and target blocks the shot"
    );
    // A target above the wall (clear sky) is in line of fire.
    assert!(
        view.line_of_fire(ae::Vec2::new(200.0, -200.0)),
        "a path that misses the wall is clear"
    );
}

#[test]
fn reachable_false_through_solid() {
    let view = WorldView {
        self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
        viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(500.0)),
        actors: vec![],
        projectiles: vec![],
        terrain: vec![wall(ae::Vec2::new(100.0, 0.0), ae::Vec2::new(20.0, 80.0))],
        portals: vec![],
        sim_time: 0.0,
        ..Default::default()
    };
    assert!(!view.reachable(ae::Vec2::new(200.0, 0.0)));
    assert!(view.reachable(ae::Vec2::new(0.0, -200.0)));
}

#[test]
fn incoming_threats_only_hostile_and_closing() {
    let me = ae::Vec2::ZERO;
    let view = WorldView {
        self_view: self_view_at(me, ActorFaction::Enemy),
        viewport: Viewport::around(me, ae::Vec2::splat(500.0)),
        actors: vec![],
        projectiles: vec![
            // hostile, closing (to the left, toward me from the right)
            PerceivedProjectile {
                pos: ae::Vec2::new(100.0, 0.0),
                vel: ae::Vec2::new(-200.0, 0.0),
                damage: 1,
                hostile_to_self: true,
            },
            // hostile, receding
            PerceivedProjectile {
                pos: ae::Vec2::new(100.0, 0.0),
                vel: ae::Vec2::new(200.0, 0.0),
                damage: 1,
                hostile_to_self: true,
            },
            // closing but friendly
            PerceivedProjectile {
                pos: ae::Vec2::new(-100.0, 0.0),
                vel: ae::Vec2::new(200.0, 0.0),
                damage: 1,
                hostile_to_self: false,
            },
        ],
        terrain: vec![],
        portals: vec![],
        sim_time: 0.0,
        ..Default::default()
    };
    assert_eq!(view.incoming_threats().count(), 1);
}

#[test]
fn linked_portal_finds_the_paired_exit() {
    let blue_a = PerceivedPortal {
        pos: ae::Vec2::new(50.0, 0.0),
        normal: ae::Vec2::new(-1.0, 0.0),
        half_extent: ae::Vec2::new(4.0, 24.0),
        channel_key: 7,
    };
    let blue_b = PerceivedPortal {
        pos: ae::Vec2::new(300.0, 0.0),
        normal: ae::Vec2::new(1.0, 0.0),
        half_extent: ae::Vec2::new(4.0, 24.0),
        channel_key: 7,
    };
    let orange = PerceivedPortal {
        pos: ae::Vec2::new(150.0, 0.0),
        normal: ae::Vec2::new(0.0, -1.0),
        half_extent: ae::Vec2::new(24.0, 4.0),
        channel_key: 9,
    };
    let view = WorldView {
        self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
        viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(500.0)),
        actors: vec![],
        projectiles: vec![],
        terrain: vec![],
        portals: vec![blue_a, blue_b, orange],
        sim_time: 0.0,
        ..Default::default()
    };
    // Entering blue_a emerges at blue_b (same channel, other aperture).
    assert_eq!(view.linked_portal(&blue_a).map(|p| p.pos), Some(blue_b.pos));
    // The orange aperture has no pair in view → no linked exit.
    assert!(view.linked_portal(&orange).is_none());
}

#[test]
fn memory_retains_target_after_it_leaves_view() {
    // Tick 1: a hostile is in view → memorized at full confidence.
    let mut mem = WorldMemory::default();
    let in_view = WorldView {
        self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
        viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(300.0)),
        actors: vec![perceived(
            "boss",
            ae::Vec2::new(100.0, 0.0),
            ActorFaction::Boss,
            true,
        )],
        projectiles: vec![],
        terrain: vec![],
        portals: vec![],
        sim_time: 0.0,
        ..Default::default()
    };
    mem.update(&in_view, 1.0 / 60.0);
    assert_eq!(mem.get("boss").map(|m| m.confidence), Some(1.0));

    // Now it leaves view: empty actor list, several ticks pass. The target is
    // still remembered (decaying), so a brain can pursue its last-known spot.
    let mut empty = in_view.clone();
    empty.actors.clear();
    for i in 0..30 {
        empty.sim_time = (i as f32 + 1.0) / 60.0;
        mem.update(&empty, 1.0 / 60.0);
    }
    let remembered = mem
        .last_known_hostile()
        .expect("the hostile that left view is still pursued");
    assert!(
        remembered.confidence < 1.0 && remembered.confidence > WorldMemory::FORGET_BELOW,
        "confidence decays but the target is not yet forgotten: {}",
        remembered.confidence
    );
    assert_eq!(remembered.faction, ActorFaction::Boss);
}

#[test]
fn memory_forgets_after_long_absence() {
    let mut mem = WorldMemory::default();
    let mut view = WorldView {
        self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
        viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(300.0)),
        actors: vec![perceived(
            "ghost",
            ae::Vec2::new(50.0, 0.0),
            ActorFaction::Boss,
            true,
        )],
        projectiles: vec![],
        terrain: vec![],
        portals: vec![],
        sim_time: 0.0,
        ..Default::default()
    };
    mem.update(&view, 1.0 / 60.0);
    view.actors.clear();
    // ~20s of absence at the 3s half-life takes confidence well below the floor.
    for _ in 0..40 {
        mem.update(&view, 0.5);
    }
    assert!(
        mem.is_empty(),
        "a target unseen for many half-lives is forgotten"
    );
}

// ── FB1: the perception delay buffer (the no-cheat contract's §1.3) ──

fn view_at_time(t: f32) -> WorldView {
    WorldView {
        sim_time: t,
        ..Default::default()
    }
}

#[test]
fn a_zero_delay_buffer_shows_the_live_view() {
    let mut d = DelayedPerception::new(0);
    d.observe(view_at_time(1.0));
    assert_eq!(d.perceive().map(|v| v.sim_time), Some(1.0));
    d.observe(view_at_time(2.0));
    assert_eq!(d.perceive().map(|v| v.sim_time), Some(2.0));
    assert!(d.warm());
}

/// The whole point: at level 9 the brain sees the world 9 ticks ago.
#[test]
fn a_warm_buffer_shows_the_world_exactly_delay_ticks_late() {
    let mut d = DelayedPerception::new(3);
    for t in 0..10 {
        d.observe(view_at_time(t as f32));
    }
    assert!(d.warm());
    assert_eq!(
        d.perceive().map(|v| v.sim_time),
        Some(6.0),
        "tick 9 observed; the brain must be looking at tick 9-3"
    );
    assert_eq!(d.buffered(), 4);
}

/// **The failure direction that matters.** While filling, the buffer returns
/// the STALEST view it holds, never a fresher one. A brain spawned mid-fight
/// reacts more slowly than its profile for a few ticks — it never gets a
/// same-tick perceive→act cheat at the exact moment a fight begins, which is
/// the moment FB4's humanity check is watching.
#[test]
fn warming_up_is_stale_never_fresh() {
    let mut d = DelayedPerception::new(5);
    assert!(d.perceive().is_none(), "blind before the first observe");
    for t in 0..5 {
        d.observe(view_at_time(t as f32));
        assert!(!d.warm());
        let seen = d.perceive().expect("something buffered").sim_time;
        let age = t as f32 - seen;
        assert!(
            age <= d.delay_ticks() as f32,
            "never fresher than the profile"
        );
        assert_eq!(seen, 0.0, "the oldest held view, not the newest");
    }
}

#[test]
fn reaction_ms_converts_at_the_sim_rate() {
    // The doc's ladder endpoints, at 60 Hz.
    assert_eq!(
        DelayedPerception::from_reaction_ms(150.0, 60.0).delay_ticks(),
        9
    );
    assert_eq!(
        DelayedPerception::from_reaction_ms(500.0, 60.0).delay_ticks(),
        30
    );
    // A frame-perfect brain is a legal profile (RL rigs, regression fixtures).
    assert_eq!(
        DelayedPerception::from_reaction_ms(0.0, 60.0).delay_ticks(),
        0
    );
}

/// A respawn or room change invalidates every buffered view. The brain goes
/// blind for a tick rather than acting on a picture of the old room.
#[test]
fn clearing_blinds_the_brain_rather_than_stranding_a_stale_room() {
    let mut d = DelayedPerception::new(2);
    for t in 0..5 {
        d.observe(view_at_time(t as f32));
    }
    d.clear();
    assert!(d.perceive().is_none());
    d.observe(view_at_time(100.0));
    assert_eq!(d.perceive().map(|v| v.sim_time), Some(100.0));
}

// ── FB1: stage geometry, the L1 classifier's missing input ──

fn stage_400() -> StageView {
    StageView {
        bounds: ae::Aabb::new(ae::Vec2::new(200.0, 200.0), ae::Vec2::new(200.0, 200.0)),
    }
}

#[test]
fn offstage_is_outside_the_rooms_envelope() {
    let s = stage_400();
    assert!(!s.offstage(ae::Vec2::new(200.0, 200.0)));
    assert!(
        !s.offstage(ae::Vec2::new(0.0, 0.0)),
        "the corner is on-stage"
    );
    assert!(s.offstage(ae::Vec2::new(-1.0, 200.0)), "past the left edge");
    assert!(
        s.offstage(ae::Vec2::new(200.0, 401.0)),
        "under the blastzone"
    );
}

#[test]
fn distance_to_edge_is_zero_offstage_and_measures_corner_pressure_on_it() {
    let s = stage_400();
    assert_eq!(s.distance_to_edge(ae::Vec2::new(-50.0, 200.0)), 0.0);
    assert_eq!(s.distance_to_edge(ae::Vec2::new(200.0, 200.0)), 200.0);
    assert_eq!(
        s.distance_to_edge(ae::Vec2::new(10.0, 200.0)),
        10.0,
        "cornered: 10px from the left wall"
    );
}

/// The two L1 predicates the stage exists to serve: `Recovery` (self offstage)
/// and `EdgeGuard` (the opponent is).
#[test]
fn the_view_answers_recovery_and_edgeguard() {
    let mut view = WorldView {
        self_view: self_view_at(ae::Vec2::new(-40.0, 200.0), ActorFaction::Enemy),
        stage: stage_400(),
        actors: vec![perceived(
            "foe",
            ae::Vec2::new(200.0, 200.0),
            ActorFaction::Player,
            true,
        )],
        ..Default::default()
    };
    assert!(view.self_offstage(), "self is past the left blastzone");
    assert!(!view.actor_offstage(&view.actors[0]));

    view.self_view.pos = ae::Vec2::new(200.0, 200.0);
    view.actors[0].pos = ae::Vec2::new(200.0, 500.0);
    assert!(!view.self_offstage());
    assert!(view.actor_offstage(&view.actors[0]), "they are recovering");
}

/// A default view has an EMPTY stage, so every point is offstage. The first
/// draft used a zero-size box at the origin, which made the origin — and only
/// the origin — read as on-stage. That is exactly the kind of quiet lie the
/// view must not tell.
#[test]
fn a_stageless_view_reads_as_entirely_offstage() {
    assert!(WorldView::default().self_offstage());
    let s = StageView::default();
    assert!(s.offstage(ae::Vec2::ZERO));
    assert!(s.offstage(ae::Vec2::new(1e6, -1e6)));
    assert_eq!(s.distance_to_edge(ae::Vec2::ZERO), 0.0);
}

// ── FB1: the damage meter (CM1's smash-percent axis) ──

#[test]
fn damage_frac_normalizes_and_survives_an_unknown_max() {
    let mut a = perceived("foe", ae::Vec2::ZERO, ActorFaction::Player, true);
    a.health_max = 200;
    a.damage_taken = 50;
    assert_eq!(a.damage_frac(), 0.25);
    a.health_max = 0;
    assert_eq!(a.damage_frac(), 0.0, "unknown max reads as undamaged");
}

#[test]
fn phase_classification_names_the_punish_windows() {
    assert!(BodyPhase::AttackStartup.is_punishable());
    assert!(BodyPhase::AttackRecovery.is_punishable());
    assert!(BodyPhase::Hitstun.is_punishable());
    assert!(
        !BodyPhase::AttackActive.is_punishable(),
        "the hitbox is out — walking in is not a punish"
    );
    assert!(!BodyPhase::Neutral.is_punishable());
    assert!(BodyPhase::AttackActive.is_attacking());
    assert!(!BodyPhase::Shielding.is_attacking());
}

/// **The tie has to break somewhere, and it has to break the same way twice.**
///
/// Two hostiles in view are both at confidence `1.0`, so `last_known_hostile`'s
/// `max_by` is deciding by iteration order. Under a `std::collections::HashMap`
/// that order is `RandomState`, seeded per process: the enemy chased a different
/// player on every run of the same binary on the same inputs (ADR 0023).
///
/// A `BTreeMap` walks ids in order, and `max_by` keeps the LAST maximum, so the
/// greatest id wins. That is not a tiebreak anyone would choose. It is a tiebreak
/// that exists, which is the whole requirement.
#[test]
fn a_tie_between_two_remembered_hostiles_breaks_the_same_way_every_time() {
    let foe = |id: &str, x: f32| PerceivedActor {
        id: id.to_string(),
        pos: ae::Vec2::new(x, 0.0),
        faction: ActorFaction::Player,
        hostile_to_self: true,
        alive: true,
        ..Default::default()
    };
    // Insertion order is deliberately the reverse of id order, so a map that
    // remembered insertion would disagree with one that sorts.
    let view = WorldView {
        self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
        actors: vec![foe("zeta", 1.0), foe("alpha", 2.0), foe("mu", 3.0)],
        ..Default::default()
    };

    let mut first = WorldMemory::default();
    first.update(&view, 0.016);
    let winner = first.last_known_hostile().expect("someone is remembered");
    assert_eq!(
        winner.pos.x, 1.0,
        "`zeta` — the greatest id, at equal confidence"
    );

    // Same inputs, a fresh map, many times: one answer.
    for _ in 0..64 {
        let mut again = WorldMemory::default();
        again.update(&view, 0.016);
        assert_eq!(
            again.last_known_hostile().map(|m| m.pos.x),
            Some(winner.pos.x),
            "the tiebreak moved between two runs of the same binary"
        );
    }

    // And confidence still outranks the id: a decayed `zeta` loses to a fresh one.
    let seen_only_mu = WorldView {
        self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
        actors: vec![foe("mu", 3.0)],
        ..Default::default()
    };
    first.update(&seen_only_mu, 1.0);
    assert_eq!(
        first.last_known_hostile().map(|m| m.pos.x),
        Some(3.0),
        "the in-view foe beats the fading one, whatever their ids"
    );
}
