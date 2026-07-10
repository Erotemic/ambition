//! **FB3's scenario fixture suite.**
//!
//! `docs/planning/engine/fighter-brain.md` §3: *"Scenario suite: fixture
//! situations (ledge trap, juggle escape, recovery from each offstage quadrant,
//! projectile camping opponent) with pass metrics (survival %, damage ratio) per
//! difficulty."*
//!
//! A scenario is a named [`WorldView`] plus the one fact everyone can agree on
//! before any brain runs: **which [`Situation`] it is.** That is what L1 owes, and
//! it is what this module asserts.
//!
//! ## Why the fixtures live here and not in a test file
//!
//! FB4's ladder rig needs the same eight situations to score survival % and damage
//! ratio against; §3's *"pass metrics per difficulty"* is a measurement OVER these
//! fixtures, not a different set of them. A fixture suite that only a `#[cfg(test)]`
//! module can see gets rebuilt, slightly differently, by the next slice.
//!
//! **The metrics half is not here.** Survival % and damage ratio need a brain to
//! survive and deal damage, and no brain exists above L1. FB4 brings the profiles
//! and the rig; these scenarios are what it will run.

use ambition_engine_core as ae;

use super::situation::Situation;
use crate::actor::ActorFaction;
use crate::perception::{BodyPhase, PerceivedActor, SelfView, StageView, WorldView};

/// The stage every fixture is played on: 800 × 600, origin at zero. Its bounds are
/// the room's world AABB — the same envelope CC3's invariant 3 polices, so
/// "offstage" here and "out of bounds" there are the same predicate.
pub const STAGE_SIZE: ae::Vec2 = ae::Vec2::new(800.0, 600.0);

fn stage() -> StageView {
    StageView {
        bounds: ae::Aabb::new(STAGE_SIZE * 0.5, STAGE_SIZE * 0.5),
    }
}

fn body(pos: ae::Vec2) -> SelfView {
    SelfView {
        pos,
        gravity_down: ae::Vec2::new(0.0, 1.0),
        faction: ActorFaction::Player,
        alive: true,
        on_ground: true,
        health_max: 100,
        ..Default::default()
    }
}

fn foe(pos: ae::Vec2) -> PerceivedActor {
    PerceivedActor {
        id: "foe".to_string(),
        pos,
        faction: ActorFaction::Enemy,
        hostile_to_self: true,
        alive: true,
        on_ground: true,
        health_max: 100,
        ..Default::default()
    }
}

/// One named tactical situation, and the [`Situation`] L1 must read out of it.
///
/// `expect` is not a prediction about a brain. It is the shared premise every
/// later layer argues from: L2 prices options *given* the situation, and a
/// disagreement here is a disagreement about the game, not about the CPU.
pub struct Scenario {
    pub name: &'static str,
    /// Why this fixture exists — the skill the situation demands.
    pub premise: &'static str,
    pub view: WorldView,
    pub expect: Situation,
}

/// The suite. Eight fixtures: §3's four named ones, plus recovery from each of the
/// four offstage quadrants (§3 asks for exactly that, and it is four fixtures, not
/// one — a body knocked off the top has different options from one knocked off the
/// side, and a classifier that conflates them will not be caught by one of them).
pub fn suite() -> Vec<Scenario> {
    let mid = ae::Vec2::new(400.0, 300.0);
    let mut out = Vec::new();

    // §3's four.
    out.push(Scenario {
        name: "ledge_trap",
        premise: "Backed against the blastzone with an opponent in front. The \
                  retreat option is gone; every remaining option is a commitment.",
        view: WorldView {
            self_view: body(ae::Vec2::new(40.0, 300.0)),
            stage: stage(),
            actors: vec![foe(ae::Vec2::new(220.0, 300.0))],
            ..Default::default()
        },
        expect: Situation::Disadvantage,
    });

    out.push(Scenario {
        name: "juggle_escape",
        premise: "Airborne, in hitstun, above an opponent who is waiting. Nothing \
                  the CPU can do is safe; the question is which unsafe thing.",
        view: WorldView {
            self_view: SelfView {
                on_ground: false,
                phase: BodyPhase::Hitstun,
                phase_remaining: 0.25,
                vel: ae::Vec2::new(0.0, -200.0),
                ..body(ae::Vec2::new(400.0, 120.0))
            },
            stage: stage(),
            actors: vec![foe(mid)],
            ..Default::default()
        },
        expect: Situation::Disadvantage,
    });

    out.push(Scenario {
        name: "projectile_camper",
        premise: "An opponent at range with a shot in the air. Not a punish window \
                  and not a disadvantage — the CPU has to WANT to approach. This is \
                  the fixture that catches an L2 which only ever reacts.",
        view: WorldView {
            self_view: body(ae::Vec2::new(200.0, 300.0)),
            stage: stage(),
            actors: vec![foe(ae::Vec2::new(700.0, 300.0))],
            projectiles: vec![crate::perception::PerceivedProjectile {
                pos: ae::Vec2::new(600.0, 300.0),
                vel: ae::Vec2::new(-400.0, 0.0),
                damage: 3,
                hostile_to_self: true,
            }],
            ..Default::default()
        },
        expect: Situation::Neutral,
    });

    out.push(Scenario {
        name: "edgeguard_window",
        premise: "The opponent is offstage and must come back through you. The \
                  single highest-value window in the game, and it expires.",
        view: WorldView {
            self_view: body(mid),
            stage: stage(),
            actors: vec![PerceivedActor {
                on_ground: false,
                vel: ae::Vec2::new(60.0, 100.0),
                ..foe(ae::Vec2::new(-40.0, 340.0))
            }],
            ..Default::default()
        },
        expect: Situation::EdgeGuard,
    });

    // Recovery, from each offstage quadrant. Four fixtures, not one: a body knocked
    // off the TOP has different options from one knocked off the SIDE, and a
    // classifier that conflates them is not caught by a single case.
    for (name, pos) in [
        ("recovery_left", ae::Vec2::new(-40.0, 300.0)),
        ("recovery_right", ae::Vec2::new(840.0, 300.0)),
        ("recovery_below", ae::Vec2::new(400.0, 640.0)),
        ("recovery_above", ae::Vec2::new(400.0, -40.0)),
    ] {
        out.push(Scenario {
            name,
            premise: "Self is past a blastzone. Nothing else about the tick matters \
                      — a stock lost there is not repaid by a punish.",
            view: WorldView {
                self_view: SelfView {
                    on_ground: false,
                    ..body(pos)
                },
                stage: stage(),
                actors: vec![foe(mid)],
                ..Default::default()
            },
            expect: Situation::Recovery,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::fighter::situation::classify;

    /// **The suite, classified.** Every fixture reads out as the situation its name
    /// claims. A failure here is a disagreement about the GAME, not about the CPU.
    #[test]
    fn l1_reads_every_scenario_the_way_its_name_says() {
        for s in suite() {
            assert_eq!(
                classify(&s.view),
                s.expect,
                "`{}` — {}\ngot {:?}, expected {:?}",
                s.name,
                s.premise,
                classify(&s.view),
                s.expect
            );
        }
    }

    /// The four recovery quadrants really are four different geometries, not the
    /// same fixture spelled four ways. Without this, a `Recovery` test that only
    /// checked `x < 0` would pass the suite and miss the ceiling.
    #[test]
    fn the_four_recovery_quadrants_are_four_distinct_positions() {
        let mut seen: Vec<ae::Vec2> = suite()
            .iter()
            .filter(|s| s.name.starts_with("recovery_"))
            .map(|s| s.view.self_view.pos)
            .collect();
        assert_eq!(seen.len(), 4);
        seen.dedup_by(|a, b| a == b);
        assert_eq!(seen.len(), 4, "four quadrants, four positions");
        assert!(seen.iter().any(|p| p.x < 0.0), "off the left");
        assert!(seen.iter().any(|p| p.x > STAGE_SIZE.x), "off the right");
        assert!(seen.iter().any(|p| p.y < 0.0), "off the top");
        assert!(seen.iter().any(|p| p.y > STAGE_SIZE.y), "off the bottom");
    }

    /// The suite covers every `Situation` a fight can be in except `Advantage`,
    /// which §3 does not name a fixture for — the punish windows are L2's to price,
    /// and `advantage_is_the_opponents_commitment_and_never_its_active_frames`
    /// already pins the classification. Recorded here so the omission is a choice.
    #[test]
    fn the_suite_covers_four_of_the_five_situations() {
        let mut seen: Vec<Situation> = suite().iter().map(|s| s.expect).collect();
        seen.sort();
        seen.dedup();
        assert_eq!(
            seen,
            vec![
                Situation::Neutral,
                Situation::EdgeGuard,
                Situation::Disadvantage,
                Situation::Recovery,
            ]
        );
    }

    /// Every fixture says WHY it exists. A scenario suite whose entries cannot
    /// explain themselves is a set of magic numbers that fail together.
    #[test]
    fn every_scenario_states_its_premise() {
        for s in suite() {
            assert!(s.premise.len() > 60, "`{}` has no premise", s.name);
        }
    }
}
