//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::perception::Perceived;

/// The tests mint `Perceived` directly. That is the ONE legal way in without a
/// delay buffer, its name says what it is, and FB4's profiles never use it.
fn seen(v: &WorldView) -> Perceived<'_> {
    Perceived::cheating(v)
}
use crate::actor::ActorFaction;
use crate::perception::{SelfView, StageView};

/// A 800×600 stage with its origin at 0 — the same envelope CC3's invariant 3
/// polices, which is what `StageView` means by "offstage".
fn stage() -> StageView {
    StageView {
        bounds: ae::Aabb::new(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(400.0, 300.0)),
    }
}

fn me_at(x: f32, y: f32) -> SelfView {
    SelfView {
        pos: ae::Vec2::new(x, y),
        gravity_down: ae::Vec2::new(0.0, 1.0),
        faction: ActorFaction::Player,
        alive: true,
        on_ground: true,
        ..Default::default()
    }
}

fn foe_at(x: f32, y: f32) -> PerceivedActor {
    PerceivedActor {
        id: "foe".to_string(),
        pos: ae::Vec2::new(x, y),
        faction: ActorFaction::Enemy,
        hostile_to_self: true,
        alive: true,
        on_ground: true,
        ..Default::default()
    }
}

fn view(me: SelfView, foes: Vec<PerceivedActor>) -> WorldView {
    WorldView {
        self_view: me,
        stage: stage(),
        actors: foes,
        ..Default::default()
    }
}

/// Two bodies in the middle of a stage, doing nothing. This is where a fight
/// actually lives, and a classifier that never returns it is broken.
#[test]
fn two_idle_bodies_mid_stage_are_in_neutral() {
    assert_eq!(
        classify(seen(&view(me_at(300.0, 300.0), vec![foe_at(500.0, 300.0)]))),
        Situation::Neutral
    );
}

/// **Precedence 1.** Self offstage is `Recovery`, whatever else is true — even
/// if the opponent is offstage too, even if you are in hitstun. A stock lost to
/// the blastzone is not repaid by a punish.
#[test]
fn self_offstage_is_recovery_and_outranks_everything() {
    let mut me = me_at(-50.0, 300.0);
    me.phase = BodyPhase::Hitstun;
    let mut foe = foe_at(-80.0, 300.0); // also offstage
    foe.phase = BodyPhase::Hitstun;
    assert_eq!(classify(seen(&view(me, vec![foe]))), Situation::Recovery);
}

/// **Precedence 2, the one worth arguing about.** A player who chases an
/// offstage opponent while himself in hitstun is not edge-guarding; he is being
/// carried.
#[test]
fn hitstun_outranks_an_offstage_opponent() {
    let mut me = me_at(400.0, 300.0);
    me.phase = BodyPhase::Hitstun;
    let foe = foe_at(900.0, 300.0); // offstage
    assert_eq!(
        classify(seen(&view(me, vec![foe]))),
        Situation::Disadvantage
    );
}

/// Cornered is a `Disadvantage` even at full health and full composure: you
/// have lost your retreat option, which is the whole of what "cornered" means.
#[test]
fn a_body_with_no_stage_behind_it_is_at_a_disadvantage() {
    let me = me_at(CORNER_MARGIN_PX - 1.0, 300.0);
    assert_eq!(
        classify(seen(&view(me, vec![foe_at(400.0, 300.0)]))),
        Situation::Disadvantage
    );
    // One pixel further in and it is a fight again.
    let me = me_at(CORNER_MARGIN_PX + 1.0, 300.0);
    assert_eq!(
        classify(seen(&view(me, vec![foe_at(400.0, 300.0)]))),
        Situation::Neutral
    );
}

#[test]
fn an_offstage_opponent_is_an_edgeguard() {
    assert_eq!(
        classify(seen(&view(me_at(400.0, 300.0), vec![foe_at(-20.0, 300.0)]))),
        Situation::EdgeGuard
    );
}

/// The three punish windows, and the one that is NOT a punish window.
#[test]
fn advantage_is_the_opponents_commitment_and_never_its_active_frames() {
    for phase in [
        BodyPhase::Hitstun,
        BodyPhase::AttackStartup,
        BodyPhase::AttackRecovery,
    ] {
        let mut foe = foe_at(500.0, 300.0);
        foe.phase = phase;
        assert_eq!(
            classify(seen(&view(me_at(300.0, 300.0), vec![foe]))),
            Situation::Advantage,
            "{phase:?} is a punish window"
        );
    }
    let mut foe = foe_at(500.0, 300.0);
    foe.phase = BodyPhase::AttackActive;
    assert_eq!(
        classify(seen(&view(me_at(300.0, 300.0), vec![foe]))),
        Situation::Neutral,
        "the hitbox is out; walking into it is not a punish"
    );
}

/// A committed landing is a punish window that no `BodyPhase` names — it is a
/// kinematic fact, and it is the most reliable one in a platform fighter.
#[test]
fn a_committed_landing_is_an_advantage() {
    let mut foe = foe_at(500.0, 200.0);
    foe.on_ground = false;
    foe.vel = ae::Vec2::new(0.0, LANDING_SPEED_PX_S + 10.0); // +y is down
    assert_eq!(
        classify(seen(&view(me_at(300.0, 300.0), vec![foe.clone()]))),
        Situation::Advantage
    );

    // Rising, or drifting: not committed to anything.
    foe.vel = ae::Vec2::new(0.0, -200.0);
    assert_eq!(
        classify(seen(&view(me_at(300.0, 300.0), vec![foe.clone()]))),
        Situation::Neutral
    );
    foe.vel = ae::Vec2::new(90.0, LANDING_SPEED_PX_S - 10.0);
    assert_eq!(
        classify(seen(&view(me_at(300.0, 300.0), vec![foe]))),
        Situation::Neutral
    );
}

/// **Gravity-relative.** A fight under rotated gravity is the same fight. The
/// landing test reads `gravity_down`, not screen `+y`, so a body falling
/// sideways under sideways gravity is still landing.
#[test]
fn landing_is_measured_along_gravity_not_along_screen_y() {
    let mut me = me_at(300.0, 300.0);
    me.gravity_down = ae::Vec2::new(1.0, 0.0); // gravity points right
    let mut foe = foe_at(500.0, 300.0);
    foe.on_ground = false;
    foe.vel = ae::Vec2::new(LANDING_SPEED_PX_S + 10.0, 0.0); // falling "down" = +x
    assert_eq!(classify(seen(&view(me, vec![foe]))), Situation::Advantage);
}

/// A body with no hostile in view is in `Neutral`, however cornered: being near
/// an edge only means something relative to someone. But reeling is reeling —
/// a hazard, a boss volume, or a stray shot still puts you at a disadvantage.
#[test]
fn with_no_opponent_cornered_is_neutral_but_hitstun_is_not() {
    let me = me_at(10.0, 300.0);
    assert_eq!(classify(seen(&view(me, vec![]))), Situation::Neutral);

    let mut me = me_at(400.0, 300.0);
    me.phase = BodyPhase::Hitstun;
    assert_eq!(classify(seen(&view(me, vec![]))), Situation::Disadvantage);
}

/// A dead opponent is nobody's advantage. `nearest_hostile` already filters
/// them, and this pins that it keeps doing so.
#[test]
fn a_dead_opponent_offers_no_window() {
    let mut foe = foe_at(500.0, 300.0);
    foe.alive = false;
    foe.phase = BodyPhase::Hitstun;
    assert_eq!(
        classify(seen(&view(me_at(300.0, 300.0), vec![foe]))),
        Situation::Neutral
    );
}

/// The precedence IS the enum's order, so `max` over the facts that hold is the
/// classification. If a future variant is inserted in the middle, this fails —
/// which is the point.
#[test]
fn the_variant_order_is_the_precedence() {
    assert!(Situation::Recovery > Situation::Disadvantage);
    assert!(Situation::Disadvantage > Situation::EdgeGuard);
    assert!(Situation::EdgeGuard > Situation::Advantage);
    assert!(Situation::Advantage > Situation::Neutral);
}

/// **Airborne past the lip of a platform is RECOVERY, even inside the room.**
///
/// `StageView` is the ROOM. On a platform stage the playable floor is a subset
/// of it, so a fighter that walked off a 420px platform in a 640px room was
/// *inside the stage* for another hundred pixels of falling: L1 answered
/// `Neutral`, L2 offered `Approach`/`Retreat`/`Jump`, and `Recover` — the one
/// verb that means "get back" — was not on the list until the body had left the
/// ROOM. Traced at level 9 on 2026-07-31: airborne at x=582 and x=627 over a
/// platform ending at 530, drifting right at 532 px/s, offered `[Retreat, Jump]`
/// and taking `Retreat` both times.
#[test]
fn airborne_with_nothing_underneath_is_recovering_even_inside_the_room() {
    let platform = crate::perception::PerceivedSolid {
        // x 110..530, top face at y=300.
        aabb: ae::Aabb::new(ae::Vec2::new(320.0, 316.0), ae::Vec2::new(210.0, 16.0)),
        kind: crate::perception::SolidKind::Solid,
    };
    let airborne_at = |x: f32| {
        let mut me = me_at(x, 240.0);
        me.on_ground = false;
        me.half_extent = ae::Vec2::new(10.0, 16.0);
        WorldView {
            self_view: me,
            stage: stage(),
            actors: vec![foe_at(300.0, 284.0)],
            terrain: vec![platform],
            ..Default::default()
        }
    };

    // Over the platform, mid-jump: there is something to land on, so this is an
    // ordinary airborne moment and NOT a recovery.
    let over = airborne_at(400.0);
    assert_ne!(classify(seen(&over)), Situation::Recovery);

    // Past the lip, still 100px inside the room: nothing underneath.
    let past = airborne_at(600.0);
    assert_eq!(
        classify(seen(&past)),
        Situation::Recovery,
        "a body with the room around it and nothing under it is recovering, \
         whatever the room says"
    );

    // ⚠ and a view that publishes NO terrain is not a body over an abyss — it is
    // a composition that does not build terrain, and reading that as recovery
    // would put every brain in such a composition into permanent recovery.
    let mut terrainless = airborne_at(600.0);
    terrainless.terrain.clear();
    assert_ne!(classify(seen(&terrainless)), Situation::Recovery);
}
