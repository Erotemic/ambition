//! Shared tactical fixtures for fighter classification and evaluation.
//! Each scenario pairs a [`WorldView`] with the expected [`Situation`] so tests
//! and measurement tools use the same setup.

use ambition_platformer2d_core as ae;

use ambition_characters::actor::ActorFaction;
use ambition_characters::brain::fighter::situation::Situation;
use ambition_characters::perception::{BodyPhase, PerceivedActor, SelfView, StageView, WorldView};

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
#[derive(Clone)]
pub struct Scenario {
    pub name: &'static str,
    /// Why this fixture exists — the skill the situation demands.
    pub premise: &'static str,
    pub view: WorldView,
    pub expect: Situation,
}

/// Eight tactical fixtures, including distinct recovery cases for each offstage quadrant.
impl Scenario {
    /// Stage positions for scenarios that include an opponent. Harnesses apply
    /// these after seating, because the stage owns fighter placement.
    pub fn starting_positions(&self) -> Option<(ae::Vec2, ae::Vec2)> {
        let foe = self.view.actors.first()?;
        Some((self.view.self_view.pos, foe.pos))
    }

    /// Stage VELOCITIES for scenarios that include an opponent, alongside
    /// [`starting_positions`](Self::starting_positions).
    ///
    /// ⭐ A harness that can set these reclaims every fixture whose only
    /// unreproduced state is `velocity` — measured 2026-09-03, that is
    /// `edgeguard_window`, one of the four the ladder rig was skipping. The
    /// transit authority already accepts one (`TransitVelocity::Set`), so the
    /// gap was an accessor, not a capability.
    ///
    /// ⚠ Positions and velocities are returned separately ON PURPOSE. A caller
    /// that can place but not push is still correct to use
    /// `starting_positions` alone and report the fixture as unreproduced;
    /// bundling them would let it silently drop the half it cannot apply.
    pub fn starting_velocities(&self) -> Option<(ae::Vec2, ae::Vec2)> {
        let foe = self.view.actors.first()?;
        Some((self.view.self_view.vel, foe.vel))
    }

    /// Seconds of HITSTUN each body starts in, when that is the whole of its
    /// phase premise.
    ///
    /// ⭐ `BodyPhase` is DERIVED, not stored: the runtime's `body_phase()` reads
    /// it from `BodyCombat.hitstun_timer` / `recoil_lock_timer`, `BodyMelee`'s
    /// attack phase, and the shield. So a harness reproduces `Hitstun` by
    /// writing the TIMER — the source of truth — and never by assigning the
    /// derived enum.
    ///
    /// ⛔ `None` unless every non-`Neutral` phase in the fixture is `Hitstun`.
    /// The attack phases need a `BodyMelee` mid-swing, which a timer cannot
    /// fake, and a harness that got `Some` for those would stage a body the
    /// fixture did not describe. `juggle_escape` is the case this serves; a
    /// startup/active fixture must still be reported unreproduced.
    pub fn starting_hitstun(&self) -> Option<(f32, f32)> {
        let foe = self.view.actors.first()?;
        let expressible = |phase: BodyPhase| {
            matches!(phase, BodyPhase::Neutral | BodyPhase::Hitstun)
        };
        if !expressible(self.view.self_view.phase) || !expressible(foe.phase) {
            return None;
        }
        let seconds = |phase: BodyPhase, remaining: f32| {
            if phase == BodyPhase::Hitstun {
                remaining.max(f32::EPSILON)
            } else {
                0.0
            }
        };
        Some((
            seconds(self.view.self_view.phase, self.view.self_view.phase_remaining),
            seconds(foe.phase, foe.phase_remaining),
        ))
    }

    /// Which bodies the fixture starts HANGING ON A LEDGE, as `(me, foe)`.
    ///
    /// ⛔ A hang is not a position — dropping a body at the ledge coordinates
    /// leaves it falling past them. A harness reproduces one by writing the
    /// ledge-grab state AND snapping the body to the contact's anchor, which is
    /// why this is separate from [`starting_positions`](Self::starting_positions):
    /// the anchor comes from the real platform, not from the fixture's stage.
    pub fn starting_ledge_hangs(&self) -> Option<(bool, bool)> {
        let foe = self.view.actors.first()?;
        // ⚠ `SelfView` carries no `ledge_hanging` field at all: the fixture
        // describes the OPPONENT hanging, and a brain's own hang reaches it
        // through its motion state rather than through perceiving itself. So
        // this reports the foe, and `false` for self is a fact about the type
        // rather than a claim about the fixture.
        Some((false, foe.ledge_hanging))
    }

    /// Hostile shots the fixture starts with in the air, as
    /// `(offset_from_self, direction)` per projectile.
    ///
    /// ⛔ **Direction and OFFSET, not position and velocity.** The fixture's
    /// coordinates describe its own 800x600 stage, and pasting them onto the
    /// running stage is the mistake `starting_positions_on` exists to avoid. A
    /// harness maps the offset the same way it maps positions, and fires a REAL
    /// authored bolt rather than a spec built from these numbers — the fixture's
    /// premise is *"a shot in the air"*, not a particular damage value.
    pub fn starting_shots(&self) -> Vec<(ae::Vec2, ae::Vec2)> {
        self.view
            .projectiles
            .iter()
            .filter(|shot| shot.hostile_to_self)
            .map(|shot| {
                let offset = shot.pos - self.view.self_view.pos;
                let dir = if shot.vel == ae::Vec2::ZERO {
                    ae::Vec2::new(-1.0, 0.0)
                } else {
                    shot.vel.normalize()
                };
                (offset, dir)
            })
            .collect()
    }

    /// Scenario state that a position-only harness cannot reproduce.
    ///
    /// Derived from the fixture itself. Grounded state is excluded because normal
    /// simulation can establish it from placement; velocity, phases, and projectiles
    /// require explicit setup.
    pub fn unreproduced_by_placement(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        let me = &self.view.self_view;
        let moving = me.vel != ae::Vec2::ZERO
            || self
                .view
                .actors
                .iter()
                .any(|actor| actor.vel != ae::Vec2::ZERO);
        if moving {
            missing.push("velocity");
        }
        let mid_phase = me.phase != BodyPhase::Neutral
            || me.phase_remaining > 0.0
            || self
                .view
                .actors
                .iter()
                .any(|actor| actor.phase != BodyPhase::Neutral || actor.phase_remaining > 0.0);
        if mid_phase {
            missing.push("body phase");
        }
        if !self.view.projectiles.is_empty() {
            missing.push("projectiles");
        }
        // A HANG IS NOT A POSITION. Dropping a body at the ledge coordinates
        // leaves it falling past them; catching the edge is a maneuver with its
        // own window, so a placement-only harness has to arrange it and say so.
        if self.view.actors.iter().any(|actor| actor.ledge_hanging) {
            missing.push("ledge hang");
        }
        if me.damage_taken > 0 || self.view.actors.iter().any(|actor| actor.damage_taken > 0) {
            missing.push("damage");
        }
        missing
    }

    /// Whether placing the two bodies reproduces this scenario's whole premise.
    pub fn is_reproduced_by_placement(&self) -> bool {
        self.unreproduced_by_placement().is_empty()
    }

    /// Map fixture-relative starting positions onto `stage`. Points outside the fixture
    /// remain outside so recovery scenarios preserve their offstage premise.
    ///
    /// This maps placement only; see [`Self::unreproduced_by_placement`] for additional
    /// scenario state a caller must reproduce.
    pub fn starting_positions_on(&self, stage: ae::Aabb) -> Option<(ae::Vec2, ae::Vec2)> {
        use ae::AabbExt;
        let (me, foe) = self.starting_positions()?;
        let size = stage.half_size() * 2.0;
        let min = stage.center() - stage.half_size();
        let map = |p: ae::Vec2| {
            ae::Vec2::new(
                min.x + (p.x / STAGE_SIZE.x) * size.x,
                min.y + (p.y / STAGE_SIZE.y) * size.y,
            )
        };
        Some((map(me), map(foe)))
    }
}

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
            projectiles: vec![ambition_characters::perception::PerceivedProjectile {
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

    out.push(Scenario {
        name: "edgeguard_ledge_hang",
        premise: "The opponent is HANGING ON THE LEDGE. Inside the room's box, \
                  phase Neutral, not landing — so every other term says nothing \
                  is happening — and it is the most punishable state in the \
                  genre: no walk, no shield, and every way out is a committed \
                  animation on a clock.",
        view: WorldView {
            self_view: body(mid),
            stage: stage(),
            actors: vec![PerceivedActor {
                on_ground: false,
                ledge_hanging: true,
                ..foe(ae::Vec2::new(40.0, 330.0))
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

    /// No two scenarios start from the same two points.
    ///
    /// found by running them: `ladder_rig --scenarios` printed
    /// byte-identical columns for `recovery_right` and `recovery_below`, which
    /// can only happen if the bodies start in the same places. Two fixtures that
    /// differ only in NAME make a suite look broader than it is — and the whole
    /// argument for §8 is that a rollout should pay in some situations and not
    /// others, which a duplicated situation cannot show.
    #[test]
    fn no_two_scenarios_start_from_the_same_positions() {
        let suite = suite();
        let mut seen: Vec<(&str, (ae::Vec2, ae::Vec2))> = Vec::new();
        for scenario in &suite {
            let Some(start) = scenario.starting_positions() else {
                continue;
            };
            if let Some((other, _)) = seen
                .iter()
                .find(|(_, s)| s.0.distance(start.0) < 0.5 && s.1.distance(start.1) < 0.5)
            {
                panic!(
                    "`{}` and `{other}` start from the same two points, so any \
                     measurement over this suite counts one situation twice",
                    scenario.name
                );
            }
            seen.push((scenario.name, start));
        }
    }

    /// Which fixtures a placement-only harness may report results for.
    ///
    /// the ladder rig ran all eight through `starting_positions_on` and
    /// printed a row per name, so `juggle_escape` ran with nobody in hitstun,
    /// `projectile_camper` with no projectile and `edgeguard_window` against a
    /// motionless opponent. Three tactical names over three positional fixtures.
    ///
    /// the membership is asserted BOTH WAYS. A one-sided check ("the three
    /// are refused") stays green if the derivation starts refusing everything,
    /// which would silently empty the ladder while looking stricter.
    #[test]
    fn only_the_positional_fixtures_are_reproduced_by_a_placement() {
        let suite = suite();
        let reproduced: Vec<&str> = suite
            .iter()
            .filter(|s| s.is_reproduced_by_placement())
            .map(|s| s.name)
            .collect();
        assert_eq!(
            reproduced,
            vec![
                "ledge_trap",
                "recovery_left",
                "recovery_right",
                "recovery_below",
                "recovery_above",
            ],
            "the set a placement-only harness may report on changed"
        );
        for (name, missing) in [
            ("juggle_escape", vec!["velocity", "body phase"]),
            ("projectile_camper", vec!["projectiles"]),
            ("edgeguard_window", vec!["velocity"]),
            ("edgeguard_ledge_hang", vec!["ledge hang"]),
        ] {
            let scenario = suite
                .iter()
                .find(|s| s.name == name)
                .expect("a fixture named in this suite");
            assert_eq!(
                scenario.unreproduced_by_placement(),
                missing,
                "`{name}` reports the wrong missing state, so a harness cannot \
                 tell a reader what its run was not"
            );
        }
    }

    /// Every scenario that names an opponent can be PLAYED.
    ///
    /// the suite was classification-only: eight `WorldView` fixtures the
    /// classifier is asked about and no fighter has ever stood in. A ladder
    /// measured "over §8's scenarios" while seating every rung at the stage's
    /// authored spawn is measuring one situation eight times.
    ///
    /// this asserts the two bodies are APART, because a scenario whose
    /// positions coincide is not a situation — and a fixture that degenerated
    /// that way would produce a bout the premise never described while looking
    /// like a scenario run.
    #[test]
    fn every_scenario_with_an_opponent_yields_two_distinct_positions() {
        let suite = suite();
        assert!(suite.len() >= 8, "the suite shrank to {}", suite.len());
        let mut playable = 0;
        for scenario in &suite {
            let Some((me, foe)) = scenario.starting_positions() else {
                // A scenario about terrain alone names no opponent; that is a
                // legitimate fixture and simply not a bout.
                assert!(
                    scenario.view.actors.is_empty(),
                    "`{}` has an opponent but yielded no positions",
                    scenario.name
                );
                continue;
            };
            playable += 1;
            assert!(
                me.distance(foe) > 1.0,
                "`{}` puts both fighters in the same place, which is not a \
                 situation",
                scenario.name
            );
        }
        assert!(
            playable >= 4,
            "only {playable} of {} scenarios can be played, so a ladder run over \
             this suite would be mostly the stage's default spawn",
            suite.len()
        );
    }
    use super::*;
    use ambition_characters::brain::fighter::situation::classify;
    use ambition_characters::perception::Perceived;

    /// The suite, classified. Every fixture reads out as the situation its name
    /// claims. A failure here is a disagreement about the GAME, not about the CPU.
    #[test]
    fn l1_reads_every_scenario_the_way_its_name_says() {
        for s in suite() {
            assert_eq!(
                classify(Perceived::cheating(&s.view)),
                s.expect,
                "`{}` — {}\ngot {:?}, expected {:?}",
                s.name,
                s.premise,
                classify(Perceived::cheating(&s.view)),
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

    /// ⭐ THE SAME SITUATION OFF EITHER LEDGE.
    ///
    /// D184: `recovery_left` engages in real bouts while `recovery_right` dies at
    /// 0%, from placements symmetric in every measured input. The DI path was
    /// cleared, so this asks the layer below the movement choice — does the brain
    /// even NAME the two sides the same way?
    ///
    /// A stage is symmetric; a body the same distance past either ledge is in the
    /// same situation, and if classification disagrees nothing downstream can
    /// recover from it.
    #[test]
    fn a_body_past_either_ledge_is_classified_the_same() {
        let width = super::STAGE_SIZE.x;
        for out_by in [40.0_f32, 120.0] {
            let left = super::Scenario {
                name: "probe_left",
                premise: "past the left edge",
                view: WorldView {
                    self_view: SelfView {
                        on_ground: false,
                        ..super::body(ae::Vec2::new(-out_by, 300.0))
                    },
                    stage: super::stage(),
                    actors: vec![super::foe(ae::Vec2::new(400.0, 300.0))],
                    ..Default::default()
                },
                expect: Situation::Recovery,
            };
            let right = super::Scenario {
                name: "probe_right",
                premise: "past the right edge",
                view: WorldView {
                    self_view: SelfView {
                        on_ground: false,
                        ..super::body(ae::Vec2::new(width + out_by, 300.0))
                    },
                    stage: super::stage(),
                    actors: vec![super::foe(ae::Vec2::new(400.0, 300.0))],
                    ..Default::default()
                },
                expect: Situation::Recovery,
            };
            assert_eq!(
                classify(Perceived::cheating(&left.view)),
                classify(Perceived::cheating(&right.view)),
                "a body {out_by}px past the LEFT ledge and one {out_by}px past the RIGHT \
                 must be in the same situation"
            );
        }
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

    /// ⛔⛔ **EVERY UNREPRODUCED STATE HAS AN ACCESSOR THAT ANSWERS IT.**
    ///
    /// The ladder rig went from staging 5 of these 9 fixtures to all 9 on
    /// 2026-09-03, by reading `starting_velocities`, `starting_hitstun`,
    /// `starting_ledge_hangs` and `starting_shots` instead of skipping. ⚠ But
    /// nothing asserted the pairing: `unreproduced_by_placement` names a state
    /// as a STRING, and a harness matches on that string. Add a fixture whose
    /// premise needs something new — or rename a state — and every harness
    /// silently goes back to skipping it, honestly reporting a smaller number
    /// that nobody is watching.
    ///
    /// ⇒ So this pins the pairing itself, not the count. A new unreproduced
    /// state must arrive with the accessor that answers it, in the same commit.
    #[test]
    fn every_unreproduced_state_has_an_accessor_that_answers_it() {
        for s in suite() {
            for state in s.unreproduced_by_placement() {
                let answered = match state {
                    "velocity" => s.starting_velocities().is_some(),
                    // ⓘ Attack phases are legitimately unanswerable — they need
                    // a `BodyMelee` mid-swing, which no accessor can fake — so
                    // `None` here is a correct answer, not a missing one. No
                    // fixture in the suite asks for one today; when one does,
                    // this arm is where the decision gets made rather than
                    // discovered.
                    "body phase" => s.starting_hitstun().is_some(),
                    "ledge hang" => s.starting_ledge_hangs().is_some(),
                    "projectiles" => !s.starting_shots().is_empty(),
                    other => panic!(
                        "`{}` reports unreproduced state `{other}`, which no accessor \
                         answers. A harness matching on this string will skip the \
                         fixture and report a smaller count without failing. Add the \
                         accessor in the commit that adds the state.",
                        s.name
                    ),
                };
                assert!(
                    answered,
                    "`{}` reports unreproduced state `{state}` and its accessor \
                     declines to answer, so every harness must skip it",
                    s.name
                );
            }
        }
    }
}
