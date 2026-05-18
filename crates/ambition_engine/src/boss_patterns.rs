//! Deterministic boss-pattern schedules.
//!
//! These are not rendering systems. They are small, reviewable design artifacts
//! that let boss attacks be generated, snapshot-tested, and later interpreted by
//! Bevy systems.

/// Coarse attack verbs for the first Ambition boss family.
///
/// Attack verbs are the boss's offensive moves; movement verbs (dash,
/// reposition) live on `BossMovementKind`. A `BossPatternStep` carries
/// one attack OR optionally pairs the attack with a movement beat
/// (e.g. a "dash + slam" combo). The Bevy-side boss controller
/// interprets these into actual world transforms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BossAttackKind {
    FloorSlam,
    SideSweep,
    SpikeHalo,
    DashEcho,
    Rest,
}

/// Coarse movement verbs for boss traversal choreography.
///
/// Replaces the implicit "boss is a stationary target" model with
/// explicit traversal beats. Phases that should feel mobile pair an
/// attack step with a movement step (or use `Hold` movement to
/// commit to a stationary swing). Bevy interprets these into actual
/// position changes; the engine schedule stays a pure data plan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BossMovementKind {
    /// Stay where you are. Default movement when a step doesn't
    /// otherwise specify one.
    Hold,
    /// Quick lateral dash by the given signed magnitude (negative =
    /// left, positive = right). Distance is in world units; speed is
    /// chosen by the controller to land the dash inside `active`
    /// time. Re-uses dash sfx/vfx so it reads as the same verb the
    /// player has.
    Dash { distance: f32 },
    /// Teleport to a named arena corner. The actual coordinates are
    /// looked up by the controller against the active arena's
    /// authored `ArenaAnchor`s; this enum carries only the semantic
    /// destination so the schedule stays arena-agnostic.
    Reposition { anchor: ArenaAnchor },
    /// Walk along a cubic path defined by the controller (e.g. a
    /// boss "circling" pattern). Magnitude scales the controller's
    /// authored radius.
    Orbit { magnitude: f32 },
}

/// Named anchor positions inside a boss arena. Controllers map these
/// to authored coordinates so the schedule stays arena-agnostic
/// (the same Reposition step works on a small basement arena and a
/// large boss-rush arena).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArenaAnchor {
    Center,
    LeftWall,
    RightWall,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// One timed attack step. Durations are in simulation seconds.
///
/// `movement` is `Hold` by default; phases that want traversal
/// choreography set it via `with_movement` (e.g. "dash + slam"
/// combos use `BossMovementKind::Dash` paired with `BossAttackKind::FloorSlam`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BossPatternStep {
    pub attack: BossAttackKind,
    pub movement: BossMovementKind,
    pub telegraph: f32,
    pub active: f32,
    pub recover: f32,
}

impl BossPatternStep {
    pub const fn new(attack: BossAttackKind, telegraph: f32, active: f32, recover: f32) -> Self {
        Self {
            attack,
            movement: BossMovementKind::Hold,
            telegraph,
            active,
            recover,
        }
    }

    /// Builder: pair this step with a movement verb. Common use:
    /// `BossPatternStep::new(FloorSlam, 0.4, 0.18, 0.4).with_movement(BossMovementKind::Dash { distance: 240.0 })`.
    pub const fn with_movement(mut self, movement: BossMovementKind) -> Self {
        self.movement = movement;
        self
    }

    pub fn total_time(self) -> f32 {
        self.telegraph.max(0.0) + self.active.max(0.0) + self.recover.max(0.0)
    }

    pub fn is_valid(self) -> bool {
        self.telegraph.is_finite()
            && self.active.is_finite()
            && self.recover.is_finite()
            && self.telegraph >= 0.0
            && self.active >= 0.0
            && self.recover >= 0.0
            && self.total_time() > 0.0
            && match self.movement {
                BossMovementKind::Hold => true,
                BossMovementKind::Dash { distance } => distance.is_finite(),
                BossMovementKind::Reposition { .. } => true,
                BossMovementKind::Orbit { magnitude } => magnitude.is_finite(),
            }
    }
}

/// Which sub-phase of a boss step is currently running.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BossBeatPhase {
    /// Pre-attack windup. Hitbox not active yet; vfx telegraphs the
    /// strike.
    Telegraph,
    /// Hitbox is live. Player should be punished for staying in the
    /// danger zone.
    Active,
    /// Post-attack vulnerability window. Hitbox is gone; boss is
    /// punishable.
    Recover,
}

/// Result of `BossPatternSchedule::evaluate`. Carries the active step,
/// its phase, and a `[0, 1]` progress within the phase.
#[derive(Clone, Copy, Debug)]
pub struct ActiveBossBeat<'a> {
    pub step_index: usize,
    pub step: &'a BossPatternStep,
    pub phase: BossBeatPhase,
    /// `[0, 1]` progress within the current phase. Useful for
    /// telegraph-fade interpolation, dash-distance lerp, etc.
    pub phase_progress: f32,
}

/// Reviewable schedule for a boss phase.
#[derive(Clone, Debug, PartialEq)]
pub struct BossPatternSchedule {
    pub boss_id: String,
    pub phase: u8,
    pub seed: u64,
    pub steps: Vec<BossPatternStep>,
}

impl BossPatternSchedule {
    pub fn new(
        boss_id: impl Into<String>,
        phase: u8,
        seed: u64,
        steps: Vec<BossPatternStep>,
    ) -> Self {
        Self {
            boss_id: boss_id.into(),
            phase,
            seed,
            steps,
        }
    }

    pub fn gradient_sentinel_phase1() -> Self {
        Self::new(
            "gradient_sentinel",
            1,
            0xA481_7101,
            vec![
                BossPatternStep::new(BossAttackKind::FloorSlam, 0.55, 0.18, 0.62),
                BossPatternStep::new(BossAttackKind::SideSweep, 0.42, 0.22, 0.48),
                BossPatternStep::new(BossAttackKind::Rest, 0.00, 0.35, 0.35),
            ],
        )
    }

    pub fn gradient_sentinel_phase2() -> Self {
        Self::new(
            "gradient_sentinel",
            2,
            0xA481_7102,
            vec![
                BossPatternStep::new(BossAttackKind::FloorSlam, 0.45, 0.18, 0.38),
                BossPatternStep::new(BossAttackKind::SpikeHalo, 0.65, 1.20, 0.30),
                BossPatternStep::new(BossAttackKind::SideSweep, 0.34, 0.20, 0.36),
                BossPatternStep::new(BossAttackKind::DashEcho, 0.50, 0.28, 0.55),
            ],
        )
    }

    /// Phase 3 traversal pattern: pairs attacks with movement beats so
    /// the boss reads as mobile. Demonstrates the full
    /// `BossMovementKind` vocabulary so future phases can pick from
    /// one already-tested template.
    ///
    /// Choreography:
    /// 1. Dash right + slam (close the gap, then commit to the swing).
    /// 2. Reposition to TopLeft + halo (set up a high-ground arena
    ///    sweep).
    /// 3. Orbit (mid-range circling under telegraph; no offensive
    ///    output during the orbit, just pressure positioning).
    /// 4. Dash left + sweep (cap the combo with a wide ground-level
    ///    swing back across the arena).
    pub fn gradient_sentinel_phase3_traversal() -> Self {
        Self::new(
            "gradient_sentinel",
            3,
            0xA481_7103,
            vec![
                BossPatternStep::new(BossAttackKind::FloorSlam, 0.40, 0.18, 0.38)
                    .with_movement(BossMovementKind::Dash { distance: 240.0 }),
                BossPatternStep::new(BossAttackKind::SpikeHalo, 0.55, 1.00, 0.28).with_movement(
                    BossMovementKind::Reposition {
                        anchor: ArenaAnchor::TopLeft,
                    },
                ),
                BossPatternStep::new(BossAttackKind::Rest, 0.10, 0.50, 0.10)
                    .with_movement(BossMovementKind::Orbit { magnitude: 1.0 }),
                BossPatternStep::new(BossAttackKind::SideSweep, 0.35, 0.22, 0.40)
                    .with_movement(BossMovementKind::Dash { distance: -320.0 }),
            ],
        )
    }

    /// Mockingbird Phase 1 — "hover-first" loop the user picked:
    /// Hover (Rest) → Swoop (DashEcho left) → Fireball (SpikeHalo) →
    /// Swoop (DashEcho right). The hover-first opener gives the
    /// player a clean read of the boss before the first commit, then
    /// alternates aerial swoops with a single ranged beat. Roughly
    /// 6-7s per loop; tune in the controller, not here.
    pub fn mockingbird_phase1() -> Self {
        Self::new(
            "mockingbird",
            1,
            0xB1D5_0001,
            vec![
                BossPatternStep::new(BossAttackKind::Rest, 0.10, 1.80, 0.20),
                BossPatternStep::new(BossAttackKind::DashEcho, 0.45, 0.32, 0.40)
                    .with_movement(BossMovementKind::Dash { distance: -260.0 }),
                BossPatternStep::new(BossAttackKind::SpikeHalo, 0.55, 0.45, 0.55),
                BossPatternStep::new(BossAttackKind::DashEcho, 0.45, 0.32, 0.40)
                    .with_movement(BossMovementKind::Dash { distance: 260.0 }),
            ],
        )
    }

    /// Mockingbird Phase 2 — same opening shape as Phase 1 with
    /// tighter telegraphs and an extra swoop pair. Doubles as the
    /// Enrage schedule for now; phase-distinct content can split
    /// later when the Bevy controller has more verbs to mix in.
    pub fn mockingbird_phase2() -> Self {
        Self::new(
            "mockingbird",
            2,
            0xB1D5_0002,
            vec![
                BossPatternStep::new(BossAttackKind::Rest, 0.10, 1.20, 0.18),
                BossPatternStep::new(BossAttackKind::DashEcho, 0.32, 0.28, 0.30)
                    .with_movement(BossMovementKind::Dash { distance: -280.0 }),
                BossPatternStep::new(BossAttackKind::SpikeHalo, 0.45, 0.45, 0.40),
                BossPatternStep::new(BossAttackKind::DashEcho, 0.32, 0.28, 0.30)
                    .with_movement(BossMovementKind::Dash { distance: 280.0 }),
                BossPatternStep::new(BossAttackKind::DashEcho, 0.30, 0.26, 0.34)
                    .with_movement(BossMovementKind::Dash { distance: -240.0 }),
            ],
        )
    }

    /// GNU-ton Phase 1 — hands-only pressure: alternating hand slams and
    /// sweeps while the head stays high. The player must dodge the
    /// hands without being able to strike the head yet.
    ///
    /// Choreography: slam → rest beat → sweep → rest beat
    pub fn gnu_ton_phase1() -> Self {
        Self::new(
            "gnu_ton",
            1,
            0xC7A3_0001,
            vec![
                BossPatternStep::new(BossAttackKind::FloorSlam, 0.60, 0.30, 0.65),
                BossPatternStep::new(BossAttackKind::Rest, 0.00, 0.45, 0.10),
                BossPatternStep::new(BossAttackKind::SideSweep, 0.50, 0.35, 0.55),
                BossPatternStep::new(BossAttackKind::Rest, 0.00, 0.35, 0.10),
            ],
        )
    }

    /// GNU-ton Phase 2 — head descends: SpikeHalo opens the vulnerability
    /// window. The player must survive hand attacks to get punish windows
    /// when the head is low.
    pub fn gnu_ton_phase2() -> Self {
        Self::new(
            "gnu_ton",
            2,
            0xC7A3_0002,
            vec![
                BossPatternStep::new(BossAttackKind::FloorSlam, 0.48, 0.28, 0.52),
                BossPatternStep::new(BossAttackKind::SpikeHalo, 0.70, 1.40, 0.40),
                BossPatternStep::new(BossAttackKind::SideSweep, 0.40, 0.30, 0.45),
                BossPatternStep::new(BossAttackKind::FloorSlam, 0.42, 0.26, 0.48),
            ],
        )
    }

    /// GNU-ton Enrage — rapid-fire pressure with shorter rests, both hand
    /// attacks interleaved with head exposure. The head window gets shorter
    /// under enrage — player must act quickly.
    pub fn gnu_ton_enrage() -> Self {
        Self::new(
            "gnu_ton",
            3,
            0xC7A3_0003,
            vec![
                BossPatternStep::new(BossAttackKind::FloorSlam, 0.35, 0.22, 0.36),
                BossPatternStep::new(BossAttackKind::SpikeHalo, 0.55, 1.00, 0.30),
                BossPatternStep::new(BossAttackKind::SideSweep, 0.32, 0.26, 0.38),
                BossPatternStep::new(BossAttackKind::DashEcho,  0.42, 0.30, 0.40),
                BossPatternStep::new(BossAttackKind::SpikeHalo, 0.48, 0.85, 0.28),
            ],
        )
    }

    pub fn is_valid(&self) -> bool {
        !self.boss_id.is_empty()
            && self.phase > 0
            && !self.steps.is_empty()
            && self.steps.iter().copied().all(BossPatternStep::is_valid)
    }

    pub fn total_time(&self) -> f32 {
        self.steps
            .iter()
            .copied()
            .map(BossPatternStep::total_time)
            .sum()
    }

    /// Evaluate the schedule at `elapsed` simulation seconds. Returns
    /// the active beat (which step + which phase of that step we're
    /// in) so a Bevy controller can drive sprite / collider / vfx
    /// state without re-implementing the timing math.
    ///
    /// `wrap` controls behavior past the schedule's `total_time()`:
    /// - `true`: loop the schedule (mod `total_time`), useful for
    ///   "this phase keeps cycling until HP drains".
    /// - `false`: clamp to the final step's recover phase, useful for
    ///   one-shot patterns.
    ///
    /// Returns `None` only when the schedule is empty / invalid.
    pub fn evaluate(&self, elapsed: f32, wrap: bool) -> Option<ActiveBossBeat<'_>> {
        if self.steps.is_empty() {
            return None;
        }
        let total = self.total_time();
        if total <= 0.0 {
            return None;
        }
        let mut t = if wrap {
            elapsed.rem_euclid(total)
        } else {
            elapsed.max(0.0).min(total - f32::EPSILON)
        };
        // Walk steps accumulating their durations. The first step whose
        // remaining-budget contains `t` is the active step.
        for (index, step) in self.steps.iter().enumerate() {
            let step_total = step.total_time();
            if t < step_total {
                let phase = if t < step.telegraph {
                    BossBeatPhase::Telegraph
                } else if t < step.telegraph + step.active {
                    BossBeatPhase::Active
                } else {
                    BossBeatPhase::Recover
                };
                let phase_progress = match phase {
                    BossBeatPhase::Telegraph if step.telegraph > 0.0 => t / step.telegraph,
                    BossBeatPhase::Active if step.active > 0.0 => {
                        (t - step.telegraph) / step.active
                    }
                    BossBeatPhase::Recover if step.recover > 0.0 => {
                        (t - step.telegraph - step.active) / step.recover
                    }
                    _ => 0.0,
                };
                return Some(ActiveBossBeat {
                    step_index: index,
                    step,
                    phase,
                    phase_progress: phase_progress.clamp(0.0, 1.0),
                });
            }
            t -= step_total;
        }
        // Should be unreachable when total > 0 and elapsed clamped, but
        // return the final step in Recover at progress=1.0 as a safe
        // default rather than panicking.
        let last = self.steps.last().expect("non-empty checked above");
        Some(ActiveBossBeat {
            step_index: self.steps.len() - 1,
            step: last,
            phase: BossBeatPhase::Recover,
            phase_progress: 1.0,
        })
    }

    pub fn summary(&self) -> String {
        let mut out = format!(
            "boss={} phase={} seed={} total={:.3}s",
            self.boss_id,
            self.phase,
            self.seed,
            self.total_time(),
        );
        for (index, step) in self.steps.iter().enumerate() {
            out.push_str(&format!(
                "\n{:02}: {:?} telegraph={:.3} active={:.3} recover={:.3}",
                index, step.attack, step.telegraph, step.active, step.recover,
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mockingbird_schedules_are_valid_and_loop() {
        let p1 = BossPatternSchedule::mockingbird_phase1();
        let p2 = BossPatternSchedule::mockingbird_phase2();
        assert!(p1.is_valid());
        assert!(p2.is_valid());
        // First step must be the Hover (Rest) opener — the user
        // explicitly picked the hover-first shape to give the player
        // a clean read before the boss commits.
        assert_eq!(p1.steps[0].attack, BossAttackKind::Rest);
        // The pattern includes both a Swoop (DashEcho) and a Fireball
        // (SpikeHalo) beat per loop — the user's prose contract.
        assert!(p1
            .steps
            .iter()
            .any(|s| s.attack == BossAttackKind::DashEcho));
        assert!(p1
            .steps
            .iter()
            .any(|s| s.attack == BossAttackKind::SpikeHalo));
        // Loop length is bounded (sanity, not a strict tuning gate).
        assert!(p1.total_time() > 1.5 && p1.total_time() < 12.0);
        assert!(p2.total_time() > 1.5 && p2.total_time() < 12.0);
    }

    #[test]
    fn gradient_sentinel_schedules_are_valid() {
        assert!(BossPatternSchedule::gradient_sentinel_phase1().is_valid());
        assert!(BossPatternSchedule::gradient_sentinel_phase2().is_valid());
    }

    #[test]
    fn empty_boss_id_is_invalid() {
        let mut sched = BossPatternSchedule::gradient_sentinel_phase1();
        sched.boss_id.clear();
        assert!(!sched.is_valid());
    }

    #[test]
    fn zero_phase_is_invalid() {
        let mut sched = BossPatternSchedule::gradient_sentinel_phase1();
        sched.phase = 0;
        assert!(!sched.is_valid());
    }

    #[test]
    fn empty_steps_is_invalid() {
        let mut sched = BossPatternSchedule::gradient_sentinel_phase1();
        sched.steps.clear();
        assert!(!sched.is_valid());
    }

    #[test]
    fn total_time_sums_step_times() {
        let sched = BossPatternSchedule::gradient_sentinel_phase1();
        let expected: f32 = sched.steps.iter().copied().map(|s| s.total_time()).sum();
        assert!((sched.total_time() - expected).abs() < 1e-3);
    }

    #[test]
    fn gradient_sentinel_phase3_traversal_is_valid() {
        let sched = BossPatternSchedule::gradient_sentinel_phase3_traversal();
        assert!(
            sched.is_valid(),
            "phase 3 traversal pattern should validate"
        );
        assert_eq!(sched.phase, 3);
        // The traversal pattern should pair offensive steps with non-Hold
        // movements: at least 3 of the 4 steps move (the Rest step
        // pairs with Orbit, the others with Dash/Reposition).
        let movement_count = sched
            .steps
            .iter()
            .filter(|s| !matches!(s.movement, BossMovementKind::Hold))
            .count();
        assert!(
            movement_count >= 3,
            "traversal pattern should have at least 3 non-Hold movement beats; got {movement_count}"
        );
    }

    #[test]
    fn step_with_movement_round_trips_dash_and_reposition() {
        let dash_step = BossPatternStep::new(BossAttackKind::FloorSlam, 0.4, 0.2, 0.4)
            .with_movement(BossMovementKind::Dash { distance: 200.0 });
        match dash_step.movement {
            BossMovementKind::Dash { distance } => assert_eq!(distance, 200.0),
            _ => panic!("expected Dash movement"),
        }
        let reposition_step = BossPatternStep::new(BossAttackKind::SpikeHalo, 0.5, 1.0, 0.3)
            .with_movement(BossMovementKind::Reposition {
                anchor: ArenaAnchor::TopLeft,
            });
        match reposition_step.movement {
            BossMovementKind::Reposition { anchor } => assert_eq!(anchor, ArenaAnchor::TopLeft),
            _ => panic!("expected Reposition movement"),
        }
    }

    #[test]
    fn step_with_invalid_dash_distance_fails_validation() {
        let bad = BossPatternStep::new(BossAttackKind::FloorSlam, 0.4, 0.2, 0.4)
            .with_movement(BossMovementKind::Dash { distance: f32::NAN });
        assert!(!bad.is_valid(), "NaN dash distance should fail validation");
    }

    #[test]
    fn default_movement_is_hold() {
        let step = BossPatternStep::new(BossAttackKind::Rest, 0.1, 0.1, 0.1);
        assert_eq!(step.movement, BossMovementKind::Hold);
    }

    #[test]
    fn evaluate_returns_first_step_telegraph_at_t0() {
        let sched = BossPatternSchedule::gradient_sentinel_phase1();
        let beat = sched.evaluate(0.0, false).expect("non-empty schedule");
        assert_eq!(beat.step_index, 0);
        assert_eq!(beat.phase, BossBeatPhase::Telegraph);
        assert!(
            beat.phase_progress < 0.05,
            "progress should be ~0 at t=0, got {}",
            beat.phase_progress
        );
    }

    #[test]
    fn evaluate_walks_through_step_phases() {
        let sched = BossPatternSchedule::gradient_sentinel_phase1();
        let s0 = sched.steps[0];
        // Inside the telegraph half-way.
        let beat = sched.evaluate(s0.telegraph * 0.5, false).unwrap();
        assert_eq!(beat.phase, BossBeatPhase::Telegraph);
        assert!((beat.phase_progress - 0.5).abs() < 0.05);
        // Inside the active phase.
        let beat = sched
            .evaluate(s0.telegraph + s0.active * 0.5, false)
            .unwrap();
        assert_eq!(beat.phase, BossBeatPhase::Active);
        assert!((beat.phase_progress - 0.5).abs() < 0.05);
        // Inside the recover phase.
        let beat = sched
            .evaluate(s0.telegraph + s0.active + s0.recover * 0.5, false)
            .unwrap();
        assert_eq!(beat.phase, BossBeatPhase::Recover);
        assert!((beat.phase_progress - 0.5).abs() < 0.05);
    }

    #[test]
    fn evaluate_advances_to_next_step_after_first_completes() {
        let sched = BossPatternSchedule::gradient_sentinel_phase1();
        let s0_total = sched.steps[0].total_time();
        let beat = sched.evaluate(s0_total + 0.01, false).unwrap();
        assert_eq!(beat.step_index, 1);
        assert_eq!(beat.phase, BossBeatPhase::Telegraph);
    }

    #[test]
    fn evaluate_wraps_when_wrap_is_true() {
        let sched = BossPatternSchedule::gradient_sentinel_phase1();
        let total = sched.total_time();
        // After two full cycles + 0.1s, with wrap=true, we should be
        // 0.1s into step 0 again.
        let beat = sched.evaluate(total * 2.0 + 0.1, true).unwrap();
        assert_eq!(beat.step_index, 0);
        assert_eq!(beat.phase, BossBeatPhase::Telegraph);
    }

    #[test]
    fn evaluate_clamps_when_wrap_is_false() {
        let sched = BossPatternSchedule::gradient_sentinel_phase1();
        let total = sched.total_time();
        // Past the end with wrap=false: should sit on the last step's
        // recover phase. The exact phase_progress isn't pinned.
        let beat = sched.evaluate(total * 2.0, false).unwrap();
        assert_eq!(beat.step_index, sched.steps.len() - 1);
        assert_eq!(beat.phase, BossBeatPhase::Recover);
    }

    #[test]
    fn evaluate_returns_none_for_empty_schedule() {
        let sched = BossPatternSchedule::new("empty", 1, 0, Vec::new());
        assert!(sched.evaluate(0.0, false).is_none());
        assert!(sched.evaluate(0.0, true).is_none());
    }

    #[test]
    fn summary_contains_boss_id_and_step_count() {
        let sched = BossPatternSchedule::gradient_sentinel_phase1();
        let summary = sched.summary();
        assert!(summary.contains(&sched.boss_id));
        assert!(summary.contains("phase=1"));
        // One header line plus one line per step.
        let line_count = summary.lines().count();
        assert_eq!(line_count, 1 + sched.steps.len());
    }
}
