//! **Running the scenario suite through the real brain, and reporting what it did.**
//!
//! `docs/planning/engine/fighter-brain.md` asks for "the smallest headless runner
//! that executes those scenarios through the real fighter-brain/controller seam
//! and records useful outcomes". This is that runner's first half: everything
//! measurable WITHOUT a match.
//!
//! ⛔ **survival and damage are deliberately not here.** They need two bodies
//! actually fighting, which is a match harness (`ambition_demo_smash_app` has
//! one) rather than a brain rig — and a number labelled "survival %" produced
//! without anyone dying would be worse than no number at all. What a brain alone
//! can be asked is: *what did you decide, how often did you press, and do you do
//! the same thing twice.*
//!
//! ⛔⛔ **AND THE FIRST THING IT MEASURED IS THAT IT CANNOT YET MEASURE THE
//! LADDER.** Run across all nine rungs and every scenario, the brain emits
//! **zero presses** and produces identical frames at every level. That is not a
//! degenerate ladder — it is `BrainSnapshot::idle()`, which carries no attack
//! kit, and the decision tests say so in their own words: *"no scene here can
//! arm one"*. An empty kit means `generate_options` offers movement only, so
//! there is no attack for a rung's scoring to differ about.
//!
//! ⇒ **two checks are deliberately absent until the rig hands the brain a kit.**
//! `apm_cap` carries the comment *"Data today; enforcement is FB4's rig"* — the
//! enforcement actually exists (`ApmLedger::may_press` gates every press); what
//! was missing is the measurement, and a within-cap assertion at zero presses is
//! a check that cannot fail. Same for ladder ordering. Building the kit fixture
//! is this rig's next slice and is what also unlocks survival/damage.

use super::decision::{tick_fighter, FighterCfg, FighterState};
use super::profile::FighterBrainProfile;
use super::scenarios::{suite, Scenario};
use crate::actor::control::ActorControlFrame;
use crate::brain::BrainSnapshot;

/// Ticks per second the rig scores against — the sim's fixed rate, so an APM
/// reading here means the same thing it would in a match.
pub const RIG_TICK_HZ: f32 = 60.0;

/// Long enough for a rate to be a rate: ten seconds of decisions.
pub const RIG_TICKS: u32 = (RIG_TICK_HZ as u32) * 10;

/// One scenario played by one ladder rung.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioOutcome {
    pub scenario: &'static str,
    pub level: u8,
    /// Actions per minute the brain actually emitted.
    pub apm: f32,
    /// What the profile authorises.
    pub apm_cap: f32,
    /// Distinct control frames produced — a decision that never changes is a
    /// brain that is not reacting to the situation it was handed.
    pub distinct_frames: usize,
}

impl ScenarioOutcome {
    /// Did the brain stay inside the press budget its level authors?
    pub fn within_apm_cap(&self) -> bool {
        self.apm <= self.apm_cap
    }
}

/// Play one scenario with one profile and report what the brain did.
///
/// The seed is a parameter because determinism is one of the things being
/// measured: the same seed must produce the same run.
pub fn play(scenario: &Scenario, profile: FighterBrainProfile, seed: u64) -> ScenarioOutcome {
    let level = profile.level;
    let apm_cap = profile.apm_cap;
    let cfg = FighterCfg::new(profile);
    let mut state = FighterState::new(&cfg, seed);
    let snapshot = BrainSnapshot::idle();
    let mut out = ActorControlFrame::neutral();
    let mut frames: Vec<ActorControlFrame> = Vec::new();
    let mut view = scenario.view.clone();
    for tick in 0..RIG_TICKS {
        // ⛔ **THE OPPONENT HAS TO MOVE, or the ladder is invisible.** A rung's
        // headline difference is `reaction_ms`, and a delayed view of a world
        // that never changes IS the live view — so a static scenario makes every
        // level emit identical frames and a rig built on one would report the
        // ladder as degenerate when it had simply never been asked a question
        // that distinguishes rungs.
        //
        // So the opponent paces: one slow horizontal sweep across the stage,
        // deterministic in `tick`, which is exactly the case a late-seeing brain
        // must lead and an early-seeing one need not.
        let phase = (tick as f32) / (RIG_TICK_HZ * 2.0);
        let sweep = (phase * std::f32::consts::TAU).sin() * 120.0;
        for (actor, origin) in view.actors.iter_mut().zip(scenario.view.actors.iter()) {
            actor.pos.x = origin.pos.x + sweep;
        }
        view.sim_time = tick as f32 / RIG_TICK_HZ;
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        if !frames.iter().any(|seen| seen == &out) {
            frames.push(out.clone());
        }
    }
    ScenarioOutcome {
        scenario: scenario.name,
        level,
        apm: state.apm.apm(RIG_TICK_HZ),
        apm_cap,
        distinct_frames: frames.len(),
    }
}

/// The whole suite across the whole authored ladder — the repeatable report.
pub fn report(seed: u64) -> Vec<ScenarioOutcome> {
    let scenarios = suite();
    let mut rows = Vec::new();
    for level in 1..=9u8 {
        let profile = FighterBrainProfile::for_level(level);
        for scenario in &scenarios {
            rows.push(play(scenario, profile.clone(), seed));
        }
    }
    rows
}

#[cfg(test)]
mod tests;
