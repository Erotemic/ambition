//! Generic ENCOUNTER SCRIPT — ordered beats `{ when: Trigger, then: [Effect] }`
//! that advance as triggers fire.
//!
//! An encounter entity ([`EncounterDef`]) may carry an [`EncounterScript`] for
//! its bespoke scripted beats (a cut-rope puzzle, a timed add-wave, a gauntlet
//! cue). This is the encounter's OWN mechanism, parallel to — and sharing a
//! vocabulary with — the entity-local phase triggers ([`PhaseTrigger`]), but a
//! distinct concern: phase triggers are the boss's intrinsic self-progression;
//! the script is the orchestration the encounter layers on top.
//!
//! Triggers OBSERVE the world / members; effects COMMAND members / world. The
//! cut-rope fight is expressed ENTIRELY as a script: `Gate("rope_cut")` →
//! [`EncounterEffect::CommandMoveTo`] (lure the behemoth under the drop) +
//! [`EncounterEffect::DropHazard`] (a generic [`FallingHazard`] that hangs,
//! waits for alignment, falls, and fires its impact gate) → `ForceKill`. The
//! lure + falling-hazard are GENERIC mechanics ([`CommandedMove`] /
//! [`FallingHazard`]) any future "stand-under-the-thing" puzzle reuses; cut-rope
//! owns only its rope-cut detection + flavor visuals. The swallowed-NPC release
//! falls out of the generic [`ReleaseOnDeath`](super::encounter_entity::ReleaseOnDeath).
//!
//! See `docs/planning/boss-entity-local-refactor.md` (encounter-script shape).
//!
//! [`PhaseTrigger`]: crate::boss_encounter::PhaseTrigger
//! [`EncounterDef`]: super::encounter_entity::EncounterDef

use bevy::prelude::*;

use super::encounter_entity::EncounterDef;
use crate::combat::boss_clusters::{BossClusterRef, BossStatus};
use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use crate::features::CenteredAabb;

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
/// / world.
#[derive(Clone, Debug, PartialEq)]
pub enum EncounterEffect {
    /// Force the Nth member straight to `Death` (an environmental kill that
    /// bypasses `environmental_kill_only`).
    ForceKill(usize),
    /// Show a gameplay banner for `secs` seconds.
    Banner { text: String, secs: f32 },
    /// Set (`Some`) or clear (`None`) the adaptive-music request.
    SetMusic(Option<String>),
    /// Lure the Nth member toward `target.x` at `speed` (stopping within
    /// `arrive_tolerance`) by overriding its brain control — attaches a generic
    /// [`CommandedMove`]. The cut-rope behemoth is lured under the anvil this way.
    CommandMoveTo {
        member: usize,
        target: ae::Vec2,
        speed: f32,
        arrive_tolerance: f32,
    },
    /// Spawn a generic [`FallingHazard`] hanging at `anchor`: it waits until the
    /// `target_member` is within `align_tolerance.x`, then falls under `gravity`
    /// (capped at `terminal`) and fires `EncounterGate(impact_gate)` on contact.
    DropHazard {
        anchor: ae::Vec2,
        size: ae::Vec2,
        gravity: f32,
        terminal: f32,
        align_tolerance: f32,
        target_member: usize,
        impact_gate: String,
    },
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
    mut commands: Commands,
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
                EncounterEffect::CommandMoveTo {
                    member,
                    target,
                    speed,
                    arrive_tolerance,
                } => {
                    if let Some(&m) = def.members.get(*member) {
                        commands.entity(m).insert(CommandedMove {
                            target: *target,
                            speed: *speed,
                            arrive_tolerance: *arrive_tolerance,
                        });
                    }
                }
                EncounterEffect::DropHazard {
                    anchor,
                    size,
                    gravity,
                    terminal,
                    align_tolerance,
                    target_member,
                    impact_gate,
                } => {
                    if let Some(&target) = def.members.get(*target_member) {
                        commands.spawn((
                            CenteredAabb::from_center_size(*anchor, *size),
                            FallingHazard {
                                size: *size,
                                gravity: *gravity,
                                terminal: *terminal,
                                align_tolerance: *align_tolerance,
                                target,
                                impact_gate: impact_gate.clone(),
                                vel_y: 0.0,
                                dropping: false,
                            },
                        ));
                    }
                }
            }
        }
        script.cursor += 1;
        script.elapsed = 0.0;
    }
}

/// Generic "lured movement" override: while present on a boss, its brain control
/// is overridden to steer toward `target.x` at `speed` (stopping within
/// `arrive_tolerance`). Attached by [`EncounterEffect::CommandMoveTo`]; the
/// encounter removes it (e.g. the member dies / the script ends). Reusable by
/// any "walk the boss to a spot" beat.
#[derive(Component, Clone, Copy, Debug)]
pub struct CommandedMove {
    pub target: ae::Vec2,
    pub speed: f32,
    pub arrive_tolerance: f32,
}

/// Steer every [`CommandedMove`] boss toward its target, overriding the brain's
/// `ActorControl` (and clearing its attack intent). Runs in the boss steer slot
/// (between the brain tick and the body integrate).
pub fn tick_commanded_moves(
    mut bosses: Query<(
        BossClusterRef,
        &mut crate::brain::ActorControl,
        &mut crate::brain::BossAttackState,
        &CommandedMove,
    )>,
) {
    for (feature, mut control, mut attack_state, cmd) in &mut bosses {
        let boss = feature.as_boss_ref();
        if !boss.status.alive {
            continue;
        }
        let dx = cmd.target.x - boss.kin.pos.x;
        attack_state.clear();
        control.0.melee_pressed = false;
        control.0.special_pressed = false;
        control.0.facing = if dx.abs() > 2.0 {
            dx.signum()
        } else {
            boss.kin.facing
        };
        control.0.velocity_target = if dx.abs() <= cmd.arrive_tolerance {
            ae::Vec2::ZERO
        } else {
            ae::Vec2::new(dx.signum() * cmd.speed, 0.0)
        };
    }
}

/// A generic hazard that hangs at its spawn point until its `target` is aligned
/// under it (within `align_tolerance` in x), then falls under `gravity` (capped
/// at `terminal`) and fires `EncounterGate(impact_gate)` on contact with the
/// target — then despawns. The cut-rope anvil/piano is one of these.
#[derive(Component, Clone, Debug)]
pub struct FallingHazard {
    pub size: ae::Vec2,
    pub gravity: f32,
    pub terminal: f32,
    pub align_tolerance: f32,
    pub target: Entity,
    pub impact_gate: String,
    pub vel_y: f32,
    pub dropping: bool,
}

/// Integrate every [`FallingHazard`]: wait for the target to align, then fall +
/// clamp to the floor + fire the impact gate on contact. Despawns the hazard on
/// impact (or if its target left the world).
pub fn tick_falling_hazards(
    mut commands: Commands,
    world_time: Res<crate::WorldTime>,
    world: Res<crate::GameWorld>,
    mut gates: MessageWriter<EncounterGate>,
    mut hazards: Query<(Entity, &mut CenteredAabb, &mut FallingHazard)>,
    targets: Query<&CenteredAabb, Without<FallingHazard>>,
) {
    let dt = world_time.sim_dt().max(0.0);
    for (entity, mut aabb, mut hazard) in &mut hazards {
        let Ok(target) = targets.get(hazard.target) else {
            // Target gone (room change / despawn) — retire the hazard.
            commands.entity(entity).despawn();
            continue;
        };
        if !hazard.dropping {
            if (target.center.x - aabb.center.x).abs() <= hazard.align_tolerance {
                hazard.dropping = true;
            } else {
                continue;
            }
        }
        hazard.vel_y = (hazard.vel_y + hazard.gravity * dt).min(hazard.terminal);
        aabb.center.y += hazard.vel_y * dt;
        let floor_y = world.0.size.y - hazard.size.y * 0.5;
        if aabb.center.y > floor_y {
            aabb.center.y = floor_y;
            hazard.vel_y = 0.0;
        }
        if aabb.aabb().strict_intersects(target.aabb()) {
            gates.write(EncounterGate::new(hazard.impact_gate.clone()));
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_encounter::BossEncounterPhase;
    use crate::combat::boss_clusters::test_support::{test_boss_config, test_boss_status};
    use crate::combat::boss_clusters::BossStatus;
    use crate::encounter::BossEncounterMusicRequest;
    use crate::features::GameplayBanner;
    use crate::WorldTime;

    fn member(hp: i32) -> BossStatus {
        test_boss_status(hp, BossEncounterPhase::Phase1)
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

    fn boss_config() -> crate::combat::boss_clusters::BossConfig {
        test_boss_config("b", "B", "mockingbird")
    }

    /// A `CommandedMove` steers the boss's control toward the target's x.
    #[test]
    fn commanded_move_steers_the_boss_toward_target() {
        let mut app = App::new();
        app.add_systems(Update, tick_commanded_moves);
        let boss = app
            .world_mut()
            .spawn((
                crate::features::BodyKinematics {
                    pos: ae::Vec2::ZERO,
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::splat(40.0),
                    facing: -1.0,
                },
                boss_config(),
                member(100),
                crate::brain::ActorControl::default(),
                crate::brain::BossAttackState::default(),
                CommandedMove {
                    target: ae::Vec2::new(300.0, 0.0),
                    speed: 150.0,
                    arrive_tolerance: 10.0,
                },
            ))
            .id();

        app.update();

        let control = app
            .world()
            .entity(boss)
            .get::<crate::brain::ActorControl>()
            .unwrap();
        assert!(
            control.0.velocity_target.x > 0.0,
            "the boss is lured toward the +x target"
        );
        assert_eq!(control.0.facing, 1.0, "and faces the target");
    }

    /// An aligned `FallingHazard` falls onto its target and fires its impact gate.
    #[test]
    fn falling_hazard_drops_when_aligned_and_fires_impact_gate() {
        let mut app = App::new();
        app.insert_resource(WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.insert_resource(crate::GameWorld(ae::World::new(
            "t",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(50.0, 50.0),
            Vec::new(),
        )));
        app.add_message::<EncounterGate>();
        app.add_systems(Update, tick_falling_hazards);

        // Target sits directly below the hazard anchor (aligned in x).
        let target = app
            .world_mut()
            .spawn(CenteredAabb::from_center_size(
                ae::Vec2::new(500.0, 500.0),
                ae::Vec2::splat(40.0),
            ))
            .id();
        app.world_mut().spawn((
            CenteredAabb::from_center_size(ae::Vec2::new(500.0, 100.0), ae::Vec2::splat(60.0)),
            FallingHazard {
                size: ae::Vec2::splat(60.0),
                gravity: 1400.0,
                terminal: 920.0,
                align_tolerance: 50.0,
                target,
                impact_gate: "boom".into(),
                vel_y: 0.0,
                dropping: false,
            },
        ));

        let mut fired = false;
        for _ in 0..180 {
            app.update();
            let msgs = app
                .world()
                .resource::<bevy::ecs::message::Messages<EncounterGate>>();
            if msgs
                .iter_current_update_messages()
                .any(|g| g.gate == "boom")
            {
                fired = true;
                break;
            }
        }
        assert!(
            fired,
            "an aligned hazard falls onto its target and fires its impact gate"
        );
        // The hazard despawns on impact.
        let mut q = app.world_mut().query::<&FallingHazard>();
        assert_eq!(q.iter(app.world()).count(), 0, "hazard retires on impact");
    }
}
