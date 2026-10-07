//! The simulation schedule an outside capability joins, and the coarse phase
//! vocabulary it orders against.
//!
//! ⭐ **THE NARROW SEAM.** A capability that lives outside the engine (a content
//! pack's gameplay plugin, `examples/capability_demo`) must put its systems in
//! the host's SIMULATION schedule, not bare `Update`, so they run on the fixed
//! tick and resimulate under rollback. That needs exactly two things: which
//! schedule that is ([`SimScheduleExt::sim_schedule`]) and where in a tick to
//! stand ([`Platformer2dSimulationPhaseMonolith`]). They used to be reachable only through
//! `ambition_platformer2d_shared_tangle`, a crate named for the engine's
//! internal topology, so the extension's dependency closure grew by that
//! crate's whole tree to name two items. This crate has no engine dependency:
//! only `bevy`.
//!
//! `ambition_platformer2d_shared_tangle::schedule` re-exports every item here, so
//! the engine's own paths did not move. An extension should name THIS crate (or
//! the `ambition_platformer2d::sim` facade, which also re-exports it), not the
//! tangle crate.
//!
//! Not here, on purpose: the fine-grained sets inside each phase
//! (`PlayerInputSet`, `CombatSet`, ...). Those are the engine's own ordering and
//! are not a supported extension surface.

use core::sync::atomic::{AtomicBool, Ordering};

use bevy::app::{App, FixedUpdate, Update};
use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

/// Bevy schedule that advances the canonical simulation timeline.
///
/// Runtime construction selects `Update`, `FixedUpdate`, or a rollback schedule
/// before simulation plugins register systems. Plugins query this resource rather
/// than naming a simulation schedule directly. The value seals on first read; a
/// later change panics instead of splitting the simulation graph across schedules.
#[derive(Resource, Debug)]
pub struct SimSchedule {
    label: InternedScheduleLabel,
    /// Set once some plugin has committed systems to `label`.
    observed: AtomicBool,
}

impl Default for SimSchedule {
    fn default() -> Self {
        Self::new(Update)
    }
}

/// Host-owned marker for a historical replay pass through the simulation.
///
/// A rollback host raises it after loading historical state and clears it after the host
/// finishes servicing that rollback request batch. Diagnostic systems use the marker to avoid
/// treating replayed history as a new irreversible event while gameplay systems continue to run
/// normally.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SimulationReplayState {
    pub replaying_history: bool,
}

/// Run condition for diagnostics that should sample each authoritative tick but
/// not duplicate observations while a rollback host resimulates history.
pub fn simulation_pass_is_authoritative(replay: Option<Res<SimulationReplayState>>) -> bool {
    !replay.is_some_and(|replay| replay.replaying_history)
}

impl SimSchedule {
    pub fn new(label: impl ScheduleLabel) -> Self {
        Self {
            label: label.intern(),
            observed: AtomicBool::new(false),
        }
    }

    /// The sim schedule label, marking it sealed.
    pub fn label(&self) -> InternedScheduleLabel {
        self.observed.store(true, Ordering::Relaxed);
        self.label
    }

    /// Peek without sealing — for assertions and mode-dependent wiring.
    pub fn peek(&self) -> InternedScheduleLabel {
        self.label
    }

    /// Whether the sim schedule IS `FixedUpdate` — not whether the host steps
    /// at a fixed rate.
    ///
    /// ⛔⛤ **THE NAME IS NARROWER THAN IT READS, AND SOMETHING LOAD-BEARING
    /// DEPENDS ON THE NARROW MEANING.** A rollback host steps at a fixed tick in
    /// every ordinary sense of the phrase, and
    /// `latched_input_reaches_the_tick.rs` says so in as many words — *"Fixed60Hz,
    /// NOT Rollback. Both are fixed-tick."* This returns FALSE for it, because a
    /// rollback host's sim schedule is `GgrsSchedule`.
    ///
    /// That is what keeps the two destructive latch drains apart.
    /// `install_latched_slot_publication` installs
    /// `publish_latched_slot_controls` only when this is true, and
    /// `capture_latched_local_input` drains the same `SlotControlLatches` at
    /// `ReadInputs` under GGRS. Widen this predicate to mean "the host steps on a
    /// fixed tick" and both install: the GGRS one empties the table at
    /// `ReadInputs`, then the sim one writes NEUTRAL over every seat in
    /// `Platformer2dSimulationPhaseMonolith::PlayerInput` — inside
    /// `CoreSimulation`, while `publish_ggrs_input` is `.before(CoreSimulation)`,
    /// so nothing puts the confirmed input back. Total input loss under rollback,
    /// one predicate away.
    ///
    /// ⇒ A caller that wants "fixed stepping" in the broad sense must ask a
    /// different question; this one is about WHICH SCHEDULE systems land in.
    pub fn is_fixed_tick(&self) -> bool {
        self.is(FixedUpdate)
    }

    /// Compare the configured host schedule without sealing it. This keeps the
    /// platformer vocabulary independent of optional schedule-owner crates such
    /// as `bevy_ggrs`; callers name the label they understand.
    pub fn is(&self, label: impl ScheduleLabel) -> bool {
        self.label == label.intern()
    }
}

/// App-level accessors for [`SimSchedule`]. See that type for the contract.
pub trait SimScheduleExt {
    /// The schedule SIM systems register into. Seals the value.
    fn sim_schedule(&mut self) -> InternedScheduleLabel;

    /// Choose the sim schedule. Panics if some plugin already read a different
    /// one — see [`SimSchedule`]'s seal.
    fn set_sim_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self;

    /// Does not seal.
    fn sim_is_fixed_tick(&self) -> bool;

    /// Compare the configured host schedule without sealing it.
    fn sim_is(&self, label: impl ScheduleLabel) -> bool;
}

impl SimScheduleExt for App {
    fn sim_schedule(&mut self) -> InternedScheduleLabel {
        self.init_resource::<SimSchedule>();
        self.world().resource::<SimSchedule>().label()
    }

    fn set_sim_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self {
        let label = label.intern();
        if let Some(existing) = self.world().get_resource::<SimSchedule>() {
            assert!(
                !(existing.observed.load(Ordering::Relaxed) && existing.label != label),
                "sim schedule already sealed as {:?}; cannot change it to {:?} after a sim \
                 plugin has registered systems (that would split the sim schedule graph). \
                 Call set_sim_schedule before adding any sim plugin.",
                existing.label,
                label,
            );
        }
        self.insert_resource(SimSchedule {
            label,
            observed: AtomicBool::new(false),
        });
        self
    }

    fn sim_is_fixed_tick(&self) -> bool {
        self.world()
            .get_resource::<SimSchedule>()
            .is_some_and(SimSchedule::is_fixed_tick)
    }

    fn sim_is(&self, label: impl ScheduleLabel) -> bool {
        self.world()
            .get_resource::<SimSchedule>()
            .is_some_and(|schedule| schedule.is(label))
    }
}

/// Umbrella for every gameplay-simulation phase in the sim schedule.
/// Hosts with `SessionGatedSimulation` gate this whole set; direct/headless
/// compositions without that marker remain always-on.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct GameplaySimulationRoot;

/// Coarse simulation-order vocabulary shared by host, runtime, content, view,
/// and render. Every phase is nested inside [`GameplaySimulationRoot`].
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum Platformer2dSimulationPhaseMonolith {
    /// Top-level set that contains the six sub-sets below. Kept as a
    /// distinct label so existing `.before/.after(CoreSimulation)`
    /// constraints from presentation/audio/HUD systems continue to
    /// cover the full main chain after this finer-grained split.
    CoreSimulation,

    /// Pre-player-tick world prep: LDtk hot-reload polling, feature
    /// ECS world overlay rebuild, feature ticks (hazards / actors /
    /// bosses). Feeds the collision world that the player simulation
    /// consults.
    WorldPrep,
    /// Pre-player-tick input pipeline: dev-edit sync, input-driven
    /// reset, gameplay timer decay, interaction buffer update, and
    /// the suspended-time fallback.
    PlayerInput,
    /// Main player tick: `player_control_system` + `player_simulation_system`
    /// (control + simulation) plus the post-sim damage / safe-respawn
    /// resolver.
    PlayerSimulation,
    /// Room transition detection, apply, and per-room feature reset.
    RoomTransition,
    /// Attack lifecycle, projectile updates, and feature damage apply.
    Combat,
    /// Player ECS write-back + presentation timer decays.
    PresentationSync,

    /// Pickup collection and player heal request consumption.
    FeatureCollection,
    /// Actor/switch/chest/breakable interaction systems.
    FeatureInteraction,
    /// LDtk runtime spine index rebuild + parity check.
    LdtkRuntimeSpine,
    /// Moving platforms + encounter state + gameplay banner.
    EncounterSimulation,
    /// Auto-triggered cutscenes and cutscene drain/tick.
    Cutscene,
    /// Flag/quest/switch/boss/NPC/sfx gameplay-effect routing.
    GameplayEffects,
    /// Boss save sync, quest events, body-mode, room metadata, map sync.
    Progression,
    /// Processes resets before feature-view sync because reset mutates room and
    /// feature entities that the presentation cache must observe this frame.
    ResetProcessing,
    /// Rebuild the presentation-facing feature-view cache after every
    /// same-frame mutation to feature state.
    FeatureViewSync,
    /// Presentation-side container set for visual systems that read
    /// the feature view cache. Configured after [`Platformer2dSimulationPhaseMonolith::FeatureViewSync`].
    PresentationVisualSync,
    /// Trace recording + dump flush. Runs after CoreSimulation.
    Trace,
}
