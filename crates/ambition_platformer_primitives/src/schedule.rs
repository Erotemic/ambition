//! Runtime schedule vocabulary that is independent of Ambition content.
//!
//! `SandboxSet` remains the concrete app schedule for now. These labels document
//! the future crate-level concepts and give new runtime modules names that do
//! not depend on app assembly details.

use core::sync::atomic::{AtomicBool, Ordering};

use bevy::app::{App, FixedUpdate, Update};
use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

/// **Which Bevy schedule the SIMULATION runs in** (netcode N0.1, the two clocks).
///
/// The engine has two clocks: the **sim tick** (the canonical timeline — N0.2
/// input streams and N0.4 state hashes key on its count) and the **frame/feel**
/// clock (raw render dt, driving presentation, device sampling, and per-player
/// feel-time effects). This resource names the schedule the *tick* clock lives
/// in, and it has exactly two legal values:
///
/// - [`Update`] (**default**) — *frame-stepped*: one sim step per rendered
///   frame, dt = the frame's dt. This is Ambition today.
/// - [`FixedUpdate`] — *fixed-tick*: the sim advances on Bevy's `Time<Fixed>`
///   accumulator at a pinned rate; presentation stays in `Update`. Demos, Super
///   Smash Siblings, lockstep, and rollback need this.
///
/// Bullet-time composes **inside** the tick, never with the tick rate: in
/// fixed-tick mode `WorldTime::scaled_dt == TICK_DT × time_scale` while the
/// cadence stays pinned. Nothing ever scales the accumulator.
///
/// # Reading it
///
/// Every plugin that registers a SIM system asks the app, rather than naming a
/// schedule literal:
///
/// ```ignore
/// impl Plugin for MySimPlugin {
///     fn build(&self, app: &mut App) {
///         let sim = app.sim_schedule();
///         app.add_systems(sim, my_system.in_set(SandboxSet::WorldPrep));
///     }
/// }
/// ```
///
/// Presentation, input-device, audio, and HUD plugins keep naming [`Update`]
/// literally — they are the feel clock, and that is the point of the split.
///
/// # The seal
///
/// The value is **sealed on first read**: once any plugin has asked for the
/// label, changing it panics rather than silently splitting the schedule graph
/// in half (some sim systems in `Update`, the rest in `FixedUpdate` — a
/// split-brain whose symptom is systems mysteriously never ordering against one
/// another). Set the mode BEFORE adding any sim plugin, or let
/// `PlatformerEnginePlugins` set it as its first act.
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

    /// True when the sim advances on `Time<Fixed>` rather than the render frame.
    pub fn is_fixed_tick(&self) -> bool {
        self.label == FixedUpdate.intern()
    }
}

/// App-level accessors for [`SimSchedule`]. See that type for the contract.
pub trait SimScheduleExt {
    /// The schedule SIM systems register into. Seals the value.
    fn sim_schedule(&mut self) -> InternedScheduleLabel;

    /// Choose the sim schedule. Panics if some plugin already read a different
    /// one — see [`SimSchedule`]'s seal.
    fn set_sim_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self;

    /// True when the sim advances on `Time<Fixed>`. Does not seal.
    fn sim_is_fixed_tick(&self) -> bool;
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

/// Startup-phase slot for the host's SIMULATION setup (room geometry,
/// player spawn, sim registries). The machinery-facing label for "the
/// world exists now": engine/host systems that must initialize after
/// the sim world is set up (e.g. attaching input components to the
/// spawned player) order `.after(this set)` instead of naming the
/// host's setup system — the same inversion as [`PresentationSetupSet`].
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct SimulationSetupSet;

/// Slot inside the `WorldPrep` boss tick chain where the content layer
/// inserts per-boss steering systems (e.g. the cut-rope boss tracking
/// its anvil). Configured `.after(tick_boss_brains_system)` and
/// `.before(update_ecs_bosses)` so a content system in this set runs at
/// exactly the point the old inline registration occupied.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct BossSteerSlot;

/// Content extension slots inside the [`SandboxSet::Combat`] chain.
///
/// The engine owns the generic combat spine (action consumers → effect
/// executors → projectile step → hitbox resolution → bookkeeping); the
/// *named* Ambition content that participates in combat hangs on these
/// slots instead of being registered inline by the app. A content plugin
/// adds its systems `.in_set(CombatSet::ContentSpecials)` (or
/// `ContentFlavor`) and the app's combat schedule configures where
/// each slot sits in the chain.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum CombatSet {
    /// Per-boss special-attack Techniques (content-owned).
    ContentSpecials,
    /// Post-damage encounter flavor (content-owned).
    ContentFlavor,
}

/// Coarse simulation ordering for sandbox gameplay systems.
///
/// This is the concrete sandbox app realization of the lower
/// [`PlatformerRuntimeSet`] vocabulary, plus Ambition-specific tail phases. It
/// lives here because host, runtime, content, sim-view, and render all need to
/// order against the same labels without depending on the actor-domain crate.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum SandboxSet {
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
    /// Room transition detection + apply + per-room feature reset.
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
    /// Auto-triggered cutscenes, cutscene drain/tick.
    Cutscene,
    /// Flag/quest/switch/boss/NPC/sfx gameplay-effect routing.
    GameplayEffects,
    /// Boss save sync, quest events, body-mode, room metadata, map sync.
    Progression,
    /// Sandbox reset request processor. Joined into the main post-core
    /// chain (between `Progression` and `FeatureViewSync`) because the
    /// reset path despawns every `RoomScopedEntity` (including every
    /// `RoomVisual`) and every feature sim entity, flips the active
    /// room, and re-spawns the start room's feature set via
    /// `spawn_room_feature_entities` — all mutations the cache must
    /// observe before presentation reads it.
    ResetProcessing,
    /// Rebuild the presentation-facing feature-view cache after every
    /// same-frame mutation to feature state.
    FeatureViewSync,
    /// Presentation-side container set for visual systems that read
    /// the feature view cache. Configured after [`SandboxSet::FeatureViewSync`].
    PresentationVisualSync,
    /// Trace recording + dump flush. Runs after CoreSimulation.
    Trace,
}

/// Bevy run condition: returns `true` only in [`GameMode::Playing`].
///
/// Use this to gate simulation systems that must not run while paused,
/// in dialogue, in a room transition, or in a cutscene.
///
/// ```ignore
/// app.add_systems(Update, my_system.run_if(gameplay_allowed));
/// ```
pub fn gameplay_allowed(mode: Res<State<GameMode>>) -> bool {
    mode.get().allows_gameplay()
}

/// Bevy run condition: complement of [`gameplay_allowed`]. True in any mode
/// that suspends gameplay (paused, dialogue, room transition, cutscene).
///
/// Use this to gate the small set of systems that should only run while
/// gameplay is suspended, such as forcing world time to zero.
pub fn gameplay_suspended(mode: Res<State<GameMode>>) -> bool {
    !mode.get().allows_gameplay()
}

/// Coarse gameplay/session mode shared by runtime, input, host, and render.
///
/// `GameMode` is intentionally broader than per-entity behavior. It belongs
/// with the schedule vocabulary because it answers the same question as the
/// runtime sets: which groups of systems may mutate gameplay state this frame?
/// Enemy, chest, boss, and dialogue state machines can layer narrower state on
/// top of this coarse mode without teaching every mechanic how to pause itself.
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
    /// Reserved for NPC conversations and other text-driven interactions.
    Dialogue,
    /// Reserved for scripted room loads or door/edge transition presentation.
    RoomTransition,
    /// Reserved for future cutscenes or scripted set pieces.
    Cutscene,
}

impl GameMode {
    pub fn allows_gameplay(self) -> bool {
        matches!(self, Self::Playing)
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
