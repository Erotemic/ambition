//! Wave POLICY: the spawn-cadence director for wave/arena encounters.
//!
//! E8/E9 split the old wave state machine in two. The LIFECYCLE
//! (inactive/starting/active/completed/failed, elapsed time, signals, the
//! objective decision) is the generic [`EncounterLifecycle`](crate::EncounterLifecycle);
//! this module keeps only what is genuinely wave-shaped — which mob spawns
//! when. Completion is not decided here: the director publishes the
//! [`WAVES_EXHAUSTED_SIGNAL`] once the last wave has fully spawned, and the
//! authored objective ("that signal received AND every minion defeated")
//! completes through the generic reducer like any other encounter.

use bevy::prelude::Component;

use crate::objective::{EncounterObjective, Objective};
use crate::participants::{EncounterParticipant, EncounterParticipants, EncounterRole};
use crate::{EncounterEvent, EncounterMobSpec, EncounterPhase, EncounterSpec};

/// Stable signal the wave director publishes when every authored wave has
/// fully spawned (content publishes facts; generic objectives consume them).
pub const WAVES_EXHAUSTED_SIGNAL: &str = "waves_exhausted";

/// Extra breathing room between waves after the previous wave is fully defeated.
///
/// This is intentionally short: long enough for the player and the adaptive
/// music to register that the fight escalated, but not long enough to make the
/// encounter feel idle.
pub const ENCOUNTER_INTER_WAVE_DELAY_SECONDS: f32 = 0.70;

fn add_inter_wave_delay(mobs: &[EncounterMobSpec]) -> Vec<EncounterMobSpec> {
    mobs.iter()
        .cloned()
        .map(|mut mob| {
            mob.delay += ENCOUNTER_INTER_WAVE_DELAY_SECONDS;
            mob
        })
        .collect()
}

/// Per-activation run state of the wave director. Reset when the lifecycle
/// returns to `Inactive` (fresh attempt).
#[derive(Clone, Debug, Default)]
pub struct EncounterRun {
    /// The wave currently spawning/fighting; `None` until the first Active
    /// tick arms wave 0.
    pub wave_index: Option<usize>,
    /// MobSpecs the active wave hasn't spawned yet (entries drop out when
    /// their delay elapses).
    pub pending: Vec<EncounterMobSpec>,
    /// Seconds since the active wave started.
    pub wave_elapsed: f32,
    /// Whether [`WAVES_EXHAUSTED_SIGNAL`] was already published this
    /// activation.
    pub exhausted_signaled: bool,
}

/// The wave policy COMPONENT: the authored spec plus the live spawn-cadence
/// run. Lives beside the generic lifecycle on a wave encounter entity;
/// a boss wrap or signal-driven encounter simply has no `EncounterWaves`.
#[derive(Component, Clone, Debug)]
pub struct EncounterWaves {
    pub spec: EncounterSpec,
    pub run: EncounterRun,
    /// Bumped each time a unique mob id needs to be generated. Lets
    /// successive encounter attempts produce non-colliding ids. Deliberately
    /// NOT reset per activation.
    pub spawn_counter: u32,
}

impl EncounterWaves {
    pub fn new(spec: EncounterSpec) -> Self {
        Self {
            spec,
            run: EncounterRun::default(),
            spawn_counter: 0,
        }
    }

    /// The generic objective this wave spec completes through: every authored
    /// wave fully spawned (the director's signal) and every minion defeated.
    /// A spec with no waves completes on the signal alone (nothing to defeat).
    pub fn objective(&self) -> EncounterObjective {
        let exhausted = Objective::ReceiveSignal(WAVES_EXHAUSTED_SIGNAL.to_string());
        let win = if self.spec.waves.is_empty() {
            exhausted
        } else {
            Objective::All(vec![
                exhausted,
                Objective::AllWithRoleDefeated(EncounterRole::Minion),
            ])
        };
        EncounterObjective::win(win)
    }

    /// Live minions still standing (dead members are retained for the
    /// objective, so this filters on `alive`).
    pub fn alive_minions(participants: &EncounterParticipants) -> usize {
        participants
            .with_role(EncounterRole::Minion)
            .filter(|m| m.alive)
            .count()
    }

    /// Mobs the HUD should report as remaining: not-yet-spawned plus alive.
    pub fn remaining_mobs(&self, participants: &EncounterParticipants) -> usize {
        self.run.pending.len() + Self::alive_minions(participants)
    }

    /// Fresh attempt — called by the adapter when the lifecycle returns to
    /// `Inactive`. (`spawn_counter` survives so mob ids never collide.)
    pub fn reset_run(&mut self) {
        self.run = EncounterRun::default();
    }

    /// Advance the spawn cadence one `Active` tick. Appends spawned `Minion`
    /// participants, emits `WaveStarted`/`EnemySpawned`/`SpawnCommand` events,
    /// and returns `true` when [`WAVES_EXHAUSTED_SIGNAL`] should be published
    /// this tick (once per activation). Pure — the ECS adapter applies the
    /// spawn commands and routes the signal through the command ingress.
    pub fn tick_active(
        &mut self,
        dt: f32,
        participants: &mut EncounterParticipants,
        events: &mut Vec<EncounterEvent>,
    ) -> bool {
        // Arm wave 0 on the first Active tick of this activation.
        if self.run.wave_index.is_none() {
            if self.spec.waves.is_empty() {
                return self.mark_exhausted();
            }
            self.run.wave_index = Some(0);
            self.run.pending = self.spec.waves[0].mobs.clone();
            self.run.wave_elapsed = 0.0;
            events.push(EncounterEvent::WaveStarted {
                wave_index: 0,
                label: self.spec.waves[0].label.clone(),
            });
        }
        let Some(wave_index) = self.run.wave_index else {
            return false;
        };
        self.run.wave_elapsed += dt;

        // Spawn pending mobs whose delay has elapsed → a `SpawnCommand` the
        // adapter applies + a live `Minion` participant (spawned + owned).
        let mut still_pending = Vec::with_capacity(self.run.pending.len());
        for mob in std::mem::take(&mut self.run.pending) {
            if mob.delay <= self.run.wave_elapsed {
                self.spawn_counter = self.spawn_counter.saturating_add(1);
                let id = format!(
                    "encounter:{}:w{}:{}",
                    self.spec.id, wave_index, self.spawn_counter
                );
                events.push(EncounterEvent::EnemySpawned {
                    kind: mob.kind.clone(),
                });
                events.push(EncounterEvent::SpawnCommand {
                    id: id.clone(),
                    kind: mob.kind.clone(),
                    pos: mob.spawn,
                    size: mob.size,
                });
                participants.members.push(EncounterParticipant::spawned(
                    id,
                    None,
                    EncounterRole::Minion,
                ));
            } else {
                still_pending.push(mob);
            }
        }
        self.run.pending = still_pending;

        // Wave defeated → arm the next wave (dead members are retained; only
        // the alive count gates the advance).
        if self.run.pending.is_empty() && Self::alive_minions(participants) == 0 {
            let next_wave = wave_index + 1;
            if let Some(next) = self.spec.waves.get(next_wave) {
                self.run.wave_index = Some(next_wave);
                self.run.pending = add_inter_wave_delay(&next.mobs);
                self.run.wave_elapsed = 0.0;
                events.push(EncounterEvent::WaveStarted {
                    wave_index: next_wave,
                    label: next.label.clone(),
                });
                return false;
            }
        }

        // Every authored wave has fully spawned → publish the exhaustion
        // signal (the objective then completes when the last minion falls).
        if self.run.pending.is_empty()
            && self.run.wave_index == Some(self.spec.waves.len().saturating_sub(1))
        {
            return self.mark_exhausted();
        }
        false
    }

    fn mark_exhausted(&mut self) -> bool {
        if self.run.exhausted_signaled {
            return false;
        }
        self.run.exhausted_signaled = true;
        true
    }

    pub fn hud_summary(
        &self,
        phase: EncounterPhase,
        participants: &EncounterParticipants,
    ) -> String {
        let id = self.spec.id.as_str();
        match phase {
            EncounterPhase::Inactive => format!("[{id}] inactive"),
            EncounterPhase::Starting { remaining } => {
                let bar = countdown_bar(remaining, 3.0);
                format!("[{id}] LOCKED IN — wave 1 in {remaining:.1}s {bar}")
            }
            EncounterPhase::Active => {
                let wave_index = self.run.wave_index.unwrap_or(0);
                let total = self.spec.waves.len();
                let label = self
                    .spec
                    .waves
                    .get(wave_index)
                    .map(|w| w.label.as_str())
                    .unwrap_or("wave");
                let remaining = self.remaining_mobs(participants);
                format!(
                    "[{id}] WAVE {}/{} :: {label} :: {} left",
                    wave_index + 1,
                    total,
                    remaining
                )
            }
            EncounterPhase::Completed => format!("[{id}] CLEARED"),
            EncounterPhase::Failed => format!("[{id}] FAILED — reset to retry"),
        }
    }
}

/// The camera zoom the active encounters want this frame: the max authored
/// `camera_zoom` over every in-flight encounter (`1.0` if none). The host
/// publishes it into [`EncounterView`](crate::EncounterView) each tick.
/// `max` (not first) so the result is order-independent — the encounter
/// entities are queried, and query order is not stable.
pub fn active_encounter_camera_zoom<'a>(
    states: impl IntoIterator<Item = (EncounterPhase, &'a EncounterSpec)>,
) -> f32 {
    let mut zoom = 1.0_f32;
    for (phase, spec) in states {
        if phase.in_flight() {
            zoom = zoom.max(spec.camera_zoom);
        }
    }
    zoom
}

fn countdown_bar(remaining: f32, total: f32) -> String {
    if total <= 0.0 {
        return String::new();
    }
    let ratio = (1.0 - (remaining / total).clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let filled = (ratio * 8.0).round() as usize;
    let empty = 8usize.saturating_sub(filled);
    format!("[{}{}]", "#".repeat(filled), ".".repeat(empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::{EncounterCommandKind, EncounterLifecycle};
    use crate::spec::{EncounterMobSpec, EncounterSpec, EncounterWaveSpec};
    use ambition_interaction::PickupKind;

    fn wave(label: &str, mob_count: usize) -> EncounterWaveSpec {
        EncounterWaveSpec {
            label: label.into(),
            mobs: (0..mob_count)
                .map(|i| EncounterMobSpec::new(format!("mob_{i}"), [0.0, 0.0]))
                .collect(),
        }
    }

    fn spec(waves: Vec<EncounterWaveSpec>) -> EncounterSpec {
        EncounterSpec {
            id: "test_enc".into(),
            waves,
            trigger_min: [0.0, 0.0],
            trigger_size: [100.0, 100.0],
            camera_zoom: 1.2,
            lock_wall: None,
            intro_seconds: 0.0,
            music_track: String::new(),
            reward: PickupKind::Health { amount: 2 },
        }
    }

    /// Drive one wave-encounter tick the way the ECS adapter does: director
    /// cadence, then the generic reducer with the director's objective.
    fn drive(
        waves: &mut EncounterWaves,
        lifecycle: &mut EncounterLifecycle,
        participants: &mut EncounterParticipants,
        dt: f32,
    ) -> Vec<EncounterEvent> {
        let mut events = Vec::new();
        let mut commands = Vec::new();
        if lifecycle.phase == EncounterPhase::Active
            && waves.tick_active(dt, participants, &mut events)
        {
            commands.push(EncounterCommandKind::Signal(
                WAVES_EXHAUSTED_SIGNAL.to_string(),
            ));
        }
        let objective = waves.objective();
        events.extend(lifecycle.reduce(dt, commands.iter(), participants, Some(&objective)));
        events
    }

    fn kill_all(participants: &mut EncounterParticipants) {
        for m in &mut participants.members {
            m.alive = false;
        }
    }

    #[test]
    fn defeating_the_last_mob_of_the_last_wave_completes_through_the_objective() {
        let mut waves = EncounterWaves::new(spec(vec![wave("only", 1)]));
        let mut lc = EncounterLifecycle::default();
        let mut parts = EncounterParticipants::default();
        lc.reduce(0.0, [&EncounterCommandKind::Start], &parts, None);
        assert_eq!(lc.phase, EncounterPhase::Active);

        // First Active tick: wave 0 arms + its mob spawns + exhaustion signals
        // (single wave, fully spawned) — but the minion is alive, so the
        // encounter stays Active.
        let events = drive(&mut waves, &mut lc, &mut parts, 0.1);
        assert!(events
            .iter()
            .any(|e| matches!(e, EncounterEvent::WaveStarted { wave_index: 0, .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, EncounterEvent::SpawnCommand { .. })));
        assert_eq!(lc.phase, EncounterPhase::Active, "minion still alive");

        // The minion dies → the generic objective completes the lifecycle.
        kill_all(&mut parts);
        let events = drive(&mut waves, &mut lc, &mut parts, 0.1);
        assert_eq!(lc.phase, EncounterPhase::Completed);
        assert!(events.contains(&EncounterEvent::Completed));
        assert!(events.contains(&EncounterEvent::LockChanged { locked: false }));
    }

    #[test]
    fn defeating_a_wave_advances_to_the_next_and_does_not_complete_early() {
        let mut waves = EncounterWaves::new(spec(vec![wave("first", 1), wave("second", 2)]));
        let mut lc = EncounterLifecycle::default();
        let mut parts = EncounterParticipants::default();
        lc.reduce(0.0, [&EncounterCommandKind::Start], &parts, None);

        drive(&mut waves, &mut lc, &mut parts, 0.1); // arm + spawn wave 0
        kill_all(&mut parts);
        let events = drive(&mut waves, &mut lc, &mut parts, 0.1);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EncounterEvent::WaveStarted { wave_index: 1, .. })),
            "advanced to the second wave"
        );
        assert_eq!(
            lc.phase,
            EncounterPhase::Active,
            "wave-1 mobs dead but wave 2 pending — the exhaustion signal has \
             not fired, so the objective must NOT complete between waves"
        );

        // Let the inter-wave delay elapse so wave 2 spawns, then kill them.
        let mut guard = 0;
        while EncounterWaves::alive_minions(&parts) == 0 {
            drive(&mut waves, &mut lc, &mut parts, 0.5);
            guard += 1;
            assert!(guard < 10, "wave 2 must spawn");
        }
        kill_all(&mut parts);
        drive(&mut waves, &mut lc, &mut parts, 0.1);
        assert_eq!(lc.phase, EncounterPhase::Completed);
    }

    #[test]
    fn an_encounter_with_no_waves_completes_on_the_exhaustion_signal_alone() {
        let mut waves = EncounterWaves::new(spec(vec![]));
        let mut lc = EncounterLifecycle::default();
        let mut parts = EncounterParticipants::default();
        lc.reduce(0.0, [&EncounterCommandKind::Start], &parts, None);
        drive(&mut waves, &mut lc, &mut parts, 0.1);
        assert_eq!(lc.phase, EncounterPhase::Completed);
    }

    #[test]
    fn delayed_mobs_spawn_when_their_delay_elapses() {
        let mut delayed = wave("delayed", 2);
        delayed.mobs[1].delay = 1.0;
        let mut waves = EncounterWaves::new(spec(vec![delayed]));
        let mut lc = EncounterLifecycle::default();
        let mut parts = EncounterParticipants::default();
        lc.reduce(0.0, [&EncounterCommandKind::Start], &parts, None);

        let events = drive(&mut waves, &mut lc, &mut parts, 0.1);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, EncounterEvent::SpawnCommand { .. }))
                .count(),
            1,
            "only the undelayed mob spawns on the first tick"
        );
        assert!(!waves.run.exhausted_signaled, "a mob is still pending");

        let events = drive(&mut waves, &mut lc, &mut parts, 1.5);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, EncounterEvent::SpawnCommand { .. }))
                .count(),
            1,
            "the delayed mob spawns once its delay elapses"
        );
        assert!(waves.run.exhausted_signaled);
    }

    #[test]
    fn reset_run_arms_a_fresh_attempt_with_non_colliding_ids() {
        let mut waves = EncounterWaves::new(spec(vec![wave("only", 1)]));
        let mut parts = EncounterParticipants::default();
        let mut events = Vec::new();
        waves.tick_active(0.1, &mut parts, &mut events);
        let first_ids: Vec<String> = parts.members.iter().map(|m| m.id.clone()).collect();

        waves.reset_run();
        parts.members.clear();
        assert!(waves.run.wave_index.is_none());
        assert!(!waves.run.exhausted_signaled);
        let mut events = Vec::new();
        waves.tick_active(0.1, &mut parts, &mut events);
        for m in &parts.members {
            assert!(
                !first_ids.contains(&m.id),
                "spawn_counter survives reset so ids never collide"
            );
        }
    }

    #[test]
    fn remaining_mobs_counts_pending_plus_alive() {
        let mut delayed = wave("w", 2);
        delayed.mobs[1].delay = 5.0;
        let mut waves = EncounterWaves::new(spec(vec![delayed]));
        let mut parts = EncounterParticipants::default();
        let mut events = Vec::new();
        waves.tick_active(0.1, &mut parts, &mut events);
        assert_eq!(waves.remaining_mobs(&parts), 2, "1 pending + 1 alive");
        kill_all(&mut parts);
        assert_eq!(waves.remaining_mobs(&parts), 1, "1 pending + 0 alive");
    }

    #[test]
    fn camera_zoom_reads_in_flight_encounters_only() {
        let s = spec(vec![wave("w", 1)]);
        assert_eq!(
            active_encounter_camera_zoom([(EncounterPhase::Active, &s)]),
            1.2
        );
        assert_eq!(
            active_encounter_camera_zoom([(EncounterPhase::Completed, &s)]),
            1.0
        );
    }
}
