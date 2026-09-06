//! Diagnostic, non-blocking boss-fight validator over authored patterns, seeds, and data-driven
//! bands. Validation is per telegraph -> strike -> rest [`Beat`], because punish windows belong to
//! occurrences rather than attack definitions.
//!
//! Implemented checks cover telegraph proportionality, answer coverage, punish windows, and
//! telegraph readability. Simultaneity is not checked until overlapping threat lifetime is visible
//! to the validator. Missing readability metadata remains a warning rather than an install gate.

use std::collections::BTreeSet;

use super::seeds::{MovementVerb, SeedLibrary, ThreatClass};
use ambition_characters::brain::boss_pattern::{
    BossAttackPattern, BossAttackProfile, BossEncounterPhase, BossPatternStep, TelegraphSpec,
};

/// Per-threat-class tick floors.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreatTicks {
    pub pressure: f32,
    pub light: f32,
    pub medium: f32,
    pub heavy: f32,
}

impl ThreatTicks {
    pub fn for_class(&self, class: ThreatClass) -> f32 {
        match class {
            ThreatClass::Pressure => self.pressure,
            ThreatClass::Light => self.light,
            ThreatClass::Medium => self.medium,
            ThreatClass::Heavy => self.heavy,
        }
    }
}

/// The per-game calibration §3 pins. Content data, never a const here.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatorBands {
    /// Sim ticks per second. The bands are in ticks; the authored data is in
    /// seconds. This is the only conversion, and it lives with the numbers.
    pub tick_hz: f32,
    /// Rule 1: minimum telegraph, by the attack's threat class.
    pub telegraph_ticks: ThreatTicks,
    /// Rule 3: minimum punish window after the active frames, by threat class.
    /// `Pressure` is exempt and its entry is ignored.
    pub recovery_ticks: ThreatTicks,
    /// Rule 2's second half: the verbs a fight must demand between them.
    pub core_verbs: Vec<MovementVerb>,
    /// A deviation at or under this fraction of the floor is a WARNING; beyond it,
    /// an ERROR (§3: *"band deviations ≤ 20% = WARNINGS … > 20% = ERROR"*).
    pub warn_deviation_frac: f32,
}

/// §3's hard error vs. warning split.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Requires an inline `// boss-tuning:` justification the validator prints.
    Warning,
    /// A hard finding (diagnostic today; not an install rejection).
    Error,
}

/// Which §3 rule a finding belongs to. Numbered as the doc numbers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// 1 — telegraph proportionality.
    TelegraphProportionality,
    /// 2 — answer coverage.
    AnswerCoverage,
    /// 3 — the commitment rule (punish window).
    Commitment,
    /// 5 — the readability floor: distinct attacks must differ in telegraph.
    ReadabilityFloor,
    /// A move the seed library does not catalogue. Not one of §3's five: it is the
    /// precondition for rules 1–3, all of which read the seed.
    UncataloguedMove,
}

/// One thing wrong with a fight.
#[derive(Clone, Debug, PartialEq)]
pub struct FightFinding {
    pub severity: Severity,
    pub rule: Rule,
    /// The move key, or the fight id for a fight-wide finding.
    pub subject: String,
    pub detail: String,
}

/// One authored occurrence of an attack: what a player actually experiences.
///
/// `recovery_s` is the `Rest` that FOLLOWS the strike — zero when the next beat is
/// another telegraph, which is exactly the un-punishable chain §3 rule 3 forbids.
#[derive(Clone, Debug, PartialEq)]
pub struct Beat {
    pub move_key: String,
    pub phase: BossEncounterPhase,
    pub telegraph_s: f32,
    pub active_s: f32,
    pub recovery_s: f32,
    /// BD3's authored anticipation, if the step carried one. §3 rule 5 reads it.
    pub telegraph: Option<TelegraphSpec>,
}

fn move_key(profile: &BossAttackProfile) -> &str {
    match profile {
        BossAttackProfile::Strike(k) | BossAttackProfile::Special(k) => k.as_str(),
    }
}

/// Walk one phase's authored step list into beats.
///
/// A `Select` arm's steps are beats the fight can play, so they are walked as
/// their own sequence (arms are alternatives, not a continuation). A `Stance`
/// marker is a jump; its body is walked by the caller from `pattern.stances`.
fn beats_in(steps: &[BossPatternStep], phase: BossEncounterPhase, out: &mut Vec<Beat>) {
    let mut pending_telegraph: Option<(String, f32, Option<TelegraphSpec>)> = None;
    let mut awaiting_recovery: Option<usize> = None;

    for step in steps {
        match step {
            BossPatternStep::Telegraph {
                profile,
                duration,
                telegraph,
            } => {
                // A telegraph ends the previous strike's punish window: whatever
                // Rest we had (possibly none) is what the player got.
                awaiting_recovery = None;
                pending_telegraph = Some((
                    move_key(profile).to_string(),
                    *duration,
                    telegraph.clone().filter(|t| t.is_authored()),
                ));
            }
            BossPatternStep::Strike { profile, duration } => {
                let key = move_key(profile).to_string();
                let (telegraph_s, telegraph) = match pending_telegraph.take() {
                    Some((tk, d, spec)) if tk == key => (d, spec),
                    // A strike with no telegraph of its own. Rule 1 will report it
                    // as a zero-telegraph attack, which is what it is.
                    _ => (0.0, None),
                };
                out.push(Beat {
                    move_key: key,
                    phase,
                    telegraph_s,
                    active_s: *duration,
                    recovery_s: 0.0,
                    telegraph,
                });
                awaiting_recovery = Some(out.len() - 1);
            }
            BossPatternStep::Rest { duration } => {
                if let Some(i) = awaiting_recovery.take() {
                    out[i].recovery_s += *duration;
                }
                pending_telegraph = None;
            }
            BossPatternStep::Select { table } => {
                awaiting_recovery = None;
                pending_telegraph = None;
                for arm in table {
                    beats_in(&arm.steps, phase, out);
                }
            }
            BossPatternStep::Stance { .. } => {
                awaiting_recovery = None;
                pending_telegraph = None;
            }
        }
    }

    // A phase's timeline LOOPS. So a strike that ends the list is followed by
    // whatever begins it — and if that is a `Rest`, the player got a punish window
    // after all. Without this, every phase whose last beat is a strike reads as
    // unpunishable, which is a fact about the walker, not about the fight.
    if let Some(i) = awaiting_recovery {
        for step in steps {
            match step {
                BossPatternStep::Rest { duration } => out[i].recovery_s += *duration,
                _ => break,
            }
        }
    }
}

/// Every beat a fight can play, across every phase, plus every stance body.
///
/// A `Cycle` boss has no per-step timings: it rotates `attacks` on the profile's
/// flat windup/active/cooldown, and its cooldown IS its punish window. Both shapes
/// produce beats, because both shapes hit the player.
pub fn fight_beats(
    pattern: &BossAttackPattern,
    cycle_attacks: &[BossAttackProfile],
    cycle_windup_s: f32,
    cycle_active_s: f32,
    cycle_cooldown_s: f32,
) -> Vec<Beat> {
    let mut out = Vec::new();
    match pattern {
        BossAttackPattern::Cycle => {
            for profile in cycle_attacks {
                out.push(Beat {
                    move_key: move_key(profile).to_string(),
                    phase: BossEncounterPhase::Phase1,
                    telegraph_s: cycle_windup_s,
                    active_s: cycle_active_s,
                    recovery_s: cycle_cooldown_s,
                    // A `Cycle` boss authors no per-step telegraph: the flat windup
                    // is all it has. Rule 5 reports it as un-identified, which it is.
                    telegraph: None,
                });
            }
        }
        BossAttackPattern::Scripted {
            intro,
            phase1,
            transition,
            phase2,
            enrage,
        } => {
            for (phase, pattern) in [
                (BossEncounterPhase::Phase1, intro),
                (BossEncounterPhase::Phase1, phase1),
                (BossEncounterPhase::Phase2, transition),
                (BossEncounterPhase::Phase2, phase2),
                (BossEncounterPhase::Enrage, enrage),
            ] {
                beats_in(&pattern.steps, phase, &mut out);
                for stance in pattern.stances.values() {
                    beats_in(stance, phase, &mut out);
                }
            }
        }
    }
    out
}

/// Run §3's rules 1, 2 and 3 over one fight. Findings come back sorted
/// most-severe first, then by rule, then by subject — a stable order, so a CI
/// diff of two runs is readable.
pub fn validate_fight(
    fight_id: &str,
    beats: &[Beat],
    seeds: &SeedLibrary,
    bands: &ValidatorBands,
) -> Vec<FightFinding> {
    let mut findings = Vec::new();
    let tick = 1.0 / bands.tick_hz.max(1.0);
    let mut demanded: BTreeSet<MovementVerb> = BTreeSet::new();

    for beat in beats {
        let Some((_seed_id, seed)) = seeds.seed_for_move(&beat.move_key) else {
            findings.push(FightFinding {
                severity: Severity::Error,
                rule: Rule::UncataloguedMove,
                subject: beat.move_key.clone(),
                detail: format!(
                    "no seed catalogues `{}`, so its threat class and fair counters are \
                     unknown and rules 1–3 cannot judge it",
                    beat.move_key
                ),
            });
            continue;
        };
        demanded.extend(seed.fair_counters.iter().copied());

        // Rule 2, first half: an attack with no answer is a fight the player
        // cannot play. Always an error, never a band.
        if seed.fair_counters.is_empty() {
            findings.push(FightFinding {
                severity: Severity::Error,
                rule: Rule::AnswerCoverage,
                subject: beat.move_key.clone(),
                detail: "declares no fair counter".to_string(),
            });
        }

        // Rule 1: telegraph proportionality.
        let floor_s = bands.telegraph_ticks.for_class(seed.threat) * tick;
        if beat.telegraph_s < floor_s {
            findings.push(band_finding(
                Rule::TelegraphProportionality,
                &beat.move_key,
                beat.phase,
                "telegraph",
                beat.telegraph_s,
                floor_s,
                bands,
            ));
        }

        // Rule 3: the commitment rule. A `Pressure` attack is exempt — it is
        // chip-level by declaration, and §3 caps it by damage instead.
        if seed.threat != ThreatClass::Pressure {
            let floor_s = bands.recovery_ticks.for_class(seed.threat) * tick;
            if beat.recovery_s < floor_s {
                findings.push(band_finding(
                    Rule::Commitment,
                    &beat.move_key,
                    beat.phase,
                    "punish window",
                    beat.recovery_s,
                    floor_s,
                    bands,
                ));
            }
        }
    }

    // Rule 5 — the readability floor. *"Distinct attacks must differ in telegraph
    // (pose row OR cue)."* Only expressible since BD3 gave a telegraph an IDENTITY:
    // a DURATION cannot distinguish two attacks, and a fight in which everything
    // looks the same is unreadable however generous its timings.
    //
    // Scanned over a `Vec`, sorted — never a hash map (ADR 0023): a validator's
    // error list must not depend on hash seed.
    let mut identities: Vec<(String, String)> = Vec::new();
    let mut unidentified: Vec<String> = Vec::new();
    for beat in beats {
        match beat.telegraph.as_ref().filter(|t| t.is_authored()) {
            Some(spec) => {
                let (pose, cue) = spec.identity();
                let key = format!("{}|{}", pose.unwrap_or(""), cue.unwrap_or(""));
                identities.push((key, beat.move_key.clone()));
            }
            None => {
                if !unidentified.contains(&beat.move_key) {
                    unidentified.push(beat.move_key.clone());
                }
            }
        }
    }
    identities.sort();
    for pair in identities.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if a.0 == b.0 && a.1 != b.1 {
            findings.push(FightFinding {
                severity: Severity::Error,
                rule: Rule::ReadabilityFloor,
                subject: format!("{} / {}", a.1, b.1),
                detail: format!(
                    "two distinct attacks share the telegraph identity `{}` — nothing \
                     on screen tells them apart",
                    a.0
                ),
            });
        }
    }
    if !unidentified.is_empty() {
        unidentified.sort();
        findings.push(FightFinding {
            severity: Severity::Warning,
            rule: Rule::ReadabilityFloor,
            subject: fight_id.to_string(),
            detail: format!(
                "{} attack(s) author no telegraph (pose or cue): {unidentified:?}. \
                 Promotion to a hard error requires calibration and a separate \
                 maintainer decision; today the roster authors none.",
                unidentified.len()
            ),
        });
    }

    // Rule 2, second half: forced-movement variety. A fight that never demands a
    // verb never teaches it. A warning, not an error: which verbs a GAME considers
    // core is a design statement, and the shipped roster demonstrably never asks
    // for a parry.
    let missing: Vec<MovementVerb> = bands
        .core_verbs
        .iter()
        .copied()
        .filter(|v| !demanded.contains(v))
        .collect();
    if !missing.is_empty() {
        findings.push(FightFinding {
            severity: Severity::Warning,
            rule: Rule::AnswerCoverage,
            subject: fight_id.to_string(),
            detail: format!(
                "the fight never demands {missing:?} — it does not exercise the kit \
                 (§3 rule 2, forced-movement variety)"
            ),
        });
    }

    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then(a.rule.cmp(&b.rule))
            .then(a.subject.cmp(&b.subject))
            .then(a.detail.cmp(&b.detail))
    });
    findings.dedup();
    findings
}

/// A deviation ≤ `warn_deviation_frac` below the floor is a warning; beyond it, an
/// error. The detail carries both numbers in TICKS, because that is the unit the
/// designer authored against.
fn band_finding(
    rule: Rule,
    move_key: &str,
    phase: BossEncounterPhase,
    what: &str,
    actual_s: f32,
    floor_s: f32,
    bands: &ValidatorBands,
) -> FightFinding {
    let deviation = if floor_s > 0.0 {
        (floor_s - actual_s) / floor_s
    } else {
        0.0
    };
    let severity = if deviation <= bands.warn_deviation_frac {
        Severity::Warning
    } else {
        Severity::Error
    };
    FightFinding {
        severity,
        rule,
        subject: move_key.to_string(),
        detail: format!(
            "{phase:?}: {what} is {:.0} ticks, floor is {:.0} ({:.0}% under)",
            actual_s * bands.tick_hz,
            floor_s * bands.tick_hz,
            deviation * 100.0
        ),
    }
}

impl ValidatorBands {
    /// Parse the per-game calibration RON.
    pub fn from_ron(ron: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(ron)
    }
}

#[cfg(test)]
mod tests;
