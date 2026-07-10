//! **FB3 — L1, the situation classifier.**
//!
//! `docs/planning/engine/fighter-brain.md` §1:
//!
//! > *"L1 — Situation classifier (cheap, every tick): derives the tactical state
//! > from the view — `Neutral`, `Advantage` (opponent in hitstun/landing),
//! > `Disadvantage` (self in hitstun/shield-broken/cornered), `Recovery`
//! > (offstage/knocked out of arena), `EdgeGuard` (opponent recovering). Pure
//! > function of the view; unit-tested per scenario fixture."*
//!
//! It is a pure function of [`WorldView`] and nothing else. That is the no-cheat
//! contract's first clause, and it is why FB1's audit had to come first: before
//! it, the view carried no move phase, no damage meter, and no stage geometry, so
//! three of these five states were not derivable at all.
//!
//! ## Precedence, and why it is not a scoring function
//!
//! Two facts can hold at once — you can be offstage AND in hitstun, or juggling an
//! opponent while cornered. L1 answers ONE question: *what is this tick about?*
//! So the states are ranked, and the rank is the design:
//!
//! 1. [`Recovery`](Situation::Recovery) — **self is offstage.** Nothing else
//!    matters; a stock lost to the blastzone is not repaid by a punish.
//! 2. [`Disadvantage`](Situation::Disadvantage) — **self is in hitstun, or
//!    cornered.** You are the one who has to solve something.
//! 3. [`EdgeGuard`](Situation::EdgeGuard) — **the opponent is offstage.** The
//!    single highest-value window in the game, and it expires.
//! 4. [`Advantage`](Situation::Advantage) — the opponent is punishable: in
//!    hitstun, in an attack's startup or recovery, or committed to a landing.
//! 5. [`Neutral`](Situation::Neutral) — nobody has anything.
//!
//! Disadvantage outranks EdgeGuard on purpose: a player who chases an offstage
//! opponent while himself in hitstun is not edge-guarding, he is being carried.
//!
//! ## What "cornered" and "landing" mean, concretely
//!
//! Both are thresholds, and both are here rather than in a profile, because they
//! are facts about the STAGE and the KIT, not about difficulty. A level-1 CPU and
//! a level-9 CPU agree about whether they are cornered; they disagree about what
//! to do next, and that is L2's job (§1).

use ambition_engine_core as ae;

use crate::perception::{BodyPhase, PerceivedActor, WorldView};

/// How close to a blastzone counts as cornered, in world px. A body with less
/// than this much stage behind it has lost its retreat option, which is what
/// "cornered" means — not that it is about to die.
pub const CORNER_MARGIN_PX: f32 = 120.0;

/// A body is "landing" when it is airborne, moving toward the ground fast enough
/// that it cannot change its mind, and low enough that it is committed. Landing
/// lag is the most reliable punish window in a platform fighter.
pub const LANDING_SPEED_PX_S: f32 = 60.0;

/// The tactical state of one tick, from one body's point of view.
///
/// Ordered by the precedence above: a larger variant OUTRANKS a smaller one, so
/// `max` over the facts that hold is the classification. That is a property the
/// tests lean on, and the reason the derive is not decorative.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Situation {
    /// Nobody has anything. Where a fight actually lives.
    Neutral,
    /// The opponent is punishable — hitstun, attack startup or recovery, or a
    /// committed landing.
    Advantage,
    /// The opponent is offstage. The highest-value window in the game.
    EdgeGuard,
    /// Self is in hitstun, or cornered against a blastzone.
    Disadvantage,
    /// Self is offstage. Everything else waits.
    Recovery,
}

/// Is this actor committed to a landing? Airborne, descending, and past the point
/// of changing its mind.
///
/// Gravity-relative, because a fight under rotated gravity is the same fight. The
/// view carries `gravity_down` for exactly this.
pub fn is_landing(actor: &PerceivedActor, gravity_down: ae::Vec2) -> bool {
    !actor.on_ground && actor.vel.dot(gravity_down) > LANDING_SPEED_PX_S
}

/// Is this actor punishable RIGHT NOW — the thing L2 will price?
///
/// [`BodyPhase::is_punishable`] covers hitstun, attack startup, and attack
/// recovery. Active frames are deliberately NOT punishable: that is where the
/// hitbox is, and walking into it is not a punish. A committed landing is added
/// here because landing lag is not a `BodyPhase` — it is a kinematic fact.
pub fn is_punishable(actor: &PerceivedActor, gravity_down: ae::Vec2) -> bool {
    actor.alive && (actor.phase.is_punishable() || is_landing(actor, gravity_down))
}

/// **L1.** Classify the tick from this view's own point of view.
///
/// The opponent is the nearest hostile, which is the same body every other layer
/// of the brain targets — L1 does not get a private query, and a body with no
/// hostile in view is in [`Neutral`] however cornered it is, because "cornered"
/// only means something relative to someone.
pub fn classify(view: &WorldView) -> Situation {
    let me = &view.self_view;
    let gravity_down = me.gravity_down;

    // 1. Self offstage. Nothing else matters.
    if view.self_offstage() {
        return Situation::Recovery;
    }

    // A body with nobody to fight is in neutral, however uncomfortable its
    // position. "Cornered" against an empty stage is just standing near an edge.
    let Some(foe) = view.nearest_hostile() else {
        return if me.phase == BodyPhase::Hitstun {
            // ...unless it is being hit by something that is not an actor: a
            // hazard, a boss volume, a stray projectile. Reeling is reeling.
            Situation::Disadvantage
        } else {
            Situation::Neutral
        };
    };

    // 2. Self is the one with a problem.
    let cornered = view.stage.distance_to_edge(me.pos) < CORNER_MARGIN_PX;
    if me.phase == BodyPhase::Hitstun || cornered {
        return Situation::Disadvantage;
    }

    // 3. The opponent is offstage and has to come back through you.
    if view.actor_offstage(foe) {
        return Situation::EdgeGuard;
    }

    // 4. The opponent is committed to something.
    if is_punishable(foe, gravity_down) {
        return Situation::Advantage;
    }

    Situation::Neutral
}

#[cfg(test)]
mod tests {
    use super::*;
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
            classify(&view(me_at(300.0, 300.0), vec![foe_at(500.0, 300.0)])),
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
        assert_eq!(classify(&view(me, vec![foe])), Situation::Recovery);
    }

    /// **Precedence 2, the one worth arguing about.** A player who chases an
    /// offstage opponent while himself in hitstun is not edge-guarding; he is being
    /// carried.
    #[test]
    fn hitstun_outranks_an_offstage_opponent() {
        let mut me = me_at(400.0, 300.0);
        me.phase = BodyPhase::Hitstun;
        let foe = foe_at(900.0, 300.0); // offstage
        assert_eq!(classify(&view(me, vec![foe])), Situation::Disadvantage);
    }

    /// Cornered is a `Disadvantage` even at full health and full composure: you
    /// have lost your retreat option, which is the whole of what "cornered" means.
    #[test]
    fn a_body_with_no_stage_behind_it_is_at_a_disadvantage() {
        let me = me_at(CORNER_MARGIN_PX - 1.0, 300.0);
        assert_eq!(
            classify(&view(me, vec![foe_at(400.0, 300.0)])),
            Situation::Disadvantage
        );
        // One pixel further in and it is a fight again.
        let me = me_at(CORNER_MARGIN_PX + 1.0, 300.0);
        assert_eq!(
            classify(&view(me, vec![foe_at(400.0, 300.0)])),
            Situation::Neutral
        );
    }

    #[test]
    fn an_offstage_opponent_is_an_edgeguard() {
        assert_eq!(
            classify(&view(me_at(400.0, 300.0), vec![foe_at(-20.0, 300.0)])),
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
                classify(&view(me_at(300.0, 300.0), vec![foe])),
                Situation::Advantage,
                "{phase:?} is a punish window"
            );
        }
        let mut foe = foe_at(500.0, 300.0);
        foe.phase = BodyPhase::AttackActive;
        assert_eq!(
            classify(&view(me_at(300.0, 300.0), vec![foe])),
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
            classify(&view(me_at(300.0, 300.0), vec![foe.clone()])),
            Situation::Advantage
        );

        // Rising, or drifting: not committed to anything.
        foe.vel = ae::Vec2::new(0.0, -200.0);
        assert_eq!(
            classify(&view(me_at(300.0, 300.0), vec![foe.clone()])),
            Situation::Neutral
        );
        foe.vel = ae::Vec2::new(90.0, LANDING_SPEED_PX_S - 10.0);
        assert_eq!(
            classify(&view(me_at(300.0, 300.0), vec![foe])),
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
        assert_eq!(classify(&view(me, vec![foe])), Situation::Advantage);
    }

    /// A body with no hostile in view is in `Neutral`, however cornered: being near
    /// an edge only means something relative to someone. But reeling is reeling —
    /// a hazard, a boss volume, or a stray shot still puts you at a disadvantage.
    #[test]
    fn with_no_opponent_cornered_is_neutral_but_hitstun_is_not() {
        let me = me_at(10.0, 300.0);
        assert_eq!(classify(&view(me, vec![])), Situation::Neutral);

        let mut me = me_at(400.0, 300.0);
        me.phase = BodyPhase::Hitstun;
        assert_eq!(classify(&view(me, vec![])), Situation::Disadvantage);
    }

    /// A dead opponent is nobody's advantage. `nearest_hostile` already filters
    /// them, and this pins that it keeps doing so.
    #[test]
    fn a_dead_opponent_offers_no_window() {
        let mut foe = foe_at(500.0, 300.0);
        foe.alive = false;
        foe.phase = BodyPhase::Hitstun;
        assert_eq!(
            classify(&view(me_at(300.0, 300.0), vec![foe])),
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
}
