//! Deterministic boss-pattern schedules.
//!
//! These are not rendering systems. They are small, reviewable design artifacts
//! that let boss attacks be generated, snapshot-tested, and later interpreted by
//! Bevy systems.

/// Coarse attack verbs for the first Ambition boss family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BossAttackKind {
    FloorSlam,
    SideSweep,
    SpikeHalo,
    DashEcho,
    Rest,
}

/// One timed attack step. Durations are in simulation seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BossPatternStep {
    pub attack: BossAttackKind,
    pub telegraph: f32,
    pub active: f32,
    pub recover: f32,
}

impl BossPatternStep {
    pub const fn new(attack: BossAttackKind, telegraph: f32, active: f32, recover: f32) -> Self {
        Self {
            attack,
            telegraph,
            active,
            recover,
        }
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
    }
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
    pub fn new(boss_id: impl Into<String>, phase: u8, seed: u64, steps: Vec<BossPatternStep>) -> Self {
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

    pub fn is_valid(&self) -> bool {
        !self.boss_id.is_empty()
            && self.phase > 0
            && !self.steps.is_empty()
            && self.steps.iter().copied().all(BossPatternStep::is_valid)
    }

    pub fn total_time(&self) -> f32 {
        self.steps.iter().copied().map(BossPatternStep::total_time).sum()
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
                index,
                step.attack,
                step.telegraph,
                step.active,
                step.recover,
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_sentinel_schedules_are_valid() {
        assert!(BossPatternSchedule::gradient_sentinel_phase1().is_valid());
        assert!(BossPatternSchedule::gradient_sentinel_phase2().is_valid());
    }
}
