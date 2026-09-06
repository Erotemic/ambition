use super::*;
use crate::perception::Perceived;

/// The tests mint `Perceived` directly. That is the ONE legal way in without a
/// delay buffer, its name says what it is, and FB4's profiles never use it.
fn seen(v: &WorldView) -> Perceived<'_> {
    Perceived::cheating(v)
}
use crate::actor::ActorFaction;
use crate::perception::{PerceivedSolid, SelfView, SolidKind, StageView};

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

/// Precedence 1. Self offstage is `Recovery`, whatever else is true — even
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

/// Precedence 2, the one worth arguing about. A player who chases an
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

/// Gravity-relative. A fight under rotated gravity is the same fight. The
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

/// Airborne past the lip of a platform is RECOVERY, even inside the room.
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

    //  and a view that publishes NO terrain is not a body over an abyss — it is
    // a composition that does not build terrain, and reading that as recovery
    // would put every brain in such a composition into permanent recovery.
    let mut terrainless = airborne_at(600.0);
    terrainless.terrain.clear();
    assert_ne!(classify(seen(&terrainless)), Situation::Recovery);
}

/// THE CORNER IS A SHARE OF THE FLOOR, so the same body at the same FRACTION of
/// two differently sized platforms gets the same answer.
///
/// The property, not the pixel count. An absolute margin is a claim about one
/// stage's size, and the Smash demo's 480px platform made HALF of itself a
/// corner under the old 120px one — which is why 43% of every decision its
/// fighters made was answered from `Disadvantage`.
#[test]
fn cornering_scales_with_the_floor_a_body_stands_on() {
    let floor = |min_x: f32, max_x: f32| PerceivedSolid {
        aabb: ae::Aabb::new(
            ae::Vec2::new((min_x + max_x) / 2.0, 340.0),
            ae::Vec2::new((max_x - min_x) / 2.0, 16.0),
        ),
        kind: SolidKind::Solid,
    };
    // A body a quarter of the way in from the left edge of its floor, on a
    // narrow platform and on one four times wider.
    let stood_at = |min_x: f32, max_x: f32| {
        let width = max_x - min_x;
        let mut me = me_at(min_x + width * 0.25, 300.0);
        me.half_extent = ae::Vec2::new(10.0, 24.0);
        WorldView {
            self_view: me,
            stage: StageView {
                bounds: ae::Aabb::new(ae::Vec2::new(2000.0, 300.0), ae::Vec2::new(2000.0, 300.0)),
            },
            actors: vec![foe_at(min_x + width * 0.75, 300.0)],
            terrain: vec![floor(min_x, max_x)],
            ..Default::default()
        }
    };

    let narrow = stood_at(1500.0, 1900.0);
    let wide = stood_at(400.0, 2000.0);
    assert_eq!(
        classify(seen(&narrow)),
        classify(seen(&wide)),
        "the same body at the same fraction of two floors classified differently, \
         so the corner is still a pixel count wearing a fraction's name"
    );
}

/// CORNERED IS A DIRECTION, AND THE EDGE-GUARD IS THE PROOF.
///
/// Both terms of the corner test asked for the NEAREST edge, so standing beside
/// a ledge read the same as being backed against it — and the ledge you stand
/// beside to punish a hanging opponent is by construction the nearest edge
/// there is. The fighter walked out to edge-guard, flipped to `Disadvantage`
/// one body-width from the lip, and retreated: the situation was unreachable
/// from the only position it is played from.
///
/// Both ends are asserted. The right ledge with the foe off it is the guard;
/// the LEFT ledge with the same foe to the right is still a corner, and a fix
/// that simply stopped calling anything cornered would fail that half.
#[test]
fn standing_at_your_own_ledge_to_punish_a_hang_is_not_being_cornered() {
    // A floor spanning x 100..700, top face at y=340.
    let floor = PerceivedSolid {
        aabb: ae::Aabb::new(ae::Vec2::new(400.0, 370.0), ae::Vec2::new(300.0, 30.0)),
        kind: SolidKind::Solid,
    };
    let hanging_right = PerceivedActor {
        on_ground: false,
        ledge_hanging: true,
        ..foe_at(706.0, 350.0)
    };
    let at = |x: f32| {
        let mut me = me_at(x, 320.0);
        me.half_extent = ae::Vec2::new(16.0, 20.0);
        WorldView {
            self_view: me,
            stage: stage(),
            actors: vec![hanging_right.clone()],
            terrain: vec![floor],
            ..Default::default()
        }
    };
    // Out at the lip the foe is hanging from: the whole floor is behind me.
    assert_eq!(
        classify(seen(&at(640.0))),
        Situation::EdgeGuard,
        "the edge-guard position must classify as the edge-guard"
    );
    assert_eq!(classify(seen(&at(690.0))), Situation::EdgeGuard);
    // The FAR ledge, with the same foe: retreat runs out, and that is a corner.
    assert_eq!(
        classify(seen(&at(130.0))),
        Situation::Disadvantage,
        "backed against the far ledge is still cornered"
    );
}
