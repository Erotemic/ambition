//! Runtime schedule vocabulary independent of game content.
//!
//! `SimSchedule` names the Bevy schedule that advances the canonical simulation
//! tick. Presentation, device input, audio, and HUD continue to use the frame
//! schedule.

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

/// Generic platformer runtime phases.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PlatformerRuntimeSet {
    /// Build or refresh world-derived runtime inputs before actors tick.
    WorldPrep,
    /// Translate input/control intent into actor control frames.
    ControlInput,
    /// Integrate actors, held items, projectiles, and other gameplay bodies.
    ActorSimulation,
    /// Handle room unload/load, room-scoped cleanup, and authored room respawn.
    RoomLifecycle,
    /// Resolve damage, hitboxes, combat intents, and gameplay consequences.
    Combat,
    /// Publish simulation state to presentation-facing mirrors/caches.
    PresentationSync,
}

/// Startup-phase slot for the app's presentation setup (camera, root
/// UI scaffolding). Machinery that must initialize after presentation
/// setup (e.g. audio channel/cue loading) orders `.after(this set)`
/// instead of naming the app's setup system.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct PresentationSetupSet;

/// Startup-phase slot for audio initialization (channel registration, cue
/// loading).
///
/// ⭐ It exists so INSTRUMENTATION can be attached from outside. The audio
/// plugin used to bracket its own startup with
/// `ambition_dev_tools::profiling::phase_mark` calls, which made a simulation
/// crate depend on a developer tool to describe itself — the last code residue
/// of the `ambition_dev_tools` carve in `audio/`. A named slot lets the host
/// bracket the same work with the marks that already live beside every other
/// one, and the kernel stops naming a profiler.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct AudioInitSet;

/// Startup-phase slot for the host's SIMULATION setup (room geometry,
/// player spawn, sim registries). The machinery-facing label for "the
/// world exists now": engine/host systems that must initialize after
/// the sim world is set up (e.g. attaching input components to the
/// spawned player) order `.after(this set)` instead of naming the
/// host's setup system — the same inversion as [`PresentationSetupSet`].
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct SimulationSetupSet;

/// Slot inside the `WorldPrep` boss tick chain where the content layer inserts per-boss
/// steering systems (e.g. the cut-rope boss tracking its anvil).
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct BossSteerSlot;

/// The phases inside [`Platformer2dSimulationPhaseMonolith::Combat`], and the content slots between
/// them.
///
/// The engine owns the combat spine — trigger, playback, materialize, resolve,
/// settle — and named content hangs on [`Self::ContentSpecials`] /
/// [`Self::ContentFlavor`] instead of being registered inline by the app.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum CombatSet {
    /// Intent becomes a started move. Cooldowns decay, an attack gesture
    /// resolves to a verb, and a moveset move begins — for a player body and for
    /// a boss alike.
    Trigger,
    /// The move clock advances and its volumes open and close. Strike volumes
    /// are spawned and retired here, timed events fire, and the `BodyMelee`
    /// read-model is projected back for every consumer that still reads it.
    Playback,
    /// Declarations become entities. Effects execute, projectiles spawn and
    /// step, summons and programmatic actor spawns materialize. The phase that
    /// exists because a thing must EXIST before it can hit anything — the reason
    /// projectile presentation is stamped here rather than inherited later.
    Materialize,
    Resolve,
    /// Post-damage bookkeeping. Victim staging and mount/rider link
    /// enforcement — everything that reads this tick's damage outcome rather than
    /// producing it.
    Settle,
    /// Per-boss special-attack Techniques (content-owned). Sits inside
    /// [`Self::Materialize`]: a special dispatched this frame reaches its content
    /// technique THIS frame.
    ContentSpecials,
    /// Post-damage encounter flavor (content-owned). Sits between
    /// [`Self::Resolve`] and [`Self::Settle`], so it observes this frame's
    /// alive-flag transitions before the bookkeeping runs.
    ContentFlavor,
}

/// Systems that may run only while the mode allows gameplay.
///
/// ⭐⭐ ONE CONDITION, NOT EIGHTY-FOUR. Bevy evaluates a system's run condition
/// once PER SYSTEM per schedule run, and a SET's once per run however many
/// systems the set holds. `gameplay_allowed` was attached to 84 systems
/// individually, so a frame that simulated two bodies asked the same question
/// about `GameMode` 84 times and got the same answer every time.
///
/// ⛔ MEMBERSHIP IS THE GATE — do not also write `.run_if(gameplay_allowed)` on
/// a system in this set. The condition would be evaluated twice and mean
/// nothing more the second time.
///
/// ⚠ THIS SET CARRIES ITS CONDITION FROM `configure_platformer2d_simulation_phases`.
/// A composition that registers these systems without calling that function
/// leaves the set unconditioned, which does not fail to compile and does not
/// fail loudly — it silently runs gameplay systems at a menu. That is what
/// `the_gameplay_gate_is_carried_by_the_set` guards.
///
/// It is deliberately NOT nested in [`GameplaySimulationRoot`]: that set answers
/// a different question (which SESSION owns the simulation), the two gates are
/// independent, and a system may need one without the other.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct GameplayGated;

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

/// Ordered semantic phases inside [`Platformer2dSimulationPhaseMonolith::PlayerInput`].
/// Cross-crate consumers order against these sets rather than leaf systems.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PlayerInputSet {
    /// This frame's device input reaches the canonical slot model: timers,
    /// interaction buffer, the controlled subject, `SlotControls`, and the
    /// input-stream recorder. No input state is copied onto bodies.
    Device,
    /// Content-declared character data lands on the body — a registered
    /// `CharacterDefinition`'s moveset and silhouette — BEFORE the persona is
    /// constructed from it. Its own phase because the dependency is real:
    /// [`Self::Persona`] treats what this projects as its baseline.
    CharacterProjection,
    /// The canonical persona derive: action set, moveset and identity kit built
    /// together, with equipment grants overlaid. The phase most external code
    /// wants to order against, and the one that was hardest to name before.
    Persona,
    /// The universal-brain seam: this frame's slot input becomes each controlled
    /// body's `ActorControl`.
    Brain,
    /// Everything that may VETO or rewrite a published control frame — worn-kit
    /// gating, a sustained shield.
    ///
    /// ⛔⛔ IT DOES NOT LIVE IN `PlayerInput`, whatever this enum is called.
    /// Control is published TWICE — a possessed body's in [`Self::Brain`], an
    /// autonomous body's in the actor decision chain a phase later — and a
    /// restriction over control has exactly one legal home: after the LATER of
    /// them. This set is placed in `WorldPrep` between publication and
    /// integration; a copy that ran here would run before every AI frame in the
    /// world, which is why every restriction used to be registered twice (D202).
    ControlGate,
    /// Body-mode policy (crouch / morph / climb) and the actor pose sync, both
    /// of which consume the finished control frame. Placed beside
    /// [`Self::ControlGate`] and for the same reason.
    BodyMode,
}

/// Ordered semantic phases inside [`Platformer2dSimulationPhaseMonolith::Progression`].
/// Boss advance and hazards remain separate because encounter lifecycle slots lie between them.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProgressionSet {
    /// Boss encounters advance: mount-death notification, encounter + entity
    /// sync, and the participant-liveness refresh that ends with
    /// `update_encounter_progress`. Everything that DECIDES what the encounter
    /// is this frame.
    BossAdvance,
    /// What the advanced encounter then DOES: falling hazards, encounter
    /// scripts, death payloads, and the phase-transition feedback that closes
    /// the boss group.
    BossHazards,
    /// Save → ECS mirrors for actors and bosses, once the encounter state they
    /// mirror is settled.
    SaveMirror,
    /// Push and apply quest events.
    Quest,
    /// Room metadata, music request and portal phase timers — the world catching
    /// up with the progression that just happened.
    WorldSync,
    /// Map-menu visit tracking and the map's own save mirror. Last, because the
    /// dev inspector mirror anchors after it.
    Map,
}

/// Detect, apply, and reset phases inside
/// [`Platformer2dSimulationPhaseMonolith::RoomTransition`]. Hosts that replace
/// transition policy join the [`Self::Apply`] slot.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum RoomTransitionSet {
    /// Has a transition been requested? Edge/door/walk detection publishes the
    /// intent; nothing has moved yet.
    Detect,
    Apply,
    /// Per-room feature reset over the unified actor cluster, once the
    /// transition has committed. `ContentRoomResetSet` follows it, and generic
    /// plugins (gravity, portal) order against that SET rather than against any
    /// content system.
    Reset,
}

/// Movement-order anchors inside [`Platformer2dSimulationPhaseMonolith::WorldPrep`].
/// Consumers can state whether they run before or after body integration.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum WorldPrepSet {
    BeforeIntegrate,
    /// The ONE movement phase for every non-boss sim body: actor bodies and
    /// the home/player body integrate through the same engine entry.
    Integrate,
    AfterIntegrate,
    /// Shared body-contact damage pass. It is deliberately not chained after
    /// [`Self::AfterIntegrate`]; consumers add only the edge they require.
    /// Stomp resolution, for example, must precede this pass so a resolved enemy
    /// does not also damage the stomper.
    ContactDamage,
}

/// Ordered authority boundaries for one autonomous actor decision, inside
/// [`Platformer2dSimulationPhaseMonolith::WorldPrep`].
///
/// These are deliberately coarser than individual systems. The contract is
/// semantic: targeting settles first, eligibility/projections are prepared,
/// observations are frozen, reaction clocks advance, decision produces plain
/// intent values, and only then does publication mutate `ActorControl`.
/// Movement begins after the whole chain, through [`WorldPrepSet`].
///
/// ⭐ IT LIVES HERE, NOT IN THE ACTOR KERNEL, SO AN INSTRUMENT CAN NAME IT.
/// The kernel still CONFIGURES the chain — where these sets sit in the sim
/// schedule is its business and moved nowhere. But a census that bills a tick
/// to `Targeting` versus `Decide` has to order a system between two of them,
/// and `ambition_dev_tools` may not name the monolith
/// (`engine.ambition_dev_tools-source-purity`). While the enum was
/// `pub(crate)` in the kernel, the only crate that could install those marks
/// was the kernel — so the simulation package carried a registration for a
/// developer facility, which is the anti-god rule the dev-tools carve exists
/// to undo. Publishing the VOCABULARY downward is the same inversion
/// [`AudioInitSet`] already made for the startup profiler's brackets.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ActorDecisionSet {
    Targeting,
    Prepare,
    Observe,
    StateMaintenance,
    Decide,
    Publish,
}

/// Ordered possession, host extension, and outcome phases inside
/// [`Platformer2dSimulationPhaseMonolith::PlayerSimulation`].
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PlayerSimulationSet {
    /// Who is driving which body. Possession triggers and releases; a target
    /// that stopped existing hands control back.
    Possession,
    /// Host extension slot after possession settles and before damage lands.
    PostPossession,
    /// This frame's damage and death facts applied to the player body.
    /// Includes the kernel's own death path (pit, drown, tile hazard), which
    /// never reaches the hit resolver and publishes here instead.
    Outcome,
}

/// Ordered phases inside [`Platformer2dSimulationPhaseMonolith::FeatureInteraction`]. The host
/// chains these sets to encode a total cross-domain order and preserve Bevy's deferred-command
/// sync points; [`Self::SwitchIndex`] therefore observes all preceding switch mutations.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum FeatureInteractionSet {
    /// The narrative running out of lines is an INPUT to the simulation, and
    /// it lands before anything judges the conversation for separation —
    /// otherwise a conversation that ended this frame gets barked about on its
    /// way out.
    NarrativeIntake,
    /// Somebody pressed Interact: actors and switches. The phase that OPENS
    /// a conversation, which is why [`Self::Continuity`] may not precede it.
    Actuate,
    /// The break rule. AFTER [`Self::Actuate`]: a dialogue opened this
    /// frame must not be judged for separation before the bodies that opened it
    /// have been read. Both use the same `strict_intersects` reach, so a
    /// conversation cannot begin and immediately break.
    Continuity,
    /// Cast after continuity so a cut-bark is published on the same tick.
    /// Continuity decides when to bark; the cast decides what to say.
    CutBarkCast,
    /// The hold, PROJECTED — whatever [`Self::Continuity`] decided (a break,
    /// a body that stopped existing, or nothing at all), the world is made to
    /// match the authority on the same frame. it is not a "release": it both
    /// takes and releases the hold, because a projection that only let go would
    /// be a second rule about when to hold.
    HoldProjection,
    /// Interactable world objects: chests opening, breakables breaking,
    /// falling chests falling, and the save → switch mirror. Downstream of
    /// [`Self::Actuate`] because that is what opens a chest.
    WorldObjects,
    /// The encounter switch index, rebuilt last. It is a cache of
    /// `SwitchFeature + SwitchOn` over the whole world, so it must observe every
    /// switch mutation this phase makes — the Interact toggle in
    /// [`Self::Actuate`] and the save mirror in [`Self::WorldObjects`] — or the
    /// encounter arms a frame late off a stale index.
    SwitchIndex,
}

/// Run condition for systems that may advance gameplay only in [`GameMode::Playing`].
pub fn gameplay_allowed(mode: Res<State<GameMode>>) -> bool {
    mode.get().allows_gameplay()
}

/// Run condition for modes that stop simulation time. Dialogue follows
/// [`DialogueStopsTheWorld`].
pub fn gameplay_suspended(
    mode: Res<State<GameMode>>,
    dialogue_policy: Option<Res<DialogueStopsTheWorld>>,
) -> bool {
    mode.get()
        .stops_the_world(dialogue_policy.map(|p| *p).unwrap_or_default())
}

/// Coarse gameplay/session mode used to gate simulation and gameplay input.
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default, Reflect)]
pub enum GameMode {
    /// Normal gameplay: controlled actors, NPCs, enemies, hazards, room
    /// triggers, and pickups may consume gameplay inputs and advance
    /// simulation time.
    #[default]
    Playing,
    /// Simulation is stopped, but pause/menu input and developer tools remain
    /// responsive. Gameplay actions are deliberately not converted into an
    /// engine `InputState` while this mode is active.
    Paused,
    /// Text-driven interaction mode.
    Dialogue,
    /// Scripted room-load or transition-presentation mode.
    RoomTransition,
    /// Scripted cutscene or set-piece mode.
    Cutscene,
}

/// Whether dialogue freezes simulation time.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DialogueStopsTheWorld(pub bool);

impl GameMode {
    /// Whether GAMEPLAY INPUT may route this frame.
    pub fn allows_gameplay(self) -> bool {
        matches!(self, Self::Playing)
    }

    /// Whether the simulation clock freezes in this mode. Pause, room transition,
    /// and cutscene always freeze; dialogue follows [`DialogueStopsTheWorld`].
    pub fn stops_the_world(self, dialogue_policy: DialogueStopsTheWorld) -> bool {
        match self {
            Self::Playing => false,
            Self::Dialogue => dialogue_policy.0,
            Self::Paused | Self::RoomTransition | Self::Cutscene => true,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Playing => "playing",
            Self::Paused => "paused",
            Self::Dialogue => "dialogue",
            Self::RoomTransition => "room-transition",
            Self::Cutscene => "cutscene",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_gameplay_only_in_playing() {
        assert!(GameMode::Playing.allows_gameplay());
        assert!(!GameMode::Paused.allows_gameplay());
        assert!(!GameMode::Dialogue.allows_gameplay());
        assert!(!GameMode::RoomTransition.allows_gameplay());
        assert!(!GameMode::Cutscene.allows_gameplay());
    }

    #[test]
    fn default_is_playing() {
        assert_eq!(GameMode::default(), GameMode::Playing);
    }

    #[test]
    fn gameplay_suspended_is_complement_of_allowed() {
        for mode in [
            GameMode::Playing,
            GameMode::Paused,
            GameMode::Dialogue,
            GameMode::RoomTransition,
            GameMode::Cutscene,
        ] {
            assert_eq!(mode.allows_gameplay(), !gameplay_suspended_for_value(mode));
        }
    }

    #[test]
    fn labels_are_unique_and_non_empty() {
        let labels = [
            GameMode::Playing.label(),
            GameMode::Paused.label(),
            GameMode::Dialogue.label(),
            GameMode::RoomTransition.label(),
            GameMode::Cutscene.label(),
        ];
        for label in labels {
            assert!(!label.is_empty());
        }
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len(), "labels must be unique");
    }

    fn gameplay_suspended_for_value(mode: GameMode) -> bool {
        !mode.allows_gameplay()
    }
}
