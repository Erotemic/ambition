//! Running the scenario suite through the real brain, and reporting what it did.
//!
//! `docs/planning/engine/fighter-brain.md` asks for "the smallest headless runner
//! that executes those scenarios through the real fighter-brain/controller seam
//! and records useful outcomes". This is that runner's first half: everything
//! measurable WITHOUT a match.
//!
//! survival and damage are deliberately not here. They need two bodies
//! actually fighting, which is a match harness (`ambition_demo_smash_app` has
//! one) rather than a brain rig — and a number labelled "survival %" produced
//! without anyone dying would be worse than no number at all. What a brain alone
//! can be asked is: *what did you decide, how often did you press, and do you do
//! the same thing twice.*
//!
//! ⭐ THE KIT FIXTURE LANDED — [`rig_kit`] — and the ladder stopped reading as
//! degenerate. Before it, the rig ran `BrainSnapshot::idle()`, whose empty
//! `attack_kit` leaves `generate_options` offering movement only, so every rung
//! emitted zero presses and no scoring difference could show.
//!
//! ⚠ AND A KIT IS NOT ENOUGH ON ITS OWN: [`ScenarioOutcome::apm`] counts attack
//! presses, an attack is offered only where it can touch the opponent, and the
//! suite's fixtures are authored far enough apart that the pacing in [`play`]
//! has to CLOSE for any of them to press at all. Read that comment before
//! quoting a number from here.

use super::decision::tick_fighter;
use super::scenarios::{suite, Scenario};
use ambition_characters::actor::attack_gesture::AttackDir;
use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::brain::attack_kit::{
    ActionLegality, AttackBinding, AttackCandidate, AttackVerb,
};
use ambition_characters::brain::fighter::data::{FighterCfg, FighterState};
use ambition_characters::brain::fighter::profile::FighterBrainProfile;
use ambition_characters::brain::BrainSnapshot;

/// Ticks per second the rig scores against — the sim's fixed rate, so an APM
/// reading here means the same thing it would in a match.
pub const RIG_TICK_HZ: f32 = 60.0;

/// Long enough for a rate to be a rate: ten seconds of decisions.
pub const RIG_TICKS: u32 = (RIG_TICK_HZ as u32) * 10;

/// How close the rig's opponent comes at the near end of its pacing.
///
/// Inside the longest move [`rig_kit`] authors (90px) and outside the shortest
/// (40px), so the spacing trade-off the scorer is being measured on is live for
/// part of every pass rather than decided by the fixture's authored gap.
pub const RIG_ARMS_LENGTH: f32 = 60.0;

/// A kit shaped like the one production builds.
///
/// the rig ran with `BrainSnapshot::idle()` first and every rung emitted zero
/// presses, because an empty kit leaves `generate_options` offering movement
/// only. A brain with nothing to throw cannot be told apart from another brain
/// with nothing to throw, so the ladder read as degenerate.
///
/// Mirrors `build_attack_kit` in the actor tick: one candidate per (verb,
/// direction) the moveset answers for, each carrying its move's frame data. The
/// numbers here are a plausible spread rather than any character's real moveset
/// — the rig measures the DECIDING, and a scenario that named a specific
/// character would be measuring content instead.
///
fn rig_kit() -> Vec<AttackCandidate> {
    let base_frames = |startup_s: f32,
                       reach: f32,
                       damage: i32,
                       coverage: Option<ambition_entity_catalog::MoveCoverage>| ambition_entity_catalog::MoveFrameData {
        total_s: startup_s + 0.1 + 0.2,
        charge_hold_at_s: None,
        startup_s,
        active_spans: vec![(startup_s, startup_s + 0.1)],
        recovery_s: 0.2,
        cancel_windows: Vec::new(),
        reach,
        ignores_guard: false,
        hazard_reach: 0.0,
        coverage,
        // This fixture authors no windbox.
        push_coverage: None,
        push_dir: None,
        max_damage: damage,
        max_knockback: 0.0,
        // This rig's moves author no launch, so no move on it is a finisher.
        launch: ambition_entity_catalog::LaunchEnvelope::default(),
        start_impulse: (0.0, 0.0),
        // The rig's moves displace nobody: it measures the DECIDING, and a
        // candidate that carried a way home would put a recovery route into
        // every scenario that has nothing to do with one.
        lift_speed: 0.0,
        lift_at_s: 0.0,
        lift_side: 0.0,
        recovery_route: Default::default(),
    };
    // ⛔⛤ **AND THE `Up` CANDIDATE WAS A THIRD FORWARD POKE.** Every move here
    // was built with the same `min: (0, -12), max: (reach, 12)` box whatever
    // direction it was bound to, so the kit had no answer above or below it —
    // which is the exact defect `MoveCoverage`'s own doc records about George
    // Booul's vertical game, reproduced in the rig that is supposed to catch
    // it. The box now follows the binding.
    let frames = |startup_s: f32, reach: f32, damage: i32, up: bool| {
        let coverage = (reach > 0.0).then(|| {
            if up {
                ambition_entity_catalog::MoveCoverage {
                    min: (-12.0, -reach),
                    max: (12.0, 0.0),
                }
            } else {
                ambition_entity_catalog::MoveCoverage {
                    min: (0.0, -12.0),
                    max: (reach, 12.0),
                }
            }
        });
        base_frames(startup_s, reach, damage, coverage)
    };
    // Fast-and-short, slow-and-long, and an aerial — enough that scoring has a
    // trade-off to make. One candidate is not a choice.
    vec![
        AttackCandidate {
            move_id: "rig_jab".into(),
            frames: frames(0.03, 40.0, 2, false),
            binding: AttackBinding {
                verb: AttackVerb::Basic,
                direction: AttackDir::Forward,
            },
            legality: ActionLegality::Now,
        },
        AttackCandidate {
            move_id: "rig_smash".into(),
            frames: frames(0.18, 90.0, 12, false),
            binding: AttackBinding {
                verb: AttackVerb::Smash,
                direction: AttackDir::Forward,
            },
            legality: ActionLegality::Now,
        },
        AttackCandidate {
            move_id: "rig_uptilt".into(),
            frames: frames(0.06, 55.0, 5, true),
            binding: AttackBinding {
                verb: AttackVerb::Basic,
                direction: AttackDir::Up,
            },
            legality: ActionLegality::Now,
        },
    ]
}

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
    let mut snapshot = BrainSnapshot::idle();
    snapshot.attack_kit = rig_kit();
    let mut out = ActorControlFrame::neutral();
    let mut frames: Vec<ActorControlFrame> = Vec::new();
    let mut view = scenario.view.clone();
    for tick in 0..RIG_TICKS {
        // THE OPPONENT HAS TO MOVE, or the ladder is invisible. A rung's
        // headline difference is `reaction_ms`, and a delayed view of a world
        // that never changes IS the live view — so a static scenario makes every
        // level emit identical frames and a rig built on one would report the
        // ladder as degenerate when it had simply never been asked a question
        // that distinguishes rungs.
        //
        // So the opponent paces: one slow horizontal sweep across the stage,
        // deterministic in `tick`, which is exactly the case a late-seeing brain
        // must lead and an early-seeing one need not.
        //
        // ⛔⛤ **AND IT HAS TO PASS THROUGH REACH, WHICH A FIXED ±120px SWEEP
        // NEVER DID.** [`ScenarioOutcome::apm`] counts ATTACK presses and
        // nothing else, and the option layer offers an attack only where the
        // move's own region touches the opponent. Measured 2026-09-20 against
        // the 90px longest move [`rig_kit`] authors: the suite's fixtures are
        // authored 180..620px apart, and four of them are `Recovery`, where a
        // kit with no lift offers nothing at all — so EIGHT of the nine
        // scenarios could not contribute one press and the ladder's mean was
        // one scenario divided by nine. It was invisible while the option layer
        // admitted anything within three times a move's reach.
        //
        // ⇒ The pacing CLOSES: the opponent walks from where the fixture put
        // them to arm's length and back, once every two seconds. The fixture
        // still says where they START, which is what `Situation` is classified
        // from and what `starting_positions` reports; this only says the rig is
        // a rig.
        let phase = (tick as f32) / (RIG_TICK_HZ * 2.0);
        let closing = 0.5 - 0.5 * (phase * std::f32::consts::TAU).cos();
        let me = scenario.view.self_view.pos;
        for (actor, origin) in view.actors.iter_mut().zip(scenario.view.actors.iter()) {
            let arms_length = me + (origin.pos - me).normalize_or_zero() * RIG_ARMS_LENGTH;
            let was = actor.pos;
            actor.pos = origin.pos.lerp(arms_length, closing);
            // ⛔⛤ **AND THE VELOCITY HAS TO SAY THE SAME THING THE POSITION
            // DOES.** This loop imposes the motion, and it used to leave
            // `vel` at whatever the fixture authored — so a paced opponent
            // walked toward this body every tick while reporting that it was
            // flying off the stage. Nothing read `vel` here, so the
            // contradiction was free, until attack admission started LEADING
            // ITS AIM by the relative velocity and `edgeguard_window` stopped
            // pressing at all: the brain was aiming where the fixture's lie
            // said the opponent would be. The fixture still says where they
            // START; it does not also get to say where they are going while
            // this loop moves them somewhere else.
            actor.vel = (actor.pos - was) * RIG_TICK_HZ;
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
/// ⛔⛔ IT WALKS 1..=9, AND A COMPOSITION NEED NOT REGISTER ALL NINE. This crate
/// cannot know which rungs a game publishes — `FighterBrainProfile::for_level`
/// answers for any level, so an unregistered one yields a GENERIC FALLBACK that
/// is not a ladder rung. The smash demo registers five (1, 3, 5, 6, 9) and its
/// `ladder_rig` says the rest are *"invalid for this measurement"*. ⇒ **a caller
/// reading a calibration off these rows must filter to the levels ITS
/// composition registers**, or four of the nine rows are synthetic and the curve
/// through them is partly invented.
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
