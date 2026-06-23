//! Generic ENCOUNTER SCRIPT — ordered beats `{ when: Trigger, then: [Effect] }`
//! that advance as triggers fire (refactor Stage R5).
//!
//! An encounter entity ([`EncounterDef`]) may carry an [`EncounterScript`] for
//! its bespoke scripted beats (a cut-rope puzzle, a timed add-wave, a gauntlet
//! cue). This is the encounter's OWN mechanism, parallel to — and sharing a
//! vocabulary with — the entity-local phase triggers ([`PhaseTrigger`]), but a
//! distinct concern: phase triggers are the boss's intrinsic self-progression;
//! the script is the orchestration the encounter layers on top.
//!
//! Triggers OBSERVE the world / members; effects COMMAND members / world. The
//! cut-rope fight is expressed as a script (rope/anvil are the hazard mechanism
//! that fires `Gate("cut_rope_impact")`; the script does the `ForceKill` +
//! victory banner), and the swallowed-NPC release falls out of the generic
//! [`ReleaseOnDeath`](super::encounter_entity::ReleaseOnDeath) capability.
//!
//! See `docs/planning/boss-entity-local-refactor.md` (encounter-script shape).
//!
//! [`PhaseTrigger`]: crate::boss_encounter::PhaseTrigger
//! [`EncounterDef`]: super::encounter_entity::EncounterDef

use bevy::prelude::*;

use super::encounter_entity::EncounterDef;
use crate::combat::boss_clusters::BossStatus;

/// A named gate fired by gameplay code (rope cut, hazard impact, cutscene cue,
/// "all adds dead") to advance an [`EncounterScript`] beat waiting on it. The
/// external / scripted hook — the same role `PhaseTrigger::External` plays for
/// intrinsic phases.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct EncounterGate {
    pub gate: String,
}

impl EncounterGate {
    pub fn new(gate: impl Into<String>) -> Self {
        Self { gate: gate.into() }
    }
}

/// A condition that advances the current script beat. Observes world / members.
#[derive(Clone, Debug, PartialEq)]
pub enum EncounterTrigger {
    /// An external [`EncounterGate`] with this name fired this tick.
    Gate(String),
    /// The Nth member (by `EncounterDef.members` index) is dead (or gone).
    MemberDied(usize),
    /// Every member is dead (or gone).
    AllMembersDead,
    /// `secs` elapsed since this beat became current.
    Timer(f32),
}

/// A command the script issues when its beat's trigger fires. Commands members
/// / world. (Hazard spawning + member luring stay the cut-rope mechanism's job
/// for now — their proven feel-constants aren't reimplemented here; this is the
/// kill / presentation / chaining vocabulary.)
#[derive(Clone, Debug, PartialEq)]
pub enum EncounterEffect {
    /// Force the Nth member straight to `Death` (an environmental kill that
    /// bypasses `environmental_kill_only`).
    ForceKill(usize),
    /// Show a gameplay banner for `secs` seconds.
    Banner { text: String, secs: f32 },
    /// Set (`Some`) or clear (`None`) the adaptive-music request.
    SetMusic(Option<String>),
}

/// One scripted beat: when `when` fires, apply `then` and advance the cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct EncounterBeat {
    pub when: EncounterTrigger,
    pub then: Vec<EncounterEffect>,
}

impl EncounterBeat {
    pub fn new(when: EncounterTrigger, then: Vec<EncounterEffect>) -> Self {
        Self { when, then }
    }
}

/// An ordered beat sequence attached to an encounter entity. Advances one beat
/// per fired trigger; inert once the cursor passes the last beat.
#[derive(Component, Clone, Debug, Default)]
pub struct EncounterScript {
    pub beats: Vec<EncounterBeat>,
    cursor: usize,
    /// Seconds in the current beat (for `Timer`).
    elapsed: f32,
}

impl EncounterScript {
    pub fn new(beats: Vec<EncounterBeat>) -> Self {
        Self {
            beats,
            cursor: 0,
            elapsed: 0.0,
        }
    }

    /// True once every beat has fired.
    pub fn done(&self) -> bool {
        self.cursor >= self.beats.len()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }
}

/// Advance every encounter script: check the current beat's trigger against this
/// tick's fired gates + member state, and on a match apply the effects and step
/// the cursor. Runs in the Progression set after the encounter entity exists.
pub fn tick_encounter_scripts(
    world_time: Res<crate::WorldTime>,
    mut gates: MessageReader<EncounterGate>,
    mut scripts: Query<(&EncounterDef, &mut EncounterScript)>,
    mut members: Query<&mut BossStatus>,
    mut banner: ResMut<crate::features::GameplayBanner>,
    mut music: ResMut<crate::encounter::BossEncounterMusicRequest>,
) {
    let dt = world_time.sim_dt();
    let fired: Vec<String> = gates.read().map(|g| g.gate.clone()).collect();

    for (def, mut script) in &mut scripts {
        if script.done() {
            continue;
        }
        script.elapsed += dt;
        let beat = &script.beats[script.cursor];
        // A member is "dead" if its status is non-alive or it has left the world.
        let triggered = match &beat.when {
            EncounterTrigger::Gate(g) => fired.iter().any(|f| f == g),
            EncounterTrigger::MemberDied(i) => def
                .members
                .get(*i)
                .map_or(true, |&m| members.get(m).map_or(true, |s| !s.alive)),
            EncounterTrigger::AllMembersDead => {
                !def.members.is_empty()
                    && def
                        .members
                        .iter()
                        .all(|&m| members.get(m).map_or(true, |s| !s.alive))
            }
            EncounterTrigger::Timer(secs) => script.elapsed >= *secs,
        };
        if !triggered {
            continue;
        }
        let effects = beat.then.clone();
        for effect in &effects {
            match effect {
                EncounterEffect::ForceKill(i) => {
                    if let Some(&m) = def.members.get(*i) {
                        if let Ok(mut status) = members.get_mut(m) {
                            status.health.current = 0;
                            status.alive = false;
                            if let Some(phase) = status.encounter.as_mut() {
                                let _ = phase.kill();
                            }
                        }
                    }
                }
                EncounterEffect::Banner { text, secs } => banner.show(text.clone(), *secs),
                EncounterEffect::SetMusic(track) => music.desired_track = track.clone(),
            }
        }
        script.cursor += 1;
        script.elapsed = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_encounter::{BossEncounterPhase, BossPhaseState};
    use crate::combat::boss_clusters::BossStatus;
    use crate::encounter::BossEncounterMusicRequest;
    use crate::features::GameplayBanner;
    use crate::WorldTime;

    fn member(hp: i32) -> BossStatus {
        let mut phase = BossPhaseState::new(Vec::new());
        phase.phase = BossEncounterPhase::Phase1;
        let mut status = BossStatus {
            health: crate::actor::Health::new(hp),
            alive: true,
            hit_flash: 0.0,
            encounter_phase: BossEncounterPhase::Phase1,
            sprite_metrics: None,
            encounter: Some(phase),
        };
        status.health.current = hp;
        status
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.insert_resource(WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.add_message::<EncounterGate>();
        app.init_resource::<GameplayBanner>();
        app.init_resource::<BossEncounterMusicRequest>();
        app.add_systems(Update, tick_encounter_scripts);
        app
    }

    /// A gate-triggered ForceKill beat kills the named member when the gate fires.
    #[test]
    fn gate_beat_force_kills_its_member() {
        let mut app = test_app();
        let boss = app.world_mut().spawn(member(9999)).id();
        app.world_mut().spawn((
            EncounterDef {
                placement_id: "cut_rope".into(),
                members: vec![boss],
                hud: true,
                win: crate::boss_encounter::EncounterWin::AllMembersDead,
            },
            EncounterScript::new(vec![EncounterBeat::new(
                EncounterTrigger::Gate("impact".into()),
                vec![EncounterEffect::ForceKill(0)],
            )]),
        ));

        // No gate yet → the boss lives.
        app.update();
        assert!(app.world().entity(boss).get::<BossStatus>().unwrap().alive);

        // Fire the gate → the script force-kills the member.
        app.world_mut().write_message(EncounterGate::new("impact"));
        app.update();
        let status = app.world().entity(boss).get::<BossStatus>().unwrap();
        assert!(!status.alive);
        assert_eq!(status.health.current, 0);
        assert_eq!(
            status.encounter.as_ref().unwrap().phase,
            BossEncounterPhase::Death
        );
    }

    /// Beats advance one per fired trigger; a Timer beat fires after its delay,
    /// and effects (Banner) apply.
    #[test]
    fn beats_advance_in_order_with_timer_and_banner() {
        let mut app = test_app();
        let boss = app.world_mut().spawn(member(10)).id();
        app.world_mut().spawn((
            EncounterDef {
                placement_id: "enc".into(),
                members: vec![boss],
                hud: false,
                win: crate::boss_encounter::EncounterWin::AllMembersDead,
            },
            EncounterScript::new(vec![
                EncounterBeat::new(
                    EncounterTrigger::Gate("go".into()),
                    vec![EncounterEffect::Banner {
                        text: "BEAT 1".into(),
                        secs: 1.0,
                    }],
                ),
                EncounterBeat::new(
                    EncounterTrigger::Timer(0.1),
                    vec![EncounterEffect::ForceKill(0)],
                ),
            ]),
        ));

        // Fire the first gate → beat 0 applies, cursor advances to beat 1.
        app.world_mut().write_message(EncounterGate::new("go"));
        app.update();
        {
            let mut q = app.world_mut().query::<&EncounterScript>();
            assert_eq!(q.single(app.world()).unwrap().cursor(), 1);
        }
        assert!(app.world().entity(boss).get::<BossStatus>().unwrap().alive);

        // Tick past the 0.1s timer (1/60 per frame) → beat 1 fires the kill.
        for _ in 0..10 {
            app.update();
        }
        assert!(!app.world().entity(boss).get::<BossStatus>().unwrap().alive);
        let mut q = app.world_mut().query::<&EncounterScript>();
        assert!(q.single(app.world()).unwrap().done());
    }
}
